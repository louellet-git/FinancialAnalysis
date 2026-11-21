//! Loades and the weekly data rules of the pre-registered premium test
//! (design `premium-design-55f2e92`, rule 4 and its data rules 1 to 10).
//!
//! Units: `s1`, `zero10` and `ret_w` of the series file are decimals. The outcome `y` and `dzero10`
//! are in percentage points (x100, as the design says). `ret_w` stays a decimal. DFII10 and
//! T10YIE keep the percent points of the FRED files. VIX keeps index points.

use serde::Serialize;

/// First Friday of the sample (rule 3).
pub const SAMPLE_START: &str = "2019-01-04";
/// Data rule 4: a value older than this many calendar days gives no value.
pub const MAX_AGE_DAYS: i64 = 3;

pub const HEADER: &str = "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change";

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn valid_ymd(y: i64, m: i64, d: i64) -> bool {
    (1..=12).contains(&m) && d >= 1 && civil_from_days(days_from_civil(y, m, d)) == (y, m, d)
}

/// `YYYY-MM-DD` to days since 1970-01-01.
pub fn parse_date(s: &str) -> Result<i64, String> {
    let p: Vec<&str> = s.trim().split('-').collect();
    if p.len() == 3 && p[0].len() == 4 && p[1].len() == 2 && p[2].len() == 2 {
        if let (Ok(y), Ok(m), Ok(d)) = (p[0].parse::<i64>(), p[1].parse::<i64>(), p[2].parse::<i64>()) {
            if valid_ymd(y, m, d) {
                return Ok(days_from_civil(y, m, d));
            }
        }
    }
    Err(format!("bad date '{s}' (want YYYY-MM-DD)"))
}

/// `MM/DD/YYYY` (Cboe) to days since 1970-01-01.
pub fn parse_us_date(s: &str) -> Result<i64, String> {
    let p: Vec<&str> = s.trim().split('/').collect();
    if p.len() == 3 {
        if let (Ok(m), Ok(d), Ok(y)) = (p[0].parse::<i64>(), p[1].parse::<i64>(), p[2].parse::<i64>()) {
            if valid_ymd(y, m, d) {
                return Ok(days_from_civil(y, m, d));
            }
        }
    }
    Err(format!("bad date '{s}' (want MM/DD/YYYY)"))
}

pub fn fmt_date(d: i64) -> String {
    let (y, m, dd) = civil_from_days(d);
    format!("{y:04}-{m:02}-{dd:02}")
}

/// 5 is Friday (0 is Sunday).
fn weekday(d: i64) -> i64 {
    (d + 4).rem_euclid(7)
}

/// A daily or weekly series. `pts` holds the non-missing values only, sorted by day.
#[derive(Debug, Clone)]
pub struct Daily {
    pub name: String,
    pub pts: Vec<(i64, f64)>,
    /// Rows of the file with "." or an empty value (missing, never 0).
    pub missing_rows: usize,
}

fn finish_daily(name: &str, mut pts: Vec<(i64, f64)>, missing_rows: usize) -> Result<Daily, String> {
    pts.sort_by_key(|p| p.0);
    if let Some(w) = pts.windows(2).find(|w| w[0].0 == w[1].0) {
        return Err(format!("{name}: two values for {}", fmt_date(w[0].0)));
    }
    Ok(Daily { name: name.to_string(), pts, missing_rows })
}

fn is_missing(v: &str) -> bool {
    let v = v.trim();
    v.is_empty() || v == "."
}

/// Cboe `VIX_History.csv`: header `DATE,OPEN,HIGH,LOW,CLOSE`, dates MM/DD/YYYY, uses CLOSE.
pub fn parse_vix(text: &str) -> Result<Daily, String> {
    let mut lines = text.lines();
    let head = lines.next().unwrap_or("").trim_start_matches('\u{feff}').trim();
    if head != "DATE,OPEN,HIGH,LOW,CLOSE" {
        return Err(format!("VIX file: unexpected header '{head}' (want DATE,OPEN,HIGH,LOW,CLOSE)"));
    }
    let (mut pts, mut miss) = (Vec::new(), 0);
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != 5 {
            return Err(format!("VIX file line {}: want 5 fields", i + 2));
        }
        let day = parse_us_date(f[0]).map_err(|e| format!("VIX file line {}: {e}", i + 2))?;
        if is_missing(f[4]) {
            miss += 1;
            continue;
        }
        let v: f64 = f[4].trim().parse().map_err(|_| format!("VIX file line {}: bad CLOSE '{}'", i + 2, f[4]))?;
        if !v.is_finite() {
            return Err(format!("VIX file line {}: CLOSE '{}' is not a finite number", i + 2, f[4].trim()));
        }
        pts.push((day, v));
    }
    finish_daily("VIX", pts, miss)
}

/// FRED graph CSV: header `observation_date,ID` (or `DATE,ID`), dates YYYY-MM-DD, "." is missing.
pub fn parse_fred(text: &str, id: &str) -> Result<Daily, String> {
    let mut lines = text.lines();
    let head = lines.next().unwrap_or("").trim_start_matches('\u{feff}').trim();
    let hf: Vec<&str> = head.split(',').map(str::trim).collect();
    let ok = hf.len() == 2 && (hf[0].eq_ignore_ascii_case("observation_date") || hf[0].eq_ignore_ascii_case("date")) && (hf[1].eq_ignore_ascii_case(id) || hf[1].eq_ignore_ascii_case("value"));
    if !ok {
        return Err(format!("{id} file: unexpected header '{head}' (want observation_date,{id} or DATE,value)"));
    }
    let (mut pts, mut miss) = (Vec::new(), 0);
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != 2 {
            return Err(format!("{id} file line {}: want 2 fields", i + 2));
        }
        let day = parse_date(f[0]).map_err(|e| format!("{id} file line {}: {e}", i + 2))?;
        if is_missing(f[1]) {
            miss += 1;
            continue;
        }
        let v: f64 = f[1].trim().parse().map_err(|_| format!("{id} file line {}: bad value '{}'", i + 2, f[1]))?;
        if !v.is_finite() {
            return Err(format!("{id} file line {}: value '{}' is not a finite number", i + 2, f[1].trim()));
        }
        pts.push((day, v));
    }
    finish_daily(id, pts, miss)
}

/// One row of the series file of `fa dcf premium-series`.
#[derive(Debug, Clone)]
pub struct SeriesRow {
    pub friday: i64,
    pub anchor: i64,
    pub curve_source: String,
    pub zero10: Option<f64>,
    pub s1: Option<f64>,
    pub ret_w: Option<f64>,
    pub s2_rates: Option<f64>,
    pub s2_premium: Option<f64>,
    /// The parts of S2 and the change, in file order: facts, time, rates, premium, growth, residual, nomodel, change.
    pub s2_parts: [Option<f64>; 8],
}

fn opt_num(f: &str, col: &str, line: usize) -> Result<Option<f64>, String> {
    let f = f.trim();
    if f.is_empty() {
        return Ok(None);
    }
    let v = f.parse::<f64>().map_err(|_| format!("series file line {line}: bad number '{f}' in {col}"))?;
    if !v.is_finite() {
        return Err(format!("series file line {line}: '{f}' in {col} is not a finite number"));
    }
    Ok(Some(v))
}

pub fn parse_series(text: &str) -> Result<Vec<SeriesRow>, String> {
    let mut lines = text.lines();
    let head = lines.next().unwrap_or("").trim_start_matches('\u{feff}').trim();
    if head != HEADER {
        return Err(format!("series file: unexpected header '{head}'\n  want: {HEADER}"));
    }
    let cols: Vec<&str> = HEADER.split(',').collect();
    let mut rows: Vec<SeriesRow> = Vec::new();
    for (i, line) in lines.enumerate() {
        let ln = i + 2;
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != cols.len() {
            return Err(format!("series file line {ln}: {} fields, want {}", f.len(), cols.len()));
        }
        let friday = parse_date(f[0]).map_err(|e| format!("series file line {ln}: {e}"))?;
        let anchor = parse_date(f[1]).map_err(|e| format!("series file line {ln}: {e}"))?;
        if weekday(friday) != 5 {
            return Err(format!("series file line {ln}: {} is not a Friday", f[0]));
        }
        if anchor > friday || friday - anchor > 6 {
            return Err(format!("series file line {ln}: anchor {} is not in the 7 days up to the Friday", f[1]));
        }
        if let Some(prev) = rows.last() {
            if friday <= prev.friday {
                return Err(format!("series file line {ln}: Fridays are not in increasing order"));
            }
        }
        for c in [6, 7, 9] {
            opt_num(f[c], cols[c], ln)?;
        }
        rows.push(SeriesRow {
            friday,
            anchor,
            curve_source: f[2].trim().to_string(),
            zero10: opt_num(f[4], cols[4], ln)?,
            s1: opt_num(f[5], cols[5], ln)?,
            ret_w: opt_num(f[8], cols[8], ln)?,
            s2_rates: opt_num(f[12], cols[12], ln)?,
            s2_premium: opt_num(f[13], cols[13], ln)?,
            s2_parts: {
                let mut p = [None; 8];
                for (k, c) in (10..18).enumerate() {
                    p[k] = opt_num(f[c], cols[c], ln)?;
                }
                p
            },
        });
    }
    if rows.is_empty() {
        return Err("series file: no data rows".into());
    }
    Ok(rows)
}

/// Result of one level lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lookup {
    /// A value; `carried` is true when it is older than the anchor (age 1 to 3 days).
    Value { v: f64, carried: bool },
    Missing,
}

/// Data rules 2 to 4: the last non-missing value on or before the anchor, at most 3 days old.
pub fn lookup_daily(d: &Daily, anchor: i64) -> Lookup {
    let k = d.pts.partition_point(|p| p.0 <= anchor);
    if k == 0 {
        return Lookup::Missing;
    }
    let (day, v) = d.pts[k - 1];
    if anchor - day > MAX_AGE_DAYS {
        Lookup::Missing
    } else {
        Lookup::Value { v, carried: day < anchor }
    }
}

/// Exact-date lookup: the value dated on that day. No age limit, no carry (used for SPY on the anchor).
pub fn lookup_exact(d: &Daily, friday: i64) -> Lookup {
    match d.pts.binary_search_by_key(&friday, |p| p.0) {
        Ok(k) => Lookup::Value { v: d.pts[k].1, carried: false },
        Err(_) => Lookup::Missing,
    }
}

pub struct External {
    pub vix: Daily,
    pub dfii: Option<Daily>,
    pub bei: Option<Daily>,
    /// SPY closes (rule 6.7). Used on the exact anchor date.
    pub spy: Option<Daily>,
}

/// SPY `prices.json`: an array of objects with `date` (YYYY-MM-DD) and `close`.
pub fn parse_spy_json(text: &str) -> Result<Daily, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("SPY file: {e}"))?;
    let arr = v.as_array().ok_or("SPY file: want an array")?;
    let mut pts = Vec::with_capacity(arr.len());
    for (i, o) in arr.iter().enumerate() {
        let d = o.get("date").and_then(|d| d.as_str()).ok_or(format!("SPY file item {i}: no date"))?;
        let c = o.get("close").and_then(serde_json::Value::as_f64).ok_or(format!("SPY file item {i}: no close"))?;
        pts.push((parse_date(d)?, c));
    }
    finish_daily("SPY", pts, 0)
}

/// One week with its changes. A change is `None` when it is dropped.
#[derive(Debug, Clone)]
pub struct Week {
    pub friday: i64,
    pub in_sample: bool,
    pub switch: bool,
    pub excluded: bool,
    pub y: Option<f64>,
    pub ret: Option<f64>,
    pub dz: Option<f64>,
    pub dvix: Option<f64>,
    pub ddfii: Option<f64>,
    pub dbei: Option<f64>,
    /// S2 plug part, percentage points.
    pub s2: Option<f64>,
    pub s2_rates: Option<f64>,
    /// The 8 parts of the series file (shares), see `SeriesRow::s2_parts`.
    pub s2_parts: [Option<f64>; 8],
    /// SPY weekly return on the anchors (decimal).
    pub spy_ret: Option<f64>,
}

impl Week {
    /// Weeks that all models need (y, ret, dzero10) and that are not dropped by a rule.
    pub fn core_ok(&self) -> bool {
        self.in_sample && !self.switch && !self.excluded && self.y.is_some() && self.ret.is_some() && self.dz.is_some()
    }
    /// Weeks of the test (M0 and M1).
    pub fn test_ok(&self) -> bool {
        self.core_ok() && self.dvix.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct SeriesCount {
    pub name: String,
    /// Weeks whose value is older than the anchor (age 1 to 3 days).
    pub carried: usize,
    /// Weeks without a value (so the changes of w and w+1 drop).
    pub missing: usize,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Counts {
    pub rows: usize,
    pub sample_weeks: usize,
    pub kept: usize,
    pub dropped: usize,
    /// The reasons overlap: one dropped week can have several.
    pub drop_switch: usize,
    pub drop_excluded: usize,
    pub drop_no_y: usize,
    pub drop_no_ret: usize,
    pub drop_no_dzero10: usize,
    pub drop_no_vix: usize,
    pub switch_weeks: Vec<String>,
    pub series: Vec<SeriesCount>,
}

fn levels(rows: &[SeriesRow], f: impl Fn(&SeriesRow) -> Lookup, name: &str) -> (Vec<Option<f64>>, SeriesCount) {
    let mut c = SeriesCount { name: name.into(), ..Default::default() };
    let lv = rows
        .iter()
        .map(|r| match f(r) {
            Lookup::Value { v, carried } => {
                c.carried += usize::from(carried);
                Some(v)
            }
            Lookup::Missing => {
                c.missing += 1;
                None
            }
        })
        .collect();
    (lv, c)
}

/// Data rules 6 and 7: the change of week i needs the levels of week i and of the row before,
/// and the row before must be the Friday 7 days earlier.
fn change(rows: &[SeriesRow], lv: &[Option<f64>], i: usize) -> Option<f64> {
    if i == 0 || rows[i].friday - rows[i - 1].friday != 7 {
        return None;
    }
    Some(lv[i]? - lv[i - 1]?)
}

/// Build the weeks. `excl` is the closed window of Fridays that the sensitivity run drops.
pub fn build_weeks(rows: &[SeriesRow], ext: &External, excl: Option<(i64, i64)>) -> (Vec<Week>, Counts) {
    let start = parse_date(SAMPLE_START).expect("constant");
    let (s1, _) = levels(rows, |r| r.s1.map_or(Lookup::Missing, |v| Lookup::Value { v, carried: false }), "s1");
    let (z10, _) = levels(rows, |r| r.zero10.map_or(Lookup::Missing, |v| Lookup::Value { v, carried: false }), "zero10");
    let (vix, cv) = levels(rows, |r| lookup_daily(&ext.vix, r.anchor), "VIX");
    let mut series = vec![cv];
    let spy = ext.spy.as_ref().map(|d| levels(rows, |r| lookup_exact(d, r.anchor), "SPY"));
    let spy = spy.map(|(l, c)| {
        series.push(c);
        l
    });
    let dfii = ext.dfii.as_ref().map(|d| levels(rows, |r| lookup_daily(d, r.anchor), "DFII10"));
    let bei = ext.bei.as_ref().map(|d| levels(rows, |r| lookup_daily(d, r.anchor), "T10YIE"));
    let (dfii, bei) = (
        dfii.map(|(l, c)| {
            series.push(c);
            l
        }),
        bei.map(|(l, c)| {
            series.push(c);
            l
        }),
    );
    let mut counts = Counts { rows: rows.len(), series, ..Default::default() };
    let mut weeks = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let switch = i > 0 && rows[i - 1].curve_source != r.curve_source;
        let w = Week {
            friday: r.friday,
            in_sample: r.friday >= start,
            switch,
            excluded: excl.is_some_and(|(a, b)| r.friday >= a && r.friday <= b),
            y: change(rows, &s1, i).map(|d| d * 100.0),
            ret: if i > 0 { r.ret_w } else { None },
            dz: change(rows, &z10, i).map(|d| d * 100.0),
            dvix: change(rows, &vix, i),
            ddfii: dfii.as_ref().and_then(|l| change(rows, l, i)),
            dbei: bei.as_ref().and_then(|l| change(rows, l, i)),
            s2: r.s2_premium.map(|v| v * 100.0),
            s2_rates: r.s2_rates,
            s2_parts: r.s2_parts,
            spy_ret: spy.as_ref().and_then(|l| change(rows, l, i).map(|d| d / l[i - 1].unwrap_or(f64::NAN))),
        };
        if w.in_sample {
            counts.sample_weeks += 1;
            counts.drop_switch += usize::from(w.switch);
            if w.switch {
                counts.switch_weeks.push(fmt_date(w.friday));
            }
            counts.drop_excluded += usize::from(w.excluded);
            counts.drop_no_y += usize::from(w.y.is_none());
            counts.drop_no_ret += usize::from(w.ret.is_none());
            counts.drop_no_dzero10 += usize::from(w.dz.is_none());
            counts.drop_no_vix += usize::from(w.dvix.is_none());
            counts.kept += usize::from(w.test_ok());
        }
        weeks.push(w);
    }
    counts.dropped = counts.sample_weeks - counts.kept;
    (weeks, counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERIES: &str = "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change
2018-12-28,2018-12-28,gsw,2018-12-28,0.0270,0.0200,500,500,,,,,,,,,,
2019-01-04,2019-01-04,gsw,2019-01-04,0.0260,0.0210,500,500,0.0100,500,,,,,,,,
2019-01-11,2019-01-11,gsw,2019-01-11,0.0250,0.0205,500,500,-0.0200,500,,,,,,,,
2019-01-18,2019-01-18,gsw,2019-01-18,0.0255,0.0207,500,500,0.0050,500,,,,,,,,
2019-01-25,2019-01-25,gsw,2019-01-25,0.0258,0.0209,500,500,0.0040,500,,,,,,,,
2019-02-01,2019-02-01,gsw,2019-02-01,0.0259,0.0208,500,500,0.0030,500,,,,,,,,
2019-02-08,2019-02-08,par,2019-02-08,0.0260,0.0210,500,500,0.0020,500,,,,,,,,
";
    const VIX: &str = "DATE,OPEN,HIGH,LOW,CLOSE
12/28/2018,1,1,1,20.0
01/04/2019,1,1,1,18.0
01/10/2019,1,1,1,16.0
01/25/2019,1,1,1,15.0
02/01/2019,1,1,1,14.0
02/08/2019,1,1,1,13.5
";
    const REAL: &str = "observation_date,DFII10
2018-12-28,2.2
2019-01-03,2.0
2019-01-04,.
2019-01-11,1.9
2019-01-18,1.8
2019-01-25,1.7
2019-02-01,1.6
2019-02-08,1.5
";

    #[test]
    fn date_round_trip_and_friday() {
        // 2019-01-04 is a Friday; 1970-01-01 is day 0.
        assert_eq!(parse_date("1970-01-01").unwrap(), 0);
        assert_eq!(parse_date("2019-01-04").unwrap(), 17_900);
        assert_eq!(fmt_date(17_900), "2019-01-04");
        assert_eq!(weekday(17_900), 5);
        assert_eq!(parse_us_date("01/04/2019").unwrap(), 17_900);
        assert!(parse_date("2019-02-30").is_err());
    }

    #[test]
    fn dot_is_missing_not_zero() {
        // DFII10 has "." on 2019-01-04. The point list must not hold a 0 for that day.
        let d = parse_fred(REAL, "DFII10").unwrap();
        assert_eq!(d.missing_rows, 1);
        assert!(d.pts.iter().all(|p| p.1 != 0.0));
        // Anchor 2019-01-04: the last non-missing value is 2.0 of 2019-01-03 (age 1, carried).
        assert_eq!(lookup_daily(&d, parse_date("2019-01-04").unwrap()), Lookup::Value { v: 2.0, carried: true });
        // A "." with nothing within 3 days gives Missing, never 0: anchor 2019-01-07 is 4 days after 01-03.
        let only_dot = parse_fred("observation_date,DFII10\n2019-01-03,2.0\n2019-01-04,.\n", "DFII10").unwrap();
        assert_eq!(lookup_daily(&only_dot, parse_date("2019-01-07").unwrap()), Lookup::Missing);
        // The exact lookup: a "." day is Missing, not the day before (2019-01-04 holds "." above).
        assert_eq!(lookup_exact(&d, parse_date("2019-01-04").unwrap()), Lookup::Missing);
        assert_eq!(lookup_exact(&d, parse_date("2019-01-11").unwrap()), Lookup::Value { v: 1.9, carried: false });
    }

    /// NaN and inf are numbers for Rust but not data: each loader names the file and the line.
    #[test]
    fn non_finite_values_are_rejected() {
        for bad in ["NaN", "inf", "-inf", "infinity"] {
            let e = parse_vix(&format!("DATE,OPEN,HIGH,LOW,CLOSE\n01/04/2019,1,1,1,{bad}\n")).unwrap_err();
            assert!(e.contains("VIX file line 2") && e.contains("finite"), "{e}");
            let e = parse_fred(&format!("observation_date,DFII10\n2019-01-03,2.0\n2019-01-04,{bad}\n"), "DFII10").unwrap_err();
            assert!(e.contains("DFII10 file line 3") && e.contains("finite"), "{e}");
            let row = format!("2019-01-04,2019-01-04,gsw,x,{bad},0.0210,,,0.01,,,,,,,,,\n");
            let e = parse_series(&format!("{HEADER}\n{row}")).unwrap_err();
            assert!(e.contains("series file line 2") && e.contains("zero10"), "{e}");
        }
        assert!(parse_vix("DATE,OPEN,HIGH,LOW,CLOSE\n01/04/2019,1,1,1,18.5\n").is_ok());
    }

    #[test]
    fn headers_are_checked() {
        assert!(parse_series("friday,anchor\n").is_err());
        assert!(parse_vix("Date,Close\n").is_err());
        assert!(parse_fred("observation_date,OTHER\n", "DFII10").is_err());
        assert!(parse_fred("DATE,DFII10\n2019-01-04,.\n", "DFII10").is_ok());
        assert!(parse_fred("DATE,value\n2019-01-04,1.5\n", "DFII10").is_ok());
    }

    /// Design data rule 10: the loader with a fixed answer, computed by hand.
    ///
    /// Fridays F0 2018-12-28 .. F6 2019-02-08. Sample weeks (from 2019-01-04): F1..F6 = 6.
    /// VIX levels: F0 20, F1 18, F2 16 (value of 01-10, age 1, carried), F3 none (last value 01-10 is
    /// 8 days old), F4 15, F5 14, F6 13.5. So dVIX: F1 -2, F2 -2, F3 and F4 dropped (rule 6).
    /// F5 (2019-02-01): dVIX = 14 - 15 = -1, y = (0.0208-0.0209)*100 = -0.01, dz = (0.0259-0.0258)*100 = 0.01,
    /// ret 0.003, and F4 is 7 days before: kept.
    /// F6 is the switch week (gsw to par) and drops. Kept weeks: F1, F2 and F5. Dropped: 3 (F3, F4, F6).
    /// F1: y = (0.0210-0.0200)*100 = 0.10, dz = (0.0260-0.0270)*100 = -0.10, ret 0.01.
    /// F2: y = (0.0205-0.0210)*100 = -0.05, dz = -0.10, ret -0.02.
    #[test]
    fn loader_fixed_answer() {
        let rows = parse_series(SERIES).unwrap();
        let ext = External {
            vix: parse_vix(VIX).unwrap(),
            dfii: None,
            bei: None,
            spy: None,
        };
        let (weeks, c) = build_weeks(&rows, &ext, None);
        let kept: Vec<String> = weeks.iter().filter(|w| w.test_ok()).map(|w| fmt_date(w.friday)).collect();
        assert_eq!(kept, ["2019-01-04", "2019-01-11", "2019-02-01"]);
        let close = |a: Option<f64>, b: f64| (a.unwrap() - b).abs() < 1e-9;
        let (f1, f2) = (&weeks[1], &weeks[2]);
        assert!(close(f1.y, 0.10) && close(f1.dz, -0.10) && close(f1.ret, 0.01));
        assert!(close(f1.dvix, -2.0));
        assert!(close(f2.y, -0.05) && close(f2.dz, -0.10) && close(f2.ret, -0.02));
        assert!(close(f2.dvix, -2.0));
        assert!(weeks[3].dvix.is_none() && weeks[4].dvix.is_none() && weeks[5].dvix.is_some());
        assert!(close(weeks[5].dvix, -1.0) && close(weeks[5].y, -0.01) && close(weeks[5].dz, 0.01) && close(weeks[5].ret, 0.003));
        assert!(weeks[6].switch);
        assert_eq!((c.rows, c.sample_weeks, c.kept, c.dropped), (7, 6, 3, 3));
        assert_eq!((c.drop_switch, c.drop_no_vix), (1, 2));
        assert_eq!(c.switch_weeks, ["2019-02-08"]);
        let by = |n: &str| c.series.iter().find(|s| s.name == n).unwrap().clone();
        assert_eq!((by("VIX").carried, by("VIX").missing), (1, 1));
        // The exclude window drops F2 only.
        let f2d = parse_date("2019-01-11").unwrap();
        let (w2, c2) = build_weeks(&rows, &ext, Some((f2d, f2d)));
        assert_eq!((c2.kept, c2.drop_excluded), (2, 1)); // F1 and F5 stay
        assert!(w2[2].excluded && !w2[2].test_ok());
    }

    #[test]
    fn spy_json_and_weekly_return() {
        // Closes on the anchors 2019-01-04 = 100, 2019-01-11 = 105: return 0.05. A missing anchor gives none.
        let j = r#"[{"date":"2019-01-04","close":100.0},{"date":"2019-01-11","close":105.0},{"date":"2019-01-25","close":99.0}]"#;
        let spy = parse_spy_json(j).unwrap();
        let rows = parse_series(SERIES).unwrap();
        let ext = External { vix: parse_vix(VIX).unwrap(), dfii: None, bei: None, spy: Some(spy) };
        let (w, _) = build_weeks(&rows, &ext, None);
        assert!(w[0].spy_ret.is_none() && w[1].spy_ret.is_none());
        assert!((w[2].spy_ret.unwrap() - 0.05).abs() < 1e-12);
        assert!(w[3].spy_ret.is_none() && w[4].spy_ret.is_none());
    }

    #[test]
    fn a_gap_in_the_fridays_gives_no_change() {
        // 2019-01-11 is missing: the change of 2019-01-18 would span two weeks.
        let text = "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change
2019-01-04,2019-01-04,gsw,x,0.0260,0.0210,,,0.01,,,,,,,,,
2019-01-18,2019-01-18,gsw,x,0.0255,0.0207,,,0.01,,,,,,,,,
";
        let rows = parse_series(text).unwrap();
        let ext = External { vix: parse_vix("DATE,OPEN,HIGH,LOW,CLOSE\n").unwrap(), dfii: None, bei: None, spy: None };
        let (weeks, _) = build_weeks(&rows, &ext, None);
        assert!(weeks[1].y.is_none() && weeks[1].dz.is_none());
    }
}
