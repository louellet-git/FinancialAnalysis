use super::*;

fn ob(y: i32, m: u32, price: f64, dividend: Option<f64>, cpi: f64) -> Obs {
    Obs { year: y, month: m, price, dividend, cpi, gs10: 5.0, estimate: false }
}

fn fred(v: &[((i32, u32), f64)]) -> Fred {
    v.iter().cloned().collect()
}

#[test]
fn outcome_hand_case_h2() {
    // t = 2000-01, H = 2. P 100, 110, 121; D = 12 a year so D/12 = 1; CPI 100, 105, 110; bills 12% a year.
    let obs = vec![ob(2000, 1, 100.0, Some(12.0), 100.0), ob(2000, 2, 110.0, Some(12.0), 105.0), ob(2000, 3, 121.0, Some(12.0), 110.0)];
    let cpi = fred(&[((2000, 1), 100.0), ((2000, 2), 105.0), ((2000, 3), 110.0)]);
    let bill = fred(&[((2000, 1), 12.0), ((2000, 2), 12.0)]);
    let empty = Fred::new();
    let inp = Inputs { cpi: &cpi, tb3ms: &bill, nber: &empty, gs10: &empty };
    let s = Series::new(&obs, &inp);
    let x = s.outcome(mi(2000, 1), 2).unwrap();
    // G_TR = (111/100) (122/110); c = 100/110; RT = (G_TR c)^6 - 1; RB = (1.12^(1/6) c)^6 - 1 = 1.12 c^6 - 1
    let g: f64 = 1.11 * (122.0 / 110.0);
    let c: f64 = 100.0 / 110.0;
    let expect = c.powi(6) * (g.powi(6) - 1.12);
    assert!((x - expect).abs() < 1e-12, "{x} {expect}");
}

#[test]
fn deflator_step_and_crossing_1913() {
    // Shiller CPI 10, 11, 12, 13 at 1912-11, 1912-12, 1913-01, 1913-02; FRED 1913-01 = 20, 1913-02 = 22
    let obs = vec![ob(1912, 11, 1.0, Some(1.0), 10.0), ob(1912, 12, 1.0, Some(1.0), 11.0), ob(1913, 1, 1.0, Some(1.0), 12.0), ob(1913, 2, 1.0, Some(1.0), 13.0)];
    let cpi = fred(&[((1913, 1), 20.0), ((1913, 2), 22.0)]);
    let empty = Fred::new();
    let inp = Inputs { cpi: &cpi, tb3ms: &empty, nber: &empty, gs10: &empty };
    let s = Series::new(&obs, &inp);
    // 1912-11 to 1913-02: Shiller up to 1913-01 (10 / 12), then FRED (20 / 22)
    let c = s.cpi_ratio(mi(1912, 11), mi(1913, 2)).unwrap();
    assert!((c - (10.0 / 12.0) * (20.0 / 22.0)).abs() < 1e-15);
    // a window inside Shiller months uses Shiller only
    assert!((s.cpi_ratio(mi(1912, 11), mi(1912, 12)).unwrap() - 10.0 / 11.0).abs() < 1e-15);
    // a window inside FRED months uses FRED only
    assert!((s.cpi_ratio(mi(1913, 1), mi(1913, 2)).unwrap() - 20.0 / 22.0).abs() < 1e-15);
}

#[test]
fn missing_cpi_at_the_end_month_drops_the_window_and_only_it() {
    // R2: the window that ends at the month without a CPI drops out; a window that ends later does not need that month
    let obs = vec![
        ob(2025, 8, 100.0, Some(12.0), 100.0),
        ob(2025, 9, 100.0, Some(12.0), 100.0),
        ob(2025, 10, 100.0, Some(12.0), 100.0),
        ob(2025, 11, 100.0, Some(12.0), 100.0),
        ob(2025, 12, 100.0, Some(12.0), 100.0),
    ];
    let cpi = fred(&[((2025, 8), 100.0), ((2025, 9), 100.0), ((2025, 11), 100.0), ((2025, 12), 100.0)]); // 2025-10 missing
    let bill = fred(&[((2025, 8), 4.0), ((2025, 9), 4.0), ((2025, 10), 4.0), ((2025, 11), 4.0)]);
    let empty = Fred::new();
    let inp = Inputs { cpi: &cpi, tb3ms: &bill, nber: &empty, gs10: &empty };
    let s = Series::new(&obs, &inp);
    assert_eq!(s.outcome(mi(2025, 8), 2), Err(Drop::NoCpi)); // ends 2025-10
    assert!(s.outcome(mi(2025, 8), 3).is_ok()); // ends 2025-11: the ratio telescopes
    assert_eq!(s.outcome(mi(2025, 8), 5), Err(Drop::NoData));
}

#[test]
fn missing_dividend_drops_the_window() {
    let obs = vec![ob(2000, 1, 100.0, Some(12.0), 100.0), ob(2000, 2, 100.0, None, 100.0)];
    let cpi = fred(&[((2000, 1), 100.0), ((2000, 2), 100.0)]);
    let bill = fred(&[((2000, 1), 4.0)]);
    let empty = Fred::new();
    let inp = Inputs { cpi: &cpi, tb3ms: &bill, nber: &empty, gs10: &empty };
    assert_eq!(Series::new(&obs, &inp).outcome(mi(2000, 1), 1), Err(Drop::NoDividend));
}

#[test]
fn bill_series_joins_at_1934() {
    // 1933-12 bill rate from NBER (6%), 1934-01 from TB3MS (0%); H = 2 from 1933-11 uses y(1933-11) and y(1933-12)
    let obs = vec![ob(1933, 11, 100.0, Some(0.0), 10.0), ob(1933, 12, 100.0, Some(0.0), 10.0), ob(1934, 1, 100.0, Some(0.0), 10.0)];
    let cpi = fred(&[((1933, 11), 10.0), ((1933, 12), 10.0), ((1934, 1), 10.0)]);
    let nber = fred(&[((1933, 11), 6.0), ((1933, 12), 6.0)]);
    let tb = fred(&[((1934, 1), 50.0)]);
    let empty = Fred::new();
    let inp = Inputs { cpi: &cpi, tb3ms: &tb, nber: &nber, gs10: &empty };
    let s = Series::new(&obs, &inp);
    // zero price change, zero dividend: X = 0 - RB with RB = 1.06 - 1 (two months of 6% a year, annualized over 2 months)
    let x = s.outcome(mi(1933, 11), 2).unwrap();
    assert!((x + 0.06).abs() < 1e-12, "{x}");
}

fn line_pairs(n: usize, a: f64, b: f64) -> Vec<Pair> {
    // premium: integers with variation in each phase; X = a + b premium exactly
    (0..n).map(|i| {
        let p = ((i * 37) % 101) as f64;
        Some((p, a + b * p))
    }).collect()
}

#[test]
fn slope_and_oos_on_a_known_line() {
    let pairs = line_pairs(1486, 2.0, 3.0);
    let b = phase_mean_slope(&pairs).unwrap();
    assert!((b - 3.0).abs() < 1e-9, "{b}");
    let o = oos_r2(&pairs).unwrap();
    assert!((o.r2 - 1.0).abs() < 1e-9, "{}", o.r2);
    // the pairs of s = 0..239 are complete at i = 239 + 120 = 359: the first forecast
    assert_eq!(o.forecasts, 1486 - 359);
}

#[test]
fn bootstrap_is_deterministic_and_has_known_p_on_extreme_cases() {
    let pairs = line_pairs(600, 2.0, 3.0);
    let a = bootstrap(&pairs, 50, 7).unwrap();
    let b = bootstrap(&pairs, 50, 7).unwrap();
    assert_eq!((a.b_hat, a.p), (b.b_hat, b.p));
    // an exact line: every pseudo-sample has the slope 3, b* - b_hat = 0 < 3: p = 1 / (K + 1)
    assert!((a.p - 1.0 / 51.0).abs() < 1e-15, "{}", a.p);
    // a flat outcome: b_hat = 0, every b* - 0 >= 0: p = 1
    let flat: Vec<Pair> = (0..600).map(|i| Some((((i * 37) % 101) as f64, 5.0))).collect();
    let f = bootstrap(&flat, 50, 7).unwrap();
    assert_eq!(f.b_hat, 0.0);
    assert!((f.p - 1.0).abs() < 1e-15);
}

#[test]
fn a_period_with_fewer_than_3_windows_has_no_regression() {
    // P-6 has 350 start months: floor(350 / 120) = 2
    assert_eq!(independent_windows(350), 2);
    assert!(!has_regression(350));
    assert!(!has_regression(359));
    assert!(has_regression(360));
    // the regime of a start month, by the start
    assert_eq!(regime(mi(1979, 9)), "P-5");
    assert_eq!(regime(mi(1979, 10)), "P-6");
    assert_eq!(regime(mi(2008, 11)), "P-6");
    assert_eq!(regime(mi(2008, 12)), "P-7");
    assert_eq!(regime(mi(2022, 3)), "P-8");
    assert_eq!(regime(mi(1951, 2)), "P-3");
}

#[test]
fn holm_hand_case() {
    let adj = holm(&[0.01, 0.02, 0.04, 0.03, 0.2]);
    // sorted 0.01, 0.02, 0.03, 0.04, 0.2: 0.05, 0.08, 0.09, max(0.08, 0.09) = 0.09, 0.2
    let expect = [0.05, 0.08, 0.09, 0.09, 0.2];
    for (a, e) in adj.iter().zip(expect) {
        assert!((a - e).abs() < 1e-12, "{adj:?}");
    }
}
