//! `fa dcf implied`: what the market price implies for each ticker, and the medians of groups
//! (ticket `T-20261120-dcf-market-implied`). A fixed procedure with no random part.
//!
//! A market-implied value is a measurement. It is never an input of the model (`docs/QUALITY-STANDARDS.md`, section 6):
//! no default and no rule of the builder reads a result of this file.
//!
//! The value of a ticker is the value of the one path at the means (`PathEngine::deterministic`: every parameter at its
//! mean, every shock zero). Three searches, each moves one number and keeps all others; each builds the model again with
//! changed `Rules`, so every cap, fade and guard of the builder applies:
//! 1. the mean of the equity risk premium (so the cost of capital),
//! 2. the growth (the company mean and the growth of year 1),
//! 3. the free cash flow margin (the start and the target).
//!
//! A last search finds the premium for which the median value over price of the whole list is 1.

use super::auto::*;
use super::auto_cli::{as_of_of, cache_dir, curve_and_index, jobs_of, load_table, map_inputs, map_inputs_pit, model_rate_of, num, read_prices, table_path, tickers_of, Opts, BANNER};
use super::calibrate::{FadeCell, ProcessTable};
use super::model::bisect;
use super::paths::{PathEngine, F_FCFF, F_REV};
use super::panel::BUCKET_LABELS;
use super::process::PathsSpec;
use super::stats::{percentile, sorted_copy};
use super::DcfError;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::time::Instant;

type R<T> = Result<T, DcfError>;

fn e<T>(m: impl Into<String>) -> R<T> {
    Err(DcfError(m.into()))
}

pub const IMPLIED_USAGE: &str = "\
fa dcf implied [--tickers-file FILE | TICKER ...] [OPTIONS]
  What the market price implies, for each ticker and for groups of tickers. A fixed procedure with no random part: the
  value is the value of the one path at the means (every parameter at its mean, every shock zero); no seed, no
  simulation. A market-implied value is a measurement, never an input of the model. Three searches, each moves one
  number and keeps all others (the model is built again for each step, so every cap and guard of the builder applies):
    cost of capital  the mean of the equity risk premium (search range -4% to +20%)
    growth           a shift of the company mean of the growth and of the growth of year 1 (-30 to +60 points)
    margin           a shift of the start and of the target of the free cash flow margin (-50 to +50 points)
  No solution in the range gives the bound and a flag in the column `flags` (`<`: the price needs a lower number than the
  bound, `>`: a higher number; `nm`: the value is not monotone in the number on the ends of the range; such a row has
  no number for that search and is not in the quartiles of that measure). A `<` can also be the floor of the rate at 0%
  (the engine gives no value for a cost of capital that is not positive), not the low end of the range.
  Then the premium that the whole list implies (the median of value / price is 1) and, for each group, the median of the
  value at that premium over the price. Without a ticker: all tickers of the cache.
  --cache DIR     as for `fa dcf auto`
  --as-of DATE    date of the model and of the yield curve (default today). With a date, the facts are the facts that were
                  public on it (filed on or before it); the 10-K of the cache fills a missing input only if its filing date is on or before the date
  --price-date D  a past date: the price is the last close on or before D (not more than 10 days older), the yield curve,
                  the beta and the model date are those of D, and the facts are those of D unless --as-of is given
  --prices FILE   CSV `ticker,price`; else the last close of the price cache (fa prices get)
  --table FILE    parameter table
  --rules K=V,..  change a rule of the builder (the list: fa dcf auto --help)
  --groups FILE   CSV lines `ticker,sector,industry` (a header line is allowed): the groups by sector and by industry
                  (industries with 5 tickers or more). It wins over the SIC code. Without it: the SIC code of the cache
                  (sector = Fama-French 12 name, industry = code and description; the code of today).
  --out FILE      write one CSV row for each ticker
  --jobs N        threads (default: all); the result does not depend on N";

/// The value that a search uses for a number at which the engine gives no value (for example a cost of capital that is not
/// positive): a number above any price, and finite (a ratio to a price stays finite).
pub const NO_VALUE: f64 = 1e30;

/// The value over the price at a premium, or `None` when the engine gives no value there (no number is printed for it).
pub fn value_over_price_at(p: Option<Point>, price: f64) -> Option<f64> {
    p.map(|p| p.value / price)
}

/// The search ranges (the number that moves).
pub const ERP_RANGE: (f64, f64) = (-0.04, 0.20);
pub const GROWTH_RANGE: (f64, f64) = (-0.30, 0.60);
pub const MARGIN_RANGE: (f64, f64) = (-0.50, 0.50);
/// Smallest number of tickers of an industry that gets its own line.
pub const INDUSTRY_MIN: usize = 5;

// ---------------------------------------------------------------------------------------------
// The search
// ---------------------------------------------------------------------------------------------

/// Where the solution of a search is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bound {
    /// The number solves the equation.
    Exact,
    /// No solution in the range: the price needs a lower number than `x` (the low end of the range).
    Below,
    /// No solution in the range: the price needs a higher number than `x` (the high end of the range).
    Above,
}

/// The result of a search. `x` is `NaN` when the ends of the range do not have the expected direction (no search).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Found {
    pub x: f64,
    pub bound: Bound,
    /// The value has the expected direction on the ends of the range and at the model number.
    pub monotone: bool,
}

impl Found {
    fn sign(&self) -> &'static str {
        match self.bound {
            Bound::Exact => "",
            Bound::Below => "<",
            Bound::Above => ">",
        }
    }
}

/// Solve `f(x) = target` on `[lo, hi]`. `increasing`: the direction that the value must have (a higher premium gives a
/// lower value). `base`: the model number and its value, to check the direction on three points. The solution is checked
/// by one more evaluation: a jump of `f` (the engine gave no value for a part of the range) gives a bound, not a number.
pub fn solve<F: FnMut(f64) -> R<f64>>(mut f: F, lo: f64, hi: f64, target: f64, increasing: bool, base: Option<(f64, f64)>) -> R<Found> {
    let (vl, vh) = (f(lo)?, f(hi)?);
    let s = if increasing { 1.0 } else { -1.0 };
    let (a, b, t) = (s * vl, s * vh, s * target);
    let mut monotone = a <= b;
    if let Some((x0, v0)) = base {
        if x0 > lo && x0 < hi {
            monotone &= a <= s * v0 && s * v0 <= b;
        }
    }
    if a > b {
        return Ok(Found { x: f64::NAN, bound: Bound::Exact, monotone: false });
    }
    if t < a {
        return Ok(Found { x: lo, bound: Bound::Below, monotone });
    }
    if t > b {
        return Ok(Found { x: hi, bound: Bound::Above, monotone });
    }
    let Some(x) = bisect(&mut f, lo, hi, target)? else {
        return Ok(Found { x: if t <= a { lo } else { hi }, bound: if t <= a { Bound::Below } else { Bound::Above }, monotone });
    };
    let v = f(x)?;
    if (v - target).abs() > 1e-6 * target.abs().max(1.0) {
        // a jump: the target is in a gap that the function does not reach. The side with the value nearer to the target
        // is the side that the engine reaches; the price needs a number beyond it, in the direction of the other side.
        let eps = (hi - lo) * 1e-9;
        let (l, r) = (s * f(x - eps)?, s * f(x + eps)?);
        let bound = if (r - t).abs() <= (l - t).abs() { Bound::Below } else { Bound::Above };
        return Ok(Found { x, bound, monotone });
    }
    Ok(Found { x, bound: Bound::Exact, monotone })
}

// ---------------------------------------------------------------------------------------------
// The value at the means
// ---------------------------------------------------------------------------------------------

/// The value of the path at the means and the numbers that the report shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Value for each share.
    pub value: f64,
    /// Cost of capital of year 1 at the means (`model_rate_of`).
    pub rate: f64,
    /// Compound annual growth of the revenue of years 1 to N (`NaN` when the revenue of year N is not positive).
    pub cagr: f64,
    /// Free cash flow (FCFF, after SBC and the charge) over revenue of year N.
    pub margin: f64,
}

/// Why a point has no value.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    Refusal(Refusal),
    /// The engine gave no value (for example the cost of capital is not positive).
    Engine(String),
}

/// The value of one path at the means of a model file.
pub fn deterministic_value(spec: &PathsSpec) -> R<(f64, f64, f64)> {
    let eng = PathEngine::new(spec)?;
    let sim = eng.deterministic()?;
    let n = sim.years;
    let rev = sim.at(0, n - 1, F_REV);
    let fcff = sim.at(0, n - 1, F_FCFF);
    Ok((sim.per_share[0], rev, fcff))
}

pub(super) fn point(inp: &AutoInputs, table: &ProcessTable, fade: &[FadeCell], market: &Market, rules: &Rules) -> Result<(Point, Summary), Failure> {
    let m = build(inp, table, fade, market, rules, BuildOptions::default()).map_err(Failure::Refusal)?;
    let spec = PathsSpec::from_toml(&m.toml).map_err(|x| Failure::Refusal(Refusal::NoPriors(format!("the model file does not parse: {x}"))))?;
    let (value, rev, fcff) = deterministic_value(&spec).map_err(|x| Failure::Engine(x.to_string()))?;
    if !value.is_finite() {
        return Err(Failure::Engine("the value is not a finite number".into()));
    }
    let years = spec.projection_years as f64;
    let sm = m.summary;
    let cagr = if rev > 0.0 && sm.base_revenue > 0.0 { (rev / sm.base_revenue).powf(1.0 / years) - 1.0 } else { f64::NAN };
    let margin = if rev > 0.0 { fcff / rev } else { f64::NAN };
    Ok((Point { value, rate: model_rate_of(&sm), cagr, margin }, sm))
}

/// The point for a premium `x` (a failure of the engine is a value above any price, as in `implied_discount_rate`).
pub(super) fn point_at_premium(inp: &AutoInputs, cfg: &ImpliedConfig, market: &Market, x: f64) -> R<Option<Point>> {
    let rules = Rules { erp: (x, cfg.rules.erp.1), ..cfg.rules.clone() };
    match point(inp, cfg.table, cfg.fade, market, &rules) {
        Ok((p, _)) => Ok(Some(p)),
        Err(Failure::Engine(_)) => Ok(None),
        Err(Failure::Refusal(r)) => e(r.to_string()),
    }
}

pub(super) fn point_with(inp: &AutoInputs, cfg: &ImpliedConfig, market: &Market, growth_shift: f64, margin_shift: f64) -> R<Point> {
    let rules = Rules { growth_shift, margin_shift, ..cfg.rules.clone() };
    match point(inp, cfg.table, cfg.fade, market, &rules) {
        Ok((p, _)) => Ok(p),
        Err(Failure::Engine(m)) => e(m),
        Err(Failure::Refusal(r)) => e(r.to_string()),
    }
}

// ---------------------------------------------------------------------------------------------
// The rows
// ---------------------------------------------------------------------------------------------

pub struct ImpliedConfig<'a> {
    pub table: &'a ProcessTable,
    pub fade: &'a [FadeCell],
    pub rules: Rules,
    /// The cache for the SIC code (`groups_of`); `None`: only the groups of the file.
    pub cache: Option<&'a Path>,
}

/// The result for one ticker with a value.
#[derive(Debug, Clone, PartialEq)]
pub struct ImpliedValue {
    pub size: String,
    pub price: Option<f64>,
    /// The value of the path at the means (a share).
    pub value: f64,
    pub model_premium: f64,
    pub model_rate: f64,
    pub model_cagr: f64,
    pub model_margin: f64,
    /// The searches (`None`: no price, or the search gave an error).
    pub premium: Option<Found>,
    pub implied_rate: f64,
    pub growth: Option<Found>,
    pub implied_cagr: f64,
    pub margin: Option<Found>,
    pub implied_margin: f64,
    /// Value at the premium of the whole list, over the price.
    pub at_market: Option<f64>,
    /// A search stopped with an error of the engine (the text).
    pub notes: Vec<String>,
}

impl ImpliedValue {
    pub fn value_over_price(&self) -> Option<f64> {
        self.price.map(|p| self.value / p)
    }
    /// Implied less model cost of capital.
    pub fn rate_diff(&self) -> Option<f64> {
        self.premium.filter(|f| f.x.is_finite()).map(|_| self.implied_rate - self.model_rate)
    }
    pub fn growth_diff(&self) -> Option<f64> {
        self.growth.filter(|f| f.x.is_finite()).map(|_| self.implied_cagr - self.model_cagr).filter(|x| x.is_finite())
    }
    pub fn margin_diff(&self) -> Option<f64> {
        self.margin.filter(|f| f.x.is_finite()).map(|_| self.implied_margin - self.model_margin).filter(|x| x.is_finite())
    }
    /// The flags of the row: the bound of each search and the check of the direction.
    pub fn flags(&self) -> String {
        let mut v: Vec<String> = Vec::new();
        for (name, f) in [("premium", &self.premium), ("growth", &self.growth), ("margin", &self.margin)] {
            if let Some(f) = f {
                match f.bound {
                    Bound::Exact => {}
                    Bound::Below => v.push(format!("{name}<")),
                    Bound::Above => v.push(format!("{name}>")),
                }
                if !f.monotone {
                    v.push(format!("{name}:nm"));
                }
            }
        }
        v.extend(self.notes.iter().cloned());
        v.join(" ")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImpliedRow {
    pub ticker: String,
    pub sector: String,
    pub industry: String,
    /// The value, or the key and the text of the refusal.
    pub outcome: Result<ImpliedValue, (String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImpliedTiming {
    pub model_s: f64,
    pub search_s: f64,
    pub market_s: f64,
}

/// The premium that the whole list implies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarketPremium {
    pub found: Found,
    /// Tickers in the median.
    pub n: usize,
    /// Median of value / price at the model premium.
    pub median_at_model: f64,
}

/// The sector and the industry of a ticker. One function. The file of `--groups` wins; else the SIC code of the cache
/// (`mapcheck::sec::company_sic`, from `<T>/sec_8k_earnings.json`, no network): the sector is the Fama-French 12 name and
/// the industry is the code with its description (for example "3570 Computer & office equipment"). A ticker with no code
/// has no group. The code is the code of the filer TODAY (`docs/HIDDEN-ASSUMPTIONS.md` #170); this command values today
/// only, so that limit does not apply to it.
pub fn groups_of(ticker: &str, groups: &HashMap<String, (String, String)>, cache: Option<&Path>) -> (String, String) {
    if let Some(g) = groups.get(ticker) {
        return g.clone();
    }
    match cache.and_then(|c| mapcheck::sec::company_sic(c, ticker)) {
        Some((sic, desc)) => (mapcheck::sec::ff12_sector(sic).to_string(), if desc.is_empty() { sic.to_string() } else { format!("{sic} {desc}") }),
        None => (String::new(), String::new()),
    }
}

/// CSV lines `ticker,sector,industry` (a header line, comments after `#` and empty lines are allowed). The industry is the
/// rest of the line, so it can have a comma.
pub fn read_groups(path: &str) -> R<HashMap<String, (String, String)>> {
    let text = std::fs::read_to_string(path).map_err(|x| DcfError(format!("{path}: {x}")))?;
    parse_groups(&text, path)
}

pub fn parse_groups(text: &str, name: &str) -> R<HashMap<String, (String, String)>> {
    let mut out = HashMap::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, ',').map(str::trim).collect();
        if parts.len() < 3 {
            return e(format!("{name} line {}: expected ticker,sector,industry", i + 1));
        }
        if i == 0 && parts[0].eq_ignore_ascii_case("ticker") {
            continue;
        }
        out.insert(parts[0].to_uppercase(), (parts[1].to_string(), parts[2].to_string()));
    }
    Ok(out)
}

struct Base<'a> {
    inp: &'a AutoInputs,
    market: Market,
    base: Result<(Point, Summary), Failure>,
}

/// The value at the means of each ticker (stage 1 of `implied_models`).
fn bases_of<'a>(inputs: &'a [AutoInputs], market_of: &(dyn Fn(&str) -> Market + Sync), cfg: &ImpliedConfig) -> Vec<Base<'a>> {
    inputs
        .par_iter()
        .map(|inp| {
            let market = market_of(&inp.ticker);
            let base = point(inp, cfg.table, cfg.fade, &market, &cfg.rules);
            Base { inp, market, base }
        })
        .collect()
}

/// The premium that the whole list implies, with no search for each ticker (stages 1 and 3 of `implied_models`: the same
/// functions, so the same number). The rayon pool of the caller runs it. A market-implied value is a measurement.
pub fn list_premium(inputs: &[AutoInputs], market_of: &(dyn Fn(&str) -> Market + Sync), cfg: &ImpliedConfig) -> R<Option<MarketPremium>> {
    market_premium(&bases_of(inputs, market_of, cfg), cfg)
}

/// The rows for a list of company inputs. `market_of` gives the market inputs of a ticker. Each ticker runs on one thread
/// (the median over the list is a sorted list), so the result does not depend on the number of threads.
pub fn implied_models(
    inputs: &[AutoInputs],
    market_of: &(dyn Fn(&str) -> Market + Sync),
    groups: &HashMap<String, (String, String)>,
    cfg: &ImpliedConfig,
    jobs: Option<usize>,
) -> R<(Vec<ImpliedRow>, Option<MarketPremium>, ImpliedTiming)> {
    let run = || -> R<(Vec<ImpliedRow>, Option<MarketPremium>, ImpliedTiming)> {
        // stage 1: the value at the means
        let t0 = Instant::now();
        let bases = bases_of(inputs, market_of, cfg);
        let model_s = t0.elapsed().as_secs_f64();
        // stage 2: the three searches of each ticker with a price
        let t1 = Instant::now();
        let mut rows: Vec<ImpliedRow> = bases
            .par_iter()
            .map(|b| {
                let (sector, industry) = groups_of(&b.inp.ticker, groups, cfg.cache);
                let outcome = match &b.base {
                    Err(Failure::Refusal(r)) => Err((r.key(), r.to_string())),
                    Err(Failure::Engine(m)) => Err(("engine error".to_string(), m.clone())),
                    Ok((p, sm)) => Ok(searches(b, p, sm, cfg)),
                };
                ImpliedRow { ticker: b.inp.ticker.clone(), sector, industry, outcome }
            })
            .collect();
        let search_s = t1.elapsed().as_secs_f64();
        // stage 3: the premium of the whole list
        let t2 = Instant::now();
        let market = market_premium(&bases, cfg)?;
        if let Some(mp) = &market {
            let x = mp.found.x;
            let values: Vec<Option<f64>> = bases
                .par_iter()
                .map(|b| match (&b.base, b.market.price) {
                    (Ok(_), Some(price)) if x.is_finite() => point_at_premium(b.inp, cfg, &b.market, x).ok().and_then(|p| value_over_price_at(p, price)),
                    _ => None,
                })
                .collect();
            for (r, v) in rows.iter_mut().zip(values) {
                if let Ok(val) = &mut r.outcome {
                    val.at_market = v;
                }
            }
        }
        Ok((rows, market, ImpliedTiming { model_s, search_s, market_s: t2.elapsed().as_secs_f64() }))
    };
    match jobs {
        Some(n) => {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(n.max(1)).build().map_err(|x| DcfError(x.to_string()))?;
            pool.install(run)
        }
        None => run(),
    }
}

fn searches(b: &Base, p: &Point, sm: &Summary, cfg: &ImpliedConfig) -> ImpliedValue {
    let mut v = ImpliedValue {
        size: sm.size.clone(),
        price: b.market.price,
        value: p.value,
        model_premium: sm.erp,
        model_rate: p.rate,
        model_cagr: p.cagr,
        model_margin: p.margin,
        premium: None,
        implied_rate: f64::NAN,
        growth: None,
        implied_cagr: f64::NAN,
        margin: None,
        implied_margin: f64::NAN,
        at_market: None,
        notes: Vec::new(),
    };
    let Some(price) = b.market.price else { return v };
    // 1. the premium (a higher premium gives a lower value)
    let erp_search = solve(|x| Ok(point_at_premium(b.inp, cfg, &b.market, x)?.map_or(NO_VALUE, |p| p.value)), ERP_RANGE.0, ERP_RANGE.1, price, false, Some((sm.erp, p.value)));
    match erp_search {
        Ok(f) => {
            if f.x.is_finite() {
                // the implied cost of capital of year 1: `model_rate_of` at the premium
                match point_at_premium(b.inp, cfg, &b.market, f.x) {
                    Ok(Some(q)) => v.implied_rate = q.rate,
                    _ => v.implied_rate = f64::NAN,
                }
            }
            v.premium = Some(f);
        }
        Err(x) => v.notes.push(format!("premium:{x}")),
    }
    // 2. the growth (a higher growth gives a higher value)
    match solve(|d| Ok(point_with(b.inp, cfg, &b.market, d, cfg.rules.margin_shift)?.value), GROWTH_RANGE.0, GROWTH_RANGE.1, price, true, Some((0.0, p.value))) {
        Ok(f) => {
            if f.x.is_finite() {
                if let Ok(q) = point_with(b.inp, cfg, &b.market, f.x, cfg.rules.margin_shift) {
                    v.implied_cagr = q.cagr;
                }
            }
            v.growth = Some(f);
        }
        Err(x) => v.notes.push(format!("growth:{x}")),
    }
    // 3. the margin
    match solve(|d| Ok(point_with(b.inp, cfg, &b.market, cfg.rules.growth_shift, d)?.value), MARGIN_RANGE.0, MARGIN_RANGE.1, price, true, Some((0.0, p.value))) {
        Ok(f) => {
            if f.x.is_finite() {
                if let Ok(q) = point_with(b.inp, cfg, &b.market, cfg.rules.growth_shift, f.x) {
                    v.implied_margin = q.margin;
                }
            }
            v.margin = Some(f);
        }
        Err(x) => v.notes.push(format!("margin:{x}")),
    }
    v
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    percentile(&v, 50.0)
}

/// The premium for which the median of value / price over the tickers with a value and a price is 1.
fn market_premium(bases: &[Base], cfg: &ImpliedConfig) -> R<Option<MarketPremium>> {
    let list: Vec<(&Base, f64)> = bases.iter().filter_map(|b| match (&b.base, b.market.price) {
        (Ok(_), Some(p)) => Some((b, p)),
        _ => None,
    }).collect();
    if list.is_empty() {
        return Ok(None);
    }
    let median_at = |x: f64| -> R<f64> {
        let ratios: Vec<f64> = list
            .par_iter()
            .map(|(b, price)| point_at_premium(b.inp, cfg, &b.market, x).map(|p| p.map_or(NO_VALUE, |p| p.value) / price))
            .collect::<R<Vec<f64>>>()?;
        Ok(median(ratios))
    };
    let model_erp = cfg.rules.erp.0;
    let at_model = median_at(model_erp)?;
    let found = solve(median_at, ERP_RANGE.0, ERP_RANGE.1, 1.0, false, Some((model_erp, at_model)))?;
    Ok(Some(MarketPremium { found, n: list.len(), median_at_model: at_model }))
}

// ---------------------------------------------------------------------------------------------
// The groups
// ---------------------------------------------------------------------------------------------

/// The quartiles of a measure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quartiles {
    pub n: usize,
    /// Values that are bounds of a search (`<` or `>`), not solutions.
    pub bounded: usize,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
}

impl Quartiles {
    /// More than 20% of the values are bounds: a quartile of censored values is not a measured number.
    pub fn censored(&self) -> bool {
        self.bounded * 5 > self.n
    }
}

pub const MEASURES: [&str; 5] = ["value / price", "implied - model cost of capital", "implied - model 10-year growth", "implied - model margin", "value at the list premium / price"];

#[derive(Debug, Clone, PartialEq)]
pub struct GroupLine {
    pub name: String,
    /// Tickers with a value.
    pub n: usize,
    /// One for each of `MEASURES` (`None`: no value).
    pub measures: [Option<Quartiles>; 5],
}

fn quartiles(v: Vec<(f64, bool)>) -> Option<Quartiles> {
    let bounded = v.iter().filter(|x| x.0.is_finite() && x.1).count();
    let vals: Vec<f64> = v.iter().map(|x| x.0).filter(|x| x.is_finite()).collect();
    if vals.is_empty() {
        return None;
    }
    let s = sorted_copy(&vals);
    Some(Quartiles { n: s.len(), bounded, p25: percentile(&s, 25.0), p50: percentile(&s, 50.0), p75: percentile(&s, 75.0) })
}

fn group_line(name: String, rows: &[&ImpliedValue]) -> GroupLine {
    let col = |f: &dyn Fn(&ImpliedValue) -> Option<(f64, bool)>| quartiles(rows.iter().filter_map(|r| f(r)).collect());
    let bound = |f: &Option<Found>| f.is_some_and(|f| f.bound != Bound::Exact);
    GroupLine {
        name,
        n: rows.len(),
        measures: [
            col(&|r| r.value_over_price().map(|x| (x, false))),
            col(&|r| r.rate_diff().map(|x| (x, bound(&r.premium)))),
            col(&|r| r.growth_diff().map(|x| (x, bound(&r.growth)))),
            col(&|r| r.margin_diff().map(|x| (x, bound(&r.margin)))),
            col(&|r| r.at_market.map(|x| (x, false))),
        ],
    }
}

/// The medians and the quartiles for all tickers, each size bucket, each sector and each industry with `INDUSTRY_MIN`
/// tickers or more (a ticker with no sector is in no sector group). With no group data: all and size.
pub fn group_table(rows: &[ImpliedRow]) -> Vec<GroupLine> {
    let with: Vec<(&ImpliedRow, &ImpliedValue)> = rows.iter().filter_map(|r| r.outcome.as_ref().ok().map(|v| (r, v))).collect();
    let mut out = vec![group_line("all".into(), &with.iter().map(|x| x.1).collect::<Vec<_>>())];
    for (i, label) in BUCKET_LABELS.iter().enumerate() {
        let _ = i;
        let sel: Vec<&ImpliedValue> = with.iter().filter(|x| x.1.size == *label).map(|x| x.1).collect();
        if !sel.is_empty() {
            out.push(group_line(format!("size {label}"), &sel));
        }
    }
    let mut sectors: BTreeMap<&str, Vec<&ImpliedValue>> = BTreeMap::new();
    let mut industries: BTreeMap<&str, Vec<&ImpliedValue>> = BTreeMap::new();
    for (r, v) in &with {
        if !r.sector.is_empty() {
            sectors.entry(r.sector.as_str()).or_default().push(v);
        }
        if !r.industry.is_empty() {
            industries.entry(r.industry.as_str()).or_default().push(v);
        }
    }
    for (k, sel) in &sectors {
        out.push(group_line(format!("sector {k}"), sel));
    }
    // the largest industries first (the same count: by name)
    let mut big: Vec<(&&str, &Vec<&ImpliedValue>)> = industries.iter().filter(|(_, sel)| sel.len() >= INDUSTRY_MIN).collect();
    big.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    for (k, sel) in big {
        out.push(group_line(format!("industry {k}"), sel));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The output
// ---------------------------------------------------------------------------------------------

pub const CSV_HEADER: &str = "ticker,size,sector,industry,price,value_at_means,value_over_price,model_rate,implied_rate,model_premium,implied_premium,model_growth10,implied_growth10,growth_shift,model_margin10,implied_margin10,margin_shift,value_over_price_at_market_premium,flags,refusal";

fn cell(s: &str) -> String {
    s.replace(',', ";")
}

fn opt(x: Option<f64>, d: usize) -> String {
    x.filter(|x| x.is_finite()).map_or(String::new(), |x| num(x, d))
}

fn found_x(f: &Option<Found>, d: usize) -> String {
    f.as_ref().filter(|f| f.x.is_finite()).map_or(String::new(), |f| num(f.x, d))
}

/// One CSV line of a row.
pub fn csv_line(r: &ImpliedRow) -> String {
    match &r.outcome {
        Ok(v) => [
            r.ticker.clone(),
            v.size.clone(),
            cell(&r.sector),
            cell(&r.industry),
            opt(v.price, 2),
            num(v.value, 2),
            opt(v.value_over_price(), 3),
            num(v.model_rate, 4),
            opt(v.premium.filter(|f| f.x.is_finite()).map(|_| v.implied_rate), 4),
            num(v.model_premium, 4),
            found_x(&v.premium, 4),
            opt(Some(v.model_cagr), 4),
            opt(v.growth.filter(|f| f.x.is_finite()).map(|_| v.implied_cagr), 4),
            found_x(&v.growth, 4),
            opt(Some(v.model_margin), 4),
            opt(v.margin.filter(|f| f.x.is_finite()).map(|_| v.implied_margin), 4),
            found_x(&v.margin, 4),
            opt(v.at_market, 3),
            cell(&v.flags()),
            String::new(),
        ]
        .join(","),
        Err((key, _)) => {
            let mut f = vec![String::new(); 20];
            f[0] = r.ticker.clone();
            f[2] = cell(&r.sector);
            f[3] = cell(&r.industry);
            f[19] = cell(key);
            f.join(",")
        }
    }
}

pub fn csv_text(rows: &[ImpliedRow]) -> String {
    let mut text = String::from(CSV_HEADER);
    text.push('\n');
    for r in rows {
        text.push_str(&csv_line(r));
        text.push('\n');
    }
    text
}

fn q_text(q: &Option<Quartiles>, scale: f64, d: usize, n: usize) -> String {
    match q {
        None => "-".into(),
        Some(q) => format!("{} {} {}{}{}", num(q.p25 * scale, d), num(q.p50 * scale, d), num(q.p75 * scale, d), if q.censored() { "*" } else { "" }, if q.n != n { format!(" ({})", q.n) } else { String::new() }),
    }
}

/// The group table as text: for each group the count and, for each measure, P25, P50 and P75 (the count of a measure in
/// brackets when it is not the count of the group). The ratios are plain numbers; the differences are in points of percent.
pub fn group_text(lines: &[GroupLine]) -> String {
    let mut s = String::new();
    s.push_str(&format!("{:<34} {:>5}", "group", "n"));
    for (i, m) in MEASURES.iter().enumerate() {
        s.push_str(&format!(" | {m}{}", if i == 0 || i == 4 { "" } else { " (points)" }));
    }
    s.push_str("\n  each measure: P25 P50 P75; * more than 20% of the values are bounds of a search (< or >): a quartile of censored values is not a measured number\n");
    for l in lines {
        s.push_str(&format!("{:<34} {:>5}", l.name, l.n));
        for (i, q) in l.measures.iter().enumerate() {
            let (scale, d) = if i == 0 || i == 4 { (1.0, 2) } else { (100.0, 2) };
            s.push_str(&format!(" | {}", q_text(q, scale, d, l.n)));
        }
        s.push('\n');
    }
    s
}

/// `fa dcf implied`. Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> u8 {
    match command_inner(repo, args) {
        Ok(()) => 0,
        Err(x) => {
            eprintln!("fa: {x}");
            2
        }
    }
}

fn command_inner(repo: &Path, args: &[String]) -> R<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{IMPLIED_USAGE}");
        return Ok(());
    }
    let o = Opts::parse(args, &["--cache", "--out", "--jobs", "--prices", "--table", "--as-of", "--price-date", "--tickers-file", "--rules", "--groups"], &[])?;
    let cache = cache_dir(repo, &o);
    let (table, fade) = load_table(&table_path(repo, &o))?;
    let (as_of, as_of_day) = as_of_of(&o)?;
    // the market date: the price date if given, else the as-of date. The facts date: --as-of, else the price date.
    let price_date: Option<String> = match o.get("--price-date") {
        Some(d) if mapcheck::dates::parse(d).is_some() => Some(d.to_string()),
        Some(d) => return e(format!("--price-date '{d}' is not YYYY-MM-DD")),
        None => None,
    };
    let market_date = price_date.clone().unwrap_or_else(|| as_of.clone());
    let (facts_date, facts_day, filter_facts) = match (o.get("--as-of"), &price_date) {
        (Some(_), _) => (as_of.clone(), as_of_day, true),
        (None, Some(d)) => (d.clone(), mapcheck::dates::parse(d).unwrap_or(as_of_day), true),
        (None, None) => (as_of.clone(), as_of_day, false),
    };
    let jobs = jobs_of(&o)?;
    let tickers = tickers_of(&o, &cache)?;
    let prices = match o.get("--prices") {
        Some(p) => read_prices(p)?,
        None => HashMap::new(),
    };
    let groups = match o.get("--groups") {
        Some(g) => read_groups(g)?,
        None => HashMap::new(),
    };
    let rules = super::auto_pit::parse_rules(o.get("--rules").unwrap_or(""))?;
    let (inputs, map_s) = if filter_facts { map_inputs_pit(&cache, &tickers, &facts_date, facts_day, jobs)? } else { map_inputs(&cache, &tickers, as_of_day, jobs)? };
    let (curve, index) = curve_and_index(&cache, &market_date);
    let market_of = |t: &str| -> Market { market_at(&cache, t, &market_date, prices.get(t).copied(), price_date.as_deref(), &rules, curve.as_ref(), index.as_deref()) };
    let cfg = ImpliedConfig { table: &table, fade: &fade, rules: rules.clone(), cache: Some(&cache) };
    let (rows, market, timing) = implied_models(&inputs, &market_of, &groups, &cfg, jobs)?;
    if let Some(path) = o.get("--out") {
        std::fs::write(path, csv_text(&rows)).map_err(|x| DcfError(format!("{path}: {x}")))?;
    }
    println!("IMPLIED by the price of {} tickers, market date {market_date}, facts date {}, no random part. {BANNER} A market-implied value is a measurement, not an input.", rows.len(), if filter_facts { facts_date.clone() } else { "all filings of the cache".to_string() });
    if rows.len() <= 40 {
        println!("{CSV_HEADER}");
        for r in &rows {
            println!("{}", csv_line(r));
        }
    }
    let values: Vec<&ImpliedValue> = rows.iter().filter_map(|r| r.outcome.as_ref().ok()).collect();
    println!("\nRESULT: {} tickers with a value ({} with a price), {} refused", values.len(), values.iter().filter(|v| v.price.is_some()).count(), rows.len() - values.len());
    let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();
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
    let count = |f: &dyn Fn(&ImpliedValue) -> Option<Found>, b: Bound| values.iter().filter(|v| f(v).is_some_and(|f| f.bound == b)).count();
    for (name, f) in [("premium", (&|v: &ImpliedValue| v.premium) as &dyn Fn(&ImpliedValue) -> Option<Found>), ("growth", &|v: &ImpliedValue| v.growth), ("margin", &|v: &ImpliedValue| v.margin)] {
        println!("  search {name}: {} at a bound below the range (<), {} above (>), {} not monotone (nm)", count(f, Bound::Below), count(f, Bound::Above), values.iter().filter(|v| f(v).is_some_and(|f| !f.monotone)).count());
    }
    match &market {
        Some(mp) => println!(
            "\nPREMIUM OF THE WHOLE LIST ({} tickers): the equity risk premium for which the median value / price is 1.0 is {}{:.2}% (the model: {:.2}%, median value / price {:.3})",
            mp.n,
            mp.found.sign(),
            mp.found.x * 100.0,
            cfg.rules.erp.0 * 100.0,
            mp.median_at_model
        ),
        None => println!("\nPREMIUM OF THE WHOLE LIST: no ticker has a value and a price"),
    }
    if groups.is_empty() {
        println!("  Groups: the SIC code of the cache (`<T>/sec_8k_earnings.json`), sector = Fama-French 12, industry = the code with its description. The code is the code of the filer today (trap #170); this command values today only, so that limit does not apply. A ticker with no code has no group.");
    }
    println!("\nGROUPS (tickers with a value; the numbers are the quartiles of each measure)");
    print!("{}", group_text(&group_table(&rows)));
    println!(
        "\nTIME by stage ({} threads): mapping {map_s:.2} s | models {:.3} s | searches {:.2} s | list premium {:.2} s | for each ticker with a value {:.3} s",
        jobs.unwrap_or_else(rayon::current_num_threads),
        timing.model_s,
        timing.search_s,
        timing.market_s,
        (timing.model_s + timing.search_s + timing.market_s) / values.len().max(1) as f64
    );
    Ok(())
}

#[cfg(test)]
#[path = "auto_implied_tests.rs"]
mod tests;
