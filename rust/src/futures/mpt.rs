//! Atlanta Fed Market Probability Tracker (MPT): the path of the policy rate that the
//! prices of CME 3-month SOFR options imply.
//!
//! The Bank gives one Excel file (xlsx) with all days since 2023-03-29. The program reads
//! the sheet DATA: one row for each day, reference window and statistic. The file is a zip
//! of XML files; the program has a small zip reader (stored and deflate entries).

use super::{load, number, Result};
use flate2::read::DeflateDecoder;
use mapcheck::dates;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

pub const URL: &str = "https://www.atlantafed.org/-/media/Project/Atlanta/FRBA/Documents/research-and-data/data/market-probability-tracker/mpt_histdata.xlsx";
pub const PAGE: &str = "https://www.atlantafed.org/research-and-data/data/market-probability-tracker";
pub const SOURCE: &str = "Federal Reserve Bank of Atlanta, Market Probability Tracker (mpt_histdata.xlsx)";
const MAX_AGE: Duration = Duration::from_secs(12 * 3600);
/// Days from 1899-12-30 (Excel day 0) to 1970-01-01.
const EXCEL_TO_UNIX: i64 = 25_569;

// ---------------------------------------------------------------------------
// Zip and XML
// ---------------------------------------------------------------------------

fn u16_at(b: &[u8], i: usize) -> Result<usize> {
    b.get(i..i + 2).map(|s| u16::from_le_bytes([s[0], s[1]]) as usize).ok_or_else(|| "zip file is too short".to_string())
}

fn u32_at(b: &[u8], i: usize) -> Result<usize> {
    b.get(i..i + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize).ok_or_else(|| "zip file is too short".to_string())
}

/// The bytes of one file of a zip archive. Zip64 and encryption are not supported.
pub fn zip_entry(zip: &[u8], name: &str) -> Result<Vec<u8>> {
    let end = (0..zip.len().saturating_sub(21))
        .rev()
        .take(70_000)
        .find(|&i| zip[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])
        .ok_or("this is not a zip file (no end record)")?;
    let count = u16_at(zip, end + 10)?;
    let mut at = u32_at(zip, end + 16)?;
    for _ in 0..count {
        if zip.get(at..at + 4) != Some(&[0x50, 0x4b, 0x01, 0x02]) {
            return Err("zip central directory is damaged".into());
        }
        let method = u16_at(zip, at + 10)?;
        let (compressed, size) = (u32_at(zip, at + 20)?, u32_at(zip, at + 24)?);
        let (n, m, k) = (u16_at(zip, at + 28)?, u16_at(zip, at + 30)?, u16_at(zip, at + 32)?);
        let local = u32_at(zip, at + 42)?;
        let entry_name = zip.get(at + 46..at + 46 + n).ok_or("zip central directory is damaged")?;
        if entry_name == name.as_bytes() {
            if compressed == 0xFFFF_FFFF || size == 0xFFFF_FFFF {
                return Err("zip64 files are not supported".into());
            }
            if zip.get(local..local + 4) != Some(&[0x50, 0x4b, 0x03, 0x04]) {
                return Err("zip local header is damaged".into());
            }
            let start = local + 30 + u16_at(zip, local + 26)? + u16_at(zip, local + 28)?;
            let data = zip.get(start..start + compressed).ok_or("zip entry is cut off")?;
            return match method {
                0 => Ok(data.to_vec()),
                8 => {
                    let mut out = Vec::with_capacity(size);
                    DeflateDecoder::new(data).read_to_end(&mut out).map_err(|e| format!("zip entry {name}: {e}"))?;
                    if out.len() != size {
                        return Err(format!("zip entry {name} has {} bytes; the directory says {size}", out.len()));
                    }
                    Ok(out)
                }
                other => Err(format!("zip entry {name} has compression method {other}; the program reads 0 and 8")),
            };
        }
        at += 46 + n + m + k;
    }
    Err(format!("zip file has no entry {name}"))
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// The value of an XML attribute in a start tag, for example attr("<c r=\"A1\" t=\"s\">", "t").
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let mut from = 0;
    while let Some(i) = tag[from..].find(&key) {
        let at = from + i;
        if at == 0 || tag.as_bytes()[at - 1].is_ascii_whitespace() {
            let start = at + key.len();
            let len = tag[start..].find('"')?;
            return Some(&tag[start..start + len]);
        }
        from = at + 1;
    }
    None
}

/// The text of each `<si>` of sharedStrings.xml.
fn shared_strings(xml: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<si>") {
        let body_start = i + 4;
        let end = rest[body_start..].find("</si>").ok_or("sharedStrings.xml is cut off")? + body_start;
        let mut s = String::new();
        let mut part = &rest[body_start..end];
        while let Some(t) = part.find("<t") {
            let open_end = part[t..].find('>').ok_or("sharedStrings.xml is damaged")? + t + 1;
            let close = part[open_end..].find("</t>").ok_or("sharedStrings.xml is damaged")? + open_end;
            // `<t/>` has no text.
            if !part[t..open_end].ends_with("/>") {
                s.push_str(&unescape(&part[open_end..close]));
            }
            part = &part[close + 4..];
        }
        out.push(s);
        rest = &rest[end + 5..];
    }
    Ok(out)
}

/// The path of the sheet that has this name, from workbook.xml and its relations.
fn sheet_path(zip: &[u8], sheet: &str) -> Result<String> {
    let workbook = String::from_utf8(zip_entry(zip, "xl/workbook.xml")?).map_err(|_| "workbook.xml is not UTF-8")?;
    let rels = String::from_utf8(zip_entry(zip, "xl/_rels/workbook.xml.rels")?).map_err(|_| "workbook.xml.rels is not UTF-8")?;
    let tag = workbook
        .split("<sheet ")
        .skip(1)
        .map(|t| t.split('>').next().unwrap_or(""))
        .find(|t| attr(t, "name") == Some(sheet))
        .ok_or_else(|| format!("the workbook has no sheet {sheet}"))?;
    let id = attr(tag, "r:id").ok_or("the sheet has no r:id")?;
    let rel = rels
        .split("<Relationship ")
        .skip(1)
        .map(|t| t.split('>').next().unwrap_or(""))
        .find(|t| attr(t, "Id") == Some(id))
        .ok_or_else(|| format!("no relation {id} in workbook.xml.rels"))?;
    let target = attr(rel, "Target").ok_or("the relation has no Target")?;
    Ok(format!("xl/{}", target.trim_start_matches('/').trim_start_matches("xl/")))
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct Row {
    day: i64,
    reference_start: i64,
    range: u32,
    field: u32,
    value: f64,
}

#[derive(Debug)]
pub struct Dataset {
    strings: Vec<String>,
    rows: Vec<Row>,
}

const HEADER: [&str; 5] = ["date", "reference_start", "target_range", "field", "value"];

/// The cells of one `<row>` as (column letter, type, text).
fn cells(row: &str) -> Vec<(char, &str, &str)> {
    let mut out = Vec::new();
    let mut rest = row;
    while let Some(i) = rest.find("<c ") {
        let tag_end = match rest[i..].find('>') {
            Some(e) => i + e,
            None => break,
        };
        let tag = &rest[i..tag_end];
        let column = attr(tag, "r").and_then(|r| r.chars().next()).unwrap_or('?');
        let kind = attr(tag, "t").unwrap_or("n");
        if tag.ends_with('/') {
            rest = &rest[tag_end + 1..];
            continue;
        }
        let close = rest[tag_end..].find("</c>").map_or(rest.len(), |c| tag_end + c);
        let body = &rest[tag_end + 1..close];
        let value = body.find("<v>").and_then(|a| body[a + 3..].find("</v>").map(|b| &body[a + 3..a + 3 + b])).unwrap_or("");
        out.push((column, kind, value));
        rest = &rest[close.min(rest.len())..];
        rest = rest.strip_prefix("</c>").unwrap_or(rest);
    }
    out
}

pub fn parse_xlsx(bytes: &[u8]) -> Result<Dataset> {
    let strings = shared_strings(&String::from_utf8(zip_entry(bytes, "xl/sharedStrings.xml")?).map_err(|_| "sharedStrings.xml is not UTF-8")?)?;
    let path = sheet_path(bytes, "DATA")?;
    let sheet = String::from_utf8(zip_entry(bytes, &path)?).map_err(|_| format!("{path} is not UTF-8"))?;
    let data = sheet.find("<sheetData>").ok_or("the sheet has no sheetData")? + 11;
    let mut rows = Vec::new();
    let mut seen_header = false;
    for (n, row) in sheet[data..].split("<row ").skip(1).enumerate() {
        let row = row.split("</row>").next().unwrap_or("");
        let c = cells(row);
        let string = |i: usize, want: char| -> Result<u32> {
            match c.get(i) {
                Some(&(col, "s", v)) if col == want => v.parse::<u32>().ok().filter(|k| (*k as usize) < strings.len()).ok_or_else(|| format!("sheet row {}: bad string index '{v}'", n + 1)),
                _ => Err(format!("sheet row {}: column {want} is not a text cell", n + 1)),
            }
        };
        if !seen_header {
            let names: Vec<&str> = (0..5).map(|i| string(i, (b'A' + i as u8) as char).map(|k| strings[k as usize].as_str())).collect::<Result<_>>()?;
            if names != HEADER {
                return Err(format!("the DATA sheet has the header {names:?}; expected {HEADER:?}"));
            }
            seen_header = true;
            continue;
        }
        if c.is_empty() {
            continue;
        }
        let date_text = &strings[string(0, 'A')? as usize];
        let day = dates::parse(date_text).ok_or_else(|| format!("sheet row {}: bad date '{date_text}'", n + 1))?;
        let start = match c.get(1) {
            Some(&('B', _, v)) => number(v, &format!("sheet row {}: reference start", n + 1))? as i64 - EXCEL_TO_UNIX,
            _ => return Err(format!("sheet row {}: column B is missing", n + 1)),
        };
        let value_text = &strings[string(4, 'E')? as usize];
        rows.push(Row {
            day,
            reference_start: start,
            range: string(2, 'C')?,
            field: string(3, 'D')?,
            value: number(value_text, &format!("sheet row {}: value", n + 1))?,
        });
    }
    if rows.is_empty() {
        return Err("the DATA sheet has no data rows".into());
    }
    Ok(Dataset { strings, rows })
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Window {
    /// The first day of the 3-month reference window of the SOFR contract.
    pub reference_start: String,
    /// The FOMC target range on the observation date, for example "375bps - 400bps".
    pub target_range: String,
    /// Mean of the implied distribution of the average SOFR (decimal: 0.0388 = 3.88%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_mean: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_mode: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_p25: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_p75: Option<f64>,
    /// Probability that the average SOFR is below the target range (0.25 = 25%).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prob_below_range: Option<f64>,
    /// Probability that the average SOFR is above the target range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prob_above_range: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PolicyPath {
    pub date: String,
    pub requested_date: String,
    pub first_date_in_file: String,
    pub source: String,
    pub url: String,
    pub note: String,
    pub windows: Vec<Window>,
}

fn rate(bps: f64) -> f64 {
    (bps / 10_000.0 * 1e8).round() / 1e8
}

fn fraction(percent: f64) -> f64 {
    (percent / 100.0 * 1e8).round() / 1e8
}

pub fn path_on(ds: &Dataset, requested: &str, day: i64) -> Result<PolicyPath> {
    let first = ds.rows.iter().map(|r| r.day).min().expect("rows exist");
    let used = ds
        .rows
        .iter()
        .map(|r| r.day)
        .filter(|d| *d <= day)
        .max()
        .ok_or_else(|| format!("no MPT data on or before {requested}; the first date is {}", dates::format(first)))?;
    let mut windows: BTreeMap<i64, Window> = BTreeMap::new();
    for r in ds.rows.iter().filter(|r| r.day == used) {
        let w = windows.entry(r.reference_start).or_default();
        w.reference_start = dates::format(r.reference_start);
        w.target_range = ds.strings[r.range as usize].clone();
        match ds.strings[r.field as usize].as_str() {
            "Rate: mean" => w.rate_mean = Some(rate(r.value)),
            "Rate: mode" => w.rate_mode = Some(rate(r.value)),
            "Rate: 25th percentile" => w.rate_p25 = Some(rate(r.value)),
            "Rate: 75th percentile" => w.rate_p75 = Some(rate(r.value)),
            "Prob: cut" => w.prob_below_range = Some(fraction(r.value)),
            "Prob: hike" => w.prob_above_range = Some(fraction(r.value)),
            _ => {}
        }
    }
    let mut note = String::from(
        "An estimate from the prices of CME options on 3-month SOFR futures. It is not a forecast of the Bank, the Federal \
         Reserve System or the FOMC. The Bank gives the rate in basis points; the program gives decimals. Use of the data \
         is for personal and educational purposes only (the file says so).",
    );
    if dates::format(used) != requested {
        note.push_str(&format!(" No row for {requested}; the day {} is the last one before it.", dates::format(used)));
    }
    Ok(PolicyPath {
        date: dates::format(used),
        requested_date: requested.into(),
        first_date_in_file: dates::format(first),
        source: SOURCE.into(),
        url: URL.into(),
        note,
        windows: windows.into_values().collect(),
    })
}

pub fn policy_path(repo: &Path, date: Option<&str>) -> Result<PolicyPath> {
    let (text, day) = crate::forwards::parse_date_arg(date.unwrap_or("today"))?;
    let ds = load(repo, "mpt_histdata.xlsx", URL, MAX_AGE, parse_xlsx, |_| false)?;
    path_on(&ds, &text, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zip with stored entries. The reader does not check the CRC.
    fn make_zip(files: &[(&str, &str)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for (name, body) in files {
            let offset = out.len() as u32;
            let (n, len) = (name.len() as u16, body.len() as u32);
            out.extend([0x50, 0x4b, 0x03, 0x04, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend(len.to_le_bytes());
            out.extend(len.to_le_bytes());
            out.extend(n.to_le_bytes());
            out.extend([0, 0]);
            out.extend(name.as_bytes());
            out.extend(body.as_bytes());
            directory.extend([0x50, 0x4b, 0x01, 0x02, 20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            directory.extend(len.to_le_bytes());
            directory.extend(len.to_le_bytes());
            directory.extend(n.to_le_bytes());
            directory.extend([0; 12]);
            directory.extend(offset.to_le_bytes());
            directory.extend(name.as_bytes());
        }
        let (dir_at, dir_len) = (out.len() as u32, directory.len() as u32);
        out.extend(directory);
        out.extend([0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]);
        out.extend((files.len() as u16).to_le_bytes());
        out.extend((files.len() as u16).to_le_bytes());
        out.extend(dir_len.to_le_bytes());
        out.extend(dir_at.to_le_bytes());
        out.extend([0, 0]);
        out
    }

    /// Strings: 0..4 header, 5 date, 6 range, 7 "Rate: mean", 8 "Rate: mode", 9 value " 388.5",
    /// 10 value "380", 11 "Prob: cut", 12 value "12.5", 13 other date.
    fn xlsx(extra_row: &str) -> Vec<u8> {
        let strings = ["date", "reference_start", "target_range", "field", "value", "2026-11-17", "375bps - 400bps", "Rate: mean", "Rate: mode", " 388.5", "380", "Prob: cut", "12.5", "2026-11-18"];
        let sst: String = strings.iter().map(|s| format!("<si><t xml:space=\"preserve\">{s}</t></si>")).collect();
        let row = |r: u32, d: u32, start: u32, f: u32, v: u32| {
            format!("<row r=\"{r}\"><c r=\"A{r}\" t=\"s\"><v>{d}</v></c><c r=\"B{r}\" s=\"7\" t=\"n\"><v>{start}</v></c><c r=\"C{r}\" t=\"s\"><v>6</v></c><c r=\"D{r}\" t=\"s\"><v>{f}</v></c><c r=\"E{r}\" t=\"s\"><v>{v}</v></c></row>")
        };
        let header = "<row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" t=\"s\"><v>1</v></c><c r=\"C1\" t=\"s\"><v>2</v></c><c r=\"D1\" t=\"s\"><v>3</v></c><c r=\"E1\" t=\"s\"><v>4</v></c></row>";
        // 2026-12-16 as Excel serial: 20_803 + 25_569 = 46_372 (2026-12-16 is day 20_803)
        let sheet = format!(
            "<worksheet><sheetData>{header}{}{}{}{}{extra_row}</sheetData></worksheet>",
            row(2, 5, 46_372, 7, 9),
            row(3, 5, 46_372, 8, 10),
            row(4, 5, 46_372, 11, 12),
            row(5, 13, 46_372, 7, 10)
        );
        let workbook = "<workbook><sheets><sheet name=\"LICENSE\" sheetId=\"2\" r:id=\"rId1\"/><sheet name=\"DATA\" sheetId=\"4\" r:id=\"rId3\"/></sheets></workbook>";
        let rels = "<Relationships><Relationship Id=\"rId1\" Target=\"worksheets/sheet1.xml\"/><Relationship Id=\"rId3\" Target=\"worksheets/sheet3.xml\"/></Relationships>";
        make_zip(&[
            ("xl/workbook.xml", workbook),
            ("xl/_rels/workbook.xml.rels", rels),
            ("xl/sharedStrings.xml", &format!("<sst>{sst}</sst>")),
            ("xl/worksheets/sheet3.xml", &sheet),
        ])
    }

    #[test]
    fn reads_a_zip_entry() {
        let z = make_zip(&[("a.txt", "hello"), ("b/c.txt", "world")]);
        assert_eq!(zip_entry(&z, "b/c.txt").unwrap(), b"world");
        assert!(zip_entry(&z, "x").unwrap_err().contains("no entry"));
        assert!(zip_entry(b"not a zip file at all, no end record here", "a").unwrap_err().contains("not a zip"));
        assert!(zip_entry(&z[..z.len() - 30], "a.txt").is_err());
    }

    #[test]
    fn policy_path_from_a_small_workbook() {
        assert_eq!(dates::format(46_372 - EXCEL_TO_UNIX), "2026-12-16");
        let ds = parse_xlsx(&xlsx("")).unwrap();
        let p = path_on(&ds, "2026-11-17", dates::parse("2026-11-17").unwrap()).unwrap();
        assert_eq!(p.date, "2026-11-17");
        assert_eq!(p.windows.len(), 1);
        let w = &p.windows[0];
        assert_eq!(w.reference_start, "2026-12-16");
        assert_eq!(w.target_range, "375bps - 400bps");
        assert_eq!((w.rate_mean, w.rate_mode, w.prob_below_range), (Some(0.03885), Some(0.038), Some(0.125)));
        assert!(!p.note.contains("No row for"));
        // A later request date uses the last row on or before it.
        let p = path_on(&ds, "2026-11-30", dates::parse("2026-11-30").unwrap()).unwrap();
        assert_eq!(p.date, "2026-11-18");
        assert!(p.note.contains("No row for 2026-11-30"));
        let e = path_on(&ds, "2020-01-01", dates::parse("2020-01-01").unwrap()).unwrap_err();
        assert!(e.contains("first date is 2026-11-17"), "{e}");
    }

    #[test]
    fn workbook_errors() {
        let bad_value = xlsx("<row r=\"6\"><c r=\"A6\" t=\"s\"><v>5</v></c><c r=\"B6\" t=\"n\"><v>46372</v></c><c r=\"C6\" t=\"s\"><v>6</v></c><c r=\"D6\" t=\"s\"><v>7</v></c><c r=\"E6\" t=\"s\"><v>8</v></c></row>");
        assert!(parse_xlsx(&bad_value).unwrap_err().contains("bad number"));
        assert!(parse_xlsx(b"PK nothing").is_err());
        let no_data = make_zip(&[("xl/workbook.xml", "<workbook><sheets><sheet name=\"X\" r:id=\"rId1\"/></sheets></workbook>"), ("xl/_rels/workbook.xml.rels", "<Relationships/>"), ("xl/sharedStrings.xml", "<sst/>")]);
        assert!(parse_xlsx(&no_data).unwrap_err().contains("no sheet DATA"));
    }
}
