//! Financial Modeling Prep: analyst estimates and earnings surprises (the Rust form of `fmp.py`).
//!
//! The key is in the environment variable `FMP_API_KEY`, or in the line `FMP_API_KEY=...` of the
//! file `.env` in the repository folder (the file is not committed). The program never prints the
//! key: the error texts of a request have the key taken out.
//!
//! Without a key, and without a cache file, the loaders give `FmpError::NoKey`. There is no other free
//! source for estimates in this program (open decision of the owner: `PLANNING/board`, ticket
//! `report-rust`).

use super::{download, parse_date, round6, ticker_dir, write_atomic};
use serde_json::Value;
use std::path::Path;

pub const BASE: &str = "https://financialmodelingprep.com/api/v3";
pub const KEY_ENV: &str = "FMP_API_KEY";

#[derive(Debug, Clone, PartialEq)]
pub struct EarningsEstimate {
    pub period_end: String,
    /// `annual` or `quarterly`.
    pub period_type: String,
    pub estimated_eps: Option<f64>,
    pub estimated_revenue: Option<f64>,
    pub number_of_analysts: Option<f64>,
}

/// An earnings surprise. A value can be NaN (a date in the future has no actual result).
#[derive(Debug, Clone, PartialEq)]
pub struct EarningsSurprise {
    pub date: String,
    pub actual_eps: f64,
    pub estimated_eps: f64,
    pub surprise: f64,
    pub surprise_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FmpError {
    NoKey,
    Failed(String),
}

impl std::fmt::Display for FmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FmpError::NoKey => write!(f, "{KEY_ENV} is not set in the environment or in .env"),
            FmpError::Failed(m) => write!(f, "{m}"),
        }
    }
}

/// The key of the environment variable, else of the `.env` file in `repo`.
pub fn api_key(repo: &Path) -> Option<String> {
    if let Ok(v) = std::env::var(KEY_ENV) {
        if !v.trim().is_empty() {
            return Some(v.trim().to_string());
        }
    }
    let text = std::fs::read_to_string(repo.join(".env")).ok()?;
    key_from_env_file(&text)
}

/// The value of `FMP_API_KEY` in the text of a `.env` file (`KEY=value`, `export KEY="value"`).
pub fn key_from_env_file(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        if let Some(value) = line.strip_prefix(KEY_ENV).and_then(|r| r.trim_start().strip_prefix('=')) {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// A request function: URL in, body out. The tests give their own.
pub type Fetch<'a> = &'a dyn Fn(&str) -> Result<Vec<u8>, String>;

pub struct Fmp<'a> {
    key: String,
    fetch: Fetch<'a>,
}

/// The text with the key taken out.
pub fn scrub(text: &str, key: &str) -> String {
    if key.is_empty() {
        text.to_string()
    } else {
        text.replace(key, "***")
    }
}

/// A request function for the real server. Its error texts have no key.
pub fn http_fetch(key: &str) -> impl Fn(&str) -> Result<Vec<u8>, String> {
    let key = key.to_string();
    move |url: &str| download(url).map_err(|e| scrub(&e, &key))
}

impl<'a> Fmp<'a> {
    pub fn new(key: &str, fetch: Fetch<'a>) -> Fmp<'a> {
        Fmp { key: key.to_string(), fetch }
    }

    /// The JSON list of one endpoint. A reply that is not a list is an error (FMP answers an error
    /// message as an object with HTTP 200).
    pub fn get(&self, endpoint: &str) -> Result<Vec<Value>, FmpError> {
        let url = format!("{BASE}/{endpoint}?apikey={}", self.key);
        let bytes = (self.fetch)(&url).map_err(|e| FmpError::Failed(format!("FMP request failed: {}", scrub(&e, &self.key))))?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|e| FmpError::Failed(format!("FMP reply for {endpoint} is not JSON: {e}")))?;
        match value {
            Value::Array(items) => Ok(items),
            other => {
                let text = scrub(&other.to_string(), &self.key);
                let text: String = text.chars().take(200).collect();
                Err(FmpError::Failed(format!("FMP reply for {endpoint} is not a list: {text}")))
            }
        }
    }
}

fn number(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
}

/// The estimate records. A record with no valid date is skipped.
pub fn parse_estimates(raw: &[Value]) -> Vec<EarningsEstimate> {
    let mut out = Vec::new();
    for r in raw {
        let Some(date) = r.get("date").and_then(Value::as_str).filter(|d| parse_date(d).is_some()) else {
            eprintln!("fa: skipping a malformed estimate record");
            continue;
        };
        let quarterly = r.get("period").and_then(Value::as_str) == Some("quarter");
        out.push(EarningsEstimate {
            period_end: date.to_string(),
            period_type: if quarterly { "quarterly" } else { "annual" }.to_string(),
            estimated_eps: number(r.get("estimatedEpsAvg")),
            estimated_revenue: number(r.get("estimatedRevenueAvg")),
            number_of_analysts: number(r.get("numberAnalystEstimatedEps")),
        });
    }
    out
}

fn float_of(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// The surprise records. A record with a missing or bad date or number is skipped.
pub fn parse_surprises(raw: &[Value]) -> Vec<EarningsSurprise> {
    let mut out = Vec::new();
    for r in raw {
        let date = r.get("date").and_then(Value::as_str).filter(|d| parse_date(d).is_some());
        let (Some(date), Some(actual), Some(estimated)) = (date, float_of(r.get("actualEarningResult")), float_of(r.get("estimatedEarning"))) else {
            eprintln!("fa: skipping a malformed surprise record");
            continue;
        };
        let surprise = actual - estimated;
        let pct = if estimated != 0.0 { surprise / estimated.abs() } else { 0.0 };
        out.push(EarningsSurprise {
            date: date.to_string(),
            actual_eps: actual,
            estimated_eps: estimated,
            surprise: round6(surprise),
            surprise_pct: round6(pct),
        });
    }
    out
}

fn cached_list(path: &Path) -> Option<Vec<Value>> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(items)) => Some(items),
        _ => {
            eprintln!("fa: corrupt cache file {}; asking FMP again", path.display());
            None
        }
    }
}

fn load_list(cache: &Path, ticker: &str, file: &str, endpoint: &str, refresh: bool, fmp: Option<&Fmp>) -> Result<Vec<Value>, FmpError> {
    let path = ticker_dir(cache, ticker).join(file);
    if !refresh {
        if let Some(items) = cached_list(&path) {
            return Ok(items);
        }
    }
    let fmp = fmp.ok_or(FmpError::NoKey)?;
    let items = fmp.get(&format!("{endpoint}/{}", ticker.to_uppercase()))?;
    let text = serde_json::to_string_pretty(&items).map_err(|e| FmpError::Failed(e.to_string()))?;
    write_atomic(&path, &text).map_err(FmpError::Failed)?;
    Ok(items)
}

/// `data/raw/<T>/estimates.json`, else FMP `analyst-estimates/<T>`.
pub fn load_estimates(cache: &Path, ticker: &str, refresh: bool, fmp: Option<&Fmp>) -> Result<Vec<EarningsEstimate>, FmpError> {
    Ok(parse_estimates(&load_list(cache, ticker, "estimates.json", "analyst-estimates", refresh, fmp)?))
}

/// `data/raw/<T>/surprises.json`, else FMP `earnings-surprises/<T>`.
pub fn load_surprises(cache: &Path, ticker: &str, refresh: bool, fmp: Option<&Fmp>) -> Result<Vec<EarningsSurprise>, FmpError> {
    Ok(parse_surprises(&load_list(cache, ticker, "surprises.json", "earnings-surprises", refresh, fmp)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn fixture() -> Value {
        let path = format!("{}/tests/fixtures/market/fmp.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn list(v: &Value) -> Vec<Value> {
        v.as_array().unwrap().clone()
    }

    #[test]
    fn estimates_match_the_python_parser() {
        let f = fixture();
        let got = parse_estimates(&list(&f["estimates_raw"]));
        let want = list(&f["estimates"]);
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(&want) {
            assert_eq!(g.period_end, w["period_end"].as_str().unwrap());
            assert_eq!(g.period_type, w["period_type"].as_str().unwrap());
            assert_eq!(g.estimated_eps, w["estimated_eps"].as_f64());
            assert_eq!(g.estimated_revenue, w["estimated_revenue"].as_f64());
            assert_eq!(g.number_of_analysts, w["number_of_analysts"].as_f64());
        }
    }

    #[test]
    fn surprises_match_the_python_parser() {
        let f = fixture();
        let got = parse_surprises(&list(&f["surprises_raw"]));
        let want = list(&f["surprises"]);
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(&want) {
            assert_eq!(g.date, w["date"].as_str().unwrap());
            assert_eq!(g.actual_eps, w["actual_eps"].as_f64().unwrap());
            assert_eq!(g.estimated_eps, w["estimated_eps"].as_f64().unwrap());
            assert_eq!(g.surprise, w["surprise"].as_f64().unwrap());
            assert_eq!(g.surprise_pct, w["surprise_pct"].as_f64().unwrap());
        }
    }

    #[test]
    fn env_file_gives_the_key() {
        assert_eq!(key_from_env_file("A=1\nFMP_API_KEY=abc123\n"), Some("abc123".into()));
        assert_eq!(key_from_env_file("export FMP_API_KEY = \"abc\""), Some("abc".into()));
        assert_eq!(key_from_env_file("FMP_API_KEY=\nOTHER=1"), None);
        assert_eq!(key_from_env_file("# FMP_API_KEY=abc"), None);
    }

    #[test]
    fn a_request_has_the_key_and_the_error_text_has_not() {
        let seen = RefCell::new(String::new());
        let fetch = |u: &str| -> Result<Vec<u8>, String> {
            *seen.borrow_mut() = u.to_string();
            Err(format!("HTTP 403 for {u}"))
        };
        let fmp = Fmp::new("SECRETKEY", &fetch);
        let e = fmp.get("analyst-estimates/IBM").unwrap_err();
        assert_eq!(*seen.borrow(), "https://financialmodelingprep.com/api/v3/analyst-estimates/IBM?apikey=SECRETKEY");
        let text = e.to_string();
        assert!(text.contains("HTTP 403") && !text.contains("SECRETKEY"), "{text}");
        // The scrub works for the text of the real client too.
        assert_eq!(scrub("a SECRETKEY b", "SECRETKEY"), "a *** b");
    }

    #[test]
    fn an_error_object_with_status_200_is_an_error_and_not_cached() {
        let dir = std::env::temp_dir().join(format!("fa-fmp-err-{}", std::process::id()));
        let fetch = |_: &str| -> Result<Vec<u8>, String> { Ok(br#"{"Error Message": "Limit Reach. SECRETKEY"}"#.to_vec()) };
        let fmp = Fmp::new("SECRETKEY", &fetch);
        let e = load_estimates(&dir, "IBM", false, Some(&fmp)).unwrap_err();
        let text = e.to_string();
        assert!(text.contains("not a list") && text.contains("Limit Reach") && !text.contains("SECRETKEY"), "{text}");
        assert!(!dir.join("IBM/estimates.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_key_and_no_cache_is_no_source_and_a_cache_file_is_enough() {
        let dir = std::env::temp_dir().join(format!("fa-fmp-cache-{}", std::process::id()));
        assert_eq!(load_surprises(&dir, "IBM", false, None).unwrap_err(), FmpError::NoKey);
        let f = fixture();
        let calls = RefCell::new(0);
        let fetch = |u: &str| -> Result<Vec<u8>, String> {
            *calls.borrow_mut() += 1;
            assert!(u.contains("earnings-surprises/IBM?apikey="), "{u}");
            Ok(f["surprises_raw"].to_string().into_bytes())
        };
        let fmp = Fmp::new("K", &fetch);
        let first = load_surprises(&dir, "ibm", false, Some(&fmp)).unwrap();
        assert_eq!((first.len(), *calls.borrow()), (5, 1));
        // The second call reads the cache file: no key, no request.
        assert_eq!(load_surprises(&dir, "IBM", false, None).unwrap(), first);
        assert_eq!(*calls.borrow(), 1);
        // refresh asks again
        load_surprises(&dir, "IBM", true, Some(&fmp)).unwrap();
        assert_eq!(*calls.borrow(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
