//! Treasury HQM corporate bond yield curve, the monthly average spot rates (public data of the
//! U.S. Treasury). Source: the page
//! `https://home.treasury.gov/data/treasury-coupon-issues-and-corporate-bond-yield-curve/corporate-bond-yield-curve`
//! has one Excel 97 file (`.xls`) for each five years, for example
//! `https://home.treasury.gov/system/files/226/hqm_24_28.xls`. The sheet has the maturities (0.5 to
//! 100 years) in the rows and, for each year, 12 month columns. The FRED series `HQMCB<n>YR` have the
//! same numbers (spot rates, not par yields: the par file has 2, 5, 10 and 30 years only).
//!
//! This reader is small on purpose and has no new crate. It reads the number cells of the first
//! worksheet of an Excel 97 file (BIFF8 in a compound file): NUMBER, RK and MULRK records, and text
//! cells from the shared string table. The year header is text; the month is the column place in
//! the year block (the month names are not read).

use crate::rates::{parse_fred_csv, Res};
use mapcheck::dates;
use std::collections::BTreeMap;

const FREESECT: u32 = 0xFFFF_FFFE;

fn u16le(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
fn u32le(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// The bytes of the `Workbook` stream of a compound file.
fn workbook_stream(b: &[u8]) -> Res<Vec<u8>> {
    if b.len() < 512 || b[..8] != [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
        return Err("not an Excel 97 file (no compound file header)".into());
    }
    let shift = u16le(b, 30) as u32;
    if !(7..=12).contains(&shift) {
        return Err("bad sector size in the compound file".into());
    }
    let ss = 1usize << shift;
    let sector = |n: u32| -> Res<&[u8]> {
        let start = (n as usize + 1) * ss;
        b.get(start..start + ss).ok_or_else(|| "compound file: sector out of range".to_string())
    };
    // The sector table (FAT): the first 109 places are in the header, more in DIFAT sectors.
    let mut fat_secs: Vec<u32> = (0..109).map(|i| u32le(b, 76 + 4 * i)).filter(|s| *s < FREESECT).collect();
    let (mut dif, mut left) = (u32le(b, 68), u32le(b, 72));
    while dif < FREESECT && left > 0 {
        let s = sector(dif)?;
        for i in 0..(ss / 4 - 1) {
            let v = u32le(s, 4 * i);
            if v < FREESECT {
                fat_secs.push(v);
            }
        }
        dif = u32le(s, ss - 4);
        left -= 1;
    }
    let mut fat = Vec::new();
    for s in &fat_secs {
        let d = sector(*s)?;
        fat.extend((0..ss / 4).map(|i| u32le(d, 4 * i)));
    }
    let chain = |first: u32| -> Res<Vec<u8>> {
        let (mut out, mut n, mut guard) = (Vec::new(), first, 0usize);
        while n < FREESECT {
            out.extend_from_slice(sector(n)?);
            n = *fat.get(n as usize).ok_or("compound file: bad sector chain")?;
            guard += 1;
            if guard > fat.len() + 1 {
                return Err("compound file: loop in a sector chain".into());
            }
        }
        Ok(out)
    };
    let dir = chain(u32le(b, 48))?;
    let entries: Vec<&[u8]> = dir.chunks_exact(128).collect();
    for e in &entries {
        let len = (u16le(e, 64) as usize).saturating_sub(2) / 2;
        let name: String = (0..len.min(31)).filter_map(|i| char::from_u32(u16le(e, 2 * i) as u32)).collect();
        if (name == "Workbook" || name == "Book") && e[66] == 2 {
            let (first, size) = (u32le(e, 116), u32le(e, 120) as usize);
            let mut data = if size >= u32le(b, 56) as usize {
                chain(first)?
            } else {
                // A short stream is in the mini stream: pieces of 64 bytes in the stream of the root entry.
                let root = entries.iter().find(|r| r[66] == 5).ok_or("compound file: no root entry")?;
                let container = chain(u32le(root, 116))?;
                let mini_fat_bytes = chain(u32le(b, 60))?;
                let mini_fat: Vec<u32> = mini_fat_bytes.chunks_exact(4).map(|c| u32le(c, 0)).collect();
                let (mut out, mut n, mut guard) = (Vec::new(), first, 0usize);
                while n < FREESECT {
                    let start = n as usize * 64;
                    out.extend_from_slice(container.get(start..start + 64).ok_or("compound file: mini sector out of range")?);
                    n = *mini_fat.get(n as usize).ok_or("compound file: bad mini sector chain")?;
                    guard += 1;
                    if guard > mini_fat.len() + 1 {
                        return Err("compound file: loop in a mini sector chain".into());
                    }
                }
                out
            };
            data.truncate(size);
            return Ok(data);
        }
    }
    Err("no Workbook stream in the file".into())
}

fn rk_value(rk: u32) -> f64 {
    let v = if rk & 2 != 0 { f64::from((rk as i32) >> 2) } else { f64::from_bits(((rk & 0xFFFF_FFFC) as u64) << 32) };
    if rk & 1 != 0 {
        v / 100.0
    } else {
        v
    }
}

/// One cell value: a number, or a text (from the shared string table).
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Num(f64),
    Text(String),
}

/// The shared strings (SST record, with its CONTINUE records). A string can cross a record
/// border; then the next record starts with a new flag byte for the rest of the characters.
fn shared_strings(parts: &[&[u8]]) -> Vec<String> {
    let (mut seg, mut pos) = (0usize, 0usize);
    let mut out = Vec::new();
    let rd = |n: usize, seg: &mut usize, pos: &mut usize| -> Option<Vec<u8>> {
        let mut v = Vec::new();
        while v.len() < n {
            let cur = parts.get(*seg)?;
            if *pos >= cur.len() {
                *seg += 1;
                *pos = 0;
                continue;
            }
            let take = (n - v.len()).min(cur.len() - *pos);
            v.extend_from_slice(&cur[*pos..*pos + take]);
            *pos += take;
        }
        Some(v)
    };
    // The first 8 bytes: total and unique counts.
    if rd(8, &mut seg, &mut pos).is_none() {
        return out;
    }
    while let Some(h) = rd(3, &mut seg, &mut pos) {
        let (cch, flags) = (u16::from_le_bytes([h[0], h[1]]) as usize, h[2]);
        let runs = if flags & 8 != 0 { rd(2, &mut seg, &mut pos).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]) as usize) } else { 0 };
        let ext = if flags & 4 != 0 { rd(4, &mut seg, &mut pos).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize) } else { 0 };
        let mut wide = flags & 1 != 0;
        let mut text = String::new();
        let mut left = cch;
        while left > 0 {
            // at a record border the string continues with a new flag byte
            if pos >= parts.get(seg).map_or(0, |p| p.len()) && seg + 1 < parts.len() {
                seg += 1;
                pos = 0;
                match rd(1, &mut seg, &mut pos) {
                    Some(f) => wide = f[0] & 1 != 0,
                    None => return out,
                }
            }
            let avail = parts.get(seg).map_or(0, |p| p.len().saturating_sub(pos));
            let per = if wide { 2 } else { 1 };
            let n = left.min((avail / per).max(1));
            let Some(b) = rd(n * per, &mut seg, &mut pos) else { return out };
            if wide {
                text.extend(b.chunks_exact(2).filter_map(|c| char::from_u32(u16::from_le_bytes([c[0], c[1]]) as u32)));
            } else {
                text.extend(b.iter().map(|c| *c as char));
            }
            left -= n;
        }
        if rd(runs * 4 + ext, &mut seg, &mut pos).is_none() {
            return out;
        }
        out.push(text);
    }
    out
}

/// The cells of the first worksheet: `{(row, column): value}`. Number cells (NUMBER, RK, MULRK)
/// and text cells (LABELSST).
pub fn cells(file: &[u8]) -> Res<BTreeMap<(u32, u16), Cell>> {
    let w = workbook_stream(file)?;
    let (mut i, mut in_sheet, mut done) = (0usize, false, false);
    let mut recs: Vec<(u16, &[u8])> = Vec::new();
    while i + 4 <= w.len() {
        let (id, len) = (u16le(&w, i), u16le(&w, i + 2) as usize);
        let body = w.get(i + 4..i + 4 + len).ok_or("Excel file: record out of range")?;
        recs.push((id, body));
        i += 4 + len;
    }
    let mut sst_parts: Vec<&[u8]> = Vec::new();
    let mut strings: Vec<String> = Vec::new();
    let mut in_sst = false;
    let mut out = BTreeMap::new();
    for (id, body) in recs {
        if done {
            break;
        }
        if in_sst && id != 0x003C {
            strings = shared_strings(&sst_parts);
            in_sst = false;
        }
        let len = body.len();
        match id {
            0x00FC => {
                sst_parts = vec![body];
                in_sst = true;
            }
            0x003C if in_sst => sst_parts.push(body),
            0x0809 if len >= 4 && u16le(body, 2) == 0x0010 => in_sheet = true,
            0x000A if in_sheet => done = true,
            0x0203 if in_sheet && len >= 14 => {
                out.insert((u16le(body, 0) as u32, u16le(body, 2)), Cell::Num(f64::from_le_bytes(body[6..14].try_into().unwrap())));
            }
            0x027E if in_sheet && len >= 10 => {
                out.insert((u16le(body, 0) as u32, u16le(body, 2)), Cell::Num(rk_value(u32le(body, 6))));
            }
            0x00BD if in_sheet && len >= 6 => {
                let (row, first) = (u16le(body, 0) as u32, u16le(body, 2));
                for k in 0..(len - 6) / 6 {
                    out.insert((row, first + k as u16), Cell::Num(rk_value(u32le(body, 4 + 6 * k + 2))));
                }
            }
            0x00FD if in_sheet && len >= 10 => {
                if let Some(t) = strings.get(u32le(body, 6) as usize) {
                    out.insert((u16le(body, 0) as u32, u16le(body, 2)), Cell::Text(t.clone()));
                }
            }
            _ => {}
        }
    }
    if out.is_empty() {
        return Err("no cells in the first worksheet".into());
    }
    Ok(out)
}

/// Monthly spot rates in percent: `{first day of the month: {maturity in years: rate}}` for the
/// wanted whole-year maturities. A month with no value (a future month) is not in the map.
pub fn parse_hqm_spot(file: &[u8], maturities: &[i64]) -> Res<BTreeMap<i64, BTreeMap<i64, f64>>> {
    let cells = cells(file)?;
    let year_of = |c: &Cell| match c {
        Cell::Text(t) => t.trim().parse::<i64>().ok().filter(|y| (1980..2200).contains(y)),
        Cell::Num(v) if v.fract() == 0.0 && (1980.0..2200.0).contains(v) => Some(*v as i64),
        _ => None,
    };
    // The year header cells: one at the start of each block of 12 month columns.
    let years: Vec<(u32, u16, i64)> = cells.iter().filter(|((_, c), _)| *c >= 1).filter_map(|((r, c), v)| year_of(v).map(|y| (*r, *c, y))).collect();
    let year_row = years.iter().map(|y| y.0).min().ok_or("no year header row (the HQM spot layout changed?)")?;
    let years: Vec<(u16, i64)> = years.iter().filter(|y| y.0 == year_row).map(|y| (y.1, y.2)).collect();
    let mut out: BTreeMap<i64, BTreeMap<i64, f64>> = BTreeMap::new();
    for ((r, c), v) in &cells {
        let Cell::Num(v) = v else { continue };
        if *r <= year_row || *c == 0 {
            continue;
        }
        let Some(Cell::Num(m)) = cells.get(&(*r, 0)) else { continue };
        if m.fract() != 0.0 || !maturities.contains(&(*m as i64)) {
            continue;
        }
        let Some((c0, y)) = years.iter().rev().find(|(c0, _)| c0 <= c) else { continue };
        let month = (c - c0) as i64 + 1;
        if month > 12 {
            continue;
        }
        out.entry(dates::days_from_civil(*y, month, 1)).or_default().insert(*m as i64, *v);
    }
    if out.is_empty() {
        return Err("no HQM rates found for the wanted maturities".into());
    }
    Ok(out)
}

/// Result of the comparison of one maturity with a FRED `HQMCB<n>YR` file.
#[derive(Debug, PartialEq)]
pub struct HqmCompare {
    pub maturity: i64,
    pub both: usize,
    pub equal: usize,
    pub off_by_one_hundredth: usize,
    pub larger: usize,
    pub max_diff: f64,
    pub first_ours: Option<String>,
    pub first_fred: Option<String>,
}

pub fn compare_with_fred(months: &BTreeMap<i64, BTreeMap<i64, f64>>, maturity: i64, fred_text: &str) -> Res<HqmCompare> {
    let fred = parse_fred_csv(fred_text, false)?;
    let mut r = HqmCompare { maturity, both: 0, equal: 0, off_by_one_hundredth: 0, larger: 0, max_diff: 0.0, first_ours: None, first_fred: fred.keys().next().map(|d| mapcheck::dates::format(*d)) };
    for (day, m) in months {
        let Some(o) = m.get(&maturity) else { continue };
        if r.first_ours.is_none() {
            r.first_ours = Some(dates::format(*day));
        }
        let Some(f) = fred.get(day) else { continue };
        r.both += 1;
        let diff = ((o - f).abs() * 100.0).round() as i64;
        r.max_diff = r.max_diff.max(diff as f64 / 100.0);
        match diff {
            0 => r.equal += 1,
            1 => r.off_by_one_hundredth += 1,
            _ => r.larger += 1,
        }
    }
    Ok(r)
}

pub const PAGE_URL: &str = "https://home.treasury.gov/data/treasury-coupon-issues-and-corporate-bond-yield-curve/corporate-bond-yield-curve";
pub const FILE_URL: &str = "https://home.treasury.gov/system/files/226/{name}.xls";
/// The monthly spot rate files, one for each five years. The last one is the open file.
pub const FILES: [&str; 9] = ["hqm_84_88", "hqm_89_93", "hqm_94_98", "hqm_99_03", "hqm_04_08", "hqm_09_13", "hqm_14_18", "hqm_19_23", "hqm_24_28"];
/// The open file is downloaded again when the copy is older than this (days).
pub const OPEN_FILE_MAX_AGE_DAYS: f64 = 20.0;
pub const SOURCE: &str = "U.S. Treasury HQM";

/// `<cache>/bonds/treasury_hqm/`: the files and `meta.json`.
pub fn dir(cache: &std::path::Path) -> std::path::PathBuf {
    cache.join("bonds").join("treasury_hqm")
}

fn write_bytes(path: &std::path::Path, bytes: &[u8]) -> Res<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Download the files that are missing, the open file when it is old, and all with `force`. A
/// download must parse as an HQM file before it is kept. `meta.json` records the address, the
/// date of each download and the date of the last check. Gives `(file, what happened)`.
/// A failed file does not stop the others, except a block or a connection failure (the caller's
/// `fetch` returns the error; two errors in a row end the run).
pub fn get(cache: &std::path::Path, force: bool, today: i64, fetch: &mut dyn FnMut(&str) -> Res<Vec<u8>>) -> Res<Vec<(String, String)>> {
    let d = dir(cache);
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    let meta_path = d.join("meta.json");
    let mut meta: serde_json::Value = std::fs::read_to_string(&meta_path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| serde_json::json!({}));
    meta["source"] = SOURCE.into();
    meta["page"] = PAGE_URL.into();
    meta["terms"] = "U.S. federal data; no reuse clause found on the site policy pages (2026-11-21)".into();
    let mut out = Vec::new();
    let mut failures = 0;
    for name in FILES {
        let path = d.join(format!("{name}.xls"));
        let open = name == FILES[FILES.len() - 1];
        let old = open && !crate::rates::is_fresh_file(&path, OPEN_FILE_MAX_AGE_DAYS * 24.0);
        if path.exists() && !force && !old {
            out.push((name.to_string(), "kept".to_string()));
            continue;
        }
        if failures >= 2 {
            out.push((name.to_string(), "not tried (two failures)".to_string()));
            continue;
        }
        let url = FILE_URL.replace("{name}", name);
        let got = fetch(&url).and_then(|b| parse_hqm_spot(&b, &MATURITIES).map(|_| b));
        match got {
            Ok(bytes) => {
                write_bytes(&path, &bytes)?;
                meta["files"][name] = serde_json::json!({"url": url, "fetched": dates::format(today), "bytes": bytes.len()});
                out.push((name.to_string(), format!("downloaded, {} bytes", bytes.len())));
            }
            Err(e) => {
                failures += 1;
                out.push((name.to_string(), format!("failed: {e}")));
            }
        }
    }
    meta["checked"] = dates::format(today).into();
    let text = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    write_bytes(&meta_path, text.as_bytes())?;
    Ok(out)
}

/// All months of the files in `dir(cache)`: `{first day of the month: {maturity: percent}}`.
pub fn load_monthly(cache: &std::path::Path) -> Res<BTreeMap<i64, BTreeMap<i64, f64>>> {
    let d = dir(cache);
    let mut months: BTreeMap<i64, BTreeMap<i64, f64>> = BTreeMap::new();
    for name in FILES {
        let p = d.join(format!("{name}.xls"));
        let Ok(bytes) = std::fs::read(&p) else { continue };
        for (day, m) in parse_hqm_spot(&bytes, &MATURITIES).map_err(|e| format!("{}: {e}", p.display()))? {
            months.entry(day).or_default().extend(m);
        }
    }
    if months.is_empty() {
        return Err(format!("no Treasury HQM files in {}; run `fa bonds hqm get`", d.display()));
    }
    Ok(months)
}

pub const MATURITIES: [i64; 8] = [1, 2, 3, 5, 7, 10, 20, 30];

/// Read the Treasury `.xls` files and compare each maturity with `<fred_dir>/fred_HQMCB<n>YR.csv`.
pub fn compare_files(xls: &[std::path::PathBuf], fred_dir: &std::path::Path) -> Res<Vec<HqmCompare>> {
    let mut months: BTreeMap<i64, BTreeMap<i64, f64>> = BTreeMap::new();
    for p in xls {
        let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
        for (d, m) in parse_hqm_spot(&bytes, &MATURITIES).map_err(|e| format!("{}: {e}", p.display()))? {
            months.entry(d).or_default().extend(m);
        }
    }
    MATURITIES
        .iter()
        .map(|n| {
            let p = fred_dir.join(format!("fred_HQMCB{n}YR.csv"));
            compare_with_fred(&months, *n, &std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The fixture is cut from the real file hqm_24_28.xls (9 maturities, Jan 2025 to Mar 2026), written as .xls by LibreOffice.
    fn fixture() -> Vec<u8> {
        std::fs::read(format!("{}/tests/fixtures/rates/hqm_spot_small.xls", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn reads_the_spot_rates_of_the_fixture() {
        let m = parse_hqm_spot(&fixture(), &MATURITIES).unwrap();
        assert_eq!(m.len(), 15);
        let jan25 = dates::parse("2025-01-01").unwrap();
        let mar26 = dates::parse("2026-03-01").unwrap();
        assert_eq!(m[&jan25][&10], 5.44);
        assert_eq!(m[&jan25][&1], 4.58);
        assert_eq!(m[&mar26][&30], 6.14);
        assert_eq!(m[&mar26][&2], 4.19);
        assert_eq!(m[&jan25].len(), 8, "the 0.5 year row is not one of the 8 maturities");
        assert_eq!(m.keys().next(), Some(&jan25));
        assert_eq!(m.keys().next_back(), Some(&mar26));
    }

    #[test]
    fn compare_counts_equal_and_different_months() {
        let m = parse_hqm_spot(&fixture(), &MATURITIES).unwrap();
        let fred = "observation_date,HQMCB10YR\n2024-12-01,5.00\n2025-01-01,5.44\n2025-02-01,5.29\n2025-03-01,5.40\n";
        let c = compare_with_fred(&m, 10, fred).unwrap();
        // Feb 2025: 5.28 against 5.29 (0.01); Mar 2025: 5.21 against 5.40 (0.19)
        assert_eq!((c.both, c.equal, c.off_by_one_hundredth, c.larger), (3, 1, 1, 1));
        assert_eq!(c.max_diff, 0.19);
        assert_eq!(c.first_ours.as_deref(), Some("2025-01-01"));
        assert_eq!(c.first_fred.as_deref(), Some("2024-12-01"));
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("fa-hqm-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn get_downloads_missing_files_keeps_the_others_and_records_the_origin() {
        let cache = tmp("get");
        let today = dates::parse("2026-11-21").unwrap();
        let calls = std::cell::RefCell::new(Vec::new());
        let mut fetch = |url: &str| -> Res<Vec<u8>> {
            calls.borrow_mut().push(url.to_string());
            Ok(fixture())
        };
        let first = get(&cache, false, today, &mut fetch).unwrap();
        assert_eq!(first.len(), 9);
        assert!(first.iter().all(|(_, w)| w.starts_with("downloaded")));
        assert_eq!(calls.borrow()[0], "https://home.treasury.gov/system/files/226/hqm_84_88.xls");
        let meta: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir(&cache).join("meta.json")).unwrap()).unwrap();
        assert_eq!(meta["checked"], "2026-11-21");
        assert_eq!(meta["files"]["hqm_24_28"]["url"], "https://home.treasury.gov/system/files/226/hqm_24_28.xls");
        assert_eq!(meta["source"], SOURCE);
        // A second run: all files exist and the open file is young, so there is no request.
        let n = calls.borrow().len();
        let second = get(&cache, false, today, &mut fetch).unwrap();
        assert!(second.iter().all(|(_, w)| w == "kept"));
        assert_eq!(calls.borrow().len(), n);
        // With --force all 9 files are asked again.
        get(&cache, true, today, &mut fetch).unwrap();
        assert_eq!(calls.borrow().len(), n + 9);
        // The reader merges the files.
        assert_eq!(load_monthly(&cache).unwrap().len(), 15);
    }

    #[test]
    fn a_bad_answer_is_not_kept_and_two_failures_stop_the_run() {
        let cache = tmp("bad");
        let today = dates::parse("2026-11-21").unwrap();
        let mut n = 0;
        let mut fetch = |_: &str| -> Res<Vec<u8>> {
            n += 1;
            Ok(b"<html>not found</html>".to_vec())
        };
        let r = get(&cache, false, today, &mut fetch).unwrap();
        assert!(r[0].1.starts_with("failed") && r[1].1.starts_with("failed"));
        assert!(r[2].1.starts_with("not tried"));
        assert_eq!(n, 2);
        assert!(!dir(&cache).join("hqm_84_88.xls").exists());
        assert!(load_monthly(&cache).unwrap_err().contains("fa bonds hqm get"));
    }

    #[test]
    fn rk_numbers_and_bad_files() {
        assert_eq!(rk_value((12 << 2) | 2), 12.0); // integer
        assert_eq!(rk_value((1234 << 2) | 3), 12.34); // integer / 100
        assert_eq!(rk_value(0x3FF0_0000), 1.0); // top 30 bits of the double 1.0
        assert!(parse_hqm_spot(b"not an excel file", &MATURITIES).is_err());
        assert!(parse_hqm_spot(&fixture(), &[99]).is_err());
    }
}
