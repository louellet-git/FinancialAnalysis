//! Analyst consensus, price targets, earnings surprises and the earnings calendar from the Nasdaq site API.
//!
//! The data vendor that the Nasdaq pages show is Zacks. The endpoints are the ones that the pages call. This is
//! **not a published API**: the replies can change without a notice. The program gives its real name, asks with
//! no key, no cookie and no login, and does not imitate a browser. It stops at HTTP 429, at HTTP 403 and at a reply
//! that is not JSON (a block page): it waits one time, tries again, and then stops the source for the run.
//!
//! | Endpoint (`https://api.nasdaq.com/api/...`) | Data | Requests |
//! |---|---|---|
//! | `analyst/<T>/earnings-forecast` | consensus EPS of the next quarters and years: mean, high, low, number of analysts, revisions of 4 weeks | necessary |
//! | `company/<T>/earnings-surprise` | the last 4 reported quarters: report date, EPS, consensus, surprise | necessary |
//! | `analyst/<T>/targetprice` | price target (mean, low, high), the number of buy, hold and sell ratings, 13 months of history | necessary |
//! | `analyst/<T>/estimate-momentum` | the consensus EPS now, 1 week ago and 1 month ago | optional (`--momentum`) |
//! | `calendar/earnings?date=D` | all companies that report on the date: time of day, EPS forecast, number of estimates | 1 for each day (`fa earnings calendar`) |
//!
//! Not used on purpose: `quote/<T>/eps` (the same numbers as the first two), `analyst/<T>/ratings` (names of
//! brokers only) and `company/<T>/revenue` (reported values, no estimates). **The source has no revenue estimates.**
//!
//! Text number forms: `"$0.64"`, `"4.71"`, `"$1,010,441,910,000"`, `"N/A"`, `""`. A value that does not parse is
//! missing (`None`), and it is never zero. A symbol with no data gives HTTP 200 with `data: null` and code 1002
//! (`No record found`), or a table of `null`; an unknown symbol gives code 1001 (`Symbol not exists`).
//!
//! The parsers are functions of the reply bytes, so the tests need no network.

use super::consensus::ConsensusEstimates;
use super::price_source::classify;
use super::price_source::FetchError;
use super::{parse_date, Ordered};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

pub const BASE: &str = "https://api.nasdaq.com/api";
pub const SOURCE: &str = "nasdaq";

/// The symbol of the site: a share class has a dot (`BRK.B`). `BRK-B` gives `Symbol not exists`,
/// and `BRK/B` is the name that the site uses in its replies.
pub fn symbol(ticker: &str) -> String {
    ticker.trim().to_uppercase().replace(['-', '/'], ".")
}

/// The same company in two spellings (`BRK-B`, `BRK.B`, `BRK/B`).
pub fn same_symbol(a: &str, b: &str) -> bool {
    symbol(a) == symbol(b)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endpoint {
    Forecast,
    Surprise,
    Targets,
    Momentum,
}

impl Endpoint {
    pub const DEFAULT: [Endpoint; 3] = [Endpoint::Forecast, Endpoint::Surprise, Endpoint::Targets];

    pub fn name(self) -> &'static str {
        match self {
            Endpoint::Forecast => "forecast",
            Endpoint::Surprise => "surprise",
            Endpoint::Targets => "targets",
            Endpoint::Momentum => "momentum",
        }
    }

    pub fn from_name(text: &str) -> Option<Endpoint> {
        [Endpoint::Forecast, Endpoint::Surprise, Endpoint::Targets, Endpoint::Momentum].into_iter().find(|e| e.name() == text)
    }

    pub fn url(self, ticker: &str) -> String {
        let s = symbol(ticker);
        match self {
            Endpoint::Forecast => format!("{BASE}/analyst/{s}/earnings-forecast"),
            Endpoint::Surprise => format!("{BASE}/company/{s}/earnings-surprise"),
            Endpoint::Targets => format!("{BASE}/analyst/{s}/targetprice"),
            Endpoint::Momentum => format!("{BASE}/analyst/{s}/estimate-momentum"),
        }
    }
}

pub fn calendar_url(date: &str) -> String {
    format!("{BASE}/calendar/earnings?date={date}")
}

// ---------------------------------------------------------------- text forms

/// A number from a JSON number or from text with `$`, `,` or `%`. `N/A`, `--` and the empty text give `None`.
pub fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64().filter(|x| x.is_finite()),
        Value::String(s) => {
            let t: String = s.trim().chars().filter(|c| !matches!(c, '$' | ',' | '%')).collect();
            t.parse::<f64>().ok().filter(|x| x.is_finite())
        }
        _ => None,
    }
}

fn field(row: &Value, key: &str) -> Option<f64> {
    row.get(key).and_then(number)
}

fn text(row: &Value, key: &str) -> String {
    row.get(key).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

const MONTHS: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];

/// `Jun 2027`, `Sep/2026` or `Fiscal Quarter End Dec 2026` to `2027-06`. The last two words decide.
pub fn fiscal_month(text: &str) -> Option<String> {
    let words: Vec<&str> = text.split(|c: char| c.is_whitespace() || c == '/').filter(|w| !w.is_empty()).collect();
    let [.., month, year] = words.as_slice() else { return None };
    let m = MONTHS.iter().position(|x| month.to_lowercase().starts_with(x))? + 1;
    let y: i32 = year.parse().ok().filter(|y| (1990..2100).contains(y))?;
    Some(format!("{y:04}-{m:02}"))
}

/// `7/29/2026` and `07/29/2026` to `2026-07-29`.
pub fn us_date(text: &str) -> Option<String> {
    let p: Vec<&str> = text.trim().split('/').collect();
    let [m, d, y] = p.as_slice() else { return None };
    let iso = format!("{y}-{:0>2}-{:0>2}", m, d);
    parse_date(&iso).map(|_| iso)
}

/// The data part of a reply. `Value::Null` means that the source has no record for the symbol.
/// An error code is an `Err`; so is a text that is not JSON (a block page).
pub fn data_of(bytes: &[u8]) -> Result<Value, String> {
    let root: Value = serde_json::from_slice(bytes).map_err(|e| format!("Nasdaq reply is not JSON: {e}"))?;
    if let Some(first) = root["status"]["bCodeMessage"].as_array().and_then(|a| a.first()) {
        let message: String = first["errorMessage"].as_str().unwrap_or("error").chars().take(200).collect();
        // Code 1002 is "No record found": the source has no data for this symbol.
        if first["code"].as_i64() == Some(1002) {
            return Ok(Value::Null);
        }
        return Err(format!("Nasdaq error: {message}"));
    }
    if let Some(code) = root["status"]["rCode"].as_i64().filter(|c| *c != 200) {
        return Err(format!("Nasdaq status {code}"));
    }
    Ok(root["data"].clone())
}

fn rows_of<'a>(table: &'a Value, key: &str) -> &'a [Value] {
    table.get(key).and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

// ---------------------------------------------------------------- forecast

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForecastRow {
    /// `YYYY-MM`.
    pub fiscal_end: String,
    pub consensus: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub estimates: Option<f64>,
    /// Revisions up and down in the last 4 weeks.
    pub up: Option<f64>,
    pub down: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Forecast {
    pub quarterly: Vec<ForecastRow>,
    pub yearly: Vec<ForecastRow>,
}

impl Forecast {
    pub fn is_empty(&self) -> bool {
        self.quarterly.is_empty() && self.yearly.is_empty()
    }
}

fn forecast_rows(table: &Value) -> Vec<ForecastRow> {
    let mut out = Vec::new();
    for r in rows_of(table, "rows") {
        // A row with no period or no consensus is not a forecast.
        let (Some(end), Some(consensus)) = (r.get("fiscalEnd").and_then(Value::as_str).and_then(fiscal_month), field(r, "consensusEPSForecast")) else { continue };
        out.push(ForecastRow { fiscal_end: end, consensus: Some(consensus), high: field(r, "highEPSForecast"), low: field(r, "lowEPSForecast"), estimates: field(r, "noOfEstimates"), up: field(r, "up"), down: field(r, "down") });
    }
    out
}

/// `analyst/<T>/earnings-forecast`. A symbol with no forecast gives `quarterlyForecast: null`: an empty result.
pub fn parse_forecast(bytes: &[u8]) -> Result<Forecast, String> {
    let data = data_of(bytes)?;
    if data.is_null() {
        return Ok(Forecast::default());
    }
    Ok(Forecast { quarterly: forecast_rows(&data["quarterlyForecast"]), yearly: forecast_rows(&data["yearlyForecast"]) })
}

// ---------------------------------------------------------------- surprises

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurpriseRow {
    /// `YYYY-MM`.
    pub fiscal_qtr_end: String,
    /// The report date, `YYYY-MM-DD`.
    pub date: String,
    pub eps: Option<f64>,
    pub consensus: Option<f64>,
    /// The surprise in percent, as the source shows it (`3.82`).
    pub pct: Option<f64>,
}

/// `company/<T>/earnings-surprise`, newest first. A row with a bad date is skipped.
pub fn parse_surprises(bytes: &[u8]) -> Result<Vec<SurpriseRow>, String> {
    let data = data_of(bytes)?;
    let mut out = Vec::new();
    for r in rows_of(&data["earningsSurpriseTable"], "rows") {
        let Some(date) = r.get("dateReported").and_then(Value::as_str).and_then(us_date) else { continue };
        out.push(SurpriseRow {
            fiscal_qtr_end: r.get("fiscalQtrEnd").and_then(Value::as_str).and_then(fiscal_month).unwrap_or_default(),
            date,
            eps: field(r, "eps"),
            consensus: field(r, "consensusForecast"),
            pct: field(r, "percentageSurprise"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------- targets

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetPoint {
    pub date: String,
    pub target: Option<f64>,
    pub buy: Option<f64>,
    pub hold: Option<f64>,
    pub sell: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Targets {
    pub mean: Option<f64>,
    pub low: Option<f64>,
    pub high: Option<f64>,
    pub buy: Option<f64>,
    pub hold: Option<f64>,
    pub sell: Option<f64>,
    /// The monthly history, oldest first.
    pub history: Vec<TargetPoint>,
}

impl Targets {
    pub fn is_empty(&self) -> bool {
        self.mean.is_none() && self.low.is_none() && self.high.is_none() && self.buy.is_none() && self.history.is_empty()
    }
}

/// `analyst/<T>/targetprice`.
pub fn parse_targets(bytes: &[u8]) -> Result<Targets, String> {
    let data = data_of(bytes)?;
    if data.is_null() {
        return Ok(Targets::default());
    }
    let o = &data["consensusOverview"];
    let mut history = Vec::new();
    for p in data["historicalConsensus"].as_array().map_or(&[][..], Vec::as_slice) {
        let Some(date) = p["z"]["date"].as_str().and_then(us_date) else { continue };
        history.push(TargetPoint { date, target: field(p, "y"), buy: field(&p["z"], "buy"), hold: field(&p["z"], "hold"), sell: field(&p["z"], "sell") });
    }
    history.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(Targets { mean: field(o, "priceTarget"), low: field(o, "lowPriceTarget"), high: field(o, "highPriceTarget"), buy: field(o, "buy"), hold: field(o, "hold"), sell: field(o, "sell"), history })
}

// ---------------------------------------------------------------- momentum

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Momentum {
    pub qtr_now: Option<f64>,
    pub qtr_1w: Option<f64>,
    pub qtr_1m: Option<f64>,
    pub yr_now: Option<f64>,
    pub yr_1w: Option<f64>,
    pub yr_1m: Option<f64>,
}

/// `analyst/<T>/estimate-momentum`.
pub fn parse_momentum(bytes: &[u8]) -> Result<Momentum, String> {
    let data = data_of(bytes)?;
    let c = &data["changeInConsensus"];
    Ok(Momentum {
        qtr_now: field(&c["currentData"], "qtrMean"),
        qtr_1w: field(&c["weekData"], "qtrMean"),
        qtr_1m: field(&c["monthData"], "qtrMean"),
        yr_now: field(&c["currentData"], "yrMean"),
        yr_1w: field(&c["weekData"], "yrMean"),
        yr_1m: field(&c["monthData"], "yrMean"),
    })
}

// ---------------------------------------------------------------- calendar

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarRow {
    pub symbol: String,
    pub name: String,
    /// `pre-market`, `after-hours` or `not-supplied`, as the site says (the text after `time-`).
    pub time: String,
    /// `YYYY-MM` of the fiscal quarter that the company reports.
    pub fiscal_quarter: String,
    pub eps_forecast: Option<f64>,
    pub estimates: Option<f64>,
    /// The report date of the year before, `YYYY-MM-DD`.
    pub last_year_report: Option<String>,
    pub last_year_eps: Option<f64>,
    pub market_cap: Option<f64>,
}

/// `calendar/earnings?date=D`: all companies of the date. A day with no report (a weekend) has `rows: null`. A day
/// that is beyond the horizon of the vendor (about 8 weeks) has `data: null`, code 1002 and HTTP 200: an empty day.
pub fn parse_calendar(bytes: &[u8]) -> Result<Vec<CalendarRow>, String> {
    let data = data_of(bytes)?;
    if data.is_null() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for r in rows_of(&data, "rows") {
        let symbol = text(r, "symbol");
        if symbol.is_empty() {
            continue;
        }
        out.push(CalendarRow {
            symbol,
            name: text(r, "name"),
            time: text(r, "time").trim_start_matches("time-").to_string(),
            fiscal_quarter: fiscal_month(&text(r, "fiscalQuarterEnding")).unwrap_or_default(),
            eps_forecast: field(r, "epsForecast"),
            estimates: field(r, "noOfEsts"),
            last_year_report: us_date(&text(r, "lastYearRptDt")),
            last_year_eps: field(r, "lastYearEPS"),
            market_cap: field(r, "marketCap"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------- the request layer

/// A request function: URL in, body out. The tests give their own.
pub type Get<'a> = &'a dyn Fn(&str) -> Result<Vec<u8>, String>;

#[derive(Debug, Clone, PartialEq)]
pub enum SourceError {
    /// HTTP 429, HTTP 403 or a block page, two times. The source is stopped for the run.
    Blocked(String),
    Failed(String),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::Blocked(m) | SourceError::Failed(m) => write!(f, "{m}"),
        }
    }
}

/// The client of the run. It counts the requests, waits one time at a block, and stops after the second block.
pub struct Client<'a> {
    get: Get<'a>,
    retry_wait: Duration,
    pub blocked: Option<String>,
    pub requests: usize,
}

impl<'a> Client<'a> {
    pub fn new(get: Get<'a>, retry_wait: Duration) -> Client<'a> {
        Client { get, retry_wait, blocked: None, requests: 0 }
    }

    /// One reply body. A block (HTTP 429, HTTP 403, or a body that is not a JSON object) is tried again one time after
    /// `retry_wait`; the second block stops the source (`blocked`), and the following calls make no request.
    pub fn request(&mut self, url: &str) -> Result<Vec<u8>, SourceError> {
        if let Some(why) = &self.blocked {
            return Err(SourceError::Blocked(format!("source stopped for this run ({why})")));
        }
        let mut why = String::new();
        for attempt in 0..2 {
            if attempt == 1 {
                std::thread::sleep(self.retry_wait);
            }
            self.requests += 1;
            match (self.get)(url) {
                Ok(body) if body.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{') => return Ok(body),
                Ok(_) => why = "the reply is not JSON (a block page)".to_string(),
                Err(e) => match classify(e) {
                    FetchError::RateLimited(m) => why = m,
                    FetchError::Failed(m) if m.starts_with("HTTP 403 ") => why = m,
                    FetchError::Failed(m) => return Err(SourceError::Failed(m)),
                },
            }
        }
        self.blocked = Some(why.clone());
        Err(SourceError::Blocked(why))
    }
}

/// An `Ok` for a parser that reads the bytes of a request; a reply that is not JSON is a block, not a failure.
pub fn fetch<T>(client: &mut Client, url: &str, parse: fn(&[u8]) -> Result<T, String>) -> Result<T, SourceError> {
    let body = client.request(url)?;
    parse(&body).map_err(SourceError::Failed)
}

// ---------------------------------------------------------------- into the snapshot

fn period_keys(kind: char, n: usize) -> Vec<String> {
    (0..n).map(|i| if i == 0 { format!("0{kind}") } else { format!("+{i}{kind}") }).collect()
}

/// The EPS table of a snapshot: `0q`, `+1q`, ... and `0y`, `+1y`, ... Each period has avg, low, high,
/// numberOfAnalysts, up and down (the revisions of 4 weeks). The first row of each table is the first
/// period that is not reported. `period_ends` has the end month of each period.
pub fn apply_forecast(s: &mut ConsensusEstimates, f: &Forecast) {
    let mut table = Vec::new();
    let mut ends = Vec::new();
    for (kind, rows) in [('q', &f.quarterly), ('y', &f.yearly)] {
        for (key, r) in period_keys(kind, rows.len()).into_iter().zip(rows.iter()) {
            let metrics = [("avg", r.consensus), ("low", r.low), ("high", r.high), ("numberOfAnalysts", r.estimates), ("up", r.up), ("down", r.down)];
            table.push((key.clone(), Ordered(metrics.iter().map(|(k, v)| (k.to_string(), *v)).collect())));
            ends.push((key, r.fiscal_end.clone()));
        }
    }
    s.earnings_estimate = Ordered(table);
    s.period_ends = Ordered(ends);
    mark(s, Endpoint::Forecast);
}

/// The price target (`mean`, `low`, `high`) and the ratings (`buy`, `hold`, `sell`). The source has no `current`
/// price and no `median`: these stay absent.
pub fn apply_targets(s: &mut ConsensusEstimates, t: &Targets) {
    s.analyst_price_targets = Ordered(vec![("mean".into(), t.mean), ("low".into(), t.low), ("high".into(), t.high)].into_iter().filter(|(_, v): &(String, Option<f64>)| v.is_some()).collect());
    s.ratings = Ordered(vec![("buy".into(), t.buy), ("hold".into(), t.hold), ("sell".into(), t.sell)].into_iter().filter(|(_, v): &(String, Option<f64>)| v.is_some()).collect());
    mark(s, Endpoint::Targets);
}

pub fn apply_momentum(s: &mut ConsensusEstimates, m: &Momentum) {
    let all = [("qtr_now", m.qtr_now), ("qtr_1w", m.qtr_1w), ("qtr_1m", m.qtr_1m), ("yr_now", m.yr_now), ("yr_1w", m.yr_1w), ("yr_1m", m.yr_1m)];
    s.consensus_change = Ordered(all.iter().filter(|(_, v)| v.is_some()).map(|(k, v)| (k.to_string(), *v)).collect());
    mark(s, Endpoint::Momentum);
}

fn mark(s: &mut ConsensusEstimates, e: Endpoint) {
    if !s.endpoints.iter().any(|x| x == e.name()) {
        s.endpoints.push(e.name().to_string());
    }
}

/// Does the snapshot have the data of this endpoint?
pub fn has_endpoint(s: &ConsensusEstimates, e: Endpoint) -> bool {
    s.endpoints.iter().any(|x| x == e.name())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORECAST: &str = include_str!("../../tests/fixtures/market/nasdaq/forecast_MSFT.json");
    const SURPRISE: &str = include_str!("../../tests/fixtures/market/nasdaq/surprise_MSFT.json");
    const TARGETS: &str = include_str!("../../tests/fixtures/market/nasdaq/targetprice_MSFT.json");
    const MOMENTUM: &str = include_str!("../../tests/fixtures/market/nasdaq/momentum_MSFT.json");
    const CALENDAR: &str = include_str!("../../tests/fixtures/market/nasdaq/calendar_2026-11-20.json");
    const WEEKEND: &str = include_str!("../../tests/fixtures/market/nasdaq/calendar_weekend.json");
    const BAD_SYMBOL: &str = include_str!("../../tests/fixtures/market/nasdaq/error_symbol.json");
    const UNAVAILABLE: &str = include_str!("../../tests/fixtures/market/nasdaq/forecast_unavailable.json");
    const NO_RECORD: &str = include_str!("../../tests/fixtures/market/nasdaq/no_record.json");

    #[test]
    fn text_number_forms() {
        let n = |s: &str| number(&Value::String(s.to_string()));
        assert_eq!(n("$0.64"), Some(0.64));
        assert_eq!(n("4.71"), Some(4.71));
        assert_eq!(n("$1,010,441,910,000"), Some(1_010_441_910_000.0));
        assert_eq!(n("-3.5%"), Some(-3.5));
        // A text that is not a number is missing, and it is not zero.
        for bad in ["N/A", "", "--", "  ", "abc"] {
            assert_eq!(n(bad), None, "{bad:?}");
        }
        assert_eq!(number(&Value::Null), None);
        assert_eq!(number(&serde_json::json!(4.86)), Some(4.86));
    }

    #[test]
    fn dates_and_symbols() {
        assert_eq!(fiscal_month("Jun 2027").as_deref(), Some("2027-06"));
        assert_eq!(fiscal_month("Sep/2026").as_deref(), Some("2026-09"));
        assert_eq!(fiscal_month("Fiscal Quarter End Dec 2026").as_deref(), Some("2026-12"));
        assert_eq!(fiscal_month("N/A"), None);
        assert_eq!(us_date("7/29/2026").as_deref(), Some("2026-07-29"));
        assert_eq!(us_date("10/28/2026").as_deref(), Some("2026-10-28"));
        assert_eq!(us_date("13/40/2026"), None);
        assert_eq!(us_date("N/A"), None);
        assert_eq!((symbol("brk-b"), symbol("BRK/B"), symbol("MSFT")), ("BRK.B".to_string(), "BRK.B".to_string(), "MSFT".to_string()));
        assert!(same_symbol("BRK-B", "BRK.B"));
        assert_eq!(Endpoint::Forecast.url("BRK-B"), "https://api.nasdaq.com/api/analyst/BRK.B/earnings-forecast");
        assert_eq!(calendar_url("2026-11-20"), "https://api.nasdaq.com/api/calendar/earnings?date=2026-11-20");
    }

    #[test]
    fn forecast_of_the_real_reply() {
        let f = parse_forecast(FORECAST.as_bytes()).unwrap();
        assert_eq!((f.quarterly.len(), f.yearly.len()), (5, 4));
        let q0 = &f.quarterly[0];
        assert_eq!((q0.fiscal_end.as_str(), q0.consensus, q0.high, q0.low, q0.estimates, q0.up, q0.down), ("2026-12", Some(4.86), Some(4.97), Some(4.81), Some(14.0), Some(7.0), Some(4.0)));
        assert_eq!((f.yearly[0].fiscal_end.as_str(), f.yearly[0].consensus), ("2027-06", Some(19.91)));
    }

    #[test]
    fn forecast_without_data_and_with_errors() {
        // The site answers HTTP 200 with null tables: an empty result, not an error.
        assert!(parse_forecast(UNAVAILABLE.as_bytes()).unwrap().is_empty());
        // Code 1002: no record. The same for the other endpoints.
        assert!(parse_forecast(NO_RECORD.as_bytes()).unwrap().is_empty());
        assert!(parse_surprises(NO_RECORD.as_bytes()).unwrap().is_empty());
        assert!(parse_targets(NO_RECORD.as_bytes()).unwrap().is_empty());
        // `data: null` with status 200 and no message: no data.
        let null_data = br#"{"data":null,"message":null,"status":{"rCode":200,"bCodeMessage":null,"developerMessage":null}}"#;
        assert!(parse_forecast(null_data).unwrap().is_empty());
        assert!(parse_calendar(null_data).unwrap().is_empty(), "a day beyond the horizon of the calendar is empty, not an error");
        // An unknown symbol (code 1001) and an error status are errors.
        assert!(parse_forecast(BAD_SYMBOL.as_bytes()).unwrap_err().contains("Symbol not exists"));
        assert!(parse_forecast(br#"{"data":{},"status":{"rCode":503}}"#).unwrap_err().contains("503"));
        // A block page is not JSON.
        assert!(parse_forecast(b"<html>Access Denied</html>").unwrap_err().contains("not JSON"));
        // A table with no rows, and rows with a missing value.
        assert!(parse_forecast(br#"{"data":{"quarterlyForecast":{"rows":null},"yearlyForecast":{"rows":[]}},"status":{"rCode":200}}"#).unwrap().is_empty());
        let partial = br#"{"data":{"yearlyForecast":{"rows":[{"fiscalEnd":"Dec 2026","consensusEPSForecast":2.5,"highEPSForecast":"N/A","noOfEstimates":null},{"fiscalEnd":"Dec 2027","consensusEPSForecast":null}]}},"status":{"rCode":200}}"#;
        let f = parse_forecast(partial).unwrap();
        assert_eq!(f.yearly.len(), 1, "a row with no consensus is skipped");
        assert_eq!((f.yearly[0].high, f.yearly[0].low, f.yearly[0].estimates), (None, None, None));
    }

    #[test]
    fn surprises_of_the_real_reply() {
        let rows = parse_surprises(SURPRISE.as_bytes()).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0], SurpriseRow { fiscal_qtr_end: "2026-09".into(), date: "2026-10-28".into(), eps: Some(4.89), consensus: Some(4.71), pct: Some(3.82) });
        assert_eq!(rows[1].date, "2026-07-29", "a date with one digit month and day");
        // An empty table, and a row with no consensus.
        assert!(parse_surprises(br#"{"data":{"earningsSurpriseTable":{"rows":[]}},"status":{"rCode":200}}"#).unwrap().is_empty());
        let one = br#"{"data":{"earningsSurpriseTable":{"rows":[{"fiscalQtrEnd":"Sep 2026","dateReported":"10/28/2026","eps":1.5,"consensusForecast":"N/A","percentageSurprise":"--"},{"dateReported":"bad","eps":1}]}},"status":{"rCode":200}}"#;
        let rows = parse_surprises(one).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].eps, rows[0].consensus, rows[0].pct), (Some(1.5), None, None));
    }

    #[test]
    fn targets_of_the_real_reply() {
        let t = parse_targets(TARGETS.as_bytes()).unwrap();
        assert_eq!((t.mean, t.low, t.high, t.buy, t.hold, t.sell), (Some(632.48), Some(440.0), Some(750.0), Some(34.0), Some(4.0), Some(0.0)));
        assert_eq!(t.history.len(), 13);
        assert_eq!((t.history[0].date.as_str(), t.history[0].target, t.history[0].buy), ("2025-11-01", Some(517.81), Some(34.0)));
        assert!(t.history.windows(2).all(|w| w[0].date < w[1].date), "oldest first");
        // A symbol with no overview has an empty result, not a zero target.
        let none = parse_targets(br#"{"data":{"consensusOverview":null,"historicalConsensus":[]},"status":{"rCode":200}}"#).unwrap();
        assert!(none.is_empty() && none.mean.is_none());
    }

    #[test]
    fn momentum_of_the_real_reply() {
        let m = parse_momentum(MOMENTUM.as_bytes()).unwrap();
        assert_eq!((m.qtr_now, m.qtr_1w, m.qtr_1m, m.yr_now, m.yr_1m), (Some(4.86), Some(4.86), Some(4.84), Some(19.91), Some(19.65)));
    }

    #[test]
    fn calendar_of_the_real_reply() {
        let rows = parse_calendar(CALENDAR.as_bytes()).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0], CalendarRow {
            symbol: "BSP".into(), name: "Bending Spoons S.p.A.".into(), time: "pre-market".into(), fiscal_quarter: "2026-09".into(),
            eps_forecast: Some(0.25), estimates: Some(1.0), last_year_report: None, last_year_eps: None, market_cap: Some(29_716_768_800.0),
        });
        assert_eq!(rows[1].last_year_report.as_deref(), Some("2025-11-21"));
        assert_eq!((rows[1].eps_forecast, rows[1].last_year_eps), (Some(1.24), Some(1.16)));
        // An empty `epsForecast` is missing, and the share class symbol keeps its dot.
        assert_eq!((rows[3].symbol.as_str(), rows[3].eps_forecast), ("MOG.B", None));
        // A weekend: `rows: null`.
        assert!(parse_calendar(WEEKEND.as_bytes()).unwrap().is_empty());
    }

    #[test]
    fn snapshot_from_the_endpoints_has_the_yahoo_period_keys() {
        let mut s = ConsensusEstimates::empty("msft", "2026-11-19", SOURCE);
        apply_forecast(&mut s, &parse_forecast(FORECAST.as_bytes()).unwrap());
        apply_targets(&mut s, &parse_targets(TARGETS.as_bytes()).unwrap());
        apply_momentum(&mut s, &parse_momentum(MOMENTUM.as_bytes()).unwrap());
        let keys: Vec<&str> = s.earnings_estimate.0.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["0q", "+1q", "+2q", "+3q", "+4q", "0y", "+1y", "+2y", "+3y"]);
        assert_eq!(s.eps("0y", "avg"), Some(19.91));
        assert_eq!(s.eps("+1y", "numberOfAnalysts"), Some(17.0));
        assert_eq!(s.period_ends.0[5], ("0y".to_string(), "2027-06".to_string()));
        assert_eq!(s.analyst_price_targets.0.len(), 3, "no current price, no median: the source has none");
        assert_eq!(s.ratings.0[0], ("buy".to_string(), Some(34.0)));
        assert!(s.revenue_estimate.is_empty(), "the source has no revenue estimates");
        assert_eq!(s.endpoints, ["forecast", "targets", "momentum"]);
        // The growth of the consensus EPS: FY2028 over FY2027 is 23.81 / 19.91 - 1.
        let g = s.eps_growth();
        assert_eq!(g[0].0, "+1y");
        assert!((g[0].1 - (23.81 / 19.91 - 1.0)).abs() < 1e-12);
        assert!((s.eps_next_four_quarters().unwrap() - (4.86 + 4.92 + 5.21 + 5.7)).abs() < 1e-12);
        // The file form reads back to the same snapshot.
        let text = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<ConsensusEstimates>(&text).unwrap(), s);
    }

    #[test]
    fn a_block_is_tried_once_more_and_then_the_source_stops() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        let get = |_: &str| -> Result<Vec<u8>, String> {
            calls.set(calls.get() + 1);
            Err("HTTP 429 for https://api.nasdaq.com/x".to_string())
        };
        let mut c = Client::new(&get, Duration::ZERO);
        assert!(matches!(c.request("u"), Err(SourceError::Blocked(_))));
        assert_eq!(calls.get(), 2, "one try and one second try");
        // After the stop there is no request at all.
        assert!(matches!(c.request("u"), Err(SourceError::Blocked(m)) if m.contains("stopped")));
        assert_eq!(calls.get(), 2);
        // A block page (HTML with HTTP 200) is a block too.
        let page = |_: &str| -> Result<Vec<u8>, String> { Ok(b"<html>captcha</html>".to_vec()) };
        let mut c = Client::new(&page, Duration::ZERO);
        assert!(matches!(c.request("u"), Err(SourceError::Blocked(m)) if m.contains("block page")));
        // A second try that works gives the body; any other error is a failure of the ticker, not a block.
        let n = Cell::new(0);
        let flaky = |_: &str| -> Result<Vec<u8>, String> {
            n.set(n.get() + 1);
            if n.get() == 1 { Err("HTTP 429 for x".into()) } else { Ok(br#"{"data":null}"#.to_vec()) }
        };
        let mut c = Client::new(&flaky, Duration::ZERO);
        assert!(c.request("u").is_ok() && c.blocked.is_none());
        let missing = |_: &str| -> Result<Vec<u8>, String> { Err("HTTP 404 for x".into()) };
        let mut c = Client::new(&missing, Duration::ZERO);
        assert!(matches!(c.request("u"), Err(SourceError::Failed(_))) && c.blocked.is_none() && c.requests == 1);
    }
}
