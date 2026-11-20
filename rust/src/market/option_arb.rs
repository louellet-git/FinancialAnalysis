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
//!
//! A quote is used when bid > 0 and ask >= bid. The other quotes are counted by reason in `Report::dropped`.

use super::option_history::occ_root;
use super::option_iv::forward_from_parity;
use super::options::OptionChainSnapshot;
use crate::trading::calendar::parse_iso;
use std::collections::BTreeMap;

pub const CHECK_NAMES: [&str; 6] = ["monotony", "slope", "butterfly", "calendar", "parity", "intrinsic"];
const EPS: f64 = 1e-9;

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
    bid: f64,
    ask: f64,
}

#[derive(Debug, Clone)]
pub struct Break {
    /// 0 to 5: index in `CHECK_NAMES` (5 = intrinsic).
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
}

#[derive(Debug, Clone, Default)]
pub struct SeriesRow {
    pub root: String,
    pub expiry: String,
    pub days: i64,
    pub quotes_used: usize,
    pub tests: [usize; 6],
    pub breaks: [usize; 6],
    /// The number of distinct quotes in at least one break of the series.
    pub distinct: usize,
    pub largest: f64,
    /// The parity forward from the mids of the strikes nearest the spot.
    pub forward: Option<f64>,
    pub rate: Option<f64>,
    /// q = r - ln(F / S) / t. An approximation for American options.
    pub implied_div: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct Dropped {
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
    /// Quotes with an ask below the intrinsic value (American options): not used in the checks 1 to 5.
    pub below_intrinsic: usize,
}

type StrikeMap<'a> = BTreeMap<(String, bool, u64), Vec<(&'a String, &'a Used)>>;

fn push_break(out: &mut Vec<Break>, check: usize, root: &str, expiry: &str, size: f64, legs: &[(&str, f64)], text: String) {
    let top = legs.iter().map(|l| l.1).fold(0.0, f64::max);
    out.push(Break { check, root: root.into(), expiry: expiry.into(), size, ticks: size / tick_of(top), text, symbols: legs.iter().map(|l| l.0.to_string()).collect() });
}

fn kind(call: bool) -> &'static str {
    if call {
        "call"
    } else {
        "put"
    }
}

/// Run the checks. `american`: stocks and ETFs (false for an index such as SPX: no calendar and no parity bound).
/// `rate(date, t)` gives the rate for `t` years (a flat rate or a curve); the date is the snapshot date.
pub fn check_chain(snap: &OptionChainSnapshot, american: bool, rate: &mut dyn FnMut(&str, f64) -> Result<f64, String>) -> Result<Report, String> {
    let day = parse_iso(&snap.snapshot_date).ok_or_else(|| format!("bad snapshot date {}", snap.snapshot_date))?;
    let mut rep = Report { quotes_total: snap.quotes.len(), ..Default::default() };
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
        if bid.is_nan() || bid <= 0.0 {
            rep.dropped.no_bid += 1;
            continue;
        }
        if ask.is_nan() || ask < bid {
            rep.dropped.ask_below_bid += 1;
            continue;
        }
        rep.quotes_used += 1;
        series.entry((root, q.expiry.clone())).or_default().push(Used { symbol: q.contract_symbol.clone(), call: q.option_type == "call", strike: q.strike, bid, ask });
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
        // 0 intrinsic (American options): an ask below the intrinsic value. Buy at the ask and exercise at once.
        // S is a last price of the file, not a bid or an ask, and the quotes are not at one time. The quote leaves the checks 1 to 5.
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
                    if size > EPS {
                        push_break(&mut breaks, 0, root, expiry, size, &[(&cheap.symbol, cheap.bid), (&dear.symbol, dear.ask)], format!("{tag}: bid {} of {} is above ask {} of {}", cheap.bid, cheap.symbol, dear.ask, dear.symbol));
                    }
                    // 2 slope: the price difference is not more than the strike difference.
                    row.tests[1] += 1;
                    let width = b.strike - a.strike;
                    let size = if call { a.bid - b.ask } else { b.bid - a.ask } - width;
                    if size > EPS {
                        let (s, l) = if call { (a, b) } else { (b, a) };
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
                if size > EPS {
                    push_break(&mut breaks, 2, root, expiry, size, &[(&m.symbol, m.bid), (&a.symbol, a.ask), (&b.symbol, b.ask)], format!("{tag}: bid {} of {} is above {:.4} = {:.4} x ask {} of {} + {:.4} x ask {} of {}", m.bid, m.symbol, bound, w, a.ask, a.symbol, 1.0 - w, b.ask, b.symbol));
                }
            }
        }
        // 5 parity upper bound, and 6 the implied dividend
        let mut pairs_mid = Vec::new();
        for c in quotes.iter().filter(|q| q.call) {
            let Some(p) = quotes.iter().find(|q| !q.call && q.strike == c.strike) else { continue };
            pairs_mid.push((c.strike, 0.5 * (c.bid + c.ask), 0.5 * (p.bid + p.ask)));
            if let (true, Some(s)) = (american, spot) {
                row.tests[4] += 1;
                let bound = s - c.strike * df;
                let size = c.bid - p.ask - bound;
                if size > EPS {
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
        good.insert((root.clone(), expiry.clone()), quotes);
        rows.insert((root.clone(), expiry.clone()), row);
    }

    // 4 calendar, American only
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
                    if size > EPS {
                        push_break(&mut breaks, 3, &root, e2, size, &[(&q1.symbol, q1.bid), (&q2.symbol, q2.ask)], format!("{}: bid {} of {} ({e1}) is above ask {} of {} ({e2})", kind(call), q1.bid, q1.symbol, q2.ask, q2.symbol));
                    }
                }
            }
        }
    }

    breaks.sort_by(|a, b| b.size.total_cmp(&a.size));
    let mut seen: BTreeMap<(String, String), std::collections::BTreeSet<&str>> = BTreeMap::new();
    for b in &breaks {
        let key = (b.root.clone(), b.expiry.clone());
        if let Some(row) = rows.get_mut(&key) {
            row.breaks[b.check] += 1;
            row.largest = row.largest.max(b.size);
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

/// The text for a person: the table of the series, the ten largest breaks, the limits.
pub fn render(ticker: &str, snap: &OptionChainSnapshot, rep: &Report, american: bool, top: usize) -> String {
    let mut s = String::new();
    let d = &rep.dropped;
    s.push_str(&format!(
        "options check {ticker} {}: spot {} ({}), {} quotes, {} used, {} dropped (bid missing or 0: {}, ask missing: {}, ask below bid: {}, bad symbol: {}, bad expiry: {}, expiry on or before the snapshot date: {}, option type not call or put: {}); {} used quotes have an ask below the intrinsic value and are removed from the checks 1 to 5\n",
        snap.snapshot_date,
        snap.spot_price.map_or("none".to_string(), |x| format!("{x:.2}")),
        if american { "American options" } else { "European index: no calendar test, no parity bound" },
        rep.quotes_total,
        rep.quotes_used,
        d.total(),
        d.no_bid,
        d.no_ask,
        d.ask_below_bid,
        d.bad_symbol,
        d.bad_expiry,
        d.past_expiry,
        d.bad_type,
        rep.below_intrinsic
    ));
    s.push_str(&format!("source {:?}, quote_kind {:?}: a file with no source field is a Cboe delayed snapshot; the file has no time of the quotes.\n\n", snap.source, snap.quote_kind));
    s.push_str("Each cell is breaks/tests. int = ask below intrinsic value (American options only; S is a last price, not a bid or ask; the quotes are not at one time), distinct = quotes in at least one break. mono = monotony, slope, bfly = butterfly, cal = calendar (counted on the later expiry), par = parity bound.\n");
    s.push_str(&format!("{:<6} {:<10} {:>4} {:>6} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>8} {:>9} {:>8} {:>7}\n", "root", "expiry", "days", "quotes", "int", "mono", "slope", "bfly", "cal", "par", "distinct", "max $", "rate %", "q %"));
    for r in &rep.rows {
        let cell = |i: usize| format!("{}/{}", r.breaks[i], r.tests[i]);
        s.push_str(&format!(
            "{:<6} {:<10} {:>4} {:>6} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>8} {:>9.2} {:>8} {:>7}\n",
            r.root,
            r.expiry,
            r.days,
            r.quotes_used,
            cell(5),
            cell(0),
            cell(1),
            cell(2),
            cell(3),
            cell(4),
            r.distinct,
            r.largest,
            r.rate.map_or("-".into(), |x| format!("{:.2}", x * 100.0)),
            r.implied_div.map_or("-".into(), |x| format!("{:.2}", x * 100.0)),
        ));
    }
    let intrinsic = rep.breaks.iter().filter(|b| b.check == 5).count();
    let rest: Vec<&Break> = rep.breaks.iter().filter(|b| b.check != 5).collect();
    s.push_str(&format!("\nbreaks: {} (intrinsic: {intrinsic}; checks 1 to 5 after the removal: {}); the {} largest of the checks 1 to 5:\n", rep.breaks.len(), rest.len(), top.min(rest.len())));
    for b in rest.iter().take(top) {
        s.push_str(&format!("  {:>9.2} $ {:>8.1} ticks  {} {} {}: {}\n", b.size, b.ticks, b.root, b.expiry, CHECK_NAMES[b.check], b.text));
    }
    s.push_str(&format!("the {} largest intrinsic breaks:\n", top.min(intrinsic)));
    for b in rep.breaks.iter().filter(|b| b.check == 5).take(top) {
        s.push_str(&format!("  {:>9.2} $ {:>8.1} ticks  {} {} {}: {}\n", b.size, b.ticks, b.root, b.expiry, CHECK_NAMES[b.check], b.text));
    }
    s.push_str("\nNotes:\n");
    s.push_str("  Prices are executable: buy at the ask, sell at the bid. A break is not a profit: no costs, no borrow, no time of the quotes, early exercise not priced.\n");
    s.push_str("  Ticks: size / tick, tick 0.01 below $3, else 0.05, by the highest price among the legs.\n");
    s.push_str("  Monotony and slope use all strike pairs, so one bad quote makes many breaks. Butterfly uses consecutive strikes.\n");
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
        OptionChainSnapshot { ticker: "XYZ".into(), snapshot_date: "2026-01-02".into(), spot_price: Some(100.0), quotes, source: None, feed: None, quote_kind: None, note: None }
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
        check_chain(&snap(v), true, &mut zero).unwrap()
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
        let r2 = check_chain(&snap(v2), false, &mut zero).unwrap();
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
        let r2 = check_chain(&snap(v), false, &mut zero).unwrap();
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
        let r = check_chain(&snap(v), false, &mut zero).unwrap();
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
        let r = check_chain(&snap(v), true, &mut rate).unwrap();
        let q = r.rows[0].implied_div.unwrap();
        assert!((q - (0.05 - (0.98f64).ln())).abs() < 1e-9, "{q}");
        assert!((r.rows[0].forward.unwrap() - 98.0).abs() < 1e-9);
    }
}
