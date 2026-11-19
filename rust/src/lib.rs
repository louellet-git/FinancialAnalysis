//! Rust interface to the FinancialAnalysis Python pipeline.
//!
//! Python is the one implementation. This crate runs the Python entry points with
//! `--json` and turns their output into typed Rust values, so Rust code can call
//! everything without a second copy of the logic.
//!
//! All rates are decimals (0.0507 == 5.07%). All dates are ISO strings (YYYY-MM-DD).
//!
//! ```no_run
//! use financial_analysis::{Fa, VerdictOptions};
//! let fa = Fa::from_env();
//! let rf = fa.treasury_yield("10Y", None, false)?;          // U.S. Treasury 10Y par yield
//! let inflation = fa.inflation(None)?;                       // breakevens, 5y5y, CPI
//! let v = fa.verdict("IBM", 231.09, &VerdictOptions { rf_date: Some("today".into()), ..Default::default() })?;
//! println!("{} on {}: P50 ${:.2}, 5y5y {:.2}%", rf.series, rf.date, v.p50, inflation.forward_5y5y.value * 100.0);
//! # Ok::<(), financial_analysis::FaError>(())
//! ```

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Treasury tenors in curve order (a `BTreeMap` sorts them as text).
pub const TENORS: [&str; 14] = [
    "1M", "1.5M", "2M", "3M", "4M", "6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "20Y", "30Y",
];

// ---------------------------------------------------------------------------
// Types (mirror the Python JSON)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateObservation {
    pub series: String,
    /// The observation date, not the date asked for.
    pub date: String,
    /// Decimal: 0.0507 == 5.07%.
    pub value: f64,
    pub source: String,
    /// A caveat the caller must see, for example a data gap that moved the date.
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub date: String,
    pub source: String,
    #[serde(default)]
    pub real: bool,
    pub tenors: BTreeMap<String, f64>,
}

impl Curve {
    /// Tenors in curve order (1M first, 30Y last).
    pub fn ordered(&self) -> Vec<(&'static str, f64)> {
        TENORS
            .iter()
            .filter_map(|t| self.tenors.get(*t).map(|v| (*t, *v)))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inflation {
    pub breakeven_5y: RateObservation,
    pub breakeven_10y: RateObservation,
    /// Market-implied inflation for years 6-10: the anchor for terminal growth.
    pub forward_5y5y: RateObservation,
    pub real_10y: RateObservation,
    /// Realized 12-month CPI-U change (BLS). Read `note` for data gaps.
    pub cpi_12m: RateObservation,
    pub core_cpi_12m: RateObservation,
    /// Other breakevens (7Y, 20Y, 30Y) by key, for example "breakeven_30y".
    #[serde(flatten)]
    pub other: BTreeMap<String, RateObservation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FomcTarget {
    #[serde(rename = "from")]
    pub lower: Option<f64>,
    #[serde(rename = "to")]
    pub upper: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub nominal_curve: Curve,
    pub real_curve: Curve,
    pub sofr: RateObservation,
    pub effr: RateObservation,
    pub fomc_target: FomcTarget,
    pub inflation: Inflation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub check: String,
    pub fred_series: String,
    pub compared: u32,
    /// (date, ours, FRED) for each disagreement. Empty means the sources agree.
    pub mismatches: Vec<(String, f64, f64)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseSplit {
    pub date: String,
    pub nominal_10y: f64,
    pub nominal_change: f64,
    pub real_change: f64,
    pub breakeven_change: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatesContext {
    pub rf: RateObservation,
    pub model_rf_mean: f64,
    pub rf_change_vs_model: f64,
    pub real_10y: f64,
    pub breakeven_10y: f64,
    pub forward_5y5y: f64,
    pub cpi_12m: RateObservation,
    pub core_cpi_12m: RateObservation,
    pub terminal_growth_mean: f64,
    pub terminal_growth_real_vs_5y5y: f64,
    pub base: Option<BaseSplit>,
}

/// One rating index (ICE BofA): spread and yield, all decimals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatingQuote {
    /// Option-adjusted spread over the Treasury curve at each bond's maturity.
    pub oas: RateObservation,
    #[serde(rename = "yield")]
    pub effective_yield: RateObservation,
    /// Index yield minus UST 10Y: the engine's `kd_spread` definition.
    pub spread_over_10y: RateObservation,
}

/// U.S. Treasury HQM corporate spot curve (monthly average), maturities "1Y".."30Y".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HqmCurve {
    pub date: String,
    pub source: String,
    pub maturities: BTreeMap<String, f64>,
}

impl HqmCurve {
    /// Maturities in increasing order (a `BTreeMap` sorts "10Y" before "1Y").
    pub fn ordered(&self) -> Vec<(String, f64)> {
        let mut v: Vec<(String, f64)> = self.maturities.iter().map(|(k, x)| (k.clone(), *x)).collect();
        v.sort_by_key(|(k, _)| k.trim_end_matches('Y').parse::<u32>().unwrap_or(u32::MAX));
        v
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditSnapshot {
    /// Keyed by "AAA", "AA", "A", "BBB", "BB", "B", "CCC".
    pub ratings: BTreeMap<String, RatingQuote>,
    /// Keyed by "Aaa" and "Baa".
    pub moodys: BTreeMap<String, RateObservation>,
    pub hqm_curve: HqmCurve,
}

/// What `--rating` did to the cost of debt in a verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditContext {
    pub rating: String,
    /// The ICE index used: the rating without notches ("A-" -> "A").
    pub index: String,
    pub kd_spread: RateObservation,
    pub model_kd_spread: f64,
    pub kd_spread_change_vs_model: f64,
    pub index_yield: f64,
    pub oas: f64,
    pub moodys_aaa: f64,
    pub moodys_baa: f64,
    pub kd_pretax: f64,
    pub kd_after_tax: f64,
    pub tax_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verdict {
    pub ticker: String,
    pub simulations: u32,
    pub seed: i64,
    pub price: f64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub chance_price_too_high: f64,
    pub verdict: String,
    /// Present only when `rf_date` was given.
    pub rates: Option<RatesContext>,
    /// Present only when `rating` was given.
    #[serde(default)]
    pub credit: Option<CreditContext>,
}

#[derive(Debug, Clone, Default)]
pub struct VerdictOptions {
    pub simulations: Option<u32>,
    pub seed: Option<i64>,
    /// "YYYY-MM-DD" or "today": use the Treasury 10Y on that date as rf.
    pub rf_date: Option<String>,
    /// "YYYY-MM-DD": split the 10Y change since this date into real rate and inflation.
    pub rf_base_date: Option<String>,
    /// S&P ("A-") or Moody's ("A3") rating: use the market `kd_spread` for it. Needs `rf_date`.
    pub rating: Option<String>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum FaError {
    /// Python could not start (wrong FA_PYTHON?).
    Spawn { python: PathBuf, source: std::io::Error },
    /// The Python command failed. `kind` is the Python exception type when known.
    Python { code: Option<i32>, kind: Option<String>, message: String },
    /// The output was not the JSON shape this crate expects.
    Json { source: serde_json::Error, output: String },
}

impl fmt::Display for FaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FaError::Spawn { python, source } => write!(f, "cannot start {}: {source}", python.display()),
            FaError::Python { code, kind, message } => {
                write!(f, "python failed (exit {code:?})")?;
                if let Some(k) = kind {
                    write!(f, " {k}")?;
                }
                write!(f, ": {message}")
            }
            FaError::Json { source, output } => {
                let head: String = output.chars().take(200).collect();
                write!(f, "unexpected JSON ({source}): {head}")
            }
        }
    }
}

impl std::error::Error for FaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FaError::Spawn { source, .. } => Some(source),
            FaError::Json { source, .. } => Some(source),
            FaError::Python { .. } => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, FaError>;

/// Turn a failed run's stderr into an error. The Python CLIs print
/// `{"error": ..., "type": ...}` on stderr in `--json` mode.
pub fn python_error(code: Option<i32>, stderr: &str) -> FaError {
    #[derive(Deserialize)]
    struct PyErr {
        error: String,
        #[serde(rename = "type")]
        kind: Option<String>,
    }
    let last = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
    match serde_json::from_str::<PyErr>(last) {
        Ok(e) => FaError::Python { code, kind: e.kind, message: e.error },
        Err(_) => FaError::Python { code, kind: None, message: stderr.trim().to_string() },
    }
}

/// Parse JSON output into a type, keeping the raw text for the error message.
pub fn parse_output<T: DeserializeOwned>(output: &str) -> Result<T> {
    serde_json::from_str(output).map_err(|source| FaError::Json { source, output: output.to_string() })
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Runs the Python pipeline. `repo` is the repository root, `python` the interpreter.
#[derive(Debug, Clone)]
pub struct Fa {
    pub python: PathBuf,
    pub repo: PathBuf,
}

impl Fa {
    pub fn new(python: impl Into<PathBuf>, repo: impl Into<PathBuf>) -> Self {
        Fa { python: python.into(), repo: repo.into() }
    }

    /// `FA_REPO` (default: the repository that holds this crate) and `FA_PYTHON`
    /// (default: see [`default_python`]).
    pub fn from_env() -> Self {
        let repo = std::env::var_os("FA_REPO")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(".."));
        let python = std::env::var_os("FA_PYTHON")
            .map(PathBuf::from)
            .unwrap_or_else(|| default_python(&repo, cfg!(windows), |p| p.exists()));
        Fa { python, repo }
    }

    fn command(&self, module: &str, args: &[String]) -> Command {
        let mut cmd = Command::new(&self.python);
        cmd.current_dir(&self.repo).arg("-m").arg(module).args(args);
        cmd
    }

    /// `python -m <module> <args> --json`, parsed into `T`.
    pub fn call<T: DeserializeOwned>(&self, module: &str, args: &[String]) -> Result<T> {
        let mut args = args.to_vec();
        args.push("--json".into());
        let out = self
            .command(module, &args)
            .output()
            .map_err(|source| FaError::Spawn { python: self.python.clone(), source })?;
        if !out.status.success() {
            return Err(python_error(out.status.code(), &String::from_utf8_lossy(&out.stderr)));
        }
        parse_output(&String::from_utf8_lossy(&out.stdout))
    }

    /// Run any repository entry point (for example `scripts.analyze IBM`) with its
    /// output passed through to this process. This makes every Python command callable.
    pub fn run(&self, module: &str, args: &[String]) -> Result<ExitStatus> {
        self.command(module, args)
            .status()
            .map_err(|source| FaError::Spawn { python: self.python.clone(), source })
    }

    fn rates<T: DeserializeOwned>(&self, args: Vec<String>) -> Result<T> {
        self.call("financial_analysis.rates", &args)
    }

    /// Treasury par yield on or before `date` ("YYYY-MM-DD", "today", or None for latest).
    pub fn treasury_yield(&self, tenor: &str, date: Option<&str>, real: bool) -> Result<RateObservation> {
        let mut a = vec!["yield".into(), "--tenor".into(), tenor.into()];
        push_date(&mut a, date);
        if real {
            a.push("--real".into());
        }
        self.rates(a)
    }

    pub fn curve(&self, date: Option<&str>, real: bool) -> Result<Curve> {
        let mut a = vec!["curve".into()];
        push_date(&mut a, date);
        if real {
            a.push("--real".into());
        }
        self.rates(a)
    }

    /// Daily Treasury series for one tenor: {date: rate}.
    pub fn history(&self, tenor: &str, start: &str, end: Option<&str>, real: bool) -> Result<BTreeMap<String, f64>> {
        let mut a = vec!["history".into(), "--tenor".into(), tenor.into(), "--start".into(), start.into()];
        if let Some(e) = end {
            a.extend(["--end".into(), e.into()]);
        }
        if real {
            a.push("--real".into());
        }
        self.rates(a)
    }

    /// NY Fed reference rate: SOFR, EFFR, OBFR, TGCR or BGCR.
    pub fn nyfed(&self, rate: &str, date: Option<&str>) -> Result<RateObservation> {
        let mut a = vec!["nyfed".into(), "--rate".into(), rate.into()];
        push_date(&mut a, date);
        self.rates(a)
    }

    pub fn inflation(&self, date: Option<&str>) -> Result<Inflation> {
        let mut a = vec!["inflation".into()];
        push_date(&mut a, date);
        self.rates(a)
    }

    pub fn snapshot(&self, date: Option<&str>) -> Result<Snapshot> {
        let mut a = vec!["snapshot".into()];
        push_date(&mut a, date);
        self.rates(a)
    }

    /// Any FRED series. `raw` keeps index levels (for example CPIAUCNS); otherwise percent -> decimal.
    pub fn fred(&self, series: &str, start: Option<&str>, raw: bool) -> Result<BTreeMap<String, f64>> {
        let mut a = vec!["fred".into(), "--series".into(), series.into()];
        if let Some(s) = start {
            a.extend(["--start".into(), s.into()]);
        }
        if raw {
            a.push("--raw".into());
        }
        self.rates(a)
    }

    /// Cross-check every source against FRED over the last `days`.
    pub fn check(&self, days: u32) -> Result<Vec<CheckResult>> {
        self.rates(vec!["check".into(), "--days".into(), days.to_string()])
    }

    /// Driver-engine verdict for a ticker with `analysis/<TICKER>/driver_model.py`.
    pub fn verdict(&self, ticker: &str, price: f64, opts: &VerdictOptions) -> Result<Verdict> {
        self.call("scripts.verdict", &verdict_args(ticker, price, opts))
    }

    fn bonds<T: DeserializeOwned>(&self, args: Vec<String>) -> Result<T> {
        self.call("financial_analysis.bonds", &args)
    }

    fn rating_cmd<T: DeserializeOwned>(&self, cmd: &str, rating: &str, date: Option<&str>) -> Result<T> {
        let mut a = vec![cmd.into(), "--rating".into(), rating.into()];
        push_date(&mut a, date);
        self.bonds(a)
    }

    /// ICE BofA option-adjusted spread for a rating ("A-", "Baa2", ...).
    pub fn credit_spread(&self, rating: &str, date: Option<&str>) -> Result<RateObservation> {
        self.rating_cmd("spread", rating, date)
    }

    /// ICE BofA effective yield for a rating.
    pub fn corporate_yield(&self, rating: &str, date: Option<&str>) -> Result<RateObservation> {
        self.rating_cmd("yield", rating, date)
    }

    /// Index yield minus UST 10Y on the same date: the engine's `kd_spread`.
    pub fn spread_over_10y(&self, rating: &str, date: Option<&str>) -> Result<RateObservation> {
        self.rating_cmd("over10y", rating, date)
    }

    /// Moody's seasoned corporate yield: grade "Aaa" or "Baa".
    pub fn moodys(&self, grade: &str, date: Option<&str>) -> Result<RateObservation> {
        let mut a = vec!["moodys".into(), "--grade".into(), grade.into()];
        push_date(&mut a, date);
        self.bonds(a)
    }

    pub fn hqm_curve(&self, date: Option<&str>) -> Result<HqmCurve> {
        let mut a = vec!["hqm".into()];
        push_date(&mut a, date);
        self.bonds(a)
    }

    pub fn credit_snapshot(&self, date: Option<&str>) -> Result<CreditSnapshot> {
        let mut a = vec!["snapshot".into()];
        push_date(&mut a, date);
        self.bonds(a)
    }

    /// Moody's minus UST 10Y vs FRED AAA10Y / BAA10Y over the last `days`.
    pub fn check_bonds(&self, days: u32) -> Result<Vec<CheckResult>> {
        self.bonds(vec!["check".into(), "--days".into(), days.to_string()])
    }
}

/// The interpreter to use when `FA_PYTHON` is not set. Linux and macOS:
/// `<repo>/.venv/bin/python` if it exists, else `python3`. Windows:
/// `<repo>\.venv\Scripts\python.exe` if it exists, else `python`.
pub fn default_python(repo: &Path, windows: bool, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let (venv, fallback) = if windows {
        (repo.join(".venv").join("Scripts").join("python.exe"), "python")
    } else {
        (repo.join(".venv").join("bin").join("python"), "python3")
    };
    if exists(&venv) { venv } else { PathBuf::from(fallback) }
}

fn push_date(a: &mut Vec<String>, date: Option<&str>) {
    if let Some(d) = date {
        a.extend(["--date".into(), d.into()]);
    }
}

/// Arguments for `scripts.verdict` (without `--json`).
pub fn verdict_args(ticker: &str, price: f64, opts: &VerdictOptions) -> Vec<String> {
    let mut a = vec![ticker.to_string(), "--price".into(), price.to_string()];
    if let Some(n) = opts.simulations {
        a.extend(["--simulations".into(), n.to_string()]);
    }
    if let Some(s) = opts.seed {
        a.extend(["--seed".into(), s.to_string()]);
    }
    if let Some(d) = &opts.rf_date {
        a.extend(["--rf-date".into(), d.clone()]);
    }
    if let Some(d) = &opts.rf_base_date {
        a.extend(["--rf-base-date".into(), d.clone()]);
    }
    if let Some(r) = &opts.rating {
        a.extend(["--rating".into(), r.clone()]);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBS: &str = r#"{"series": "UST 10Y", "date": "2026-11-18", "value": 0.0507, "source": "U.S. Treasury", "note": ""}"#;

    fn obs_json(series: &str, value: f64, note: &str) -> String {
        format!(r#"{{"series": "{series}", "date": "2026-11-18", "value": {value}, "source": "s", "note": "{note}"}}"#)
    }

    #[test]
    fn parses_rate_observation() {
        let o: RateObservation = parse_output(OBS).unwrap();
        assert_eq!((o.series.as_str(), o.date.as_str(), o.value), ("UST 10Y", "2026-11-18", 0.0507));
    }

    #[test]
    fn note_defaults_to_empty() {
        let o: RateObservation =
            parse_output(r#"{"series": "SOFR", "date": "2026-11-17", "value": 0.0392, "source": "NY Fed"}"#).unwrap();
        assert_eq!(o.note, "");
    }

    #[test]
    fn curve_orders_tenors() {
        let c: Curve = parse_output(
            r#"{"date": "2026-11-18", "source": "U.S. Treasury", "tenors": {"10Y": 0.0507, "1M": 0.041, "2Y": 0.048}}"#,
        )
        .unwrap();
        assert_eq!(c.ordered(), vec![("1M", 0.041), ("2Y", 0.048), ("10Y", 0.0507)]);
        assert!(!c.real);
    }

    #[test]
    fn inflation_keeps_extra_breakevens() {
        let j = format!(
            r#"{{"breakeven_5y": {}, "breakeven_7y": {}, "breakeven_10y": {}, "breakeven_30y": {},
                "forward_5y5y": {}, "real_10y": {}, "cpi_12m": {}, "core_cpi_12m": {}}}"#,
            obs_json("be5", 0.0223, ""),
            obs_json("be7", 0.022, ""),
            obs_json("be10", 0.0219, ""),
            obs_json("be30", 0.0216, ""),
            obs_json("fwd", 0.0215, ""),
            obs_json("real", 0.0288, ""),
            obs_json("cpi", 0.0374, "gap"),
            obs_json("core", 0.0256, "gap"),
        );
        let i: Inflation = parse_output(&j).unwrap();
        assert_eq!(i.forward_5y5y.value, 0.0215);
        assert_eq!(i.cpi_12m.note, "gap");
        assert_eq!(i.other.keys().collect::<Vec<_>>(), vec!["breakeven_30y", "breakeven_7y"]);
    }

    #[test]
    fn parses_verdict_with_and_without_rates() {
        let base = r#""ticker": "IBM", "simulations": 10000, "seed": 7, "price": 231.09,
            "p10": 73.46, "p25": 99.43, "p50": 133.9, "p75": 175.86, "p90": 224.81,
            "chance_price_too_high": 0.911, "verdict": "Strongly overpriced""#;
        let v: Verdict = parse_output(&format!("{{{base}, \"rates\": null}}")).unwrap();
        assert!(v.rates.is_none());
        let rates = format!(
            r#"{{"rf": {OBS}, "model_rf_mean": 0.046, "rf_change_vs_model": 0.0047, "real_10y": 0.0288,
                "breakeven_10y": 0.0219, "forward_5y5y": 0.0215, "cpi_12m": {c}, "core_cpi_12m": {c},
                "terminal_growth_mean": 0.0225, "terminal_growth_real_vs_5y5y": 0.001,
                "base": {{"date": "2026-07-16", "nominal_10y": 0.0457, "nominal_change": 0.005,
                          "real_change": 0.0053, "breakeven_change": -0.0003}}}}"#,
            c = obs_json("cpi", 0.0374, "")
        );
        let v: Verdict = parse_output(&format!("{{{base}, \"rates\": {rates}}}")).unwrap();
        let r = v.rates.unwrap();
        assert_eq!(r.rf.value, 0.0507);
        assert_eq!(r.base.unwrap().breakeven_change, -0.0003);
    }

    #[test]
    fn parses_check_results() {
        let c: Vec<CheckResult> = parse_output(
            r#"[{"check": "UST 10Y", "fred_series": "DGS10", "compared": 21, "mismatches": []},
                {"check": "CPI", "fred_series": "CPIAUCNS", "compared": 2, "mismatches": [["2026-09-01", 336.94, 336.9]]}]"#,
        )
        .unwrap();
        assert!(c[0].mismatches.is_empty());
        assert_eq!(c[1].mismatches[0].0, "2026-09-01");
    }

    #[test]
    fn python_json_error_is_typed() {
        let e = python_error(Some(2), "some warning\n{\"error\": \"no SOFR\", \"type\": \"LookupError\"}\n");
        match e {
            FaError::Python { code, kind, message } => {
                assert_eq!((code, kind.as_deref(), message.as_str()), (Some(2), Some("LookupError"), "no SOFR"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn python_plain_error_keeps_text() {
        let e = python_error(Some(1), "Traceback ...\nValueError: boom\n");
        assert!(e.to_string().contains("ValueError: boom"));
    }

    #[test]
    fn bad_json_reports_output() {
        let e = parse_output::<RateObservation>("not json").unwrap_err();
        assert!(e.to_string().contains("not json"));
    }

    #[test]
    fn parses_credit_snapshot_and_orders_hqm() {
        let q = format!(
            r#"{{"oas": {}, "yield": {}, "spread_over_10y": {}}}"#,
            obs_json("A OAS", 0.0076, ""),
            obs_json("A yield", 0.0577, ""),
            obs_json("A - 10Y", 0.0067, "")
        );
        let j = format!(
            r#"{{"ratings": {{"A": {q}}}, "moodys": {{"Baa": {}}},
                "hqm_curve": {{"date": "2026-10-01", "source": "U.S. Treasury HQM via FRED",
                               "maturities": {{"10Y": 0.0602, "1Y": 0.048, "2Y": 0.0517, "30Y": 0.0681}}}}}}"#,
            obs_json("Baa", 0.0663, "")
        );
        let s: CreditSnapshot = parse_output(&j).unwrap();
        assert_eq!(s.ratings["A"].effective_yield.value, 0.0577);
        assert_eq!(s.ratings["A"].spread_over_10y.value, 0.0067);
        let order: Vec<String> = s.hqm_curve.ordered().into_iter().map(|(k, _)| k).collect();
        assert_eq!(order, vec!["1Y", "2Y", "10Y", "30Y"]);
    }

    #[test]
    fn parses_verdict_credit_and_old_output_without_it() {
        let base = r#""ticker": "IBM", "simulations": 10000, "seed": 7, "price": 231.09,
            "p10": 69.91, "p25": 94.69, "p50": 127.71, "p75": 167.72, "p90": 214.08,
            "chance_price_too_high": 0.932, "verdict": "Strongly overpriced", "rates": null"#;
        let v: Verdict = parse_output(&format!("{{{base}}}")).unwrap();
        assert!(v.credit.is_none());
        let credit = format!(
            r#"{{"rating": "A-", "index": "A", "kd_spread": {}, "model_kd_spread": -0.001,
                "kd_spread_change_vs_model": 0.0077, "index_yield": 0.0577, "oas": 0.0076,
                "moodys_aaa": 0.0618, "moodys_baa": 0.0663, "kd_pretax": 0.0574,
                "kd_after_tax": 0.045346, "tax_rate": 0.21}}"#,
            obs_json("A - 10Y", 0.0067, "A- uses the A index")
        );
        let v: Verdict = parse_output(&format!("{{{base}, \"credit\": {credit}}}")).unwrap();
        let c = v.credit.unwrap();
        assert_eq!((c.index.as_str(), c.kd_spread.value), ("A", 0.0067));
    }

    #[test]
    fn default_python_for_each_platform() {
        let repo = Path::new("repo");
        assert_eq!(default_python(repo, false, |_| false), PathBuf::from("python3"));
        assert_eq!(default_python(repo, true, |_| false), PathBuf::from("python"));
        assert_eq!(default_python(repo, false, |_| true), repo.join(".venv").join("bin").join("python"));
        assert_eq!(default_python(repo, true, |_| true), repo.join(".venv").join("Scripts").join("python.exe"));
    }

    #[test]
    fn verdict_args_include_options() {
        let a = verdict_args(
            "IBM",
            231.09,
            &VerdictOptions {
                seed: Some(7),
                rf_date: Some("today".into()),
                rating: Some("A-".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            a,
            vec!["IBM", "--price", "231.09", "--seed", "7", "--rf-date", "today", "--rating", "A-"]
        );
    }
}
