//! The two reference strategies. Copy one as the start of a new strategy: implement `Strategy`, read prices through the
//! `View` only, and read other data (SEC facts) by its filing date, before the rebalance date.

use super::{Strategy, View};
use crate::dcf::auto::YearData;
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

/// The automatic model value over the price (`fa dcf returns`): the P50 of the model made from the fiscal years filed
/// before the date, with the curve, the beta and the price of the day before the date.
pub struct ValueRank<'a> {
    pub years: BTreeMap<String, Vec<YearData>>,
    pub series: &'a BTreeMap<String, Vec<(String, f64)>>,
    pub index: Option<&'a [(String, f64)]>,
    pub params: RetParams<'a>,
}

/// The fiscal years known on `day`: filed before `day` (a year with no filing date: 100 days after its end).
pub fn known_years(years: &[YearData], day: i64) -> Vec<YearData> {
    years
        .iter()
        .filter(|y| {
            let filed = y.filed.get(..10).and_then(mapcheck::dates::parse).or_else(|| y.end.get(..10).and_then(mapcheck::dates::parse).map(|e| e + 100));
            filed.is_some_and(|f| f < day)
        })
        .cloned()
        .collect()
}

/// The part of an ISO-dated series before `date` (text).
fn before<'s>(series: &'s [(String, f64)], date: &str) -> &'s [(String, f64)] {
    &series[..series.partition_point(|(d, _)| d.get(..10).unwrap_or("") < date)]
}

impl Strategy for ValueRank<'_> {
    fn name(&self) -> &str {
        "value-rank"
    }

    fn weights(&self, view: &View, universe: &[String]) -> Result<Vec<(String, f64)>, String> {
        let date = mapcheck::dates::format(view.date());
        let index = self.index.map(|m| before(m, &date));
        let params = RetParams { index, ..self.params_copy() };
        let scored: Vec<(String, f64)> = universe
            .par_iter()
            .filter_map(|t| {
                let years = known_years(self.years.get(t)?, view.date());
                let end = mapcheck::dates::parse(years.last()?.end.get(..10)?)?;
                if view.date() - end > 550 {
                    return None; // the last known year is too old
                }
                let (day, p0) = view.last_close(t)?;
                if view.date() - 1 - day > FRESH {
                    return None;
                }
                let series = before(self.series.get(t)?, &date);
                let (p50, _) = model_p50(t, &years, &mapcheck::dates::format(day), p0, series, &params)?;
                let ratio = p50 / p0;
                (ratio.is_finite() && p50 > 0.0).then(|| (t.clone(), ratio))
            })
            .collect();
        Ok(top_quintile(scored))
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

    fn year(end: &str, filed: &str) -> YearData {
        YearData { end: end.into(), filed: filed.into(), ..Default::default() }
    }

    #[test]
    fn a_year_is_known_only_after_its_filing_date() {
        let ys = vec![year("2019-12-31", "2020-02-20"), year("2020-12-31", "2021-02-19"), year("2021-12-31", "")];
        let d = |s: &str| mapcheck::dates::parse(s).unwrap();
        assert_eq!(known_years(&ys, d("2020-02-20")).len(), 0); // filed on the day: not before it
        assert_eq!(known_years(&ys, d("2020-02-21")).len(), 1);
        assert_eq!(known_years(&ys, d("2021-02-20")).len(), 2);
        assert_eq!(known_years(&ys, d("2022-04-10")).len(), 2); // no filing date: end + 100 days = 2022-04-10, not before
        assert_eq!(known_years(&ys, d("2022-04-11")).len(), 3);
    }
}
