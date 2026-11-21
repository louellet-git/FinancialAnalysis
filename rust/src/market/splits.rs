//! Stock splits from Alpaca corporate actions, and the factor of a date range.
//!
//! The store price files are adjusted for splits to the date of the download (`prices.meta.json`:
//! `"adjusted": "splits"`). The shares of the SEC facts are as filed. The table of this module
//! (`<cache>/_corporate/splits.csv`) connects the two: `docs/traps/2026-11-21-split-adjusted-prices-against-filed-shares.md`.
//!
//! Rules that clean the Alpaca rows (price factors of a spin-off or a merger, ticker reuse), and the second source
//! of splits (steps of the cover share count): `splits_rules.rs`.
//!
//! Source: `GET https://data.alpaca.markets/v1/corporate-actions` (`types=forward_split,reverse_split`), the reply
//! `{"corporate_actions": {"forward_splits": [...], "reverse_splits": [...]}, "next_page_token": ...}`.
//! Terms for a public copy of the data: NOT read. The parser follows a real reply of 2026-11-21 (BKNG).

use serde_json::Value;
use std::io::Read;
use std::path::Path;

pub const HEADER: &str = "ticker,ex_date,old_rate,new_rate,factor,kind,sec_check,source,status,ex_date_estimated";
/// The `source` texts. No comma in a text: the CSV has no quoting. Only derived facts are stored: no raw Alpaca
/// reply, no id, no announcement, record or payable date.
pub const SRC_MATCH: &str = "alpaca corporate actions; checked against SEC";
pub const SRC_COVER: &str = "alpaca corporate actions; confirmed by SEC cover-count step";
pub const SRC_RECAST: &str = "alpaca corporate actions; confirmed by SEC share-count recast";
pub const SRC_NO_FACT: &str = "alpaca corporate actions; no SEC fact";
pub const SRC_UNCONFIRMED: &str = "alpaca corporate actions; not confirmed by SEC";
pub const SRC_STEP: &str = "SEC cover-count step";
pub const SRC_CORRECTED: &str = "SEC cover-count step; ex_date from alpaca corporate actions";
pub const SRC_FILE: &str = "correction file";
/// The SEC fact tag of a split ratio.
pub const SEC_TAG: &str = "StockholdersEquityNoteStockSplitConversionRatio1";
/// A SEC fact counts for a split when its end date is at most this many days from the ex date.
pub const SEC_WINDOW_DAYS: i32 = 120;
/// The tolerance of the ratio of a SEC fact (rounded forms such as 0.067 for 1/15).
pub const SEC_TOLERANCE: f64 = 0.01;

#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    pub ticker: String,
    pub ex_date: String,
    pub old_rate: f64,
    pub new_rate: f64,
    /// `new_rate / old_rate`: 25 for a 1:25 split, 0.1 for a 10:1 reverse split.
    pub factor: f64,
    /// `forward` or `reverse`.
    pub kind: String,
    /// `match`, `no_sec_fact`, `mismatch`, or empty before the check.
    pub sec_check: String,
}

/// A row of the table with all columns. `status`: `applied` (the ex date is not after the last date of the price
/// file of the ticker) or `pending` (the price file is not adjusted for it yet).
#[derive(Debug, Clone, PartialEq)]
pub struct SplitRow {
    pub split: Split,
    pub source: String,
    pub status: String,
    pub ex_date_estimated: bool,
}

/// The splits of one reply page. A page with `"corporate_actions": null` is empty. An error reply has a `message`.
/// A row with a factor that is not finite and positive is left out.
pub fn parse_page(page: &Value) -> Result<Vec<Split>, String> {
    let ca = match page.get("corporate_actions") {
        Some(Value::Object(m)) => m,
        Some(Value::Null) => return Ok(Vec::new()),
        _ => {
            let msg = page["message"].as_str().map(|m| m.chars().take(200).collect::<String>());
            return Err(match msg {
                Some(m) => format!("Alpaca error: {m}"),
                None => "Alpaca reply has no corporate_actions".into(),
            });
        }
    };
    let mut out = Vec::new();
    for (key, kind) in [("forward_splits", "forward"), ("reverse_splits", "reverse")] {
        for r in ca.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
            let (Some(sym), Some(ex), Some(old), Some(new)) = (r["symbol"].as_str(), r["ex_date"].as_str(), r["old_rate"].as_f64(), r["new_rate"].as_f64()) else { continue };
            if !(old.is_finite() && new.is_finite() && old > 0.0 && new > 0.0) {
                continue;
            }
            out.push(Split { ticker: from_alpaca_symbol(sym), ex_date: ex.chars().take(10).collect(), old_rate: old, new_rate: new, factor: new / old, kind: kind.into(), sec_check: String::new() });
        }
    }
    Ok(out)
}

/// Alpaca writes `BRK.B`; the store writes `BRK-B`.
pub fn to_alpaca_symbol(t: &str) -> String {
    t.replace('-', ".")
}
pub fn from_alpaca_symbol(s: &str) -> String {
    s.to_uppercase().replace('.', "-")
}

/// Sort by ticker and ex date, and drop an exact copy.
pub fn sort_dedupe(rows: &mut Vec<Split>) {
    rows.sort_by(|a, b| (&a.ticker, &a.ex_date, a.factor.to_bits()).cmp(&(&b.ticker, &b.ex_date, b.factor.to_bits())));
    rows.dedup_by(|a, b| a.ticker == b.ticker && a.ex_date == b.ex_date && a.factor == b.factor);
}

pub fn sort_dedupe_rows(rows: &mut Vec<SplitRow>) {
    rows.sort_by(|a, b| (&a.split.ticker, &a.split.ex_date, a.split.factor.to_bits()).cmp(&(&b.split.ticker, &b.split.ex_date, b.split.factor.to_bits())));
    rows.dedup_by(|a, b| a.split.ticker == b.split.ticker && a.split.ex_date == b.split.ex_date && a.split.factor == b.split.factor);
}

/// A fact of the SEC split ratio: the end date and the value.
pub type SecFact = (String, f64);

pub(crate) fn days(date: &str) -> Option<i32> {
    crate::trading::calendar::parse_iso(date)
}

/// The rule of `sec_check`. `match`: a fact within 120 days has the ratio, to 1% (or to 2 places for a value below 1). The ratio can be the factor, its
/// inverse, or, for a factor below 2, the factor minus 1 (5:4 as 0.25). `no_sec_fact`: no fact within 120 days. `mismatch`: facts near the
/// date, none with the ratio.
pub fn sec_check(ex_date: &str, factor: f64, facts: &[SecFact]) -> &'static str {
    let Some(ex) = days(ex_date) else { return "no_sec_fact" };
    // 1%, or a value rounded to 2 places (0.07 for 1/15).
    let same = |a: f64, b: f64| b.is_finite() && b > 0.0 && ((a - b).abs() <= SEC_TOLERANCE * b || (b < 1.0 && (a - b).abs() <= 0.0051));
    let mut near = false;
    for (end, val) in facts {
        let Some(e) = days(end) else { continue };
        if (e - ex).abs() > SEC_WINDOW_DAYS {
            continue;
        }
        near = true;
        if *val > 0.0 && (same(*val, factor) || same(*val, 1.0 / factor) || (factor < 2.0 && same(*val, factor - 1.0))) {
            return "match";
        }
    }
    if near {
        "mismatch"
    } else {
        "no_sec_fact"
    }
}

/// The split-ratio facts of a company facts text (`us-gaap`, `SEC_TAG`, unit `pure`).
pub fn sec_facts_from_json(v: &Value) -> Vec<SecFact> {
    let list = v["facts"]["us-gaap"][SEC_TAG]["units"]["pure"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    list.iter().filter_map(|f| Some((f["end"].as_str()?.to_string(), f["val"].as_f64()?))).collect()
}

/// The company facts of `<cache>/<T>/company_facts.json.gz` (none when the file is missing or bad).
pub fn read_company_facts(cache: &Path, ticker: &str) -> Option<Value> {
    let file = std::fs::File::open(cache.join(ticker).join("company_facts.json.gz")).ok()?;
    let mut text = String::new();
    flate2::read::GzDecoder::new(file).read_to_string(&mut text).ok()?;
    serde_json::from_str(&text).ok()
}

/// The facts of `<cache>/<T>/company_facts.json.gz` (none when the file is missing or bad).
pub fn sec_facts(cache: &Path, ticker: &str) -> Vec<SecFact> {
    read_company_facts(cache, ticker).map(|v| sec_facts_from_json(&v)).unwrap_or_default()
}

impl SplitRow {
    /// A row with the fields of an Alpaca row.
    pub fn new(split: Split, source: &str, status: &str, ex_date_estimated: bool) -> SplitRow {
        SplitRow { split, source: source.into(), status: status.into(), ex_date_estimated }
    }
}

pub fn to_csv(rows: &[SplitRow]) -> String {
    let mut s = format!("{HEADER}\n");
    for r in rows {
        let x = &r.split;
        s.push_str(&format!("{},{},{},{},{},{},{},{},{},{}\n", x.ticker, x.ex_date, x.old_rate, x.new_rate, x.factor, x.kind, x.sec_check, r.source, r.status, r.ex_date_estimated));
    }
    s
}

/// The rows of a `splits.csv` text. A bad line is an error: a factor that is not finite and positive, a bad date,
/// or a wrong count of fields. The old forms with 7 or 8 fields are read too. The ticker is upper case.
pub fn from_csv(text: &str) -> Result<Vec<SplitRow>, String> {
    let mut lines = text.lines();
    let first = lines.next().map(str::trim).unwrap_or("");
    if !HEADER.starts_with(first) || !first.starts_with("ticker,ex_date,old_rate,new_rate,factor,kind,sec_check") {
        return Err(format!("splits.csv: the first line must be '{HEADER}'"));
    }
    let width = first.split(',').count();
    let mut out = Vec::new();
    for (i, l) in lines.enumerate().filter(|(_, l)| !l.trim().is_empty()) {
        let n = i + 2;
        let f: Vec<&str> = l.split(',').collect();
        if f.len() != width {
            return Err(format!("splits.csv line {n}: need {width} fields"));
        }
        let num = |k: usize| f[k].trim().parse::<f64>().ok().filter(|x| x.is_finite() && *x > 0.0);
        let (Some(old_rate), Some(new_rate), Some(factor)) = (num(2), num(3), num(4)) else { return Err(format!("splits.csv line {n}: a rate or the factor is not a positive number")) };
        if days(f[1]).is_none() {
            return Err(format!("splits.csv line {n}: bad date '{}'", f[1]));
        }
        let split = Split { ticker: f[0].trim().to_uppercase(), ex_date: f[1].trim().into(), old_rate, new_rate, factor, kind: f[5].into(), sec_check: f[6].into() };
        let (source, status, est) = if width >= 10 { (f[7], f[8], f[9]) } else if width == 8 { (f[7], "applied", "false") } else { ("", "applied", "false") };
        if !matches!(status, "applied" | "pending") || !matches!(est, "true" | "false") {
            return Err(format!("splits.csv line {n}: bad status or ex_date_estimated"));
        }
        out.push(SplitRow { split, source: source.into(), status: status.into(), ex_date_estimated: est == "true" });
    }
    Ok(out)
}

/// Read `<cache>/_corporate/splits.csv` with all columns.
pub fn load_rows(cache: &Path) -> Result<Vec<SplitRow>, String> {
    let f = cache.join("_corporate").join("splits.csv");
    from_csv(&std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?)
}

/// Read `<cache>/_corporate/splits.csv`. Pending rows are in the list: ask `split_factor` with `to` = the last date
/// of the price file of the ticker (or use `split_factor_for_prices`).
pub fn load_splits(cache: &Path) -> Result<Vec<Split>, String> {
    Ok(load_rows(cache)?.into_iter().map(|r| r.split).collect())
}

/// The product of the factors of the splits of `ticker` with `after < ex_date <= to` (ISO dates compare as
/// text). 1.0 when there is none, and when `after >= to`. Shares of date `after` become shares of date `to` when
/// multiplied by it.
pub fn split_factor(splits: &[Split], ticker: &str, after: &str, to: &str) -> f64 {
    splits.iter().filter(|s| s.ticker == ticker && s.ex_date.as_str() > after && s.ex_date.as_str() <= to).map(|s| s.factor).product()
}

/// The factor between a date and the price file: the split adjusted price file ends at `price_last` (its last
/// date). A split with an ex date after it (status `pending`) is not in the prices, so it is not in the factor:
/// raw close of day d = adj_close(d) x `split_factor_for_prices(.., d, price_last)`.
pub fn split_factor_for_prices(splits: &[Split], ticker: &str, after: &str, price_last: &str) -> f64 {
    split_factor(splits, ticker, after, price_last)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(t: &str, d: &str, f: f64) -> Split {
        Split { ticker: t.into(), ex_date: d.into(), old_rate: 1.0, new_rate: f, factor: f, kind: "forward".into(), sec_check: String::new() }
    }
    fn row(s: Split) -> SplitRow {
        SplitRow::new(s, SRC_MATCH, "applied", false)
    }

    #[test]
    fn parse_a_reply() {
        let page: Value = serde_json::from_str(
            r#"{"corporate_actions":{"forward_splits":[{"symbol":"BKNG","ex_date":"2026-04-06","old_rate":1,"new_rate":25,"process_date":"2026-04-06","record_date":"2026-03-30","payable_date":"2026-04-02"},{"symbol":"BAD","ex_date":"2026-04-06","old_rate":0,"new_rate":25}],
            "reverse_splits":[{"symbol":"BRK.B","ex_date":"2020-01-02","old_rate":10,"new_rate":1}]},"next_page_token":null}"#,
        )
        .unwrap();
        let rows = parse_page(&page).unwrap();
        assert_eq!(rows.len(), 2, "a zero rate is left out");
        assert_eq!((rows[0].ticker.as_str(), rows[0].factor, rows[0].kind.as_str()), ("BKNG", 25.0, "forward"));
        assert_eq!((rows[1].ticker.as_str(), rows[1].factor, rows[1].kind.as_str()), ("BRK-B", 0.1, "reverse"));
        assert!(parse_page(&serde_json::json!({"corporate_actions": null})).unwrap().is_empty());
        assert!(parse_page(&serde_json::json!({"message": "forbidden"})).unwrap_err().contains("forbidden"));
        assert_eq!(to_alpaca_symbol("BRK-B"), "BRK.B");
    }

    #[test]
    fn factor_of_a_range() {
        let s = vec![sp("BKNG", "2026-04-06", 25.0), sp("NVDA", "2021-07-20", 4.0), sp("NVDA", "2024-06-10", 10.0)];
        assert_eq!(split_factor(&s, "BKNG", "2018-01-05", "2026-11-21"), 25.0);
        assert_eq!(split_factor(&s, "NVDA", "2020-01-01", "2026-11-21"), 40.0);
        assert_eq!(split_factor(&s, "NVDA", "2021-07-20", "2026-11-21"), 10.0, "after is exclusive");
        assert_eq!(split_factor(&s, "NVDA", "2020-01-01", "2024-06-10"), 40.0, "to is inclusive");
        assert_eq!(split_factor(&s, "NVDA", "2020-01-01", "2024-06-09"), 4.0);
        assert_eq!(split_factor(&s, "IBM", "2000-01-01", "2026-11-21"), 1.0);
        assert_eq!(split_factor(&s, "NVDA", "2025-01-01", "2020-01-01"), 1.0, "after > to: no split");
        let r = vec![Split { kind: "reverse".into(), factor: 0.1, ..sp("X", "2022-03-01", 0.1) }, sp("X", "2023-03-01", 2.0)];
        assert!((split_factor(&r, "X", "2022-01-01", "2024-01-01") - 0.2).abs() < 1e-12);
    }

    #[test]
    fn pending_split_is_not_in_the_prices() {
        let s = vec![sp("SNDK", "2026-11-20", 0.125)];
        assert_eq!(split_factor_for_prices(&s, "SNDK", "2025-01-01", "2026-11-19"), 1.0);
        assert_eq!(split_factor_for_prices(&s, "SNDK", "2025-01-01", "2026-11-20"), 0.125);
    }

    #[test]
    fn sec_rules() {
        let facts = vec![("2021-07-19".to_string(), 4.0), ("2024-05-31".to_string(), 10.0)];
        assert_eq!(sec_check("2021-07-20", 4.0, &facts), "match");
        assert_eq!(sec_check("2024-06-10", 10.0, &facts), "match");
        assert_eq!(sec_check("2021-07-20", 5.0, &facts), "mismatch");
        assert_eq!(sec_check("2015-01-01", 2.0, &facts), "no_sec_fact");
        assert_eq!(sec_check("2021-07-20", 4.0, &[]), "no_sec_fact");
        // the window: 120 days is inside, 121 is outside
        assert_eq!(sec_check("2021-11-16", 4.0, &facts), "match");
        assert_eq!(sec_check("2021-11-17", 4.0, &facts), "no_sec_fact");
        // a reverse split: the SEC value can be the inverse, or a rounded form
        let rf = vec![("2022-03-01".to_string(), 10.0)];
        assert_eq!(sec_check("2022-03-02", 0.1, &rf), "match");
        assert_eq!(sec_check("2022-03-02", 0.1, &[("2022-03-01".to_string(), 0.1)]), "match");
        assert_eq!(sec_check("2022-03-02", 0.1, &[("2022-03-01".to_string(), 5.0)]), "mismatch");
        assert_eq!(sec_check("2022-03-02", 1.0 / 15.0, &[("2022-03-01".to_string(), 0.067)]), "match", "1% tolerance");
        assert_eq!(sec_check("2022-03-02", 1.0 / 15.0, &[("2022-03-01".to_string(), 0.07)]), "match", "0.07 is 1/15 to 2 places");
        assert_eq!(sec_check("2022-03-02", 1.0 / 15.0, &[("2022-03-01".to_string(), 0.08)]), "mismatch");
        // 5:4 as 0.25 (the factor minus 1)
        assert_eq!(sec_check("2022-03-02", 1.25, &[("2022-03-01".to_string(), 0.25)]), "match");
    }

    #[test]
    fn sec_facts_from_a_text() {
        let v = serde_json::json!({"facts": {"us-gaap": {SEC_TAG: {"units": {"pure": [{"end": "2021-07-19", "val": 4}]}}}}});
        assert_eq!(sec_facts_from_json(&v), vec![("2021-07-19".to_string(), 4.0)]);
        assert!(sec_facts_from_json(&serde_json::json!({})).is_empty());
        assert!(sec_facts(Path::new("/nonexistent-cache"), "XXX").is_empty(), "a missing file gives no facts");
    }

    #[test]
    fn csv_form() {
        let mut rows = vec![row(sp("NVDA", "2024-06-10", 10.0)), row(sp("BKNG", "2026-04-06", 25.0)), row(sp("BKNG", "2026-04-06", 25.0))];
        rows.push(row(Split { kind: "reverse".into(), old_rate: 10.0, new_rate: 1.0, factor: 0.1, ..sp("AAA", "2022-01-03", 0.1) }));
        sort_dedupe_rows(&mut rows);
        rows[1].split.sec_check = "match".into();
        rows[2].status = "pending".into();
        rows[2].ex_date_estimated = true;
        let csv = to_csv(&rows);
        assert_eq!(
            csv,
            "ticker,ex_date,old_rate,new_rate,factor,kind,sec_check,source,status,ex_date_estimated\n\
             AAA,2022-01-03,10,1,0.1,reverse,,alpaca corporate actions; checked against SEC,applied,false\n\
             BKNG,2026-04-06,1,25,25,forward,match,alpaca corporate actions; checked against SEC,applied,false\n\
             NVDA,2024-06-10,1,10,10,forward,,alpaca corporate actions; checked against SEC,pending,true\n"
        );
        assert_eq!(from_csv(&csv).unwrap(), rows);
    }

    #[test]
    fn csv_bad_input() {
        assert!(from_csv("a,b\n").is_err(), "bad header");
        let h = HEADER;
        let bad_factor = format!("{h}\nA,2020-01-02,1,2,0,forward,,s,applied,false\n");
        assert!(from_csv(&bad_factor).unwrap_err().contains("positive"));
        let nan = format!("{h}\nA,2020-01-02,1,2,NaN,forward,,s,applied,false\n");
        assert!(from_csv(&nan).is_err());
        let few = format!("{h}\nA,2020-01-02,1,2\n");
        assert!(from_csv(&few).unwrap_err().contains("fields"));
        let date = format!("{h}\nA,2020-13-xx,1,2,2,forward,,s,applied,false\n");
        assert!(from_csv(&date).is_err());
        let status = format!("{h}\nA,2020-01-02,1,2,2,forward,,s,later,false\n");
        assert!(from_csv(&status).is_err());
        // an old file with 7 or 8 fields, and a lower case ticker
        let old = "ticker,ex_date,old_rate,new_rate,factor,kind,sec_check\nbkng,2026-04-06,1,25,25,forward,match\n";
        let r = from_csv(old).unwrap();
        assert_eq!((r[0].split.ticker.as_str(), r[0].status.as_str()), ("BKNG", "applied"));
        assert!(load_splits(Path::new("/nonexistent-cache")).is_err(), "a missing file is an error");
    }
}
