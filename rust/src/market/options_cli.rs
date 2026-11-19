//! `fa options`: option chain snapshots from a second source, and the option surface of past dates.
//!
//! - `fa options snapshot SYMBOL [--source cboe|alpaca]`: today's chain, written to `data/raw/<T>/options/<date>.json`.
//!   `cboe` (no key) is the delayed quotes file that `fa analyze` uses; `alpaca` is the indicative feed of the free plan.
//! - `fa options history SYMBOL --date D | --from D --to D`: the surface of past dates from the daily closes of the contracts
//!   (`option_history`). The files have the form of the live snapshots and `quote_kind: "close"`. A live snapshot
//!   (real bid and ask) of the same date is never replaced by closes.

use super::alpaca::{self, Client, Keys};
use super::option_history::{self, AlpacaHistory, ContractQuery, OptionHistorySource, Selection};
use super::options::{self, OptionChainSnapshot, OptionQuote};
use super::{download, today_iso, ticker_dir};
use crate::trading::calendar::{iso, parse_iso, weekday};
use crate::trading::chain::parse_option;
use std::collections::BTreeMap;
use std::path::Path;

pub const USAGE: &str = "\
fa options - option chain snapshots and the option surface of past dates (data/raw/<TICKER>/options/<date>.json)

USAGE:
  fa options snapshot SYMBOL [--source cboe|alpaca] [--refresh]
                              today's chain. cboe: delayed quotes, no key. alpaca: the free 'indicative' feed
                              (needs APCA_API_KEY_ID and APCA_API_SECRET_KEY); bid and ask are modified quotes.
  fa options history SYMBOL (--date D | --from D --to D) [--max-days N] [--moneyness LO,HI] [--rate R] [--refresh] [--plan]
                              the surface of past dates from the daily closes of the contracts (Alpaca option bars,
                              since 2024-02). One snapshot file for each trading day, with quote_kind \"close\":
                              the close of the day in last_price, no bid, no ask, implied volatility of Black-76
                              (European model; the options are American). A live snapshot of the same date stays.
                              Default selection: expiry up to 365 days, strike 0.8 to 1.2 times the spot.
                              --rate R is one flat rate (decimal); default: the Treasury par curve of each date.
                              --plan lists the contracts and the number of requests, and gets no closes.
                              A stopped run continues when you run it again (a month with its snapshot files is skipped).";

#[derive(Debug, Default)]
struct Args {
    symbol: Option<String>,
    flags: Vec<(String, String)>,
    switches: Vec<String>,
}

impl Args {
    fn parse(rest: &[String], values: &[&str], switches: &[&str]) -> Result<Args, String> {
        let mut a = Args::default();
        let mut i = 0;
        while i < rest.len() {
            let x = &rest[i];
            if values.contains(&x.as_str()) {
                let v = rest.get(i + 1).filter(|v| !v.starts_with("--")).ok_or_else(|| format!("{x} needs a value"))?;
                a.flags.push((x.clone(), v.clone()));
                i += 2;
            } else if switches.contains(&x.as_str()) {
                a.switches.push(x.clone());
                i += 1;
            } else if x.starts_with("--") {
                return Err(format!("unknown option {x}; see fa options --help"));
            } else {
                if a.symbol.replace(x.to_uppercase()).is_some() {
                    return Err("give one symbol".into());
                }
                i += 1;
            }
        }
        Ok(a)
    }
    fn flag(&self, n: &str) -> Option<&str> {
        self.flags.iter().find(|(k, _)| k == n).map(|(_, v)| v.as_str())
    }
    fn has(&self, n: &str) -> bool {
        self.switches.iter().any(|s| s == n)
    }
}

/// The chain of the Alpaca snapshot reply as a snapshot file. The quotes are the modified quotes of the free feed.
pub fn chain_snapshot(items: &[alpaca::ChainItem], ticker: &str, date: &str, spot: Option<f64>, feed: &str) -> OptionChainSnapshot {
    let mut quotes = Vec::new();
    let root = ticker.to_uppercase();
    for i in items {
        let Some((expiry, call, thousandths)) = parse_option(&i.symbol, &root) else { continue };
        let strike = thousandths as f64 / 1000.0;
        quotes.push(OptionQuote {
            contract_symbol: i.symbol.clone(),
            option_type: if call { "call" } else { "put" }.into(),
            expiry: iso(expiry),
            strike,
            last_price: i.last,
            bid: i.bid,
            ask: i.ask,
            volume: i.volume.map(|v| v as i64),
            open_interest: None,
            implied_volatility: i.iv,
            in_the_money: spot.map(|s| if call { strike < s } else { strike > s }),
        });
    }
    quotes.sort_by(|a, b| (&a.expiry, &a.option_type).cmp(&(&b.expiry, &b.option_type)).then(a.strike.total_cmp(&b.strike)));
    OptionChainSnapshot {
        ticker: ticker.to_uppercase(),
        snapshot_date: date.to_string(),
        spot_price: spot,
        quotes,
        source: Some("alpaca".into()),
        feed: Some(feed.to_string()),
        quote_kind: Some(if feed == "indicative" { "indicative" } else { "bid_ask" }.into()),
        note: Some("Chain snapshot of Alpaca. The spot is the last daily close of the IEX feed. The implied volatility is the one of Alpaca. No open interest.".into()),
    }
}

/// May the file of this date be written? A file with real quotes is kept. A file of closes is kept unless `refresh`.
pub fn may_write(cache: &Path, ticker: &str, date: &str, refresh: bool) -> Result<bool, String> {
    if !options::snapshot_path(cache, ticker, date).is_file() {
        return Ok(true);
    }
    match options::load_snapshot(cache, ticker, Some(date)) {
        Ok(s) if s.quote_kind.as_deref() == Some("close") => Ok(refresh),
        Ok(_) => Ok(false),
        Err(_) => Ok(true), // a corrupt file is replaced
    }
}

fn snapshot(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let a = Args::parse(rest, &["--source"], &["--refresh"])?;
    let ticker = a.symbol.clone().ok_or("give a symbol: fa options snapshot SPY")?;
    let cache = super::price_cli::cache_dir(repo);
    let today = today_iso();
    match a.flag("--source").unwrap_or("cboe") {
        "cboe" => {
            let (snap, source) = options::fetch_option_chain(&cache, &ticker, a.has("--refresh"), &today, &|u| download(u))?;
            println!("options {ticker} {today}: {} quotes, spot {:?}, source {:?} (Cboe delayed quotes)", snap.quotes.len(), snap.spot_price, source);
        }
        "alpaca" => {
            if !may_write(&cache, &ticker, &today, a.has("--refresh"))? {
                println!("options {ticker} {today}: a snapshot of today exists; --refresh replaces it");
                return Ok(0);
            }
            let keys = Keys::read(repo).ok_or_else(|| format!("{} and {} are not both set in the environment or in .env", alpaca::KEY_ID_ENV, alpaca::SECRET_ENV))?;
            let client = Client::live(keys);
            let now = alpaca::now_utc();
            let from = iso((now.div_euclid(86_400) - 10) as i32);
            let bars = client.stock_bars(&alpaca::StockQuery { symbols: &[alpaca::api_symbol(&ticker)], timeframe: "1Day", start: &from, to: &today, today: &today, adjustment: "raw", feed: &alpaca::feed_from_env() }).map_err(|e| e.to_string())?;
            let spot = bars.get(&alpaca::api_symbol(&ticker)).and_then(|v| v.last()).map(|b| b.c);
            let items = client.chain(&ticker, "indicative").map_err(|e| e.to_string())?;
            let snap = chain_snapshot(&items, &ticker, &today, spot, "indicative");
            let path = options::save_snapshot(&snap, &cache)?;
            println!("options {ticker} {today}: {} quotes, spot {:?}, {} requests, feed indicative, written to {}", snap.quotes.len(), snap.spot_price, client.requests(), path.display());
        }
        other => return Err(format!("unknown source '{other}' (use cboe or alpaca)")),
    }
    Ok(0)
}

/// The years of a tenor of the Treasury curve.
fn tenor_years(t: &str) -> Option<f64> {
    let (n, unit) = t.split_at(t.len().checked_sub(1)?);
    let n: f64 = n.parse().ok()?;
    match unit {
        "M" => Some(n / 12.0),
        "Y" => Some(n),
        _ => None,
    }
}

/// The par yield at `t` years: linear between the tenors, flat outside them.
pub fn interpolate_curve(curve: &BTreeMap<String, f64>, t: f64) -> Option<f64> {
    let mut points: Vec<(f64, f64)> = curve.iter().filter_map(|(k, v)| Some((tenor_years(k)?, *v))).collect();
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (first, last) = (points.first()?, points.last()?);
    if t <= first.0 {
        return Some(first.1);
    }
    if t >= last.0 {
        return Some(last.1);
    }
    let i = points.windows(2).position(|w| t >= w[0].0 && t <= w[1].0)?;
    let (a, b) = (points[i], points[i + 1]);
    Some(a.1 + (b.1 - a.1) * (t - a.0) / (b.0 - a.0))
}

fn weekdays(a: &str, b: &str) -> Vec<String> {
    let (Some(x), Some(y)) = (parse_iso(a), parse_iso(b)) else { return Vec::new() };
    (x..=y).filter(|d| weekday(*d) < 5).map(iso).collect()
}

fn history(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let a = Args::parse(rest, &["--date", "--from", "--to", "--max-days", "--moneyness", "--rate", "--source"], &["--refresh", "--plan"])?;
    let ticker = a.symbol.clone().ok_or("give a symbol: fa options history SPY --date 2024-06-07")?;
    let (from, to) = match (a.flag("--date"), a.flag("--from"), a.flag("--to")) {
        (Some(d), None, None) => (d.to_string(), d.to_string()),
        (None, Some(f), Some(t)) => (f.to_string(), t.to_string()),
        _ => return Err("give --date D, or --from D --to D".into()),
    };
    if parse_iso(&from).is_none() || parse_iso(&to).is_none() || from > to {
        return Err(format!("bad range {from} to {to}"));
    }
    if a.flag("--source").is_some_and(|s| s != "alpaca") {
        return Err("the option history comes from alpaca only (see docs/ARCHITECTURE.md for Massive)".into());
    }
    let mut sel = Selection::default();
    if let Some(n) = a.flag("--max-days") {
        sel.max_days = n.parse().map_err(|_| format!("bad --max-days '{n}'"))?;
    }
    if let Some(m) = a.flag("--moneyness") {
        let (lo, hi) = m.split_once(',').ok_or("--moneyness takes LO,HI, for example 0.8,1.2")?;
        sel.money_lo = lo.parse().map_err(|_| format!("bad --moneyness '{m}'"))?;
        sel.money_hi = hi.parse().map_err(|_| format!("bad --moneyness '{m}'"))?;
        if !(sel.money_lo > 0.0 && sel.money_lo < sel.money_hi) {
            return Err("--moneyness needs 0 < LO < HI".into());
        }
    }
    let flat_rate: Option<f64> = a.flag("--rate").map(|r| r.parse().map_err(|_| format!("bad --rate '{r}'"))).transpose()?;
    let keys = Keys::read(repo).ok_or_else(|| format!("{} and {} are not both set in the environment or in .env", alpaca::KEY_ID_ENV, alpaca::SECRET_ENV))?;
    let src = AlpacaHistory { client: Client::live(keys), feed: alpaca::feed_from_env(), now: alpaca::now_utc() };
    let cache = super::price_cli::cache_dir(repo);
    let refresh = a.has("--refresh");

    if a.has("--plan") {
        let spots = src.spot_closes(&ticker, &from, &to)?;
        let (smin, smax) = spots.values().fold((f64::MAX, 0.0f64), |(x, y), s| (x.min(*s), y.max(*s)));
        let last = parse_iso(&to).unwrap();
        let q = ContractQuery { expiry_from: from.clone(), expiry_to: iso(last + sel.max_days as i32), strike_min: (smin * sel.money_lo).floor(), strike_max: (smax * sel.money_hi).ceil() };
        let n = src.contracts(&ticker, &q)?.len();
        let bars = n.div_ceil(src.bars_per_request());
        println!("plan {ticker} {from} to {to}: {} trading days with a close, {n} contracts (expiry {} to {}, strike {} to {}); at least {bars} requests of closes; {} requests used so far; the free plan allows 200 each minute", spots.len(), q.expiry_from, q.expiry_to, q.strike_min, q.strike_max, src.requests());
        return Ok(0);
    }

    let mut rates = crate::rates::Rates::live(repo);
    let mut memo: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    let mut rate = |date: &str, t: f64| -> Result<f64, String> {
        if let Some(r) = flat_rate {
            return Ok(r);
        }
        if !memo.contains_key(date) {
            let day = super::parse_date(date).ok_or_else(|| format!("bad date {date}"))?;
            let (_, curve) = rates.treasury_curve(Some(day), false).map_err(|e| format!("{e}; give --rate R (a flat rate, decimal) to go on"))?;
            memo.insert(date.to_string(), curve);
        }
        interpolate_curve(&memo[date], t).ok_or_else(|| format!("no Treasury rate for {date}; give --rate R"))
    };

    let (mut written, mut kept, mut skipped_months) = (0, 0, 0);
    for (a_day, b_day) in super::intraday::month_windows(&from, &to) {
        let days = weekdays(&a_day, &b_day);
        if let (Some(first), Some(last)) = (days.first(), days.last()) {
            let have = |d: &String| options::snapshot_path(&cache, &ticker, d).is_file();
            if !refresh && have(first) && have(last) {
                skipped_months += 1;
                eprintln!("fa: {a_day} to {b_day}: snapshot files exist; skipped");
                continue;
            }
        }
        let built = option_history::build_range(&src, &ticker, &a_day, &b_day, &sel, &mut rate)?;
        for d in &built.days {
            if may_write(&cache, &ticker, &d.snapshot.snapshot_date, refresh)? {
                options::save_snapshot(&d.snapshot, &cache)?;
                written += 1;
            } else {
                kept += 1;
            }
        }
        eprintln!("fa: {a_day} to {b_day}: {} days, {} contracts listed, {} with a close, {} requests so far", built.days.len(), built.contracts_listed, built.contracts_with_bars, built.requests);
    }
    println!("options history {ticker} {from} to {to}: {written} snapshot files written, {kept} kept (a file exists), {skipped_months} months skipped, {} requests, in {}", src.requests(), ticker_dir(&cache, &ticker).join("options").display());
    Ok(0)
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(0)
        }
        Some("snapshot") => snapshot(repo, &args[1..]),
        Some("history") => history(repo, &args[1..]),
        Some(other) => Err(format!("unknown options command '{other}'; see fa options --help")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpaca_chain_becomes_a_snapshot_in_the_live_form() {
        let items = vec![
            alpaca::ChainItem { symbol: "SPY240419C00500000".into(), bid: Some(4.15), ask: Some(4.3), last: Some(4.1), iv: Some(0.14), volume: Some(120.0) },
            alpaca::ChainItem { symbol: "SPY240419P00500000".into(), ..Default::default() },
            alpaca::ChainItem { symbol: "SPY1240419C00500000".into(), ..Default::default() },
        ];
        let s = chain_snapshot(&items, "spy", "2024-04-01", Some(505.0), "indicative");
        assert_eq!(s.quotes.len(), 2, "an adjusted root is not in the chain");
        assert_eq!(s.quotes[0].option_type, "call");
        assert_eq!((s.quotes[0].strike, s.quotes[0].expiry.as_str()), (500.0, "2024-04-19"));
        assert_eq!((s.source.as_deref(), s.feed.as_deref(), s.quote_kind.as_deref()), (Some("alpaca"), Some("indicative"), Some("indicative")));
        // The file has the old fields, so the older readers still read it; the new fields are optional.
        let text = serde_json::to_string(&s).unwrap();
        let back: OptionChainSnapshot = serde_json::from_str(&text).unwrap();
        assert_eq!(back, s);
        let old = r#"{"ticker":"IBM","snapshot_date":"2026-01-02","spot_price":1.0,"quotes":[]}"#;
        let parsed: OptionChainSnapshot = serde_json::from_str(old).unwrap();
        assert!(parsed.source.is_none() && parsed.quote_kind.is_none());
        assert!(!serde_json::to_string(&parsed).unwrap().contains("quote_kind"), "no new field in an old file");
    }

    #[test]
    fn a_live_snapshot_is_never_replaced_by_closes() {
        let dir = std::env::temp_dir().join(format!("fa_oc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(may_write(&dir, "SPY", "2024-03-06", false).unwrap(), "no file");
        let mut s = OptionChainSnapshot { ticker: "SPY".into(), snapshot_date: "2024-03-06".into(), spot_price: Some(1.0), quotes: vec![], source: None, feed: None, quote_kind: None, note: None };
        options::save_snapshot(&s, &dir).unwrap();
        assert!(!may_write(&dir, "SPY", "2024-03-06", true).unwrap(), "a Cboe file stays, even with refresh");
        s.quote_kind = Some("close".into());
        options::save_snapshot(&s, &dir).unwrap();
        assert!(!may_write(&dir, "SPY", "2024-03-06", false).unwrap(), "a file of closes stays unless refresh");
        assert!(may_write(&dir, "SPY", "2024-03-06", true).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_rate_from_the_curve() {
        let curve: BTreeMap<String, f64> = [("1M", 0.052), ("3M", 0.053), ("1Y", 0.048), ("10Y", 0.042)].iter().map(|(k, v)| (k.to_string(), *v)).collect();
        assert_eq!(interpolate_curve(&curve, 0.01), Some(0.052));
        assert_eq!(interpolate_curve(&curve, 40.0), Some(0.042));
        let mid = interpolate_curve(&curve, 0.625).unwrap();
        assert!(mid < 0.053 && mid > 0.048, "{mid}");
        assert_eq!(tenor_years("6M"), Some(0.5));
        assert_eq!(tenor_years("x"), None);
        assert_eq!(interpolate_curve(&BTreeMap::new(), 1.0), None);
    }

    #[test]
    fn arguments() {
        let v = |x: &[&str]| x.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let a = Args::parse(&v(&["spy", "--date", "2024-06-07", "--plan"]), &["--date"], &["--plan"]).unwrap();
        assert_eq!((a.symbol.as_deref(), a.flag("--date"), a.has("--plan")), (Some("SPY"), Some("2024-06-07"), true));
        assert!(Args::parse(&v(&["spy", "--bad"]), &[], &[]).is_err());
        assert!(Args::parse(&v(&["spy", "qqq"]), &[], &[]).is_err());
        assert_eq!(weekdays("2024-03-01", "2024-03-05"), ["2024-03-01", "2024-03-04", "2024-03-05"]);
    }
}
