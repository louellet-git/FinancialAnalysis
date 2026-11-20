//! The plan of a run: what is old, what is current, and which requests go to which host. The plan reads the caches and the
//! ledger only; it makes no network request. `--dry-run` prints it, `--status` prints the counts of it.

use super::config::SyncConfig;
use super::dataset::{host_seconds, DataSet, Scope, ALL};
use super::fresh::{earnings_sec_state, keys_of, state_of, Counts, Ctx, State};
use super::ledger::Ledger;
use crate::market::{earnings_dates, price_cache};
use std::collections::BTreeMap;

/// Which keys exist (a key is never read here beyond "is it set").
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct KeyInfo {
    pub tiingo: bool,
    pub alpaca: bool,
    pub fred: bool,
    pub fmp: bool,
}

impl KeyInfo {
    pub fn read(repo: &std::path::Path) -> KeyInfo {
        use crate::market::keys::read_key;
        KeyInfo {
            tiingo: read_key(repo, crate::market::price_source::TIINGO_KEY_ENV).is_some(),
            alpaca: crate::market::alpaca::Keys::read(repo).is_some(),
            fred: read_key(repo, crate::market::fred_api::KEY_ENV).is_some(),
            fmp: crate::market::fmp::api_key(repo).is_some(),
        }
    }
}

/// The key of the item of `earnings` that scans the Nasdaq calendar.
pub const CALENDAR_KEY: &str = "_calendar";
/// The days that the calendar scan covers (the default of `fa earnings calendar`).
pub const CALENDAR_DAYS: i64 = 60;

#[derive(Debug, Clone, PartialEq)]
pub struct PlanItem {
    pub set: DataSet,
    pub key: String,
    /// The hosts that the item can ask, the first source first. The first host is the queue of the item.
    pub hosts: Vec<String>,
    /// The requests that the item needs (an estimate).
    pub requests: usize,
    /// `missing`, `old` or `forced`.
    pub why: String,
}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub items: Vec<PlanItem>,
    /// The state counts of each data set in the plan (the data sets that were selected and not skipped).
    pub counts: Vec<(DataSet, Counts)>,
    /// The oldest date of each data set (the cache record of the item that is the oldest).
    pub oldest: Vec<(DataSet, String)>,
    /// A data set that does not run, with the reason.
    pub skipped: Vec<(DataSet, String)>,
    /// Lines for the summary: what a missing key or contact would add.
    pub hints: Vec<String>,
}

impl Plan {
    /// The requests of each host (the first host of an item), and the seconds that they take.
    pub fn host_load(&self) -> BTreeMap<String, (usize, f64)> {
        let mut out: BTreeMap<String, usize> = BTreeMap::new();
        for it in &self.items {
            *out.entry(it.hosts[0].clone()).or_default() += it.requests;
        }
        out.into_iter().map(|(h, n)| (h.clone(), (n, host_seconds(&h, n)))).collect()
    }

    /// The seconds of the run: the hosts run at the same time, so the longest queue decides.
    pub fn estimated_seconds(&self) -> f64 {
        self.host_load().values().map(|(_, s)| *s).fold(0.0, f64::max)
    }

    pub fn counts_of(&self, set: DataSet) -> Option<Counts> {
        self.counts.iter().find(|(s, _)| *s == set).map(|(_, c)| *c)
    }
}

pub struct PlanInput<'a> {
    pub ctx: &'a Ctx,
    pub cfg: &'a SyncConfig,
    pub ledger: &'a Ledger,
    /// All tickers of the run, in a stable order.
    pub tickers: Vec<String>,
    /// The tickers that the person named on the command line (they get the filings).
    pub focus: Vec<String>,
    pub sets: Vec<DataSet>,
    pub force: bool,
    pub sec_contact: bool,
    pub keys: KeyInfo,
    pub intraday_extra: Vec<String>,
    pub option_symbols: Vec<String>,
}

/// The sources of the prices of a ticker, in the order of the existing code: Tiingo and Alpaca (with keys), Yahoo, Nasdaq. A
/// cache of one source keeps that source: the cache is not mixed, and a block of its host does not make the sync replace the
/// cache with the data of another source. Only when the key of the cache source is gone, the other sources are tried.
pub fn price_hosts(ctx: &Ctx, ticker: &str, keys: &KeyInfo) -> Vec<String> {
    let mut available: Vec<&str> = Vec::new();
    if keys.tiingo {
        available.push("tiingo");
    }
    if keys.alpaca {
        available.push("alpaca");
    }
    available.extend(["yahoo", "nasdaq"]);
    let cached = std::fs::read_to_string(price_cache::meta_path(&ctx.cache, ticker)).ok().and_then(|t| serde_json::from_str::<price_cache::Meta>(&t).ok()).map(|m| m.source);
    match cached {
        Some(s) if available.contains(&s.as_str()) => vec![s],
        _ => available.into_iter().map(String::from).collect(),
    }
}

fn why(state: &State, force: bool) -> Option<&'static str> {
    if force {
        return Some("forced");
    }
    match state {
        State::Current(_) => None,
        State::Old { .. } => Some("old"),
        State::Missing => Some("missing"),
    }
}

/// The number of days of the calendar scan that need a request.
fn calendar_requests(ctx: &Ctx, force: bool) -> usize {
    let today = ctx.today_text();
    earnings_dates::scan_dates(&today, CALENDAR_DAYS)
        .iter()
        .filter(|d| force || !earnings_dates::load_calendar_day(&ctx.cache, d).is_some_and(|day| earnings_dates::calendar_is_current(d, &day.fetched, &today)))
        .count()
}

/// The windows of one request for the Alpaca bars: one month each.
fn intraday_requests(ctx: &Ctx, symbol: &str, alpaca: bool) -> usize {
    if !alpaca {
        return 1;
    }
    let file = ctx.trading.join(crate::trading::data::file_name(symbol, "5m"));
    let from = last_date_of_file(&file).and_then(|d| mapcheck::dates::parse(&d)).unwrap_or(ctx.today - DEFAULT_INTRADAY_DAYS);
    (((ctx.today - from) / 28) + 1).max(1) as usize
}

/// The first run of the intraday bars goes back this many days (the depth of the Yahoo source).
pub const DEFAULT_INTRADAY_DAYS: i64 = 60;

/// The date of the last data line of a bars file.
pub fn last_date_of_file(path: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let line = text.lines().rev().find(|l| !l.is_empty() && !l.starts_with("time") && !l.starts_with("date"))?;
    let d: String = line.chars().take(10).collect();
    mapcheck::dates::parse(&d).map(|_| d)
}

pub fn make_plan(inp: &PlanInput) -> Plan {
    let mut plan = Plan::default();
    for set in ALL {
        if !inp.sets.contains(&set) {
            continue;
        }
        if set.needs_sec_contact() && !inp.sec_contact {
            plan.skipped.push((set, format!("{} is not set (the SEC asks for a contact in each request)", mapcheck::http::SEC_USER_AGENT_ENV)));
            continue;
        }
        let keys = keys_of(set, inp.ctx, inp.cfg, &inp.tickers, &inp.intraday_extra, &inp.option_symbols);
        if keys.is_empty() {
            let why = match set.scope() {
                Scope::Tickers => "no tickers: give TICKER..., --tickers-file F, or tickers_file in sync.toml".to_string(),
                _ => "no symbols".to_string(),
            };
            plan.skipped.push((set, why));
            continue;
        }
        let mut counts = Counts::default();
        let mut oldest: Option<String> = None;
        for key in &keys {
            let state = state_of(inp.ctx, inp.ledger, inp.cfg, set, key);
            if let Some(d) = state.date() {
                if oldest.as_deref().is_none_or(|o| d < o) {
                    oldest = Some(d.to_string());
                }
            }
            // Filings: only a ticker that the person named, or that has a filings folder already.
            if set == DataSet::Filings {
                let has_folder = inp.ctx.cache.join(key).join("filings").is_dir();
                if !has_folder && !inp.focus.contains(key) {
                    continue;
                }
            }
            // Earnings: the state of the ticker is the worst of the Nasdaq part and the SEC part.
            let combined = if set == DataSet::Earnings && inp.sec_contact {
                let sec = earnings_sec_state(inp.ctx, inp.ledger, inp.cfg, key);
                match (&state, &sec) {
                    (State::Current(_), State::Current(_)) => state.clone(),
                    (State::Missing, State::Missing) => State::Missing,
                    (State::Current(_), _) => State::Old { date: sec.date().map(String::from), why: "the filing list of the SEC is old".into() },
                    _ => state.clone(),
                }
            } else {
                state.clone()
            };
            counts.add(&combined);
            match set {
                DataSet::Earnings => {
                    // The Nasdaq part is one scan of the calendar; the SEC part is one request for each ticker.
                    if let Some(w) = why(&state, inp.force) {
                        if !plan.items.iter().any(|i| i.set == set && i.key == CALENDAR_KEY) {
                            plan.items.push(PlanItem { set, key: CALENDAR_KEY.into(), hosts: vec!["nasdaq".into()], requests: calendar_requests(inp.ctx, inp.force), why: w.into() });
                        }
                    }
                    if inp.sec_contact {
                        let sec = earnings_sec_state(inp.ctx, inp.ledger, inp.cfg, key);
                        if let Some(w) = why(&sec, inp.force) {
                            // The `sec` item of the same ticker looks at the same filing list: then no new request is needed.
                            let shared = inp.sets.contains(&DataSet::Sec) && plan.items.iter().any(|i| i.set == DataSet::Sec && &i.key == key);
                            plan.items.push(PlanItem { set, key: key.clone(), hosts: vec!["sec".into()], requests: usize::from(!shared), why: w.into() });
                        }
                    }
                }
                _ => {
                    let Some(w) = why(&combined, inp.force) else { continue };
                    let (hosts, requests): (Vec<String>, usize) = match set {
                        DataSet::Sec => (vec!["sec".into()], if matches!(state, State::Missing) { 3 } else { 2 }),
                        DataSet::Filings => (vec!["sec".into()], 3),
                        DataSet::Prices => (price_hosts(inp.ctx, key, &inp.keys), 1),
                        DataSet::Estimates => (vec!["nasdaq".into()], 3),
                        DataSet::Intraday => {
                            let alpaca = inp.keys.alpaca;
                            (if alpaca { vec!["alpaca".into(), "yahoo".into()] } else { vec!["yahoo".into()] }, intraday_requests(inp.ctx, key, alpaca))
                        }
                        DataSet::Options => {
                            let mut h = vec!["cboe".to_string()];
                            if inp.keys.alpaca {
                                h.push("alpaca".into());
                            }
                            (h, 1)
                        }
                        DataSet::Rates => {
                            if key == "treasury" {
                                (vec!["treasury".into()], 2)
                            } else {
                                (vec!["bls".into()], 1)
                            }
                        }
                        DataSet::Gsw => (vec!["federalreserve".into()], 1),
                        DataSet::Fred => (vec!["fred".into()], 1),
                        DataSet::Futures => {
                            let host = if key == "vx" {
                                "cboe"
                            } else if key == "policy-path" {
                                "atlantafed"
                            } else if key.starts_with("cftc:") {
                                "cftc"
                            } else {
                                "yahoo"
                            };
                            (vec![host.into()], 1)
                        }
                        DataSet::Vix => (vec!["cboe".into()], 1),
                        DataSet::Bonds => (vec!["treasurydirect".into()], 4),
                        DataSet::Earnings => unreachable!("handled above"),
                    };
                    plan.items.push(PlanItem { set, key: key.clone(), hosts, requests, why: w.into() });
                }
            }
        }
        plan.counts.push((set, counts));
        if let Some(o) = oldest {
            plan.oldest.push((set, o));
        }
    }
    // What a missing key would add (said once, in the summary).
    let wants = |s: DataSet| inp.sets.contains(&s);
    if !inp.keys.alpaca && (wants(DataSet::Prices) || wants(DataSet::Intraday) || wants(DataSet::Options)) {
        plan.hints.push("APCA_API_KEY_ID and APCA_API_SECRET_KEY would add Alpaca (prices, intraday bars, option chain as a second source)".into());
    }
    if !inp.keys.tiingo && wants(DataSet::Prices) {
        plan.hints.push("TIINGO_API_KEY would add Tiingo (prices with split and dividend adjusted history of more than 30 years)".into());
    }
    if !inp.keys.fred && wants(DataSet::Fred) {
        plan.hints.push("FRED_API_KEY would add the FRED API (the first source of the FRED series; now the keyless file)".into());
    }
    if !inp.keys.fmp && wants(DataSet::Estimates) {
        plan.hints.push("FMP_API_KEY would add revenue estimates (FMP); the sync does not use FMP, use `fa estimates get` for it".into());
    }
    plan
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::fresh::tests::{ctx_at, temp, write_prices};
    use super::*;

    pub fn input<'a>(ctx: &'a Ctx, cfg: &'a SyncConfig, ledger: &'a Ledger, tickers: &[&str], sets: &[DataSet]) -> PlanInput<'a> {
        PlanInput {
            ctx,
            cfg,
            ledger,
            tickers: tickers.iter().map(|s| s.to_string()).collect(),
            focus: tickers.iter().map(|s| s.to_string()).collect(),
            sets: sets.to_vec(),
            force: false,
            sec_contact: true,
            keys: KeyInfo::default(),
            intraday_extra: vec![],
            option_symbols: vec!["SPY".into()],
        }
    }

    fn keys_of_plan(p: &Plan, set: DataSet) -> Vec<String> {
        p.items.iter().filter(|i| i.set == set).map(|i| i.key.clone()).collect()
    }

    #[test]
    fn prices_plan_follows_the_cache_state_and_the_calendar() {
        let repo = temp("plan-prices");
        let cache = repo.join("data/raw");
        write_prices(&cache, "IBM", "2026-11-20", "2026-11-20"); // current on Saturday
        write_prices(&cache, "MSFT", "2026-11-18", "2026-11-18"); // old on Saturday
        let (cfg, ledger) = (SyncConfig::default(), Ledger::in_memory());
        let ctx = ctx_at(&repo, "2026-11-21");
        let p = make_plan(&input(&ctx, &cfg, &ledger, &["IBM", "MSFT", "NVDA"], &[DataSet::Prices]));
        assert_eq!(keys_of_plan(&p, DataSet::Prices), ["MSFT", "NVDA"]);
        assert_eq!(p.counts_of(DataSet::Prices), Some(Counts { current: 1, old: 1, missing: 1 }));
        assert_eq!(p.items[0].why, "old");
        assert_eq!(p.items[1].why, "missing");
        // On Monday IBM is old too.
        let monday = ctx_at(&repo, "2026-11-23");
        let p = make_plan(&input(&monday, &cfg, &ledger, &["IBM", "MSFT", "NVDA"], &[DataSet::Prices]));
        assert_eq!(keys_of_plan(&p, DataSet::Prices), ["IBM", "MSFT", "NVDA"]);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn force_plans_everything_and_only_and_skip_choose_the_data_sets() {
        let repo = temp("plan-force");
        write_prices(&repo.join("data/raw"), "IBM", "2026-11-20", "2026-11-20");
        let (cfg, ledger) = (SyncConfig::default(), Ledger::in_memory());
        let ctx = ctx_at(&repo, "2026-11-21");
        let mut inp = input(&ctx, &cfg, &ledger, &["IBM"], &[DataSet::Prices]);
        assert!(make_plan(&inp).items.is_empty());
        inp.force = true;
        let p = make_plan(&inp);
        assert_eq!(p.items.len(), 1);
        assert_eq!(p.items[0].why, "forced");
        // The data sets that are not selected are not in the plan.
        inp.sets = vec![DataSet::Gsw, DataSet::Vix];
        let p = make_plan(&inp);
        let sets: Vec<DataSet> = p.items.iter().map(|i| i.set).collect();
        assert_eq!(sets, [DataSet::Gsw, DataSet::Vix]);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn without_the_sec_contact_the_sec_data_sets_are_skipped_and_the_others_run() {
        let repo = temp("plan-sec");
        let (cfg, ledger) = (SyncConfig::default(), Ledger::in_memory());
        let ctx = ctx_at(&repo, "2026-11-19");
        let mut inp = input(&ctx, &cfg, &ledger, &["IBM"], &[DataSet::Sec, DataSet::Filings, DataSet::Prices, DataSet::Earnings]);
        inp.sec_contact = false;
        let p = make_plan(&inp);
        let skipped: Vec<DataSet> = p.skipped.iter().map(|(s, _)| *s).collect();
        assert_eq!(skipped, [DataSet::Sec, DataSet::Filings]);
        assert!(p.skipped[0].1.contains("SEC_USER_AGENT is not set"));
        assert_eq!(keys_of_plan(&p, DataSet::Prices), ["IBM"]);
        // The earnings data set runs with the Nasdaq part only: one scan of the calendar, no item of the SEC host.
        assert_eq!(keys_of_plan(&p, DataSet::Earnings), [CALENDAR_KEY]);
        // With the contact: the same ticker has an item of the SEC host too.
        inp.sec_contact = true;
        let p = make_plan(&inp);
        assert_eq!(keys_of_plan(&p, DataSet::Earnings), [CALENDAR_KEY, "IBM"]);
        assert_eq!(keys_of_plan(&p, DataSet::Sec), ["IBM"]);
        // The earnings item of the SEC host shares the look at the filing list with the `sec` item: no request of its own.
        assert_eq!(p.items.iter().find(|i| i.set == DataSet::Earnings && i.key == "IBM").unwrap().requests, 0);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn filings_only_for_the_named_tickers_and_the_tickers_that_have_a_folder() {
        let repo = temp("plan-filings");
        std::fs::create_dir_all(repo.join("data/raw/MSFT/filings/0001")).unwrap();
        let (cfg, ledger) = (SyncConfig::default(), Ledger::in_memory());
        let ctx = ctx_at(&repo, "2026-11-19");
        let mut inp = input(&ctx, &cfg, &ledger, &["IBM", "MSFT", "AAPL"], &[DataSet::Filings]);
        inp.focus = vec!["IBM".into()];
        let p = make_plan(&inp);
        assert_eq!(keys_of_plan(&p, DataSet::Filings), ["IBM", "MSFT"]);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn the_price_source_of_the_cache_is_kept() {
        let repo = temp("plan-hosts");
        write_prices(&repo.join("data/raw"), "IBM", "2026-11-18", "2026-11-18"); // a cache of nasdaq
        let ctx = ctx_at(&repo, "2026-11-19");
        let none = KeyInfo::default();
        assert_eq!(price_hosts(&ctx, "IBM", &none), ["nasdaq"], "the cache of Nasdaq stays with Nasdaq");
        assert_eq!(price_hosts(&ctx, "NEW", &none), ["yahoo", "nasdaq"], "no cache: the order of the existing code");
        let all = KeyInfo { tiingo: true, alpaca: true, fred: false, fmp: false };
        assert_eq!(price_hosts(&ctx, "NEW", &all), ["tiingo", "alpaca", "yahoo", "nasdaq"]);
        assert_eq!(price_hosts(&ctx, "IBM", &all), ["nasdaq"]);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn the_estimate_of_time_is_the_longest_queue_not_the_sum() {
        let repo = temp("plan-time");
        let (cfg, ledger) = (SyncConfig::default(), Ledger::in_memory());
        let ctx = ctx_at(&repo, "2026-11-19");
        let p = make_plan(&input(&ctx, &cfg, &ledger, &["IBM", "MSFT"], &[DataSet::Estimates, DataSet::Gsw, DataSet::Vix]));
        let load = p.host_load();
        assert_eq!(load["nasdaq"].0, 6);
        assert_eq!(load["federalreserve"].0, 1);
        assert_eq!(load["cboe"].0, 1);
        let sum: f64 = load.values().map(|(_, s)| s).sum();
        assert!(p.estimated_seconds() < sum);
        assert!((p.estimated_seconds() - load["nasdaq"].1).abs() < 1e-9);
        // A missing key gives a hint (once).
        assert!(p.hints.iter().any(|h| h.contains("FMP_API_KEY")));
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn no_tickers_skips_the_ticker_data_sets_with_a_reason() {
        let repo = temp("plan-none");
        let (cfg, ledger) = (SyncConfig::default(), Ledger::in_memory());
        let ctx = ctx_at(&repo, "2026-11-19");
        let p = make_plan(&input(&ctx, &cfg, &ledger, &[], &[DataSet::Prices, DataSet::Vix]));
        assert_eq!(p.skipped.len(), 1);
        assert!(p.skipped[0].1.contains("no tickers"));
        assert_eq!(keys_of_plan(&p, DataSet::Vix), ["history"]);
        let _ = std::fs::remove_dir_all(&repo);
    }
}
