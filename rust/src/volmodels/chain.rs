//! A stored option snapshot as a quote set: series (root, expiry), forward from put-call parity, the live filters.
//!
//! The filters are in this order, and each quote is dropped for one reason only (the counts add up to the total):
//! no bid or ask, ask not above bid, in the money, no implied volatility of bid, mid or ask, spread too wide.
//! Then a series with fewer than `MIN_QUOTES` quotes goes, with all its quotes. A series with no forward goes too.

use super::base::{quote_from_prices, QuoteSet, Series};
use crate::market::option_history::{occ_root, RateFn};
use crate::market::options::OptionChainSnapshot;
use crate::trading::calendar::parse_iso;
use std::collections::BTreeMap;

pub const MIN_QUOTES: usize = 5;

#[derive(Debug, Clone)]
pub struct Filters {
    /// The largest ask volatility minus bid volatility (decimal): 0.01 is one vol point; 0.05 for VIX options.
    pub max_spread: f64,
    pub expiry_max_days: Option<i64>,
}

impl Filters {
    pub fn for_symbol(symbol: &str, expiry_max_days: Option<i64>) -> Filters {
        let vix = symbol.to_uppercase().starts_with("VIX");
        Filters { max_spread: if vix { 0.05 } else { 0.01 }, expiry_max_days }
    }
}

/// What the filters dropped. All fields count quotes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Counts {
    pub total: usize,
    pub expired_or_today: usize,
    pub beyond_expiry_max: usize,
    pub no_forward: usize,
    pub no_bid_or_ask: usize,
    pub ask_not_above_bid: usize,
    pub in_the_money: usize,
    pub no_iv: usize,
    pub spread_too_wide: usize,
    pub short_series: usize,
    pub kept: usize,
    pub series_kept: usize,
    pub series_total: usize,
}

impl Counts {
    pub fn dropped(&self) -> usize {
        self.total - self.kept
    }
}

/// Strikes of the median of the forward.
pub const PARITY_STRIKES: usize = 5;

/// The forward of a series: the strike where call and put mids are nearest is the guess; the median of `K + (C - P) / df`
/// over the `PARITY_STRIKES` strikes nearest to it (with both mids) is the forward. `pairs` are `(strike, call mid, put mid)`.
fn forward_from_parity_5(pairs: &[(f64, f64, f64)], df: f64) -> Option<f64> {
    let ok: Vec<&(f64, f64, f64)> = pairs.iter().filter(|p| p.0 > 0.0 && p.1 > 0.0 && p.2 > 0.0).collect();
    let guess = ok.iter().min_by(|a, b| (a.1 - a.2).abs().total_cmp(&(b.1 - b.2).abs()))?.0;
    let mut near = ok;
    near.sort_by(|a, b| (a.0 - guess).abs().total_cmp(&(b.0 - guess).abs()));
    near.truncate(PARITY_STRIKES);
    let mut f: Vec<f64> = near.iter().map(|(k, c, p)| k + (c - p) / df).collect();
    f.sort_by(f64::total_cmp);
    let n = f.len();
    let median = if n % 2 == 1 { f[n / 2] } else { 0.5 * (f[n / 2 - 1] + f[n / 2]) };
    (median > 0.5 * guess && median < 2.0 * guess).then_some(median)
}

pub fn build(snap: &OptionChainSnapshot, rate: RateFn, f: &Filters) -> Result<(QuoteSet, Counts), String> {
    let day = parse_iso(&snap.snapshot_date).ok_or_else(|| format!("bad date {}", snap.snapshot_date))?;
    let mut by_series: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (i, q) in snap.quotes.iter().enumerate() {
        by_series.entry((occ_root(&q.contract_symbol), q.expiry.clone())).or_default().push(i);
    }
    let mut c = Counts { total: snap.quotes.len(), series_total: by_series.len(), ..Counts::default() };
    let mut set = QuoteSet::default();
    for ((root, expiry), idx) in &by_series {
        let days = parse_iso(expiry).map_or(0, |e| (e - day) as i64);
        if days <= 0 {
            c.expired_or_today += idx.len();
            continue;
        }
        if f.expiry_max_days.is_some_and(|m| days > m) {
            c.beyond_expiry_max += idx.len();
            continue;
        }
        let t = days as f64 / 365.0;
        let df = (-rate(&snap.snapshot_date, t)? * t).exp();
        let live = |i: usize| match (snap.quotes[i].bid, snap.quotes[i].ask) {
            (Some(b), Some(a)) if b > 0.0 && a > b => Some((b, a)),
            _ => None,
        };
        let mut pairs: BTreeMap<i64, (f64, f64, f64)> = BTreeMap::new();
        for &i in idx {
            let q = &snap.quotes[i];
            if let Some((b, a)) = live(i) {
                let slot = pairs.entry((q.strike * 1000.0).round() as i64).or_insert((q.strike, 0.0, 0.0));
                if q.option_type == "call" {
                    slot.1 = 0.5 * (a + b);
                } else {
                    slot.2 = 0.5 * (a + b);
                }
            }
        }
        let list: Vec<(f64, f64, f64)> = pairs.values().copied().collect();
        let Some(forward) = forward_from_parity_5(&list, df) else {
            c.no_forward += idx.len();
            continue;
        };
        let mut s = Series { root: root.clone(), expiry: expiry.clone(), days, t, forward, df, quotes: Vec::new() };
        let mut dropped = Counts::default();
        for &i in idx {
            let q = &snap.quotes[i];
            let call = q.option_type == "call";
            let (bid, ask) = (q.bid.unwrap_or(0.0), q.ask.unwrap_or(0.0));
            if !(bid > 0.0 && ask > 0.0) {
                dropped.no_bid_or_ask += 1;
            } else if ask <= bid {
                dropped.ask_not_above_bid += 1;
            } else if (call && q.strike < forward) || (!call && q.strike >= forward) {
                dropped.in_the_money += 1;
            } else {
                match quote_from_prices(q.strike, call, bid, ask, forward, t, df) {
                    None => dropped.no_iv += 1,
                    Some(x) if x.iv_spread() > f.max_spread => dropped.spread_too_wide += 1,
                    Some(x) => s.quotes.push(x),
                }
            }
        }
        if s.quotes.len() < MIN_QUOTES {
            c.short_series += s.quotes.len();
        }
        c.no_bid_or_ask += dropped.no_bid_or_ask;
        c.ask_not_above_bid += dropped.ask_not_above_bid;
        c.in_the_money += dropped.in_the_money;
        c.no_iv += dropped.no_iv;
        c.spread_too_wide += dropped.spread_too_wide;
        if s.quotes.len() >= MIN_QUOTES {
            s.quotes.sort_by(|a, b| a.strike.total_cmp(&b.strike));
            c.kept += s.quotes.len();
            c.series_kept += 1;
            set.series.push(s);
        }
    }
    debug_assert_eq!(c.total, c.kept + c.dropped());
    Ok((set, c))
}
