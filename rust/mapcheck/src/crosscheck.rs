//! Compare the standard variables with the statements of the filing.
//!
//! The standard variables come from tag names (the registry). The filing values come
//! from the structure of the company's own statements (rust/mapcheck/src/filing.rs).
//! These are two different ways to the same numbers, and the filing is the source of
//! truth. Where they agree, an error in the mapping is improbable. Where they differ,
//! read the filing lines (`mapcheck filing TICKER --focus`).
//!
//! This check uses no outside data source.

use crate::filing::FilingEstimates;
use crate::mapper::PeriodData;
use crate::registry::Registry;
use std::collections::BTreeMap;
use std::fmt::Write;

pub const TOLERANCE: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Agree,
    /// The values agree with a different definition (see the note).
    Definition,
    Differ,
    /// The standard variable has no value and the filing has lines.
    NoValueOurs,
    /// The filing has no line for this input.
    NoValueFiling,
}

pub fn outcome_label(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Agree => "agree",
        Outcome::Definition => "definition",
        Outcome::Differ => "differ",
        Outcome::NoValueOurs => "no value (standard variable)",
        Outcome::NoValueFiling => "no line (filing)",
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    pub variable: &'static str,
    pub outcome: Outcome,
    pub ours: Option<f64>,
    pub filing: Option<f64>,
    pub note: &'static str,
}

fn near(ours: f64, theirs: f64) -> bool {
    (ours.abs() - theirs.abs()).abs() <= TOLERANCE * theirs.abs().max(1.0)
}

fn row(variable: &'static str, ours: Option<f64>, filing: Option<f64>, alternatives: &[(&'static str, Option<f64>, Option<f64>)]) -> Row {
    let (outcome, note) = match (ours, filing) {
        (_, None) => (Outcome::NoValueFiling, ""),
        (None, Some(_)) => (Outcome::NoValueOurs, ""),
        (Some(o), Some(f)) if near(o, f) => (Outcome::Agree, ""),
        (Some(_), Some(_)) => alternatives
            .iter()
            .find(|(_, o, f)| matches!((o, f), (Some(o), Some(f)) if near(*o, *f)))
            .map(|(name, _, _)| (Outcome::Definition, *name))
            .unwrap_or((Outcome::Differ, "")),
    };
    Row { variable, outcome, ours, filing, note }
}

/// Capital expenditure, depreciation and amortization, and total debt: the standard
/// variable against the value from the filing lines.
pub fn compare(registry: &Registry, period: &PeriodData, filing: &FilingEstimates) -> Vec<Row> {
    let get = |name: &str| period.get(registry, name);
    let capex_total = get("capex_total").or_else(|| get("capex"));
    vec![
        // capex_total is equipment and capitalized software (PLANNING/RULES.md R2).
        row(
            "capex",
            capex_total,
            filing.capex,
            &[("the filing lines with purchased intangibles", capex_total, filing.capex_with_intangibles)],
        ),
        row("da_total", get("da_total"), filing.da_total, &[]),
        row("total_debt", get("total_debt"), filing.total_debt, &[]),
    ]
}

#[derive(Default)]
pub struct Summary {
    pub companies: usize,
    pub counts: Vec<(&'static str, BTreeMap<Outcome, usize>)>,
    pub examples: Vec<(&'static str, String, Row)>,
}

impl Summary {
    pub fn add(&mut self, ticker: &str, rows: &[Row]) {
        self.companies += 1;
        for r in rows {
            let index = match self.counts.iter().position(|(v, _)| *v == r.variable) {
                Some(i) => i,
                None => {
                    self.counts.push((r.variable, BTreeMap::new()));
                    self.counts.len() - 1
                }
            };
            *self.counts[index].1.entry(r.outcome).or_default() += 1;
            if matches!(r.outcome, Outcome::Differ | Outcome::NoValueOurs | Outcome::Definition) {
                self.examples.push((r.variable, ticker.to_string(), r.clone()));
            }
        }
    }

    /// Share of the companies where the two sides have a value and the values agree.
    pub fn agreement(&self, variable: &str) -> Option<f64> {
        let counts = &self.counts.iter().find(|(v, _)| *v == variable)?.1;
        let get = |o: Outcome| counts.get(&o).copied().unwrap_or(0);
        let both = get(Outcome::Agree) + get(Outcome::Definition) + get(Outcome::Differ);
        (both > 0).then(|| get(Outcome::Agree) as f64 / both as f64)
    }

    pub fn text(&self, skipped: &[(String, String)], examples: usize) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "Standard variables against the statements of the latest 10-K: {} companies, {} skipped\n", self.companies, skipped.len());
        let _ = writeln!(out, "{:14}{:>7}{:>12}{:>8}{:>26}{:>18}{:>11}", "variable", "agree", "definition", "differ", "no value (std variable)", "no line (filing)", "agreement");
        for (variable, counts) in &self.counts {
            let get = |o: Outcome| counts.get(&o).copied().unwrap_or(0);
            let share = self.agreement(variable).map(|s| format!("{:.0}%", s * 100.0)).unwrap_or_else(|| "-".to_string());
            let _ = writeln!(
                out,
                "{:14}{:>7}{:>12}{:>8}{:>26}{:>18}{:>11}",
                variable,
                get(Outcome::Agree),
                get(Outcome::Definition),
                get(Outcome::Differ),
                get(Outcome::NoValueOurs),
                get(Outcome::NoValueFiling),
                share
            );
        }
        let b = |v: Option<f64>| v.map(|v| format!("{:.2}", v / 1e9)).unwrap_or_else(|| "-".to_string());
        for (variable, _) in &self.counts {
            let rows: Vec<&(&str, String, Row)> = self.examples.iter().filter(|e| e.0 == *variable).collect();
            let differ: Vec<&&(&str, String, Row)> =
                rows.iter().filter(|e| matches!(e.2.outcome, Outcome::Differ | Outcome::NoValueOurs)).take(examples).collect();
            if !differ.is_empty() {
                let _ = writeln!(out, "\n{variable}: differences (ticker: standard variable / filing lines, $B)");
                for e in differ {
                    let _ = writeln!(out, "  {:6} {:>10} / {:>10}", e.1, b(e.2.ours), b(e.2.filing));
                }
            }
            let definition: Vec<&str> = rows.iter().filter(|e| e.2.outcome == Outcome::Definition).map(|e| e.1.as_str()).collect();
            if !definition.is_empty() {
                let note = rows.iter().find(|e| e.2.outcome == Outcome::Definition).map(|e| e.2.note).unwrap_or("");
                let _ = writeln!(out, "\n{variable}: {} agree with '{note}': {}", definition.len(), definition.iter().take(12).copied().collect::<Vec<_>>().join(" "));
            }
        }
        if !skipped.is_empty() {
            let _ = writeln!(out, "\nSkipped ({}):", skipped.len());
            for (ticker, reason) in skipped.iter().take(20) {
                let _ = writeln!(out, "  {ticker:6} {reason}");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::Forms;

    fn period(registry: &Registry, values: &[(&str, f64)]) -> PeriodData {
        let n = registry.variables.len();
        let mut p = PeriodData {
            end: "2024-12-31".into(),
            forms: Forms::Annual,
            filed: None,
            values: vec![None; n],
            sources: vec![None; n],
            raw_extras: Default::default(),
        };
        for (name, value) in values {
            p.values[registry.var(name)] = Some(*value);
        }
        p
    }

    fn filing(capex: Option<f64>, with_intangibles: Option<f64>, da: Option<f64>, debt: Option<f64>) -> FilingEstimates {
        FilingEstimates { capex, capex_with_intangibles: with_intangibles, da_total: da, total_debt: debt }
    }

    fn outcome(rows: &[Row], variable: &str) -> Outcome {
        rows.iter().find(|r| r.variable == variable).unwrap().outcome
    }

    #[test]
    fn values_that_agree_and_values_that_differ() {
        let r = Registry::embedded().unwrap();
        let p = period(&r, &[("capex_total", -370.0), ("da_total", 500.0), ("total_debt", 1000.0)]);
        let rows = compare(&r, &p, &filing(Some(-372.0), Some(-372.0), Some(800.0), Some(1010.0)));
        assert_eq!(outcome(&rows, "capex"), Outcome::Agree);
        assert_eq!(outcome(&rows, "da_total"), Outcome::Differ);
        assert_eq!(outcome(&rows, "total_debt"), Outcome::Agree); // 1%, tolerance 2%
    }

    #[test]
    fn capex_falls_back_to_equipment_when_capex_total_has_no_value() {
        let r = Registry::embedded().unwrap();
        let rows = compare(&r, &period(&r, &[("capex", -50.0)]), &filing(Some(-50.0), Some(-50.0), None, None));
        assert_eq!(outcome(&rows, "capex"), Outcome::Agree);
    }

    #[test]
    fn capex_that_agrees_only_with_intangibles_is_a_definition_difference() {
        let r = Registry::embedded().unwrap();
        let rows = compare(&r, &period(&r, &[("capex_total", -400.0)]), &filing(Some(-370.0), Some(-400.0), None, None));
        assert_eq!(outcome(&rows, "capex"), Outcome::Definition);
    }

    #[test]
    fn missing_values_are_reported_by_side() {
        let r = Registry::embedded().unwrap();
        let rows = compare(&r, &period(&r, &[("da_total", 5.0)]), &filing(Some(-12.5), Some(-12.5), None, None));
        assert_eq!(outcome(&rows, "capex"), Outcome::NoValueOurs);      // a company tag in the filing
        assert_eq!(outcome(&rows, "da_total"), Outcome::NoValueFiling);
    }

    #[test]
    fn summary_counts_and_agreement_share() {
        let r = Registry::embedded().unwrap();
        let mut summary = Summary::default();
        summary.add("A", &compare(&r, &period(&r, &[("total_debt", 100.0)]), &filing(None, None, None, Some(100.0))));
        summary.add("B", &compare(&r, &period(&r, &[("total_debt", 100.0)]), &filing(None, None, None, Some(150.0))));
        summary.add("C", &compare(&r, &period(&r, &[]), &filing(None, None, None, Some(150.0))));
        assert_eq!(summary.companies, 3);
        assert_eq!(summary.agreement("total_debt"), Some(0.5));
        assert!(summary.text(&[], 6).contains("total_debt"));
    }
}
