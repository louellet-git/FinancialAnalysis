//! Forward consensus snapshots and the earnings cache of the Python code (the Rust form of `estimates.py`
//! and the cache part of `market_data.py`).
//!
//! The Python code got these from yfinance, which uses Yahoo endpoints that need a cookie and a crumb.
//! Yahoo refuses those for a program that gives its real name (trap #93 of `docs/HIDDEN-ASSUMPTIONS.md`),
//! and this program does not imitate a browser. So there is **no source** in Rust for:
//!
//! - the forward consensus snapshot (`data/raw/<T>/estimates/<date>.json`), and
//! - the earnings dates with the estimate and the actual result (`data/raw/<T>/yf_earnings.json`).
//!
//! The program reads the files that exist (made earlier by the Python code), and it says that there is
//! no source when a file does not exist. The snapshot has its date in each row of the sheet.
//! `fmp` gives estimates and surprises when the key exists.

use super::fmp::EarningsSurprise;
use super::{lenient_json, parse_date, round6, ticker_dir, write_atomic, Ordered};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub type Periods = Ordered<Ordered<Option<f64>>>;

/// A dated snapshot. The periods are the keys of yfinance (`0q`, `+1q`, `0y`, `+1y`), and the columns
/// are as reported. An empty dictionary means: not available for this ticker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusEstimates {
    pub ticker: String,
    pub snapshot_date: String,
    #[serde(default)]
    pub revenue_estimate: Periods,
    #[serde(default)]
    pub earnings_estimate: Periods,
    #[serde(default)]
    pub analyst_price_targets: Ordered<Option<f64>>,
}

impl ConsensusEstimates {
    /// The number of records (the count that `analyze` printed).
    pub fn record_count(&self) -> usize {
        self.revenue_estimate.len() + self.earnings_estimate.len() + self.analyst_price_targets.len()
    }
}

pub fn estimates_dir(cache: &Path, ticker: &str) -> PathBuf {
    ticker_dir(cache, ticker).join("estimates")
}

pub fn snapshot_path(cache: &Path, ticker: &str, date: &str) -> PathBuf {
    estimates_dir(cache, ticker).join(format!("{date}.json"))
}

pub fn save_snapshot(snapshot: &ConsensusEstimates, cache: &Path) -> Result<PathBuf, String> {
    let path = snapshot_path(cache, &snapshot.ticker, &snapshot.snapshot_date);
    write_atomic(&path, &serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?)?;
    Ok(path)
}

/// The dates of the snapshots of a ticker, oldest first.
pub fn list_snapshots(cache: &Path, ticker: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(estimates_dir(cache, ticker)) else { return Vec::new() };
    let mut dates: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let stem = name.strip_suffix(".json")?;
            parse_date(stem).map(|_| stem.to_string())
        })
        .collect();
    dates.sort();
    dates
}

/// A snapshot by date; `None` is the latest one.
pub fn load_snapshot(cache: &Path, ticker: &str, date: Option<&str>) -> Result<ConsensusEstimates, String> {
    let date = match date {
        Some(d) => d.to_string(),
        None => list_snapshots(cache, ticker)
            .pop()
            .ok_or_else(|| format!("No estimates snapshots for {} under {}", ticker.to_uppercase(), estimates_dir(cache, ticker).display()))?,
    };
    let path = snapshot_path(cache, ticker, &date);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("No estimates snapshot for {} on {date} at {}: {e}", ticker.to_uppercase(), path.display()))?;
    serde_json::from_str(&lenient_json(&text)).map_err(|e| format!("{}: {e}", path.display()))
}

/// The most recent snapshot, or `None` if the ticker has none.
pub fn latest(cache: &Path, ticker: &str) -> Option<ConsensusEstimates> {
    load_snapshot(cache, ticker, None).ok()
}

/// The snapshot for the sheet and a text for the console. Today's snapshot, else the latest older one
/// (its date is in the sheet), else nothing.
pub fn snapshot_for_report(cache: &Path, ticker: &str, today: &str) -> (Option<ConsensusEstimates>, String) {
    if let Ok(s) = load_snapshot(cache, ticker, Some(today)) {
        let n = s.record_count();
        return (Some(s), format!("{n} forward consensus records loaded (snapshot of {today})"));
    }
    match latest(cache, ticker) {
        Some(s) => {
            let (n, date) = (s.record_count(), s.snapshot_date.clone());
            (Some(s), format!("{n} forward consensus records loaded from the snapshot of {date} (older than today; there is no source for a new one)"))
        }
        None => (None, "no source for forward consensus (Yahoo needs a browser identity; the program does not give one) and no snapshot in the cache".to_string()),
    }
}

/// `data/raw/<T>/yf_earnings.json`: records `{date, actual_eps, estimated_eps, surprise, surprise_pct}`.
/// A record with a missing key or a bad date is skipped. A missing number (a date in the future) is NaN.
pub fn cached_surprises(cache: &Path, ticker: &str) -> Option<Vec<EarningsSurprise>> {
    let path = ticker_dir(cache, ticker).join("yf_earnings.json");
    let text = std::fs::read_to_string(path).ok()?;
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&lenient_json(&text)) else {
        eprintln!("fa: corrupt earnings cache for {ticker}");
        return None;
    };
    let num = |r: &Value, k: &str| -> Option<f64> {
        match r.get(k)? {
            Value::Null => Some(f64::NAN),
            v => v.as_f64(),
        }
    };
    let mut out = Vec::new();
    for r in &items {
        let date = r.get("date").and_then(Value::as_str).filter(|d| parse_date(d).is_some());
        let (Some(date), Some(actual), Some(estimated), Some(surprise), Some(pct)) =
            (date, num(r, "actual_eps"), num(r, "estimated_eps"), num(r, "surprise"), num(r, "surprise_pct"))
        else {
            eprintln!("fa: skipping a malformed earnings record");
            continue;
        };
        out.push(EarningsSurprise { date: date.to_string(), actual_eps: actual, estimated_eps: estimated, surprise: round6(surprise), surprise_pct: round6(pct) });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNAPSHOT: &str = include_str!("../../tests/fixtures/market/consensus_IBM.json");
    const EARNINGS: &str = include_str!("../../tests/fixtures/market/yf_earnings_IBM.json");

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fa-consensus-{name}-{}", std::process::id()))
    }

    #[test]
    fn a_python_snapshot_is_read_in_key_order_and_written_the_same_way() {
        let s: ConsensusEstimates = serde_json::from_str(SNAPSHOT).unwrap();
        let periods: Vec<&str> = s.revenue_estimate.0.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(periods, ["0q", "+1q", "0y", "+1y"], "the file order, not the sort order");
        assert_eq!(s.revenue_estimate.0[0].1 .0[0], ("avg".to_string(), Some(20382528670.0)));
        assert_eq!(s.revenue_estimate.0[0].1 .0.last().unwrap(), &("currency".to_string(), None));
        assert!(s.record_count() > 5);
        // The text of the Python writer (json.dumps indent=2) and of this writer is one text for the same data.
        let dir = temp("write");
        let path = save_snapshot(&s, &dir).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), SNAPSHOT.trim_end());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn report_snapshot_rules() {
        let dir = temp("rules");
        let (none, text) = snapshot_for_report(&dir, "IBM", "2026-11-19");
        assert!(none.is_none() && text.contains("no source"), "{text}");
        let mut s: ConsensusEstimates = serde_json::from_str(SNAPSHOT).unwrap();
        s.snapshot_date = "2026-11-10".into();
        save_snapshot(&s, &dir).unwrap();
        let (older, text) = snapshot_for_report(&dir, "IBM", "2026-11-19");
        assert_eq!(older.unwrap().snapshot_date, "2026-11-10");
        assert!(text.contains("snapshot of 2026-11-10"), "{text}");
        s.snapshot_date = "2026-11-19".into();
        save_snapshot(&s, &dir).unwrap();
        let (today, _) = snapshot_for_report(&dir, "IBM", "2026-11-19");
        assert_eq!(today.unwrap().snapshot_date, "2026-11-19");
        assert_eq!(list_snapshots(&dir, "IBM"), ["2026-11-10", "2026-11-19"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn earnings_cache_of_python_has_nan_for_the_future() {
        let dir = temp("earnings");
        std::fs::create_dir_all(dir.join("IBM")).unwrap();
        std::fs::write(dir.join("IBM/yf_earnings.json"), EARNINGS).unwrap();
        let rows = cached_surprises(&dir, "IBM").unwrap();
        assert_eq!(rows.len(), 25);
        assert!(rows[0].actual_eps.is_nan() && rows[0].surprise.is_nan() && rows[0].estimated_eps == 4.63);
        assert_eq!((rows[1].date.as_str(), rows[1].actual_eps, rows[1].surprise_pct), ("2026-10-21", 2.95, 0.027875));
        assert!(cached_surprises(&dir, "NONE").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
