//! The `fa models ...` commands: thin wrappers over the library. Output is JSON.

use super::calib::{calibrate, Method, ModelKind};
use super::curve::ZeroCurve;
use super::data::{sample, Freq, Series, Span};
use super::models::{Cir, Ckls, HoLee, HullWhite, RateModel, Vasicek};
use super::rolling::{compare, frequencies, rolling, StudyInput};
use super::sim::{mean_sd_se, quantile_sorted, simulate_map, SimConfig};
use crate::forwards;
use mapcheck::dates;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Instant;

pub type Result<T> = std::result::Result<T, String>;

pub const USAGE: &str = "\
  fa models calibrate --model M --method X [--series S] [--start D] [--end D] [--freq F]
                                                  fit one model to a rate history
  fa models rolling --model M --method X --window 5y --step 3m [--out FILE]
                                                  parameters in many windows (table; FILE: .csv or .json)
  fa models frequencies --model M --method X      same interval at daily, weekly, monthly sampling
  fa models compare                               vasicek, cir, ckls, holee on one interval: log-likelihood, AIC
  fa models simulate --model M [parameters | --calibrated] --years Y --steps-per-year N --paths P --seed S
                                                  quantiles of the rate, and bond price against the closed form
    M: vasicek cir ckls holee hullwhite      X: ml ols gmm realvol     F: daily weekly monthly
    S: gsw1y (default), gsw2y ... gsw30y, SVENY01 ..., csv:PATH (lines: YYYY-MM-DD,decimal)
    --vol close|rolling:N|ewma:L (for realvol, default close)   --gamma G (ckls realvol, default 0.5)
    simulate parameters: --kappa --theta --sigma --gamma --a --r0 --horizons 1,5,10 --curve-date D";

const BOOLEAN: [&str; 1] = ["--calibrated"];

struct Flags(Vec<(String, String)>);

impl Flags {
    fn parse(args: &[String], allowed: &[&str]) -> Result<Flags> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < args.len() {
            let a = &args[i];
            if !allowed.contains(&a.as_str()) {
                return Err(format!("unknown option '{a}' for this command"));
            }
            if BOOLEAN.contains(&a.as_str()) {
                out.push((a.clone(), "true".into()));
                i += 1;
            } else {
                let v = args.get(i + 1).ok_or_else(|| format!("option {a} needs a value"))?;
                out.push((a.clone(), v.clone()));
                i += 2;
            }
        }
        Ok(Flags(out))
    }
    fn get(&self, name: &str) -> Option<&str> {
        self.0.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
    fn num(&self, name: &str) -> Result<Option<f64>> {
        self.get(name).map(|v| v.parse::<f64>().map_err(|_| format!("bad number for {name}: '{v}'"))).transpose()
    }
    fn count(&self, name: &str, default: usize) -> Result<usize> {
        self.get(name).map_or(Ok(default), |v| v.parse::<usize>().map_err(|_| format!("bad whole number for {name}: '{v}'")))
    }
}

const STUDY_FLAGS: [&str; 9] = ["--model", "--method", "--vol", "--gamma", "--series", "--start", "--end", "--freq", "--out"];

pub fn load_series(repo: &Path, spec: &str) -> Result<Series> {
    if let Some(path) = spec.strip_prefix("csv:") {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let (mut days, mut values) = (Vec::new(), Vec::new());
        for (n, line) in text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
            let (d, v) = line.split_once(',').ok_or_else(|| format!("{path} line {}: expected DATE,VALUE", n + 1))?;
            match (dates::parse(d.trim()), v.trim().parse::<f64>()) {
                (Some(d), Ok(v)) => {
                    days.push(d);
                    values.push(v);
                }
                _ if n == 0 => {} // a header line
                _ => return Err(format!("{path} line {}: bad date or number", n + 1)),
            }
        }
        return Series::new(spec, days, values);
    }
    let lower = spec.to_lowercase();
    let column = if let Some(n) = lower.strip_prefix("gsw").and_then(|r| r.strip_suffix('y')).and_then(|n| n.parse::<u32>().ok()) {
        format!("SVENY{n:02}")
    } else if spec.starts_with("SVENY") {
        spec.to_string()
    } else {
        return Err(format!("unknown series '{spec}': use gsw1y ... gsw30y, SVENY01 ... or csv:PATH"));
    };
    let (days, values) = forwards::load(repo)?.history(&column)?;
    Series::new(&format!("GSW {column} zero yield (a proxy of the short rate)"), days, values)
}

struct Study {
    series: Series,
    kind: ModelKind,
    method: Method,
    gamma: f64,
    start: Option<i64>,
    end: Option<i64>,
    freq: Freq,
}

impl Study {
    fn from_flags(repo: &Path, f: &Flags) -> Result<Study> {
        let day = |name: &str| f.get(name).map(|t| forwards::parse_date_arg(t).map(|x| x.1)).transpose();
        Ok(Study {
            series: load_series(repo, f.get("--series").unwrap_or("gsw1y"))?,
            kind: ModelKind::parse(f.get("--model").unwrap_or("vasicek"))?,
            method: Method::parse(f.get("--method").unwrap_or("ml"), f.get("--vol").unwrap_or("close"))?,
            gamma: f.num("--gamma")?.unwrap_or(0.5),
            start: day("--start")?,
            end: day("--end")?,
            freq: Freq::parse(f.get("--freq").unwrap_or("daily"))?,
        })
    }
    fn input(&self) -> StudyInput<'_> {
        StudyInput { series: &self.series, kind: self.kind, method: self.method, gamma: self.gamma, start: self.start, end: self.end, freq: self.freq }
    }
}

fn print(v: &impl serde::Serialize) {
    println!("{}", serde_json::to_string_pretty(v).expect("serializable"));
}

fn timed<T>(what: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let out = f();
    eprintln!("fa: {what} took {:.3} s", t.elapsed().as_secs_f64());
    out
}

/// The entry point of `fa models`. `args` are the words after "models".
pub fn run(repo: &Path, args: &[String]) -> Result<()> {
    let (sub, rest) = args.split_first().ok_or("fa models needs a command: calibrate, rolling, frequencies, compare or simulate")?;
    match sub.as_str() {
        "calibrate" => {
            let st = Study::from_flags(repo, &Flags::parse(rest, &STUDY_FLAGS)?)?;
            let s = sample(&st.series, st.start, st.end, st.freq)?;
            print(&timed("calibration", || calibrate(st.kind, st.method, &s, st.gamma))?);
        }
        "rolling" => {
            let mut allowed = STUDY_FLAGS.to_vec();
            allowed.extend(["--window", "--step"]);
            let f = Flags::parse(rest, &allowed)?;
            let st = Study::from_flags(repo, &f)?;
            let window = Span::parse(f.get("--window").ok_or("rolling needs --window, for example 5y")?)?;
            let step = Span::parse(f.get("--step").ok_or("rolling needs --step, for example 3m")?)?;
            let table = timed("rolling calibration", || rolling(&st.input(), window, step))?;
            write_table(&table, f.get("--out"))?;
        }
        "frequencies" => {
            let st = Study::from_flags(repo, &Flags::parse(rest, &STUDY_FLAGS)?)?;
            print(&timed("frequency comparison", || frequencies(&st.input())));
        }
        "compare" => {
            let st = Study::from_flags(repo, &Flags::parse(rest, &STUDY_FLAGS)?)?;
            let rows = timed("model comparison", || compare(&st.input()));
            print(&json!({
                "series": st.series.name,
                "freq": st.freq.name(),
                "note": "All models: maximum likelihood on the same transitions. vasicek and cir use the exact density. ckls and holee use quasi-likelihoods, so their AIC is only close to the others. delta_aic is the AIC minus the lowest AIC.",
                "results": rows,
            }));
        }
        "simulate" => simulate(repo, rest)?,
        other => return Err(format!("unknown models command '{other}'")),
    }
    Ok(())
}

fn write_table(table: &super::rolling::Table, out: Option<&str>) -> Result<()> {
    match out {
        Some(path) => {
            let text = if path.ends_with(".json") { serde_json::to_string_pretty(table).expect("serializable") } else { table.to_csv() };
            std::fs::write(path, text).map_err(|e| format!("{path}: {e}"))?;
            eprintln!("fa: wrote {} rows to {path}", table.rows.len());
        }
        None => print(table),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// simulate
// ---------------------------------------------------------------------------

const SIM_FLAGS: &[&str] = &[
    "--model", "--method", "--vol", "--gamma", "--series", "--start", "--end", "--freq", "--kappa", "--theta", "--sigma", "--a", "--r0",
    "--horizons", "--curve-date", "--years", "--steps-per-year", "--paths", "--seed", "--calibrated",
];

fn curve_for(repo: &Path, date: Option<&str>) -> Result<(ZeroCurve, Value)> {
    let (text, day) = forwards::parse_date_arg(date.unwrap_or("today"))?;
    let ds = forwards::load(repo)?;
    let f = forwards::forwards_on(&ds, &text, day)?;
    let curve = ZeroCurve::from_forwards(&f)?;
    let max_err = f.zero_yields.iter().map(|p| (curve.zero_yield(p.years as f64) - p.rate).abs()).fold(0.0, f64::max);
    Ok((curve, json!({"date": f.date, "max_abs_error_vs_published_zero_yields": max_err, "short_rate": curve.forward(0.0)})))
}

fn simulate(repo: &Path, rest: &[String]) -> Result<()> {
    let f = Flags::parse(rest, SIM_FLAGS)?;
    let kind = ModelKind::parse(f.get("--model").unwrap_or("vasicek"))?;
    let cfg0 = SimConfig {
        years: f.num("--years")?.unwrap_or(10.0),
        steps_per_year: f.count("--steps-per-year", 252)?,
        paths: f.count("--paths", 100_000)?,
        seed: f.count("--seed", 1)? as u64,
        r0: f.num("--r0")?,
    };
    let mut horizons: Vec<f64> = match f.get("--horizons") {
        Some(h) => h.split(',').map(|x| x.trim().parse::<f64>().map_err(|_| format!("bad horizon '{x}'"))).collect::<Result<_>>()?,
        None => [1.0, 2.0, 5.0, 10.0, 20.0, 30.0].into_iter().filter(|h| *h <= cfg0.years + 1e-9).collect(),
    };
    horizons.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    // parameters: from the history or from the flags
    let mut source = json!("flags");
    let mut get: Box<dyn Fn(&str) -> Option<f64>> = Box::new(|n| f.num(&format!("--{n}")).ok().flatten());
    let mut cfg = cfg0;
    let fit;
    if f.get("--calibrated").is_some() {
        let st = Study::from_flags(repo, &f)?;
        let s = sample(&st.series, st.start, st.end, st.freq)?;
        let fitted = calibrate(st.kind, st.method, &s, st.gamma)?;
        if cfg.r0.is_none() && !matches!(kind, ModelKind::HoLee | ModelKind::HullWhite) {
            cfg.r0 = s.r.last().copied();
        }
        source = json!({"calibrated": {"series": s.series, "method": fitted.method, "freq": fitted.freq, "start": fitted.start, "end": fitted.end}});
        fit = Some(fitted);
        let fr = fit.as_ref().expect("set");
        let table: Vec<(String, f64)> = fr.params.iter().map(|p| (p.name.clone(), p.value)).collect();
        get = Box::new(move |n| table.iter().find(|(k, _)| k == n).map(|(_, v)| *v));
    } else {
        fit = None;
    }
    let need = |n: &str| get(n).ok_or_else(|| format!("{} needs --{n} (or --calibrated)", kind.name()));
    let (report, curve_info) = match kind {
        ModelKind::Vasicek => (report(&Vasicek { kappa: need("kappa")?, theta: need("theta")?, sigma: need("sigma")? }, &cfg, &horizons)?, Value::Null),
        ModelKind::Cir => {
            let m = Cir { kappa: need("kappa")?, theta: need("theta")?, sigma: need("sigma")? };
            let mut r = report(&m, &cfg, &horizons)?;
            r["feller_ratio"] = json!(m.feller_ratio());
            r["feller_satisfied"] = json!(m.feller_satisfied());
            (r, Value::Null)
        }
        ModelKind::Ckls => (report(&Ckls { kappa: need("kappa")?, theta: need("theta")?, sigma: need("sigma")?, gamma: need("gamma")? }, &cfg, &horizons)?, Value::Null),
        ModelKind::HoLee | ModelKind::HullWhite => {
            let (curve, info) = curve_for(repo, f.get("--curve-date"))?;
            let r = if kind == ModelKind::HoLee {
                report(&HoLee { sigma: need("sigma")?, curve }, &cfg, &horizons)?
            } else {
                report(&HullWhite { a: need("a")?, sigma: need("sigma")?, curve }, &cfg, &horizons)?
            };
            (r, info)
        }
    };
    let mut out = report;
    out["source_of_parameters"] = source;
    if !curve_info.is_null() {
        out["zero_curve"] = curve_info;
    }
    if let Some(fit) = fit {
        out["calibration_warnings"] = json!(fit.warnings);
    }
    print(&out);
    Ok(())
}

/// Simulate, then compare with the closed forms. Prints nothing; gives the JSON value.
fn report<M: RateModel>(m: &M, cfg: &SimConfig, horizons: &[f64]) -> Result<Value> {
    let (steps, dt) = (cfg.steps(), cfg.dt());
    let r0 = cfg.r0.unwrap_or_else(|| m.default_r0());
    let idx: Vec<usize> = horizons
        .iter()
        .map(|h| {
            let i = (h / dt).round() as usize;
            if i == 0 || i > steps {
                Err(format!("horizon {h} years is outside the simulation (0, {}] years", cfg.years))
            } else {
                Ok(i)
            }
        })
        .collect::<Result<_>>()?;
    let t0 = Instant::now();
    let rows: Vec<(Vec<f64>, Vec<f64>)> = simulate_map(m, cfg, |_, path| {
        let mut rates = Vec::with_capacity(idx.len());
        let mut integrals = Vec::with_capacity(idx.len());
        let (mut acc, mut k) = (0.0, 0);
        for i in 1..=*idx.last().expect("at least one horizon") {
            acc += 0.5 * (path[i - 1] + path[i]) * dt;
            while k < idx.len() && idx[k] == i {
                rates.push(path[i]);
                integrals.push(acc);
                k += 1;
            }
        }
        (rates, integrals)
    })?;
    let secs = t0.elapsed().as_secs_f64();
    let mut hz = Vec::new();
    for (k, &i) in idx.iter().enumerate() {
        let t = i as f64 * dt;
        let mut col: Vec<f64> = rows.iter().map(|r| r.0[k]).collect();
        let (mean, sd, se) = mean_sd_se(&col);
        col.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let disc: Vec<f64> = rows.iter().map(|r| (-r.1[k]).exp()).collect();
        let (pm, _, pse) = mean_sd_se(&disc);
        let exact_mean = m.mean(r0, t);
        let exact_var = m.variance(r0, t);
        let bond = m.bond_price(r0, t);
        hz.push(json!({
            "years": t,
            "quantiles": {"p05": quantile_sorted(&col, 0.05), "p25": quantile_sorted(&col, 0.25), "p50": quantile_sorted(&col, 0.5), "p75": quantile_sorted(&col, 0.75), "p95": quantile_sorted(&col, 0.95)},
            "mc_mean": mean, "mc_mean_se": se, "mc_sd": sd,
            "exact_mean": exact_mean, "exact_sd": exact_var.map(f64::sqrt),
            "mean_diff_in_se": exact_mean.map(|e| (mean - e) / se),
            "mc_bond_price": pm, "mc_bond_price_se": pse,
            "closed_form_bond_price": bond,
            "bond_diff_in_se": bond.map(|b| (pm - b) / pse),
        }));
    }
    Ok(json!({
        "model": m.name(),
        "parameters": m.parameters().into_iter().collect::<std::collections::BTreeMap<_, _>>(),
        "r0": r0,
        "stationary_mean": m.stationary_mean(), "stationary_sd": m.stationary_variance().map(f64::sqrt),
        "years": cfg.years, "steps": steps, "dt": dt, "paths": cfg.paths, "seed": cfg.seed,
        "seconds": secs, "path_steps_per_second": cfg.paths as f64 * steps as f64 / secs,
        "bond_price_note": "mc_bond_price = mean of exp(-integral of r), integral by the trapezoid rule (error of order dt^2); the closed form is P(0,T) of the model",
        "horizons": hz,
    }))
}
