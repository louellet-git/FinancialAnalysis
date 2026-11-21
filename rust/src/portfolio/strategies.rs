//! The two reference strategies. Copy one as the start of a new strategy: implement `Strategy`, read prices through the
//! `View` only, and read other data (SEC facts) as a vintage: the facts filed before the rebalance date.

use super::{Strategy, View};
use crate::dcf::auto::YearData;
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

impl Momentum {
    /// The 12-1 month momentum of a ticker on the date of the view (None: no fresh close or not enough history).
    fn score(view: &View, t: &str) -> Option<f64> {
        let yesterday = view.date() - 1;
        let (last_day, _) = view.last_close(t)?;
        if yesterday - last_day > FRESH {
            return None;
        }
        let at = |day: i64| view.close_asof(t, day).ok().flatten().filter(|(d, _)| day - d <= FRESH).map(|x| x.1);
        let (recent, old) = (at(yesterday - 30)?, at(yesterday - 365)?);
        Some(recent / old - 1.0)
    }
}

impl Strategy for Momentum {
    fn name(&self) -> &str {
        "momentum"
    }

    fn weights(&self, view: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
        let scored = universe.iter().filter_map(|t| Some((t.clone(), Momentum::score(view, t)?))).collect();
        Ok(top_quintile(scored))
    }

    fn signal(&self, view: &View, ticker: &str) -> Option<(f64, String)> {
        Momentum::score(view, ticker).map(|x| (x, String::new()))
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
    /// The signal (model P50 over the price) and the accession number of the filing of the vintage, by rebalance day and ticker.
    pub signals: std::sync::Mutex<BTreeMap<(i64, String), (f64, String)>>,
}

/// The fiscal years of one company as the SEC facts showed them after each annual filing: `(filing day, years)` in
/// date order. The years of a vintage come from the facts filed on that day or before, so a later restatement is not in it.
/// The second list has the accession number of the annual filing of each vintage (same order; it can be empty in a test).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vintages(pub Vec<(i64, Vec<YearData>)>, pub Vec<String>);

impl Vintages {
    /// The years of the latest vintage filed before `day` (empty if there is none).
    pub fn known(&self, day: i64) -> &[YearData] {
        self.index(day).map_or(&[], |k| self.0[k].1.as_slice())
    }

    /// The accession number of the filing of the vintage that `known(day)` gives.
    pub fn known_accession(&self, day: i64) -> Option<&str> {
        self.1.get(self.index(day)?).map(String::as_str)
    }

    fn index(&self, day: i64) -> Option<usize> {
        self.0.partition_point(|(f, _)| *f < day).checked_sub(1)
    }
}

/// The vintages of a company from its facts, and its template. The vintages before the last one filed before `from`
/// are not made: no decision from `from` on uses them.
pub fn vintages_of(registry: &Registry, facts: &CompanyFacts, from: i64) -> Option<(String, Vintages)> {
    if !facts.has_us_gaap() {
        return None;
    }
    let ends = |f: &CompanyFacts| period_ends(f, &registry.anchors, Forms::Annual, None);
    let years = |f: &CompanyFacts| -> Vec<YearData> {
        ends(f).iter().map(|e| YearData::from_period(registry, &resolve_period_with(registry, f, e, Forms::Annual, None))).collect()
    };
    let last = ends(facts).last().map(|e| resolve_period_with(registry, facts, e, Forms::Annual, None))?;
    let template = filer_template(registry, &last).to_string();
    let days: Vec<(i64, String)> = facts.annual_filing_dates().into_iter().filter_map(|d| Some((mapcheck::dates::parse(&d)?, d))).collect();
    let first = days.partition_point(|(f, _)| *f < from).saturating_sub(1);
    let list = days[first..].iter().map(|(f, d)| (*f, years(&facts.filed_up_to(d)))).collect();
    let accns = facts.annual_accessions();
    let accn_list = days[first..].iter().map(|(_, d)| accns.get(d).cloned().unwrap_or_default()).collect();
    Some((template, Vintages(list, accn_list)))
}

/// The vintages of a ticker from the cache (see [`vintages_of`]).
pub fn load_vintages(registry: &Registry, cache: &Path, ticker: &str, from: i64) -> Option<(String, Vintages)> {
    load_vintages_with(registry, cache, ticker, from, |_| {})
}

/// As [`load_vintages`], but `change` can edit the facts before the vintages are made (the poison check uses it).
pub fn load_vintages_with(registry: &Registry, cache: &Path, ticker: &str, from: i64, change: impl FnOnce(&mut CompanyFacts)) -> Option<(String, Vintages)> {
    let mut facts = mapcheck::run::read_facts(&mapcheck::run::facts_path(cache, ticker)?).ok()?;
    change(&mut facts);
    vintages_of(registry, &facts, from)
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

    fn signal(&self, view: &View, ticker: &str) -> Option<(f64, String)> {
        self.signals.lock().ok()?.get(&(view.date(), ticker.to_string())).cloned()
    }

    fn weights(&self, view: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
        let date = mapcheck::dates::format(view.date());
        let index = self.index.map(|m| before(m, &date));
        let params = RetParams { index, ..self.params_copy() };
        let scored: Vec<(String, f64, String)> = universe
            .par_iter()
            .filter_map(|t| {
                let vintages = self.years.get(t)?;
                let years = vintages.known(view.date());
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
                (ratio.is_finite() && p50 > 0.0).then(|| (t.clone(), ratio, vintages.known_accession(view.date()).unwrap_or_default().to_string()))
            })
            .collect();
        if let Ok(mut n) = self.scored.lock() {
            n.push(scored.len());
        }
        if let Ok(mut sig) = self.signals.lock() {
            for (t, x, accn) in &scored {
                sig.insert((view.date(), t.clone()), (*x, accn.clone()));
            }
        }
        Ok(top_quintile(scored.into_iter().map(|(t, x, _)| (t, x)).collect()))
    }
}

impl ValueRank<'_> {
    fn params_copy(&self) -> RetParams<'_> {
        let p = &self.params;
        RetParams { table: p.table, fade: p.fade, rules: p.rules, curves: p.curves, index: p.index, lag: p.lag, paths: p.paths, seed: p.seed }
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
        {"start":"2019-01-01","end":"2019-12-31","val":100,"form":"10-K","filed":"2020-02-20","accn":"0000000001-20-000001"},
        {"start":"2019-01-01","end":"2019-12-31","val":1000,"form":"10-K/A","filed":"2021-03-01","accn":"0000000001-21-000002"},
        {"start":"2020-01-01","end":"2020-12-31","val":120,"form":"10-K","filed":"2021-02-19","accn":"0000000001-21-000001"}]}}}}}"#;

    #[test]
    fn a_vintage_has_the_value_of_the_filings_before_the_date_not_the_restatement() {
        let reg = Registry::embedded().unwrap();
        let facts: CompanyFacts = serde_json::from_str(FACTS).unwrap();
        let (_, v) = vintages_of(&reg, &facts, 0).unwrap();
        assert_eq!(v.0.len(), 3);
        assert_eq!(v.known_accession(d("2021-03-02")), Some("0000000001-21-000002")); // the filing of the vintage in use
        assert_eq!(v.known_accession(d("2021-03-01")), Some("0000000001-21-000001"));
        assert_eq!(v.known_accession(d("2020-02-20")), None);
        assert!(v.known(d("2020-02-20")).is_empty()); // filed on the day: not before it
        let y = v.known(d("2020-02-21"));
        assert_eq!((y.len(), y[0].revenue), (1, Some(100.0)));
        let y = v.known(d("2021-03-01")); // the restatement is filed on this day: not known
        assert_eq!((y.len(), y[0].revenue, y[1].revenue), (2, Some(100.0), Some(120.0)));
        let y = v.known(d("2021-03-02"));
        assert_eq!((y[0].revenue, y[1].revenue), (Some(1000.0), Some(120.0)));
        // `from` drops the vintages that no decision from that day on uses (keeps the last one before it)
        let (_, v) = vintages_of(&reg, &facts, d("2021-02-25")).unwrap();
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
        Vintages(vec![(d("2020-02-20"), v1), (d("2021-02-19"), v2)], vec![])
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
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let (table, fade) = crate::dcf::auto_cli::load_table(&repo.join(crate::dcf::auto::DEFAULT_TABLE)).map_err(|e| e.0).unwrap();
        let rules = crate::dcf::auto_pit::parse_rules("").map_err(|e| e.0).unwrap();
        let params = RetParams { table: &table, fade: &fade, rules: &rules, curves: None, index: None, lag: 100, paths: 100, seed: 7 };
        let series: BTreeMap<String, Vec<(String, f64)>> = (0..N).map(|k| (format!("T{k:02}"), closes(k, poison_from))).collect();
        let prices = crate::portfolio::Prices::from_raw(&series);
        let index = market();
        let s = ValueRank { years: years.clone(), series: &series, index: Some(&index), params, scored: Default::default(), signals: Default::default() };
        let universe: Vec<String> = series.keys().cloned().collect();
        s.weights(&View::new(d(at), &prices), &universe).unwrap()
    }

    fn all_vintages() -> BTreeMap<String, Vintages> {
        (0..N).map(|k| (format!("T{k:02}"), vintages(k))).collect()
    }

    #[test]
    fn value_rank_weights_do_not_move_with_a_fact_filed_after_the_date() {
        let at = "2020-09-01"; // after vintage 1, 17 months before vintage 2
        let clean: BTreeMap<String, Vintages> = (0..N).map(|k| (format!("T{k:02}"), Vintages(vec![vintages(k).0.remove(0)], vec![]))).collect();
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
}
