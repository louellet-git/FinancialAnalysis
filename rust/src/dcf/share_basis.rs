//! The share basis of the models: the share counts of the SEC facts on the basis of the split-adjusted closes of the store.
//!
//! The price files of the store are split-adjusted up to the last date of each file (`prices.meta.json`: "adjusted":
//! "splits"). The SEC facts show the share count of the date of the fact. A model value per share that divides by the share
//! count of the facts and is compared with an adjusted close is wrong by the split factor when a split happened after the
//! fact date. The correction: the share count of a fact filed on `s` is multiplied by the product of the factors of the
//! splits with `s < ex_date <= to` (`to`: the last date of the price file).
//!
//! The split table and the factor are in `market::splits` (the one reader, the one factor function). This module has only
//! what the models need on top of them: the `to` date of a ticker, the basis for one ticker, a load with a warning for the
//! live commands, and the check report for a backtest or a study.

use crate::market::splits::{load_splits, split_factor, Split};
use std::path::Path;

/// For a live command (`fa dcf auto`, `screen`, ...): the table of the cache, or a warning on stderr and no splits.
pub fn load_or_warn(cache: &Path) -> Vec<Split> {
    match load_splits(cache) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("fa: WARNING no split table ({e}): the share counts are not put on the basis of today; a company that split after its last filing has a value per share that is wrong by the split factor");
            Vec::new()
        }
    }
}

/// The last date of the price file of a ticker: `last_date` of `prices.meta.json`, else the date of the last row of
/// `prices.json`. `None` when the ticker has no price file. The price file is adjusted for the splits up to this date
/// only: a split after it (an ex-date after the last close) is not in the closes (SNDK: 1:8 on 2026-11-20, prices to 2026-11-19).
pub fn price_last_date(cache: &Path, ticker: &str) -> Option<String> {
    let prices = crate::market::prices::cache_path(cache, ticker);
    let meta = prices.with_file_name("prices.meta.json");
    let from_meta = std::fs::read_to_string(meta).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()).and_then(|v| v.get("last_date")?.as_str().map(str::to_string));
    let date = from_meta.or_else(|| crate::market::prices::read_cache(&prices).ok()?.last().map(|p| p.date.clone()))?;
    Some(date.get(..10).unwrap_or(&date).to_string())
}

/// The `to` date of the share basis of a ticker: the last date of its price file. With no price file: today, and a
/// warning on stderr when `warn` (the model has no price then; a split after the last close cannot be told, so every
/// split up to today counts).
pub fn basis_to(cache: &Path, ticker: &str, warn: bool) -> String {
    price_last_date(cache, ticker).unwrap_or_else(|| {
        if warn {
            eprintln!("fa: WARNING {ticker} has no price file: the share basis uses the splits up to today; a split after the last close of the market would be counted twice");
        }
        crate::market::today_iso()
    })
}

/// The `to` date for the share basis of a ticker, for the three readers of the facts (`load_vintages_with` of the portfolio,
/// `load_company_years` of the point-in-time test, `map_inputs_with` of the live commands): the last date of the price file
/// of the ticker, with no warning; an empty text when there are no splits (no factor is asked then).
pub fn to_for(cache: &Path, ticker: &str, splits: &[Split]) -> String {
    if splits.is_empty() {
        String::new()
    } else {
        basis_to(cache, ticker, false)
    }
}

/// The share basis for one ticker: `factor(s)` is `split_factor(splits, ticker, s, to)`. A model that reads a share
/// count takes `s` = the FILING date of the share fact (see `YearData::from_period`): a filing that is issued after a
/// split shows the share counts already adjusted for it (ASC 260), also for a period that ended before the split. Only
/// the splits after the filing change the count. `to` is the last date of the price file of the ticker (`basis_to`),
/// not today: the closes are adjusted only up to their own last date.
#[derive(Debug, Clone, Copy)]
pub struct Basis<'a> {
    pub splits: &'a [Split],
    pub ticker: &'a str,
    pub to: &'a str,
}

impl Basis<'_> {
    /// F for a share fact filed on `filed` (`YYYY-MM-DD`, or a longer text that starts with the date).
    pub fn factor(&self, filed: &str) -> f64 {
        split_factor(self.splits, self.ticker, filed.get(..10).unwrap_or(filed), self.to)
    }
}

/// The lines for the output of a backtest or a study: the tickers of the rows with `mismatch` and with `no_sec_fact`
/// (`sec_check` of the table) that are in `tickers`. Empty when there are none.
pub fn check_report(splits: &[Split], tickers: &[String]) -> String {
    let with = |check: &str| -> Vec<String> {
        let mut v: Vec<String> = splits.iter().filter(|s| s.sec_check == check && tickers.iter().any(|x| x.eq_ignore_ascii_case(&s.ticker))).map(|s| s.ticker.clone()).collect();
        v.sort();
        v.dedup();
        v
    };
    let mut out = String::new();
    let m = with("mismatch");
    if !m.is_empty() {
        out.push_str(&format!("split table: {} ticker(s) with sec_check = mismatch (the SEC share counts do not show the split; the share basis of these tickers is not confirmed): {}\n", m.len(), m.join(" ")));
    }
    let n = with("no_sec_fact");
    if !n.is_empty() {
        out.push_str(&format!("split table: {} ticker(s) with sec_check = no_sec_fact (no SEC share fact to check the split): {}\n", n.len(), n.join(" ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sp(t: &str, d: &str, factor: f64, check: &str) -> Split {
        Split { ticker: t.into(), ex_date: d.into(), old_rate: 1.0, new_rate: factor, factor, kind: if factor < 1.0 { "reverse" } else { "forward" }.into(), sec_check: check.into() }
    }

    fn price_dir(name: &str, rows: &[&str], meta: Option<&str>) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fa_share_basis_{name}_{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(dir.join("SNDK")).unwrap();
        let body: Vec<String> = rows.iter().map(|d| format!(r#"{{"date":"{d}","open":1,"high":1,"low":1,"close":1,"volume":1}}"#)).collect();
        std::fs::write(dir.join("SNDK").join("prices.json"), format!("[{}]", body.join(","))).unwrap();
        if let Some(m) = meta {
            std::fs::write(dir.join("SNDK").join("prices.meta.json"), m).unwrap();
        }
        dir
    }

    #[test]
    fn the_to_date_is_the_last_date_of_the_price_file() {
        // meta first, then the last row, then none
        let a = price_dir("a", &["2026-11-18", "2026-11-19"], Some(r#"{"adjusted":"splits","last_date":"2026-11-19"}"#));
        assert_eq!(price_last_date(&a, "SNDK").as_deref(), Some("2026-11-19"));
        let b = price_dir("b", &["2026-11-18", "2026-11-19"], None);
        assert_eq!(price_last_date(&b, "SNDK").as_deref(), Some("2026-11-19"));
        let c = price_dir("c", &["2026-11-17"], Some(r#"{"rows":1}"#)); // meta with no last_date: the last row
        assert_eq!(price_last_date(&c, "SNDK").as_deref(), Some("2026-11-17"));
        assert_eq!(price_last_date(&a, "NOPE"), None);
        // no price file: today (and a warning)
        assert_eq!(basis_to(&a, "NOPE", false), crate::market::today_iso());
        assert_eq!(basis_to(&a, "SNDK", false), "2026-11-19");
        for d in [a, b, c] {
            std::fs::remove_dir_all(d).ok();
        }
    }

    #[test]
    fn a_split_after_the_last_price_date_is_not_counted_and_one_on_it_is() {
        // SNDK: 1:8 on 2026-11-20 (factor 8), prices to 2026-11-19
        let s = [sp("SNDK", "2026-11-20", 8.0, "match")];
        let basis = |to| Basis { splits: &s, ticker: "SNDK", to };
        assert_eq!(basis("2026-11-19").factor("2026-02-15"), 1.0); // one day after the last price date
        assert_eq!(basis("2026-11-20").factor("2026-02-15"), 8.0); // on the last price date
        assert_eq!(basis("2026-11-21").factor("2026-02-15"), 8.0);
        // the filing date with a time part is cut to the date
        assert_eq!(basis("2026-11-21").factor("2026-02-15T10:00:00"), 8.0);
        assert_eq!(basis("2026-11-21").factor("2026-11-20"), 1.0); // a filing on the ex-date is already after the split
    }

    #[test]
    fn a_missing_table_is_an_error_and_the_live_load_warns() {
        let dir = std::env::temp_dir().join("fa_share_basis_missing_cache_test");
        assert!(load_splits(&dir).is_err());
        assert!(load_or_warn(&dir).is_empty());
    }

    #[test]
    fn the_check_report_names_the_tickers() {
        let s = [sp("BKNG", "2026-04-06", 25.0, "match"), sp("XYZ", "2022-01-03", 0.1, "mismatch"), sp("ZZZ", "2023-02-01", 2.0, "no_sec_fact")];
        let all = ["BKNG".to_string(), "XYZ".to_string(), "ZZZ".to_string()];
        let r = check_report(&s, &all);
        assert!(r.contains("mismatch") && r.contains("XYZ") && r.contains("no_sec_fact") && r.contains("ZZZ"));
        assert!(!r.contains("BKNG"));
        // a ticker that is not in the run is not named
        assert_eq!(check_report(&s, &["BKNG".to_string()]), "");
    }
}
