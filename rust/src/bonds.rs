//! US corporate bond yields and credit spreads from free, keyless sources (the Rust form of
//! `financial_analysis/bonds.py`).
//!
//! The driver engine prices debt as `kd = rf + kd_spread`, with `rf` the Treasury 10Y. This
//! module gives market values for `kd_spread` and the context around them.
//!
//! All sources come through the FRED public CSV (no key):
//!
//! - ICE BofA US corporate indexes by rating (AAA, AA, A, BBB, BB, B, CCC and lower):
//!   option-adjusted spread (OAS) and effective yield, daily. FRED keeps only the last three
//!   years (ICE licence), and ICE does not allow redistribution: the cache is local only
//!   (`data/` is not in git).
//! - Moody's seasoned Aaa and Baa corporate bond yields (20 years or more to maturity).
//! - U.S. Treasury HQM corporate bond yield curve: monthly average spot rates, 1 to 100 years,
//!   a blend of AAA, AA and A bonds (FRED `HQMCB<n>YR` has the same numbers).
//!
//! Two spread measures:
//!
//! - `spread_over_10y` = index effective yield - UST 10Y on the same date. This is the
//!   engine's `kd_spread` (kd = rf_10Y + kd_spread), so the verdict uses it.
//! - OAS = spread over the Treasury curve at each bond's own maturity. It differs from
//!   `spread_over_10y` by the slope of the curve between the index duration and 10 years.
//!
//! UNITS: decimals (0.0067 = 0.67%). The cache is `data/raw/bonds/fred_<ID>.csv`.

use crate::rates::{compare, is_fresh_file, iso, observation, parse_fred_csv, round6, save_cache, Rates, Res};
use crate::{CheckResult, CreditSnapshot, HqmCurve, RateObservation, RatingQuote};
use std::collections::BTreeMap;

pub const RATINGS: [&str; 7] = ["AAA", "AA", "A", "BBB", "BB", "B", "CCC"];
const ICE_OAS: [(&str, &str); 7] = [
    ("AAA", "BAMLC0A1CAAA"),
    ("AA", "BAMLC0A2CAA"),
    ("A", "BAMLC0A3CA"),
    ("BBB", "BAMLC0A4CBBB"),
    ("BB", "BAMLH0A1HYBB"),
    ("B", "BAMLH0A2HYB"),
    ("CCC", "BAMLH0A3HYC"),
];
const MOODYS: [(&str, &str); 2] = [("Aaa", "DAAA"), ("Baa", "DBAA")];
/// FRED: Moody's yield minus DGS10.
const MOODYS_SPREAD_10Y: [(&str, &str); 2] = [("Aaa", "AAA10Y"), ("Baa", "BAA10Y")];
pub const HQM_MATURITIES: [i64; 8] = [1, 2, 3, 5, 7, 10, 20, 30];

pub const ICE_SOURCE: &str = "ICE BofA via FRED";
pub const MOODYS_SOURCE: &str = "Moody's via FRED";
pub const HQM_SOURCE: &str = "U.S. Treasury HQM via FRED";

/// Days: weekends, holidays and the one-day FRED lag.
const DAILY_MAX_AGE: i64 = 7;
/// Days: a monthly average has the date of the 1st and is published after the month end.
const MONTHLY_MAX_AGE: i64 = 70;

fn ice_oas_id(bucket: &str) -> &'static str {
    ICE_OAS.iter().find(|(b, _)| *b == bucket).map(|(_, id)| *id).unwrap_or("")
}

fn ice_yield_id(bucket: &str) -> String {
    format!("{}EY", ice_oas_id(bucket))
}

/// An S&P or Fitch rating (`A-`, `BBB+`, `AAA`) or a Moody's rating (`A3`, `Baa2`, `Aaa`) gives
/// the ICE index. The notches (+/-, 1/2/3) are dropped: ICE indexes are by letter grade. CC and
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
        format!("{} uses the {bucket} index", rating.trim())
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
        let path = self.cache.join("bonds").join(format!("fred_{series_id}.csv"));
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

    /// ICE BofA option-adjusted spread for the index of the rating.
    pub fn credit_spread(&mut self, rating: &str, on: Option<i64>) -> Res<RateObservation> {
        let b = rating_bucket(rating)?;
        let on = self.on_or_today(on);
        let s = self.load_fred(ice_oas_id(b), false)?;
        let (d, v) = latest(&s, on, DAILY_MAX_AGE, &format!("ICE {b} OAS"))?;
        Ok(observation(format!("ICE BofA {b} OAS"), d, v, ICE_SOURCE, bucket_note(rating, b)))
    }

    /// ICE BofA effective yield for the index of the rating.
    pub fn corporate_yield(&mut self, rating: &str, on: Option<i64>) -> Res<RateObservation> {
        let b = rating_bucket(rating)?;
        let on = self.on_or_today(on);
        let s = self.load_fred(&ice_yield_id(b), false)?;
        let (d, v) = latest(&s, on, DAILY_MAX_AGE, &format!("ICE {b} yield"))?;
        Ok(observation(format!("ICE BofA {b} effective yield"), d, v, ICE_SOURCE, bucket_note(rating, b)))
    }

    /// Index effective yield minus the UST 10Y on the same date: the engine's `kd_spread`.
    pub fn spread_over_10y(&mut self, rating: &str, on: Option<i64>) -> Res<RateObservation> {
        let y = self.corporate_yield(rating, on)?;
        let y_day = mapcheck::dates::parse(&y.date).ok_or("bad date")?;
        let t = self.treasury_yield("10Y", Some(y_day), false)?;
        let mut notes: Vec<String> = Vec::new();
        if !y.note.is_empty() {
            notes.push(y.note.clone());
        }
        if t.date != y.date {
            notes.push(format!("UST 10Y from {}", t.date));
        }
        let b = rating_bucket(rating)?;
        Ok(observation(format!("{b} yield minus UST 10Y"), y_day, round6(y.value - t.value), &format!("{ICE_SOURCE}; U.S. Treasury"), notes.join("; ")))
    }

    /// Moody's seasoned corporate bond yield, "Aaa" or "Baa".
    pub fn moodys_yield(&mut self, grade: &str, on: Option<i64>) -> Res<RateObservation> {
        let id = MOODYS.iter().find(|(g, _)| *g == grade).map(|(_, id)| *id).ok_or("grade must be one of ['Aaa', 'Baa']")?;
        let on = self.on_or_today(on);
        let s = self.load_fred(id, false)?;
        let (d, v) = latest(&s, on, DAILY_MAX_AGE, &format!("Moody's {grade}"))?;
        Ok(observation(format!("Moody's seasoned {grade}"), d, v, MOODYS_SOURCE, ""))
    }

    /// The Treasury HQM corporate spot curve for the latest month on or before `on`.
    pub fn hqm_curve(&mut self, on: Option<i64>) -> Res<HqmCurve> {
        let on = self.on_or_today(on);
        let mut series = Vec::new();
        for n in HQM_MATURITIES {
            series.push((n, self.load_fred(&format!("HQMCB{n}YR"), false)?));
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
                RatingQuote { oas: self.credit_spread(b, on)?, effective_yield: self.corporate_yield(b, on)?, spread_over_10y: self.spread_over_10y(b, on)? },
            );
        }
        let mut moodys = BTreeMap::new();
        for (g, _) in MOODYS {
            moodys.insert(g.to_string(), self.moodys_yield(g, on)?);
        }
        Ok(CreditSnapshot { ratings, moodys, hqm_curve: self.hqm_curve(on)? })
    }

    /// Moody's yield minus the Treasury 10Y (our join) against FRED AAA10Y and BAA10Y, which FRED
    /// calculates from DGS10. Both inputs have 2 decimals, so they must agree exactly.
    pub fn cross_check_bonds(&mut self, days: i64) -> Res<Vec<CheckResult>> {
        let start = self.today - days;
        let ust = self.treasury_history("10Y", Some(start), None, false)?;
        let mut out = Vec::new();
        for (grade, fid) in MOODYS_SPREAD_10Y {
            let id = MOODYS.iter().find(|(g, _)| *g == grade).map(|(_, id)| *id).unwrap_or("");
            let y = self.load_fred(id, false)?;
            let ours: BTreeMap<i64, f64> = ust.iter().filter_map(|(d, u)| y.get(d).map(|v| (*d, round6(v - u)))).collect();
            let fred: BTreeMap<i64, f64> = self.load_fred(fid, false)?.into_iter().filter(|(d, _)| *d >= start).collect();
            out.push(compare(&format!("Moody's {grade} - UST 10Y"), &ours, &fred, fid, 0.5e-4 + 1e-9));
        }
        Ok(out)
    }
}
