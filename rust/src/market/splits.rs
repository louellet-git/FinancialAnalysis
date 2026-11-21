//! Stock splits from Alpaca corporate actions, and the factor of a date range.
//!
//! The store price files are adjusted for splits to the date of the download (`prices.meta.json`:
//! `"adjusted": "splits"`). The shares of the SEC facts are as filed. The table of this module
//! (`<cache>/_corporate/splits.csv`) connects the two: `docs/traps/2026-11-21-split-adjusted-prices-against-filed-shares.md`.
//!
//! Source: `GET https://data.alpaca.markets/v1/corporate-actions` (`types=forward_split,reverse_split`), the reply
//! `{"corporate_actions": {"forward_splits": [...], "reverse_splits": [...]}, "next_page_token": ...}`.
//! Terms for a public copy of the data: NOT read. The parser follows a real reply of 2026-11-21 (BKNG).

use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

pub const HEADER: &str = "ticker,ex_date,old_rate,new_rate,factor,kind,sec_check,source";
/// The last column of each row. Only derived facts are stored: no raw Alpaca reply, no id, no announcement,
/// record or payable date. (No comma in the text: the CSV has no quoting.)
pub const SOURCE: &str = "alpaca corporate actions (checked against SEC)";
/// The SEC fact tag of a split ratio.
pub const SEC_TAG: &str = "StockholdersEquityNoteStockSplitConversionRatio1";
/// A SEC fact counts for a split when its end date is at most this many days from the ex date.
pub const SEC_WINDOW_DAYS: i32 = 120;

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

/// The splits of one reply page. A page with `"corporate_actions": null` is empty. An error reply has a `message`.
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
            if old <= 0.0 || new <= 0.0 {
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

/// A fact of the SEC split ratio: the end date and the value.
pub type SecFact = (String, f64);

fn days(date: &str) -> Option<i32> {
    crate::trading::calendar::parse_iso(date)
}

/// The rule of `sec_check`. `match`: a fact within 120 days has the ratio (for a reverse split the inverse
/// is also accepted). `no_sec_fact`: no fact within 120 days. `mismatch`: facts near the date, none with the ratio.
pub fn sec_check(ex_date: &str, factor: f64, facts: &[SecFact]) -> &'static str {
    let Some(ex) = days(ex_date) else { return "no_sec_fact" };
    let same = |a: f64, b: f64| (a - b).abs() <= 1e-6 * b.abs().max(1e-12);
    let mut near = false;
    for (end, val) in facts {
        let Some(e) = days(end) else { continue };
        if (e - ex).abs() > SEC_WINDOW_DAYS {
            continue;
        }
        near = true;
        if *val > 0.0 && (same(*val, factor) || same(*val, 1.0 / factor)) {
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

/// The facts of `<cache>/<T>/company_facts.json.gz` (none when the file is missing or bad).
pub fn sec_facts(cache: &Path, ticker: &str) -> Vec<SecFact> {
    let Ok(file) = std::fs::File::open(cache.join(ticker).join("company_facts.json.gz")) else { return Vec::new() };
    let mut text = String::new();
    if flate2::read::GzDecoder::new(file).read_to_string(&mut text).is_err() {
        return Vec::new();
    }
    serde_json::from_str::<Value>(&text).map(|v| sec_facts_from_json(&v)).unwrap_or_default()
}

/// Fill `sec_check` of each row from the company facts in `cache`.
pub fn apply_sec_check(rows: &mut [Split], cache: &Path) {
    let mut memo: BTreeMap<String, Vec<SecFact>> = BTreeMap::new();
    for r in rows {
        let facts = memo.entry(r.ticker.clone()).or_insert_with(|| sec_facts(cache, &r.ticker));
        r.sec_check = sec_check(&r.ex_date, r.factor, facts).into();
    }
}

pub fn to_csv(rows: &[Split]) -> String {
    let mut s = format!("{HEADER}\n");
    for r in rows {
        s.push_str(&format!("{},{},{},{},{},{},{},{SOURCE}\n", r.ticker, r.ex_date, r.old_rate, r.new_rate, r.factor, r.kind, r.sec_check));
    }
    s
}

/// The rows of a `splits.csv` text. A bad line is an error.
pub fn from_csv(text: &str) -> Result<Vec<Split>, String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some(HEADER) {
        return Err(format!("splits.csv: the first line must be '{HEADER}'"));
    }
    let mut out = Vec::new();
    for (i, l) in lines.enumerate().filter(|(_, l)| !l.trim().is_empty()) {
        let f: Vec<&str> = l.split(',').collect();
        let num = |k: usize| f.get(k).and_then(|x| x.parse::<f64>().ok());
        let (Some(old_rate), Some(new_rate), Some(factor)) = (num(2), num(3), num(4)) else { return Err(format!("splits.csv line {}: bad number", i + 2)) };
        if f.len() != 8 {
            return Err(format!("splits.csv line {}: need 8 fields", i + 2));
        }
        out.push(Split { ticker: f[0].into(), ex_date: f[1].into(), old_rate, new_rate, factor, kind: f[5].into(), sec_check: f[6].into() });
    }
    Ok(out)
}

/// Read `<cache>/_corporate/splits.csv`.
pub fn load_splits(cache: &Path) -> Result<Vec<Split>, String> {
    let f = cache.join("_corporate").join("splits.csv");
    from_csv(&std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?)
}

/// The product of the factors of the splits of `ticker` with `after < ex_date <= to` (ISO dates compare as
/// text). 1.0 when there is none. Shares of date `after` become shares of date `to` when multiplied by it.
pub fn split_factor(splits: &[Split], ticker: &str, after: &str, to: &str) -> f64 {
    splits.iter().filter(|s| s.ticker == ticker && s.ex_date.as_str() > after && s.ex_date.as_str() <= to).map(|s| s.factor).product()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(t: &str, d: &str, f: f64) -> Split {
        Split { ticker: t.into(), ex_date: d.into(), old_rate: 1.0, new_rate: f, factor: f, kind: "forward".into(), sec_check: String::new() }
    }

    #[test]
    fn parse_a_reply() {
        let page: Value = serde_json::from_str(
            r#"{"corporate_actions":{"forward_splits":[{"symbol":"BKNG","ex_date":"2026-04-06","old_rate":1,"new_rate":25,"process_date":"2026-04-06","record_date":"2026-03-30","payable_date":"2026-04-02"}],
            "reverse_splits":[{"symbol":"BRK.B","ex_date":"2020-01-02","old_rate":10,"new_rate":1}]},"next_page_token":null}"#,
        )
        .unwrap();
        let rows = parse_page(&page).unwrap();
        assert_eq!(rows.len(), 2);
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
        assert_eq!(split_factor(&s, "NVDA", "2020-01-01", "2024-06-10"), 40.0, "upto is inclusive");
        assert_eq!(split_factor(&s, "NVDA", "2020-01-01", "2024-06-09"), 4.0);
        assert_eq!(split_factor(&s, "IBM", "2000-01-01", "2026-11-21"), 1.0);
        let r = vec![Split { kind: "reverse".into(), factor: 0.1, ..sp("X", "2022-03-01", 0.1) }, sp("X", "2023-03-01", 2.0)];
        assert!((split_factor(&r, "X", "2022-01-01", "2024-01-01") - 0.2).abs() < 1e-12);
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
        // a reverse split: the SEC value can be the inverse
        let rf = vec![("2022-03-01".to_string(), 10.0)];
        assert_eq!(sec_check("2022-03-02", 0.1, &rf), "match");
        assert_eq!(sec_check("2022-03-02", 0.1, &[("2022-03-01".to_string(), 0.1)]), "match");
        assert_eq!(sec_check("2022-03-02", 0.1, &[("2022-03-01".to_string(), 5.0)]), "mismatch");
    }

    #[test]
    fn sec_facts_from_a_text() {
        let v = serde_json::json!({"facts": {"us-gaap": {SEC_TAG: {"units": {"pure": [{"end": "2021-07-19", "val": 4}]}}}}});
        assert_eq!(sec_facts_from_json(&v), vec![("2021-07-19".to_string(), 4.0)]);
        assert!(sec_facts_from_json(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn csv_form() {
        let mut rows = vec![sp("NVDA", "2024-06-10", 10.0), sp("BKNG", "2026-04-06", 25.0), sp("BKNG", "2026-04-06", 25.0)];
        rows.push(Split { kind: "reverse".into(), old_rate: 10.0, new_rate: 1.0, factor: 0.1, ..sp("AAA", "2022-01-03", 0.1) });
        sort_dedupe(&mut rows);
        rows[1].sec_check = "match".into();
        let csv = to_csv(&rows);
        assert_eq!(
            csv,
            "ticker,ex_date,old_rate,new_rate,factor,kind,sec_check,source\nAAA,2022-01-03,10,1,0.1,reverse,,alpaca corporate actions (checked against SEC)\nBKNG,2026-04-06,1,25,25,forward,match,alpaca corporate actions (checked against SEC)\nNVDA,2024-06-10,1,10,10,forward,,alpaca corporate actions (checked against SEC)\n"
        );
        assert_eq!(from_csv(&csv).unwrap(), rows);
        assert!(from_csv("a,b\n").is_err());
    }
}
