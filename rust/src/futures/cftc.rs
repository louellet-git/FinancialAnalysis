//! CFTC Commitments of Traders, report "Traders in Financial Futures" (futures only).
//!
//! The CFTC public reporting site (Socrata) gives each week as JSON. The positions are
//! those of Tuesday. The CFTC publishes them on the Friday after, so the newest week is
//! three to eleven days old.

use super::facts::Root;
use super::{load, text, Result};
use mapcheck::dates;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

pub const DATASET_URL: &str = "https://publicreporting.cftc.gov/resource/gpe5-46if.json";
pub const SOURCE: &str = "CFTC Commitments of Traders, Traders in Financial Futures, futures only (public reporting dataset gpe5-46if)";
const MAX_AGE: Duration = Duration::from_secs(12 * 3600);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Group {
    pub long: i64,
    pub short: i64,
    /// Not reported for non-reportable positions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread: Option<i64>,
    /// long - short
    pub net: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Week {
    /// The Tuesday of the positions.
    pub date: String,
    pub open_interest: i64,
    pub dealer: Group,
    pub asset_manager: Group,
    pub leveraged_funds: Group,
    pub other_reportable: Group,
    pub non_reportable: Group,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Positions {
    pub root: String,
    pub market: String,
    pub cftc_code: String,
    pub source: String,
    pub url: String,
    pub note: String,
    pub weeks: Vec<Week>,
}

fn int(row: &Value, key: &str) -> Result<i64> {
    let v = row.get(key).ok_or_else(|| format!("CFTC row has no field {key}"))?;
    let n = match v {
        Value::String(s) => s.trim().parse::<i64>().ok(),
        Value::Number(n) => n.as_i64(),
        _ => None,
    };
    n.ok_or_else(|| format!("CFTC field {key} is not an integer: {v}"))
}

fn group(row: &Value, prefix: &str, long: &str, short: &str, spread: Option<&str>) -> Result<Group> {
    let l = int(row, long).map_err(|e| format!("{prefix}: {e}"))?;
    let s = int(row, short).map_err(|e| format!("{prefix}: {e}"))?;
    let sp = match spread {
        Some(k) => Some(int(row, k).map_err(|e| format!("{prefix}: {e}"))?),
        None => None,
    };
    Ok(Group { long: l, short: s, spread: sp, net: l - s })
}

/// The rows of the JSON array, newest first. Gives the market name and the weeks.
pub fn parse_weeks(bytes: &[u8], code: &str) -> Result<(String, Vec<Week>)> {
    let rows: Value = serde_json::from_str(&text(bytes, "CFTC response")?).map_err(|e| format!("CFTC response is not JSON: {e}"))?;
    let rows = rows.as_array().ok_or_else(|| format!("CFTC response is not an array: {}", rows.to_string().chars().take(200).collect::<String>()))?;
    if rows.is_empty() {
        return Err(format!("CFTC has no rows for code {code}"));
    }
    let mut market = String::new();
    let mut weeks: Vec<Week> = Vec::new();
    for row in rows {
        let got = row.get("cftc_contract_market_code").and_then(Value::as_str).unwrap_or("");
        if got != code {
            return Err(format!("CFTC row has code '{got}'; expected {code}"));
        }
        if row.get("futonly_or_combined").and_then(Value::as_str) != Some("FutOnly") {
            return Err("CFTC row is not a futures-only row".into());
        }
        market = row.get("market_and_exchange_names").and_then(Value::as_str).unwrap_or("").to_string();
        let date = row.get("report_date_as_yyyy_mm_dd").and_then(Value::as_str).unwrap_or("");
        let date = date.get(..10).filter(|d| dates::parse(d).is_some()).ok_or_else(|| format!("CFTC row has a bad date '{date}'"))?;
        if weeks.last().is_some_and(|w| date >= w.date.as_str()) {
            return Err(format!("CFTC row {date} is not older than the row before it"));
        }
        weeks.push(Week {
            date: date.to_string(),
            open_interest: int(row, "open_interest_all")?,
            dealer: group(row, "dealer", "dealer_positions_long_all", "dealer_positions_short_all", Some("dealer_positions_spread_all"))?,
            asset_manager: group(row, "asset manager", "asset_mgr_positions_long", "asset_mgr_positions_short", Some("asset_mgr_positions_spread"))?,
            leveraged_funds: group(row, "leveraged funds", "lev_money_positions_long", "lev_money_positions_short", Some("lev_money_positions_spread"))?,
            other_reportable: group(row, "other reportable", "other_rept_positions_long", "other_rept_positions_short", Some("other_rept_positions_spread"))?,
            non_reportable: group(row, "non-reportable", "nonrept_positions_long_all", "nonrept_positions_short_all", None)?,
        });
    }
    Ok((market, weeks))
}

pub fn positions(repo: &Path, root: &Root, weeks: u32) -> Result<Positions> {
    let url = format!(
        "{DATASET_URL}?cftc_contract_market_code={}&$order=report_date_as_yyyy_mm_dd%20DESC&$limit={weeks}",
        root.cftc_code
    );
    let code = root.cftc_code;
    let (market, weeks) = load(repo, &format!("cftc_{code}_{weeks}.json"), &url, MAX_AGE, |b| parse_weeks(b, code), |_| false)?;
    Ok(Positions {
        root: root.root.into(),
        market,
        cftc_code: code.into(),
        source: SOURCE.into(),
        url,
        note: "Positions of Tuesday, in contracts, all contract months together. The CFTC publishes them on the Friday after. \
               A position is the position of a trader class, not of one firm. Net = long - short."
            .into(),
        weeks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(date: &str) -> String {
        format!(
            r#"{{"cftc_contract_market_code":"13874A","futonly_or_combined":"FutOnly","market_and_exchange_names":"E-MINI S&P 500 - CHICAGO MERCANTILE EXCHANGE",
            "report_date_as_yyyy_mm_dd":"{date}T00:00:00.000","open_interest_all":"2017687",
            "dealer_positions_long_all":"100","dealer_positions_short_all":"300","dealer_positions_spread_all":"50",
            "asset_mgr_positions_long":"900","asset_mgr_positions_short":"100","asset_mgr_positions_spread":"10",
            "lev_money_positions_long":"200","lev_money_positions_short":"700","lev_money_positions_spread":"20",
            "other_rept_positions_long":"5","other_rept_positions_short":"6","other_rept_positions_spread":"7",
            "nonrept_positions_long_all":"11","nonrept_positions_short_all":"12"}}"#
        )
    }

    #[test]
    fn parses_weeks() {
        let text = format!("[{},{}]", row("2026-11-10"), row("2026-11-03"));
        let (market, weeks) = parse_weeks(text.as_bytes(), "13874A").unwrap();
        assert!(market.starts_with("E-MINI S&P 500"));
        assert_eq!(weeks.len(), 2);
        assert_eq!(weeks[0].date, "2026-11-10");
        assert_eq!(weeks[0].leveraged_funds, Group { long: 200, short: 700, spread: Some(20), net: -500 });
        assert_eq!(weeks[0].non_reportable.spread, None);
        assert_eq!(weeks[0].open_interest, 2_017_687);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_weeks(b"<html>", "13874A").unwrap_err().contains("not JSON"));
        assert!(parse_weeks(b"[]", "13874A").unwrap_err().contains("no rows"));
        assert!(parse_weeks(br#"{"error":true}"#, "13874A").unwrap_err().contains("not an array"));
        let one = format!("[{}]", row("2026-11-10"));
        assert!(parse_weeks(one.as_bytes(), "209742").unwrap_err().contains("expected 209742"));
        assert!(parse_weeks(one.replace("FutOnly", "Combined").as_bytes(), "13874A").unwrap_err().contains("futures-only"));
        assert!(parse_weeks(one.replace("\"100\"", "\"x\"").as_bytes(), "13874A").unwrap_err().contains("not an integer"));
        assert!(parse_weeks(one.replace("2026-11-10", "10/11/2026").as_bytes(), "13874A").unwrap_err().contains("bad date"));
        let order = format!("[{},{}]", row("2026-11-03"), row("2026-11-10"));
        assert!(parse_weeks(order.as_bytes(), "13874A").unwrap_err().contains("not older"));
        assert!(parse_weeks(one.replace("\"open_interest_all\"", "\"x\"").as_bytes(), "13874A").unwrap_err().contains("no field open_interest_all"));
    }
}
