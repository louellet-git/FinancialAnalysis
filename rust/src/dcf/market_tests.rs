use super::*;

/// FRED series made from the CPI of the synthetic file, times `k`.
fn fred_scaled(obs: &[Obs], k: f64) -> Fred {
    obs.iter().map(|o| ((o.year, o.month), o.cpi * k)).collect()
}

/// `compute` with a FRED series equal to the Shiller CPI.
fn compute(obs: &[Obs], l: f64, n: usize) -> Vec<MarketRow> {
    super::compute(obs, Some(&fred_scaled(obs, 1.0)), l, n).unwrap()
}


/// A synthetic file, 1871-01 to 1950-12: dividend grows by `gd` a year (annual rate, one value for each year), the CPI
/// grows by 0.2% a month, gs10 is `gs`, the price is `price`.
fn synth(gd: f64, gs: f64, price: f64) -> Vec<Obs> {
    let mut v = Vec::new();
    for y in 1871..=1950 {
        for m in 1..=12u32 {
            let k = (y - 1871) * 12 + (m as i32 - 1);
            v.push(Obs {
                year: y,
                month: m,
                price,
                dividend: Some(10.0 * (1.0 + gd).powi(y - 1871)),
                earnings: None,
                cpi: 10.0 * 1.002f64.powi(k),
                gs10: gs,
                estimate: false,
            });
        }
    }
    v
}

fn row(rows: &[MarketRow], y: i32, m: u32) -> &MarketRow {
    rows.iter().find(|r| r.year == y && r.month == m).expect("row")
}

#[test]
fn gordon_equals_implied_when_g0_is_l() {
    // real dividend growth = L = 2%: nominal dividend growth (1.02 * 1.002^12) - 1, CPI 0.2% a month
    let gd = 1.02 * 1.002f64.powi(12) - 1.0;
    let rows = compute(&synth(gd, 6.0, 300.0), 0.02, 10);
    assert!(!rows.is_empty());
    for r in rows.iter().filter(|r| r.year <= 1900) {
        assert_eq!(r.status, "ok");
        assert!((r.g0 - 0.02).abs() < 1e-9, "g0 {}", r.g0);
        assert!((r.erp_implied.unwrap() - r.erp_gordon).abs() < 1e-9, "{} {}", r.erp_implied.unwrap(), r.erp_gordon);
    }
}

#[test]
fn value_hand_case_with_two_years() {
    // d0 = 100, g1 = 10%, g2 = 2% (L), r = 6%: D1 = 110, D2 = 112.2, terminal 112.2 * 1.02 / 0.04 at year 2
    let v = value(100.0, 0.10, 0.02, 2, 0.06);
    assert!((v - 2750.0).abs() < 1e-6, "{v}");
    // the search returns the premium: rr = 0.01, erp = 0.05
    let erp = solve_erp(100.0, 0.10, 0.02, 2, 0.01, v).unwrap();
    assert!((erp - 0.05).abs() < 1e-9, "{erp}");
}

#[test]
fn first_row_needs_ten_known_years_and_120_months_of_cpi() {
    let rows = compute(&synth(0.03, 5.0, 100.0), 0.02, 10);
    // before 1936 the known year is Y-2: 1882-12 has 1880 (1870 is missing); 1883-01 has 1881 and 1871.
    assert_eq!((rows[0].year, rows[0].month), (1883, 1));
    assert_eq!((rows.last().unwrap().year, rows.last().unwrap().month), (1950, 12));
}

#[test]
fn point_in_time_rule_and_poison() {
    let base = synth(0.05, 5.0, 100.0);
    let r0 = compute(&base, 0.02, 10);
    // March 1940 uses December 1938, April 1940 uses December 1939. d_known_real = D * CPI(t-1) / CPI(Dec).
    let d_of = |y: i32| 10.0 * 1.05f64.powi(y - 1871);
    let cpi = |y: i32, m: u32| 10.0 * 1.002f64.powi((y - 1871) * 12 + m as i32 - 1);
    let mar = row(&r0, 1940, 3);
    assert!((mar.d_known_real - d_of(1938) * cpi(1940, 2) / cpi(1938, 12)).abs() < 1e-9);
    let apr = row(&r0, 1940, 4);
    assert!((apr.d_known_real - d_of(1939) * cpi(1940, 3) / cpi(1939, 12)).abs() < 1e-9);
    // poison: all dividends from 1939-01 on are absurd: March 1940 does not change (December 1938 is the last known)
    let mut p = base.clone();
    for o in p.iter_mut().filter(|o| o.year >= 1939) {
        o.dividend = Some(9999.0);
    }
    let r1 = compute(&p, 0.02, 10);
    let (a, b) = (row(&r0, 1940, 3), row(&r1, 1940, 3));
    assert_eq!((a.d_known_real, a.g0, a.erp_implied, a.erp_gordon), (b.d_known_real, b.g0, b.erp_implied, b.erp_gordon));
    // the same poison from 1940-01 on: April 1940 does not change (December 1939 is known), but it changes at 1939-12
    let mut p = base.clone();
    for o in p.iter_mut().filter(|o| o.year >= 1940) {
        o.dividend = Some(9999.0);
    }
    let r2 = compute(&p, 0.02, 10);
    assert_eq!(row(&r0, 1940, 4).erp_implied, row(&r2, 1940, 4).erp_implied);
    let mut p = base.clone();
    p.iter_mut().find(|o| o.year == 1939 && o.month == 12).unwrap().dividend = Some(9999.0);
    let r3 = compute(&p, 0.02, 10);
    assert_ne!(row(&r0, 1940, 4).d_known_real, row(&r3, 1940, 4).d_known_real);
    // a dividend of a month that is not December changes nothing
    let mut p = base.clone();
    p.iter_mut().find(|o| o.year == 1939 && o.month == 6).unwrap().dividend = Some(9999.0);
    let r4 = compute(&p, 0.02, 10);
    assert_eq!(row(&r0, 1940, 4).erp_implied, row(&r4, 1940, 4).erp_implied);
    // the price of a later month does not change an earlier row
    let mut p = base.clone();
    p.iter_mut().find(|o| o.year == 1940 && o.month == 5).unwrap().price = 1e9;
    let r5 = compute(&p, 0.02, 10);
    assert_eq!(row(&r0, 1940, 4).erp_implied, row(&r5, 1940, 4).erp_implied);
}

#[test]
fn no_root_gives_status() {
    // a price far above any value in the range: the premium would be below the lower bound
    let rows = compute(&synth(0.05, 20.0, 1e9), 0.02, 10);
    assert!(rows.iter().all(|r| r.erp_implied.is_none() && r.status == "no_root_low"));
    assert!(csv_text(&rows).contains(",none,"));
    // a price near zero: the premium would be above the upper bound
    let rows = compute(&synth(0.05, 5.0, 1e-6), 0.02, 10);
    assert!(rows.iter().all(|r| r.erp_implied.is_none() && r.status == "no_root_high"));
}

#[test]
fn reader_reads_the_columns_and_the_note() {
    let t = "month,price,dividend,earnings,cpi,gs10,cape,estimate,note\n1871-01,4.44,0.26,0.4,12.4,5.32,,false,\n1871-02,4.5,,,12.8,5.3,,true,a note, with comma\n";
    let o = read_shiller(t).unwrap();
    assert_eq!(o.len(), 2);
    assert_eq!(o[0].dividend, Some(0.26));
    assert_eq!(o[1].dividend, None);
    assert!(o[1].estimate);
    assert!(read_shiller("h\n1871-01,1,1,1,1,1,,false,\n1871-03,1,1,1,1,1,,false,\n").is_err());
}

#[test]
fn boundary_1935_1936() {
    let r = compute(&synth(0.05, 5.0, 100.0), 0.02, 10);
    let d_of = |y: i32| 10.0 * 1.05f64.powi(y - 1871);
    let cpi = |y: i32, m: u32| 10.0 * 1.002f64.powi((y - 1871) * 12 + m as i32 - 1);
    // 1935-06 uses December 1933; 1936-06 uses December 1935; 1936-03 uses December 1934
    let a = row(&r, 1935, 6);
    assert!((a.d_known_real - d_of(1933) * cpi(1935, 5) / cpi(1933, 12)).abs() < 1e-9);
    let b = row(&r, 1936, 6);
    assert!((b.d_known_real - d_of(1935) * cpi(1936, 5) / cpi(1935, 12)).abs() < 1e-9);
    let c = row(&r, 1936, 3);
    assert!((c.d_known_real - d_of(1934) * cpi(1936, 2) / cpi(1934, 12)).abs() < 1e-9);
}

#[test]
fn rr_is_gs10_minus_cpi_change_over_120_months() {
    let r = compute(&synth(0.05, 5.0, 100.0), 0.02, 10);
    let x = row(&r, 1930, 7);
    let expect = 0.05 - (1.002f64.powi(12) - 1.0);
    assert!((x.rr - expect).abs() < 1e-12);
}

fn cpi_of(y: i32, m: u32) -> f64 {
    10.0 * 1.002f64.powi((y - 1871) * 12 + m as i32 - 1)
}
fn div_of(y: i32) -> f64 {
    10.0 * 1.05f64.powi(y - 1871)
}

#[test]
fn cpik_is_a_step_before_1913() {
    let r = compute(&synth(0.05, 5.0, 100.0), 0.02, 10);
    // known year of 1900 is 1898; CPIK is the Shiller December 1899 for every month of 1900
    for m in [1u32, 6, 12] {
        let x = row(&r, 1900, m);
        assert!((x.d_known_real - div_of(1898) * cpi_of(1899, 12) / cpi_of(1898, 12)).abs() < 1e-9, "month {m}");
        let rr = 0.05 - ((cpi_of(1899, 12) / cpi_of(1889, 12)).powf(0.1) - 1.0);
        assert!((x.rr - rr).abs() < 1e-12, "month {m}");
    }
}

#[test]
fn cpik_switches_at_1913_01() {
    let obs = synth(0.05, 5.0, 100.0);
    let f = fred_scaled(&obs, 1.5); // FRED differs from Shiller, so the source shows
    let r = super::compute(&obs, Some(&f), 0.02, 10).unwrap();
    // 1913-01: the Shiller December 1912
    let a = row(&r, 1913, 1);
    assert!((a.d_known_real - div_of(1911) * cpi_of(1912, 12) / cpi_of(1911, 12)).abs() < 1e-9);
    // 1913-02: the FRED value of 1913-01 (Shiller December of 1911 for the dividend: before 1913)
    let b = row(&r, 1913, 2);
    assert!((b.d_known_real - div_of(1911) * 1.5 * cpi_of(1913, 1) / cpi_of(1911, 12)).abs() < 1e-9);
    // the December of 1913 and later years is a FRED value
    let c = row(&r, 1915, 6); // known year 1913: its December is FRED
    assert!((c.d_known_real - div_of(1913) * 1.5 * cpi_of(1915, 5) / (1.5 * cpi_of(1913, 12))).abs() < 1e-9);
}

#[test]
fn rr_uses_cpik_at_both_ends() {
    let obs = synth(0.05, 5.0, 100.0);
    let f = fred_scaled(&obs, 1.5);
    let r = super::compute(&obs, Some(&f), 0.02, 10).unwrap();
    // 1920-07: CPIK = FRED 1920-06; 120 months back is 1910-07: the Shiller December 1909
    let x = row(&r, 1920, 7);
    let rr = 0.05 - ((1.5 * cpi_of(1920, 6) / cpi_of(1909, 12)).powf(0.1) - 1.0);
    assert!((x.rr - rr).abs() < 1e-12, "{} {}", x.rr, rr);
}

#[test]
fn poison_shiller_cpi_after_the_known_december_before_1913() {
    let base = synth(0.05, 5.0, 100.0);
    let r0 = compute(&base, 0.02, 10);
    let mut p = base.clone();
    for o in p.iter_mut().filter(|o| o.year == 1900 && o.month < 12) {
        o.cpi = 1e6;
    }
    let r1 = compute(&p, 0.02, 10);
    let (a, b) = (row(&r0, 1900, 6), row(&r1, 1900, 6));
    assert_eq!((a.d_known_real, a.g0, a.rr, a.erp_implied), (b.d_known_real, b.g0, b.rr, b.erp_implied));
    // the December of 1899 is the known one: a change of it moves the row
    let mut p = base.clone();
    p.iter_mut().find(|o| o.year == 1899 && o.month == 12).unwrap().cpi = 1e6;
    let r2 = compute(&p, 0.02, 10);
    assert_ne!(row(&r0, 1900, 6).rr, row(&r2, 1900, 6).rr);
}

#[test]
fn fred_file_is_needed_from_1913_and_a_missing_month_gives_no_cpi() {
    let obs = synth(0.05, 5.0, 100.0);
    assert!(super::compute(&obs, None, 0.02, 10).is_err());
    let pre: Vec<Obs> = obs.iter().filter(|o| o.year <= 1912).cloned().collect();
    assert!(!super::compute(&pre, None, 0.02, 10).unwrap().is_empty());
    let mut f = fred_scaled(&obs, 1.0);
    f.remove(&(1940, 5));
    let r = super::compute(&obs, Some(&f), 0.02, 10).unwrap();
    assert_eq!(row(&r, 1940, 6).status, "no_cpi");
    assert!(row(&r, 1940, 6).erp_implied.is_none());
    assert!(row(&r, 1940, 7).status != "no_cpi");
}

#[test]
fn cpiaucns_reader_skips_blank_values() {
    let f = read_fred_monthly("observation_date,CPIAUCNS\n1913-01-01,9.800\n2025-10-01,\n2025-11-01,324.1\n").unwrap();
    assert_eq!(f.get(&(1913, 1)), Some(&9.8));
    assert_eq!(f.get(&(2025, 10)), None);
    assert_eq!(f.get(&(2025, 11)), Some(&324.1));
}
