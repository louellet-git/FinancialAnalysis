//! Cross-check of two price sources (`fa prices check`).
//!
//! The check compares the closes that are split adjusted and not dividend adjusted (`Row::close`)
//! on the dates that both sources have. Adjusted closes are not compared: a source that
//! adjusts for dividends and one that does not differ on purpose.
//!
//! It also verifies, in one source, that the adjusted series agrees with the events of that
//! source: a split (the split adjusted close has no jump) and a dividend (the adjustment factor
//! changes by `dividend / previous close`).

use super::price_source::Row;
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub a: String,
    pub b: String,
    pub common: usize,
    pub only_a: usize,
    pub only_b: usize,
    pub first: String,
    pub last: String,
    /// Largest relative difference `|a - b| / b` and its date.
    pub max_rel: f64,
    pub max_date: String,
    pub tolerance: f64,
    /// Date, close of a, close of b, relative difference: the dates above the tolerance.
    pub above: Vec<(String, f64, f64, f64)>,
    /// Splits seen in either source: date and ratio.
    pub splits: Vec<(String, f64)>,
}

pub fn compare(a_name: &str, a: &[Row], b_name: &str, b: &[Row], tolerance: f64) -> Comparison {
    let bmap: HashMap<&str, &Row> = b.iter().map(|r| (r.date.as_str(), r)).collect();
    let amap: HashMap<&str, &Row> = a.iter().map(|r| (r.date.as_str(), r)).collect();
    let mut c = Comparison {
        a: a_name.into(),
        b: b_name.into(),
        common: 0,
        only_a: 0,
        only_b: b.iter().filter(|r| !amap.contains_key(r.date.as_str())).count(),
        first: String::new(),
        last: String::new(),
        max_rel: 0.0,
        max_date: String::new(),
        tolerance,
        above: Vec::new(),
        splits: Vec::new(),
    };
    for r in a {
        let Some(o) = bmap.get(r.date.as_str()) else {
            c.only_a += 1;
            continue;
        };
        c.common += 1;
        if c.first.is_empty() {
            c.first = r.date.clone();
        }
        c.last = r.date.clone();
        let rel = ((r.close - o.close) / o.close).abs();
        if rel > c.max_rel {
            c.max_rel = rel;
            c.max_date = r.date.clone();
        }
        if rel > tolerance {
            c.above.push((r.date.clone(), r.close, o.close, rel));
        }
    }
    let mut splits: Vec<(String, f64)> = a.iter().chain(b.iter()).filter_map(|r| r.split.map(|s| (r.date.clone(), s))).collect();
    splits.sort_by(|x, y| x.0.cmp(&y.0));
    splits.dedup_by(|y, x| x.0 == y.0);
    c.splits = splits;
    c
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventCheck {
    pub dividends_checked: usize,
    /// Largest `|implied - expected|` of the dividend yield, and its date.
    pub dividend_max_error: f64,
    pub dividend_worst_date: String,
    /// Date, ratio, close before, close on the date: the split adjusted close must have no jump.
    pub splits: Vec<(String, f64, f64, f64)>,
}

/// Check the adjusted close of one source against its own dividend and split events.
pub fn check_events(rows: &[Row]) -> EventCheck {
    let mut out = EventCheck { dividends_checked: 0, dividend_max_error: 0.0, dividend_worst_date: String::new(), splits: Vec::new() };
    for i in 1..rows.len() {
        let (p, r) = (&rows[i - 1], &rows[i]);
        if let (Some(div), Some(ap), Some(ar)) = (r.dividend, p.adj_close, r.adj_close) {
            let (rp, rr) = (ap / p.close, ar / r.close);
            let implied = 1.0 - rp / rr;
            let err = (implied - div / p.close).abs();
            out.dividends_checked += 1;
            if err > out.dividend_max_error {
                out.dividend_max_error = err;
                out.dividend_worst_date = r.date.clone();
            }
        }
        if let Some(s) = r.split {
            out.splits.push((r.date.clone(), s, p.close, r.close));
        }
    }
    out
}

pub fn render(c: &Comparison, events: &[(String, EventCheck)]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "{} against {} (split adjusted closes, tolerance {:.2}%)", c.a, c.b, c.tolerance * 100.0);
    let _ = writeln!(s, "  common dates {} ({} to {}); only in {}: {}; only in {}: {}", c.common, c.first, c.last, c.a, c.only_a, c.b, c.only_b);
    let _ = writeln!(s, "  largest relative difference {:.4}% on {}", c.max_rel * 100.0, c.max_date);
    let _ = writeln!(s, "  dates above the tolerance: {}", c.above.len());
    for (d, x, y, rel) in c.above.iter().take(10) {
        let _ = writeln!(s, "    {d}  {}={x:.4}  {}={y:.4}  {:.3}%", c.a, c.b, rel * 100.0);
    }
    if c.above.len() > 10 {
        let _ = writeln!(s, "    ... {} more", c.above.len() - 10);
    }
    if c.splits.is_empty() {
        let _ = writeln!(s, "  splits: none in the common range (a source gives splits only for Tiingo and Yahoo)");
    } else {
        let list: Vec<String> = c.splits.iter().map(|(d, r)| format!("{d} {r}:1")).collect();
        let _ = writeln!(s, "  splits: {}. Both series are split adjusted. If they differ by a split ratio before a date, one source does not adjust.", list.join(", "));
    }
    for (name, e) in events {
        let _ = writeln!(s, "  {name} dividends: {} checked, largest error of the adjustment {:.6} (a fraction of the price){}", e.dividends_checked, e.dividend_max_error, if e.dividend_worst_date.is_empty() { String::new() } else { format!(" on {}", e.dividend_worst_date) });
        for (d, ratio, before, on) in &e.splits {
            let _ = writeln!(s, "  {name} split {ratio}:1 on {d}: close before {before:.4}, on the date {on:.4} (ratio {:.3}; near 1 means the series is split adjusted)", on / before);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(date: &str, close: f64) -> Row {
        Row { date: date.into(), open: close, high: close, low: close, close, volume: 1, adj_close: None, dividend: None, split: None }
    }

    #[test]
    fn compare_counts_dates_and_differences() {
        let a = vec![row("2024-01-02", 100.0), row("2024-01-03", 101.0), row("2024-01-04", 102.0)];
        let b = vec![row("2024-01-03", 101.0), row("2024-01-04", 104.04), row("2024-01-05", 105.0)];
        let c = compare("yahoo", &a, "nasdaq", &b, 0.001);
        assert_eq!((c.common, c.only_a, c.only_b), (2, 1, 1));
        assert_eq!((c.first.as_str(), c.last.as_str()), ("2024-01-03", "2024-01-04"));
        assert_eq!(c.max_date, "2024-01-04");
        assert!((c.max_rel - 0.019607843).abs() < 1e-6);
        assert_eq!(c.above.len(), 1);
        assert!(render(&c, &[]).contains("dates above the tolerance: 1"));
    }

    #[test]
    fn a_split_in_one_source_only_shows_as_a_large_difference_and_a_note() {
        let mut a = vec![row("2024-06-07", 120.888), row("2024-06-10", 121.79)];
        a[1].split = Some(10.0);
        let b = vec![row("2024-06-07", 1208.88), row("2024-06-10", 121.79)];
        let c = compare("tiingo", &a, "raw", &b, 0.001);
        assert_eq!(c.above.len(), 1);
        assert_eq!(c.splits, vec![("2024-06-10".to_string(), 10.0)]);
    }

    #[test]
    fn dividend_and_split_events() {
        // Close 100, a dividend of 1 on day 2: adjusted close of day 1 is 99% of its ratio.
        let mut r1 = row("2024-01-02", 100.0);
        r1.adj_close = Some(99.0);
        let mut r2 = row("2024-01-03", 100.0);
        r2.adj_close = Some(100.0);
        r2.dividend = Some(1.0);
        let mut r3 = row("2024-01-04", 50.0);
        r3.split = Some(2.0);
        let e = check_events(&[r1, r2, r3]);
        assert_eq!(e.dividends_checked, 1);
        assert!(e.dividend_max_error < 1e-12, "{}", e.dividend_max_error);
        assert_eq!(e.splits.len(), 1);
        // A wrong adjustment shows.
        let mut w1 = row("2024-01-02", 100.0);
        w1.adj_close = Some(100.0);
        let mut w2 = row("2024-01-03", 100.0);
        w2.adj_close = Some(100.0);
        w2.dividend = Some(1.0);
        assert!(check_events(&[w1, w2]).dividend_max_error > 0.009);
    }

    #[test]
    fn empty_lists() {
        let c = compare("a", &[], "b", &[], 0.001);
        assert_eq!((c.common, c.max_rel), (0, 0.0));
        assert!(render(&c, &[]).contains("common dates 0"));
    }
}
