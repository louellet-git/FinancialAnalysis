//! From company facts to the standard variables of one period.
//!
//! This is the only implementation of the mapping. The valuation code (`fa`) reads
//! the result from the store file (rust/mapcheck/src/store.rs).

use crate::facts::{period_ends, select, CompanyFacts, Forms};
use crate::registry::{Period, Registry};
use crate::supplement::{Filled, FillMode, FILLED, FROM_LINES, TRIGGERS};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct PeriodData {
    pub end: String,
    pub forms: Forms,
    /// The filing date of the revenue fact, or the latest filing date of the facts in use.
    pub filed: Option<String>,
    /// The filing date of the fact that gave the value of each registry variable (a tag value only: `None` for a value
    /// that a formula or a 10-K line gave), in the registry sequence. Empty in a period that was not made by the mapper.
    pub filed_of: Vec<Option<String>>,
    /// One entry for each registry variable, in the registry sequence.
    pub values: Vec<Option<f64>>,
    pub sources: Vec<Option<String>>,
    /// us-gaap USD tags that have no standard variable: tag -> value.
    pub raw_extras: BTreeMap<String, f64>,
}

impl PeriodData {
    pub fn get(&self, registry: &Registry, name: &str) -> Option<f64> {
        registry.try_var(name).and_then(|i| self.values[i])
    }

    /// The filing date of the tag fact that gave the value of a variable (`None`: no value, or a formula value).
    pub fn filed_of(&self, registry: &Registry, name: &str) -> Option<&str> {
        registry.try_var(name).and_then(|i| self.filed_of.get(i)).and_then(|d| d.as_deref())
    }

    pub fn source(&self, registry: &Registry, name: &str) -> &str {
        registry.try_var(name).and_then(|i| self.sources[i].as_deref()).unwrap_or("")
    }

    /// True if the value came from a tag (false: from a formula, or no value).
    pub fn from_tag(&self, i: usize) -> bool {
        matches!(&self.sources[i], Some(s) if !s.starts_with('='))
    }

    /// The variables whose value came from the 10-K, not from the company facts.
    pub fn from_filing<'a>(&'a self, registry: &'a Registry) -> Vec<(&'a str, &'a str)> {
        registry
            .variables
            .iter()
            .zip(&self.sources)
            .filter_map(|(v, s)| s.as_deref().filter(|s| crate::supplement::from_filing(s)).map(|s| (v.name.as_str(), s)))
            .collect()
    }
}

struct FormulaRun<'a> {
    registry: &'a Registry,
    values: &'a mut Vec<Option<f64>>,
    sources: &'a mut Vec<Option<String>>,
    in_work: Vec<bool>,
    /// Variables whose formula value is not used (it is larger than its total).
    blocked: &'a [bool],
}

impl FormulaRun<'_> {
    fn resolve(&mut self, i: usize) -> Option<f64> {
        if let Some(v) = self.values[i] {
            return Some(v);
        }
        let registry = self.registry;
        let variable = &registry.variables[i];
        if variable.formulas.is_empty() || self.in_work[i] || self.blocked[i] {
            return None;
        }
        self.in_work[i] = true;
        let mut result = None;
        for formula in &variable.formulas {
            let all_plus = formula.terms.iter().all(|t| t.sign > 0.0);
            let mut total = 0.0;
            let mut used: Vec<usize> = Vec::new();
            let mut seen: Vec<(f64, usize)> = Vec::new();
            let mut same: Vec<String> = Vec::new();
            let mut ok = true;
            for term in &formula.terms {
                let value = match self.resolve(term.var) {
                    Some(v) => v,
                    None if term.optional => continue,
                    None => {
                        ok = false;
                        break;
                    }
                };
                used.push(term.var);
                // Two terms of a sum with the same value are one item that the filer
                // gave two tags. Count the value one time.
                if term.sign > 0.0 && value != 0.0 && all_plus {
                    if let Some(&(_, first)) = seen.iter().find(|(v, _)| *v == value) {
                        same.push(format!("{} = {}", registry.variables[first].name, registry.variables[term.var].name));
                        continue;
                    }
                }
                if !seen.iter().any(|(v, _)| *v == value) {
                    seen.push((value, term.var));
                }
                total += term.sign * value;
            }
            if ok && !used.is_empty() {
                let missing: Vec<&str> = formula
                    .terms
                    .iter()
                    .filter(|t| t.optional && !used.contains(&t.var))
                    .map(|t| registry.variables[t.var].name.as_str())
                    .collect();
                let mut notes: Vec<String> = Vec::new();
                if !missing.is_empty() {
                    notes.push(format!("no value: {}", missing.join(", ")));
                }
                if !same.is_empty() {
                    notes.push(format!("same value, counted one time: {}", same.join("; ")));
                }
                let note = if notes.is_empty() { String::new() } else { format!(" ({})", notes.join("; ")) };
                self.values[i] = Some(total);
                self.sources[i] = Some(format!("= {}{}", formula.text.replace('?', ""), note));
                result = Some(total);
                break;
            }
        }
        self.in_work[i] = false;
        result
    }
}

pub const SHARE_SCALE_FACTORS: [(f64, &str); 2] = [(1_000.0, "1,000"), (1_000_000.0, "1,000,000")];
pub const SHARE_SCALE_TOLERANCE: f64 = 0.10;
pub const MIN_EPS_FOR_SHARE_CHECK: f64 = 0.05;

/// Net income for the common shareholders divided by EPS (None if not available).
pub fn implied_share_count(registry: &Registry, values: &[Option<f64>], eps_name: &str) -> Option<f64> {
    let value = |name: &str| registry.try_var(name).and_then(|i| values[i]);
    let eps = value(eps_name)?;
    let income = value("net_income_to_common").or_else(|| value("net_income"))?;
    if eps.abs() < MIN_EPS_FOR_SHARE_CHECK {
        return None;
    }
    let implied = income / eps;
    if implied > 0.0 {
        Some(implied)
    } else {
        None
    }
}

/// Correct a share count that the filer reports in thousands or in millions (for
/// example 716.4 for 716.4 million shares). Net income divided by EPS gives the real
/// size.
fn correct_share_scale(registry: &Registry, values: &mut [Option<f64>], sources: &mut [Option<String>]) {
    let mut applied: Option<(f64, &str)> = None;
    for (shares_name, eps_name) in [("shares_diluted", "eps_diluted"), ("shares_basic", "eps_basic")] {
        let Some(i) = registry.try_var(shares_name) else { continue };
        let Some(shares) = values[i] else { continue };
        let Some(implied) = implied_share_count(registry, values, eps_name) else { continue };
        if shares <= 0.0 {
            continue;
        }
        for (factor, text) in SHARE_SCALE_FACTORS {
            if (implied / (shares * factor) - 1.0).abs() <= SHARE_SCALE_TOLERANCE {
                values[i] = Some(shares * factor);
                sources[i] = Some(format!("{} (x{}: scale from net income / EPS)", sources[i].as_deref().unwrap_or(""), text));
                applied = Some((factor, text));
                break;
            }
        }
    }
    // A filer with no share-count tag (for example VTRS): net income / EPS gives the count.
    if let Some(i) = registry.try_var("shares_diluted") {
        if values[i].is_none() {
            if let Some(implied) = implied_share_count(registry, values, "eps_diluted") {
                values[i] = Some(implied);
                sources[i] = Some("= net income / eps_diluted (the filer has no share-count tag)".to_string());
            }
        }
    }
    // The balance-sheet share count has the same scale if it is near the weighted count.
    let Some((factor, text)) = applied else { return };
    let value = |name: &str, values: &[Option<f64>]| registry.try_var(name).and_then(|i| values[i]);
    let weighted = value("shares_diluted", values).filter(|v| *v != 0.0).or_else(|| value("shares_basic", values).filter(|v| *v != 0.0));
    if let (Some(i), Some(weighted)) = (registry.try_var("shares_outstanding"), weighted) {
        if let Some(outstanding) = values[i].filter(|v| *v != 0.0) {
            let ratio = outstanding * factor / weighted;
            if (0.5..=2.0).contains(&ratio) {
                values[i] = Some(outstanding * factor);
                sources[i] = Some(format!(
                    "{} (x{}: same scale as the weighted share count)",
                    sources[i].as_deref().unwrap_or(""),
                    text
                ));
            }
        }
    }
}

/// A source of values from the 10-K: (period end, variable, mode) -> (value, source text).
/// `Gap`: a critical input that the company facts do not have. `Lines`: the capex lines of the
/// 10-K replace the company-facts value. See supplement.rs.
pub type Fill<'a> = &'a dyn Fn(&str, &str, FillMode) -> Option<Filled>;

/// A formula value can be larger than its total by this share (rounding).
pub const FORMULA_OVER_TOTAL: f64 = 0.02;
/// In the source text of a variable whose formula value is not used.
pub const NOT_USED: &str = ": not used,";

/// The values that come from formulas, and the share-count corrections.
///
/// A formula value that is larger than its total (the `[[within]]` rules of the
/// registry) counts an amount two times: the filer gave two tags to one amount, or one
/// term is a part of a different term. Such a value is not used: the variable has no
/// value, and the formulas run again without it.
fn finish(registry: &Registry, tag_values: Vec<Option<f64>>, tag_sources: Vec<Option<String>>, mixed_vintage: bool) -> (Vec<Option<f64>>, Vec<Option<String>>) {
    let n = registry.variables.len();
    let mut blocked = vec![false; n];
    let mut notes: Vec<Option<String>> = vec![None; n];
    // Gross profit from revenue and cost of revenue of two different filings (a restatement in
    // between) mixes two bases: no value.
    let gross_profit = registry.try_var("gross_profit");
    if let (true, Some(g)) = (mixed_vintage, gross_profit) {
        blocked[g] = true;
        notes[g] = Some(format!("{}{NOT_USED} revenue and cogs_total are from different filings", registry.variables[g].formulas.first().map_or("=".to_string(), |f| format!("= {}", f.text))));
    }
    loop {
        let (mut values, mut sources) = (tag_values.clone(), tag_sources.clone());
        let mut run = FormulaRun { registry, values: &mut values, sources: &mut sources, in_work: vec![false; n], blocked: &blocked };
        for i in 0..n {
            run.resolve(i);
        }
        correct_share_scale(registry, &mut values, &mut sources);
        let over: Vec<usize> = registry
            .within
            .iter()
            .filter_map(|&(part, total)| match (values[part], values[total]) {
                (Some(p), Some(t)) if t != 0.0 && p.abs() > t.abs() * (1.0 + FORMULA_OVER_TOTAL) => Some((part, total, p, t)),
                _ => None,
            })
            .filter(|&(part, ..)| !blocked[part] && matches!(&sources[part], Some(s) if s.starts_with('=')))
            .map(|(part, total, p, t)| {
                notes[part] = Some(format!(
                    "{}{NOT_USED} {} is more than {} {}",
                    sources[part].as_deref().unwrap_or("="),
                    p,
                    registry.variables[total].name,
                    t
                ));
                part
            })
            .collect();
        if over.is_empty() {
            // The source text of a variable with no value says why it has no value.
            for i in 0..n {
                if values[i].is_none() && notes[i].is_some() {
                    sources[i] = notes[i].take();
                }
            }
            return (values, sources);
        }
        for part in over {
            blocked[part] = true;
        }
    }
}

/// The standard variables for one period.
pub fn resolve_period(registry: &Registry, facts: &CompanyFacts, end: &str, forms: Forms) -> PeriodData {
    resolve_period_with(registry, facts, end, forms, None)
}

/// The standard variables for one period. `fill` gives a value for a critical input
/// of a fiscal year that has no value in the company facts.
pub fn resolve_period_with(registry: &Registry, facts: &CompanyFacts, end: &str, forms: Forms, fill: Option<Fill>) -> PeriodData {
    let n = registry.variables.len();
    let mut values: Vec<Option<f64>> = vec![None; n];
    let mut sources: Vec<Option<String>> = vec![None; n];
    let mut filed_dates: Vec<Option<String>> = vec![None; n];
    let mut accn_of: Vec<String> = vec![String::new(); n];
    let mut tag_of: Vec<Option<&crate::registry::Tag>> = vec![None; n];

    // A tag with the scope of a fund or a mortgage REIT (`unclassified_only`) gives no value to a
    // filer that has a current-assets total (in any period), or whose value is a very small share of total assets.
    let first_value = |name: &str| {
        let i = registry.try_var(name)?;
        let v = &registry.variables[i];
        v.tags.iter().find_map(|t| select(facts.entries(t, &v.unit), end, forms, v.period)).map(|s| s.value)
    };
    // The filer decides, not the period: a filer with a current-assets total in any period has a
    // classified balance sheet, also in a period that has only income-statement facts.
    let classified = registry.try_var("total_current_assets").is_some_and(|i| {
        let v = &registry.variables[i];
        v.tags.iter().any(|t| !facts.entries(t, &v.unit).is_empty())
    });
    let assets = first_value("total_assets").filter(|a| *a > 0.0);
    for (i, variable) in registry.variables.iter().enumerate() {
        for tag in &variable.tags {
            if variable.unclassified_only.contains(&tag.full) {
                if classified {
                    continue;
                }
                // The share is for a year; a quarter has about a quarter of it.
                let year_share = if forms == Forms::Annual { 1.0 } else { 0.25 };
                let small = |v: f64| assets.is_some_and(|a| v.abs() < year_share * variable.scoped_min_assets_share * a);
                let Some(sel) = select(facts.entries(tag, &variable.unit), end, forms, variable.period) else { continue };
                if small(sel.value) {
                    continue;
                }
            }
            let Some(sel) = select(facts.entries(tag, &variable.unit), end, forms, variable.period) else {
                continue;
            };
            values[i] = Some(if variable.sign_flip {
                -sel.value.abs()
            } else if variable.negate {
                -sel.value
            } else {
                sel.value
            });
            sources[i] = Some(if sel.note.is_empty() { tag.full.clone() } else { format!("{} ({})", tag.full, sel.note) });
            if !sel.filed.is_empty() {
                filed_dates[i] = Some(sel.filed);
            }
            accn_of[i] = sel.accn;
            tag_of[i] = Some(tag);
            break;
        }
    }

    let (mut tag_values, mut tag_sources) = (values, sources);
    // The company-facts value stays in its own variable when the 10-K lines replace it.
    for (i, variable) in registry.variables.iter().enumerate() {
        if let Some(target) = variable.companyfacts_of.as_deref().and_then(|name| registry.try_var(name)) {
            tag_values[i] = tag_values[target];
            tag_sources[i] = tag_sources[target].clone();
        }
    }
    // Capex lines of the 10-K for each fiscal year that the 10-K holds: they replace the
    // company-facts value (calculation tree rule). An earlier year keeps the company facts.
    if let (Forms::Annual, Some(fill)) = (forms, fill) {
        for name in FROM_LINES {
            let Some(i) = registry.try_var(name) else { continue };
            match fill(end, name, FillMode::Lines) {
                Some(Filled::Value(value, source)) => {
                    tag_values[i] = Some(value);
                    tag_sources[i] = Some(source);
                }
                Some(Filled::Absent(source)) => {
                    tag_values[i] = None;
                    tag_sources[i] = Some(source);
                }
                Some(Filled::Keep(note)) => {
                    if let Some(source) = tag_sources[i].as_mut() {
                        source.push_str(&format!(" ({note})"));
                    }
                }
                None => {}
            }
        }
    }
    // Revenue and cost of revenue of one period from two filings, where the filing of the one has a
    // different value for the other (a restatement): two bases.
    let mixed_vintage = {
        let pick = |names: &[&str]| {
            names.iter().find_map(|name| {
                let i = registry.try_var(name)?;
                let tag = tag_of[i]?;
                let value = tag_values[i]?;
                (!accn_of[i].is_empty()).then(|| (i, tag, value, accn_of[i].clone()))
            })
        };
        // The value of a tag in one filing (the same period).
        let in_filing = |i: usize, tag: &crate::registry::Tag, accn: &str| {
            let variable = &registry.variables[i];
            let own: Vec<_> = facts.entries(tag, &variable.unit).iter().filter(|e| e.accn.as_deref() == Some(accn)).cloned().collect();
            select(&own, end, forms, variable.period).map(|sel| if variable.sign_flip { -sel.value.abs() } else if variable.negate { -sel.value } else { sel.value })
        };
        match (pick(&["revenue", "revenue_from_contracts", "revenue_total_pre2018"]), pick(&["cogs_total"])) {
            (Some((ri, rt, rv, ra)), Some((ci, ct, cv, ca))) if ra != ca => {
                in_filing(ri, rt, &ca).is_some_and(|v| v != rv) || in_filing(ci, ct, &ra).is_some_and(|v| v != cv)
            }
            _ => false,
        }
    };
    let (values, sources) = finish(registry, tag_values.clone(), tag_sources.clone(), mixed_vintage);

    // The revenue fact (most companies have it as contract revenue).
    let revenue_filed = ["revenue", "revenue_from_contracts", "revenue_total_pre2018"].iter().find_map(|name| registry.try_var(name).and_then(|i| filed_dates[i].clone()));
    let filed = revenue_filed.or_else(|| filed_dates.iter().flatten().max().cloned());

    let mut raw_extras = BTreeMap::new();
    if let Some(gaap) = facts.facts.get("us-gaap") {
        for (name, concept) in gaap {
            let full = format!("us-gaap/{name}");
            if registry.canonical.contains(&full) {
                continue;
            }
            let Some(entries) = concept.units.get("USD") else { continue };
            if entries.is_empty() {
                continue;
            }
            let period = if entries.iter().any(|e| e.start.is_some()) { Period::Duration } else { Period::Instant };
            if let Some(sel) = select(entries, end, forms, period) {
                raw_extras.insert(full, sel.value);
            }
        }
    }

    let mut period = PeriodData { end: end.to_string(), forms, filed, filed_of: filed_dates, values, sources, raw_extras };
    if let (Forms::Annual, Some(fill)) = (forms, fill) {
        let critical = crate::checks::critical_for(crate::checks::filer_template(registry, &period));
        if TRIGGERS.iter().any(|name| critical.contains(name) && period.get(registry, name).is_none()) {
            let (mut values, mut sources, mut changed) = (tag_values, tag_sources, false);
            for name in FILLED {
                let Some(i) = registry.try_var(name) else { continue };
                if period.values[i].is_none() {
                    if let Some(Filled::Value(value, source)) = fill(end, name, FillMode::Gap) {
                        values[i] = Some(value);
                        sources[i] = Some(source);
                        changed = true;
                    }
                }
            }
            if changed {
                // The formulas run again: a filled value is a term of other variables.
                (period.values, period.sources) = finish(registry, values, sources, mixed_vintage);
            }
        }
    }
    period
}

#[derive(Debug, Clone)]
pub struct Company {
    pub name: String,
    pub annual: Vec<PeriodData>,
    pub quarterly: Vec<PeriodData>,
}

/// The latest `n_periods` fiscal years and `n_periods` quarters.
pub fn map_company(registry: &Registry, facts: &CompanyFacts, n_periods: usize) -> Company {
    map_company_with(registry, facts, n_periods, None)
}

pub fn map_company_with(registry: &Registry, facts: &CompanyFacts, n_periods: usize, fill: Option<Fill>) -> Company {
    let mut out = Company {
        name: facts.entity_name.clone().unwrap_or_default(),
        annual: Vec::new(),
        quarterly: Vec::new(),
    };
    for forms in [Forms::Annual, Forms::Quarterly] {
        for end in period_ends(facts, &registry.anchors, forms, Some(n_periods)) {
            let period = resolve_period_with(registry, facts, &end, forms, fill);
            match forms {
                Forms::Annual => out.annual.push(period),
                Forms::Quarterly => out.quarterly.push(period),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::testing::{company, fact};

    const FY: (Option<&str>, &str) = (Some("2024-01-01"), "2024-12-31");

    fn flow(val: f64) -> Vec<crate::facts::Fact> {
        vec![fact(FY.0, FY.1, val, "10-K", "2025-02-01")]
    }

    fn flow_at(start: &str, end: &str, val: f64) -> Vec<crate::facts::Fact> {
        vec![fact(Some(start), end, val, "10-K", "2023-02-01")]
    }

    fn balance(val: f64) -> Vec<crate::facts::Fact> {
        vec![fact(None, FY.1, val, "10-K", "2025-02-01")]
    }

    fn resolve(concepts: Vec<(&str, Vec<crate::facts::Fact>)>) -> (Registry, PeriodData) {
        let registry = Registry::embedded().unwrap();
        let facts = company(concepts);
        let period = resolve_period(&registry, &facts, FY.1, Forms::Annual);
        (registry, period)
    }

    #[test]
    fn same_variable_from_different_tags() {
        let (r, a) = resolve(vec![("Revenues", flow(100.0)), ("InterestExpense", flow(5.0))]);
        let (_, b) = resolve(vec![("Revenues", flow(100.0)), ("InterestExpenseNonoperating", flow(7.0))]);
        assert_eq!(a.get(&r, "interest_expense"), Some(5.0));
        assert_eq!(b.get(&r, "interest_expense"), Some(7.0));
        assert_eq!(a.source(&r, "interest_expense"), "us-gaap/InterestExpense");
        assert_eq!(b.source(&r, "interest_expense"), "us-gaap/InterestExpenseNonoperating");
    }

    #[test]
    fn formula_fills_a_variable_with_no_tag_and_a_tag_wins_over_a_formula() {
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("CostOfRevenue", flow(60.0))]);
        assert_eq!(p.get(&r, "gross_profit"), Some(40.0));
        assert_eq!(p.source(&r, "gross_profit"), "= revenue - cogs_total");
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("CostOfRevenue", flow(60.0)), ("GrossProfit", flow(41.0))]);
        assert_eq!(p.get(&r, "gross_profit"), Some(41.0));
        assert_eq!(p.source(&r, "gross_profit"), "us-gaap/GrossProfit");
    }

    #[test]
    fn total_debt_from_parts() {
        let (r, p) = resolve(vec![
            ("Revenues", flow(100.0)),
            ("CommercialPaper", balance(2.0)),
            ("LongTermDebtCurrent", balance(10.0)),
            ("LongTermDebtNoncurrent", balance(68.0)),
        ]);
        assert_eq!(p.get(&r, "debt_current"), Some(12.0));
        assert_eq!(p.get(&r, "long_term_debt_total"), Some(78.0));
        assert_eq!(p.get(&r, "total_debt"), Some(80.0));
    }

    #[test]
    fn two_tags_with_the_same_value_count_one_time() {
        let (r, p) = resolve(vec![
            ("Revenues", flow(100.0)),
            ("ShortTermBorrowings", balance(6.4)),
            ("LongTermDebtAndCapitalLeaseObligationsCurrent", balance(6.4)),
            ("LongTermDebtNoncurrent", balance(54.8)),
        ]);
        assert_eq!(p.get(&r, "debt_current"), Some(6.4));
        assert!(p.source(&r, "debt_current").contains("counted one time"));
        assert!((p.get(&r, "total_debt").unwrap() - 61.2).abs() < 1e-9);
    }

    #[test]
    fn optional_term_with_no_value_is_recorded() {
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("Depreciation", flow(8.0))]);
        assert_eq!(p.get(&r, "da_total"), Some(8.0));
        assert!(p.source(&r, "da_total").contains("no value: opex_amortization"));
    }

    #[test]
    fn sign_flip_gives_a_negative_outflow() {
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("PaymentsToAcquirePropertyPlantAndEquipment", flow(9.0))]);
        assert_eq!(p.get(&r, "capex"), Some(-9.0));
    }

    fn shares_company(shares: f64, eps: f64, income: f64) -> (Registry, PeriodData) {
        let registry = Registry::embedded().unwrap();
        let mut facts = company(vec![("Revenues", flow(26_000_000_000.0)), ("NetIncomeLoss", flow(income))]);
        let gaap = facts.facts.get_mut("us-gaap").unwrap();
        for (name, unit, val) in [
            ("EarningsPerShareDiluted", "USD/shares", eps),
            ("WeightedAverageNumberOfDilutedSharesOutstanding", "shares", shares),
        ] {
            let mut units = std::collections::HashMap::new();
            units.insert(unit.to_string(), flow(val));
            gaap.insert(name.to_string(), crate::facts::Concept { units });
        }
        let period = resolve_period(&registry, &facts, FY.1, Forms::Annual);
        (registry, period)
    }

    #[test]
    fn share_count_in_millions_is_corrected() {
        let (r, p) = shares_company(716.4, 11.95, 8_563_000_000.0);
        assert!((p.get(&r, "shares_diluted").unwrap() - 716_400_000.0).abs() < 1.0);
        assert!(p.source(&r, "shares_diluted").contains("x1,000,000"));
    }

    #[test]
    fn share_count_from_net_income_and_eps_when_the_filer_has_no_tag() {
        let registry = Registry::embedded().unwrap();
        let mut facts = company(vec![("Revenues", flow(100.0)), ("NetIncomeLoss", flow(-3_510_000_000.0))]);
        let mut units = std::collections::HashMap::new();
        units.insert("USD/shares".to_string(), flow(-3.0));
        facts.facts.get_mut("us-gaap").unwrap().insert("EarningsPerShareDiluted".to_string(), crate::facts::Concept { units });
        let p = resolve_period(&registry, &facts, FY.1, Forms::Annual);
        assert!((p.get(&registry, "shares_diluted").unwrap() - 1_170_000_000.0).abs() < 1.0);
        assert!(p.source(&registry, "shares_diluted").contains("net income / eps_diluted"));
    }

    #[test]
    fn correct_share_count_and_a_stock_split_do_not_change() {
        let (r, p) = shares_company(716_400_000.0, 11.95, 8_563_000_000.0);
        assert_eq!(p.get(&r, "shares_diluted"), Some(716_400_000.0));
        assert_eq!(p.source(&r, "shares_diluted"), "us-gaap/WeightedAverageNumberOfDilutedSharesOutstanding");
        let (r, p) = shares_company(179_100_000.0, 11.95, 8_563_000_000.0);   // a factor of 4
        assert_eq!(p.get(&r, "shares_diluted"), Some(179_100_000.0));
    }

    #[test]
    fn alias_sequence_decides_when_a_filer_has_two_tags() {
        // Dividends: total cash paid > common only > declared.
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("PaymentsOfDividendsCommonStock", flow(6.0)), ("PaymentsOfDividends", flow(7.0)), ("DividendsCommonStockCash", flow(8.0))]);
        assert_eq!(p.get(&r, "dividends_paid"), Some(-7.0));
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("DividendsCommonStockCash", flow(8.0))]);
        assert_eq!(p.get(&r, "dividends_paid"), Some(-8.0));
        // Interest income: the specific tag before the catch-all tag.
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("InterestIncomeOther", flow(1.0)), ("InvestmentIncomeInterest", flow(2.0))]);
        assert_eq!(p.get(&r, "interest_income"), Some(2.0));
        assert_eq!(p.source(&r, "interest_income"), "us-gaap/InvestmentIncomeInterest");
    }

    #[test]
    fn negate_changes_the_sign_and_keeps_a_receipt_positive() {
        // The tag is positive for a payment; the variable is a cash flow.
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("PaymentsForProceedsFromOtherInvestingActivities", flow(7.0))]);
        assert_eq!(p.get(&r, "other_investing"), Some(-7.0));
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("PaymentsForProceedsFromOtherInvestingActivities", flow(-3.0))]);
        assert_eq!(p.get(&r, "other_investing"), Some(3.0));
    }

    #[test]
    fn capex_total_is_equipment_and_capitalized_software() {
        // PLANNING/RULES.md R2. Purchased intangibles are not in capex_total.
        let (r, p) = resolve(vec![
            ("Revenues", flow(1000.0)),
            ("PaymentsToAcquirePropertyPlantAndEquipment", flow(50.0)),
            ("PaymentsToDevelopSoftware", flow(30.0)),
            ("PaymentsToAcquireIntangibleAssets", flow(10.0)),
        ]);
        assert_eq!(p.get(&r, "capex"), Some(-50.0));
        assert_eq!(p.get(&r, "capex_total"), Some(-80.0));
        assert_eq!(p.get(&r, "purchases_of_intangibles"), Some(-10.0));
        assert_eq!(p.source(&r, "capex_total"), "= capex + capitalized_software");
    }

    #[test]
    fn revenue_of_a_bank_a_landlord_and_an_investment_company() {
        // A bank: the contract-revenue tag is a small part of its income.
        let (r, p) = resolve(vec![
            ("InterestIncomeExpenseNet", flow(600.0)),
            ("NoninterestIncome", flow(140.0)),
            ("RevenueFromContractWithCustomerExcludingAssessedTax", flow(90.0)),
        ]);
        assert_eq!(p.get(&r, "revenue"), Some(740.0));
        assert_eq!(p.source(&r, "revenue"), "= net_interest_income + noninterest_income");
        // A total tag wins.
        let (r, p) = resolve(vec![("Revenues", flow(750.0)), ("InterestIncomeExpenseNet", flow(600.0)), ("NoninterestIncome", flow(140.0))]);
        assert_eq!(p.get(&r, "revenue"), Some(750.0));
        // Most companies: contract revenue. The sales tag (before 2018) comes after it.
        let (r, p) = resolve(vec![("RevenueFromContractWithCustomerExcludingAssessedTax", flow(90.0)), ("SalesRevenueNet", flow(95.0)), ("OperatingLeaseLeaseIncome", flow(4.0))]);
        assert_eq!(p.get(&r, "revenue"), Some(90.0));
        assert_eq!(p.source(&r, "revenue"), "= revenue_from_contracts");
        // A landlord and an investment company.
        let (r, p) = resolve(vec![("OperatingLeaseLeaseIncome", flow(3094.0))]);
        assert_eq!(p.source(&r, "revenue"), "= lease_income");
        let (r, p) = resolve(vec![("GrossInvestmentIncomeOperating", flow(3052.0))]);
        assert_eq!((p.get(&r, "revenue"), p.source(&r, "revenue")), (Some(3052.0), "= investment_income_operating"));
        // A mortgage REIT (DX, NLY, AGNC): the interest income on its loans and securities.
        let (r, p) = resolve(vec![("InterestIncomeOperating", flow(533.0))]);
        assert_eq!((p.get(&r, "revenue"), p.source(&r, "revenue")), (Some(533.0), "= investment_income_operating"));
        // A company with no revenue and only a small interest income on debt securities (BHVN): no revenue.
        let (r, p) = resolve(vec![("InterestIncomeDebtSecuritiesOperating", flow(5.0))]);
        assert_eq!(p.get(&r, "revenue"), None);
    }

    #[test]
    fn revenue_before_2018_uses_the_goods_and_services_sales_tags() {
        // A filer with only SalesRevenueGoodsNet in 2015 (KO, MRK, SYY): the value is the revenue.
        let (r, p) = resolve(vec![("SalesRevenueGoodsNet", flow(44294.0)), ("SalesAllowancesGoods", flow(6800.0))]);
        assert_eq!(p.get(&r, "revenue"), Some(44294.0));
        assert_eq!(p.source(&r, "revenue"), "= revenue_total_pre2018");
        assert_eq!(p.get(&r, "revenue_from_contracts"), None);
        assert_eq!(p.source(&r, "revenue_total_pre2018"), "us-gaap/SalesRevenueGoodsNet");
        // A seller of services (FIS, VRSK).
        let (r, p) = resolve(vec![("SalesRevenueServicesNet", flow(6596.0))]);
        assert_eq!(p.get(&r, "revenue"), Some(6596.0));
        // The total tags come first: a later tag never replaces them.
        let (r, p) = resolve(vec![("SalesRevenueNet", flow(100.0)), ("SalesRevenueGoodsNet", flow(60.0)), ("SalesRevenueServicesNet", flow(40.0))]);
        assert_eq!(p.get(&r, "revenue"), Some(100.0));
        let (r, p) = resolve(vec![("RevenueFromContractWithCustomerExcludingAssessedTax", flow(90.0)), ("SalesRevenueGoodsNet", flow(95.0))]);
        assert_eq!(p.get(&r, "revenue"), Some(90.0));
        // A refiner (VLO).
        let (r, p) = resolve(vec![("RefiningAndMarketingRevenue", flow(87804.0))]);
        assert_eq!(p.get(&r, "revenue"), Some(87804.0));
        // A part tag never stands in the contract-revenue variable: it holds a total only.
        let (r, p) = resolve(vec![("SalesRevenueGoodsNet", flow(29970.0))]);
        assert_eq!(p.get(&r, "revenue_from_contracts"), None);
        assert_eq!(p.get(&r, "revenue_total_pre2018"), Some(29970.0));
    }

    #[test]
    fn revenue_total_tags_of_a_utility_a_hospital_operator_and_a_lessor() {
        // Each tag alone gives the revenue; a mutant that drops one tag fails here.
        for (tag, value) in [("UtilityRevenue", 17_000.0), ("HealthCareOrganizationRevenue", 51_336.0), ("RealEstateRevenueNet", 2_817.0)] {
            let (r, p) = resolve(vec![(tag, flow(value))]);
            assert_eq!(p.get(&r, "revenue"), Some(value), "{tag}");
            assert_eq!(p.source(&r, "revenue"), "= revenue_total_pre2018", "{tag}");
            assert_eq!(p.source(&r, "revenue_total_pre2018"), format!("us-gaap/{tag}"), "{tag}");
        }
        // A total tag of the period wins over the pre-2018 total.
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("UtilityRevenue", flow(60.0))]);
        assert_eq!(p.get(&r, "revenue"), Some(100.0));
    }

    #[test]
    fn gross_profit_needs_one_basis() {
        // One fact of the year in one filing.
        let one = |val: f64, filed: &str, accn: &str| {
            let mut f = fact(FY.0, FY.1, val, "10-K", filed);
            f.accn = Some(accn.to_string());
            f
        };
        let (old, new) = ("2015-02-19", "2017-02-28");
        // HWM 2014: revenue restated in the filing of 2017, cost of revenue only in the filing of 2015
        // (the filing of 2015 gives the old revenue 13000, a different value): no value.
        let (r, p) = resolve(vec![
            ("Revenues", vec![one(13000.0, old, "A-15"), one(12542.0, new, "A-17")]),
            ("CostOfRevenue", vec![one(19137.0, old, "A-15")]),
        ]);
        assert_eq!(p.get(&r, "revenue"), Some(12542.0));
        assert_eq!(p.get(&r, "gross_profit"), None);
        assert!(p.source(&r, "gross_profit").contains("different filings"), "{}", p.source(&r, "gross_profit"));
        // Restated cost of revenue and old revenue: no value either.
        let (r, p) = resolve(vec![
            ("Revenues", vec![one(100.0, old, "A-15")]),
            ("CostOfRevenue", vec![one(60.0, old, "A-15"), one(58.0, new, "A-17")]),
        ]);
        assert_eq!(p.get(&r, "gross_profit"), None);
        // Two filings, but the other filing gives the same value: a value. (Revenue tag with the contract tag.)
        let (r, p) = resolve(vec![
            ("RevenueFromContractWithCustomerExcludingAssessedTax", vec![one(100.0, old, "A-15"), one(100.0, new, "A-17")]),
            ("CostOfRevenue", vec![one(60.0, old, "A-15")]),
        ]);
        assert_eq!(p.get(&r, "gross_profit"), Some(40.0));
        // Two filings and a tag that the other filing does not have (a tag change): a value.
        let (r, p) = resolve(vec![("Revenues", vec![one(100.0, new, "A-17")]), ("CostOfRevenue", vec![one(60.0, old, "A-15")])]);
        assert_eq!(p.get(&r, "gross_profit"), Some(40.0));
        // No accession number: no guard.
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("CostOfRevenue", vec![one(60.0, old, "A-15")])]);
        assert_eq!(p.get(&r, "gross_profit"), Some(40.0));
        // One basis and a negative result (a cruise line in 2020): the value stays.
        let (r, p) = resolve(vec![("Revenues", vec![one(12542.0, new, "A-17")]), ("CostOfRevenue", vec![one(19137.0, new, "A-17")])]);
        assert_eq!(p.get(&r, "gross_profit"), Some(-6595.0));
        // The operating income formula does not use a value that was removed for a mixed basis.
        let (r, p) = resolve(vec![
            ("Revenues", vec![one(13000.0, old, "A-15"), one(12542.0, new, "A-17")]),
            ("CostOfRevenue", vec![one(19137.0, old, "A-15")]),
            ("OperatingExpenses", vec![one(100.0, new, "A-17")]),
        ]);
        assert_eq!(p.get(&r, "operating_income"), None);
        // A reported gross-profit tag is never changed.
        let (r, p) = resolve(vec![
            ("Revenues", vec![one(13000.0, old, "A-15"), one(12542.0, new, "A-17")]),
            ("CostOfRevenue", vec![one(19137.0, old, "A-15")]),
            ("GrossProfit", flow(41.0)),
        ]);
        assert_eq!(p.get(&r, "gross_profit"), Some(41.0));
    }

    #[test]
    fn interest_income_is_revenue_only_for_an_unclassified_balance_sheet_and_a_large_share_of_assets() {
        // APTV, a biotech: a current-assets total, thus the interest income is not revenue.
        let (r, p) = resolve(vec![("InterestIncomeOperating", flow(5.0)), ("AssetsCurrent", balance(900.0)), ("Assets", balance(1000.0))]);
        assert_eq!(p.get(&r, "revenue"), None);
        // ARE, UMH: no current-assets total, but the interest income is 0.05% of total assets.
        let (r, p) = resolve(vec![("InterestIncomeOperating", flow(5.0)), ("Assets", balance(10000.0))]);
        assert_eq!(p.get(&r, "revenue"), None);
        // A mortgage REIT: 5% of total assets.
        let (r, p) = resolve(vec![("InterestAndDividendIncomeOperating", flow(50.0)), ("Assets", balance(1000.0))]);
        assert_eq!((p.get(&r, "revenue"), p.source(&r, "revenue")), (Some(50.0), "= investment_income_operating"));
        // The total of investment income of a fund has no scope.
        let (r, p) = resolve(vec![("GrossInvestmentIncomeOperating", flow(5.0)), ("AssetsCurrent", balance(900.0)), ("Assets", balance(100000.0))]);
        assert_eq!(p.get(&r, "revenue"), Some(5.0));
    }

    #[test]
    fn scope_of_the_interest_income_tags_edge_cases() {
        let revenue = |concepts: Vec<(&str, Vec<crate::facts::Fact>)>| {
            let (r, p) = resolve(concepts);
            p.get(&r, "revenue")
        };
        // A large share of total assets, but the filer has a current-assets total.
        assert_eq!(revenue(vec![("InterestIncomeOperating", flow(500.0)), ("AssetsCurrent", balance(900.0)), ("Assets", balance(1000.0))]), None);
        // Exactly 1% of total assets: a value.
        assert_eq!(revenue(vec![("InterestIncomeOperating", flow(10.0)), ("Assets", balance(1000.0))]), Some(10.0));
        // Just below 1%: no value.
        assert_eq!(revenue(vec![("InterestIncomeOperating", flow(9.99)), ("Assets", balance(1000.0))]), None);
        // Total assets of zero count as unknown: no share test.
        assert_eq!(revenue(vec![("InterestIncomeOperating", flow(1.0)), ("Assets", balance(0.0))]), Some(1.0));
        // Total assets unknown, no current-assets total (a fund or a mortgage REIT): a value.
        assert_eq!(revenue(vec![("InterestIncomeOperating", flow(1.0))]), Some(1.0));
    }

    #[test]
    fn scope_of_the_interest_income_tags_in_a_quarter() {
        let q = |val: f64| {
            let registry = Registry::embedded().unwrap();
            let facts = company(vec![
                ("InterestIncomeOperating", vec![fact(Some("2024-07-01"), "2024-09-30", val, "10-Q", "2024-11-01")]),
                ("Assets", vec![fact(None, "2024-09-30", 1000.0, "10-Q", "2024-11-01")]),
            ]);
            let p = resolve_period(&registry, &facts, "2024-09-30", Forms::Quarterly);
            p.get(&registry, "revenue")
        };
        // A quarter needs 0.25% of total assets (2.5), not 1% (10).
        assert_eq!(q(5.0), Some(5.0));
        assert_eq!(q(2.0), None);
    }

    #[test]
    fn a_period_with_no_balance_sheet_of_a_classified_filer_has_no_interest_revenue() {
        // The current-assets total is in the next year; the earlier year has income facts only (APTV FY2010).
        let registry = Registry::embedded().unwrap();
        let facts = company(vec![
            ("InterestIncomeOperating", vec![fact(Some("2023-01-01"), "2023-12-31", 29.0, "10-K", "2025-02-01")]),
            ("AssetsCurrent", balance(900.0)),
            ("Assets", balance(1000.0)),
        ]);
        let p = resolve_period(&registry, &facts, "2023-12-31", Forms::Annual);
        assert_eq!(p.get(&registry, "revenue"), None);
        // An unclassified filer with no balance sheet in the period (ARCC, DX): a value.
        let facts = company(vec![("InterestIncomeOperating", vec![fact(Some("2023-01-01"), "2023-12-31", 29.0, "10-K", "2025-02-01")])]);
        let p = resolve_period(&registry, &facts, "2023-12-31", Forms::Annual);
        assert_eq!(p.get(&registry, "revenue"), Some(29.0));
    }

    #[test]
    fn total_assets_from_liabilities_and_equity_when_the_filer_has_no_assets_tag() {
        let (r, p) = resolve(vec![("Revenues", flow(100.0)), ("LiabilitiesAndStockholdersEquity", balance(3260.0))]);
        assert_eq!(p.get(&r, "total_assets"), Some(3260.0));
        assert_eq!(p.source(&r, "total_assets"), "= total_liabilities_and_equity");
    }

    #[test]
    fn formula_value_larger_than_its_total_is_not_used() {
        // The short-term investments are inside the cash line of this filer: the sum
        // counts them two times and is larger than the current assets.
        let (r, p) = resolve(vec![
            ("Revenues", flow(100.0)),
            ("CashAndCashEquivalentsAtCarryingValue", balance(2336.0)),
            ("ShortTermInvestments", balance(2000.0)),
            ("AssetsCurrent", balance(2459.0)),
        ]);
        assert_eq!(p.get(&r, "cash_and_short_term_investments"), None);
        assert!(p.source(&r, "cash_and_short_term_investments").contains(NOT_USED));
        assert_eq!(p.get(&r, "cash_and_equivalents"), Some(2336.0));
        // The usual case: the sum is smaller than the total, and it is used.
        let (r, p) = resolve(vec![
            ("Revenues", flow(100.0)),
            ("CashAndCashEquivalentsAtCarryingValue", balance(300.0)),
            ("ShortTermInvestments", balance(200.0)),
            ("AssetsCurrent", balance(2459.0)),
        ]);
        assert_eq!(p.get(&r, "cash_and_short_term_investments"), Some(500.0));
    }

    #[test]
    fn filing_value_fills_a_critical_input_and_the_formulas_use_it() {
        let registry = Registry::embedded().unwrap();
        let facts = company(vec![("Revenues", flow(1000.0)), ("PaymentsToDevelopSoftware", flow(30.0))]);
        let fill = |end: &str, name: &str, mode: FillMode| {
            assert_eq!(end, FY.1);
            if name == "capex" && mode == FillMode::Gap { Some(Filled::Value(-50.0, "filing 0000000001-25-000001: acme_Capex".to_string())) } else { None }
        };
        let p = resolve_period_with(&registry, &facts, FY.1, Forms::Annual, Some(&fill));
        assert_eq!(p.get(&registry, "capex"), Some(-50.0));
        assert_eq!(p.get(&registry, "capex_total"), Some(-80.0));
        assert_eq!(p.from_filing(&registry), vec![("capex", "filing 0000000001-25-000001: acme_Capex")]);
        // A quarter, and a year with all critical inputs: the filing is not used.
        let never = |_: &str, _: &str, mode: FillMode| -> Option<Filled> {
            assert_eq!(mode, FillMode::Lines, "a year with all critical inputs needs no gap fill");
            None
        };
        let never_quarter = |_: &str, _: &str, _: FillMode| -> Option<Filled> { panic!("the filing must not be read") };
        let mut full = company(vec![("Revenues", flow(1000.0)), ("PaymentsToAcquirePropertyPlantAndEquipment", flow(50.0))]);
        let mut units = std::collections::HashMap::new();
        units.insert("shares".to_string(), flow(10.0));
        full.facts.get_mut("us-gaap").unwrap().insert("WeightedAverageNumberOfDilutedSharesOutstanding".to_string(), crate::facts::Concept { units });
        resolve_period_with(&registry, &full, FY.1, Forms::Annual, Some(&never));
        resolve_period_with(&registry, &facts, FY.1, Forms::Quarterly, Some(&never_quarter));
    }

    #[test]
    fn filing_lines_replace_capex_for_the_fiscal_years_of_the_filing_only() {
        let registry = Registry::embedded().unwrap();
        let facts = company(vec![("Revenues", flow(1000.0)), ("PaymentsToAcquirePropertyPlantAndEquipment", flow(50.0)), ("PaymentsToDevelopSoftware", flow(5.0))]);
        // The 10-K holds the year FY.1 only: capex is 70 (equipment) and 20 (software) there.
        let fill = |end: &str, name: &str, mode: FillMode| {
            if mode != FillMode::Lines || end != FY.1 {
                return None;
            }
            match name {
                "capex" => Some(Filled::Value(-70.0, "10-K lines 0000000001-25-000001: us-gaap_A + acme_B".to_string())),
                "capitalized_software" => Some(Filled::Value(-20.0, "10-K lines 0000000001-25-000001: acme_S".to_string())),
                _ => None,
            }
        };
        let p = resolve_period_with(&registry, &facts, FY.1, Forms::Annual, Some(&fill));
        assert_eq!(p.get(&registry, "capex"), Some(-70.0));
        assert_eq!(p.source(&registry, "capex"), "10-K lines 0000000001-25-000001: us-gaap_A + acme_B");
        assert_eq!(p.get(&registry, "capex_total"), Some(-90.0));
        assert!(p.from_filing(&registry).is_empty(), "a replaced value is not a gap fill");
        // The company-facts value stays in its own variable.
        assert_eq!(p.get(&registry, "capex_companyfacts"), Some(-50.0));
        assert_eq!(p.get(&registry, "capitalized_software_companyfacts"), Some(-5.0));
        assert_eq!(p.source(&registry, "capex_companyfacts"), "us-gaap/PaymentsToAcquirePropertyPlantAndEquipment");
        // An older year: the filing does not hold it, the company facts stay.
        let old = company(vec![("Revenues", flow_at("2022-01-01", "2022-12-31", 900.0)), ("PaymentsToAcquirePropertyPlantAndEquipment", flow_at("2022-01-01", "2022-12-31", 40.0))]);
        let q = resolve_period_with(&registry, &old, "2022-12-31", Forms::Annual, Some(&fill));
        assert_eq!(q.get(&registry, "capex"), Some(-40.0));
        assert_eq!(q.source(&registry, "capex"), "us-gaap/PaymentsToAcquirePropertyPlantAndEquipment");
        assert_eq!(q.get(&registry, "capex_companyfacts"), Some(-40.0));
    }

    #[test]
    fn tag_with_no_variable_stays_in_raw_extras() {
        let (_, p) = resolve(vec![("Revenues", flow(100.0)), ("SomeCompanyDetail", flow(3.0))]);
        assert_eq!(p.raw_extras.get("us-gaap/SomeCompanyDetail"), Some(&3.0));
        assert!(!p.raw_extras.contains_key("us-gaap/Revenues"));
    }

    #[test]
    fn filed_date_comes_from_the_revenue_fact() {
        let registry = Registry::embedded().unwrap();
        let facts = company(vec![
            ("Revenues", vec![fact(FY.0, FY.1, 100.0, "10-K", "2025-02-01")]),
            ("NetIncomeLoss", vec![fact(FY.0, FY.1, 9.0, "10-K", "2026-02-01")]),
        ]);
        let p = resolve_period(&registry, &facts, FY.1, Forms::Annual);
        assert_eq!(p.filed.as_deref(), Some("2025-02-01"));
        let facts = company(vec![("NetIncomeLoss", vec![fact(FY.0, FY.1, 9.0, "10-K", "2025-02-20")])]);
        let p = resolve_period(&registry, &facts, FY.1, Forms::Annual);
        assert_eq!(p.filed.as_deref(), Some("2025-02-20"));
    }

    /// Cameco FY2025 (40-F, accession 0001193125-26-116229): values from the SEC company facts, in CAD.
    fn ifrs_fixture() -> CompanyFacts {
        let f = |start: Option<&str>, val: f64| {
            let s = start.map(|s| format!(r#""start":"{s}","#)).unwrap_or_default();
            format!(r#"{{{s}"end":"2025-12-31","val":{val},"form":"40-F","filed":"2026-02-20","accn":"0001193125-26-116229"}}"#)
        };
        let flow = |unit: &str, val: f64| format!(r#"{{"units":{{"{unit}":[{}]}}}}"#, f(Some("2025-01-01"), val));
        let bal = |val: f64| format!(r#"{{"units":{{"CAD":[{}]}}}}"#, f(None, val));
        let json = format!(
            r#"{{"cik":1009001,"entityName":"CAMECO CORP","facts":{{"ifrs-full":{{"Revenue":{},"CostOfSales":{},"ProfitLossFromOperatingActivities":{},"ProfitLoss":{},"ProfitLossAttributableToOwnersOfParent":{},"CashFlowsFromUsedInOperatingActivities":{},"PurchaseOfPropertyPlantAndEquipmentClassifiedAsInvestingActivities":{},"DepreciationAndAmortisationExpense":{},"WeightedAverageShares":{},"DilutedEarningsLossPerShare":{},"Borrowings":{},"CashAndCashEquivalents":{},"EquityAttributableToOwnersOfParent":{},"Assets":{}}}}}}}"#,
            flow("CAD", 3481933000.0),
            flow("CAD", 2511660000.0),
            flow("CAD", 618142000.0),
            flow("CAD", 589542000.0),
            flow("CAD", 589577000.0),
            flow("CAD", 1408432000.0),
            flow("CAD", 333025000.0),
            flow("CAD", 293094000.0),
            flow("shares", 435361956.0),
            flow("CAD/shares", 1.35),
            bal(996348000.0),
            bal(1114860000.0),
            bal(6903363000.0),
            bal(10301282000.0),
        );
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn ifrs_full_filer_gives_the_variables_in_its_own_currency() {
        let registry = Registry::embedded().unwrap();
        let p = resolve_period(&registry, &ifrs_fixture(), "2025-12-31", Forms::Annual);
        for (name, value) in [
            ("revenue", 3481933000.0),
            ("cogs_total", 2511660000.0),
            ("operating_income", 618142000.0),
            ("net_income", 589577000.0),
            ("net_income_incl_nci", 589542000.0),
            ("operating_cash_flow", 1408432000.0),
            ("capex", -333025000.0),
            ("da_total", 293094000.0),
            ("shares_basic", 435361956.0),
            ("eps_diluted", 1.35),
            ("total_debt", 996348000.0),
            ("cash_and_equivalents", 1114860000.0),
            ("equity", 6903363000.0),
            ("total_assets", 10301282000.0),
        ] {
            assert_eq!(p.get(&registry, name), Some(value), "{name}");
        }
        assert_eq!(p.source(&registry, "revenue"), "ifrs-full/Revenue");
    }

    #[test]
    fn annual_forms_include_40f_and_20f() {
        for form in ["10-K", "10-K/A", "40-F", "40-F/A", "20-F", "20-F/A"] {
            assert!(Forms::Annual.has(Some(form)), "{form}");
        }
        assert!(!Forms::Annual.has(Some("10-Q")) && !Forms::Quarterly.has(Some("40-F")));
    }
}
