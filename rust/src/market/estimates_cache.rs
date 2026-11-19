//! The cache of the Nasdaq consensus and the earnings surprises, and the update of one ticker.
//!
//! Files of a ticker under `data/raw/<T>/`:
//!
//! - `estimates/<date>.json`: the dated snapshot (`consensus::ConsensusEstimates`, `source: nasdaq`). A free source gives
//!   the consensus of today only, so each day is a new file; the files in the sequence are the history of the consensus
//!   (the revisions). A snapshot of the Python code for the same date (it has no `source`) is never replaced.
//! - `earnings_surprises.json`: the report dates, the EPS and the consensus of the reported quarters. The source shows
//!   the last 4 quarters; the rows add up over the runs.
//! - `earnings_dates.json`: see `earnings_dates`.
//!
//! A source that fails never deletes cache data: a failed endpoint leaves its file as it was.

use super::consensus::{self, ConsensusEstimates};
use super::earnings_dates::{self, PastDate};
use super::fmp::EarningsSurprise;
use super::nasdaq_est::{self as nq, Client, Endpoint, SourceError, SurpriseRow};
use super::{round6, ticker_dir, write_atomic};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurprisesFile {
    pub ticker: String,
    pub source: String,
    /// The date of the last successful request.
    pub fetched: String,
    pub rows: Vec<SurpriseRow>,
}

pub fn surprises_path(cache: &Path, ticker: &str) -> PathBuf {
    ticker_dir(cache, ticker).join("earnings_surprises.json")
}

pub fn load_surprises(cache: &Path, ticker: &str) -> Option<SurprisesFile> {
    let text = std::fs::read_to_string(surprises_path(cache, ticker)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Add new rows to the old rows: a date that exists gets the new row. Newest first.
pub fn merge_surprises(old: &[SurpriseRow], new: &[SurpriseRow]) -> Vec<SurpriseRow> {
    let mut rows: Vec<SurpriseRow> = new.to_vec();
    rows.extend(old.iter().filter(|o| !new.iter().any(|n| n.date == o.date)).cloned());
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    rows
}

/// The rows of the cache as the sheet and the report read them: the next report first (no actual result: NaN),
/// then the reported quarters, newest first. A quarter with no EPS or no consensus has NaN in these fields.
/// `surprise_pct` is a ratio (0.0382), as the older files have it; it is the surprise over the absolute consensus.
pub fn surprises_for_sheet(cache: &Path, ticker: &str, today: &str) -> Option<Vec<EarningsSurprise>> {
    let file = load_surprises(cache, ticker)?;
    let nan = f64::NAN;
    let mut out = Vec::new();
    if let Some(next) = earnings_dates::load_dates(cache, ticker).and_then(|d| d.next).filter(|n| n.date.as_str() >= today) {
        out.push(EarningsSurprise { date: next.date, actual_eps: nan, estimated_eps: next.eps_forecast.unwrap_or(nan), surprise: nan, surprise_pct: nan });
    }
    for r in &file.rows {
        let (actual, est) = (r.eps.unwrap_or(nan), r.consensus.unwrap_or(nan));
        let surprise = actual - est;
        let pct = if est.is_finite() && est != 0.0 { surprise / est.abs() } else { nan };
        out.push(EarningsSurprise { date: r.date.clone(), actual_eps: actual, estimated_eps: est, surprise: round6(surprise), surprise_pct: round6(pct) });
    }
    Some(out)
}

#[derive(Debug, Clone)]
pub struct Options {
    pub today: String,
    /// Ask again for the endpoints that today's files already have.
    pub refresh: bool,
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    /// Today's files have all the data that was asked for. No request.
    Current(String),
    Updated(String),
    /// The source answered and has no estimates for this ticker.
    NoData(String),
    Failed(String),
    /// The source stopped (HTTP 429, HTTP 403 or a block page, two times). The run stops.
    Blocked(String),
}

/// Today's snapshot of the source, a new empty one, or `None` when a snapshot of another source has today's file.
fn working_snapshot(cache: &Path, t: &str, today: &str) -> Option<ConsensusEstimates> {
    match consensus::load_snapshot(cache, t, Some(today)) {
        Ok(s) if s.source.as_deref() == Some(nq::SOURCE) => Some(s),
        Ok(_) => None,
        Err(_) => Some(ConsensusEstimates::empty(t, today, nq::SOURCE)),
    }
}

/// Get the endpoints of one ticker that the cache does not have for today, and write the files.
/// One request for each endpoint. A symbol that the source does not know ends the ticker after the first request.
pub fn update_ticker(cache: &Path, ticker: &str, opt: &Options, client: &mut Client) -> Status {
    let t = ticker.to_uppercase();
    let snapshot = working_snapshot(cache, &t, &opt.today);
    let surprise_current = !opt.refresh && load_surprises(cache, &t).is_some_and(|s| s.fetched == opt.today);
    let needed: Vec<Endpoint> = opt
        .endpoints
        .iter()
        .copied()
        .filter(|e| match e {
            Endpoint::Surprise => !surprise_current,
            // A snapshot of another source for today stays as it is.
            _ => snapshot.as_ref().is_some_and(|s| opt.refresh || !nq::has_endpoint(s, *e)),
        })
        .collect();
    if needed.is_empty() {
        let note = if snapshot.is_none() { "a snapshot of another source has today's file; it stays" } else { "today's files are current" };
        return Status::Current(note.to_string());
    }

    let mut snap = snapshot.unwrap_or_else(|| ConsensusEstimates::empty(&t, &opt.today, nq::SOURCE));
    let (mut notes, mut errors, mut blocked, mut touched_snapshot) = (Vec::<String>::new(), Vec::<String>::new(), None, false);
    let (mut any_data, mut any_empty) = (false, false);
    for ep in &needed {
        let url = ep.url(&t);
        let result: Result<(), SourceError> = match ep {
            Endpoint::Forecast => nq::fetch(client, &url, nq::parse_forecast).map(|f| {
                if f.is_empty() {
                    any_empty = true;
                } else {
                    any_data = true;
                    touched_snapshot = true;
                    nq::apply_forecast(&mut snap, &f);
                    notes.push(format!("forecast {}q+{}y", f.quarterly.len(), f.yearly.len()));
                }
            }),
            Endpoint::Targets => nq::fetch(client, &url, nq::parse_targets).map(|x| {
                if x.is_empty() {
                    any_empty = true;
                } else {
                    any_data = true;
                    touched_snapshot = true;
                    nq::apply_targets(&mut snap, &x);
                    notes.push("targets".to_string());
                }
            }),
            Endpoint::Momentum => nq::fetch(client, &url, nq::parse_momentum).map(|m| {
                touched_snapshot = true;
                nq::apply_momentum(&mut snap, &m);
                notes.push("momentum".to_string());
            }),
            Endpoint::Surprise => nq::fetch(client, &url, nq::parse_surprises).and_then(|rows| {
                if rows.is_empty() {
                    any_empty = true;
                    return Ok(());
                }
                any_data = true;
                let merged = merge_surprises(&load_surprises(cache, &t).map(|f| f.rows).unwrap_or_default(), &rows);
                let file = SurprisesFile { ticker: t.clone(), source: nq::SOURCE.to_string(), fetched: opt.today.clone(), rows: merged };
                let text = serde_json::to_string_pretty(&file).map_err(|e| SourceError::Failed(e.to_string()))?;
                write_atomic(&surprises_path(cache, &t), &text).map_err(SourceError::Failed)?;
                let past: Vec<PastDate> = rows.iter().map(|r| PastDate { date: r.date.clone(), fiscal_qtr_end: r.fiscal_qtr_end.clone(), source: nq::SOURCE.to_string() }).collect();
                earnings_dates::add_past(cache, &t, &opt.today, &past).map_err(SourceError::Failed)?;
                notes.push(format!("{} surprises", rows.len()));
                Ok(())
            }),
        };
        match result {
            Ok(()) => {}
            Err(SourceError::Blocked(why)) => {
                blocked = Some(why);
                break;
            }
            Err(SourceError::Failed(why)) => {
                let unknown = why.contains("Symbol not exists");
                errors.push(format!("{}: {why}", ep.name()));
                if unknown {
                    break;
                }
            }
        }
    }
    if touched_snapshot {
        if let Err(e) = consensus::save_snapshot(&snap, cache) {
            errors.push(e);
        }
    }
    let said = notes.join(", ");
    if let Some(why) = blocked {
        return Status::Blocked(format!("{why}{}", if said.is_empty() { String::new() } else { format!(" (saved: {said})") }));
    }
    if !errors.is_empty() && !any_data {
        return Status::Failed(errors.join("; "));
    }
    if !any_data && any_empty {
        return Status::NoData("the source has no estimates for this ticker".to_string());
    }
    let tail = if errors.is_empty() { String::new() } else { format!("; failed: {}", errors.join("; ")) };
    Status::Updated(format!("{said}{tail}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::time::Duration;

    const FORECAST: &str = include_str!("../../tests/fixtures/market/nasdaq/forecast_MSFT.json");
    const SURPRISE: &str = include_str!("../../tests/fixtures/market/nasdaq/surprise_MSFT.json");
    const TARGETS: &str = include_str!("../../tests/fixtures/market/nasdaq/targetprice_MSFT.json");
    const UNAVAILABLE: &str = include_str!("../../tests/fixtures/market/nasdaq/forecast_unavailable.json");
    const NO_RECORD: &str = include_str!("../../tests/fixtures/market/nasdaq/no_record.json");
    const BAD_SYMBOL: &str = include_str!("../../tests/fixtures/market/nasdaq/error_symbol.json");

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fa-estcache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn opt(today: &str, refresh: bool) -> Options {
        Options { today: today.into(), refresh, endpoints: Endpoint::DEFAULT.to_vec() }
    }

    fn reply(url: &str) -> Result<Vec<u8>, String> {
        let body = if url.contains("earnings-forecast") {
            FORECAST
        } else if url.contains("earnings-surprise") {
            SURPRISE
        } else if url.contains("targetprice") {
            TARGETS
        } else {
            return Err(format!("HTTP 404 for {url}"));
        };
        Ok(body.as_bytes().to_vec())
    }

    #[test]
    fn one_run_writes_the_snapshot_the_surprises_and_the_dates() {
        let cache = dir("run");
        let urls = RefCell::new(Vec::<String>::new());
        let get = |u: &str| {
            urls.borrow_mut().push(u.to_string());
            reply(u)
        };
        let mut client = Client::new(&get, Duration::ZERO);
        let st = update_ticker(&cache, "msft", &opt("2026-11-19", false), &mut client);
        assert!(matches!(&st, Status::Updated(n) if n.contains("forecast 5q+4y") && n.contains("4 surprises")), "{st:?}");
        assert_eq!(client.requests, 3, "three necessary endpoints, one request each");
        let s = consensus::load_snapshot(&cache, "MSFT", Some("2026-11-19")).unwrap();
        assert_eq!((s.source.as_deref(), s.eps("0y", "avg"), s.analyst_price_targets.0[0].1), (Some("nasdaq"), Some(19.91), Some(632.48)));
        assert_eq!(s.endpoints, ["forecast", "targets"]);
        let sf = load_surprises(&cache, "MSFT").unwrap();
        assert_eq!((sf.rows.len(), sf.rows[0].date.as_str(), sf.fetched.as_str()), (4, "2026-10-28", "2026-11-19"));
        assert_eq!(earnings_dates::load_dates(&cache, "MSFT").unwrap().past.len(), 4);
        // The same day again: no request.
        let mut client = Client::new(&get, Duration::ZERO);
        assert!(matches!(update_ticker(&cache, "MSFT", &opt("2026-11-19", false), &mut client), Status::Current(_)));
        assert_eq!(client.requests, 0);
        // --refresh asks again; the next day is a new file and the old file stays: the history of the consensus.
        let mut client = Client::new(&get, Duration::ZERO);
        update_ticker(&cache, "MSFT", &opt("2026-11-19", true), &mut client);
        assert_eq!(client.requests, 3);
        let mut client = Client::new(&get, Duration::ZERO);
        update_ticker(&cache, "MSFT", &opt("2026-11-20", false), &mut client);
        assert_eq!(consensus::list_snapshots(&cache, "MSFT"), ["2026-11-19", "2026-11-20"]);
        // An extra endpoint on the same day asks for that endpoint only.
        let mut o = opt("2026-11-20", false);
        o.endpoints.push(Endpoint::Momentum);
        let momentum = |u: &str| -> Result<Vec<u8>, String> {
            assert!(u.contains("estimate-momentum"), "{u}");
            Ok(include_bytes!("../../tests/fixtures/market/nasdaq/momentum_MSFT.json").to_vec())
        };
        let mut client = Client::new(&momentum, Duration::ZERO);
        update_ticker(&cache, "MSFT", &o, &mut client);
        assert_eq!(client.requests, 1);
        let s = consensus::load_snapshot(&cache, "MSFT", Some("2026-11-20")).unwrap();
        assert_eq!(s.endpoints, ["forecast", "targets", "momentum"]);
        assert_eq!(s.consensus_change.0.len(), 6);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn a_snapshot_of_the_python_code_is_never_replaced() {
        let cache = dir("legacy");
        let legacy = include_str!("../../tests/fixtures/market/consensus_IBM.json");
        let mut s: ConsensusEstimates = serde_json::from_str(legacy).unwrap();
        s.snapshot_date = "2026-11-19".into();
        consensus::save_snapshot(&s, &cache).unwrap();
        let before = std::fs::read_to_string(consensus::snapshot_path(&cache, "IBM", "2026-11-19")).unwrap();
        let get = |u: &str| reply(u);
        let mut client = Client::new(&get, Duration::ZERO);
        let mut o = opt("2026-11-19", true);
        o.endpoints = vec![Endpoint::Forecast, Endpoint::Targets];
        assert!(matches!(update_ticker(&cache, "IBM", &o, &mut client), Status::Current(n) if n.contains("another source")));
        assert_eq!(client.requests, 0);
        assert_eq!(std::fs::read_to_string(consensus::snapshot_path(&cache, "IBM", "2026-11-19")).unwrap(), before);
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn no_data_unknown_symbol_and_block() {
        let cache = dir("edge");
        // A listing with no coverage (a new listing): HTTP 200, null tables, code 1002.
        let none = |u: &str| -> Result<Vec<u8>, String> { Ok((if u.contains("earnings-forecast") { UNAVAILABLE } else { NO_RECORD }).as_bytes().to_vec()) };
        let mut client = Client::new(&none, Duration::ZERO);
        let st = update_ticker(&cache, "ANT", &opt("2026-11-19", false), &mut client);
        assert!(matches!(st, Status::NoData(_)), "{st:?}");
        assert!(!cache.join("ANT").exists(), "no file for a ticker with no data");
        // An unknown symbol ends the ticker after one request.
        let bad = |_: &str| -> Result<Vec<u8>, String> { Ok(BAD_SYMBOL.as_bytes().to_vec()) };
        let mut client = Client::new(&bad, Duration::ZERO);
        let st = update_ticker(&cache, "BRK-B", &opt("2026-11-19", false), &mut client);
        assert!(matches!(&st, Status::Failed(m) if m.contains("Symbol not exists")), "{st:?}");
        assert_eq!(client.requests, 1);
        // A block after the first endpoint: what was saved stays, the status is Blocked.
        let calls = std::cell::Cell::new(0);
        let flaky = |u: &str| -> Result<Vec<u8>, String> {
            calls.set(calls.get() + 1);
            if calls.get() == 1 { reply(u) } else { Err("HTTP 429 for x".into()) }
        };
        let mut client = Client::new(&flaky, Duration::ZERO);
        let st = update_ticker(&cache, "MSFT", &opt("2026-11-19", false), &mut client);
        assert!(matches!(&st, Status::Blocked(m) if m.contains("saved: forecast")), "{st:?}");
        let s = consensus::load_snapshot(&cache, "MSFT", Some("2026-11-19")).unwrap();
        assert_eq!(s.endpoints, ["forecast"]);
        assert!(load_surprises(&cache, "MSFT").is_none());
        let _ = std::fs::remove_dir_all(&cache);
    }

    #[test]
    fn surprise_rows_add_up_and_the_sheet_rows_have_the_old_form() {
        let cache = dir("sheet");
        let row = |d: &str, eps: f64, c: f64| SurpriseRow { fiscal_qtr_end: "2026-09".into(), date: d.into(), eps: Some(eps), consensus: Some(c), pct: None };
        let old = vec![row("2026-07-29", 4.74, 4.21), row("2026-04-29", 4.27, 4.07)];
        let merged = merge_surprises(&old, &[row("2026-10-28", 4.89, 4.71), row("2026-07-29", 4.74, 4.22)]);
        assert_eq!(merged.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(), ["2026-10-28", "2026-07-29", "2026-04-29"]);
        assert_eq!(merged[1].consensus, Some(4.22), "the new row of a date replaces the old row");
        write_atomic(&surprises_path(&cache, "MSFT"), &serde_json::to_string(&SurprisesFile { ticker: "MSFT".into(), source: "nasdaq".into(), fetched: "2026-11-19".into(), rows: merged }).unwrap()).unwrap();
        earnings_dates::set_next(&cache, "MSFT", "2026-11-19", Some(earnings_dates::NextDate { date: "2027-01-27".into(), time: "after-hours".into(), fiscal_quarter: "2026-12".into(), eps_forecast: Some(4.86), estimates: Some(14.0), source: "x".into() }), "2027-02-01").unwrap();
        let rows = surprises_for_sheet(&cache, "MSFT", "2026-11-19").unwrap();
        assert_eq!(rows.len(), 4);
        // The next report first, with no actual result, as in the file of the Python code.
        assert!(rows[0].actual_eps.is_nan() && rows[0].surprise.is_nan() && rows[0].estimated_eps == 4.86 && rows[0].date == "2027-01-27");
        assert_eq!((rows[1].date.as_str(), rows[1].surprise, rows[1].surprise_pct), ("2026-10-28", 0.18, 0.038217));
        // A date in the past is not a next date.
        assert_eq!(surprises_for_sheet(&cache, "MSFT", "2027-02-01").unwrap().len(), 3);
        assert!(surprises_for_sheet(&cache, "NONE", "2026-11-19").is_none());
        let _ = std::fs::remove_dir_all(&cache);
    }
}
