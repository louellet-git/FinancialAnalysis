//! Values from the 10-K for a critical input that the company facts do not have.
//!
//! The company facts have no fact with a segment and no company tag. Thus a company
//! that gives its capex only for each segment, or with its own tag, has no capex in the
//! company facts. The 10-K has the value. This module gets it.
//!
//! The scope is small on purpose. Only the variables in `FILLED` can get a value, and
//! only when a critical input of the fiscal year has no value (`TRIGGERS`). Each value
//! has a source text that starts with "filing", and the checks give a warning for it.

use crate::filing::{capex_lines_in, has_investing_lines, load_filing, member_value, outflow, variable_of_tag, Filing, Line, MemberFact, Statement, YearView};
use crate::registry::Registry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A critical input with no value starts the search in the filing.
pub const TRIGGERS: [&str; 3] = ["revenue", "capex", "shares_diluted"];
/// The variables that can get a value from the filing.
pub const FILLED: [&str; 4] = ["revenue", "capex", "shares_diluted", "shares_basic"];

/// The variables whose value the lines of the 10-K replace, for each fiscal year that the 10-K holds.
pub const FROM_LINES: [&str; 2] = ["capex", "capitalized_software"];

/// What the mapper asks of the filing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    /// A value for a critical input that the company facts do not have (`Supplement::fill`).
    Gap,
    /// The value from the capex lines of the 10-K, also if the company facts have a value (`Supplement::lines`).
    Lines,
}

/// The answer of the filing to the mapper.
#[derive(Debug, Clone, PartialEq)]
pub enum Filled {
    /// A value with its source text.
    Value(f64, String),
    /// The filing has no such line: the variable has no value in this year (the company-facts value is cleared,
    /// because both variables of one year come from one source).
    Absent(String),
    /// The lines of the filing are not reliable for this year: keep the company-facts value, and add the note to its source.
    Keep(String),
}

pub const SOURCE_PREFIX: &str = "filing ";
/// The source text of a value from the capex lines of the 10-K (not a gap fill, no warning).
pub const LINES_PREFIX: &str = "10-K lines ";
pub const NO_CAPEX_LINE: &str = "= 0 (filing ";

/// True if the source text of a value says that the value came from the 10-K.
pub fn from_filing(source: &str) -> bool {
    source.starts_with(SOURCE_PREFIX) || source.starts_with(NO_CAPEX_LINE)
}

pub struct Supplement {
    filing: Filing,
    variables: HashMap<String, String>,
}

/// The folder of the latest filing of a ticker in the cache (None: no filing).
pub fn latest_filing_folder(cache: &Path, ticker: &str) -> Option<PathBuf> {
    // An accession number is "<filer>-<year>-<sequence>": the latest has the largest
    // year and sequence.
    let key = |path: &PathBuf| -> (String, String) {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mut parts = name.split('-').skip(1);
        (parts.next().unwrap_or("").to_string(), parts.next().unwrap_or("").to_string())
    };
    std::fs::read_dir(cache.join(ticker).join("filings"))
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("MetaLinks.json").is_file())
        .max_by_key(key)
}

impl Supplement {
    pub fn new(registry: &Registry, filing: Filing) -> Supplement {
        Supplement { filing, variables: variable_of_tag(registry) }
    }

    /// The latest filing of the ticker in the cache (None: no filing, or it cannot be read).
    pub fn load(registry: &Registry, cache: &Path, ticker: &str) -> Option<Supplement> {
        let folder = latest_filing_folder(cache, ticker)?;
        load_filing(ticker, &folder).ok().map(|filing| Supplement::new(registry, filing))
    }

    /// `capex` (equipment lines) or `capitalized_software` for the fiscal year that ends on `end`, from the
    /// capex lines of the 10-K (calculation tree rule, `filing::capex_lines_in`). The source text lists the
    /// lines that the value sums. Both variables of one year come from one source: if the equipment lines are
    /// not reliable, both keep the company-facts value (`Filled::Keep`). None: the filing does not hold this year.
    pub fn lines(&self, end: &str, name: &str) -> Option<Filled> {
        if !FROM_LINES.contains(&name) {
            return None;
        }
        let year = self.filing.year(end)?;
        let cash = year.statements.iter().find(|s| s.kind == "cash")?;
        let found = capex_lines_in(Some(cash), &self.variables);
        let accession = &self.filing.accession;
        let tag_list = |lines: &[(&Line, f64)]| lines.iter().map(|(l, _): &(&Line, f64)| l.tag.as_str()).collect::<Vec<_>>().join(" + ");
        let line_list = |lines: &[&Line]| lines.iter().map(|l| l.tag.as_str()).collect::<Vec<_>>().join(", ");
        if let Some(line) = found.unresolved.first() {
            return Some(Filled::Keep(format!("10-K {accession} not used: the line {} has no consolidated value", line.tag)));
        }
        if found.equipment.is_empty() {
            let why = if found.excluded.is_empty() {
                "no capex line".to_string()
            } else {
                format!("only excluded lines: {}", line_list(&found.excluded))
            };
            return Some(Filled::Keep(format!("10-K {accession} not used: {why}")));
        }
        let lines = if name == "capex" { &found.equipment } else { &found.software };
        let edge = if self.filing.is_oldest_year(end) { " [oldest year of this 10-K: earlier years use the company facts, with a different line definition]" } else { "" };
        Some(match outflow(lines) {
            Some(value) => Filled::Value(value, format!("{LINES_PREFIX}{accession}: {}{edge}", tag_list(lines))),
            None => Filled::Absent(format!("{LINES_PREFIX}{accession}: no software line{edge}")),
        })
    }

    /// The value of a variable for the fiscal year that ends on `end`, with its source.
    pub fn fill(&self, registry: &Registry, end: &str, name: &str) -> Option<(f64, String)> {
        if !FILLED.contains(&name) {
            return None;
        }
        let year = self.filing.year(end)?;
        let (statements, members) = (year.statements, year.members);
        let variable = &registry.variables[registry.try_var(name)?];
        let accession = &self.filing.accession;
        let signed = |value: f64| {
            if variable.sign_flip {
                -value.abs()
            } else if variable.negate {
                -value
            } else {
                value
            }
        };
        // 1. A tag of the variable that has values only with a segment member. A
        // formula with one term is a second source of the same amount (revenue from
        // contract revenue): the tags of the term are used after the tags of the variable.
        let one_term = variable.formulas.iter().filter(|f| f.terms.len() == 1).flat_map(|f| registry.variables[f.terms[0].var].tags.iter());
        for tag in variable.tags.iter().chain(one_term) {
            let key = format!("{}_{}", tag.namespace, tag.name);
            let Some(facts) = members.get(&key) else { continue };
            let result = if variable.unit == "shares" {
                let eps = if name == "shares_diluted" { "eps_diluted" } else { "eps_basic" };
                share_count(facts, |axis, member| self.implied_count(registry, &year, eps, axis, member))
            } else {
                member_value(facts, false)
            };
            if let Some((value, how)) = result {
                return Some((signed(value), format!("{SOURCE_PREFIX}{accession}: {} ({how})", tag.full)));
            }
        }
        // 2. Capex: the lines of the investing section, company tags included.
        if name == "capex" {
            let cash: Option<&Statement> = statements.iter().find(|s| s.kind == "cash");
            // Equipment lines only: capitalized software is a different standard variable.
            let lines = capex_lines_in(cash, &self.variables).equipment;
            if let Some(value) = outflow(&lines) {
                let tags: Vec<&str> = lines.iter().map(|(l, _)| l.tag.as_str()).collect();
                return Some((value, format!("{SOURCE_PREFIX}{accession}: {}", tags.join(" + "))));
            }
            let found = capex_lines_in(cash, &self.variables);
            // A line was left out (intangibles, a business purchase) or has no value: no explicit zero.
            if has_investing_lines(cash) && found.excluded.is_empty() && found.unresolved.is_empty() {
                // The filer does not show capex as a line: the amount is small for this
                // filer, and it is in a different investing line.
                return Some((0.0, format!("{NO_CAPEX_LINE}{accession}: the investing activities have no capital expenditure line)")));
            }
        }
        None
    }

    /// Net income divided by the earnings per share of one share class.
    fn implied_count(&self, registry: &Registry, year: &YearView, eps_name: &str, axis: &str, member: &str) -> Option<f64> {
        let keys = |name: &str| -> Vec<String> {
            let tags = registry.try_var(name).map(|i| registry.variables[i].tags.as_slice()).unwrap_or(&[]);
            tags.iter().map(|t| format!("{}_{}", t.namespace, t.name)).collect()
        };
        let eps = keys(eps_name).iter().find_map(|key| {
            let of_class = year.members.get(key).and_then(|facts| facts.iter().find(|f| f.axis == axis && f.member == member)).map(|f| f.value);
            of_class.or_else(|| year.values.get(key).copied())
        })?;
        let income = ["net_income_to_common", "net_income"].iter().find_map(|name| keys(name).iter().find_map(|key| year.values.get(key).copied()))?;
        if eps.abs() < crate::mapper::MIN_EPS_FOR_SHARE_CHECK || income / eps <= 0.0 {
            return None;
        }
        Some(income / eps)
    }
}

pub const SHARE_SUM_TOLERANCE: f64 = 0.10;

/// The share count of a company that gives a count for each share class.
/// - The largest class, if its earnings per share give the net income of the company
///   (the count of the class already includes the other classes after conversion).
/// - The sum of the classes, if the sum agrees with net income / EPS and the largest
///   class does not (classes with the same rights).
/// `implied(axis, member)`: net income / EPS of the class (None: not available).
pub fn share_count(facts: &[MemberFact], implied: impl Fn(&str, &str) -> Option<f64>) -> Option<(f64, String)> {
    let largest = facts.iter().filter(|f| f.axis != "dei:LegalEntityAxis").max_by(|a, b| a.value.abs().total_cmp(&b.value.abs()))?;
    let mut classes: Vec<&MemberFact> = Vec::new();
    for fact in facts.iter().filter(|f| f.axis == largest.axis) {
        if !classes.iter().any(|c| c.member == fact.member) {
            classes.push(fact);
        }
    }
    let sum: f64 = classes.iter().map(|f| f.value).sum();
    if classes.len() > 1 {
        if let Some(implied) = implied(&largest.axis, &largest.member) {
            let off = |count: f64| (count / implied - 1.0).abs();
            if off(sum) < off(largest.value) && off(sum) <= SHARE_SUM_TOLERANCE {
                return Some((sum, format!("sum of {} members of {}; the sum agrees with net income / EPS", classes.len(), largest.axis)));
            }
        }
    }
    Some((largest.value, format!("the largest member {} of {}", largest.member, largest.axis)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filing::Line;

    fn line(tag: &str, weight: f64, value: f64, children: Vec<Line>) -> Line {
        Line { tag: tag.into(), label: tag.into(), weight, value: Some(value), children }
    }

    fn filing(investing: Vec<Line>, members: crate::filing::Members) -> Filing {
        let total = line("us-gaap_NetCashProvidedByUsedInInvestingActivities", 1.0, -100.0, investing);
        let cash = Statement { kind: "cash".into(), name: "Cash flows".into(), role: "r".into(), roots: vec![total] };
        Filing {
            ticker: "ACME".into(),
            accession: "0000000001-25-000001".into(),
            fiscal_year_end: Some("2024-12-31".into()),
            statements: vec![cash],
            values: HashMap::new(),
            members,
            earlier: Vec::new(),
            facts_total: 0,
            facts_with_segment: 0,
        }
    }

    #[test]
    fn capex_from_company_tag_lines_and_not_from_the_software_line() {
        let registry = Registry::embedded().unwrap();
        let lines = vec![
            line("acme_CapitalExpendituresAndInvestments", -1.0, 70.0, vec![]),
            line("acme_PaymentsForRentalEquipment", -1.0, 20.0, vec![]),
            line("us-gaap_PaymentsToDevelopSoftware", -1.0, 9.0, vec![]),
            line("us-gaap_PaymentsToAcquireBusinessesNetOfCashAcquired", -1.0, 40.0, vec![]),
        ];
        let supplement = Supplement::new(&registry, filing(lines, HashMap::new()));
        let (value, source) = supplement.fill(&registry, "2024-12-31", "capex").unwrap();
        assert_eq!(value, -90.0);
        assert_eq!(source, "filing 0000000001-25-000001: acme_CapitalExpendituresAndInvestments + acme_PaymentsForRentalEquipment");
        assert!(from_filing(&source));
        // A different fiscal year, and a variable that is not in the list: no value.
        assert!(supplement.fill(&registry, "2021-12-31", "capex").is_none());
        assert!(supplement.fill(&registry, "2024-12-31", "net_income").is_none());
    }

    #[test]
    fn capex_is_zero_when_the_investing_section_has_no_capex_line() {
        let registry = Registry::embedded().unwrap();
        let lines = vec![line("us-gaap_PaymentsToAcquireShortTermInvestments", -1.0, 100.0, vec![])];
        let supplement = Supplement::new(&registry, filing(lines, HashMap::new()));
        let (value, source) = supplement.fill(&registry, "2024-12-31", "capex").unwrap();
        assert_eq!(value, 0.0);
        assert!(source.starts_with(NO_CAPEX_LINE) && from_filing(&source));
        // No investing section: no decision.
        let mut empty = filing(Vec::new(), HashMap::new());
        empty.statements.clear();
        assert!(Supplement::new(&registry, empty).fill(&registry, "2024-12-31", "capex").is_none());
    }

    #[test]
    fn value_from_segment_members_of_a_standard_tag() {
        let registry = Registry::embedded().unwrap();
        let fact = |axis: &str, member: &str, value: f64| MemberFact { axis: axis.into(), member: member.into(), value };
        let mut members = HashMap::new();
        members.insert(
            "us-gaap_PaymentsToAcquirePropertyPlantAndEquipment".to_string(),
            vec![fact("us-gaap:PropertyPlantAndEquipmentByTypeAxis", "x:Ongoing", 3824.0), fact("dei:LegalEntityAxis", "x:Subsidiary", 3314.0)],
        );
        members.insert(
            "us-gaap_RevenueFromContractWithCustomerExcludingAssessedTax".to_string(),
            vec![fact("srt:ProductOrServiceAxis", "us-gaap:ProductMember", 15000.0), fact("srt:ProductOrServiceAxis", "us-gaap:ServiceMember", 6865.0)],
        );
        let supplement = Supplement::new(&registry, filing(Vec::new(), members));
        // Capex is a cash outflow: the value is negative. The subsidiary is not used.
        assert_eq!(supplement.fill(&registry, "2024-12-31", "capex").unwrap().0, -3824.0);
        // Revenue: the contract-revenue tag is a tag of a one-term formula of revenue.
        let (value, source) = supplement.fill(&registry, "2024-12-31", "revenue").unwrap();
        assert_eq!(value, 21865.0);
        assert!(source.contains("sum of 2 members of srt:ProductOrServiceAxis"));
    }

    fn value(f: Option<Filled>) -> (f64, String) {
        match f {
            Some(Filled::Value(v, s)) => (v, s),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn lines_value_lists_the_lines_and_only_the_fiscal_years_of_the_filing() {
        let registry = Registry::embedded().unwrap();
        let lines = vec![
            line("us-gaap_PaymentsToAcquirePropertyPlantAndEquipment", -1.0, 60.0, vec![]),
            line("acme_PaymentsForRentalEquipment", -1.0, 10.0, vec![]),
            line("us-gaap_PaymentsToDevelopSoftware", -1.0, 9.0, vec![]),
            line("us-gaap_PaymentsToAcquireIntangibleAssets", -1.0, 40.0, vec![]),
            line("us-gaap_PaymentsToAcquireBusinessesNetOfCashAcquired", -1.0, 40.0, vec![]),
        ];
        let supplement = Supplement::new(&registry, filing(lines, HashMap::new()));
        let (v, source) = value(supplement.lines("2024-12-31", "capex"));
        assert_eq!(v, -70.0);
        assert!(source.starts_with("10-K lines 0000000001-25-000001: us-gaap_PaymentsToAcquirePropertyPlantAndEquipment + acme_PaymentsForRentalEquipment"));
        assert!(source.contains("oldest year"), "the filing holds one year: it is the edge");
        assert!(!from_filing(&source), "a replaced value gives no filled_from_filing warning");
        let (software, text) = value(supplement.lines("2024-12-31", "capitalized_software"));
        assert_eq!(software, -9.0);
        assert!(text.contains("us-gaap_PaymentsToDevelopSoftware"));
        // An earlier year that the filing does not hold, and another variable: no answer (the company facts stay).
        assert!(supplement.lines("2021-12-31", "capex").is_none());
        assert!(supplement.lines("2024-12-31", "revenue").is_none());
    }

    #[test]
    fn a_line_that_is_left_out_or_has_no_value_keeps_the_company_facts_for_both_variables() {
        let registry = Registry::embedded().unwrap();
        // VSAT: the only line is PP&E and intangibles in one amount with no "And" before "Equipment" is counted as equipment.
        let vsat = vec![line("vsat_PaymentsToAcquirePropertyPlantEquipmentAndAcquireIntangibleAssets", -1.0, 993.0, vec![])];
        let (v, source) = value(Supplement::new(&registry, filing(vsat, HashMap::new())).lines("2024-12-31", "capex"));
        assert_eq!(v, -993.0);
        assert!(source.contains("vsat_PaymentsToAcquirePropertyPlantEquipmentAndAcquireIntangibleAssets"));
        // Only an intangibles-like line: no explicit zero, the company facts stay (and no gap zero).
        let only = vec![line("acme_PaymentsToAcquireEquipmentAndIntangibleAssetsOfBusinesses", -1.0, 50.0, vec![])];
        let supplement = Supplement::new(&registry, filing(only, HashMap::new()));
        for name in ["capex", "capitalized_software"] {
            assert!(matches!(supplement.lines("2024-12-31", name), Some(Filled::Keep(note)) if note.contains("only excluded lines")));
        }
        assert!(supplement.fill(&registry, "2024-12-31", "capex").is_none(), "no explicit 0 when a line was left out");
        // RYN: a capex line of the tree has no consolidated value: keep the company facts, also for the sibling lines.
        let mut missing = line("us-gaap_PaymentsToAcquireOtherPropertyPlantAndEquipment", -1.0, 0.0, vec![]);
        missing.value = None;
        let ryn = vec![missing, line("us-gaap_PaymentsToDevelopRealEstateAssets", -1.0, 22.0, vec![])];
        let supplement = Supplement::new(&registry, filing(ryn, HashMap::new()));
        assert!(matches!(supplement.lines("2024-12-31", "capex"), Some(Filled::Keep(note)) if note.contains("no consolidated value")));
        // SYM: one line for equipment and software goes to capex, and software has no line of its own.
        let sym = vec![line("sym_PaymentsToAcquirePropertyPlantAndEquipmentAndDevelopSoftware", -1.0, 79.0, vec![])];
        let supplement = Supplement::new(&registry, filing(sym, HashMap::new()));
        assert_eq!(value(supplement.lines("2024-12-31", "capex")).0, -79.0);
        assert!(matches!(supplement.lines("2024-12-31", "capitalized_software"), Some(Filled::Absent(_))));
    }

    fn class(name: &str, value: f64) -> MemberFact {
        MemberFact { axis: "us-gaap:StatementClassOfStockAxis".into(), member: name.into(), value }
    }

    #[test]
    fn share_count_is_the_largest_class_or_the_sum_of_classes_with_the_same_rights() {
        // Class A after conversion of the other classes: net income / EPS agrees with class A.
        let converted = [class("x:A", 1907.0), class("x:B", 73.0), class("x:C", 12.0)];
        assert_eq!(share_count(&converted, |_, _| Some(1910.0)).unwrap().0, 1907.0);
        // Two classes with the same rights: net income / EPS agrees with the sum.
        let equal = [class("x:A", 146.0), class("x:B", 375.0)];
        let (value, text) = share_count(&equal, |_, member| if member == "x:B" { Some(520.0) } else { None }).unwrap();
        assert_eq!(value, 521.0);
        assert!(text.contains("sum of 2 members"));
        // No earnings per share: the largest class.
        assert_eq!(share_count(&equal, |_, _| None).unwrap().0, 375.0);
        assert!(share_count(&[], |_, _| None).is_none());
    }
}
