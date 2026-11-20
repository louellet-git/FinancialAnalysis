//! Static no-arbitrage checks of one stored option chain, at executable prices: buy at the ask, sell at the bid.
//!
//! Pure functions, no I/O. A break at mid prices only is not a finding. A break here is a quote pair that
//! breaks a bound at executable prices; it is not a profit (no costs, no borrow, no time of the quotes, early
//! exercise). A series is one root and one expiry.
//!
//! Checks (K1 < K2 < K3 are strikes of one series and one type):
//! 1. Monotony. Call: bid C(K2) > ask C(K1). Put: bid P(K1) > ask P(K2). All strike pairs.
//! 2. Slope. Call: bid C(K1) - ask C(K2) > K2 - K1. Put: bid P(K2) - ask P(K1) > K2 - K1. All strike pairs.
//! 3. Butterfly. w = (K3 - K2) / (K3 - K1): bid X(K2) > w ask X(K1) + (1 - w) ask X(K3). Consecutive strike triples.
//! 4. Calendar (American options only). Same root, type and strike, T1 < T2: bid X(T1) > ask X(T2). All expiry pairs;
//!    the pair is counted on the series of T2.
//! 5. Parity upper bound (American options only): bid C - ask P > S - K df. S is the snapshot spot; a stock price has
//!    no bid and no ask, so the spot is a last price of the same file.
//! 6. Intrinsic (American options only): ask below the intrinsic value. The quote leaves the other checks.
//! 7. Bound (American options only): bid C > S, or bid P > K. The quote leaves the other checks.
//!
//! A quote is used when ask >= bid > 0. A quote with bid 0 (or no bid) and an ask above 0 is a buy-only quote: its
//! ask is a buy leg and it is never a sell leg. The other quotes are counted by reason in `Report::dropped`.
//!
//! A quote is marked when its Black-76 implied volatility (forward from put-call parity, mid of bid and ask) is far from
//! the median of its two neighbours on each side in the same series and type (`CheckOpts::iv_mark`). A break with a marked
//! leg is counted apart. The marker is a flag on the quote: the checks still use it.

use super::option_history::occ_root;
use super::option_iv::{forward_from_parity, forward_from_parity_blind, implied_vol};
use super::options::OptionChainSnapshot;
use crate::trading::calendar::parse_iso;
use std::collections::{BTreeMap, BTreeSet};

pub const CHECK_NAMES: [&str; 7] = ["monotony", "slope", "butterfly", "calendar", "parity", "intrinsic", "bound"];
/// Index of the checks that remove a quote (intrinsic, bound).
pub const REMOVING: [usize; 2] = [5, 6];
const EPS: f64 = 1e-9;

/// The indexes with European options (cash settled): no calendar test, no parity bound, no intrinsic test, no bound.
pub const EUROPEAN_INDEXES: [&str; 5] = ["SPX", "XSP", "NDX", "RUT", "VIX"];

pub fn is_european(ticker: &str) -> bool {
    EUROPEAN_INDEXES.contains(&ticker.to_uppercase().as_str())
}

/// The size bands of the breaks, in dollars for one share: below 0.10, 0.10 to 1, 1 to 10, 10 and above.
pub const BAND_NAMES: [&str; 4] = ["below 0.10", "0.10 to 1", "1 to 10", "10 and above"];

pub fn band_of(size: f64) -> usize {
    if size < 0.10 {
        0
    } else if size < 1.0 {
        1
    } else if size < 10.0 {
        2
    } else {
        3
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CheckOpts {
    /// A break below this size (USD) is not counted or listed.
    pub min_size: f64,
    /// A quote is marked when its implied volatility differs from the median of its neighbours by more than this (volatility, 0.10 = 10 points). 0 or less: no marking.
    pub iv_mark: f64,
    /// Keep only the quotes out of the money or within 1 % of the forward of their series: a call with K >= 0.99 F, a put with K <= 1.01 F (the rule of the volatility fit).
    pub otm: bool,
}

impl Default for CheckOpts {
    fn default() -> Self {
        CheckOpts { min_size: 0.0, iv_mark: 0.10, otm: false }
    }
}

/// The tick of a price: 0.01 below $3, else 0.05 (the penny pilot rule; an assumption for the ticks, not a rule of each class).
pub fn tick_of(price: f64) -> f64 {
    if price < 3.0 {
        0.01
    } else {
        0.05
    }
}

#[derive(Debug, Clone)]
struct Used {
    symbol: String,
    call: bool,
    strike: f64,
    /// 0 for a buy-only quote.
    bid: f64,
    ask: f64,
}

impl Used {
    fn buy_only(&self) -> bool {
        self.bid <= 0.0
    }
}

#[derive(Debug, Clone)]
pub struct Break {
    /// 0 to 6: index in `CHECK_NAMES`.
    pub check: usize,
    pub root: String,
    pub expiry: String,
    /// The amount by which the bound is broken, in dollars (for one share).
    pub size: f64,
    /// The size in ticks; the tick is by the highest price among the legs.
    pub ticks: f64,
    pub text: String,
    /// The contract symbols of the legs.
    pub symbols: Vec<String>,
    /// A leg is a marked quote (far from its neighbours in implied volatility).
    pub marked: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SeriesRow {
    pub root: String,
    pub expiry: String,
    pub days: i64,
    pub quotes_used: usize,
    pub tests: [usize; 7],
    pub breaks: [usize; 7],
    /// The number of distinct quotes in at least one break of the series.
    pub distinct: usize,
    /// The number of marked quotes of the series.
    pub marked: usize,
    /// The number of breaks with no marked leg.
    pub clean: usize,
    pub largest: f64,
    /// The parity forward from the mids of the strikes nearest the spot.
    pub forward: Option<f64>,
    pub rate: Option<f64>,
    /// q = r - ln(F / S) / t. An approximation for American options.
    pub implied_div: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct Dropped {
    /// No bid (or 0) and no ask above 0.
    pub no_bid: usize,
    pub no_ask: usize,
    pub ask_below_bid: usize,
    pub bad_symbol: usize,
    pub bad_expiry: usize,
    /// An expiry on or before the snapshot date.
    pub past_expiry: usize,
    /// An option type that is not "call" or "put".
    pub bad_type: usize,
}

impl Dropped {
    pub fn total(&self) -> usize {
        self.no_bid + self.no_ask + self.ask_below_bid + self.bad_symbol + self.bad_expiry + self.past_expiry + self.bad_type
    }
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub rows: Vec<SeriesRow>,
    pub breaks: Vec<Break>,
    pub dropped: Dropped,
    pub quotes_total: usize,
    pub quotes_used: usize,
    /// Used quotes with bid 0 and an ask above 0 (buy legs only).
    pub buy_only: usize,
    /// Quotes with an ask below the intrinsic value (American options): not used in the other checks.
    pub below_intrinsic: usize,
    /// Quotes with a bid above the bound (call above S, put above K): not used in the other checks.
    pub above_bound: usize,
    /// Quotes removed by the out-of-the-money filter, and the series with no forward (no filter).
    pub otm_removed: usize,
    pub otm_no_forward: usize,
    /// The marked quotes (symbols).
    pub marked: BTreeSet<String>,
    pub opts: Option<CheckOpts>,
}

impl Report {
    /// The breaks of the checks 1 to 5 (not the removing checks) and their number in the four size bands.
    pub fn bands(&self, clean_only: bool) -> [usize; 4] {
        let mut n = [0; 4];
        for b in self.breaks.iter().filter(|b| !REMOVING.contains(&b.check) && !(clean_only && b.marked)) {
            n[band_of(b.size)] += 1;
        }
        n
    }
}

type StrikeMap<'a> = BTreeMap<(String, bool, u64), Vec<(&'a String, &'a Used)>>;

fn push_break(out: &mut Vec<Break>, check: usize, root: &str, expiry: &str, size: f64, legs: &[(&str, f64)], text: String) {
    let top = legs.iter().map(|l| l.1).fold(0.0, f64::max);
    out.push(Break { check, root: root.into(), expiry: expiry.into(), size, ticks: size / tick_of(top), text, symbols: legs.iter().map(|l| l.0.to_string()).collect(), marked: false });
}

fn kind(call: bool) -> &'static str {
    if call {
        "call"
    } else {
        "put"
    }
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// The symbols of the quotes of one side (one type) whose implied volatility is far from the median of their neighbours:
/// up to 2 neighbours below and 2 above in the strike order, among the quotes that have an implied volatility. At least
/// 2 neighbours are needed. A quote without a bid has no mid and no volatility.
fn marked_quotes(side: &[&Used], call: bool, forward: f64, t: f64, df: f64, threshold: f64) -> Vec<String> {
    let ivs: Vec<(&Used, f64)> = side.iter().filter(|q| q.bid > 0.0).filter_map(|q| implied_vol(call, 0.5 * (q.bid + q.ask), forward, q.strike, t, df).map(|v| (*q, v))).collect();
    let mut out = Vec::new();
    for (pos, (q, iv)) in ivs.iter().enumerate() {
        let mut nb: Vec<f64> = ivs[pos.saturating_sub(2)..pos].iter().map(|x| x.1).collect();
        nb.extend(ivs[pos + 1..(pos + 3).min(ivs.len())].iter().map(|x| x.1));
        if nb.len() >= 2 && (iv - median(&mut nb)).abs() > threshold {
            out.push(q.symbol.clone());
        }
    }
    out
}

/// Run the checks. `american`: stocks and ETFs (false for a European index such as SPX: no calendar, parity, intrinsic or bound test).
/// `rate(date, t)` gives the rate for `t` years (a flat rate or a curve); the date is the snapshot date.
pub fn check_chain(snap: &OptionChainSnapshot, american: bool, rate: &mut dyn FnMut(&str, f64) -> Result<f64, String>, opts: &CheckOpts) -> Result<Report, String> {
    let day = parse_iso(&snap.snapshot_date).ok_or_else(|| format!("bad snapshot date {}", snap.snapshot_date))?;
    let mut rep = Report { quotes_total: snap.quotes.len(), opts: Some(*opts), ..Default::default() };
    let mut series: BTreeMap<(String, String), Vec<Used>> = BTreeMap::new();
    for q in &snap.quotes {
        let root = occ_root(&q.contract_symbol);
        if root.is_empty() {
            rep.dropped.bad_symbol += 1;
            continue;
        }
        let Some(exp_day) = parse_iso(&q.expiry) else {
            rep.dropped.bad_expiry += 1;
            continue;
        };
        if exp_day <= day {
            rep.dropped.past_expiry += 1;
            continue;
        }
        if q.option_type != "call" && q.option_type != "put" {
            rep.dropped.bad_type += 1;
            continue;
        }
        let bid = q.bid.unwrap_or(0.0);
        let Some(ask) = q.ask else {
            rep.dropped.no_ask += 1;
            continue;
        };
        let no_bid = bid.is_nan() || bid <= 0.0;
        if no_bid && (ask.is_nan() || ask <= 0.0) {
            rep.dropped.no_bid += 1;
            continue;
        }
        if !no_bid && (ask.is_nan() || ask < bid) {
            rep.dropped.ask_below_bid += 1;
            continue;
        }
        if no_bid {
            rep.buy_only += 1;
        }
        rep.quotes_used += 1;
        series.entry((root, q.expiry.clone())).or_default().push(Used { symbol: q.contract_symbol.clone(), call: q.option_type == "call", strike: q.strike, bid: if no_bid { 0.0 } else { bid }, ask });
    }
    let mut rows: BTreeMap<(String, String), SeriesRow> = BTreeMap::new();
    let mut breaks: Vec<Break> = Vec::new();

    let mut good: BTreeMap<(String, String), Vec<Used>> = BTreeMap::new();
    let spot = snap.spot_price.filter(|s| *s > 0.0);
    for ((root, expiry), all) in &series {
        let days = parse_iso(expiry).map_or(0, |e| (e - day) as i64);
        let t = days as f64 / 365.0;
        let r = rate(&snap.snapshot_date, t.max(1.0 / 365.0))?;
        let df = (-r * t).exp();
        let mut row = SeriesRow { root: root.clone(), expiry: expiry.clone(), days, rate: Some(r), ..Default::default() };
        let filtered: Vec<Used>;
        let all: &Vec<Used> = if opts.otm {
            let mut pairs = Vec::new();
            for c in all.iter().filter(|q| q.call && !q.buy_only()) {
                if let Some(p) = all.iter().find(|q| !q.call && !q.buy_only() && q.strike == c.strike) {
                    pairs.push((c.strike, 0.5 * (c.bid + c.ask), 0.5 * (p.bid + p.ask)));
                }
            }
            let fwd = spot.and_then(|s| forward_from_parity(&pairs, s, df)).or_else(|| forward_from_parity_blind(&pairs, df));
            match fwd {
                Some(f) => {
                    filtered = all.iter().filter(|q| if q.call { q.strike >= 0.99 * f } else { q.strike <= 1.01 * f }).cloned().collect();
                    rep.otm_removed += all.len() - filtered.len();
                    &filtered
                }
                None => {
                    rep.otm_no_forward += 1;
                    all
                }
            }
        } else {
            all
        };
        // 5 intrinsic (American options): an ask below the intrinsic value. Buy at the ask and exercise at once.
        // 6 bound: a bid above S (call) or K (put): sell it. S is a last price of the file, not a bid or an ask, and the quotes
        // are not at one time. A quote of these two checks leaves the other checks.
        let mut quotes: Vec<Used> = Vec::new();
        for q in all {
            if let (true, Some(s)) = (american, spot) {
                row.tests[5] += 1;
                let intrinsic = if q.call { s - q.strike } else { q.strike - s };
                if q.ask + EPS < intrinsic {
                    rep.below_intrinsic += 1;
                    push_break(&mut breaks, 5, root, expiry, intrinsic - q.ask, &[(&q.symbol, q.ask)], format!("{}: ask {} of {} is below the intrinsic value {:.4} (S {:.2} less K, or K less S; S is a last price)", kind(q.call), q.ask, q.symbol, intrinsic, s));
                    continue;
                }
                if !q.buy_only() {
                    row.tests[6] += 1;
                    let cap = if q.call { s } else { q.strike };
                    if q.bid > cap + EPS {
                        rep.above_bound += 1;
                        push_break(&mut breaks, 6, root, expiry, q.bid - cap, &[(&q.symbol, q.bid)], format!("{}: bid {} of {} is above {:.2} ({}; S is a last price)", kind(q.call), q.bid, q.symbol, cap, if q.call { "S" } else { "K" }));
                        continue;
                    }
                }
            }
            quotes.push(q.clone());
        }
        row.quotes_used = quotes.len();
        for call in [true, false] {
            let mut side: Vec<&Used> = quotes.iter().filter(|q| q.call == call).collect();
            side.sort_by(|a, b| a.strike.total_cmp(&b.strike));
            let tag = kind(call);
            for i in 0..side.len() {
                for j in i + 1..side.len() {
                    let (a, b) = (side[i], side[j]);
                    if a.strike >= b.strike {
                        continue; // a duplicate strike
                    }
                    // 1 monotony: the lower-strike call is worth more; the higher-strike put is worth more.
                    row.tests[0] += 1;
                    let (cheap, dear) = if call { (b, a) } else { (a, b) }; // `dear` must be worth at least `cheap`
                    let size = cheap.bid - dear.ask;
                    if !cheap.buy_only() && size > EPS {
                        push_break(&mut breaks, 0, root, expiry, size, &[(&cheap.symbol, cheap.bid), (&dear.symbol, dear.ask)], format!("{tag}: bid {} of {} is above ask {} of {}", cheap.bid, cheap.symbol, dear.ask, dear.symbol));
                    }
                    // 2 slope: the price difference is not more than the strike difference.
                    row.tests[1] += 1;
                    let width = b.strike - a.strike;
                    let (s, l) = if call { (a, b) } else { (b, a) };
                    let size = s.bid - l.ask - width;
                    if !s.buy_only() && size > EPS {
                        push_break(&mut breaks, 1, root, expiry, size, &[(&s.symbol, s.bid), (&l.symbol, l.ask)], format!("{tag}: bid {} of {} less ask {} of {} is above the strike difference {}", s.bid, s.symbol, l.ask, l.symbol, width));
                    }
                }
            }
            // 3 butterfly on consecutive strikes
            for w3 in side.windows(3) {
                let (a, m, b) = (w3[0], w3[1], w3[2]);
                if !(a.strike < m.strike && m.strike < b.strike) {
                    continue;
                }
                row.tests[2] += 1;
                let w = (b.strike - m.strike) / (b.strike - a.strike);
                let bound = w * a.ask + (1.0 - w) * b.ask;
                let size = m.bid - bound;
                if !m.buy_only() && size > EPS {
                    push_break(&mut breaks, 2, root, expiry, size, &[(&m.symbol, m.bid), (&a.symbol, a.ask), (&b.symbol, b.ask)], format!("{tag}: bid {} of {} is above {:.4} = {:.4} x ask {} of {} + {:.4} x ask {} of {}", m.bid, m.symbol, bound, w, a.ask, a.symbol, 1.0 - w, b.ask, b.symbol));
                }
            }
        }
        // 4 parity upper bound, and the implied dividend (a quote with no bid has no mid: it is not in the pairs)
        let mut pairs_mid = Vec::new();
        for c in quotes.iter().filter(|q| q.call) {
            let Some(p) = quotes.iter().find(|q| !q.call && q.strike == c.strike) else { continue };
            if !(c.buy_only() || p.buy_only()) {
                pairs_mid.push((c.strike, 0.5 * (c.bid + c.ask), 0.5 * (p.bid + p.ask)));
            }
            if let (true, Some(s)) = (american, spot) {
                row.tests[4] += 1;
                let bound = s - c.strike * df;
                let size = c.bid - p.ask - bound;
                if !c.buy_only() && size > EPS {
                    push_break(&mut breaks, 4, root, expiry, size, &[(&c.symbol, c.bid), (&p.symbol, p.ask)], format!("bid {} of {} less ask {} of {} is above S - K df = {:.4} (S {:.2} is a last price, not a bid or ask)", c.bid, c.symbol, p.ask, p.symbol, bound, s));
                }
            }
        }
        if let Some(s) = spot {
            if let Some(f) = forward_from_parity(&pairs_mid, s, df) {
                row.forward = Some(f);
                if days > 0 {
                    row.implied_div = Some(r - (f / s).ln() / t);
                }
            }
        }
        // the marker: implied volatility against the neighbours
        if opts.iv_mark > 0.0 {
            let forward = row.forward.or_else(|| forward_from_parity_blind(&pairs_mid, df)).or(spot.map(|s| s / df));
            if let Some(f) = forward {
                for call in [true, false] {
                    let mut side: Vec<&Used> = quotes.iter().filter(|q| q.call == call).collect();
                    side.sort_by(|a, b| a.strike.total_cmp(&b.strike));
                    for sym in marked_quotes(&side, call, f, t, df, opts.iv_mark) {
                        if rep.marked.insert(sym) {
                            row.marked += 1;
                        }
                    }
                }
            }
        }
        good.insert((root.clone(), expiry.clone()), quotes);
        rows.insert((root.clone(), expiry.clone()), row);
    }

    // 3 calendar, American only
    if american {
        let mut by_strike: StrikeMap = BTreeMap::new();
        for ((root, expiry), quotes) in &good {
            for q in quotes {
                by_strike.entry((root.clone(), q.call, q.strike.to_bits())).or_default().push((expiry, q));
            }
        }
        for ((root, call, _), mut v) in by_strike {
            v.sort_by(|a, b| a.0.cmp(b.0));
            for i in 0..v.len() {
                for j in i + 1..v.len() {
                    let ((e1, q1), (e2, q2)) = (v[i], v[j]);
                    if let Some(row) = rows.get_mut(&(root.clone(), e2.clone())) {
                        row.tests[3] += 1;
                    }
                    let size = q1.bid - q2.ask;
                    if !q1.buy_only() && size > EPS {
                        push_break(&mut breaks, 3, &root, e2, size, &[(&q1.symbol, q1.bid), (&q2.symbol, q2.ask)], format!("{}: bid {} of {} ({e1}) is above ask {} of {} ({e2})", kind(call), q1.bid, q1.symbol, q2.ask, q2.symbol));
                    }
                }
            }
        }
    }

    breaks.retain(|b| b.size >= opts.min_size);
    for b in breaks.iter_mut() {
        b.marked = b.symbols.iter().any(|s| rep.marked.contains(s));
    }
    breaks.sort_by(|a, b| b.size.total_cmp(&a.size));
    let mut seen: BTreeMap<(String, String), BTreeSet<&str>> = BTreeMap::new();
    for b in &breaks {
        let key = (b.root.clone(), b.expiry.clone());
        if let Some(row) = rows.get_mut(&key) {
            row.breaks[b.check] += 1;
            row.largest = row.largest.max(b.size);
            if !b.marked {
                row.clean += 1;
            }
        }
        seen.entry(key).or_default().extend(b.symbols.iter().map(String::as_str));
    }
    for (key, set) in &seen {
        if let Some(row) = rows.get_mut(key) {
            row.distinct = set.len();
        }
    }
    rep.rows = rows.into_values().collect();
    rep.rows.sort_by(|a, b| (&a.expiry, &a.root).cmp(&(&b.expiry, &b.root)));
    rep.breaks = breaks;
    Ok(rep)
}

/// The result of the comparison of two days of one chain.
#[derive(Debug, Clone)]
pub struct Compare {
    pub date1: String,
    pub date2: String,
    /// The breaks of day 1 (checks 1 to 5; the removing checks 6 and 7 are listed apart) and the number of them.
    pub total: usize,
    /// Breaks whose series has an expiry on or before day 2: they cannot stay.
    pub expired: usize,
    /// The breaks of day 1 with the same check and the same legs on day 2: (break of day 1, size on day 2).
    pub stay: Vec<(Break, f64)>,
    /// The same for the removing checks (intrinsic, bound): the quote on both days.
    pub total_removing: usize,
    pub stay_removing: usize,
}

/// Which breaks of `a` (day 1) stay on `b` (day 2): the same check and the same legs. A leg is a contract symbol.
pub fn compare(a: &Report, b: &Report, date1: &str, date2: &str) -> Compare {
    let key = |x: &Break| (x.check, { let mut s = x.symbols.clone(); s.sort(); s });
    let on_b: BTreeMap<(usize, Vec<String>), f64> = b.breaks.iter().map(|x| (key(x), x.size)).collect();
    let mut c = Compare { date1: date1.into(), date2: date2.into(), total: 0, expired: 0, stay: Vec::new(), total_removing: 0, stay_removing: 0 };
    for x in &a.breaks {
        let hit = on_b.get(&key(x)).copied();
        if REMOVING.contains(&x.check) {
            c.total_removing += 1;
            c.stay_removing += hit.is_some() as usize;
            continue;
        }
        c.total += 1;
        if x.expiry.as_str() <= date2 {
            c.expired += 1;
        }
        if let Some(sz) = hit {
            c.stay.push((x.clone(), sz));
        }
    }
    c
}

pub fn render_compare(ticker: &str, c: &Compare, top: usize) -> String {
    let marked = c.stay.iter().filter(|(b, _)| b.marked).count();
    let mut s = format!(
        "\ncompare {ticker} {} with {}: {} breaks (checks 1 to 5) on {}; {} cannot stay (an expiry on or before {}); {} stay (same check, same legs): {} with a marked leg, {} without. Intrinsic and bound breaks: {} of {} stay.\n",
        c.date1, c.date2, c.total, c.date1, c.expired, c.date2, c.stay.len(), marked, c.stay.len() - marked, c.stay_removing, c.total_removing
    );
    let mut stay: Vec<&(Break, f64)> = c.stay.iter().collect();
    stay.sort_by(|a, b| b.0.size.total_cmp(&a.0.size));
    for (b, sz2) in stay.iter().take(top).map(|x| (&x.0, x.1)) {
        s.push_str(&format!("  {:>9.2} $ on {}, {:>9.2} $ on {}  {} {} {}{}: {}\n", b.size, c.date1, sz2, c.date2, b.root, b.expiry, CHECK_NAMES[b.check], if b.marked { " (marked leg)" } else { "" }, b.text));
    }
    s.push_str("  A break that stays is more likely a quote that is kept, or a price. It is not shown to be tradable: the time of the quotes and the costs are not known.\n");
    s
}

/// The text for a person: the table of the series, the largest breaks, the bands, the limits.
pub fn render(ticker: &str, snap: &OptionChainSnapshot, rep: &Report, american: bool, top: usize) -> String {
    let mut s = String::new();
    let d = &rep.dropped;
    let opts = rep.opts.unwrap_or_default();
    s.push_str(&format!(
        "options check {ticker} {}: spot {} ({}), {} quotes, {} used ({} with bid 0 and an ask above 0: buy legs only), {} dropped (no bid and no ask above 0: {}, ask missing: {}, ask below bid: {}, bad symbol: {}, bad expiry: {}, expiry on or before the snapshot date: {}, option type not call or put: {}); {} used quotes have an ask below the intrinsic value and {} have a bid above S or K: they are removed from the other checks. {} quotes are marked (implied volatility more than {:.0} points from the median of the neighbours). Size floor: {} USD. {}\n",
        snap.snapshot_date,
        snap.spot_price.map_or("none".to_string(), |x| format!("{x:.2}")),
        if american { "American options" } else { "European index: no calendar test, no parity bound, no intrinsic test, no bound test" },
        rep.quotes_total,
        rep.quotes_used,
        rep.buy_only,
        d.total(),
        d.no_bid,
        d.no_ask,
        d.ask_below_bid,
        d.bad_symbol,
        d.bad_expiry,
        d.past_expiry,
        d.bad_type,
        rep.below_intrinsic,
        rep.above_bound,
        rep.marked.len(),
        opts.iv_mark * 100.0,
        opts.min_size,
        if opts.otm {
            format!("Strike filter on (--otm): only quotes out of the money or within 1 % of the forward of their series (call K >= 0.99 F, put K <= 1.01 F); {} quotes removed; {} series had no forward and were not filtered. --all-strikes turns it off.", rep.otm_removed, rep.otm_no_forward)
        } else {
            "Strike filter off: all strikes.".to_string()
        }
    ));
    match snap.source.as_deref() {
        None => s.push_str(&format!("source none, quote_kind {:?}: a file with no source field is a Cboe delayed snapshot; the file has no time of the quotes.\n\n", snap.quote_kind)),
        Some("alpaca") => s.push_str(&format!("source alpaca, feed {:?}, quote_kind {:?}: Alpaca indicative feed: modified quotes, not executable prices (#280). A break here is not a finding.\n\n", snap.feed, snap.quote_kind)),
        Some(other) => s.push_str(&format!("source {other}, feed {:?}, quote_kind {:?}: the time and the kind of the quotes must be checked before a break is a finding.\n\n", snap.feed, snap.quote_kind)),
    }
    s.push_str("Each cell is breaks/tests. int = ask below intrinsic value, bnd = bid above S or K (American options only; S is a last price, not a bid or ask; the quotes are not at one time), distinct = quotes in at least one break, marked = marked quotes, clean = breaks with no marked leg. mono = monotony, slope, bfly = butterfly, cal = calendar (counted on the later expiry), par = parity bound.\n");
    s.push_str(&format!("{:<6} {:<10} {:>4} {:>6} {:>9} {:>7} {:>9} {:>9} {:>9} {:>9} {:>9} {:>8} {:>6} {:>6} {:>9} {:>8} {:>7}\n", "root", "expiry", "days", "quotes", "int", "bnd", "mono", "slope", "bfly", "cal", "par", "distinct", "marked", "clean", "max $", "rate %", "q %"));
    for r in &rep.rows {
        let cell = |i: usize| format!("{}/{}", r.breaks[i], r.tests[i]);
        s.push_str(&format!(
            "{:<6} {:<10} {:>4} {:>6} {:>9} {:>7} {:>9} {:>9} {:>9} {:>9} {:>9} {:>8} {:>6} {:>6} {:>9.2} {:>8} {:>7}\n",
            r.root,
            r.expiry,
            r.days,
            r.quotes_used,
            cell(5),
            cell(6),
            cell(0),
            cell(1),
            cell(2),
            cell(3),
            cell(4),
            r.distinct,
            r.marked,
            r.clean,
            r.largest,
            r.rate.map_or("-".into(), |x| format!("{:.2}", x * 100.0)),
            r.implied_div.map_or("-".into(), |x| format!("{:.2}", x * 100.0)),
        ));
    }
    let removing = rep.breaks.iter().filter(|b| REMOVING.contains(&b.check)).count();
    let rest: Vec<&Break> = rep.breaks.iter().filter(|b| !REMOVING.contains(&b.check)).collect();
    let clean: Vec<&Break> = rest.iter().copied().filter(|b| !b.marked).collect();
    s.push_str(&format!(
        "\nbreaks: {} (intrinsic and bound: {removing}; checks 1 to 5 after the removal: {}, of which with a marked leg: {}, with no marked leg: {})\n",
        rep.breaks.len(),
        rest.len(),
        rest.len() - clean.len(),
        clean.len()
    ));
    let (all, cl) = (rep.bands(false), rep.bands(true));
    s.push_str("size bands of the checks 1 to 5 (USD for one share): all breaks / breaks with no marked leg\n");
    for i in 0..4 {
        s.push_str(&format!("  {:<13} {:>6} / {:>6}\n", BAND_NAMES[i], all[i], cl[i]));
    }
    s.push_str(&format!("the {} largest of the checks 1 to 5 with no marked leg:\n", top.min(clean.len())));
    for b in clean.iter().take(top) {
        s.push_str(&format!("  {:>9.2} $ {:>8.1} ticks  {} {} {}: {}\n", b.size, b.ticks, b.root, b.expiry, CHECK_NAMES[b.check], b.text));
    }
    let marked_rest: Vec<&&Break> = rest.iter().filter(|b| b.marked).collect();
    s.push_str(&format!("the {} largest of the checks 1 to 5 with a marked leg:\n", top.min(marked_rest.len())));
    for b in marked_rest.iter().take(top) {
        s.push_str(&format!("  {:>9.2} $ {:>8.1} ticks  {} {} {}: {}\n", b.size, b.ticks, b.root, b.expiry, CHECK_NAMES[b.check], b.text));
    }
    s.push_str(&format!("the {} largest intrinsic and bound breaks:\n", top.min(removing)));
    for b in rep.breaks.iter().filter(|b| REMOVING.contains(&b.check)).take(top) {
        s.push_str(&format!("  {:>9.2} $ {:>8.1} ticks  {} {} {}: {}\n", b.size, b.ticks, b.root, b.expiry, CHECK_NAMES[b.check], b.text));
    }
    s.push_str("\nNotes:\n");
    s.push_str("  Prices are executable: buy at the ask, sell at the bid. A break is not a profit: no costs, no borrow, no time of the quotes, early exercise not priced.\n");
    s.push_str("  A quote with bid 0 is a buy leg only (its ask), never a sell leg.\n");
    s.push_str("  Ticks: size / tick, tick 0.01 below $3, else 0.05, by the highest price among the legs.\n");
    s.push_str("  Monotony and slope use all strike pairs, so one bad quote makes many breaks. Butterfly uses consecutive strikes.\n");
    s.push_str("  Marked quote: Black-76 implied volatility of the mid, forward from parity, against the median of up to 2 neighbours on each side (same series and type). A marked quote is flagged and still used. An American quote deep in the money can have a distorted volatility.\n");
    s.push_str("  Parity: the stock price has no bid and no ask; S is the snapshot spot. q = r - ln(F / S) / t, F from the mids of the 3 strikes nearest the spot: an approximation for American options (early exercise and borrow are inside q).\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::options::OptionQuote;

    fn q(sym: &str, call: bool, expiry: &str, strike: f64, bid: f64, ask: f64) -> OptionQuote {
        OptionQuote { contract_symbol: sym.into(), option_type: if call { "call" } else { "put" }.into(), expiry: expiry.into(), strike, last_price: None, bid: Some(bid), ask: Some(ask), volume: None, open_interest: None, implied_volatility: None, in_the_money: None }
    }
    fn sym(root: &str, ymd: &str, call: bool, strike: f64) -> String {
        format!("{root}{ymd}{}{:08}", if call { 'C' } else { 'P' }, (strike * 1000.0).round() as i64)
    }
    fn snap(quotes: Vec<OptionQuote>) -> OptionChainSnapshot {
        OptionChainSnapshot { ticker: "XYZ".into(), snapshot_date: "2026-01-02".into(), spot_price: Some(100.0), quotes, source: None, feed: None, quote_kind: None, note: None, snapshot_time: None }
    }
    fn add(v: &mut Vec<OptionQuote>, root: &str, ymd: &str, expiry: &str, call: bool, strike: f64, bid: f64, ask: f64) {
        v.push(q(&sym(root, ymd, call, strike), call, expiry, strike, bid, ask));
    }
    fn zero(_: &str, _: f64) -> Result<f64, String> {
        Ok(0.0)
    }

    /// A clean chain at rate 0, spot 100, two expiries, strikes 90/100/110. Calls are convex and falling, puts are convex and rising.
    fn clean() -> Vec<OptionQuote> {
        let mut v = Vec::new();
        for (ymd, e, table) in [
            ("260116", "2026-01-16", [(90.0, 11.0, 1.0), (100.0, 4.0, 4.0), (110.0, 1.0, 11.0)]),
            ("260220", "2026-02-20", [(90.0, 12.0, 2.0), (100.0, 5.0, 5.0), (110.0, 1.5, 12.0)]),
        ] {
            for (k, c, p) in table {
                add(&mut v, "XYZ", ymd, e, true, k, c - 0.1, c + 0.1);
                add(&mut v, "XYZ", ymd, e, false, k, p - 0.1, p + 0.1);
            }
        }
        v
    }
    fn run(v: Vec<OptionQuote>) -> Report {
        check_chain(&snap(v), true, &mut zero, &CheckOpts::default()).unwrap()
    }
    fn count(r: &Report, c: usize) -> usize {
        r.breaks.iter().filter(|b| b.check == c).count()
    }

    #[test]
    fn a_clean_chain_has_no_break() {
        let r = run(clean());
        assert_eq!(r.breaks.len(), 0, "{:?}", r.breaks);
        assert_eq!(r.rows.len(), 2);
        assert!(r.rows[0].tests.iter().take(3).all(|t| *t > 0));
        assert_eq!(r.rows[1].tests[3], 6, "calendar pairs are counted on the later expiry");
        assert_eq!(r.rows[0].tests[4], 3);
    }

    #[test]
    fn planted_monotony_break_has_the_exact_size() {
        let mut v = clean();
        // Call 110 bid 4.5 is above ask 4.1 of call 100: size 0.4.
        let i = v.iter().position(|x| x.strike == 110.0 && x.option_type == "call" && x.expiry == "2026-01-16").unwrap();
        v[i].bid = Some(4.5);
        v[i].ask = Some(4.6);
        let r = run(v);
        let m: Vec<_> = r.breaks.iter().filter(|b| b.check == 0).collect();
        assert_eq!(m.len(), 1);
        assert!((m[0].size - 0.4).abs() < 1e-9, "{}", m[0].size); // 4.5 - 4.1
        assert!((m[0].ticks - 8.0).abs() < 1e-6, "{}", m[0].ticks); // top price 4.5 is above 3: tick 0.05 -> 8 ticks
    }

    #[test]
    fn planted_slope_break_has_the_exact_size() {
        let mut v = clean();
        // Put 110 of the first expiry: bid 15.0 less ask 4.1 of put 100 is 10.9; the width is 10; size 0.9.
        let i = v.iter().position(|x| x.strike == 110.0 && x.option_type == "put" && x.expiry == "2026-01-16").unwrap();
        v[i].bid = Some(15.0);
        v[i].ask = Some(15.2);
        let r = run(v);
        let s: Vec<_> = r.breaks.iter().filter(|b| b.check == 1).collect();
        // Pair (90,110): 15 - 1.1 - 20 is below 0, no break. Pair (100,110): 15 - 4.1 - 10 = 0.9.
        assert_eq!(s.len(), 1, "{:?}", s);
        assert!((s[0].size - 0.9).abs() < 1e-9, "{}", s[0].size);
    }

    #[test]
    fn planted_butterfly_break_has_the_exact_size() {
        let mut v = clean();
        // Bound for the call 100: 0.5 x ask 11.1 (call 90) + 0.5 x ask 1.1 (call 110) = 6.1. A bid 7.0 breaks it by 0.9.
        // Monotony stays clean: bid 7.0 is below ask 11.1.
        let i = v.iter().position(|x| x.strike == 100.0 && x.option_type == "call" && x.expiry == "2026-01-16").unwrap();
        v[i].bid = Some(7.0);
        v[i].ask = Some(7.2);
        let r = run(v);
        let b: Vec<_> = r.breaks.iter().filter(|b| b.check == 2).collect();
        assert_eq!(b.len(), 1, "{:?}", r.breaks);
        assert!((b[0].size - 0.9).abs() < 1e-9, "{}", b[0].size);
    }

    #[test]
    fn planted_calendar_break_has_the_exact_size() {
        let mut v = clean();
        // First expiry put 100 bid 5.5; second expiry put 100 ask 5.1: break 0.4. The other checks stay clean.
        let i = v.iter().position(|x| x.strike == 100.0 && x.option_type == "put" && x.expiry == "2026-01-16").unwrap();
        v[i].bid = Some(5.5);
        v[i].ask = Some(5.6);
        let r = run(v);
        let c: Vec<_> = r.breaks.iter().filter(|b| b.check == 3).collect();
        assert_eq!(c.len(), 1, "{:?}", r.breaks);
        assert!((c[0].size - 0.4).abs() < 1e-9, "{}", c[0].size);
        assert_eq!(c[0].expiry, "2026-02-20");
        // An index (European) has no calendar test.
        let mut v2 = clean();
        v2[i].bid = Some(5.5);
        v2[i].ask = Some(5.6);
        let r2 = check_chain(&snap(v2), false, &mut zero, &CheckOpts::default()).unwrap();
        assert_eq!(count(&r2, 3), 0);
        assert!(r2.rows.iter().all(|x| x.tests[3] == 0 && x.tests[4] == 0));
    }

    #[test]
    fn planted_parity_break_has_the_exact_size() {
        let mut v = clean();
        // Rate 0, S = 100, K = 100: C - P <= 0. Call 100 bid 4.5, put 100 ask 4.1: 0.4 above 0.
        let i = v.iter().position(|x| x.strike == 100.0 && x.option_type == "call" && x.expiry == "2026-01-16").unwrap();
        v[i].bid = Some(4.5);
        v[i].ask = Some(4.6);
        let r = run(v);
        let p: Vec<_> = r.breaks.iter().filter(|b| b.check == 4).collect();
        assert_eq!(p.len(), 1, "{:?}", r.breaks);
        assert!((p[0].size - 0.4).abs() < 1e-9, "{}", p[0].size);
        // The other tests: the same quote is not a break of monotony (bid 4.5 < ask 11.1 of the 90 call).
        assert_eq!(count(&r, 0) + count(&r, 1) + count(&r, 2), 0);
    }

    #[test]
    fn a_quote_below_intrinsic_is_one_break_and_leaves_the_other_checks() {
        let mut v = clean();
        // Call 90 of the first expiry: ask 3.0 against the intrinsic value 10 (S 100). Kept in the checks it would break monotony (bid 3.9 of the call 100 above its ask 3.0).
        let i = v.iter().position(|x| x.strike == 90.0 && x.option_type == "call" && x.expiry == "2026-01-16").unwrap();
        v[i].bid = Some(2.9);
        v[i].ask = Some(3.0);
        let r = run(v.clone());
        let b: Vec<_> = r.breaks.iter().filter(|b| b.check == 5).collect();
        assert_eq!(b.len(), 1, "{:?}", r.breaks);
        assert!((b[0].size - 7.0).abs() < 1e-9, "{}", b[0].size);
        assert_eq!(r.breaks.len(), 1, "no other break: {:?}", r.breaks);
        assert_eq!((r.below_intrinsic, r.rows[0].distinct, r.rows[0].quotes_used, r.rows[0].tests[5]), (1, 1, 5, 6));
        // An index (European) has no intrinsic test: the quote stays and breaks monotony.
        let r2 = check_chain(&snap(v), false, &mut zero, &CheckOpts::default()).unwrap();
        assert_eq!(count(&r2, 5), 0);
        assert!(count(&r2, 0) >= 1);
    }

    #[test]
    fn a_series_with_an_expiry_on_or_before_the_snapshot_date_is_dropped_and_counted() {
        let mut v = clean();
        add(&mut v, "XYZ", "260102", "2026-01-02", true, 100.0, 1.0, 1.2);
        add(&mut v, "XYZ", "251219", "2025-12-19", false, 100.0, 1.0, 1.2);
        let mut odd = q(&sym("XYZ", "260116", true, 120.0), true, "2026-01-16", 120.0, 1.0, 1.2);
        odd.option_type = "weird".into();
        v.push(odd);
        let r = run(v);
        assert_eq!((r.dropped.past_expiry, r.dropped.bad_type), (2, 1));
        assert_eq!(r.rows.len(), 2, "no series for the past expiries");
        assert_eq!(r.breaks.len(), 0);
    }

    #[test]
    fn a_quote_with_bid_zero_is_not_used_and_is_counted() {
        let mut v = clean();
        let i = v.iter().position(|x| x.strike == 110.0 && x.option_type == "call" && x.expiry == "2026-01-16").unwrap();
        v[i].bid = Some(0.0);
        v[i].ask = Some(0.0);
        let j = v.iter().position(|x| x.strike == 90.0 && x.option_type == "put" && x.expiry == "2026-01-16").unwrap();
        v[j].bid = None;
        v[j].ask = Some(0.0);
        let k = v.iter().position(|x| x.strike == 90.0 && x.option_type == "put" && x.expiry == "2026-02-20").unwrap();
        v[k].bid = Some(3.0);
        v[k].ask = Some(2.0);
        let total = v.len();
        let r = run(v);
        assert_eq!((r.dropped.no_bid, r.dropped.ask_below_bid, r.quotes_used), (2, 1, total - 3));
        assert_eq!(r.breaks.len(), 0);
    }

    #[test]
    fn two_roots_of_one_expiry_are_two_series() {
        let mut v = Vec::new();
        for root in ["SPX", "SPXW"] {
            for (k, c) in [(90.0, 11.0), (100.0, 4.0), (110.0, 1.0)] {
                add(&mut v, root, "260116", "2026-01-16", true, k, c - 0.1, c + 0.1);
            }
        }
        // A call of SPXW at 110 with a bid above the SPX call at 100 must not be compared with it: it is another series.
        add(&mut v, "SPX", "260116", "2026-01-16", true, 120.0, 4.5, 4.6);
        let r = check_chain(&snap(v), false, &mut zero, &CheckOpts::default()).unwrap();
        assert_eq!(r.rows.len(), 2);
        assert_eq!((r.rows[0].root.as_str(), r.rows[1].root.as_str()), ("SPX", "SPXW"));
        // SPX has the break (120 call bid 4.5 > ask 4.1 of the 100 call, and > ask 1.1 of the 110 call); SPXW has none.
        assert!(r.rows[0].breaks[0] >= 1);
        assert_eq!(r.rows[1].breaks.iter().sum::<usize>(), 0);
        assert_eq!(r.rows[0].quotes_used, 4);
        assert_eq!(r.rows[1].quotes_used, 3);
    }

    #[test]
    fn implied_dividend_from_the_parity_forward() {
        // Rate 5 percent, 1 year (365 days), forward 98: q = r - ln(F/S) / t.
        let mut v = Vec::new();
        let df = (-0.05f64).exp();
        for k in [95.0, 100.0, 105.0] {
            let c: f64 = 8.0;
            let p = c - df * (98.0 - k);
            add(&mut v, "XYZ", "270102", "2027-01-02", true, k, c, c);
            add(&mut v, "XYZ", "270102", "2027-01-02", false, k, p, p);
        }
        let mut rate = |_: &str, _: f64| -> Result<f64, String> { Ok(0.05) };
        let r = check_chain(&snap(v), true, &mut rate, &CheckOpts::default()).unwrap();
        let q = r.rows[0].implied_div.unwrap();
        assert!((q - (0.05 - (0.98f64).ln())).abs() < 1e-9, "{q}");
        assert!((r.rows[0].forward.unwrap() - 98.0).abs() < 1e-9);
    }

    fn pos(v: &[OptionQuote], strike: f64, call: bool, expiry: &str) -> usize {
        v.iter().position(|x| x.strike == strike && (x.option_type == "call") == call && x.expiry == expiry).unwrap()
    }

    #[test]
    fn a_missing_ask_is_dropped_and_counted() {
        let mut v = clean();
        let i = pos(&v, 100.0, true, "2026-01-16");
        v[i].ask = None;
        let r = run(v);
        assert_eq!((r.dropped.no_ask, r.dropped.no_bid), (1, 0));
        assert_eq!(r.breaks.len(), 0);
    }

    #[test]
    fn a_series_with_one_strike_has_no_strike_test() {
        let mut v = clean();
        add(&mut v, "XYZ", "260320", "2026-03-20", true, 100.0, 5.9, 6.0);
        let r = run(v);
        let row = r.rows.iter().find(|x| x.expiry == "2026-03-20").unwrap();
        assert_eq!((row.tests[0], row.tests[1], row.tests[2], row.tests[4]), (0, 0, 0, 0));
        assert_eq!(row.tests[3], 2, "the call 100 of the two earlier expiries");
        assert_eq!(r.breaks.len(), 0);
    }

    #[test]
    fn a_chain_with_only_0_day_quotes_has_no_series() {
        let mut v = Vec::new();
        add(&mut v, "XYZ", "260102", "2026-01-02", true, 100.0, 1.0, 1.2);
        add(&mut v, "XYZ", "260102", "2026-01-02", false, 100.0, 1.0, 1.2);
        let r = run(v);
        assert_eq!((r.rows.len(), r.dropped.past_expiry, r.quotes_used), (0, 2, 0));
    }

    #[test]
    fn a_calendar_pair_across_two_roots_is_not_a_pair() {
        let mut v = Vec::new();
        add(&mut v, "AAA", "260116", "2026-01-16", false, 100.0, 5.5, 5.6);
        add(&mut v, "BBB", "260220", "2026-02-20", false, 100.0, 5.0, 5.1);
        let r = run(v);
        assert_eq!(r.rows.len(), 2);
        assert!(r.rows.iter().all(|x| x.tests[3] == 0));
        assert_eq!(r.breaks.len(), 0);
        // The same two quotes in one root are a calendar break of 5.5 - 5.1 = 0.4.
        let mut w = Vec::new();
        add(&mut w, "AAA", "260116", "2026-01-16", false, 100.0, 5.5, 5.6);
        add(&mut w, "AAA", "260220", "2026-02-20", false, 100.0, 5.0, 5.1);
        let r2 = run(w);
        assert_eq!(count(&r2, 3), 1);
        assert!((r2.breaks[0].size - 0.4).abs() < 1e-9);
    }

    #[test]
    fn a_duplicate_strike_is_not_a_pair() {
        let mut v = Vec::new();
        add(&mut v, "XYZ", "260116", "2026-01-16", true, 100.0, 3.9, 4.1);
        v.push(q("XYZ260116C00100000", true, "2026-01-16", 100.0, 4.5, 4.6)); // the same strike again
        let r = run(v);
        assert_eq!(r.rows[0].tests[0] + r.rows[0].tests[1] + r.rows[0].tests[2], 0);
        assert_eq!(r.breaks.len(), 0);
    }

    #[test]
    fn size_bands_and_the_size_floor() {
        let mut v = clean();
        let i = pos(&v, 100.0, true, "2026-01-16");
        v[i].bid = Some(7.0);
        v[i].ask = Some(7.2);
        let r = run(v.clone());
        let fly: Vec<_> = r.breaks.iter().filter(|b| b.check == 2).collect();
        assert!((fly[0].size - 0.9).abs() < 1e-9);
        let b = r.bands(false);
        assert!(b[1] >= 1, "0.9 is in the band 0.10 to 1: {b:?}");
        assert_eq!(band_of(0.0999), 0);
        assert_eq!((band_of(0.10), band_of(0.99), band_of(1.0), band_of(9.99), band_of(10.0)), (1, 1, 2, 2, 3));
        // A floor of 1 USD removes the breaks of 0.9; the parity break of 1.4 and the calendar break of 1.9 stay.
        let r2 = check_chain(&snap(v), true, &mut zero, &CheckOpts { min_size: 1.0, iv_mark: 0.10, otm: false }).unwrap();
        assert_eq!(count(&r2, 2), 0);
        assert!(r2.breaks.iter().all(|b| b.size >= 1.0));
        assert_eq!(r2.breaks.len(), r.breaks.iter().filter(|b| b.size >= 1.0).count());
    }

    #[test]
    fn a_bid_above_the_bound_is_one_break_and_leaves_the_other_checks() {
        let mut v = clean();
        let i = pos(&v, 90.0, true, "2026-01-16");
        v[i].bid = Some(105.0);
        v[i].ask = Some(106.0);
        let j = pos(&v, 90.0, false, "2026-02-20");
        v[j].bid = Some(95.0);
        v[j].ask = Some(96.0);
        let r = run(v);
        let b: Vec<_> = r.breaks.iter().filter(|b| b.check == 6).collect();
        assert_eq!(b.len(), 2, "{:?}", r.breaks);
        let mut sizes: Vec<f64> = b.iter().map(|x| x.size).collect();
        sizes.sort_by(f64::total_cmp);
        assert!((sizes[0] - 5.0).abs() < 1e-9 && (sizes[1] - 5.0).abs() < 1e-9, "{sizes:?}"); // 105 - S 100, 95 - K 90
        assert_eq!((r.above_bound, r.breaks.len()), (2, 2));
    }

    #[test]
    fn a_quote_with_bid_zero_gives_a_buy_leg_only() {
        let mut v = Vec::new();
        // Call 100 has no bid and ask 4.1. Call 110 has bid 4.5: bid 4.5 is above the ask 4.1 of the call 100 (0.4).
        add(&mut v, "XYZ", "260116", "2026-01-16", true, 100.0, 0.0, 4.1);
        add(&mut v, "XYZ", "260116", "2026-01-16", true, 110.0, 4.5, 4.6);
        let r = run(v);
        assert_eq!((r.buy_only, r.quotes_used, r.dropped.no_bid), (1, 2, 0));
        assert_eq!(r.breaks.len(), 1, "{:?}", r.breaks);
        assert!((r.breaks[0].size - 0.4).abs() < 1e-9);
        assert!(r.breaks[0].symbols.iter().any(|s| s.contains("C00100000")), "the ask of the quote with no bid is the buy leg");
        // The same quote as a sell leg: call 90 with no bid and ask 20 against the call 100 ask 4.1 gives no break.
        let mut w = Vec::new();
        add(&mut w, "XYZ", "260116", "2026-01-16", true, 90.0, 0.0, 20.0);
        add(&mut w, "XYZ", "260116", "2026-01-16", true, 100.0, 3.9, 4.1);
        let r2 = run(w);
        assert_eq!(r2.breaks.len(), 0);
    }

    #[test]
    fn a_quote_far_from_its_neighbours_in_volatility_is_marked() {
        use crate::market::option_iv::black76;
        let t = 90.0 / 365.0;
        let mut v = Vec::new();
        for k in [85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0] {
            let p = black76(true, 100.0, k, t, 1.0, 0.2) + if k == 100.0 { 3.0 } else { 0.0 };
            add(&mut v, "XYZ", "260402", "2026-04-02", true, k, p - 0.02, p + 0.02);
        }
        let r = run(v.clone());
        assert_eq!(r.marked.iter().cloned().collect::<Vec<_>>(), vec!["XYZ260402C00100000".to_string()]);
        assert_eq!(r.rows[0].marked, 1);
        let fly: Vec<_> = r.breaks.iter().filter(|b| b.check == 2).collect();
        assert!(!fly.is_empty() && fly.iter().all(|b| b.marked), "{:?}", r.breaks);
        assert_eq!(r.rows[0].clean, r.breaks.iter().filter(|b| !b.marked).count());
        // Marking off: no quote is marked and no break is flagged.
        let r0 = check_chain(&snap(v), true, &mut zero, &CheckOpts { min_size: 0.0, iv_mark: 0.0, otm: false }).unwrap();
        assert!(r0.marked.is_empty() && r0.breaks.iter().all(|b| !b.marked));
        assert_eq!(r0.breaks.len(), r.breaks.len(), "the marker flags; it does not remove");
    }

    #[test]
    fn the_european_indexes_are_a_list() {
        for t in ["SPX", "spx", "XSP", "NDX", "RUT", "VIX"] {
            assert!(is_european(t), "{t}");
        }
        assert!(!is_european("IBM") && !is_european("SPY") && !is_european("QQQ"));
    }

    #[test]
    fn compare_gives_the_breaks_that_stay() {
        let mut d1 = clean();
        let i = pos(&d1, 100.0, true, "2026-01-16");
        d1[i].bid = Some(7.0);
        d1[i].ask = Some(7.2);
        let mut d2 = d1.clone();
        let j = pos(&d1, 100.0, false, "2026-01-16");
        d1[j].bid = Some(5.5); // a calendar break of day 1 that is not on day 2
        d1[j].ask = Some(5.6);
        let (a, b) = (run(d1), run(d2.split_off(0)));
        let c = compare(&a, &b, "2026-01-02", "2026-01-03");
        assert!(c.total >= 3);
        assert_eq!(c.stay.len(), c.total - 1, "one break of day 1 is not on day 2");
        assert_eq!(c.expired, 0);
        assert!(c.stay.iter().any(|(x, s2)| x.check == 2 && (x.size - 0.9).abs() < 1e-9 && (s2 - 0.9).abs() < 1e-9));
        // After the first expiry the breaks of its series cannot stay.
        let c2 = compare(&a, &b, "2026-01-02", "2026-01-16");
        assert!(c2.expired > 0);
        assert!(render_compare("XYZ", &c, 5).contains("stay"));
    }

    #[test]
    fn the_strike_filter_keeps_the_out_of_the_money_quotes_and_those_near_the_forward() {
        // A European index chain: no spot, forward 100 from put-call parity (rate 0), strikes 90 to 110.
        let mut v = Vec::new();
        for k in [90.0f64, 95.0, 100.0, 105.0, 110.0] {
            let (c, p) = ((100.0 - k).max(0.0) + 2.0, (k - 100.0).max(0.0) + 2.0);
            add(&mut v, "SPX", "260116", "2026-01-16", true, k, c - 0.1, c + 0.1);
            add(&mut v, "SPX", "260116", "2026-01-16", false, k, p - 0.1, p + 0.1);
        }
        // The deep in-the-money call 90 has a low ask: bid 6.9 of the call 95 is above it (3.9).
        let i = pos(&v, 90.0, true, "2026-01-16");
        v[i].bid = Some(2.9);
        v[i].ask = Some(3.0);
        let mut s = snap(v);
        s.spot_price = None;
        let all = check_chain(&s, false, &mut zero, &CheckOpts { otm: false, ..CheckOpts::default() }).unwrap();
        assert_eq!(count(&all, 0), 1, "{:?}", all.breaks);
        assert!((all.breaks.iter().find(|b| b.check == 0).unwrap().size - 3.9).abs() < 1e-9, "{:?}", all.breaks);
        assert_eq!(all.breaks.len(), 2, "monotony 3.9 and butterfly 4.35");
        let f = check_chain(&s, false, &mut zero, &CheckOpts { otm: true, ..CheckOpts::default() }).unwrap();
        // Removed: the calls 90 and 95 (K < 99) and the puts 105 and 110 (K > 101).
        assert_eq!((f.otm_removed, f.otm_no_forward, f.quotes_used), (4, 0, 10));
        assert_eq!(f.rows[0].quotes_used, 6);
        assert_eq!(f.breaks.len(), 0);
    }

    #[test]
    fn the_header_names_the_source_of_the_file() {
        let r = run(clean());
        let mut s = snap(clean());
        assert!(render("XYZ", &s, &r, true, 3).contains("a file with no source field is a Cboe delayed snapshot"));
        s.source = Some("alpaca".into());
        s.feed = Some("indicative".into());
        let text = render("XYZ", &s, &r, true, 3);
        assert!(text.contains("Alpaca indicative feed: modified quotes, not executable prices (#280)"));
        assert!(!text.contains("Cboe delayed snapshot"));
    }
}
