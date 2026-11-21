//! Cross-checks of the futures data against a second file.
//!
//! 1. VIX futures: the settlement file of one date against the history file of each contract
//!    (two files of Cboe).
//! 2. 30-day fed funds futures: the implied rate of the contract of the current month against
//!    the average EFFR of that month (New York Fed). Rate futures of the spot month are
//!    the one case where the price has a second, independent source.
//!
//! There is no second source for the CME prices (ES, NQ, Treasury, SOFR). The CME website
//! refuses automated access and Stooq asks for JavaScript. See HIDDEN-ASSUMPTIONS #76.

use super::facts::{self, Contract, Symbol};
use super::{cboe, last_weekday_before_today, load, text, yahoo, Result};
use crate::CheckResult;
use mapcheck::dates;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

/// Settlement prices in the two Cboe files are given to 4 digits.
const VX_TOLERANCE: f64 = 0.005;
/// 5 basis points. A change of the target range inside the month can move the result more.
const ZQ_TOLERANCE: f64 = 0.0005;
pub const NYFED_URL: &str = "https://markets.newyorkfed.org/api/rates/unsecured/effr/search.json?startDate={start}&endDate={end}";

/// Mismatches as `CheckResult` keeps them: (label, ours, theirs).
type Mismatches = Vec<(String, f64, f64)>;

/// One date of the VX check, without file access. Each monthly contract of the daily file is
/// compared with the settle of the same date in its history file. `history` gives the rows of
/// a contract for the expiry of the daily file; a "not found" error leaves the contract out
/// (it is not counted in `compared`). Gives (compared, mismatches).
fn compare_vx(
    day: i64,
    settles: &[cboe::Settle],
    history: &dyn Fn(&Contract, i64) -> Result<Vec<cboe::HistoryRow>>,
) -> Result<(u32, Mismatches)> {
    let vx = facts::root_or_error("VX")?;
    let mut compared = 0;
    let mut mismatches = Vec::new();
    for s in settles.iter().filter(|s| s.is_monthly()) {
        let expiry = dates::parse(&s.expiry).ok_or("bad expiry")?;
        let civil = dates::format(expiry);
        let contract = Contract { root: vx, month: civil[5..7].parse().map_err(|_| "bad expiry")?, year: civil[..4].parse().map_err(|_| "bad expiry")? };
        let rows = match history(&contract, expiry) {
            Ok(rows) => rows,
            Err(e) if super::is_not_found(&e) => continue,
            Err(e) => return Err(e),
        };
        if let Some(row) = rows.iter().find(|r| r.day == day) {
            compared += 1;
            if (row.settle - s.price).abs() > VX_TOLERANCE {
                mismatches.push((format!("{} {}", dates::format(day), s.symbol), s.price, row.settle));
            }
        }
    }
    Ok((compared, mismatches))
}

pub fn check_vx(repo: &Path, days: &[i64]) -> Result<CheckResult> {
    for &day in days {
        let settles = cboe::settlement(repo, day)?;
        let (compared, mismatches) =
            compare_vx(day, &settles, &|contract, expiry| cboe::history(repo, contract, Some(expiry), Some(day)).map(|(_, rows)| rows))?;
        if compared > 0 {
            return Ok(CheckResult {
                check: format!("VX settlement of {}: Cboe daily file against Cboe contract history files (tolerance {VX_TOLERANCE} points)", dates::format(day)),
                fred_series: "Cboe history file of each monthly VX contract".into(),
                compared,
                mismatches,
            });
        }
    }
    Err("VX check: no date had a row in both Cboe files".into())
}

/// Published EFFR values (percent) by day. Gives {day: percent}.
pub fn parse_effr(bytes: &[u8]) -> Result<BTreeMap<i64, f64>> {
    let root: Value = serde_json::from_str(&text(bytes, "NY Fed response")?).map_err(|e| format!("NY Fed response is not JSON: {e}"))?;
    let rows = root.get("refRates").and_then(Value::as_array).ok_or("NY Fed response has no refRates")?;
    let mut out = BTreeMap::new();
    for row in rows {
        let day = row.get("effectiveDate").and_then(Value::as_str).and_then(dates::parse).ok_or("NY Fed row has a bad effectiveDate")?;
        let rate = row.get("percentRate").and_then(Value::as_f64).filter(|r| r.is_finite()).ok_or("NY Fed row has a bad percentRate")?;
        out.insert(day, rate);
    }
    if out.is_empty() {
        return Err("NY Fed response has no rows".into());
    }
    Ok(out)
}

/// The average of the daily EFFR over the calendar days `first..=last` as a decimal.
/// A day with no published rate (weekend, holiday, not yet published, future) has the
/// rate of the last day before it that has one.
pub fn expected_average(published: &BTreeMap<i64, f64>, first: i64, last: i64) -> Option<f64> {
    let mut total = 0.0;
    for day in first..=last {
        total += published.range(..=day).next_back().map(|(_, r)| *r)?;
    }
    Some(total / (last - first + 1) as f64 / 100.0)
}

/// The ZQ comparison without file access: the implied rate of the contract `code` (100 minus
/// `price`) against the average EFFR of the calendar days `first..next` (`next` = first day of the next month).
fn compare_zq(code: &str, civil: &str, price: f64, published: &BTreeMap<i64, f64>, first: i64, next: i64) -> Result<CheckResult> {
    let expected = expected_average(published, first, next - 1).ok_or("ZQ check: the NY Fed file has no EFFR before the month")?;
    let implied = super::implied_rate(price);
    let mismatches = if (implied - expected).abs() > ZQ_TOLERANCE { vec![(civil.to_string(), implied, expected)] } else { vec![] };
    Ok(CheckResult {
        check: format!(
            "{code} implied rate (Yahoo, 100 - price) against the average EFFR of the month, known days and the last rate for the rest (tolerance {} pp)",
            ZQ_TOLERANCE * 100.0
        ),
        fred_series: "New York Fed EFFR (markets.newyorkfed.org)".into(),
        compared: 1,
        mismatches,
    })
}

pub fn check_zq(repo: &Path, day: i64) -> Result<CheckResult> {
    let civil = dates::format(day);
    let (year, month): (i64, i64) = (civil[..4].parse().map_err(|_| "bad date")?, civil[5..7].parse().map_err(|_| "bad date")?);
    let first = dates::days_from_civil(year, month, 1);
    let next = if month == 12 { dates::days_from_civil(year + 1, 1, 1) } else { dates::days_from_civil(year, month + 1, 1) };
    let zq = facts::root_or_error("ZQ")?;
    let contract = Contract { root: zq, month: month as u32, year };
    let series = yahoo::series(repo, &Symbol::Contract(contract.clone()), day, day, true)
        .map_err(|e| format!("ZQ check needs the contract {} of the month of the date: {e}", contract.code()))?;
    let price = series.observations[0].price;
    if series.observations[0].date != civil {
        return Err(format!("ZQ check: Yahoo has no bar of {} on {civil}", contract.code()));
    }
    let (start, end) = (dates::format(first - 7), civil.clone());
    let published = load(
        repo,
        &format!("nyfed_effr_{start}_{end}.json"),
        &NYFED_URL.replace("{start}", &start).replace("{end}", &end),
        Duration::from_secs(6 * 3600),
        parse_effr,
        |_| day < dates::today() - 3,
    )?;
    compare_zq(&contract.code(), &civil, price, &published, first, next)
}

/// Run the checks. `date`: one day; without it the checks use the last weekdays before today.
pub fn run(repo: &Path, date: Option<&str>) -> Result<Vec<CheckResult>> {
    let days: Vec<i64> = match date {
        Some(d) => vec![crate::forwards::parse_date_arg(d)?.1],
        None => {
            let mut days = vec![last_weekday_before_today()];
            while days.len() < 5 {
                let mut d = days[days.len() - 1] - 1;
                while super::weekday(d) >= 5 {
                    d -= 1;
                }
                days.push(d);
            }
            days
        }
    };
    Ok(vec![check_vx(repo, &days)?, check_zq(repo, days[0])?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effr_average_carries_the_last_rate() {
        let d = |s: &str| dates::parse(s).unwrap();
        // Fri 2026-11-06 3.90, Mon 11-09 3.88; the month ends 11-30.
        let published: BTreeMap<i64, f64> = [(d("2026-10-30"), 3.90), (d("2026-11-02"), 3.90), (d("2026-11-09"), 3.88)].into();
        // 2026-11-01..08 = 8 days at 3.90, 11-09..30 = 22 days at 3.88.
        let expected = (8.0 * 3.90 + 22.0 * 3.88) / 30.0 / 100.0;
        let got = expected_average(&published, d("2026-11-01"), d("2026-11-30")).unwrap();
        assert!((got - expected).abs() < 1e-12);
        assert!(expected_average(&BTreeMap::from([(d("2026-12-01"), 3.9)]), d("2026-11-01"), d("2026-11-30")).is_none());
    }

    #[test]
    fn effr_file_parses_and_rejects() {
        let ok = r#"{"refRates":[{"effectiveDate":"2026-11-17","type":"EFFR","percentRate":3.88},{"effectiveDate":"2026-11-16","type":"EFFR","percentRate":3.87}]}"#;
        let m = parse_effr(ok.as_bytes()).unwrap();
        assert_eq!((m.len(), m[&dates::parse("2026-11-17").unwrap()]), (2, 3.88));
        assert!(parse_effr(b"<html>").unwrap_err().contains("not JSON"));
        assert!(parse_effr(br#"{"refRates":[]}"#).unwrap_err().contains("no rows"));
        assert!(parse_effr(br#"{}"#).unwrap_err().contains("no refRates"));
        assert!(parse_effr(ok.replace("3.88", "\"x\"").as_bytes()).unwrap_err().contains("percentRate"));
        assert!(parse_effr(ok.replace("2026-11-17", "17.11.2026").as_bytes()).unwrap_err().contains("effectiveDate"));
    }

    // --- check_vx: the comparison of one date (compare_vx), with fixture files and no network ---

    const SETTLE_FILE: &str = "Product,Symbol,Expiration Date,Price\n\
        VX,VX/Z6,2026-12-16,22.6881\n\
        VX47/Z6,VX47/Z6,2026-12-02,22.0000\n\
        VX,VX/F7,2027-01-20,22.0000\n\
        VX,VX/G7,2027-02-17,21.5000\n\
        VX,VX/H7,2027-03-17,21.0040\n";
    const HISTORY_HEADER: &str = "Trade Date,Futures,Open,High,Low,Close,Settle,Change,Total Volume,EFP,Open Interest\n";

    /// A history file with one row, the trade date 2026-11-17.
    fn history_file(cell: &str, settle: f64) -> Vec<cboe::HistoryRow> {
        let text = format!("{HISTORY_HEADER}2026-11-17,{cell},0,0,0,0,{settle},0,0,0,0\n");
        cboe::parse_history(text.as_bytes()).unwrap()
    }

    /// The history of each contract by expiry. February 2027 has no file (as the store answers 404).
    fn fixture_history(contract: &Contract, expiry: i64) -> Result<Vec<cboe::HistoryRow>> {
        match dates::format(expiry).as_str() {
            "2026-12-16" => Ok(history_file("Z (Dec 2026)", 22.6881)), // agrees
            "2027-01-20" => Ok(history_file("F (Jan 2027)", 22.0060)), // 0.006 away: fails
            "2027-03-17" => Ok(history_file("H (Mar 2027)", 21.0000)), // 0.004 away: passes
            _ => Err(format!("HTTP 404 Not Found for {}", contract.code())),
        }
    }

    #[test]
    fn vx_compare_agrees_fails_and_skips() {
        let day = dates::parse("2026-11-17").unwrap();
        let settles = cboe::parse_settlement(SETTLE_FILE.as_bytes()).unwrap();
        let (compared, mismatches) = compare_vx(day, &settles, &fixture_history).unwrap();
        // Dec (agrees), Jan (0.006), Mar (0.004) are compared. The weekly contract is not a monthly one.
        // Feb has no history file: it is left out and is not in `compared`.
        assert_eq!(compared, 3);
        assert_eq!(mismatches.len(), 1);
        let (label, ours, theirs) = &mismatches[0];
        assert_eq!(label, "2026-11-17 VX/F7");
        assert_eq!((*ours, *theirs), (22.0, 22.006));
    }

    #[test]
    fn vx_compare_other_cases() {
        let day = dates::parse("2026-11-17").unwrap();
        let only_dec = cboe::parse_settlement("Product,Symbol,Expiration Date,Price\nVX,VX/Z6,2026-12-16,22.6881\n".as_bytes()).unwrap();
        // A history file with no row of the date: nothing to compare.
        let other_day = dates::parse("2026-11-16").unwrap();
        assert_eq!(compare_vx(other_day, &only_dec, &fixture_history).unwrap(), (0, vec![]));
        // An agreeing contract gives no mismatch.
        assert_eq!(compare_vx(day, &only_dec, &fixture_history).unwrap(), (1, vec![]));
        // An error that is not "not found" stops the check.
        let broken = |_: &Contract, _: i64| -> Result<Vec<cboe::HistoryRow>> { Err("HTTP 500 Server Error".into()) };
        assert!(compare_vx(day, &only_dec, &broken).unwrap_err().contains("500"));
        // A daily file with no monthly contract compares nothing.
        assert_eq!(compare_vx(day, &[], &fixture_history).unwrap(), (0, vec![]));
    }

    // --- check_zq: the comparison (compare_zq), with a fixture EFFR month and no network ---

    /// EFFR 3.88 percent on each day from 2026-10-30. The average of November 2026 is 3.88 percent.
    fn effr_fixture() -> BTreeMap<i64, f64> {
        BTreeMap::from([(dates::parse("2026-10-30").unwrap(), 3.88)])
    }

    fn zq(price: f64, published: &BTreeMap<i64, f64>) -> Result<CheckResult> {
        let first = dates::parse("2026-11-01").unwrap();
        compare_zq("ZQX26", "2026-11-19", price, published, first, dates::parse("2026-12-01").unwrap())
    }

    #[test]
    fn zq_compare_within_and_beyond_5_bp() {
        let published = effr_fixture();
        // Price 96.12 gives 3.88 percent: 0 bp. Price 96.071 gives 3.929 percent: 4.9 bp, passes.
        for price in [96.12, 96.071, 96.169] {
            let r = zq(price, &published).unwrap();
            assert_eq!((r.compared, r.mismatches.len()), (1, 0), "price {price}");
        }
        // Price 96.06 gives 3.94 percent: 6 bp above, fails. Price 96.18 gives 3.82 percent: 6 bp below, fails.
        let r = zq(96.06, &published).unwrap();
        assert_eq!(r.mismatches.len(), 1);
        let (date, implied, expected) = &r.mismatches[0];
        assert_eq!(date, "2026-11-19");
        assert!((implied - 0.0394).abs() < 1e-9 && (expected - 0.0388).abs() < 1e-9);
        assert_eq!(zq(96.18, &published).unwrap().mismatches.len(), 1);
        assert!(r.check.starts_with("ZQX26 implied rate") && r.check.contains("tolerance 0.05 pp"));
    }

    #[test]
    fn zq_compare_reports_a_month_with_no_effr() {
        // The first rate is after the month: the average has no value for 2026-11-01.
        let late = BTreeMap::from([(dates::parse("2026-12-01").unwrap(), 3.9)]);
        assert!(zq(96.12, &late).unwrap_err().contains("no EFFR before the month"));
        assert!(zq(96.12, &BTreeMap::new()).is_err());
    }
}
