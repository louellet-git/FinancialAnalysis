//! Tests of `fa dcf premium-series` on small data made by hand (fixed answers). No network and no store: the point-in-time test
//! uses a cache of a temporary folder that the test writes.

use super::super::auto::tests::{plain_rules, synthetic, table};
use super::*;
use std::sync::{Arc, Mutex};

fn s(v: &[(&str, f64)]) -> Vec<(String, f64)> {
    v.iter().map(|(d, c)| (d.to_string(), *c)).collect()
}

fn day(t: &str) -> i64 {
    mapcheck::dates::parse(t).unwrap()
}

#[test]
fn the_header_is_the_contract_exactly() {
    assert_eq!(HEADER, "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change");
    assert_eq!(HEADER.split(',').count(), 18);
    // a row has the same count of fields, and a missing value is an empty field
    let row = Row { friday: "2026-11-20".into(), anchor: "2026-11-20".into(), source: Some(CurveSource::Par), curve_date: "2026-11-19".into(), zero10: Some(0.049), s1: Some(0.0232), n_value: 97, n_list: 100, ..Default::default() };
    assert_eq!(row.csv_line(), "2026-11-20,2026-11-20,par,2026-11-19,0.049,0.0232,97,100,,,,,,,,,,");
    assert_eq!(row.csv_line().split(',').count(), 18);
    let full = Row { ret: Some(-0.0856), n_ret: Some(95), s2: Some([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, -0.0856]), ..row };
    assert_eq!(full.csv_line(), "2026-11-20,2026-11-20,par,2026-11-19,0.049,0.0232,97,100,-0.0856,95,0.1,0.2,0.3,0.4,0.5,0.6,0.7,-0.0856");
}

#[test]
fn the_fridays_run_from_the_first_friday_on_or_after_the_start() {
    let v = fridays(day("2026-09-15"), day("2026-11-20"));
    assert_eq!(v.len(), 10);
    assert_eq!((v[0].as_str(), v[9].as_str()), ("2026-09-18", "2026-11-20"));
    // a start that is a Friday is the first row; an end before the first Friday gives none
    assert_eq!(fridays(day("2026-09-18"), day("2026-09-18")), vec!["2026-09-18".to_string()]);
    assert!(fridays(day("2026-09-19"), day("2026-09-24")).is_empty());
}

#[test]
fn the_anchor_of_a_friday_holiday_is_the_thursday_close() {
    // Good Friday 2026-04-03 has no close
    let spy = s(&[("2026-04-01", 1.0), ("2026-04-02", 2.0), ("2026-04-06", 3.0)]);
    assert_eq!(anchor_of(&spy, "2026-04-03").unwrap(), "2026-04-02");
    // a trading Friday is its own anchor
    let spy = s(&[("2026-04-02", 2.0), ("2026-04-10", 3.0)]);
    assert_eq!(anchor_of(&spy, "2026-04-10").unwrap(), "2026-04-10");
    // a SPY series that ends a week before the Friday is an error, not a repeated anchor
    assert!(anchor_of(&spy, "2026-04-17").is_err());
    assert!(anchor_of(&spy, "2026-03-27").is_err());
}

#[test]
fn a_gsw_row_four_days_old_gives_the_par_fit() {
    let a = "2026-11-20";
    assert_eq!(curve_source(Some("2026-11-20"), a), CurveSource::Gsw);
    assert_eq!(curve_source(Some("2026-11-17"), a), CurveSource::Gsw, "3 days is allowed");
    assert_eq!(curve_source(Some("2026-11-16"), a), CurveSource::Par, "4 days is too old");
    assert_eq!(curve_source(None, a), CurveSource::Par);
    // a row after the anchor is not a row on or before it
    assert_eq!(curve_source(Some("2026-11-21"), a), CurveSource::Par);
}

#[test]
fn ret_w_is_a_median_and_a_stale_name_is_left_out() {
    let spy = s(&[("2026-11-13", 750.0), ("2026-11-20", 690.0)]);
    let data: HashMap<String, Vec<(String, f64)>> = [
        ("A", s(&[("2026-11-13", 10.0), ("2026-11-20", 11.0)])),
        ("B", s(&[("2026-11-13", 10.0), ("2026-11-20", 12.0)])),
        ("C", s(&[("2026-11-13", 10.0), ("2026-11-20", 13.0)])),
        // the close of 11-20 is of 11-18: stale (a return of +1000% would move the median)
        ("D", s(&[("2026-11-13", 10.0), ("2026-11-18", 110.0)])),
        // no close at the first anchor
        ("F", s(&[("2026-11-20", 99.0)])),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let tickers: Vec<String> = ["A", "B", "C", "D", "E", "F"].iter().map(|x| x.to_string()).collect();
    let (ret, n) = week_return(&tickers, &|t| data.get(t).map(Vec::as_slice), &spy, "2026-11-13", "2026-11-20");
    // E has no series, D is stale, F has no first close: A, B, C stay; the median is 20%
    assert_eq!(n, 3);
    assert!((ret.unwrap() - 0.2).abs() < 1e-12, "{ret:?}");
    // an even count: the mean of the two middle values (A and C only: 10% and 30%)
    let two: Vec<String> = ["A", "C"].iter().map(|x| x.to_string()).collect();
    let (ret, n) = week_return(&two, &|t| data.get(t).map(Vec::as_slice), &spy, "2026-11-13", "2026-11-20");
    assert_eq!(n, 2);
    assert!((ret.unwrap() - 0.2).abs() < 1e-12);
    // no name: no median
    assert_eq!(week_return(&[], &|t| data.get(t).map(Vec::as_slice), &spy, "2026-11-13", "2026-11-20"), (None, 0));
}

// ---- the point-in-time test -------------------------------------------------------------------------------------

fn weekdays(from: &str, to: &str) -> Vec<String> {
    (day(from)..=day(to)).filter(|d| (d + 3).rem_euclid(7) < 5).map(mapcheck::dates::format).collect()
}

/// A cache folder with the price files. `poison`: absurd closes on every weekday after 2026-11-20.
fn write_cache(name: &str, poison: bool) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fa_premium_series_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let last = if poison { "2026-12-04" } else { "2026-11-20" };
    for (k, t) in ["SPY", "T00", "T01", "T02"].iter().enumerate() {
        let rows: Vec<String> = weekdays("2026-11-02", last)
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let close = if d.as_str() > "2026-11-20" { 1.0e6 } else { 20.0 + 3.0 * k as f64 + 0.3 * i as f64 };
                format!("{{\"date\":\"{d}\",\"open\":{close},\"high\":{close},\"low\":{close},\"close\":{close},\"volume\":1000}}")
            })
            .collect();
        std::fs::create_dir_all(dir.join(t)).unwrap();
        std::fs::write(dir.join(t).join("prices.json"), format!("[{}]", rows.join(","))).unwrap();
    }
    dir
}

fn curve_numbers(date: &str) -> CurveNumbers {
    CurveNumbers { date: date.into(), beta0: 3.0, beta1: 1.0, beta2: 5.0, beta3: 17.0, tau1: 1.5, tau2: 6.0, zero10: 0.04 }
}

/// The two weeks 2026-11-13 and 2026-11-20 with the bridge. The mapper records the date that it gets.
fn two_weeks(cache: &Path, seen: &Arc<Mutex<Vec<String>>>) -> Vec<Row> {
    let t = table();
    let tickers: Vec<String> = ["T00", "T01", "T02"].iter().map(|x| x.to_string()).collect();
    let log = seen.clone();
    let ctx = Ctx {
        cache,
        table: &t,
        fade: &[],
        rules: plain_rules(),
        closes: Ctx::load_closes(cache, &tickers),
        tickers,
        spy: price_series(cache, "SPY").unwrap(),
        bridge: true,
        curve_of: Box::new(|a| Ok((curve_numbers(a), CurveSource::Gsw))),
        mapper: Box::new(move |a| {
            log.lock().unwrap().push(a.to_string());
            Ok(synthetic(3))
        }),
    };
    let (r1, st) = week_row(&ctx, "2026-11-13", None).unwrap();
    let (r2, _) = week_row(&ctx, "2026-11-20", Some(&st)).unwrap();
    vec![r1, r2]
}

#[test]
fn a_poison_of_a_close_after_the_anchor_does_not_change_the_row_and_the_truncation_gives_the_same_row() {
    let (clean, poisoned) = (write_cache("clean", false), write_cache("poison", true));
    let (seen_a, seen_b) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())));
    // the clean cache is the truncated cache: it has no close after the anchor
    let a = two_weeks(&clean, &seen_a);
    let b = two_weeks(&poisoned, &seen_b);
    assert_eq!(a, b);
    // the rows are not empty: the test can fail
    assert_eq!(a[0].n_ret, None);
    assert_eq!(a[1].n_ret, Some(3));
    assert!(a[1].s1.is_some() && a[1].n_value == 3 && a[1].n_list == 3, "{:?}", a[1]);
    assert!(a[1].ret.is_some_and(|r| r > 0.0 && r < 0.5), "{:?}", a[1].ret);
    assert!(a[1].s2.is_some_and(|x| x.iter().all(|v| v.is_finite())), "{:?}", a[1].s2);
    assert_eq!(a[0].s2, None);
    // the mapper gets the anchor (the facts of the anchor), once for each week
    assert_eq!(*seen_a.lock().unwrap(), vec!["2026-11-13".to_string(), "2026-11-20".to_string()]);
    assert_eq!(*seen_b.lock().unwrap(), *seen_a.lock().unwrap());
    // the bridge adds up: the seven parts add to the change of the list
    let x = a[1].s2.unwrap();
    assert!((x[..7].iter().sum::<f64>() - x[7]).abs() < 1e-9, "{x:?}");
    let _ = (std::fs::remove_dir_all(clean), std::fs::remove_dir_all(poisoned));
}

#[test]
fn the_per_ticker_header_and_line_are_the_contract() {
    assert_eq!(PER_TICKER_HEADER, "friday,anchor,ticker,weight,change,facts,time,rates,premium,growth,residual,nomodel");
    assert_eq!(PER_TICKER_HEADER.split(',').count(), 12);
    let t = TickerRow { ticker: "NVDA".into(), weight: 0.25, change: -0.0856, parts: [0.0, 0.0, -0.01, -0.02, 0.0, 0.5, f64::NAN] };
    // a missing value (NaN) is an empty field
    assert_eq!(t.csv_line("2026-11-20", "2026-11-20"), "2026-11-20,2026-11-20,NVDA,0.25,-0.0856,0,0,-0.01,-0.02,0,0.5,");
    assert_eq!(t.csv_line("2026-11-20", "2026-11-20").split(',').count(), 12);
}

#[test]
fn the_per_ticker_rows_of_a_week_add_to_the_main_row() {
    let cache = write_cache("per_ticker", false);
    let rows = two_weeks(&cache, &Arc::new(Mutex::new(Vec::new())));
    // the first week has no bridge and no rows
    assert!(rows[0].per_ticker.is_empty());
    let w = &rows[1];
    assert_eq!(w.per_ticker.len(), 3);
    assert_eq!(w.per_ticker.iter().map(|t| t.ticker.as_str()).collect::<Vec<_>>(), vec!["T00", "T01", "T02"]);
    let s2 = w.s2.unwrap();
    for (k, want) in s2.iter().take(7).enumerate() {
        let sum: f64 = w.per_ticker.iter().map(|t| t.parts[k]).sum();
        assert!((sum - want).abs() < 1e-12, "part {k}: {sum} against {want}");
    }
    // the weights add to 1 and weight x change adds to the change of the list
    assert!((w.per_ticker.iter().map(|t| t.weight).sum::<f64>() - 1.0).abs() < 1e-12);
    assert!((w.per_ticker.iter().map(|t| t.weight * t.change).sum::<f64>() - s2[7]).abs() < 1e-12);
    // the parts are not all zero: the test can fail
    assert!(w.per_ticker.iter().any(|t| t.parts.iter().any(|p| p.abs() > 1e-6)));
    // the main CSV line is the same as without the per-ticker rows
    let mut bare = w.clone();
    bare.per_ticker.clear();
    assert_eq!(w.csv_line(), bare.csv_line());
    assert_eq!(w.csv_line().split(',').count(), 18);
    let _ = std::fs::remove_dir_all(cache);
}

#[test]
fn the_windows_file_has_the_fixed_header_and_one_row_of_two_anchors() {
    // the input of `fa macro view --windows FILE` (the Head of Economics): header d1,date; d1 = the previous anchor
    assert_eq!(WINDOWS_HEADER, "d1,date");
    assert_eq!(windows_line("2026-11-13", "2026-11-20"), "2026-11-13,2026-11-20");
}
