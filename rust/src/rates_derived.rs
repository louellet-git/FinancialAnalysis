//! Series that FRED computes from the Treasury curves, computed here from the curves of the
//! Treasury itself (public domain): T10Y3M, DFII10, T10YIE. The output has the form of the
//! FRED files: header `observation_date,<ID>`, one line `YYYY-MM-DD,<percent, 2 places>`.
//! A day without a value is not written (FRED writes an empty cell for such a weekday).

use crate::rates::{iso, CurveByDay, Rates, Res};
use std::collections::BTreeMap;
use std::path::Path;

pub const DERIVED_IDS: [&str; 3] = ["T10Y3M", "DFII10", "T10YIE"];

/// A yield in decimal as whole hundredths of a percent (the Treasury publishes 2 places).
fn hundredths(v: f64) -> i64 {
    (v * 10_000.0).round() as i64
}

/// `{id: {day: value in percent}}`. The difference is made on the published 2-place values, so
/// it is exact to 2 places. T10Y3M: 10Y minus 3M nominal. DFII10: 10Y real. T10YIE: 10Y
/// nominal minus 10Y real. A day needs both tenors; on other days the series has no value.
pub fn derived_series(nominal: &CurveByDay, real: &CurveByDay) -> BTreeMap<&'static str, BTreeMap<i64, f64>> {
    let pct = |h: i64| h as f64 / 100.0;
    let mut out: BTreeMap<&'static str, BTreeMap<i64, f64>> = DERIVED_IDS.iter().map(|id| (*id, BTreeMap::new())).collect();
    for (day, c) in nominal {
        if let (Some(t10), Some(t3m)) = (c.get("10Y"), c.get("3M")) {
            out.get_mut("T10Y3M").unwrap().insert(*day, pct(hundredths(*t10) - hundredths(*t3m)));
        }
    }
    for (day, r) in real {
        let Some(r10) = r.get("10Y") else { continue };
        out.get_mut("DFII10").unwrap().insert(*day, pct(hundredths(*r10)));
        if let Some(n10) = nominal.get(day).and_then(|c| c.get("10Y")) {
            out.get_mut("T10YIE").unwrap().insert(*day, pct(hundredths(*n10) - hundredths(*r10)));
        }
    }
    out
}

/// The text of one series file in the FRED form.
pub fn fred_form_csv(id: &str, series: &BTreeMap<i64, f64>) -> String {
    let mut s = format!("observation_date,{id}\n");
    for (d, v) in series {
        s.push_str(&format!("{},{v:.2}\n", iso(*d)));
    }
    s
}

/// Compare with a FRED file text on the days in both: `(days in both, days equal, days that differ
/// by 0.01 only, largest difference, days that differ by more)`.
#[derive(Debug, PartialEq)]
pub struct DerivedCompare {
    pub both: usize,
    pub equal: usize,
    pub off_by_one_hundredth: usize,
    pub larger: usize,
    pub max_diff: f64,
    pub first_ours: Option<String>,
    pub first_fred: Option<String>,
}

pub fn compare_with_fred(ours: &BTreeMap<i64, f64>, fred_text: &str) -> Res<DerivedCompare> {
    let fred = crate::rates::parse_fred_csv(fred_text, false)?;
    let mut r = DerivedCompare {
        both: 0,
        equal: 0,
        off_by_one_hundredth: 0,
        larger: 0,
        max_diff: 0.0,
        first_ours: ours.keys().next().map(|d| iso(*d)),
        first_fred: fred.keys().next().map(|d| iso(*d)),
    };
    for (d, o) in ours {
        let Some(f) = fred.get(d) else { continue };
        r.both += 1;
        let diff = hundredths(((o - f) / 100.0).abs());
        r.max_diff = r.max_diff.max(diff as f64 / 100.0);
        match diff {
            0 => r.equal += 1,
            1 => r.off_by_one_hundredth += 1,
            _ => r.larger += 1,
        }
    }
    Ok(r)
}

impl Rates {
    /// The three series for the years `from_year..=to_year` (a year not in the cache is loaded from the Treasury).
    pub fn derived(&mut self, from_year: i64, to_year: i64) -> Res<BTreeMap<&'static str, BTreeMap<i64, f64>>> {
        let (mut nom, mut real) = (CurveByDay::new(), CurveByDay::new());
        for y in from_year.max(crate::rates::treasury_first_year(false))..=to_year {
            nom.extend(self.load_treasury_curve(y, false, false)?);
        }
        for y in from_year.max(crate::rates::treasury_first_year(true))..=to_year {
            real.extend(self.load_treasury_curve(y, true, false)?);
        }
        Ok(derived_series(&nom, &real))
    }

    /// Write one file `<ID>.csv` for each series into `dir`. Gives the row counts.
    pub fn write_derived(&mut self, dir: &Path, from_year: i64, to_year: i64) -> Res<Vec<(String, usize)>> {
        let all = self.derived(from_year, to_year)?;
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut counts = Vec::new();
        for (id, s) in &all {
            let p = dir.join(format!("{id}.csv"));
            std::fs::write(&p, fred_form_csv(id, s)).map_err(|e| format!("{}: {e}", p.display()))?;
            counts.push((id.to_string(), s.len()));
        }
        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapcheck::dates;

    fn curve(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn computes_the_three_series_with_fixed_answers() {
        let d1 = dates::parse("2025-12-31").unwrap();
        let d2 = dates::parse("2025-12-30").unwrap();
        let mut nom = CurveByDay::new();
        nom.insert(d1, curve(&[("3M", 0.0367), ("10Y", 0.0418)]));
        nom.insert(d2, curve(&[("10Y", 0.0414)])); // no 3M: no T10Y3M
        let mut real = CurveByDay::new();
        real.insert(d1, curve(&[("10Y", 0.0193)]));
        real.insert(d2, curve(&[("10Y", 0.0190)]));
        let s = derived_series(&nom, &real);
        assert_eq!(s["T10Y3M"].len(), 1);
        assert_eq!(s["T10Y3M"][&d1], 0.51);
        assert_eq!(s["DFII10"][&d1], 1.93);
        assert_eq!(s["DFII10"][&d2], 1.90);
        assert_eq!(s["T10YIE"][&d1], 2.25);
        assert_eq!(s["T10YIE"][&d2], 2.24);
    }

    #[test]
    fn csv_has_the_fred_form_and_compare_counts() {
        let d1 = dates::parse("2025-12-30").unwrap();
        let d2 = dates::parse("2025-12-31").unwrap();
        let s: BTreeMap<i64, f64> = [(d1, 2.24), (d2, 2.25)].into_iter().collect();
        assert_eq!(fred_form_csv("T10YIE", &s), "observation_date,T10YIE\n2025-12-30,2.24\n2025-12-31,2.25\n");
        let fred = "observation_date,T10YIE\n2025-12-29,2.20\n2025-12-30,2.24\n2025-12-31,2.26\n2026-01-01,\n";
        let c = compare_with_fred(&s, fred).unwrap();
        assert_eq!((c.both, c.equal, c.off_by_one_hundredth, c.larger), (2, 1, 1, 0));
        assert_eq!(c.max_diff, 0.01);
        assert_eq!(c.first_ours.as_deref(), Some("2025-12-30"));
        assert_eq!(c.first_fred.as_deref(), Some("2025-12-29"));
    }

    #[test]
    fn real_treasury_fixture_gives_the_fred_values() {
        // The files of tests/fixtures/rates have on 2025-12-31: 3M 3.67, 10Y 4.18, 10Y real 1.90 (the store file of
        // today has 1.93 for the real 10Y on this day: the fixture is an older copy).
        let read = |n: &str| std::fs::read_to_string(format!("{}/tests/fixtures/rates/{n}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let nom = crate::rates::parse_treasury_csv(&read("treasury_nominal_2025.csv")).unwrap();
        let real = crate::rates::parse_treasury_csv(&read("treasury_real_2025.csv")).unwrap();
        let s = derived_series(&nom, &real);
        let d = dates::parse("2025-12-31").unwrap();
        assert_eq!((s["T10Y3M"][&d], s["DFII10"][&d], s["T10YIE"][&d]), (0.51, 1.90, 2.28));
        assert_eq!(s["DFII10"].len(), real.len());
    }
}
