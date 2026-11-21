//! Stock splits and the share basis of the models.
//!
//! The price files of the store are split-adjusted to today (`prices.meta.json`: "adjusted": "splits"). The SEC facts
//! show the share count of the date of the fact. A model value per share that divides by the share count of the facts
//! and is compared with an adjusted close is wrong by the split factor when a split happened after the fact date.
//! The correction: the share count of a fact dated `s` is multiplied by `F(s)`, the product of the factors of the
//! splits with `s < ex_date <= today`. The adjusted close stays as it is. The shares are then on the basis of today.
//!
//! The file is `<cache>/_corporate/splits.csv`: `ticker,ex_date,old_rate,new_rate,factor,kind,sec_check`, one row for each
//! split, sorted by ticker and ex_date. `factor` is `new_rate / old_rate` (a 25:1 split: 25; a 1-for-10 reverse split: 0.1).
//!
//! Limits: a spin-off and a class change are not splits and are not in the file. A row with `sec_check` other than
//! `match` is not confirmed by a share-count step in the SEC facts (`mismatch_tickers`, `unchecked_tickers`).

use std::path::{Path, PathBuf};

/// One row of `splits.csv`.
#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    pub ticker: String,
    /// `YYYY-MM-DD`
    pub ex_date: String,
    pub old_rate: f64,
    pub new_rate: f64,
    /// `new_rate / old_rate`
    pub factor: f64,
    /// `forward` or `reverse`
    pub kind: String,
    /// `match`, `no_sec_fact` or `mismatch`
    pub sec_check: String,
    /// Where the row comes from (the eighth column; empty in a file with seven columns).
    pub source: String,
}

const HEADER: &str = "ticker,ex_date,old_rate,new_rate,factor,kind,sec_check,source";
const HEADER_7: &str = "ticker,ex_date,old_rate,new_rate,factor,kind,sec_check";

/// The file of a cache.
pub fn splits_path(cache: &Path) -> PathBuf {
    cache.join("_corporate").join("splits.csv")
}

/// The rows of a `splits.csv` text.
pub fn parse_splits(text: &str, name: &str) -> Result<Vec<Split>, String> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let n_fields = match lines.next() {
        Some(h) if h == HEADER => 8,
        Some(h) if h == HEADER_7 => 7,
        Some(h) => return Err(format!("{name}: the header is '{h}', expected '{HEADER}'")),
        None => return Err(format!("{name}: the file is empty")),
    };
    let mut out = Vec::new();
    for (i, line) in lines.enumerate() {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.len() != n_fields {
            return Err(format!("{name}: row {} has {} fields, expected {n_fields}", i + 1, f.len()));
        }
        let num = |k: usize| -> Result<f64, String> {
            f[k].parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0).ok_or_else(|| format!("{name}: row {}: '{}' is not a positive number", i + 1, f[k]))
        };
        if f[1].len() != 10 || f[1].as_bytes()[4] != b'-' || f[1].as_bytes()[7] != b'-' {
            return Err(format!("{name}: row {}: ex_date '{}' is not YYYY-MM-DD", i + 1, f[1]));
        }
        out.push(Split { ticker: f[0].to_uppercase(), ex_date: f[1].to_string(), old_rate: num(2)?, new_rate: num(3)?, factor: num(4)?, kind: f[5].to_string(), sec_check: f[6].to_string(), source: f.get(7).map_or(String::new(), |s| s.to_string()) });
    }
    Ok(out)
}

/// The rows of `<cache>/_corporate/splits.csv`. A missing file is an error (for a backtest or a study of returns: the
/// prices of the store are split-adjusted, the SEC share counts are not).
pub fn load_splits(cache: &Path) -> Result<Vec<Split>, String> {
    let path = splits_path(cache);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e} (the split table is necessary)", path.display()))?;
    parse_splits(&text, &path.display().to_string())
}

/// For a live command (`fa dcf auto`, `screen`, ...): the file of the cache, or a warning on stderr and no splits.
pub fn load_or_warn(cache: &Path) -> Vec<Split> {
    match load_splits(cache) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("fa: WARNING no split table ({e}): the share counts are not put on the basis of today; a company that split after its last filing has a value per share that is wrong by the split factor");
            Vec::new()
        }
    }
}

/// The product of the factors of the splits of `ticker` with `after < ex_date <= to` (dates `YYYY-MM-DD`; 1.0 with none).
pub fn split_factor(splits: &[Split], ticker: &str, after: &str, to: &str) -> f64 {
    let after = after.get(..10).unwrap_or(after);
    splits.iter().filter(|s| s.ticker.eq_ignore_ascii_case(ticker) && s.ex_date.as_str() > after && s.ex_date.as_str() <= to).map(|s| s.factor).product()
}

/// The sorted tickers with a split row whose `sec_check` is `mismatch` (the SEC share counts do not show the split).
pub fn mismatch_tickers(splits: &[Split]) -> Vec<String> {
    tickers_with(splits, |c| c == "mismatch")
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
    /// F for a share fact filed on `filed` (`YYYY-MM-DD`).
    pub fn factor(&self, filed: &str) -> f64 {
        split_factor(self.splits, self.ticker, filed, self.to)
    }
}

/// The sorted tickers with a split row that has no SEC fact to check it (`no_sec_fact`).
pub fn unchecked_tickers(splits: &[Split]) -> Vec<String> {
    tickers_with(splits, |c| c == "no_sec_fact")
}

fn tickers_with(splits: &[Split], keep: impl Fn(&str) -> bool) -> Vec<String> {
    let mut v: Vec<String> = splits.iter().filter(|s| keep(&s.sec_check)).map(|s| s.ticker.clone()).collect();
    v.sort();
    v.dedup();
    v
}

/// The lines for the output of a backtest or a study: the tickers of the rows with `mismatch` and with `no_sec_fact`
/// that are in `tickers`. Empty when there are none.
pub fn check_report(splits: &[Split], tickers: &[String]) -> String {
    let inside = |v: Vec<String>| -> Vec<String> { v.into_iter().filter(|t| tickers.iter().any(|x| x.eq_ignore_ascii_case(t))).collect() };
    let mut out = String::new();
    let m = inside(mismatch_tickers(splits));
    if !m.is_empty() {
        out.push_str(&format!("split table: {} ticker(s) with sec_check = mismatch (the SEC share counts do not show the split; the share basis of these tickers is not confirmed): {}\n", m.len(), m.join(" ")));
    }
    let n = inside(unchecked_tickers(splits));
    if !n.is_empty() {
        out.push_str(&format!("split table: {} ticker(s) with sec_check = no_sec_fact (no SEC share fact to check the split): {}\n", n.len(), n.join(" ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(t: &str, d: &str, factor: f64, check: &str) -> Split {
        Split { ticker: t.into(), ex_date: d.into(), old_rate: 1.0, new_rate: factor, factor, kind: if factor < 1.0 { "reverse" } else { "forward" }.into(), sec_check: check.into(), source: String::new() }
    }
    const TODAY: &str = "2026-11-21";

    #[test]
    fn no_split_is_one() {
        assert_eq!(split_factor(&[], "BKNG", "2018-01-05", TODAY), 1.0);
        assert_eq!(split_factor(&[sp("AAPL", "2020-08-31", 4.0, "match")], "BKNG", "2018-01-05", TODAY), 1.0);
    }

    #[test]
    fn one_split_after_the_date() {
        let s = [sp("BKNG", "2026-04-06", 25.0, "match")];
        assert_eq!(split_factor(&s, "BKNG", "2018-01-05", TODAY), 25.0);
        assert_eq!(split_factor(&s, "bkng", "2025-12-31", TODAY), 25.0);
    }

    #[test]
    fn split_before_the_date_is_one() {
        let s = [sp("NVDA", "2024-06-10", 10.0, "match")];
        assert_eq!(split_factor(&s, "NVDA", "2025-01-26", TODAY), 1.0);
    }

    fn price_dir(name: &str, rows: &[&str], meta: Option<&str>) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fa_splits_{name}_{}", std::process::id()));
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
    }

    #[test]
    fn two_splits_multiply_and_the_reverse_split_divides() {
        let s = [sp("X", "2015-01-02", 2.0, "match"), sp("X", "2021-03-04", 3.0, "match"), sp("X", "2024-05-06", 0.1, "match")];
        assert!((split_factor(&s, "X", "2010-12-31", TODAY) - 0.6).abs() < 1e-12);
        assert!((split_factor(&s, "X", "2016-12-31", TODAY) - 0.3).abs() < 1e-12);
        assert!((split_factor(&s, "X", "2022-12-31", TODAY) - 0.1).abs() < 1e-12);
        assert_eq!(split_factor(&s, "X", "2024-12-31", TODAY), 1.0);
    }

    #[test]
    fn the_boundary_ex_date_counts_only_after_s() {
        let s = [sp("X", "2020-06-30", 4.0, "match")];
        assert_eq!(split_factor(&s, "X", "2020-06-29", TODAY), 4.0);
        // ex_date == s: the fact of that day is already after the split
        assert_eq!(split_factor(&s, "X", "2020-06-30", TODAY), 1.0);
        assert_eq!(split_factor(&s, "X", "2020-07-01", TODAY), 1.0);
        // the end of the range is included; a split after `to` is not
        assert_eq!(split_factor(&s, "X", "2020-06-29", "2020-06-30"), 4.0);
        assert_eq!(split_factor(&s, "X", "2020-06-29", "2020-06-29"), 1.0);
    }

    const CSV: &str = "ticker,ex_date,old_rate,new_rate,factor,kind,sec_check\nBKNG,2026-04-06,1,25,25,forward,match\nXYZ,2022-01-03,10,1,0.1,reverse,mismatch\nZZZ,2023-02-01,1,2,2,forward,no_sec_fact\n";

    #[test]
    fn the_file_text_is_read() {
        let s = parse_splits(CSV, "t").unwrap();
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], Split { ticker: "BKNG".into(), ex_date: "2026-04-06".into(), old_rate: 1.0, new_rate: 25.0, factor: 25.0, kind: "forward".into(), sec_check: "match".into(), source: String::new() });
        assert_eq!(s[1].factor, 0.1);
        let eight = parse_splits("ticker,ex_date,old_rate,new_rate,factor,kind,sec_check,source\nBKNG,2026-04-06,1,25,25,forward,match,alpaca\n", "t").unwrap();
        assert_eq!(eight[0].source, "alpaca");
        assert!(parse_splits("a,b\n", "t").is_err());
        assert!(parse_splits("", "t").is_err());
        assert!(parse_splits(&CSV.replace("25,25,forward", "25,x,forward"), "t").is_err());
        assert!(parse_splits(&CSV.replace("2026-04-06", "04/06/2026"), "t").is_err());
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let dir = std::env::temp_dir().join("fa_splits_missing_cache_test");
        assert!(load_splits(&dir).is_err());
        assert!(load_or_warn(&dir).is_empty());
    }

    #[test]
    fn the_mismatch_report_names_the_tickers() {
        let s = parse_splits(CSV, "t").unwrap();
        assert_eq!(mismatch_tickers(&s), vec!["XYZ".to_string()]);
        let all = ["BKNG".to_string(), "XYZ".to_string(), "ZZZ".to_string()];
        let r = check_report(&s, &all);
        assert!(r.contains("mismatch") && r.contains("XYZ") && r.contains("no_sec_fact") && r.contains("ZZZ"));
        assert!(!r.contains("BKNG"));
        // a ticker that is not in the run is not named
        assert_eq!(check_report(&s, &["BKNG".to_string()]), "");
    }
}
