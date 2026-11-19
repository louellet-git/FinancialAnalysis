//! The path engine as data: processes, parameters and the model file of `engine = "paths"`.
//!
//! A model file of the path engine has these parts (each part is a TOML table):
//!
//! - `[[segments]]`: base revenue, an optional pin of the year-1 growth, and the name of the
//!   growth process of the segment.
//! - `[process.<name>]`: a process with a `role`. The roles are `growth` (one for each segment),
//!   `margin` (one), `sbc` (one), `charge` (at most one: other costs, for example M&A) and
//!   `s2c` (the sales-to-capital ratio, only for `reinvestment.mode = "sales_to_capital"`).
//! - `[reinvestment]`, `[tax]`, `[discount]`, `[rates]`, `[terminal]`, `[failure]`.
//! - `[[corr]]`: correlations of the parameters that are drawn one time for each path.
//! - `[[shock_corr]]`: correlations of the yearly shocks of the processes (and of `rate`).
//!
//! A number that can be uncertain is a `Param`: a plain number (fixed) or a table
//! `{ mean, std, lo, hi }` (drawn one time for each path: uncertainty of the parameter,
//! normal, cut at `lo` and `hi`). The name of a parameter is its TOML path, for example
//! `process.growth.target` or `discount.erp`. The names are used in `[[corr]]`, in the
//! command options `--pset`, and in the attribution table.

use super::driver::Ramp;
use super::DcfError;
use serde::Deserialize;
use std::collections::BTreeMap;

fn err<T>(m: impl Into<String>) -> Result<T, DcfError> {
    Err(DcfError(m.into()))
}

/// A fixed number or a drawn number.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ParamSpec {
    Fixed(f64),
    Draw(DrawSpec),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawSpec {
    pub mean: f64,
    #[serde(default)]
    pub std: f64,
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub hi: Option<f64>,
}

impl ParamSpec {
    pub fn fixed(v: f64) -> ParamSpec {
        ParamSpec::Fixed(v)
    }
    pub fn draw(mean: f64, std: f64) -> ParamSpec {
        ParamSpec::Draw(DrawSpec { mean, std, lo: None, hi: None })
    }
    pub fn mean(&self) -> f64 {
        match self {
            ParamSpec::Fixed(v) => *v,
            ParamSpec::Draw(d) => d.mean,
        }
    }
    pub fn std(&self) -> f64 {
        match self {
            ParamSpec::Fixed(_) => 0.0,
            ParamSpec::Draw(d) => d.std,
        }
    }
    pub fn bounds(&self) -> (Option<f64>, Option<f64>) {
        match self {
            ParamSpec::Fixed(_) => (None, None),
            ParamSpec::Draw(d) => (d.lo, d.hi),
        }
    }
}

/// The start value of a process: a number, a drawn number, or the text "target" (the start is
/// the target of the same path, so the center of the process is flat).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum StartSpec {
    Tied(String),
    Value(ParamSpec),
}

fn default_shape() -> String {
    "ou".to_string()
}

/// One process of the model. The value of year `t` is `center(t) + u(t) + drift(t)`, cut at
/// `lo` and `hi`.
///
/// - Center, shape `ou`: `target + (start - target) * phi^(t-1)`, with
///   `phi = exp(-ln 2 * dt / half_life)` (`half_life = inf` gives `phi = 1`: no reversion).
/// - Center, shape `ramp`: linear from `start` in year 1 to `target` in year `ramp_years`, then flat.
/// - Deviation: `u(1) = 0`, `u(t) = rho * u(t-1) + sigma * e(t)` with `e` a standard normal shock
///   of the channel of this process. `rho` is `persistence`; the default is `phi` for `ou`
///   (then `x(t) = target + phi (x(t-1) - target) + sigma e(t)` exactly) and 0 for `ramp`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessSpec {
    pub role: String,
    pub start: StartSpec,
    pub target: ParamSpec,
    #[serde(default = "default_shape")]
    pub shape: String,
    /// Years. A fixed number or a draw. Needed for the shape `ou`.
    #[serde(default)]
    pub half_life: Option<ParamSpec>,
    /// Years. Needed for the shape `ramp`.
    #[serde(default)]
    pub ramp_years: Option<usize>,
    /// Standard deviation of the yearly shock (in the unit of the process). 0: no shock.
    #[serde(default)]
    pub sigma: f64,
    /// With a value above 0 the shock size of a year is `max(sigma, sigma_level * |center|)`: the shock is
    /// proportional to the level of the slow part (growth is more uncertain when it is high); `sigma` is the floor.
    #[serde(default)]
    pub sigma_level: f64,
    #[serde(default)]
    pub persistence: Option<f64>,
    /// Bounds of the value of each year.
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub hi: Option<f64>,
    /// A schedule that is added to the value (for example the mix drift of the IBM margin).
    #[serde(default)]
    pub drift: Option<Ramp>,
    #[serde(default)]
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathSegment {
    pub name: String,
    pub base_revenue: f64,
    /// Name of the process (role `growth`).
    pub process: String,
    /// Pin of the growth of year 1. Without it, year 1 uses the value of the process.
    #[serde(default)]
    pub year1_growth: Option<f64>,
}

fn default_mode() -> String {
    "margin".to_string()
}

/// `margin`: the margin is the FCFF margin (after reinvestment), as in the old engine.
/// `sales_to_capital`: the margin is the operating margin before tax; the reinvestment is
/// `max(revenue(t) - revenue(t-1), 0) / s2c(t)`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReinvestSpec {
    #[serde(default = "default_mode")]
    pub mode: String,
}

impl Default for ReinvestSpec {
    fn default() -> Self {
        ReinvestSpec { mode: default_mode() }
    }
}

fn default_one() -> f64 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxSpec {
    pub rate: f64,
    /// Loss carry-forward at the start of year 1 (dollars). Used by the mode `sales_to_capital`.
    #[serde(default)]
    pub opening_loss: f64,
    /// A year can use at most this share of its positive operating income for the carry-forward.
    #[serde(default = "default_one")]
    pub loss_limit_share: f64,
}

impl Default for TaxSpec {
    fn default() -> Self {
        TaxSpec { rate: 0.21, opening_loss: 0.0, loss_limit_share: 1.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscountSpec {
    pub beta: ParamSpec,
    pub erp: ParamSpec,
    #[serde(default)]
    pub kd_spread: f64,
    #[serde(default = "default_one")]
    pub equity_weight: f64,
}

fn default_rate_model() -> String {
    "fixed".to_string()
}
fn default_tenor() -> f64 {
    10.0
}
fn default_rf_mode() -> String {
    "long".to_string()
}

/// The six numbers of one day of the GSW file, as published (`beta` in percent, `tau` in years).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurveSpec {
    #[serde(default)]
    pub date: String,
    pub beta0: f64,
    pub beta1: f64,
    pub beta2: f64,
    #[serde(default)]
    pub beta3: f64,
    pub tau1: f64,
    #[serde(default = "default_one")]
    pub tau2: f64,
}

/// The rate process.
///
/// - `fixed`: the risk-free rate is the number `rf` (a `Param`) in every year.
/// - `hullwhite`: Hull-White with the drift from the curve (`curve`), `a`, `sigma`.
/// - `vasicek`, `cir`: `kappa`, `theta`, `sigma`, `r0` (default `theta`).
///
/// The risk-free rate of the CAPM in year `t` is, with `mode = "long"`, the zero-coupon yield
/// of maturity `tenor` years that the model gives at the start of year `t` for the state of the
/// path; with `mode = "long_flat"` it is the same yield with its expected value fixed at the level of
/// year 1 (only the deviations of the model: the shape of the curve is not used); with `mode = "short"` it is the short rate of that time plus `spread`. `shift` is a
/// parameter that is added to the rate in each year and path (uncertainty of the level).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RatesSpec {
    #[serde(default = "default_rate_model")]
    pub model: String,
    #[serde(default)]
    pub rf: Option<ParamSpec>,
    #[serde(default = "default_tenor")]
    pub tenor: f64,
    #[serde(default = "default_rf_mode")]
    pub mode: String,
    #[serde(default)]
    pub spread: f64,
    #[serde(default)]
    pub a: Option<f64>,
    #[serde(default)]
    pub kappa: Option<f64>,
    #[serde(default)]
    pub theta: Option<f64>,
    #[serde(default)]
    pub sigma: Option<f64>,
    #[serde(default)]
    pub r0: Option<f64>,
    #[serde(default)]
    pub curve: Option<CurveSpec>,
    #[serde(default)]
    pub shift: Option<ParamSpec>,
}

impl Default for RatesSpec {
    fn default() -> Self {
        RatesSpec {
            model: default_rate_model(),
            rf: None,
            tenor: default_tenor(),
            mode: default_rf_mode(),
            spread: 0.0,
            a: None,
            kappa: None,
            theta: None,
            sigma: None,
            r0: None,
            curve: None,
            shift: None,
        }
    }
}

/// Terminal value. `growth` is the long-run growth after the last year. The caps, in this order:
/// the target of the process `cap_by_process` (growth cannot be above the long-run growth of
/// the revenue process), the risk-free rate of the path at the end of the horizon plus
/// `rf_margin`, and the guard of the old engine (`growth <= terminal discount rate - 2%`, T-DCF-2).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSpec {
    pub growth: ParamSpec,
    #[serde(default)]
    pub cap_by_process: Option<String>,
    #[serde(default)]
    pub rf_margin: Option<f64>,
    /// Return on new capital after the horizon (mode `sales_to_capital` only).
    #[serde(default)]
    pub roic: Option<StartSpec>,
}

fn default_failure_mode() -> String {
    "sampled".to_string()
}

/// A failure hazard for each year. At failure in year `t` the cash flows of year `t` and later
/// are lost, and the company is sold at `recovery_revenue_multiple * revenue(t-1)` at the end
/// of year `t`. `mode = "expected"` weights each year by the survival probability instead of a
/// draw (no sampling noise; used in the tests).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailureSpec {
    pub hazard: ParamSpec,
    #[serde(default)]
    pub recovery_revenue_multiple: f64,
    #[serde(default = "default_failure_mode")]
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrSpec {
    pub a: String,
    pub b: String,
    pub rho: f64,
    #[serde(default)]
    pub note: String,
}

fn default_engine() -> String {
    "paths".to_string()
}
fn default_years() -> usize {
    10
}

/// The content of a model file of the path engine.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathsSpec {
    #[serde(default = "default_engine")]
    pub engine: String,
    #[serde(default)]
    pub ticker: String,
    #[serde(default)]
    pub notes: String,
    pub net_debt: f64,
    pub shares: f64,
    #[serde(default = "default_years")]
    pub projection_years: usize,
    pub segments: Vec<PathSegment>,
    pub process: BTreeMap<String, ProcessSpec>,
    #[serde(default)]
    pub reinvestment: ReinvestSpec,
    #[serde(default)]
    pub tax: TaxSpec,
    pub discount: DiscountSpec,
    #[serde(default)]
    pub rates: RatesSpec,
    pub terminal: TerminalSpec,
    #[serde(default)]
    pub failure: Option<FailureSpec>,
    #[serde(default)]
    pub corr: Vec<CorrSpec>,
    #[serde(default)]
    pub shock_corr: Vec<CorrSpec>,
    /// Segment revenue by fiscal year (record only).
    #[serde(default)]
    pub segment_history: BTreeMap<String, BTreeMap<String, f64>>,
}

/// True when the text of a model file selects the path engine (`engine = "paths"`).
pub fn is_paths_model(text: &str) -> bool {
    text.parse::<toml::Table>().ok().and_then(|t| t.get("engine").and_then(|v| v.as_str()).map(|s| s == "paths")).unwrap_or(false)
}

impl PathsSpec {
    pub fn from_toml(text: &str) -> Result<PathsSpec, DcfError> {
        let spec: PathsSpec = toml::from_str(text).map_err(|e| DcfError(format!("paths model file: {e}")))?;
        if spec.engine != "paths" {
            return err(format!("paths model file: engine must be \"paths\", got {:?}", spec.engine));
        }
        Ok(spec)
    }

    /// The text with changed numbers. Each change is `(path, value)` with a dotted TOML path.
    /// A path that leads to a table with a `mean` changes the mean. Nothing else is changed.
    pub fn from_toml_with(text: &str, sets: &[(String, f64)], shift_bounds: bool) -> Result<PathsSpec, DcfError> {
        let mut root: toml::Value = text.parse().map_err(|e| DcfError(format!("paths model file: {e}")))?;
        for (path, v) in sets {
            set_path(&mut root, path, *v, shift_bounds)?;
        }
        let spec: PathsSpec = root.try_into().map_err(|e| DcfError(format!("paths model file: {e}")))?;
        if spec.engine != "paths" {
            return err(format!("paths model file: engine must be \"paths\", got {:?}", spec.engine));
        }
        Ok(spec)
    }
}

fn set_path(root: &mut toml::Value, path: &str, v: f64, shift_bounds: bool) -> Result<(), DcfError> {
    let mut cur = root;
    let parts: Vec<&str> = path.split('.').collect();
    for (i, key) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let table = cur.as_table_mut().ok_or_else(|| DcfError(format!("--pset {path}: '{}' is not a table", parts[..i].join("."))))?;
        let next = table.get_mut(*key).ok_or_else(|| DcfError(format!("--pset {path}: no key '{key}'")))?;
        if last {
            match next {
                toml::Value::Table(t) => {
                    if !t.contains_key("mean") {
                        return err(format!("--pset {path}: the table has no 'mean'"));
                    }
                    let old = t["mean"].as_float().or_else(|| t["mean"].as_integer().map(|i| i as f64)).unwrap_or(v);
                    if shift_bounds {
                        for key in ["lo", "hi"] {
                            if let Some(b) = t.get(key).and_then(|b| b.as_float().or_else(|| b.as_integer().map(|i| i as f64))) {
                                t.insert(key.into(), toml::Value::Float(b + (v - old)));
                            }
                        }
                    }
                    t.insert("mean".into(), toml::Value::Float(v));
                }
                toml::Value::Float(_) | toml::Value::Integer(_) => *next = toml::Value::Float(v),
                _ => return err(format!("--pset {path}: the value is not a number")),
            }
            return Ok(());
        }
        cur = next;
    }
    err(format!("--pset {path}: empty path"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_spec_reads_a_number_and_a_table() {
        #[derive(Deserialize)]
        struct T {
            a: ParamSpec,
            b: ParamSpec,
            c: StartSpec,
        }
        let t: T = toml::from_str("a = 2\nb = { mean = 0.1, std = 0.02, lo = 0.0 }\nc = \"target\"").unwrap();
        assert_eq!(t.a, ParamSpec::Fixed(2.0));
        assert_eq!(t.b, ParamSpec::Draw(DrawSpec { mean: 0.1, std: 0.02, lo: Some(0.0), hi: None }));
        assert_eq!(t.c, StartSpec::Tied("target".into()));
    }

    #[test]
    fn unknown_keys_are_errors() {
        let e = toml::from_str::<ProcessSpec>("role = \"growth\"\nstart = 0.1\ntarget = 0.1\nhalflife = 2.0").unwrap_err();
        assert!(e.to_string().contains("halflife"), "{e}");
    }

    #[test]
    fn set_path_changes_a_mean_or_a_number() {
        let mut v: toml::Value = "[a]\nx = { mean = 1.0, std = 0.5 }\ny = 3.0".parse().unwrap();
        set_path(&mut v, "a.x", 2.0, false).unwrap();
        set_path(&mut v, "a.y", 4.0, false).unwrap();
        assert_eq!(v["a"]["x"]["mean"].as_float(), Some(2.0));
        assert_eq!(v["a"]["y"].as_float(), Some(4.0));
        assert!(set_path(&mut v, "a.z", 1.0, false).is_err());
        assert!(set_path(&mut v, "a.x.std.q", 1.0, false).is_err());
    }
}
