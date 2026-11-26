//! `fa fund holdings TICKER`: the holdings of an ETF from its newest SEC N-PORT report (form NPORT-P).
//!
//! Steps (3 requests for a new ticker, at most 5 in the worst case):
//! 1. The SEC file of fund tickers (`company_tickers_mf.json`: cik, seriesId, classId, symbol) gives the trust and the
//!    series id. It is cached in `<cache>/_sec/`.
//! 2. The EDGAR filing list of the SERIES (the series id is a filer key) gives the newest NPORT-P filings. A trust
//!    files one NPORT-P for each series, so the list of the trust would need many reads.
//! 3. The `primary_doc.xml` of the filing is read and kept in `<cache>/<TICKER>/nport/<accession>.xml`.
//!
//! The SEC publishes a N-PORT report about 60 days after the quarter end, for the third month of the quarter only.
//! The data is public domain.

use super::{today_iso, write_atomic};
use mapcheck::http::Client;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa fund holdings - the holdings of an ETF from its newest SEC N-PORT report

USAGE:
  fa fund holdings TICKER [--cache DIR] [--top N] [--from YYYY-MM-DD] [--to YYYY-MM-DD]
    --cache DIR   default data/raw. The ticker file is DIR/_sec/company_tickers_mf.json, the report is
                  DIR/TICKER/nport/ACCESSION.xml (and ACCESSION.meta.json: source, address, date, form, filing date).
    --top N       the number of holdings shown (default 25), sorted by the signed pctVal (a short position is last)
    --from D      history: read each report with a report date from D (and up to --to), one line for each report.
                  Rule: an NPORT-P is used; an NPORT-P/A (amendment) is used only if the NPORT-P of that report date is missing.
                  A report that is in the cache costs no request.
Source: SEC N-PORT (public domain). The newest public report is about 60 days old.";

const TICKERS_URL: &str = "https://www.sec.gov/files/company_tickers_mf.json";
/// Seconds between two requests (the SEC limit is 10 requests in one second).
const PAUSE: f64 = 0.2;
/// The number of filings of the series that are read before the program stops.
const MAX_FILINGS: usize = 3;
/// The number of filings in the list for a history run (the EDGAR limit of one page is 100).
const HISTORY_LIST: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub cik: u64,
    pub series_id: String,
    pub class_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filing {
    pub accession: String,
    pub form: String,
    pub filed: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Holding {
    pub name: String,
    pub cusip: String,
    pub isin: String,
    pub val_usd: f64,
    pub pct_val: f64,
    pub balance: f64,
    pub units: String,
    /// The currency of the line (`curCd`): `USD`, or the currency of `currencyConditional`. Empty if absent.
    pub currency: String,
    /// `exchangeRt` of `currencyConditional`: units of the line currency for one US dollar. `None` for a USD line.
    pub exchange_rt: Option<f64>,
    /// The `ticker` identifier of the line, if the filer gives one (a repo line gives a counterparty code, not a stock ticker).
    pub ticker: String,
    /// `assetCat`: EC equity common, RA repurchase agreement, STIV short-term investment vehicle, DE derivative, and so on.
    pub asset_cat: String,
    /// `invCountry`: the country of investment (ISO code).
    pub country: String,
    /// `payoffProfile`: `Long` or `Short`.
    pub payoff: String,
}

/// The monthly total returns of one share class, in percent, for the 3 months of the fiscal quarter that ends at the
/// report date (`monthlyTotReturn` rtn1, rtn2, rtn3). MEASURED 2026-11-25 for URA against the Nasdaq month-end closes:
/// rtn1 is the NEWEST month (the month of the report date) and rtn3 the oldest (11 of 13 reports clear, 2 undecided). The
/// monthly flows `mon1Flow` to `mon3Flow` probably follow the same order (not verified).
#[derive(Debug, Clone, PartialEq)]
pub struct MonthlyReturn {
    pub class_id: String,
    pub rtn: [Option<f64>; 3],
}

/// The flows of one month (`monNFlow`): the shares sold, redeemed and reinvested, in US dollars.
#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    pub sales: Option<f64>,
    pub redemption: Option<f64>,
    pub reinvestment: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub series_id: String,
    pub report_date: String,
    /// `submissionType`: `NPORT-P`, or `NPORT-P/A` for an amendment. Empty if absent.
    pub form: String,
    /// `repPdEnd`: the end of the fiscal period (not the date of the holdings).
    pub rep_pd_end: String,
    pub tot_assets: Option<f64>,
    pub tot_liabs: Option<f64>,
    pub net_assets: Option<f64>,
    pub monthly: Vec<MonthlyReturn>,
    pub flows: [Option<Flow>; 3],
    pub holdings: Vec<Holding>,
    /// The count of values (pctVal, valUSD, balance) that are missing or not a number (they are 0.0 in the rows).
    pub bad_numbers: usize,
}

/// The series of a ticker in the text of `company_tickers_mf.json`.
pub fn series_of(json: &str, ticker: &str) -> Result<Series, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("company_tickers_mf.json: {e}"))?;
    let rows = v["data"].as_array().ok_or("company_tickers_mf.json: no data array")?;
    for r in rows {
        if r[3].as_str().is_some_and(|s| s.eq_ignore_ascii_case(ticker)) {
            return Ok(Series {
                cik: r[0].as_u64().ok_or("company_tickers_mf.json: bad cik")?,
                series_id: r[1].as_str().unwrap_or("").to_string(),
                class_id: r[2].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Err(format!("{ticker}: not in the SEC file of fund tickers (an ETF or a mutual fund is needed)"))
}

/// The text of the first `<tag>` in `s` (a tag with no prefix), trimmed.
fn tag<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let a = s.find(&open)? + open.len();
    let b = s[a..].find(&format!("</{name}>"))? + a;
    Some(s[a..b].trim())
}

/// The value of the first `<tag attr="..."` in `s`.
fn attr<'a>(s: &'a str, name: &str, attr: &str) -> Option<&'a str> {
    let a = s.find(&format!("<{name} "))?;
    let end = s[a..].find('>')? + a;
    el_attr(&s[a..end], attr)
}

/// The value of `attr="..."` in the text of one start tag.
fn el_attr<'a>(el: &'a str, attr: &str) -> Option<&'a str> {
    // The name must start after a space, so that `rtn1` does not match inside another name.
    let k = el.find(&format!(" {attr}=\""))? + attr.len() + 3;
    let e = el[k..].find('"')? + k;
    Some(&el[k..e])
}

fn opt_num(v: Option<&str>) -> Option<f64> {
    v.and_then(|t| t.trim().parse::<f64>().ok()).filter(|x| x.is_finite())
}

pub(crate) fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';').filter(|e| *e <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "amp" => Some('&'),
            _ => ent
                .strip_prefix("#x")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// An accession number is 10 digits, 2 digits, 6 digits (it goes into a path and a URL).
pub fn valid_accession(a: &str) -> bool {
    let p: Vec<&str> = a.split('-').collect();
    p.len() == 3
        && [10, 2, 6].iter().zip(&p).all(|(n, x)| x.len() == *n && x.bytes().all(|b| b.is_ascii_digit()))
}

/// A ticker is letters, digits, `.` and `-` only.
pub fn valid_ticker(t: &str) -> bool {
    t.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric) && t.len() <= 12 && t.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
}

/// The filings in the Atom answer of EDGAR, in the order of the answer. The type filter of EDGAR is a prefix match:
/// the caller keeps form `NPORT-P` only (an amendment is `NPORT-P/A`).
pub fn parse_filings(atom: &str) -> Vec<Filing> {
    atom.split("<entry>")
        .skip(1)
        .filter_map(|e| {
            Some(Filing {
                accession: tag(e, "accession-number")?.to_string(),
                form: tag(e, "filing-type").unwrap_or("").to_string(),
                filed: tag(e, "filing-date")?.to_string(),
            })
        })
        .collect()
}

/// The report in the text of `primary_doc.xml`.
pub fn parse_report(xml: &str) -> Result<Report, String> {
    let gen = tag(xml, "genInfo").ok_or("no genInfo in the report")?;
    let series_id = tag(gen, "seriesId").ok_or("no seriesId")?.to_string();
    let report_date = tag(xml, "repPdDate").ok_or("no repPdDate")?.to_string();
    let mut bad_numbers = 0usize;
    let mut num = |s: &str, t: &str| match tag(s, t).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite()) {
        Some(v) => v,
        None => {
            bad_numbers += 1;
            0.0
        }
    };
    let mut holdings = Vec::new();
    for part in xml.split("<invstOrSec>").skip(1) {
        let item = part.split("</invstOrSec>").next().unwrap_or(part);
        holdings.push(Holding {
            name: unescape(tag(item, "name").unwrap_or("")),
            cusip: tag(item, "cusip").unwrap_or("").to_string(),
            isin: attr(item, "isin", "value").unwrap_or("").to_string(),
            val_usd: num(item, "valUSD"),
            pct_val: num(item, "pctVal"),
            balance: num(item, "balance"),
            units: tag(item, "units").unwrap_or("").to_string(),
            currency: attr(item, "currencyConditional", "curCd").or_else(|| tag(item, "curCd")).unwrap_or("").to_string(),
            exchange_rt: opt_num(attr(item, "currencyConditional", "exchangeRt")),
            ticker: attr(item, "ticker", "value").unwrap_or("").to_string(),
            asset_cat: tag(item, "assetCat").unwrap_or("").to_string(),
            country: tag(item, "invCountry").unwrap_or("").to_string(),
            payoff: tag(item, "payoffProfile").unwrap_or("").to_string(),
        });
    }
    // The fund level (Part B of the form). A value that is absent is `None`: an old or trimmed report has no fundInfo.
    let info = tag(xml, "fundInfo").unwrap_or("");
    let mut monthly = Vec::new();
    for part in info.split("<monthlyTotReturn ").skip(1) {
        let el = format!(" {}", part.split('>').next().unwrap_or(part));
        monthly.push(MonthlyReturn {
            class_id: el_attr(&el, "classId").unwrap_or("").to_string(),
            rtn: [1, 2, 3].map(|i| opt_num(el_attr(&el, &format!("rtn{i}")))),
        });
    }
    let flows = [1, 2, 3].map(|i| {
        let el = info.find(&format!("<mon{i}Flow ")).map(|a| &info[a..]).and_then(|t| t.split('>').next())?;
        Some(Flow {
            sales: opt_num(el_attr(el, "sales")),
            redemption: opt_num(el_attr(el, "redemption")),
            reinvestment: opt_num(el_attr(el, "reinvestment")),
        })
    });
    Ok(Report {
        series_id,
        report_date,
        form: tag(xml, "submissionType").unwrap_or("").to_string(),
        rep_pd_end: tag(gen, "repPdEnd").unwrap_or("").to_string(),
        tot_assets: opt_num(tag(info, "totAssets")),
        tot_liabs: opt_num(tag(info, "totLiabs")),
        net_assets: opt_num(tag(info, "netAssets")),
        monthly,
        flows,
        holdings,
        bad_numbers,
    })
}

fn get(client: &mut Client, url: &str, count: &mut usize) -> Result<String, String> {
    *count += 1;
    let b = client.get(url).map_err(|e| e.to_string())?;
    String::from_utf8(b).map_err(|e| format!("{url}: not UTF-8: {e}"))
}

/// What the command line asks for. Nothing is sent before this is checked.
#[derive(Debug, PartialEq)]
pub enum Cmd {
    Help,
    Holdings { ticker: String, cache: Option<PathBuf>, top: usize, from: Option<String>, to: Option<String> },
}

pub fn parse_args(args: &[String]) -> Result<Cmd, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => return Ok(Cmd::Help),
        Some("holdings") => {}
        Some(o) => return Err(format!("fa fund: unknown command {o}\n{USAGE}")),
    }
    let t = args.get(1).ok_or("fa fund holdings TICKER")?;
    if t.starts_with('-') {
        return if matches!(t.as_str(), "-h" | "--help") { Ok(Cmd::Help) } else { Err(format!("unknown option {t}\n{USAGE}")) };
    }
    let ticker = t.to_uppercase();
    if !valid_ticker(&ticker) {
        return Err(format!("{t}: not a ticker (letters, digits, . and - only)"));
    }
    let (mut cache, mut top, mut from, mut to) = (None, 25usize, None, None);
    let mut it = args[2..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => return Ok(Cmd::Help),
            "--cache" => cache = Some(PathBuf::from(it.next().ok_or("--cache needs a folder")?)),
            "--top" => top = it.next().and_then(|v| v.parse().ok()).ok_or("--top needs a number")?,
            "--from" => from = Some(date_arg(it.next(), "--from")?),
            "--to" => to = Some(date_arg(it.next(), "--to")?),
            o => return Err(format!("unknown option {o}")),
        }
    }
    if to.is_some() && from.is_none() {
        return Err("--to needs --from".to_string());
    }
    if let (Some(f), Some(t)) = (&from, &to) {
        if f > t {
            return Err(format!("--from {f} is after --to {t}"));
        }
    }
    Ok(Cmd::Holdings { ticker, cache, top, from, to })
}

fn date_arg(v: Option<&String>, name: &str) -> Result<String, String> {
    let v = v.ok_or_else(|| format!("{name} needs a date YYYY-MM-DD"))?;
    super::parse_date(v).ok_or_else(|| format!("{name}: {v} is not a date YYYY-MM-DD"))?;
    Ok(v.clone())
}

/// A report with the data of its filing.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub accession: String,
    /// The filing date from the record file of the cache; empty if there is no record.
    pub filed: String,
    pub report: Report,
}

/// The sort key of an accession number inside one filer agent: (year, sequence). The filer agent part is not ordered.
fn accession_tail(a: &str) -> (String, String) {
    let p: Vec<&str> = a.split('-').collect();
    (p.get(1).unwrap_or(&"").to_string(), p.get(2).unwrap_or(&"").to_string())
}

/// One report for each report date. Rule: an `NPORT-P` is used. An `NPORT-P/A` (amendment) is used only for a report date
/// that has no `NPORT-P`. If a report date has two `NPORT-P`, the later filing is used. The list is sorted by report date.
/// The notes say each case where the rule did something.
pub fn select_reports(all: Vec<Loaded>) -> (Vec<Loaded>, Vec<String>) {
    let mut dates: Vec<String> = all.iter().map(|l| l.report.report_date.clone()).collect();
    dates.sort();
    dates.dedup();
    let (mut out, mut notes) = (Vec::new(), Vec::new());
    for d in dates {
        let mut same: Vec<&Loaded> = all.iter().filter(|l| l.report.report_date == d).collect();
        let is_orig = |l: &&Loaded| l.report.form == "NPORT-P";
        let amendments = same.iter().filter(|l| !is_orig(l)).count();
        let has_orig = same.iter().any(is_orig);
        if has_orig {
            same.retain(is_orig);
        }
        same.sort_by_key(|l| (l.filed.clone(), accession_tail(&l.accession)));
        let Some(pick) = same.last() else { continue };
        if has_orig && amendments > 0 {
            notes.push(format!("{d}: {amendments} amendment(s) ignored, the NPORT-P {} is used", pick.accession));
        }
        if !has_orig {
            notes.push(format!("{d}: no NPORT-P in the cache, the amendment {} ({}) is used", pick.accession, pick.report.form));
        }
        if has_orig && same.len() > 1 {
            notes.push(format!("{d}: {} NPORT-P filings, the latest {} is used", same.len(), pick.accession));
        }
        out.push((*pick).clone());
    }
    (out, notes)
}

/// The reports in `<cache>/<TICKER>/nport/*.xml` (no request). A file that does not parse, or has another series, is skipped.
/// `series_id` is checked only if given.
pub fn read_cached_reports(cache: &Path, ticker: &str, series_id: Option<&str>) -> Vec<Loaded> {
    let dir = cache.join(ticker).join("nport");
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        let Some(acc) = p.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".xml")).map(str::to_string) else { continue };
        if !valid_accession(&acc) {
            continue;
        }
        let Ok(rep) = std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| parse_report(&t)) else { continue };
        if series_id.is_some_and(|s| s != rep.series_id) {
            continue;
        }
        let filed = std::fs::read_to_string(dir.join(format!("{acc}.meta.json")))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["filed"].as_str().map(str::to_string))
            .unwrap_or_default();
        out.push(Loaded { accession: acc, filed, report: rep });
    }
    out.sort_by(|a, b| (&a.report.report_date, &a.accession).cmp(&(&b.report.report_date, &b.accession)));
    out
}

fn read_report(client: &mut Client, path: &Path, series: &Series, f: &Filing, requests: &mut usize) -> Result<(Report, bool), String> {
    if let Ok(t) = std::fs::read_to_string(path) {
        if let Ok(r) = parse_report(&t) {
            return Ok((r, true));
        }
    }
    let url = format!("https://www.sec.gov/Archives/edgar/data/{}/{}/primary_doc.xml", series.cik, f.accession.replace('-', ""));
    let t = get(client, &url, requests)?;
    let r = parse_report(&t).map_err(|e| format!("{}: {e}", f.accession))?;
    if r.series_id == series.series_id {
        write_atomic(path, &t)?;
        let meta = serde_json::json!({"source": "SEC N-PORT primary_doc.xml", "url": url, "requested": today_iso(), "form": f.form, "filed": f.filed, "accession": f.accession});
        write_atomic(&path.with_extension("meta.json"), &serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?)?;
    }
    Ok((r, false))
}

/// `fa fund holdings TICKER --from D [--to D]`: every report with a report date in the range, one line for each.
#[allow(clippy::too_many_arguments)]
fn run_history(client: &mut Client, cache: &Path, ticker: &str, series: &Series, from: &str, to: Option<&str>, top: usize, mut requests: usize) -> Result<i32, String> {
    let list_url = format!(
        "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={}&type=NPORT-P&dateb=&owner=include&count={HISTORY_LIST}&output=atom",
        series.series_id
    );
    let atom = get(client, &list_url, &mut requests)?;
    let listed = parse_filings(&atom);
    // A report is filed after its report date: a filing older than `from` cannot hold a report date from `from`.
    let filings: Vec<Filing> = listed.iter().filter(|f| matches!(f.form.as_str(), "NPORT-P" | "NPORT-P/A") && valid_accession(&f.accession) && f.filed.as_str() >= from).cloned().collect();
    if listed.len() >= HISTORY_LIST && listed.last().is_some_and(|f| f.filed.as_str() >= from) {
        eprintln!("warning: the filing list has {HISTORY_LIST} entries and the oldest is not older than --from: older reports can exist");
    }
    let (mut loaded, mut from_cache) = (Vec::new(), 0usize);
    for f in &filings {
        let path = cache.join(ticker).join("nport").join(format!("{}.xml", f.accession));
        let (report, cached) = read_report(client, &path, series, f, &mut requests)?;
        if report.series_id != series.series_id {
            continue;
        }
        from_cache += usize::from(cached);
        loaded.push(Loaded { accession: f.accession.clone(), filed: f.filed.clone(), report });
    }
    let in_range = |d: &str| d >= from && to.is_none_or(|t| d <= t);
    loaded.retain(|l| in_range(&l.report.report_date));
    let (picked, notes) = select_reports(loaded);
    println!("{ticker}  series {}  report dates {from} to {}  {} reports  ({} filings listed, {from_cache} read from the cache)", series.series_id, to.unwrap_or("now"), picked.len(), filings.len());
    println!("source: SEC N-PORT ({}), {requests} requests. Rule: NPORT-P; an amendment only where the NPORT-P of the date is missing.", today_iso());
    for n in &notes {
        println!("note: {n}");
    }
    println!("{:<11} {:<21} {:<10} {:>4} {:>15} {:>15} {:>14} {:>8} {:>8} {:>8}  flows sales/redemption (USD m)", "report date", "accession", "filed", "n", "netAssets", "sum valUSD", "totLiabs", "rtn1 % new", "rtn2 %", "rtn3 % old");
    for l in &picked {
        let r = &l.report;
        let m = r.monthly.iter().find(|m| m.class_id == series.class_id).or(r.monthly.first());
        let pc = |i: usize| m.and_then(|m| m.rtn[i]).map_or("n/a".to_string(), |v| format!("{v:.3}"));
        let fl: Vec<String> = r.flows.iter().map(|f| f.as_ref().map_or("n/a".to_string(), |f| format!("{}/{}", mm(f.sales), mm(f.redemption)))).collect();
        let n = |v: Option<f64>| v.map_or("n/a".to_string(), |v| format!("{v:.0}"));
        println!(
            "{:<11} {:<21} {:<10} {:>4} {:>15} {:>15.0} {:>14} {:>8} {:>8} {:>8}  {}",
            r.report_date, l.accession, l.filed, r.holdings.len(), n(r.net_assets), r.holdings.iter().map(|h| h.val_usd).sum::<f64>(), n(r.tot_liabs), pc(0), pc(1), pc(2), fl.join(" ")
        );
    }
    let bad: usize = picked.iter().map(|l| l.report.bad_numbers).sum();
    if bad > 0 {
        println!("warning: {bad} values (pctVal, valUSD, balance) are missing or not numbers; they count as 0");
    }
    if let Some(last) = picked.last() {
        let mut rep = last.report.clone();
        rep.holdings.sort_by(|a, b| b.pct_val.total_cmp(&a.pct_val));
        println!("\nnewest report {}: {} holdings, top {}", rep.report_date, rep.holdings.len(), top.min(rep.holdings.len()));
        println!("{:<40} {:>9} {:>15} {:>4} {:>10}  {:<14} {:<4}", "name", "pctVal", "valUSD", "cur", "exchRt", "isin/cusip", "cat");
        for h in rep.holdings.iter().take(top) {
            let id = if h.isin.is_empty() { &h.cusip } else { &h.isin };
            println!(
                "{:<40} {:>9.4} {:>15.0} {:>4} {:>10} {:<14} {:<4}",
                h.name.chars().take(40).collect::<String>(), h.pct_val, h.val_usd, h.currency, h.exchange_rt.map_or(String::new(), |v| format!("{v:.4}")), id, h.asset_cat
            );
        }
    }
    eprintln!("fa: fund holdings: {requests} requests");
    Ok(0)
}

fn mm(v: Option<f64>) -> String {
    v.map_or("n/a".to_string(), |v| format!("{:.1}", v / 1e6))
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    if args.first().map(String::as_str) == Some("attribution") {
        return super::fund_attribution::run(repo, &args[1..]);
    }
    let (ticker, cache, top, from, to) = match parse_args(args)? {
        Cmd::Help => {
            println!("{USAGE}");
            return Ok(0);
        }
        Cmd::Holdings { ticker, cache, top, from, to } => (ticker, cache.unwrap_or_else(|| repo.join("data").join("raw")), top, from, to),
    };
    let mut client = Client::sec(PAUSE)?;
    let mut requests = 0usize;

    let map_path = cache.join("_sec").join("company_tickers_mf.json");
    let map_text = match std::fs::read_to_string(&map_path) {
        Ok(t) if series_of(&t, &ticker).is_ok() => t,
        _ => {
            let t = get(&mut client, TICKERS_URL, &mut requests)?;
            series_of(&t, &ticker)?;
            write_atomic(&map_path, &t)?;
            t
        }
    };
    let series = series_of(&map_text, &ticker)?;
    if let Some(from) = from {
        return run_history(&mut client, &cache, &ticker, &series, &from, to.as_deref(), top, requests);
    }

    let list_url = format!(
        "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={}&type=NPORT-P&dateb=&owner=include&count=10&output=atom",
        series.series_id
    );
    let atom = get(&mut client, &list_url, &mut requests)?;
    let trust = tag(&atom, "conformed-name").map(unescape).unwrap_or_default();
    // The filter is a prefix match: keep the form NPORT-P only, with a valid accession number.
    let filings: Vec<Filing> = parse_filings(&atom).into_iter().filter(|f| f.form == "NPORT-P" && valid_accession(&f.accession)).collect();

    // The newest report date among the filings read (the order of the Atom list is not trusted).
    let mut best: Option<(Report, Filing, bool)> = None;
    for f in filings.iter().take(MAX_FILINGS) {
        let path = cache.join(&ticker).join("nport").join(format!("{}.xml", f.accession));
        let (rep, cached) = read_report(&mut client, &path, &series, f, &mut requests)?;
        if rep.series_id != series.series_id {
            continue;
        }
        let newer = best.as_ref().is_none_or(|(b, bf, _)| (&rep.report_date, &f.filed) > (&b.report_date, &bf.filed));
        if newer {
            best = Some((rep, f.clone(), cached));
        }
    }
    let Some((mut rep, f, cached)) = best else {
        return Err(format!("{ticker}: no NPORT-P of series {} found in the newest {MAX_FILINGS} filings", series.series_id));
    };
    rep.holdings.sort_by(|a, b| b.pct_val.total_cmp(&a.pct_val));
    println!("{ticker}  trust {trust} (CIK {})  series {}  report date {}  accession {}  filed {}", series.cik, rep.series_id, rep.report_date, f.accession, f.filed);
    println!(
        "source: SEC N-PORT ({}), {}, {requests} {}",
        today_iso(),
        if cached { "XML from the cache" } else { "XML fetched" },
        if requests == 1 { "request" } else { "requests" }
    );
    if rep.bad_numbers > 0 {
        println!("warning: {} values (pctVal, valUSD, balance) are missing or not numbers; they show as 0", rep.bad_numbers);
    }
    println!("{:<44} {:>10} {:>16}  cusip / isin", "name", "pctVal", "valUSD");
    let mut shown = 0.0;
    for h in rep.holdings.iter().take(top) {
        shown += h.pct_val;
        let id = if h.cusip.is_empty() || h.cusip == "N/A" { &h.isin } else { &h.cusip };
        println!("{:<44} {:>10.4} {:>16.0}  {id}", h.name.chars().take(44).collect::<String>(), h.pct_val, h.val_usd);
    }
    println!("shown {} of {} holdings, sum of pctVal shown {shown:.4}", rep.holdings.len().min(top), rep.holdings.len());
    eprintln!("fa: fund holdings: {requests} requests");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = r#"{"fields":["cik","seriesId","classId","symbol"],"data":[[1,"S1","C1","AAA"],[1432353,"S000029442","C000090393","URA"]]}"#;
    const XML: &str = r#"<edgarSubmission><formData><genInfo><regName>X Funds</regName><seriesId>S000029442</seriesId></genInfo>
<genInfo2><repPdDate>2026-08-31</repPdDate></genInfo2><invstOrSecs>
<invstOrSec><name>Cameco &amp; Co</name><cusip>13321L108</cusip><balance>100.00</balance><units>NS</units><valUSD>2000.50</valUSD><pctVal>20.5</pctVal></invstOrSec>
<invstOrSec><name>Kazatomprom</name><cusip>N/A</cusip><identifiers><isin value="US48581R2058"/></identifiers><balance>5</balance><units>NS</units><valUSD>500</valUSD><pctVal>5.25</pctVal></invstOrSec>
<invstOrSec><name>Nexgen</name><cusip>65340P106</cusip><balance>7</balance><units>NS</units><valUSD>900</valUSD><pctVal>9</pctVal></invstOrSec>
</invstOrSecs></formData></edgarSubmission>"#;

    #[test]
    fn ticker_to_series() {
        let s = series_of(MAP, "ura").unwrap();
        assert_eq!((s.cik, s.series_id.as_str(), s.class_id.as_str()), (1432353, "S000029442", "C000090393"));
        assert!(series_of(MAP, "ZZZ").is_err());
    }

    #[test]
    fn parse_xml() {
        let r = parse_report(XML).unwrap();
        assert_eq!((r.series_id.as_str(), r.report_date.as_str(), r.holdings.len()), ("S000029442", "2026-08-31", 3));
        assert_eq!(r.holdings[0].name, "Cameco & Co");
        assert_eq!(r.holdings[0].pct_val, 20.5);
        assert_eq!(r.holdings[0].val_usd, 2000.5);
        assert_eq!(r.holdings[1].isin, "US48581R2058");
        assert_eq!(r.holdings.iter().map(|h| h.pct_val).sum::<f64>(), 34.75);
    }

    #[test]
    fn parse_atom() {
        let a = "<feed><entry><content><accession-number>0001-26-000002</accession-number><filing-date>2026-09-29</filing-date></content></entry><entry><content><accession-number>0001-26-000001</accession-number><filing-date>2026-06-29</filing-date></content></entry></feed>";
        let f = parse_filings(a);
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].accession.as_str(), f[0].filed.as_str()), ("0001-26-000002", "2026-09-29"));
    }

    #[test]
    fn atom_with_amendment_first_is_filtered_by_form() {
        let a = "<feed><entry><content><accession-number>0001234567-26-000009</accession-number><filing-date>2026-10-05</filing-date><filing-type>NPORT-P/A</filing-type></content></entry><entry><content><accession-number>0001234567-26-000002</accession-number><filing-date>2026-09-29</filing-date><filing-type>NPORT-P</filing-type></content></entry></feed>";
        let f: Vec<Filing> = parse_filings(a).into_iter().filter(|f| f.form == "NPORT-P").collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].accession, "0001234567-26-000002");
    }

    #[test]
    fn bad_numbers_are_counted_and_entities_decoded() {
        let x = XML.replace("<pctVal>9</pctVal>", "<pctVal>x</pctVal>").replace("Nexgen", "It&#39;s &#x27;A&#x27;");
        let r = parse_report(&x).unwrap();
        assert_eq!(r.bad_numbers, 1);
        assert_eq!(r.holdings[2].name, "It's 'A'");
    }

    #[test]
    fn accession_and_ticker_checks() {
        assert!(valid_accession("0002048251-26-008623"));
        assert!(!valid_accession("../../etc-26-008623"));
        assert!(!valid_accession("0002048251-26-00862"));
        assert!(valid_ticker("BRK-B") && valid_ticker("URA"));
        assert!(!valid_ticker("--HELP") && !valid_ticker("../x") && !valid_ticker(""));
    }

    #[test]
    fn bad_arguments_fail_before_any_request() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // `run` checks the arguments first; a refused call never reaches `Client::sec` (no cache folder is made).
        let dir = std::env::temp_dir().join("fa_fund_no_request_test");
        let _ = std::fs::remove_dir_all(&dir);
        let d = dir.to_str().unwrap();
        for a in [v(&["holdings", "--HELP", "--cache", d]), v(&["holdings", "../x", "--cache", d]), v(&["holdings", "URA", "--bogus"])] {
            assert!(run(Path::new("."), &a).is_err(), "{a:?}");
        }
        assert!(!dir.exists());
        assert_eq!(parse_args(&v(&["holdings", "--help"])).unwrap(), Cmd::Help);
        assert_eq!(run(Path::new("."), &v(&["holdings", "-h"])).unwrap(), 0);
    }

    /// Trimmed from the real report 0002048251-26-008623 of the URA series (report date 2026-07-31): the same element names
    /// and values for the fund level and for three lines (a CAD line, a KRW line, a repo line with a ticker identifier).
    const REAL: &str = r#"<edgarSubmission xmlns="http://www.sec.gov/edgar/nport"><headerData><submissionType>NPORT-P</submissionType></headerData>
<formData><genInfo><regName>GLOBAL X FUNDS</regName><seriesId>S000029442</seriesId><repPdEnd>2026-10-31</repPdEnd><repPdDate>2026-07-31</repPdDate></genInfo>
<fundInfo><totAssets>5909582028.82</totAssets><totLiabs>470243426.86</totLiabs><netAssets>5439338601.96</netAssets>
<returnInfo><monthlyTotReturns><monthlyTotReturn classId="C000090393" rtn1="-9.61140500" rtn2="-14.40661300" rtn3="-8.53285300"/></monthlyTotReturns></returnInfo>
<mon1Flow redemption="13775040.10000000" reinvestment="0.00000000" sales="351107429.43000000"/>
<mon2Flow redemption="66958271.11000000" reinvestment="0.00000000" sales="15391360.42000000"/>
<mon3Flow redemption="142071173.92000000" reinvestment="0.00000000" sales="62420348.88000000"/></fundInfo>
<invstOrSecs>
<invstOrSec><name>CAMECO CORPORATION</name><cusip>13321L108</cusip><identifiers><isin value="CA13321L1085"/></identifiers><balance>13990654.00000000</balance><units>NS</units><currencyConditional curCd="CAD" exchangeRt="1.40285000"/><valUSD>1206536209.00000000</valUSD><pctVal>22.18167128932</pctVal><payoffProfile>Long</payoffProfile><assetCat>EC</assetCat><invCountry>CA</invCountry></invstOrSec>
<invstOrSec><name>HYUNDAI ENGINEERING &amp; CONSTRUCTION CO.,LTD</name><cusip>N/A</cusip><identifiers><isin value="KR7000720003"/></identifiers><balance>1591715.00000000</balance><units>NS</units><currencyConditional curCd="KRW" exchangeRt="1425.70000000"/><valUSD>105615654.80000000</valUSD><pctVal>1.941700315585</pctVal><payoffProfile>Long</payoffProfile><assetCat>EC</assetCat><invCountry>KR</invCountry></invstOrSec>
<invstOrSec><name>DAIWA CAPITAL MARKETS AMERICA INC</name><cusip>N/A</cusip><identifiers><ticker value="DAIW"/></identifiers><balance>28329647</balance><units>PA</units><curCd>USD</curCd><valUSD>28329647</valUSD><pctVal>0.520828892501</pctVal><payoffProfile>Long</payoffProfile><assetCat>RA</assetCat><invCountry>US</invCountry></invstOrSec>
</invstOrSecs></formData></edgarSubmission>"#;

    #[test]
    fn parse_fund_level_fields_of_a_real_report() {
        let r = parse_report(REAL).unwrap();
        assert_eq!((r.form.as_str(), r.rep_pd_end.as_str(), r.report_date.as_str()), ("NPORT-P", "2026-10-31", "2026-07-31"));
        assert_eq!((r.tot_assets, r.tot_liabs, r.net_assets), (Some(5909582028.82), Some(470243426.86), Some(5439338601.96)));
        assert_eq!(r.monthly.len(), 1);
        assert_eq!(r.monthly[0].class_id, "C000090393");
        assert_eq!(r.monthly[0].rtn, [Some(-9.611405), Some(-14.406613), Some(-8.532853)]);
        let f = r.flows[2].as_ref().unwrap();
        assert_eq!((f.sales, f.redemption, f.reinvestment), (Some(62420348.88), Some(142071173.92), Some(0.0)));
        assert_eq!(r.flows[0].as_ref().unwrap().sales, Some(351107429.43));
        // The assets minus the liabilities are the net assets (to the cent).
        assert!((r.tot_assets.unwrap() - r.tot_liabs.unwrap() - r.net_assets.unwrap()).abs() < 0.01);
    }

    #[test]
    fn parse_currency_exchange_rate_and_identifiers() {
        let r = parse_report(REAL).unwrap();
        let (c, h, d) = (&r.holdings[0], &r.holdings[1], &r.holdings[2]);
        assert_eq!((c.currency.as_str(), c.exchange_rt, c.isin.as_str(), c.cusip.as_str()), ("CAD", Some(1.40285), "CA13321L1085", "13321L108"));
        assert_eq!((h.currency.as_str(), h.exchange_rt, h.cusip.as_str(), h.country.as_str()), ("KRW", Some(1425.7), "N/A", "KR"));
        assert_eq!((d.currency.as_str(), d.exchange_rt, d.ticker.as_str(), d.asset_cat.as_str(), d.units.as_str()), ("USD", None, "DAIW", "RA", "PA"));
        assert_eq!(c.payoff, "Long");
        assert_eq!(h.name, "HYUNDAI ENGINEERING & CONSTRUCTION CO.,LTD");
        // The price in USD of the Cameco line: valUSD / balance.
        assert!((c.val_usd / c.balance - 86.2383).abs() < 1e-3);
        assert_eq!(r.bad_numbers, 0);
    }

    #[test]
    fn a_report_without_fund_level_data_has_none() {
        let r = parse_report(XML).unwrap();
        assert_eq!((r.tot_assets, r.net_assets, r.monthly.len(), r.form.as_str()), (None, None, 0, ""));
        assert!(r.flows.iter().all(Option::is_none));
    }

    fn loaded(acc: &str, form: &str, date: &str, filed: &str) -> Loaded {
        let mut report = parse_report(REAL).unwrap();
        report.form = form.to_string();
        report.report_date = date.to_string();
        Loaded { accession: acc.to_string(), filed: filed.to_string(), report }
    }

    #[test]
    fn amendment_is_used_only_if_the_original_is_missing() {
        let all = vec![
            loaded("0001-26-000001", "NPORT-P", "2026-01-31", "2026-03-30"),
            loaded("0001-26-000002", "NPORT-P/A", "2026-01-31", "2026-04-02"),
            loaded("0001-26-000003", "NPORT-P/A", "2026-04-30", "2026-06-29"),
            loaded("0001-26-000004", "NPORT-P", "2026-07-31", "2026-09-29"),
            loaded("0001-26-000005", "NPORT-P", "2026-07-31", "2026-10-02"),
        ];
        let (sel, notes) = select_reports(all);
        let got: Vec<(&str, &str)> = sel.iter().map(|l| (l.report.report_date.as_str(), l.accession.as_str())).collect();
        assert_eq!(got, [("2026-01-31", "0001-26-000001"), ("2026-04-30", "0001-26-000003"), ("2026-07-31", "0001-26-000005")]);
        assert_eq!(notes.len(), 3);
        assert!(notes[0].contains("1 amendment(s) ignored") && notes[1].contains("no NPORT-P") && notes[2].contains("2 NPORT-P filings"));
    }

    #[test]
    fn history_arguments() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        match parse_args(&v(&["holdings", "ura", "--from", "2023-09-30", "--to", "2026-08-01"])).unwrap() {
            Cmd::Holdings { from, to, .. } => assert_eq!((from.as_deref(), to.as_deref()), (Some("2023-09-30"), Some("2026-08-01"))),
            c => panic!("{c:?}"),
        }
        assert!(parse_args(&v(&["holdings", "URA", "--to", "2026-08-01"])).is_err());
        assert!(parse_args(&v(&["holdings", "URA", "--from", "2026-08-01", "--to", "2026-01-01"])).is_err());
        assert!(parse_args(&v(&["holdings", "URA", "--from", "yesterday"])).is_err());
    }
}
