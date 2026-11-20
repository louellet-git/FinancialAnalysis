//! Is a data set old or current? The answer comes from the cache files and the ledger only: no network request.
//!
//! The date of an item is the newest of two records: the date that the cache keeps (the `updated` field of
//! `prices.meta.json`, the date of the newest snapshot file, the date of a file) and the date in the ledger of the sync.
//! The age of the date is counted in market days or in calendar days (`DataSet::age_unit`).

use super::calendar::{calendar_age, market_age, previous_market_day};
use super::config::SyncConfig;
use super::dataset::{AgeUnit, DataSet};
use super::ledger::Ledger;
use crate::market::{consensus, earnings_dates, options, price_cache, price_cli};
use mapcheck::dates;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Where the caches are and what day it is.
#[derive(Debug, Clone)]
pub struct Ctx {
    pub repo: PathBuf,
    pub cache: PathBuf,
    /// The folder of the bars of the option bots (`data/raw/trading`, or `TRADING_DATA_DIR`).
    pub trading: PathBuf,
    pub today: i64,
}

impl Ctx {
    pub fn new(repo: &Path) -> Ctx {
        Ctx {
            repo: repo.to_path_buf(),
            cache: price_cli::cache_dir(repo),
            trading: crate::trading::data::Store::from_env(repo).dir().to_path_buf(),
            today: dates::today(),
        }
    }

    pub fn today_text(&self) -> String {
        dates::format(self.today)
    }
}

/// The state of one item.
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// The newest record is young enough.
    Current(String),
    /// A record exists and it is too old (or something newer exists at the source), or a part of the data is old.
    Old { date: Option<String>, why: String },
    /// No cache file and no ledger record.
    Missing,
}

impl State {
    pub fn is_current(&self) -> bool {
        matches!(self, State::Current(_))
    }

    pub fn date(&self) -> Option<&str> {
        match self {
            State::Current(d) => Some(d),
            State::Old { date, .. } => date.as_deref(),
            State::Missing => None,
        }
    }
}

/// The date (UTC) of the last change of a file.
pub fn file_date(path: &Path) -> Option<String> {
    let secs = std::fs::metadata(path).and_then(|m| m.modified()).ok()?.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs() as i64;
    Some(dates::format(secs / 86_400))
}

/// The newest of a date of the cache and a date of the ledger (ISO dates compare as text).
fn newest(a: Option<String>, b: Option<&str>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if b > a.as_str() { b.to_string() } else { a }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b.to_string()),
        (None, None) => None,
    }
}

/// Is a record of this date young enough for the maximum age?
pub fn young_enough(set: DataSet, date: &str, today: i64, max_age: u32) -> bool {
    match set.age_unit() {
        AgeUnit::MarketDays => market_age(date, today).is_some_and(|a| a < max_age as i64),
        AgeUnit::CalendarDays => calendar_age(date, today).is_some_and(|a| a <= max_age as i64),
    }
}

/// The age text of a date for the status line and the warning: `3 market days` or `9 days`.
pub fn age_text(set: DataSet, date: &str, today: i64) -> String {
    match set.age_unit() {
        AgeUnit::MarketDays => {
            let a = market_age(date, today).unwrap_or(0);
            format!("{a} market day{}", if a == 1 { "" } else { "s" })
        }
        AgeUnit::CalendarDays => {
            let a = calendar_age(date, today).unwrap_or(0);
            format!("{a} day{}", if a == 1 { "" } else { "s" })
        }
    }
}

fn judge(set: DataSet, date: Option<String>, ctx: &Ctx, cfg: &SyncConfig) -> State {
    match date {
        None => State::Missing,
        Some(d) if young_enough(set, &d, ctx.today, cfg.age_of(set)) => State::Current(d),
        Some(d) => {
            let why = format!("{} old", age_text(set, &d, ctx.today));
            State::Old { date: Some(d), why }
        }
    }
}

/// The keys of a data set: the tickers, the symbols of the config, or the fixed items.
pub fn keys_of(set: DataSet, ctx: &Ctx, cfg: &SyncConfig, tickers: &[String], intraday_extra: &[String], option_symbols: &[String]) -> Vec<String> {
    match set {
        DataSet::Sec | DataSet::Filings | DataSet::Prices | DataSet::Estimates | DataSet::Earnings => tickers.to_vec(),
        DataSet::Intraday => {
            let mut out: Vec<String> = Vec::new();
            for s in cfg.intraday_symbols.iter().chain(intraday_extra) {
                if !out.contains(s) {
                    out.push(s.clone());
                }
            }
            out
        }
        DataSet::Options => option_symbols.to_vec(),
        DataSet::Rates => vec!["treasury".into(), "bls".into()],
        DataSet::Gsw => vec!["gsw".into()],
        DataSet::Fred => crate::bonds::cached_series_ids(),
        DataSet::Futures => futures_keys(),
        DataSet::Vix => vec!["history".into()],
        DataSet::Bonds => vec![dates::format(previous_market_day(ctx.today))],
    }
}

/// The items of the futures data set: the VIX futures curve, the policy path, and the CFTC positions and the Yahoo
/// series of each root.
pub fn futures_keys() -> Vec<String> {
    let mut out = vec!["vx".to_string(), "policy-path".to_string()];
    for r in crate::futures::facts::ROOTS.iter() {
        out.push(format!("cftc:{}", r.root));
    }
    for r in crate::futures::facts::ROOTS.iter().filter(|r| !r.yahoo_suffix.is_empty()) {
        out.push(format!("yahoo:{}", r.root));
    }
    out
}

/// The files of the filings folder of a ticker: the newest change of a file in it.
fn newest_file_date(dir: &Path) -> Option<String> {
    let mut best: Option<String> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        let d = if p.is_dir() { newest_file_date(&p) } else { file_date(&p) };
        best = newest(best, d.as_deref());
    }
    best
}

/// The date of the cache record of an item (without the ledger).
fn cache_date(set: DataSet, ctx: &Ctx, key: &str) -> Option<String> {
    let c = &ctx.cache;
    match set {
        DataSet::Sec => {
            let check = mapcheck::sec::load_filing_list_check(c, key).map(|k| k.checked);
            check.or_else(|| mapcheck::run::facts_path(c, key).and_then(|p| file_date(&p)))
        }
        DataSet::Filings => newest_file_date(&c.join(key).join("filings")),
        DataSet::Prices => {
            let meta = std::fs::read_to_string(price_cache::meta_path(c, key)).ok().and_then(|t| serde_json::from_str::<price_cache::Meta>(&t).ok()).map(|m| m.updated).filter(|u| !u.is_empty());
            meta.or_else(|| file_date(&price_cache::prices_path(c, key)))
        }
        DataSet::Estimates => {
            let snap = consensus::list_snapshots(c, key).pop();
            let surprise = crate::market::estimates_cache::load_surprises(c, key).map(|s| s.fetched);
            newest(snap, surprise.as_deref())
        }
        DataSet::Earnings => earnings_dates::load_dates(c, key).map(|f| f.updated).filter(|u| !u.is_empty()),
        DataSet::Intraday => file_date(&ctx.trading.join(crate::trading::data::file_name(key, "5m"))),
        DataSet::Options => options::list_snapshots(c, key).pop(),
        DataSet::Rates => match key {
            "treasury" => {
                let year = dates::format(ctx.today)[..4].to_string();
                let a = file_date(&c.join("rates").join(format!("treasury_nominal_{year}.csv")));
                let b = file_date(&c.join("rates").join(format!("treasury_real_{year}.csv")));
                // Both files must be young: the older one counts.
                match (a, b) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    _ => None,
                }
            }
            _ => file_date(&c.join("rates").join("bls_cpi.json")),
        },
        DataSet::Gsw => file_date(&crate::forwards::cache_path(&ctx.repo)),
        DataSet::Fred => file_date(&crate::bonds::cached_series_path(c, key)),
        DataSet::Futures => {
            let dir = c.join("futures");
            if key == "vx" {
                let mut best = None;
                for e in std::fs::read_dir(&dir).ok()?.flatten() {
                    let n = e.file_name().to_string_lossy().to_string();
                    if n.starts_with("cboe_settle_") {
                        best = newest(best, file_date(&e.path()).as_deref());
                    }
                }
                best
            } else if key == "policy-path" {
                file_date(&dir.join("mpt_histdata.xlsx"))
            } else if let Some(root) = key.strip_prefix("cftc:") {
                let code = crate::futures::facts::root(root)?.cftc_code;
                file_date(&dir.join(format!("cftc_{code}_{}.json", FUTURES_CFTC_WEEKS)))
            } else if let Some(root) = key.strip_prefix("yahoo:") {
                file_date(&dir.join(format!("yahoo_{root}_F.json")))
            } else {
                None
            }
        }
        DataSet::Vix => file_date(&c.join("vix").join("VIX_History.csv")),
        DataSet::Bonds => c.join("prices").join(format!("fedinvest_{key}.csv")).is_file().then(|| key.to_string()),
    }
}

/// The weeks of the CFTC positions that the sync asks for (the file name has the number).
pub const FUTURES_CFTC_WEEKS: u32 = 4;

/// The state of one item of a data set.
pub fn state_of(ctx: &Ctx, ledger: &Ledger, cfg: &SyncConfig, set: DataSet, key: &str) -> State {
    let date = newest(cache_date(set, ctx, key), ledger.get(set.name(), key));
    match set {
        DataSet::Bonds => match date {
            // The file of the date exists (or the ledger says that the date was asked): the item is current.
            Some(d) => State::Current(d),
            None => State::Missing,
        },
        DataSet::Filings => {
            let facts = mapcheck::run::facts_path(&ctx.cache, key).and_then(|p| file_date(&p));
            match (date, facts) {
                (None, _) => State::Missing,
                (Some(d), Some(f)) if d.as_str() < f.as_str() => State::Old { date: Some(d), why: "the company facts are newer than the filing files".into() },
                (Some(d), _) => State::Current(d),
            }
        }
        DataSet::Sec => {
            let base = judge(set, date.clone(), ctx, cfg);
            if let State::Current(_) = &base {
                // A filing that is newer than the cached facts makes the facts old, whatever the date of the look.
                let check = mapcheck::sec::load_filing_list_check(&ctx.cache, key);
                let facts = mapcheck::run::facts_path(&ctx.cache, key).and_then(|p| file_date(&p));
                if let (Some(filed), Some(facts)) = (check.and_then(|k| k.latest_periodic), facts) {
                    if filed >= facts {
                        return State::Old { date, why: format!("a filing of {filed} is newer than the cached facts ({facts})") };
                    }
                }
            }
            base
        }
        _ => judge(set, date, ctx, cfg),
    }
}

/// The state of the part of `earnings` that the SEC gives (the filing list with the 8-K of item 2.02).
pub fn earnings_sec_state(ctx: &Ctx, ledger: &Ledger, cfg: &SyncConfig, key: &str) -> State {
    let date = newest(mapcheck::sec::load_filing_list_check(&ctx.cache, key).map(|k| k.checked), ledger.get("sec", key));
    judge(DataSet::Earnings, date, ctx, cfg)
}

/// The counts of a data set: current, old, with no data.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Counts {
    pub current: usize,
    pub old: usize,
    pub missing: usize,
}

impl Counts {
    pub fn add(&mut self, s: &State) {
        match s {
            State::Current(_) => self.current += 1,
            State::Old { .. } => self.old += 1,
            State::Missing => self.missing += 1,
        }
    }

    pub fn total(&self) -> usize {
        self.current + self.old + self.missing
    }
}

/// The data sets that the value commands read, and the warning when one of them is old. This is the one function of
/// the age check: `fa dcf auto`, `fa verdict` and `fa estimates show` call it, and `fa sync --status` uses the same
/// states. A missing cache gives no warning (the command says that itself). Returns the text of one line, or `None`.
pub fn old_data_warning(repo: &Path, ticker: &str, sets: &[DataSet]) -> Option<String> {
    let ctx = Ctx::new(repo);
    let cfg = SyncConfig::load(repo).unwrap_or_default();
    let ledger = Ledger::load(&ctx.cache);
    let ticker = ticker.to_uppercase();
    let mut old = Vec::new();
    for set in sets {
        let key = match set.scope() {
            super::dataset::Scope::Tickers => ticker.clone(),
            _ => keys_of(*set, &ctx, &cfg, &[], &[], &[]).first().cloned().unwrap_or_default(),
        };
        if let State::Old { date: Some(d), .. } = state_of(&ctx, &ledger, &cfg, *set, &key) {
            old.push(format!("{} from {d} ({} old, maximum {})", set.name(), age_text(*set, &d, ctx.today), cfg.age_of(*set)));
        }
    }
    if old.is_empty() {
        None
    } else {
        Some(format!("warning: old data for {ticker}: {}; run: fa sync {ticker}", old.join(", ")))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fa-sync-fresh-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    pub fn ctx_at(repo: &Path, today: &str) -> Ctx {
        let mut ctx = Ctx::new(repo);
        ctx.today = dates::parse(today).unwrap();
        ctx
    }

    /// A prices cache of a ticker with an `updated` date.
    pub fn write_prices(cache: &Path, ticker: &str, updated: &str, last_date: &str) {
        let dir = cache.join(ticker);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("prices.json"), format!(r#"[{{"date":"{last_date}","open":1,"high":1,"low":1,"close":1,"volume":1}}]"#)).unwrap();
        std::fs::write(
            dir.join("prices.meta.json"),
            format!(r#"{{"source":"nasdaq","adjusted":"splits","updated":"{updated}","first_date":"{last_date}","last_date":"{last_date}","rows":1}}"#),
        )
        .unwrap();
    }

    #[test]
    fn prices_age_in_market_days_with_weekends_and_holidays() {
        let repo = temp("prices");
        let cache = repo.join("data/raw");
        let cfg = SyncConfig::default();
        let ledger = Ledger::in_memory();
        write_prices(&cache, "IBM", "2026-11-20", "2026-11-20"); // a Friday
        for (today, current) in [("2026-11-20", true), ("2026-11-21", true), ("2026-11-22", true), ("2026-11-23", false), ("2026-11-24", false)] {
            let st = state_of(&ctx_at(&repo, today), &ledger, &cfg, DataSet::Prices, "IBM");
            assert_eq!(st.is_current(), current, "{today}: {st:?}");
        }
        // Wednesday data is current on Thanksgiving (no session) and old on the Friday after.
        write_prices(&cache, "IBM", "2026-11-25", "2026-11-25");
        assert!(state_of(&ctx_at(&repo, "2026-11-26"), &ledger, &cfg, DataSet::Prices, "IBM").is_current());
        assert!(!state_of(&ctx_at(&repo, "2026-11-27"), &ledger, &cfg, DataSet::Prices, "IBM").is_current());
        assert_eq!(state_of(&ctx_at(&repo, "2026-11-27"), &ledger, &cfg, DataSet::Prices, "NONE"), State::Missing);
        // A longer maximum age from the config.
        let mut cfg3 = SyncConfig::default();
        cfg3.max_age.insert(DataSet::Prices, 3);
        assert!(state_of(&ctx_at(&repo, "2026-11-30"), &ledger, &cfg3, DataSet::Prices, "IBM").is_current(), "2 market days old, maximum 3");
        assert!(!state_of(&ctx_at(&repo, "2026-12-01"), &ledger, &cfg3, DataSet::Prices, "IBM").is_current(), "3 market days old: the age must be less than the maximum");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn the_ledger_makes_an_item_current_when_the_cache_has_no_record() {
        let repo = temp("ledger");
        let cfg = SyncConfig::default();
        let mut ledger = Ledger::in_memory();
        let ctx = ctx_at(&repo, "2026-11-19");
        assert_eq!(state_of(&ctx, &ledger, &cfg, DataSet::Estimates, "SPY"), State::Missing);
        ledger.set("estimates", "SPY", "2026-11-15");
        assert!(state_of(&ctx, &ledger, &cfg, DataSet::Estimates, "SPY").is_current(), "4 days, maximum 7");
        ledger.set("estimates", "SPY", "2026-11-10");
        let st = state_of(&ctx, &ledger, &cfg, DataSet::Estimates, "SPY");
        assert!(matches!(&st, State::Old { why, .. } if why.contains("9 days")), "{st:?}");
    }

    #[test]
    fn estimates_are_old_after_seven_calendar_days() {
        let repo = temp("est");
        let cache = repo.join("data/raw");
        let dir = cache.join("IBM/estimates");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("2026-11-12.json"), "{}").unwrap();
        let (cfg, ledger) = (SyncConfig::default(), Ledger::in_memory());
        assert!(state_of(&ctx_at(&repo, "2026-11-19"), &ledger, &cfg, DataSet::Estimates, "IBM").is_current(), "7 days");
        assert!(!state_of(&ctx_at(&repo, "2026-11-20"), &ledger, &cfg, DataSet::Estimates, "IBM").is_current(), "8 days");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn a_new_filing_makes_the_facts_old_whatever_the_date_of_the_look() {
        let repo = temp("sec");
        let cache = repo.join("data/raw");
        let dir = cache.join("IBM");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("company_facts.json.gz"), "x").unwrap();
        let facts_day = file_date(&dir.join("company_facts.json.gz")).unwrap();
        let today = dates::format(dates::parse(&facts_day).unwrap() + 1);
        let (cfg, ledger) = (SyncConfig::default(), Ledger::in_memory());
        let write_check = |checked: &str, filed: &str| {
            std::fs::write(
                dir.join("sec_8k_earnings.json"),
                format!(r#"{{"ticker":"IBM","checked":"{checked}","latest_periodic":{{"form":"10-Q","filed":"{filed}"}},"filings":[]}}"#),
            )
            .unwrap();
        };
        let ctx = ctx_at(&repo, &today);
        // No look at the filing list yet: the date is the date of the facts file (1 day old: current only on the same day).
        std::fs::remove_file(dir.join("sec_8k_earnings.json")).ok();
        assert!(matches!(state_of(&ctx, &ledger, &cfg, DataSet::Sec, "IBM"), State::Current(_) | State::Old { .. }));
        // A look today that shows a filing older than the facts: current.
        write_check(&today, &dates::format(dates::parse(&facts_day).unwrap() - 20));
        assert!(state_of(&ctx, &ledger, &cfg, DataSet::Sec, "IBM").is_current());
        // A look today that shows a filing of the day of the facts or later: old.
        write_check(&today, &facts_day);
        let st = state_of(&ctx, &ledger, &cfg, DataSet::Sec, "IBM");
        assert!(matches!(&st, State::Old { why, .. } if why.contains("newer than the cached facts")), "{st:?}");
        // The look of 3 market days ago is old, whatever it shows.
        let later = ctx_at(&repo, &dates::format(dates::parse(&today).unwrap() + 10));
        write_check(&today, &dates::format(dates::parse(&facts_day).unwrap() - 20));
        assert!(!state_of(&later, &ledger, &cfg, DataSet::Sec, "IBM").is_current());
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn the_warning_names_the_old_data_and_the_command() {
        let repo = temp("warn");
        let cache = repo.join("data/raw");
        write_prices(&cache, "IBM", "2020-01-02", "2020-01-02");
        let text = old_data_warning(&repo, "ibm", &[DataSet::Prices, DataSet::Estimates]).unwrap();
        assert!(text.contains("warning: old data for IBM: prices from 2020-01-02") && text.ends_with("run: fa sync IBM"), "{text}");
        assert!(!text.contains("estimates"), "a missing cache gives no warning: {text}");
        write_prices(&cache, "IBM", &dates::format(dates::today()), "2020-01-02");
        assert_eq!(old_data_warning(&repo, "IBM", &[DataSet::Prices]), None);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn bonds_item_is_the_last_market_day_before_today() {
        let repo = temp("bonds");
        let ctx = ctx_at(&repo, "2026-11-23"); // a Monday
        let cfg = SyncConfig::default();
        assert_eq!(keys_of(DataSet::Bonds, &ctx, &cfg, &[], &[], &[]), ["2026-11-20"]);
        let ledger = Ledger::in_memory();
        assert_eq!(state_of(&ctx, &ledger, &cfg, DataSet::Bonds, "2026-11-20"), State::Missing);
        let dir = repo.join("data/raw/prices");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fedinvest_2026-11-20.csv"), "x").unwrap();
        assert!(state_of(&ctx, &ledger, &cfg, DataSet::Bonds, "2026-11-20").is_current());
        let _ = std::fs::remove_dir_all(&repo);
    }
}
