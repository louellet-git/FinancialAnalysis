//! SEC EDGAR: the ticker list, the company facts, and the files of a filing.
//!
//! Cache layout (the same folders as before):
//!   <cache>/_sec/company_tickers.json            the SEC ticker list
//!   <cache>/<TICKER>/company_facts.json[.gz]     the company facts
//!   <cache>/<TICKER>/filings/<accession>/        MetaLinks.json and the instance document,
//!                                                or the primary document of a prospectus

use crate::facts::CompanyFacts;
use crate::http::{Client, HttpError};
use crate::run::{facts_path, read_facts};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const TICKERS_URL: &str = "https://www.sec.gov/files/company_tickers.json";
const EARLIER_REGISTRANTS: &str = include_str!("../universe/earlier_registrants.txt");

/// The earlier SEC registrants of a ticker (universe/earlier_registrants.txt).
pub fn earlier_registrants(ticker: &str) -> Vec<String> {
    for line in EARLIER_REGISTRANTS.lines() {
        let mut words = line.split('#').next().unwrap_or("").split_whitespace();
        if words.next().map(|t| t.eq_ignore_ascii_case(ticker)).unwrap_or(false) {
            return words.map(str::to_string).collect();
        }
    }
    Vec::new()
}

/// Add the facts of an earlier registrant to the facts of the current registrant.
/// The mapper selects the latest filing for each period, thus a fact that the two
/// registrants have is not counted two times.
pub fn merge_facts(current: &mut Value, earlier: &Value) {
    let Some(namespaces) = earlier.get("facts").and_then(Value::as_object) else { return };
    if !current.get("facts").map(Value::is_object).unwrap_or(false) {
        current["facts"] = Value::Object(Default::default());
    }
    for (namespace, concepts) in namespaces {
        let Some(concepts) = concepts.as_object() else { continue };
        for (tag, concept) in concepts {
            let target = &mut current["facts"][namespace][tag];
            if target.is_null() {
                *target = concept.clone();
                continue;
            }
            let Some(units) = concept.get("units").and_then(Value::as_object) else { continue };
            for (unit, entries) in units {
                let Some(entries) = entries.as_array() else { continue };
                match target["units"][unit].as_array_mut() {
                    Some(list) => list.extend(entries.iter().cloned()),
                    None => target["units"][unit] = Value::Array(entries.clone()),
                }
            }
        }
    }
}

fn facts_url(cik: &str) -> String {
    format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{:0>10}.json", cik)
}

fn submissions_url(cik: &str) -> String {
    format!("https://data.sec.gov/submissions/CIK{:0>10}.json", cik)
}

fn archive_url(cik: &str, accession: &str, name: &str) -> String {
    format!("https://www.sec.gov/Archives/edgar/data/{cik}/{}/{name}", accession.replace('-', ""))
}

/// [(ticker, cik)] in the sequence of the SEC ticker list (largest companies first).
pub fn parse_ticker_list(json: &[u8]) -> Result<Vec<(String, String)>, String> {
    let value: Value = serde_json::from_slice(json).map_err(|e| format!("ticker list: {e}"))?;
    let map = value.as_object().ok_or("ticker list: not a JSON object")?;
    let mut rows: Vec<(u64, String, String)> = Vec::new();
    for (key, entry) in map {
        let index: u64 = key.parse().map_err(|_| format!("ticker list: key '{key}' is not a number"))?;
        let ticker = entry.get("ticker").and_then(Value::as_str).unwrap_or("").to_uppercase();
        let cik = match entry.get("cik_str") {
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        if !ticker.is_empty() && !cik.is_empty() {
            rows.push((index, ticker, cik));
        }
    }
    rows.sort_by_key(|r| r.0);
    Ok(rows.into_iter().map(|(_, t, c)| (t, c)).collect())
}

/// The SEC ticker list, from the cache or from the SEC.
pub fn ticker_list(client: &mut Client, cache: &Path, refresh: bool) -> Result<Vec<(String, String)>, String> {
    let path = cache.join("_sec").join("company_tickers.json");
    if !refresh && path.is_file() {
        return parse_ticker_list(&fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?);
    }
    let body = client.get(TICKERS_URL).map_err(|e| e.to_string())?;
    let list = parse_ticker_list(&body)?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&path, &body).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(list)
}

/// The first `n` companies of the list (a second share class of a company is not counted).
pub fn top_tickers(list: &[(String, String)], n: usize) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (ticker, cik) in list {
        if seen.insert(cik.clone()) {
            out.push(ticker.clone());
            if out.len() >= n {
                break;
            }
        }
    }
    out
}

/// Get the company facts of one company and save them compressed. Returns the file size.
pub fn fetch_facts(client: &mut Client, cache: &Path, ticker: &str, cik: &str) -> Result<u64, String> {
    let mut body = client.get(&facts_url(cik)).map_err(|e| e.to_string())?;
    let earlier = earlier_registrants(ticker);
    if earlier.is_empty() {
        // Make sure that the body is JSON before it goes into the cache.
        let _: serde::de::IgnoredAny = serde_json::from_slice(&body).map_err(|e| format!("{ticker}: not JSON: {e}"))?;
    } else {
        let mut facts: Value = serde_json::from_slice(&body).map_err(|e| format!("{ticker}: not JSON: {e}"))?;
        for earlier_cik in earlier.iter().filter(|c| c.trim_start_matches('0') != cik.trim_start_matches('0')) {
            let earlier_body = client.get(&facts_url(earlier_cik)).map_err(|e| e.to_string())?;
            let earlier_facts: Value = serde_json::from_slice(&earlier_body).map_err(|e| format!("{ticker}: CIK {earlier_cik}: not JSON: {e}"))?;
            merge_facts(&mut facts, &earlier_facts);
        }
        body = serde_json::to_vec(&facts).map_err(|e| e.to_string())?;
    }
    let folder = cache.join(ticker);
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join("company_facts.json.gz");
    let file = fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&body).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())?;
    Ok(fs::metadata(&path).map(|m| m.len()).unwrap_or(0))
}

/// The company facts of a ticker: from the cache, or from the SEC if they are not there.
pub fn load_or_fetch_facts(cache: &Path, ticker: &str, delay: f64) -> Result<CompanyFacts, String> {
    if let Some(path) = facts_path(cache, ticker) {
        return read_facts(&path);
    }
    let mut client = Client::sec(delay)?;
    let list = ticker_list(&mut client, cache, false)?;
    let cik = list
        .iter()
        .find(|(t, _)| t == ticker)
        .map(|(_, c)| c.clone())
        .ok_or_else(|| format!("{ticker} is not in the SEC ticker list"))?;
    fetch_facts(&mut client, cache, ticker, &cik)?;
    let path = facts_path(cache, ticker).ok_or_else(|| format!("{ticker}: the file was not saved"))?;
    read_facts(&path)
}

/// (cik, accession number) of the latest 10-K in the company facts.
pub fn latest_annual_filing(path: &Path) -> Result<(String, String), String> {
    // The typed reader does not keep the accession number: read the file as JSON here.
    let bytes = crate::run::read_bytes(path)?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let cik = match value.get("cik") {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let mut best: Option<(&str, &str)> = None; // (filed, accession)
    if let Some(namespaces) = value.get("facts").and_then(Value::as_object) {
        for concepts in namespaces.values().filter_map(Value::as_object) {
            for concept in concepts.values() {
                let Some(units) = concept.get("units").and_then(Value::as_object) else { continue };
                for entries in units.values().filter_map(Value::as_array) {
                    for e in entries {
                        let form = e.get("form").and_then(Value::as_str).unwrap_or("");
                        if form != "10-K" && form != "10-K/A" {
                            continue;
                        }
                        let (Some(accn), filed) = (e.get("accn").and_then(Value::as_str), e.get("filed").and_then(Value::as_str).unwrap_or("")) else {
                            continue;
                        };
                        if best.map(|(f, _)| filed > f).unwrap_or(true) {
                            best = Some((filed, accn));
                        }
                    }
                }
            }
        }
    }
    let (_, accession) = best.ok_or("the company facts have no 10-K")?;
    Ok((cik, accession.to_string()))
}

pub fn filing_folder(cache: &Path, ticker: &str, accession: &str) -> PathBuf {
    cache.join(ticker).join("filings").join(accession)
}

/// The instance document in a filing folder (`*_htm.xml`).
pub fn instance_path(folder: &Path) -> Option<PathBuf> {
    let mut names: Vec<PathBuf> = fs::read_dir(folder)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).map(|n| n.ends_with("_htm.xml")).unwrap_or(false))
        .collect();
    names.sort();
    names.into_iter().next()
}

/// Get MetaLinks.json and the instance document of the latest 10-K into the cache.
/// Returns the folder. Network: www.sec.gov, 3 requests for a filing that is not in the cache.
pub fn fetch_filing(cache: &Path, ticker: &str, delay: f64, refresh: bool) -> Result<PathBuf, String> {
    let facts = facts_path(cache, ticker).ok_or_else(|| format!("{ticker}: no company facts in the cache (run: mapcheck fetch {ticker})"))?;
    let (cik, accession) = latest_annual_filing(&facts)?;
    let folder = filing_folder(cache, ticker, &accession);
    if !refresh && folder.join("MetaLinks.json").is_file() && instance_path(&folder).is_some() {
        return Ok(folder);
    }
    let mut client = Client::sec(delay)?;
    let mut get = |name: &str| -> Result<Vec<u8>, String> {
        client.get(&archive_url(&cik, &accession, name)).map_err(|e| match e {
            HttpError::Status(403, url) => format!("HTTP 403 for {url} (www.sec.gov refuses a User-Agent with no contact)"),
            other => other.to_string(),
        })
    };
    let index: Value = serde_json::from_slice(&get("index.json")?).map_err(|e| format!("{ticker} {accession}: index.json: {e}"))?;
    let names: Vec<&str> = index
        .pointer("/directory/item")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(|i| i.get("name").and_then(Value::as_str)).collect())
        .unwrap_or_default();
    let instance = names.iter().find(|n| n.ends_with("_htm.xml")).copied();
    let (true, Some(instance)) = (names.contains(&"MetaLinks.json"), instance) else {
        return Err(format!("{ticker} {accession}: the filing has no MetaLinks.json or no instance document"));
    };
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    fs::write(folder.join("MetaLinks.json"), get("MetaLinks.json")?).map_err(|e| e.to_string())?;
    fs::write(folder.join(instance), get(instance)?).map_err(|e| e.to_string())?;
    Ok(folder)
}

/// One filing of the SEC submissions data.
#[derive(Debug, Clone, PartialEq)]
pub struct FilingRef {
    pub form: String,
    pub accession: String,
    pub document: String,
    pub filed: String,
}

/// The prospectus forms, in the sequence of the selection: the final prospectus, then
/// the changed registration statement, then the first registration statement.
pub const PROSPECTUS_FORMS: [&str; 3] = ["424B4", "S-1/A", "S-1"];

/// The latest prospectus in the SEC submissions data (`filings.recent`): the latest
/// 424B4; if none, the latest S-1/A; if none, the latest S-1.
pub fn latest_prospectus(json: &[u8]) -> Result<FilingRef, String> {
    let value: Value = serde_json::from_slice(json).map_err(|e| format!("submissions: {e}"))?;
    let recent = value.pointer("/filings/recent").ok_or("submissions: no filings.recent")?;
    let column = |name: &str| recent.get(name).and_then(Value::as_array).ok_or_else(|| format!("submissions: no filings.recent.{name}"));
    let (forms, accessions, documents, dates) = (column("form")?, column("accessionNumber")?, column("primaryDocument")?, column("filingDate")?);
    for wanted in PROSPECTUS_FORMS {
        let mut best: Option<FilingRef> = None;
        for (i, form) in forms.iter().enumerate() {
            let text = |list: &[Value]| list.get(i).and_then(Value::as_str).unwrap_or("").to_string();
            if form.as_str() != Some(wanted) || text(documents).is_empty() || text(accessions).is_empty() {
                continue;
            }
            // The same date two times: the first row stays (the SEC puts the latest filing first).
            if best.as_ref().map(|b| text(dates) > b.filed).unwrap_or(true) {
                best = Some(FilingRef { form: wanted.to_string(), accession: text(accessions), document: text(documents), filed: text(dates) });
            }
        }
        if let Some(filing) = best {
            return Ok(filing);
        }
    }
    Err("the registrant has no 424B4, S-1/A or S-1 in the recent filings".to_string())
}

/// Get the primary document of the latest prospectus into the cache. Returns the filing
/// and the path of the file. The company facts must be in the cache (they give the CIK).
/// Network: data.sec.gov, 1 request; www.sec.gov, 1 request for a file that is not in the cache.
pub fn fetch_prospectus(cache: &Path, ticker: &str, delay: f64, refresh: bool) -> Result<(FilingRef, PathBuf), String> {
    let facts = facts_path(cache, ticker).ok_or_else(|| format!("{ticker}: no company facts in the cache (run: mapcheck fetch {ticker})"))?;
    let cik = match read_facts(&facts)?.cik {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.trim_start_matches('0').to_string(),
        _ => return Err(format!("{ticker}: the company facts have no CIK")),
    };
    let mut client = Client::sec(delay)?;
    let filing = latest_prospectus(&client.get(&submissions_url(&cik)).map_err(|e| e.to_string())?).map_err(|e| format!("{ticker}: {e}"))?;
    // The names come from the SEC data and become a path: accept a plain name only.
    let plain = |name: &str| !name.starts_with('.') && name.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    if !plain(&filing.document) || !plain(&filing.accession) {
        return Err(format!("{ticker}: '{}' in {} is not a plain file name", filing.document, filing.accession));
    }
    let folder = filing_folder(cache, ticker, &filing.accession);
    let path = folder.join(&filing.document);
    if refresh || !path.is_file() {
        let body = client.get(&archive_url(&cik, &filing.accession, &filing.document)).map_err(|e| e.to_string())?;
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok((filing, path))
}

/// The form 8-K of one company that has item 2.02 (results of operations and financial condition).
/// A company files such a form on the day that it reports its earnings.
#[derive(Debug, Clone, PartialEq)]
pub struct EarningsFiling {
    pub filed: String,
    /// The `reportDate` of the filing (the date of the event); empty when the SEC has none.
    pub report_date: String,
    pub accession: String,
    pub items: String,
}

/// The item that an earnings release has.
pub const EARNINGS_ITEM: &str = "2.02";

/// The forms 8-K with item 2.02 in the SEC submissions data (`filings.recent`), newest first.
/// `items` is a text with the items separated by commas (`2.02,9.01`).
pub fn earnings_filings(json: &[u8]) -> Result<Vec<EarningsFiling>, String> {
    let value: Value = serde_json::from_slice(json).map_err(|e| format!("submissions: {e}"))?;
    let recent = value.pointer("/filings/recent").ok_or("submissions: no filings.recent")?;
    let column = |name: &str| recent.get(name).and_then(Value::as_array).ok_or_else(|| format!("submissions: no filings.recent.{name}"));
    let (forms, items, filed, accessions) = (column("form")?, column("items")?, column("filingDate")?, column("accessionNumber")?);
    // `reportDate` is optional: a company with no such column gets empty report dates.
    let report = recent.get("reportDate").and_then(Value::as_array);
    let mut out = Vec::new();
    for (i, form) in forms.iter().enumerate() {
        if form.as_str() != Some("8-K") {
            continue;
        }
        let text = |list: &[Value]| list.get(i).and_then(Value::as_str).unwrap_or("").to_string();
        let item_text = text(items);
        if !item_text.split(',').any(|item| item.trim() == EARNINGS_ITEM) {
            continue;
        }
        out.push(EarningsFiling {
            filed: text(filed),
            report_date: report.map(|r| text(r)).unwrap_or_default(),
            accession: text(accessions),
            items: item_text,
        });
    }
    out.sort_by(|a, b| b.filed.cmp(&a.filed));
    Ok(out)
}

/// The file of the earnings filings of a ticker: `<cache>/<TICKER>/sec_8k_earnings.json`.
pub fn earnings_filings_path(cache: &Path, ticker: &str) -> PathBuf {
    cache.join(ticker).join("sec_8k_earnings.json")
}

/// Get the SEC submissions data of a ticker and save the forms 8-K with item 2.02 in the cache
/// (the form that `fa earnings dates` reads). The CIK comes from the company facts in the cache, else
/// from the SEC ticker list. Network: data.sec.gov, 1 request (and the ticker list, when it is not in the cache).
pub fn fetch_earnings_filings(cache: &Path, ticker: &str, delay: f64) -> Result<(Vec<EarningsFiling>, PathBuf), String> {
    let mut client = Client::sec(delay)?;
    let cik = match facts_path(cache, ticker) {
        Some(path) => match read_facts(&path)?.cik {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.trim_start_matches('0').to_string(),
            _ => return Err(format!("{ticker}: the company facts have no CIK")),
        },
        None => ticker_list(&mut client, cache, false)?
            .into_iter()
            .find(|(t, _)| t == ticker)
            .map(|(_, c)| c)
            .ok_or_else(|| format!("{ticker} is not in the SEC ticker list"))?,
    };
    let body = client.get(&submissions_url(&cik)).map_err(|e| format!("{ticker}: {e}"))?;
    let filings = earnings_filings(&body).map_err(|e| format!("{ticker}: {e}"))?;
    let rows: Vec<Value> = filings
        .iter()
        .map(|f| serde_json::json!({"filed": f.filed, "report_date": f.report_date, "accession": f.accession, "items": f.items}))
        .collect();
    let text = serde_json::to_string_pretty(&serde_json::json!({"ticker": ticker, "cik": cik, "source": "sec submissions, form 8-K with item 2.02", "filings": rows})).map_err(|e| e.to_string())?;
    let path = earnings_filings_path(cache, ticker);
    fs::create_dir_all(path.parent().unwrap_or(cache)).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((filings, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_form_8k_with_item_202_is_an_earnings_filing() {
        let json = serde_json::json!({"filings": {"recent": {
            "form":            ["8-K", "10-Q", "8-K", "8-K/A", "8-K", "8-K"],
            "items":           ["2.02,9.01", "", "5.02", "2.02", "12.02", "7.01,2.02"],
            "filingDate":      ["2026-10-28", "2026-10-29", "2026-09-01", "2026-07-30", "2026-07-29", "2026-04-29"],
            "reportDate":      ["2026-10-28", "2026-09-30", "2026-09-01", "", "", "2026-04-29"],
            "accessionNumber": ["a1", "a2", "a3", "a4", "a5", "a6"]}}});
        let got = earnings_filings(&serde_json::to_vec(&json).unwrap()).unwrap();
        let dates: Vec<&str> = got.iter().map(|f| f.filed.as_str()).collect();
        // Form 8-K/A, item 12.02 (an old number of the item) and other items are not in the list.
        assert_eq!(dates, ["2026-10-28", "2026-04-29"]);
        assert_eq!(got[1].items, "7.01,2.02");
        assert_eq!(got[0].report_date, "2026-10-28");
        // A reply with no `items` column is an error; a reply with no `reportDate` column still works.
        let no_items = serde_json::json!({"filings": {"recent": {"form": ["8-K"], "filingDate": ["2026-01-01"], "accessionNumber": ["x"]}}});
        assert!(earnings_filings(&serde_json::to_vec(&no_items).unwrap()).is_err());
        let no_report = serde_json::json!({"filings": {"recent": {"form": ["8-K"], "items": ["2.02"], "filingDate": ["2026-01-01"], "accessionNumber": ["x"]}}});
        assert_eq!(earnings_filings(&serde_json::to_vec(&no_report).unwrap()).unwrap()[0].report_date, "");
    }

    #[test]
    fn ticker_list_keeps_the_sec_sequence_and_top_skips_second_share_classes() {
        let json = br#"{"1":{"cik_str":2,"ticker":"bbb","title":"B"},"0":{"cik_str":1,"ticker":"AAA","title":"A"},
                       "2":{"cik_str":2,"ticker":"BBB-B","title":"B class B"},"10":{"cik_str":3,"ticker":"CCC","title":"C"}}"#;
        let list = parse_ticker_list(json).unwrap();
        assert_eq!(list[0], ("AAA".to_string(), "1".to_string()));
        assert_eq!(list.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(), vec!["AAA", "BBB", "BBB-B", "CCC"]);
        assert_eq!(top_tickers(&list, 3), vec!["AAA", "BBB", "CCC"]);
    }

    #[test]
    fn urls_have_the_sec_format() {
        assert_eq!(facts_url("51143"), "https://data.sec.gov/api/xbrl/companyfacts/CIK0000051143.json");
        assert_eq!(
            archive_url("51143", "0000051143-26-000010", "MetaLinks.json"),
            "https://www.sec.gov/Archives/edgar/data/51143/000005114326000010/MetaLinks.json"
        );
    }

    #[test]
    fn facts_of_an_earlier_registrant_are_added() {
        assert_eq!(earlier_registrants("xom"), vec!["34088"]);
        assert!(earlier_registrants("IBM").is_empty());
        let mut current: Value = serde_json::from_str(r#"{"cik":2,"facts":{"us-gaap":{"Assets":{"label":"Assets","units":{"USD":[{"end":"2026-03-31","val":3}]}}}}}"#).unwrap();
        let earlier: Value = serde_json::from_str(r#"{"cik":1,"facts":{"us-gaap":{"Assets":{"units":{"USD":[{"end":"2025-12-31","val":2}]}},"Revenues":{"units":{"USD":[{"end":"2025-12-31","val":9}]}}},"dei":{"X":{"units":{"shares":[{"end":"2025-12-31","val":1}]}}}}}"#).unwrap();
        merge_facts(&mut current, &earlier);
        assert_eq!(current["cik"], 2);
        assert_eq!(current["facts"]["us-gaap"]["Assets"]["units"]["USD"].as_array().unwrap().len(), 2);
        assert_eq!(current["facts"]["us-gaap"]["Revenues"]["units"]["USD"][0]["val"], 9);
        assert_eq!(current["facts"]["dei"]["X"]["units"]["shares"][0]["val"], 1);
    }

    #[test]
    fn latest_annual_filing_is_the_latest_10k() {
        let dir = std::env::temp_dir().join(format!("mapcheck-sec-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("company_facts.json");
        fs::write(&path, br#"{"cik":7,"facts":{"us-gaap":{"Assets":{"units":{"USD":[
            {"end":"2023-12-31","val":1,"form":"10-K","filed":"2024-02-01","accn":"0000000007-24-000001"},
            {"end":"2024-12-31","val":1,"form":"10-K","filed":"2025-02-01","accn":"0000000007-25-000001"},
            {"end":"2025-03-31","val":1,"form":"10-Q","filed":"2025-05-01","accn":"0000000007-25-000009"}]}}}}}"#).unwrap();
        assert_eq!(latest_annual_filing(&path).unwrap(), ("7".to_string(), "0000000007-25-000001".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    /// Submissions data from rows of [form, accession number, primary document, filing date].
    fn submissions(rows: &[[&str; 4]]) -> Vec<u8> {
        let column = |i: usize| rows.iter().map(|r| r[i]).collect::<Vec<_>>();
        let recent = serde_json::json!({"form": column(0), "accessionNumber": column(1), "primaryDocument": column(2), "filingDate": column(3)});
        serde_json::to_vec(&serde_json::json!({"cik": "7", "filings": {"recent": recent}})).unwrap()
    }

    #[test]
    fn latest_prospectus_is_the_424b4_then_the_s1a_then_the_s1() {
        assert_eq!(submissions_url("2130357"), "https://data.sec.gov/submissions/CIK0002130357.json");
        let s1 = ["S-1", "0000000007-26-000001", "s1.htm", "2026-10-26"];
        let s1a_first = ["S-1/A", "0000000007-26-000002", "s1a1.htm", "2026-11-02"];
        let s1a_last = ["S-1/A", "0000000007-26-000003", "s1a2.htm", "2026-11-17"];
        let fwp = ["FWP", "0000000007-26-000004", "fwp.htm", "2026-11-19"];
        let final_prospectus = ["424B4", "0000000007-26-000005", "final.htm", "2026-11-19"];
        // The final prospectus wins, also when a later row has a later form.
        let all = latest_prospectus(&submissions(&[fwp, final_prospectus, s1a_last, s1a_first, s1])).unwrap();
        assert_eq!(all, FilingRef { form: "424B4".into(), accession: "0000000007-26-000005".into(), document: "final.htm".into(), filed: "2026-11-19".into() });
        // No 424B4: the latest S-1/A, in any row sequence.
        assert_eq!(latest_prospectus(&submissions(&[s1, s1a_first, s1a_last, fwp])).unwrap().document, "s1a2.htm");
        // No S-1/A: the S-1.
        assert_eq!(latest_prospectus(&submissions(&[fwp, s1])).unwrap().accession, "0000000007-26-000001");
        // A row with no primary document is not used.
        let no_document = ["424B4", "0000000007-26-000006", "", "2026-11-20"];
        assert_eq!(latest_prospectus(&submissions(&[no_document, s1])).unwrap().form, "S-1");
        assert!(latest_prospectus(&submissions(&[fwp])).is_err());
        assert!(latest_prospectus(br#"{"filings":{}}"#).is_err());
    }
}
