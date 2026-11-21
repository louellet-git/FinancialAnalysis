//! The registry of standard variables (financial_analysis/standard_variables.toml).
//!
//! Only Rust reads this file. `Registry::load`
//! also checks the rules of the file, thus an edit that can cause a double count
//! does not load.

use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

/// The registry file, put into the program at build time.
pub const EMBEDDED: &str = include_str!("../../../financial_analysis/standard_variables.toml");

#[derive(Debug, Deserialize)]
struct RawFile {
    anchors: Vec<String>,
    variable: Vec<RawVariable>,
    #[serde(default)]
    rollup: Vec<RawRollup>,
    #[serde(default)]
    excluded: Vec<RawExcluded>,
    #[serde(default)]
    within: Vec<RawWithin>,
}

#[derive(Debug, Deserialize)]
struct RawWithin {
    part: String,
    total: String,
}

#[derive(Debug, Deserialize)]
struct RawVariable {
    name: String,
    statement: String,
    tags: Vec<String>,
    #[serde(default)]
    formulas: Vec<String>,
    unit: Option<String>,
    #[serde(default)]
    sign_flip: bool,
    #[serde(default)]
    negate: bool,
    #[serde(default)]
    core: bool,
    #[serde(default)]
    memo: bool,
    #[serde(default)]
    unclassified_only: Vec<String>,
    #[serde(default)]
    scoped_min_assets_share: f64,
}

#[derive(Debug, Deserialize)]
struct RawRollup {
    name: String,
    total: String,
    parts: Vec<String>,
    #[serde(default)]
    details: Vec<String>,
    #[serde(default)]
    combined: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    signed: bool,
    #[serde(default)]
    exact: bool,
}

#[derive(Debug, Deserialize)]
struct RawExcluded {
    pattern: String,
    reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Duration,
    Instant,
}

/// One XBRL tag: namespace and name ("us-gaap", "Revenues").
#[derive(Debug, Clone)]
pub struct Tag {
    pub full: String,
    pub namespace: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Term {
    pub sign: f64,
    pub var: usize,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct Formula {
    pub text: String,
    pub terms: Vec<Term>,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub statement: String,
    pub period: Period,
    pub tags: Vec<Tag>,
    pub formulas: Vec<Formula>,
    pub unit: String,
    /// Store the value as a negative number (a cash outflow).
    pub sign_flip: bool,
    /// Change the sign of the tag value.
    pub negate: bool,
    pub core: bool,
    pub memo: bool,
    /// Tags (a subset of `tags`) that give a value only if the filer has no current-assets
    /// total (an unclassified balance sheet: a fund or a mortgage REIT).
    pub unclassified_only: Vec<String>,
    /// The scoped tags give a value only if it is at least this share of total assets (0: no test).
    /// An equity REIT has a small interest income on a classified-looking balance sheet.
    pub scoped_min_assets_share: f64,
}

#[derive(Debug, Clone)]
pub struct Rollup {
    pub name: String,
    pub total: usize,
    /// Lines that are usually separate lines on the statement.
    pub parts: Vec<usize>,
    /// Lines that are frequently a detail inside one of the parts.
    pub details: Vec<usize>,
    /// (one-line variable, the parts it replaces)
    pub combined: Vec<(usize, Vec<usize>)>,
    pub signed: bool,
    pub exact: bool,
}

#[derive(Debug)]
pub struct Registry {
    pub variables: Vec<Variable>,
    pub index: HashMap<String, usize>,
    pub anchors: Vec<Tag>,
    pub rollups: Vec<Rollup>,
    /// (part, total): the part must not be larger than the total.
    pub within: Vec<(usize, usize)>,
    pub excluded: Vec<(Regex, String)>,
    /// Full names of all tags that have a variable.
    pub canonical: HashSet<String>,
}

fn tag(full: &str) -> Result<Tag, String> {
    let (namespace, name) = full
        .split_once('/')
        .ok_or_else(|| format!("tag '{full}' has no namespace"))?;
    Ok(Tag { full: full.to_string(), namespace: namespace.to_string(), name: name.to_string() })
}

/// "a - b + c?" -> [(+1, a, required), (-1, b, required), (+1, c, optional)]
fn parse_formula(text: &str, index: &HashMap<String, usize>, owner: &str) -> Result<Formula, String> {
    let mut terms = Vec::new();
    let mut sign = 1.0;
    for token in text.split_whitespace() {
        match token {
            "+" => sign = 1.0,
            "-" => sign = -1.0,
            _ => {
                let optional = token.ends_with('?');
                let name = token.trim_end_matches('?');
                let var = *index
                    .get(name)
                    .ok_or_else(|| format!("{owner}: formula term '{name}' is not a standard variable"))?;
                terms.push(Term { sign, var, optional });
                sign = 1.0;
            }
        }
    }
    if terms.is_empty() {
        return Err(format!("{owner}: formula '{text}' has no terms"));
    }
    Ok(Formula { text: text.to_string(), terms })
}

impl Registry {
    pub fn embedded() -> Result<Registry, String> {
        Registry::load(EMBEDDED)
    }

    pub fn load(text: &str) -> Result<Registry, String> {
        let raw: RawFile = toml::from_str(text).map_err(|e| format!("registry file: {e}"))?;

        let mut index = HashMap::new();
        for (i, v) in raw.variable.iter().enumerate() {
            if index.insert(v.name.clone(), i).is_some() {
                return Err(format!("variable '{}' is in the registry two times", v.name));
            }
        }

        let mut canonical = HashSet::new();
        let mut owner_of_tag: HashMap<String, String> = HashMap::new();
        let mut variables = Vec::with_capacity(raw.variable.len());
        for v in &raw.variable {
            let period = match v.statement.as_str() {
                "income_statement" | "cash_flow" => Period::Duration,
                "balance_sheet" | "disclosure" => Period::Instant,
                other => return Err(format!("{}: statement '{other}' is not known", v.name)),
            };
            // A variable with no tag gets its value from a formula only.
            if v.tags.is_empty() && v.formulas.is_empty() {
                return Err(format!("{}: no tags and no formula", v.name));
            }
            let mut tags = Vec::new();
            for full in &v.tags {
                // Rule: one tag gives a value to one variable only (no double count).
                if let Some(other) = owner_of_tag.insert(full.clone(), v.name.clone()) {
                    return Err(format!("tag '{full}' is in '{other}' and in '{}'", v.name));
                }
                canonical.insert(full.clone());
                tags.push(tag(full)?);
            }
            if let Some(bad) = v.unclassified_only.iter().find(|t| !v.tags.contains(t)) {
                return Err(format!("{}: unclassified_only tag '{bad}' is not in the tags", v.name));
            }
            if !(0.0..=1.0).contains(&v.scoped_min_assets_share) {
                return Err(format!("{}: scoped_min_assets_share must be from 0 to 1", v.name));
            }
            let mut formulas = Vec::new();
            for text in &v.formulas {
                let formula = parse_formula(text, &index, &v.name)?;
                if formula.terms.iter().any(|t| raw.variable[t.var].name == v.name) {
                    return Err(format!("{}: formula '{text}' uses the variable itself", v.name));
                }
                // Rule: a formula must not mix memo ("of which") and line variables.
                if formula.terms.len() > 1 {
                    let memo_terms = formula.terms.iter().filter(|t| raw.variable[t.var].memo).count();
                    if memo_terms != 0 && memo_terms != formula.terms.len() {
                        return Err(format!("{}: formula '{text}' mixes memo and line variables", v.name));
                    }
                }
                formulas.push(formula);
            }
            variables.push(Variable {
                name: v.name.clone(),
                statement: v.statement.clone(),
                period,
                tags,
                formulas,
                unit: v.unit.clone().unwrap_or_else(|| "USD".to_string()),
                sign_flip: v.sign_flip,
                negate: v.negate,
                core: v.core,
                memo: v.memo,
                unclassified_only: v.unclassified_only.clone(),
                scoped_min_assets_share: v.scoped_min_assets_share,
            });
        }

        let lookup = |name: &str, owner: &str| -> Result<usize, String> {
            index
                .get(name)
                .copied()
                .ok_or_else(|| format!("rollup {owner}: '{name}' is not a standard variable"))
        };
        let mut rollups = Vec::new();
        for r in &raw.rollup {
            let total = lookup(&r.total, &r.name)?;
            let mut parts = Vec::new();
            for p in &r.parts {
                let i = lookup(p, &r.name)?;
                if parts.contains(&i) {
                    return Err(format!("rollup {}: part '{p}' is there two times", r.name));
                }
                parts.push(i);
            }
            let mut details = Vec::new();
            for d in &r.details {
                let i = lookup(d, &r.name)?;
                if parts.contains(&i) || details.contains(&i) {
                    return Err(format!("rollup {}: '{d}' is there two times", r.name));
                }
                details.push(i);
            }
            let mut combined = Vec::new();
            for (one_line, replaces) in &r.combined {
                let one = lookup(one_line, &r.name)?;
                let mut replaced = Vec::new();
                for p in replaces {
                    let i = lookup(p, &r.name)?;
                    if !parts.contains(&i) {
                        return Err(format!("rollup {}: '{p}' (replaced by '{one_line}') is not a part", r.name));
                    }
                    replaced.push(i);
                }
                combined.push((one, replaced));
            }
            // Rule: a rollup has no memo variable.
            for &i in parts.iter().chain(details.iter()).chain(std::iter::once(&total)).chain(combined.iter().map(|(o, _)| o)) {
                if variables[i].memo {
                    return Err(format!("rollup {}: '{}' is a memo variable", r.name, variables[i].name));
                }
            }
            rollups.push(Rollup { name: r.name.clone(), total, parts, details, combined, signed: r.signed, exact: r.exact });
        }

        let mut excluded = Vec::new();
        for e in &raw.excluded {
            let regex = Regex::new(&e.pattern).map_err(|err| format!("excluded pattern '{}': {err}", e.pattern))?;
            excluded.push((regex, e.reason.clone()));
        }

        let mut within = Vec::new();
        for w in &raw.within {
            let part = *index.get(&w.part).ok_or_else(|| format!("within: '{}' is not a standard variable", w.part))?;
            let total = *index.get(&w.total).ok_or_else(|| format!("within: '{}' is not a standard variable", w.total))?;
            within.push((part, total));
        }

        let anchors = raw.anchors.iter().map(|a| tag(a)).collect::<Result<Vec<_>, _>>()?;
        Ok(Registry { variables, index, anchors, rollups, within, excluded, canonical })
    }

    pub fn var(&self, name: &str) -> usize {
        *self.index.get(name).unwrap_or_else(|| panic!("'{name}' is not a standard variable"))
    }

    pub fn try_var(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }

    /// The reason why a tag has no variable on purpose (None = no exclusion).
    pub fn exclusion_reason(&self, full_tag: &str) -> Option<&str> {
        let name = full_tag.split_once('/').map(|(_, n)| n).unwrap_or(full_tag);
        self.excluded.iter().find(|(re, _)| re.is_match(name)).map(|(_, reason)| reason.as_str())
    }

    pub fn core(&self) -> Vec<usize> {
        (0..self.variables.len()).filter(|&i| self.variables[i].core).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI: &str = r#"
anchors = ["us-gaap/Revenues"]
[[variable]]
name = "a"
statement = "income_statement"
tags = ["us-gaap/A"]
[[variable]]
name = "b"
statement = "income_statement"
tags = ["us-gaap/B"]
formulas = ["a - c?"]
[[variable]]
name = "c"
statement = "income_statement"
tags = ["us-gaap/C"]
"#;

    #[test]
    fn embedded_registry_loads_and_obeys_the_rules() {
        let registry = Registry::embedded().expect("the registry file must load");
        assert!(registry.variables.len() >= 110);
        assert!(registry.core().len() >= 14);
        assert!(!registry.rollups.is_empty());
        assert!(registry.try_var("total_debt").is_some());
    }

    /// The facts that `tests/test_xbrl_tags.py` checked for the Python loader of the registry. The loader
    /// (`load`) checks the other rules of that file: a tag or a formula for each variable, a namespace for each
    /// tag, one tag for one variable, the statement names, the rollup and within names, and the memo rule.
    #[test]
    fn embedded_registry_has_the_known_facts() {
        use crate::registry::Period::{Duration, Instant};
        let r = Registry::embedded().unwrap();
        let v = |name: &str| &r.variables[r.var(name)];
        assert!(v("capex").sign_flip && v("repurchases_common_stock").sign_flip && !v("revenue").sign_flip);
        assert_eq!((v("eps_diluted").unit.as_str(), v("shares_diluted").unit.as_str()), ("USD/shares", "shares"));
        for variable in &r.variables {
            let expected = match variable.statement.as_str() {
                "income_statement" | "cash_flow" => Duration,
                _ => Instant,
            };
            assert_eq!(variable.period, expected, "{}", variable.name);
        }
        let core: Vec<&str> = r.core().iter().map(|&i| r.variables[i].name.as_str()).collect();
        for name in ["revenue", "operating_cash_flow", "capex", "total_debt", "cash_and_equivalents", "shares_diluted"] {
            assert!(core.contains(&name), "{name} is a core input");
        }
        let tags = |name: &str| v(name).tags.iter().map(|t| t.full.as_str()).collect::<Vec<_>>();
        assert!(!tags("long_term_debt").contains(&"us-gaap/LongTermDebt"));
        assert!(tags("long_term_debt_total").contains(&"us-gaap/LongTermDebt"));
        assert_eq!(v("remaining_performance_obligation").statement, "disclosure");
        assert_eq!(tags("remaining_performance_obligation"), ["us-gaap/RevenueRemainingPerformanceObligation"]);
        // The last tag of short_term_investments (NVDA: 10-Q rows only) and the second business-purchase line.
        assert_eq!(tags("short_term_investments").last(), Some(&"us-gaap/DebtSecuritiesCurrent"));
        assert_eq!(tags("acquisitions_net_of_cash_second"), ["us-gaap/PaymentsToAcquireBusinessTwoNetOfCashAcquired"]);
        let investing = r.rollups.iter().find(|x| x.name == "investing_cash_flow").expect("investing rollup");
        assert!(investing.parts.contains(&r.var("acquisitions_net_of_cash_second")));
        // Tags with the scope of a fund or a mortgage REIT.
        assert_eq!(v("investment_income_operating").unclassified_only.len(), 2);
        assert!(!v("investment_income_operating").unclassified_only.iter().any(|t| t.ends_with("GrossInvestmentIncomeOperating")));
        assert!(!r.within.is_empty());
        assert!(r.excluded.iter().all(|(_, reason)| !reason.is_empty()));
        for rollup in &r.rollups {
            let lines: Vec<usize> = rollup.parts.iter().chain(rollup.details.iter()).copied().collect();
            let unique: std::collections::HashSet<usize> = lines.iter().copied().collect();
            assert_eq!(lines.len(), unique.len(), "rollup {}", rollup.name);
        }
    }

    #[test]
    fn formula_is_parsed() {
        let registry = Registry::load(MINI).unwrap();
        let formula = &registry.variables[registry.var("b")].formulas[0];
        assert_eq!(formula.terms.len(), 2);
        assert_eq!(formula.terms[1].sign, -1.0);
        assert!(formula.terms[1].optional);
    }

    #[test]
    fn one_tag_in_two_variables_does_not_load() {
        let text = MINI.replace("us-gaap/C", "us-gaap/A");
        let error = Registry::load(&text).unwrap_err();
        assert!(error.contains("us-gaap/A"), "{error}");
    }

    #[test]
    fn scoped_tag_that_is_not_in_the_tags_does_not_load() {
        let text = MINI.replace("tags = [\"us-gaap/C\"]", "tags = [\"us-gaap/C\"]\nunclassified_only = [\"us-gaap/Z\"]");
        assert_ne!(text, MINI);
        let error = Registry::load(&text).unwrap_err();
        assert!(error.contains("us-gaap/Z") && error.contains("unclassified_only"), "{error}");
    }

    #[test]
    fn scoped_share_outside_zero_to_one_does_not_load() {
        let text = MINI.replace("tags = [\"us-gaap/C\"]", "tags = [\"us-gaap/C\"]\nscoped_min_assets_share = 1.5");
        let error = Registry::load(&text).unwrap_err();
        assert!(error.contains("scoped_min_assets_share"), "{error}");
    }

    #[test]
    fn formula_that_mixes_memo_and_line_variables_does_not_load() {
        let text = MINI.replace("name = \"c\"\nstatement = \"income_statement\"", "name = \"c\"\nmemo = true\nstatement = \"income_statement\"");
        let error = Registry::load(&text).unwrap_err();
        assert!(error.contains("mixes memo and line"), "{error}");
    }

    #[test]
    fn variable_with_no_tag_needs_a_formula() {
        let text = MINI.replace("tags = [\"us-gaap/B\"]", "tags = []");
        assert!(Registry::load(&text).is_ok());
        let text = MINI.replace("tags = [\"us-gaap/C\"]", "tags = []");
        assert!(Registry::load(&text).unwrap_err().contains("no tags and no formula"));
    }

    #[test]
    fn unknown_formula_term_does_not_load() {
        let text = MINI.replace("a - c?", "a - zzz");
        assert!(Registry::load(&text).unwrap_err().contains("zzz"));
    }

    #[test]
    fn exclusion_reason_is_found_by_tag_name() {
        let registry = Registry::embedded().unwrap();
        assert!(registry.exclusion_reason("us-gaap/ComprehensiveIncomeNetOfTax").is_some());
        assert!(registry.exclusion_reason("us-gaap/AssetRetirementObligation").is_none());
    }
}
