//! `fa macro premium-test`: the frozen pre-registered test of the weekly change of the implied
//! premium (S1) against macro risk measures. Design: `premium-design-55f2e92` (rules 1 to 12).
//!
//! Choices where the design is unclear (accepted by the coordinator, 2026-11-21):
//! 1. `--list` is a label only; run the command once for each list with its series file.
//! 2. A missing `ret_w` drops only its own week. Rule 6 (drop w and w+1) is applied to the levels
//!    s1, zero10, VIX, DFII10, T10YIE (and SPY for rule 6.7).
//! 3. DFII10 and T10YIE use the daily rule of VIX: last non-missing value on or before the anchor, at most 3 days old.
//! 4. The switch week is the row whose `curve_source` differs from the row before; week w+1 stays.
//! 5. The sensitivity run flags the weeks of the window as dropped; neighbouring changes still use the levels.
//! 6. Units: y, S2 and dzero10 in percentage points (x100); ret_w a decimal; VIX in index points;
//!    DFII10, T10YIE in the percent points of the FRED files. R2, p and gain do not depend on the scale.
//! 7. Clark-West p from the normal, upper tail; a variance that is not positive gives p = 1. The placebo
//!    shifts over the kept weeks in time order, applied to all kept weeks.
//! 8. Columns are scaled inside the OLS for numerics only.
//! 9. M1 = M0 + dVIX. No credit spread (record R1) and no NFCI (record R2: the Chicago Fed terms do not permit a
//!    stored copy). Records: `analysis/macro/2026-11-21-premium-macro-design.md`.
//!
//! Added with rules 7.2, 6.3, 6.4, 6.7 (also accepted): the stress R2 uses the residuals of one full-sample fit
//! (R2 of a subset = 1 - SSE/SST of the subset); the variance split uses the kept weeks with all 8 S2 columns
//! and sample variances; the SPY return is the change of the close on the exact anchor dates (a missing anchor
//! date gives no value) and the SPY check uses the kept weeks that have it.
//!
//! The command reads files only. It starts no download and sends no order. It is a statistical
//! link, not a cause, and not a trade signal.

pub mod data;
pub mod design;
pub mod extras;
pub mod stats;
pub mod view;

use data::{build_weeks, fmt_date, parse_date, parse_fred, parse_series, parse_vix, Counts, External};
use design::{real_bei_rows, run_block, run_primary, s2_rows, test_rows, Block, Primary};
use extras::{sanity, spy_check, stress, var_split, Sanity, SpyCheck, Stress, VarSplit};
use serde::Serialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa macro premium-test --series FILE [--dfii10 FILE] [--t10yie FILE]
                      [--cache DIR] [--out FILE]
                      [--exclude-from YYYY-MM-DD --exclude-to YYYY-MM-DD] [--list sp500|sp100]

The frozen test of the weekly change of the implied premium S1 (design premium-design-55f2e92).
  --series FILE   CSV of `fa dcf premium-series` (branch claude/premium-series, not yet on main; fixed header; decimals; empty field = missing)
  --cache DIR     data store, default <repo>/data/raw (repo = FA_REPO, else the repository that holds the crate; a relative path is joined to it) (vix/VIX_History.csv, SPY/prices.json)
  --dfii10 FILE, --t10yie FILE  FRED CSV (optional; without them the descriptive block prints 'not run')
                  --series, --dfii10 and --t10yie are paths relative to the current directory.
                  FRED files: header observation_date,<ID> or DATE,value; '.' or an empty value is missing
  --out FILE      JSON of the results only (no weekly series)
  --exclude-from/--exclude-to  also run the sensitivity without the seen weeks (both flags, closed window)
  --list          label of the list of the series file (default sp500); run once for each list
Models: M0 = intercept, ret_w, ret_w^2, dzero10. M1 = M0 + dVIX (records R1, R2). Expanding-window OLS,
first fit on 104 kept weeks. One test: Clark-West (one-sided, Newey-West lag 8). Placebo: 80 circular shifts of the dVIX column.
Answer: yes, or no share shown. Rates are decimals in the series file; y and dzero10 are in percentage points.

fa macro view --windows FILE [--cache DIR] [--out FILE]
                      the economics view engine, layer A (SPF level); see fa macro view --help";

#[derive(Debug, Serialize)]
pub struct Results {
    pub design: &'static str,
    pub list: String,
    pub series_file: String,
    pub first_friday: String,
    pub last_friday: String,
    pub counts: Counts,
    /// Rule 7.2, before the primary. When it fails, nothing below is run.
    pub sanity: Sanity,
    pub primary: Option<Primary>,
    pub stress_windows: Option<Stress>,
    pub s2_variance_split: Option<VarSplit>,
    pub spy_check: Option<SpyCheck>,
    pub description_notes: Vec<String>,
    pub s2_description: Option<Block>,
    pub s2_note: Option<String>,
    pub real_bei_description: Option<Block>,
    pub real_bei_note: Option<String>,
    pub sensitivity: Option<Sensitivity>,
}

#[derive(Debug, Serialize)]
pub struct Sensitivity {
    pub excluded_from: String,
    pub excluded_to: String,
    pub counts: Counts,
    pub primary: Option<Primary>,
    pub note: Option<String>,
    /// True when the sensitivity answer differs from the primary answer.
    pub answer_depends_on_seen_weeks: bool,
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e} (this command downloads nothing)", path.display()))
}

pub fn run(series_text: &str, ext: &External, list: &str, series_name: &str, excl: Option<(i64, i64)>) -> Result<Results, String> {
    let rows = parse_series(series_text)?;
    let (weeks, counts) = build_weeks(&rows, ext, None);
    let sanity = sanity(&weeks);
    let head = |primary: Option<Primary>| Results {
        design: "premium-design-55f2e92",
        list: list.into(),
        series_file: series_name.into(),
        first_friday: fmt_date(rows[0].friday),
        last_friday: fmt_date(rows[rows.len() - 1].friday),
        counts: counts.clone(),
        sanity: sanity.clone(),
        primary,
        stress_windows: None,
        s2_variance_split: None,
        spy_check: None,
        description_notes: Vec::new(),
        s2_description: None,
        s2_note: None,
        real_bei_description: None,
        real_bei_note: None,
        sensitivity: None,
    };
    if !sanity.passed {
        return Ok(head(None));
    }
    let primary = run_primary(&test_rows(&weeks))?;
    let mut notes = Vec::new();
    let stress_windows = stress(&test_rows(&weeks)).map_err(|e| notes.push(format!("stress windows not run: {e}"))).ok();
    let s2_variance_split = var_split(&weeks).map_err(|e| notes.push(format!("variance split not run: {e}"))).ok();
    let spy_check = if ext.spy.is_some() { spy_check(&weeks).map_err(|e| notes.push(format!("SPY check not run: {e}"))).ok() } else {
        notes.push("SPY check not run: no SPY file".into());
        None
    };
    let s2 = run_block(&s2_rows(&weeks));
    let (s2_description, s2_note) = match s2 {
        Ok(b) => (Some(b), None),
        Err(e) => (None, Some(format!("S2 description not run: {e}"))),
    };
    let (real_bei_description, real_bei_note) = if ext.dfii.is_some() && ext.bei.is_some() {
        match run_block(&real_bei_rows(&weeks)) {
            Ok(b) => (Some(b), None),
            Err(e) => (None, Some(format!("descriptive block not run: {e}"))),
        }
    } else {
        let mut missing = Vec::new();
        if ext.dfii.is_none() {
            missing.push("DFII10 (--dfii10 FILE not given or not usable)");
        }
        if ext.bei.is_none() {
            missing.push("T10YIE (--t10yie FILE not given or not usable)");
        }
        (None, Some(format!("descriptive block not run: {}", missing.join(", "))))
    };
    let sensitivity = excl.map(|(a, b)| {
        let (w2, c2) = build_weeks(&rows, ext, Some((a, b)));
        let (p, note) = match run_primary(&test_rows(&w2)) {
            Ok(p) => (Some(p), None),
            Err(e) => (None, Some(e)),
        };
        let differs = p.as_ref().is_some_and(|p| p.verdict != primary.verdict);
        Sensitivity { excluded_from: fmt_date(a), excluded_to: fmt_date(b), counts: c2, primary: p, note, answer_depends_on_seen_weeks: differs }
    });
    let mut r = head(Some(primary));
    r.stress_windows = stress_windows;
    r.s2_variance_split = s2_variance_split;
    r.spy_check = spy_check;
    r.description_notes = notes;
    r.s2_description = s2_description;
    r.s2_note = s2_note;
    r.real_bei_description = real_bei_description;
    r.real_bei_note = real_bei_note;
    r.sensitivity = sensitivity;
    Ok(r)
}

fn f(v: f64) -> String {
    format!("{v:.4}")
}

fn block_text(out: &mut String, b: &Block, label: &str) {
    let _ = writeln!(out, "  {label}: weeks {} (train {}, OOS {}: {} to {})", b.oos.n_kept, b.oos.n_train, b.oos.n_oos, b.oos.first_oos_friday, b.oos.last_oos_friday);
    let _ = writeln!(out, "    M0 R2 in sample {} | OOS vs zero {} | OOS vs expanding mean {}", f(b.m0_insample.r2), f(b.oos.m0_r2_vs_zero), f(b.oos.m0_r2_vs_expanding_mean));
    let _ = writeln!(out, "    M1 R2 in sample {} | OOS vs zero {} | OOS vs expanding mean {}", f(b.m1_insample.r2), f(b.oos.m1_r2_vs_zero), f(b.oos.m1_r2_vs_expanding_mean));
    let _ = writeln!(out, "    gain of M1 over M0 {} | Clark-West t {} p (one-sided) {}", f(b.oos.gain_m1_over_m0), f(b.oos.cw_t), f(b.oos.cw_p_one_sided));
    let _ = writeln!(out, "    share of OOS SSE from the 5 largest weeks: M0 {} M1 {}", f(b.oos.top5_sse_share_m0), f(b.oos.top5_sse_share_m1));
    let _ = writeln!(out, "    coefficients in sample (description, not a test; Newey-West t, lag 8):");
    for (n, m) in [("M0", &b.m0_insample), ("M1", &b.m1_insample)] {
        let cs: Vec<String> = m.coefs.iter().map(|c| format!("{} {:.5} (t {:.2})", c.name, c.beta, c.t_nw)).collect();
        let _ = writeln!(out, "      {n}: {}", cs.join("; "));
    }
}

fn counts_text(out: &mut String, c: &Counts) {
    let _ = writeln!(out, "  sample weeks {} | kept {} | dropped {}", c.sample_weeks, c.kept, c.dropped);
    let _ = writeln!(
        out,
        "  drop reasons (overlap): switch {} | excluded {} | no y {} | no ret {} | no dzero10 {} | no dVIX {}",
        c.drop_switch, c.drop_excluded, c.drop_no_y, c.drop_no_ret, c.drop_no_dzero10, c.drop_no_vix
    );
    let _ = writeln!(out, "  switch weeks (dropped from all models): {}", if c.switch_weeks.is_empty() { "none".into() } else { c.switch_weeks.join(", ") });
    for s in &c.series {
        let _ = writeln!(out, "  {}: carried values {} | weeks without value {}", s.name, s.carried, s.missing);
    }
}

pub fn report_text(r: &Results) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "Premium test ({}) | list {} | file {} | Fridays {} to {}", r.design, r.list, r.series_file, r.first_friday, r.last_friday);
    let _ = writeln!(o, "A statistical link, not a cause and not a trade signal. Survivorship: the lists are today's lists.");
    let _ = writeln!(o, "\nSANITY CHECK (rule 7.2): correlation of s2_rates with the change of zero10 over {} kept weeks = {} (must be below {})", r.sanity.n, f(r.sanity.corr_s2_rates_dzero10), extras::SANITY_MAX_CORR);
    let Some(primary) = &r.primary else {
        let _ = writeln!(o, "sanity check failed: the bridge has a defect; the test stops");
        return o;
    };
    let _ = writeln!(o, "  sanity check passed");
    let b = &primary.block;
    let _ = writeln!(o, "\nIDENTITY CHECK (M0, mechanical), read first:");
    let _ = writeln!(o, "  M0 R2 in sample {} | OOS vs zero {} | OOS vs expanding mean {}", f(b.m0_insample.r2), f(b.oos.m0_r2_vs_zero), f(b.oos.m0_r2_vs_expanding_mean));
    let _ = writeln!(o, "\nCOUNTS (data rules):");
    counts_text(&mut o, &r.counts);
    let _ = writeln!(o, "\nPRIMARY (rule 5):");
    block_text(&mut o, b, "M0 and M1");
    let p = &primary.placebo;
    let _ = writeln!(o, "  placebo: {} runs, {} yes (limit {}); gain of placebo runs min {} median {} max {}", p.runs, p.yes, design::PLACEBO_MAX_YES, f(p.gain_min), f(p.gain_median), f(p.gain_max));
    let _ = writeln!(o, "  real result passes p < {} and gain >= {}: {}", design::CW_P_MAX, design::GAIN_MIN, primary.real_yes_rule);
    let _ = writeln!(o, "  ANSWER: {}", primary.verdict);
    let _ = writeln!(o, "\nDESCRIPTION, S2 plug part in points (not a test):");
    match (&r.s2_description, &r.s2_note) {
        (Some(b), _) => block_text(&mut o, b, "S2"),
        (None, n) => {
            let _ = writeln!(o, "  {}", n.clone().unwrap_or_default());
        }
    }
    let _ = writeln!(o, "\nDESCRIPTION, M0 with dDFII10 and dT10YIE (not a test; not in the verdict):");
    match (&r.real_bei_description, &r.real_bei_note) {
        (Some(b), _) => block_text(&mut o, b, "M0 + real + breakeven"),
        (None, n) => {
            let _ = writeln!(o, "  {}", n.clone().unwrap_or_default());
        }
    }
    let _ = writeln!(o, "\nDESCRIPTION, in-sample R2 inside the stress windows against the other weeks (rule 6.3; not a test):");
    if let Some(st) = &r.stress_windows {
        let _ = writeln!(o, "  weeks in a window {} | other weeks {}", st.n_stress, st.n_other);
        for row in &st.rows {
            let _ = writeln!(o, "  {}: stress {} | other {}", row.model, f(row.r2_stress), f(row.r2_other));
        }
    }
    if let Some(v) = &r.s2_variance_split {
        let _ = writeln!(o, "\nDESCRIPTION, variance split of s2_change over {} weeks (rule 6.4; var(change) {}):", v.n, f(v.var_change));
        let vs: Vec<String> = v.var_share.iter().map(|p| format!("{} {}", p.0, f(p.1))).collect();
        let _ = writeln!(o, "  variance shares: {}", vs.join("; "));
        let cs: Vec<String> = v.cov_share.iter().filter(|p| p.1.abs() >= 0.01).map(|p| format!("{} {}", p.0, f(p.1))).collect();
        let _ = writeln!(o, "  covariance shares (2 cov / var, |share| >= 0.01; all pairs in the JSON): {}", cs.join("; "));
        let _ = writeln!(o, "  sum of variance shares {} | sum of covariance shares {} | other (not in the 7 parts) {}", f(v.sum_var_share), f(v.sum_cov_share), f(v.other_share));
    }
    if let Some(sp) = &r.spy_check {
        let _ = writeln!(o, "\nDESCRIPTION, SPY weekly return next to ret_w over {} weeks (rule 6.7):", sp.n);
        let _ = writeln!(o, "  correlation {} | M0 R2 in sample: ret_w {} SPY {}", f(sp.corr_ret_spy), f(sp.m0_r2_in_sample_ret_w), f(sp.m0_r2_in_sample_spy));
        if let (Some(a), Some(b)) = (sp.m0_r2_oos_vs_zero_ret_w, sp.m0_r2_oos_vs_zero_spy) {
            let _ = writeln!(o, "  M0 R2 OOS vs zero: ret_w {} SPY {}", f(a), f(b));
        }
    }
    for n in &r.description_notes {
        let _ = writeln!(o, "  note: {n}");
    }
    if let Some(s) = &r.sensitivity {
        let _ = writeln!(o, "\nSENSITIVITY without the seen weeks {} to {} (description):", s.excluded_from, s.excluded_to);
        counts_text(&mut o, &s.counts);
        match &s.primary {
            Some(p) => {
                block_text(&mut o, &p.block, "M0 and M1");
                let _ = writeln!(o, "  placebo yes {} of {}; answer without the seen weeks: {}", p.placebo.yes, p.placebo.runs, p.verdict);
            }
            None => {
                let _ = writeln!(o, "  not run: {}", s.note.clone().unwrap_or_default());
            }
        }
        if s.answer_depends_on_seen_weeks {
            let _ = writeln!(o, "  the answer depends on the seen weeks");
        }
    }
    o
}

/// An optional file that cannot be used is named in the report next to the item it feeds. The exit code
/// stays 0: the main test does not use these files.
fn mark_optional_not_run(r: &mut Results, errors: &[String]) {
    if errors.is_empty() {
        return;
    }
    r.real_bei_description = None;
    r.real_bei_note = Some(format!("NOT RUN: {}. The exit code stays 0 because the main test is still valid.", errors.join("; ")));
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

/// The flags that take a value.
const VALUE_FLAGS: [&str; 8] = ["--series", "--cache", "--out", "--exclude-from", "--exclude-to", "--list", "--dfii10", "--t10yie"];

/// An error of the command. A usage error prints the usage and exits 64; a data error prints its own
/// message and exits 65 (EX_DATAERR).
#[derive(Debug, PartialEq)]
pub enum CmdError {
    Usage(String),
    Data(String),
}

/// Reject an unknown flag, a stray word, a flag without a value, and a flag whose value is another flag.
fn check_args(a: &[String]) -> Result<(), String> {
    let mut i = 0;
    while i < a.len() {
        let x = &a[i];
        if !VALUE_FLAGS.contains(&x.as_str()) {
            return Err(if x.starts_with('-') { format!("unknown flag '{x}'") } else { format!("unexpected argument '{x}'") });
        }
        match a.get(i + 1) {
            None => return Err(format!("{x} needs a value")),
            Some(v) if v.starts_with("--") => return Err(format!("{x} needs a value, but the next word is the flag '{v}'")),
            Some(_) => {}
        }
        i += 2;
    }
    Ok(())
}

/// `fa macro premium-test ...` (the arguments after `macro`). Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> Result<i32, CmdError> {
    command_inner(repo, args)
}

fn command_inner(repo: &Path, args: &[String]) -> Result<i32, CmdError> {
    use CmdError::{Data, Usage};
    match args.first().map(String::as_str) {
        Some("premium-test") => {}
        Some("view") => return view::command(repo, &args[1..]),
        Some("-h" | "--help") | None => {
            println!("{USAGE}");
            return Ok(0);
        }
        Some(o) => return Err(Usage(format!("unknown macro command '{o}' (premium-test, view)"))),
    }
    let a = &args[1..];
    if a.iter().any(|x| x == "-h" || x == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    check_args(a).map_err(Usage)?;
    let Some(series) = flag(a, "--series") else { return Err(Usage("premium-test needs --series FILE".into())) };
    let path = |name: &str, default: &str| -> PathBuf {
        let p = PathBuf::from(flag(a, name).unwrap_or_else(|| default.into()));
        if p.is_absolute() {
            p
        } else {
            repo.join(p)
        }
    };
    let cache = path("--cache", "data/raw");
    let list = flag(a, "--list").unwrap_or_else(|| "sp500".into());
    if list != "sp500" && list != "sp100" {
        return Err(Usage(format!("bad --list '{list}' (sp500 or sp100)")));
    }
    let excl = match (flag(a, "--exclude-from"), flag(a, "--exclude-to")) {
        (None, None) => None,
        (Some(x), Some(y)) => {
            let (x, y) = (parse_date(&x).map_err(Usage)?, parse_date(&y).map_err(Usage)?);
            if x > y {
                return Err(Usage("--exclude-from is after --exclude-to".into()));
            }
            Some((x, y))
        }
        _ => return Err(Usage("--exclude-from and --exclude-to go together".into())),
    };
    let series_path = {
        let p = PathBuf::from(&series);
        if p.is_absolute() {
            p
        } else {
            std::env::current_dir().map_err(|e| Data(e.to_string()))?.join(p)
        }
    };
    let text = read(&series_path).map_err(Data)?;
    let mut optional_errors: Vec<String> = Vec::new();
    let mut optional = |name: &str, id: &str| -> Option<data::Daily> {
        let file = flag(a, name)?;
        match read(&PathBuf::from(&file)).and_then(|t| parse_fred(&t, id)) {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("note: {e}; the descriptive block is not run");
                optional_errors.push(format!("{name} {file}: {e}"));
                None
            }
        }
    };
    let (dfii, bei) = (optional("--dfii10", "DFII10"), optional("--t10yie", "T10YIE"));
    let spy = match read(&cache.join("SPY/prices.json")).and_then(|t| data::parse_spy_json(&t)) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("note: {e}; the SPY check is not run");
            None
        }
    };
    let ext = External {
        spy,
        vix: read(&cache.join("vix/VIX_History.csv")).and_then(|t| parse_vix(&t)).map_err(Data)?,
        dfii,
        bei,
    };
    let name = series_path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let mut res = run(&text, &ext, &list, &name, excl).map_err(Data)?;
    mark_optional_not_run(&mut res, &optional_errors);
    print!("{}", report_text(&res));
    if let Some(out) = flag(a, "--out") {
        let json = serde_json::to_string_pretty(&res).map_err(|e| Data(e.to_string()))?;
        std::fs::write(&out, json + "\n").map_err(|e| Data(format!("cannot write {out}: {e}")))?;
        println!("\nwrote {out}");
    }
    Ok(if res.primary.is_some() { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A series with empty s2 columns: the sanity check fails (n = 0), there is no primary result,
    /// the text says that the test stops, and the JSON has no primary.
    #[test]
    fn empty_s2_stops_the_test() {
        let text = "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change
2019-01-04,2019-01-04,gsw,x,0.0260,0.0210,,,0.01,,,,,,,,,
2019-01-11,2019-01-11,gsw,x,0.0255,0.0207,,,0.01,,,,,,,,,
";
        let e = parse_vix("DATE,OPEN,HIGH,LOW,CLOSE\n").unwrap();
        let ext = External { vix: e, dfii: None, bei: None, spy: None };
        let r = run(text, &ext, "sp500", "t.csv", None).unwrap();
        assert!(r.primary.is_none() && !r.sanity.passed && r.sanity.n == 0);
        assert!(report_text(&r).contains("sanity check failed: the bridge has a defect; the test stops"));
        assert!(serde_json::to_string(&r).unwrap().contains("\"primary\":null"));
    }

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// Item 2: unknown flag, flag without a value (last or before another flag), a stray word.
    #[test]
    fn flags_are_checked() {
        assert!(check_args(&args(&["--series", "a.csv", "--list", "sp100"])).is_ok());
        assert!(check_args(&args(&["--series", "a.csv", "--bogus", "x"])).unwrap_err().contains("unknown flag '--bogus'"));
        assert!(check_args(&args(&["--series", "a.csv", "--nfci", "n.csv"])).unwrap_err().contains("unknown flag"));
        assert!(check_args(&args(&["--series", "a.csv", "--out"])).unwrap_err().contains("--out needs a value"));
        assert!(check_args(&args(&["--series", "--out", "o.json"])).unwrap_err().contains("next word is the flag '--out'"));
        assert!(check_args(&args(&["--series", "a.csv", "stray"])).unwrap_err().contains("unexpected argument 'stray'"));
    }

    /// Item 4: a bad flag is a usage error; a missing file is a data error.
    #[test]
    fn usage_and_data_errors_differ() {
        let repo = std::env::temp_dir();
        let u = command(&repo, &args(&["premium-test", "--series", "a.csv", "--bogus", "1"])).unwrap_err();
        assert!(matches!(u, CmdError::Usage(_)), "{u:?}");
        let u = command(&repo, &args(&["premium-test"])).unwrap_err();
        assert!(matches!(u, CmdError::Usage(_)), "{u:?}");
        let d = command(&repo, &args(&["premium-test", "--series", "/no/such/file.csv"])).unwrap_err();
        assert!(matches!(d, CmdError::Data(ref m) if m.contains("cannot read")), "{d:?}");
    }

    /// Item 6: a bad optional file is named in the report next to its item; the main result stays.
    #[test]
    fn bad_optional_file_is_named_in_the_report() {
        let text = "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change
2019-01-04,2019-01-04,gsw,x,0.0260,0.0210,,,0.01,,,,,,,,,
";
        let ext = External { vix: parse_vix("DATE,OPEN,HIGH,LOW,CLOSE\n").unwrap(), dfii: None, bei: None, spy: None };
        let mut r = run(text, &ext, "sp500", "t.csv", None).unwrap();
        let err = parse_fred("bad,header\n", "DFII10").unwrap_err();
        mark_optional_not_run(&mut r, &[format!("--dfii10 d.csv: {err}")]);
        let note = r.real_bei_note.clone().unwrap();
        assert!(note.contains("NOT RUN: --dfii10 d.csv: DFII10 file: unexpected header 'bad,header'"), "{note}");
        assert!(note.contains("exit code stays 0"));
        assert!(serde_json::to_string(&r).unwrap().contains("NOT RUN: --dfii10"));
    }
}
