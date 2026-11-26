//! Commodity monthly prices: the World Bank Commodity Price Data ("Pink Sheet"), monthly file.
//!
//! `fa commodity get gold` downloads `CMO-Historical-Data-Monthly.xlsx` (sheet "Monthly Prices": one row for each month
//! since 1960, one column for each commodity) and keeps each copy with the download date and a record (address, date,
//! size, SHA-256, licence). `fa commodity show gold` prints the months of the gold column.
//! Terms (read 2026-11-25): the World Bank licenses its own datasets under CC BY 4.0 by default
//! (https://datacatalog.worldbank.org/public-licenses). A stored copy is permitted; the user must give credit.
//! The file is not a vintage file: the Bank can revise old values, so a later copy never replaces an earlier one.
//! Uranium is not in this file. The IMF terms page could not be read (HTTP 403, 2026-11-25), so no IMF series is loaded.

use super::{today_iso, write_atomic};
use crate::futures::mpt::{attr, shared_strings, sheet_path, zip_entry};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa commodity - monthly commodity prices (World Bank Commodity Price Data, the \"Pink Sheet\")

USAGE:
  fa commodity get gold [--cache DIR]     download CMO-Historical-Data-Monthly.xlsx into DIR/commodity/worldbank/
                                          CMO-Historical-Data-Monthly_<date>.xlsx (+ .meta.json). Each dated copy is kept.
  fa commodity show gold [--from YYYY-MM] [--cache DIR] [--file F]
                                          one row for each month: month, price in US dollars per troy ounce.
                                          Default file: the newest copy of DIR/commodity/worldbank. Default --cache: data/raw.

gold:    monthly average of daily spot rates (see the note that `show` prints: the source changed in June 2025).
uranium: not loaded. The IMF terms page could not be read (docs/DATA-SOURCES.md); the World Bank file has no uranium.
Credit: World Bank Commodity Price Data (The Pink Sheet), CC BY 4.0.";

pub const PAGE: &str = "https://www.worldbank.org/en/research/commodity-markets";
/// The pause between two requests to the host, in seconds.
pub const PAUSE: f64 = 3.0;
pub const SOURCE: &str = "World Bank, Commodity Price Data (The Pink Sheet), monthly prices";
pub const LICENCE: &str = "CC BY 4.0 (https://datacatalog.worldbank.org/public-licenses): credit the World Bank, say if changed";
const FILE_STEM: &str = "CMO-Historical-Data-Monthly";
const SHEET: &str = "Monthly Prices";
/// The sentence of the Description sheet that the gold column has today (2026-11-25). A change of it is a break of the series.
pub const GOLD_DEFINITION_START: &str = "Gold, spot average of daily rates, from June 2025; previously (UK), 99.5% fine, London afternoon fixing";

/// A series of the file: the command name, the column title, the unit text, the start of its Description line.
#[derive(Debug)]
pub struct Series {
    pub name: &'static str,
    pub column: &'static str,
    pub unit: &'static str,
    pub description: &'static str,
}

pub const SERIES: [Series; 1] = [Series { name: "gold", column: "Gold", unit: "($/troy oz)", description: GOLD_DEFINITION_START }];

pub fn series_dir(cache: &Path) -> PathBuf {
    cache.join("commodity").join("worldbank")
}

/// The address of the monthly file in the text of the commodity page. The link has a hash that changes with each
/// monthly update, so the program reads it from the page each time.
pub fn file_link(page: &str) -> Option<String> {
    let i = page.find(&format!("{FILE_STEM}.xlsx"))?;
    let start = page[..i].rfind("href=\"")? + 6;
    let end = i + page[i..].find('"')?;
    let href = page[start..end].replace("&amp;", "&");
    Some(if href.starts_with("http") { href } else { format!("https://www.worldbank.org{href}") })
}

/// The cells of one `<row>`: (column letters, type, text). Unlike the Atlanta Fed reader, columns can have two letters (`BR`).
fn row_cells(row: &str) -> Vec<(String, &str, &str)> {
    let mut out = Vec::new();
    let mut rest = row;
    while let Some(i) = rest.find("<c ") {
        let Some(e) = rest[i..].find('>') else { break };
        let tag = &rest[i..i + e];
        let column: String = attr(tag, "r").unwrap_or("").chars().take_while(char::is_ascii_alphabetic).collect();
        let kind = attr(tag, "t").unwrap_or("n");
        if tag.ends_with('/') {
            rest = &rest[i + e + 1..];
            continue;
        }
        let body_start = i + e + 1;
        let close = rest[body_start..].find("</c>").map_or(rest.len(), |c| body_start + c);
        let body = &rest[body_start..close];
        let value = body.find("<v>").and_then(|a| body[a + 3..].find("</v>").map(|b| &body[a + 3..a + 3 + b])).unwrap_or("");
        out.push((column, kind, value));
        rest = &rest[close..];
        rest = rest.strip_prefix("</c>").unwrap_or(rest);
    }
    out
}

/// What the program reads from one copy of the file.
#[derive(Debug, PartialEq)]
pub struct Parsed {
    /// "YYYY-MM" to the price.
    pub months: BTreeMap<String, f64>,
    /// The text of cell A4, for example "Updated on November 03, 2026".
    pub updated: String,
    /// The line of the Description sheet that starts with the series name, if the sheet has one.
    pub description: Option<String>,
    /// Months that have a cell but no number (the file writes "…").
    pub missing: usize,
}

fn text_of(strings: &[String], kind: &str, v: &str) -> Option<String> {
    if kind == "s" { v.parse::<usize>().ok().and_then(|n| strings.get(n)).cloned() } else { None }
}

/// "1960M01" to "1960-01"; `None` if the text has another form.
pub fn period_to_month(p: &str) -> Option<String> {
    let (y, m) = p.split_once('M')?;
    let (yn, mn) = (y.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (y.len() == 4 && m.len() == 2 && (1..=12).contains(&mn)).then(|| format!("{yn:04}-{mn:02}"))
}

/// Read one series. The function checks the form: the header row has the column title, and the row below it has the unit.
pub fn read_series(xlsx: &[u8], s: &Series) -> Result<Parsed, String> {
    let strings = shared_strings(&String::from_utf8(zip_entry(xlsx, "xl/sharedStrings.xml")?).map_err(|_| "sharedStrings.xml is not UTF-8")?)?;
    let path = sheet_path(xlsx, SHEET)?;
    let xml = String::from_utf8(zip_entry(xlsx, &path)?).map_err(|_| format!("{path} is not UTF-8"))?;
    let data = xml.find("<sheetData>").ok_or("the sheet has no sheetData")? + 11;
    let mut col: Option<String> = None;
    let mut unit_ok = false;
    let mut updated = String::new();
    let mut months = BTreeMap::new();
    let mut missing = 0;
    for row in xml[data..].split("<row ").skip(1) {
        let c = row_cells(row.split("</row>").next().unwrap_or(""));
        let first = c.iter().find(|x| x.0 == "A").and_then(|x| text_of(&strings, x.1, x.2));
        if col.is_none() {
            if let Some(t) = first.as_deref().filter(|t| t.starts_with("Updated on")) {
                updated = t.to_string();
            }
            col = c.iter().find(|x| text_of(&strings, x.1, x.2).as_deref() == Some(s.column)).map(|x| x.0.clone());
            continue;
        }
        let column = col.as_deref().unwrap_or("");
        if !unit_ok {
            let unit = c.iter().find(|x| x.0 == column).and_then(|x| text_of(&strings, x.1, x.2));
            if unit.as_deref() != Some(s.unit) {
                return Err(format!("the column {} has the unit {unit:?} below the title, not {}: the form of the file changed", s.column, s.unit));
            }
            unit_ok = true;
            continue;
        }
        let Some(month) = first.as_deref().and_then(period_to_month) else { continue };
        match c.iter().find(|x| x.0 == column) {
            Some(x) if x.1 == "n" => match x.2.parse::<f64>() {
                Ok(v) => {
                    months.insert(month, v);
                }
                Err(_) => missing += 1,
            },
            Some(_) => missing += 1,
            None => {}
        }
    }
    if col.is_none() {
        return Err(format!("the sheet {SHEET} has no column {}: the form of the file changed", s.column));
    }
    if months.is_empty() {
        return Err(format!("the column {} has no month rows", s.column));
    }
    Ok(Parsed { months, updated, description: description_of(xlsx, &strings, s.description)?, missing })
}

/// The line of the Description sheet that starts with `start` (the sheet may be missing in a changed file: `None`).
fn description_of(xlsx: &[u8], strings: &[String], start: &str) -> Result<Option<String>, String> {
    let Ok(path) = sheet_path(xlsx, "Description") else { return Ok(None) };
    let xml = String::from_utf8(zip_entry(xlsx, &path)?).map_err(|_| format!("{path} is not UTF-8"))?;
    let head = start.split(',').next().unwrap_or(start);
    for row in xml.split("<row ").skip(1) {
        for (_, k, v) in row_cells(row.split("</row>").next().unwrap_or("")) {
            if let Some(t) = text_of(strings, k, v) {
                if t.starts_with(&format!("{head},")) {
                    return Ok(Some(t));
                }
            }
        }
    }
    Ok(None)
}

pub fn newest(dir: &Path) -> Option<PathBuf> {
    super::spf::newest(dir, &format!("{FILE_STEM}_"), ".xlsx")
}

pub fn format_rows(p: &Parsed, from: Option<&str>) -> String {
    let mut s = format!("{:<8} {:>10}\n", "month", "USD/oz");
    for (m, v) in p.months.iter().filter(|(m, _)| from.is_none_or(|f| m.as_str() >= f)) {
        s += &format!("{m:<8} {v:>10.2}\n");
    }
    s
}

fn find_series(name: Option<&str>) -> Result<&'static Series, String> {
    let n = name.map(str::to_lowercase).ok_or("name a series: gold (see fa commodity --help)")?;
    if n == "uranium" {
        return Err("uranium is not loaded: the IMF terms page could not be read (HTTP 403, 2026-11-25) and the World Bank file has no uranium; see docs/DATA-SOURCES.md".into());
    }
    SERIES.iter().find(|s| s.name == n).ok_or_else(|| format!("unknown series '{n}'; see fa commodity --help"))
}

fn get(repo: &Path, args: &[String]) -> Result<i32, String> {
    let p = super::price_cli::parse(args, &["--cache"], &[])?;
    let series = find_series(p.tickers.first().map(String::as_str))?;
    let cache = p.flag("--cache").map(PathBuf::from).unwrap_or_else(|| super::price_cli::cache_dir(repo));
    let dir = series_dir(&cache);
    let day = today_iso();
    // Request 1: the commodity page (the link of the file has a hash). Request 2: the file.
    let page = crate::hostnet::get(PAGE, &[], PAUSE)?;
    let link = file_link(&String::from_utf8_lossy(&page)).ok_or("the commodity page has no link to CMO-Historical-Data-Monthly.xlsx")?;
    let body = crate::hostnet::get(&link, &[], PAUSE)?;
    if !body.starts_with(b"PK") {
        return Err(format!("{FILE_STEM}.xlsx: the answer is not an xlsx file (no zip header)"));
    }
    // Check the form before the file is kept: a changed file must not enter the cache without a word.
    let parsed = read_series(&body, series)?;
    let (first, last) = (parsed.months.keys().next().cloned().unwrap_or_default(), parsed.months.keys().next_back().cloned().unwrap_or_default());
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = format!("{FILE_STEM}_{day}.xlsx");
    let file = dir.join(&name);
    std::fs::write(&file, &body).map_err(|e| format!("{}: {e}", file.display()))?;
    let sha = crate::portfolio::harness::sha256_hex(&body);
    let meta = serde_json::json!({
        "source": SOURCE, "address": link, "page": PAGE, "date": day, "bytes": body.len(), "sha256": sha,
        "licence": LICENCE, "file_says": parsed.updated,
        "series": series.name, "months": parsed.months.len(), "first": first, "last": last,
        "definition": parsed.description,
        "note": "private cache; not a vintage file: a later download can revise old values",
    });
    write_atomic(&dir.join(format!("{FILE_STEM}_{day}.meta.json")), &(serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())? + "\n"))?;
    println!("saved {name} ({} bytes, sha256 {sha}); {}: {} months, {first} to {last}; {}", body.len(), series.name, parsed.months.len(), parsed.updated);
    println!("requests: 2");
    Ok(0)
}

fn show(repo: &Path, args: &[String]) -> Result<i32, String> {
    let p = super::price_cli::parse(args, &["--cache", "--file", "--from"], &[])?;
    let series = find_series(p.tickers.first().map(String::as_str))?;
    let cache = p.flag("--cache").map(PathBuf::from).unwrap_or_else(|| super::price_cli::cache_dir(repo));
    let file = match p.flag("--file") {
        Some(f) => PathBuf::from(f),
        None => newest(&series_dir(&cache)).ok_or_else(|| format!("no {FILE_STEM}_*.xlsx in {}: run fa commodity get {}", series_dir(&cache).display(), series.name))?,
    };
    let from = p.flag("--from").map(|f| period_to_month(&f.replacen('-', "M", 1)).ok_or_else(|| format!("--from {f}: write YYYY-MM"))).transpose()?;
    let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let sha = crate::portfolio::harness::sha256_hex(&bytes);
    let meta_path = file.with_extension("meta.json");
    match std::fs::read_to_string(&meta_path).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
        Some(m) if m["sha256"].as_str() == Some(sha.as_str()) => {}
        Some(_) => eprintln!("fa: WARNING the SHA-256 of {} differs from its record: the file changed after the download", file.display()),
        None => eprintln!("fa: no record file {}: the origin of the copy is not known", meta_path.display()),
    }
    let parsed = read_series(&bytes, series)?;
    eprintln!("fa: {} ({} months; {}; {} without a number)", file.file_name().and_then(|n| n.to_str()).unwrap_or(""), parsed.months.len(), parsed.updated, parsed.missing);
    match parsed.description.as_deref() {
        Some(d) if d.starts_with(series.description) => eprintln!("fa: definition: {d}"),
        Some(d) => eprintln!("fa: WARNING the definition changed (a new break of the series?): {d}"),
        None => eprintln!("fa: WARNING no Description line for {}: check the definition", series.column),
    }
    eprintln!("fa: credit: World Bank Commodity Price Data (The Pink Sheet), CC BY 4.0. Months are monthly averages of daily rates, nominal US dollars.");
    print!("{}", format_rows(&parsed, from.as_deref()));
    Ok(0)
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(0)
        }
        Some("get") => get(repo, &args[1..]),
        Some("show") => show(repo, &args[1..]),
        Some(other) => Err(format!("unknown commodity command '{other}'; see fa commodity --help")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zip file with stored entries (the reader does not check the CRC).
    fn zip(files: &[(&str, String)]) -> Vec<u8> {
        let (mut out, mut dir) = (Vec::new(), Vec::new());
        for (name, data) in files {
            let at = out.len() as u32;
            let n = name.len() as u16;
            let size = data.len() as u32;
            out.extend([0x50, 0x4b, 3, 4, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend(size.to_le_bytes().iter().chain(size.to_le_bytes().iter()));
            out.extend(n.to_le_bytes());
            out.extend([0, 0]);
            out.extend(name.bytes());
            out.extend(data.bytes());
            dir.extend([0x50, 0x4b, 1, 2, 20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            dir.extend(size.to_le_bytes().iter().chain(size.to_le_bytes().iter()));
            dir.extend(n.to_le_bytes());
            dir.extend([0u8; 12]);
            dir.extend(at.to_le_bytes());
            dir.extend(name.bytes());
        }
        let start = out.len() as u32;
        let count = files.len() as u16;
        let dir_len = dir.len() as u32;
        out.extend(dir);
        out.extend([0x50, 0x4b, 5, 6, 0, 0, 0, 0]);
        out.extend(count.to_le_bytes().iter().chain(count.to_le_bytes().iter()));
        out.extend(dir_len.to_le_bytes());
        out.extend(start.to_le_bytes());
        out.extend([0, 0]);
        out
    }

    /// The layout of CMO-Historical-Data-Monthly.xlsx (downloaded 2026-11-25), trimmed to the cells that the reader uses:
    /// a hidden first sheet, "Monthly Prices" with the title in row 5 and the unit in row 6, the month in column A, Gold in
    /// column BR, the text "…" for no value. The months and the gold values are from the real file.
    /// `unit` and `description` are parameters so that a test can break the form.
    fn fixture(unit: &str, description: &str) -> Vec<u8> {
        let strings = ["Updated on November 03, 2026", "Gold", unit, "…", "1960M01", "1960M02", "1979M12", "2025M05", "2025M06", "2026M10", "Silver", "($/troy oz)", description];
        let sst: String = strings.iter().map(|s| format!("<si><t>{s}</t></si>")).collect();
        let sst = format!("<sst>{sst}</sst>");
        let s = |r: u32, c: &str, i: u32| format!("<c r=\"{c}{r}\" t=\"s\"><v>{i}</v></c>");
        let n = |r: u32, c: &str, v: &str| format!("<c r=\"{c}{r}\" s=\"83\"><v>{v}</v></c>");
        let mut rows = vec![format!("<row r=\"4\">{}</row>", s(4, "A", 0)), format!("<row r=\"5\"><c r=\"A5\" s=\"2\"/>{}{}</row>", s(5, "BR", 1), s(5, "BT", 10)), format!("<row r=\"6\">{}{}</row>", s(6, "BR", 2), s(6, "BT", 11))];
        // (month string index, gold cell, silver cell)
        let data: [(u32, Option<&str>, &str); 6] = [(4, Some("35"), "0.9"), (5, None, "0.9"), (6, Some("455"), "20"), (7, Some("3309"), "32"), (8, Some("3353"), "33"), (9, Some("4105"), "50")];
        for (i, (m, g, sv)) in data.iter().enumerate() {
            let r = 7 + i as u32;
            let gold = match g {
                Some(v) => n(r, "BR", v),
                None => s(r, "BR", 3),
            };
            rows.push(format!("<row r=\"{r}\">{}{gold}{}</row>", s(r, "A", *m), n(r, "BT", sv)));
        }
        let prices = format!("<worksheet><sheetData>{}</sheetData></worksheet>", rows.join(""));
        let desc = format!("<worksheet><sheetData><row r=\"2\">{}</row><row r=\"9\">{}</row></sheetData></worksheet>", s(2, "B", 10), s(9, "B", 12));
        let hidden = "<worksheet><sheetData><row r=\"1\"><c r=\"A1\" t=\"s\"><v>3</v></c></row></sheetData></worksheet>".to_string();
        let wb = "<workbook><sheets><sheet name=\"Mismatch Details\" sheetId=\"22\" state=\"hidden\" r:id=\"rId1\"/><sheet name=\"Monthly Prices\" sheetId=\"24\" r:id=\"rId2\"/><sheet name=\"Description\" sheetId=\"29\" r:id=\"rId3\"/></sheets></workbook>".to_string();
        let rels = "<Relationships><Relationship Id=\"rId1\" Type=\"x\" Target=\"worksheets/sheet1.xml\"/><Relationship Id=\"rId2\" Type=\"x\" Target=\"worksheets/sheet2.xml\"/><Relationship Id=\"rId3\" Type=\"x\" Target=\"worksheets/sheet3.xml\"/></Relationships>".to_string();
        zip(&[("xl/sharedStrings.xml", sst), ("xl/worksheets/sheet1.xml", hidden), ("xl/worksheets/sheet2.xml", prices), ("xl/worksheets/sheet3.xml", desc), ("xl/workbook.xml", wb), ("xl/_rels/workbook.xml.rels", rels)])
    }

    const DEF: &str = "Gold, spot average of daily rates, from June 2025; previously (UK), 99.5% fine, London afternoon fixing, average of daily rates";

    #[test]
    fn gold_column_two_letters_and_missing_cell() {
        let p = read_series(&fixture("($/troy oz)", DEF), &SERIES[0]).unwrap();
        // 1960M02 has "…": no entry, one missing.
        assert_eq!(p.months.len(), 5);
        assert_eq!(p.missing, 1);
        assert_eq!(p.months["1960-01"], 35.0);
        assert_eq!(p.months["1979-12"], 455.0);
        assert_eq!(p.months["2025-05"], 3309.0);
        assert_eq!(p.months["2025-06"], 3353.0);
        assert_eq!(p.months["2026-10"], 4105.0);
        assert_eq!(p.updated, "Updated on November 03, 2026");
        assert_eq!(p.description.as_deref(), Some(DEF));
    }

    #[test]
    fn rows_from_a_month() {
        let p = read_series(&fixture("($/troy oz)", DEF), &SERIES[0]).unwrap();
        let all = format_rows(&p, None);
        assert!(all.starts_with("month        USD/oz\n1960-01       35.00\n"));
        let late = format_rows(&p, Some("2025-06"));
        assert_eq!(late, "month        USD/oz\n2025-06     3353.00\n2026-10     4105.00\n");
    }

    #[test]
    fn a_changed_unit_is_an_error() {
        let e = read_series(&fixture("($/kg)", DEF), &SERIES[0]).unwrap_err();
        assert!(e.contains("form of the file changed"), "{e}");
    }

    #[test]
    fn a_missing_column_is_an_error() {
        let s = Series { name: "x", column: "Uranium", unit: "($/lb)", description: "Uranium," };
        assert!(read_series(&fixture("($/troy oz)", DEF), &s).unwrap_err().contains("no column Uranium"));
    }

    #[test]
    fn periods() {
        assert_eq!(period_to_month("1960M01").as_deref(), Some("1960-01"));
        assert_eq!(period_to_month("2026M10").as_deref(), Some("2026-10"));
        assert_eq!(period_to_month("2026M13"), None);
        assert_eq!(period_to_month("2026"), None);
        assert_eq!(period_to_month("Gold"), None);
    }

    #[test]
    fn link_has_a_hash_and_the_host_is_known() {
        let page = "<a href=\"https://thedocs.worldbank.org/en/doc/74e8be41ceb20fa0da750cda2f6b9e4e-0050012026/related/CMO-Historical-Data-Monthly.xlsx\">Monthly prices</a>";
        assert_eq!(
            file_link(page).unwrap(),
            "https://thedocs.worldbank.org/en/doc/74e8be41ceb20fa0da750cda2f6b9e4e-0050012026/related/CMO-Historical-Data-Monthly.xlsx"
        );
        assert!(file_link("<p>none</p>").is_none());
        assert_eq!(crate::hostnet::group_of_url("https://thedocs.worldbank.org/en/doc/x"), "worldbank");
        assert_eq!(crate::hostnet::group_of_url(PAGE), "worldbank");
    }

    #[test]
    fn uranium_and_unknown_series_are_refused() {
        assert!(find_series(Some("URANIUM")).unwrap_err().contains("not loaded"));
        assert!(find_series(Some("silver")).unwrap_err().contains("unknown series"));
        assert!(find_series(None).is_err());
        assert_eq!(find_series(Some("Gold")).unwrap().column, "Gold");
    }
}
