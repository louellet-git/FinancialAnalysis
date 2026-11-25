//! EDINET (Financial Services Agency of Japan): the annual securities report (有価証券報告書, document type 120)
//! of a Japanese filer, read from the CSV form (API v2, `type=5`) into company facts.
//!
//! Source: EDINET API v2, https://api.edinet-fsa.go.jp/api/v2 . Reuse is under the Public Data License 1.0
//! (name the source: EDINET, Financial Services Agency). The API needs a free key. The key is a query parameter
//! (`Subscription-Key`) and not a header, so each request URL has the key. Rules of this module:
//! - the key comes from the environment variable `EDINET_API_KEY` (or `.env`); it is never printed and never
//!   written to a file; each error text is scrubbed (the key value and the parameter value);
//! - at most one request each `DEFAULT_DELAY_SECONDS`; an answer with HTTP 429 or 403 (or the same code in the
//!   JSON body) stops the run; two connection failures in a row stop it;
//! - the CSV form: ZIP, one or more files `XBRL_TO_CSV/*.csv`, UTF-16LE, tab separated, each value in
//!   double quotes, 9 columns (element ID, item name, context ID, relative year, consolidated or not,
//!   period or instant, unit ID, unit, value). Source: 書類閲覧操作ガイド (ESE140133.pdf), chapter 1, 3-2-1.
//!
//! The amounts stay in JPY. No conversion. `facts_from_rows` makes the company-facts form that the mapper reads
//! (namespaces `jpigp_cor`, `jpcrp_cor`, `ifrs-full`; unit `JPY`; form `EDINET-120`).

use crate::dates;
use crate::http::{contact_search_dirs, Client};
use flate2::read::DeflateDecoder;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The form label of the facts of an annual securities report (EDINET document type 120).
pub const FORM_ANNUAL: &str = "EDINET-120";
/// The EDINET document type code of the annual securities report.
pub const DOC_TYPE_ANNUAL: &str = "120";
/// The environment variable of the key.
pub const KEY_ENV: &str = "EDINET_API_KEY";
/// The text for a missing key.
pub const KEY_MISSING: &str = "EDINET_API_KEY not set: this read runs on a laptop with the key";
/// The client name (the same as the other clients of the repo). No contact text, no browser name.
pub const USER_AGENT: &str = "FinancialAnalysis fa (personal research)";
/// Seconds between two requests. The EDINET FAQ asks for moderate use (about one request a minute to poll
/// the same list); a scan of a few weeks needs one request for each day.
pub const DEFAULT_DELAY_SECONDS: f64 = 3.0;
/// The floor of `--delay`: the same as the default, as the row in docs/DATA-SOURCES.md says.
pub const MIN_DELAY_SECONDS: f64 = 3.0;
const API_BASE: &str = "https://api.edinet-fsa.go.jp/api/v2";
/// The namespaces that the facts keep. The registry reads these.
const KEEP: [&str; 3] = ["jpigp_cor", "jpcrp_cor", "ifrs-full"];

// ---------------------------------------------------------------------------------------------------------------
// The key and the secret-free text
// ---------------------------------------------------------------------------------------------------------------

/// The key from the environment, else from a `.env` file of the repo (the folders of `contact_search_dirs`).
pub fn api_key() -> Result<String, String> {
    key_from(std::env::var(KEY_ENV).ok().as_deref(), &contact_search_dirs())
}

/// The pure part of `api_key`.
pub fn key_from(variable: Option<&str>, dirs: &[PathBuf]) -> Result<String, String> {
    if let Some(v) = variable.map(str::trim).filter(|v| !v.is_empty()) {
        return Ok(v.to_string());
    }
    for dir in dirs {
        if let Ok(text) = std::fs::read_to_string(dir.join(".env")) {
            for line in text.lines() {
                let line = line.trim();
                let line = line.strip_prefix("export ").unwrap_or(line).trim();
                if let Some(value) = line.strip_prefix(KEY_ENV).and_then(|r| r.trim_start().strip_prefix('=')) {
                    let value = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
                    if !value.is_empty() {
                        return Ok(value.to_string());
                    }
                }
            }
        }
    }
    Err(KEY_MISSING.to_string())
}

/// The text with the key value and each `Subscription-Key=` parameter value taken out.
pub fn scrub(text: &str, key: &str) -> String {
    // An empty key would match everywhere; any other key is taken out, short or long.
    let replaced = if key.is_empty() { text.to_string() } else { text.replace(key, "***") };
    let mut result = String::with_capacity(replaced.len());
    let mut rest = replaced.as_str();
    const PARAM: &str = "Subscription-Key=";
    while let Some(i) = rest.find(PARAM) {
        let (head, tail) = rest.split_at(i + PARAM.len());
        result.push_str(head);
        let end = tail.find(|c: char| c == '&' || c.is_whitespace() || c == '"' || c == '\'' || c == ')').unwrap_or(tail.len());
        result.push_str("***");
        rest = &tail[end..];
    }
    result.push_str(rest);
    result
}

// ---------------------------------------------------------------------------------------------------------------
// ZIP (stored or deflate; no other dependency)
// ---------------------------------------------------------------------------------------------------------------

fn u16_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as usize)
}

fn u32_at(b: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize)
}

/// The files of a ZIP archive: (name, content). It reads the central directory; methods 0 (stored) and 8 (deflate).
pub fn read_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let bad = || "not a valid ZIP file".to_string();
    let eocd = (0..bytes.len().saturating_sub(21)).rev().find(|&i| bytes[i..].starts_with(&[0x50, 0x4b, 0x05, 0x06])).ok_or_else(bad)?;
    let count = u16_at(bytes, eocd + 10).ok_or_else(bad)?;
    let mut pos = u32_at(bytes, eocd + 16).ok_or_else(bad)?;
    let mut out = Vec::new();
    for _ in 0..count {
        if bytes.get(pos..pos + 4) != Some(&[0x50, 0x4b, 0x01, 0x02]) {
            return Err(bad());
        }
        let method = u16_at(bytes, pos + 10).ok_or_else(bad)?;
        let compressed = u32_at(bytes, pos + 20).ok_or_else(bad)?;
        let name_len = u16_at(bytes, pos + 28).ok_or_else(bad)?;
        let extra_len = u16_at(bytes, pos + 30).ok_or_else(bad)?;
        let comment_len = u16_at(bytes, pos + 32).ok_or_else(bad)?;
        let local = u32_at(bytes, pos + 42).ok_or_else(bad)?;
        let name = String::from_utf8_lossy(bytes.get(pos + 46..pos + 46 + name_len).ok_or_else(bad)?).to_string();
        pos += 46 + name_len + extra_len + comment_len;
        if bytes.get(local..local + 4) != Some(&[0x50, 0x4b, 0x03, 0x04]) {
            return Err(bad());
        }
        let start = local + 30 + u16_at(bytes, local + 26).ok_or_else(bad)? + u16_at(bytes, local + 28).ok_or_else(bad)?;
        let data = bytes.get(start..start + compressed).ok_or_else(bad)?;
        if name.ends_with('/') {
            continue;
        }
        let content = match method {
            0 => data.to_vec(),
            8 => {
                let mut v = Vec::new();
                DeflateDecoder::new(data).read_to_end(&mut v).map_err(|e| format!("{name}: {e}"))?;
                v
            }
            m => return Err(format!("{name}: ZIP method {m} is not supported")),
        };
        out.push((name, content));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------------------------
// The CSV form
// ---------------------------------------------------------------------------------------------------------------

/// One row of the CSV file (the 9 columns).
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub element: String,
    pub label: String,
    pub context: String,
    pub relative_year: String,
    pub scope: String,
    pub period_kind: String,
    pub unit_id: String,
    pub unit: String,
    pub value: String,
}

/// UTF-16LE text (a byte order mark is accepted; UTF-16BE only with its mark) to a string.
pub fn decode_utf16(bytes: &[u8]) -> Result<String, String> {
    let (big, body) = match bytes {
        [0xFF, 0xFE, rest @ ..] => (false, rest),
        [0xFE, 0xFF, rest @ ..] => (true, rest),
        _ => (false, bytes),
    };
    if body.len() % 2 != 0 {
        return Err("the CSV file is not UTF-16 (odd byte count)".to_string());
    }
    let units = body.chunks_exact(2).map(|c| if big { u16::from_be_bytes([c[0], c[1]]) } else { u16::from_le_bytes([c[0], c[1]]) });
    char::decode_utf16(units).collect::<Result<String, _>>().map_err(|e| format!("the CSV file is not valid UTF-16: {e}"))
}

/// Split tab separated text with double-quoted values (`""` is one quote; a quoted value can have line breaks).
fn split_records(text: &str) -> Vec<Vec<String>> {
    let (mut records, mut record, mut field) = (Vec::new(), Vec::new(), String::new());
    let (mut quoted, mut was_quoted) = (false, false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if field.is_empty() && !was_quoted => {
                quoted = true;
                was_quoted = true;
            }
            '\t' => {
                record.push(std::mem::take(&mut field));
                was_quoted = false;
            }
            '\r' => {}
            '\n' => {
                record.push(std::mem::take(&mut field));
                was_quoted = false;
                if record.iter().any(|f| !f.is_empty()) {
                    records.push(std::mem::take(&mut record));
                } else {
                    record.clear();
                }
            }
            other => field.push(other),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}

/// The rows of the CSV text. The first line is the header (its first cell is `要素ID`). A row with fewer than 9
/// columns is dropped (it is counted in the second value).
pub fn parse_csv(text: &str) -> Result<(Vec<Row>, usize), String> {
    let records = split_records(text.trim_start_matches('\u{feff}'));
    let Some(first) = records.first() else { return Err("the CSV file is empty".to_string()) };
    let has_header = first.first().map(|c| c.trim() == "要素ID").unwrap_or(false);
    if !has_header && !first.first().is_some_and(|c| c.contains(':')) {
        return Err(format!("the CSV file has no header line with 要素ID (first cell: '{}')", first.first().map(String::as_str).unwrap_or("").chars().take(40).collect::<String>()));
    }
    let (mut rows, mut short) = (Vec::new(), 0usize);
    for record in records.iter().skip(usize::from(has_header)) {
        if record.len() < 9 {
            short += 1;
            continue;
        }
        let c = |i: usize| record[i].trim().to_string();
        rows.push(Row {
            element: c(0),
            label: c(1),
            context: c(2),
            relative_year: c(3),
            scope: c(4),
            period_kind: c(5),
            unit_id: c(6),
            unit: c(7),
            value: record[8].trim().to_string(),
        });
    }
    Ok((rows, short))
}

// ---------------------------------------------------------------------------------------------------------------
// From rows to company facts
// ---------------------------------------------------------------------------------------------------------------

/// The cover data (DEI elements) of the report.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dei {
    pub edinet_code: Option<String>,
    pub security_code: Option<String>,
    pub name: Option<String>,
    pub accounting_standard: Option<String>,
    pub fy_start: Option<String>,
    pub fy_end: Option<String>,
    pub previous_start: Option<String>,
    pub previous_end: Option<String>,
    pub filing_date: Option<String>,
}

pub fn read_dei(rows: &[Row]) -> Dei {
    let get = |element: &str| rows.iter().find(|r| r.element == element && !r.value.is_empty() && r.value != "-").map(|r| r.value.clone());
    let date = |element: &str| get(element).filter(|v| v.len() >= 10 && dates::parse(&v[..10]).is_some()).map(|v| v[..10].to_string());
    Dei {
        edinet_code: get("jpdei_cor:EDINETCodeDEI"),
        security_code: get("jpdei_cor:SecurityCodeDEI"),
        name: get("jpdei_cor:FilerNameInEnglishDEI").or_else(|| get("jpdei_cor:FilerNameInJapaneseDEI")),
        accounting_standard: get("jpdei_cor:AccountingStandardsDEI"),
        fy_start: date("jpdei_cor:CurrentFiscalYearStartDateDEI"),
        fy_end: date("jpdei_cor:CurrentFiscalYearEndDateDEI"),
        previous_start: date("jpdei_cor:PreviousFiscalYearStartDateDEI"),
        previous_end: date("jpdei_cor:PreviousFiscalYearEndDateDEI"),
        filing_date: date("jpcrp_cor:FilingDateCoverPage"),
    }
}

/// What the build of the facts did, for the report to the person.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    pub rows: usize,
    pub facts: usize,
    pub other_namespace: usize,
    pub other_context: usize,
    pub not_numeric: usize,
    pub unknown_unit: usize,
}

/// The context ID as (is a duration, years back): `CurrentYearDuration`, `Prior2YearInstant`. Any other ID
/// (a member of a dimension, a quarter, the filing date) gives None.
fn context_kind(context: &str) -> Option<(bool, i64)> {
    let (back, rest) = if let Some(r) = context.strip_prefix("CurrentYear") {
        (0, r)
    } else {
        let r = context.strip_prefix("Prior")?;
        let digits: String = r.chars().take_while(|c| c.is_ascii_digit()).collect();
        let n: i64 = digits.parse().ok()?;
        (n, r[digits.len()..].strip_prefix("Year")?)
    };
    match rest {
        "Duration" => Some((true, back)),
        "Instant" => Some((false, back)),
        _ => None,
    }
}

/// The date `years` years earlier. The last day of a month stays the last day of the month.
fn shift_years(date: &str, years: i64) -> Option<String> {
    dates::parse(date)?;
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().ok()?, date[5..7].parse().ok()?, date[8..10].parse().ok()?);
    let ny = y - years;
    let nd = if d == dates::days_in_month(y, m) { dates::days_in_month(ny, m) } else { d.min(dates::days_in_month(ny, m)) };
    Some(format!("{ny:04}-{m:02}-{nd:02}"))
}

/// The unit name of the facts: the currency (`JPY`), `JPY/shares`, `shares`. None: not a unit that is read.
fn unit_name(unit_id: &str) -> Option<String> {
    match unit_id {
        "JPYPerShares" => Some("JPY/shares".to_string()),
        "shares" | "Shares" => Some("shares".to_string()),
        "pure" => Some("pure".to_string()),
        u if u.len() == 3 && u.chars().all(|c| c.is_ascii_uppercase()) => Some(u.to_string()),
        _ => None,
    }
}

/// The company facts (the form of the SEC company facts, read by `CompanyFacts`) of one report. `filed`: the filing
/// date (YYYY-MM-DD); if None, the cover-page date of the report. Only the consolidated, non-dimension contexts
/// of the current year and the prior years are kept, and only the namespaces that the registry reads.
pub fn facts_from_rows(rows: &[Row], doc_id: &str, filed: Option<&str>) -> Result<(Value, Dei, Stats), String> {
    let dei = read_dei(rows);
    if let Some(standard) = dei.accounting_standard.as_deref() {
        if !standard.eq_ignore_ascii_case("IFRS") {
            return Err(format!("the accounting standard of the report is '{standard}': this reader maps IFRS reports only (ITOCHU is an IFRS filer)"));
        }
    }
    let (Some(fy_start), Some(fy_end)) = (dei.fy_start.clone(), dei.fy_end.clone()) else {
        return Err("the report has no fiscal year dates (jpdei_cor:CurrentFiscalYearStartDateDEI / EndDateDEI)".to_string());
    };
    let filed = filed
        .map(str::to_string)
        .or_else(|| dei.filing_date.clone())
        .ok_or("no filing date: the cover page has none; give the date with --filed YYYY-MM-DD")?;
    let mut stats = Stats { rows: rows.len(), ..Stats::default() };
    let period_of = |duration: bool, back: i64| -> Option<(Option<String>, String)> {
        if back == 1 {
            if let (Some(s), Some(e)) = (&dei.previous_start, &dei.previous_end) {
                return Some((duration.then(|| s.clone()), e.clone()));
            }
        }
        let end = shift_years(&fy_end, back)?;
        let start = if duration { Some(shift_years(&fy_start, back)?) } else { None };
        Some((start, end))
    };
    let mut tree: BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<Value>>>> = BTreeMap::new();
    for row in rows {
        let Some((namespace, name)) = row.element.split_once(':') else { continue };
        if !KEEP.contains(&namespace) {
            stats.other_namespace += 1;
            continue;
        }
        let Some((duration, back)) = context_kind(&row.context) else {
            stats.other_context += 1;
            continue;
        };
        let Some(unit) = unit_name(&row.unit_id) else {
            stats.unknown_unit += usize::from(row.unit_id != "-" && !row.unit_id.is_empty());
            stats.not_numeric += usize::from(row.unit_id == "-" || row.unit_id.is_empty());
            continue;
        };
        // A value "-" is an explicit zero (書類閲覧操作ガイド, "ブランク（空白）と「-」との違い").
        let value: f64 = if row.value == "-" { 0.0 } else { match row.value.parse() { Ok(v) => v, Err(_) => { stats.not_numeric += 1; continue } } };
        let Some((start, end)) = period_of(duration, back) else {
            stats.other_context += 1;
            continue;
        };
        let fact = json!({"start": start, "end": end, "val": value, "form": FORM_ANNUAL, "filed": filed, "accn": doc_id});
        let list = tree.entry(namespace.to_string()).or_default().entry(name.to_string()).or_default().entry(unit).or_default();
        if !list.contains(&fact) {
            list.push(fact);
            stats.facts += 1;
        }
    }
    let mut facts = Map::new();
    for (namespace, concepts) in tree {
        let mut ns = Map::new();
        for (name, units) in concepts {
            let units: Map<String, Value> = units.into_iter().map(|(u, v)| (u, Value::Array(v))).collect();
            ns.insert(name, json!({ "units": units }));
        }
        facts.insert(namespace, Value::Object(ns));
    }
    let value = json!({
        "cik": dei.edinet_code.clone().unwrap_or_default(),
        "entityName": dei.name.clone().unwrap_or_default(),
        "facts": facts,
    });
    Ok((value, dei, stats))
}

/// Add the facts of `other` to `base` (the lists of the same tag and unit are joined).
pub fn merge_facts(base: &mut Value, other: Value) {
    let Some(other_ns) = other.get("facts").and_then(Value::as_object).cloned() else { return };
    if base.get("entityName").and_then(Value::as_str).unwrap_or("").is_empty() {
        base["entityName"] = other.get("entityName").cloned().unwrap_or(Value::Null);
        base["cik"] = other.get("cik").cloned().unwrap_or(Value::Null);
    }
    for (ns, concepts) in other_ns {
        for (name, concept) in concepts.as_object().cloned().unwrap_or_default() {
            for (unit, list) in concept.get("units").and_then(Value::as_object).cloned().unwrap_or_default() {
                let target = &mut base["facts"][&ns][&name]["units"][&unit];
                if !target.is_array() {
                    *target = json!([]);
                }
                let target = target.as_array_mut().expect("array");
                for fact in list.as_array().cloned().unwrap_or_default() {
                    if !target.contains(&fact) {
                        target.push(fact);
                    }
                }
            }
        }
    }
}

/// The facts of one ZIP answer (all CSV files of it).
pub fn facts_from_zip(zip: &[u8], doc_id: &str, filed: Option<&str>) -> Result<(Value, Dei, Stats), String> {
    let files = read_zip(zip)?;
    let mut csv: Vec<&(String, Vec<u8>)> = files.iter().filter(|(n, _)| n.to_lowercase().ends_with(".csv")).collect();
    csv.sort_by(|a, b| a.0.cmp(&b.0));
    if csv.is_empty() {
        return Err(format!("the ZIP file has no CSV file (files: {})", files.iter().map(|f| f.0.as_str()).collect::<Vec<_>>().join(", ")));
    }
    let mut all_rows = Vec::new();
    for (name, bytes) in csv {
        let text = decode_utf16(bytes).map_err(|e| format!("{name}: {e}"))?;
        let (rows, _) = parse_csv(&text).map_err(|e| format!("{name}: {e}"))?;
        all_rows.extend(rows);
    }
    facts_from_rows(&all_rows, doc_id, filed)
}

// ---------------------------------------------------------------------------------------------------------------
// The requests
// ---------------------------------------------------------------------------------------------------------------

/// One GET request: (HTTP code, body). The error text of a failure has no key.
pub trait Transport {
    fn get(&mut self, url: &str) -> Result<(u16, Vec<u8>), String>;
}

/// The real transport: the client of `http.rs` with a delay between requests.
pub struct Http {
    client: Client,
    key: String,
}

impl Http {
    pub fn new(key: &str, delay_seconds: f64) -> Http {
        // No redirect: the key is in the URL, and a redirect to another host would send it there.
        Http { client: Client::no_redirect_paced(USER_AGENT, delay_seconds.max(MIN_DELAY_SECONDS)), key: key.to_string() }
    }
}

impl Transport for Http {
    fn get(&mut self, url: &str) -> Result<(u16, Vec<u8>), String> {
        self.client.request("GET", url, &[], None).map_err(|e| scrub(&e.to_string(), &self.key))
    }
}

/// One document of the list of a day.
#[derive(Debug, Clone, PartialEq)]
pub struct DocMeta {
    pub doc_id: String,
    pub edinet_code: String,
    pub sec_code: String,
    pub filer_name: String,
    pub doc_type_code: String,
    pub period_start: String,
    pub period_end: String,
    pub submit_date_time: String,
    pub csv_flag: String,
    pub withdrawal_status: String,
}

impl DocMeta {
    fn from_value(v: &Value) -> DocMeta {
        let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        DocMeta {
            doc_id: s("docID"),
            edinet_code: s("edinetCode"),
            sec_code: s("secCode"),
            filer_name: s("filerName"),
            doc_type_code: s("docTypeCode"),
            period_start: s("periodStart"),
            period_end: s("periodEnd"),
            submit_date_time: s("submitDateTime"),
            csv_flag: s("csvFlag"),
            withdrawal_status: s("withdrawalStatus"),
        }
    }

    fn to_value(&self) -> Value {
        json!({"docID": self.doc_id, "edinetCode": self.edinet_code, "secCode": self.sec_code, "filerName": self.filer_name,
               "docTypeCode": self.doc_type_code, "periodStart": self.period_start, "periodEnd": self.period_end,
               "submitDateTime": self.submit_date_time, "csvFlag": self.csv_flag, "withdrawalStatus": self.withdrawal_status})
    }

    /// An annual securities report with a CSV form that is not withdrawn, of this security code.
    pub fn is_annual_report_of(&self, sec_code: &str) -> bool {
        self.sec_code == sec_code && self.doc_type_code == DOC_TYPE_ANNUAL && self.csv_flag == "1" && (self.withdrawal_status.is_empty() || self.withdrawal_status == "0")
    }
}

/// The EDINET security code of a Tokyo ticker: `8001.T` -> `80010` (the 4-character code and a check digit place 0).
pub fn security_code(ticker: &str) -> Result<String, String> {
    let code = ticker.to_uppercase();
    let code = code.strip_suffix(".T").ok_or_else(|| format!("'{ticker}' is not a Tokyo ticker (form 8001.T)"))?;
    if code.len() == 4 && code.chars().all(|c| c.is_ascii_digit() || c.is_ascii_uppercase()) {
        Ok(format!("{code}0"))
    } else {
        Err(format!("'{ticker}' is not a Tokyo ticker (form 8001.T)"))
    }
}

/// The requests of one run. Any error stops the run: the caller does not go on after an `Err`.
pub struct Session<T: Transport> {
    transport: T,
    key: String,
    pub requests: usize,
}

impl<T: Transport> Session<T> {
    pub fn new(transport: T, key: &str) -> Session<T> {
        Session { transport, key: key.to_string(), requests: 0 }
    }

    fn url(&self, path: &str, query: &str) -> String {
        format!("{API_BASE}/{path}?{query}&Subscription-Key={}", self.key)
    }

    /// The error of a JSON answer, if the body is the JSON of an error (EDINET answers some errors with HTTP 200).
    fn json_error(&self, body: &[u8]) -> Option<String> {
        let v: Value = serde_json::from_slice(body).ok()?;
        let code = v.get("StatusCode").and_then(Value::as_i64).or_else(|| v.pointer("/metadata/status").and_then(Value::as_str).and_then(|s| s.parse().ok()))?;
        if code == 200 {
            return None;
        }
        Some(Self::describe(code as u16))
    }

    fn describe(code: u16) -> String {
        match code {
            429 | 403 => format!("HTTP {code}: EDINET refuses more requests now. The run stops; try again later, with a longer delay"),
            401 => "HTTP 401: EDINET refuses the key (wrong, or not active)".to_string(),
            404 => "HTTP 404: EDINET has no such data (check the date or the document ID)".to_string(),
            c @ 300..=399 => format!("HTTP {c} from EDINET: a redirect, not followed (the key is in the URL); the run stops"),
            c => format!("HTTP {c} from EDINET"),
        }
    }

    /// One request. A code of 300 or more, or an error JSON, is an `Err`. Two connection failures in a row are an `Err`.
    fn call(&mut self, path: &str, query: &str) -> Result<Vec<u8>, String> {
        let url = self.url(path, query);
        let mut failures = 0;
        loop {
            self.requests += 1;
            match self.transport.get(&url) {
                Ok((code, body)) => {
                    // A 3xx is an error too: the client follows no redirect (the key is in the URL).
                    if code >= 300 {
                        return Err(Self::describe(code));
                    }
                    return Ok(body);
                }
                Err(e) => {
                    failures += 1;
                    if failures >= 2 {
                        return Err(format!("two connection failures in a row; the run stops ({})", scrub(&e, &self.key)));
                    }
                }
            }
        }
    }

    /// The documents of one file date (`documents.json?date=D&type=2`).
    pub fn list_day(&mut self, date: &str) -> Result<Vec<DocMeta>, String> {
        let body = self.call("documents.json", &format!("date={date}&type=2"))?;
        if let Some(e) = self.json_error(&body) {
            return Err(e);
        }
        let v: Value = serde_json::from_slice(&body).map_err(|e| format!("the list of {date} is not JSON: {e}"))?;
        Ok(v.get("results").and_then(Value::as_array).map(|a| a.iter().map(DocMeta::from_value).collect()).unwrap_or_default())
    }

    /// The CSV form of a document (`documents/<id>?type=5`), as the ZIP bytes.
    pub fn csv_zip(&mut self, doc_id: &str) -> Result<Vec<u8>, String> {
        if doc_id.is_empty() || !doc_id.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(format!("'{doc_id}' is not a document ID (for example S100XXXX)"));
        }
        let body = self.call(&format!("documents/{doc_id}"), "type=5")?;
        if !body.starts_with(b"PK") {
            return Err(self.json_error(&body).unwrap_or_else(|| "the answer is not a ZIP file (no CSV form for this document?)".to_string()));
        }
        Ok(body)
    }

    /// The first annual report of a security code in the file dates `from` to `to` (Saturdays and Sundays have no
    /// filing and are skipped). `progress` gets (date, document count).
    pub fn find_annual_report(&mut self, sec_code: &str, from: &str, to: &str, progress: &mut dyn FnMut(&str, usize)) -> Result<Option<DocMeta>, String> {
        let (a, b) = (dates::parse(from).ok_or("--from is not YYYY-MM-DD")?, dates::parse(to).ok_or("--to is not YYYY-MM-DD")?);
        if b < a {
            return Err("--to is before --from".to_string());
        }
        if b - a > 90 {
            return Err("the range is longer than 90 days: use a shorter one (one request for each weekday)".to_string());
        }
        for day in a..=b {
            if matches!((day + 4).rem_euclid(7), 0 | 6) {
                continue;
            }
            let date = dates::format(day);
            let docs = self.list_day(&date)?;
            progress(&date, docs.len());
            if let Some(found) = docs.into_iter().find(|d| d.is_annual_report_of(sec_code)) {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The cache
// ---------------------------------------------------------------------------------------------------------------

/// The folder of a ticker and the names of its EDINET files.
pub fn folder(cache: &Path, ticker: &str) -> PathBuf {
    cache.join(ticker)
}

fn zip_name(doc_id: &str) -> String {
    format!("edinet_{doc_id}_csv.zip")
}

fn record_name(doc_id: &str) -> String {
    format!("edinet_{doc_id}.json")
}

/// Write the ZIP answer and its record (source, date, request without the key). Returns the path of the ZIP file.
pub fn save_document(folder: &Path, doc_id: &str, zip: &[u8], meta: Option<&DocMeta>, filed_override: Option<&str>, today: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let zip_path = folder.join(zip_name(doc_id));
    std::fs::write(&zip_path, zip).map_err(|e| format!("{}: {e}", zip_path.display()))?;
    let record = json!({
        "source": "EDINET API v2, Financial Services Agency of Japan (https://api.edinet-fsa.go.jp/api/v2)",
        "licence": "Public Data License 1.0 (PDL 1.0). Name the source: EDINET, Financial Services Agency of Japan",
        "retrieved": today,
        "request": format!("GET {API_BASE}/documents/{doc_id}?type=5&Subscription-Key=<not stored>"),
        "file": zip_name(doc_id),
        "bytes": zip.len(),
        "filed_override": filed_override,
        "document": meta.map(DocMeta::to_value),
        "derived": "company_facts.json is made from the ZIP files of this folder (mapcheck fetch --edinet-rebuild); the amounts are in JPY",
    });
    let path = folder.join(record_name(doc_id));
    std::fs::write(&path, serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(zip_path)
}

/// Make `company_facts.json` from all `edinet_*_csv.zip` files of the folder. Returns the path and one text line for each document.
pub fn rebuild_facts(folder: &Path) -> Result<(PathBuf, Vec<String>), String> {
    let mut zips: Vec<PathBuf> = std::fs::read_dir(folder)
        .map_err(|e| format!("{}: {e}", folder.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("edinet_") && n.ends_with("_csv.zip")))
        .collect();
    zips.sort();
    if zips.is_empty() {
        return Err(format!("{}: no edinet_*_csv.zip file", folder.display()));
    }
    let mut merged = json!({"cik": "", "entityName": "", "facts": {}});
    let mut lines = Vec::new();
    for path in zips {
        let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let doc_id = file.trim_start_matches("edinet_").trim_end_matches("_csv.zip").to_string();
        let record: Option<Value> = std::fs::read_to_string(folder.join(record_name(&doc_id))).ok().and_then(|t| serde_json::from_str(&t).ok());
        let filed = record.as_ref().and_then(|r| {
            r.get("filed_override").and_then(Value::as_str).map(str::to_string).or_else(|| r.pointer("/document/submitDateTime").and_then(Value::as_str).and_then(|s| s.get(..10)).map(str::to_string))
        });
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (facts, dei, stats) = facts_from_zip(&bytes, &doc_id, filed.as_deref()).map_err(|e| format!("{file}: {e}"))?;
        lines.push(format!(
            "{doc_id}: {} | fiscal year {} to {} | accounting standard {} | {} rows, {} facts kept ({} other namespace, {} other context, {} not numeric, {} unknown unit)",
            dei.name.as_deref().unwrap_or("?"),
            dei.fy_start.as_deref().unwrap_or("?"),
            dei.fy_end.as_deref().unwrap_or("?"),
            dei.accounting_standard.as_deref().unwrap_or("not given"),
            stats.rows,
            stats.facts,
            stats.other_namespace,
            stats.other_context,
            stats.not_numeric,
            stats.unknown_unit
        ));
        merge_facts(&mut merged, facts);
    }
    let out = folder.join("company_facts.json");
    std::fs::write(&out, serde_json::to_vec(&merged).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok((out, lines))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::CompanyFacts;
    use crate::mapper::map_company;
    use crate::registry::Registry;
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    /// MADE FROM THE DOCUMENTATION, NOT A REAL ANSWER; replace with the real file at the first real read.
    const FIXTURE: &str = include_str!("../tests/fixtures/edinet_8001_type5_documentation_fixture.csv");
    const SECRET: &str = "k3y-SECRET-0123456789";

    /// The fixture as the real file is: the lines with # removed, UTF-16LE with a byte order mark.
    fn fixture_utf16() -> Vec<u8> {
        let text: String = FIXTURE.lines().filter(|l| !l.starts_with('#')).collect::<Vec<_>>().join("\n") + "\n";
        let mut out = vec![0xFF, 0xFE];
        for u in text.replace('\n', "\r\n").encode_utf16() {
            out.extend(u.to_le_bytes());
        }
        out
    }

    fn make_zip(files: &[(&str, Vec<u8>)], deflate: bool) -> Vec<u8> {
        let (mut out, mut central) = (Vec::new(), Vec::new());
        for (name, data) in files {
            let packed = if deflate {
                let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
                e.write_all(data).unwrap();
                e.finish().unwrap()
            } else {
                data.clone()
            };
            let offset = out.len() as u32;
            let method: u16 = if deflate { 8 } else { 0 };
            let mut local = vec![0x50, 0x4b, 0x03, 0x04];
            local.extend([20, 0, 0, 0]);
            local.extend(method.to_le_bytes());
            local.extend([0u8; 8]); // time, date, crc
            local.extend((packed.len() as u32).to_le_bytes());
            local.extend((data.len() as u32).to_le_bytes());
            local.extend((name.len() as u16).to_le_bytes());
            local.extend([0, 0]);
            local.extend(name.as_bytes());
            out.extend(&local);
            out.extend(&packed);
            let mut c = vec![0x50, 0x4b, 0x01, 0x02, 20, 0, 20, 0, 0, 0];
            c.extend(method.to_le_bytes());
            c.extend([0u8; 8]);
            c.extend((packed.len() as u32).to_le_bytes());
            c.extend((data.len() as u32).to_le_bytes());
            c.extend((name.len() as u16).to_le_bytes());
            c.extend([0u8; 12]);
            c.extend(offset.to_le_bytes());
            c.extend(name.as_bytes());
            central.extend(c);
        }
        let start = out.len() as u32;
        out.extend(&central);
        out.extend([0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]);
        out.extend((files.len() as u16).to_le_bytes());
        out.extend((files.len() as u16).to_le_bytes());
        out.extend((central.len() as u32).to_le_bytes());
        out.extend(start.to_le_bytes());
        out.extend([0, 0]);
        out
    }

    fn fixture_zip(deflate: bool) -> Vec<u8> {
        make_zip(&[("XBRL_TO_CSV/jpcrp030000-asr-001_E02513-000_2025-03-31_01_2025-06-20.csv", fixture_utf16())], deflate)
    }

    fn fixture_facts() -> (Value, Dei, Stats) {
        facts_from_zip(&fixture_zip(true), "S100TEST", None).unwrap()
    }

    #[test]
    fn zip_stored_and_deflate_are_read() {
        for deflate in [false, true] {
            let files = read_zip(&fixture_zip(deflate)).unwrap();
            assert_eq!(files.len(), 1);
            assert!(files[0].0.starts_with("XBRL_TO_CSV/"));
            assert_eq!(files[0].1, fixture_utf16());
        }
        assert!(read_zip(b"not a zip at all, not a zip at all").is_err());
    }

    #[test]
    fn the_csv_is_utf16_tab_separated_with_quotes_and_line_breaks_in_a_value() {
        let (rows, short) = parse_csv(&decode_utf16(&fixture_utf16()).unwrap()).unwrap();
        assert_eq!(short, 0);
        assert_eq!(rows[0].element, "jpdei_cor:EDINETCodeDEI");
        let cover = rows.iter().find(|r| r.element.ends_with("AnnualSecuritiesReportCoverPage")).unwrap();
        assert_eq!(cover.value, "有価証券報告書\r\n\"sample\" text");
        let revenue = rows.iter().find(|r| r.element == "jpigp_cor:RevenueIFRS" && r.context == "CurrentYearDuration").unwrap();
        assert_eq!((revenue.unit_id.as_str(), revenue.unit.as_str(), revenue.period_kind.as_str()), ("JPY", "円", "期間"));
        assert_eq!(revenue.value, "14724234000000");
        assert!(parse_csv("a\tb\n").is_err());
        assert!(decode_utf16(&[1, 2, 3]).is_err());
    }

    #[test]
    fn the_facts_have_the_periods_of_the_contexts_and_skip_the_rest() {
        let (facts, dei, stats) = fixture_facts();
        assert_eq!(dei.edinet_code.as_deref(), Some("E02513"));
        assert_eq!((dei.fy_start.as_deref(), dei.fy_end.as_deref()), (Some("2024-04-01"), Some("2025-03-31")));
        assert_eq!(dei.accounting_standard.as_deref(), Some("IFRS"));
        assert_eq!(facts["entityName"], "ITOCHU Corporation");
        let revenue = &facts["facts"]["jpigp_cor"]["RevenueIFRS"]["units"]["JPY"];
        let revenue = revenue.as_array().unwrap();
        // The segment member, the year-to-date context and the other namespace are not in the list.
        assert_eq!(revenue.len(), 2);
        assert_eq!(revenue[0]["start"], "2024-04-01");
        assert_eq!(revenue[0]["end"], "2025-03-31");
        assert_eq!(revenue[0]["val"], 14724234e6);
        assert_eq!(revenue[0]["form"], FORM_ANNUAL);
        assert_eq!(revenue[0]["filed"], "2025-06-20");
        assert_eq!(revenue[0]["accn"], "S100TEST");
        assert_eq!((revenue[1]["start"].as_str(), revenue[1]["end"].as_str()), (Some("2023-04-01"), Some("2024-03-31")));
        let assets = facts["facts"]["jpigp_cor"]["AssetsIFRS"]["units"]["JPY"].as_array().unwrap();
        assert!(assets[0]["start"].is_null() && assets[0]["end"] == "2025-03-31");
        // The summary item of two years before: the dates are shifted from the fiscal year.
        let old = facts["facts"]["jpcrp_cor"]["RevenueIFRSSummaryOfBusinessResults"]["units"]["JPY"].as_array().unwrap();
        assert_eq!((old[0]["start"].as_str(), old[0]["end"].as_str()), (Some("2022-04-01"), Some("2023-03-31")));
        assert!(facts["facts"].get("jppfs_cor").is_none() && facts["facts"].get("jpdei_cor").is_none());
        assert_eq!(facts["facts"]["jpcrp_cor"]["BasicEarningsLossPerShareIFRSSummaryOfBusinessResults"]["units"]["JPY/shares"][0]["val"], 615.65);
        // A "-" value is an explicit zero.
        assert_eq!(facts["facts"]["jpigp_cor"]["ProfitLossFromDiscontinuedOperationsIFRS"]["units"]["JPY"][0]["val"], 0.0);
        assert!(stats.facts == 35 && stats.other_context == 4 && stats.other_namespace == 9);
    }

    #[test]
    fn a_report_that_is_not_ifrs_is_refused() {
        let text = decode_utf16(&fixture_utf16()).unwrap().replace("\"IFRS\"", "\"Japan GAAP\"");
        let (rows, _) = parse_csv(&text).unwrap();
        let e = facts_from_rows(&rows, "S1", None).unwrap_err();
        assert!(e.contains("Japan GAAP") && e.contains("IFRS"), "{e}");
    }

    #[test]
    fn the_standard_variables_come_from_the_facts_in_jpy() {
        let registry = Registry::embedded().unwrap();
        let (value, _, _) = fixture_facts();
        let facts: CompanyFacts = serde_json::from_value(value).unwrap();
        assert_eq!(facts.currency().as_deref(), Some("JPY"));
        let company = map_company(&registry, &facts, 12);
        assert_eq!(company.currency, "JPY");
        assert_eq!(company.name, "ITOCHU Corporation");
        let ends: Vec<&str> = company.annual.iter().map(|p| p.end.as_str()).collect();
        assert_eq!(ends, ["2022-03-31", "2023-03-31", "2024-03-31", "2025-03-31"]);
        let p = company.annual.last().unwrap();
        for (name, value) in [
            ("revenue", 14724234e6),
            ("net_income", 880251e6),
            ("net_income_incl_nci", 933015e6),
            ("operating_cash_flow", 997278e6),
            ("capex", -227465e6),
            ("total_assets", 15134264e6),
            ("total_liabilities", 8843552e6),
            ("equity", 5755072e6),
            ("total_equity_incl_nci", 6290712e6),
            ("cash_and_equivalents", 549573e6),
            ("gross_profit", 2376456e6),
        ] {
            assert_eq!(p.get(&registry, name), Some(value), "{name}");
        }
        assert_eq!(p.source(&registry, "revenue"), "jpigp_cor/RevenueIFRS");
        // ITOCHU shows no operating profit line; the registry has no clear element for the shares.
        assert_eq!(p.get(&registry, "operating_income"), None);
        assert_eq!(p.get(&registry, "shares_outstanding"), None);
        // An older year comes from the summary of business results.
        let old = &company.annual[1];
        assert_eq!(old.get(&registry, "revenue"), Some(13945633e6));
        assert_eq!(old.source(&registry, "revenue"), "jpcrp_cor/RevenueIFRSSummaryOfBusinessResults");
        // The store marks the currency: fa refuses it (Company::usd_only).
        let store = crate::store::store_json(&registry, "8001.T", "E02513", &company);
        assert_eq!(store["periods"][3]["currency"], "JPY");
        assert_eq!(store["periods"][3]["form"], FORM_ANNUAL);
    }

    // ---- the key and the requests ----

    #[test]
    fn a_missing_key_has_a_clear_message_and_the_env_file_is_read() {
        let dir = std::env::temp_dir().join(format!("mapcheck-edinet-key-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(key_from(None, std::slice::from_ref(&dir)).unwrap_err(), "EDINET_API_KEY not set: this read runs on a laptop with the key");
        assert!(key_from(Some("  "), std::slice::from_ref(&dir)).is_err());
        std::fs::write(dir.join(".env"), "FRED_API_KEY=a\nexport EDINET_API_KEY=\"abc123\"\n").unwrap();
        assert_eq!(key_from(None, std::slice::from_ref(&dir)).unwrap(), "abc123");
        assert_eq!(key_from(Some("fromenv"), std::slice::from_ref(&dir)).unwrap(), "fromenv");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scrub_takes_out_the_key_and_the_parameter() {
        let text = format!("https://api.edinet-fsa.go.jp/api/v2/documents.json?date=2025-06-20&type=2&Subscription-Key={SECRET}&x=1: timeout {SECRET}");
        let clean = scrub(&text, SECRET);
        assert!(!clean.contains(SECRET), "{clean}");
        assert!(clean.contains("Subscription-Key=***&x=1"));
        // Without the key value known, the parameter value still goes.
        assert!(!scrub(&text, "").contains("Subscription-Key=k3y"));
    }

    /// A transport from a list of answers; each URL is kept.
    struct Fake {
        answers: Vec<Result<(u16, Vec<u8>), String>>,
        urls: Vec<String>,
    }

    impl Transport for Fake {
        fn get(&mut self, url: &str) -> Result<(u16, Vec<u8>), String> {
            self.urls.push(url.to_string());
            if self.answers.is_empty() {
                return Err(format!("{url}: no more answers"));
            }
            self.answers.remove(0)
        }
    }

    fn session(answers: Vec<Result<(u16, Vec<u8>), String>>) -> Session<Fake> {
        Session::new(Fake { answers, urls: Vec::new() }, SECRET)
    }

    fn list_json(rows: &[(&str, &str, &str)]) -> Vec<u8> {
        let results: Vec<Value> = rows
            .iter()
            .map(|(id, sec, ty)| json!({"docID": id, "edinetCode": "E02513", "secCode": sec, "filerName": "ITOCHU Corporation", "docTypeCode": ty,
                "periodStart": "2024-04-01", "periodEnd": "2025-03-31", "submitDateTime": "2025-06-20 09:00", "csvFlag": "1", "withdrawalStatus": "0"}))
            .collect();
        serde_json::to_vec(&json!({"metadata": {"status": "200", "message": "OK"}, "results": results})).unwrap()
    }

    #[test]
    fn the_scan_skips_weekends_and_finds_the_annual_report() {
        // 2025-06-19 is a Thursday, 2025-06-20 a Friday, 2025-06-21 a Saturday.
        let mut s = session(vec![
            Ok((200, list_json(&[("S1", "99990", "120")]))),
            Ok((200, list_json(&[("S2", "80010", "140"), ("S3", "80010", "120")]))),
        ]);
        let mut seen = Vec::new();
        let found = s.find_annual_report("80010", "2025-06-19", "2025-06-23", &mut |d, n| seen.push((d.to_string(), n))).unwrap().unwrap();
        assert_eq!(found.doc_id, "S3");
        assert_eq!(seen, [("2025-06-19".to_string(), 1), ("2025-06-20".to_string(), 2)]);
        assert!(s.transport.urls[0].starts_with("https://api.edinet-fsa.go.jp/api/v2/documents.json?date=2025-06-19&type=2&Subscription-Key="));
        // Only the weekdays are asked: Fri, then Mon and Tue in a longer range.
        let mut s = session(vec![Ok((200, list_json(&[]))), Ok((200, list_json(&[]))), Ok((200, list_json(&[])))]);
        assert!(s.find_annual_report("80010", "2025-06-20", "2025-06-24", &mut |_, _| {}).unwrap().is_none());
        assert_eq!(s.requests, 3);
        assert!(s.find_annual_report("80010", "2025-06-24", "2025-06-20", &mut |_, _| {}).is_err());
    }

    #[test]
    fn a_429_or_403_stops_the_run_also_in_the_json_body() {
        for code in [429u16, 403] {
            let mut s = session(vec![Ok((code, b"{}".to_vec()))]);
            let e = s.list_day("2025-06-20").unwrap_err();
            assert!(e.contains(&code.to_string()) && e.contains("stops"), "{e}");
        }
        let mut s = session(vec![Ok((200, br#"{"StatusCode": 429, "message": "Too Many Requests"}"#.to_vec()))]);
        assert!(s.list_day("2025-06-20").unwrap_err().contains("429"));
        let mut s = session(vec![Ok((200, br#"{"StatusCode": 401, "message": "Access denied due to invalid subscription key."}"#.to_vec()))]);
        assert!(s.list_day("2025-06-20").unwrap_err().contains("key"));
        let mut s = session(vec![Ok((200, br#"{"metadata": {"title": "x", "status": "404", "message": "Not Found"}}"#.to_vec()))]);
        assert!(s.csv_zip("S1").unwrap_err().contains("404"));
    }

    #[test]
    fn two_connection_failures_in_a_row_stop_and_one_failure_is_tried_again() {
        let url_error = |u: &str| Err(format!("https://x/?Subscription-Key={SECRET}: {u}"));
        let mut s = session(vec![url_error("timeout"), url_error("reset")]);
        let e = s.list_day("2025-06-20").unwrap_err();
        assert!(e.contains("two connection failures"), "{e}");
        assert!(!e.contains(SECRET), "{e}");
        let mut s = session(vec![url_error("timeout"), Ok((200, list_json(&[("S1", "80010", "120")])))]);
        assert_eq!(s.list_day("2025-06-20").unwrap().len(), 1);
        assert_eq!(s.requests, 2);
    }

    #[test]
    fn no_error_text_has_the_key() {
        // The fake transport puts the full URL and the key in its error text, as a real client can.
        let mut s = session(vec![Err(format!("{API_BASE}/documents.json?date=d&Subscription-Key={SECRET}: boom")), Err(format!("again {SECRET}"))]);
        let e = s.list_day("2025-06-20").unwrap_err();
        assert!(!e.contains(SECRET) && !e.contains("Subscription-Key=k3y"), "{e}");
        let e = session(vec![Ok((307, Vec::new()))]).csv_zip("S1").unwrap_err();
        assert!(e.contains("redirect, not followed"), "{e}");
        assert_eq!(scrub(&format!("x {SECRET} y"), ""), format!("x {SECRET} y"));
        assert_eq!(scrub("a k3 b", "k3"), "a *** b");
        for code in [400u16, 401, 404, 429, 500] {
            let e = session(vec![Ok((code, format!("body {SECRET}").into_bytes()))]).csv_zip("S1").unwrap_err();
            assert!(!e.contains(SECRET), "{e}");
        }
        assert!(session(vec![]).csv_zip("S1/../x").unwrap_err().contains("not a document ID"));
    }

    #[test]
    fn the_document_request_is_type_5_and_a_non_zip_answer_is_an_error() {
        let zip = fixture_zip(true);
        let mut s = session(vec![Ok((200, zip.clone()))]);
        assert_eq!(s.csv_zip("S100TEST").unwrap(), zip);
        assert!(s.transport.urls[0].starts_with("https://api.edinet-fsa.go.jp/api/v2/documents/S100TEST?type=5&Subscription-Key="));
        let mut s = session(vec![Ok((200, b"<html>maintenance</html>".to_vec()))]);
        assert!(s.csv_zip("S1").unwrap_err().contains("not a ZIP"));
    }

    #[test]
    fn tokyo_tickers_have_a_security_code() {
        assert_eq!(security_code("8001.T").unwrap(), "80010");
        assert_eq!(security_code("285a.t").unwrap(), "285A0");
        assert!(security_code("IBM").is_err() && security_code("80011.T").is_err());
    }

    #[test]
    fn the_cache_has_the_zip_and_a_record_without_the_key_and_the_facts_file_is_made() {
        let dir = std::env::temp_dir().join(format!("mapcheck-edinet-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let folder = folder(&dir, "8001.T");
        let meta = DocMeta::from_value(&json!({"docID": "S100TEST", "edinetCode": "E02513", "secCode": "80010", "docTypeCode": "120", "submitDateTime": "2025-06-20 09:00", "csvFlag": "1"}));
        save_document(&folder, "S100TEST", &fixture_zip(true), Some(&meta), None, "2026-11-25").unwrap();
        let record = std::fs::read_to_string(folder.join("edinet_S100TEST.json")).unwrap();
        assert!(record.contains("Public Data License 1.0") && record.contains("2026-11-25") && record.contains("<not stored>"));
        assert!(!record.contains("k3y") && !record.to_lowercase().contains("secret"));
        let (path, lines) = rebuild_facts(&folder).unwrap();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("ITOCHU Corporation") && lines[0].contains("2025-03-31"));
        // The filing date comes from the record of the list (the submit date), as the facts file is read back.
        let facts = crate::run::read_facts(&path).unwrap();
        assert_eq!(facts.currency().as_deref(), Some("JPY"));
        assert_eq!(crate::run::facts_path(&dir, "8001.T"), Some(path));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
