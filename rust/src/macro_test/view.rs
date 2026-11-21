//! `fa macro view`: the economics view engine, version 1, layer A only (the SPF level).
//! Design (frozen at 3982324, records R1 to R4): `analysis/macro/2026-11-21-view-engine-design.md`.
//!
//! - Survey of t: the latest first-quarter survey whose release date is on or before t (rule 1.1).
//! - ERP_A(t) = median STOCK10 - GSW10 at the release date (points per year); Growth_A(t) = RGDP10 + CPI10 (rule 1).
//! - A window D1 to D2 is covered when a release date falls after D1 and on or before D2, and a survey is out
//!   at D1 (rules 2.2 to 2.4). A window that is not covered gives EMPTY values and the source "no release".
//! - Units: the file has decimals per year (SPF points / 100).
//!
//! GSW10 is the column SVENY10 of `rates/gsw_feds200628.csv` (store copy, read as a file, never downloaded). The yield is
//! continuously compounded. Source: the header block of the Board file `feds200628.csv` ("Zero-coupon yield,
//! Continuously Compounded,SVENYXX"), as copied in the test fixture of `forwards.rs`, and docs/HIDDEN-ASSUMPTIONS.md
//! #58. The real file is not in this checkout, so the header itself was not read here. The return form of STOCK10 is
//! open (design rule 7): ERP_A may mix two units.
//! A survey whose last GSW10 row is more than 10 days before the release is left out (R4.2); an older survey fills its place.
//!
//! The design does not say which windows the command writes. This command takes them from the caller (`--windows FILE`).
//! The split of the plug (design rule 6.6, last item) belongs to the reader of Fundamental Research (`fa dcf attribute --common file --common-file FILE`, record R4.1).
//!
//! The command reads files only. It starts no download and sends no order.

use super::data::{fmt_date, parse_date};
use super::CmdError;
use crate::market::spf::{self, Row};
use std::path::{Path, PathBuf};

pub const HEADER: &str = "date,d1,erp_change,growth_change,source,growth_parts";
pub const SOURCE_SPF: &str = "spf";
pub const SOURCE_NONE: &str = "no release";
/// The allowed values of the column `source`.
pub const SOURCES: [&str; 2] = [SOURCE_SPF, SOURCE_NONE];

pub const USAGE: &str = "\
fa macro view --windows FILE [--cache DIR] [--out FILE]

Economics view engine, version 1, layer A (SPF level). Design: analysis/macro/2026-11-21-view-engine-design.md.
  --windows FILE  CSV, one window each line: d1,date (YYYY-MM-DD, D1 before D2; a header line is allowed)
  --cache DIR     data store, default data/raw: macro/spf/medianLevel_<date>.xlsx (newest), macro/spf/spf-release-dates_<date>.txt
                  (newest), rates/gsw_feds200628.csv (SVENY10, continuously compounded). Nothing is downloaded.
  --out FILE      write the CSV there (default: standard output)
Output header: date,d1,erp_change,growth_change,source,growth_parts. The changes are decimals per year (0.0040 = 0.40 points).
source is 'spf' or 'no release' (then the changes are empty, never 0). A reading of the market, not a cause, not a trade signal.";

/// The level of one survey, known from its release date. SPF points per year.
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub release: i64,
    pub rgdp10: f64,
    pub cpi10: f64,
    pub stock10: f64,
    /// GSW10 at the release date, in points (percent per year).
    pub gsw10_points: f64,
    /// Calendar days from the last GSW10 row on or before the release to the release (R4.2).
    pub gsw_gap_days: i64,
}

/// A survey is left out when the last GSW10 row is more than this many calendar days before the release (R4.2).
pub const GSW_MAX_GAP_DAYS: i64 = 10;

impl Level {
    pub fn erp_points(&self) -> f64 {
        self.stock10 - self.gsw10_points
    }
    pub fn growth_points(&self) -> f64 {
        self.rgdp10 + self.cpi10
    }
}

/// GSW10 (decimals) on the last day on or before `day`. `days` is sorted.
pub fn gsw_on(days: &[i64], values: &[f64], day: i64) -> Option<f64> {
    let n = days.partition_point(|&d| d <= day);
    n.checked_sub(1).map(|i| values[i])
}

/// The last GSW10 row on or before `day`: (its day, value in decimals).
pub fn gsw_last(days: &[i64], values: &[f64], day: i64) -> Option<(i64, f64)> {
    let n = days.partition_point(|&d| d <= day);
    n.checked_sub(1).map(|i| (days[i], values[i]))
}

/// The levels, sorted by release date, from the Q1 rows of `fa spf show`. A row with a bound instead of a published
/// release date, a missing RGDP10, CPI10 or STOCK10, or no GSW10 on or before the release is left out; the count of
/// the left-out rows is the second value.
pub fn levels(rows: &[Row], gsw_days: &[i64], gsw_values: &[f64]) -> (Vec<Level>, usize) {
    let mut out = Vec::new();
    let mut skipped = 0;
    for (_, v, date, bound) in rows {
        let release = parse_date(date).ok();
        let ok = (!bound).then_some(()).and(release).zip(v[0]).zip(v[1]).zip(v[2]);
        let Some((((release, rgdp10), cpi10), stock10)) = ok else {
            skipped += 1;
            continue;
        };
        let Some((gday, g)) = gsw_last(gsw_days, gsw_values, release).filter(|&(gd, _)| release - gd <= GSW_MAX_GAP_DAYS) else {
            skipped += 1;
            continue;
        };
        out.push(Level { release, rgdp10, cpi10, stock10, gsw10_points: g * 100.0, gsw_gap_days: release - gday });
    }
    out.sort_by_key(|l| l.release);
    (out, skipped)
}

/// The survey of `t`: the latest release on or before `t` (rule 1.1). `levels` is sorted by release.
pub fn survey_at(levels: &[Level], t: i64) -> Option<&Level> {
    levels.iter().rev().find(|l| l.release <= t)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewRow {
    pub date: i64,
    pub d1: i64,
    /// Decimals per year; None = EMPTY.
    pub erp_change: Option<f64>,
    pub growth_change: Option<f64>,
    pub source: &'static str,
    pub growth_parts: String,
}

fn num(x: f64) -> String {
    format!("{x:?}")
}

/// One window (rules 2.1 to 2.4).
pub fn view_row(levels: &[Level], d1: i64, d2: i64) -> ViewRow {
    let empty = ViewRow { date: d2, d1, erp_change: None, growth_change: None, source: SOURCE_NONE, growth_parts: String::new() };
    if !levels.iter().any(|l| l.release > d1 && l.release <= d2) {
        return empty;
    }
    let (Some(a), Some(b)) = (survey_at(levels, d1), survey_at(levels, d2)) else { return empty };
    // Round the change in points first, then divide: 0.40 points is 0.0040.
    let dec = |pts: f64| (pts * 1e8).round() / 1e8 / 100.0;
    ViewRow {
        date: d2,
        d1,
        erp_change: Some(dec(b.erp_points() - a.erp_points())),
        growth_change: Some(dec(b.growth_points() - a.growth_points())),
        source: SOURCE_SPF,
        growth_parts: format!("RGDP10 {} + CPI10 {}", num(b.rgdp10), num(b.cpi10)),
    }
}

pub fn csv(rows: &[ViewRow]) -> String {
    let mut s = format!("{HEADER}\n");
    for r in rows {
        let f = |x: Option<f64>| x.map_or(String::new(), |x| format!("{x:.8}"));
        s += &format!("{},{},{},{},{},{}\n", fmt_date(r.date), fmt_date(r.d1), f(r.erp_change), f(r.growth_change), r.source, r.growth_parts);
    }
    s
}

/// The windows file: `d1,date` on each line; a first line that is not a date pair is a header.
pub fn parse_windows(text: &str) -> Result<Vec<(i64, i64)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split(',').map(str::trim).collect();
        if p.len() < 2 {
            return Err(format!("windows line {}: want d1,date", i + 1));
        }
        match (parse_date(p[0]), parse_date(p[1])) {
            (Ok(a), Ok(b)) if a < b => out.push((a, b)),
            (Ok(_), Ok(_)) => return Err(format!("windows line {}: d1 must be before date", i + 1)),
            _ if out.is_empty() && i == 0 => {}
            _ => return Err(format!("windows line {}: bad date", i + 1)),
        }
    }
    if out.is_empty() {
        return Err("the windows file has no window".into());
    }
    Ok(out)
}

/// All windows; the second value is the number of covered windows.
pub fn build(levels: &[Level], windows: &[(i64, i64)]) -> (Vec<ViewRow>, usize) {
    let rows: Vec<ViewRow> = windows.iter().map(|&(a, b)| view_row(levels, a, b)).collect();
    let covered = rows.iter().filter(|r| r.source == SOURCE_SPF).count();
    (rows, covered)
}

fn resolve(repo: &Path, p: &str) -> PathBuf {
    let p = PathBuf::from(p);
    if p.is_absolute() {
        p
    } else {
        repo.join(p)
    }
}

pub fn command(repo: &Path, a: &[String]) -> Result<i32, CmdError> {
    use CmdError::{Data, Usage};
    if a.iter().any(|x| x == "-h" || x == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let (mut windows, mut cache, mut out) = (None, "data/raw".to_string(), None);
    let mut i = 0;
    while i < a.len() {
        let slot = match a[i].as_str() {
            "--windows" => &mut windows,
            "--out" => &mut out,
            "--cache" => {
                let v = a.get(i + 1).filter(|v| !v.starts_with("--")).ok_or_else(|| Usage("--cache needs a value".into()))?;
                cache = v.clone();
                i += 2;
                continue;
            }
            x => return Err(Usage(if x.starts_with('-') { format!("unknown flag '{x}'") } else { format!("unexpected argument '{x}'") })),
        };
        let v = a.get(i + 1).filter(|v| !v.starts_with("--")).ok_or_else(|| Usage(format!("{} needs a value", a[i])))?;
        *slot = Some(v.clone());
        i += 2;
    }
    let windows = windows.ok_or_else(|| Usage("view needs --windows FILE".into()))?;
    let cache = resolve(repo, &cache);
    let read = |p: &Path| std::fs::read(p).map_err(|e| Data(format!("cannot read {}: {e} (this command downloads nothing)", p.display())));
    let wtext = String::from_utf8(read(&resolve(repo, &windows))?).map_err(|_| Data("windows file is not UTF-8".into()))?;
    let wins = parse_windows(&wtext).map_err(Data)?;
    let dir = spf::spf_dir(&cache);
    let xlsx = spf::newest(&dir, "medianLevel_", ".xlsx").ok_or_else(|| Data(format!("no medianLevel_*.xlsx in {}", dir.display())))?;
    let rel = spf::newest(&dir, "spf-release-dates_", ".txt").ok_or_else(|| Data(format!("no spf-release-dates_*.txt in {}", dir.display())))?;
    let rel_text = String::from_utf8(read(&rel)?).map_err(|_| Data("release dates file is not UTF-8".into()))?;
    let rows = spf::q1_rows(&read(&xlsx)?, Some(&rel_text)).map_err(Data)?;
    // The store copy only: `forwards::load` can download, so it is not used here.
    let gsw_path = cache.join("rates").join("gsw_feds200628.csv");
    let gsw_text = String::from_utf8(read(&gsw_path)?).map_err(|_| Data("GSW file is not UTF-8".into()))?;
    let gsw = crate::forwards::parse_gsw(&gsw_text).map_err(Data)?;
    let (gd, gv) = gsw.history("SVENY10").map_err(Data)?;
    let (levels, skipped) = levels(&rows, &gd, &gv);
    for l in &levels {
        eprintln!("fa: survey {} GSW10 gap {} days", fmt_date(l.release), l.gsw_gap_days);
    }
    let (view, covered) = build(&levels, &wins);
    let text = csv(&view);
    match out {
        Some(o) => {
            let o = resolve(repo, &o);
            std::fs::write(&o, &text).map_err(|e| Data(format!("cannot write {}: {e}", o.display())))?;
            println!("wrote {}", o.display());
        }
        None => print!("{text}"),
    }
    let rel_windows = view.iter().filter(|r| r.source == SOURCE_SPF).count();
    eprintln!("fa: windows {} | covered {} | release windows {} | surveys used {} | surveys left out {}", view.len(), covered, rel_windows, levels.len(), skipped);
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> i64 {
        parse_date(s).unwrap()
    }

    /// Rows as `q1_rows` gives them: year, [RGDP10, CPI10, STOCK10, BOND10, BILL10], release date, bound.
    fn row(y: i32, rgdp: f64, cpi: f64, stock: f64, rel: &str, bound: bool) -> Row {
        (y, [Some(rgdp), Some(cpi), Some(stock), Some(4.0), Some(3.0)], rel.into(), bound)
    }

    fn gsw() -> (Vec<i64>, Vec<f64>) {
        // decimals: 0.0400 on 2019-03-01, 0.0450 on 2020-03-19 and 2020-03-20, 0.0300 on 2021-03-01
        (vec![d("2019-03-01"), d("2020-03-19"), d("2020-03-20"), d("2021-03-01")], vec![0.04, 0.045, 0.045, 0.03])
    }

    fn base_rows() -> Vec<Row> {
        vec![row(2019, 2.0, 2.3, 6.0, "2019-03-01", false), row(2020, 2.2, 2.0, 6.4, "2020-03-20", false), row(2021, 1.8, 2.1, 7.0, "2021-03-01", false)]
    }

    #[test]
    fn levels_are_erp_over_gsw10_and_growth_sum() {
        let (g, v) = gsw();
        let (l, skipped) = levels(&base_rows(), &g, &v);
        assert_eq!(skipped, 0);
        assert!((l[0].erp_points() - 2.0).abs() < 1e-9); // 6.0 - 4.0
        assert!((l[1].erp_points() - 1.9).abs() < 1e-9); // 6.4 - 4.5
        assert!((l[0].growth_points() - 4.3).abs() < 1e-9);
    }

    #[test]
    fn unit_040_points_is_0040_decimals() {
        let (g, v) = gsw();
        // ERP 2019 = 6.0 - 4.0 = 2.0; ERP 2020 with the same GSW10 would be 2.4: use stock 6.4 and gsw 4.0.
        let mut v2 = v.clone();
        v2[1] = 0.04;
        v2[2] = 0.04;
        let (l, _) = levels(&base_rows(), &g, &v2);
        let r = view_row(&l, d("2019-06-01"), d("2020-04-01"));
        assert_eq!(r.erp_change, Some(0.0040));
        assert_eq!(csv(&[r]).lines().nth(1).unwrap().split(',').nth(2), Some("0.00400000"));
    }

    #[test]
    fn release_date_not_survey_quarter_sets_the_step() {
        let (g, v) = gsw();
        let (l, _) = levels(&base_rows(), &g, &v);
        // The 2020 survey is a Q1 survey, but its release date is 2020-03-20.
        let before = view_row(&l, d("2020-03-06"), d("2020-03-19"));
        assert_eq!((before.source, before.erp_change), (SOURCE_NONE, None));
        let on = view_row(&l, d("2020-03-13"), d("2020-03-20"));
        assert_eq!(on.source, SOURCE_SPF);
        // D1 on the release date: the release is not after D1, so no release window.
        let after = view_row(&l, d("2020-03-20"), d("2020-03-27"));
        assert_eq!(after.source, SOURCE_NONE);
        assert_eq!(survey_at(&l, d("2020-03-19")).unwrap().release, d("2019-03-01"));
        assert_eq!(survey_at(&l, d("2020-03-20")).unwrap().release, d("2020-03-20"));
    }

    #[test]
    fn real_release_file_lines_set_the_date() {
        // The line format of the Board file: year, quarter, deadline, release; "**" is a note mark.
        let m = spf::parse_release_dates("2026 Q1 3/2/26** 3/6/26**\n     Q2   5/15/26   5/22/26\n");
        assert_eq!(m[&(2026, 1)], "2026-03-06");
    }

    #[test]
    fn empty_row_has_source_no_release_and_no_zero() {
        let (g, v) = gsw();
        let (l, _) = levels(&base_rows(), &g, &v);
        let r = view_row(&l, d("2019-06-07"), d("2019-06-14"));
        assert_eq!((r.erp_change, r.growth_change, r.source), (None, None, "no release"));
        assert!(r.growth_parts.is_empty());
        assert_eq!(csv(&[r]).lines().nth(1).unwrap(), "2019-06-14,2019-06-07,,,no release,");
    }

    #[test]
    fn no_survey_at_d1_is_the_empty_row() {
        let (g, v) = gsw();
        let (l, _) = levels(&base_rows(), &g, &v);
        let r = view_row(&l, d("2018-12-01"), d("2019-03-08"));
        assert_eq!((r.source, r.erp_change), (SOURCE_NONE, None));
    }

    #[test]
    fn output_header_and_sources() {
        let (g, v) = gsw();
        let (l, _) = levels(&base_rows(), &g, &v);
        let wins = [(d("2019-06-07"), d("2019-06-14")), (d("2020-03-13"), d("2020-03-20"))];
        let (rows, covered) = build(&l, &wins);
        assert_eq!(covered, 1);
        let t = csv(&rows);
        let mut lines = t.lines();
        assert_eq!(lines.next().unwrap(), "date,d1,erp_change,growth_change,source,growth_parts");
        for line in lines {
            let f: Vec<&str> = line.split(',').collect();
            assert_eq!(f.len(), 6);
            assert!(SOURCES.contains(&f[4]));
        }
        assert_eq!(rows[1].growth_parts, "RGDP10 2.2 + CPI10 2.0");
        assert!((rows[1].growth_change.unwrap() - (4.2 - 4.3) / 100.0).abs() < 1e-12);
        assert!((rows[1].erp_change.unwrap() - (1.9 - 2.0) / 100.0).abs() < 1e-12);
    }

    /// Poison: change the later surveys (and the later GSW rows) absurdly. No earlier window moves.
    #[test]
    fn poison_later_survey_moves_no_earlier_window() {
        let (g, v) = gsw();
        let wins = [(d("2018-06-01"), d("2018-12-01")), (d("2019-02-22"), d("2019-03-01")), (d("2019-06-07"), d("2020-03-13")), (d("2020-03-13"), d("2020-03-20"))];
        let (l0, _) = levels(&base_rows(), &g, &v);
        let mut rows = base_rows();
        rows[2] = row(2021, 99.0, -50.0, 1234.0, "2021-03-01", false);
        let mut v2 = v.clone();
        v2[3] = 9.99;
        let (l1, _) = levels(&rows, &g, &v2);
        for &(a, b) in &wins {
            assert_eq!(view_row(&l0, a, b), view_row(&l1, a, b), "window {a} {b}");
        }
        // A window that holds the poisoned release does move.
        assert_ne!(view_row(&l0, d("2020-06-01"), d("2021-03-05")), view_row(&l1, d("2020-06-01"), d("2021-03-05")));
    }

    /// Truncation: cut the survey rows and the GSW rows after the 2020 release. Each window up to it is the same row.
    #[test]
    fn truncation_after_a_survey_gives_the_same_rows() {
        let (g, v) = gsw();
        let (full, _) = levels(&base_rows(), &g, &v);
        let cut = d("2020-03-20");
        let rows: Vec<Row> = base_rows().into_iter().filter(|r| d(&r.2) <= cut).collect();
        let n = g.iter().filter(|&&x| x <= cut).count();
        let (short, _) = levels(&rows, &g[..n], &v[..n]);
        let wins = [(d("2018-06-01"), d("2019-03-01")), (d("2019-03-01"), d("2019-09-01")), (d("2019-09-01"), d("2020-03-20")), (d("2020-03-13"), d("2020-03-20"))];
        for &(a, b) in &wins {
            assert_eq!(view_row(&full, a, b), view_row(&short, a, b));
        }
    }

    #[test]
    fn a_bound_release_or_missing_value_is_left_out() {
        let (g, v) = gsw();
        let mut rows = base_rows();
        rows[1] = row(2020, 2.2, 2.0, 6.4, "2020-03-31", true);
        rows[2].1[2] = None;
        let (l, skipped) = levels(&rows, &g, &v);
        assert_eq!((l.len(), skipped), (1, 2));
    }

    #[test]
    fn gsw_takes_the_last_row_on_or_before_the_release() {
        let (g, v) = gsw();
        assert_eq!(gsw_on(&g, &v, d("2020-03-19")), Some(0.045));
        assert_eq!(gsw_on(&g, &v, d("2020-03-25")), Some(0.045));
        assert_eq!(gsw_on(&g, &v, d("2019-02-28")), None);
    }

    #[test]
    fn windows_file_parses_with_a_header() {
        let w = parse_windows("d1,date\n2019-01-04,2019-01-11\n").unwrap();
        assert_eq!(w, vec![(d("2019-01-04"), d("2019-01-11"))]);
        assert!(parse_windows("2019-01-11,2019-01-04\n").is_err());
        assert!(parse_windows("d1,date\n").is_err());
    }

    #[test]
    fn gsw_gap_ten_days_is_kept_eleven_is_left_out() {
        // Release 2020-03-20; GSW rows on 2020-03-10 (gap 10) and 2020-03-09 (gap 11).
        let rows = vec![row(2020, 2.2, 2.0, 6.4, "2020-03-20", false)];
        let (l, skipped) = levels(&rows, &[d("2020-03-10")], &[0.04]);
        assert_eq!((l.len(), skipped, l[0].gsw_gap_days), (1, 0, 10));
        let (l, skipped) = levels(&rows, &[d("2020-03-09")], &[0.04]);
        assert_eq!((l.len(), skipped), (0, 1));
    }

    #[test]
    fn a_left_out_survey_lets_the_older_survey_fill_its_place() {
        let rows = base_rows();
        // The 2020 survey has no GSW row within 10 days; 2019 and 2021 do.
        let (g, v) = (vec![d("2019-03-01"), d("2021-03-01")], vec![0.04, 0.03]);
        let (l, skipped) = levels(&rows, &g, &v);
        assert_eq!((l.len(), skipped), (2, 1));
        assert_eq!(survey_at(&l, d("2020-12-31")).unwrap().release, d("2019-03-01"));
    }

    fn xlsx_fixture() -> Vec<u8> {
        use rust_xlsxwriter::Workbook;
        let mut wb = Workbook::new();
        let data: [(&str, [f64; 2]); 5] =
            [("RGDP10", [2.0, 2.2]), ("CPI10", [2.3, 2.0]), ("STOCK10", [6.0, 6.4]), ("BOND10", [4.0, 4.0]), ("BILL10", [3.0, 3.0])];
        for (name, v) in data {
            let ws = wb.add_worksheet();
            ws.set_name(name).unwrap();
            ws.write_string(0, 0, "YEAR").unwrap();
            ws.write_string(0, 1, "QUARTER").unwrap();
            ws.write_string(0, 2, "VALUE").unwrap();
            for (i, (y, x)) in [(2019.0, v[0]), (2020.0, v[1])].into_iter().enumerate() {
                ws.write_number(i as u32 + 1, 0, y).unwrap();
                ws.write_number(i as u32 + 1, 1, 1.0).unwrap();
                ws.write_number(i as u32 + 1, 2, x).unwrap();
            }
        }
        wb.save_to_buffer().unwrap()
    }

    fn gsw_csv() -> String {
        let mut h = vec!["Date".to_string()];
        h.extend(["BETA0", "BETA1", "BETA2", "BETA3", "TAU1", "TAU2"].map(String::from));
        for n in [1, 4, 9] {
            h.push(format!("SVEN1F{n:02}"));
        }
        for p in ["SVENF", "SVENPY", "SVENY"] {
            for n in 1..=30 {
                h.push(format!("{p}{n:02}"));
            }
        }
        let mut t = format!("Zero-coupon yield,Continuously Compounded,SVENYXX\n\n{}\n", h.join(","));
        for (day, y10) in [("2019-03-01", "4.0"), ("2020-03-20", "4.5")] {
            let f: Vec<String> = h.iter().map(|c| if c == "Date" { day.into() } else if c == "SVENY10" { y10.into() } else { "1.0".into() }).collect();
            t += &(f.join(",") + "\n");
        }
        t
    }

    /// The reader path of `command()`: synthetic xlsx, release dates and GSW in a temp dir; fixed answer.
    #[test]
    fn command_reads_the_cache_and_writes_the_rows() {
        let root = std::env::temp_dir().join(format!("fa_view_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("macro/spf")).unwrap();
        std::fs::create_dir_all(root.join("rates")).unwrap();
        std::fs::write(root.join("macro/spf/medianLevel_2026-11-21.xlsx"), xlsx_fixture()).unwrap();
        std::fs::write(root.join("macro/spf/spf-release-dates_2026-11-21.txt"), "2019 Q1 2/22/19 3/1/19\n2020 Q1 3/2/20 3/20/20\n").unwrap();
        std::fs::write(root.join("rates/gsw_feds200628.csv"), gsw_csv()).unwrap();
        std::fs::write(root.join("w.csv"), "d1,date\n2019-06-07,2019-06-14\n2019-06-07,2020-04-01\n").unwrap();
        let out = root.join("out.csv");
        let args: Vec<String> = ["--cache", root.to_str().unwrap(), "--windows", root.join("w.csv").to_str().unwrap(), "--out", out.to_str().unwrap()]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(command(&root, &args).unwrap(), 0);
        let text = std::fs::read_to_string(&out).unwrap();
        // ERP 2019 = 6.0 - 4.0 = 2.0; ERP 2020 = 6.4 - 4.5 = 1.9; growth 4.3 to 4.2.
        let want = "date,d1,erp_change,growth_change,source,growth_parts\n2019-06-14,2019-06-07,,,no release,\n2020-04-01,2019-06-07,-0.00100000,-0.00100000,spf,RGDP10 2.2 + CPI10 2.0\n";
        assert_eq!(text, want);
        let _ = std::fs::remove_dir_all(&root);
    }
}
