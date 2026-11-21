//! `fa dcf attribute`: where the market value of a stock went between two dates (ticket
//! `T-20261120-valuation-move-attribution`). A fixed procedure with no random part; the value is the value of the one
//! path at the means (`fa dcf implied`), which is NOT the expected value.
//!
//! A market-implied value is a measurement. It is never an input of the model (`docs/QUALITY-STANDARDS.md`, section 6).
//!
//! The price of each date is the value of the model of that date at the premium that the price implies (`x1` at D1, `x2` at
//! D2; the search of `fa dcf implied`). So `ln(P2 / P1) = ln V(model 2, x2) - ln V(model 1, x1)`, and the change is moved in
//! steps, one input group at a time, in a fixed order. Each step is the change of the value at the means:
//!
//! | node | facts | valuation date | curve | beta | price (equity weight) | premium |
//! |------|-------|----------------|-------|------|-----------------------|---------|
//! | N0   | D1    | D1             | D1    | D1   | P1                    | x1      |
//! | N1   | D2    | D1             | D1    | D1   | P1                    | x1      |
//! | N2   | D2    | D1             | D2    | D1   | P1                    | x1      |
//! | N3   | D2    | D2             | D2    | D1   | P1                    | x1      |
//! | N4   | D2    | D2             | D2    | D2   | P2                    | x2      |
//!
//! The free variable of the market side is the premium (premium view) or, in a second chain, the shift of the growth at the
//! model premium (growth view). The two chains are separate: a stock whose price needs a cost of capital at or below 0 (the
//! engine gives no value there) has no premium view, only a growth view.
//!
//! * a (new facts) = ln N1/N0. Zero when no filing came between D1 and D2.
//! * b (rates) = ln N2/N1.
//! * c (implied premium) = ln N4/N3. It includes the equity weight (the price enters the weight) and the beta of D2:
//!   these are the market-side inputs that move with the price.
//! * rest = ln(P2/P1) - a - b - c = the roll of the valuation date (N3/N2, the stub of the first year) plus the residual of
//!   the two searches (zero when both searches are exact).
//!
//! The reverse order of b and c: a, then the premium (N1 to the node with the date, beta, price and premium of D2 and the
//! curve of D1), then the rates. Both orders are printed; the difference is the order effect (the parts do not add in an
//! independent way, because the value is not linear in them).
//!
//! Third view: the shift of the growth that gives P2 with the premium x1 of D1 (and the facts and curve of D2).

use super::auto::*;
use super::auto_cli::{cache_dir, curve_and_index, jobs_of, load_table, map_inputs_pit, num, table_path, tickers_of, Opts, BANNER};
use super::auto_implied::{groups_of, point, point_at_premium, point_with, read_groups, solve, Bound, Failure, Found, ImpliedConfig, ERP_RANGE, GROWTH_RANGE};
use super::stats::percentile;
use super::DcfError;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::time::Instant;

type R<T> = Result<T, DcfError>;

fn e<T>(m: impl Into<String>) -> R<T> {
    Err(DcfError(m.into()))
}

pub const ATTRIBUTE_USAGE: &str = "\
fa dcf attribute [--tickers-file FILE | TICKER ...] --from D1 --to D2 [OPTIONS]
  Where the market value of a stock went between two dates. A fixed procedure with no random part (the value of the one
  path at the means, NOT the expected value). The price of each date is the value of the model of that date at the
  equity risk premium that the price implies (the search of `fa dcf implied`). The change ln(P2 / P1) is split, in this
  fixed order, into
    facts    the facts of D2 (filed on or before D2) in place of the facts of D1 (zero with no new filing)
    rates    the yield curve of D2 in place of the curve of D1
    premium  the change of the implied premium (with the equity weight and the beta of D2: the inputs that move with the price)
    rest     the roll of the valuation date and the residual of the searches
  and the same with rates and premium in the reverse order (the order effect). This is the PREMIUM VIEW. The GROWTH VIEW is
  the same chain with the shift of the growth at the model premium as the free variable (it exists when the price needs a
  cost of capital at or below 0, where the premium view has no value). A third line gives the growth shift that P2 needs at
  the premium of D1. The facts of each date are the facts that
  were public on it; the 10-K of the cache does not fill a missing input. The price is the last close on or before the
  date (not more than 10 days older); the curve is the last GSW row on or before the date.
  One ticker: one table. A list: one CSV row for each ticker, the medians by size and sector, and the sum weighted by the
  market value at D1 (price x shares), which is the answer for the whole list.
  --from D1 --to D2  the two dates (YYYY-MM-DD)
  --cache DIR     as for `fa dcf auto`
  --table FILE    parameter table
  --rules K=V,..  change a rule of the builder (the list: fa dcf auto --help)
  --groups FILE   CSV lines `ticker,sector,industry` (wins over the SIC code of the cache)
  --out FILE      write one CSV row for each ticker
  --jobs N        threads (default: all); the result does not depend on N";

// ---------------------------------------------------------------------------------------------
// One ticker
// ---------------------------------------------------------------------------------------------

/// One chain of steps (`Chain::run`). The `*_ln` numbers are changes of `ln` (they add to `ln(P2 / P1)`).
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
    /// The free variable that the price implies at D1 and at D2: the premium, or the growth shift at the model premium (a
    /// bound when the search has no solution in the range).
    pub x1: Found,
    pub x2: Found,
    pub facts_ln: f64,
    pub rates_ln: f64,
    /// The change of the free variable (with the equity weight and the beta of D2).
    pub free_ln: f64,
    /// The roll of the valuation date (a part of the rest).
    pub time_ln: f64,
    /// The rest: `ln(P2 / P1)` less the three parts.
    pub rest_ln: f64,
    /// The order facts, free variable, rates.
    pub rev_free_ln: f64,
    pub rev_rates_ln: f64,
    pub rev_rest_ln: f64,
    total_ln: f64,
}

impl Chain {
    /// The parts as shares of P1 for the order facts, rates, free variable, rest: each part is the change of the level in the
    /// sequence (the level after the part is `P1 exp(sum of the parts so far)`), so the four parts add to `P2 / P1 - 1`.
    pub fn pct(&self) -> [f64; 4] {
        sequential(&[self.facts_ln, self.rates_ln, self.free_ln, self.rest_ln])
    }
    /// The same for the order facts, free variable, rates, rest (the order of the parts is that of `pct`: the free variable
    /// is the third).
    pub fn pct_rev(&self) -> [f64; 4] {
        let q = sequential(&[self.facts_ln, self.rev_free_ln, self.rev_rates_ln, self.rev_rest_ln]);
        [q[0], q[2], q[1], q[3]]
    }
    /// The search bounds and the residual of the searches.
    fn flags(&self, name: &str) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for (n, f) in [("1", &self.x1), ("2", &self.x2)] {
            match f.bound {
                Bound::Exact => {}
                Bound::Below => v.push(format!("{name}{n}<")),
                Bound::Above => v.push(format!("{name}{n}>")),
            }
            if !f.monotone {
                v.push(format!("{name}{n}:nm"));
            }
        }
        if (self.rest_ln - self.time_ln).abs() > 1e-5 {
            v.push(format!("{name}:search-residual"));
        }
        v
    }

    /// The chain for a free variable. `eval(inp, market, x)` is the value at the means; `x1` and `x2` are the values that
    /// the prices imply. The order of the nodes is the table of the module comment.
    fn run(inp1: &AutoInputs, m1: &Market, inp2: &AutoInputs, m2: &Market, x1: Found, x2: Found, eval: Eval) -> Result<Chain, (String, String)> {
        let (p1, p2) = (m1.price.unwrap_or(f64::NAN), m2.price.unwrap_or(f64::NAN));
        // a search at a bound has no solution: the value at the bound is not the price, so the steps would move a value that is
        // not the price of the date
        for (d, f) in [("D1", &x1), ("D2", &x2)] {
            if f.bound != Bound::Exact {
                return fail("search at a bound", format!("the search at {d} has no solution in the range (bound {}{:.4})", if f.bound == Bound::Below { "<" } else { ">" }, f.x));
            }
        }
        let mut m_rates = m1.clone();
        m_rates.curve = m2.curve.clone();
        let mut m_time = m_rates.clone();
        m_time.as_of = m2.as_of.clone();
        let n0 = eval(inp1, m1, x1.x)?;
        let n1 = eval(inp2, m1, x1.x)?;
        let n2 = eval(inp2, &m_rates, x1.x)?;
        let n3 = eval(inp2, &m_time, x1.x)?;
        let n4 = eval(inp2, m2, x2.x)?;
        // the reverse order: the date, the beta, the price and the free variable of D2 with the curve of D1
        let mut m_rev = m2.clone();
        m_rev.curve = m1.curve.clone();
        let mut m_date1 = m1.clone();
        m_date1.as_of = m2.as_of.clone();
        let n1t = eval(inp2, &m_date1, x1.x)?;
        let n2r = eval(inp2, &m_rev, x2.x)?;
        let total_ln = (p2 / p1).ln();
        if ![n0, n1, n2, n3, n4, n1t, n2r].iter().all(|v| v.is_finite() && *v > 0.0) {
            return fail("engine error", "a value of a step is not a positive number");
        }
        let (facts_ln, rates_ln, time_ln, free_ln) = ((n1 / n0).ln(), (n2 / n1).ln(), (n3 / n2).ln(), (n4 / n3).ln());
        let (rev_free_ln, rev_rates_ln) = ((n2r / n1t).ln(), (n4 / n2r).ln());
        Ok(Chain {
            x1,
            x2,
            facts_ln,
            rates_ln,
            free_ln,
            time_ln,
            rest_ln: total_ln - facts_ln - rates_ln - free_ln,
            rev_free_ln,
            rev_rates_ln,
            rev_rest_ln: total_ln - facts_ln - rev_free_ln - rev_rates_ln,
            total_ln,
        })
    }
}

/// The result for one ticker.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribution {
    pub p1: f64,
    pub p2: f64,
    pub model_premium: f64,
    /// The premium view (`None`: the engine gives no value at a premium of the chain, for example when the price needs a cost
    /// of capital that is not positive), with the reason.
    pub prem: Result<Chain, String>,
    /// The growth view: the free variable is the shift of the growth at the model premium.
    pub grow: Result<Chain, String>,
    /// Compound annual growth of the revenue of years 1 to N that the price implies at the model premium, at D1 and at D2
    /// (`NaN`: no solution).
    pub cagr1: f64,
    pub cagr2: f64,
    /// The third view: the growth shift that gives P2 with the premium of D1, and the 10-year growth at that shift.
    pub growth_at_x1: Option<Found>,
    pub cagr_at_x1: f64,
    /// Shares of D1 and the size bucket of D1.
    pub shares1: f64,
    pub size: String,
    /// The facts of D2 differ from the facts of D1 (the latest fiscal year or the balance sheet).
    pub new_facts: bool,
    pub curve1: String,
    pub curve2: String,
}

impl Attribution {
    pub fn total_ln(&self) -> f64 {
        (self.p2 / self.p1).ln()
    }
    /// Market value at D1 (the weight of the sum over a list).
    pub fn weight(&self) -> f64 {
        self.p1 * self.shares1
    }
    pub fn chain(&self, view: View) -> Option<&Chain> {
        match view {
            View::Premium => self.prem.as_ref().ok(),
            View::Growth => self.grow.as_ref().ok(),
        }
    }
}

/// The free variable of a chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Premium,
    Growth,
}

impl View {
    pub fn name(self) -> &'static str {
        match self {
            View::Premium => "premium",
            View::Growth => "growth",
        }
    }
}

/// The change of `exp(cumulative sum)` at each step (a share of the start level).
fn sequential(parts: &[f64; 4]) -> [f64; 4] {
    let mut out = [0.0; 4];
    let mut cum = 0.0f64;
    for (i, p) in parts.iter().enumerate() {
        let before = cum.exp();
        cum += p;
        out[i] = cum.exp() - before;
    }
    out
}

/// The value at the means for a value of the free variable.
type Eval<'a> = &'a dyn Fn(&AutoInputs, &Market, f64) -> Result<f64, (String, String)>;

fn fail<T>(key: &str, text: impl Into<String>) -> Result<T, (String, String)> {
    Err((key.to_string(), text.into()))
}

fn engine_err(x: DcfError) -> (String, String) {
    ("engine error".to_string(), x.to_string())
}

/// The value at the means for a premium. A premium for which the engine gives no value is an error here.
fn value_at(inp: &AutoInputs, cfg: &ImpliedConfig, market: &Market, x: f64) -> Result<f64, (String, String)> {
    match point_at_premium(inp, cfg, market, x).map_err(engine_err)? {
        Some(p) => Ok(p.value),
        None => fail("engine error", format!("the engine gives no value at the premium {x:.4}")),
    }
}

/// The premium that the price implies (the search of `fa dcf implied`).
fn premium_search(inp: &AutoInputs, cfg: &ImpliedConfig, market: &Market, price: f64) -> Result<Found, (String, String)> {
    let model = cfg.rules.erp.0;
    let v0 = value_at(inp, cfg, market, model)?;
    let f = solve(|x| Ok(point_at_premium(inp, cfg, market, x)?.map_or(f64::MAX, |p| p.value)), ERP_RANGE.0, ERP_RANGE.1, price, false, Some((model, v0))).map_err(engine_err)?;
    if !f.x.is_finite() {
        return fail("premium search", "the value does not fall with the premium on the ends of the range");
    }
    Ok(f)
}

/// The growth shift that gives `price` at the premium `x`, and the 10-year growth of the model at that shift.
fn growth_search(inp: &AutoInputs, cfg: &ImpliedConfig, x: f64, market: &Market, price: f64) -> Result<(Found, f64), (String, String)> {
    let c = ImpliedConfig { table: cfg.table, fade: cfg.fade, rules: Rules { erp: (x, cfg.rules.erp.1), ..cfg.rules.clone() }, cache: cfg.cache };
    let ms = c.rules.margin_shift;
    let v0 = point_with(inp, &c, market, 0.0, ms).map_err(engine_err)?.value;
    let f = solve(|d| Ok(point_with(inp, &c, market, d, ms)?.value), GROWTH_RANGE.0, GROWTH_RANGE.1, price, true, Some((0.0, v0))).map_err(engine_err)?;
    let cagr = if f.x.is_finite() { point_with(inp, &c, market, f.x, ms).map_err(engine_err)?.cagr } else { f64::NAN };
    Ok((f, cagr))
}

fn same_facts(a: &AutoInputs, b: &AutoInputs) -> bool {
    a.years == b.years && a.balance == b.balance
}

/// The value at the means for a growth shift at the premium `x` of the rules (a failure of the engine is an error).
fn value_with_growth(inp: &AutoInputs, cfg: &ImpliedConfig, market: &Market, shift: f64) -> Result<f64, (String, String)> {
    Ok(point_with(inp, cfg, market, shift, cfg.rules.margin_shift).map_err(engine_err)?.value)
}

/// The attribution of one ticker from the inputs and the market of the two dates. The error is the key and the text.
pub fn attribute_models(inp1: &AutoInputs, m1: &Market, inp2: &AutoInputs, m2: &Market, cfg: &ImpliedConfig) -> Result<Attribution, (String, String)> {
    let (Some(p1), Some(p2)) = (m1.price, m2.price) else { return fail("no price", "no close on or before one of the dates") };
    let sm1 = match point(inp1, cfg.table, cfg.fade, m1, &cfg.rules) {
        Ok((_, sm)) => sm,
        Err(Failure::Refusal(r)) => return Err((r.key(), format!("D1: {r}"))),
        Err(Failure::Engine(m)) => return fail("engine error", format!("D1: {m}")),
    };
    match point(inp2, cfg.table, cfg.fade, m2, &cfg.rules) {
        Ok(_) => {}
        Err(Failure::Refusal(r)) => return Err((r.key(), format!("D2: {r}"))),
        Err(Failure::Engine(m)) => return fail("engine error", format!("D2: {m}")),
    }
    let model = cfg.rules.erp.0;
    // the premium view
    let prem = (|| {
        let x1 = premium_search(inp1, cfg, m1, p1)?;
        let x2 = premium_search(inp2, cfg, m2, p2)?;
        Chain::run(inp1, m1, inp2, m2, x1, x2, &|inp, m, x| value_at(inp, cfg, m, x))
    })()
    .map_err(|(_, t)| t);
    // the growth view, at the model premium
    let (g1, cagr1) = growth_search(inp1, cfg, model, m1, p1)?;
    let (g2, cagr2) = growth_search(inp2, cfg, model, m2, p2)?;
    let grow = if g1.x.is_finite() && g2.x.is_finite() {
        Chain::run(inp1, m1, inp2, m2, g1, g2, &|inp, m, x| value_with_growth(inp, cfg, m, x)).map_err(|(_, t)| t)
    } else {
        Err("the value does not rise with the growth on the ends of the range".to_string())
    };
    // the third view: the growth that gives P2 at the premium of D1
    let (growth_at_x1, cagr_at_x1) = match &prem {
        Ok(c) => match growth_search(inp2, cfg, c.x1.x, m2, p2) {
            Ok((f, cagr)) => (Some(f), cagr),
            Err(_) => (None, f64::NAN),
        },
        Err(_) => (None, f64::NAN),
    };
    Ok(Attribution {
        p1,
        p2,
        model_premium: model,
        prem,
        grow,
        cagr1,
        cagr2,
        growth_at_x1,
        cagr_at_x1,
        shares1: sm1.shares,
        size: sm1.size.clone(),
        new_facts: !same_facts(inp1, inp2),
        curve1: m1.curve.as_ref().map_or(String::new(), |c| c.date.clone()),
        curve2: m2.curve.as_ref().map_or(String::new(), |c| c.date.clone()),
    })
}

impl Attribution {
    /// Flags of the row: the search bounds, a view that is not available, a rest that the date roll does not explain.
    pub fn flags(&self) -> String {
        let mut v: Vec<String> = Vec::new();
        for (name, c) in [("premium", &self.prem), ("growth", &self.grow)] {
            match c {
                Ok(c) => v.extend(c.flags(name)),
                Err(_) => v.push(format!("{name}-view:none")),
            }
        }
        if let Some(f) = &self.growth_at_x1 {
            if f.bound != Bound::Exact {
                v.push(format!("growth_at_premium1{}", if f.bound == Bound::Below { "<" } else { ">" }));
            }
        }
        v.join(" ")
    }
}

// ---------------------------------------------------------------------------------------------
// A list
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct AttrRow {
    pub ticker: String,
    pub sector: String,
    pub industry: String,
    pub outcome: Result<Attribution, (String, String)>,
}

/// The rows for two lists of inputs of the same tickers in the same sequence. Each ticker runs on one thread, so the result
/// does not depend on the number of threads.
pub fn attribute_list(
    inputs1: &[AutoInputs],
    inputs2: &[AutoInputs],
    market1_of: &(dyn Fn(&str) -> Market + Sync),
    market2_of: &(dyn Fn(&str) -> Market + Sync),
    groups: &HashMap<String, (String, String)>,
    cfg: &ImpliedConfig,
    jobs: Option<usize>,
) -> R<Vec<AttrRow>> {
    let run = || -> Vec<AttrRow> {
        inputs1
            .par_iter()
            .zip(inputs2.par_iter())
            .map(|(a, b)| {
                let (sector, industry) = groups_of(&a.ticker, groups, cfg.cache);
                let outcome = attribute_models(a, &market1_of(&a.ticker), b, &market2_of(&a.ticker), cfg);
                AttrRow { ticker: a.ticker.clone(), sector, industry, outcome }
            })
            .collect()
    };
    match jobs {
        Some(n) => Ok(rayon::ThreadPoolBuilder::new().num_threads(n.max(1)).build().map_err(|x| DcfError(x.to_string()))?.install(run)),
        None => Ok(run()),
    }
}

/// The names of the parts, in the order of `Chain::pct` (the third is the free variable of the view).
pub fn part_names(view: View) -> [&'static str; 4] {
    ["facts", "rates", view.name(), "rest"]
}

/// One line of the group table: the medians of the total and of the four parts (as a share of P1, in the order facts, rates,
/// free variable, rest) over the tickers that have the view, and of the free variable at the two dates.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupMedians {
    pub name: String,
    /// Tickers in the group with a value, and with the view.
    pub n: usize,
    pub n_view: usize,
    pub total: f64,
    pub parts: [f64; 4],
    pub x1: f64,
    pub x2: f64,
}

fn median(mut v: Vec<f64>) -> f64 {
    v.retain(|x| x.is_finite());
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    percentile(&v, 50.0)
}

fn medians(name: String, sel: &[&Attribution], view: View) -> GroupMedians {
    let with: Vec<(&Attribution, &Chain)> = sel.iter().filter_map(|a| a.chain(view).map(|c| (*a, c))).collect();
    let col = |f: &dyn Fn(&Attribution, &Chain) -> f64| median(with.iter().map(|(a, c)| f(a, c)).collect());
    GroupMedians {
        name,
        n: sel.len(),
        n_view: with.len(),
        total: col(&|a, _| a.p2 / a.p1 - 1.0),
        parts: [col(&|_, c| c.pct()[0]), col(&|_, c| c.pct()[1]), col(&|_, c| c.pct()[2]), col(&|_, c| c.pct()[3])],
        x1: col(&|_, c| c.x1.x),
        x2: col(&|_, c| c.x2.x),
    }
}

/// The medians of all tickers, of each size bucket and of each sector (a ticker with no sector is in no sector group).
pub fn group_medians(rows: &[AttrRow], view: View) -> Vec<GroupMedians> {
    let with: Vec<(&AttrRow, &Attribution)> = rows.iter().filter_map(|r| r.outcome.as_ref().ok().map(|a| (r, a))).collect();
    let mut out = vec![medians("all".into(), &with.iter().map(|x| x.1).collect::<Vec<_>>(), view)];
    let mut sizes: BTreeMap<&str, Vec<&Attribution>> = BTreeMap::new();
    let mut sectors: BTreeMap<&str, Vec<&Attribution>> = BTreeMap::new();
    for (r, a) in &with {
        sizes.entry(a.size.as_str()).or_default().push(a);
        if !r.sector.is_empty() {
            sectors.entry(r.sector.as_str()).or_default().push(a);
        }
    }
    for (k, sel) in &sizes {
        out.push(medians(format!("size {k}"), sel, view));
    }
    for (k, sel) in &sectors {
        out.push(medians(format!("sector {k}"), sel, view));
    }
    out
}

/// The sum over the list weighted by the market value at D1 (price x shares): the change of the market value of the tickers
/// that have the view and the four parts, each as a share of the market value at D1. The parts add to the change.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightedSum {
    /// Tickers in the sum, and tickers with a value that are not (no view).
    pub n: usize,
    pub n_without: usize,
    /// Market value at D1 of the tickers in the sum, and of the tickers with a value (in the unit of the shares).
    pub value1: f64,
    pub value1_all: f64,
    pub total: f64,
    pub parts: [f64; 4],
    /// The order facts, free variable, rates (the parts in the order of `parts`).
    pub parts_rev: [f64; 4],
}

pub fn weighted_sum(rows: &[AttrRow], view: View) -> Option<WeightedSum> {
    let all: Vec<&Attribution> = rows.iter().filter_map(|r| r.outcome.as_ref().ok()).filter(|a| a.weight().is_finite() && a.weight() > 0.0).collect();
    let list: Vec<(&Attribution, &Chain)> = all.iter().filter_map(|a| a.chain(view).map(|c| (*a, c))).collect();
    let value1: f64 = list.iter().map(|(a, _)| a.weight()).sum();
    if list.is_empty() || value1 <= 0.0 {
        return None;
    }
    let wsum = |f: &dyn Fn(&Attribution, &Chain) -> f64| list.iter().map(|(a, c)| a.weight() * f(a, c)).sum::<f64>() / value1;
    Some(WeightedSum {
        n: list.len(),
        n_without: all.len() - list.len(),
        value1,
        value1_all: all.iter().map(|a| a.weight()).sum(),
        total: wsum(&|a, _| a.p2 / a.p1 - 1.0),
        parts: [wsum(&|_, c| c.pct()[0]), wsum(&|_, c| c.pct()[1]), wsum(&|_, c| c.pct()[2]), wsum(&|_, c| c.pct()[3])],
        parts_rev: [wsum(&|_, c| c.pct_rev()[0]), wsum(&|_, c| c.pct_rev()[1]), wsum(&|_, c| c.pct_rev()[2]), wsum(&|_, c| c.pct_rev()[3])],
    })
}

// ---------------------------------------------------------------------------------------------
// The output
// ---------------------------------------------------------------------------------------------

pub const CSV_HEADER: &str = "ticker,size,sector,industry,price1,price2,change_pct,p_facts_pct,p_rates_pct,p_premium_pct,p_rest_pct,p_rev_rates_pct,p_rev_premium_pct,premium1,premium2,g_facts_pct,g_rates_pct,g_growth_pct,g_rest_pct,g_rev_rates_pct,g_rev_growth_pct,growth_shift1,growth_shift2,growth10_1,growth10_2,growth_shift_at_premium1,growth10_at_premium1,shares1,value1,curve1,curve2,new_facts,flags,refusal";

fn cell(s: &str) -> String {
    s.replace(',', ";")
}

fn pc(x: f64) -> String {
    if x.is_finite() {
        num(x * 100.0, 3)
    } else {
        String::new()
    }
}

fn nm(x: f64, d: usize) -> String {
    if x.is_finite() {
        num(x, d)
    } else {
        String::new()
    }
}

/// The CSV cells of a view: the four parts, the two parts of the reverse order that differ, the two free values.
fn chain_cells(c: Option<&Chain>, with_values: bool) -> Vec<String> {
    match c {
        Some(c) => {
            let (p, q) = (c.pct(), c.pct_rev());
            let mut v = vec![pc(p[0]), pc(p[1]), pc(p[2]), pc(p[3]), pc(q[1]), pc(q[2])];
            if with_values {
                v.extend([nm(c.x1.x, 4), nm(c.x2.x, 4)]);
            }
            v
        }
        None => vec![String::new(); if with_values { 8 } else { 6 }],
    }
}

pub fn csv_line(r: &AttrRow) -> String {
    match &r.outcome {
        Ok(a) => {
            let mut f = vec![r.ticker.clone(), a.size.clone(), cell(&r.sector), cell(&r.industry), num(a.p1, 2), num(a.p2, 2), pc(a.p2 / a.p1 - 1.0)];
            // premium view: parts, then the premium at the two dates; growth view: parts, then the shifts
            let p = chain_cells(a.chain(View::Premium), true);
            f.extend(p[..6].iter().cloned());
            f.extend(p[6..].iter().cloned());
            f.extend(chain_cells(a.chain(View::Growth), false));
            let g = chain_cells(a.chain(View::Growth), true);
            f.extend(g[6..].iter().cloned());
            f.extend([nm(a.cagr1, 4), nm(a.cagr2, 4), a.growth_at_x1.as_ref().map_or(String::new(), |g| nm(g.x, 4)), nm(a.cagr_at_x1, 4), num(a.shares1, 0), num(a.weight(), 0), a.curve1.clone(), a.curve2.clone(), a.new_facts.to_string(), cell(&a.flags()), String::new()]);
            f.join(",")
        }
        Err((key, _)) => {
            let n = CSV_HEADER.split(',').count();
            let mut f = vec![String::new(); n];
            f[0] = r.ticker.clone();
            f[2] = cell(&r.sector);
            f[3] = cell(&r.industry);
            f[n - 1] = cell(key);
            f.join(",")
        }
    }
}

pub fn csv_text(rows: &[AttrRow]) -> String {
    let mut text = String::from(CSV_HEADER);
    text.push('\n');
    for r in rows {
        text.push_str(&csv_line(r));
        text.push('\n');
    }
    text
}

fn sgn(x: f64, d: usize) -> String {
    format!("{x:+.d$}")
}

fn bounded(x: &Found, scale: f64) -> String {
    format!("{}{}", match x.bound { Bound::Exact => "", Bound::Below => "<", Bound::Above => ">" }, num(x.x * scale, 2))
}

/// The table of one ticker.
pub fn table_text(ticker: &str, d1: &str, d2: &str, a: &Attribution) -> String {
    let mut s = String::new();
    let total = a.p2 / a.p1 - 1.0;
    s.push_str(&format!("{ticker}: price {} ({d1}) to {} ({d2}), change {}% (ln {}).\n", num(a.p1, 2), num(a.p2, 2), sgn(total * 100.0, 2), sgn(a.total_ln(), 4)));
    for view in [View::Premium, View::Growth] {
        let names = part_names(view);
        match a.chain(view) {
            None => {
                let why = match view { View::Premium => a.prem.as_ref().err(), View::Growth => a.grow.as_ref().err() };
                s.push_str(&format!("\n{} VIEW: none ({})\n", view.name().to_uppercase(), why.map_or("", String::as_str)));
            }
            Some(c) => {
                s.push_str(&format!("\n{} VIEW (the free variable is the {}{})\n", view.name().to_uppercase(), match view { View::Premium => "equity risk premium", View::Growth => "growth shift at the model premium" }, ""));
                s.push_str(&format!("{:<10} {:>10} {:>10} | order with the {} before the rates: {:>10} {:>10}\n", "part", "% of P1", "$ a share", view.name(), "% of P1", "$ a share"));
                let (p, q) = (c.pct(), c.pct_rev());
                for (i, name) in names.iter().enumerate() {
                    s.push_str(&format!("{:<10} {:>10} {:>10} | {:>45} {:>10}\n", name, sgn(p[i] * 100.0, 2), sgn(p[i] * a.p1, 2), sgn(q[i] * 100.0, 2), sgn(q[i] * a.p1, 2)));
                }
                s.push_str(&format!("{:<10} {:>10} {:>10} | {:>45} {:>10}\n", "total", sgn(total * 100.0, 2), sgn(a.p2 - a.p1, 2), sgn(total * 100.0, 2), sgn(a.p2 - a.p1, 2)));
                s.push_str(&format!("  rest = the roll of the valuation date ({} of ln, in percent) + the residual of the searches ({})\n", sgn(c.time_ln * 100.0, 3), sgn((c.rest_ln - c.time_ln) * 100.0, 3)));
                match view {
                    View::Premium => s.push_str(&format!("  implied premium (model {}%): {}% at D1, {}% at D2\n", num(a.model_premium * 100.0, 2), bounded(&c.x1, 100.0), bounded(&c.x2, 100.0))),
                    View::Growth => s.push_str(&format!("  implied growth shift (points, at the model premium {}%): {} at D1, {} at D2; implied 10-year growth {}% at D1, {}% at D2\n", num(a.model_premium * 100.0, 2), bounded(&c.x1, 100.0), bounded(&c.x2, 100.0), num(a.cagr1 * 100.0, 2), num(a.cagr2 * 100.0, 2))),
                }
            }
        }
    }
    if let (Some(g), Ok(c)) = (&a.growth_at_x1, &a.prem) {
        s.push_str(&format!("\nTHIRD VIEW: at the premium of D1 ({}%) and the facts and curve of D2, P2 needs a growth shift of {} points ({}% 10-year growth)\n", num(c.x1.x * 100.0, 2), bounded(g, 100.0), num(a.cagr_at_x1 * 100.0, 2)));
    }
    s.push_str(&format!("\n  new facts between the dates: {}\n", if a.new_facts { "yes" } else { "no" }));
    if a.curve1 == a.curve2 {
        s.push_str(&format!("  rates: curve of {} at both ends\n", a.curve1));
    } else {
        s.push_str(&format!("  rates: curve of {} at D1, curve of {} at D2\n", a.curve1, a.curve2));
    }
    let fl = a.flags();
    if !fl.is_empty() {
        s.push_str(&format!("  flags: {fl}\n"));
    }
    s
}

fn group_text(lines: &[GroupMedians], view: View) -> String {
    let n = part_names(view);
    let mut s = format!("{:<34} {:>5} {:>5} {:>9} | {:>8} {:>8} {:>8} {:>8} | {:>9} {:>9}\n", "group", "n", "view", "change", n[0], n[1], n[2], n[3], "x at D1", "x at D2");
    s.push_str("  medians over the tickers with the view; the parts are a share of P1 in percent (the median of a part is not the part of the median); x = the free variable in percent (premium, or growth shift)\n");
    for l in lines {
        s.push_str(&format!("{:<34} {:>5} {:>5} {:>9}", l.name, l.n, l.n_view, sgn(l.total * 100.0, 2)));
        s.push_str(" |");
        for p in l.parts {
            s.push_str(&format!(" {:>8}", sgn(p * 100.0, 2)));
        }
        s.push_str(&format!(" | {:>9} {:>9}\n", num(l.x1 * 100.0, 2), num(l.x2 * 100.0, 2)));
    }
    s
}

fn weighted_text(w: &WeightedSum, view: View) -> String {
    let n = part_names(view);
    let mut s = format!(
        "VALUE-WEIGHTED SUM, {} VIEW: {} tickers ({:.1}% of the market value at D1 of the tickers with a value; {} have no {} view). Weight = price x shares at D1; the parts are a share of the market value at D1, in percent.\n",
        view.name().to_uppercase(),
        w.n,
        w.value1 / w.value1_all * 100.0,
        w.n_without,
        view.name()
    );
    s.push_str(&format!("  change {}%\n", sgn(w.total * 100.0, 2)));
    for (i, name) in n.iter().enumerate() {
        s.push_str(&format!("  {:<8} {:>8}%   (order with the {} before the rates: {}%)\n", name, sgn(w.parts[i] * 100.0, 2), view.name(), sgn(w.parts_rev[i] * 100.0, 2)));
    }
    s
}

/// `fa dcf attribute`. Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> u8 {
    match command_inner(repo, args) {
        Ok(()) => 0,
        Err(x) => {
            eprintln!("fa: {x}");
            2
        }
    }
}

fn date_of(o: &Opts, key: &str) -> R<(String, i64)> {
    match o.get(key) {
        Some(t) => match mapcheck::dates::parse(t) {
            Some(d) => Ok((t.to_string(), d)),
            None => e(format!("{key} '{t}' is not YYYY-MM-DD")),
        },
        None => e(format!("{key} DATE is required")),
    }
}

fn command_inner(repo: &Path, args: &[String]) -> R<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{ATTRIBUTE_USAGE}");
        return Ok(());
    }
    let o = Opts::parse(args, &["--cache", "--out", "--jobs", "--table", "--from", "--to", "--tickers-file", "--rules", "--groups"], &[])?;
    let cache = cache_dir(repo, &o);
    let (table, fade) = load_table(&table_path(repo, &o))?;
    let ((d1, day1), (d2, day2)) = (date_of(&o, "--from")?, date_of(&o, "--to")?);
    if day2 <= day1 {
        return e("--to must be later than --from");
    }
    let jobs = jobs_of(&o)?;
    let tickers = tickers_of(&o, &cache)?;
    let groups = match o.get("--groups") {
        Some(g) => read_groups(g)?,
        None => HashMap::new(),
    };
    let rules = super::auto_pit::parse_rules(o.get("--rules").unwrap_or(""))?;
    let t0 = Instant::now();
    let (inputs1, map1) = map_inputs_pit(&cache, &tickers, &d1, day1, jobs)?;
    let (inputs2, map2) = map_inputs_pit(&cache, &tickers, &d2, day2, jobs)?;
    let (curve1, index) = curve_and_index(&cache, &d1);
    let (curve2, _) = curve_and_index(&cache, &d2);
    let market1_of = |t: &str| -> Market { market_at(&cache, t, &d1, None, Some(&d1), &rules, curve1.as_ref(), index.as_deref()) };
    let market2_of = |t: &str| -> Market { market_at(&cache, t, &d2, None, Some(&d2), &rules, curve2.as_ref(), index.as_deref()) };
    let cfg = ImpliedConfig { table: &table, fade: &fade, rules: rules.clone(), cache: Some(&cache) };
    let t1 = Instant::now();
    let rows = attribute_list(&inputs1, &inputs2, &market1_of, &market2_of, &groups, &cfg, jobs)?;
    let attr_s = t1.elapsed().as_secs_f64();
    if let Some(path) = o.get("--out") {
        std::fs::write(path, csv_text(&rows)).map_err(|x| DcfError(format!("{path}: {x}")))?;
    }
    println!("ATTRIBUTION of the change of the price from {d1} to {d2}, {} tickers, no random part. {BANNER} A market-implied value is a measurement, not an input.", rows.len());
    println!("  The value is the value of the one path at the means (not the expected value). Order of the steps: facts, rates, free variable (premium or growth), rest; the reverse order of rates and free variable is shown. Facts of each date: filed on or before it; the 10-K of the cache does not fill an input. Price: last close on or before the date. Curve: last GSW row on or before the date.");
    if rows.len() == 1 {
        match &rows[0].outcome {
            Ok(a) => print!("{}", table_text(&rows[0].ticker, &d1, &d2, a)),
            Err((k, t)) => println!("{}: refused: {k} ({t})", rows[0].ticker),
        }
    } else {
        let ok: Vec<&Attribution> = rows.iter().filter_map(|r| r.outcome.as_ref().ok()).collect();
        println!("\nRESULT: {} tickers with a value, {} refused", ok.len(), rows.len() - ok.len());
        let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();
        for r in &rows {
            if let Err((k, _)) = &r.outcome {
                *by_reason.entry(k.clone()).or_default() += 1;
            }
        }
        let mut reasons: Vec<(String, usize)> = by_reason.into_iter().collect();
        reasons.sort_by_key(|r| (std::cmp::Reverse(r.1), r.0.clone()));
        for (k, n) in reasons {
            println!("  refused {n:>4}  {k}");
        }
        println!("  new facts between the dates: {} tickers; premium view: {} tickers, growth view: {} tickers", ok.iter().filter(|a| a.new_facts).count(), ok.iter().filter(|a| a.prem.is_ok()).count(), ok.iter().filter(|a| a.grow.is_ok()).count());
        let c1: std::collections::BTreeSet<&str> = ok.iter().map(|a| a.curve1.as_str()).collect();
        let c2: std::collections::BTreeSet<&str> = ok.iter().map(|a| a.curve2.as_str()).collect();
        println!("  curve date at D1: {c1:?}; at D2: {c2:?}{}", if c1 == c2 && c1.len() == 1 { " (rates: the same curve at both ends)" } else { "" });
        for view in [View::Premium, View::Growth] {
            println!("\nGROUPS, {} VIEW", view.name().to_uppercase());
            print!("{}", group_text(&group_medians(&rows, view), view));
        }
        for view in [View::Premium, View::Growth] {
            match weighted_sum(&rows, view) {
                Some(w) => print!("\n{}", weighted_text(&w, view)),
                None => println!("\nVALUE-WEIGHTED SUM, {} VIEW: no ticker has the view", view.name().to_uppercase()),
            }
        }
    }
    println!("\nTIME: mapping {:.2} s | attribution {attr_s:.2} s ({} threads) | total {:.2} s", map1 + map2, jobs.unwrap_or_else(rayon::current_num_threads), t0.elapsed().as_secs_f64());
    Ok(())
}

#[cfg(test)]
#[path = "auto_attribute_tests.rs"]
mod tests;
