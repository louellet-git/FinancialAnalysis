//! `fa portfolio backtest`: one portfolio backtester. A strategy gives target weights on each rebalance date; the engine
//! holds the positions, marks them to the close each day, charges a cost in basis points on each traded value, keeps
//! the cash, and compares the result with SPY. No order goes to a broker.
//!
//! Point in time: a strategy gets a [`View`] of the rebalance date `D`. The view has the prices of the days BEFORE `D`
//! only; it refuses a read of `D` or a later day. The engine trades at the close of `D`. A strategy with other data
//! (the SEC facts) must keep to the filing dates itself (`strategies::ValueRank` does).
//!
//! Limits: the price files are adjusted for splits, not for dividends (price returns, for the portfolio and for SPY);
//! the ticker lists are today's lists (survivorship, the result is too good); a ticker that stops trading keeps its last
//! price in the portfolio (no failure loss); no tax, no short sale, no leverage, no market impact.

pub mod calibrate;
pub mod cli;
pub mod strategies;

use std::collections::BTreeMap;

/// A price series: `(day, close)` in date order. The day is the day number of `mapcheck::dates`.
pub type Series = Vec<(i64, f64)>;

/// A close older than this many days is not a price to trade at.
pub const STALE_DAYS: i64 = 10;

/// The closes of all tickers.
#[derive(Debug, Default, Clone)]
pub struct Prices(pub BTreeMap<String, Series>);

impl Prices {
    pub fn from_raw(raw: &BTreeMap<String, Vec<(String, f64)>>) -> Prices {
        Prices(raw.iter().map(|(t, s)| (t.clone(), crate::dcf::auto_returns::days_of(s))).collect())
    }

    /// The last close on or before `day`.
    pub fn asof(&self, ticker: &str, day: i64) -> Option<(i64, f64)> {
        let s = self.0.get(ticker)?;
        let i = s.partition_point(|(d, _)| *d <= day);
        i.checked_sub(1).map(|k| s[k])
    }
}

/// What a strategy sees on the rebalance date: the prices before that date.
pub struct View<'a> {
    date: i64,
    prices: &'a Prices,
}

impl<'a> View<'a> {
    pub fn new(date: i64, prices: &'a Prices) -> View<'a> {
        View { date, prices }
    }

    /// The rebalance date. The engine trades at the close of this day.
    pub fn date(&self) -> i64 {
        self.date
    }

    /// The closes of a ticker on the days before the rebalance date.
    pub fn closes(&self, ticker: &str) -> &'a [(i64, f64)] {
        match self.prices.0.get(ticker) {
            Some(s) => &s[..s.partition_point(|(d, _)| *d < self.date)],
            None => &[],
        }
    }

    /// The last close before the rebalance date.
    pub fn last_close(&self, ticker: &str) -> Option<(i64, f64)> {
        self.closes(ticker).last().copied()
    }

    /// The last close on or before `day`. An error if `day` is the rebalance date or later: the strategy would read the future.
    pub fn close_asof(&self, ticker: &str, day: i64) -> Result<Option<(i64, f64)>, String> {
        if day >= self.date {
            return Err(format!("look-ahead: the strategy reads {} on the rebalance date {}", mapcheck::dates::format(day), mapcheck::dates::format(self.date)));
        }
        Ok(self.closes(ticker).iter().rev().find(|(d, _)| *d <= day).copied())
    }
}

/// A strategy: target weights (ticker, weight) for a rebalance date. A weight is 0 or more; the sum is 1 or less
/// (the rest is cash).
pub trait Strategy: Sync {
    fn name(&self) -> &str;
    fn weights(&self, view: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String>;
    /// The number of scored tickers at each rebalance so far (a strategy that scores gives it; the others give none).
    fn scored_counts(&self) -> Vec<usize> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rebalance {
    Monthly,
    Quarterly,
    /// The Friday of each week, or the last trading day of the week if Friday is not one.
    Weekly,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub start: i64,
    pub end: i64,
    pub rebalance: Rebalance,
    pub cost_bp: f64,
}

/// The week of a day number (weeks start on Monday; day 0 is Thursday 1970-01-01).
fn week_of(day: i64) -> i64 {
    (day + 3).div_euclid(7)
}

/// The first day of each new month (or quarter) of the calendar, from `start` to `end` (the first day counts). For
/// `Weekly`: the last trading day of each week, which is the Friday if it is a trading day; a day counts if it is a Friday
/// or the next trading day of the calendar is in a later week (the last day of the data, if it is not a Friday, does not count).
pub fn rebalance_days(cal: &[i64], start: i64, end: i64, r: Rebalance) -> Vec<i64> {
    let mut out = Vec::new();
    if r == Rebalance::Weekly {
        let days: Vec<i64> = cal.iter().copied().filter(|d| *d >= start && *d <= end).collect();
        for (i, &d) in days.iter().enumerate() {
            let friday = (d + 3).rem_euclid(7) == 4;
            if friday || days.get(i + 1).is_some_and(|n| week_of(*n) != week_of(d)) {
                out.push(d);
            }
        }
        return out;
    }
    let mut last = String::new();
    for &d in cal.iter().filter(|d| **d >= start && **d <= end) {
        let t = mapcheck::dates::format(d);
        let (y, m) = (&t[..4], t[5..7].parse::<u32>().unwrap_or(1));
        let key = match r {
            Rebalance::Monthly => format!("{y}-{m:02}"),
            Rebalance::Quarterly => format!("{y}-Q{}", (m - 1) / 3),
            Rebalance::Weekly => unreachable!("handled above"),
        };
        if key != last {
            out.push(d);
            last = key;
        }
    }
    out
}

/// The figures of a net asset value series (the value before the first day is 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Perf {
    pub total: f64,
    pub annual: f64,
    pub vol: f64,
    pub max_drawdown: f64,
}

pub fn perf(nav: &[f64], days: &[i64]) -> Perf {
    let nan = f64::NAN;
    if nav.is_empty() || nav.len() != days.len() {
        return Perf { total: nan, annual: nan, vol: nan, max_drawdown: nan };
    }
    let total = nav[nav.len() - 1] - 1.0;
    let span = (days[days.len() - 1] - days[0]) as f64;
    let annual = if span > 0.0 && nav[nav.len() - 1] > 0.0 { nav[nav.len() - 1].powf(365.25 / span) - 1.0 } else { nan };
    // The daily returns start at index 1: the move from the start value 1 to nav[0] is the entry cost, not a return.
    let rets: Vec<f64> = nav.windows(2).map(|w| w[1] / w[0] - 1.0).collect();
    let vol = if rets.len() > 1 {
        let m = rets.iter().sum::<f64>() / rets.len() as f64;
        (rets.iter().map(|r| (r - m).powi(2)).sum::<f64>() / (rets.len() as f64 - 1.0)).sqrt() * 252f64.sqrt()
    } else {
        nan
    };
    let (mut peak, mut dd) = (1.0f64, 0.0f64);
    for v in nav {
        peak = peak.max(*v);
        dd = dd.min(v / peak - 1.0);
    }
    Perf { total, annual, vol, max_drawdown: dd }
}

/// One line of text for a name and its figures.
pub fn perf_line(name: &str, p: &Perf) -> String {
    format!("{name:<12} total {:+.1}%, annual {:+.2}%, volatility {:.1}%, max drawdown {:.1}%", 100.0 * p.total, 100.0 * p.annual, 100.0 * p.vol, 100.0 * p.max_drawdown)
}

#[derive(Debug, Clone)]
pub struct Report {
    pub strategy: String,
    pub dates: Vec<i64>,
    /// The value of the portfolio at each close, after the trades of the day (start value 1).
    pub nav: Vec<f64>,
    /// SPY bought at the close of the first rebalance date, without cost.
    pub bench: Vec<f64>,
    pub rebalances: usize,
    /// The sum over the rebalances of (traded value / value of the portfolio) / 2; the start counts.
    pub turnover_total: f64,
    /// The costs, in units of the start value.
    pub cost_total: f64,
    /// Target weights dropped because the ticker had no fresh close.
    pub skipped: usize,
    /// Positions sold at a close older than `STALE_DAYS` (the ticker stopped trading; no failure loss in the price).
    pub stale_sales: usize,
    /// Days of the series with no position (all cash), after the first position.
    pub cash_days: usize,
    /// The number of scored tickers at each rebalance (empty if the strategy gives none).
    pub scored: Vec<usize>,
    /// The target weights of each rebalance date that the series covers (after the targets with no fresh close are dropped).
    pub holdings: Vec<(i64, Vec<(String, f64)>)>,
    pub fund: Perf,
    pub bench_perf: Perf,
}

impl Report {
    pub fn years(&self) -> f64 {
        match (self.dates.first(), self.dates.last()) {
            (Some(a), Some(b)) => (b - a) as f64 / 365.25,
            _ => f64::NAN,
        }
    }
}

fn check_weights(w: &[(String, f64)], prices: &Prices) -> Result<(), String> {
    let mut sum = 0.0;
    let mut seen = std::collections::BTreeSet::new();
    for (t, x) in w {
        if !seen.insert(t.as_str()) {
            return Err(format!("weight of {t} twice"));
        }
        if !x.is_finite() || *x < 0.0 {
            return Err(format!("weight of {t} is {x}: a weight is 0 or more"));
        }
        if !prices.0.contains_key(t) {
            return Err(format!("weight for {t}: no price series"));
        }
        sum += x;
    }
    if sum > 1.0 + 1e-9 {
        return Err(format!("the weights sum to {sum}: 1 or less"));
    }
    Ok(())
}

/// Run a strategy. The calendar is the SPY trading days; `prices` must hold a "SPY" series.
pub fn run(prices: &Prices, universe: &[String], strat: &dyn Strategy, cfg: &Config) -> Result<Report, String> {
    let spy = prices.0.get("SPY").ok_or("no SPY price series (the calendar and the benchmark)")?;
    let cal: Vec<i64> = spy.iter().map(|x| x.0).filter(|d| *d >= cfg.start && *d <= cfg.end).collect();
    let rdays = rebalance_days(&cal, cfg.start, cfg.end, cfg.rebalance);
    let first = *rdays.first().ok_or("no trading day between the start and the end")?;
    let rate = cfg.cost_bp / 1e4;
    let mut shares: BTreeMap<String, f64> = BTreeMap::new();
    let mut cash = 1.0f64;
    let (mut ri, mut turnover_total, mut cost_total, mut skipped, mut stale_sales) = (0usize, 0.0f64, 0.0f64, 0usize, 0usize);
    let (mut dates, mut nav, mut bench) = (Vec::new(), Vec::new(), Vec::new());
    let mut cash_days = 0usize;
    let mut holdings: Vec<(i64, Vec<(String, f64)>)> = Vec::new();
    // The series starts on the first rebalance date with a position: a strategy with no data yet holds cash, and that
    // time is not in the result (it would be flat, and SPY would run alone).
    let mut spy_first = f64::NAN;
    let marked = |shares: &BTreeMap<String, f64>, cash: f64, d: i64| -> f64 { cash + shares.iter().map(|(t, s)| s * prices.asof(t, d).map_or(0.0, |x| x.1)).sum::<f64>() };
    for &d in cal.iter().filter(|d| **d >= first) {
        if rdays.get(ri) == Some(&d) {
            ri += 1;
            let w = strat.weights(&View::new(d, prices), universe)?;
            check_weights(&w, prices)?;
            let v = marked(&shares, cash, d);
            let mut target: BTreeMap<String, (f64, f64)> = BTreeMap::new();
            for (t, x) in w.iter().filter(|(_, x)| *x > 0.0) {
                match prices.asof(t, d) {
                    Some((pd, p)) if d - pd <= STALE_DAYS => {
                        target.insert(t.clone(), (*x, p));
                    }
                    _ => skipped += 1,
                }
            }
            let cur: BTreeMap<String, f64> = shares.iter().map(|(t, s)| (t.clone(), s * prices.asof(t, d).map_or(0.0, |x| x.1))).collect();
            stale_sales += cur.iter().filter(|(t, v)| **v > 0.0 && !target.contains_key(*t) && prices.asof(t, d).is_some_and(|(pd, _)| d - pd > STALE_DAYS)).count();
            let names: Vec<&String> = cur.keys().chain(target.keys().filter(|t| !cur.contains_key(*t))).collect();
            // The cost comes out of the portfolio: the targets use the value less the cost of the first estimate.
            let cost0 = rate * names.iter().map(|t| (target.get(*t).map_or(0.0, |x| x.0 * v) - cur.get(*t).copied().unwrap_or(0.0)).abs()).sum::<f64>();
            let scale = (v - cost0).max(0.0);
            let mut traded = 0.0;
            let mut new_shares = BTreeMap::new();
            let mut target_sum = 0.0;
            for t in names {
                let tv = target.get(t).map_or(0.0, |x| x.0 * scale);
                traded += (tv - cur.get(t).copied().unwrap_or(0.0)).abs();
                if tv > 0.0 {
                    new_shares.insert(t.clone(), tv / target[t].1);
                    target_sum += tv;
                }
            }
            let cost = rate * traded;
            cash = v - target_sum - cost;
            shares = new_shares;
            cost_total += cost;
            if v > 0.0 {
                turnover_total += traded / v / 2.0;
            }
            if spy_first.is_nan() && !shares.is_empty() {
                spy_first = prices.asof("SPY", d).map_or(f64::NAN, |x| x.1);
            }
            if !spy_first.is_nan() {
                holdings.push((d, target.iter().map(|(t, x)| (t.clone(), x.0)).collect()));
            }
        }
        if spy_first.is_nan() {
            cash = 1.0;
            cost_total = 0.0;
            turnover_total = 0.0;
            continue;
        }
        dates.push(d);
        cash_days += usize::from(shares.is_empty());
        nav.push(marked(&shares, cash, d));
        bench.push(prices.asof("SPY", d).map_or(f64::NAN, |x| x.1) / spy_first);
    }
    if dates.is_empty() {
        return Err("the strategy held no position on any rebalance date".into());
    }
    let (fund, bench_perf) = (perf(&nav, &dates), perf(&bench, &dates));
    Ok(Report { strategy: strat.name().to_string(), dates, nav, bench, rebalances: rdays.len(), turnover_total, cost_total, skipped, stale_sales, cash_days, scored: strat.scored_counts(), holdings, fund, bench_perf })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D0: &str = "2020-01-27";

    /// Consecutive calendar days from 2020-01-27 (a Monday), `n` days; the close is `f(i)`.
    fn series(n: i64, f: impl Fn(i64) -> f64) -> Series {
        let d0 = mapcheck::dates::parse(D0).unwrap();
        (0..n).map(|i| (d0 + i, f(i))).collect()
    }

    fn prices(items: Vec<(&str, Series)>) -> Prices {
        Prices(items.into_iter().map(|(t, s)| (t.to_string(), s)).collect())
    }

    /// Fixed weights by rebalance index.
    struct Fixed(Vec<Vec<(&'static str, f64)>>, std::sync::atomic::AtomicUsize);
    impl Fixed {
        fn new(w: Vec<Vec<(&'static str, f64)>>) -> Fixed {
            Fixed(w, std::sync::atomic::AtomicUsize::new(0))
        }
    }
    impl Strategy for Fixed {
        fn name(&self) -> &str {
            "fixed"
        }
        fn weights(&self, _: &View, _: &[String]) -> Result<Vec<(String, f64)>, String> {
            let i = self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.0[i.min(self.0.len() - 1)].iter().map(|(t, w)| (t.to_string(), *w)).collect())
        }
    }

    fn cfg(cost_bp: f64, end_day: i64) -> Config {
        let d0 = mapcheck::dates::parse(D0).unwrap();
        Config { start: d0, end: d0 + end_day, rebalance: Rebalance::Monthly, cost_bp }
    }

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn rebalance_days_are_the_first_day_of_each_month_and_quarter() {
        let cal: Vec<i64> = series(100, |_| 1.0).iter().map(|x| x.0).collect(); // 2020-01-27 to 2020-05-05
        let m: Vec<String> = rebalance_days(&cal, cal[0], cal[99], Rebalance::Monthly).iter().map(|d| mapcheck::dates::format(*d)).collect();
        assert_eq!(m, ["2020-01-27", "2020-02-01", "2020-03-01", "2020-04-01", "2020-05-01"]);
        let q: Vec<String> = rebalance_days(&cal, cal[0], cal[99], Rebalance::Quarterly).iter().map(|d| mapcheck::dates::format(*d)).collect();
        assert_eq!(q, ["2020-01-27", "2020-04-01"]);
    }

    #[test]
    fn weekly_days_are_the_friday_or_the_last_trading_day_of_the_week() {
        let d = |s: &str| mapcheck::dates::parse(s).unwrap();
        // Weekdays from Monday 2020-01-27 to Friday 2020-02-14, less the holiday Friday 2020-02-07, plus Wednesday 2020-02-19
        // (a week that the data ends before its Friday).
        let mut cal: Vec<i64> = (d("2020-01-27")..=d("2020-02-14")).filter(|x| (x + 3).rem_euclid(7) < 5).filter(|x| *x != d("2020-02-07")).collect();
        cal.extend([d("2020-02-17"), d("2020-02-18"), d("2020-02-19")]);
        let days: Vec<String> = rebalance_days(&cal, d("2020-01-29"), d("2020-02-19"), Rebalance::Weekly).iter().map(|x| mapcheck::dates::format(*x)).collect();
        // the start is a Wednesday: the first day is the Friday 01-31, not the start day
        assert_eq!(days, ["2020-01-31", "2020-02-06", "2020-02-14"]);
    }

    #[test]
    fn one_ticker_no_cost_return_and_drawdown() {
        // A: 100, 110, 99 on the first three days. All in A on the first day.
        let a = series(3, |i| [100.0, 110.0, 99.0][i as usize]);
        let p = prices(vec![("SPY", series(3, |_| 50.0)), ("A", a)]);
        let r = run(&p, &["A".into()], &Fixed::new(vec![vec![("A", 1.0)]]), &cfg(0.0, 2)).unwrap();
        assert_eq!(r.nav.len(), 3);
        near(r.nav[1], 1.1);
        near(r.fund.total, -0.01);
        near(r.fund.max_drawdown, 99.0 / 110.0 - 1.0);
        near(r.cost_total, 0.0);
        near(r.turnover_total, 0.5); // one purchase of the whole value: 1 / 2
        near(r.bench_perf.total, 0.0);
    }

    #[test]
    fn cost_on_the_entry() {
        // 10 bp on the traded value; the cost comes out of the portfolio: value 1 -> targets 0.999, cost 0.001 * 0.999.
        let p = prices(vec![("SPY", series(3, |_| 50.0)), ("A", series(3, |_| 100.0))]);
        let r = run(&p, &["A".into()], &Fixed::new(vec![vec![("A", 1.0)]]), &cfg(10.0, 2)).unwrap();
        near(r.cost_total, 0.001 * 0.999);
        near(r.nav[0], 1.0 - 0.001 * 0.999);
        near(r.nav[2], 1.0 - 0.001 * 0.999); // the price is flat: no other change
    }

    #[test]
    fn days_with_no_position_are_counted() {
        // 40 days from 2020-01-27; rebalances on day 0, day 5 (Feb 1) and day 34 (Mar 1). The second one is all cash.
        let p = prices(vec![("SPY", series(40, |_| 50.0)), ("A", series(40, |_| 100.0))]);
        let r = run(&p, &["A".into()], &Fixed::new(vec![vec![("A", 1.0)], vec![], vec![("A", 1.0)]]), &cfg(0.0, 39)).unwrap();
        assert_eq!(r.rebalances, 3);
        assert_eq!(r.cash_days, 29); // days 5 to 33
        assert!(r.scored.is_empty());
    }

    #[test]
    fn switch_on_the_rebalance_date_turnover_and_cash() {
        // 40 days. A rises 1 a day from 100; B is flat at 200. Rebalance 1 (day 0): all A. Rebalance 2 (day 5, 2020-02-01):
        // A is 105, value 1.05; half to B, half cash.
        let p = prices(vec![("SPY", series(40, |_| 50.0)), ("A", series(40, |i| 100.0 + i as f64)), ("B", series(40, |_| 200.0))]);
        let r = run(&p, &["A".into(), "B".into()], &Fixed::new(vec![vec![("A", 1.0)], vec![("B", 0.5)]]), &cfg(0.0, 39)).unwrap();
        assert_eq!(r.rebalances, 3); // Jan 27, Feb 1, Mar 1
        near(r.nav[5], 1.05);
        near(r.nav[39], 1.05); // B flat, cash flat
        near(r.fund.max_drawdown, 0.0);
        // turnover: start 1/2; second 1.05 sold and 0.525 bought: 1.575 / 1.05 / 2 = 0.75; third: B 0.5 -> 0.5 and A none: 0
        near(r.turnover_total, 0.5 + 0.75);
        // the holdings are the targets of each rebalance date
        let d = |s: &str| mapcheck::dates::parse(s).unwrap();
        assert_eq!(
            r.holdings,
            vec![(d("2020-01-27"), vec![("A".to_string(), 1.0)]), (d("2020-02-01"), vec![("B".to_string(), 0.5)]), (d("2020-03-01"), vec![("B".to_string(), 0.5)])]
        );
    }

    #[test]
    fn benchmark_is_spy_from_the_first_rebalance_date() {
        let p = prices(vec![("SPY", series(5, |i| 100.0 + 10.0 * i as f64)), ("A", series(5, |_| 10.0))]);
        let r = run(&p, &["A".into()], &Fixed::new(vec![vec![("A", 1.0)]]), &cfg(0.0, 4)).unwrap();
        near(r.bench_perf.total, 0.4);
        near(r.bench[2], 1.2);
    }

    #[test]
    fn the_series_starts_on_the_first_rebalance_date_with_a_position() {
        let p = prices(vec![("SPY", series(40, |i| 100.0 + i as f64)), ("A", series(40, |_| 10.0))]);
        let r = run(&p, &["A".into()], &Fixed::new(vec![vec![], vec![("A", 1.0)]]), &cfg(0.0, 39)).unwrap();
        assert_eq!(mapcheck::dates::format(r.dates[0]), "2020-02-01");
        near(r.nav[0], 1.0);
        near(r.bench[0], 1.0);
        near(r.bench_perf.total, 139.0 / 105.0 - 1.0);
        assert!(run(&p, &["A".into()], &Fixed::new(vec![vec![]]), &cfg(0.0, 39)).is_err());
    }

    #[test]
    fn bad_weights_are_refused() {
        let p = prices(vec![("SPY", series(3, |_| 50.0)), ("A", series(3, |_| 100.0))]);
        let u = ["A".to_string()];
        assert!(run(&p, &u, &Fixed::new(vec![vec![("A", 0.7), ("A", 0.7)]]), &cfg(0.0, 2)).is_err());
        assert!(run(&p, &u, &Fixed::new(vec![vec![("A", -0.1)]]), &cfg(0.0, 2)).is_err());
        assert!(run(&p, &u, &Fixed::new(vec![vec![("Z", 0.1)]]), &cfg(0.0, 2)).is_err());
    }

    #[test]
    fn a_ticker_twice_in_the_weights_is_refused() {
        let p = prices(vec![("SPY", series(3, |_| 50.0)), ("A", series(3, |_| 100.0))]);
        let e = run(&p, &["A".into()], &Fixed::new(vec![vec![("A", 0.3), ("A", 0.3)]]), &cfg(0.0, 2)).unwrap_err();
        assert!(e.contains("twice"), "{e}");
    }

    #[test]
    fn a_stale_close_is_not_bought_and_a_held_ticker_that_stops_trading_is_sold_at_its_last_close() {
        // 40 days. A has closes on days 0 to 9 only (100 -> 109), then stops. The first rebalance (day 0) buys A. The
        // second (day 5, 2020-02-01) keeps A. The third (day 34, 2020-03-01): A is 25 days old, more than STALE_DAYS.
        let a: Series = series(10, |i| 100.0 + i as f64);
        let p = prices(vec![("SPY", series(40, |_| 50.0)), ("A", a)]);
        let r = run(&p, &["A".into()], &Fixed::new(vec![vec![("A", 1.0)], vec![("A", 1.0)], vec![("A", 1.0)]]), &cfg(0.0, 39)).unwrap();
        assert_eq!(r.rebalances, 3);
        assert_eq!(r.skipped, 1); // the third target is dropped: no fresh close
        assert_eq!(r.stale_sales, 1); // and A is sold at its last close (109)
        near(r.nav[39], 1.09); // the value stays at the last close: no loss for a ticker that failed
        // a stale ticker is never bought
        let r = run(&p, &["A".into()], &Fixed::new(vec![vec![], vec![], vec![("A", 1.0)]]), &cfg(0.0, 39));
        assert!(r.is_err()); // no position on any rebalance date
    }

    #[test]
    fn a_ticker_with_a_late_first_price_is_bought_when_it_has_a_close() {
        // B has no close before day 5. The target on day 0 drops B (no price); on day 5 (2020-02-01) B is bought.
        let b: Series = series(40, |_| 20.0).into_iter().filter(|x| x.0 >= mapcheck::dates::parse(D0).unwrap() + 5).collect();
        let p = prices(vec![("SPY", series(40, |_| 50.0)), ("A", series(40, |_| 10.0)), ("B", b)]);
        let w = vec![vec![("A", 0.5), ("B", 0.5)], vec![("A", 0.5), ("B", 0.5)]];
        let r = run(&p, &["A".into(), "B".into()], &Fixed::new(w), &cfg(0.0, 39)).unwrap();
        assert_eq!(r.skipped, 1);
        near(r.nav[0], 1.0); // half in A, half cash on the first day
        near(r.nav[39], 1.0); // flat prices; the second rebalance adds B at 20 with the cash
        // the second rebalance puts the cash into B: turnover 0.5 / 2 on top of the first 0.5 / 2
        near(r.turnover_total, 0.25 + 0.25);
    }

    #[test]
    fn volatility_does_not_count_the_entry_cost_as_a_return() {
        // The first value is 0.99 (the entry cost) and the value is flat after it: the volatility is 0.
        let days: Vec<i64> = (0..10).collect();
        let mut nav = vec![0.99; 10];
        let p = perf(&nav, &days);
        assert!(p.vol.abs() < 1e-12, "{}", p.vol);
        nav[5] = 1.0;
        assert!(perf(&nav, &days).vol > 0.0);
    }

    #[test]
    fn equal_weight_holds_each_ticker_with_a_fresh_close() {
        use strategies::EqualWeight;
        let p = prices(vec![("SPY", series(40, |_| 50.0)), ("A", series(40, |_| 10.0)), ("B", series(40, |i| 20.0 + i as f64)), ("C", series(5, |_| 1.0))]);
        let u: Vec<String> = ["A", "B", "C"].iter().map(|t| t.to_string()).collect();
        let d = mapcheck::dates::parse(D0).unwrap();
        let w = EqualWeight.weights(&View::new(d + 20, &p), &u).unwrap(); // C last traded on day 4: stale
        assert_eq!(w, vec![("A".to_string(), 0.5), ("B".to_string(), 0.5)]);
        let r = run(&p, &u, &EqualWeight, &cfg(0.0, 39)).unwrap();
        assert!(r.fund.total > 0.0);
    }

    #[test]
    fn view_refuses_the_rebalance_day_and_later() {
        let p = prices(vec![("A", series(10, |i| 100.0 + i as f64))]);
        let d = mapcheck::dates::parse(D0).unwrap() + 5;
        let v = View::new(d, &p);
        assert_eq!(v.closes("A").len(), 5);
        assert_eq!(v.last_close("A"), Some((d - 1, 104.0)));
        assert_eq!(v.close_asof("A", d - 1).unwrap(), Some((d - 1, 104.0)));
        assert!(v.close_asof("A", d).is_err());
        assert!(v.close_asof("A", d + 3).is_err());
    }

    /// A strategy that reads the price of the rebalance day.
    struct Cheat;
    impl Strategy for Cheat {
        fn name(&self) -> &str {
            "cheat"
        }
        fn weights(&self, v: &View, _: &[String]) -> Result<Vec<(String, f64)>, String> {
            v.close_asof("A", v.date())?;
            Ok(vec![("A".into(), 1.0)])
        }
    }

    #[test]
    fn a_strategy_that_reads_the_rebalance_day_fails() {
        let p = prices(vec![("SPY", series(3, |_| 50.0)), ("A", series(3, |_| 100.0))]);
        let e = run(&p, &["A".into()], &Cheat, &cfg(0.0, 2)).unwrap_err();
        assert!(e.contains("look-ahead"), "{e}");
    }

    #[test]
    fn weights_do_not_change_when_the_prices_of_the_rebalance_day_and_later_change() {
        use strategies::Momentum;
        // 30 tickers over 500 days; the strategy decides on day 450.
        let n = 500;
        let mk = |future: f64| -> Prices {
            let mut items = Vec::new();
            for k in 0..30i64 {
                let name: &'static str = Box::leak(format!("T{k:02}").into_boxed_str());
                items.push((name, series(n, move |i| if i >= 450 { future * 100.0 } else { 100.0 * (1.0 + 0.001 * (k + 1) as f64).powi(i as i32) })));
            }
            prices(items)
        };
        let universe: Vec<String> = (0..30).map(|k| format!("T{k:02}")).collect();
        let d = mapcheck::dates::parse(D0).unwrap() + 450;
        let (a, b) = (mk(1.0), mk(7.0));
        let wa = Momentum.weights(&View::new(d, &a), &universe).unwrap();
        let wb = Momentum.weights(&View::new(d, &b), &universe).unwrap();
        assert_eq!(wa, wb);
        assert_eq!(wa.len(), 6); // top quintile of 30
        assert_eq!(wa[0].0, "T24"); // the six highest drifts T24 to T29, in ticker order
    }
}
