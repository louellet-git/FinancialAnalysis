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

pub fn check_vx(repo: &Path, days: &[i64]) -> Result<CheckResult> {
    let vx = facts::root_or_error("VX")?;
    for &day in days {
        let mut compared = 0;
        let mut mismatches = Vec::new();
        for s in cboe::settlement(repo, day)?.iter().filter(|s| s.is_monthly()) {
            let expiry = dates::parse(&s.expiry).ok_or("bad expiry")?;
            let civil = dates::format(expiry);
            let contract = Contract { root: vx, month: civil[5..7].parse().map_err(|_| "bad expiry")?, year: civil[..4].parse().map_err(|_| "bad expiry")? };
            let rows = match cboe::history(repo, &contract, Some(expiry), Some(day)) {
                Ok((_, rows)) => rows,
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
    let expected = expected_average(&published, first, next - 1).ok_or("ZQ check: the NY Fed file has no EFFR before the month")?;
    let implied = super::implied_rate(price);
    let mismatches = if (implied - expected).abs() > ZQ_TOLERANCE { vec![(civil, implied, expected)] } else { vec![] };
    Ok(CheckResult {
        check: format!(
            "{} implied rate (Yahoo, 100 - price) against the average EFFR of the month, known days and the last rate for the rest (tolerance {} pp)",
            contract.code(),
            ZQ_TOLERANCE * 100.0
        ),
        fred_series: "New York Fed EFFR (markets.newyorkfed.org)".into(),
        compared: 1,
        mismatches,
    })
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
}
