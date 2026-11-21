//! Market data of one company for the valuation part (the Rust form of `market_data.py`,
//! `options.py`, `estimates.py` and `fmp.py`).
//!
//! Sources (one request for each source in one run, 2 seconds apart, the honest client name of
//! `futures`):
//!
//! | Data | Source | Module |
//! |---|---|---|
//! | Daily prices | Tiingo (key), Alpaca (key), Yahoo chart endpoint, Nasdaq site API | `price_source`, `price_cache` (`fa prices`); `prices` is the older Yahoo loader |
//! | Option chain | Cboe delayed quotes; Alpaca indicative feed (key) | `options`, `options_cli`, `alpaca` |
//! | Past option surface | Alpaca option bars (key) | `option_history`, `option_iv` |
//! | Intraday bars | Alpaca (key) | `intraday`, `alpaca` |
//! | FRED series | FRED API (key), keyless CSV | `fred_api` (used by `rates`) |
//! | Estimates, earnings surprises | Financial Modeling Prep (key in `FMP_API_KEY`) | `fmp` |
//! | Stock splits (corporate actions) | Alpaca (key) | `splits`, `splits_cli` (`fa splits get`) |
//! | Forward consensus snapshot, price targets, surprises, earnings dates | Nasdaq site API (no key) | `nasdaq_est`, `estimates_cache`, `earnings_dates` (`fa estimates`, `fa earnings`) |
//!
//! The cache files keep the formats of the Python code, so each reader of them still works:
//! `data/raw/<T>/prices.json`, `options/<date>.json`, `estimates/<date>.json`, `yf_earnings.json`,
//! `estimates.json`, `surprises.json`.

pub mod alpaca;
pub mod consensus;
pub mod earnings_dates;
pub mod estimates_cache;
pub mod estimates_cli;
pub mod fmp;
pub mod fred_api;
pub mod intraday;
pub mod keys;
pub mod macro_cli;
pub mod option_arb;
pub mod option_history;
pub mod option_iv;
pub mod nasdaq_est;
pub mod options;
pub mod options_archive;
pub mod options_cli;
pub mod price_cache;
pub mod price_check;
pub mod price_cli;
pub mod price_source;
pub mod prices;
pub mod splits;
pub mod splits_rules;
pub mod splits_cli;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;
use std::marker::PhantomData;
use std::path::Path;

/// The name that outside sites see (the same as `fa futures`).
pub use crate::futures::USER_AGENT;

/// Seconds between two requests to outside sites.
pub const REQUEST_PAUSE_SECONDS: f64 = 2.0;

/// One GET request to an outside site. The request goes through the queue of its host (`hostnet`): two requests
/// to one host are at least 2 seconds apart and never run at the same time; two hosts do not wait for each other.
pub fn download(url: &str) -> Result<Vec<u8>, String> {
    crate::hostnet::get(url, &[], REQUEST_PAUSE_SECONDS)
}

/// The same as `download`, with more request headers (the Tiingo key is a header). The error text has no header value.
pub fn download_with_headers(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    crate::hostnet::get(url, headers, REQUEST_PAUSE_SECONDS)
}

/// Python `round(x, 6)`: the exact decimal value of the number, rounded to 6 places.
pub fn round6(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{x:.6}").parse().unwrap_or(x)
}

/// `float(v)` with NaN and infinity as `None`, rounded to 6 places (`_clean_float` of the Python code).
pub fn clean(x: Option<f64>) -> Option<f64> {
    x.filter(|v| v.is_finite()).map(round6)
}

/// JSON text that Python wrote may have the bare words `NaN`, `Infinity` and `-Infinity`. They
/// are not JSON. This changes each of them (outside a string) to `null`.
pub fn lenient_json(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let (mut i, mut in_string, mut escaped) = (0, false, false);
    let mut start = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == b'"' {
            in_string = true;
            i += 1;
            continue;
        }
        let rest = &text[i..];
        let word = ["NaN", "-Infinity", "Infinity"].into_iter().find(|w| rest.starts_with(w));
        if let Some(word) = word {
            out.push_str(&text[start..i]);
            out.push_str("null");
            i += word.len();
            start = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&text[start..]);
    out
}

pub fn parse_lenient(text: &str) -> Result<Value, String> {
    serde_json::from_str(&lenient_json(text)).map_err(|e| e.to_string())
}

/// A JSON object that keeps the order of its keys (Python dicts do; `serde_json::Map` does not).
#[derive(Debug, Clone, PartialEq)]
pub struct Ordered<T>(pub Vec<(String, T)>);

impl<T> Default for Ordered<T> {
    fn default() -> Self {
        Ordered(Vec::new())
    }
}

impl<T> Ordered<T> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<T: Serialize> Serialize for Ordered<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Ordered<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = Ordered<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some((k, v)) = map.next_entry::<String, T>()? {
                    out.push((k, v));
                }
                Ok(Ordered(out))
            }
        }
        deserializer.deserialize_map(V(PhantomData))
    }
}

/// `YYYY-MM-DD` (and nothing else) to a day number; `None` for any other text.
pub fn parse_date(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    if !b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return None;
    }
    let day = mapcheck::dates::parse(text)?;
    // `dates::parse` can accept a day that does not exist (for example 2026-02-30): check the round trip.
    (mapcheck::dates::format(day) == text).then_some(day)
}

pub fn today_iso() -> String {
    mapcheck::dates::format(mapcheck::dates::today())
}

/// Write a file in one step: the old file stays if the write fails.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The path text of a ticker folder: `<cache>/<TICKER>`.
pub fn ticker_dir(cache: &Path, ticker: &str) -> std::path::PathBuf {
    cache.join(ticker.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round6_is_python_round() {
        // Values from Python `round(x, 6)`. These are not ties in binary, so the exact value decides.
        for (x, y) in [(2.0000005, 2.000001), (0.1234565, 0.123456), (5e-07, 0.0), (1.0000005, 1.000001), (123456.7890125, 123456.789012), (0.285714285714, 0.285714)] {
            assert_eq!(round6(x), y, "round6({x:?})");
        }
        assert!(round6(f64::NAN).is_nan());
        assert_eq!(clean(Some(f64::INFINITY)), None);
        assert_eq!(clean(Some(1.23456789)), Some(1.234568));
    }

    #[test]
    fn lenient_json_changes_bare_nan_only_outside_strings() {
        let text = r#"[{"a": NaN, "b": "NaN", "c": -Infinity, "d": 1.5, "e": "x\"NaN"}]"#;
        let v = parse_lenient(text).unwrap();
        assert!(v[0]["a"].is_null() && v[0]["c"].is_null());
        assert_eq!(v[0]["b"], "NaN");
        assert_eq!(v[0]["d"], 1.5);
        assert_eq!(v[0]["e"], "x\"NaN");
    }

    #[test]
    fn ordered_keeps_the_key_order() {
        let o: Ordered<f64> = serde_json::from_str(r#"{"0q": 1, "+1q": 2, "0y": 3}"#).unwrap();
        assert_eq!(o.0.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), ["0q", "+1q", "0y"]);
        assert_eq!(serde_json::to_string(&o).unwrap(), r#"{"0q":1.0,"+1q":2.0,"0y":3.0}"#);
    }

    #[test]
    fn dates_are_strict() {
        assert!(parse_date("2026-11-19").is_some());
        assert!(parse_date("20261119").is_none());
        assert!(parse_date("2026-02-30").is_none());
        assert!(parse_date("2026-1-9").is_none());
    }
}
