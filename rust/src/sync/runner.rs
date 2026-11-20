//! The real runner of `fa sync`: for each item it calls the existing function of the data command. It does not copy their
//! logic: the cache rules (no mixing of two price sources, the overlap check of the prices, the files of one day) stay in
//! the functions that had them.
//!
//! A block (HTTP 429, 403, a block page) is seen by the request queues (`hostnet`): the host is stopped for the rest of the
//! run and the next request to it makes no network call. This module turns the error texts into the statuses of the
//! executor.

use super::dataset::DataSet;
use super::exec::{JobOutcome, Runner, Status};
use super::fresh::{earnings_sec_state, file_date, Ctx};
use super::config::SyncConfig;
use super::ledger::Ledger;
use super::plan::{last_date_of_file, KeyInfo, PlanItem, CALENDAR_DAYS, CALENDAR_KEY, DEFAULT_INTRADAY_DAYS};
use crate::hostnet;
use crate::market::{self, alpaca, earnings_dates, estimates_cache, intraday, nasdaq_est, options, options_cli, price_cache, price_source};
use mapcheck::http::Client as SecClient;
use mapcheck::registry::Registry;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

pub const VIX_HISTORY_URL: &str = "https://cdn.cboe.com/api/global/us_indices/daily_prices/VIX_History.csv";
pub const VIX_HEADER: &str = "DATE,OPEN,HIGH,LOW,CLOSE";
/// The pause of the SEC client between two requests (10 requests each second are the limit of the SEC).
pub const SEC_DELAY_SECONDS: f64 = 0.15;

pub struct RealRunner {
    pub ctx: Ctx,
    pub cfg: SyncConfig,
    pub keys: KeyInfo,
    pub force: bool,
    /// All tickers of the run (the calendar scan sets the next report date of each of them).
    pub tickers: Vec<String>,
    ledger: Mutex<Ledger>,
    sec: Mutex<Option<SecClient>>,
    registry: Mutex<Option<Registry>>,
}

/// The host group in the text of an error (the first URL in it).
fn group_in(text: &str, fallback: &str) -> String {
    text.find("https://").or_else(|| text.find("http://")).map(|i| {
        let url: String = text[i..].chars().take_while(|c| !c.is_whitespace() && *c != ')' && *c != '"').collect();
        hostnet::group_of_url(&url)
    })
    .unwrap_or_else(|| fallback.to_string())
}

/// Update the prices of one ticker with the stop of the whole run. A source that the run already stopped (`hostnet::blocked`)
/// is not asked and gives no log line. A source that answers HTTP 429 or 403 in this call stops the host for the run.
/// The run does not get around the limit: no other address, no faster try.
/// The stop of the run is the global list of `hostnet`, so the parallel jobs share it with no lock around the requests.
/// Limits: the stop is stored with code 429 also after a 403 (the log line of `price_cache::fetch` gives the real code);
/// the stop is published when `update_ticker` returns, so parallel jobs of one source can each make their 2 requests
/// first (the thread count bounds this).
pub(crate) fn update_prices(cache: &std::path::Path, ticker: &str, sources: &[Box<dyn price_source::PriceSource>], opt: &price_cache::Options) -> price_cache::Status {
    let mut run = price_cache::Run::default();
    for s in sources {
        if hostnet::blocked(s.name()).is_some() {
            run.blocked.insert(s.name().to_string());
        }
    }
    let status = price_cache::update_ticker(cache, ticker, sources, opt, &mut run);
    for name in run.blocked.iter() {
        if hostnet::blocked(name).is_none() {
            hostnet::block(name, 429, &format!("HTTP 429 or 403 from the price source {name} (stopped for this run)"));
        }
    }
    status
}

fn modified(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl RealRunner {
    pub fn new(ctx: Ctx, cfg: SyncConfig, keys: KeyInfo, force: bool, tickers: Vec<String>, ledger: Ledger) -> RealRunner {
        RealRunner { ctx, cfg, keys, force, tickers, ledger: Mutex::new(ledger), sec: Mutex::new(None), registry: Mutex::new(None) }
    }

    pub fn ledger_snapshot(&self) -> Ledger {
        self.ledger.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn mark(&self, set: DataSet, key: &str) {
        let mut l = self.ledger.lock().unwrap_or_else(|e| e.into_inner());
        l.set(set.name(), key, &self.ctx.today_text());
        let _ = l.save();
    }

    /// An error that came from the network: a block, a stop, or a failure.
    fn fail(&self, item: &PlanItem, err: &str) -> JobOutcome {
        let text = hostnet::block_code(err).map(|_| err.to_string()).unwrap_or_else(|| err.to_string());
        if text.starts_with("stopped") {
            return JobOutcome::new(Status::Failed, text);
        }
        if let Some(code) = hostnet::block_code(err) {
            let group = group_in(err, &item.hosts[0]);
            hostnet::block(&group, code, err);
        }
        let stopped: Vec<&String> = item.hosts.iter().filter(|h| hostnet::blocked(h).is_some()).collect();
        if stopped.len() == item.hosts.len() {
            let (_, why) = hostnet::blocked(&item.hosts[0]).unwrap_or((0, String::new()));
            return JobOutcome::new(Status::Blocked, format!("{}: {}", item.hosts[0], short(&why)));
        }
        JobOutcome::new(Status::Failed, short(&text))
    }

    /// The item is current: the answer when a cache file that the function would use is young.
    fn done(&self, item: &PlanItem, status: Status, note: impl Into<String>, requests: usize) -> JobOutcome {
        if matches!(status, Status::Updated | Status::Current | Status::NoData) {
            self.mark(item.set, &item.key);
        }
        JobOutcome::new(status, note).requests(requests)
    }

    fn with_sec<R>(&self, f: impl FnOnce(&mut SecClient) -> Result<R, String>) -> Result<R, String> {
        let mut guard = self.sec.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            *guard = Some(SecClient::sec(SEC_DELAY_SECONDS)?);
        }
        f(guard.as_mut().expect("client"))
    }

    // ------------------------------------------------------------ SEC

    /// The look at the filing list of the SEC (one request). `Ok(false)` when the record of today is there.
    fn look_at_filing_list(&self, ticker: &str) -> Result<bool, String> {
        let state = {
            let l = self.ledger.lock().unwrap_or_else(|e| e.into_inner());
            earnings_sec_state(&self.ctx, &l, &self.cfg, ticker)
        };
        if !self.force && state.is_current() {
            return Ok(false);
        }
        let cache = self.ctx.cache.clone();
        let t = ticker.to_string();
        self.with_sec(|c| mapcheck::sec::fetch_earnings_filings_with(c, &cache, &t).map(|_| ()))?;
        hostnet::note_requests("sec", 1);
        Ok(true)
    }

    fn cik_of(&self, ticker: &str) -> Option<String> {
        let text = std::fs::read_to_string(mapcheck::sec::earnings_filings_path(&self.ctx.cache, ticker)).ok()?;
        let v: serde_json::Value = serde_json::from_str(&text).ok()?;
        match v.get("cik")? {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }

    fn job_sec(&self, item: &PlanItem) -> JobOutcome {
        let t = item.key.as_str();
        let mut requests = 0;
        match self.look_at_filing_list(t) {
            Ok(asked) => requests += usize::from(asked),
            Err(e) => {
                if e.contains("is not in the SEC ticker list") || e.contains("HTTP 404") {
                    return self.done(item, Status::NoData, "the SEC has no filings for this symbol (an ETF, a fund or a foreign filer)", requests);
                }
                return self.fail(item, &e);
            }
        }
        // The facts: missing, or a filing of the list that is newer than the cached facts.
        let facts = mapcheck::run::facts_path(&self.ctx.cache, t);
        let facts_day = facts.as_deref().and_then(file_date);
        let latest = mapcheck::sec::load_filing_list_check(&self.ctx.cache, t).and_then(|k| k.latest_periodic);
        let need = self.force || facts.is_none() || matches!((&latest, &facts_day), (Some(f), Some(d)) if f >= d);
        if !need {
            return self.done(item, Status::Current, "the filing list shows nothing newer than the cached facts", requests);
        }
        let Some(cik) = self.cik_of(t) else {
            return JobOutcome::new(Status::Failed, format!("{t}: no CIK in the filing list file")).requests(requests);
        };
        let cache = self.ctx.cache.clone();
        let tt = t.to_string();
        match self.with_sec(|c| mapcheck::sec::fetch_facts(c, &cache, &tt, &cik)) {
            Ok(_) => {
                requests += 1;
                hostnet::note_requests("sec", 1);
            }
            Err(e) => {
                if e.contains("HTTP 404") {
                    return self.done(item, Status::NoData, "the SEC has no company facts for this symbol", requests);
                }
                return self.fail(item, &e);
            }
        }
        // The store file of the valuation code (no network: the facts come from the cache that was just written).
        let store_note = {
            let mut reg = self.registry.lock().unwrap_or_else(|e| e.into_inner());
            if reg.is_none() {
                *reg = Registry::embedded().ok();
            }
            match reg.as_ref() {
                Some(r) => {
                    let store_dir = self.ctx.repo.join("data").join("processed");
                    match mapcheck::import::import_ticker(r, &self.ctx.cache, &store_dir, t, 12, SEC_DELAY_SECONDS, false) {
                        Ok(_) => "store written".to_string(),
                        Err(mapcheck::import::ImportError::Unsupported(_)) => "store: not a us-gaap filer".to_string(),
                        Err(e) => format!("store failed: {e}"),
                    }
                }
                None => "store not written (no registry)".to_string(),
            }
        };
        self.done(item, Status::Updated, format!("company facts; {store_note}"), requests)
    }

    fn job_filings(&self, item: &PlanItem) -> JobOutcome {
        let t = item.key.as_str();
        let dir = self.ctx.cache.join(t).join("filings");
        let files_before = count_files(&dir);
        match mapcheck::sec::fetch_filing(&self.ctx.cache, t, SEC_DELAY_SECONDS, self.force) {
            Ok(folder) => {
                let requests = if count_files(&dir) != files_before || self.force { 3 } else { 0 };
                hostnet::note_requests("sec", requests);
                let name = folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                self.done(item, if requests == 0 { Status::Current } else { Status::Updated }, format!("10-K {name}"), requests)
            }
            Err(e) => {
                if e.contains("no company facts in the cache") {
                    return JobOutcome::new(Status::Failed, format!("{t}: no company facts yet (run: fa sync {t} --only sec)"));
                }
                self.fail(item, &e)
            }
        }
    }

    fn job_earnings_sec(&self, item: &PlanItem) -> JobOutcome {
        match self.look_at_filing_list(&item.key) {
            Ok(asked) => self.done(item, if asked { Status::Updated } else { Status::Current }, "SEC 8-K list (item 2.02)", usize::from(asked)),
            Err(e) if e.contains("is not in the SEC ticker list") || e.contains("HTTP 404") => self.done(item, Status::NoData, "no SEC filings for this symbol", 0),
            Err(e) => self.fail(item, &e),
        }
    }

    // ------------------------------------------------------------ prices

    fn job_prices(&self, item: &PlanItem) -> JobOutcome {
        let all = match price_source::sources(&self.ctx.repo, None) {
            Ok(a) => a,
            Err(e) => return JobOutcome::new(Status::Failed, e),
        };
        // The sources of the plan, in the order of the plan (the cache source alone, or the order of the existing code).
        let mut sources: Vec<Box<dyn price_source::PriceSource>> = Vec::new();
        let mut pool: Vec<Option<Box<dyn price_source::PriceSource>>> = all.into_iter().map(Some).collect();
        for h in &item.hosts {
            if let Some(slot) = pool.iter_mut().find(|s| s.as_ref().is_some_and(|b| b.name() == h)) {
                sources.push(slot.take().expect("source"));
            }
        }
        if sources.is_empty() {
            return JobOutcome::new(Status::Failed, "no price source is available");
        }
        let opt = price_cache::Options { today: self.ctx.today_text(), full: false, forced: false, retry_wait: Duration::ZERO, again: self.force };
        match update_prices(&self.ctx.cache, &item.key, &sources, &opt) {
            price_cache::Status::Current => self.done(item, Status::Current, "cache of today", 0),
            price_cache::Status::Updated { source, added, full, .. } => self.done(item, Status::Updated, format!("{source} {} +{added} rows", if full { "full history" } else { "update" }), 1),
            price_cache::Status::Failed(why) => {
                if why.contains("Symbol not exists") || why.contains("the source gave no rows") {
                    return self.done(item, Status::NoData, "the source has no prices for this symbol", 1);
                }
                self.fail(item, &why)
            }
        }
    }

    // ------------------------------------------------------------ intraday

    fn job_intraday(&self, item: &PlanItem) -> JobOutcome {
        let s = item.key.as_str();
        let dir = &self.ctx.trading;
        let file = dir.join(crate::trading::data::file_name(s, "5m"));
        let mut errors: Vec<String> = Vec::new();
        if let Some(keys) = alpaca::Keys::read(&self.ctx.repo) {
            let client = alpaca::Client::live(keys);
            let today = self.ctx.today_text();
            let from = last_date_of_file(&file).unwrap_or_else(|| mapcheck::dates::format(self.ctx.today - DEFAULT_INTRADAY_DAYS));
            match intraday::sync(&client, &file, s, 5, &from, &today, &alpaca::feed_from_env(), alpaca::now_utc(), &mut |_| {}) {
                Ok(st) => return self.done(item, if st.rows_added > 0 { Status::Updated } else { Status::Current }, format!("alpaca +{} rows", st.rows_added), client.requests()),
                Err(e) => {
                    if e.starts_with("stopped") {
                        return JobOutcome::new(Status::Failed, e);
                    }
                    if let Some(code) = hostnet::block_code(&e) {
                        hostnet::block("alpaca", code, &e);
                    }
                    errors.push(format!("alpaca: {e}"));
                }
            }
        }
        match crate::trading::fetch::refresh_file(dir, s, crate::trading::data::Interval::FiveMinute) {
            Ok(m) => self.done(item, if m.added > 0 { Status::Updated } else { Status::Current }, format!("yahoo +{} rows (60 days at most)", m.added), 1),
            Err(e) => {
                errors.push(format!("yahoo: {e}"));
                self.fail(item, &errors.join("; "))
            }
        }
    }

    // ------------------------------------------------------------ estimates and earnings

    fn job_estimates(&self, item: &PlanItem) -> JobOutcome {
        let opt = estimates_cache::Options { today: self.ctx.today_text(), refresh: self.force, endpoints: nasdaq_est::Endpoint::DEFAULT.to_vec() };
        let get = |url: &str| market::download(url);
        let mut client = nasdaq_est::Client::new(&get, Duration::ZERO);
        let status = estimates_cache::update_ticker(&self.ctx.cache, &item.key, &opt, &mut client);
        let requests = client.requests;
        match status {
            estimates_cache::Status::Current(n) => self.done(item, Status::Current, n, requests),
            estimates_cache::Status::Updated(n) => self.done(item, Status::Updated, n, requests),
            estimates_cache::Status::NoData(n) => self.done(item, Status::NoData, n, requests),
            estimates_cache::Status::Failed(w) => self.fail(item, &w).requests(requests),
            estimates_cache::Status::Blocked(w) => {
                hostnet::block("nasdaq", 429, &w);
                JobOutcome::new(Status::Blocked, format!("nasdaq: {}", short(&w))).requests(requests)
            }
        }
    }

    fn job_calendar(&self, _item: &PlanItem) -> JobOutcome {
        let today = self.ctx.today_text();
        let dates = earnings_dates::scan_dates(&today, CALENDAR_DAYS);
        let get = |url: &str| market::download(url);
        let mut client = nasdaq_est::Client::new(&get, Duration::ZERO);
        let st = earnings_dates::scan(&self.ctx.cache, &dates, &today, self.force, &mut client, &|_, _, _| {});
        let covered: Vec<String> = match &st.stopped_at {
            Some(stop) => dates.iter().take_while(|d| *d != stop).cloned().collect(),
            None => dates.clone(),
        };
        let next = earnings_dates::apply_calendar(&self.ctx.cache, &self.tickers, &covered, &today, &today);
        let found = next.iter().filter(|n| n.calendar.is_some()).count();
        let note = format!("{} of {} days, {} request(s), {found} of {} tickers have a date in the window", covered.len(), dates.len(), st.requests, self.tickers.len());
        if let Some(stop) = &st.stopped_at {
            hostnet::block("nasdaq", 429, client.blocked.as_deref().unwrap_or("blocked"));
            return JobOutcome::new(Status::Blocked, format!("stopped at {stop}: {}; {note}", short(client.blocked.as_deref().unwrap_or("blocked")))).requests(st.requests);
        }
        if !st.failed.is_empty() && st.failed.len() >= dates.len().saturating_sub(st.cached) && st.requests > 0 {
            return JobOutcome::new(Status::Failed, format!("{}: {}", st.failed[0].0, st.failed[0].1)).requests(st.requests);
        }
        for t in &self.tickers {
            let mut l = self.ledger.lock().unwrap_or_else(|e| e.into_inner());
            l.set("earnings", t, &today);
        }
        let _ = self.ledger.lock().unwrap_or_else(|e| e.into_inner()).save();
        JobOutcome::new(if st.requests == 0 { Status::Current } else { Status::Updated }, note).requests(st.requests)
    }

    // ------------------------------------------------------------ options

    fn job_options(&self, item: &PlanItem) -> JobOutcome {
        let today = self.ctx.today_text();
        let s = item.key.as_str();
        let first = options::fetch_option_chain(&self.ctx.cache, s, self.force, &today, &|u| market::download(u));
        match first {
            Ok((snap, options::Source::Cache)) => self.done(item, Status::Current, format!("snapshot of {today} ({} quotes)", snap.quotes.len()), 0),
            Ok((snap, options::Source::Cboe)) => self.done(item, Status::Updated, format!("cboe, {} quotes", snap.quotes.len()), 1),
            Err(e) => {
                if e.contains("stopped") {
                    return JobOutcome::new(Status::Failed, e);
                }
                if let Some(code) = hostnet::block_code(e.trim_start_matches("Cboe: ")) {
                    hostnet::block("cboe", code, &e);
                }
                if self.keys.alpaca {
                    match options_cli::snapshot_alpaca(&self.ctx.repo, &self.ctx.cache, s, &today, self.force) {
                        Ok(line) => return self.done(item, Status::Updated, format!("alpaca indicative feed (Cboe: {}); {}", short(&e), hostnet::plain(&line)), 1),
                        Err(e2) => return self.fail(item, &format!("{e}; alpaca: {e2}")),
                    }
                }
                self.fail(item, &e)
            }
        }
    }

    // ------------------------------------------------------------ rates, gsw, fred

    /// A function that falls back to the old file when the download fails: the job is good only when the file is new.
    fn refreshed(&self, item: &PlanItem, paths: &[PathBuf], before: &[Option<SystemTime>], what: &str) -> JobOutcome {
        let all_new = paths.iter().zip(before).all(|(p, b)| modified(p).is_some() && modified(p) != *b);
        if all_new {
            return self.done(item, Status::Updated, what, paths.len());
        }
        let host = &item.hosts[0];
        if let Some((_, why)) = hostnet::blocked(host) {
            return JobOutcome::new(Status::Blocked, format!("{host}: {}", short(&why)));
        }
        let why = hostnet::stats().get(host).and_then(|s| s.last_error.clone()).unwrap_or_else(|| "the download did not give a new file".into());
        JobOutcome::new(Status::Failed, format!("{}; the old file stays", short(&why)))
    }

    fn job_rates(&self, item: &PlanItem) -> JobOutcome {
        let mut rates = crate::rates::Rates::live(&self.ctx.repo);
        let year: i64 = mapcheck::dates::format(self.ctx.today)[..4].parse().unwrap_or(2026);
        if item.key == "treasury" {
            let paths: Vec<PathBuf> = [false, true].iter().map(|real| self.ctx.cache.join("rates").join(format!("treasury_{}_{year}.csv", if *real { "real" } else { "nominal" }))).collect();
            let before: Vec<_> = paths.iter().map(|p| modified(p)).collect();
            for real in [false, true] {
                if let Err(e) = rates.load_treasury_curve(year, real, true) {
                    return self.fail(item, &e);
                }
            }
            self.refreshed(item, &paths, &before, "Treasury nominal and real curves of the year")
        } else {
            let path = self.ctx.cache.join("rates").join("bls_cpi.json");
            let before = [modified(&path)];
            if let Err(e) = rates.load_cpi(true) {
                return self.fail(item, &e);
            }
            self.refreshed(item, &[path], &before, "BLS CPI")
        }
    }

    fn job_gsw(&self, item: &PlanItem) -> JobOutcome {
        let path = crate::forwards::cache_path(&self.ctx.repo);
        let before = [modified(&path)];
        match crate::forwards::load_with(&self.ctx.repo, true) {
            Ok(_) => self.refreshed(item, &[path], &before, "Federal Reserve GSW curve"),
            Err(e) => self.fail(item, &e),
        }
    }

    fn job_fred(&self, item: &PlanItem) -> JobOutcome {
        let path = crate::bonds::cached_series_path(&self.ctx.cache, &item.key);
        let before = [modified(&path)];
        let mut rates = crate::rates::Rates::live(&self.ctx.repo);
        match rates.load_fred(&item.key, true) {
            Ok(s) => {
                let o = self.refreshed(item, &[path], &before, &format!("{} observations", s.len()));
                o
            }
            Err(e) => self.fail(item, &e),
        }
    }

    // ------------------------------------------------------------ futures, vix, bonds

    fn job_futures(&self, item: &PlanItem) -> JobOutcome {
        crate::futures::ignore_cache_age(true);
        let errors_before = hostnet::stats().get(&item.hosts[0]).map_or(0, |s| s.errors);
        let repo = &self.ctx.repo;
        let k = item.key.as_str();
        let result: Result<String, String> = if k == "vx" {
            crate::futures::curve(repo, "VX", None).map(|c| format!("VX curve of {} ({} points)", c.date, c.points.len()))
        } else if k == "policy-path" {
            crate::futures::mpt::policy_path(repo, None).map(|_| "policy-rate path".to_string())
        } else if let Some(root) = k.strip_prefix("cftc:") {
            match crate::futures::facts::root(root) {
                Some(r) => crate::futures::cftc::positions(repo, r, super::fresh::FUTURES_CFTC_WEEKS).map(|_| "CFTC positions".to_string()),
                None => Err(format!("unknown root {root}")),
            }
        } else if let Some(root) = k.strip_prefix("yahoo:") {
            crate::futures::quote(repo, root, None).map(|s| format!("{} ({} days)", s.symbol, s.observations.len()))
        } else {
            Err(format!("unknown futures item {k}"))
        };
        match result {
            Ok(note) => {
                // A function that uses the old file after a failed download says Ok: a stopped host shows it.
                if let Some((_, why)) = hostnet::blocked(&item.hosts[0]) {
                    return JobOutcome::new(Status::Blocked, format!("{}; the old file stays", short(&why)));
                }
                // The function uses the old file when a download fails: an error of the host during the job shows it.
                let after = hostnet::stats().get(&item.hosts[0]).cloned().unwrap_or_default();
                if after.errors > errors_before {
                    let why = after.last_error.map_or_else(|| "the download failed".to_string(), |e| short(&e));
                    return JobOutcome::new(Status::Failed, format!("{why}; the old file stays")).requests(1);
                }
                self.done(item, Status::Updated, note, 1)
            }
            Err(e) => self.fail(item, &e),
        }
    }

    fn job_vix(&self, item: &PlanItem) -> JobOutcome {
        let path = self.ctx.cache.join("vix").join("VIX_History.csv");
        match hostnet::get(VIX_HISTORY_URL, &[], 2.0) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                if !text.trim_start().to_uppercase().starts_with(VIX_HEADER) {
                    return JobOutcome::new(Status::Failed, format!("{VIX_HISTORY_URL}: the file has no header {VIX_HEADER}")).requests(1);
                }
                let rows = text.lines().count().saturating_sub(1);
                match market::write_atomic(&path, &text) {
                    Ok(()) => self.done(item, Status::Updated, format!("Cboe VIX history, {rows} rows"), 1),
                    Err(e) => JobOutcome::new(Status::Failed, e).requests(1),
                }
            }
            Err(e) => self.fail(item, &e),
        }
    }

    fn job_bonds(&self, item: &PlanItem) -> JobOutcome {
        let date = item.key.as_str();
        match crate::prices::load(&self.ctx.cache, date) {
            Ok(q) => {
                let file = self.ctx.cache.join("prices").join(format!("fedinvest_{date}.csv"));
                if file.is_file() {
                    self.done(item, Status::Updated, format!("FedInvest {date}, {} securities", q.len()), 4)
                } else {
                    JobOutcome::new(Status::NotYet, format!("FedInvest has no end-of-day prices for {date} yet")).requests(4)
                }
            }
            Err(e) if e.contains("gave no price rows") => JobOutcome::new(Status::NotYet, format!("FedInvest has no price rows for {date} yet")).requests(4),
            Err(e) => self.fail(item, &e),
        }
    }
}

/// The short reason of a text, for a line of the summary: the HTTP code or the kind of error, with no URL.
fn short(text: &str) -> String {
    hostnet::reason(text)
}

impl Runner for RealRunner {
    fn run(&self, item: &PlanItem) -> JobOutcome {
        match item.set {
            DataSet::Sec => self.job_sec(item),
            DataSet::Filings => self.job_filings(item),
            DataSet::Prices => self.job_prices(item),
            DataSet::Intraday => self.job_intraday(item),
            DataSet::Estimates => self.job_estimates(item),
            DataSet::Earnings => {
                if item.key == CALENDAR_KEY {
                    self.job_calendar(item)
                } else {
                    self.job_earnings_sec(item)
                }
            }
            DataSet::Options => self.job_options(item),
            DataSet::Rates => self.job_rates(item),
            DataSet::Gsw => self.job_gsw(item),
            DataSet::Fred => self.job_fred(item),
            DataSet::Futures => self.job_futures(item),
            DataSet::Vix => self.job_vix(item),
            DataSet::Bonds => self.job_bonds(item),
        }
    }

    fn blocked_hosts(&self) -> Vec<(String, String)> {
        hostnet::stats().into_iter().filter_map(|(h, s)| s.blocked.map(|(_, why)| (h, short(&why)))).collect()
    }

    fn host_requests(&self, host: &str) -> Option<usize> {
        hostnet::stats().get(host).map(|s| s.requests)
    }

    fn request_stop(&self) {
        hostnet::request_stop();
    }
}

/// The number of files below a folder (0 when the folder is missing).
fn count_files(dir: &std::path::Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    entries.flatten().map(|e| if e.path().is_dir() { count_files(&e.path()) } else { 1 }).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::price_source::{FetchError, PriceSource, Row};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A source with fixed answers: it always answers HTTP 429 (`limited`), or it gives 30 rows. It counts its requests.
    struct Fixed {
        name: &'static str,
        limited: Option<u16>,
        calls: Arc<AtomicUsize>,
    }

    impl PriceSource for Fixed {
        fn name(&self) -> &'static str {
            self.name
        }
        fn adjusts_dividends(&self) -> bool {
            false
        }
        fn fetch(&self, _t: &str, _from: Option<&str>, _to: &str) -> Result<Vec<Row>, FetchError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(code) = self.limited {
                return Err(FetchError::RateLimited(format!("HTTP {code} for https://{}.test/x", self.name)));
            }
            Ok((0..30).map(|i| Row { date: format!("2024-01-{:02}", i + 1), open: 10.0, high: 11.0, low: 9.0, close: 10.0, volume: 1, adj_close: None, dividend: None, split: None }).collect())
        }
    }

    fn setup(tag: &str, limited_name: &'static str, next_name: &'static str) -> (std::path::PathBuf, Vec<Box<dyn PriceSource>>, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let dir = std::env::temp_dir().join(format!("fa_runner_429_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (a, b) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let sources: Vec<Box<dyn PriceSource>> = vec![Box::new(Fixed { name: limited_name, limited: Some(429), calls: a.clone() }), Box::new(Fixed { name: next_name, limited: None, calls: b.clone() })];
        (dir, sources, a, b)
    }

    fn options() -> price_cache::Options {
        price_cache::Options { today: "2024-01-30".into(), full: false, forced: false, retry_wait: Duration::ZERO, again: false }
    }

    #[test]
    fn a_price_source_that_answers_429_gets_no_more_than_two_requests_in_a_run_of_many_tickers_and_the_next_source_gives_the_prices() {
        let (dir, sources, limited, next) = setup("many", "fake429a", "fakeok_a");
        let opt = options();
        for i in 0..40 {
            let t = format!("T{i}");
            let s = update_prices(&dir, &t, &sources, &opt);
            assert!(matches!(s, price_cache::Status::Updated { ref source, .. } if source == "fakeok_a"), "{t}: {s:?}");
        }
        assert_eq!(limited.load(Ordering::SeqCst), 2, "the first request and one new try, then no more");
        assert_eq!(next.load(Ordering::SeqCst), 40);
        assert!(hostnet::blocked("fake429a").is_some(), "the host is stopped for the run");
        let _ = std::fs::remove_dir_all(&dir);
    }


    #[test]
    fn two_sources_that_answer_403_and_429_stop_after_two_requests_each_and_the_ticker_fails() {
        let dir = std::env::temp_dir().join(format!("fa_runner_403_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (a, b) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let sources: Vec<Box<dyn PriceSource>> = vec![Box::new(Fixed { name: "fake403c", limited: Some(403), calls: a.clone() }), Box::new(Fixed { name: "fake429c", limited: Some(429), calls: b.clone() })];
        for t in ["A", "B", "C"] {
            let s = update_prices(&dir, t, &sources, &options());
            assert!(matches!(s, price_cache::Status::Failed(_)), "{t}: {s:?}");
        }
        assert_eq!((a.load(Ordering::SeqCst), b.load(Ordering::SeqCst)), (2, 2));
        assert!(hostnet::blocked("fake403c").is_some() && hostnet::blocked("fake429c").is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn a_host_that_the_run_stopped_is_not_asked_by_the_price_pass() {
        let (dir, sources, limited, next) = setup("pre", "fake429b", "fakeok_b");
        hostnet::block("fake429b", 429, "HTTP 429 for https://fake429b.test/x");
        let s = update_prices(&dir, "AAA", &sources, &options());
        assert!(matches!(s, price_cache::Status::Updated { ref source, .. } if source == "fakeok_b"), "{s:?}");
        assert_eq!(limited.load(Ordering::SeqCst), 0);
        assert_eq!(next.load(Ordering::SeqCst), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
