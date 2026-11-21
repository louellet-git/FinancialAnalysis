//! Rust interface to FinancialAnalysis: rates, bonds, inflation, the DCF verdict.
//!
//! The rates, the bonds and the verdict are Rust (`rates`, `bonds`, `verdict_rates`, `dcf`).
//! `Fa` keeps the old method names and calls them. No part of this crate starts Python.
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

pub mod dcf;
pub mod bonds;
pub mod curve;
pub mod prices;
pub mod recon;
pub mod fit;
pub mod richcheap;
pub mod history;
pub mod explain;
pub mod billdiag;
pub mod forwards;
pub mod rates;
pub mod rates_derived;
pub mod hqm;
pub mod refdata;
pub mod verdict_rates;

#[cfg(test)]
mod rates_tests;
pub mod futures;
pub mod ratemodels;
pub mod portfolio;
pub mod volmodels;
pub mod trading;
pub mod analyze;
pub mod company;
pub mod hostnet;
pub mod market;
pub mod sync;
pub mod report;
pub mod macro_test;
pub mod selftest;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Treasury tenors in curve order (a `BTreeMap` sorts them as text).
pub const TENORS: [&str; 14] = [
    "1M", "1.5M", "2M", "3M", "4M", "6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "20Y", "30Y",
];

// ---------------------------------------------------------------------------
// Types (the JSON fields are the same as in the earlier Python commands)
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

/// One rating row: step, cost of debt and spread over the UST 10Y, all decimals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatingQuote {
    /// The fixed step of the bucket over the HQM reference bucket.
    pub step: f64,
    /// UST 10Y + HQM basis + the step: the cost of debt of the row.
    #[serde(rename = "yield")]
    pub effective_yield: RateObservation,
    /// Cost of debt minus UST 10Y: the engine's `kd_spread` definition.
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
    pub hqm_curve: HqmCurve,
}

/// What `--rating` did to the cost of debt in a verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditContext {
    pub rating: String,
    /// The bucket used: the rating without notches ("A-" -> "A").
    pub index: String,
    pub kd_spread: RateObservation,
    pub model_kd_spread: f64,
    pub kd_spread_change_vs_model: f64,
    /// HQM spot yield at the default maturity (10 years), monthly average of the HQM month.
    pub hqm_spot: f64,
    /// Mean UST 10Y par yield of the same month.
    pub ust_mean_10y: f64,
    /// `hqm_spot - ust_mean_10y`.
    pub basis: f64,
    /// The step of the bucket over the HQM reference bucket.
    pub step: f64,
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
    /// A Rust data command failed (a source did not answer, a value is missing, a bad input).
    Data(String),
    /// The output was not the JSON shape this crate expects.
    Json { source: serde_json::Error, output: String },
}

impl fmt::Display for FaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FaError::Data(message) => write!(f, "{message}"),
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
            FaError::Json { source, .. } => Some(source),
            FaError::Data(_) => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, FaError>;

/// Parse JSON output into a type, keeping the raw text for the error message.
pub fn parse_output<T: DeserializeOwned>(output: &str) -> Result<T> {
    serde_json::from_str(output).map_err(|source| FaError::Json { source, output: output.to_string() })
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// The entry point of the typed Rust calls. `repo` is the repository root.
#[derive(Debug, Clone)]
pub struct Fa {
    pub repo: PathBuf,
}

impl Fa {
    pub fn new(repo: impl Into<PathBuf>) -> Self {
        Fa { repo: repo.into() }
    }

    /// `FA_REPO` (default: the repository that holds this crate).
    pub fn from_env() -> Self {
        let repo = std::env::var_os("FA_REPO")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(".."));
        Fa { repo }
    }

    /// The network, the cache `data/raw` of this repository, today from the clock.
    fn source(&self) -> rates::Rates {
        rates::Rates::live(&self.repo)
    }

    /// Treasury par yield on or before `date` ("YYYY-MM-DD", "today", or None for latest).
    pub fn treasury_yield(&self, tenor: &str, date: Option<&str>, real: bool) -> Result<RateObservation> {
        data(|| self.source().treasury_yield(tenor, rates::parse_on(date)?, real))
    }

    pub fn curve(&self, date: Option<&str>, real: bool) -> Result<Curve> {
        data(|| self.source().curve(rates::parse_on(date)?, real))
    }

    /// Daily Treasury series for one tenor: {date: rate}.
    pub fn history(&self, tenor: &str, start: &str, end: Option<&str>, real: bool) -> Result<BTreeMap<String, f64>> {
        data(|| {
            let (start, end) = (rates::parse_on(Some(start))?, rates::parse_on(end)?);
            let h = self.source().treasury_history(tenor, start, end, real)?;
            Ok(h.into_iter().map(|(d, v)| (mapcheck::dates::format(d), v)).collect())
        })
    }

    /// NY Fed reference rate: SOFR, EFFR, OBFR, TGCR or BGCR.
    pub fn nyfed(&self, rate: &str, date: Option<&str>) -> Result<RateObservation> {
        data(|| self.source().nyfed_rate(rate, rates::parse_on(date)?))
    }

    pub fn inflation(&self, date: Option<&str>) -> Result<Inflation> {
        data(|| self.source().inflation_snapshot(rates::parse_on(date)?))
    }

    pub fn snapshot(&self, date: Option<&str>) -> Result<Snapshot> {
        data(|| self.source().snapshot(rates::parse_on(date)?))
    }

    /// Any FRED series. `raw` keeps index levels (for example CPIAUCNS); otherwise percent -> decimal.
    pub fn fred(&self, series: &str, start: Option<&str>, raw: bool) -> Result<BTreeMap<String, f64>> {
        data(|| {
            let h = self.source().fred_series(series, rates::parse_on(start)?, !raw)?;
            Ok(h.into_iter().map(|(d, v)| (mapcheck::dates::format(d), v)).collect())
        })
    }

    /// Write T10Y3M, DFII10 and T10YIE (computed from the Treasury curves) as FRED-form CSV files into `dir`.
    /// Gives `(series, rows)`. `from_year` defaults to the first year of the nominal curves.
    pub fn rates_derived(&self, dir: &std::path::Path, from_year: Option<i64>, to_year: Option<i64>) -> Result<Vec<(String, usize)>> {
        data(|| {
            let mut r = self.source();
            let to = to_year.unwrap_or_else(|| curve::civil_from_days(r.today).0);
            r.write_derived(dir, from_year.unwrap_or(rates::treasury_first_year(false)), to)
        })
    }

    /// Cross-check every source against FRED over the last `days`.
    pub fn check(&self, days: u32) -> Result<Vec<CheckResult>> {
        data(|| self.source().check(days as i64))
    }

    /// Driver-engine verdict for a ticker with `analysis/<TICKER>/driver_model.toml`. With
    /// `rf_date` the Treasury 10Y is the `rf`; with `rating` the market spread of the rating is the
    /// `kd_spread` (it needs `rf_date`). All in Rust; the rates come from the network.
    pub fn verdict(&self, ticker: &str, price: f64, opts: &VerdictOptions) -> Result<Verdict> {
        let (simulations, seed) = (opts.simulations.unwrap_or(10_000), opts.seed.unwrap_or(7));
        let spec = dcf::model::ModelSpec::load(&self.repo, ticker)
            .map_err(|e| FaError::Data(e.0))?
            .ok_or_else(|| {
                let t = ticker.to_uppercase();
                FaError::Data(format!(
                    "No driver model for {t}: expected analysis/{t}/driver_model.toml. Models: {}.",
                    dcf::model::available_models(&self.repo).join(", ")
                ))
            })?;
        if opts.rf_date.is_none() && (opts.rf_base_date.is_some() || opts.rating.is_some()) {
            return Err(FaError::Data("--rf-base-date and --rating need --rf-date".into()));
        }
        let (mut spec, mut rates_ctx, mut credit_ctx) = (spec, None, None);
        if let Some(rf_date) = &opts.rf_date {
            let mut src = self.source();
            let (s, ctx) = data(|| verdict_rates::rates_context(&spec, &mut src, rf_date, opts.rf_base_date.as_deref()))?;
            spec = s;
            if let Some(rating) = &opts.rating {
                let (s, c) = data(|| verdict_rates::credit_context(&spec, &mut src, rating, rf_date, ctx.rf.value))?;
                spec = s;
                credit_ctx = Some(c);
            }
            rates_ctx = Some(ctx);
        }
        dcf::cli::verdict_for_spec(&spec, ticker, price, simulations, seed, rates_ctx, credit_ctx).map_err(|e| FaError::Data(e.0))
    }

    /// The step of the bucket of a rating ("A-", "Baa2", ...) over the HQM reference bucket.
    pub fn rating_step(&self, rating: &str) -> Result<RateObservation> {
        data(|| self.source().rating_step(rating))
    }

    /// The cost of debt of a rating: HQM 10Y spot + the step of its bucket.
    pub fn corporate_yield(&self, rating: &str, date: Option<&str>) -> Result<RateObservation> {
        data(|| self.source().corporate_yield(rating, rates::parse_on(date)?))
    }

    /// Cost of debt minus UST 10Y: the engine's `kd_spread`.
    pub fn spread_over_10y(&self, rating: &str, date: Option<&str>) -> Result<RateObservation> {
        data(|| self.source().spread_over_10y(rating, rates::parse_on(date)?))
    }

    pub fn hqm_curve(&self, date: Option<&str>) -> Result<HqmCurve> {
        data(|| self.source().hqm_curve(rates::parse_on(date)?))
    }

    pub fn credit_snapshot(&self, date: Option<&str>) -> Result<CreditSnapshot> {
        data(|| self.source().credit_snapshot(rates::parse_on(date)?))
    }
}

/// A Rust data call that gives a `String` error, as a `FaError`.
fn data<T>(f: impl FnOnce() -> std::result::Result<T, String>) -> Result<T> {
    f().map_err(FaError::Data)
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
    fn bad_json_reports_output() {
        let e = parse_output::<RateObservation>("not json").unwrap_err();
        assert!(e.to_string().contains("not json"));
    }

    #[test]
    fn parses_credit_snapshot_and_orders_hqm() {
        let q = format!(
            r#"{{"step": 0.0, "yield": {}, "spread_over_10y": {}}}"#,
            obs_json("A yield", 0.0577, ""),
            obs_json("A - 10Y", 0.0067, "")
        );
        let j = format!(
            r#"{{"ratings": {{"A": {q}}},
                "hqm_curve": {{"date": "2026-10-01", "source": "U.S. Treasury HQM",
                               "maturities": {{"10Y": 0.0602, "1Y": 0.048, "2Y": 0.0517, "30Y": 0.0681}}}}}}"#
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
                "kd_spread_change_vs_model": 0.0077, "hqm_spot": 0.0577, "ust_mean_10y": 0.0507, "basis": 0.007, "step": 0.0,
                "kd_pretax": 0.0574,
                "kd_after_tax": 0.045346, "tax_rate": 0.21}}"#,
            obs_json("A - 10Y", 0.0067, "A- uses the A step")
        );
        let v: Verdict = parse_output(&format!("{{{base}, \"credit\": {credit}}}")).unwrap();
        let c = v.credit.unwrap();
        assert_eq!((c.index.as_str(), c.kd_spread.value), ("A", 0.0067));
    }
}
