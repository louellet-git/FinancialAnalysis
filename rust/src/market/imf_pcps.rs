//! IMF Primary Commodity Prices (PCPS): the uranium series, monthly, from the official IMF data API (SDMX 2.1).
//!
//! `fa commodity show uranium` makes ONE request for the one series `G001.PURAN.USD.M` (world, uranium, US dollars, monthly),
//! keeps the answer in memory only, prints the months, and adds one line to `reads.log` (date, address, bytes, SHA-256,
//! months, first and last month). No copy of the series is stored (owner decision, 2026-11-26). `get uranium` only says this.
//! Terms (copyright page of the IMF, read by the Head of Staff on 2026-11-26, section "Special Terms for Statistical
//! Data"): "Primary Commodity Prices" is Data; users may download, copy and use it; the data must appear accurately
//! with attribution to the IMF. The IMF prohibits bulk download by automated technology: this loader asks for one
//! series only, never for the whole dataflow.
//! Definition (structure of the dataflow, read 2026-11-26): "Uranium, NUEXCO, Restricted Price, Nuexco exchange spot,
//! US$ per pound"; monthly averages converted from weekly or daily values.

use super::today_iso;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The one address that the program asks. The host is fixed in the code, not read from an answer.
pub const URL: &str = "https://api.imf.org/external/sdmx/2.1/data/PCPS/G001.PURAN.USD.M";
pub const PAUSE: f64 = 3.0;
pub const SOURCE: &str = "International Monetary Fund, Primary Commodity Price System (PCPS), series G001.PURAN.USD.M";
pub const ATTRIBUTION: &str = "Source: International Monetary Fund, Primary Commodity Prices, https://data.imf.org/en/datasets//IMF.RES:PCPS";
pub const DEFINITION: &str = "Uranium, NUEXCO, Restricted Price, Nuexco exchange spot, US$ per pound (monthly average of weekly or daily values; the IMF names NUEXCO as the price source)";
pub const TERMS: &str = "IMF copyright page, section Special Terms for Statistical Data: download, copy and use permitted with attribution to the IMF; no bulk download by automated technology (https://www.imf.org/en/About/copyright-and-terms)";

pub fn dir(cache: &Path) -> PathBuf {
    cache.join("commodity").join("imf")
}

#[derive(Debug, PartialEq)]
pub struct Parsed {
    /// "YYYY-MM" to US dollars per pound.
    pub months: BTreeMap<String, f64>,
    /// The `prepared` time of the answer.
    pub updated: String,
}

/// "2026-M08" to "2026-08".
pub fn month_of(p: &str) -> Option<String> {
    let (y, m) = p.split_once("-M")?;
    let (yn, mn) = (y.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (y.len() == 4 && m.len() == 2 && (1..=12).contains(&mn)).then(|| format!("{yn:04}-{mn:02}"))
}

/// Read the answer (SDMX-JSON). The function checks the form: one series with the four expected dimension values.
pub fn parse(text: &str) -> Result<Parsed, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("the answer is not JSON: {e}"))?;
    let updated = v["header"]["prepared"].as_str().unwrap_or("").to_string();
    let series = v["dataSets"][0]["series"].as_object().ok_or("the answer has no series: the key did not match (an empty data set)")?;
    if series.len() != 1 {
        return Err(format!("the answer has {} series, not one: the request key is wrong", series.len()));
    }
    let dims = v["structure"]["dimensions"]["series"].as_array().ok_or("the answer has no series dimensions")?;
    for (k, want) in [("COUNTRY", "G001"), ("INDICATOR", "PURAN"), ("DATA_TRANSFORMATION", "USD"), ("FREQUENCY", "M")] {
        let got = dims.iter().find(|d| d["id"] == k).and_then(|d| d["values"][0]["id"].as_str());
        if got != Some(want) || dims.iter().find(|d| d["id"] == k).and_then(|d| d["values"].as_array()).map(Vec::len) != Some(1) {
            return Err(format!("the series has {k}={got:?}, not {want}: the form of the answer changed"));
        }
    }
    let times = v["structure"]["dimensions"]["observation"][0]["values"].as_array().ok_or("the answer has no time values")?;
    let obs = series.values().next().and_then(|s| s["observations"].as_object()).ok_or("the series has no observations")?;
    let mut months = BTreeMap::new();
    for (i, o) in obs {
        let idx: usize = i.parse().map_err(|_| format!("observation index {i}: not a number"))?;
        let p = times.get(idx).and_then(|t| t["id"].as_str()).ok_or_else(|| format!("observation {i}: no time value"))?;
        let m = month_of(p).ok_or_else(|| format!("TIME_PERIOD {p}: not a month"))?;
        let x = &o[0];
        let value = x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse::<f64>().ok()));
        match value {
            Some(val) => {
                months.insert(m, val);
            }
            None if x.is_null() => {}
            None => return Err(format!("observation {p}: the value {x} is not a number")),
        }
    }
    if months.is_empty() {
        return Err("the series has no observations".into());
    }
    Ok(Parsed { months, updated })
}

pub fn format_rows(p: &Parsed, from: Option<&str>) -> String {
    let mut s = format!("{:<8} {:>10}\n", "month", "USD/lb");
    for (m, v) in p.months.iter().filter(|(m, _)| from.is_none_or(|f| m.as_str() >= f)) {
        s += &format!("{m:<8} {v:>10.2}\n");
    }
    s
}

/// One request for the one series. The answer stays in memory: no file of the series is written.
/// Returns the answer bytes and the parsed series. An answer that does not parse gives its first 200 characters in the error.
pub fn read() -> Result<(Vec<u8>, Parsed), String> {
    let body = crate::hostnet::get(URL, &[("Accept", "application/json")], PAUSE)?;
    let head = |b: &[u8]| String::from_utf8_lossy(&b[..b.len().min(200)]).into_owned();
    let text = std::str::from_utf8(&body).map_err(|_| format!("the IMF answer is not UTF-8 ({} bytes, starts with {:02x?})", body.len(), &body[..body.len().min(8)]))?;
    let parsed = parse(text).map_err(|e| format!("{e} (answer: {} bytes, starts with {:?})", body.len(), head(&body)))?;
    Ok((body, parsed))
}

/// The line of the read log: date, address, bytes, SHA-256, months, first and last month, IMF update time (tab separated).
pub fn log_line(day: &str, bytes: &[u8], p: &Parsed) -> String {
    let (first, last) = (p.months.keys().next().map_or("", String::as_str), p.months.keys().next_back().map_or("", String::as_str));
    format!("{day}\t{URL}\t{}\t{}\t{}\t{first}\t{last}\t{}\n", bytes.len(), crate::portfolio::harness::sha256_hex(bytes), p.months.len(), p.updated)
}

/// Add one line to `<cache>/commodity/imf/reads.log` (the only file of this source; it holds no series value).
pub fn append_log(cache: &Path, line: &str) -> Result<(), String> {
    use std::io::Write;
    let d = dir(cache);
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    let f = d.join("reads.log");
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&f).map_err(|e| format!("{}: {e}", f.display()))?;
    file.write_all(line.as_bytes()).map_err(|e| format!("{}: {e}", f.display()))
}

/// `fa commodity get uranium`: the program keeps no copy; it says so and makes no request.
pub fn get() -> Result<i32, String> {
    println!("uranium is read from the IMF API at each run (fa commodity show uranium); no copy is stored. {ATTRIBUTION}");
    Ok(0)
}

/// One run-time read for a caller (for example a model): the series, the read date and the SHA-256 of the answer.
/// It makes the one request, writes the log line, and keeps no copy of the series.
pub struct RunRead {
    pub parsed: Parsed,
    pub read_date: String,
    pub sha256: String,
}

/// The run-time read with its log line: use this from other code, not `read()` alone, so each read is logged.
pub fn read_logged(cache: &Path) -> Result<RunRead, String> {
    let (body, parsed) = read()?;
    let read_date = today_iso();
    append_log(cache, &log_line(&read_date, &body, &parsed))?;
    Ok(RunRead { sha256: crate::portfolio::harness::sha256_hex(&body), parsed, read_date })
}

pub fn show(cache: &Path, from: Option<&str>) -> Result<i32, String> {
    let RunRead { parsed, .. } = read_logged(cache)?;
    eprintln!("fa: IMF PCPS uranium, read now: {} months, IMF update {}; logged in {}", parsed.months.len(), parsed.updated, dir(cache).join("reads.log").display());
    eprintln!("fa: definition: {DEFINITION}");
    eprintln!("fa: {ATTRIBUTION}");
    print!("{}", format_rows(&parsed, from));
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The form of the IMF API answer for `G001.PURAN.USD.M` (SDMX-JSON, seen 2026-11-26), cut to 3 months. The numbers
    /// are INVENTED (11.125, 22.25, 33.5): the repo keeps no real value of the series. A value is a string, as in the answer.
    const ANSWER: &str = r#"{"header":{"id":"DS1","prepared":"2026-11-26T01:26:56Z","test":false,"sender":{"id":"IMF","name":"unknown"}},"dataSets":[{"action":"Replace","attributes":[0],"series":{"0:0:0:0":{"attributes":[0,null],"observations":{"0":["11.125",null,0,0,null,0,null],"1":["22.25",null,0,0,null,0,null],"2":["33.5",null,0,0,null,0,null]}}}}],"structure":{"name":"x","dimensions":{"dataset":[],"series":[{"id":"COUNTRY","keyPosition":0,"values":[{"id":"G001","name":"World"}]},{"id":"INDICATOR","keyPosition":1,"values":[{"id":"PURAN","name":"Uranium, US dollars per pound, Unit prices"}]},{"id":"DATA_TRANSFORMATION","keyPosition":2,"values":[{"id":"USD","name":"US dollars"}]},{"id":"FREQUENCY","keyPosition":3,"values":[{"id":"M","name":"Monthly"}]}],"observation":[{"id":"TIME_PERIOD","keyPosition":4,"role":"time","values":[{"start":"2026-08-01T00:00:00Z","end":"2026-08-31T23:59:59Z","id":"2026-M08","name":"2026-M08"},{"start":"2026-09-01T00:00:00Z","end":"2026-09-30T23:59:59Z","id":"2026-M09","name":"2026-M09"},{"start":"2026-10-01T00:00:00Z","end":"2026-10-31T23:59:59Z","id":"2026-M10","name":"2026-M10"}]}]}}}"#;

    /// A key that matches no series: a data set with no series (the form of the empty answer, invented text).
    const EMPTY: &str = r#"{"header":{"prepared":"2026-11-26T01:26:56Z"},"dataSets":[{"action":"Replace"}],"structure":{"dimensions":{"series":[],"observation":[]}}}"#;

    #[test]
    fn the_answer_form_gives_three_months() {
        let p = parse(ANSWER).unwrap();
        assert_eq!(p.months.len(), 3);
        assert_eq!(p.months["2026-08"], 11.125);
        assert_eq!(p.months["2026-09"], 22.25);
        assert_eq!(p.months["2026-10"], 33.5);
        assert_eq!(p.updated, "2026-11-26T01:26:56Z");
        assert_eq!(format_rows(&p, Some("2026-09")), "month        USD/lb\n2026-09       22.25\n2026-10       33.50\n");
    }

    #[test]
    fn an_empty_or_changed_answer_is_an_error() {
        assert!(parse(EMPTY).unwrap_err().contains("no series"));
        assert!(parse("<html>Access Denied</html>").unwrap_err().contains("not JSON"));
        assert!(parse(&ANSWER.replace("\"PURAN\"", "\"PGOLD\"")).unwrap_err().contains("INDICATOR"));
        assert!(parse(&ANSWER.replace("\"USD\"", "\"INDEX\"")).unwrap_err().contains("DATA_TRANSFORMATION"));
        assert!(parse(&ANSWER.replace("\"2026-M08\"", "\"2026-Q3\"")).unwrap_err().contains("not a month"));
    }

    #[test]
    fn the_address_is_the_imf_api_host_with_one_series_key() {
        assert_eq!(crate::hostnet::group_of_url(URL), "imf");
        assert!(URL.starts_with("https://api.imf.org/external/sdmx/2.1/data/PCPS/G001.PURAN."));
        assert_eq!(month_of("1990-M01").as_deref(), Some("1990-01"));
        assert_eq!(month_of("1990-M13"), None);
        assert!(ATTRIBUTION.starts_with("Source: International Monetary Fund, Primary Commodity Prices, https://"));
    }

    #[test]
    fn the_log_line_has_date_hash_and_months_and_get_makes_no_request() {
        let p = parse(ANSWER).unwrap();
        let line = log_line("2026-11-26", ANSWER.as_bytes(), &p);
        let f: Vec<&str> = line.trim_end().split('\t').collect();
        assert_eq!(f.len(), 8);
        assert_eq!((f[0], f[1]), ("2026-11-26", URL));
        assert_eq!(f[2], ANSWER.len().to_string());
        assert_eq!(f[3].len(), 64);
        // The log line has no price value.
        assert!(!line.contains("11.125") && !line.contains("22.25") && !line.contains("33.5"));
        assert_eq!((f[4], f[5], f[6]), ("3", "2026-08", "2026-10"));
        assert!(line.ends_with('\n'));
        // `get` only prints a sentence (no network, no file).
        assert_eq!(get().unwrap(), 0);
    }

    /// Check of the reader on a real answer that a person saved outside the repo: `FA_IMF_ANSWER_FILE=F cargo test -- --ignored imf`.
    /// It prints the count, the first and last month and the last three values. It is ignored in a normal run (no file, no network).
    #[test]
    #[ignore]
    fn imf_real_answer_file() {
        let f = std::env::var("FA_IMF_ANSWER_FILE").expect("set FA_IMF_ANSWER_FILE");
        let p = parse(&std::fs::read_to_string(f).unwrap()).unwrap();
        let last: Vec<_> = p.months.iter().rev().take(3).collect();
        println!("months {} first {:?} last {:?} last3 {:?}", p.months.len(), p.months.keys().next(), p.months.keys().next_back(), last);
    }
}
