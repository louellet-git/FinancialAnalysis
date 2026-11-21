//! Tests of the reality check of `fa dcf attribute --reality` on small data made by hand (fixed answers). No network and no
//! store; the test of the release path writes a small cache in a temporary folder.

use super::*;
use super::super::auto::AutoInputs;
use super::super::auto_attribute::{csv_text, csv_text_with, release_path, CSV_HEADER};
use super::super::auto_baserate::base_rates;

const D2: &str = "2026-11-20";

fn yr(year: i32, revenue: f64) -> YearData {
    YearData { end: format!("{year}-12-31"), filed: format!("{}-02-15", year + 1), revenue: Some(revenue), ..Default::default() }
}

fn company(ticker: &str, base: f64, g: f64) -> AutoInputs {
    AutoInputs { ticker: ticker.into(), years: (2015..=2025).map(|y| yr(y, base * (1.0 + g).powi(y - 2015))).collect(), ..Default::default() }
}

/// Three companies of 50B or more with last windows of 5%, 10% and 20%.
fn rates() -> BaseRates {
    base_rates(&[company("A", 60e9, 0.05), company("B", 70e9, 0.10), company("C", 80e9, 0.20)], &|_| None, 10, D2)
}

#[test]
fn the_row_has_the_base_revenue_the_own_growth_and_the_place_of_the_implied_growth() {
    let r = rates();
    // the base revenue is the last fiscal year (2025): 100B x 1.03^10 for a 3% a year company; the own 5-year growth is 3% (closed)
    let c = company("X", 100e9, 0.03);
    let row = reality_row("X", &c.years, D2, Some((0.12, Bound::Exact)), &r, String::new());
    assert_eq!(row.bucket, Some(3));
    assert!((row.base_revenue.unwrap() - 100e9 * 1.03f64.powi(10)).abs() < 1.0);
    assert!((row.growth5.unwrap() - 0.03).abs() < 1e-12);
    // 0.12 is above 5% and 10% and below 20%: 2 of 3 are below
    let p = row.position.unwrap();
    assert!((p.pctile - 200.0 / 3.0).abs() < 1e-9 && !p.above_all && p.n == 3);
    // above every case: the flag; below every case: percentile 0
    let high = reality_row("X", &c.years, D2, Some((0.25, Bound::Above)), &r, String::new());
    assert!(high.position.unwrap().above_all && high.position.unwrap().pctile == 100.0);
    assert_eq!(high.bound, Bound::Above);
    assert_eq!(reality_row("X", &c.years, D2, Some((0.01, Bound::Exact)), &r, String::new()).position.unwrap().pctile, 0.0);
    // no implied value (NaN, or none): no place
    assert!(reality_row("X", &c.years, D2, Some((f64::NAN, Bound::Exact)), &r, String::new()).position.is_none());
    assert!(reality_row("X", &c.years, D2, None, &r, String::new()).position.is_none());
    // a small company is put in its own bucket: no last window there, so no place (and no invented one)
    let small = company("S", 2e9, 0.03);
    let row = reality_row("S", &small.years, D2, Some((0.12, Bound::Exact)), &r, String::new());
    assert_eq!(row.bucket, Some(1));
    assert!(row.position.is_none());
}

#[test]
fn a_year_filed_after_the_date_is_not_the_base_year_of_the_row() {
    // the poison test of the row: a revenue filed after D2 (absurd) changes neither the bucket nor the own growth
    let r = rates();
    let mut c = company("X", 100e9, 0.03);
    let clean = reality_row("X", &c.years, D2, Some((0.12, Bound::Exact)), &r, String::new());
    c.years.push(YearData { end: "2026-06-30".into(), filed: "2026-12-05".into(), revenue: Some(9e18), ..Default::default() });
    assert_eq!(clean, reality_row("X", &c.years, D2, Some((0.12, Bound::Exact)), &r, String::new()));
}

#[test]
fn the_premium_history_percentile_uses_only_the_rows_before_d1() {
    let text = "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list\n\
                2026-10-02,2026-10-02,gsw,2026-10-01,0.04,0.040,90,100\n\
                2026-10-09,2026-10-09,gsw,2026-10-08,0.04,0.050,90,100\n\
                2026-10-16,2026-10-16,gsw,2026-10-15,0.04,0.060,90,100\n\
                2026-10-23,2026-10-23,gsw,2026-10-22,0.04,,90,100\n\
                2026-11-06,2026-11-06,gsw,2026-11-05,0.04,0.070,90,100\n\
                2026-11-13,2026-11-13,gsw,2026-11-12,0.04,0.080,90,100\n\
                2026-11-20,2026-11-20,par,2026-11-19,0.04,0.090,90,100\n";
    let h = parse_history(text).unwrap();
    // the row with no s1 is out
    assert_eq!(h.len(), 6);
    // D1 is 2026-11-13: the rows before it are 4 (the row of D1 and the row after it are not used)
    let before = history_before(&h, "2026-11-13");
    assert_eq!(before, vec![0.040, 0.050, 0.060, 0.070]);
    let p = premium_position(&h, "2026-11-13", Some(0.055), Some(0.075));
    // 0.055: two of four are below (50); 0.075: all four are below (100, above every row)
    assert_eq!(p.n_before, 4);
    assert_eq!((p.first.as_str(), p.last.as_str()), ("2026-10-02", "2026-11-06"));
    assert_eq!(p.p1.unwrap().pctile, 50.0);
    assert!(p.p2.unwrap().above_all && p.p2.unwrap().pctile == 100.0);
    // the poison: rows on D1 and after it with absurd values change nothing
    let mut poison = h.clone();
    poison.push(("2026-11-13".into(), 9.0));
    poison.push(("2026-12-04".into(), -9.0));
    assert_eq!(premium_position(&poison, "2026-11-13", Some(0.055), Some(0.075)), p);
    // no solution at a date: no number
    assert!(premium_position(&h, "2026-11-13", None, Some(0.075)).p1.is_none());
    // a file that is not the file of `premium-series`
    assert!(parse_history("date,value\n2026-01-01,1\n").is_err());
}

#[test]
fn the_headers_end_with_the_new_names_and_the_old_columns_keep_their_places() {
    assert!(HEADER_TAIL.starts_with("base_revenue,base_bucket,growth5_own,growth10_implied") && HEADER_TAIL.ends_with("list_premium2_pctile,release_8k_path"));
    // the cells of a row have the columns of the tail
    let r = rates();
    let c = company("X", 100e9, 0.03);
    let row = reality_row("X", &c.years, D2, Some((0.12, Bound::Exact)), &r, "a,b".into());
    let prem = premium_position(&[("2026-10-02".into(), 0.04), ("2026-10-09".into(), 0.05)], "2026-11-13", Some(0.045), None);
    let cells = cells(&row, Some(&prem));
    assert_eq!(cells.split(',').count(), HEADER_TAIL.split(',').count(), "{cells}");
    assert!(cells.starts_with("134391637934,50B or more,0.0300,0.1200,,66.7,false,0.0450,,50.0,,"), "{cells}");
    // a comma in the path is not a column
    assert!(cells.ends_with("a;b"));
    // without --reality the header is the header of before; with it the tail is at the end and the old columns keep their places
    let without = csv_text(&[]);
    assert_eq!(without.trim_end(), CSV_HEADER);
    let with = csv_text_with(&[], Some((HEADER_TAIL, &[])));
    assert_eq!(with.trim_end(), format!("{CSV_HEADER},{HEADER_TAIL}"));
    assert!(with.starts_with(CSV_HEADER));
    // the header of `fa dcf premium-series` is not changed by this work
    assert_eq!(super::super::auto_premium::HEADER, "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change");
    // an empty row (no implied growth, no list premium) has the same count of cells
    let empty = reality_row("X", &[], D2, None, &r, String::new());
    assert_eq!(super::cells(&empty, None).split(',').count(), HEADER_TAIL.split(',').count());
}

#[test]
fn the_text_says_above_every_case_and_the_limits() {
    let r = rates();
    let c = company("X", 100e9, 0.03);
    let rows = vec![reality_row("X", &c.years, D2, Some((0.25, Bound::Above)), &r, String::new()), reality_row("Y", &c.years, D2, Some((0.12, Bound::Exact)), &r, String::new())];
    let t = block_text(&rows, &r, "the list of the run", None);
    assert!(t.contains("(5) REALITY CHECK") && t.contains("above every case of 50B or more (n 3)") && t.contains("percentile 67 of 50B or more (n 3)"), "{t}");
    assert!(t.contains("floor (>)") && t.contains("survivorship") && t.contains("no file in the cache"), "{t}");
}

#[test]
fn the_release_is_the_newest_file_of_the_newest_accession_in_the_window() {
    let cache = std::env::temp_dir().join(format!("fa_reality_release_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let t = cache.join("NVDA");
    std::fs::create_dir_all(t.join("filings/8k/0001-26-000019")).unwrap();
    std::fs::create_dir_all(t.join("filings/8k/0001-26-000088")).unwrap();
    std::fs::write(t.join("filings/8k/0001-26-000019/q1.htm"), "old").unwrap();
    std::fs::write(t.join("filings/8k/0001-26-000088/q3.htm"), "new").unwrap();
    // 000073 is in the window and has no folder; 000051 is before the window
    std::fs::write(
        t.join("sec_8k_earnings.json"),
        r#"{"cik":"1","filings":[
            {"accession":"0001-26-000088","filed":"2026-11-17","items":"2.02,9.01"},
            {"accession":"0001-26-000073","filed":"2026-11-14","items":"2.02"},
            {"accession":"0001-26-000051","filed":"2026-05-20","items":"2.02"},
            {"accession":"0001-26-000019","filed":"2026-02-25","items":"2.02"}]}"#,
    )
    .unwrap();
    let p = release_path(&cache, "NVDA", "2026-11-13", "2026-11-20");
    assert!(p.ends_with("NVDA/filings/8k/0001-26-000088/q3.htm"), "{p}");
    // a window that has the filing of 02-25 only
    let p = release_path(&cache, "NVDA", "2026-02-01", "2026-03-01");
    assert!(p.ends_with("0001-26-000019/q1.htm"), "{p}");
    // a window with an 8-K and no folder, a window with none, and a ticker with no list: empty
    assert_eq!(release_path(&cache, "NVDA", "2026-05-01", "2026-06-01"), "");
    assert_eq!(release_path(&cache, "NVDA", "2026-06-01", "2026-07-01"), "");
    assert_eq!(release_path(&cache, "AAPL", "2026-11-13", "2026-11-20"), "");
    let _ = std::fs::remove_dir_all(&cache);
}
