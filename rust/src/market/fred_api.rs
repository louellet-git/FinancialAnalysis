//! The FRED API with a key: `https://api.stlouisfed.org/fred/series/observations` (official documentation
//! `https://fred.stlouisfed.org/docs/api/fred/series_observations.html`, read 2026-11-19). It was NOT run against the real
//! server: there was no key.
//!
//! - The key is the URL parameter `api_key` (32 lowercase letters and digits). It is `FRED_API_KEY` in the environment or in
//!   `.env`. The key is in the URL, so each error text and each log line passes through `scrub` (the key value and each
//!   `api_key=` value become `***`).
//! - `file_type=json`: `{"count": N, "offset": 0, "limit": 100000, "observations": [{"date": "2024-01-02", "value": "4.05"}]}`.
//!   `limit` is 100,000 at most (the default); `offset` goes to the next page. A missing value is the text `.` in the CSV of the
//!   site; the page of the API does not say how it writes one, so `.` and an empty text are both skipped.
//! - Limit: 120 requests each minute, then HTTP 429 and a temporary block (`docs/api/fred/errors.html`). The terms of use
//!   do not name a limit of storage; they forbid an application that replaces the FRED site and ask each user to have an own key.
//! - The loaders keep their cache: the text of the reply becomes the CSV text of the keyless address
//!   (`observation_date,<SERIES>` and one `date,value` line for each value), so each cache file and each parser stays as it was.

use super::keys::{scrub_all, scrub_api_key_param};
use serde_json::Value;

pub const KEY_ENV: &str = "FRED_API_KEY";
pub const BASE: &str = "https://api.stlouisfed.org/fred/series/observations";
pub const PAGE_LIMIT: usize = 100_000;

pub fn url(series_id: &str, start: &str, key: &str, offset: usize) -> String {
    format!("{BASE}?series_id={series_id}&api_key={key}&file_type=json&observation_start={start}&limit={PAGE_LIMIT}&offset={offset}")
}

/// The text with the key taken out, whatever it came from (an error, a log line).
pub fn scrub(text: &str, key: &str) -> String {
    scrub_api_key_param(&scrub_all(text, &[key]))
}

/// One page: the count of all observations and the `(date, value text)` of this page.
/// An error reply is `{"error_code": 400, "error_message": "..."}`.
pub fn parse_page(text: &str) -> Result<(usize, Vec<(String, String)>), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("FRED reply is not JSON: {e}"))?;
    if let Some(msg) = v["error_message"].as_str() {
        return Err(format!("FRED error {}: {}", v["error_code"], msg.chars().take(200).collect::<String>()));
    }
    let list = v["observations"].as_array().ok_or("FRED reply has no observations")?;
    let rows = list.iter().filter_map(|o| Some((o["date"].as_str()?.to_string(), o["value"].as_str().map(String::from).or_else(|| o["value"].as_f64().map(|x| x.to_string()))?))).collect();
    Ok((v["count"].as_u64().unwrap_or(list.len() as u64) as usize, rows))
}

/// The CSV text of the keyless address for these observations. A missing value is left out.
pub fn to_csv(series_id: &str, rows: &[(String, String)]) -> String {
    let mut out = format!("observation_date,{series_id}\n");
    for (d, v) in rows {
        let v = v.trim();
        if v.is_empty() || v == "." {
            continue;
        }
        out.push_str(&format!("{d},{v}\n"));
    }
    out
}

/// All pages of one series from `start` as CSV text. `get` is one request (URL in, text out).
pub fn fetch_csv(series_id: &str, start: &str, key: &str, get: &mut dyn FnMut(&str) -> Result<String, String>) -> Result<String, String> {
    let mut rows: Vec<(String, String)> = Vec::new();
    loop {
        let text = get(&url(series_id, start, key, rows.len())).map_err(|e| scrub(&e, key))?;
        let (count, page) = parse_page(&text).map_err(|e| scrub(&e, key))?;
        let n = page.len();
        rows.extend(page);
        if n == 0 || rows.len() >= count {
            break;
        }
    }
    Ok(to_csv(series_id, &rows))
}

/// The line that a loader prints when the key path failed and the keyless path goes on.
pub fn fallback_message(series_id: &str, error: &str, key: &str) -> String {
    format!("fa: FRED API failed for {series_id} ({}); trying the keyless CSV", scrub(error, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rates::{Net, Rates};
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    const KEY: &str = "abcdef0123456789abcdef0123456789";

    /// The documented reply (`series_observations.html`), with the `.` value of a day with no rate.
    const PAGE: &str = r#"{"realtime_start":"2026-11-19","realtime_end":"2026-11-19","observation_start":"2024-01-01","observation_end":"9999-12-31",
        "units":"lin","output_type":1,"file_type":"json","order_by":"observation_date","sort_order":"asc","count":3,"offset":0,"limit":100000,
        "observations":[{"realtime_start":"2026-11-19","realtime_end":"2026-11-19","date":"2024-01-01","value":"."},
        {"realtime_start":"2026-11-19","realtime_end":"2026-11-19","date":"2024-01-02","value":"3.95"},
        {"realtime_start":"2026-11-19","realtime_end":"2026-11-19","date":"2024-01-03","value":"4.05"}]}"#;

    #[test]
    fn page_to_csv_in_the_old_form() {
        let (count, rows) = parse_page(PAGE).unwrap();
        assert_eq!((count, rows.len()), (3, 3));
        let csv = to_csv("DGS10", &rows);
        assert_eq!(csv, "observation_date,DGS10\n2024-01-02,3.95\n2024-01-03,4.05\n");
        // The parser of the keyless CSV reads it.
        let parsed = crate::rates::parse_fred_csv(&csv, true).unwrap();
        assert_eq!(parsed.len(), 2);
        assert!((parsed.values().next().unwrap() - 0.0395).abs() < 1e-12);
        let err = parse_page(r#"{"error_code":400,"error_message":"Bad Request.  The value for variable api_key is not registered."}"#).unwrap_err();
        assert!(err.contains("not registered") && err.contains("400"), "{err}");
        assert!(parse_page("<html>").is_err());
    }

    #[test]
    fn paging_by_offset() {
        let urls = RefCell::new(Vec::<String>::new());
        let mut get = |u: &str| -> Result<String, String> {
            urls.borrow_mut().push(u.to_string());
            Ok(if u.ends_with("offset=0") {
                r#"{"count":3,"observations":[{"date":"2024-01-02","value":"1.0"},{"date":"2024-01-03","value":"2.0"}]}"#.into()
            } else {
                r#"{"count":3,"observations":[{"date":"2024-01-04","value":"3.0"}]}"#.into()
            })
        };
        let csv = fetch_csv("DGS10", "2024-01-01", KEY, &mut get).unwrap();
        assert_eq!(csv.lines().count(), 4);
        assert_eq!(urls.borrow().len(), 2);
        assert!(urls.borrow()[1].ends_with("offset=2") && urls.borrow()[0].contains("observation_start=2024-01-01&limit=100000"));
    }

    #[test]
    fn the_key_is_not_in_any_error_or_log_line() {
        let mut get = |u: &str| -> Result<String, String> { Err(format!("HTTP 400 for {u}")) };
        let e = fetch_csv("DGS10", "2024-01-01", KEY, &mut get).unwrap_err();
        assert!(!e.contains(KEY) && e.contains("api_key=***"), "{e}");
        let mut get2 = |_: &str| -> Result<String, String> { Ok(format!(r#"{{"error_code":400,"error_message":"key {KEY} is bad"}}"#)) };
        let e = fetch_csv("DGS10", "2024-01-01", KEY, &mut get2).unwrap_err();
        assert!(!e.contains(KEY), "{e}");
        let line = fallback_message("DGS10", &format!("HTTP 429 for https://x?api_key={KEY}&file_type=json"), KEY);
        assert!(!line.contains(KEY) && line.contains("DGS10"), "{line}");
    }

    struct Fake {
        log: Rc<RefCell<Vec<String>>>,
        keyed_fails: bool,
    }

    impl Net for Fake {
        fn get(&mut self, url: &str) -> Result<String, String> {
            self.log.borrow_mut().push(url.to_string());
            if url.contains("api.stlouisfed.org") {
                if self.keyed_fails {
                    return Err(format!("HTTP 500 for {url}"));
                }
                return Ok(PAGE.to_string());
            }
            Ok("observation_date,DGS10\n2024-01-02,3.90\n".into())
        }
        fn post_json(&mut self, _u: &str, _b: &str) -> Result<String, String> {
            Err("no".into())
        }
    }

    fn rates(keyed_fails: bool, key: Option<&str>) -> (Rates, Rc<RefCell<Vec<String>>>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("fa_fred_{}_{keyed_fails}_{}", std::process::id(), key.is_some()));
        let _ = std::fs::remove_dir_all(&dir);
        let log = Rc::new(RefCell::new(vec![]));
        let r = Rates::with_net(dir.clone(), Box::new(Fake { log: log.clone(), keyed_fails }), 19_700).with_fred_key(key.map(String::from));
        (r, log, dir)
    }

    #[test]
    fn the_keyed_api_is_the_first_source_and_the_cache_file_keeps_its_form() {
        let (mut r, log, dir) = rates(false, Some(KEY));
        let s: BTreeMap<i64, f64> = r.load_fred("DGS10", true).unwrap();
        assert_eq!(s.len(), 2);
        assert!(log.borrow().iter().all(|u| u.contains("api.stlouisfed.org")), "no keyless request when the key works: {:?}", log.borrow());
        let cached = std::fs::read_to_string(dir.join("local/bonds/fred_DGS10.csv")).unwrap();
        assert!(cached.starts_with("observation_date,DGS10\n2024-01-02,3.95"), "{cached}");
        let one = r.fred_series("DGS10", Some(19_000), true).unwrap();
        assert_eq!(one.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_key_path_falls_back_to_the_keyless_csv_without_the_key() {
        let (mut r, log, dir) = rates(true, Some(KEY));
        let s = r.load_fred("DGS10", true).unwrap();
        assert_eq!(s.len(), 1, "the keyless text has one value");
        assert!(log.borrow().iter().any(|u| u.contains("fredgraph.csv")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn with_no_key_only_the_keyless_path_is_used() {
        let (mut r, log, dir) = rates(false, None);
        r.load_fred("DGS10", true).unwrap();
        assert!(log.borrow().iter().all(|u| u.contains("fredgraph.csv")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
