//! SEC EDGAR: the ticker list, the company facts, and the files of a filing.
//!
//! Cache layout (the same folders as before):
//!   <cache>/_sec/company_tickers.json            the SEC ticker list
//!   <cache>/<TICKER>/company_facts.json[.gz]     the company facts
//!   <cache>/<TICKER>/filings/<accession>/        MetaLinks.json and the instance document

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

fn facts_url(cik: &str) -> String {
    format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{:0>10}.json", cik)
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
    let body = client.get(&facts_url(cik)).map_err(|e| e.to_string())?;
    // Make sure that the body is JSON before it goes into the cache.
    let _: serde::de::IgnoredAny = serde_json::from_slice(&body).map_err(|e| format!("{ticker}: not JSON: {e}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
