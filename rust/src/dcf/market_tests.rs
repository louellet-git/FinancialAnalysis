use super::*;

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
    let x = row(&r, 1920, 7);
    let expect = 0.05 - (1.002f64.powi(12) - 1.0);
    assert!((x.rr - expect).abs() < 1e-12);
}
