//! `fa dcf attribute`: where the market value of a stock went between two dates (ticket
//! `T-20261120-valuation-move-attribution`). A fixed procedure with no random part; the value is the value of the one
//! path at the means (`fa dcf implied`), which is NOT the expected value.
//!
//! A market-implied value is a measurement. It is never an input of the model (`docs/QUALITY-STANDARDS.md`, section 6).
//! The split is an accounting split of the price change with the model. It is not a cause.
//!
//! The price of each date is the value of the model of that date at the premium that the price implies (`x1` at D1, `x2` at
//! D2; the search of `fa dcf implied`). So `ln(P2 / P1) = ln V(model 2, x2) - ln V(model 1, x1)`, and the change is moved in
//! steps, one input group at a time, in a fixed order. Each step is the change of the value at the means:
//!
//! | node | facts | valuation date | curve | beta | price (equity weight) | free variable |
//! |------|-------|----------------|-------|------|-----------------------|---------------|
//! | N0   | D1    | D1             | D1    | D1   | P1                    | x1            |
//! | N1   | D2    | D1             | D1    | D1   | P1                    | x1            |
//! | Nt   | D2    | D2             | D1    | D1   | P1                    | x1            |
//! | N2   | D2    | D2             | D2    | D1   | P1                    | x1            |
//! | N4   | D2    | D2             | D2    | D2   | P2                    | x2            |
//!
//! The free variable is the premium (premium view) or, in a second chain, the shift of the growth at the model premium
//! (growth view). The two chains are separate: a stock whose price needs a cost of capital at or below 0 (the engine gives no
//! value there) has no premium view. A chain needs an exact solution of both searches.
//!
//! * facts = ln N1/N0. Zero when no filing came between D1 and D2. Fixed position (first).
//! * time (roll of the valuation date) = ln Nt/N1 (the stub of the first year). Fixed position (second).
//! * rates = ln N2/Nt.
//! * implied premium (plug) / implied growth (plug) = ln N4/N2. It includes the equity weight (the price enters the weight)
//!   and the beta of D2: the market-side inputs that move with the price.
//! * search residual = ln(P2/P1) less the four parts (zero when both searches are exact).
//!
//! The reverse order of rates and the plug: facts, time, plug (with the curve of D1), rates. Both orders are printed; the
//! range of the two is the order effect (the value is not linear in the parts).
//!
//! Third view: the shift of the growth that gives P2 with the premium x1 of D1 (and the facts and curve of D2).
//!
//! The weight of a list is the market value at D1: price x shares. The share count is guarded (`guarded_shares`): a count
//! that is outside 0.01 to 100 times the reference count (the count at the end of the period, else the count of the year
//! before) is a scale error and is replaced by the reference count; a count outside 0.5 to 2 times stays and has a note.

use super::auto::*;
use super::auto_cli::{cache_dir, curve_and_index, jobs_of, load_table, map_inputs_pit, num, table_path, tickers_of, Opts, BANNER};
use super::auto_implied::{groups_of, NO_VALUE, point, point_at_premium, point_with, read_groups, solve, Bound, Failure, Found, ImpliedConfig, ERP_RANGE, GROWTH_RANGE};
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
  path at the means, NOT the expected value). This is an accounting split of the price change with the model; it is not a
  cause. The price of each date is the value of the model of that date at the equity risk premium that the price implies
  (the search of `fa dcf implied`). The change ln(P2 / P1) is split, in this fixed order, into
    facts     the facts of D2 (filed on or before D2) in place of the facts of D1 (zero with no new filing)
    time      the roll of the valuation date from D1 to D2
    rates     the yield curve of D2 in place of the curve of D1
    plug      the change of the implied premium, with the equity weight and the beta of D2 (PREMIUM VIEW); or the change of
              the implied growth shift at the model premium (GROWTH VIEW, exists where the premium view has no value)
    residual  the part that the exact searches do not leave (zero when both searches are exact)
  and the same with rates and plug in the reverse order (the range of the two orders is the order effect). A third line
  gives the growth shift that P2 needs at the premium of D1. The facts of each date are the facts that were public on it;
  the 10-K of the cache does not fill a missing input. The price is the last close on or before the date (not more than
  10 days older); the curve is the last GSW row on or before the date (a curve more than 3 days older is flagged).
  One ticker: one table. A list: one CSV row for each ticker, the medians by size and sector, then the change of the WHOLE
  list (value-weighted by price x shares at D1, with a guard on the share count), the sum on the set that has both views,
  and each view on its own set (covered part only), with the top contributors and the flagged tickers.
  --from D1 --to D2  the two dates (YYYY-MM-DD)
  --cache DIR     as for `fa dcf auto`
  --table FILE    parameter table
  --rules K=V,..  change a rule of the builder (the list: fa dcf auto --help)
  --groups FILE   CSV lines `ticker,sector,industry` (wins over the SIC code of the cache)
  --curve SRC     gsw (default): the GSW row of the Federal Reserve Board (it lags about one week); fedinvest: the NSS
                  curve that `fa recon rich-cheap` fits to the FedInvest Treasury prices of the last file on or before each
                  date; par: the same NSS fit to the Treasury daily par yield curve (1 to 30 years, each tenor a par
                  bond) of the last row on or before each date. One source is used at both dates.
  --out FILE      write one CSV row for each ticker
  --jobs N        threads (default: all); the result does not depend on N";

// ---------------------------------------------------------------------------------------------
// One ticker
// ---------------------------------------------------------------------------------------------

/// The number of parts of a chain: facts, time, rates, plug, residual.
pub const N_PARTS: usize = 5;

/// One chain of steps (`Chain::run`). The `*_ln` numbers are changes of `ln` (they add to `ln(P2 / P1)`).
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
    /// The free variable that the price implies at D1 and at D2: the premium, or the growth shift at the model premium.
    pub x1: Found,
    pub x2: Found,
    pub facts_ln: f64,
    pub time_ln: f64,
    pub rates_ln: f64,
    /// The change of the free variable (with the equity weight and the beta of D2).
    pub free_ln: f64,
    /// `ln(P2 / P1)` less the four parts (the residual of the searches).
    pub resid_ln: f64,
    /// The order facts, time, free variable, rates.
    pub rev_free_ln: f64,
    pub rev_rates_ln: f64,
}

impl Chain {
    /// The parts as shares of P1 in the order facts, time, rates, plug, residual. Each part is the change of the level in the
    /// sequence (the level after the part is `P1 exp(sum of the parts so far)`), so the parts add to `P2 / P1 - 1`.
    pub fn pct(&self) -> [f64; N_PARTS] {
        sequential(&[self.facts_ln, self.time_ln, self.rates_ln, self.free_ln, self.resid_ln])
    }
    /// The same for the order facts, time, plug, rates, residual (the parts are returned in the order of `pct`).
    pub fn pct_rev(&self) -> [f64; N_PARTS] {
        let q = sequential(&[self.facts_ln, self.time_ln, self.rev_free_ln, self.rev_rates_ln, self.resid_ln]);
        [q[0], q[1], q[3], q[2], q[4]]
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
        if self.resid_ln.abs() > 1e-5 {
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
        let mut m_time = m1.clone();
        m_time.as_of = m2.as_of.clone();
        let mut m_rates = m_time.clone();
        m_rates.curve = m2.curve.clone();
        let n0 = eval(inp1, m1, x1.x)?;
        let n1 = eval(inp2, m1, x1.x)?;
        let nt = eval(inp2, &m_time, x1.x)?;
        let n2 = eval(inp2, &m_rates, x1.x)?;
        let n4 = eval(inp2, m2, x2.x)?;
        // the reverse order: the beta, the price and the free variable of D2 with the curve of D1 (and the date of D2)
        let mut m_rev = m2.clone();
        m_rev.curve = m1.curve.clone();
        let n2r = eval(inp2, &m_rev, x2.x)?;
        let total_ln = (p2 / p1).ln();
        if ![n0, n1, nt, n2, n4, n2r].iter().all(|v| v.is_finite() && *v > 0.0) {
            return fail("engine error", "a value of a step is not a positive number");
        }
        let (facts_ln, time_ln, rates_ln, free_ln) = ((n1 / n0).ln(), (nt / n1).ln(), (n2 / nt).ln(), (n4 / n2).ln());
        Ok(Chain {
            x1,
            x2,
            facts_ln,
            time_ln,
            rates_ln,
            free_ln,
            resid_ln: total_ln - facts_ln - time_ln - rates_ln - free_ln,
            rev_free_ln: (n2r / nt).ln(),
            rev_rates_ln: (n4 / n2r).ln(),
        })
    }
}

/// The result for one ticker.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribution {
    pub p1: f64,
    pub p2: f64,
    pub model_premium: f64,
    /// The premium view (`Err`: no chain, with the reason; for example the engine gives no value at a premium of the chain
    /// when the price needs a cost of capital that is not positive).
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
    /// The share count of the model at D1 (before the guard) and the size bucket of D1.
    pub shares1: f64,
    pub size: String,
    /// The facts of D2 differ from the facts of D1 (the latest fiscal year or the balance sheet).
    pub new_facts: bool,
    pub curve1: String,
    pub curve2: String,
    /// The dates of the two markets (the curve is the last row on or before them).
    pub date1: String,
    pub date2: String,
}

/// Days that a curve may be older than its date before it is flagged.
pub const CURVE_LAG_DAYS: i64 = 3;

fn lag_days(curve: &str, date: &str) -> Option<i64> {
    Some(mapcheck::dates::parse(date.get(..10)?)? - mapcheck::dates::parse(curve.get(..10)?)?)
}

impl Attribution {
    pub fn total_ln(&self) -> f64 {
        (self.p2 / self.p1).ln()
    }
    pub fn chain(&self, view: View) -> Option<&Chain> {
        match view {
            View::Premium => self.prem.as_ref().ok(),
            View::Growth => self.grow.as_ref().ok(),
        }
    }
    /// The curve of a side is more than `CURVE_LAG_DAYS` days older than the date of the side (days).
    pub fn curve_lag(&self) -> (Option<i64>, Option<i64>) {
        let f = |c: &str, d: &str| lag_days(c, d).filter(|l| *l > CURVE_LAG_DAYS);
        (f(&self.curve1, &self.date1), f(&self.curve2, &self.date2))
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
fn sequential(parts: &[f64; N_PARTS]) -> [f64; N_PARTS] {
    let mut out = [0.0; N_PARTS];
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
    let f = solve(|x| Ok(point_at_premium(inp, cfg, market, x)?.map_or(NO_VALUE, |p| p.value)), ERP_RANGE.0, ERP_RANGE.1, price, false, Some((model, v0))).map_err(engine_err)?;
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

/// The value at the means for a growth shift at the premium of the rules (a failure of the engine is an error).
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
        date1: m1.as_of.clone(),
        date2: m2.as_of.clone(),
    })
}

impl Attribution {
    /// Flags of the row: the search bounds, a view that is not available, a curve that is old.
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
        let (l1, l2) = self.curve_lag();
        for (n, curve, lag) in [("1", &self.curve1, l1), ("2", &self.curve2, l2)] {
            if curve.is_empty() {
                v.push(format!("curve{n}-none"));
            } else if lag.is_some() {
                v.push(format!("curve{n}-old"));
            }
        }
        v.join(" ")
    }
}

// ---------------------------------------------------------------------------------------------
// The share count of the weight
// ---------------------------------------------------------------------------------------------

/// Smallest and largest ratio of the share count to its reference that has no note.
pub const SHARES_RATIO: (f64, f64) = (0.5, 2.0);
/// Smallest and largest ratio that keeps the count of the model. Outside it the count is a scale error (a filer that gives
/// the count in the wrong unit: 1000 times) and the reference count replaces it. Between the two bands the count stays and has
/// a note: a split after the reference date (BKNG: 25 for 1) or one class of several gives a ratio of 2 to 100 and is a true
/// count.
pub const SHARES_SCALE: (f64, f64) = (0.01, 100.0);

/// The share count for the market value at D1 and a note when the count differs from the reference. `primary` is the count of the model (or
/// `None`: the count of the balance sheet). The reference is the count at the end of the period (`shares_outstanding`), else
/// the count of the year before. A count outside `SHARES_SCALE` times the reference (a scale error of a filer) is replaced by
/// the reference (the note starts with "replaced"); a count outside `SHARES_RATIO` but inside `SHARES_SCALE` stays and has a
/// note that starts with "differs". The note has both counts.
pub fn guarded_shares(inp: &AutoInputs, primary: Option<f64>) -> (Option<f64>, Option<String>) {
    let primary = primary.or(inp.balance.as_ref().and_then(|b| b.shares)).filter(|s| s.is_finite() && *s > 0.0);
    let ok = |x: &Option<f64>| x.filter(|s| s.is_finite() && *s > 0.0);
    let reference = ok(&inp.balance.as_ref().and_then(|b| b.shares_outstanding)).or_else(|| {
        let n = inp.years.len();
        let balance_end = inp.balance.as_ref().map(|b| b.end.as_str());
        // the year before the balance sheet: the latest fiscal year if the balance sheet is newer, else the one before it
        let k = if balance_end == inp.years.last().map(|y| y.end.as_str()) { n.checked_sub(2) } else { n.checked_sub(1) };
        k.and_then(|k| ok(&inp.years[k].shares))
    });
    match (primary, reference) {
        (Some(p), Some(r)) if p / r < SHARES_SCALE.0 || p / r > SHARES_SCALE.1 => (Some(r), Some(format!("replaced: shares {:.1}M -> {:.1}M (outside {} to {} times the reference count)", p / 1e6, r / 1e6, SHARES_SCALE.0, SHARES_SCALE.1))),
        (Some(p), Some(r)) if p / r < SHARES_RATIO.0 || p / r > SHARES_RATIO.1 => (Some(p), Some(format!("differs: shares {:.1}M, reference {:.1}M (kept)", p / 1e6, r / 1e6))),
        (Some(p), _) => (Some(p), None),
        (None, Some(r)) => (Some(r), None),
        (None, None) => (None, None),
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
    /// The two prices, when both dates have a close.
    pub prices: Option<(f64, f64)>,
    /// The share count of the weight (after the guard) and the note of the guard.
    pub shares: Option<f64>,
    pub shares_note: Option<String>,
    /// The SEC central index key of the company (`<T>/sec_8k_earnings.json`).
    pub cik: Option<String>,
    /// The ticker of the same company that carries the weight (one company, one weight): this ticker has none.
    pub weight_of: Option<String>,
    pub outcome: Result<Attribution, (String, String)>,
    /// A close of this ticker is older than the close of the market index on the same date (`stale_note`): the row has
    /// no weight, so no sum mixes the dates. The text names the dates.
    pub stale: Option<String>,
}

impl AttrRow {
    /// Market value at D1 (the weight): price x shares.
    pub fn weight(&self) -> Option<f64> {
        if self.weight_of.is_some() {
            return None;
        }
        self.weight_raw()
    }
    /// The market value at D1 with no rule of one weight for one company (none for a stale price).
    pub fn weight_raw(&self) -> Option<f64> {
        if self.stale.is_some() {
            return None;
        }
        let (p1, _) = self.prices?;
        self.shares.map(|s| p1 * s).filter(|w| w.is_finite() && *w > 0.0)
    }
    /// The change of the price, as a share.
    pub fn change(&self) -> Option<f64> {
        self.prices.map(|(p1, p2)| p2 / p1 - 1.0)
    }
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
                let (m1, m2) = (market1_of(&a.ticker), market2_of(&a.ticker));
                let outcome = attribute_models(a, &m1, b, &m2, cfg);
                let prices = m1.price.zip(m2.price);
                let primary = outcome.as_ref().ok().map(|x| x.shares1);
                let (shares, shares_note) = guarded_shares(a, primary);
                let cik = cfg.cache.and_then(|c| cik_of(c, &a.ticker));
                AttrRow { ticker: a.ticker.clone(), sector, industry, prices, shares, shares_note, cik, weight_of: None, outcome, stale: None }
            })
            .collect()
    };
    let mut rows = match jobs {
        Some(n) => rayon::ThreadPoolBuilder::new().num_threads(n.max(1)).build().map_err(|x| DcfError(x.to_string()))?.install(run),
        None => run(),
    };
    one_weight_for_one_company(&mut rows);
    Ok(rows)
}

/// The CIK of a ticker from the filing list of the cache (no network).
pub fn cik_of(cache: &Path, ticker: &str) -> Option<String> {
    let text = std::fs::read(mapcheck::sec::earnings_filings_path(cache, ticker)).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&text).ok()?;
    v.get("cik").and_then(|c| c.as_str().map(String::from).or_else(|| c.as_u64().map(|n| n.to_string())))
}

/// The date of the close that `close_on_or_before` takes for `date` (`None`: no close in `PRICE_GAP_DAYS`).
pub fn close_date(series: &[(String, f64)], date: &str) -> Option<String> {
    close_on_or_before(series, date).map(|(d, _)| d[..10].to_string())
}

/// A row is stale when the date of its close at D1 or at D2 is older than the date of the close of the market index (SPY)
/// for the same date: the ticker has a price of an earlier day, so a sum would mix two dates. `closes(ticker)` gives the
/// close series. A ticker with no close at all is not stale (it has no prices and no weight already). Run it before
/// `one_weight_for_one_company`, so that a stale class does not keep the weight of its company. Returns the number of
/// stale rows.
pub fn mark_stale(rows: &mut [AttrRow], closes: &dyn Fn(&str) -> Option<Vec<(String, f64)>>, d1: &str, d2: &str) -> usize {
    let Some(index) = closes("SPY") else { return 0 };
    let (r1, r2) = (close_date(&index, d1), close_date(&index, d2));
    let mut n = 0;
    for r in rows.iter_mut() {
        let Some(s) = closes(&r.ticker) else { continue };
        let notes: Vec<String> = [(d1, &r1), (d2, &r2)].into_iter().filter_map(|(d, want)| stale_note(&s, d, want.as_deref())).collect();
        if !notes.is_empty() {
            r.stale = Some(notes.join("; "));
            n += 1;
        }
    }
    n
}

/// A close series: `(date, close)`.
pub type CloseSeries = Vec<(String, f64)>;

/// The text of the stale close of one series at one date, or `None`: the close that `close_on_or_before` takes is older
/// than the close of SPY (`want`, its date) for the same date. The one rule of `mark_stale` and of `fa dcf premium-series`.
pub fn stale_note(series: &[(String, f64)], date: &str, want: Option<&str>) -> Option<String> {
    let (own, want) = (close_date(series, date)?, want?);
    (own.as_str() < want).then(|| format!("close of {own} for {date} (SPY {want})"))
}

/// After `attribute_list`: a stale close takes the row out of the sums, then the one-weight rule runs again.
pub fn finish_weights(rows: &mut [AttrRow], closes: &dyn Fn(&str) -> Option<CloseSeries>, d1: &str, d2: &str) -> usize {
    for r in rows.iter_mut() {
        r.weight_of = None;
    }
    let n = mark_stale(rows, closes, d1, d2);
    one_weight_for_one_company(rows);
    n
}

/// One company, one weight. Two classes of one company (GOOG and GOOGL) have the same share count of the company, so a sum
/// that weights both would count the company two times. The first ticker of the list with a weight keeps it; each later ticker
/// with the same CIK has none in the sums (`weight_of` names the ticker that carries it). The rows, the CSV and the medians
/// keep every ticker.
pub fn one_weight_for_one_company(rows: &mut [AttrRow]) {
    let mut first: HashMap<String, String> = HashMap::new();
    for r in rows.iter_mut() {
        let (Some(cik), true) = (r.cik.clone(), r.weight().is_some()) else { continue };
        match first.get(&cik) {
            Some(t) => r.weight_of = Some(t.clone()),
            None => {
                first.insert(cik, r.ticker.clone());
            }
        }
    }
}

/// The names of the parts, in the order of `Chain::pct`.
pub fn part_names(view: View) -> [&'static str; N_PARTS] {
    ["facts", "time", "rates", match view { View::Premium => "premium", View::Growth => "growth" }, "residual"]
}

/// The long names of the parts, for the text.
fn long_names(view: View) -> [&'static str; N_PARTS] {
    ["facts", "time (roll of the valuation date)", "rates", match view { View::Premium => "implied premium (plug)", View::Growth => "implied growth (plug)" }, "search residual"]
}

/// One line of the group table: the medians of the total and of the parts (as a share of P1) over the tickers that have the
/// view, and of the free variable at the two dates.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupMedians {
    pub name: String,
    /// Tickers in the group with a value, and with the view.
    pub n: usize,
    pub n_view: usize,
    pub total: f64,
    pub parts: [f64; N_PARTS],
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
        parts: [col(&|_, c| c.pct()[0]), col(&|_, c| c.pct()[1]), col(&|_, c| c.pct()[2]), col(&|_, c| c.pct()[3]), col(&|_, c| c.pct()[4])],
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

// ---------------------------------------------------------------------------------------------
// The sums of a list
// ---------------------------------------------------------------------------------------------

/// The change of the market value of the whole list: every ticker with two prices and a share count, weighted by the market
/// value at D1.
#[derive(Debug, Clone, PartialEq)]
pub struct WholeList {
    pub n: usize,
    /// Market value at D1 and its change in dollars (the unit of price x shares).
    pub value1: f64,
    pub dvalue: f64,
    /// The change as a share of the value at D1.
    pub change: f64,
    /// The five largest weights: ticker, market value at D1, share of the list.
    pub top: Vec<(String, f64, f64)>,
    /// Tickers whose share count the guard replaced, and tickers whose count differs from the reference (kept).
    pub guarded: Vec<String>,
    pub differs: Vec<String>,
    /// Tickers with no weight because another ticker of the same company has it: ticker and the ticker that carries it.
    pub dropped: Vec<(String, String)>,
    /// Tickers with a weight and no CIK in the cache: the one-weight rule could not apply to them.
    pub no_cik: Vec<String>,
    /// Tickers with a weight and no model value (a template with no model, a refusal): ticker and share of the list.
    pub no_model: Vec<(String, f64)>,
}

fn top_weights(rows: &[&AttrRow], total: f64) -> Vec<(String, f64, f64)> {
    let mut v: Vec<(String, f64)> = rows.iter().filter_map(|r| r.weight().map(|w| (r.ticker.clone(), w))).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v.into_iter().take(5).map(|(t, w)| (t, w, w / total)).collect()
}

pub fn whole_list(rows: &[AttrRow]) -> Option<WholeList> {
    whole_where(rows, true, &|_| true)
}

/// The whole-list change over the tickers that `keep` accepts. `one_weight`: one company, one weight (else every ticker keeps
/// its own weight, and a company with two classes is counted two times).
pub fn whole_where(rows: &[AttrRow], one_weight: bool, keep: &dyn Fn(&AttrRow) -> bool) -> Option<WholeList> {
    let weight = |r: &AttrRow| if one_weight { r.weight() } else { r.weight_raw() };
    let list: Vec<&AttrRow> = rows.iter().filter(|r| weight(r).is_some() && r.change().is_some() && keep(r)).collect();
    let value1: f64 = list.iter().filter_map(|r| weight(r)).sum();
    if list.is_empty() || value1 <= 0.0 {
        return None;
    }
    let dvalue: f64 = list.iter().map(|r| weight(r).unwrap_or(0.0) * r.change().unwrap_or(0.0)).sum();
    Some(WholeList {
        n: list.len(),
        value1,
        dvalue,
        change: dvalue / value1,
        top: top_weights(&list, value1),
        guarded: list.iter().filter(|r| r.shares_note.as_ref().is_some_and(|n| n.starts_with("replaced"))).map(|r| r.ticker.clone()).collect(),
        differs: list.iter().filter(|r| r.shares_note.as_ref().is_some_and(|n| n.starts_with("differs"))).map(|r| r.ticker.clone()).collect(),
        no_cik: list.iter().filter(|r| r.cik.is_none()).map(|r| r.ticker.clone()).collect(),
        no_model: {
            let mut v: Vec<(String, f64)> = list.iter().filter(|r| r.outcome.is_err()).map(|r| (r.ticker.clone(), weight(r).unwrap_or(0.0) / value1)).collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            v
        },
        dropped: rows.iter().filter(|r| r.weight_raw().is_some()).filter_map(|r| r.weight_of.as_ref().map(|t| (r.ticker.clone(), t.clone()))).collect(),
    })
}

/// The sum of one view over a set of tickers (a share of the market value at D1 of the set).
#[derive(Debug, Clone, PartialEq)]
pub struct SetSum {
    pub view: View,
    pub n: usize,
    pub value1: f64,
    /// The share of the market value of the whole list, and the change of the set and of the part of the list outside it.
    pub share_of_list: f64,
    pub change: f64,
    pub change_excluded: f64,
    /// The parts in the order of `part_names`, and the same in the reverse order of rates and plug.
    pub parts: [f64; N_PARTS],
    pub parts_rev: [f64; N_PARTS],
    /// The five largest weights.
    pub top_weights: Vec<(String, f64, f64)>,
    /// For each part the five largest contributions in percentage points of the market value of the set: ticker, points.
    pub top_parts: Vec<Vec<(String, f64)>>,
}

/// Which tickers a sum has: the tickers with the view, or the tickers with both views.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SetKind {
    /// The tickers with this view.
    Own,
    /// The tickers with both views.
    Common,
}

/// A ticker that the sum shows as a flag: the premium that the price implies at D1 is below 1% (the cost of equity is near the
/// risk-free rate and the value is a very steep function of the premium), or a part is above 15% of P1.
pub fn flagged(c: &Chain, view: View) -> bool {
    (view == View::Premium && c.x1.x < 0.01) || c.pct().iter().any(|p| p.abs() > 0.15)
}

pub fn set_sum(rows: &[AttrRow], view: View, kind: SetKind, whole: &WholeList) -> Option<SetSum> {
    let list: Vec<(&AttrRow, &Chain)> = rows
        .iter()
        .filter(|r| r.weight().is_some())
        .filter_map(|r| {
            let a = r.outcome.as_ref().ok()?;
            let c = a.chain(view)?;
            if kind == SetKind::Common && a.chain(match view { View::Premium => View::Growth, View::Growth => View::Premium }).is_none() {
                return None;
            }
            Some((r, c))
        })
        .collect();
    let w = |r: &AttrRow| r.weight().unwrap_or(0.0);
    let value1: f64 = list.iter().map(|(r, _)| w(r)).sum();
    if list.is_empty() || value1 <= 0.0 {
        return None;
    }
    let dvalue: f64 = list.iter().map(|(r, _)| w(r) * r.change().unwrap_or(0.0)).sum();
    let wsum = |f: &dyn Fn(&Chain) -> f64| list.iter().map(|(r, c)| w(r) * f(c)).sum::<f64>() / value1;
    let rest_value = whole.value1 - value1;
    let mut top_parts = Vec::new();
    for k in 0..N_PARTS {
        let mut v: Vec<(String, f64)> = list.iter().map(|(r, c)| (r.ticker.clone(), w(r) * c.pct()[k] / value1)).collect();
        v.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()).then(a.0.cmp(&b.0)));
        v.truncate(5);
        top_parts.push(v);
    }
    let set_rows: Vec<&AttrRow> = list.iter().map(|(r, _)| *r).collect();
    Some(SetSum {
        view,
        n: list.len(),
        value1,
        share_of_list: value1 / whole.value1,
        change: dvalue / value1,
        change_excluded: if rest_value > 0.0 { (whole.dvalue - dvalue) / rest_value } else { f64::NAN },
        parts: [wsum(&|c| c.pct()[0]), wsum(&|c| c.pct()[1]), wsum(&|c| c.pct()[2]), wsum(&|c| c.pct()[3]), wsum(&|c| c.pct()[4])],
        parts_rev: [wsum(&|c| c.pct_rev()[0]), wsum(&|c| c.pct_rev()[1]), wsum(&|c| c.pct_rev()[2]), wsum(&|c| c.pct_rev()[3]), wsum(&|c| c.pct_rev()[4])],
        top_weights: top_weights(&set_rows, value1),
        top_parts,
    })
}

/// The number of lines of a full bridge: facts, time, rates, implied premium, implied growth, residual, no model value.
pub const N_BRIDGE: usize = 7;
pub const BRIDGE_NAMES: [&str; N_BRIDGE] =
    ["facts", "time (roll of the valuation date)", "rates", "implied premium (plug)", "implied growth (plug)", "search residual", "no model value (price change only)"];

/// A list with this many weighted tickers or fewer gets one bridge line for each ticker in the text.
pub const BRIDGE_TICKER_LINES: usize = 20;

/// One line of a full bridge: a group of the whole list (all, one sector, or one ticker).
#[derive(Debug, Clone, PartialEq)]
pub struct BridgeLine {
    pub name: String,
    pub n: usize,
    /// The share of the market value of the whole list at D1, and the change of the group (value-weighted).
    pub share: f64,
    pub change: f64,
    /// The contributions in percentage points (as a share) of the market value of the WHOLE list at D1; they add to
    /// `share x change`. `rev`: the same with the reverse order of rates and the plug.
    pub parts: [f64; N_BRIDGE],
    pub rev: [f64; N_BRIDGE],
}

/// The full bridge of the whole list: each ticker with a weight is in it once, with the view `first` where it exists, else the
/// other view, else on the line "no model value" (its price change). So the lines add to the change of the whole list. It is
/// one reading of the price: the two views give two readings of the same change (a higher return asked, or a lower growth
/// expected), and the model cannot tell them apart.
#[derive(Debug, Clone, PartialEq)]
pub struct Bridge {
    pub first: View,
    /// Tickers (and share of the market value of the list) with the first view, with the other view only, with no view.
    pub n_view: [usize; 3],
    pub share_view: [f64; 3],
    pub all: BridgeLine,
    pub sectors: Vec<BridgeLine>,
    /// One line for each ticker with a weight (the largest negative contribution first); they add to `all`.
    pub tickers: Vec<BridgeLine>,
    /// The largest contributions to the change of the list: ticker, share of the list, change, contribution (all shares), the view.
    pub top: Vec<(String, f64, f64, f64, &'static str)>,
}

/// The contributions of one row to a bridge (shares of the row's own price), and which view it uses (0 first, 1 other, 2 none).
fn bridge_row(r: &AttrRow, first: View) -> Option<([f64; N_BRIDGE], [f64; N_BRIDGE], usize)> {
    let change = r.change()?;
    let other = match first { View::Premium => View::Growth, View::Growth => View::Premium };
    let a = r.outcome.as_ref().ok();
    let (c, k, which) = match (a.and_then(|a| a.chain(first)), a.and_then(|a| a.chain(other))) {
        (Some(c), _) => (c, first, 0),
        (None, Some(c)) => (c, other, 1),
        (None, None) => {
            let mut p = [0.0; N_BRIDGE];
            p[6] = change;
            return Some((p, p, 2));
        }
    };
    let plug = if k == View::Premium { 3 } else { 4 };
    let put = |q: [f64; N_PARTS]| {
        let mut p = [0.0; N_BRIDGE];
        p[0] = q[0];
        p[1] = q[1];
        p[2] = q[2];
        p[plug] = q[3];
        p[5] = q[4];
        p
    };
    Some((put(c.pct()), put(c.pct_rev()), which))
}

fn bridge_line(name: String, rows: &[(&AttrRow, [f64; N_BRIDGE], [f64; N_BRIDGE])], total: f64) -> BridgeLine {
    let mut parts = [0.0; N_BRIDGE];
    let mut rev = [0.0; N_BRIDGE];
    let (mut v, mut dv) = (0.0, 0.0);
    for (r, p, q) in rows {
        let w = r.weight().unwrap_or(0.0);
        v += w;
        dv += w * r.change().unwrap_or(0.0);
        for k in 0..N_BRIDGE {
            parts[k] += w * p[k] / total;
            rev[k] += w * q[k] / total;
        }
    }
    BridgeLine { name, n: rows.len(), share: v / total, change: if v > 0.0 { dv / v } else { f64::NAN }, parts, rev }
}

pub fn full_bridge(rows: &[AttrRow], first: View) -> Option<Bridge> {
    let whole = whole_list(rows)?;
    let mut list = Vec::new();
    let (mut n_view, mut share_view) = ([0usize; 3], [0.0; 3]);
    let mut top = Vec::new();
    for r in rows.iter().filter(|r| r.weight().is_some()) {
        let Some((p, q, which)) = bridge_row(r, first) else { continue };
        let w = r.weight().unwrap_or(0.0) / whole.value1;
        n_view[which] += 1;
        share_view[which] += w;
        let view = match which { 0 => first.name(), 1 => match first { View::Premium => "growth", View::Growth => "premium" }, _ => "none" };
        top.push((r.ticker.clone(), w, r.change().unwrap_or(0.0), w * r.change().unwrap_or(0.0), view));
        list.push((r, p, q));
    }
    top.sort_by(|a, b| a.3.total_cmp(&b.3).then(a.0.cmp(&b.0)));
    top.truncate(10);
    let all = bridge_line("all".into(), &list, whole.value1);
    let mut by: BTreeMap<&str, Vec<(&AttrRow, [f64; N_BRIDGE], [f64; N_BRIDGE])>> = BTreeMap::new();
    for x in &list {
        by.entry(if x.0.sector.is_empty() { "(none)" } else { x.0.sector.as_str() }).or_default().push(*x);
    }
    let mut sectors: Vec<BridgeLine> = by.into_iter().map(|(k, v)| bridge_line(k.to_string(), &v, whole.value1)).collect();
    sectors.sort_by(|a, b| (a.share * a.change).total_cmp(&(b.share * b.change)).then(a.name.cmp(&b.name)));
    let mut tickers: Vec<BridgeLine> = list.iter().map(|x| bridge_line(x.0.ticker.clone(), std::slice::from_ref(x), whole.value1)).collect();
    tickers.sort_by(|a, b| (a.share * a.change).total_cmp(&(b.share * b.change)).then(a.name.cmp(&b.name)));
    Some(Bridge { first, n_view, share_view, all, sectors, tickers, top })
}

fn bridge_text(b: &Bridge) -> String {
    let other = match b.first { View::Premium => "growth", View::Growth => "premium" };
    let mut s = format!(
        "FULL BRIDGE, {} view first: each ticker of the whole list once ({} view: {} tickers, {:.1}% of the value; {} view where the {} view has no value: {}, {:.1}%; no model value: {}, {:.1}%). The lines add to the change of the whole list. Points of the market value of the WHOLE list at D1; the other order of rates and the plugs is on the line below the table.\n",
        b.first.name().to_uppercase(), b.first.name(), b.n_view[0], b.share_view[0] * 100.0, other, b.first.name(), b.n_view[1], b.share_view[1] * 100.0, b.n_view[2], b.share_view[2] * 100.0
    );
    let cells = |l: &BridgeLine| -> String { (0..N_BRIDGE).map(|k| format!("{:>8}", sgn(l.parts[k] * 100.0, 2))).collect::<Vec<_>>().join("") };
    s.push_str(&format!("  {:<8} {:>4} {:>7} {:>8} | {:>8}{:>8}{:>8}{:>8}{:>8}{:>8}{:>8} | {:>8}\n", "group", "n", "weight", "change", "facts", "time", "rates", "premium", "growth", "resid", "nomodel", "sum"));
    // a short list (up to `BRIDGE_TICKER_LINES` tickers) also gets one line for each ticker, after the sectors
    let per_ticker: &[BridgeLine] = if b.tickers.len() <= BRIDGE_TICKER_LINES { &b.tickers } else { &[] };
    for l in std::iter::once(&b.all).chain(b.sectors.iter()).chain(per_ticker.iter()) {
        s.push_str(&format!("  {:<8} {:>4} {:>6.1}% {:>7}% | {} | {:>8}\n", l.name, l.n, l.share * 100.0, sgn(l.change * 100.0, 2), cells(l), sgn(l.parts.iter().sum::<f64>() * 100.0, 2)));
    }
    s.push_str(&format!(
        "  the other order (plug before rates), all: rates {}, premium {}, growth {}\n",
        sgn(b.all.rev[2] * 100.0, 2),
        sgn(b.all.rev[3] * 100.0, 2),
        sgn(b.all.rev[4] * 100.0, 2)
    ));
    let v: Vec<String> = b.top.iter().map(|(t, w, c, p, view)| format!("{t} {} ({:.1}% x {}%, {view})", sgn(p * 100.0, 2), w * 100.0, sgn(c * 100.0, 1))).collect();
    s.push_str(&format!("  largest contributions to the change (points): {}\n", v.join(", ")));
    s
}

/// The flagged tickers of a view: ticker, the premium at D1 (premium view), the largest part in percent of P1 and its name.
pub fn flagged_list(rows: &[AttrRow], view: View) -> Vec<(String, f64, f64, &'static str)> {
    let names = part_names(view);
    let mut out = Vec::new();
    for r in rows {
        let Some(c) = r.outcome.as_ref().ok().and_then(|a| a.chain(view)) else { continue };
        if r.weight().is_none() || !flagged(c, view) {
            continue;
        }
        let p = c.pct();
        let k = (0..N_PARTS).max_by(|a, b| p[*a].abs().total_cmp(&p[*b].abs())).unwrap_or(0);
        out.push((r.ticker.clone(), c.x1.x, p[k], names[k]));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The output
// ---------------------------------------------------------------------------------------------

pub const CSV_HEADER: &str = "ticker,size,sector,industry,price1,price2,change_pct,p_facts_pct,p_time_pct,p_rates_pct,p_premium_pct,p_residual_pct,p_rev_rates_pct,p_rev_premium_pct,premium1,premium2,g_facts_pct,g_time_pct,g_rates_pct,g_growth_pct,g_residual_pct,g_rev_rates_pct,g_rev_growth_pct,growth_shift1,growth_shift2,growth10_1,growth10_2,growth_shift_at_premium1,growth10_at_premium1,shares,value1,curve1,curve2,new_facts,flags,refusal";

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

/// The CSV cells of a view: the five parts, the two parts of the reverse order that differ, the two free values.
fn chain_cells(c: Option<&Chain>) -> Vec<String> {
    match c {
        Some(c) => {
            let (p, q) = (c.pct(), c.pct_rev());
            vec![pc(p[0]), pc(p[1]), pc(p[2]), pc(p[3]), pc(p[4]), pc(q[2]), pc(q[3]), nm(c.x1.x, 4), nm(c.x2.x, 4)]
        }
        None => vec![String::new(); 9],
    }
}

pub fn csv_line(r: &AttrRow) -> String {
    let n = CSV_HEADER.split(',').count();
    let mut f = vec![String::new(); n];
    let idx = |name: &str| CSV_HEADER.split(',').position(|h| h == name).unwrap_or(0);
    f[0] = r.ticker.clone();
    f[idx("sector")] = cell(&r.sector);
    f[idx("industry")] = cell(&r.industry);
    if let Some((p1, p2)) = r.prices {
        f[idx("price1")] = num(p1, 2);
        f[idx("price2")] = num(p2, 2);
        f[idx("change_pct")] = pc(p2 / p1 - 1.0);
    }
    f[idx("shares")] = r.shares.map_or(String::new(), |s| num(s, 0));
    f[idx("value1")] = r.weight().map_or(String::new(), |w| num(w, 0));
    let mut flags: Vec<String> = Vec::new();
    if let Some(note) = &r.stale {
        flags.push(format!("price-stale({})", note.replace(", ", ";").replace(',', ";")));
    }
    if let Some(note) = &r.shares_note {
        flags.push(format!("shares-guard({})", note.replace(", ", ";").replace(',', ";")));
    }
    match &r.outcome {
        Ok(a) => {
            f[idx("size")] = a.size.clone();
            let p = chain_cells(a.chain(View::Premium));
            let g = chain_cells(a.chain(View::Growth));
            for (i, name) in ["p_facts_pct", "p_time_pct", "p_rates_pct", "p_premium_pct", "p_residual_pct", "p_rev_rates_pct", "p_rev_premium_pct", "premium1", "premium2"].iter().enumerate() {
                f[idx(name)] = p[i].clone();
            }
            for (i, name) in ["g_facts_pct", "g_time_pct", "g_rates_pct", "g_growth_pct", "g_residual_pct", "g_rev_rates_pct", "g_rev_growth_pct", "growth_shift1", "growth_shift2"].iter().enumerate() {
                f[idx(name)] = g[i].clone();
            }
            f[idx("growth10_1")] = nm(a.cagr1, 4);
            f[idx("growth10_2")] = nm(a.cagr2, 4);
            f[idx("growth_shift_at_premium1")] = a.growth_at_x1.as_ref().map_or(String::new(), |g| nm(g.x, 4));
            f[idx("growth10_at_premium1")] = nm(a.cagr_at_x1, 4);
            f[idx("curve1")] = a.curve1.clone();
            f[idx("curve2")] = a.curve2.clone();
            f[idx("new_facts")] = a.new_facts.to_string();
            flags.push(a.flags());
        }
        Err((key, _)) => f[n - 1] = cell(key),
    }
    f[idx("flags")] = cell(&flags.iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" "));
    f.join(",")
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
    s.push_str("This is an accounting split of the price change with the model; it is not a cause.\n");
    for view in [View::Premium, View::Growth] {
        let names = long_names(view);
        match a.chain(view) {
            None => {
                let why = match view { View::Premium => a.prem.as_ref().err(), View::Growth => a.grow.as_ref().err() };
                s.push_str(&format!("\n{} VIEW: none ({})\n", view.name().to_uppercase(), why.map_or("", String::as_str)));
            }
            Some(c) => {
                s.push_str(&format!("\n{} VIEW (the plug is the {})\n", view.name().to_uppercase(), match view { View::Premium => "equity risk premium that the price implies", View::Growth => "growth shift at the model premium that the price implies" }));
                s.push_str(&format!("{:<36} {:>10} {:>10} | order with the plug before the rates: {:>10} {:>10}\n", "part", "% of P1", "$ a share", "% of P1", "$ a share"));
                let (p, q) = (c.pct(), c.pct_rev());
                for (i, name) in names.iter().enumerate() {
                    s.push_str(&format!("{:<36} {:>10} {:>10} | {:>45} {:>10}\n", name, sgn(p[i] * 100.0, 2), sgn(p[i] * a.p1, 2), sgn(q[i] * 100.0, 2), sgn(q[i] * a.p1, 2)));
                }
                s.push_str(&format!("{:<36} {:>10} {:>10} | {:>45} {:>10}\n", "total", sgn(total * 100.0, 2), sgn(a.p2 - a.p1, 2), sgn(total * 100.0, 2), sgn(a.p2 - a.p1, 2)));
                s.push_str("  facts and time have a fixed position (first and second); only rates and the plug change places.\n");
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
    let (l1, l2) = a.curve_lag();
    for (n, l) in [("D1", l1), ("D2", l2)] {
        if let Some(l) = l {
            s.push_str(&format!("  the curve at {n} is {l} days older than the date (the source lags)\n"));
        }
    }
    let fl = a.flags();
    if !fl.is_empty() {
        s.push_str(&format!("  flags: {fl}\n"));
    }
    s
}

fn group_text(lines: &[GroupMedians], view: View) -> String {
    let n = part_names(view);
    let mut s = format!("{:<34} {:>5} {:>5} {:>9} | {:>8} {:>8} {:>8} {:>8} {:>8} | {:>9} {:>9}\n", "group", "n", "view", "change", n[0], n[1], n[2], n[3], n[4], "x at D1", "x at D2");
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

fn weights_text(top: &[(String, f64, f64)]) -> String {
    let v: Vec<String> = top.iter().map(|(t, _, sh)| format!("{t} {:.1}%", sh * 100.0)).collect();
    format!("  largest weights: {}\n", v.join(", "))
}

fn whole_text(w: &WholeList) -> String {
    let mut s = format!("WHOLE LIST: {} tickers with two prices and a share count (the models need not have a value: banks, REITs, insurers and refused tickers are in; one weight for each company); market value at D1 {:.1}B (price x shares); change {}% (value-weighted)\n", w.n, w.value1 / 1e9, sgn(w.change * 100.0, 2));
    if !w.dropped.is_empty() {
        let v: Vec<String> = w.dropped.iter().map(|(t, k)| format!("{t} (weight with {k})")).collect();
        s.push_str(&format!("  one company, one weight: the first ticker of the list carries the weight; no weight for {}\n", v.join(", ")));
    }
    s.push_str(&weights_text(&w.top));
    if !w.guarded.is_empty() {
        s.push_str(&format!("  share count replaced by the guard (a scale error): {}\n", w.guarded.join(" ")));
    }
    if !w.no_cik.is_empty() {
        s.push_str(&format!("  no CIK in the cache, so the one-weight rule could not apply to: {}\n", w.no_cik.join(" ")));
    }
    if !w.no_model.is_empty() {
        let total: f64 = w.no_model.iter().map(|x| x.1).sum();
        let v: Vec<String> = w.no_model.iter().map(|(t, sh)| format!("{t} {:.2}%", sh * 100.0)).collect();
        s.push_str(&format!("  {} tickers with no model value are in the whole list, {:.1}% of its market value (weight in the list): {}\n", w.no_model.len(), total * 100.0, v.join(", ")));
    }
    if !w.differs.is_empty() {
        s.push_str(&format!("  share count kept, but more than 2 times from the reference count (a split or one class of several): {}\n", w.differs.join(" ")));
    }
    s
}

/// The text of one sum. `headline` names the set; "covered part only" is in each headline of a set that is not the list.
fn set_text(headline: &str, w: &SetSum) -> String {
    let long = long_names(w.view);
    let mut s = format!(
        "{headline} ({} tickers, {:.1}% of the market value of the list at D1): change {}%; the rest of the list (not in this set) {}\n",
        w.n,
        w.share_of_list * 100.0,
        sgn(w.change * 100.0, 2),
        if w.change_excluded.is_finite() { format!("{}%", sgn(w.change_excluded * 100.0, 2)) } else { "none".to_string() }
    );
    s.push_str(&weights_text(&w.top_weights));
    s.push_str("  part, as a share of the market value of the set at D1 (percent); for rates and the plug the range over the two orders; top 5 contributors in percentage points\n");
    for (k, name) in long.iter().enumerate() {
        let (a, b) = (w.parts[k] * 100.0, w.parts_rev[k] * 100.0);
        let range = if k == 2 || k == 3 { format!("{}..{}", sgn(a.min(b), 2), sgn(a.max(b), 2)) } else { "fixed position".to_string() };
        let top: Vec<String> = w.top_parts[k].iter().map(|(t, p)| format!("{t} {}", sgn(p * 100.0, 2))).collect();
        s.push_str(&format!("  {:<36} {:>8}%  ({range})  top: {}\n", name, sgn(a, 2), top.join(", ")));
    }
    s
}

/// The whole-list change over other sets, so that a number of another report can be put next to this one.
fn variants_text(rows: &[AttrRow]) -> String {
    let template = |r: &AttrRow| matches!(&r.outcome, Err((k, _)) if k.starts_with("template"));
    let has_value = |r: &AttrRow| r.outcome.is_ok();
    type Keep = Box<dyn Fn(&AttrRow) -> bool>;
    let sets: [(&str, bool, Keep); 5] = [
        ("all tickers with two prices, one weight for each company (the number above)", true, Box::new(|_| true)),
        ("all tickers with two prices, every ticker its own weight (a company with two classes counted two times)", false, Box::new(|_| true)),
        ("without banks, REITs, insurers and new filers (a template with no model)", true, Box::new(move |r| !template(r))),
        ("only tickers with a model value", true, Box::new(has_value)),
        ("only tickers with a model value, every ticker its own weight", false, Box::new(has_value)),
    ];
    let mut s = String::from("  the same change over other sets:\n");
    for (name, one, keep) in sets.iter() {
        match whole_where(rows, *one, keep.as_ref()) {
            Some(w) => s.push_str(&format!("    {:>7}%  {} tickers, {:.1}B at D1: {name}\n", sgn(w.change * 100.0, 2), w.n, w.value1 / 1e9)),
            None => s.push_str(&format!("    none: {name}\n")),
        }
    }
    s
}

fn flags_text(rows: &[AttrRow], view: View) -> String {
    let fl = flagged_list(rows, view);
    let mut s = format!("  FLAGGED ({} tickers: the implied premium at D1 below 1% (premium view), or a part above 15% of P1): ", fl.len());
    let items: Vec<String> = fl.iter().map(|(t, x, p, n)| format!("{t} (x1 {:.2}%, {n} {}%)", x * 100.0, sgn(p * 100.0, 1))).collect();
    s.push_str(&items.join(", "));
    s.push('\n');
    s
}

/// The headline block of a list run.
pub fn list_text(rows: &[AttrRow]) -> String {
    let Some(whole) = whole_list(rows) else { return "WHOLE LIST: no ticker has two prices and a share count\n".to_string() };
    let mut s = String::from("\nThis is an accounting split of the price change with the model; it is not a cause. facts and time have a fixed position (first and second); rates and the plug are shown as the range of the two orders.\n\n");
    s.push_str(&whole_text(&whole));
    let stale: Vec<String> = rows.iter().filter_map(|r| r.stale.as_ref().map(|n| format!("{} ({n})", r.ticker))).collect();
    if !stale.is_empty() {
        s.push_str(&format!("  STALE PRICE, left out of every sum ({} tickers: a close older than the SPY close of the same date): {}\n", stale.len(), stale.join(", ")));
    }
    s.push_str(&variants_text(rows));
    let old = |side: usize| rows.iter().filter(|r| r.outcome.as_ref().ok().is_some_and(|a| if side == 1 { a.curve_lag().0 } else { a.curve_lag().1 }.is_some())).count();
    s.push_str(&format!("  rows whose curve is more than {CURVE_LAG_DAYS} days older than the date: {} at D1, {} at D2 (the curve source lags)\n", old(1), old(2)));
    s.push_str("\n(2) COMMON SET: the tickers that have both views. Covered part only.\n");
    for view in [View::Premium, View::Growth] {
        match set_sum(rows, view, SetKind::Common, &whole) {
            Some(w) => s.push_str(&set_text(&format!("  {} VIEW on the common set, covered part only", view.name().to_uppercase()), &w)),
            None => s.push_str("  no ticker has both views\n"),
        }
    }
    s.push_str("\n(3) EACH VIEW ON ITS OWN SET. Covered part only.\n");
    for view in [View::Premium, View::Growth] {
        match set_sum(rows, view, SetKind::Own, &whole) {
            Some(w) => {
                s.push_str(&set_text(&format!("{} VIEW, covered part only", view.name().to_uppercase()), &w));
                s.push_str(&flags_text(rows, view));
            }
            None => s.push_str(&format!("{} VIEW: no ticker has the view\n", view.name().to_uppercase())),
        }
    }
    s.push_str("\n(4) FULL BRIDGE OF THE WHOLE LIST: two readings of the same change.\n");
    for view in [View::Premium, View::Growth] {
        if let Some(b) = full_bridge(rows, view) {
            s.push_str(&bridge_text(&b));
        }
    }
    let count = |f: &dyn Fn(&Attribution) -> bool| rows.iter().filter(|r| r.weight().is_some() && r.outcome.as_ref().ok().is_some_and(f)).count();
    s.push_str(&format!(
        "\nSETS (tickers with a weight): both views {}, premium only {}, growth only {}, neither {} (a model refusal has neither)\n",
        count(&|a| a.prem.is_ok() && a.grow.is_ok()),
        count(&|a| a.prem.is_ok() && a.grow.is_err()),
        count(&|a| a.prem.is_err() && a.grow.is_ok()),
        rows.iter().filter(|r| r.weight().is_some()).count() - count(&|a| a.prem.is_ok() || a.grow.is_ok())
    ));
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
    let o = Opts::parse(args, &["--cache", "--out", "--jobs", "--table", "--from", "--to", "--tickers-file", "--rules", "--groups", "--curve"], &[])?;
    let source = o.get("--curve").unwrap_or("gsw").to_string();
    if !["gsw", "fedinvest", "par"].contains(&source.as_str()) {
        return e(format!("--curve '{source}': use gsw, fedinvest or par"));
    }
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
    let (mut curve1, index) = curve_and_index(&cache, &d1);
    let (mut curve2, _) = curve_and_index(&cache, &d2);
    if source != "gsw" {
        let load = |d: &str| if source == "par" { super::auto::load_curve_par(&cache, d) } else { super::auto::load_curve_fedinvest(&cache, d) };
        curve1 = Some(load(&d1).map_err(DcfError)?);
        curve2 = Some(load(&d2).map_err(DcfError)?);
        for c in [&curve1, &curve2].into_iter().flatten() {
            println!("curve ({source} fit) {}: 10-year zero {:.4}%", c.date, c.zero10 * 100.0);
        }
    }
    let market1_of = |t: &str| -> Market { market_at(&cache, t, &d1, None, Some(&d1), &rules, curve1.as_ref(), index.as_deref()) };
    let market2_of = |t: &str| -> Market { market_at(&cache, t, &d2, None, Some(&d2), &rules, curve2.as_ref(), index.as_deref()) };
    let cfg = ImpliedConfig { table: &table, fade: &fade, rules: rules.clone(), cache: Some(&cache) };
    let t1 = Instant::now();
    let mut rows = attribute_list(&inputs1, &inputs2, &market1_of, &market2_of, &groups, &cfg, jobs)?;
    // a close older than the SPY close of the same date takes the row out of the sums; the one-weight rule runs again after it
    finish_weights(&mut rows, &|t: &str| super::auto::price_series(&cache, t), &d1, &d2);
    let attr_s = t1.elapsed().as_secs_f64();
    if let Some(path) = o.get("--out") {
        std::fs::write(path, csv_text(&rows)).map_err(|x| DcfError(format!("{path}: {x}")))?;
    }
    println!("ATTRIBUTION of the change of the price from {d1} to {d2}, {} tickers, no random part. {BANNER} A market-implied value is a measurement, not an input.", rows.len());
    println!("  The value is the value of the one path at the means (not the expected value). Order of the steps: facts, time, rates, plug (implied premium or implied growth), residual; the reverse order of rates and the plug is shown. This is an accounting split of the price change with the model; it is not a cause. Facts of each date: filed on or before it; the 10-K of the cache does not fill an input. Price: last close on or before the date. Curve: last GSW row on or before the date (with --curve fedinvest or par: the NSS fit to the last FedInvest file or Treasury par row on or before it).");
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
        print!("{}", list_text(&rows));
        for view in [View::Premium, View::Growth] {
            println!("\nGROUPS, {} VIEW", view.name().to_uppercase());
            print!("{}", group_text(&group_medians(&rows, view), view));
        }
    }
    println!("\nTIME: mapping {:.2} s | attribution {attr_s:.2} s ({} threads) | total {:.2} s", map1 + map2, jobs.unwrap_or_else(rayon::current_num_threads), t0.elapsed().as_secs_f64());
    Ok(())
}

#[cfg(test)]
#[path = "auto_attribute_tests.rs"]
mod tests;
