//! The two reference strategies. Copy one as the start of a new strategy: implement `Strategy`, read prices through the
//! `View` only, and read other data (SEC facts) as a vintage: the facts filed before the rebalance date.

use super::{Strategy, View};
use crate::dcf::auto::YearData;
use crate::dcf::splits::{Basis, Split};
use mapcheck::checks::filer_template;
use mapcheck::facts::{period_ends, CompanyFacts, Forms};
use mapcheck::mapper::resolve_period_with;
use mapcheck::registry::Registry;
use std::path::Path;
use crate::dcf::auto_returns::{model_p50, RetParams};
use rayon::prelude::*;
use std::collections::BTreeMap;

/// A strategy with fewer scored tickers than this holds cash.
pub const MIN_SCORED: usize = 10;
/// The close used for a price must be at most this many days old.
const FRESH: i64 = 7;

/// Equal weight in the top quintile (the highest scores). The result is in ticker order.
pub fn top_quintile(mut scored: Vec<(String, f64)>) -> Vec<(String, f64)> {
    if scored.len() < MIN_SCORED {
        return Vec::new();
    }
    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let k = (scored.len() / 5).max(1);
    let mut top: Vec<(String, f64)> = scored.into_iter().take(k).map(|(t, _)| (t, 1.0 / k as f64)).collect();
    top.sort_by(|a, b| a.0.cmp(&b.0));
    top
}

/// The top quintile inside each sector, with the sector weight of the scored list: a sector with `n` of the `N` scored
/// tickers gets the weight `n / N`, split equally over its top `max(1, n / 5)` tickers. So the sector weights are those
/// of the list, and only the choice inside a sector differs from the equal weight. `sector_of` gives the sector of a
/// ticker (a ticker with no sector is in "Other"). Fewer than `MIN_SCORED` scored tickers: cash. The result is in ticker order.
pub fn top_quintile_by_sector(scored: Vec<(String, f64)>, sector_of: &BTreeMap<String, String>) -> Vec<(String, f64)> {
    let n_all = scored.len();
    if n_all < MIN_SCORED {
        return Vec::new();
    }
    let mut groups: BTreeMap<&str, Vec<(String, f64)>> = BTreeMap::new();
    for (t, x) in scored {
        let s = sector_of.get(&t).map_or("Other", String::as_str);
        groups.entry(s).or_default().push((t, x));
    }
    let mut out: Vec<(String, f64)> = Vec::new();
    for (_, mut g) in groups {
        g.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let k = (g.len() / 5).max(1);
        let w = g.len() as f64 / n_all as f64 / k as f64;
        out.extend(g.into_iter().take(k).map(|(t, _)| (t, w)));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Which fifth of each sector a sector-neutral strategy holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pick {
    /// The highest scores.
    #[default]
    Top,
    /// The lowest scores (the short side test of A-009; held long here, to compare it with the equal weight).
    Bottom,
}

/// The bottom quintile inside each sector, with the same sector weights as [`top_quintile_by_sector`]: the lowest scores
/// of each sector. Ties go to the first ticker name, as for the top.
pub fn bottom_quintile_by_sector(scored: Vec<(String, f64)>, sector_of: &BTreeMap<String, String>) -> Vec<(String, f64)> {
    top_quintile_by_sector(scored.into_iter().map(|(t, x)| (t, -x)).collect(), sector_of)
}

/// A sector with fewer scored tickers than this in one week is ranked with "Other" (A-009 item 3).
pub const MIN_SECTOR: usize = 5;

/// One candidate of the weekly rule: the ticker, its sector, its rank (the mid-rank percentile inside the sector, 1 is
/// the best) and its score.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub ticker: String,
    pub sector: String,
    pub rank: f64,
    pub score: f64,
    /// The ticker is in the top fifth of its sector (the first `max(1, n / 5)` places).
    pub top_fifth: bool,
}

/// The rank of each scored ticker inside its sector (A-009 item 3). A sector with fewer than `MIN_SECTOR` scored tickers
/// is ranked with "Other". With `n` tickers in the sector and the place `p` (0 for the highest score; ties go to the first
/// ticker name), rank = `(n - p - 0.5) / n`.
pub fn rank_in_sector(scored: &[(String, f64)], sector_of: &BTreeMap<String, String>) -> Vec<Ranked> {
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for (t, _) in scored {
        *count.entry(sector_of.get(t).map_or("Other", String::as_str)).or_default() += 1;
    }
    let mut groups: BTreeMap<&str, Vec<&(String, f64)>> = BTreeMap::new();
    for item in scored {
        let s = sector_of.get(&item.0).map_or("Other", String::as_str);
        let s = if count[s] < MIN_SECTOR { "Other" } else { s };
        groups.entry(s).or_default().push(item);
    }
    let mut out = Vec::new();
    for (s, mut g) in groups {
        g.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let n = g.len();
        let k = (n / 5).max(1);
        for (p, (t, x)) in g.into_iter().enumerate() {
            out.push(Ranked { ticker: t.clone(), sector: s.to_string(), rank: (n as f64 - p as f64 - 0.5) / n as f64, score: *x, top_fifth: p < k });
        }
    }
    out
}

/// The new tickers of one week (A-009 item 4): up to `max_new`, the highest rank, not held, at most one for each sector,
/// each in the top fifth of its sector. Within a sector the first candidate is the highest not held in the top fifth.
/// Order of the result: the highest rank first; equal ranks go to the higher score, then the first ticker name.
pub fn choose_new(ranked: &[Ranked], held: &dyn Fn(&str) -> bool, max_new: usize) -> Vec<String> {
    let mut best: BTreeMap<&str, &Ranked> = BTreeMap::new();
    for r in ranked.iter().filter(|r| r.top_fifth && !held(&r.ticker)) {
        let better = |a: &Ranked, b: &Ranked| a.rank.total_cmp(&b.rank).then_with(|| a.score.total_cmp(&b.score)).then_with(|| b.ticker.cmp(&a.ticker)).is_gt();
        if best.get(r.sector.as_str()).is_none_or(|cur| better(r, cur)) {
            best.insert(r.sector.as_str(), r);
        }
    }
    let mut v: Vec<&Ranked> = best.into_values().collect();
    v.sort_by(|a, b| b.rank.total_cmp(&a.rank).then_with(|| b.score.total_cmp(&a.score)).then_with(|| a.ticker.cmp(&b.ticker)));
    v.into_iter().take(max_new).map(|r| r.ticker.clone()).collect()
}

/// Equal weight in each ticker of the universe with a fresh close: the reference for the survivorship of the list.
pub struct EqualWeight;

impl Strategy for EqualWeight {
    fn name(&self) -> &str {
        "equal-weight"
    }

    fn weights(&self, view: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
        let held: Vec<&String> = universe.iter().filter(|t| view.last_close(t).is_some_and(|(day, _)| view.date() - 1 - day <= FRESH)).collect();
        let w = 1.0 / held.len().max(1) as f64;
        Ok(held.into_iter().map(|t| (t.clone(), w)).collect())
    }
}

/// 12-1 month price momentum: the close one month before the date over the close one year before, less 1.
pub struct Momentum;

impl Strategy for Momentum {
    fn name(&self) -> &str {
        "momentum"
    }

    fn weights(&self, view: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
        let yesterday = view.date() - 1;
        let scored = universe
            .iter()
            .filter_map(|t| {
                let (last_day, _) = view.last_close(t)?;
                if yesterday - last_day > FRESH {
                    return None;
                }
                let at = |day: i64| view.close_asof(t, day).ok().flatten().filter(|(d, _)| day - d <= FRESH).map(|x| x.1);
                let (recent, old) = (at(yesterday - 30)?, at(yesterday - 365)?);
                Some((t.clone(), recent / old - 1.0))
            })
            .collect();
        Ok(top_quintile(scored))
    }
}

/// The automatic model value over the price (`fa dcf returns`): the P50 of the model made from the fiscal years of the
/// vintage known before the date, with the curve, the beta and the price of the day before the date.
pub struct ValueRank<'a> {
    pub years: BTreeMap<String, Vintages>,
    pub series: &'a BTreeMap<String, Vec<(String, f64)>>,
    pub index: Option<&'a [(String, f64)]>,
    pub params: RetParams<'a>,
    /// The number of scored tickers at each rebalance, in the order of the calls.
    pub scored: std::sync::Mutex<Vec<usize>>,
    /// With a sector of each ticker: the top quintile inside each sector, with the sector weights of the scored list
    /// (`top_quintile_by_sector`). `None`: the top quintile of the whole list.
    pub sectors: Option<BTreeMap<String, String>>,
    /// With `sectors`: the top or the bottom quintile of each sector.
    pub pick: Pick,
}

/// The fiscal years of one company as the SEC facts showed them after each annual filing: `(filing day, years)` in
/// date order. The years of a vintage come from the facts filed on that day or before, so a later restatement is not in it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vintages(pub Vec<(i64, Vec<YearData>)>);

impl Vintages {
    /// The years of the latest vintage filed before `day` (empty if there is none).
    pub fn known(&self, day: i64) -> &[YearData] {
        let i = self.0.partition_point(|(f, _)| *f < day);
        i.checked_sub(1).map_or(&[], |k| self.0[k].1.as_slice())
    }
}

/// The vintages of a company from its facts, and its template. The vintages before the last one filed before `from`
/// are not made: no decision from `from` on uses them.
///
/// The share counts are on the basis of `today` (`splits`, see `dcf/splits.rs`), the basis of the closes of the store.
pub fn vintages_of(registry: &Registry, ticker: &str, facts: &CompanyFacts, from: i64, splits: &[Split], today: &str) -> Option<(String, Vintages)> {
    if !facts.has_us_gaap() {
        return None;
    }
    let ends = |f: &CompanyFacts| period_ends(f, &registry.anchors, Forms::Annual, None);
    let years = |f: &CompanyFacts| -> Vec<YearData> {
        ends(f)
            .iter()
            .map(|e| {
                let p = resolve_period_with(registry, f, e, Forms::Annual, None);
                YearData::from_period(registry, &p, &Basis { splits, ticker, today })
            })
            .collect()
    };
    let last = ends(facts).last().map(|e| resolve_period_with(registry, facts, e, Forms::Annual, None))?;
    let template = filer_template(registry, &last).to_string();
    let days: Vec<(i64, String)> = facts.annual_filing_dates().into_iter().filter_map(|d| Some((mapcheck::dates::parse(&d)?, d))).collect();
    let first = days.partition_point(|(f, _)| *f < from).saturating_sub(1);
    let list = days[first..].iter().map(|(f, d)| (*f, years(&facts.filed_up_to(d)))).collect();
    Some((template, Vintages(list)))
}

/// The vintages of a ticker from the cache (see [`vintages_of`]).
pub fn load_vintages(registry: &Registry, cache: &Path, ticker: &str, from: i64, splits: &[Split], today: &str) -> Option<(String, Vintages)> {
    let facts = mapcheck::run::read_facts(&mapcheck::run::facts_path(cache, ticker)?).ok()?;
    vintages_of(registry, ticker, &facts, from, splits, today)
}

/// The part of an ISO-dated series before `date` (text).
fn before<'s>(series: &'s [(String, f64)], date: &str) -> &'s [(String, f64)] {
    &series[..series.partition_point(|(d, _)| d.get(..10).unwrap_or("") < date)]
}

impl Strategy for ValueRank<'_> {
    fn name(&self) -> &str {
        "value-rank"
    }

    fn scored_counts(&self) -> Vec<usize> {
        self.scored.lock().map(|n| n.clone()).unwrap_or_default()
    }

    fn weights(&self, view: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
        let scored = self.score(view, universe);
        Ok(match (&self.sectors, self.pick) {
            (Some(s), Pick::Top) => top_quintile_by_sector(scored, s),
            (Some(s), Pick::Bottom) => bottom_quintile_by_sector(scored, s),
            (None, _) => top_quintile(scored),
        })
    }
}

impl ValueRank<'_> {
    /// The score (P50 / price) of each ticker with a model value on the date of the view, in the order of the universe. The
    /// count of scored tickers is kept for the report. Both the quarterly and the weekly strategy use this one function.
    pub fn score(&self, view: &View, universe: &[String]) -> Vec<(String, f64)> {
        let date = mapcheck::dates::format(view.date());
        let index = self.index.map(|m| before(m, &date));
        let params = RetParams { index, ..self.params_copy() };
        let scored: Vec<(String, f64)> = universe
            .par_iter()
            .filter_map(|t| {
                let years = self.years.get(t)?.known(view.date());
                let end = mapcheck::dates::parse(years.last()?.end.get(..10)?)?;
                if view.date() - end > 550 {
                    return None; // the last known year is too old
                }
                let (day, p0) = view.last_close(t)?;
                if view.date() - 1 - day > FRESH {
                    return None;
                }
                let series = before(self.series.get(t)?, &date);
                let (p50, _) = model_p50(t, years, &mapcheck::dates::format(day), p0, series, &params)?;
                let ratio = p50 / p0;
                (ratio.is_finite() && p50 > 0.0).then(|| (t.clone(), ratio))
            })
            .collect();
        if let Ok(mut n) = self.scored.lock() {
            n.push(scored.len());
        }
        scored
    }

    fn params_copy(&self) -> RetParams<'_> {
        let p = &self.params;
        RetParams { table: p.table, fade: p.fade, rules: p.rules, curves: p.curves, index: p.index, lag: p.lag, paths: p.paths, seed: p.seed }
    }
}

/// A-009 item 8: an open position is dropped on the rebalance this many days (13 weeks) or more after its entry.
pub const WEEKLY_HOLD_DAYS: i64 = 91;
/// A-009 item 4: new tickers each week, at most.
pub const WEEKLY_NEW: usize = 3;
/// A-009 item 7: the number of open positions, at most; each has the weight 1 / this.
pub const WEEKLY_MAX: usize = 39;

/// One week of the A-009 rule on the open positions (ticker -> entry day): drop the old ones, choose the new ones from the
/// scores, enter them on `date`. The cap of `WEEKLY_MAX` positions holds also when a holiday shifts an exit by one week.
/// Returns the tickers entered.
pub fn weekly_step(open: &mut BTreeMap<String, i64>, date: i64, scored: &[(String, f64)], sector_of: &BTreeMap<String, String>) -> Vec<String> {
    open.retain(|_, entry| date - *entry < WEEKLY_HOLD_DAYS);
    let ranked = rank_in_sector(scored, sector_of);
    let room = WEEKLY_MAX.saturating_sub(open.len()).min(WEEKLY_NEW);
    let new = choose_new(&ranked, &|t| open.contains_key(t), room);
    for t in &new {
        open.insert(t.clone(), date);
    }
    new
}

/// The weekly rule of A-009 (`--strategy value-rank-weekly`): the scores of [`ValueRank`] (the same function, the same
/// vintage, curve, beta and freshness rules), the rank inside the sector, up to 3 new tickers each week, a hold of 13
/// weeks. The state is the open positions with their entry day: the engine calls `weights` in date order, once for each
/// rebalance date, and the strategy must be used for one run only.
///
/// Difference from A-009: the engine trades each open position back to 1/39 at each rebalance, so a position that rose
/// is cut and a position that fell is added. A-009 holds without a rebalance. The data checks of A-009 item 5 that the
/// backtest has are in the score (last fiscal year at most 550 days old, P50 above 0). The mapping status and the
/// share-count guard are not in the backtest.
pub struct WeeklyValueRank<'a> {
    /// The scorer. Its `sectors` must be set.
    pub base: ValueRank<'a>,
    pub open: std::sync::Mutex<BTreeMap<String, i64>>,
}

impl Strategy for WeeklyValueRank<'_> {
    fn name(&self) -> &str {
        "value-rank-weekly"
    }

    fn scored_counts(&self) -> Vec<usize> {
        self.base.scored_counts()
    }

    fn weights(&self, view: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
        let sectors = self.base.sectors.as_ref().ok_or("value-rank-weekly needs the sector of each ticker")?;
        let scored = self.base.score(view, universe);
        let mut open = self.open.lock().map_err(|_| "the open positions are locked".to_string())?;
        weekly_step(&mut open, view.date(), &scored, sectors);
        let w = 1.0 / WEEKLY_MAX as f64;
        Ok(open.keys().map(|t| (t.clone(), w)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_quintile_is_equal_weight_of_the_highest_scores() {
        let scored: Vec<(String, f64)> = (0..20).map(|i| (format!("T{i:02}"), i as f64)).collect();
        let w = top_quintile(scored);
        assert_eq!(w.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["T16", "T17", "T18", "T19"]);
        assert!(w.iter().all(|x| (x.1 - 0.25).abs() < 1e-12));
    }

    #[test]
    fn the_sector_top_quintile_keeps_the_sector_weights_of_the_list() {
        // sector A: 10 tickers (A0..A9, score = index), sector B: 2 tickers, one ticker with no sector (Other)
        let mut scored: Vec<(String, f64)> = (0..10).map(|i| (format!("A{i}"), i as f64)).collect();
        scored.extend([("B0".to_string(), 5.0), ("B1".to_string(), 1.0), ("Z".to_string(), 0.0)]);
        let sectors: BTreeMap<String, String> = (0..10).map(|i| (format!("A{i}"), "A".to_string())).chain([("B0".into(), "B".into()), ("B1".into(), "B".into())]).collect();
        let w = top_quintile_by_sector(scored, &sectors);
        // A: weight 10/13 over its top 2 (A8, A9); B: 2/13 on its top 1 (B0); Other: 1/13 on Z
        let n = 13.0;
        let want = [("A8", 5.0 / n), ("A9", 5.0 / n), ("B0", 2.0 / n), ("Z", 1.0 / n)];
        assert_eq!(w.len(), want.len());
        for ((t, x), (wt, wx)) in w.iter().zip(want) {
            assert_eq!(t, wt);
            assert!((x - wx).abs() < 1e-12, "{t} {x} {wx}");
        }
        assert!((w.iter().map(|x| x.1).sum::<f64>() - 1.0).abs() < 1e-12);
        assert!(top_quintile_by_sector((0..MIN_SCORED - 1).map(|i| (format!("T{i}"), 0.0)).collect(), &sectors).is_empty());
    }

    #[test]
    fn too_few_scored_tickers_give_cash() {
        let scored: Vec<(String, f64)> = (0..MIN_SCORED - 1).map(|i| (format!("T{i}"), i as f64)).collect();
        assert!(top_quintile(scored).is_empty());
    }

    fn d(s: &str) -> i64 {
        mapcheck::dates::parse(s).unwrap()
    }

    fn scores(n: usize) -> Vec<(String, f64)> {
        (0..n).map(|i| (format!("T{i:02}"), i as f64)).collect()
    }

    #[test]
    fn a_list_of_10_to_14_scores_gives_two_tickers() {
        assert!(top_quintile(scores(9)).is_empty());
        for n in 10..=14 {
            let w = top_quintile(scores(n));
            assert_eq!(w.len(), 2, "{n} scores");
            assert!(w.iter().all(|x| (x.1 - 0.5).abs() < 1e-12));
        }
        assert_eq!(top_quintile(scores(15)).len(), 3);
    }

    #[test]
    fn ties_at_the_cut_go_to_the_first_ticker_name() {
        // 10 scores; the top 2 slots: T00 has the one highest score, five tickers tie for the second place.
        let mut v: Vec<(String, f64)> = vec![("T00".into(), 9.0)];
        v.extend(["T05", "T03", "T09", "T01", "T07"].iter().map(|t| (t.to_string(), 5.0)));
        v.extend(["T02", "T04", "T06", "T08"].iter().map(|t| (t.to_string(), 1.0)));
        let w = top_quintile(v.clone());
        assert_eq!(w.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["T00", "T01"]);
        v.reverse(); // the input order does not change the result
        assert_eq!(top_quintile(v), w);
    }

    const FACTS: &str = r#"{"facts":{"us-gaap":{"Revenues":{"units":{"USD":[
        {"start":"2019-01-01","end":"2019-12-31","val":100,"form":"10-K","filed":"2020-02-20"},
        {"start":"2019-01-01","end":"2019-12-31","val":1000,"form":"10-K/A","filed":"2021-03-01"},
        {"start":"2020-01-01","end":"2020-12-31","val":120,"form":"10-K","filed":"2021-02-19"}]}}}}}"#;

    #[test]
    fn a_vintage_has_the_value_of_the_filings_before_the_date_not_the_restatement() {
        let reg = Registry::embedded().unwrap();
        let facts: CompanyFacts = serde_json::from_str(FACTS).unwrap();
        let (_, v) = vintages_of(&reg, "T", &facts, 0, &[], "2026-11-21").unwrap();
        assert_eq!(v.0.len(), 3);
        assert!(v.known(d("2020-02-20")).is_empty()); // filed on the day: not before it
        let y = v.known(d("2020-02-21"));
        assert_eq!((y.len(), y[0].revenue), (1, Some(100.0)));
        let y = v.known(d("2021-03-01")); // the restatement is filed on this day: not known
        assert_eq!((y.len(), y[0].revenue, y[1].revenue), (2, Some(100.0), Some(120.0)));
        let y = v.known(d("2021-03-02"));
        assert_eq!((y[0].revenue, y[1].revenue), (Some(1000.0), Some(120.0)));
        // `from` drops the vintages that no decision from that day on uses (keeps the last one before it)
        let (_, v) = vintages_of(&reg, "T", &facts, d("2021-02-25"), &[], "2026-11-21").unwrap();
        assert_eq!(v.0.len(), 2);
        assert_eq!(v.known(d("2021-02-25")).len(), 2);
    }

    #[test]
    fn a_fact_with_no_filing_date_is_not_in_a_vintage() {
        let facts: CompanyFacts = serde_json::from_str(r#"{"facts":{"us-gaap":{"Revenues":{"units":{"USD":[
            {"start":"2019-01-01","end":"2019-12-31","val":100,"form":"10-K"}]}}}}}"#).unwrap();
        assert!(facts.annual_filing_dates().is_empty());
        assert!(facts.filed_up_to("2099-01-01").entries(&mapcheck::registry::Tag { full: "us-gaap/Revenues".into(), namespace: "us-gaap".into(), name: "Revenues".into() }, "USD").is_empty());
    }

    // ---- ValueRank: a poison test of the point-in-time rule ----

    const N: usize = 12;

    /// One fiscal year `i` (0 = 2012) of ticker `k`; `scale` multiplies each money value.
    fn fy(k: usize, i: usize, scale: f64) -> YearData {
        let rev = scale * 1000.0 * (1.0 + 0.15 * k as f64) * (1.0 + 0.03 * i as f64);
        YearData {
            end: format!("{}-12-31", 2012 + i),
            filed: format!("{}-02-20", 2013 + i),
            revenue: Some(rev),
            operating_cash_flow: Some(0.15 * rev),
            capex: Some(0.04 * rev),
            sbc: Some(0.01 * rev),
            income_tax: Some(0.03 * rev),
            pretax_income: Some(0.12 * rev),
            total_debt: Some(0.3 * rev),
            cash: Some(0.1 * rev),
            shares: Some(100.0),
            shares_outstanding: Some(100.0),
            ..Default::default()
        }
    }

    /// The restatement factor of ticker `k`: it differs by ticker, so a poison changes the rank (a uniform scale keeps it).
    fn poison_scale(k: usize) -> f64 {
        1.0 + 400.0 * ((k * 7) % N) as f64
    }

    /// The price factor of ticker `k` from the poison date on: it differs by ticker. A factor of 1000 puts one huge weekly
    /// return into the beta of the ticker, so a strategy that reads the series after the date changes its rank.
    fn price_poison(k: usize) -> f64 {
        if k % 4 == 2 { 1000.0 } else { 1.0 + 0.5 * (k % 3) as f64 }
    }

    /// Per ticker: vintage 1 (filed 2020-02-20) has FY2012 to FY2019. Vintage 2 (filed 2021-02-19) is poisoned: all years
    /// restated by a factor that differs by ticker (`poison_scale`) and FY2020 added.
    fn vintages(k: usize) -> Vintages {
        let v1: Vec<YearData> = (0..8).map(|i| fy(k, i, 1.0)).collect();
        let v2: Vec<YearData> = (0..9).map(|i| fy(k, i, poison_scale(k))).collect();
        Vintages(vec![(d("2020-02-20"), v1), (d("2021-02-19"), v2)])
    }

    /// Daily closes from 2017-01-01 to 2021-12-31; from `poison_from` on the close is larger by a factor that differs by ticker.
    fn closes(k: usize, poison_from: &str) -> Vec<(String, f64)> {
        let (a, z, p) = (d("2017-01-01"), d("2021-12-31"), d(poison_from));
        (a..=z).map(|day| (mapcheck::dates::format(day), if day >= p { price_poison(k) } else { 1.0 } * (15.0 + 0.5 * k as f64 + 0.001 * (day - a) as f64) * (market_level(day - a) / 100.0).powf(0.4 + 0.25 * (k % 5) as f64))).collect()
    }

    /// The level of the market index `n` days after 2017-01-01.
    fn market_level(n: i64) -> f64 {
        100.0 + 0.01 * n as f64 + 3.0 * (n as f64 * 0.37).sin()
    }

    /// The market index. The tickers move with it (different betas), so the beta needs 104 weeks of both series.
    fn market() -> Vec<(String, f64)> {
        let (a, z) = (d("2017-01-01"), d("2021-12-31"));
        (a..=z).map(|day| (mapcheck::dates::format(day), market_level(day - a))).collect()
    }

    fn rank(years: &BTreeMap<String, Vintages>, poison_from: &str, at: &str) -> Vec<(String, f64)> {
        let series: BTreeMap<String, Vec<(String, f64)>> = (0..N).map(|k| (format!("T{k:02}"), closes(k, poison_from))).collect();
        with_rank(years, &series, at, |s, view, universe| s.weights(view, universe).unwrap())
    }

    /// Runs `f` on a `ValueRank` of the fixture (curve none, 100 paths, seed 7).
    fn with_rank<T>(years: &BTreeMap<String, Vintages>, series: &BTreeMap<String, Vec<(String, f64)>>, at: &str, f: impl FnOnce(&ValueRank, &View, &[String]) -> T) -> T {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let (table, fade) = crate::dcf::auto_cli::load_table(&repo.join(crate::dcf::auto::DEFAULT_TABLE)).map_err(|e| e.0).unwrap();
        let rules = crate::dcf::auto_pit::parse_rules("").map_err(|e| e.0).unwrap();
        let params = RetParams { table: &table, fade: &fade, rules: &rules, curves: None, index: None, lag: 100, paths: 100, seed: 7 };
        let prices = crate::portfolio::Prices::from_raw(series);
        let index = market();
        let s = ValueRank { years: years.clone(), series, index: Some(&index), params, scored: Default::default(), sectors: None, pick: Pick::Top };
        let universe: Vec<String> = series.keys().cloned().collect();
        f(&s, &View::new(d(at), &prices), &universe)
    }

    // ---- the split basis: the k-test ----

    /// A `PeriodData` with the values of a `YearData` (the variables that the model reads).
    fn period_of(reg: &Registry, y: &YearData) -> mapcheck::mapper::PeriodData {
        let mut values = vec![None; reg.variables.len()];
        let mut put = |name: &str, v: Option<f64>| {
            if let (Some(i), Some(v)) = (reg.try_var(name), v) {
                values[i] = Some(v);
            }
        };
        put("revenue", y.revenue);
        put("operating_cash_flow", y.operating_cash_flow);
        put("capex_total", y.capex);
        put("sbc_total", y.sbc);
        put("income_tax_expense", y.income_tax);
        put("income_before_tax", y.pretax_income);
        put("total_debt", y.total_debt);
        put("cash_and_short_term_investments", y.cash);
        put("shares_diluted", y.shares);
        put("shares_outstanding", y.shares_outstanding);
        let n = values.len();
        // every value comes from a tag fact filed on `y.filed`
        let filed_of = values.iter().map(|v| v.map(|_| y.filed.clone())).collect();
        mapcheck::mapper::PeriodData { end: y.end.clone(), forms: Forms::Annual, filed: Some(y.filed.clone()), filed_of, values, sources: vec![None; n], raw_extras: Default::default() }
    }

    /// The same period as shown in another filing (a later 10-K that repeats it as a comparative).
    fn filed_on(mut p: mapcheck::mapper::PeriodData, filed: &str) -> mapcheck::mapper::PeriodData {
        p.filed = Some(filed.to_string());
        p.filed_of = p.values.iter().map(|v| v.map(|_| filed.to_string())).collect();
        p
    }

    const TODAY: &str = "2026-11-21";
    /// The ex-date of the split of the k-test; the decisions of the test are before it.
    const D: &str = "2021-06-01";

    fn splitter(k: usize) -> bool {
        k.is_multiple_of(3)
    }

    type Closes = BTreeMap<String, Vec<(String, f64)>>;

    /// The fixture company set with a k:1 split on `D` for the tickers T00, T03, T06 and T09 (`kf` None: no split). The SEC
    /// facts before `D` are unchanged; a fact dated after `D` has the shares times k. The closes before `D` are divided by k
    /// (the store adjusts them). The vintages go through `YearData::from_period` with the rows of `splits`.
    fn k_copy(kf: Option<f64>) -> (BTreeMap<String, Vintages>, Closes) {
        let reg = Registry::embedded().unwrap();
        let mut splits = Vec::new();
        let mut years = BTreeMap::new();
        let mut series = BTreeMap::new();
        for k in 0..N {
            let t = format!("T{k:02}");
            let kk = kf.filter(|_| splitter(k));
            if let Some(f) = kk {
                splits.push(Split { ticker: t.clone(), ex_date: D.into(), old_rate: 1.0, new_rate: f, factor: f, kind: "forward".into(), sec_check: "match".into(), source: String::new() });
            }
            let v = Vintages(
                vintages(k)
                    .0
                    .into_iter()
                    .map(|(day, ys)| {
                        let ys = ys
                            .iter()
                            .map(|y| {
                                let mut raw = y.clone();
                                // a fact that is filed after the split shows the counts of the new basis
                                let after = raw.filed.as_str() > D;
                                let m = if after { kk.unwrap_or(1.0) } else { 1.0 };
                                raw.shares = raw.shares.map(|s| s * m);
                                raw.shares_outstanding = raw.shares_outstanding.map(|s| s * m);
                                let p = period_of(&reg, &raw);
                                YearData::from_period(&reg, &p, &Basis { splits: &splits, ticker: &t, today: TODAY })
                            })
                            .collect();
                        (day, ys)
                    })
                    .collect(),
            );
            years.insert(t.clone(), v);
            let c = closes(k, "2030-01-01").into_iter().map(|(day, x)| (day.clone(), match kk { Some(f) if day.as_str() < D => x / f, _ => x })).collect();
            series.insert(t, c);
        }
        (years, series)
    }

    #[test]
    fn the_k_test_no_decision_before_the_split_date_changes() {
        let (years0, series0) = k_copy(None);
        for kf in [2.0, 25.0, 0.1] {
            let (years1, series1) = k_copy(Some(kf));
            for at in ["2020-09-01", "2021-05-01"] {
                let a = with_rank(&years0, &series0, at, |s, v, u| (s.score(v, u), s.weights(v, u).unwrap()));
                let b = with_rank(&years1, &series1, at, |s, v, u| (s.score(v, u), s.weights(v, u).unwrap()));
                assert_eq!(a.0.len(), N, "k {kf} at {at}: all tickers scored");
                assert_eq!(a.1, b.1, "k {kf} at {at}: the decision");
                for ((ta, ra), (tb, rb)) in a.0.iter().zip(&b.0) {
                    assert_eq!(ta, tb);
                    assert!((ra / rb - 1.0).abs() < 1e-9, "k {kf} at {at} {ta}: P50/price {ra} {rb}");
                }
            }
        }
    }

    #[test]
    fn the_k_test_fails_without_the_correction() {
        // control: the same data with the factor 1 (the defect) moves the P50 / price of the splitters by k
        let (years0, series0) = k_copy(None);
        let (_, series1) = k_copy(Some(25.0));
        let at = "2020-09-01";
        let a = with_rank(&years0, &series0, at, |s, v, u| s.score(v, u));
        let b = with_rank(&years0, &series1, at, |s, v, u| s.score(v, u));
        for ((t, ra), (_, rb)) in a.iter().zip(&b) {
            let k: usize = t[1..].parse().unwrap();
            if splitter(k) {
                assert!(rb / ra > 15.0, "{t}: {ra} {rb}"); // the defect: too high by about the split factor
            } else {
                assert!((rb / ra - 1.0).abs() < 1e-9, "{t}: {ra} {rb}");
            }
        }
    }

    #[test]
    fn the_share_count_takes_the_split_factor_of_the_filing_date_not_the_period_end() {
        let reg = Registry::embedded().unwrap();
        let splits = [Split { ticker: "X".into(), ex_date: D.into(), old_rate: 1.0, new_rate: 25.0, factor: 25.0, kind: "forward".into(), sec_check: "match".into(), source: String::new() }];
        let basis = Basis { splits: &splits, ticker: "X", today: TODAY };
        let fy2020 = period_of(&reg, &fy(0, 8, 1.0)); // period end 2020-12-31, filed 2021-02-20 (before the split)
        let counts = |p: &mapcheck::mapper::PeriodData| {
            let y = YearData::from_period(&reg, p, &basis);
            (y.shares, y.shares_outstanding)
        };
        // a split after the filing: the factor (the 10-K shows the old basis)
        assert_eq!(counts(&fy2020), (Some(2500.0), Some(2500.0)));
        // a split between the period end and the filing: the filing shows the new basis already: the factor is 1
        assert_eq!(counts(&filed_on(fy2020.clone(), "2021-08-01")), (Some(100.0), Some(100.0)));
        // the restated comparative of a later filing: 1
        assert_eq!(counts(&filed_on(fy2020.clone(), "2022-02-20")), (Some(100.0), Some(100.0)));
        // the filing date of the split itself: the ex-date is not after the filing: 1
        assert_eq!(counts(&filed_on(fy2020.clone(), D)), (Some(100.0), Some(100.0)));
        // one period with the diluted count from one filing and the count at the end from another: each has its own factor
        let mut mixed = fy2020.clone();
        mixed.filed_of[reg.try_var("shares_outstanding").unwrap()] = Some("2021-08-01".into());
        assert_eq!(counts(&mixed), (Some(2500.0), Some(100.0)));
        // a value with no tag fact (a formula value) takes the filing date of the period
        let mut formula = fy2020.clone();
        formula.filed_of = vec![None; formula.values.len()];
        assert_eq!(counts(&formula), (Some(2500.0), Some(2500.0)));
        // no filing date at all: the period end
        formula.filed = None;
        assert_eq!(counts(&formula), (Some(2500.0), Some(2500.0)));
    }

    fn all_vintages() -> BTreeMap<String, Vintages> {
        (0..N).map(|k| (format!("T{k:02}"), vintages(k))).collect()
    }

    #[test]
    fn value_rank_weights_do_not_move_with_a_fact_filed_after_the_date() {
        let at = "2020-09-01"; // after vintage 1, 17 months before vintage 2
        let clean: BTreeMap<String, Vintages> = (0..N).map(|k| (format!("T{k:02}"), Vintages(vec![vintages(k).0.remove(0)]))).collect();
        let (a, b) = (rank(&clean, "2030-01-01", at), rank(&all_vintages(), "2030-01-01", at));
        assert_eq!(a.len(), 2, "12 scored tickers: the top 2");
        assert_eq!(a, b);
        // control: the poison is not dead code; after its filing date the weights change
        assert_ne!(rank(&all_vintages(), "2030-01-01", "2021-09-01"), rank(&clean, "2030-01-01", "2021-09-01"));
    }

    #[test]
    fn value_rank_weights_do_not_move_with_a_price_of_the_date_or_later() {
        let at = "2020-09-01";
        let years = all_vintages();
        assert_eq!(rank(&years, "2030-01-01", at), rank(&years, at, at));
    }

    // ---- the short side test and the weekly rule (A-009) ----

    #[test]
    fn the_bottom_quintile_of_each_sector_has_the_sector_weights_of_the_list() {
        // The data of the top test: A has 10 tickers (score = index), B has 2, Z has no sector.
        let mut scored: Vec<(String, f64)> = (0..10).map(|i| (format!("A{i}"), i as f64)).collect();
        scored.extend([("B0".to_string(), 5.0), ("B1".to_string(), 1.0), ("Z".to_string(), 0.0)]);
        let sectors: BTreeMap<String, String> = (0..10).map(|i| (format!("A{i}"), "A".to_string())).chain([("B0".into(), "B".into()), ("B1".into(), "B".into())]).collect();
        let w = bottom_quintile_by_sector(scored, &sectors);
        // A: weight 10/13 over its bottom 2 (A0, A1); B: 2/13 on its bottom 1 (B1); Other: 1/13 on Z
        let n = 13.0;
        let want = [("A0", 5.0 / n), ("A1", 5.0 / n), ("B1", 2.0 / n), ("Z", 1.0 / n)];
        assert_eq!(w.len(), want.len());
        for ((t, x), (wt, wx)) in w.iter().zip(want) {
            assert_eq!(t, wt);
            assert!((x - wx).abs() < 1e-12, "{t} {x} {wx}");
        }
    }

    /// Scores of 4 sectors: X and Y have 10 tickers (X0..X9 score i; Y0..Y9 score i + 0.5), Z and W have 5 (Z0..Z4 score i;
    /// W0..W4 score i - 0.5).
    fn four_sectors() -> (Vec<(String, f64)>, BTreeMap<String, String>) {
        let mut scored = Vec::new();
        let mut sectors = BTreeMap::new();
        for (s, n, add) in [("X", 10, 0.0), ("Y", 10, 0.5), ("Z", 5, 0.0), ("W", 5, -0.5)] {
            for i in 0..n {
                scored.push((format!("{s}{i}"), i as f64 + add));
                sectors.insert(format!("{s}{i}"), s.to_string());
            }
        }
        (scored, sectors)
    }

    #[test]
    fn a_small_sector_is_ranked_with_other_and_the_rank_is_the_mid_rank_percentile() {
        // V has 3 scored tickers, and 2 tickers have no sector: together 5 in "Other".
        let scored: Vec<(String, f64)> = [("V0", 5.0), ("V1", 4.0), ("V2", 3.0), ("N0", 2.0), ("N1", 1.0)].iter().map(|(t, x)| (t.to_string(), *x)).collect();
        let sectors: BTreeMap<String, String> = ["V0", "V1", "V2"].iter().map(|t| (t.to_string(), "V".to_string())).collect();
        let r = rank_in_sector(&scored, &sectors);
        assert!(r.iter().all(|x| x.sector == "Other"));
        let rank_of = |t: &str| r.iter().find(|x| x.ticker == t).unwrap().rank;
        for (t, want) in [("V0", 0.9), ("V1", 0.7), ("V2", 0.5), ("N0", 0.3), ("N1", 0.1)] {
            assert!((rank_of(t) - want).abs() < 1e-12, "{t}");
        }
        assert_eq!(r.iter().filter(|x| x.top_fifth).map(|x| x.ticker.as_str()).collect::<Vec<_>>(), ["V0"]);
    }

    #[test]
    fn the_weekly_choice_is_one_for_each_sector_in_the_top_fifth_and_fewer_than_3_when_fewer_qualify() {
        let (scored, sectors) = four_sectors();
        let mut open = BTreeMap::new();
        let d0 = d("2020-01-31");
        // Week 1: the best of each sector is X9 (rank 0.95, score 9), Y9 (0.95, 9.5), Z4 (0.9, 4), W4 (0.9, 3.5).
        // Highest rank first, then the higher score: Y9, X9, Z4. W4 is the fourth: not taken (3 at most).
        assert_eq!(weekly_step(&mut open, d0, &scored, &sectors), ["Y9", "X9", "Z4"]);
        // Week 2: Y9, X9, Z4 are held. W4 (0.9); the next in the top 2 of X and Y (X8 0.85, Y8 0.85 with the higher score).
        // Z has a top fifth of 1 place (held): no Z candidate. The third place goes to Y8, not to X8.
        assert_eq!(weekly_step(&mut open, d0 + 7, &scored, &sectors), ["W4", "Y8", "X8"]);
        // Week 3: all top-fifth places of the 4 sectors are held: no trade (fewer than 3).
        assert!(weekly_step(&mut open, d0 + 14, &scored, &sectors).is_empty());
        assert_eq!(open.len(), 6);
        // One sector with a free place gives one trade
        let mut few = BTreeMap::new();
        let only_w: Vec<(String, f64)> = scored.iter().filter(|x| x.0.starts_with('W')).cloned().collect();
        assert_eq!(weekly_step(&mut few, d0, &only_w, &sectors), ["W4"]);
    }

    #[test]
    fn a_ticker_outside_the_top_fifth_is_never_chosen() {
        let (scored, sectors) = four_sectors();
        let ranked = rank_in_sector(&scored, &sectors);
        // X9 held, X8 held: the next X ticker, X7, is not in the top 2 of 10.
        let held = |t: &str| t == "X9" || t == "X8";
        let new = choose_new(&ranked, &held, 10);
        assert!(!new.iter().any(|t| t.starts_with('X')), "{new:?}");
        assert_eq!(new, ["Y9", "Z4", "W4"]);
    }

    #[test]
    fn the_position_is_dropped_13_weeks_or_91_days_after_the_entry() {
        let d0 = d("2020-01-31");
        let mut open: BTreeMap<String, i64> = [("A".to_string(), d0), ("B".to_string(), d0 + 7)].into_iter().collect();
        weekly_step(&mut open, d0 + 90, &[], &BTreeMap::new());
        assert_eq!(open.len(), 2, "90 days: held");
        weekly_step(&mut open, d0 + 91, &[], &BTreeMap::new());
        assert_eq!(open.keys().collect::<Vec<_>>(), ["B"], "91 days: A is dropped, B (84 days) stays");
        weekly_step(&mut open, d0 + 98, &[], &BTreeMap::new());
        assert!(open.is_empty());
        // 13 weekly Fridays: the entry of week 1 is dropped at week 14 (the call of index 13)
        let (scored, sectors) = four_sectors();
        let mut open = BTreeMap::new();
        weekly_step(&mut open, d0, &scored, &sectors);
        for k in 1..13 {
            weekly_step(&mut open, d0 + 7 * k, &[], &sectors);
            assert_eq!(open.len(), 3, "week {k}");
        }
        weekly_step(&mut open, d0 + 7 * 13, &[], &sectors);
        assert!(open.is_empty());
    }

    #[test]
    fn at_most_39_positions_are_open() {
        let (scored, sectors) = four_sectors();
        let d0 = d("2020-01-31");
        // 38 open positions from last week: room for 1 new
        let mut open: BTreeMap<String, i64> = (0..38).map(|i| (format!("H{i:02}"), d0 - 7)).collect();
        assert_eq!(weekly_step(&mut open, d0, &scored, &sectors), ["Y9"]);
        assert_eq!(open.len(), 39);
        assert!(weekly_step(&mut open, d0 + 7, &scored, &sectors).is_empty());
        assert_eq!(open.len(), 39);
        // 3 a week for 12 weeks and no exit yet: 36, then 39 at the 13th week
        let mut open = BTreeMap::new();
        let mut n = 0;
        for k in 0..13 {
            // each week 4 new sectors of 5 tickers with new names: no ticker is held, 3 qualify
            let mut sc = Vec::new();
            let mut sec = BTreeMap::new();
            for s in ["P", "Q", "R", "T"] {
                for i in 0..5 {
                    let t = format!("{s}{k:02}_{i}");
                    sc.push((t.clone(), i as f64));
                    sec.insert(t, format!("{s}{k:02}"));
                }
            }
            n += weekly_step(&mut open, d0 + 7 * k, &sc, &sec).len();
        }
        assert_eq!((n, open.len()), (39, 39));
    }
}
