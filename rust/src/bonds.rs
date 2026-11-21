//! US corporate bond yields and the cost of debt by rating, from public sources.
//!
//! The driver engine prices debt as `kd = rf + kd_spread`, with `rf` the Treasury 10Y. This
//! module gives a market value for `kd_spread` and the context around it.
//!
//! The cost of debt of a rating at a date D is:
//!
//! `kd = UST 10Y (D) + basis + step`, with `basis = HQM 10Y (month) - mean UST 10Y (the same month)`.
//!
//! - HQM: U.S. Treasury high quality market corporate bond yield curve, monthly average spot
//!   rates, 1 to 100 years (a blend of AAA, AA and A bonds, from the Treasury methodology papers that we did not read). Read from the Treasury files
//!   (`fa bonds hqm get`, module `hqm`). A month is used only from its publication day
//!   (`hqm_available`), never before: no look-ahead.
//! - Basis: the HQM level is a monthly average and the UST 10Y of D is a daily close. So the spread
//!   is formed inside one month (monthly average against monthly average), and then added to the
//!   close of D. The basis has a small spot-against-par part (about 10 bp at 10 years).
//! - Step: the default spread of the rating row minus the default spread of the HQM reference
//!   bucket (`REFERENCE_BUCKET`, step 0). The spreads are the table of A. Damodaran, "Ratings,
//!   Interest Coverage Ratios and Default Spread", data of January 2026 (spreads over the par
//!   T-bond), <https://pages.stern.nyu.edu/~adamodar/New_Home_Page/datafile/ratings.html>. They
//!   are facts written in this file, not a copy of a data file. Records: `docs/assumptions/A-200`
//!   to `A-203`; model document `docs/models/cost-of-debt.md`.
//!
//! The ICE BofA spreads and the Moody's Aaa/Baa yields are not used: their terms do not permit a
//! stored copy.
//!
//! `spread_over_10y` = basis + step. This is the engine's `kd_spread` (kd = rf_10Y + kd_spread),
//! so the verdict uses it.
//!
//! UNITS: decimals (0.0067 = 0.67%). The HQM rate is a spot rate (a zero rate), the UST 10Y is a
//! par yield.

use crate::rates::{is_fresh_file, iso, observation, parse_fred_csv, round6, save_cache, Rates, Res};
use crate::{CreditSnapshot, HqmCurve, RateObservation, RatingQuote};
use std::collections::BTreeMap;

/// The rating rows of the Damodaran table, from the best to the worst (S&P form). A row is the
/// key of `DEFAULT_SPREAD`. D (default) has a row in the table and is not used here.
pub const RATINGS: [&str; 14] = ["AAA", "AA", "A+", "A", "A-", "BBB", "BB+", "BB", "B+", "B", "B-", "CCC", "CC", "C"];
/// The rating bucket of the HQM curve. Its step is 0 (record A-201).
pub const REFERENCE_BUCKET: &str = "A";
/// The maturity (years) of the cost of debt, as the `rf` of the engine (record A-202).
pub const DEFAULT_MATURITY: i64 = 10;
pub const HQM_MATURITIES: [i64; 8] = [1, 2, 3, 5, 7, 10, 20, 30];

/// Default spread over the risk-free rate by rating row (decimals): Damodaran, January 2026, table
/// for large non-financial firms (Aaa/AAA 0.40%, Aa2/AA 0.55%, A1/A+ 0.70%, A2/A 0.78%, A3/A- 0.89%,
/// Baa2/BBB 1.11%, Ba1/BB+ 1.38%, Ba2/BB 1.84%, B1/B+ 2.75%, B2/B 3.21%, B3/B- 5.09%, Caa/CCC 8.85%,
/// Ca2/CC 12.61%, C2/C 16.00%).
const DEFAULT_SPREAD: [(&str, f64); 14] = [
    ("AAA", 0.0040),
    ("AA", 0.0055),
    ("A+", 0.0070),
    ("A", 0.0078),
    ("A-", 0.0089),
    ("BBB", 0.0111),
    ("BB+", 0.0138),
    ("BB", 0.0184),
    ("B+", 0.0275),
    ("B", 0.0321),
    ("B-", 0.0509),
    ("CCC", 0.0885),
    ("CC", 0.1261),
    ("C", 0.1600),
];

pub const HQM_SOURCE: &str = crate::hqm::SOURCE;
pub const STEP_SOURCE: &str = "Damodaran default spreads, Jan 2026";
/// The date of the table: the ratings page says "as of January 2026"; the data page says the update was on 2026-01-09.
pub const STEP_DATE: &str = "2026-01-09";

/// Days: the staleness cap. The newest published month must end at most this long before the date;
/// an older month gives an error. It is not the publication lag (`hqm_available`).
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

/// The Damodaran row of a rating, and whether the table has a row for exactly this rating. An S&P or
/// Fitch rating (`A-`, `BBB+`, `AAA`) or a Moody's rating (`A3`, `Baa2`, `Aaa`) is read. The table has
/// a row for 14 notches (`RATINGS`). A notch with no row (AA+, AA-, BBB+, BBB-, BB-, CCC+, Moody's
/// Aa1, Aa3, Baa1, Baa3, Ba3, Caa1 to Caa3) takes the row of its letter grade (AA, BBB, BB, CCC). A
/// default rating (D, SD) has no cost of debt.
fn rating_row(rating: &str) -> Res<(&'static str, bool)> {
    let s = rating.trim();
    // S&P and Fitch.
    let sp: Option<&'static str> = match s {
        "AAA" => Some("AAA"),
        "AA+" | "AA" | "AA-" => Some("AA"),
        "A+" => Some("A+"),
        "A" => Some("A"),
        "A-" => Some("A-"),
        "BBB+" | "BBB" | "BBB-" => Some("BBB"),
        "BB+" => Some("BB+"),
        "BB" | "BB-" => Some("BB"),
        "B+" => Some("B+"),
        "B" => Some("B"),
        "B-" => Some("B-"),
        "CCC+" | "CCC" | "CCC-" => Some("CCC"),
        "CC" => Some("CC"),
        "C" => Some("C"),
        _ => None,
    };
    if let Some(row) = sp {
        return Ok((row, row == s));
    }
    // Moody's. The table names these ratings: Aaa, Aa2, A1, A2, A3, Baa2, Ba1, Ba2, B1, B2, B3, Caa, Ca.
    let row: &'static str = match s {
        "Aaa" => "AAA",
        "Aa1" | "Aa2" | "Aa3" => "AA",
        "A1" => "A+",
        "A2" => "A",
        "A3" => "A-",
        "Baa1" | "Baa2" | "Baa3" => "BBB",
        "Ba1" => "BB+",
        "Ba2" | "Ba3" => "BB",
        "B1" => "B+",
        "B2" => "B",
        "B3" => "B-",
        "Caa" | "Caa1" | "Caa2" | "Caa3" => "CCC",
        "Ca" => "CC",
        "D" | "SD" | "RD" => return Err(format!("rating '{rating}' is a default: no cost of debt")),
        _ => return Err(format!("unrecognized rating '{rating}'; use S&P (A-, BBB+) or Moody's (A3, Baa2)")),
    };
    Ok((row, ["Aaa", "Aa2", "A1", "A2", "A3", "Baa2", "Ba1", "Ba2", "B1", "B2", "B3", "Caa", "Ca"].contains(&s)))
}

/// The rating row (a key of `RATINGS`) of a rating.
pub fn rating_bucket(rating: &str) -> Res<&'static str> {
    rating_row(rating).map(|(r, _)| r)
}

fn bucket_note(rating: &str, bucket: &str) -> String {
    match rating_row(rating) {
        Ok((_, true)) | Err(_) => String::new(),
        Ok(_) => format!("{} has no row in the table; uses the {bucket} row", rating.trim()),
    }
}

/// The first day on which the HQM average of the month that starts on `month_first` is used: the first
/// weekday after the 5th of the next month. Evidence (2026-11-21): the file `hqm_24_28.xls` with October
/// 2026 as its last month has `Last-Modified: Wed, 04 Nov 2026 21:38:48 GMT`. Treasury publishes no
/// schedule, so this is a rule with a margin of two days over that one observation. A bank holiday is
/// not counted. Before this day the average does not exist for a person: using it is look-ahead.
pub fn hqm_available(month_first: i64) -> i64 {
    let (y, m) = civil_ym(month_first);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let mut d = mapcheck::dates::days_from_civil(ny, nm, 1) + 5;
    // 1970-01-01 (day 0) was a Thursday: weekday 0 = Sunday.
    while matches!((d + 4).rem_euclid(7), 0 | 6) {
        d += 1;
    }
    d
}

fn civil_ym(day: i64) -> (i64, i64) {
    let t = mapcheck::dates::format(day);
    (t[0..4].parse().unwrap_or(0), t[5..7].parse().unwrap_or(1))
}

/// The last day of the month that starts on `month_first`.
fn month_end(month_first: i64) -> i64 {
    let (y, m) = civil_ym(month_first);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    mapcheck::dates::days_from_civil(ny, nm, 1) - 1
}

/// The HQM month for `on`: the latest month that is published on or before `on` (`hqm_available`) and
/// whose end is at most `MONTHLY_MAX_AGE` days before `on` (a staleness cap, not the lag). Gives the
/// first day of the month and the rate in decimals. A month after `on` changes nothing.
pub fn pick_month(months: &BTreeMap<i64, BTreeMap<i64, f64>>, years: i64, on: i64) -> Res<(i64, f64)> {
    let by_end: BTreeMap<i64, f64> =
        months.iter().filter(|(d, _)| hqm_available(**d) <= on).filter_map(|(d, m)| m.get(&years).map(|v| (month_end(*d), round6(v / 100.0)))).collect();
    let (end, v) = latest(&by_end, on, MONTHLY_MAX_AGE, &format!("HQM {years}Y")).map_err(|e| format!("{e}; run `fa bonds hqm get` (a month is used from the first weekday after the 5th of the next month)"))?;
    let (y, m) = civil_ym(end);
    Ok((mapcheck::dates::days_from_civil(y, m, 1), v))
}

/// The HQM month and the mean UST 10Y of the same month, for the cost of debt.
#[derive(Debug, Clone, PartialEq)]
pub struct HqmBasis {
    /// First day of the HQM month.
    pub month: i64,
    pub hqm: f64,
    pub ust_mean: f64,
    /// `hqm - ust_mean`, decimals.
    pub basis: f64,
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

/// The cost of debt observation from the UST 10Y of the date, the basis and the rating.
fn cost_of_debt(rating: &str, t: &RateObservation, basis: &HqmBasis) -> Res<RateObservation> {
    let b = rating_bucket(rating)?;
    let t_day = mapcheck::dates::parse(&t.date).ok_or("bad date")?;
    let mut notes = vec![bucket_note(rating, b), format!("HQM month {}", iso(basis.month))];
    notes.retain(|n| !n.is_empty());
    Ok(observation(
        format!("UST 10Y + HQM basis + {b} step"),
        t_day,
        round6(t.value + basis.basis + rating_step(b)),
        &format!("U.S. Treasury; {HQM_SOURCE}; {STEP_SOURCE}"),
        notes.join("; "),
    ))
}

/// The `kd_spread` observation: basis + step.
fn spread_of(rating: &str, t: &RateObservation, basis: &HqmBasis) -> Res<RateObservation> {
    let y = cost_of_debt(rating, t, basis)?;
    let b = rating_bucket(rating)?;
    let t_day = mapcheck::dates::parse(&t.date).ok_or("bad date")?;
    Ok(observation(format!("{b} cost of debt minus UST 10Y"), t_day, round6(y.value - t.value), &y.source, y.note.clone()))
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

    /// The HQM spot rate of `years` for the latest published month on or before `on`.
    pub fn hqm_spot(&mut self, years: i64, on: Option<i64>) -> Res<RateObservation> {
        let on = self.on_or_today(on);
        let months = crate::hqm::load_monthly(&self.cache)?;
        let (d, v) = pick_month(&months, years, on)?;
        Ok(observation(format!("HQM {years}Y spot"), d, v, HQM_SOURCE, "monthly average"))
    }

    /// The basis of the cost of debt at `on`: the HQM 10Y month minus the mean UST 10Y of the same month.
    pub fn hqm_basis(&mut self, on: Option<i64>) -> Res<HqmBasis> {
        let h = self.hqm_spot(DEFAULT_MATURITY, on)?;
        let month = mapcheck::dates::parse(&h.date).ok_or("bad date")?;
        let days = self.treasury_history("10Y", Some(month), Some(month_end(month)), false)?;
        if days.len() < 10 {
            return Err(format!("UST 10Y: only {} days in the month {} (the Treasury file starts in 1990)", days.len(), h.date));
        }
        let ust_mean = round6(days.values().sum::<f64>() / days.len() as f64);
        Ok(HqmBasis { month, hqm: h.value, ust_mean, basis: round6(h.value - ust_mean) })
    }

    /// The step of the bucket of the rating over the HQM reference bucket (a fixed table).
    pub fn rating_step(&mut self, rating: &str) -> Res<RateObservation> {
        let b = rating_bucket(rating)?;
        let day = mapcheck::dates::parse(STEP_DATE).ok_or("bad date")?;
        let note = if b == REFERENCE_BUCKET { format!("{b} is the HQM reference bucket") } else { bucket_note(rating, b) };
        Ok(observation(format!("{b} step over HQM ({REFERENCE_BUCKET})"), day, rating_step(b), STEP_SOURCE, note))
    }

    /// The cost of debt of the rating: UST 10Y of `on` + basis (HQM 10Y minus the UST 10Y mean of
    /// the same month) + the step of its row.
    pub fn corporate_yield(&mut self, rating: &str, on: Option<i64>) -> Res<RateObservation> {
        let basis = self.hqm_basis(on)?;
        let t = self.treasury_yield("10Y", on, false)?;
        cost_of_debt(rating, &t, &basis)
    }

    /// The `kd_spread` of the engine: basis + step. The cost of debt is the UST 10Y of `on` plus this.
    pub fn spread_over_10y(&mut self, rating: &str, on: Option<i64>) -> Res<RateObservation> {
        let basis = self.hqm_basis(on)?;
        let t = self.treasury_yield("10Y", on, false)?;
        spread_of(rating, &t, &basis)
    }

    /// The Treasury HQM corporate spot curve for the latest published month on or before `on`.
    pub fn hqm_curve(&mut self, on: Option<i64>) -> Res<HqmCurve> {
        let on = self.on_or_today(on);
        // The Treasury files (`fa bonds hqm get`); the rates are in percent there.
        let months = crate::hqm::load_monthly(&self.cache)?;
        let mut month = i64::MAX;
        for n in HQM_MATURITIES {
            month = month.min(pick_month(&months, n, on)?.0);
        }
        let maturities = HQM_MATURITIES.iter().filter_map(|n| months.get(&month).and_then(|m| m.get(n)).map(|v| (format!("{n}Y"), round6(v / 100.0)))).collect();
        Ok(HqmCurve { date: iso(month), source: HQM_SOURCE.to_string(), maturities })
    }

    pub fn credit_snapshot(&mut self, on: Option<i64>) -> Res<CreditSnapshot> {
        let basis = self.hqm_basis(on)?;
        let t = self.treasury_yield("10Y", on, false)?;
        let mut ratings = BTreeMap::new();
        for b in RATINGS {
            ratings.insert(b.to_string(), RatingQuote { step: rating_step(b), effective_yield: cost_of_debt(b, &t, &basis)?, spread_over_10y: spread_of(b, &t, &basis)? });
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
        // Hand arithmetic from the Damodaran table (percent), less A = 0.78: AAA 0.40 -> -0.38; A- 0.89 -> +0.11;
        // BBB 1.11 -> +0.33; BB+ 1.38 -> +0.60; B- 5.09 -> +4.31; CCC 8.85 -> +8.07.
        assert_eq!(rating_step(REFERENCE_BUCKET), 0.0);
        assert_eq!(rating_step("AAA"), -0.0038);
        assert_eq!(rating_step("AA"), -0.0023);
        assert_eq!(rating_step("A+"), -0.0008);
        assert_eq!(rating_step("A-"), 0.0011);
        assert_eq!(rating_step("BBB"), 0.0033);
        assert_eq!(rating_step("BB+"), 0.0060);
        assert_eq!(rating_step("BB"), 0.0106);
        assert_eq!(rating_step("B-"), 0.0431);
        assert_eq!(rating_step("CCC"), 0.0807);
        // The steps rise with the risk.
        let v: Vec<f64> = RATINGS.iter().map(|b| rating_step(b)).collect();
        assert!(v.windows(2).all(|w| w[0] < w[1]), "{v:?}");
    }

    #[test]
    fn the_notches_use_their_own_row_and_the_missing_rows_use_the_letter_grade() {
        // (rating, row, the table has a row for exactly this rating, step)
        for (r, row, own, step) in [
            ("A3", "A-", true, 0.0011),
            ("A-", "A-", true, 0.0011),
            ("A1", "A+", true, -0.0008),
            ("Baa3", "BBB", false, 0.0033),
            ("BBB-", "BBB", false, 0.0033),
            ("BBB+", "BBB", false, 0.0033),
            ("Baa2", "BBB", true, 0.0033),
            ("Caa1", "CCC", false, 0.0807),
            ("CCC+", "CCC", false, 0.0807),
            ("BB+", "BB+", true, 0.0060),
            ("Ba1", "BB+", true, 0.0060),
            ("BB-", "BB", false, 0.0106),
            ("B3", "B-", true, 0.0431),
            ("AA-", "AA", false, -0.0023),
            ("Aaa", "AAA", true, -0.0038),
        ] {
            assert_eq!(rating_bucket(r).unwrap(), row, "{r}");
            assert_eq!(rating_row(r).unwrap().1, own, "{r}");
            assert_eq!(rating_step(rating_bucket(r).unwrap()), step, "{r}");
            assert_eq!(bucket_note(r, row).is_empty(), own, "{r}");
        }
        assert!(rating_bucket("ZZZ").is_err());
        assert!(rating_bucket("D").unwrap_err().contains("default"));
    }

    #[test]
    fn the_buckets_of_the_two_rating_scales_agree() {
        for (sp, mo) in [("AAA", "Aaa"), ("AA-", "Aa3"), ("A+", "A1"), ("A-", "A3"), ("BBB", "Baa2"), ("BB+", "Ba1"), ("B-", "B3"), ("CCC+", "Caa1"), ("CC", "Ca")] {
            assert_eq!(rating_bucket(sp).unwrap(), rating_bucket(mo).unwrap(), "{sp} {mo}");
        }
    }

    #[test]
    fn a_month_is_published_on_the_first_weekday_after_the_5th_of_the_next_month() {
        let d = |t: &str| mapcheck::dates::parse(t).unwrap();
        // October 2026: the 5th of November is a Thursday... 2026-11-05 is a Thursday, so the day is Friday 2026-11-06.
        assert_eq!(hqm_available(d("2026-10-01")), d("2026-11-06"));
        // The evidence (Last-Modified 2026-11-04) is before the rule day: the rule has a margin.
        assert!(d("2026-11-04") < hqm_available(d("2026-10-01")));
        // December 2026 -> 2027-01-05 is a Tuesday: 2027-01-06. November 2026 -> 2026-12-05 is a Saturday: Monday 2026-12-07.
        assert_eq!(hqm_available(d("2026-12-01")), d("2027-01-06"));
        assert_eq!(hqm_available(d("2026-11-01")), d("2026-12-07"));
        assert_eq!(month_end(d("2026-02-10")), d("2026-02-28"));
    }

    fn months() -> BTreeMap<i64, BTreeMap<i64, f64>> {
        let d = |t: &str| mapcheck::dates::parse(t).unwrap();
        [("2026-08-01", 5.00), ("2026-09-01", 5.10), ("2026-10-01", 5.20)].iter().map(|(t, v)| (d(t), [(10, *v)].into_iter().collect())).collect()
    }

    #[test]
    fn a_month_is_not_used_before_it_is_published_and_a_later_month_changes_nothing() {
        let d = |t: &str| mapcheck::dates::parse(t).unwrap();
        let m = months();
        // On 2026-10-15 the October average does not exist: September is the month (published 2026-10-06).
        assert_eq!(pick_month(&m, 10, d("2026-10-15")).unwrap(), (d("2026-09-01"), 0.051));
        // The day before the publication day of September (2026-10-06) it is August.
        assert_eq!(pick_month(&m, 10, d("2026-10-05")).unwrap(), (d("2026-08-01"), 0.05));
        // On 2026-11-06 October is used.
        assert_eq!(pick_month(&m, 10, d("2026-11-06")).unwrap(), (d("2026-10-01"), 0.052));
        // Truncation (poison) test: an absurd later month changes nothing at an earlier date.
        let mut poisoned = m.clone();
        poisoned.insert(d("2026-11-01"), [(10, 99.0)].into_iter().collect());
        poisoned.insert(d("2026-12-01"), [(10, 98.0)].into_iter().collect());
        for on in ["2026-10-05", "2026-10-15", "2026-11-06", "2026-11-30", "2026-12-06"] {
            assert_eq!(pick_month(&poisoned, 10, d(on)).unwrap(), pick_month(&m, 10, d(on)).unwrap(), "{on}");
        }
        // The poison month itself is used from its day (2026-12-07).
        assert_eq!(pick_month(&poisoned, 10, d("2026-12-07")).unwrap().1, 0.99);
    }

    #[test]
    fn the_staleness_cap_is_not_the_lag() {
        let d = |t: &str| mapcheck::dates::parse(t).unwrap();
        let m = months();
        // October ends on 2026-10-31; 70 days later is 2027-01-09: still used. One day more is an error that names the command.
        assert_eq!(pick_month(&m, 10, d("2027-01-09")).unwrap().0, d("2026-10-01"));
        let e = pick_month(&m, 10, d("2027-01-10")).unwrap_err();
        assert!(e.contains("more than 70 days old") && e.contains("fa bonds hqm get"), "{e}");
        // Before the first published month: an error too.
        assert!(pick_month(&m, 10, d("2026-08-31")).unwrap_err().contains("no value"));
    }
}
