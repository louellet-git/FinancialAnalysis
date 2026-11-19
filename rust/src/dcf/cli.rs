//! `fa dcf ...` commands: driver model runs, classic runs, scenarios, and the verdict.

use super::analysis::{implied_mean, sensitivity};
use super::classic::{run_classic, Company, DcfAssumptions};
use super::driver::Ramp;
use super::model::{available_models, hand_dcf_at_means, ModelSpec, Overrides};
use super::scenarios::{capital_structure_for, scenarios_for};
use super::stats::{percentile, percentiles, sorted_copy};
use super::verdict::{chance_price_too_high, verdict_band};
use super::DcfError;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa dcf - Monte Carlo DCF (Rust engines; the random numbers are the same as numpy's)

USAGE:
  fa dcf models                                   models that have analysis/<TICKER>/driver_model.toml
  fa dcf drivers TICKER [OPTIONS]                 driver model: value per share, attribution
  fa dcf classic TICKER [OPTIONS]                 classic model on data/processed/<TICKER>.json
  fa dcf scenarios TICKER [--data-dir D] [--json] bear/base/bull of the classic model, value per share

DRIVERS OPTIONS:
  --seed S (7)   --sims N (10000)   --json
  --price P            market price; repeat for several. Gives the share of paths below it.
  --set NAME=V         new mean of a driver (no schedule)       --shift-bounds  move lo/hi with it
  --flat NAME=V        a scheduled driver becomes flat          --ramp NAME=START:END:YEARS
  --corr A,B=RHO       new correlation of a pair
  --net-debt X  --shares X  --years N  --year1-revenue SEGMENT=X
  --sensitivity NAME=V1,V2,..   P50 for each value of one driver mean (repeat for more drivers)
  --implied-price P --implied-driver NAME [--range LO,HI]
                       the mean of the driver for which the P50 is P (bisection)

CLASSIC OPTIONS:
  --sims N (10000)  --years N (10)  --growth G (0.15)  --wacc W (0.10)  --terminal-growth T (0.03)
  --seed S  --data-dir D (default <repo>/data/processed)  --json

VERDICT:
  fa verdict TICKER --price P [--simulations N] [--seed S]   (also in the top-level help)";

type R<T> = Result<T, DcfError>;

fn e<T>(m: impl Into<String>) -> R<T> {
    Err(DcfError(m.into()))
}

/// The value of `--name`, and the list of all values for a repeated option.
struct Args<'a> {
    rest: &'a [String],
}

impl<'a> Args<'a> {
    fn all(&self, name: &str) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.rest.len() {
            if self.rest[i] == name {
                if let Some(v) = self.rest.get(i + 1) {
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
        self.rest.iter().any(|a| a == name)
    }
    fn num<T: std::str::FromStr>(&self, name: &str) -> R<Option<T>> {
        match self.one(name) {
            None => Ok(None),
            Some(v) => v.parse::<T>().map(Some).or_else(|_| e(format!("bad value for {name}: '{v}'"))),
        }
    }
    /// Check that each word that starts with `--` is known.
    fn check(&self, known_flags: &[&str], known_values: &[&str]) -> R<()> {
        // the first word is the ticker
        let mut i = if self.rest.first().is_some_and(|t| !t.starts_with("--")) { 1 } else { 0 };
        while i < self.rest.len() {
            let a = self.rest[i].as_str();
            if known_values.contains(&a) {
                if i + 1 >= self.rest.len() {
                    return e(format!("{a} needs a value"));
                }
                i += 1;
            } else if !known_flags.contains(&a) {
                return e(format!("unknown option '{a}'"));
            }
            i += 1;
        }
        Ok(())
    }
}

fn pair(text: &str, what: &str) -> R<(String, f64)> {
    let (k, v) = text.split_once('=').ok_or_else(|| DcfError(format!("{what}: expected NAME=VALUE, got '{text}'")))?;
    let v: f64 = v.parse().map_err(|_| DcfError(format!("{what}: bad number '{v}'")))?;
    Ok((k.to_string(), v))
}

fn overrides_from(a: &Args) -> R<Overrides> {
    let mut o = Overrides { shift_bounds: a.flag("--shift-bounds"), ..Overrides::default() };
    for t in a.all("--set") {
        o.means.push(pair(t, "--set")?);
    }
    for t in a.all("--flat") {
        o.flat.push(pair(t, "--flat")?);
    }
    for t in a.all("--ramp") {
        let (k, v) = t.split_once('=').ok_or_else(|| DcfError(format!("--ramp: expected NAME=START:END:YEARS, got '{t}'")))?;
        let p: Vec<&str> = v.split(':').collect();
        if p.len() != 3 {
            return e(format!("--ramp: expected NAME=START:END:YEARS, got '{t}'"));
        }
        let num = |s: &str| s.parse::<f64>().map_err(|_| DcfError(format!("--ramp: bad number '{s}'")));
        let years = p[2].parse::<usize>().map_err(|_| DcfError(format!("--ramp: bad years '{}'", p[2])))?;
        o.ramps.push((k.to_string(), Ramp { start: num(p[0])?, end: num(p[1])?, ramp_years: years }));
    }
    for t in a.all("--corr") {
        let (k, v) = pair(t, "--corr")?;
        let (x, y) = k.split_once(',').ok_or_else(|| DcfError(format!("--corr: expected A,B=RHO, got '{t}'")))?;
        o.corr.push((x.to_string(), y.to_string(), v));
    }
    for t in a.all("--year1-revenue") {
        o.year1_revenue.push(pair(t, "--year1-revenue")?);
    }
    o.net_debt = a.num("--net-debt")?;
    o.shares = a.num("--shares")?;
    o.years = a.num("--years")?;
    Ok(o)
}

fn load_model(repo: &Path, ticker: &str) -> R<ModelSpec> {
    ModelSpec::load(repo, ticker)?.ok_or_else(|| {
        DcfError(format!(
            "No driver model for {}: expected analysis/{}/driver_model.toml. Models: {}.",
            ticker.to_uppercase(),
            ticker.to_uppercase(),
            available_models(repo).join(", ")
        ))
    })
}

/// `$1,288.8B` (billions, one decimal, thousands separator: the format of `scripts/run_dcf.py`).
pub(crate) fn money(v: f64) -> String {
    let text = format!("{:.1}", (v / 1e9).abs());
    let (int, frac) = text.split_once('.').unwrap_or((&text, "0"));
    let mut grouped = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if v < 0.0 && text != "0.0" { "-" } else { "" };
    format!("${sign}{grouped}.{frac}B")
}

/// `fa dcf drivers`
fn drivers(repo: &Path, rest: &[String]) -> R<()> {
    let a = Args { rest };
    a.check(
        &["--json", "--shift-bounds"],
        &[
            "--seed", "--sims", "--price", "--set", "--flat", "--ramp", "--corr", "--net-debt", "--shares", "--years",
            "--year1-revenue", "--sensitivity", "--implied-price", "--implied-driver", "--range",
        ],
    )?;
    let ticker = rest.first().filter(|t| !t.starts_with("--")).ok_or_else(|| DcfError("drivers needs a ticker".into()))?;
    let seed: u64 = a.num("--seed")?.unwrap_or(7);
    let n: usize = a.num("--sims")?.unwrap_or(10_000);
    let prices: Vec<f64> = a.all("--price").iter().map(|p| p.parse::<f64>().map_err(|_| DcfError(format!("bad --price '{p}'")))).collect::<R<_>>()?;
    for p in &prices {
        if !(p.is_finite() && *p > 0.0) {
            return e(format!("--price must be a positive number, got {p}"));
        }
    }
    let base = load_model(repo, ticker)?;
    let o = overrides_from(&a)?;
    let spec = base.with_overrides(&o)?;
    let eng = spec.engine()?;
    let r = eng.run(n, Some(seed))?;
    let q = r.quantiles();
    let (hand, hand_ev, hand_tv) = hand_dcf_at_means(&spec)?;
    let mut ranked = r.attribution.clone();
    ranked.sort_by(|x, y| {
        let k = |a: &super::driver::DriverAttribution| a.p50_minus_1sd.abs().max(a.p50_plus_1sd.abs());
        k(y).partial_cmp(&k(x)).unwrap_or(std::cmp::Ordering::Equal)
    });
    let below: Vec<(f64, f64)> = prices.iter().map(|p| (*p, r.share_below(*p))).collect();

    let mut sens: Vec<(String, Vec<(f64, f64)>)> = Vec::new();
    for t in a.all("--sensitivity") {
        let (name, vals) = t.split_once('=').ok_or_else(|| DcfError(format!("--sensitivity: expected NAME=V1,V2, got '{t}'")))?;
        let values: Vec<f64> = vals.split(',').map(|v| v.parse::<f64>().map_err(|_| DcfError(format!("--sensitivity: bad number '{v}'")))).collect::<R<_>>()?;
        sens.push((name.to_string(), sensitivity(&base, &o, name, &values, n, seed)?));
    }
    let mut implied: Option<(String, f64, Option<f64>)> = None;
    if let Some(price) = a.num::<f64>("--implied-price")? {
        let name = a.one("--implied-driver").ok_or_else(|| DcfError("--implied-price needs --implied-driver".into()))?;
        let d = base.driver(name)?;
        let (lo, hi) = match a.one("--range") {
            Some(r) => {
                let (l, h) = r.split_once(',').ok_or_else(|| DcfError("--range: expected LO,HI".into()))?;
                (
                    l.parse::<f64>().map_err(|_| DcfError("--range: bad LO".into()))?,
                    h.parse::<f64>().map_err(|_| DcfError("--range: bad HI".into()))?,
                )
            }
            None => (d.mean - 5.0 * d.std.max(0.01), d.mean + 5.0 * d.std.max(0.01)),
        };
        implied = Some((name.to_string(), price, implied_mean(&base, &o, name, lo, hi, price, n, seed)?));
    }

    if a.flag("--json") {
        let attribution: Vec<Value> =
            ranked.iter().map(|x| json!({"driver": x.driver, "p50_minus_1sd": x.p50_minus_1sd, "p50_plus_1sd": x.p50_plus_1sd})).collect();
        let out = json!({
            "ticker": spec.ticker, "seed": seed, "simulations": n, "projection_years": spec.projection_years,
            "p10": q[0], "p25": q[1], "p50": q[2], "p75": q[3], "p90": q[4],
            "value_at_means": r.value_at_means,
            "hand_dcf_at_means": {"per_share": hand, "ev": hand_ev, "pv_terminal": hand_tv},
            "tv_ev_share_at_means": r.tv_ev_share_at_means,
            "tg_pinned_fraction": r.tg_pinned_fraction,
            "attribution": attribution,
            "share_below_price": below.iter().map(|(p, s)| json!({"price": p, "share": s})).collect::<Vec<_>>(),
            "sensitivity": sens.iter().map(|(n, v)| json!({"driver": n, "p50": v.iter().map(|(x, y)| json!({"mean": x, "p50": y})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "implied": implied.as_ref().map(|(d, p, v)| json!({"driver": d, "price": p, "mean": v})),
            "notes": spec.notes,
        });
        println!("{}", serde_json::to_string_pretty(&out).expect("json"));
        return Ok(());
    }
    println!("{} driver DCF - seed {seed}, {n} simulations, {} years", spec.ticker, spec.projection_years);
    if !spec.notes.is_empty() {
        println!("  Model: {}", spec.notes.lines().next().unwrap_or(""));
    }
    println!("\nHEADLINE (value for each share)");
    println!("  P10 {:.2} | P25 {:.2} | P50 {:.2} | P75 {:.2} | P90 {:.2}", q[0], q[1], q[2], q[3], q[4]);
    for (p, s) in &below {
        println!("  paths with value below ${p:.2}: {:.1}%", s * 100.0);
    }
    println!("  terminal value share of EV at means {:.1}%; terminal growth pinned on {:.2}% of paths", r.tv_ev_share_at_means * 100.0, r.tg_pinned_fraction * 100.0);
    println!("\nENGINE CHECK (each driver at its mean)");
    println!(
        "  engine ${:.4} | hand DCF ${:.4} | EV {}, of which terminal value {}",
        r.value_at_means,
        hand,
        money(hand_ev),
        money(hand_tv)
    );
    println!("\nATTRIBUTION (P50 change for mean -1 std / +1 std)");
    for x in &ranked {
        println!("  {:<18} {:+9.2} {:+9.2}", x.driver, x.p50_minus_1sd, x.p50_plus_1sd);
    }
    for (name, v) in &sens {
        println!("\nSENSITIVITY of {name} (P50, other drivers at base)");
        for (x, y) in v {
            println!("  mean {x}: {y:.2}");
        }
    }
    if let Some((d, p, v)) = &implied {
        println!("\nWHAT A PRICE IMPLIES (bisection on the engine P50)");
        match v {
            Some(v) => println!("  ${p:.2}: mean of {d} = {v:.4}"),
            None => println!("  ${p:.2}: no solution for {d} in the range"),
        }
    }
    Ok(())
}

fn default_data_dir(repo: &Path) -> PathBuf {
    repo.join("data").join("processed")
}

fn load_company(repo: &Path, a: &Args, ticker: &str) -> R<Company> {
    let dir = a.one("--data-dir").map(PathBuf::from).unwrap_or_else(|| default_data_dir(repo));
    let path = dir.join(format!("{}.json", ticker.to_uppercase()));
    if !path.is_file() {
        return e(format!("No saved data for {} at {}. Run: fa run scripts.analyze {}", ticker.to_uppercase(), path.display(), ticker.to_uppercase()));
    }
    let text = std::fs::read_to_string(&path).map_err(|x| DcfError(format!("{}: {x}", path.display())))?;
    let mut c: Company = serde_json::from_str(&text).map_err(|x| DcfError(format!("{}: {x}", path.display())))?;
    if c.ticker.is_empty() {
        c.ticker = ticker.to_uppercase();
    }
    Ok(c)
}

fn classic(repo: &Path, rest: &[String]) -> R<()> {
    let a = Args { rest };
    a.check(&["--json"], &["--sims", "--years", "--growth", "--wacc", "--terminal-growth", "--seed", "--data-dir"])?;
    let ticker = rest.first().filter(|t| !t.starts_with("--")).ok_or_else(|| DcfError("classic needs a ticker".into()))?;
    let company = load_company(repo, &a, ticker)?;
    let d = DcfAssumptions::default();
    let asm = DcfAssumptions {
        n_simulations: a.num("--sims")?.unwrap_or(d.n_simulations),
        projection_years: a.num("--years")?.unwrap_or(d.projection_years),
        growth_rate_mean: a.num("--growth")?.unwrap_or(d.growth_rate_mean),
        wacc_mean: a.num("--wacc")?.unwrap_or(d.wacc_mean),
        terminal_growth_mean: a.num("--terminal-growth")?.unwrap_or(d.terminal_growth_mean),
        seed: a.num("--seed")?,
        ..d
    };
    let r = run_classic(&company, &asm).map_err(|x| DcfError(format!("Cannot value {}: {x}", ticker.to_uppercase())))?;
    if a.flag("--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ticker": company.ticker, "simulations": asm.n_simulations, "years": asm.projection_years, "seed": r.seed,
                "p10_value": r.p10_value, "median_value": r.median_value, "p90_value": r.p90_value,
                "mean_value": r.mean_value, "std_value": r.std_value, "median_per_share": r.median_per_share,
            }))
            .expect("json")
        );
        return Ok(());
    }
    println!("\n{} ({}) - Monte Carlo DCF", company.company_name, company.ticker);
    println!("  {} simulations - {}y horizon - growth {:.0}% - WACC {:.0}%\n", asm.n_simulations, asm.projection_years, asm.growth_rate_mean * 100.0, asm.wacc_mean * 100.0);
    println!("  Enterprise value (present value; net debt is not subtracted):");
    println!("    P10 : {}", money(r.p10_value));
    println!("    P50 : {}", money(r.median_value));
    println!("    P90 : {}", money(r.p90_value));
    println!("    mean: {}  (std {})", money(r.mean_value), money(r.std_value));
    match r.median_per_share {
        Some(v) => println!("\n  P50 enterprise value per share (net debt is not subtracted): ${v:.2}"),
        None => println!("\n  Per-share: N/A (no shares outstanding in annual data)"),
    }
    Ok(())
}

fn scenarios(repo: &Path, rest: &[String]) -> R<()> {
    let a = Args { rest };
    a.check(&["--json"], &["--data-dir"])?;
    let ticker = rest.first().filter(|t| !t.starts_with("--")).ok_or_else(|| DcfError("scenarios needs a ticker".into()))?;
    let company = load_company(repo, &a, ticker)?;
    let cap = capital_structure_for(&company)?;
    let mut rows = Vec::new();
    for s in scenarios_for(&company.ticker) {
        let r = run_classic(&company, &s.assumptions)?;
        let ps: Vec<f64> = r.simulations.iter().map(|v| (v - cap.net_debt) / cap.shares).collect();
        let sorted = sorted_copy(&ps);
        let qs: Vec<f64> = [10.0, 25.0, 50.0, 75.0, 90.0].iter().map(|q| percentile(&sorted, *q)).collect();
        rows.push((s, r, qs));
    }
    if a.flag("--json") {
        let v: Vec<Value> = rows
            .iter()
            .map(|(s, r, q)| {
                json!({"key": s.key, "label": s.label, "ev_p10": r.p10_value, "ev_p50": r.median_value, "ev_p90": r.p90_value,
                       "per_share": {"p10": q[0], "p25": q[1], "p50": q[2], "p75": q[3], "p90": q[4]}})
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json!({"ticker": company.ticker, "net_debt": cap.net_debt, "shares": cap.shares, "scenarios": v})).expect("json"));
        return Ok(());
    }
    println!("\n{} scenarios - net debt {}, shares {:.1}M, seed {}", company.ticker, money(cap.net_debt), cap.shares / 1e6, super::scenarios::SCENARIO_SEED);
    for (s, r, q) in &rows {
        println!("  {}", s.label);
        println!("    EV P10 {} | P50 {} | P90 {}", money(r.p10_value), money(r.median_value), money(r.p90_value));
        println!("    per share P10 {:.2} | P25 {:.2} | P50 {:.2} | P75 {:.2} | P90 {:.2}", q[0], q[1], q[2], q[3], q[4]);
    }
    Ok(())
}

/// The verdict of a ticker with a model file and no market-rate input. `Ok(None)` when the
/// ticker has no model file (the caller can use the Python path then).
pub fn verdict(repo: &Path, ticker: &str, price: f64, simulations: u32, seed: i64) -> R<Option<crate::Verdict>> {
    if !(price.is_finite() && price > 0.0) {
        return e(format!("--price must be a positive number, got {price}"));
    }
    if seed < 0 {
        return e(format!("seed must be 0 or more, got {seed}"));
    }
    let Some(spec) = ModelSpec::load(repo, ticker)? else { return Ok(None) };
    let r = spec.engine()?.run_paths(simulations as usize, seed as u64, &[])?;
    let p = percentiles(&r.per_share, &[10.0, 25.0, 50.0, 75.0, 90.0]);
    let too_high = chance_price_too_high(&r.per_share, price);
    Ok(Some(crate::Verdict {
        ticker: ticker.to_uppercase(),
        simulations,
        seed,
        price,
        p10: p[0],
        p25: p[1],
        p50: p[2],
        p75: p[3],
        p90: p[4],
        chance_price_too_high: too_high,
        verdict: verdict_band(too_high).to_string(),
        rates: None,
        credit: None,
    }))
}

/// Run `fa dcf <args>`. Returns the exit code.
pub fn run(repo: &Path, args: &[String]) -> u8 {
    let result = match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help") => {
            println!("{USAGE}");
            return 0;
        }
        Some("models") => {
            for m in available_models(repo) {
                println!("{m}");
            }
            Ok(())
        }
        Some("drivers") => drivers(repo, &args[1..]),
        Some("classic") => classic(repo, &args[1..]),
        Some("scenarios") => scenarios(repo, &args[1..]),
        Some(other) => {
            eprintln!("fa: unknown dcf command '{other}'\n\n{USAGE}");
            return 64;
        }
    };
    match result {
        Ok(()) => 0,
        Err(x) => {
            eprintln!("fa: {x}");
            2
        }
    }
}
