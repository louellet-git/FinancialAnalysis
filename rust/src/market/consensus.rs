//! Forward consensus snapshots and the earnings cache of the Python code (the Rust form of `estimates.py`
//! and the cache part of `market_data.py`).
//!
//! The snapshot is `data/raw/<T>/estimates/<date>.json`. The source in Rust is the Nasdaq site API
//! (`nasdaq_est`, `estimates_cache`; `fa estimates get`). The Python code got the snapshot from yfinance,
//! whose Yahoo endpoints refuse a program that gives its real name (trap #93 of `docs/HIDDEN-ASSUMPTIONS.md`);
//! the files that it made are still read. The Nasdaq snapshot has the same layout (`0q`, `+1q`, `0y`, `+1y`
//! and the columns `avg`, `low`, `high`, `numberOfAnalysts`) and a few more fields (`source`, `endpoints`,
//! `period_ends`, `ratings`, `consensus_change`). It has no revenue estimates; `fmp` gives them when the key exists.
//!
//! The older earnings cache is `data/raw/<T>/yf_earnings.json` (read by `cached_surprises`).
//! The report date and the surprise of the Nasdaq source are in `earnings_surprises.json`.

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
    /// The source of the snapshot (`nasdaq`). A snapshot of the Python code has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The endpoints that gave the data of this snapshot (`forecast`, `targets`, `momentum`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endpoints: Vec<String>,
    /// The end of each fiscal period as `YYYY-MM` (`0y` is `2027-06`).
    #[serde(default, skip_serializing_if = "Ordered::is_empty")]
    pub period_ends: Ordered<String>,
    /// The number of analysts for each rating: `buy`, `hold`, `sell`.
    #[serde(default, skip_serializing_if = "Ordered::is_empty")]
    pub ratings: Ordered<Option<f64>>,
    /// The consensus EPS now, 1 week ago and 1 month ago, for the quarter (`qtr_*`) and the year (`yr_*`).
    #[serde(default, skip_serializing_if = "Ordered::is_empty")]
    pub consensus_change: Ordered<Option<f64>>,
}

impl ConsensusEstimates {
    /// The number of records (the count that `analyze` printed).
    pub fn record_count(&self) -> usize {
        self.revenue_estimate.len() + self.earnings_estimate.len() + self.analyst_price_targets.len()
    }

    /// An empty snapshot of a source.
    pub fn empty(ticker: &str, date: &str, source: &str) -> ConsensusEstimates {
        ConsensusEstimates {
            ticker: ticker.to_uppercase(),
            snapshot_date: date.to_string(),
            revenue_estimate: Ordered::default(),
            earnings_estimate: Ordered::default(),
            analyst_price_targets: Ordered::default(),
            source: Some(source.to_string()),
            endpoints: Vec::new(),
            period_ends: Ordered::default(),
            ratings: Ordered::default(),
            consensus_change: Ordered::default(),
        }
    }

    /// A value of the earnings estimate: `eps("0y", "avg")`.
    pub fn eps(&self, period: &str, metric: &str) -> Option<f64> {
        let (_, metrics) = self.earnings_estimate.0.iter().find(|(p, _)| p == period)?;
        metrics.0.iter().find(|(m, _)| m == metric).and_then(|(_, v)| *v)
    }

    /// The periods of the earnings estimate that end in years (`0y`, `+1y`, ...), in the file order.
    pub fn year_periods(&self) -> Vec<&str> {
        self.earnings_estimate.0.iter().map(|(p, _)| p.as_str()).filter(|p| p.ends_with('y')).collect()
    }

    /// The consensus EPS of the next four quarters (`0q` to `+3q`): the sum, or `None` when a quarter is missing.
    pub fn eps_next_four_quarters(&self) -> Option<f64> {
        ["0q", "+1q", "+2q", "+3q"].iter().map(|p| self.eps(p, "avg")).sum()
    }

    /// The growth of the consensus EPS of each fiscal year over the year before: `(period, growth)` from `+1y` on.
    /// A year with no EPS, or with an EPS of zero or less in the year before, gives no growth.
    pub fn eps_growth(&self) -> Vec<(String, f64)> {
        let years = self.year_periods();
        let mut out = Vec::new();
        for pair in years.windows(2) {
            if let (Some(a), Some(b)) = (self.eps(pair[0], "avg"), self.eps(pair[1], "avg")) {
                if a > 0.0 && b.is_finite() {
                    out.push((pair[1].to_string(), b / a - 1.0));
                }
            }
        }
        out
    }
}

/// The forward price/earnings ratio: price over the consensus EPS. `None` when the EPS is not positive.
pub fn forward_pe(price: f64, eps: Option<f64>) -> Option<f64> {
    eps.filter(|e| *e > 0.0 && e.is_finite() && price.is_finite()).map(|e| price / e)
}

/// Put the revenue consensus of FMP (annual and quarterly records) into a snapshot. A record goes to the period of the
/// snapshot that ends in the same month and year (`period_ends`). A period with no FMP record has no revenue.
/// FMP gives the average only; the range and the number of analysts of revenue stay empty. Returns the number of periods.
pub fn merge_revenue(snapshot: &mut ConsensusEstimates, estimates: &[super::fmp::EarningsEstimate]) -> usize {
    let mut table: Vec<(String, Ordered<Option<f64>>)> = Vec::new();
    for (period, end) in &snapshot.period_ends.0 {
        let kind = if period.ends_with('y') { "annual" } else { "quarterly" };
        let found = estimates.iter().find(|e| e.period_type == kind && e.period_end.get(..7) == Some(end.as_str()) && e.estimated_revenue.is_some());
        if let Some(e) = found {
            // `number_of_analysts` of FMP counts the EPS analysts, so it is not a revenue field.
            table.push((period.clone(), Ordered(vec![("avg".to_string(), e.estimated_revenue)])));
        }
    }
    let n = table.len();
    if n > 0 {
        snapshot.revenue_estimate = Ordered(table);
        if !snapshot.endpoints.iter().any(|e| e == "fmp-revenue") {
            snapshot.endpoints.push("fmp-revenue".to_string());
        }
    }
    n
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
            (Some(s), format!("{n} forward consensus records loaded from the snapshot of {date} (older than today; this command makes no request: run fa estimates get {ticker} for a new one)"))
        }
        None => (None, format!("no snapshot in the cache and no source in this command; run: fa estimates get {ticker}")),
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
