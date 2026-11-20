//! The option surface of a past date, rebuilt from the daily closes of the contracts.
//!
//! The question: "we lose the option surface if we do not copy it each day". The answer of this module: a source that
//! gives the PAST on request (the historical option bars of Alpaca, since February 2024; the Massive aggregates for the paid
//! plans) gives the daily close of each contract. With the close of the underlying of the same day the program calculates the
//! implied volatility of each contract (`option_iv`) and writes a dated snapshot in the form of the live snapshots
//! (`options::OptionChainSnapshot`, `data/raw/<T>/options/<date>.json`) with `quote_kind: "close"`.
//!
//! What a snapshot of closes is, and is not:
//!
//! - A quote has the close of the day in `last_price`. It has no bid and no ask, so the metrics that need a bid and an ask (the
//!   spread) do not exist, and the straddle of `compute_chain_metrics` is a sum of two closes.
//! - A contract with no trade that day has no bar, and it is not in the snapshot. A close is the price of the last trade, at any
//!   time of the day.
//! - The implied volatility is the one of Black-76 with a forward from put-call parity (see `option_iv`). It is a European model;
//!   SPY options are American. The volatility is given for the options out of the money and near the money.
//! - The close of the underlying comes from the same source (`raw` prices; the free feed is IEX, so its close is the last IEX
//!   trade and can differ from the official close by a few cents).
//!
//! The source is a trait (`OptionHistorySource`), so another source (Massive, see `docs/ARCHITECTURE.md`) is one more
//! `impl`. The cost is the number of requests: `bars_per_request` contracts for each request (Alpaca 100; Massive 1), so the
//! builder asks for a RANGE of dates in one request for each batch of contracts. One date or a whole month cost
//! nearly the same number of contract requests.

use super::alpaca::{self, Client, ContractInfo};
use super::option_iv::{forward_from_parity, forward_from_parity_blind, implied_vol};
use super::options::{OptionChainSnapshot, OptionQuote};
use super::round6;
use crate::trading::calendar::{iso, parse_iso, weekday};
use crate::trading::chain::parse_option;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Contract {
    pub symbol: String,
    pub expiry: String,
    pub call: bool,
    pub strike: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayClose {
    pub symbol: String,
    pub date: String,
    pub close: f64,
    pub volume: f64,
}

/// The contracts that the builder wants: an expiry in a window, a strike in a range.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractQuery {
    pub expiry_from: String,
    pub expiry_to: String,
    pub strike_min: f64,
    pub strike_max: f64,
}

pub trait OptionHistorySource {
    fn name(&self) -> &'static str;
    /// Contracts in one request of closes.
    fn bars_per_request(&self) -> usize;
    /// The close of the underlying for each day of the range (New York date).
    fn spot_closes(&self, underlying: &str, from: &str, to: &str) -> Result<BTreeMap<String, f64>, String>;
    /// The contracts of the underlying that exist in the query window (live and expired).
    fn contracts(&self, underlying: &str, query: &ContractQuery) -> Result<Vec<Contract>, String>;
    /// The daily closes of one batch of contracts in the range. A day with no trade has no row.
    fn closes(&self, contracts: &[Contract], from: &str, to: &str) -> Result<Vec<DayClose>, String>;
    /// The requests so far.
    fn requests(&self) -> usize;
    /// A line for the person before the long part of a run (the plan). The default prints nothing.
    fn announce(&self, _line: &str) {}
    /// A text for the snapshot note when the level of the underlying is not a price of the underlying (an index).
    fn level_note(&self) -> Option<String> {
        None
    }
}

/// The default selection (see `docs/ARCHITECTURE.md`, "Keyed data sources", for the measured request counts).
pub const DEFAULT_MAX_DAYS: i64 = 120;
pub const DEFAULT_MONEY_LO: f64 = 0.9;
pub const DEFAULT_MONEY_HI: f64 = 1.1;

/// The contracts and the dates that the builder keeps.
#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    /// Contracts with an expiry up to this many calendar days after the date.
    pub max_days: i64,
    /// Strikes from `lo` times the spot to `hi` times the spot.
    pub money_lo: f64,
    pub money_hi: f64,
}

impl Default for Selection {
    fn default() -> Self {
        Selection { max_days: DEFAULT_MAX_DAYS, money_lo: DEFAULT_MONEY_LO, money_hi: DEFAULT_MONEY_HI }
    }
}

/// The forward of one expiry on one date and where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct ForwardInfo {
    pub expiry: String,
    pub forward: f64,
    pub from_parity: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayBuild {
    pub snapshot: OptionChainSnapshot,
    pub forwards: Vec<ForwardInfo>,
}

#[derive(Debug)]
pub struct Built {
    pub days: Vec<DayBuild>,
    pub contracts_listed: usize,
    pub contracts_with_bars: usize,
    pub requests: usize,
}

/// Rate (decimal, continuous) for a date and a time to expiry in years.
pub type RateFn<'a> = &'a mut dyn FnMut(&str, f64) -> Result<f64, String>;

/// The contract of the symbol `SPY240419C00500000` with the data of the contract list.
pub fn contract_from_info(underlying: &str, c: &ContractInfo) -> Option<Contract> {
    // The root must be one of the roots of the underlying: an adjusted contract (`SPY1...`) has another root and is not in the surface.
    roots_of(underlying).iter().find_map(|r| parse_option(&c.symbol, r))?;
    Some(Contract { symbol: c.symbol.clone(), expiry: c.expiry.clone(), call: c.call, strike: c.strike })
}

/// One date: the filter, the forward of each expiry, the implied volatility of each contract.
pub fn day_snapshot(source: &str, underlying: &str, date: &str, spot: f64, entries: &[(&Contract, f64, f64)], sel: &Selection, rate: RateFn) -> Result<DayBuild, String> {
    let day = parse_iso(date).ok_or_else(|| format!("bad date {date}"))?;
    let mut by_expiry: BTreeMap<&str, Vec<&(&Contract, f64, f64)>> = BTreeMap::new();
    for e in entries {
        let (c, close, volume) = *e;
        let Some(exp) = parse_iso(&c.expiry) else { continue };
        let days = (exp - day) as i64;
        let keep = close > 0.0 && volume > 0.0 && days > 0 && days <= sel.max_days && c.strike >= spot * sel.money_lo && c.strike <= spot * sel.money_hi;
        if keep {
            by_expiry.entry(c.expiry.as_str()).or_default().push(e);
        }
    }
    let (mut quotes, mut forwards, mut parity_count) = (Vec::new(), Vec::new(), 0);
    for (expiry, group) in &by_expiry {
        let t = (parse_iso(expiry).unwrap() - day) as f64 / 365.0;
        let r = rate(date, t)?;
        let df = (-r * t).exp();
        let mut pairs: BTreeMap<i64, (f64, f64, f64)> = BTreeMap::new();
        for (c, close, _) in group.iter().map(|e| **e) {
            let slot = pairs.entry((c.strike * 1000.0).round() as i64).or_insert((c.strike, 0.0, 0.0));
            if c.call {
                slot.1 = close;
            } else {
                slot.2 = close;
            }
        }
        let pair_list: Vec<(f64, f64, f64)> = pairs.values().copied().collect();
        let parity = forward_from_parity(&pair_list, spot, df);
        let forward = parity.unwrap_or(spot / df);
        parity_count += parity.is_some() as usize;
        forwards.push(ForwardInfo { expiry: expiry.to_string(), forward: round6(forward), from_parity: parity.is_some() });
        for (c, close, volume) in group.iter().map(|e| **e) {
            // Out of the money, or within 1% of the forward: the early exercise premium of an in-the-money American option is not in the model.
            let usable = if c.call { c.strike >= forward * 0.99 } else { c.strike <= forward * 1.01 };
            let iv = if usable { implied_vol(c.call, close, forward, c.strike, t, df).map(round6) } else { None };
            quotes.push(OptionQuote {
                contract_symbol: c.symbol.clone(),
                option_type: if c.call { "call" } else { "put" }.into(),
                expiry: c.expiry.clone(),
                strike: c.strike,
                last_price: Some(close),
                bid: None,
                ask: None,
                volume: Some(volume as i64),
                open_interest: None,
                implied_volatility: iv,
                in_the_money: Some(if c.call { c.strike < spot } else { c.strike > spot }),
            });
        }
    }
    quotes.sort_by(|a, b| (&a.expiry, &a.option_type).cmp(&(&b.expiry, &b.option_type)).then(a.strike.total_cmp(&b.strike)));
    let note = format!(
        "Closes of the day, no bid and no ask. Implied volatility: Black-76, forward from put-call parity for {parity_count} of {} expiries (else spot / discount factor, no dividend), out-of-the-money and near-the-money options only; the options are American, so an error remains. Selection: expiry up to {} days, strike {} to {} times the spot.",
        forwards.len(),
        sel.max_days,
        sel.money_lo,
        sel.money_hi
    );
    Ok(DayBuild {
        snapshot: OptionChainSnapshot { ticker: underlying.to_uppercase(), snapshot_date: date.to_string(), spot_price: Some(spot), quotes, source: Some(format!("{source}-history")), feed: None, quote_kind: Some("close".into()), note: Some(note) },
        forwards,
    })
}

/// The surfaces of each day in the range that the source has an underlying close for.
pub fn build_range(src: &dyn OptionHistorySource, underlying: &str, from: &str, to: &str, sel: &Selection, rate: RateFn) -> Result<Built, String> {
    let spots = src.spot_closes(underlying, from, to)?;
    let query = contract_query(src.name(), underlying, &spots, from, to, sel)?;
    let mut contracts = src.contracts(underlying, &query)?;
    contracts.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    contracts.dedup_by(|b, a| a.symbol == b.symbol);
    let listed = contracts.len();
    src.announce(&format!(
        "{underlying} {from} to {to}: {listed} contracts (expiry {} to {}, strike {} to {}); {} requests of closes ({} contracts in a request); {} requests so far",
        query.expiry_from,
        query.expiry_to,
        query.strike_min,
        query.strike_max,
        listed.div_ceil(src.bars_per_request().max(1)),
        src.bars_per_request(),
        src.requests()
    ));
    let index: HashMap<&str, &Contract> = contracts.iter().map(|c| (c.symbol.as_str(), c)).collect();
    let mut by_date: BTreeMap<String, Vec<(&Contract, f64, f64)>> = BTreeMap::new();
    let mut with_bars = std::collections::HashSet::new();
    for batch in contracts.chunks(src.bars_per_request().max(1)) {
        for d in src.closes(batch, from, to)? {
            if let (Some(c), true) = (index.get(d.symbol.as_str()), spots.contains_key(&d.date)) {
                with_bars.insert(d.symbol.clone());
                by_date.entry(d.date.clone()).or_default().push((c, d.close, d.volume));
            }
        }
    }
    let mut days = Vec::new();
    for (date, spot) in &spots {
        let entries = by_date.remove(date).unwrap_or_default();
        let mut built = day_snapshot(src.name(), underlying, date, *spot, &entries, sel, &mut *rate)?;
        if let Some(note) = src.level_note() {
            apply_level_note(&mut built, date, &note);
        }
        days.push(built);
    }
    Ok(Built { days, contracts_listed: listed, contracts_with_bars: with_bars.len(), requests: src.requests() })
}

/// The query of contracts for a range of dates. A missing or bad level is an error text, never a range with a number
/// that is not a number.
pub fn contract_query(source: &str, underlying: &str, spots: &BTreeMap<String, f64>, from: &str, to: &str, sel: &Selection) -> Result<ContractQuery, String> {
    if spots.is_empty() {
        return Err(format!("{source}: no level of {underlying} from {from} to {to}; the options need a level of the underlying for the strike range{}", level_hint(underlying)));
    }
    if let Some((d, v)) = spots.iter().find(|(_, v)| !(v.is_finite() && **v > 0.0)) {
        return Err(format!("{source}: the level of {underlying} on {d} is {v}, not a price"));
    }
    let (smin, smax) = spots.values().fold((f64::MAX, 0.0f64), |(a, b), s| (a.min(*s), b.max(*s)));
    let last_day = parse_iso(to).ok_or_else(|| format!("bad date {to}"))?;
    Ok(ContractQuery { expiry_from: from.to_string(), expiry_to: iso(last_day + sel.max_days as i32), strike_min: ((smin * sel.money_lo * 1e6).round() / 1e6).floor(), strike_max: ((smax * sel.money_hi * 1e6).round() / 1e6).ceil() })
}

fn level_hint(underlying: &str) -> String {
    match index_info(underlying) {
        Some(IndexInfo { level: LevelKind::VixFile, .. }) => "; run: fa sync --only vix (the Cboe VIX history file)".into(),
        Some(_) => String::new(),
        None => "; for an index give --level SYMBOL:RATIO, for example --level SPY:10".into(),
    }
}

/// After the day is built with a proxy level (an index): the level is the forward of the nearest expiry (up to 10 days) that
/// has a forward from put-call parity, else the proxy. The note says so.
fn apply_level_note(day: &mut DayBuild, date: &str, note: &str) {
    let Some(d) = parse_iso(date) else { return };
    let near = day.forwards.iter().filter(|f| f.from_parity).filter_map(|f| Some((parse_iso(&f.expiry)? - d, f.forward))).filter(|(n, _)| *n > 0 && *n <= 10).min_by_key(|(n, _)| *n);
    let proxy = day.snapshot.spot_price;
    // Index options are European: the sentence about American options does not apply.
    if let Some(n) = day.snapshot.note.as_mut() {
        *n = n.replace("the options are American, so an error remains", "index options are European, so Black-76 is the model of the options");
    }
    if let Some((n, f)) = near {
        day.snapshot.spot_price = Some(f);
        day.snapshot.note = Some(format!("{} Level of the underlying: the forward of the expiry {n} days away from put-call parity ({note}).", day.snapshot.note.clone().unwrap_or_default()));
    } else {
        day.snapshot.note = Some(format!("{} Level of the underlying: a proxy {:?} ({note}).", day.snapshot.note.clone().unwrap_or_default(), proxy));
    }
}

/// What `fill_from_mids` did.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MidFill {
    pub expiries: usize,
    pub expiries_with_parity: usize,
    pub filled: usize,
    /// The level that was set: the forward of this expiry and its days.
    pub level_from: Option<(String, i64, f64)>,
}

/// A live chain with bid and ask but no implied volatility (the index chains of Alpaca): the mid of bid and ask of a call and a
/// put at the same strike gives the forward of each expiry (put-call parity; no level of the underlying is needed), and Black-76
/// on that forward gives the implied volatility of each out-of-the-money or near-the-money quote. Index options are European,
/// so the model is the right one for them. A quote that has an implied volatility keeps it. When the snapshot has no spot, the
/// spot becomes the forward of the nearest expiry (1 to 10 days away) that has a forward from parity.
pub fn fill_from_mids(snap: &mut OptionChainSnapshot, rate: RateFn) -> Result<MidFill, String> {
    let day = parse_iso(&snap.snapshot_date).ok_or_else(|| format!("bad date {}", snap.snapshot_date))?;
    let mut by_expiry: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, q) in snap.quotes.iter().enumerate() {
        by_expiry.entry(q.expiry.clone()).or_default().push(i);
    }
    let mid = |q: &OptionQuote| match (q.bid, q.ask) {
        (Some(b), Some(a)) if b > 0.0 && a >= b => Some(0.5 * (a + b)),
        _ => None,
    };
    let mut fill = MidFill::default();
    for (expiry, idx) in &by_expiry {
        let Some(exp) = parse_iso(expiry) else { continue };
        let n = (exp - day) as i64;
        if n <= 0 {
            continue;
        }
        fill.expiries += 1;
        let t = n as f64 / 365.0;
        let df = (-rate(&snap.snapshot_date, t)? * t).exp();
        let mut pairs: BTreeMap<i64, (f64, f64, f64)> = BTreeMap::new();
        for &i in idx {
            let q = &snap.quotes[i];
            let Some(m) = mid(q) else { continue };
            let slot = pairs.entry((q.strike * 1000.0).round() as i64).or_insert((q.strike, 0.0, 0.0));
            if q.option_type == "call" {
                slot.1 = m;
            } else {
                slot.2 = m;
            }
        }
        let list: Vec<(f64, f64, f64)> = pairs.values().copied().collect();
        let Some(forward) = forward_from_parity_blind(&list, df) else { continue };
        fill.expiries_with_parity += 1;
        if (1..=10).contains(&n) && fill.level_from.as_ref().is_none_or(|(_, m, _)| n < *m) {
            fill.level_from = Some((expiry.clone(), n, forward));
        }
        for &i in idx {
            let call = snap.quotes[i].option_type == "call";
            let k = snap.quotes[i].strike;
            let usable = if call { k >= forward * 0.99 } else { k <= forward * 1.01 };
            if snap.quotes[i].implied_volatility.is_none() && usable {
                if let Some(m) = mid(&snap.quotes[i]) {
                    if let Some(iv) = implied_vol(call, m, forward, k, t, df) {
                        snap.quotes[i].implied_volatility = Some(round6(iv));
                        fill.filled += 1;
                    }
                }
            }
        }
    }
    if snap.spot_price.is_none() {
        if let Some((_, _, f)) = &fill.level_from {
            snap.spot_price = Some(round6(*f));
            let spot = *f;
            for q in &mut snap.quotes {
                q.in_the_money = Some(if q.option_type == "call" { q.strike < spot } else { q.strike > spot });
            }
        }
    }
    Ok(fill)
}

// ---------------------------------------------------------------- index underlyings

/// Where the level of an index comes from. Alpaca has no price of an index.
#[derive(Debug, Clone, PartialEq)]
pub enum LevelKind {
    /// The close of an ETF times a fixed ratio. The ratio is a proxy (the ETF pays dividends and the ratio drifts by about
    /// 0.1% each year); it picks the strike range only. The forward of each expiry comes from put-call parity.
    EtfProxy { etf: &'static str, ratio: f64 },
    /// The Cboe file `<cache>/vix/VIX_History.csv` (`fa sync --only vix`).
    VixFile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexInfo {
    /// The roots of the option symbols: the monthly root and the weekly root.
    pub roots: &'static [&'static str],
    pub level: LevelKind,
}

/// The index underlyings that the program knows. Other index names need `--level SYMBOL:RATIO`.
pub fn index_info(underlying: &str) -> Option<IndexInfo> {
    match underlying.to_uppercase().as_str() {
        "SPX" => Some(IndexInfo { roots: &["SPX", "SPXW"], level: LevelKind::EtfProxy { etf: "SPY", ratio: 10.0 } }),
        "VIX" => Some(IndexInfo { roots: &["VIX", "VIXW"], level: LevelKind::VixFile }),
        _ => None,
    }
}

/// The roots of the option symbols of an underlying (a stock or an ETF has one root).
pub fn roots_of(underlying: &str) -> Vec<String> {
    match index_info(underlying) {
        Some(i) => i.roots.iter().map(|r| r.to_string()).collect(),
        None => vec![underlying.to_uppercase()],
    }
}

/// The history of the VIX level: `DATE,OPEN,HIGH,LOW,CLOSE` with dates as `MM/DD/YYYY` (the Cboe file). Gives the close of
/// each date in the range.
pub fn read_vix_history(path: &Path, from: &str, to: &str) -> Result<BTreeMap<String, f64>, String> {
    let text = std::fs::read_to_string(path).map_err(|_| format!("no VIX history file {}; run: fa sync --only vix", path.display()))?;
    parse_vix_history(&text, from, to)
}

pub fn parse_vix_history(text: &str, from: &str, to: &str) -> Result<BTreeMap<String, f64>, String> {
    let mut out = BTreeMap::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split(',').collect();
        let (Some(date), Some(close)) = (cols.first(), cols.get(4)) else { continue };
        let mut parts = date.trim().split('/');
        let (Some(m), Some(d), Some(y)) = (parts.next(), parts.next(), parts.next()) else { continue };
        let iso_date = format!("{y}-{m:0>2}-{d:0>2}");
        if iso_date.as_str() < from || iso_date.as_str() > to {
            continue;
        }
        if let Ok(v) = close.trim().parse::<f64>() {
            if v.is_finite() && v > 0.0 {
                out.insert(iso_date, v);
            }
        }
    }
    Ok(out)
}

fn is_third_friday(d: i32) -> bool {
    let day = iso(d)[8..10].parse::<u32>().unwrap_or(0);
    weekday(d) == 4 && (15..=21).contains(&day)
}

/// Contracts that may exist for an index, made from the rules of the symbols, for the expiries that the contract list of
/// Alpaca does not hold (an expired contract of an index is in the list only for about the last six months, see #281).
/// A symbol that does not exist has no bars: it costs a place in a request and nothing else.
///
/// - SPX: the weekly root `SPXW` on each weekday, the monthly root `SPX` on the third Friday; strikes every 5 points.
/// - VIX: Wednesdays only; the monthly root `VIX` when the Wednesday is 30 days before a third Friday, else `VIXW`;
///   strikes every 0.5 point.
pub fn index_candidates(underlying: &str, expiry_from: &str, expiry_to: &str, strike_min: f64, strike_max: f64) -> Vec<Contract> {
    let (Some(a), Some(b), Some(info)) = (parse_iso(expiry_from), parse_iso(expiry_to), index_info(underlying)) else { return Vec::new() };
    let vix = info.level == LevelKind::VixFile;
    let step = if vix { 0.5 } else { 5.0 };
    let mut strikes = Vec::new();
    let mut k = (strike_min / step).ceil() * step;
    while k <= strike_max + 1e-9 && strikes.len() < 20_000 {
        strikes.push(k);
        k += step;
    }
    let mut out = Vec::new();
    for d in a..=b {
        let wd = weekday(d);
        let root = if vix {
            if wd != 2 {
                continue;
            }
            if is_third_friday(d + 30) { info.roots[0] } else { info.roots[1] }
        } else {
            if wd > 4 {
                continue;
            }
            if is_third_friday(d) { info.roots[0] } else { info.roots[1] }
        };
        let e = iso(d);
        for &k in &strikes {
            for call in [true, false] {
                out.push(Contract { symbol: format!("{root}{}{}{}{}{:08}", &e[2..4], &e[5..7], &e[8..10], if call { "C" } else { "P" }, (k * 1000.0).round() as i64), expiry: e.clone(), call, strike: k });
            }
        }
    }
    out
}

/// The number of days back from today for which the list of Alpaca holds expired index contracts (measured 2026-11-20: the
/// SPX list has them from March 2026; with a margin).
pub const INDEX_LIST_DEPTH_DAYS: i64 = 180;

/// How the level of the underlying of the history comes.
#[derive(Debug, Clone, PartialEq)]
pub enum Level {
    /// The close of the underlying itself (stock bars).
    Stock,
    /// The close of another symbol times a ratio (an index proxy; `--level SPY:10`).
    Proxy { symbol: String, ratio: f64 },
    /// The Cboe VIX history file.
    VixFile(PathBuf),
}

/// The level of an underlying: `--level SYMBOL:RATIO` first, then the known indexes, else the stock itself.
pub fn level_for(underlying: &str, cache: &Path, given: Option<(&str, f64)>) -> Level {
    if let Some((symbol, ratio)) = given {
        return Level::Proxy { symbol: symbol.to_uppercase(), ratio };
    }
    match index_info(underlying).map(|i| i.level) {
        Some(LevelKind::EtfProxy { etf, ratio }) => Level::Proxy { symbol: etf.into(), ratio },
        Some(LevelKind::VixFile) => Level::VixFile(cache.join("vix").join("VIX_History.csv")),
        None => Level::Stock,
    }
}

// ---------------------------------------------------------------- Alpaca

/// Alpaca: contracts from the trading API (`GET /v2/options/contracts`, status `active` and `inactive`), closes from the
/// option bars (`GET /v1beta1/options/bars`, 100 contracts in a request), the underlying from the stock bars (`raw`).
pub struct AlpacaHistory {
    pub client: Client,
    pub feed: String,
    pub now: i64,
    pub level: Level,
}

/// Contracts in one request of closes. 1,000 symbols in one request worked on 2026-11-20 (a URL of 19 KB); the documentation
/// gives no limit that was found, so the program keeps a margin.
pub const ALPACA_BARS_PER_REQUEST: usize = 500;

impl OptionHistorySource for AlpacaHistory {
    fn name(&self) -> &'static str {
        "alpaca"
    }
    fn bars_per_request(&self) -> usize {
        ALPACA_BARS_PER_REQUEST
    }
    fn spot_closes(&self, underlying: &str, from: &str, to: &str) -> Result<BTreeMap<String, f64>, String> {
        let stock = |symbol: &str, ratio: f64| -> Result<BTreeMap<String, f64>, String> {
            let today = iso(self.now.div_euclid(86_400) as i32);
            let sym = alpaca::api_symbol(symbol);
            let bars = self.client.stock_bars(&alpaca::StockQuery { symbols: std::slice::from_ref(&sym), timeframe: "1Day", start: from, to, today: &today, adjustment: "raw", feed: &self.feed }).map_err(|e| e.to_string())?;
            Ok(bars.get(&sym).map(|v| v.iter().map(|b| (alpaca::ny_day(b.utc).0, b.c * ratio)).collect()).unwrap_or_default())
        };
        match &self.level {
            Level::Stock => stock(underlying, 1.0),
            Level::Proxy { symbol, ratio } => stock(symbol, *ratio),
            Level::VixFile(path) => read_vix_history(path, from, to),
        }
    }
    fn contracts(&self, underlying: &str, q: &ContractQuery) -> Result<Vec<Contract>, String> {
        let index = index_info(underlying).is_some();
        // An index: the list holds the expired contracts of about the last six months only. Older expiries come from the rules.
        let list_from = if index { iso(self.now.div_euclid(86_400) as i32 - INDEX_LIST_DEPTH_DAYS as i32).max(q.expiry_from.clone()) } else { q.expiry_from.clone() };
        let mut out = Vec::new();
        if list_from <= q.expiry_to {
            for status in ["active", "inactive"] {
                let list = self.client.contracts(&underlying.to_uppercase(), status, &list_from, &q.expiry_to, q.strike_min, q.strike_max).map_err(|e| e.to_string())?;
                out.extend(list.iter().filter_map(|c| contract_from_info(underlying, c)));
            }
        }
        if index && q.expiry_from < list_from {
            let to = iso(parse_iso(&list_from).unwrap_or(0) - 1).min(q.expiry_to.clone());
            out.extend(index_candidates(underlying, &q.expiry_from, &to, q.strike_min, q.strike_max));
        }
        Ok(out)
    }
    fn closes(&self, contracts: &[Contract], from: &str, to: &str) -> Result<Vec<DayClose>, String> {
        let symbols: Vec<String> = contracts.iter().map(|c| c.symbol.clone()).collect();
        let bars = self.client.option_bars(&symbols, from, to).map_err(|e| e.to_string())?;
        Ok(bars.into_iter().flat_map(|(symbol, v)| v.into_iter().map(move |b| DayClose { symbol: symbol.clone(), date: alpaca::ny_day(b.utc).0, close: b.c, volume: b.v })).collect())
    }
    fn requests(&self) -> usize {
        self.client.requests()
    }
    fn announce(&self, line: &str) {
        eprintln!("fa: plan: {line}");
    }
    fn level_note(&self) -> Option<String> {
        match &self.level {
            Level::Stock => None,
            Level::Proxy { symbol, ratio } => Some(format!("proxy: the close of {symbol} times {ratio}")),
            Level::VixFile(_) => Some("the VIX index level of the Cboe history file".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::option_iv::black76;
    use std::cell::Cell;

    const R: f64 = 0.04;
    const Q: f64 = 0.014;

    /// A known surface: the volatility is `0.18 + 1.2 * ln(K/F)^2` (a smile), the same for a call and a put.
    fn sigma(k: f64, f: f64) -> f64 {
        0.18 + 1.2 * (k / f).ln().powi(2)
    }

    fn forward(spot: f64, t: f64) -> f64 {
        spot * ((R - Q) * t).exp()
    }

    struct Synthetic {
        spots: Vec<(String, f64)>,
        count: Cell<usize>,
        /// The contract that has no trade on any day.
        silent: String,
    }

    impl Synthetic {
        fn new() -> Synthetic {
            let dates = ["2024-03-04", "2024-03-05", "2024-03-06", "2024-03-07", "2024-03-08"];
            Synthetic { spots: dates.iter().enumerate().map(|(i, d)| (d.to_string(), 500.0 + 2.0 * i as f64)).collect(), count: Cell::new(0), silent: "SPY240621C00500000".into() }
        }
        fn universe() -> Vec<Contract> {
            let mut v = Vec::new();
            for expiry in ["2024-03-15", "2024-04-19", "2024-06-21", "2025-06-20"] {
                for k in (350..=650).step_by(5) {
                    for call in [true, false] {
                        v.push(Contract { symbol: format!("SPY{}{}{}{}{:08}", &expiry[2..4], &expiry[5..7], &expiry[8..10], if call { "C" } else { "P" }, k * 1000), expiry: expiry.into(), call, strike: k as f64 });
                    }
                }
            }
            v
        }
    }

    impl OptionHistorySource for Synthetic {
        fn name(&self) -> &'static str {
            "synthetic"
        }
        fn bars_per_request(&self) -> usize {
            100
        }
        fn spot_closes(&self, _u: &str, from: &str, to: &str) -> Result<BTreeMap<String, f64>, String> {
            self.count.set(self.count.get() + 1);
            Ok(self.spots.iter().filter(|(d, _)| d.as_str() >= from && d.as_str() <= to).cloned().collect())
        }
        fn contracts(&self, _u: &str, q: &ContractQuery) -> Result<Vec<Contract>, String> {
            self.count.set(self.count.get() + 1);
            Ok(Self::universe().into_iter().filter(|c| c.expiry >= q.expiry_from && c.expiry <= q.expiry_to && c.strike >= q.strike_min && c.strike <= q.strike_max).collect())
        }
        fn closes(&self, contracts: &[Contract], from: &str, to: &str) -> Result<Vec<DayClose>, String> {
            self.count.set(self.count.get() + 1);
            let mut out = Vec::new();
            for c in contracts.iter().filter(|c| c.symbol != self.silent) {
                for (d, spot) in self.spots.iter().filter(|(d, _)| d.as_str() >= from && d.as_str() <= to) {
                    let t = (parse_iso(&c.expiry).unwrap() - parse_iso(d).unwrap()) as f64 / 365.0;
                    if t <= 0.0 {
                        continue;
                    }
                    let (f, df) = (forward(*spot, t), (-R * t).exp());
                    out.push(DayClose { symbol: c.symbol.clone(), date: d.clone(), close: black76(c.call, f, c.strike, t, df, sigma(c.strike, f)), volume: 10.0 });
                }
            }
            Ok(out)
        }
        fn requests(&self) -> usize {
            self.count.get()
        }
    }

    fn flat_rate() -> impl FnMut(&str, f64) -> Result<f64, String> {
        |_, _| Ok(R)
    }

    #[test]
    fn the_builder_recovers_the_known_surface_and_the_forward() {
        let src = Synthetic::new();
        let mut rate = flat_rate();
        let built = build_range(&src, "SPY", "2024-03-04", "2024-03-08", &Selection { max_days: 365, money_lo: 0.8, money_hi: 1.2 }, &mut rate).unwrap();
        assert_eq!(built.days.len(), 5);
        let day = &built.days[2];
        let snap = &day.snapshot;
        assert_eq!((snap.snapshot_date.as_str(), snap.spot_price), ("2024-03-06", Some(504.0)));
        assert_eq!(snap.quote_kind.as_deref(), Some("close"));
        assert_eq!(snap.source.as_deref(), Some("synthetic-history"));
        assert!(snap.quotes.iter().all(|q| q.bid.is_none() && q.ask.is_none() && q.last_price.is_some()));
        // The forward of each expiry is the true forward, from parity.
        for f in &day.forwards {
            let t = (parse_iso(&f.expiry).unwrap() - parse_iso("2024-03-06").unwrap()) as f64 / 365.0;
            assert!(f.from_parity, "{f:?}");
            assert!((f.forward - forward(504.0, t)).abs() < 2e-6, "{f:?} true {}", forward(504.0, t));
        }
        // The implied volatility of each usable quote is the volatility of the surface.
        let mut checked = 0;
        for q in &snap.quotes {
            let t = (parse_iso(&q.expiry).unwrap() - parse_iso("2024-03-06").unwrap()) as f64 / 365.0;
            let f = forward(504.0, t);
            match q.implied_volatility {
                Some(iv) => {
                    assert!((iv - sigma(q.strike, f)).abs() < 5e-5, "{} iv {iv} true {}", q.contract_symbol, sigma(q.strike, f));
                    checked += 1;
                }
                None => {
                    // No volatility: deep in the money, or a price at the intrinsic value.
                    let itm = if q.option_type == "call" { q.strike < f * 0.99 } else { q.strike > f * 1.01 };
                    let p = q.last_price.unwrap();
                    let intrinsic = if q.option_type == "call" { (f - q.strike).max(0.0) } else { (q.strike - f).max(0.0) };
                    assert!(itm || p <= intrinsic * (-R * t).exp() + 1e-9, "{} has no volatility", q.contract_symbol);
                }
            }
        }
        assert!(checked > 100, "{checked}");
        // The metrics of the live code run on the snapshot: the ATM volatility is near the smile minimum.
        let metrics = super::super::options::compute_chain_metrics(snap);
        let atm = metrics.expiries[0].atm_iv.unwrap();
        assert!((atm - 0.18).abs() < 0.01, "{atm}");
    }

    #[test]
    fn selection_and_no_trade_filter() {
        let src = Synthetic::new();
        let mut rate = flat_rate();
        let built = build_range(&src, "SPY", "2024-03-04", "2024-03-08", &Selection { max_days: 200, money_lo: 0.9, money_hi: 1.1 }, &mut rate).unwrap();
        let snap = &built.days[0].snapshot;
        assert!(snap.quotes.iter().all(|q| q.expiry != "2025-06-20"), "an expiry beyond 200 days is out");
        assert!(snap.quotes.iter().all(|q| q.strike >= 500.0 * 0.9 && q.strike <= 500.0 * 1.1), "strikes in the moneyness range of the date");
        assert!(snap.quotes.iter().all(|q| q.contract_symbol != "SPY240621C00500000"), "a contract with no trade has no row");
        assert!(built.contracts_listed > built.contracts_with_bars);
        // A contract that expires on the date, or has volume 0, is out.
        let c = Contract { symbol: "SPY240304C00500000".into(), expiry: "2024-03-04".into(), call: true, strike: 500.0 };
        let c2 = Contract { symbol: "SPY240315C00500000".into(), expiry: "2024-03-15".into(), call: true, strike: 500.0 };
        let mut rate = flat_rate();
        let built = day_snapshot("x", "SPY", "2024-03-04", 500.0, &[(&c, 1.0, 5.0), (&c2, 5.0, 0.0)], &Selection::default(), &mut rate).unwrap();
        assert!(built.snapshot.quotes.is_empty());
        // With no parity pair the forward is spot / discount factor, and the note says so.
        let c3 = Contract { symbol: "SPY240315C00510000".into(), expiry: "2024-03-15".into(), call: true, strike: 510.0 };
        let built = day_snapshot("x", "SPY", "2024-03-04", 500.0, &[(&c3, 3.0, 5.0)], &Selection::default(), &mut rate).unwrap();
        assert!(!built.forwards[0].from_parity);
        assert!(built.snapshot.note.unwrap().contains("parity for 0 of 1"));
    }

    #[test]
    fn a_range_of_dates_costs_one_set_of_requests_and_not_one_for_each_date() {
        let mut rate = flat_rate();
        let sel = Selection { max_days: 365, money_lo: 0.8, money_hi: 1.2 };
        let range = Synthetic::new();
        build_range(&range, "SPY", "2024-03-04", "2024-03-08", &sel, &mut rate).unwrap();
        let per_date = Synthetic::new();
        for (d, _) in &per_date.spots {
            build_range(&per_date, "SPY", d, d, &sel, &mut rate).unwrap();
        }
        // 1 underlying request + 1 contract request + ceil(contracts / 100) requests of closes. The window has 3 expiries
        // (the 2025 expiry is out) and 43 strikes: 258 contracts, 3 batches. Each single date repeats the same 5 requests.
        assert!(Synthetic::universe().len() > 100);
        assert_eq!(range.requests(), 2 + 3);
        assert_eq!(per_date.requests(), 5 * (2 + 3), "each date asks again for the underlying, the list and every batch");
    }

    #[test]
    fn a_rate_error_stops_the_build() {
        let src = Synthetic::new();
        let mut bad = |_: &str, _: f64| -> Result<f64, String> { Err("no curve".into()) };
        assert_eq!(build_range(&src, "SPY", "2024-03-04", "2024-03-08", &Selection::default(), &mut bad).unwrap_err(), "no curve");
    }

    #[test]
    fn adjusted_roots_are_not_in_the_surface() {
        let a = ContractInfo { symbol: "SPY240419C00500000".into(), expiry: "2024-04-19".into(), call: true, strike: 500.0 };
        let b = ContractInfo { symbol: "SPY1240419C00500000".into(), expiry: "2024-04-19".into(), call: true, strike: 500.0 };
        assert!(contract_from_info("spy", &a).is_some());
        assert!(contract_from_info("SPY", &b).is_none());
    }

    #[test]
    fn index_roots_and_contracts() {
        let info = |sym: &str| ContractInfo { symbol: sym.into(), expiry: "2026-11-20".into(), call: true, strike: 6500.0 };
        // Real symbols of the Alpaca list (2026-11-20): the monthly root, the weekly root, and a root that is not in the surface.
        assert!(contract_from_info("SPX", &info("SPX261120C06500000")).is_some());
        assert!(contract_from_info("SPX", &info("SPXW261123C03000000")).is_some());
        assert!(contract_from_info("SPX", &info("SPXO261118C06300000")).is_none());
        assert!(contract_from_info("VIX", &info("VIXW261124C00010000")).is_some());
        assert!(contract_from_info("SPY", &info("SPXW261123C03000000")).is_none(), "a stock has its own root only");
        assert_eq!(roots_of("spx"), ["SPX", "SPXW"]);
        assert_eq!(roots_of("spy"), ["SPY"]);
    }

    #[test]
    fn index_candidates_follow_the_symbol_rules() {
        // 2024-06-17 (Monday) to 2024-06-21 (Friday, the third Friday): SPXW on four days, SPX on the third Friday.
        let c = index_candidates("SPX", "2024-06-17", "2024-06-21", 5300.0, 5310.0);
        assert_eq!(c.len(), 5 * 3 * 2, "5 days, strikes 5300 5305 5310, call and put");
        assert!(c.iter().any(|x| x.symbol == "SPXW240617C05300000"));
        assert!(c.iter().any(|x| x.symbol == "SPX240621P05310000"));
        assert!(!c.iter().any(|x| x.symbol.starts_with("SPXW240621")));
        assert!(index_candidates("SPX", "2024-06-22", "2024-06-23", 5300.0, 5310.0).is_empty(), "a weekend has no expiry");
        // VIX: Wednesdays. 2024-07-17 is 30 days before the third Friday 2024-08-16: the monthly root.
        let v = index_candidates("VIX", "2024-07-15", "2024-07-24", 12.0, 13.0);
        assert!(v.iter().any(|x| x.symbol == "VIX240717C00012500"), "monthly");
        assert!(v.iter().any(|x| x.symbol == "VIXW240724P00013000"), "weekly");
        assert!(v.iter().all(|x| x.expiry == "2024-07-17" || x.expiry == "2024-07-24"));
        assert!(index_candidates("XYZ", "2024-06-17", "2024-06-21", 1.0, 2.0).is_empty());
    }

    #[test]
    fn vix_history_file_and_level_errors() {
        let text = "DATE,OPEN,HIGH,LOW,CLOSE\n06/06/2024,12.9,13.1,12.2,12.4\n06/07/2024,12.690000,13.080000,12.110000,12.220000\n06/10/2024,12.0,12.5,11.9,12.1\nbad line\n";
        let m = parse_vix_history(text, "2024-06-06", "2024-06-07").unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m["2024-06-07"], 12.22);
        // No level: a clear text, never a range of numbers that are not numbers.
        let sel = Selection::default();
        let e = contract_query("alpaca", "SPX", &BTreeMap::new(), "2024-06-07", "2024-06-07", &sel).unwrap_err();
        assert!(e.contains("no level of SPX") && !e.contains("NaN") && !e.contains("inf"), "{e}");
        assert!(contract_query("alpaca", "VIX", &BTreeMap::new(), "2024-06-07", "2024-06-07", &sel).unwrap_err().contains("fa sync --only vix"));
        let bad: BTreeMap<String, f64> = [("2024-06-07".to_string(), f64::NAN)].into();
        assert!(contract_query("alpaca", "SPX", &bad, "2024-06-07", "2024-06-07", &sel).unwrap_err().contains("not a price"));
        let ok: BTreeMap<String, f64> = [("2024-06-07".to_string(), 100.0)].into();
        let q = contract_query("alpaca", "XYZ", &ok, "2024-06-07", "2024-06-07", &sel).unwrap();
        assert_eq!((q.strike_min, q.strike_max, q.expiry_to.as_str()), (90.0, 110.0, "2024-10-05"));
    }

    #[test]
    fn a_chain_with_bid_and_ask_and_no_volatility_gets_forward_level_and_volatility() {
        // An index chain with no level and no implied volatility: bid and ask are tight mids of a known Black-76 surface.
        let (f, days) = (7050.0, 7);
        let t = days as f64 / 365.0;
        let df = (-0.04f64 * t).exp();
        let mut quotes = Vec::new();
        for k in (6800..=7300).step_by(25) {
            for call in [true, false] {
                let p = black76(call, f, k as f64, t, df, 0.16);
                quotes.push(OptionQuote { contract_symbol: format!("SPXW261127{}{:08}", if call { "C" } else { "P" }, k * 1000), option_type: if call { "call" } else { "put" }.into(), expiry: "2026-11-27".into(), strike: k as f64, last_price: None, bid: Some(p - 0.05), ask: Some(p + 0.05), volume: None, open_interest: None, implied_volatility: None, in_the_money: None });
            }
        }
        let mut snap = OptionChainSnapshot { ticker: "SPX".into(), snapshot_date: "2026-11-20".into(), spot_price: None, quotes, source: None, feed: None, quote_kind: None, note: None };
        let mut rate = flat_rate();
        let fill = fill_from_mids(&mut snap, &mut rate).unwrap();
        assert_eq!((fill.expiries, fill.expiries_with_parity), (1, 1));
        let spot = snap.spot_price.expect("a level from parity");
        assert!((spot - f).abs() < 0.2, "{spot}");
        assert!(fill.filled > 20, "{}", fill.filled);
        for q in snap.quotes.iter().filter(|q| q.implied_volatility.is_some()) {
            assert!((q.implied_volatility.unwrap() - 0.16).abs() < 0.005, "{} {:?}", q.contract_symbol, q.implied_volatility);
        }
        // A quote that has a volatility keeps it.
        snap.quotes[0].implied_volatility = Some(0.5);
        snap.spot_price = None;
        fill_from_mids(&mut snap, &mut rate).unwrap();
        assert_eq!(snap.quotes[0].implied_volatility, Some(0.5));
    }

    #[test]
    fn the_plan_line_comes_before_the_closes() {
        struct Spy(Synthetic, std::cell::RefCell<Vec<String>>);
        impl OptionHistorySource for Spy {
            fn name(&self) -> &'static str {
                "spy"
            }
            fn bars_per_request(&self) -> usize {
                500
            }
            fn spot_closes(&self, u: &str, f: &str, t: &str) -> Result<BTreeMap<String, f64>, String> {
                self.0.spot_closes(u, f, t)
            }
            fn contracts(&self, u: &str, q: &ContractQuery) -> Result<Vec<Contract>, String> {
                self.0.contracts(u, q)
            }
            fn closes(&self, c: &[Contract], f: &str, t: &str) -> Result<Vec<DayClose>, String> {
                assert!(!self.1.borrow().is_empty(), "the plan is announced first");
                self.0.closes(c, f, t)
            }
            fn requests(&self) -> usize {
                self.0.requests()
            }
            fn announce(&self, line: &str) {
                self.1.borrow_mut().push(line.to_string());
            }
        }
        let src = Spy(Synthetic::new(), Default::default());
        let mut rate = flat_rate();
        build_range(&src, "SPY", "2024-03-04", "2024-03-04", &Selection::default(), &mut rate).unwrap();
        let lines = src.1.borrow();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("contracts (expiry 2024-03-04 to 2024-07-02") && lines[0].contains("requests of closes"), "{}", lines[0]);
    }
}
