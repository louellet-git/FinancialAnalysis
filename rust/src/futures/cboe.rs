//! Cboe Futures Exchange: VIX futures settlement prices.
//!
//! Two public files, both CSV:
//!
//! - the settlement file of one trade date: all VX contracts of that date;
//! - the history file of one contract: all dates of that contract, from 2013.
//!
//! Both give the settlement price. A date is a trade date (Chicago).

use super::facts::{self, Contract};
use mapcheck::dates;
use super::{load, number, text, Curve, CurvePoint, Observation, Result, Series};
use std::path::Path;
use std::time::Duration;

pub const SETTLE_URL: &str = "https://www.cboe.com/us/futures/market_statistics/settlement/csv?dt={date}";
pub const HISTORY_URL: &str = "https://cdn.cboe.com/data/us/futures/market_statistics/historical_data/VX/VX_{expiry}.csv";
pub const SOURCE: &str = "Cboe Futures Exchange (CFE), VIX futures settlement";
pub const PRICE_TYPE: &str = "settlement price (VIX index points)";
const SETTLE_HEADER: &str = "Product,Symbol,Expiration Date,Price";
const HISTORY_HEADER: &str = "Trade Date,Futures,Open,High,Low,Close,Settle,Change,Total Volume,EFP,Open Interest";
const MAX_AGE: Duration = Duration::from_secs(6 * 3600);

#[derive(Debug, Clone, PartialEq)]
pub struct Settle {
    pub product: String,
    pub symbol: String,
    pub expiry: String,
    pub price: f64,
}

impl Settle {
    /// A monthly contract has a symbol such as VX/Z6. A weekly one has VX47/X6.
    pub fn is_monthly(&self) -> bool {
        self.product == "VX" && self.symbol.starts_with("VX/")
    }
}

/// The settlement file of one date. A file with a header and no row is valid: Cboe gives it
/// for a day with no trading and for a date that is too old.
pub fn parse_settlement(bytes: &[u8]) -> Result<Vec<Settle>> {
    let text = text(bytes, "Cboe settlement file")?;
    let mut lines = text.trim_start_matches('\u{feff}').lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next() != Some(SETTLE_HEADER) {
        return Err(format!("Cboe settlement file has an unexpected header (expected '{SETTLE_HEADER}')"));
    }
    let mut rows = Vec::new();
    for line in lines {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.len() != 4 {
            return Err(format!("Cboe settlement line has {} fields; expected 4: {line}", f.len()));
        }
        if dates::parse(f[2]).is_none() {
            return Err(format!("Cboe settlement line has a bad expiry date: {line}"));
        }
        let price = number(f[3], "Cboe settlement price")?;
        if price <= 0.0 {
            return Err(format!("Cboe settlement line has a price that is not positive: {line}"));
        }
        rows.push(Settle { product: f[0].into(), symbol: f[1].into(), expiry: f[2].into(), price });
    }
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRow {
    pub day: i64,
    /// Month letter and year of the "Futures" column, for example ('Z', 2026).
    pub contract: (char, i64),
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub settle: f64,
    pub volume: f64,
    pub open_interest: f64,
}

/// "Z (Dec 2026)" -> ('Z', 2026)
fn contract_cell(cell: &str) -> Option<(char, i64)> {
    let letter = cell.chars().next()?;
    let year = cell.trim_end_matches(')').rsplit(' ').next()?.parse().ok()?;
    facts::month_number(letter).map(|_| (letter, year))
}

/// The history file of one contract. A price of 0 in open, high or low means no trade: it becomes None.
pub fn parse_history(bytes: &[u8]) -> Result<Vec<HistoryRow>> {
    let text = text(bytes, "Cboe history file")?;
    let mut lines = text.trim_start_matches('\u{feff}').lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next() != Some(HISTORY_HEADER) {
        return Err(format!("Cboe history file has an unexpected header (expected '{HISTORY_HEADER}')"));
    }
    let mut rows: Vec<HistoryRow> = Vec::new();
    for line in lines {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.len() != 11 {
            return Err(format!("Cboe history line has {} fields; expected 11: {line}", f.len()));
        }
        let day = dates::parse(f[0]).ok_or_else(|| format!("Cboe history line has a bad date: {line}"))?;
        if rows.last().is_some_and(|r| day <= r.day) {
            return Err(format!("Cboe history line: date {} is not after the date before it", f[0]));
        }
        let contract = contract_cell(f[1]).ok_or_else(|| format!("Cboe history line has a bad contract: {line}"))?;
        let price = |i: usize, what: &str| -> Result<Option<f64>> {
            let v = number(f[i], what)?;
            Ok((v != 0.0).then_some(v))
        };
        let settle = number(f[6], "Cboe settle")?;
        if settle <= 0.0 {
            return Err(format!("Cboe history line has a settle that is not positive: {line}"));
        }
        rows.push(HistoryRow {
            day,
            contract,
            open: price(2, "Cboe open")?,
            high: price(3, "Cboe high")?,
            low: price(4, "Cboe low")?,
            settle,
            volume: number(f[8], "Cboe volume")?,
            open_interest: number(f[10], "Cboe open interest")?,
        });
    }
    if rows.is_empty() {
        return Err("Cboe history file has no data rows".into());
    }
    Ok(rows)
}

/// The expiry of a VIX monthly contract: the Wednesday that is 30 days before the third
/// Friday of the next month. The exchange moves it one business day earlier when it is a holiday.
pub fn standard_expiry(year: i64, month: u32) -> i64 {
    let (y, m) = if month == 12 { (year + 1, 1) } else { (year, month as i64 + 1) };
    let first = dates::days_from_civil(y, m, 1);
    let first_friday = first + (4 - super::weekday(first)).rem_euclid(7);
    first_friday + 14 - 30
}

pub fn settlement(repo: &Path, day: i64) -> Result<Vec<Settle>> {
    let date = dates::format(day);
    let today = dates::today();
    // The file has no date inside. For a date that is not past, Cboe gives the file of the
    // last trade date, so the program asks only for a date before today (UTC).
    if day >= today {
        return Err(format!(
            "Cboe gives the settlement file of an earlier date for {date}, because the file of that day is not final; ask for a date before {}",
            dates::format(today)
        ));
    }
    load(
        repo,
        &format!("cboe_settle_{date}.csv"),
        &SETTLE_URL.replace("{date}", &date),
        MAX_AGE,
        parse_settlement,
        // A file of an earlier day with rows does not change.
        move |rows: &Vec<Settle>| day < today - 1 && !rows.is_empty(),
    )
}

/// The history of one contract. `expiry`: the expiry date from a settlement file, when the
/// caller has it. Without it the program uses the standard date and, if the file is not there,
/// the day before (a holiday moves the expiry).
pub fn history(repo: &Path, contract: &Contract, expiry: Option<i64>, need_day: Option<i64>) -> Result<(String, Vec<HistoryRow>)> {
    let standard = standard_expiry(contract.year, contract.month);
    let mut last_error = String::new();
    let tries = match expiry {
        Some(e) => vec![e],
        None => vec![standard, standard - 1, standard - 2],
    };
    for expiry in tries {
        let date = dates::format(expiry);
        let url = HISTORY_URL.replace("{expiry}", &date);
        let loaded = load(repo, &format!("cboe_VX_{date}.csv"), &url, MAX_AGE, parse_history, |rows: &Vec<HistoryRow>| {
            need_day.is_some_and(|d| rows.last().is_some_and(|r| r.day >= d))
        });
        match loaded {
            Ok(rows) => {
                let wanted = (facts::month_letter(contract.month), contract.year);
                if rows.iter().any(|r| r.contract != wanted) {
                    return Err(format!("Cboe file {url} is for another contract than {}", contract.code()));
                }
                return Ok((url, rows));
            }
            // The file store answers 403 for a file that does not exist.
            Err(e) if super::is_not_found(&e) || e.starts_with("HTTP 403 ") => last_error = e,
            Err(e) => return Err(e),
        }
    }
    Err(format!("{last_error} (no history file for {})", contract.code()))
}

pub fn series(repo: &Path, contract: &Contract, start: i64, end: i64, latest: bool) -> Result<Series> {
    let (url, rows) = history(repo, contract, None, Some(end))?;
    let mut observations: Vec<Observation> = rows
        .iter()
        .filter(|r| r.day >= start && r.day <= end)
        .map(|r| Observation {
            date: dates::format(r.day),
            price: r.settle,
            open: r.open,
            high: r.high,
            low: r.low,
            volume: Some(r.volume),
            open_interest: Some(r.open_interest),
            implied_rate: None,
        })
        .collect();
    if latest {
        observations = observations.split_off(observations.len().saturating_sub(1));
    }
    if observations.is_empty() {
        return Err(format!(
            "no Cboe row for {} from {} to {}; the file covers {} to {}",
            contract.code(),
            dates::format(start),
            dates::format(end),
            dates::format(rows[0].day),
            dates::format(rows[rows.len() - 1].day)
        ));
    }
    let mut note = String::from(
        "The file is posted after the trading day, so the last day can be missing. The settlement of an \
         expired contract is the special opening quotation (SOQ) of the expiry day. A price of 0 in the file \
         for open, high or low means no trade; the program leaves the value out.",
    );
    if let (true, Some(last)) = (latest, observations.last()) {
        if last.date != dates::format(end) {
            note.push_str(&format!(" No row on {}; the row of {} is the last one before it.", dates::format(end), last.date));
        }
    }
    Ok(Series {
        symbol: contract.code(),
        name: format!("{} {}", contract.root.name, contract.label()),
        source: SOURCE.into(),
        url,
        price_type: PRICE_TYPE.into(),
        time_zone: "America/Chicago (trade date of CFE)".into(),
        note,
        live: None,
        observations,
    })
}

/// The curve of one trade date from the settlement file. Weekly contracts are in the file too.
pub fn curve(repo: &Path, requested: &str, day: i64) -> Result<Curve> {
    let rows = settlement(repo, day)?;
    let points: Vec<CurvePoint> = rows
        .into_iter()
        .filter(|r| r.product == "VX")
        .map(|r| CurvePoint {
            as_of: dates::format(day),
            price_type: "settlement",
            contract: if r.is_monthly() { format!("{} (monthly)", r.symbol) } else { format!("{} (weekly)", r.symbol) },
            expiry: Some(r.expiry),
            price: r.price,
            implied_rate: None,
        })
        .collect();
    if points.is_empty() {
        return Err(format!(
            "Cboe has no VX settlement for {}: no trading that day, or the date is older than the files that Cboe keeps (use history for one contract)",
            dates::format(day)
        ));
    }
    Ok(Curve {
        root: "VX".into(),
        name: "Cboe Volatility Index (VIX) futures".into(),
        date: dates::format(day),
        requested_date: requested.into(),
        source: SOURCE.into(),
        url: SETTLE_URL.replace("{date}", &dates::format(day)),
        price_type: PRICE_TYPE.into(),
        note: "A weekly contract can show the same price as the monthly contract that is next to it in this file (see HIDDEN-ASSUMPTIONS #73)."
            .into(),
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futures::facts::{parse_symbol, Symbol};

    const SETTLE: &str = "Product,Symbol,Expiration Date,Price\nVX,VX/X6,2026-11-18,24.25\nVX,VX47/X6,2026-11-24,21.9372\nVX,VX/Z6,2026-12-16,21.9372\nVA,VA/X6,2026-11-20,206.75\n";
    const HISTORY: &str = "Trade Date,Futures,Open,High,Low,Close,Settle,Change,Total Volume,EFP,Open Interest\n\
        2026-11-16,Z (Dec 2026),0.0000,21.0000,0.0000,0.0000,22.775,0,0,0,0\n\
        2026-11-17,Z (Dec 2026),22.7000,22.7000,22.6600,22.6600,22.6881,-0.1,2,0,1\n";

    #[test]
    fn settlement_parses() {
        let rows = parse_settlement(SETTLE.as_bytes()).unwrap();
        assert_eq!(rows.len(), 4);
        assert!(rows[0].is_monthly() && !rows[1].is_monthly() && !rows[3].is_monthly());
        assert_eq!(rows[2].price, 21.9372);
        assert!(parse_settlement(b"Product,Symbol,Expiration Date,Price\n").unwrap().is_empty());
    }

    #[test]
    fn settlement_rejects_bad_input() {
        assert!(parse_settlement(b"<html>").unwrap_err().contains("header"));
        let bad = |from: &str, to: &str| parse_settlement(SETTLE.replace(from, to).as_bytes()).unwrap_err();
        assert!(bad("24.25", "abc").contains("bad number"));
        assert!(bad("24.25", "-1").contains("not positive"));
        assert!(bad("2026-11-18", "18/11/2026").contains("bad expiry"));
        assert!(bad("VX/X6,2026-11-18", "VX/X6").contains("fields"));
    }

    #[test]
    fn history_parses_and_rejects() {
        let rows = parse_history(HISTORY.as_bytes()).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].open, rows[0].high, rows[0].low), (None, Some(21.0), None));
        assert_eq!(rows[1].contract, ('Z', 2026));
        assert_eq!(rows[1].settle, 22.6881);
        assert!(parse_history(b"x").unwrap_err().contains("header"));
        let header_only = HISTORY.lines().next().unwrap();
        assert!(parse_history(header_only.as_bytes()).unwrap_err().contains("no data rows"));
        assert!(parse_history(HISTORY.replace("2026-11-17", "2026-11-16").as_bytes()).unwrap_err().contains("not after"));
        assert!(parse_history(HISTORY.replace("Z (Dec 2026)", "Q (Dec)").as_bytes()).is_err());
        assert!(parse_history(HISTORY.replace("22.6881", "0").as_bytes()).unwrap_err().contains("not positive"));
    }

    #[test]
    fn standard_expiry_matches_the_files() {
        // From the Cboe settlement file of 2026-11-18.
        for (year, month, expiry) in [(2026, 11, "2026-11-18"), (2026, 12, "2026-12-16"), (2027, 1, "2027-01-20"), (2027, 2, "2027-02-17"), (2027, 7, "2027-07-21")] {
            assert_eq!(dates::format(standard_expiry(year, month)), expiry, "{year}-{month}");
        }
        let Symbol::Contract(c) = parse_symbol("VXZ26").unwrap() else { panic!() };
        assert_eq!(c.month, 12);
    }
}
