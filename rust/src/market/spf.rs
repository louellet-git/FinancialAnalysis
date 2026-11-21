//! Survey of Professional Forecasters (Federal Reserve Bank of Philadelphia): the 10-year median forecasts.
//!
//! `fa spf get` downloads `medianLevel.xlsx` (one sheet for each variable, columns YEAR, QUARTER, value) and the list of
//! release dates, and keeps each copy with the download date. `fa spf show` prints the long-run medians of the Q1 surveys.
//! The file is not a vintage file: the Bank can revise old values, so a later copy never replaces an earlier one.
//! Terms: the site has no terms page (footer "All rights reserved"). Owner decision, 2026-11-21: private copy only
//! (the cache is not in the repo); results may go into the repo.

use super::{today_iso, write_atomic};
use crate::futures::mpt::{cells, shared_strings, sheet_path, zip_entry};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa spf - Survey of Professional Forecasters, 10-year medians (Philadelphia Fed)

USAGE:
  fa spf get [--cache DIR]               download medianLevel.xlsx into DIR/macro/spf/medianLevel_<date>.xlsx (+ .meta.json) and try
                                         the release-dates list (spf-release-dates_<date>.txt). Each dated copy is kept.
  fa spf show [--cache DIR] [--file F]   one row for each Q1 survey: year, RGDP10, CPI10, STOCK10, BOND10, BILL10, release date.
                                         Default file: the newest copy of DIR/macro/spf. Default --cache: data/raw.

A release date marked 'bound' is an upper bound from a rule, not a published date.";

pub const URL: &str = "https://www.philadelphiafed.org/-/media/FRBP/Assets/Surveys-And-Data/survey-of-professional-forecasters/historical-data/medianLevel.xlsx";
pub const PAGE: &str = "https://www.philadelphiafed.org/surveys-and-data/real-time-data-research/survey-of-professional-forecasters";
pub const HOST: &str = "https://www.philadelphiafed.org";
/// The pause between two requests to the host, in seconds.
pub const PAUSE: f64 = 3.0;
/// The sheets of the 10-year medians, in output order.
pub const SHEETS: [&str; 5] = ["RGDP10", "CPI10", "STOCK10", "BOND10", "BILL10"];

fn spf_dir(cache: &Path) -> PathBuf {
    cache.join("macro").join("spf")
}

/// The address of the release-dates file in the text of the SPF page (the link has a hash; `&amp;` is `&`).
pub fn release_link(page: &str) -> Option<String> {
    let i = page.find("spf-release-dates.txt")?;
    let start = page[..i].rfind("href=\"")? + 6;
    let end = i + page[i..].find('"')?;
    let href = page[start..end].replace("&amp;", "&");
    Some(if href.starts_with("http") { href } else { format!("{HOST}{href}") })
}

/// Is the answer a list of dates (text), not an HTML page?
pub fn looks_like_dates(body: &[u8]) -> bool {
    let t = String::from_utf8_lossy(body);
    let head = t.trim_start();
    !head.starts_with('<') && head.chars().any(|c| c.is_ascii_digit()) && !t.to_lowercase().contains("<html")
}

fn get(repo: &Path, args: &[String]) -> Result<i32, String> {
    let p = super::price_cli::parse(args, &["--cache"], &[])?;
    let cache = p.flag("--cache").map(PathBuf::from).unwrap_or_else(|| super::price_cli::cache_dir(repo));
    let dir = spf_dir(&cache);
    let day = today_iso();
    let mut requests = 0;
    // Request 1: the workbook. Never replace an earlier copy; a second run on the same day replaces that day's copy only.
    let body = crate::hostnet::get(URL, &[], PAUSE)?;
    requests += 1;
    if !body.starts_with(b"PK") {
        return Err("medianLevel.xlsx: the answer is not an xlsx file (no zip header)".into());
    }
    let name = format!("medianLevel_{day}.xlsx");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join(&name);
    std::fs::write(&file, &body).map_err(|e| format!("{}: {e}", file.display()))?;
    let sha = crate::portfolio::harness::sha256_hex(&body);
    let meta = serde_json::json!({
        "source": "Federal Reserve Bank of Philadelphia, Survey of Professional Forecasters, median forecasts (levels)",
        "address": URL, "date": day, "bytes": body.len(), "sha256": sha,
        "note": "private copy; not a vintage file: a later download can revise old values",
    });
    write_atomic(&dir.join(format!("medianLevel_{day}.meta.json")), &(serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())? + "\n"))?;
    println!("saved {} ({} bytes, sha256 {sha})", name, body.len());
    // Requests 2 and 3: the SPF page, then the release-dates file that it links (at most 2 more requests).
    let dates = (|| -> Result<(String, Vec<u8>), String> {
        let page = crate::hostnet::get(PAGE, &[], PAUSE)?;
        requests += 1;
        let link = release_link(&String::from_utf8_lossy(&page)).ok_or("the SPF page has no link to spf-release-dates.txt")?;
        let text = crate::hostnet::get(&link, &[], PAUSE)?;
        requests += 1;
        if !looks_like_dates(&text) {
            return Err(format!("the release-dates answer is not a text list ({} bytes, starts with '<'?)", text.len()));
        }
        Ok((link, text))
    })();
    match dates {
        Ok((link, text)) => {
            let f = dir.join(format!("spf-release-dates_{day}.txt"));
            std::fs::write(&f, &text).map_err(|e| format!("{}: {e}", f.display()))?;
            let m = serde_json::json!({"address": link.split('?').next().unwrap_or(&link), "date": day, "bytes": text.len(), "sha256": crate::portfolio::harness::sha256_hex(&text)});
            write_atomic(&dir.join(format!("spf-release-dates_{day}.meta.json")), &(m.to_string() + "\n"))?;
            println!("saved spf-release-dates_{day}.txt ({} bytes)", text.len());
        }
        Err(e) => println!("release dates: not saved ({})", crate::hostnet::plain(&e)),
    }
    println!("requests: {requests}");
    Ok(0)
}

/// One sheet as (year, quarter) to value. A cell that is not a number (`#N/A`) gives no entry.
pub fn read_sheet(xlsx: &[u8], sheet: &str) -> Result<BTreeMap<(i32, u32), f64>, String> {
    let strings = shared_strings(&String::from_utf8(zip_entry(xlsx, "xl/sharedStrings.xml")?).map_err(|_| "sharedStrings.xml is not UTF-8")?)?;
    let path = sheet_path(xlsx, sheet)?;
    let xml = String::from_utf8(zip_entry(xlsx, &path)?).map_err(|_| format!("{path} is not UTF-8"))?;
    let data = xml.find("<sheetData>").ok_or("the sheet has no sheetData")? + 11;
    let mut out = BTreeMap::new();
    let mut header = false;
    for row in xml[data..].split("<row ").skip(1) {
        let c = cells(row.split("</row>").next().unwrap_or(""));
        if c.is_empty() {
            continue;
        }
        if !header {
            let name = |i: usize| c.get(i).and_then(|&(_, k, v)| if k == "s" { v.parse::<usize>().ok().and_then(|n| strings.get(n)) } else { None });
            if name(0).map(String::as_str) != Some("YEAR") || name(1).map(String::as_str) != Some("QUARTER") || name(2).is_none() {
                return Err(format!("sheet {sheet}: the header is not YEAR, QUARTER, value"));
            }
            header = true;
            continue;
        }
        if c.len() < 3 {
            continue;
        }
        let num = |i: usize| if c[i].1 == "n" { c[i].2.parse::<f64>().ok() } else { None };
        let (Some(y), Some(q)) = (num(0), num(1)) else { continue };
        if let Some(v) = num(2) {
            out.insert((y as i32, q as u32), v);
        }
    }
    if !header {
        return Err(format!("sheet {sheet}: no header row"));
    }
    Ok(out)
}

/// The news release dates of the release-dates file, (year, quarter) to "YYYY-MM-DD". Lines look like
/// `1992 Q1   2/22/92   2/26/92` (deadline, release) or `     Q2   5/15/92   5/22/92`; a star after a date is a note mark.
pub fn parse_release_dates(text: &str) -> BTreeMap<(i32, u32), String> {
    let mut out = BTreeMap::new();
    let mut year = 0;
    for line in text.lines() {
        let mut t: Vec<&str> = line.split_whitespace().collect();
        if t.first().is_some_and(|w| w.len() == 4 && w.chars().all(|c| c.is_ascii_digit())) {
            year = t[0].parse().unwrap_or(0);
            t.remove(0);
        }
        let Some(q) = t.first().and_then(|w| w.strip_prefix('Q')).and_then(|n| n.parse::<u32>().ok()) else { continue };
        if t.len() < 3 || year == 0 {
            continue;
        }
        if let Some(d) = us_date(t[2]) {
            out.insert((year, q), d);
        }
    }
    out
}

/// "3/6/26**" to "2026-03-06" (two-digit year: 50 and more is 19xx).
fn us_date(text: &str) -> Option<String> {
    let t = text.trim_end_matches('*');
    let mut p = t.split('/').map(|x| x.parse::<i64>().ok());
    let (m, d, y) = (p.next()??, p.next()??, p.next()??);
    let y = if y < 100 { if y >= 50 { 1900 + y } else { 2000 + y } } else { y };
    let iso = format!("{y:04}-{m:02}-{d:02}");
    (mapcheck::dates::parse(&iso).map(mapcheck::dates::format).as_deref() == Some(iso.as_str())).then_some(iso)
}

/// The upper bound of the release date of the Q1 survey of `year`, where the file has no date.
/// Evidence, not a formula: the Bank's release-dates file (downloaded 2026-11-21) gives all 35 Q1 releases from 1992 to
/// 2026; the latest is 2019-03-22 (after a shutdown). 31 March is later than each of them. It is a bound, not a date:
/// a later delay can pass it, so a row marked "bound" needs a check by hand before a point-in-time use.
pub fn q1_release_bound(year: i32) -> String {
    format!("{year:04}-03-31")
}

fn newest(dir: &Path, prefix: &str, ext: &str) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(prefix) && n.ends_with(ext) && !n.contains(".meta.")))
        .collect();
    v.sort();
    v.pop()
}

/// The table rows: (year, values in `SHEETS` order, release date, is a bound).
pub type Row = (i32, [Option<f64>; 5], String, bool);

pub fn q1_rows(xlsx: &[u8], release_text: Option<&str>) -> Result<Vec<Row>, String> {
    let sheets: Vec<_> = SHEETS.iter().map(|s| read_sheet(xlsx, s)).collect::<Result<_, _>>()?;
    let dates = release_text.map(parse_release_dates).unwrap_or_default();
    let years: std::collections::BTreeSet<i32> = sheets.iter().flat_map(|m| m.keys().filter(|k| k.1 == 1).map(|k| k.0)).collect();
    Ok(years
        .into_iter()
        .map(|y| {
            let v = std::array::from_fn(|i| sheets[i].get(&(y, 1)).copied());
            match dates.get(&(y, 1)) {
                Some(d) => (y, v, d.clone(), false),
                None => (y, v, q1_release_bound(y), true),
            }
        })
        .collect())
}

pub fn format_rows(rows: &[Row]) -> String {
    let mut s = format!("{:<5}", "year");
    for n in SHEETS {
        s += &format!(" {n:>8}");
    }
    s += "  release\n";
    for (y, v, d, bound) in rows {
        s += &format!("{y:<5}");
        for x in v {
            s += &x.map_or(format!(" {:>8}", "-"), |x| format!(" {x:>8.3}"));
        }
        s += &format!("  {d}{}\n", if *bound { " bound" } else { "" });
    }
    s
}

fn show(repo: &Path, args: &[String]) -> Result<i32, String> {
    let p = super::price_cli::parse(args, &["--cache", "--file"], &[])?;
    let cache = p.flag("--cache").map(PathBuf::from).unwrap_or_else(|| super::price_cli::cache_dir(repo));
    let file = match p.flag("--file") {
        Some(f) => PathBuf::from(f),
        None => newest(&spf_dir(&cache), "medianLevel_", ".xlsx").ok_or_else(|| format!("no medianLevel_*.xlsx in {}: run fa spf get", spf_dir(&cache).display()))?,
    };
    let dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let dates = newest(&dir, "spf-release-dates_", ".txt").and_then(|f| std::fs::read_to_string(f).ok());
    let rows = q1_rows(&bytes, dates.as_deref())?;
    eprintln!("fa: {} ({} Q1 rows; release dates: {})", file.file_name().and_then(|n| n.to_str()).unwrap_or(""), rows.len(), if dates.is_some() { "file" } else { "none, bounds" });
    print!("{}", format_rows(&rows));
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
        Some(other) => Err(format!("unknown spf command '{other}'; see fa spf --help")),
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

    /// The layout of medianLevel.xlsx (read on 2026-11-21): shared strings 0 YEAR, 1 QUARTER, 2 header, 3 "#N/A"; the number
    /// cells have no type. The values are from the real file (RGDP10 1992 Q1 2.5; CPI10 1991 Q4 4.0, 1992 Q1 3.7, and so on).
    fn fixture() -> Vec<u8> {
        let rows = |name: &str, data: &[(u32, u32, Option<f64>)]| {
            let mut s = String::from("<worksheet><sheetData><row r=\"1\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" t=\"s\"><v>1</v></c><c r=\"C1\" t=\"s\"><v>2</v></c></row>");
            for (i, (y, q, v)) in data.iter().enumerate() {
                let r = i + 2;
                let c = match v {
                    Some(v) => format!("<c r=\"C{r}\" s=\"4\" ><v>{v:.4}</v></c>"),
                    None => format!("<c r=\"C{r}\" s=\"4\" t=\"s\"><v>3</v></c>"),
                };
                s += &format!("<row r=\"{r}\"><c r=\"A{r}\" s=\"3\" ><v>{y}.0000</v></c><c r=\"B{r}\" s=\"3\" ><v>{q}.0000</v></c>{c}</row>");
            }
            let _ = name;
            s + "<row r=\"99\" ht=\"13\"/></sheetData></worksheet>"
        };
        let data = [
            ("RGDP10", [(1991, 4, None), (1992, 1, Some(2.5)), (1992, 2, None), (1993, 1, Some(2.7)), (2026, 1, Some(2.1))]),
            ("CPI10", [(1991, 4, Some(4.0)), (1992, 1, Some(3.7)), (1992, 2, Some(3.6)), (1993, 1, Some(3.5)), (2026, 1, Some(2.3))]),
            ("STOCK10", [(1991, 4, None), (1992, 1, Some(10.0)), (1992, 2, None), (1993, 1, Some(9.0)), (2026, 1, Some(7.0))]),
            ("BOND10", [(1991, 4, None), (1992, 1, Some(7.5)), (1992, 2, None), (1993, 1, Some(7.0)), (2026, 1, Some(4.0))]),
            ("BILL10", [(1991, 4, None), (1992, 1, Some(5.0)), (1992, 2, None), (1993, 1, Some(4.1)), (2026, 1, Some(3.0))]),
        ];
        let mut wb = String::from("<workbook><sheets>");
        let mut rels = String::from("<Relationships>");
        let mut files = vec![("xl/sharedStrings.xml", "<sst><si><t>YEAR</t></si><si><t>QUARTER</t></si><si><t>VALUE</t></si><si><t>#N/A</t></si></sst>".to_string())];
        let sheet_names: Vec<String> = data.iter().map(|d| format!("xl/worksheets/sheet{}.xml", d.0)).collect();
        for (i, (name, d)) in data.iter().enumerate() {
            wb += &format!("<sheet name=\"{name}\" sheetId=\"{0}\" r:id=\"rId{0}\"/>", i + 1);
            rels += &format!("<Relationship Id=\"rId{0}\" Type=\"x\" Target=\"worksheets/sheet{name}.xml\"/>", i + 1);
            files.push((sheet_names[i].as_str(), rows(name, d)));
        }
        files.push(("xl/workbook.xml", wb + "</sheets></workbook>"));
        files.push(("xl/_rels/workbook.xml.rels", rels + "</Relationships>"));
        zip(&files)
    }

    /// Lines of the real release-dates file (2026-11-21), including a star and the CRLF line ends.
    const DATES: &str = "Survey         True Deadline Date       News Release Date\r\n\r\n1992 Q1             2/22/92             2/26/92\r\n     Q2             5/15/92             5/22/92\r\n\r\n1996 Q1             3/2/96**            3/11/96**\r\n\r\n2026 Q1             3/2/26**            3/6/26** \r\n     Q2             5/12/26             5/15/26\r\n";

    #[test]
    fn release_dates_parse_with_stars_and_two_digit_years() {
        let d = parse_release_dates(DATES);
        assert_eq!(d.len(), 5);
        assert_eq!(d[&(1992, 1)], "1992-02-26");
        assert_eq!(d[&(1992, 2)], "1992-05-22");
        assert_eq!(d[&(1996, 1)], "1996-03-11");
        assert_eq!(d[&(2026, 1)], "2026-03-06");
    }

    #[test]
    fn q1_rows_have_values_and_dates_or_bounds() {
        let rows = q1_rows(&fixture(), Some(DATES)).unwrap();
        // The Q4 and Q2 rows are not in the table; CPI10 1991 Q4 is not a Q1 survey.
        assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), vec![1992, 1993, 2026]);
        assert_eq!(rows[0], (1992, [Some(2.5), Some(3.7), Some(10.0), Some(7.5), Some(5.0)], "1992-02-26".to_string(), false));
        // 1993 has no date in the sample file: the rule gives the bound.
        assert_eq!(rows[1].2, "1993-03-31");
        assert!(rows[1].3);
        assert_eq!(rows[2].1, [Some(2.1), Some(2.3), Some(7.0), Some(4.0), Some(3.0)]);
        let text = format_rows(&rows);
        assert!(text.starts_with("year    RGDP10    CPI10  STOCK10   BOND10   BILL10  release\n"));
        assert!(text.contains("1993     2.700    3.500    9.000    7.000    4.100  1993-03-31 bound\n"));
        assert!(text.contains("2026     2.100    2.300    7.000    4.000    3.000  2026-03-06\n"));
    }

    #[test]
    fn sheet_reader_skips_na_and_checks_the_header() {
        let m = read_sheet(&fixture(), "CPI10").unwrap();
        assert_eq!(m.len(), 5);
        assert_eq!(m.keys().next(), Some(&(1991, 4)));
        assert_eq!(read_sheet(&fixture(), "RGDP10").unwrap().len(), 3);
        assert!(read_sheet(&fixture(), "NOPE").is_err());
    }

    #[test]
    fn link_and_html_shell() {
        let page = "<p><a href=\"/-/media/FRBP/Assets/Surveys-And-Data/survey-of-professional-forecasters/spf-release-dates.txt?sc_lang=en&amp;hash=AB\" target=\"_blank\">Dates</a></p>";
        assert_eq!(
            release_link(page).unwrap(),
            "https://www.philadelphiafed.org/-/media/FRBP/Assets/Surveys-And-Data/survey-of-professional-forecasters/spf-release-dates.txt?sc_lang=en&hash=AB"
        );
        assert!(release_link("<p>none</p>").is_none());
        assert!(looks_like_dates(DATES.as_bytes()));
        assert!(!looks_like_dates(b"<!DOCTYPE html><html><body>1992</body></html>"));
        assert_eq!(crate::hostnet::group_of_url(URL), "philadelphiafed");
    }
}
