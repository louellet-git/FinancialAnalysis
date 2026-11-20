//! `fa dcf auto TICKER` and `fa dcf screen`: the automatic model of one ticker and of a list of tickers
//! (ticket `T-20261119-dcf-auto-model`, parts 1 and 2). An automatic model is a screen, not a reviewed valuation.

use super::auto::*;
use super::calibrate::{FadeCell, FadeFile, ProcessTable};
use super::driver::Ramp;
use super::model::bisect;
use super::paths::{Mods, PathEngine};
use super::process::{ParamSpec, PathsSpec};
use super::stats::{percentile, share_below, sorted_copy};
use super::DcfError;
use mapcheck::registry::Registry;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

type R<T> = Result<T, DcfError>;

fn e<T>(m: impl Into<String>) -> R<T> {
    Err(DcfError(m.into()))
}

/// Fiscal years and quarters that the mapping reads for `fa dcf auto` and `fa dcf screen` (the same number, so the same warnings).
pub const MAP_PERIODS: usize = 10;

pub const BANNER: &str = "Automatic model: a screen, not a reviewed valuation.";

pub const AUTO_USAGE: &str = "\
fa dcf auto TICKER [OPTIONS]
  Make a path model for a ticker from the company facts of the cache, the parameter table and the yield curve,
  and run it. No network, no hand input. Output: the numbers of the model with source and rule (--explain), then
  the output of `fa dcf drivers TICKER --model FILE`, then, with a price, what the price implies.
  A template other than `standard` (bank, insurer, reit, fund) and a mapping status FAIL give no value.
  --cache DIR     company facts (default <repo>/data/raw): <TICKER>/company_facts.json[.gz], rates/gsw_feds200628.csv,
                  <TICKER>/prices.json and SPY/prices.json (else trading/SPY_1d.csv) for the beta
  --price P       market price (else the last close of the price cache, else none)
  --explain       print each number of the model with its source and rule
  --write FILE    write the model as a TOML file (`fa dcf drivers TICKER --model FILE` runs it)
  --table FILE    parameter table (default analysis/priors/process_priors_2026-11-19.toml)
  --as-of DATE    date of the model and of the yield curve (default today)
  --rules K=V,..  change a rule of the builder (a flag is 0 or 1):
                  growth and fade: history_organic, own_sigma_nu, level_kappa, fade_to_cap, long_cap (the level of the
                  fade; else L), long_run (default 0.040: one number L for the fade level and the mean terminal
                  growth, the lower of the 10-year zero yield and this number, the growth of the economy of the FOMC
                  SEP; long_run=0, a negative number or `off`: off, the old model: the fade to the yield and the mean
                  terminal growth = terminal_growth, default 2.25%), fade_years, growth_w (weight of the history),
                  growth_phi, last_floor (default 0: a base year with growth below this value enters the company mean
                  and the first step as this value; -1 is off)
                  margin: margin_pool_se, margin_w, margin_last (the target is the margin of the base year),
                  margin_sigma_nu (weight in years of the table shock in the shock size of the margin; 0 or more;
                  0 = the table value only; default own_sigma_nu)
                  discount: rf (a fixed rate), erp, beta_weight, equity_weight_one, terminal_growth (the mean; it turns
                  long_run off, and a later long_run= in the same text turns it on again: the order matters), terminal_rf
                  (the fade level and the terminal mean are both the 10-year zero yield; wins over long_run), years
                  (horizon), stub, mid_year
                  `off` is allowed for long_run and for the flags only
                  cash flow and bridge: sbc_cost, acq_charge, no_interest_income, bridge_lti, bridge_leases, shares_latest
                  acquisitions: acq_f (a share f of the spend recurs: charge f times the mean spend ratio, and the
                  growth history loses (1 - f) acq_mult of the revenue that each dollar bought), acq_mult, acq_w (default 1: the
                  charge is w times the mean spend ratio, w = the weight of the history in the growth mean; the
                  history stays; acq_w=0: no charge; acq_charge and acq_f win over it), acq_fade (default 1: the acq_w
                  charge falls linearly to zero along the fade of the growth when the company mean is above the fade
                  level, else it is constant; acq_fade=0: constant charge)
  --seed S (7)  --sims N (10000)  --no-attribution  --json  --time   (as `fa dcf drivers --model`)";

pub const SCREEN_USAGE: &str = "\
fa dcf screen [--tickers-file FILE | TICKER ...] [OPTIONS]
  One row for each ticker: the automatic model (`fa dcf auto`) with a few paths. Parallel over tickers; the result
  does not depend on the number of threads. Without a ticker, all tickers of the cache.
  --cache DIR     as for `fa dcf auto`
  --out FILE      write all rows as CSV
  --jobs N        threads (default: all)
  --prices FILE   CSV `ticker,price` (a header line is allowed); else the last close of the price cache (fa prices get)
  --sims N (4000) paths for each ticker       --seed S (7)
  --table FILE    parameter table             --as-of DATE
  --rules K=V,..  change a rule of the builder (the list: fa dcf auto --help)
  --implied       also search the cost of capital of year 1 that gives the price (columns model_rate, implied_rate;
                  \"<x\": the price needs a rate below x, where the rate of some paths is not positive)";

#[derive(Debug, Clone, Default)]
pub(super) struct Opts {
    values: HashMap<String, String>,
    flags: Vec<String>,
    pub(super) positional: Vec<String>,
}

impl Opts {
    pub(super) fn parse(args: &[String], values: &[&str], flags: &[&str]) -> R<Opts> {
        let mut o = Opts::default();
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if values.contains(&a.as_str()) {
                let v = it.next().ok_or_else(|| DcfError(format!("{a} needs a value")))?;
                o.values.insert(a.clone(), v.clone());
            } else if flags.contains(&a.as_str()) {
                o.flags.push(a.clone());
            } else if a.starts_with("--") {
                return e(format!("unknown option '{a}'"));
            } else {
                o.positional.push(a.clone());
            }
        }
        Ok(o)
    }
    pub(super) fn get(&self, k: &str) -> Option<&str> {
        self.values.get(k).map(String::as_str)
    }
    pub(super) fn flag(&self, k: &str) -> bool {
        self.flags.iter().any(|f| f == k)
    }
    pub(super) fn num<T: std::str::FromStr>(&self, k: &str, default: T) -> R<T> {
        match self.get(k) {
            None => Ok(default),
            Some(v) => v.parse().or_else(|_| e(format!("bad value for {k}: '{v}'"))),
        }
    }
}

/// The parameter table and the fade cells of a file.
pub fn load_table(path: &Path) -> R<(ProcessTable, Vec<FadeCell>)> {
    let text = std::fs::read_to_string(path).map_err(|x| DcfError(format!("{}: {x}", path.display())))?;
    Ok((ProcessTable::from_toml(&text)?, FadeFile::from_toml(&text)?.fade))
}

pub(super) fn cache_dir(repo: &Path, o: &Opts) -> PathBuf {
    o.get("--cache").map(PathBuf::from).unwrap_or_else(|| repo.join("data").join("raw"))
}

pub(super) fn table_path(repo: &Path, o: &Opts) -> PathBuf {
    o.get("--table").map(PathBuf::from).unwrap_or_else(|| repo.join(DEFAULT_TABLE))
}

pub(super) fn as_of_of(o: &Opts) -> R<(String, i64)> {
    match o.get("--as-of") {
        Some(t) => match mapcheck::dates::parse(t) {
            Some(d) => Ok((t.to_string(), d)),
            None => e(format!("--as-of '{t}' is not YYYY-MM-DD")),
        },
        None => Ok((crate::market::today_iso(), mapcheck::dates::today())),
    }
}

fn print_rows(rows: &[ExplainRow]) {
    println!("\nTHE NUMBERS OF THE MODEL (value, source, rule)");
    for r in rows {
        println!("  {}: {}\n      source: {}\n      rule:   {}", r.input, r.value, r.source, r.rule);
    }
}

fn parse_price(text: &str) -> R<f64> {
    let p: f64 = text.parse().map_err(|_| DcfError(format!("bad --price '{text}'")))?;
    if !(p.is_finite() && p > 0.0) {
        return e(format!("--price must be a positive number, got {p}"));
    }
    Ok(p)
}

/// `fa dcf auto`. Returns the exit code: 0 a value, 2 an error, 3 no value (a refusal).
pub fn auto(repo: &Path, args: &[String]) -> u8 {
    match auto_inner(repo, args) {
        Ok(code) => code,
        Err(x) => {
            eprintln!("fa: {x}");
            2
        }
    }
}

fn auto_inner(repo: &Path, args: &[String]) -> R<u8> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{AUTO_USAGE}");
        return Ok(0);
    }
    let o = Opts::parse(args, &["--cache", "--price", "--write", "--table", "--as-of", "--seed", "--sims", "--rules"], &["--explain", "--no-attribution", "--json", "--time"])?;
    let ticker = o.positional.first().ok_or_else(|| DcfError("auto needs a ticker".into()))?.to_uppercase();
    if o.positional.len() > 1 {
        return e(format!("auto takes one ticker, got {}", o.positional.len()));
    }
    // One line when the cached data that the model reads is older than its maximum age (not for a past date or another cache).
    if o.get("--as-of").is_none() && cache_dir(repo, &o) == repo.join("data").join("raw") {
        use crate::sync::dataset::DataSet;
        let mut sets = vec![DataSet::Sec, DataSet::Estimates];
        if o.get("--price").is_none() {
            sets.push(DataSet::Prices);
        }
        if let Some(w) = crate::sync::old_data_warning(repo, &ticker, &sets) {
            eprintln!("fa: {w}");
        }
    }
    let cache = cache_dir(repo, &o);
    let (table, fade) = load_table(&table_path(repo, &o))?;
    let (as_of, as_of_day) = as_of_of(&o)?;
    let price = o.get("--price").map(parse_price).transpose()?;
    let rules = super::auto_pit::parse_rules(o.get("--rules").unwrap_or(""))?;
    let json = o.flag("--json");
    let say = |s: &str| {
        if !json {
            println!("{s}");
        }
    };

    let t0 = Instant::now();
    let registry = Registry::embedded().map_err(DcfError)?;
    let report = mapcheck::run::check_ticker(&registry, &cache, &ticker, MAP_PERIODS, as_of_day, true);
    let inputs = inputs_from_report(&registry, &report);
    let map_ms = t0.elapsed().as_secs_f64() * 1000.0;
    say(&format!("{ticker} automatic model - {BANNER}"));
    say(&format!("  company: {}; mapping status {}; template {}; {} fiscal years", if inputs.name.is_empty() { "(unknown)" } else { &inputs.name }, inputs.status, inputs.template, inputs.years.len()));
    let curve = match load_curve(&cache, &as_of) {
        Ok(c) => Some(c),
        Err(x) => {
            eprintln!("fa: no yield curve ({x})");
            None
        }
    };
    let index = index_from_cache(&cache);
    let market = market_from_cache(&cache, &ticker, &as_of, price, &rules, curve.as_ref(), index.as_deref());
    let model = match build(&inputs, &table, &fade, &market, &rules, BuildOptions::default()) {
        Ok(m) => m,
        Err(r) => {
            println!("NO VALUE: {r}");
            if matches!(&r, Refusal::Template(t) if ["bank", "insurer", "reit", "fund"].contains(&t.as_str())) {
                println!("  next step: a bank, an insurer, a reit and a fund need other models (excess return, dividends, FFO); see the ticket T-20261119-dcf-auto-model.");
            }
            if matches!(r, Refusal::NoData(_)) {
                println!("  The automatic model reads the SEC company facts of the cache ({}). {ticker} has none there (a company with no XBRL facts, for example a private company): use a hand model.", cache.join(&ticker).display());
            }
            if inputs.years.is_empty() && !matches!(r, Refusal::NoData(_)) {
                println!("  {ticker} has no fiscal year with values in the company facts of the cache ({} fiscal years): the automatic model needs the facts of at least 3 years. Use a hand model.", inputs.years.len());
            }
            return Ok(3);
        }
    };
    if let Some(path) = o.get("--write") {
        std::fs::write(path, &model.toml).map_err(|x| DcfError(format!("{path}: {x}")))?;
        say(&format!("  model written to {path}"));
    }
    if o.flag("--explain") && !json {
        print_rows(&model.rows);
    }
    if !model.warnings.is_empty() {
        say("\nWARNINGS");
        for w in &model.warnings {
            say(&format!("  - {w}"));
        }
    }
    say(&format!("  model time: mapping {map_ms:.0} ms"));
    // the engine output of the model file
    let mut run_args: Vec<String> = vec![ticker.clone()];
    for k in ["--seed", "--sims"] {
        if let Some(v) = o.get(k) {
            run_args.extend([k.to_string(), v.to_string()]);
        }
    }
    if let Some(p) = market.price {
        run_args.extend(["--price".into(), format!("{p}")]);
    }
    for k in ["--no-attribution", "--json", "--time"] {
        if o.flag(k) {
            run_args.push(k.to_string());
        }
    }
    super::paths_cli::run(&model.toml, &run_args)?;
    if let (Some(p), false) = (market.price, json) {
        let n: usize = o.num("--sims", 10_000)?;
        let seed: u64 = o.num("--seed", 7)?;
        let spec = PathsSpec::from_toml(&model.toml)?;
        print_implied(&spec, p, n, seed, false)?;
    }
    Ok(0)
}

/// P50 of a spec.
fn p50_of(spec: &PathsSpec, n: usize, seed: u64, serial: bool) -> R<f64> {
    let eng = PathEngine::new(spec)?;
    let sim = if serial { eng.simulate_serial(n, seed, &Mods::default())? } else { eng.simulate(n, seed, &Mods::default(), None, false)? };
    Ok(sim.p50())
}

/// The long-run growth (the level that the company growth fades to) that gives a P50 equal to `price`: the end of the
/// drift of the growth process moves; the company mean stays. `None` if the price is outside the search range.
pub fn implied_long_run_growth(spec: &PathsSpec, price: f64, n: usize, seed: u64, serial: bool) -> R<Option<f64>> {
    let mu = spec.process.get("growth").map(|p| p.target.mean()).ok_or_else(|| DcfError("the model has no growth process".into()))?;
    let ramp = spec.process["growth"].drift.as_ref().map_or(fade_ramp_years(4.0, spec.projection_years), |d| d.ramp_years);
    bisect(
        |x| {
            let mut s = spec.clone();
            s.process.get_mut("growth").expect("growth").drift = Some(Ramp { start: 0.0, end: x - mu, ramp_years: ramp });
            p50_of(&s, n, seed, serial)
        },
        -0.10,
        0.40,
        price,
    )
}

/// The target margin (the long-run free cash flow margin before SBC) that gives a P50 equal to `price`.
pub fn implied_target_margin(spec: &PathsSpec, price: f64, n: usize, seed: u64, serial: bool) -> R<Option<f64>> {
    bisect(
        |x| {
            let mut s = spec.clone();
            let m = s.process.get_mut("margin").ok_or_else(|| DcfError("the model has no margin process".into()))?;
            m.target = match &m.target {
                ParamSpec::Draw(d) => ParamSpec::Draw(super::process::DrawSpec { mean: x, ..d.clone() }),
                ParamSpec::Fixed(_) => ParamSpec::Fixed(x),
            };
            if let ParamSpec::Draw(d) = &mut m.target {
                d.lo = Some(d.lo.map_or(x, |l| l.min(x))).filter(|l| *l <= x);
                d.hi = Some(d.hi.map_or(x, |h| h.max(x))).filter(|h| *h >= x);
            }
            p50_of(&s, n, seed, serial)
        },
        -0.30,
        0.70,
        price,
    )
}

/// The half-life of the fade that gives a P50 equal to `price`: a scan of the ramp length (a whole number of
/// years) with the end of the drift fixed. `Ok(None)` when the model has no fade (the company mean is not above the cap).
pub fn implied_fade_half_life(spec: &PathsSpec, price: f64, n: usize, seed: u64, serial: bool) -> R<Option<(usize, f64)>> {
    let years = spec.projection_years;
    let Some(d) = spec.process.get("growth").and_then(|p| p.drift.clone()) else { return Ok(None) };
    let p50 = |ramp: usize| -> R<f64> {
        let mut s = spec.clone();
        s.process.get_mut("growth").expect("growth").drift = Some(Ramp { ramp_years: ramp, ..d.clone() });
        p50_of(&s, n, seed, serial)
    };
    // a fade to a level below the company mean: a longer ramp is a higher value
    let (mut prev_n, mut prev_v) = (2usize, p50(2)?);
    if prev_v >= price {
        return Ok(Some((2, 0.0)));
    }
    for ramp in 3..=60 {
        let v = p50(ramp)?;
        if v >= price {
            let _ = (prev_n, prev_v);
            let share = (years - 1) as f64 / (ramp - 1) as f64;
            let h = if share < 1.0 { -((years - 1) as f64) / (1.0 - share).log2() } else { 0.0 };
            return Ok(Some((ramp, h)));
        }
        prev_n = ramp;
        prev_v = v;
    }
    Ok(None)
}

fn print_implied(spec: &PathsSpec, price: f64, n: usize, seed: u64, serial: bool) -> R<()> {
    println!("\nWHAT THE PRICE ${price:.2} IMPLIES (bisection on the P50 of this model; one number moves, the others stay)");
    let mu = spec.process["growth"].target.mean();
    match implied_long_run_growth(spec, price, n, seed, serial)? {
        Some(x) => println!("  long-run growth (the level that the company growth fades to by year {}): {:.2}% (company mean {:.2}%)", spec.projection_years, x * 100.0, mu * 100.0),
        None => println!("  long-run growth: no solution between -10% and +40%"),
    }
    match implied_target_margin(spec, price, n, seed, serial)? {
        Some(x) => println!("  target free cash flow margin before SBC: {:.2}% (model: {:.2}%)", x * 100.0, spec.process["margin"].target.mean() * 100.0),
        None => println!("  target margin: no solution between -30% and +70%"),
    }
    match implied_fade_half_life(spec, price, n, seed, serial)? {
        Some((ramp, h)) => println!("  fade of the company growth: a ramp of {ramp} years, a half-life of about {h:.1} years (model: ramp {} years)", spec.process["growth"].drift.as_ref().map_or(0, |d| d.ramp_years)),
        None if spec.process["growth"].drift.is_none() => println!("  fade of the company growth: the model has none (the company mean is not above the cap)"),
        None => println!("  fade of the company growth: no ramp of 2 to 60 years gives the price"),
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// The screen
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenValue {
    pub size: String,
    pub base_revenue: f64,
    pub g_start: f64,
    pub g_long: f64,
    pub m_start: f64,
    pub m_target: f64,
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
    /// The mean of the value for each share over the paths.
    pub mean: f64,
    pub price: Option<f64>,
    pub share_below: Option<f64>,
    pub implied_g: Option<f64>,
    /// Enterprise value at the P50 over the base revenue.
    pub ev_revenue: f64,
    /// The cost of capital of year 1 at the means (`--implied`: also the one that gives the price).
    pub model_rate: f64,
    pub implied_rate: Option<f64>,
    /// The implied rate is only a bound: the price needs a lower rate than the search can reach.
    pub implied_rate_below: bool,
    pub sbc_ratio: f64,
    pub net_debt: f64,
    pub shares: f64,
    pub beta: f64,
    pub equity_weight: f64,
    /// Share of the terminal value in the enterprise value (`E[PV TV] / E[EV]` over the paths).
    pub tv_share: f64,
    /// The acquisition charge of the model (a share of revenue; 0 when off).
    pub acq_ratio: f64,
    pub warnings: usize,
    pub n_hist: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenRow {
    pub ticker: String,
    pub name: String,
    pub status: String,
    pub template: String,
    /// The value, or the key and the text of the refusal.
    pub outcome: Result<ScreenValue, (String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScreenTiming {
    pub model_s: f64,
    pub sim_s: f64,
}

pub struct ScreenConfig<'a> {
    pub table: &'a ProcessTable,
    pub fade: &'a [FadeCell],
    pub rules: Rules,
    pub sims: usize,
    pub seed: u64,
    /// Search the discount rate that gives the price (`--implied`).
    pub implied_rate: bool,
}

/// The rows for a list of company inputs. `market_of` gives the market inputs of a ticker. Stage 1 builds the
/// models, stage 2 simulates them; each ticker runs on one thread, so the result does not depend on the number of
/// threads. `jobs`: number of threads (`None`: the global pool).
pub fn screen_models(inputs: &[AutoInputs], market_of: &(dyn Fn(&str) -> Market + Sync), cfg: &ScreenConfig, jobs: Option<usize>) -> R<(Vec<ScreenRow>, ScreenTiming)> {
    let run = || -> R<(Vec<ScreenRow>, ScreenTiming)> {
        let t0 = Instant::now();
        let built: Vec<Result<(AutoModel, PathsSpec, Market), Refusal>> = inputs
            .par_iter()
            .map(|inp| {
                let market = market_of(&inp.ticker);
                let m = build(inp, cfg.table, cfg.fade, &market, &cfg.rules, BuildOptions::default())?;
                let spec = PathsSpec::from_toml(&m.toml).map_err(|x| Refusal::NoPriors(format!("the model file does not parse: {x}")))?;
                Ok((m, spec, market))
            })
            .collect();
        let model_s = t0.elapsed().as_secs_f64();
        let t1 = Instant::now();
        let rows: Vec<ScreenRow> = inputs
            .par_iter()
            .zip(built.par_iter())
            .map(|(inp, b)| {
                let outcome = match b {
                    Err(r) => Err((r.key(), r.to_string())),
                    Ok((m, spec, market)) => simulate_row(m, spec, market, cfg).map_err(|x| ("engine error".to_string(), x.to_string())),
                };
                ScreenRow { ticker: inp.ticker.clone(), name: inp.name.clone(), status: inp.status.clone(), template: inp.template.clone(), outcome }
            })
            .collect();
        Ok((rows, ScreenTiming { model_s, sim_s: t1.elapsed().as_secs_f64() }))
    };
    match jobs {
        Some(n) => {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(n.max(1)).build().map_err(|x| DcfError(x.to_string()))?;
            pool.install(run)
        }
        None => run(),
    }
}

fn simulate_row(m: &AutoModel, spec: &PathsSpec, market: &Market, cfg: &ScreenConfig) -> R<ScreenValue> {
    let eng = PathEngine::new(spec)?;
    let sim = eng.simulate_serial(cfg.sims, cfg.seed, &Mods::default())?;
    let s = sorted_copy(&sim.per_share);
    let (p10, p50, p90) = (percentile(&s, 10.0), percentile(&s, 50.0), percentile(&s, 90.0));
    let sm = &m.summary;
    let (mut share, mut implied, mut implied_rate, mut below) = (None, None, None, false);
    let model_rate = model_rate_of(sm);
    if let Some(p) = market.price {
        share = Some(share_below(&sim.per_share, p));
        implied = implied_long_run_growth(spec, p, cfg.sims.min(4000), cfg.seed, true)?;
        if cfg.implied_rate {
            if let Ok(Some((x, b))) = implied_discount_rate(spec, sm, p, cfg.sims.min(4000), cfg.seed) {
                implied_rate = Some(x);
                below = b;
            }
        }
    }
    Ok(ScreenValue {
        size: sm.size.clone(),
        base_revenue: sm.base_revenue,
        g_start: sm.growth_start,
        g_long: sm.growth_long_run,
        m_start: sm.margin_start,
        m_target: sm.margin_target,
        p10,
        p50,
        p90,
        mean: sim.mean(),
        price: market.price,
        share_below: share,
        implied_g: implied,
        ev_revenue: (p50 * sm.shares + sm.net_debt) / sm.base_revenue,
        sbc_ratio: sm.sbc_ratio,
        net_debt: sm.net_debt,
        shares: sm.shares,
        beta: sm.beta,
        equity_weight: sm.equity_weight,
        tv_share: sim.tv_share(),
        acq_ratio: sm.acq_ratio,
        model_rate,
        implied_rate,
        implied_rate_below: below,
        warnings: m.warnings.len(),
        n_hist: sm.growth_obs,
    })
}

pub const CSV_HEADER: &str = "ticker,status,template,size,base_revenue,growth_start,growth_long_run,margin_start,margin_target,p10,p50,p90,price,p50_over_price,share_below_price,implied_long_run_growth,ev_over_revenue,warnings,growth_values,refusal,model_rate,implied_rate,sbc_ratio,net_debt,shares,beta,equity_weight,tv_share,acq_ratio,mean";

/// The cost of capital of year 1 at the means of the model (the 10-year zero yield as the risk-free rate).
pub fn model_rate_of(sm: &Summary) -> f64 {
    let w = sm.equity_weight;
    w * (sm.rf + sm.beta * sm.erp) + (1.0 - w) * (sm.rf + sm.kd_spread) * (1.0 - sm.tax)
}

/// The cost of capital of year 1 that gives a P50 equal to `price`: the mean of the equity risk premium moves (the
/// risk-free rate, and so the cap of the terminal growth, stay); the cost of capital moves by `w beta` for each point.
/// The other numbers stay. `None`: no solution for a premium from -4% to +20%. The flag is true when the price needs a
/// lower rate than the search can reach (a path with a cost of capital that is not positive counts as a value above any
/// price).
pub fn implied_discount_rate(spec: &PathsSpec, sm: &Summary, price: f64, n: usize, seed: u64) -> R<Option<(f64, bool)>> {
    let base = model_rate_of(sm);
    let slope = sm.equity_weight * sm.beta;
    if slope <= 0.0 {
        return Ok(None);
    }
    let mut err_max = f64::NEG_INFINITY;
    let x = bisect(
        |x| {
            let mut s = spec.clone();
            s.discount.erp = match &s.discount.erp {
                ParamSpec::Draw(d) => ParamSpec::Draw(super::process::DrawSpec { mean: x, lo: d.lo.map(|l| l.min(l + x - sm.erp)), ..d.clone() }),
                ParamSpec::Fixed(_) => ParamSpec::Fixed(x),
            };
            match p50_of(&s, n, seed, true) {
                Ok(v) => Ok(v),
                Err(_) => {
                    err_max = err_max.max(x);
                    Ok(f64::MAX)
                }
            }
        },
        -0.04,
        0.20,
        price,
    )?;
    Ok(x.map(|x| (base + slope * (x - sm.erp), err_max >= x - 1e-6)))
}

pub(super) fn num(x: f64, d: usize) -> String {
    format!("{x:.d$}")
}

/// One CSV line of a row.
pub fn csv_line(r: &ScreenRow) -> String {
    match &r.outcome {
        Ok(v) => format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},",
            r.ticker,
            r.status,
            r.template,
            v.size,
            num(v.base_revenue, 0),
            num(v.g_start, 4),
            num(v.g_long, 4),
            num(v.m_start, 4),
            num(v.m_target, 4),
            num(v.p10, 2),
            num(v.p50, 2),
            num(v.p90, 2),
            v.price.map_or(String::new(), |p| num(p, 2)),
            v.price.map_or(String::new(), |p| num(v.p50 / p, 3)),
            v.share_below.map_or(String::new(), |s| num(s, 3)),
            v.implied_g.map_or(String::new(), |g| num(g, 4)),
            num(v.ev_revenue, 2),
            v.warnings,
            v.n_hist
        ) + &format!(
            ",{},{},{},{},{},{},{},{},{},{}",
            num(v.model_rate, 4),
            v.implied_rate.map_or(String::new(), |x| format!("{}{}", if v.implied_rate_below { "<" } else { "" }, num(x, 4))),
            num(v.sbc_ratio, 4),
            num(v.net_debt, 0),
            num(v.shares, 0),
            num(v.beta, 3),
            num(v.equity_weight, 3),
            num(v.tv_share, 3),
            num(v.acq_ratio, 4),
            num(v.mean, 2)
        ),
        Err((key, _)) => format!("{},{},{},,,,,,,,,,,,,,,,,{},,,,,,,,,,", r.ticker, r.status, r.template, key.replace(',', ";")),
    }
}

pub fn read_prices(path: &str) -> R<HashMap<String, f64>> {
    let text = std::fs::read_to_string(path).map_err(|x| DcfError(format!("{path}: {x}")))?;
    let mut out = HashMap::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split(',');
        let (Some(t), Some(p)) = (it.next(), it.next()) else { return e(format!("{path} line {}: expected ticker,price", i + 1)) };
        match p.trim().parse::<f64>() {
            Ok(p) if p.is_finite() && p > 0.0 => {
                out.insert(t.trim().to_uppercase(), p);
            }
            _ if i == 0 => {} // a header line
            _ => return e(format!("{path} line {}: bad price '{p}'", i + 1)),
        }
    }
    Ok(out)
}

fn quantile_text(mut v: Vec<f64>, d: usize) -> String {
    if v.is_empty() {
        return "no values".into();
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let s = |q: f64| num(percentile(&v, q), d);
    format!("P5 {} | P25 {} | P50 {} | P75 {} | P95 {} (n = {})", s(5.0), s(25.0), s(50.0), s(75.0), s(95.0), v.len())
}

/// `--jobs N` (`None`: the global pool).
pub(super) fn jobs_of(o: &Opts) -> R<Option<usize>> {
    match o.get("--jobs") {
        Some(j) => Ok(Some(j.parse().or_else(|_| e(format!("bad value for --jobs: '{j}'")))?)),
        None => Ok(None),
    }
}

/// The tickers of the command line: the names, then `--tickers-file`; with none, all tickers of the cache. No double.
pub(super) fn tickers_of(o: &Opts, cache: &Path) -> R<Vec<String>> {
    let mut tickers: Vec<String> = o.positional.iter().map(|t| t.to_uppercase()).collect();
    if let Some(f) = o.get("--tickers-file") {
        tickers.extend(mapcheck::cli::read_ticker_file(f).map_err(DcfError)?);
    }
    if tickers.is_empty() {
        tickers = super::panel::cache_tickers(cache).map_err(DcfError)?;
    }
    let mut seen = std::collections::HashSet::new();
    tickers.retain(|t| seen.insert(t.clone()));
    Ok(tickers)
}

/// The mapping of the company facts of the cache to the standard variables, with the checks, for a list of tickers.
pub(super) fn map_inputs(registry: &Registry, cache: &Path, tickers: &[String], as_of_day: i64, jobs: Option<usize>) -> R<Vec<AutoInputs>> {
    let run_map = || -> Vec<AutoInputs> {
        tickers
            .par_iter()
            .map(|t| {
                let r = mapcheck::run::check_ticker(registry, cache, t, MAP_PERIODS, as_of_day, true);
                inputs_from_report(registry, &r)
            })
            .collect()
    };
    Ok(match jobs {
        Some(n) => rayon::ThreadPoolBuilder::new().num_threads(n.max(1)).build().map_err(|x| DcfError(x.to_string()))?.install(run_map),
        None => run_map(),
    })
}

/// `fa dcf screen`. Returns the exit code.
pub fn screen(repo: &Path, args: &[String]) -> u8 {
    match screen_inner(repo, args) {
        Ok(()) => 0,
        Err(x) => {
            eprintln!("fa: {x}");
            2
        }
    }
}

fn screen_inner(repo: &Path, args: &[String]) -> R<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{SCREEN_USAGE}");
        return Ok(());
    }
    let o = Opts::parse(args, &["--cache", "--out", "--jobs", "--prices", "--sims", "--seed", "--table", "--as-of", "--tickers-file", "--rules"], &["--implied"])?;
    let cache = cache_dir(repo, &o);
    let (table, fade) = load_table(&table_path(repo, &o))?;
    let (as_of, as_of_day) = as_of_of(&o)?;
    let sims: usize = o.num("--sims", 4000)?;
    let seed: u64 = o.num("--seed", 7)?;
    let jobs = jobs_of(&o)?;
    if sims == 0 {
        return e("--sims must be positive");
    }
    let tickers = tickers_of(&o, &cache)?;
    let prices = match o.get("--prices") {
        Some(p) => read_prices(p)?,
        None => HashMap::new(),
    };
    let rules = super::auto_pit::parse_rules(o.get("--rules").unwrap_or(""))?;
    let registry = Registry::embedded().map_err(DcfError)?;

    // stage 1: the mapping (company facts of the cache to the standard variables, with the checks)
    let t0 = Instant::now();
    let inputs = map_inputs(&registry, &cache, &tickers, as_of_day, jobs)?;
    let map_s = t0.elapsed().as_secs_f64();

    let (curve, index) = curve_and_index(&cache, &as_of);
    let market_of = |t: &str| -> Market {
        let price = prices.get(t).copied();
        market_from_cache(&cache, t, &as_of, price, &rules, curve.as_ref(), index.as_deref())
    };
    let cfg = ScreenConfig { table: &table, fade: &fade, rules: rules.clone(), sims, seed, implied_rate: o.flag("--implied") };
    let (rows, timing) = screen_models(&inputs, &market_of, &cfg, jobs)?;
    if let Some(path) = o.get("--out") {
        let mut text = String::from(CSV_HEADER);
        text.push('\n');
        for r in &rows {
            text.push_str(&csv_line(r));
            text.push('\n');
        }
        std::fs::write(path, text).map_err(|x| DcfError(format!("{path}: {x}")))?;
    }

    println!("SCREEN of {} tickers, {} paths each, seed {seed}, as of {as_of}. {BANNER}", rows.len(), sims);
    if rows.len() <= 40 {
        println!("{CSV_HEADER}");
        for r in &rows {
            println!("{}", csv_line(r));
        }
    }
    let values: Vec<&ScreenValue> = rows.iter().filter_map(|r| r.outcome.as_ref().ok()).collect();
    println!("\nRESULT: {} tickers with a value, {} refused", values.len(), rows.len() - values.len());
    let mut by_reason: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for r in &rows {
        if let Err((k, _)) = &r.outcome {
            *by_reason.entry(k.clone()).or_default() += 1;
        }
    }
    let mut reasons: Vec<(String, usize)> = by_reason.into_iter().collect();
    reasons.sort_by_key(|r| std::cmp::Reverse(r.1));
    for (k, n) in reasons {
        println!("  refused {n:>4}  {k}");
    }
    println!("\nTIME by stage ({} threads): mapping {map_s:.2} s | models {:.3} s | simulation {:.2} s | total {:.2} s", jobs.unwrap_or_else(rayon::current_num_threads), timing.model_s, timing.sim_s, map_s + timing.model_s + timing.sim_s);
    println!("\nSANITY (tickers with a value)");
    println!("  EV at the P50 over base revenue: {}", quantile_text(values.iter().map(|v| v.ev_revenue).collect(), 2));
    println!("  P50 value per share: {}", quantile_text(values.iter().map(|v| v.p50).collect(), 2));
    println!("  P90 / P10 of the value: {}", quantile_text(values.iter().filter(|v| v.p10 > 0.0).map(|v| v.p90 / v.p10).collect(), 2));
    println!("  growth start: {}", quantile_text(values.iter().map(|v| v.g_start * 100.0).collect(), 1));
    println!("  margin target (%): {}", quantile_text(values.iter().map(|v| v.m_target * 100.0).collect(), 1));
    println!("  tickers with P50 value <= 0: {}", values.iter().filter(|v| v.p50 <= 0.0).count());
    let with_price: Vec<&&ScreenValue> = values.iter().filter(|v| v.price.is_some()).collect();
    if !with_price.is_empty() {
        println!("  tickers with a price: {}", with_price.len());
    }
    Ok(())
}

/// The yield curve of the date and the market index of the cache (a message when the curve is missing).
pub(super) fn curve_and_index(cache: &Path, as_of: &str) -> (Option<CurveNumbers>, Option<Vec<(String, f64)>>) {
    let curve = load_curve(cache, as_of).ok();
    if curve.is_none() {
        eprintln!("fa: no yield curve in the cache: the rate is the default {:.1}%", DEFAULT_RF * 100.0);
    }
    (curve, load_index(cache))
}

fn load_index(cache: &Path) -> Option<Vec<(String, f64)>> {
    index_from_cache(cache)
}
