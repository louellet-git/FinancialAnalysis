//! Yahoo Finance chart endpoint: daily bars of CME and CBOT futures.
//!
//! One request gives the whole daily history of one symbol (from 2000-01-01). The
//! response is cached. A bar is a final bar when its time stamp is midnight of the
//! exchange time zone: Yahoo labels it with the trade date of the exchange. The last
//! bar of a running session has the time of the last trade; the program gives it as
//! `live` and never as a daily value.

use super::facts::{Contract, Root, Symbol};
use super::{implied_rate, load, text, time_utc, Live, Observation, Result, Series};
use mapcheck::dates;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

pub const URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?period1=946684800&period2={end}&interval=1d";
const MAX_AGE: Duration = Duration::from_secs(3600);
/// A daily bar older than this, compared with the date asked, is a gap of the source.
const MAX_STALE_DAYS: i64 = 7;

#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub day: i64,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: f64,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chart {
    pub symbol: String,
    pub name: String,
    pub time_zone: String,
    pub bars: Vec<Bar>,
    pub live: Option<(i64, f64)>,
    /// Rows with no close. Yahoo has symbols with many such rows (SOFR futures).
    pub null_closes: usize,
}

/// The Yahoo symbol: ES=F (Yahoo chooses the contract) or ESZ26.CME.
pub fn yahoo_symbol(symbol: &Symbol) -> Result<String> {
    let (root, contract): (&Root, Option<&Contract>) = match symbol {
        Symbol::Continuous(r) => (r, None),
        Symbol::Contract(c) => (c.root, Some(c)),
    };
    if root.yahoo_suffix.is_empty() {
        return Err(format!("{} has no Yahoo data; use the Cboe source", root.root));
    }
    Ok(match contract {
        None => format!("{}=F", root.root),
        Some(c) => format!("{}{}", c.code(), root.yahoo_suffix),
    })
}

fn url_for(yahoo: &str) -> String {
    URL.replace("{symbol}", yahoo).replace("{end}", &((dates::today() + 2) * 86_400).to_string())
}

/// Yahoo gives prices as 32-bit floats: 95.7025 comes as 95.70249938964844. The shortest text
/// that gives the same 32-bit float is the price of the exchange.
fn round(v: f64) -> f64 {
    (v as f32).to_string().parse().unwrap_or(v)
}

fn numbers(array: &Value, what: &str, n: usize) -> Result<Vec<Option<f64>>> {
    let items = array.as_array().ok_or_else(|| format!("Yahoo response has no {what} array"))?;
    if items.len() != n {
        return Err(format!("Yahoo {what} array has {} items; the time stamp array has {n}", items.len()));
    }
    items
        .iter()
        .map(|v| match v {
            Value::Null => Ok(None),
            Value::Number(x) => x.as_f64().filter(|f| f.is_finite()).map(Some).ok_or_else(|| format!("Yahoo {what}: bad number")),
            _ => Err(format!("Yahoo {what}: item is not a number")),
        })
        .collect()
}

/// The arrays of a chart reply, as Yahoo gives them. The bar rules (daily or intraday) are
/// the job of the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct RawChart {
    pub symbol: String,
    pub name: String,
    pub time_zone: String,
    /// `meta.gmtoffset`: the offset of today, in seconds.
    pub offset: i64,
    pub last_trade: Option<i64>,
    /// The regular trading periods of the range (`meta.tradingPeriods`): start and end, UTC seconds. Empty
    /// when the reply has none. A day with an early close has a short period.
    pub regular_periods: Vec<(i64, i64)>,
    /// UTC seconds of each row.
    pub stamps: Vec<i64>,
    pub open: Vec<Option<f64>>,
    pub high: Vec<Option<f64>>,
    pub low: Vec<Option<f64>>,
    pub close: Vec<Option<f64>>,
    pub volume: Vec<Option<f64>>,
}

/// Read the JSON of a chart reply. All price arrays have the length of the time stamp array.
pub fn parse_raw(bytes: &[u8]) -> Result<RawChart> {
    let root: Value = serde_json::from_str(&text(bytes, "Yahoo response")?).map_err(|e| format!("Yahoo response is not JSON: {e}"))?;
    let chart = root.get("chart").ok_or("Yahoo response has no 'chart'")?;
    if let Some(error) = chart.get("error").filter(|e| !e.is_null()) {
        return Err(format!("Yahoo error: {error}"));
    }
    let result = chart.get("result").and_then(|r| r.get(0)).ok_or("Yahoo response has no result")?;
    let meta = result.get("meta").ok_or("Yahoo response has no meta")?;
    let symbol = meta.get("symbol").and_then(Value::as_str).ok_or("Yahoo meta has no symbol")?.to_string();
    let offset = meta.get("gmtoffset").and_then(Value::as_i64).ok_or("Yahoo meta has no gmtoffset")?;
    let time_zone = meta.get("exchangeTimezoneName").and_then(Value::as_str).unwrap_or("unknown").to_string();
    let name = meta.get("shortName").and_then(Value::as_str).unwrap_or("").to_string();
    let last_trade = meta.get("regularMarketTime").and_then(Value::as_i64);
    let stamps: Vec<i64> = match result.get("timestamp") {
        None | Some(Value::Null) => Vec::new(),
        Some(v) => v.as_array().ok_or("Yahoo timestamp is not an array")?.iter().map(|t| t.as_i64().ok_or("Yahoo time stamp is not an integer")).collect::<std::result::Result<_, _>>()?,
    };
    let quote = result.get("indicators").and_then(|i| i.get("quote")).and_then(|q| q.get(0));
    let n = stamps.len();
    let (open, high, low, close, volume) = match quote {
        Some(q) if n > 0 => (
            numbers(&q["open"], "open", n)?,
            numbers(&q["high"], "high", n)?,
            numbers(&q["low"], "low", n)?,
            numbers(&q["close"], "close", n)?,
            numbers(&q["volume"], "volume", n)?,
        ),
        _ => (vec![], vec![], vec![], vec![], vec![]),
    };
    Ok(RawChart { symbol, name, time_zone, offset, last_trade, regular_periods: trading_periods(meta), stamps, open, high, low, close, volume })
}

/// `meta.tradingPeriods`: a list of days, each a list of `{start, end}` (or an object with the keys `pre`,
/// `regular` and `post`, each such a list).
fn trading_periods(meta: &Value) -> Vec<(i64, i64)> {
    fn collect(v: &Value, out: &mut Vec<(i64, i64)>) {
        match v {
            Value::Array(items) => items.iter().for_each(|i| collect(i, out)),
            Value::Object(_) => {
                if let (Some(a), Some(b)) = (v.get("start").and_then(Value::as_i64), v.get("end").and_then(Value::as_i64)) {
                    out.push((a, b));
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    match meta.get("tradingPeriods") {
        Some(obj @ Value::Object(_)) if obj.get("start").is_none() => {
            if let Some(r) = obj.get("regular") {
                collect(r, &mut out);
            }
        }
        Some(v) => collect(v, &mut out),
        None => {}
    }
    out
}

pub fn parse_chart(bytes: &[u8]) -> Result<Chart> {
    let RawChart { symbol, name, time_zone, offset, last_trade, stamps, open, high, low, close, volume, .. } = parse_raw(bytes)?;
    let n = stamps.len();
    let mut bars: Vec<Bar> = Vec::new();
    let mut live = None;
    let mut null_closes = 0;
    for (i, ts) in stamps.iter().enumerate() {
        let Some(c) = close[i] else {
            null_closes += 1;
            continue;
        };
        // meta.gmtoffset is the offset of today. A bar of the other half of the year is
        // midnight with one hour of daylight saving time more or less.
        let midnight = [0, 3600].into_iter().map(|shift| ts + offset + shift).find(|local| local.rem_euclid(86_400) == 0);
        if midnight.is_none() && i + 1 == n && last_trade == Some(*ts) {
            // The running session. It is not a daily value.
            live = Some((*ts, round(c)));
            continue;
        }
        // A bar of a day with a short session has the time of the open, for example 09:30.
        let local = midnight.unwrap_or(ts + offset);
        let day = local.div_euclid(86_400);
        if bars.last().is_some_and(|b| day <= b.day) {
            return Err(format!("Yahoo bar {i}: date {} is not after the date before it", dates::format(day)));
        }
        bars.push(Bar { day, open: open[i].map(round), high: high[i].map(round), low: low[i].map(round), close: round(c), volume: volume[i] });
    }
    Ok(Chart { symbol, name, time_zone, bars, live, null_closes })
}

/// The chart of one Yahoo symbol. `need_day`: a day that the caller needs. A cache file that
/// has a final bar on or after it is enough, whatever its age.
pub fn chart(repo: &Path, yahoo: &str, need_day: Option<i64>) -> Result<Chart> {
    let file = format!("yahoo_{}.json", yahoo.replace('=', "_"));
    load(repo, &file, &url_for(yahoo), MAX_AGE, parse_chart, |c: &Chart| {
        need_day.is_some_and(|d| c.bars.last().is_some_and(|b| b.day >= d))
    })
}

fn observation(bar: &Bar, rate: bool) -> Observation {
    Observation {
        date: dates::format(bar.day),
        price: bar.close,
        open: bar.open,
        high: bar.high,
        low: bar.low,
        volume: bar.volume,
        open_interest: None,
        implied_rate: rate.then(|| implied_rate(bar.close)),
    }
}

pub const DAILY: &str = "daily close";
pub const LAST_TRADE: &str = "last trade";
pub const PRICE_TYPE: &str = "daily close of the exchange trade date, as Yahoo gives it; it is the settlement price only if Yahoo takes the settlement (not confirmed: the exchange file is not available)";

/// Bars from `start` to `end` (day numbers, both inclusive). `latest`: only the last bar on or before `end`.
pub fn series(repo: &Path, symbol: &Symbol, start: i64, end: i64, latest: bool) -> Result<Series> {
    let yahoo = yahoo_symbol(symbol)?;
    let root = match symbol {
        Symbol::Continuous(r) => r,
        Symbol::Contract(c) => c.root,
    };
    let c = chart(repo, &yahoo, Some(end))?;
    let rate = root.implied_rate.is_some();
    let mut observations: Vec<Observation> = c.bars.iter().filter(|b| b.day >= start && b.day <= end).map(|b| observation(b, rate)).collect();
    if latest {
        observations = observations.split_off(observations.len().saturating_sub(1));
    }
    let mut note = String::from(
        "Yahoo gives no history for a contract that has expired. The volume of the newest daily bar can be a copy of the day before.",
    );
    if c.null_closes > 0 {
        note.push_str(&format!(" Yahoo has no close for {} of {} daily rows; the program leaves them out.", c.null_closes, c.null_closes + c.bars.len()));
    }
    // A last bar that is old is a gap of the source, not the value of the date.
    if latest && observations.last().is_some_and(|o| dates::parse(&o.date).is_some_and(|d| d < end - MAX_STALE_DAYS)) {
        note.push_str(&format!(
            " The last daily bar is {}, more than {MAX_STALE_DAYS} days before the date. The program does not give it as the value of the date.",
            observations[observations.len() - 1].date
        ));
        observations.clear();
    }
    if observations.is_empty() && c.live.is_none() {
        let range = match (c.bars.first(), c.bars.last()) {
            (Some(a), Some(b)) => format!("{} to {}", dates::format(a.day), dates::format(b.day)),
            _ => "no bars".to_string(),
        };
        return Err(format!("no Yahoo bar for {yahoo} from {} to {}; the bars cover {range}", dates::format(start), dates::format(end)));
    }
    if matches!(symbol, Symbol::Continuous(_)) {
        note = format!(
            "{yahoo} is a series that Yahoo builds from front contracts; the contract at the end is '{}'. The roll rule and the adjustment are not published; test before you join old and new values. {note}",
            c.name
        );
    }
    if let (true, Some(last)) = (latest, observations.last()) {
        if last.date != dates::format(end) {
            note.push_str(&format!(" No bar on {}; the bar of {} is the last one before it.", dates::format(end), last.date));
        }
    }
    Ok(Series {
        symbol: yahoo.clone(),
        name: c.name.clone(),
        source: "Yahoo Finance chart endpoint (CME Group data)".into(),
        url: url_for(&yahoo),
        price_type: PRICE_TYPE.into(),
        time_zone: c.time_zone.clone(),
        note,
        live: c.live.map(|(ts, price)| Live { time_utc: time_utc(ts), price }),
        observations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futures::facts::parse_symbol;

    /// Three final bars (midnight New York = 05:00 UTC) and one running-session bar.
    pub fn sample() -> String {
        r#"{"chart":{"result":[{"meta":{"symbol":"ESZ26.CME","shortName":"E-Mini S&P 500 Dec 26","gmtoffset":-18000,
        "exchangeTimezoneName":"America/New_York","regularMarketTime":1795059997},
        "timestamp":[1794805200,1794891600,1794978000,1795059997],
        "indicators":{"quote":[{"open":[7577.25,7442.25,null,7340.5],"high":[7614.25,7456.75,7434.75,7398.0],
        "low":[7420.25,7308.0,7305.25,7338.25],"close":[7439.0,7359.0,7340.5,7381.75],
        "volume":[1986666,2259363,2259363,72344]}]}}],"error":null}}"#
            .to_string()
    }

    #[test]
    fn parses_final_bars_and_live() {
        let c = parse_chart(sample().as_bytes()).unwrap();
        assert_eq!(c.bars.len(), 3);
        assert_eq!(dates::format(c.bars[0].day), "2026-11-16");
        assert_eq!(dates::format(c.bars[2].day), "2026-11-18");
        assert_eq!(c.bars[2].open, None);
        assert_eq!(c.bars[2].close, 7340.5);
        assert_eq!(c.live, Some((1795059997, 7381.75)));
        assert_eq!(c.time_zone, "America/New_York");
    }

    #[test]
    fn summer_time_bars_have_the_right_date() {
        // 04:00 UTC = midnight New York in summer; meta.gmtoffset is the winter value.
        let text = sample().replace("1794805200", "1758168000").replace("1794891600", "1758254400");
        let c = parse_chart(text.as_bytes()).unwrap();
        assert_eq!(dates::format(c.bars[0].day), "2025-09-18");
        assert_eq!(dates::format(c.bars[1].day), "2025-09-19");
    }

    #[test]
    fn short_session_bar_is_a_final_bar() {
        // 2026-11-27 14:30 UTC = 09:30 New York; the last bar is a running session.
        let text = r#"{"chart":{"result":[{"meta":{"symbol":"ZQF27.CBT","gmtoffset":-18000,"regularMarketTime":1795900000},
            "timestamp":[1795789800,1795900000],"indicators":{"quote":[{"open":[1,2],"high":[1,2],"low":[1,2],
            "close":[95.70249938964844,96],"volume":[3,4]}]}}],"error":null}}"#;
        let c = parse_chart(text.as_bytes()).unwrap();
        assert_eq!(c.bars.len(), 1);
        assert_eq!((dates::format(c.bars[0].day).as_str(), c.bars[0].close), ("2026-11-27", 95.7025));
        assert_eq!(c.live, Some((1795900000, 96.0)));
    }

    #[test]
    fn a_symbol_with_gaps_is_reported() {
        let text = sample().replace("7359.0", "null");
        let c = parse_chart(text.as_bytes()).unwrap();
        assert_eq!((c.bars.len(), c.null_closes), (2, 1));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_chart(b"<html>").unwrap_err().contains("not JSON"));
        let e = r#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found"}}}"#;
        assert!(parse_chart(e.as_bytes()).unwrap_err().contains("Not Found"));
        assert!(parse_chart(sample().replace("7439.0", "\"x\"").as_bytes()).unwrap_err().contains("not a number"));
        assert!(parse_chart(sample().replace("[1986666,", "[").as_bytes()).unwrap_err().contains("items"));
        // Dates that do not rise.
        assert!(parse_chart(sample().replace("1794891600", "1794805200").as_bytes()).unwrap_err().contains("not after"));
    }

    #[test]
    fn symbols_for_yahoo() {
        assert_eq!(yahoo_symbol(&parse_symbol("ES").unwrap()).unwrap(), "ES=F");
        assert_eq!(yahoo_symbol(&parse_symbol("ZNZ26").unwrap()).unwrap(), "ZNZ26.CBT");
        assert_eq!(yahoo_symbol(&parse_symbol("SR3H27").unwrap()).unwrap(), "SR3H27.CME");
        assert!(yahoo_symbol(&parse_symbol("VXZ26").unwrap()).unwrap_err().contains("Cboe"));
    }

    #[test]
    fn rate_futures_give_a_rate() {
        let bar = Bar { day: 20_000, open: None, high: None, low: None, close: 95.702, volume: None };
        assert_eq!(observation(&bar, true).implied_rate, Some(0.04298));
        assert_eq!(observation(&bar, false).implied_rate, None);
    }
}
