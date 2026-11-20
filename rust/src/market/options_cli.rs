//! `fa options`: option chain snapshots from a second source, and the option surface of past dates.
//!
//! - `fa options snapshot SYMBOL [--source cboe|alpaca]`: today's chain, written to `data/raw/<T>/options/<date>.json`.
//!   `cboe` (no key) is the delayed quotes file that `fa analyze` uses; `alpaca` is the indicative feed of the free plan.
//! - `fa options history SYMBOL --date D | --from D --to D`: the surface of past dates from the daily closes of the contracts
//!   (`option_history`). The files have the form of the live snapshots and `quote_kind: "close"`. A live snapshot
//!   (real bid and ask) of the same date is never replaced by closes.

use super::alpaca::{self, Client, Keys};
use super::option_history::{self, AlpacaHistory, OptionHistorySource, Selection};
use super::options::{self, OptionChainSnapshot, OptionQuote};
use super::{download, today_iso, ticker_dir};
use crate::trading::calendar::{iso, parse_iso, weekday};
use crate::trading::chain::parse_option;
use std::collections::BTreeMap;
use std::path::Path;

pub const USAGE: &str = "\
fa options - option chain snapshots and the option surface of past dates (data/raw/<TICKER>/options/<date>.json)

USAGE:
  fa options snapshot SYMBOL [--source cboe|alpaca] [--roots A,B] [--rate R] [--refresh]
                              today's chain. cboe: delayed quotes, no key. alpaca: the free 'indicative' feed
                              (needs APCA_API_KEY_ID and APCA_API_SECRET_KEY); bid and ask are modified quotes.
                              Alpaca has index options (SPX, VIX) but no index level and no implied volatility for them:
                              the program takes the level and the volatility from put-call parity of the chain (Black-76,
                              European options). The roots of the chain: for SPX and VIX the monthly and the weekly root
                              (SPX,SPXW and VIX,VIXW); for another symbol the symbol. --roots A,B sets them.
                              --rate R is one flat rate (decimal) for the index volatility; default: the Treasury curve.
  fa options check SYMBOL --date D [--rate R] [--top N]
                              no-arbitrage checks of a stored chain at executable prices (buy at the ask, sell at the bid):
                              monotony, slope, butterfly, calendar, parity bound; no network. A break is not a profit.
  fa options history SYMBOL (--date D | --from D --to D) [--max-days N] [--moneyness LO,HI] [--rate R] [--level S:RATIO] [--refresh] [--plan]
                              the surface of past dates from the daily closes of the contracts (Alpaca option bars,
                              since 2024-02). One snapshot file for each trading day, with quote_kind \"close\":
                              the close of the day in last_price, no bid, no ask, implied volatility of Black-76
                              (European model; the options are American). A live snapshot of the same date stays.
                              Default selection: expiry up to 120 days, strike 0.9 to 1.1 times the spot.
                              Index (SPX, VIX): no price of the index at Alpaca. The level is the SPY close times 10 (SPX; picks
                              the strike range only) or the Cboe VIX history file (VIX; run fa sync --only vix). --level S:RATIO
                              gives another symbol and ratio. The contract list of Alpaca holds expired index contracts for the
                              last six months only, so for older dates the program makes the symbols from the rules (SPXW on each
                              weekday, SPX on the third Friday, strikes every 5 points; VIX on Wednesdays, every 0.5 point).
                              --rate R is one flat rate (decimal); default: the Treasury par curve of each date.
                              --plan lists the contracts and the number of requests, and gets no closes. A run prints
                              the same plan to stderr before it asks for closes.
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
pub fn chain_snapshot(items: &[alpaca::ChainItem], ticker: &str, date: &str, spot: Option<f64>, feed: &str, roots: &[String]) -> OptionChainSnapshot {
    let mut quotes = Vec::new();
    for i in items {
        let Some((expiry, call, thousandths)) = roots.iter().find_map(|r| parse_option(&i.symbol, r)) else { continue };
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

/// The text of an optional number for a line to a person.
pub fn show_opt(v: Option<f64>) -> String {
    v.map_or_else(|| "none".to_string(), |x| format!("{x}"))
}

/// The rate function of the options commands: one flat rate, or the Treasury par curve of the date (linear in the time).
pub fn rate_fn(repo: &Path, flat: Option<f64>) -> impl FnMut(&str, f64) -> Result<f64, String> {
    let mut rates = crate::rates::Rates::live(repo);
    let mut memo: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    move |date: &str, t: f64| -> Result<f64, String> {
        if let Some(r) = flat {
            return Ok(r);
        }
        if !memo.contains_key(date) {
            let day = super::parse_date(date).ok_or_else(|| format!("bad date {date}"))?;
            let (_, curve) = rates.treasury_curve(Some(day), false).map_err(|e| format!("{e}; give --rate R (a flat rate, decimal) to go on"))?;
            memo.insert(date.to_string(), curve);
        }
        interpolate_curve(&memo[date], t).ok_or_else(|| format!("no Treasury rate for {date}; give --rate R"))
    }
}

/// Today's chain from Alpaca (the free indicative feed) into `<cache>/<TICKER>/options/<today>.json`. Gives the line that
/// the command prints. A snapshot of today with real quotes stays (`refresh` does not replace it).
pub fn snapshot_alpaca(repo: &Path, cache: &Path, ticker: &str, today: &str, refresh: bool) -> Result<String, String> {
    snapshot_alpaca_with(repo, cache, ticker, today, refresh, None, None)
}

/// `snapshot_alpaca` with the roots (default: the roots of the underlying, see `option_history::roots_of`) and a flat rate.
pub fn snapshot_alpaca_with(repo: &Path, cache: &Path, ticker: &str, today: &str, refresh: bool, roots: Option<Vec<String>>, flat_rate: Option<f64>) -> Result<String, String> {
    if !may_write(cache, ticker, today, refresh)? {
        return Ok(format!("options {ticker} {today}: a snapshot of today exists; --refresh replaces it"));
    }
    let keys = Keys::read(repo).ok_or_else(|| format!("{} and {} are not both set in the environment or in .env", alpaca::KEY_ID_ENV, alpaca::SECRET_ENV))?;
    let client = Client::live(keys);
    let index = option_history::index_info(ticker).is_some();
    let roots = roots.unwrap_or_else(|| option_history::roots_of(ticker));
    // An index has no price at Alpaca: the spot stays empty and the level comes from put-call parity below.
    let spot = if index {
        None
    } else {
        let now = alpaca::now_utc();
        let from = iso((now.div_euclid(86_400) - 10) as i32);
        let bars = client.stock_bars(&alpaca::StockQuery { symbols: &[alpaca::api_symbol(ticker)], timeframe: "1Day", start: &from, to: today, today, adjustment: "raw", feed: &alpaca::feed_from_env() }).map_err(|e| e.to_string())?;
        bars.get(&alpaca::api_symbol(ticker)).and_then(|v| v.last()).map(|b| b.c)
    };
    let mut items = Vec::new();
    for root in &roots {
        items.extend(client.chain(root, "indicative").map_err(|e| e.to_string())?);
    }
    let mut snap = chain_snapshot(&items, ticker, today, spot, "indicative", &roots);
    let mut extra = String::new();
    if index {
        let mut rate = rate_fn(repo, flat_rate);
        let fill = option_history::fill_from_mids(&mut snap, &mut rate)?;
        snap.note = Some(format!(
            "Chain snapshot of Alpaca (roots {}). Index options: Alpaca gives no index level and no implied volatility. The forward of each expiry is from put-call parity of the mids of bid and ask ({} of {} series, one root and one expiry); the implied volatility is Black-76 on that forward for out-of-the-money and near-the-money quotes ({} quotes); the spot is {}. No open interest.",
            roots.join(","),
            fill.expiries_with_parity,
            fill.expiries,
            fill.filled,
            match &fill.level_from {
                Some((r, e, n, f)) => format!("the forward {f:.2} of the series {r} {e} ({n} days away)"),
                None => "not known (no series within 10 days has a forward)".to_string(),
            }
        ));
        extra = format!(", implied volatility for {} quotes from put-call parity", fill.filled);
    }
    let path = options::save_snapshot(&snap, cache)?;
    Ok(format!("options {ticker} {today}: {} quotes, spot {}, {} requests, feed indicative{extra}, written to {}", snap.quotes.len(), show_opt(snap.spot_price), client.requests(), path.display()))
}

fn snapshot(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let a = Args::parse(rest, &["--source", "--roots", "--rate"], &["--refresh"])?;
    let ticker = a.symbol.clone().ok_or("give a symbol: fa options snapshot SPY")?;
    let cache = super::price_cli::cache_dir(repo);
    let today = today_iso();
    match a.flag("--source").unwrap_or("cboe") {
        "cboe" => {
            let (snap, source) = options::fetch_option_chain(&cache, &ticker, a.has("--refresh"), &today, &|u| download(u))?;
            println!("options {ticker} {today}: {} quotes, spot {}, source {:?} (Cboe delayed quotes)", snap.quotes.len(), show_opt(snap.spot_price), source);
        }
        "alpaca" => {
            let roots = a.flag("--roots").map(|r| r.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect::<Vec<_>>());
            let flat = a.flag("--rate").map(|r| r.parse::<f64>().map_err(|_| format!("bad --rate '{r}'"))).transpose()?;
            println!("{}", snapshot_alpaca_with(repo, &cache, &ticker, &today, a.has("--refresh"), roots, flat)?);
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
    let a = Args::parse(rest, &["--date", "--from", "--to", "--max-days", "--moneyness", "--rate", "--source", "--level"], &["--refresh", "--plan"])?;
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
    let cache = super::price_cli::cache_dir(repo);
    let given_level = a
        .flag("--level")
        .map(|l| {
            let (sym, ratio) = l.split_once(':').ok_or("--level takes SYMBOL:RATIO, for example SPY:10")?;
            let ratio: f64 = ratio.parse().map_err(|_| format!("bad --level ratio '{ratio}'"))?;
            if !(ratio.is_finite() && ratio > 0.0) {
                return Err("--level needs a ratio above 0".to_string());
            }
            Ok((sym.to_string(), ratio))
        })
        .transpose()?;
    let level = option_history::level_for(&ticker, &cache, given_level.as_ref().map(|(s, r)| (s.as_str(), *r)));
    let src = AlpacaHistory { client: Client::live(keys), feed: alpaca::feed_from_env(), now: alpaca::now_utc(), level };
    let refresh = a.has("--refresh");

    if a.has("--plan") {
        let spots = src.spot_closes(&ticker, &from, &to)?;
        let q = option_history::contract_query(src.name(), &ticker, &spots, &from, &to, &sel)?;
        let n = src.contracts(&ticker, &q)?.len();
        let bars = n.div_ceil(src.bars_per_request());
        println!("plan {ticker} {from} to {to}: {} trading days with a level, {n} contracts (expiry {} to {}, strike {} to {}); at least {bars} requests of closes ({} contracts in a request); {} requests used so far; the free plan allows 200 each minute", spots.len(), q.expiry_from, q.expiry_to, q.strike_min, q.strike_max, src.bars_per_request(), src.requests());
        return Ok(0);
    }

    let mut rate = rate_fn(repo, flat_rate);

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

/// `fa options check T --date D [--rate R]`: the no-arbitrage checks of a stored chain at executable prices (`option_arb`).
fn check(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let a = Args::parse(rest, &["--date", "--rate", "--top"], &[])?;
    let ticker = a.symbol.clone().ok_or("give a symbol: fa options check IBM --date 2026-11-19")?;
    let date = a.flag("--date").ok_or("give --date D (a stored snapshot)")?;
    let flat = a.flag("--rate").map(|r| r.parse::<f64>().map_err(|_| format!("bad --rate '{r}'"))).transpose()?;
    let top = a.flag("--top").map(|n| n.parse::<usize>().map_err(|_| format!("bad --top '{n}'"))).transpose()?.unwrap_or(10);
    let cache = super::price_cli::cache_dir(repo);
    let snap = options::load_snapshot(&cache, &ticker, Some(date))?;
    let american = option_history::index_info(&ticker).is_none();
    let mut rate = rate_fn(repo, flat);
    let rep = super::option_arb::check_chain(&snap, american, &mut rate)?;
    print!("{}", super::option_arb::render(&ticker, &snap, &rep, american, top));
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
        Some("check") => check(repo, &args[1..]),
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
        let s = chain_snapshot(&items, "spy", "2024-04-01", Some(505.0), "indicative", &["SPY".to_string()]);
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

    #[test]
    fn an_option_value_is_printed_as_a_number_or_none() {
        assert_eq!(show_opt(Some(706.8)), "706.8");
        assert_eq!(show_opt(None), "none");
    }

    #[test]
    fn index_chain_has_both_roots_and_no_adjusted_root() {
        let items = vec![
            alpaca::ChainItem { symbol: "SPX261120C06500000".into(), ..Default::default() },
            alpaca::ChainItem { symbol: "SPXW261123P03000000".into(), ..Default::default() },
            alpaca::ChainItem { symbol: "SPXO261118C06300000".into(), ..Default::default() },
        ];
        let roots = option_history::roots_of("SPX");
        let s = chain_snapshot(&items, "SPX", "2026-11-20", None, "indicative", &roots);
        assert_eq!(s.quotes.len(), 2);
        assert!(s.spot_price.is_none());
        assert_eq!(chain_snapshot(&items, "SPX", "2026-11-20", None, "indicative", &["SPX".to_string()]).quotes.len(), 1);
    }
}
