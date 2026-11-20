//! `fa dcf drivers TICKER --model FILE` for a model file with `engine = "paths"`.

use super::cli::money;
use super::model::bisect;
use super::paths::*;
use super::process::PathsSpec;
use super::stats::{percentile, sorted_copy};
use super::DcfError;
use serde_json::json;
use std::time::Instant;

type R<T> = Result<T, DcfError>;

const FLAGS: [&str; 5] = ["--json", "--shift-bounds", "--time", "--no-attribution", "--no-shocks"];
const VALUES: [&str; 10] = ["--freeze", "--model", "--seed", "--sims", "--price", "--pset", "--sensitivity", "--implied-price", "--implied-param", "--range"];

fn e<T>(m: impl Into<String>) -> R<T> {
    Err(DcfError(m.into()))
}

struct Args<'a>(&'a [String]);

impl<'a> Args<'a> {
    fn all(&self, name: &str) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.0.len() {
            if self.0[i] == name {
                if let Some(v) = self.0.get(i + 1) {
                    out.push(v.as_str());
                }
                i += 1;
            }
            i += 1;
        }
        out
    }
    fn one(&self, name: &str) -> Option<&'a str> {
        self.all(name).last().copied()
    }
    fn flag(&self, name: &str) -> bool {
        self.0.iter().any(|a| a == name)
    }
    fn num<T: std::str::FromStr>(&self, name: &str) -> R<Option<T>> {
        match self.one(name) {
            None => Ok(None),
            Some(v) => v.parse::<T>().map(Some).or_else(|_| e(format!("bad value for {name}: '{v}'"))),
        }
    }
    fn check(&self) -> R<()> {
        let mut i = if self.0.first().is_some_and(|t| !t.starts_with("--")) { 1 } else { 0 };
        while i < self.0.len() {
            let a = self.0[i].as_str();
            if VALUES.contains(&a) {
                if i + 1 >= self.0.len() {
                    return e(format!("{a} needs a value"));
                }
                i += 1;
            } else if !FLAGS.contains(&a) {
                let old = ["--set", "--flat", "--ramp", "--corr", "--net-debt", "--shares", "--years", "--year1-revenue", "--keep-year1-margin", "--implied-driver"];
                if old.contains(&a) {
                    return e(format!("option '{a}' is for the old engine; the path engine has --pset NAME=VALUE (the name is the TOML path of the number)"));
                }
                return e(format!("unknown option '{a}'"));
            }
            i += 1;
        }
        Ok(())
    }
}

fn pair(text: &str, what: &str) -> R<(String, f64)> {
    let (k, v) = text.split_once('=').ok_or_else(|| DcfError(format!("{what}: expected NAME=VALUE, got '{text}'")))?;
    Ok((k.to_string(), v.parse().map_err(|_| DcfError(format!("{what}: bad number '{v}'")))?))
}

/// The text of the model file and the engine run for it.
pub fn run(model_text: &str, rest: &[String]) -> R<()> {
    let a = Args(rest);
    a.check()?;
    let ticker_arg = rest.first().filter(|t| !t.starts_with("--")).ok_or_else(|| DcfError("drivers needs a ticker".into()))?;
    let seed: u64 = a.num("--seed")?.unwrap_or(7);
    let n: usize = a.num("--sims")?.unwrap_or(10_000);
    if n == 0 {
        return e("--sims must be positive");
    }
    let prices: Vec<f64> = a.all("--price").iter().map(|p| p.parse::<f64>().map_err(|_| DcfError(format!("bad --price '{p}'")))).collect::<R<_>>()?;
    for p in &prices {
        if !(p.is_finite() && *p > 0.0) {
            return e(format!("--price must be a positive number, got {p}"));
        }
    }
    let shift_bounds = a.flag("--shift-bounds");
    let sets: Vec<(String, f64)> = a.all("--pset").iter().map(|t| pair(t, "--pset")).collect::<R<_>>()?;
    let spec = PathsSpec::from_toml_with(model_text, &sets, shift_bounds)?;
    let ticker = if spec.ticker.is_empty() { ticker_arg.to_uppercase() } else { spec.ticker.clone() };
    let eng = PathEngine::new(&spec)?;

    let mut mods = Mods { no_shocks: a.flag("--no-shocks"), ..Mods::default() };
    for g in a.all("--freeze") {
        mods.frozen.push(eng.group_index(g).ok_or_else(|| DcfError(format!("--freeze: no group '{g}'; groups: {}", eng.groups.join(", "))))?);
    }
    let t0 = Instant::now();
    let sim = eng.simulate(n, seed, &mods, None, true)?;
    let sim_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let det = eng.deterministic()?;
    let q = sim.quantiles();
    let base_p50 = q[2];
    let t1 = Instant::now();
    let (attribution, shares) = if a.flag("--no-attribution") {
        (vec![], vec![])
    } else {
        (eng.attribution(n, seed, base_p50)?, eng.variance_shares(n, seed, &sim.per_share)?)
    };
    let attr_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let below: Vec<(f64, f64)> = prices.iter().map(|p| (*p, sim.share_below(*p))).collect();

    // sensitivity and implied value: runs of the engine with other numbers (same seed)
    let p50_with = |extra: &[(String, f64)]| -> R<f64> {
        let mut all = sets.clone();
        for (k, v) in extra {
            all.retain(|(x, _)| x != k);
            all.push((k.clone(), *v));
        }
        let s = PathsSpec::from_toml_with(model_text, &all, shift_bounds)?;
        Ok(PathEngine::new(&s)?.simulate(n, seed, &mods, None, false)?.p50())
    };
    let mut sens: Vec<(String, Vec<(f64, f64)>)> = Vec::new();
    for t in a.all("--sensitivity") {
        let (name, vals) = t.split_once('=').ok_or_else(|| DcfError(format!("--sensitivity: expected NAME=V1,V2, got '{t}'")))?;
        let values: Vec<f64> = vals.split(',').map(|v| v.parse::<f64>().map_err(|_| DcfError(format!("--sensitivity: bad number '{v}'")))).collect::<R<_>>()?;
        let rows = values.iter().map(|v| Ok((*v, p50_with(&[(name.to_string(), *v)])?))).collect::<R<Vec<_>>>()?;
        sens.push((name.to_string(), rows));
    }
    let mut implied: Option<(String, f64, Option<f64>)> = None;
    if let Some(price) = a.num::<f64>("--implied-price")? {
        let name = a.one("--implied-param").ok_or_else(|| DcfError("--implied-price needs --implied-param NAME".into()))?;
        let d = eng.params.iter().find(|p| p.name == name).ok_or_else(|| DcfError(format!("--implied-param: no number named '{name}'")))?;
        let (lo, hi) = match a.one("--range") {
            Some(r) => {
                let (l, h) = r.split_once(',').ok_or_else(|| DcfError("--range: expected LO,HI".into()))?;
                (l.parse::<f64>().map_err(|_| DcfError("--range: bad LO".into()))?, h.parse::<f64>().map_err(|_| DcfError("--range: bad HI".into()))?)
            }
            None => (d.mean - 5.0 * d.std.max(0.01), d.mean + 5.0 * d.std.max(0.01)),
        };
        implied = Some((name.to_string(), price, bisect(|x| p50_with(&[(name.to_string(), x)]), lo, hi, price)?));
    }

    // tables
    let years = sim.years;
    let rev_q: Vec<[f64; 3]> = (0..years)
        .map(|t| {
            let s = sorted_copy(&sim.column(t, F_REV));
            [percentile(&s, 10.0), percentile(&s, 50.0), percentile(&s, 90.0)]
        })
        .collect();
    let mean_row = |s: &Sim, t: usize| -> [f64; NF] {
        let mut r = [0.0; NF];
        for (f, v) in r.iter_mut().enumerate() {
            *v = s.mean_of(t, f);
        }
        r
    };
    let mean_path: Vec<[f64; NF]> = (0..years).map(|t| mean_row(&sim, t)).collect();
    let det_path: Vec<[f64; NF]> = (0..years).map(|t| mean_row(&det, t)).collect();
    let det_ev = det.pv_explicit[0] + det.pv_terminal[0];
    let det_tv_share = det.pv_terminal[0] / det_ev;
    let base_revenue: f64 = spec.segments.iter().map(|s| s.base_revenue).sum();

    if a.flag("--json") {
        let out = json!({
            "ticker": ticker, "engine": "paths", "seed": seed, "simulations": n, "projection_years": years,
            "p10": q[0], "p25": q[1], "p50": q[2], "p75": q[3], "p90": q[4],
            "share_below_price": below.iter().map(|(p, s)| json!({"price": p, "share": s})).collect::<Vec<_>>(),
            "value_at_means": det.per_share[0],
            "mean": sim.mean(), "mean_se": sim.mean_se(), "top_1pct_share_of_mean": sim.top_percent_share(), "non_finite_paths": sim.non_finite(),
            "tv_share_at_means": det_tv_share,
            "tv_share_paths": sim.tv_share(),
            "terminal_growth_capped": {"by_process": sim.capped_share(CAP_PROCESS), "by_rf": sim.capped_share(CAP_RF), "by_gap_guard": sim.capped_share(CAP_GAP)},
            "failed_share": sim.failed as f64 / n as f64,
            "attribution": attribution.iter().map(|x| json!({"param": x.name, "p50_minus_1sd": x.p50_minus_1sd, "p50_plus_1sd": x.p50_plus_1sd})).collect::<Vec<_>>(),
            "variance_shares": shares.iter().map(|s| json!({"group": s.group, "alone": s.alone, "removed": s.removed})).collect::<Vec<_>>(),
            "mean_path": (0..years).map(|t| json!({"year": t + 1, "revenue": mean_path[t][F_REV], "growth": mean_path[t][F_GROWTH], "margin": mean_path[t][F_MARGIN], "sbc": mean_path[t][F_SBC], "reinvestment": mean_path[t][F_REINV], "fcff": mean_path[t][F_FCFF], "discount_factor": mean_path[t][F_DISC], "rate": mean_path[t][F_RATE]})).collect::<Vec<_>>(),
            "revenue_quantiles": (0..years).map(|t| json!({"year": t + 1, "p10": rev_q[t][0], "p50": rev_q[t][1], "p90": rev_q[t][2]})).collect::<Vec<_>>(),
            "sensitivity": sens.iter().map(|(n, v)| json!({"param": n, "p50": v.iter().map(|(x, y)| json!({"value": x, "p50": y})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "implied": implied.as_ref().map(|(d, p, v)| json!({"param": d, "price": p, "value": v})),
            "notes": spec.notes,
        });
        println!("{}", serde_json::to_string_pretty(&out).expect("json"));
        return Ok(());
    }

    println!("{ticker} path DCF - seed {seed}, {n} simulations, {years} years (engine: paths; reinvestment: {}; rates: {})", spec.reinvestment.mode, spec.rates.model);
    if !spec.notes.is_empty() {
        println!("  Model: {}", spec.notes.lines().next().unwrap_or(""));
    }
    println!("\nHEADLINE (value for each share)");
    println!("  P10 {:.2} | P25 {:.2} | P50 {:.2} | P75 {:.2} | P90 {:.2}", q[0], q[1], q[2], q[3], q[4]);
    println!(
        "  mean {:.2} (standard error {:.2}, {n} paths); the largest 1% of the paths hold {:.1}% of the sum of the positive values.\n  The mean is a measurement of the distribution, not a better estimate than the P50. It is unstable when one tail holds a large share. It is not tested against later returns.",
        sim.mean(),
        sim.mean_se(),
        sim.top_percent_share() * 100.0
    );
    if sim.non_finite() > 0 {
        println!("  WARNING: {} of {n} paths have a value that is not a finite number; the mean, its standard error and the share leave them out", sim.non_finite());
    }
    for (p, s) in &below {
        println!("  paths with value below ${p:.2}: {:.1}%", s * 100.0);
    }
    println!(
        "  terminal value share of EV: {:.1}% at the means, {:.1}% as the mean over paths",
        det_tv_share * 100.0,
        sim.tv_share() * 100.0
    );
    println!(
        "  terminal growth cut by the cap of the process on {:.1}% of paths, by the rate cap on {:.1}%, by the guard (cost of capital - 2%) on {:.2}%",
        sim.capped_share(CAP_PROCESS) * 100.0,
        sim.capped_share(CAP_RF) * 100.0,
        sim.capped_share(CAP_GAP) * 100.0
    );
    if sim.failed > 0 {
        println!("  paths that fail inside the horizon: {:.1}%", sim.failed as f64 / n as f64 * 100.0);
    }
    let equity_p50 = q[2] * spec.shares;
    println!(
        "  equity value at P50 {} on {:.1}M shares; net debt {} ({:+.2} a share); EV at P50 {} = {:.1} x trailing revenue",
        money(equity_p50),
        spec.shares / 1e6,
        money(spec.net_debt),
        spec.net_debt / spec.shares,
        money(equity_p50 + spec.net_debt),
        (equity_p50 + spec.net_debt) / base_revenue
    );
    for (p, _) in &below {
        println!("  market value at ${p:.2}: {}; EV {:.1} x trailing revenue", money(p * spec.shares), (p * spec.shares + spec.net_debt) / base_revenue);
    }
    println!("\nVALUE AT THE MEANS (every drawn number at its mean, every shock zero, expected rate path)");
    println!(
        "  ${:.4} | EV {}, of which terminal value {} | cost of capital year 1 {:.2}%, year {years} {:.2}%, terminal {:.2}%",
        det.per_share[0],
        money(det_ev),
        money(det.pv_terminal[0]),
        det.at(0, 0, F_RATE) * 100.0,
        det.at(0, years - 1, F_RATE) * 100.0,
        det.k_terminal[0] * 100.0
    );
    println!("  year   revenue   growth   margin     FCFF   discount");
    for (t, r) in det_path.iter().enumerate() {
        println!("  {:>4} {:>9} {:>7.1}% {:>7.1}% {:>8} {:>9.4}", t + 1, money(r[F_REV]), r[F_GROWTH] * 100.0, r[F_MARGIN] * 100.0, money(r[F_FCFF]), r[F_DISC]);
    }
    if !attribution.is_empty() {
        println!("\nATTRIBUTION (P50 change for the mean of a drawn number -1 std / +1 std)");
        let mut ranked = attribution.clone();
        ranked.sort_by(|x, y| {
            let k = |a: &ParamEffect| a.p50_minus_1sd.abs().max(a.p50_plus_1sd.abs());
            k(y).partial_cmp(&k(x)).unwrap_or(std::cmp::Ordering::Equal)
        });
        for x in &ranked {
            println!("  {:<30} {:+9.2} {:+9.2}", x.name, x.p50_minus_1sd, x.p50_plus_1sd);
        }
        println!("\nVARIANCE OF THE VALUE BY GROUP (alone: only this group is random; removed: this group fixed at its means)");
        let mut sh = shares.clone();
        sh.sort_by(|x, y| y.removed.partial_cmp(&x.removed).unwrap_or(std::cmp::Ordering::Equal));
        for s in &sh {
            println!("  {:<10} alone {:>6.1}%  removed {:>6.1}%", s.group, s.alone * 100.0, s.removed * 100.0);
        }
    }
    println!("\nMEAN PATH (mean over {n} paths; a failed path has revenue 0 after the failure)");
    println!("  year   revenue   growth   margin     SBC  reinvest     FCFF   rate  discount   revenue P10 / P50 / P90");
    for t in 0..years {
        let r = &mean_path[t];
        println!(
            "  {:>4} {:>9} {:>7.1}% {:>7.1}% {:>6.1}% {:>9} {:>8} {:>6.2}% {:>8.4}   {} / {} / {}",
            t + 1,
            money(r[F_REV]),
            r[F_GROWTH] * 100.0,
            r[F_MARGIN] * 100.0,
            r[F_SBC] * 100.0,
            money(r[F_REINV]),
            money(r[F_FCFF]),
            r[F_RATE] * 100.0,
            r[F_DISC],
            money(rev_q[t][0]),
            money(rev_q[t][1]),
            money(rev_q[t][2])
        );
    }
    for (name, v) in &sens {
        println!("\nSENSITIVITY of {name} (P50, the other numbers at base)");
        for (x, y) in v {
            println!("  {x}: {y:.2}");
        }
    }
    if let Some((d, p, v)) = &implied {
        println!("\nWHAT A PRICE IMPLIES (bisection on the engine P50)");
        match v {
            Some(v) => println!("  ${p:.2}: {d} = {:.4}", *v),
            None => println!("  ${p:.2}: no solution for {d} in the range"),
        }
    }
    if a.flag("--time") {
        println!("\nTIME: simulation with detail {sim_ms:.1} ms; attribution and variance shares {attr_ms:.1} ms");
    }
    Ok(())
}
