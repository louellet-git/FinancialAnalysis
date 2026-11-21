//! US corporate bond yields and the cost of debt by rating, from public sources.
//!
//! The driver engine prices debt as `kd = rf + kd_spread`, with `rf` the Treasury 10Y. This
//! module gives a market value for `kd_spread` and the context around it.
//!
//! The cost of debt of a rating is the Treasury HQM spot yield at the maturity (10 years by
//! default) plus a fixed step for the rating bucket:
//!
//! - HQM: U.S. Treasury high quality market corporate bond yield curve, monthly average spot
//!   rates, 1 to 100 years, a blend of AAA, AA and A bonds. Read from the Treasury files
//!   (`fa bonds hqm get`, module `hqm`).
//! - Step: the default spread of the bucket minus the default spread of the HQM reference
//!   bucket (`REFERENCE_BUCKET`, step 0). The spreads are the table of A. Damodaran, "Ratings,
//!   Interest Coverage Ratios and Default Spread", data of January 2026,
//!   <https://pages.stern.nyu.edu/~adamodar/New_Home_Page/datafile/ratings.html>. They are facts
//!   written in this file, not a copy of a data file. Record: `docs/assumptions/A-200` to `A-203`.
//!
//! The ICE BofA spreads and the Moody's Aaa/Baa yields are not used: their terms do not permit a
//! stored copy.
//!
//! `spread_over_10y` = cost of debt - UST 10Y on the date. This is the engine's `kd_spread`
//! (kd = rf_10Y + kd_spread), so the verdict uses it.
//!
//! UNITS: decimals (0.0067 = 0.67%). The HQM rate is a spot rate (a zero rate), the UST 10Y is a
//! par yield: the spread has this small basis.

use crate::rates::{is_fresh_file, iso, observation, parse_fred_csv, round6, save_cache, Rates, Res};
use crate::{CreditSnapshot, HqmCurve, RateObservation, RatingQuote};
use std::collections::BTreeMap;

pub const RATINGS: [&str; 7] = ["AAA", "AA", "A", "BBB", "BB", "B", "CCC"];
/// The rating bucket of the HQM curve. Its step is 0. HQM holds AAA, AA and A bonds; A is the
/// largest group of them (convention, record A-201).
pub const REFERENCE_BUCKET: &str = "A";
/// The maturity (years) of the cost of debt, as the `rf` of the engine (record A-202).
pub const DEFAULT_MATURITY: i64 = 10;
pub const HQM_MATURITIES: [i64; 8] = [1, 2, 3, 5, 7, 10, 20, 30];

/// Default spread over the risk-free rate by rating (decimals): Damodaran, January 2026, the
/// middle notch of each letter grade (Aaa/AAA, Aa2/AA, A2/A, Baa2/BBB, Ba2/BB, B2/B, Caa/CCC).
const DEFAULT_SPREAD: [(&str, f64); 7] = [("AAA", 0.0040), ("AA", 0.0055), ("A", 0.0078), ("BBB", 0.0111), ("BB", 0.0184), ("B", 0.0321), ("CCC", 0.0885)];

pub const HQM_SOURCE: &str = crate::hqm::SOURCE;
pub const STEP_SOURCE: &str = "Damodaran default spreads, Jan 2026";
/// The date of the table (the site update of 2026-01-09).
pub const STEP_DATE: &str = "2026-01-09";

/// Days: a monthly average has the date of the 1st and is published after the month end.
const MONTHLY_MAX_AGE: i64 = 70;

/// The FRED series that the bond functions cache in `data/raw/bonds/fred_<ID>.csv`, in a stable order: the Treasury
/// HQM corporate spot rates (the readers now use the Treasury files; the FRED copy is a cross-check).
pub fn cached_series_ids() -> Vec<String> {
    HQM_MATURITIES.iter().map(|n| format!("HQMCB{n}YR")).collect()
}

/// The cache file of one FRED series of the bond functions.
pub fn cached_series_path(cache: &std::path::Path, id: &str) -> std::path::PathBuf {
    cache.join("bonds").join(format!("fred_{id}.csv"))
}

/// The default spread of a bucket over the risk-free rate (Damodaran), decimals.
pub fn default_spread(bucket: &str) -> f64 {
    DEFAULT_SPREAD.iter().find(|(b, _)| *b == bucket).map_or(f64::NAN, |(_, s)| *s)
}

/// The step of a bucket over the HQM reference bucket, decimals. The step of the reference is 0.
pub fn rating_step(bucket: &str) -> f64 {
    round6(default_spread(bucket) - default_spread(REFERENCE_BUCKET))
}

/// An S&P or Fitch rating (`A-`, `BBB+`, `AAA`) or a Moody's rating (`A3`, `Baa2`, `Aaa`) gives
/// the bucket. The notches (+/-, 1/2/3) are dropped: the steps are by letter grade. CC and
/// C go to CCC.
pub fn rating_bucket(rating: &str) -> Res<&'static str> {
    let s = rating.trim();
    let core = s.strip_suffix(['+', '-']).unwrap_or(s);
    match core {
        "AAA" => return Ok("AAA"),
        "AA" => return Ok("AA"),
        "A" => return Ok("A"),
        "BBB" => return Ok("BBB"),
        "BB" => return Ok("BB"),
        "B" => return Ok("B"),
        "CCC" | "CC" | "C" => return Ok("CCC"),
        _ => {}
    }
    let core = s.strip_suffix(['1', '2', '3']).unwrap_or(s);
    match core {
        "Aaa" => Ok("AAA"),
        "Aa" => Ok("AA"),
        "A" => Ok("A"),
        "Baa" => Ok("BBB"),
        "Ba" => Ok("BB"),
        "B" => Ok("B"),
        "Caa" | "Ca" | "C" => Ok("CCC"),
        _ => Err(format!("unrecognized rating '{rating}'; use S&P (A-, BBB+) or Moody's (A3, Baa2)")),
    }
}

fn bucket_note(rating: &str, bucket: &str) -> String {
    if rating.trim() == bucket {
        String::new()
    } else {
        format!("{} uses the {bucket} step", rating.trim())
    }
}

/// The latest (day, value) on or before `on`. It is an error when that value is older than
/// `max_age_days`, so a gap never gives an old number without a word.
pub fn latest(series: &BTreeMap<i64, f64>, on: i64, max_age_days: i64, label: &str) -> Res<(i64, f64)> {
    match series.range(..=on).next_back() {
        None => {
            let first = series.keys().next().map_or("None".to_string(), |d| iso(*d));
            Err(format!("{label}: no value on or before {} (data starts {first})", iso(on)))
        }
        Some((d, v)) => {
            if on - d > max_age_days {
                Err(format!("{label}: latest value on or before {} is from {}, more than {max_age_days} days old", iso(on), iso(*d)))
            } else {
                Ok((*d, *v))
            }
        }
    }
}

impl Rates {
    /// The full history of a FRED percent series as `{day: decimal}`, cached in
    /// `data/raw/bonds/`. A failed download gives the cache, with a warning.
    pub fn load_fred(&mut self, series_id: &str, refresh: bool) -> Res<BTreeMap<i64, f64>> {
        if let Some(s) = self.fred_memo.get(series_id) {
            return Ok(s.clone());
        }
        let path = cached_series_path(&self.cache, series_id);
        let read = |p: &std::path::Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
        let parsed = if path.exists() && !refresh && is_fresh_file(&path, 6.0) {
            parse_fred_csv(&read(&path)?, true)?
        } else {
            let fetched = self.fred_text(series_id, "1900-01-01").and_then(|t| {
                let header = t.trim_start_matches('\u{feff}').split('\n').next().unwrap_or("").trim().to_string();
                if header.split(',').next_back() != Some(series_id) {
                    let head: String = header.chars().take(60).collect();
                    return Err(format!("not a FRED CSV for {series_id} (header: '{head}')"));
                }
                parse_fred_csv(&t, true).map(|p| (t, p))
            });
            match fetched {
                Ok((t, p)) => {
                    save_cache(&path, &t)?;
                    p
                }
                Err(e) => {
                    if path.exists() {
                        eprintln!("fa: FRED download failed for {series_id} ({e}); using the cached copy");
                        parse_fred_csv(&read(&path)?, true)?
                    } else {
                        return Err(e);
                    }
                }
            }
        };
        self.fred_memo.insert(series_id.to_string(), parsed.clone());
        Ok(parsed)
    }

    /// The HQM spot rate of `years` for the latest month on or before `on`.
    pub fn hqm_spot(&mut self, years: i64, on: Option<i64>) -> Res<RateObservation> {
        let on = self.on_or_today(on);
        let months = crate::hqm::load_monthly(&self.cache)?;
        let s: BTreeMap<i64, f64> = months.iter().filter_map(|(d, m)| m.get(&years).map(|v| (*d, round6(v / 100.0)))).collect();
        let (d, v) = latest(&s, on, MONTHLY_MAX_AGE, &format!("HQM {years}Y"))?;
        Ok(observation(format!("HQM {years}Y spot"), d, v, HQM_SOURCE, "monthly average"))
    }

    /// The step of the bucket of the rating over the HQM reference bucket (a fixed table).
    pub fn rating_step(&mut self, rating: &str) -> Res<RateObservation> {
        let b = rating_bucket(rating)?;
        let day = mapcheck::dates::parse(STEP_DATE).ok_or("bad date")?;
        let note = if b == REFERENCE_BUCKET { format!("{b} is the HQM reference bucket") } else { bucket_note(rating, b) };
        Ok(observation(format!("{b} step over HQM ({REFERENCE_BUCKET})"), day, rating_step(b), STEP_SOURCE, note))
    }

    /// The cost of debt of the rating: HQM spot at `DEFAULT_MATURITY` + the step of its bucket.
    pub fn corporate_yield(&mut self, rating: &str, on: Option<i64>) -> Res<RateObservation> {
        let b = rating_bucket(rating)?;
        let h = self.hqm_spot(DEFAULT_MATURITY, on)?;
        let day = mapcheck::dates::parse(&h.date).ok_or("bad date")?;
        let note = if b == rating.trim() { String::new() } else { format!("{} uses the {b} step", rating.trim()) };
        Ok(observation(format!("HQM {DEFAULT_MATURITY}Y spot + {b} step"), day, round6(h.value + rating_step(b)), &format!("{HQM_SOURCE}; {STEP_SOURCE}"), note))
    }

    /// The cost of debt minus the UST 10Y on `on`: the engine's `kd_spread`.
    pub fn spread_over_10y(&mut self, rating: &str, on: Option<i64>) -> Res<RateObservation> {
        let y = self.corporate_yield(rating, on)?;
        let t = self.treasury_yield("10Y", on, false)?;
        let t_day = mapcheck::dates::parse(&t.date).ok_or("bad date")?;
        let mut notes: Vec<String> = Vec::new();
        if !y.note.is_empty() {
            notes.push(y.note.clone());
        }
        notes.push(format!("HQM month {}", y.date));
        let b = rating_bucket(rating)?;
        Ok(observation(format!("{b} cost of debt minus UST 10Y"), t_day, round6(y.value - t.value), &format!("{}; U.S. Treasury", y.source), notes.join("; ")))
    }

    /// The Treasury HQM corporate spot curve for the latest month on or before `on`.
    pub fn hqm_curve(&mut self, on: Option<i64>) -> Res<HqmCurve> {
        let on = self.on_or_today(on);
        // The Treasury files (`fa bonds hqm get`); the rates are in percent there.
        let months = crate::hqm::load_monthly(&self.cache)?;
        let mut series: Vec<(i64, BTreeMap<i64, f64>)> = Vec::new();
        for n in HQM_MATURITIES {
            series.push((n, months.iter().filter_map(|(d, m)| m.get(&n).map(|v| (*d, round6(v / 100.0)))).collect()));
        }
        let mut month = i64::MAX;
        for (n, s) in &series {
            month = month.min(latest(s, on, MONTHLY_MAX_AGE, &format!("HQM {n}Y"))?.0);
        }
        let maturities = series.iter().filter_map(|(n, s)| s.get(&month).map(|v| (format!("{n}Y"), *v))).collect();
        Ok(HqmCurve { date: iso(month), source: HQM_SOURCE.to_string(), maturities })
    }

    pub fn credit_snapshot(&mut self, on: Option<i64>) -> Res<CreditSnapshot> {
        let mut ratings = BTreeMap::new();
        for b in RATINGS {
            ratings.insert(
                b.to_string(),
                RatingQuote { step: rating_step(b), effective_yield: self.corporate_yield(b, on)?, spread_over_10y: self.spread_over_10y(b, on)? },
            );
        }
        Ok(CreditSnapshot { ratings, hqm_curve: self.hqm_curve(on)? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_age_limit_is_inclusive() {
        // Hand arithmetic: on = 20010, limit = 7 days. A value on day 20003 is 7 days old: kept (the rule is `> max`).
        let s: BTreeMap<i64, f64> = [(19990, 0.01), (20003, 0.02)].into_iter().collect();
        assert_eq!(latest(&s, 20010, 7, "x").unwrap(), (20003, 0.02));
        // One day over: 20002 is 8 days old, so the error names the limit; the older point is not used.
        let s: BTreeMap<i64, f64> = [(19990, 0.01), (20002, 0.03)].into_iter().collect();
        let e = latest(&s, 20010, 7, "x").unwrap_err();
        assert!(e.starts_with("x: latest value") && e.contains("more than 7 days old"), "{e}");
        // A value after `on` is ignored; the same day counts as age 0.
        let s: BTreeMap<i64, f64> = [(20010, 0.04), (20011, 0.05)].into_iter().collect();
        assert_eq!(latest(&s, 20010, 0, "x").unwrap(), (20010, 0.04));
        // Empty series: the "no value" error with `None` as the data start.
        let e = latest(&BTreeMap::new(), 20010, 7, "x").unwrap_err();
        assert!(e.contains("no value on or before") && e.contains("data starts None"), "{e}");
    }

    #[test]
    fn cached_series_ids_are_the_hqm_series_only() {
        let ids = cached_series_ids();
        assert_eq!(ids.len(), 8);
        assert_eq!(ids[0], "HQMCB1YR");
        assert_eq!(ids.last().unwrap(), "HQMCB30YR");
        assert!(ids.iter().all(|i| i.starts_with("HQMCB")), "no ICE and no Moody's id: {ids:?}");
    }

    #[test]
    fn the_step_of_the_reference_bucket_is_zero_and_the_others_follow_the_table() {
        // Hand arithmetic from the Damodaran table (percent): A 0.78 -> 0; AAA 0.40 - 0.78 = -0.38; BBB 1.11 - 0.78 = +0.33; CCC 8.85 - 0.78 = +8.07.
        assert_eq!(rating_step(REFERENCE_BUCKET), 0.0);
        assert_eq!(rating_step("AAA"), -0.0038);
        assert_eq!(rating_step("AA"), -0.0023);
        assert_eq!(rating_step("BBB"), 0.0033);
        assert_eq!(rating_step("BB"), 0.0106);
        assert_eq!(rating_step("B"), 0.0243);
        assert_eq!(rating_step("CCC"), 0.0807);
        // The steps rise with the risk.
        let v: Vec<f64> = RATINGS.iter().map(|b| rating_step(b)).collect();
        assert!(v.windows(2).all(|w| w[0] < w[1]), "{v:?}");
    }

    #[test]
    fn the_buckets_of_the_two_rating_scales_agree() {
        for (sp, mo) in [("AAA", "Aaa"), ("AA-", "Aa3"), ("A+", "A1"), ("A-", "A3"), ("BBB", "Baa2"), ("BB+", "Ba1"), ("B-", "B3"), ("CCC+", "Caa1"), ("CC", "Ca")] {
            assert_eq!(rating_bucket(sp).unwrap(), rating_bucket(mo).unwrap(), "{sp} {mo}");
        }
        assert!(rating_bucket("ZZZ").is_err());
    }
}
