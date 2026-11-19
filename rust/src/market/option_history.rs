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
use super::option_iv::{forward_from_parity, implied_vol};
use super::options::{OptionChainSnapshot, OptionQuote};
use super::round6;
use crate::trading::calendar::{iso, parse_iso};
use crate::trading::chain::parse_option;
use std::collections::{BTreeMap, HashMap};

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
}

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
        Selection { max_days: 365, money_lo: 0.8, money_hi: 1.2 }
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
    // The root must be the underlying: an adjusted contract (`SPY1...`) has another root and is not in the surface.
    parse_option(&c.symbol, &underlying.to_uppercase())?;
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
    if spots.is_empty() {
        return Err(format!("{}: no close of {underlying} from {from} to {to}", src.name()));
    }
    let (smin, smax) = spots.values().fold((f64::MAX, 0.0f64), |(a, b), s| (a.min(*s), b.max(*s)));
    let last_day = parse_iso(to).ok_or_else(|| format!("bad date {to}"))?;
    let query = ContractQuery { expiry_from: from.to_string(), expiry_to: iso(last_day + sel.max_days as i32), strike_min: (smin * sel.money_lo).floor(), strike_max: (smax * sel.money_hi).ceil() };
    let mut contracts = src.contracts(underlying, &query)?;
    contracts.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    contracts.dedup_by(|b, a| a.symbol == b.symbol);
    let listed = contracts.len();
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
        days.push(day_snapshot(src.name(), underlying, date, *spot, &entries, sel, &mut *rate)?);
    }
    Ok(Built { days, contracts_listed: listed, contracts_with_bars: with_bars.len(), requests: src.requests() })
}

// ---------------------------------------------------------------- Alpaca

/// Alpaca: contracts from the trading API (`GET /v2/options/contracts`, status `active` and `inactive`), closes from the
/// option bars (`GET /v1beta1/options/bars`, 100 contracts in a request), the underlying from the stock bars (`raw`).
pub struct AlpacaHistory {
    pub client: Client,
    pub feed: String,
    pub now: i64,
}

impl OptionHistorySource for AlpacaHistory {
    fn name(&self) -> &'static str {
        "alpaca"
    }
    fn bars_per_request(&self) -> usize {
        100
    }
    fn spot_closes(&self, underlying: &str, from: &str, to: &str) -> Result<BTreeMap<String, f64>, String> {
        let today = iso(self.now.div_euclid(86_400) as i32);
        let sym = alpaca::api_symbol(underlying);
        let bars = self.client.stock_bars(&alpaca::StockQuery { symbols: std::slice::from_ref(&sym), timeframe: "1Day", start: from, to, today: &today, adjustment: "raw", feed: &self.feed }).map_err(|e| e.to_string())?;
        Ok(bars.get(&sym).map(|v| v.iter().map(|b| (alpaca::ny_day(b.utc).0, b.c)).collect()).unwrap_or_default())
    }
    fn contracts(&self, underlying: &str, q: &ContractQuery) -> Result<Vec<Contract>, String> {
        let mut out = Vec::new();
        for status in ["active", "inactive"] {
            let list = self.client.contracts(&underlying.to_uppercase(), status, &q.expiry_from, &q.expiry_to, q.strike_min, q.strike_max).map_err(|e| e.to_string())?;
            out.extend(list.iter().filter_map(|c| contract_from_info(underlying, c)));
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
        let built = build_range(&src, "SPY", "2024-03-04", "2024-03-08", &Selection::default(), &mut rate).unwrap();
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
        let sel = Selection::default();
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
}
