//! Calibration over many windows, many frequencies, or many models: the three studies of the request.
//! Each result is a `Table` that can print as JSON or write as CSV.

use super::calib::{calibrate, Fit, Method, ModelKind};
use super::data::{sample, Freq, Series, Span};
use mapcheck::dates;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;

pub type Result<T> = std::result::Result<T, String>;

/// A table: column names and rows of JSON values (null = empty).
#[derive(Debug, Clone, Serialize)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl Table {
    pub fn to_csv(&self) -> String {
        let cell = |v: &Value| match v {
            Value::Null => String::new(),
            Value::String(s) if s.contains([',', '"', '\n']) => format!("\"{}\"", s.replace('"', "\"\"")),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let mut out = self.columns.join(",");
        out.push('\n');
        for row in &self.rows {
            out.push_str(&row.iter().map(cell).collect::<Vec<_>>().join(","));
            out.push('\n');
        }
        out
    }
}

fn num(x: Option<f64>) -> Value {
    match x {
        Some(v) if v.is_finite() => Value::from(v),
        _ => Value::Null,
    }
}

/// The parameter names of a model, in the order of the table.
pub fn param_names(kind: ModelKind) -> &'static [&'static str] {
    match kind {
        ModelKind::Vasicek | ModelKind::Cir => &["kappa", "theta", "sigma"],
        ModelKind::Ckls => &["kappa", "theta", "sigma", "gamma"],
        ModelKind::HoLee => &["sigma"],
        ModelKind::HullWhite => &["a", "sigma"],
    }
}

fn fit_columns(kind: ModelKind) -> Vec<String> {
    let mut c: Vec<String> = Vec::new();
    for p in param_names(kind) {
        c.push((*p).into());
        c.push(format!("{p}_se"));
    }
    for n in ["realized_vol", "loglik", "aic", "mean_reversion_t", "mean_reversion_significant", "feller_ratio", "warnings", "error"] {
        c.push(n.into());
    }
    c
}

fn fit_cells(kind: ModelKind, fit: &std::result::Result<Fit, String>) -> Vec<Value> {
    let mut v = Vec::new();
    for p in param_names(kind) {
        v.push(num(fit.as_ref().ok().and_then(|f| f.get(p))));
        v.push(num(fit.as_ref().ok().and_then(|f| f.se(p))));
    }
    match fit {
        Ok(f) => {
            v.push(num(Some(f.realized_vol)));
            v.push(num(f.loglik));
            v.push(num(f.aic));
            v.push(num(f.mean_reversion_t));
            v.push(f.mean_reversion_significant.map_or(Value::Null, Value::from));
            v.push(num(f.feller_ratio));
            v.push(if f.warnings.is_empty() { Value::Null } else { Value::from(f.warnings.join(" | ")) });
            v.push(Value::Null);
        }
        Err(e) => {
            v.extend(std::iter::repeat(Value::Null).take(7));
            v.push(Value::from(e.clone()));
        }
    }
    v
}

pub struct StudyInput<'a> {
    pub series: &'a Series,
    pub kind: ModelKind,
    pub method: Method,
    pub gamma: f64,
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub freq: Freq,
}

fn fit_one(inp: &StudyInput, start: Option<i64>, end: Option<i64>, freq: Freq) -> std::result::Result<Fit, String> {
    let s = sample(inp.series, start, end, freq)?;
    calibrate(inp.kind, inp.method, &s, inp.gamma)
}

/// The windows [s, s + window] for s = first, first + step, ... that end on or before the last day.
/// One calibration for each window, in parallel. A window that fails gives a row with the error text.
pub fn rolling(inp: &StudyInput, window: Span, step: Span) -> Result<Table> {
    let first = inp.start.unwrap_or(inp.series.days[0]).max(inp.series.days[0]);
    let last = inp.end.unwrap_or(*inp.series.days.last().expect("series not empty")).min(*inp.series.days.last().expect("series not empty"));
    let mut windows = Vec::new();
    let mut s = first;
    loop {
        let e = window.add_to(s);
        if e > last {
            break;
        }
        windows.push((s, e));
        let next = step.add_to(s);
        if next <= s {
            return Err("the step is zero".into());
        }
        s = next;
    }
    if windows.is_empty() {
        return Err(format!(
            "no window fits: the interval {} to {} is shorter than the window",
            dates::format(first),
            dates::format(last)
        ));
    }
    let fits: Vec<std::result::Result<Fit, String>> = windows.par_iter().map(|&(s, e)| fit_one(inp, Some(s), Some(e), inp.freq)).collect();
    let mut columns: Vec<String> = ["window_start", "window_end", "n_transitions"].iter().map(|s| s.to_string()).collect();
    columns.extend(fit_columns(inp.kind));
    let rows = windows
        .iter()
        .zip(&fits)
        .map(|(&(s, e), fit)| {
            let mut row = vec![Value::from(dates::format(s)), Value::from(dates::format(e)), fit.as_ref().map_or(Value::Null, |f| Value::from(f.n_transitions))];
            row.extend(fit_cells(inp.kind, fit));
            row
        })
        .collect();
    Ok(Table { columns, rows })
}

/// The same interval at daily, weekly and monthly sampling. The parameters are annualized, so the rows compare.
pub fn frequencies(inp: &StudyInput) -> Table {
    let freqs = [Freq::Daily, Freq::Weekly, Freq::Monthly];
    let fits: Vec<std::result::Result<Fit, String>> = freqs.par_iter().map(|&f| fit_one(inp, inp.start, inp.end, f)).collect();
    let mut columns: Vec<String> = ["freq", "n_transitions", "dt_years"].iter().map(|s| s.to_string()).collect();
    columns.extend(fit_columns(inp.kind));
    let rows = freqs
        .iter()
        .zip(&fits)
        .map(|(f, fit)| {
            let mut row = vec![
                Value::from(f.name()),
                fit.as_ref().map_or(Value::Null, |x| Value::from(x.n_transitions)),
                num(fit.as_ref().ok().map(|x| x.dt_years)),
            ];
            row.extend(fit_cells(inp.kind, fit));
            row
        })
        .collect();
    Table { columns, rows }
}

#[derive(Debug, Clone, Serialize)]
pub struct CompareRow {
    pub model: String,
    pub method: String,
    pub delta_aic: Option<f64>,
    #[serde(flatten)]
    pub fit: Option<Fit>,
    pub error: Option<String>,
}

/// All models with a likelihood (vasicek, cir, ckls, holee) on one interval, by maximum likelihood.
/// The rows sort by AIC. The quasi-likelihoods (ckls, holee) are not strictly comparable with the exact ones.
pub fn compare(inp: &StudyInput) -> Vec<CompareRow> {
    let kinds = [ModelKind::Vasicek, ModelKind::Cir, ModelKind::Ckls, ModelKind::HoLee];
    let mut rows: Vec<CompareRow> = kinds
        .par_iter()
        .map(|&kind| {
            let one = StudyInput { series: inp.series, kind, method: Method::Ml, gamma: inp.gamma, start: inp.start, end: inp.end, freq: inp.freq };
            match fit_one(&one, inp.start, inp.end, inp.freq) {
                Ok(f) => CompareRow { model: kind.name().into(), method: "ml".into(), delta_aic: None, fit: Some(f), error: None },
                Err(e) => CompareRow { model: kind.name().into(), method: "ml".into(), delta_aic: None, fit: None, error: Some(e) },
            }
        })
        .collect();
    let best = rows.iter().filter_map(|r| r.fit.as_ref().and_then(|f| f.aic)).fold(f64::INFINITY, f64::min);
    for r in rows.iter_mut() {
        r.delta_aic = r.fit.as_ref().and_then(|f| f.aic).map(|a| a - best);
    }
    rows.sort_by(|a, b| a.delta_aic.unwrap_or(f64::INFINITY).partial_cmp(&b.delta_aic.unwrap_or(f64::INFINITY)).unwrap_or(std::cmp::Ordering::Equal));
    rows
}
