//! From company facts to the standard variables of one period.
//!
//! This is the same logic as financial_analysis/importer.py (`resolve_period`,
//! `_apply_formulas`, `_collect_raw_extras`). The parity test compares the two.

use crate::facts::{period_ends, select, CompanyFacts, Forms};
use crate::registry::{Period, Registry};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct PeriodData {
    pub end: String,
    pub forms: Forms,
    /// The filing date of the revenue fact, or the latest filing date of the facts in use.
    pub filed: Option<String>,
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

    pub fn source(&self, registry: &Registry, name: &str) -> &str {
        registry.try_var(name).and_then(|i| self.sources[i].as_deref()).unwrap_or("")
    }

    /// True if the value came from a tag (false: from a formula, or no value).
    pub fn from_tag(&self, i: usize) -> bool {
        matches!(&self.sources[i], Some(s) if !s.starts_with('='))
    }
}

struct FormulaRun<'a> {
    registry: &'a Registry,
    values: &'a mut Vec<Option<f64>>,
    sources: &'a mut Vec<Option<String>>,
    in_work: Vec<bool>,
}

impl FormulaRun<'_> {
    fn resolve(&mut self, i: usize) -> Option<f64> {
        if let Some(v) = self.values[i] {
            return Some(v);
        }
        let registry = self.registry;
        let variable = &registry.variables[i];
        if variable.formulas.is_empty() || self.in_work[i] {
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

/// The standard variables for one period.
pub fn resolve_period(registry: &Registry, facts: &CompanyFacts, end: &str, forms: Forms) -> PeriodData {
    let n = registry.variables.len();
    let mut values: Vec<Option<f64>> = vec![None; n];
    let mut sources: Vec<Option<String>> = vec![None; n];
    let mut filed_dates: Vec<Option<String>> = vec![None; n];

    for (i, variable) in registry.variables.iter().enumerate() {
        for tag in &variable.tags {
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
            break;
        }
    }

    let mut run = FormulaRun { registry, values: &mut values, sources: &mut sources, in_work: vec![false; n] };
    for i in 0..n {
        run.resolve(i);
    }

    let revenue_filed = registry.try_var("revenue").and_then(|i| filed_dates[i].clone());
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

    PeriodData { end: end.to_string(), forms, filed, values, sources, raw_extras }
}

#[derive(Debug, Clone)]
pub struct Company {
    pub name: String,
    pub annual: Vec<PeriodData>,
    pub quarterly: Vec<PeriodData>,
}

/// The latest `n_periods` fiscal years and `n_periods` quarters.
pub fn map_company(registry: &Registry, facts: &CompanyFacts, n_periods: usize) -> Company {
    let mut out = Company {
        name: facts.entity_name.clone().unwrap_or_default(),
        annual: Vec::new(),
        quarterly: Vec::new(),
    };
    for forms in [Forms::Annual, Forms::Quarterly] {
        for end in period_ends(facts, &registry.anchors, forms, Some(n_periods)) {
            let period = resolve_period(registry, facts, &end, forms);
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
}
