//! Forward rates and zero-coupon yields from the Federal Reserve Board dataset of
//! Gürkaynak, Sack and Wright (GSW), "The U.S. Treasury Yield Curve: 1961 to the Present".
//!
//! The Board fits a Svensson curve to Treasury prices each business day and publishes
//! the result. This module reads the published values. It does not compute any curve.
//! Each rate is a decimal (0.0507 = 5.07%). The Board publishes percent.

use crate::CheckResult;
use mapcheck::dates;
use mapcheck::http::Client;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const GSW_URL: &str = "https://www.federalreserve.gov/data/yield-curve-tables/feds200628.csv";
pub const SOURCE: &str = "Federal Reserve Board, GSW nominal Treasury yield curve (feds200628)";
const TREASURY_CSV: &str = "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/\
daily-treasury-rates.csv/{year}/all?type=daily_treasury_yield_curve&field_tdr_date_value={year}&page&_format=csv";
/// The Board updates the file each business day. A cached file is fresh for 12 hours.
const MAX_AGE: Duration = Duration::from_secs(12 * 3600);
const MAX_YEARS: u32 = 30;
const ONE_YEAR_STARTS: [u32; 3] = [1, 4, 9];
const PARAMETERS: [&str; 6] = ["BETA0", "BETA1", "BETA2", "BETA3", "TAU1", "TAU2"];
/// GSW par yield against Treasury par yield: the largest difference in the
/// last year was 0.18 pp (1Y). The curve is a fit, so the two series differ a little.
const PAR_TOLERANCE: f64 = 0.0025;
const PAR_CHECK: [(u32, &str); 8] =
    [(1, "1 Yr"), (2, "2 Yr"), (3, "3 Yr"), (5, "5 Yr"), (7, "7 Yr"), (10, "10 Yr"), (20, "20 Yr"), (30, "30 Yr")];

pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Point {
    pub years: u32,
    /// Decimal: 0.0507 == 5.07%.
    pub rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Forwards {
    /// The date of the row that the program used.
    pub date: String,
    pub requested_date: String,
    pub source: String,
    pub url: String,
    /// The caveats that the caller must see.
    pub note: String,
    /// SVENY: zero-coupon yield for each maturity, continuously compounded.
    pub zero_yields: Vec<Point>,
    /// SVENF: instantaneous forward rate at each horizon, continuously compounded.
    pub instantaneous_forwards: Vec<Point>,
    /// SVEN1F: one-year forward rate, coupon-equivalent. `years` is the start of the year.
    pub one_year_forwards: Vec<Point>,
    /// SVENPY: par yield of the fitted curve, coupon-equivalent.
    pub par_yields: Vec<Point>,
    /// Curve parameters, as published (BETA in percent, TAU in years).
    pub parameters: BTreeMap<String, f64>,
}

/// One data row: the day number and a value for each header column (None = "NA").
#[derive(Debug)]
struct Row {
    day: i64,
    values: Vec<Option<f64>>,
}

#[derive(Debug)]
pub struct Dataset {
    header: Vec<String>,
    rows: Vec<Row>,
}

impl Dataset {
    fn column(&self, name: &str) -> Result<usize> {
        self.header.iter().position(|h| h == name).ok_or_else(|| format!("GSW file has no column {name}"))
    }

    fn value(&self, row: &Row, name: &str) -> Result<Option<f64>> {
        Ok(row.values[self.column(name)?])
    }

    pub fn first_date(&self) -> String {
        dates::format(self.rows[0].day)
    }

    pub fn last_date(&self) -> String {
        dates::format(self.rows[self.rows.len() - 1].day)
    }
}

fn required_columns() -> Vec<String> {
    let mut names: Vec<String> = PARAMETERS.iter().map(|s| s.to_string()).collect();
    for n in 1..=MAX_YEARS {
        for prefix in ["SVENY", "SVENF", "SVENPY"] {
            names.push(format!("{prefix}{n:02}"));
        }
    }
    names.extend(ONE_YEAR_STARTS.iter().map(|n| format!("SVEN1F{n:02}")));
    names
}

/// Parse the GSW file. The file starts with note lines. The data table starts at the
/// line that begins with "Date,". "NA" is a missing value.
pub fn parse_gsw(text: &str) -> Result<Dataset> {
    let text = text.trim_start_matches('\u{feff}');
    let mut lines = text.lines().map(str::trim_end).skip_while(|l| !l.starts_with("Date,"));
    let header_line = lines.next().ok_or("GSW file has no line that starts with 'Date,'")?;
    let header: Vec<String> = header_line.split(',').map(|s| s.trim().to_string()).collect();
    for name in required_columns() {
        if !header.contains(&name) {
            return Err(format!("GSW file has no column {name}"));
        }
    }
    let mut rows: Vec<Row> = Vec::new();
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != header.len() {
            return Err(format!("GSW data line {} has {} fields; the header has {}", i + 1, fields.len(), header.len()));
        }
        let day = dates::parse(fields[0].trim()).ok_or_else(|| format!("GSW data line {}: bad date '{}'", i + 1, fields[0]))?;
        if let Some(previous) = rows.last() {
            if day <= previous.day {
                return Err(format!("GSW data line {}: date {} is not after the date before it", i + 1, fields[0]));
            }
        }
        let mut values = Vec::with_capacity(fields.len());
        for (name, field) in header.iter().zip(&fields).skip(1) {
            let field = field.trim();
            if field == "NA" {
                values.push(None);
            } else {
                let v: f64 = field.parse().map_err(|_| format!("GSW data line {}: bad number '{field}' in {name}", i + 1))?;
                if !v.is_finite() {
                    return Err(format!("GSW data line {}: {name} is not finite", i + 1));
                }
                values.push(Some(v));
            }
        }
        values.insert(0, None);
        rows.push(Row { day, values });
    }
    if rows.is_empty() {
        return Err("GSW file has no data rows".into());
    }
    Ok(Dataset { header, rows })
}

fn decimal(percent: f64) -> f64 {
    (percent / 100.0 * 1e8).round() / 1e8
}

fn points(ds: &Dataset, row: &Row, prefix: &str, years: impl Iterator<Item = u32>) -> Result<Vec<Point>> {
    let mut out = Vec::new();
    for n in years {
        if let Some(v) = ds.value(row, &format!("{prefix}{n:02}"))? {
            out.push(Point { years: n, rate: decimal(v) });
        }
    }
    Ok(out)
}

/// The rates of the last row on or before `day`. A row with no zero-coupon yield is no row.
pub fn forwards_on(ds: &Dataset, requested: &str, day: i64) -> Result<Forwards> {
    let row = ds
        .rows
        .iter()
        .rev()
        .find(|r| r.day <= day && ds.value(r, "SVENY01").ok().flatten().is_some())
        .ok_or_else(|| format!("no GSW data on or before {requested}; the first date is {}", ds.first_date()))?;
    let date = dates::format(row.day);
    let mut note = String::from(
        "The GSW curve is a fitted model, not a market quote. The Board calls the file a staff research \
         product that can change without notice. A value can be revised.",
    );
    if date != requested {
        note.push_str(&format!(" No row for {requested}; the row of {date} is the last one before it."));
    }
    let mut parameters = BTreeMap::new();
    for name in PARAMETERS {
        if let Some(v) = ds.value(row, name)? {
            parameters.insert(name.to_string(), v);
        }
    }
    Ok(Forwards {
        date,
        requested_date: requested.to_string(),
        source: SOURCE.into(),
        url: GSW_URL.into(),
        note,
        zero_yields: points(ds, row, "SVENY", 1..=MAX_YEARS)?,
        instantaneous_forwards: points(ds, row, "SVENF", 1..=MAX_YEARS)?,
        one_year_forwards: points(ds, row, "SVEN1F", ONE_YEAR_STARTS.iter().copied())?,
        par_yields: points(ds, row, "SVENPY", 1..=MAX_YEARS)?,
        parameters,
    })
}

// ---------------------------------------------------------------------------
// Cache and download
// ---------------------------------------------------------------------------

pub fn cache_path(repo: &Path) -> PathBuf {
    repo.join("data").join("raw").join("rates").join("gsw_feds200628.csv")
}

fn is_fresh(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < MAX_AGE)
}

fn client() -> Client {
    Client::new("FinancialAnalysis fa (personal research)", 0.2)
}

/// The dataset from the cache, or from the Board when the cache is old. A new file goes
/// to the cache only after it parses. If the download fails and a cache exists, the old
/// file is used and a warning goes to stderr.
pub fn load(repo: &Path) -> Result<Dataset> {
    let path = cache_path(repo);
    if is_fresh(&path) {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(ds) = parse_gsw(&text) {
                return Ok(ds);
            }
        }
    }
    let fetched = client()
        .get(GSW_URL)
        .map_err(|e| e.to_string())
        .and_then(|b| String::from_utf8(b).map_err(|_| "GSW file is not UTF-8 text".to_string()))
        .and_then(|text| parse_gsw(&text).map(|ds| (text, ds)));
    match fetched {
        Ok((text, ds)) => {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            let tmp = path.with_extension("csv.tmp");
            std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
            std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(ds)
        }
        Err(e) => {
            let old = std::fs::read_to_string(&path).map_err(|_| format!("GSW download failed and no cache exists: {e}"))?;
            eprintln!("fa: GSW download failed ({e}); using the old file {}", path.display());
            parse_gsw(&old)
        }
    }
}

/// D is YYYY-MM-DD or 'today'. Gives the text and the day number.
pub fn parse_date_arg(text: &str) -> Result<(String, i64)> {
    if text == "today" {
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
        let day = (secs / 86_400) as i64;
        return Ok((dates::format(day), day));
    }
    let day = dates::parse(text).ok_or_else(|| format!("bad date '{text}': use YYYY-MM-DD or today"))?;
    Ok((text.to_string(), day))
}

pub fn forwards(repo: &Path, date: Option<&str>) -> Result<Forwards> {
    let (text, day) = parse_date_arg(date.unwrap_or("today"))?;
    forwards_on(&load(repo)?, &text, day)
}

// ---------------------------------------------------------------------------
// Cross-check
// ---------------------------------------------------------------------------

/// Treasury daily par yield curve CSV: a header ("Date","1 Mo",...), then one line for each
/// date (MM/DD/YYYY). Gives {column: {day: decimal}}. An empty value or "N/A" is missing.
pub fn parse_treasury_csv(text: &str) -> Result<BTreeMap<String, BTreeMap<i64, f64>>> {
    let mut lines = text.trim_start_matches('\u{feff}').lines().filter(|l| !l.trim().is_empty());
    let header: Vec<String> = lines
        .next()
        .ok_or("Treasury file is empty")?
        .split(',')
        .map(|h| h.trim().trim_matches('"').to_string())
        .collect();
    if header.first().map(String::as_str) != Some("Date") || header.len() < 2 {
        return Err(format!("Treasury file has an unexpected header: {}", header.join(",")));
    }
    let mut out: BTreeMap<String, BTreeMap<i64, f64>> = header[1..].iter().map(|h| (h.clone(), BTreeMap::new())).collect();
    for line in lines {
        let fields: Vec<&str> = line.split(',').map(|f| f.trim().trim_matches('"')).collect();
        if fields.len() != header.len() {
            return Err(format!("Treasury line has {} fields; the header has {}: {line}", fields.len(), header.len()));
        }
        let parts: Vec<&str> = fields[0].split('/').collect();
        let day = match parts[..] {
            [m, d, y] => match (m.parse::<i64>(), d.parse::<i64>(), y.parse::<i64>()) {
                (Ok(m), Ok(d), Ok(y)) if (1..=12).contains(&m) && (1..=31).contains(&d) => dates::days_from_civil(y, m, d),
                _ => return Err(format!("Treasury line has a bad date: {line}")),
            },
            _ => return Err(format!("Treasury line has a bad date: {line}")),
        };
        for (name, value) in header[1..].iter().zip(&fields[1..]) {
            if value.is_empty() || *value == "N/A" {
                continue;
            }
            let pct: f64 = value.parse().map_err(|_| format!("Treasury line has a bad number '{value}': {line}"))?;
            out.get_mut(name).expect("column").insert(day, decimal(pct));
        }
    }
    Ok(out)
}

/// GSW par yield against a Treasury par yield, for each date from `from` on.
pub fn compare_par(ds: &Dataset, years: u32, column: &str, theirs: &BTreeMap<i64, f64>, from: i64) -> Result<CheckResult> {
    let name = format!("SVENPY{years:02}");
    let mut compared = 0;
    let mut mismatches = Vec::new();
    for row in ds.rows.iter().filter(|r| r.day >= from) {
        if let (Some(ours), Some(theirs)) = (ds.value(row, &name)?, theirs.get(&row.day)) {
            compared += 1;
            let ours = decimal(ours);
            if (ours - theirs).abs() > PAR_TOLERANCE {
                mismatches.push((dates::format(row.day), ours, *theirs));
            }
        }
    }
    Ok(CheckResult {
        check: format!("GSW par yield {years}Y vs Treasury par yield (tolerance {} pp)", PAR_TOLERANCE * 100.0),
        fred_series: format!("Treasury par yield curve, column '{column}'"),
        compared,
        mismatches,
    })
}

/// The cross-check for the last `days` calendar days of the file.
pub fn check(repo: &Path, days: u32) -> Result<Vec<CheckResult>> {
    let ds = load(repo)?;
    let last = ds.rows[ds.rows.len() - 1].day;
    let from = last - days as i64;
    let mut http = client();
    let mut table: BTreeMap<String, BTreeMap<i64, f64>> = BTreeMap::new();
    let year_of = |day: i64| dates::format(day)[..4].to_string();
    for year in [year_of(from), year_of(last)].iter().collect::<std::collections::BTreeSet<_>>() {
        let bytes = http.get(&TREASURY_CSV.replace("{year}", year)).map_err(|e| e.to_string())?;
        for (column, values) in parse_treasury_csv(&String::from_utf8_lossy(&bytes))? {
            table.entry(column).or_default().extend(values);
        }
    }
    let mut results = Vec::new();
    for (years, column) in PAR_CHECK {
        let theirs = table.get(column).ok_or_else(|| format!("Treasury file has no column '{column}'"))?;
        results.push(compare_par(&ds, years, column, theirs, from)?);
    }
    if let Some(empty) = results.iter().find(|r| r.compared == 0) {
        return Err(format!("check '{}' compared no dates", empty.check));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small file in the format of the Board file. Only the columns that matter have
    /// values; the others are NA. 30 tenors for SVENY, SVENF, SVENPY.
    fn sample(rows: &[(&str, &str)]) -> String {
        let mut header = vec!["Date".to_string()];
        header.extend(PARAMETERS.iter().map(|s| s.to_string()));
        for p in ["SVEN1F"] {
            for n in ONE_YEAR_STARTS {
                header.push(format!("{p}{n:02}"));
            }
        }
        for p in ["SVENF", "SVENPY", "SVENY"] {
            for n in 1..=MAX_YEARS {
                header.push(format!("{p}{n:02}"));
            }
        }
        let mut text = String::from("\"Note: This is not an official Federal Reserve statistical release.\"\n\nSeries,Compounding Convention,Mnemonic(s)\nZero-coupon yield,Continuously Compounded,SVENYXX\n\n");
        text.push_str(&header.join(","));
        text.push('\n');
        for (date, kind) in rows {
            let mut f = vec![date.to_string()];
            for h in &header[1..] {
                let v = match (*kind, h.as_str()) {
                    ("na", _) => "NA".to_string(),
                    (_, "SVENY01") => "4.6294".into(),
                    (_, "SVENY02") => "4.829".into(),
                    (_, "SVENPY02") => "4.8".into(),
                    (_, "SVENF01") => "4.9".into(),
                    // exp(2*0.04829 - 0.046294) - 1 = 0.05157
                    (_, "SVEN1F01") => "5.157".into(),
                    (_, "BETA0") | (_, "TAU1") => "1.5".into(),
                    _ => "NA".to_string(),
                };
                f.push(v);
            }
            text.push_str(&f.join(","));
            text.push('\n');
        }
        text
    }

    #[test]
    fn parses_and_converts_to_decimals() {
        let ds = parse_gsw(&sample(&[("2026-11-12", "ok"), ("2026-11-13", "ok")])).unwrap();
        assert_eq!((ds.first_date().as_str(), ds.last_date().as_str()), ("2026-11-12", "2026-11-13"));
        let f = forwards_on(&ds, "2026-11-13", dates::parse("2026-11-13").unwrap()).unwrap();
        assert_eq!(f.date, "2026-11-13");
        assert_eq!(f.zero_yields, vec![Point { years: 1, rate: 0.046294 }, Point { years: 2, rate: 0.04829 }]);
        assert_eq!(f.instantaneous_forwards, vec![Point { years: 1, rate: 0.049 }]);
        assert_eq!(f.one_year_forwards, vec![Point { years: 1, rate: 0.05157 }]);
        assert_eq!(f.parameters["BETA0"], 1.5);
        assert!(f.note.contains("fitted model, not a market quote"));
        assert!(!f.note.contains("No row for"));
    }

    #[test]
    fn weekend_uses_last_row_and_says_so() {
        let ds = parse_gsw(&sample(&[("2026-11-12", "ok"), ("2026-11-13", "ok")])).unwrap();
        let f = forwards_on(&ds, "2026-11-15", dates::parse("2026-11-15").unwrap()).unwrap();
        assert_eq!((f.date.as_str(), f.requested_date.as_str()), ("2026-11-13", "2026-11-15"));
        assert!(f.note.contains("No row for 2026-11-15"));
    }

    #[test]
    fn skips_a_row_with_no_yields() {
        let ds = parse_gsw(&sample(&[("2026-11-12", "ok"), ("2026-11-13", "na")])).unwrap();
        let f = forwards_on(&ds, "2026-11-13", dates::parse("2026-11-13").unwrap()).unwrap();
        assert_eq!(f.date, "2026-11-12");
    }

    #[test]
    fn date_before_the_first_row_is_an_error() {
        let ds = parse_gsw(&sample(&[("2026-11-12", "ok")])).unwrap();
        let e = forwards_on(&ds, "2020-01-01", dates::parse("2020-01-01").unwrap()).unwrap_err();
        assert!(e.contains("first date is 2026-11-12"), "{e}");
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_gsw("").unwrap_err().contains("Date,"));
        assert!(parse_gsw("<html>Access denied</html>").unwrap_err().contains("Date,"));
        assert!(parse_gsw("Date,BETA0\n2026-11-12,1\n").unwrap_err().contains("no column"));
        let good = sample(&[("2026-11-12", "ok")]);
        assert!(parse_gsw(&good.replace("2026-11-12", "12/11/2026")).unwrap_err().contains("bad date"));
        assert!(parse_gsw(&good.replace("4.6294", "abc")).unwrap_err().contains("bad number"));
        assert!(parse_gsw(&good.replace("4.6294", "NaN")).is_err());
        assert!(parse_gsw(&good.replace(",4.829,", ",")).unwrap_err().contains("fields"));
        let header_only = good.lines().take_while(|l| !l.starts_with("2026")).collect::<Vec<_>>().join("\n");
        assert!(parse_gsw(&header_only).unwrap_err().contains("no data rows"));
        let unordered = sample(&[("2026-11-13", "ok"), ("2026-11-12", "ok")]);
        assert!(parse_gsw(&unordered).unwrap_err().contains("not after"));
    }

    #[test]
    fn date_argument() {
        assert_eq!(parse_date_arg("2026-11-13").unwrap().0, "2026-11-13");
        assert!(parse_date_arg("today").is_ok());
        assert!(parse_date_arg("13-11-2026").is_err());
    }

    #[test]
    fn treasury_csv_parses_and_rejects() {
        let text = "Date,\"1 Mo\",\"2 Yr\"\n11/13/2026,4.10,4.80\n11/12/2026,4.09,N/A\n";
        let m = parse_treasury_csv(text).unwrap();
        assert_eq!(m["2 Yr"].len(), 1);
        assert_eq!(m["2 Yr"][&dates::parse("2026-11-13").unwrap()], 0.048);
        assert_eq!(m["1 Mo"].len(), 2);
        assert!(parse_treasury_csv("<html>").is_err());
        assert!(parse_treasury_csv("Date,\"2 Yr\"\n2026-11-13,4.8\n").unwrap_err().contains("bad date"));
        assert!(parse_treasury_csv("Date,\"2 Yr\"\n11/13/2026,x\n").unwrap_err().contains("bad number"));
        assert!(parse_treasury_csv("Date,\"2 Yr\"\n11/13/2026,4.8,4.9\n").unwrap_err().contains("fields"));
    }

    #[test]
    fn par_check_flags_a_big_difference() {
        let ds = parse_gsw(&sample(&[("2026-11-12", "ok"), ("2026-11-13", "ok")])).unwrap();
        let d12 = dates::parse("2026-11-12").unwrap();
        let d13 = dates::parse("2026-11-13").unwrap();
        let theirs: BTreeMap<i64, f64> = [(d12, 0.0481), (d13, 0.0600)].into();
        let r = compare_par(&ds, 2, "2 Yr", &theirs, d12).unwrap();
        assert_eq!(r.compared, 2);
        assert_eq!(r.mismatches, vec![("2026-11-13".to_string(), 0.048, 0.06)]);
    }
}
