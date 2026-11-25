//! Tests of `fa dcf baserates` on small data made by hand (fixed answers). No network and no cache.

use super::*;

/// One fiscal year ending on 31 December of `year`, filed on 15 February of the next year.
fn yr(year: i32, revenue: Option<f64>) -> YearData {
    YearData { end: format!("{year}-12-31"), filed: format!("{}-02-15", year + 1), revenue, ..Default::default() }
}

/// A company with the fiscal years `from` to `to` and a revenue `base x (1 + g)^k` (k = 0 in the year `from`).
fn company(ticker: &str, from: i32, to: i32, base: f64, g: f64) -> AutoInputs {
    AutoInputs { ticker: ticker.into(), years: (from..=to).map(|y| yr(y, Some(base * (1.0 + g).powi(y - from)))).collect(), ..Default::default() }
}

const D: &str = "2026-11-20";

fn no_cik(_: &str) -> Option<String> {
    None
}

#[test]
fn the_window_growth_and_the_bucket_of_a_hand_made_company() {
    // 2015 to 2025: eleven years, revenue 60B in 2015 and 8% a year. The one window is 2015 to 2025: growth 8% (closed).
    let c = company("AAA", 2015, 2025, 60e9, 0.08);
    let w = windows(&c.years, 10, D);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!((w[0].base_end.as_str(), w[0].end.as_str()), ("2015-12-31", "2025-12-31"));
    assert!((w[0].growth - 0.08).abs() < 1e-12 && (w[0].base_revenue - 60e9).abs() < 1.0, "{w:?}");
    assert_eq!(bucket_of(w[0].base_revenue), 3);
    assert_eq!(BUCKETS[3], "50B or more");
    // the window of 5 years of the same company: the end years 2020 to 2025 (6 windows), each 8%
    let w5 = windows(&c.years, 5, D);
    assert_eq!(w5.len(), 6);
    assert!(w5.iter().all(|w| (w.growth - 0.08).abs() < 1e-12));
    // the edges of the buckets: an edge is in the higher bucket
    let b: Vec<usize> = [0.0, 999e6, 1e9, 9.99e9, 10e9, 49.99e9, 50e9, 1e12].iter().map(|x| bucket_of(*x)).collect();
    assert_eq!(b, vec![0, 0, 1, 1, 2, 2, 3, 3]);
    // a year with no revenue (or a revenue of 0 or less) is no base: the window of 2015 to 2025 is gone
    let mut hole = c.years.clone();
    hole[0].revenue = None;
    assert!(windows(&hole, 10, D).is_empty());
    hole[0].revenue = Some(-1.0);
    assert!(windows(&hole, 10, D).is_empty());
    // a 52 or 53 week year: the end dates move by some days and the window stays; 100 days off, it does not
    let mut shifted = c.years.clone();
    shifted[0].end = "2016-01-02".into();
    assert_eq!(windows(&shifted, 10, D).len(), 1);
    shifted[0].end = "2016-04-10".into();
    assert!(windows(&shifted, 10, D).is_empty());
}

#[test]
fn a_window_whose_end_year_is_filed_after_the_date_does_not_enter_the_poison_test() {
    // 2014 to 2025; the date is 2026-03-31 and the year 2025 is filed on 2026-04-15, after it
    const D: &str = "2026-03-31";
    let clean = company("AAA", 2014, 2025, 60e9, 0.08);
    assert_eq!(windows(&clean.years, 10, D).len(), 2, "the year 2025 would give a second window");
    let mut late = clean.years.clone();
    late[11].filed = "2026-04-15".into();
    // poison: the late year gets an absurd revenue
    let mut poison = late.clone();
    poison[11].revenue = Some(9e18);
    let truncated: Vec<YearData> = clean.years[..11].to_vec();
    let (a, b, c) = (windows(&late, 10, D), windows(&poison, 10, D), windows(&truncated, 10, D));
    assert_eq!(a, b, "the absurd late year changed the windows");
    assert_eq!(a, c, "the windows differ from the windows of the data cut at D");
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].end, "2024-12-31");
    // the last window is the window of 2024; the poison year is not the last year
    assert_eq!(last_window(&poison, 10, D), last_window(&truncated, 10, D));
    assert_eq!(last_year(&poison, D).unwrap().0, "2024-12-31");
    // a base year that is filed after D does not enter either
    let mut late_base = clean.years.clone();
    late_base[1].filed = "2026-04-15".into();
    late_base[1].revenue = Some(1.0);
    assert!(windows(&late_base, 10, D).iter().all(|w| w.base_end != "2015-12-31"));
    // a year with no filing date is not proved public
    let mut none = clean.years.clone();
    none[11].filed.clear();
    assert_eq!(windows(&none, 10, D), c);
    // the whole table: the poison company has the same last window as the cut company
    let (mut p, mut t) = (company("AAA", 2014, 2025, 60e9, 0.08), company("AAA", 2014, 2025, 60e9, 0.08));
    p.years = poison;
    t.years = truncated;
    assert_eq!(base_rates(&[p], &no_cik, 10, D), base_rates(&[t], &no_cik, 10, D));
}

#[test]
fn the_percentile_and_the_flag_above_every_case() {
    let v = [0.01, 0.02, 0.03, 0.04];
    let p = position_in(&v, 0.025).unwrap();
    assert_eq!((p.pctile, p.above_all, p.n), (50.0, false, 4));
    // equal to the maximum is not above it: 3 of 4 are below
    let p = position_in(&v, 0.04).unwrap();
    assert_eq!((p.pctile, p.above_all), (75.0, false));
    // above the maximum: 100 and the flag
    let p = position_in(&v, 0.0401).unwrap();
    assert_eq!((p.pctile, p.above_all), (100.0, true));
    let p = position_in(&v, 0.005).unwrap();
    assert_eq!((p.pctile, p.above_all), (0.0, false));
    // an empty set and a number that is not a number have no place
    assert!(position_in(&[], 0.1).is_none() && position_in(&v, f64::NAN).is_none());
    // a value that is not a number is not a case
    assert_eq!(position_in(&[f64::NAN, 0.01, 0.03], 0.02).unwrap().n, 2);
}

#[test]
fn the_table_of_hand_made_companies_has_the_closed_answers() {
    // three companies of 50B or more with the last windows 5%, 10% and 20%; a class of the second company; a company with only
    // 5 years (no base-year value); a company with no year; a company that stopped filing in 2022
    let mut list = vec![company("A", 2015, 2025, 60e9, 0.05), company("GOOG", 2015, 2025, 70e9, 0.10), company("GOOGL", 2015, 2025, 70e9, 0.10), company("C", 2015, 2025, 80e9, 0.20)];
    list.push(company("SHORT", 2021, 2025, 60e9, 0.10));
    list.push(AutoInputs { ticker: "EMPTY".into(), ..Default::default() });
    list.push(company("OLD", 2012, 2022, 5e9, 0.10));
    let cik = |t: &str| match t {
        "GOOG" | "GOOGL" => Some("1652044".to_string()),
        _ => None,
    };
    let b = base_rates(&list, &cik, 10, D);
    assert_eq!((b.tickers, b.companies), (7, 6), "two classes are one company");
    assert_eq!(b.no_base, vec!["SHORT"]);
    assert_eq!(b.no_year, vec!["EMPTY"]);
    assert_eq!(b.stale, vec!["OLD"]);
    assert_eq!(b.last.len(), 3);
    let s = stats(&BaseRates::in_bucket(&b.last, Some(3))).unwrap();
    assert_eq!(s.n, 3);
    // sorted 0.05, 0.10, 0.20: the median 0.10; the 75th 0.15; the 90th 0.18; the 95th 0.19 (linear between the points)
    assert!((s.median - 0.10).abs() < 1e-9 && (s.p75 - 0.15).abs() < 1e-9 && (s.p90 - 0.18).abs() < 1e-9 && (s.p95 - 0.19).abs() < 1e-9, "{s:?}");
    assert!((s.max - 0.20).abs() < 1e-9 && s.max_ticker == "C");
    // the GOOGL class is not a second company: the first ticker is used
    assert!(b.last.iter().all(|c| c.ticker != "GOOGL") && b.pooled.iter().all(|c| c.ticker != "GOOGL"));
    // the pooled windows: one window of each of the three companies of 50B or more, and the window of OLD (2012 to 2022; 5B)
    assert_eq!(BaseRates::in_bucket(&b.pooled, Some(3)).len(), 3);
    assert_eq!(BaseRates::in_bucket(&b.pooled, Some(1)).len(), 1);
    // the percentile of an implied growth of the bucket
    let p = position_in(&b.last_growths(3), 0.12).unwrap();
    assert!((p.pctile - 200.0 / 3.0).abs() < 1e-9 && !p.above_all);
    assert!(position_in(&b.last_growths(3), 0.25).unwrap().above_all);
    // the text and the CSV carry the numbers
    let text = text(&b);
    assert!(text.contains("50B or more") && text.contains("no base-year value: 1: SHORT") && text.contains("survivorship"), "{text}");
    let csv = csv_text(&b);
    assert!(csv.starts_with(CSV_HEADER));
    assert!(csv.contains("last,50B or more,3,0.1000,0.1500,0.1800,0.1900,0.2000,C"), "{csv}");
    assert!(csv.contains("no_base_value,all,1,"), "{csv}");
    assert!(csv.lines().all(|l| l.split(',').count() == 9));
}

#[test]
fn the_company_with_the_base_year_of_a_late_start_has_no_window_and_nothing_is_patched() {
    // a company whose store starts 9 years before its last year: no base-year value; the revenue of the years that exist is
    // not used to make one
    let c = company("NEW", 2016, 2025, 10e9, 0.30);
    assert_eq!(last_window(&c.years, 10, D), Last::NoBase);
    assert!(windows(&c.years, 10, D).is_empty());
    // with 9 years the same company has a window of 9 years: 30% (closed)
    let w = windows(&c.years, 9, D);
    assert_eq!(w.len(), 1);
    assert!((w[0].growth - 0.30).abs() < 1e-12);
}
