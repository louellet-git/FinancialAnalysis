//! `fa splits get`: the table of stock splits (`<cache>/_corporate/splits.csv`) from Alpaca corporate actions.

use super::alpaca::{Client, Keys, DATA_BASE, KEY_ID_ENV, SECRET_ENV};
use super::keys::scrub_all;
use super::price_cli::{collect_tickers, parse};
use super::price_source::{FetchError, Get};
use super::splits::{self, read_company_facts, Split, SplitRow};
use super::splits_rules::{builtin_corrections, rules_for_ticker, sec_data_from_json, Event};
use super::{today_iso, write_atomic};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

pub const USAGE: &str = "\
fa splits - stock splits of the tickers (Alpaca corporate actions), with a check against the SEC facts

USAGE:
  fa splits get [TICKER...] [--tickers-file F] [--start D] [--end D] [--cache DIR]
                              ask Alpaca (needs APCA_API_KEY_ID and APCA_API_SECRET_KEY; 50 tickers in one request)
                              and write DIR/_corporate/splits.csv and splits.meta.json.
                              Default: --start 2009-01-01, --end today, --cache data/raw.

Columns: ticker,ex_date,old_rate,new_rate,factor,kind,sec_check,source (derived facts only; no raw reply is saved). factor = new_rate / old_rate (25 for 1:25).
sec_check: match, no_sec_fact or mismatch (the fact StockholdersEquityNoteStockSplitConversionRatio1 of DIR/<T>/company_facts.json.gz).
The price files are adjusted for splits; the SEC shares are as filed: use split_factor (rust/src/market/splits.rs).";

/// Tickers in one request.
pub const BATCH: usize = 50;
/// Seconds between two requests: 120 each minute (the free plan allows 200).
pub const PAUSE: f64 = 0.5;
const PAGE_LIMIT: usize = 1000;

fn client(keys: Keys) -> Client {
    let (id, secret) = (keys.id.clone(), keys.secret.clone());
    let get: Get = Box::new(move |url| {
        let headers = [("APCA-API-KEY-ID", id.as_str()), ("APCA-API-SECRET-KEY", secret.as_str())];
        crate::hostnet::get(url, &headers, PAUSE).map_err(|e| scrub_all(&e, &[&id, &secret]))
    });
    Client::new(keys, get)
}

/// All splits of `tickers` in the window. A network error is tried one more time.
pub fn fetch(client: &Client, tickers: &[String], start: &str, end: &str, retry_wait: Duration, log: &mut dyn FnMut(&str)) -> Result<Vec<Split>, String> {
    let mut all = Vec::new();
    let nb = tickers.len().div_ceil(BATCH);
    for (i, chunk) in tickers.chunks(BATCH).enumerate() {
        let syms: Vec<String> = chunk.iter().map(|t| splits::to_alpaca_symbol(t)).collect();
        let base = format!("{DATA_BASE}/v1/corporate-actions?symbols={}&types=forward_split,reverse_split&start={start}&end={end}&limit={PAGE_LIMIT}", syms.join(","));
        let mut got = Vec::new();
        for attempt in 0..2 {
            got.clear();
            match client.paged(&base, |page| splits::parse_page(page).map(|mut r| got.append(&mut r))) {
                Ok(()) => break,
                Err(FetchError::RateLimited(m)) => return Err(format!("Alpaca refused the request: {m}")),
                Err(FetchError::Failed(m)) if attempt == 0 => {
                    log(&format!("batch {}/{nb}: {m}; one more try", i + 1));
                    std::thread::sleep(retry_wait);
                }
                Err(FetchError::Failed(m)) => return Err(format!("batch {}/{nb}: {m}", i + 1)),
            }
        }
        all.append(&mut got);
        if (i + 1) % 5 == 0 || i + 1 == nb {
            log(&format!("batch {}/{nb}: {} splits so far, {} requests", i + 1, all.len(), client.requests()));
        }
    }
    all.retain(|s| tickers.contains(&s.ticker));
    Ok(all)
}

/// The table from the Alpaca rows: the rules of `splits_rules` for each ticker (the SEC facts come from `cache`).
pub fn build(cache: &Path, tickers: &[String], alpaca: &[Split], start: &str, log: &mut dyn FnMut(&str)) -> (Vec<SplitRow>, Vec<Event>) {
    let corrections = builtin_corrections();
    let (mut rows, mut events) = (Vec::new(), Vec::new());
    for (i, t) in tickers.iter().enumerate() {
        let sec = read_company_facts(cache, t).map(|v| sec_data_from_json(&v)).unwrap_or_default();
        let own: Vec<Split> = alpaca.iter().filter(|s| &s.ticker == t).cloned().collect();
        let last = super::price_cache::last_close(cache, t).map(|l| l.date);
        let (mut r, mut e) = rules_for_ticker(t, &own, &sec, last.as_deref(), start, &corrections);
        rows.append(&mut r);
        events.append(&mut e);
        if (i + 1) % 500 == 0 {
            log(&format!("rules: {}/{} tickers", i + 1, tickers.len()));
        }
    }
    splits::sort_dedupe_rows(&mut rows);
    (rows, events)
}

/// The facts of a run for the record: the window (start, end), the run date, and the counts (requests, Alpaca rows).
pub struct RunInfo<'a> {
    pub start: &'a str,
    pub end: &'a str,
    pub run_date: &'a str,
    pub requests: usize,
    pub alpaca_rows: usize,
}

pub fn meta_json(tickers: &[String], run: &RunInfo, rows: &[SplitRow], events: &[Event]) -> serde_json::Value {
    let mut counts: BTreeMap<&str, usize> = ["match", "no_sec_fact", "mismatch"].into_iter().map(|k| (k, 0)).collect();
    let mut by_source: BTreeMap<&str, usize> = BTreeMap::new();
    for r in rows {
        *counts.entry(r.split.sec_check.as_str()).or_insert(0) += 1;
        *by_source.entry(r.source.as_str()).or_insert(0) += 1;
    }
    let n = |w: &str| events.iter().filter(|e| e.what == w).count();
    serde_json::json!({
        "source": "Alpaca corporate actions (GET /v1/corporate-actions, types forward_split,reverse_split), cleaned by the rules of splits_rules.rs against the SEC cover share count (dei:EntityCommonStockSharesOutstanding) and the tag us-gaap StockholdersEquityNoteStockSplitConversionRatio1",
        "start": run.start, "end": run.end, "run_date": run.run_date, "requests": run.requests,
        "ticker_count": tickers.len(), "tickers": tickers, "alpaca_rows": run.alpaca_rows, "rows": rows.len(),
        "sec_check_counts": counts, "source_counts": by_source,
        "dropped": n("dropped"), "added": n("added"), "corrected": n("corrected"), "correction_file": n("file"),
        "pending": rows.iter().filter(|r| r.status == "pending").count(),
    })
}

/// Write the CSV and the record.
pub fn write_files(cache: &Path, rows: &[SplitRow], meta: &serde_json::Value) -> Result<(), String> {
    let dir = cache.join("_corporate");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    write_atomic(&dir.join("splits.csv"), &splits::to_csv(rows))?;
    write_atomic(&dir.join("splits.meta.json"), &(serde_json::to_string_pretty(meta).map_err(|e| e.to_string())? + "\n"))
}

fn get(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let mut p = parse(rest, &["--tickers-file", "--start", "--end", "--cache"], &[])?;
    collect_tickers(&mut p)?;
    let (start, end) = (p.flag("--start").unwrap_or("2009-01-01").to_string(), p.flag("--end").map(String::from).unwrap_or_else(today_iso));
    if crate::trading::calendar::parse_iso(&start).is_none() || crate::trading::calendar::parse_iso(&end).is_none() || start > end {
        return Err(format!("bad range {start} to {end}"));
    }
    let cache = p.flag("--cache").map(std::path::PathBuf::from).unwrap_or_else(|| super::price_cli::cache_dir(repo));
    let keys = Keys::read(repo).ok_or_else(|| format!("{KEY_ID_ENV} and {SECRET_ENV} are not both set in the environment or in .env"))?;
    let c = client(keys);
    let mut tickers = p.tickers.clone();
    tickers.sort();
    eprintln!("fa: splits of {} ticker(s), {start} to {end}, into {}", tickers.len(), cache.join("_corporate").display());
    let started = Instant::now();
    let mut alpaca = fetch(&c, &tickers, &start, &end, Duration::from_secs(5), &mut |m| eprintln!("fa: {m}"))?;
    splits::sort_dedupe(&mut alpaca);
    let (rows, events) = build(&cache, &tickers, &alpaca, &start, &mut |m| eprintln!("fa: {m}"));
    for e in &events {
        eprintln!("rule: {} {} {} factor {} {}: {}", e.what, e.ticker, e.ex_date, e.factor, if e.what == "added" { "(estimated ex date)" } else { "" }, e.why);
    }
    let meta = meta_json(&tickers, &RunInfo { start: &start, end: &end, run_date: &today_iso(), requests: c.requests(), alpaca_rows: alpaca.len() }, &rows, &events);
    write_files(&cache, &rows, &meta)?;
    println!(
        "splits: {} Alpaca rows, {} kept, {} dropped, {} corrected, {} added from cover steps, {} pending; {} requests, {:.0} s; sec_check {}",
        alpaca.len(), rows.len(), meta["dropped"], meta["corrected"], meta["added"], meta["pending"], c.requests(), started.elapsed().as_secs_f64(), meta["sec_check_counts"]
    );
    Ok(0)
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(0)
        }
        Some("get") => get(repo, &args[1..]),
        Some(other) => Err(format!("unknown splits command '{other}'; see fa splits --help")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn pages_batches_and_one_retry() {
        let urls: Rc<RefCell<Vec<String>>> = Rc::default();
        let u = urls.clone();
        let get: Get = Box::new(move |url| {
            u.borrow_mut().push(url.to_string());
            let n = u.borrow().len();
            let body = match n {
                1 => Err("connection reset".to_string()),
                2 => Ok(r#"{"corporate_actions":{"forward_splits":[{"symbol":"AAA","ex_date":"2020-01-02","old_rate":1,"new_rate":2}]},"next_page_token":"T1"}"#),
                3 => Ok(r#"{"corporate_actions":{"reverse_splits":[{"symbol":"AAA","ex_date":"2022-01-03","old_rate":10,"new_rate":1}]},"next_page_token":null}"#),
                _ => Ok(r#"{"corporate_actions":null}"#),
            };
            body.map(|b| b.as_bytes().to_vec())
        });
        let c = Client::new(Keys { id: "id-1234".into(), secret: "secret-1234".into() }, get);
        let tickers: Vec<String> = ["AAA"].iter().map(|s| s.to_string()).chain((0..BATCH).map(|i| format!("Z{i}"))).collect();
        let rows = fetch(&c, &tickers, "2009-01-01", "2026-11-21", Duration::ZERO, &mut |_| {}).unwrap();
        assert_eq!(rows.len(), 2);
        let u = urls.borrow();
        assert_eq!(u.len(), 4, "retry, 2 pages, second batch");
        assert!(u[2].contains("page_token=T1") && u[0].contains("types=forward_split,reverse_split") && u[0].contains("limit=1000"));
        assert!(!u[3].contains("AAA"));
    }

    #[test]
    fn two_errors_stop() {
        let get: Get = Box::new(|_| Err("down".to_string()));
        let c = Client::new(Keys { id: "id-1234".into(), secret: "secret-1234".into() }, get);
        assert!(fetch(&c, &["AAA".to_string()], "2009-01-01", "2026-11-21", Duration::ZERO, &mut |_| {}).unwrap_err().contains("down"));
        assert_eq!(c.requests(), 2);
    }

    #[test]
    fn a_429_stops_the_run() {
        let get: Get = Box::new(|_| Err("HTTP 429 for https://data.alpaca.markets/v1/corporate-actions".to_string()));
        let c = Client::new(Keys { id: "id-1234".into(), secret: "secret-1234".into() }, get);
        let tickers: Vec<String> = (0..BATCH + 1).map(|i| format!("Z{i}")).collect();
        let e = fetch(&c, &tickers, "2009-01-01", "2026-11-21", Duration::ZERO, &mut |_| {}).unwrap_err();
        assert!(e.contains("refused"), "{e}");
        assert_eq!(c.requests(), 1, "no retry and no second batch");
    }

    #[test]
    fn meta_counts() {
        let sp = Split { ticker: "A".into(), ex_date: "2020-01-02".into(), old_rate: 1.0, new_rate: 2.0, factor: 2.0, kind: "forward".into(), sec_check: "match".into() };
        let rows = vec![SplitRow::new(sp, splits::SRC_MATCH, "pending", false)];
        let m = meta_json(&["A".to_string()], &RunInfo { start: "2009-01-01", end: "2026-11-21", run_date: "2026-11-21", requests: 1, alpaca_rows: 3 }, &rows, &[]);
        assert_eq!(m["sec_check_counts"]["match"], 1);
        assert_eq!(m["sec_check_counts"]["mismatch"], 0);
        assert_eq!((m["pending"].as_u64(), m["alpaca_rows"].as_u64()), (Some(1), Some(3)));
    }
}
