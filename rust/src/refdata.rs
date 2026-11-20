//! Public reference data that can explain a rich or cheap Treasury security. Three free,
//! documented sources; each file is cached under `data/raw/`:
//!
//! - New York Fed Markets Data API, secured reference rates (SOFR, BGCR, TGCR). These are
//!   general collateral rates. A special repo rate for one CUSIP is not free data.
//! - New York Fed SOMA holdings of Treasury securities by CUSIP (weekly): par held and the
//!   share of the amount outstanding.
//! - U.S. Treasury Fiscal Data API: Monthly Statement of the Public Debt table 5 (amount
//!   outstanding and amount in stripped form for each CUSIP) and the auction records (issue
//!   date, original term).
//!
//! The requests use the client name `FinancialAnalysis fa (personal research)` and wait
//! 2 seconds between requests. A refusal is an error; the code does not retry.

use crate::curve::{self, Day};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const USER_AGENT: &str = "FinancialAnalysis fa (personal research)";
pub const DELAY_SECONDS: f64 = 2.0;
const NYFED: &str = "https://markets.newyorkfed.org/api";
const FISCAL: &str = "https://api.fiscaldata.treasury.gov/services/api/fiscal_service/v1";

/// A cache directory (`data/raw`). A download goes through the queue of its host (`hostnet`).
pub struct Fetcher {
    root: PathBuf,
}

impl Fetcher {
    pub fn new(cache_root: &Path) -> Fetcher {
        Fetcher { root: cache_root.to_path_buf() }
    }

    /// The text of `rel` from the cache, else from `url`. A download is cached only when
    /// `valid` accepts it.
    pub fn cached(&mut self, rel: &str, url: &str, valid: &dyn Fn(&str) -> bool) -> Result<String, String> {
        let path = self.root.join(rel);
        if let Ok(t) = std::fs::read_to_string(&path) {
            if valid(&t) {
                return Ok(t);
            }
        }
        let bytes = crate::hostnet::get(url, &[], DELAY_SECONDS)?;
        let text = String::from_utf8(bytes).map_err(|_| format!("{url}: the answer is not UTF-8 text"))?;
        if !valid(&text) {
            return Err(format!("{url}: the answer did not parse as expected"));
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, &text);
        Ok(text)
    }
}

fn s(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_string)
}

/// A number in a string field; "", "null" and text that is not a number give `None`.
fn num(v: &Value, key: &str) -> Option<f64> {
    match v.get(key)? {
        Value::String(t) => t.trim().parse().ok(),
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// New York Fed: secured reference rates
// ---------------------------------------------------------------------------

/// (effective date, rate in percent) from the answer of `.../rates/secured/<type>/search.json`.
pub fn parse_secured_rates(json: &str) -> Result<Vec<(String, f64)>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("secured rates: {e}"))?;
    let rows = v.get("refRates").and_then(|x| x.as_array()).ok_or("secured rates: no refRates list")?;
    let mut out: Vec<(String, f64)> = rows
        .iter()
        .filter_map(|r| Some((s(r, "effectiveDate")?, num(r, "percentRate")?)))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// SOFR, BGCR and TGCR (percent) by date from `from` to `to`, for the dates that all
/// three have. One request for each rate.
pub fn secured_rates(f: &mut Fetcher, from: &str, to: &str) -> Result<BTreeMap<String, [f64; 3]>, String> {
    let mut table: BTreeMap<String, [f64; 3]> = BTreeMap::new();
    for (i, name) in ["sofr", "bgcr", "tgcr"].iter().enumerate() {
        let url = format!("{NYFED}/rates/secured/{name}/search.json?startDate={from}&endDate={to}");
        let text = f.cached(&format!("nyfed/secured_{name}_{from}_{to}.json"), &url, &|t| parse_secured_rates(t).is_ok())?;
        for (d, r) in parse_secured_rates(&text)? {
            table.entry(d).or_insert([f64::NAN; 3])[i] = r;
        }
    }
    table.retain(|_, v| v.iter().all(|x| x.is_finite()));
    Ok(table)
}

// ---------------------------------------------------------------------------
// New York Fed: SOMA holdings of Treasury securities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Soma {
    pub cusip: String,
    /// Par value held (dollars).
    pub par: f64,
    /// Share of the amount outstanding that the SOMA holds (a fraction, 0.07 = 7 %).
    pub share: f64,
    pub security_type: String,
}

pub fn parse_soma_dates(json: &str) -> Result<Vec<String>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("SOMA dates: {e}"))?;
    let list = v.pointer("/soma/asOfDates").and_then(|x| x.as_array()).ok_or("SOMA dates: no asOfDates list")?;
    let mut out: Vec<String> = list.iter().filter_map(|d| d.as_str().map(str::to_string)).collect();
    out.sort();
    Ok(out)
}

pub fn parse_soma(json: &str) -> Result<Vec<Soma>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("SOMA holdings: {e}"))?;
    let rows = v.pointer("/soma/holdings").and_then(|x| x.as_array()).ok_or("SOMA holdings: no holdings list")?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            Some(Soma {
                cusip: s(r, "cusip")?,
                par: num(r, "parValue")?,
                share: num(r, "percentOutstanding").unwrap_or(0.0),
                security_type: s(r, "securityType").unwrap_or_default(),
            })
        })
        .collect())
}

/// The latest SOMA as-of date that is on or before `date` (the file is weekly).
pub fn soma_asof(f: &mut Fetcher, date: &str) -> Result<String, String> {
    // A cached holdings file within 7 days before `date` is the answer; no request.
    let dir = f.root.join("nyfed");
    let mut cached: Vec<String> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter_map(|n| n.strip_prefix("soma_tsy_")?.strip_suffix(".json").map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    cached.sort();
    if let Some(d) = cached.iter().rev().find(|d| d.as_str() <= date) {
        if let (Some(a), Some(b)) = (curve::parse_iso(d), curve::parse_iso(date)) {
            if b - a < 7 {
                return Ok(d.clone());
            }
        }
    }
    // The list of dates is cached for one day.
    let list_path = f.root.join("nyfed/soma_asofdates.json");
    let old = std::fs::metadata(&list_path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map_or(true, |age| age > std::time::Duration::from_secs(24 * 3600));
    if old {
        let _ = std::fs::remove_file(&list_path);
    }
    let text = f.cached("nyfed/soma_asofdates.json", &format!("{NYFED}/soma/asofdates/list.json"), &|t| parse_soma_dates(t).is_ok())?;
    let list = parse_soma_dates(&text)?;
    list.into_iter().rev().find(|d| d.as_str() <= date).ok_or_else(|| format!("no SOMA as-of date on or before {date}"))
}

/// SOMA Treasury holdings by CUSIP for the as-of date.
pub fn soma_holdings(f: &mut Fetcher, asof: &str) -> Result<BTreeMap<String, Soma>, String> {
    let url = format!("{NYFED}/soma/tsy/get/asof/{asof}.json");
    let text = f.cached(&format!("nyfed/soma_tsy_{asof}.json"), &url, &|t| parse_soma(t).map_or(false, |h| !h.is_empty()))?;
    Ok(parse_soma(&text)?.into_iter().map(|h| (h.cusip.clone(), h)).collect())
}

// ---------------------------------------------------------------------------
// Fiscal Data: Monthly Statement of the Public Debt, table 5 (stripped securities)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Stripped {
    /// CUSIP of the note or bond (not of its strip).
    pub cusip: String,
    pub maturity: String,
    /// Amount outstanding (dollars; the table has thousands).
    pub outstanding: f64,
    /// Amount held in stripped form (dollars).
    pub stripped: f64,
}

fn fiscal_rows(json: &str, what: &str) -> Result<Vec<Value>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("{what}: {e}"))?;
    v.get("data").and_then(|x| x.as_array()).cloned().ok_or_else(|| format!("{what}: no data list"))
}

pub fn parse_mspd5(json: &str) -> Result<Vec<Stripped>, String> {
    Ok(fiscal_rows(json, "MSPD table 5")?
        .iter()
        .filter_map(|r| {
            Some(Stripped {
                // `cusip` is the CUSIP of the principal strip; `security_class2_desc` is the
                // CUSIP of the note or bond.
                cusip: s(r, "security_class2_desc")?,
                maturity: s(r, "maturity_date")?,
                outstanding: num(r, "outstanding_amt")? * 1000.0,
                stripped: num(r, "portion_stripped_amt")? * 1000.0,
            })
        })
        .collect())
}

/// The last day of the month before `day`, and of the month before that (ISO strings).
fn month_ends_before(day: Day) -> [String; 2] {
    let (y, m, _) = curve::civil_from_days(day);
    let first = curve::days_from_civil(y, m, 1);
    let e1 = first - 1;
    let (y1, m1, _) = curve::civil_from_days(e1);
    let e2 = curve::days_from_civil(y1, m1, 1) - 1;
    [curve::iso(e1), curve::iso(e2)]
}

/// Table 5 for the latest month end that the Fiscal Service has published before `date`.
/// Returns (record date, rows by CUSIP).
pub fn mspd5(f: &mut Fetcher, date: &str) -> Result<(String, BTreeMap<String, Stripped>), String> {
    let day = curve::parse_iso(date).ok_or("bad date")?;
    for rec in month_ends_before(day) {
        let url = format!(
            "{FISCAL}/debt/mspd/mspd_table_5?filter=record_date:eq:{rec}&page%5Bsize%5D=10000&format=json"
        );
        if let Ok(text) = f.cached(&format!("fiscaldata/mspd_table_5_{rec}.json"), &url, &|t| parse_mspd5(t).map_or(false, |r| !r.is_empty())) {
            let rows = parse_mspd5(&text)?;
            return Ok((rec, rows.into_iter().map(|r| (r.cusip.clone(), r)).collect()));
        }
    }
    Err(format!("no MSPD table 5 for the two month ends before {date}"))
}

// ---------------------------------------------------------------------------
// Fiscal Data: auction records of notes and bonds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Auction {
    pub cusip: String,
    /// "Note" or "Bond".
    pub security_type: String,
    /// The term of this auction (a reopening has a shorter term, e.g. "29-Year 6-Month").
    pub term: String,
    /// The term of the first issue, e.g. "30-Year".
    pub original_term: String,
    pub auction_date: String,
    pub issue_date: String,
    pub maturity: String,
    pub reopening: bool,
}

pub fn parse_auctions(json: &str) -> Result<Vec<Auction>, String> {
    Ok(fiscal_rows(json, "auctions")?
        .iter()
        .filter_map(|r| {
            Some(Auction {
                cusip: s(r, "cusip")?,
                security_type: s(r, "security_type")?,
                term: s(r, "security_term")?,
                original_term: s(r, "original_security_term")?,
                auction_date: s(r, "auction_date")?,
                issue_date: s(r, "issue_date")?,
                maturity: s(r, "maturity_date")?,
                reopening: s(r, "reopening")? == "Yes",
            })
        })
        .collect())
}

/// Auction records of all notes and bonds that mature on or after `date` and were auctioned
/// on or before `date`. Fields are limited to the ones used (the full record has 80 fields).
pub fn auctions(f: &mut Fetcher, date: &str) -> Result<Vec<Auction>, String> {
    let url = format!(
        "{FISCAL}/accounting/od/auctions_query?fields=cusip,security_type,security_term,original_security_term,auction_date,issue_date,maturity_date,reopening\
         &filter=security_type:in:(Note,Bond),maturity_date:gte:{date},auction_date:lte:{date}&page%5Bsize%5D=10000&sort=maturity_date,auction_date&format=json"
    );
    let text = f.cached(&format!("fiscaldata/auctions_notes_bonds_{date}.json"), &url, &|t| parse_auctions(t).map_or(false, |r| !r.is_empty()))?;
    parse_auctions(&text)
}

/// "10-Year" or "9-Year 11-Month" in months.
pub fn term_months(term: &str) -> Option<i64> {
    let mut total = 0;
    let mut found = false;
    for part in term.split_whitespace() {
        let (n, unit) = part.split_once('-')?;
        let n: i64 = n.parse().ok()?;
        match unit {
            "Year" => total += 12 * n,
            "Month" => total += n,
            _ => return None,
        }
        found = true;
    }
    found.then_some(total)
}

/// The standard term of an original term: a new issue can have a short first period
/// ("29-Year 9-Month" is a 30-year bond; "9-Year 10-Month" is a 10-year note). The terms
/// 2, 3, 5, 7, 10, 20 and 30 years are the standard ones; a term that is within 3 months
/// below or 4 months above one of them is that one. Any other term stays as the text gives it.
pub fn standard_term(term: &str) -> String {
    match term_months(term) {
        Some(m) => match [24, 36, 60, 84, 120, 240, 360].iter().find(|s| m >= **s - 3 && m <= **s + 4) {
            Some(s) => format!("{}-Year", s / 12),
            None => term.to_string(),
        },
        None => term.to_string(),
    }
}

/// What the auction records say about one CUSIP.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    /// Date of the first issue (ISO).
    pub first_issue: String,
    /// The standard original term ("30-Year"), see `standard_term`.
    pub original_term: String,
    /// Number of auctions of this CUSIP (1 = no reopening).
    pub auctions: usize,
}

/// The first issue date, the original term and the auction count of each CUSIP, and the
/// on-the-run CUSIP of each original term: the CUSIP of the latest auction of that term
/// that settled on or before `date`.
pub fn issues(a: &[Auction], date: &str) -> (BTreeMap<String, Issue>, BTreeMap<String, String>) {
    let mut by: BTreeMap<String, Issue> = BTreeMap::new();
    let mut latest: BTreeMap<String, (String, String)> = BTreeMap::new();
    for r in a {
        let e = by.entry(r.cusip.clone()).or_insert_with(|| Issue {
            first_issue: r.issue_date.clone(),
            original_term: standard_term(&r.original_term),
            auctions: 0,
        });
        e.auctions += 1;
        if r.issue_date < e.first_issue {
            e.first_issue = r.issue_date.clone();
        }
        if r.issue_date.as_str() <= date {
            let slot = latest.entry(standard_term(&r.original_term)).or_insert_with(|| (String::new(), String::new()));
            if r.auction_date > slot.0 {
                *slot = (r.auction_date.clone(), r.cusip.clone());
            }
        }
    }
    (by, latest.into_iter().map(|(k, (_, c))| (k, c)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secured_rates_parse_and_sort() {
        let j = r#"{ "refRates": [ { "effectiveDate": "2026-11-18", "type": "SOFR" ,"percentRate": 3.90 ,"volumeInBillions": 3134 },
            { "effectiveDate": "2026-11-17", "type": "SOFR" ,"percentRate": 3.92 }, { "effectiveDate": "2026-11-16", "type": "SOFR" } ] }"#;
        let r = parse_secured_rates(j).unwrap();
        assert_eq!(r, vec![("2026-11-17".to_string(), 3.92), ("2026-11-18".to_string(), 3.90)]);
        assert!(parse_secured_rates("{}").is_err());
        assert!(parse_secured_rates("not json").is_err());
    }

    #[test]
    fn soma_parse_share_and_dates() {
        let j = r#"{"soma": { "holdings": [
          { "asOfDate": "2026-11-10", "cusip": "912810FT0", "maturityDate": "2036-02-15", "coupon": "4.5", "parValue": "1234500000", "percentOutstanding": "0.0467", "securityType": "Bonds" },
          { "asOfDate": "2026-11-10", "cusip": "912797UY1", "parValue": "5", "percentOutstanding": "", "securityType": "Bills" },
          { "asOfDate": "2026-11-10", "parValue": "5" } ] } }"#;
        let h = parse_soma(j).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0], Soma { cusip: "912810FT0".into(), par: 1234500000.0, share: 0.0467, security_type: "Bonds".into() });
        assert_eq!(h[1].share, 0.0);
        let d = parse_soma_dates(r#"{"soma":{"asOfDates":["2026-11-10","2026-11-04"]}}"#).unwrap();
        assert_eq!(d, vec!["2026-11-04".to_string(), "2026-11-10".to_string()]);
        assert!(parse_soma("{}").is_err());
    }

    #[test]
    fn mspd5_units_are_thousands() {
        let j = r#"{"data":[{"record_date":"2026-10-31","cusip":"912803CX9","security_class1_desc":"Treasury Bonds","security_class2_desc":"912810FT0","interest_rate_pct":"4.5","maturity_date":"2036-02-15","outstanding_amt":"26397130","portion_unstripped_amt":"24358281.8","portion_stripped_amt":"2038848.2","reconstituted_amt":"0"},
                    {"record_date":"2026-10-31","cusip":"Y","security_class2_desc":"X","maturity_date":"2030-01-01","outstanding_amt":"null","portion_stripped_amt":"1"}]}"#;
        let r = parse_mspd5(j).unwrap();
        assert_eq!(r.len(), 1);
        assert!((r[0].outstanding - 26_397_130_000.0).abs() < 1.0 && (r[0].stripped - 2_038_848_200.0).abs() < 1.0);
    }

    #[test]
    fn month_ends() {
        let d = curve::parse_iso("2026-11-18").unwrap();
        assert_eq!(month_ends_before(d), ["2026-10-31".to_string(), "2026-09-30".to_string()]);
        let d = curve::parse_iso("2027-01-05").unwrap();
        assert_eq!(month_ends_before(d), ["2026-12-31".to_string(), "2026-11-30".to_string()]);
    }

    fn auc(cusip: &str, term: &str, orig: &str, auction: &str, issue: &str, re: bool) -> Auction {
        Auction {
            cusip: cusip.into(),
            security_type: "Bond".into(),
            term: term.into(),
            original_term: orig.into(),
            auction_date: auction.into(),
            issue_date: issue.into(),
            maturity: "2036-02-15".into(),
            reopening: re,
        }
    }

    #[test]
    fn auctions_parse_and_issue_facts() {
        let j = r#"{"data":[{"cusip":"912810FT0","security_type":"Bond","security_term":"29-Year 6-Month","original_security_term":"30-Year","auction_date":"2006-08-10","issue_date":"2006-08-15","maturity_date":"2036-02-15","reopening":"Yes"}]}"#;
        let a = parse_auctions(j).unwrap();
        assert_eq!(a.len(), 1);
        assert!(a[0].reopening && a[0].original_term == "30-Year");
        let all = vec![
            auc("A", "30-Year", "30-Year", "2006-02-09", "2006-02-15", false),
            auc("A", "29-Year 6-Month", "30-Year", "2006-08-10", "2006-08-15", true),
            auc("B", "30-Year", "30-Year", "2026-02-12", "2026-02-17", false),
            auc("C", "30-Year", "30-Year", "2026-11-12", "2026-11-17", true),
            auc("D", "30-Year", "30-Year", "2026-11-19", "2026-11-24", true), // settles after the date
        ];
        let (by, otr) = issues(&all, "2026-11-18");
        assert_eq!(by["A"], Issue { first_issue: "2006-02-15".into(), original_term: "30-Year".into(), auctions: 2 });
        assert_eq!(otr["30-Year"], "C");
        assert_eq!(standard_term("29-Year 9-Month"), "30-Year");
        assert_eq!(standard_term("9-Year 10-Month"), "10-Year");
        assert_eq!(standard_term("19-Year 10-Month"), "20-Year");
        assert_eq!(standard_term("30-Year 3-Month"), "30-Year");
        assert_eq!(standard_term("4-Year 6-Month"), "4-Year 6-Month");
        assert_eq!(term_months("9-Year 11-Month"), Some(119));
        assert_eq!(term_months("nonsense"), None);
    }
}
