//! Filing-level XBRL: the statements of one 10-K as the filer structures them.
//!
//! The companyfacts source gives values for standard tags only, and does not say
//! which line is a part of which line. Each filing has this information:
//! - `MetaLinks.json`: the list of statements, each tag with its label, and the
//!   calculation tree (the parent of each line, with a weight of +1 or -1);
//! - the instance document (`*_htm.xml`): the value of each tag, company-specific
//!   tags included, for each context (period, and segment if the fact has one).
//!
//! This module reads the consolidated values of each fiscal year in the filing (the
//! cash-flow statement has three years). A fact with one segment member is kept for
//! one use: a tag that has no consolidated value (`member_value`). Facts with two or
//! more segment members are counted but not used (PLANNING/tickets.md, Ticket 7).

use crate::dates;
use crate::registry::Registry;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

pub const SUM_TOLERANCE: f64 = 0.005;
pub const NEAR_DAYS: i64 = 14; // two period ends this near are one period
const XBRLI: &str = "http://www.xbrl.org/2003/instance";
const XBRLDI: &str = "http://xbrl.org/2006/xbrldi";
const STANDARD_PREFIXES: [&str; 11] =
    ["us-gaap", "srt", "dei", "ecd", "cyd", "country", "currency", "exch", "naics", "sic", "stpr"];

pub const INVESTING_TOTALS: [&str; 2] = [
    "us-gaap_NetCashProvidedByUsedInInvestingActivities",
    "us-gaap_NetCashProvidedByUsedInInvestingActivitiesContinuingOperations",
];
pub const OPERATING_TOTALS: [&str; 2] = [
    "us-gaap_NetCashProvidedByUsedInOperatingActivities",
    "us-gaap_NetCashProvidedByUsedInOperatingActivitiesContinuingOperations",
];
pub const LIABILITY_TOTALS: [&str; 2] = ["us-gaap_Liabilities", "us-gaap_LiabilitiesAndStockholdersEquity"];

struct Patterns {
    capex: Regex,
    capex_label: Regex,
    intangibles: Regex,
    not_a_payment: Regex,
    da: Regex,
    not_da: Regex,
    debt: Regex,
    not_debt: Regex,
}

fn patterns() -> &'static Patterns {
    static CELL: OnceLock<Patterns> = OnceLock::new();
    CELL.get_or_init(|| {
        let re = |text: &str| Regex::new(&format!("(?i){text}")).expect("pattern");
        Patterns {
            // Payments for long-life assets of the business: equipment and software
            // (PLANNING/RULES.md R2). Purchased intangibles are a separate pattern.
            // An oil company: the payments to explore and develop, not the purchase of
            // properties from other companies. A rental company: the rental equipment.
            // A utility: nuclear fuel and power-plant investments.
            capex: re("Equipment|ProductiveAssets|CapitalExpenditure|CapitalImprovements|ExploreAndDevelop|ToDevelop|DevelopRealEstate|Redevelop|Software|Construction|FixedAssets|FurnitureAndFixtures|AvailableForOperatingLease|NuclearFuel|IndependentPower"),
            // A filer can use a tag with a different name for its capex line. The label
            // of the line in the statement says what the line is.
            capex_label: re("capital expenditure|purchases? of property|additions to property"),
            intangibles: re("IntangibleAssets|Spectrum|Licenses"),
            not_a_payment: re("^Proceeds|Businesses|Acquisitions?Net"),
            da: re("Depreciation|Amortization|Depletion"),
            not_da: re("DebtDiscount|DebtIssuance|FinancingCosts|DeferredCharges|Premium|ShareBased|Contract|Content"),
            debt: re("Debt|Borrowings|NotesPayable|CommercialPaper|LineOfCredit|LongTermNotes|SeniorNotes"),
            not_debt: re("DebtSecurities|RightOfUse|PropertyPlant|Receivable|Investments?$"),
        }
    })
}

fn local(tag: &str) -> &str {
    tag.split_once('_').map(|(_, l)| l).unwrap_or(tag)
}

#[derive(Debug, Clone)]
pub struct Line {
    /// "us-gaap_Assets", or a company tag such as "vz_...".
    pub tag: String,
    pub label: String,
    /// +1 or -1 toward the parent line.
    pub weight: f64,
    pub value: Option<f64>,
    pub children: Vec<Line>,
}

impl Line {
    pub fn company_tag(&self) -> bool {
        let prefix = self.tag.split_once('_').map(|(p, _)| p).unwrap_or("");
        !STANDARD_PREFIXES.contains(&prefix)
    }

    /// The standard variable of the tag (None: the registry does not have the tag).
    pub fn variable<'a>(&self, variable_of_tag: &'a HashMap<String, String>) -> Option<&'a str> {
        variable_of_tag.get(&self.tag).map(String::as_str)
    }

    pub fn walk(&self) -> Vec<&Line> {
        let mut out = vec![self];
        for child in &self.children {
            out.extend(child.walk());
        }
        out
    }

    /// Sum of the parts with their weights (None if a part has no value).
    pub fn parts_sum(&self) -> Option<f64> {
        if self.children.is_empty() {
            return None;
        }
        let mut total = 0.0;
        for child in &self.children {
            total += child.weight * child.value?;
        }
        Some(total)
    }

    pub fn sum_agrees(&self) -> Option<bool> {
        let (total, value) = (self.parts_sum()?, self.value?);
        Some((total - value).abs() <= SUM_TOLERANCE * value.abs().max(1.0))
    }
}

#[derive(Debug, Clone)]
pub struct Statement {
    /// "income", "balance" or "cash".
    pub kind: String,
    pub name: String,
    pub role: String,
    pub roots: Vec<Line>,
}

impl Statement {
    pub fn lines(&self) -> Vec<&Line> {
        self.roots.iter().flat_map(|r| r.walk()).collect()
    }

    pub fn find(&self, tag: &str) -> Option<&Line> {
        self.lines().into_iter().find(|l| l.tag == tag)
    }
}

/// A fact that has one segment member, for example the capex of one business segment.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberFact {
    pub axis: String,
    pub member: String,
    pub value: f64,
}

/// Tag -> the facts of the tag that have one segment member.
pub type Members = HashMap<String, Vec<MemberFact>>;

/// An earlier fiscal year in the filing (the comparison columns).
#[derive(Debug, Clone)]
pub struct FilingYear {
    pub end: String,
    pub statements: Vec<Statement>,
    /// Consolidated values: tag -> value.
    pub values: HashMap<String, f64>,
    pub members: Members,
}

/// One fiscal year of a filing.
pub struct YearView<'a> {
    pub statements: &'a [Statement],
    pub values: &'a HashMap<String, f64>,
    pub members: &'a Members,
}

#[derive(Debug, Clone)]
pub struct Filing {
    pub ticker: String,
    pub accession: String,
    pub fiscal_year_end: Option<String>,
    /// The statements with the values of the fiscal year of the filing.
    pub statements: Vec<Statement>,
    pub values: HashMap<String, f64>,
    pub members: Members,
    pub earlier: Vec<FilingYear>,
    /// Numeric facts in the instance document, and those with a segment.
    pub facts_total: usize,
    pub facts_with_segment: usize,
}

impl Filing {
    pub fn statement(&self, kind: &str) -> Option<&Statement> {
        self.statements.iter().find(|s| s.kind == kind)
    }

    /// The statements and the member facts of the fiscal year that ends on `end`
    /// (or not more than 14 days from it).
    pub fn year(&self, end: &str) -> Option<YearView<'_>> {
        let near = |other: &str| match (dates::parse(end), dates::parse(other)) {
            (Some(a), Some(b)) => (a - b).abs() <= NEAR_DAYS,
            _ => end == other,
        };
        if self.fiscal_year_end.as_deref().map(near).unwrap_or(false) {
            return Some(YearView { statements: &self.statements, values: &self.values, members: &self.members });
        }
        self.earlier.iter().find(|y| near(&y.end)).map(|y| YearView { statements: &y.statements, values: &y.values, members: &y.members })
    }
}

/// The value of a tag that has no consolidated fact, from its facts with one segment
/// member. The result has a text that says how the value was made.
/// - `largest_only` (share counts): the largest member. A company with two share
///   classes gives the count of each class; the largest class is the count that
///   agrees with the earnings per share of the ticker.
/// - A member of the legal-entity axis is not used.
/// - Other tags: the sum of the members of one axis. If one member is equal to the sum
///   of the other members, it is their total, and it is the value.
/// - Two or more axes: the axis with the largest result (the full breakdown).
pub fn member_value(facts: &[MemberFact], largest_only: bool) -> Option<(f64, String)> {
    let mut axes: Vec<(&str, Vec<&MemberFact>)> = Vec::new();
    // A member of the legal-entity axis is a subsidiary that files in the same
    // document (a utility and its parent). Its value is not the group value.
    for fact in facts.iter().filter(|f| f.axis != "dei:LegalEntityAxis") {
        match axes.iter_mut().find(|(axis, _)| *axis == fact.axis) {
            Some((_, list)) => {
                if !list.iter().any(|f| f.member == fact.member) {
                    list.push(fact);
                }
            }
            None => axes.push((&fact.axis, vec![fact])),
        }
    }
    axes.sort_by(|a, b| a.0.cmp(b.0));
    let mut best: Option<(f64, String)> = None;
    for (axis, list) in axes {
        let largest = list.iter().copied().max_by(|a, b| a.value.abs().total_cmp(&b.value.abs()))?;
        let rest: f64 = list.iter().filter(|f| f.member != largest.member).map(|f| f.value).sum();
        let result = if largest_only {
            (largest.value, format!("the largest member {} of {axis}", largest.member))
        } else if list.len() == 1 {
            (largest.value, format!("the member {} of {axis}", largest.member))
        } else if (largest.value - rest).abs() <= SUM_TOLERANCE * largest.value.abs() {
            (largest.value, format!("the member {} of {axis}, the total of the other members", largest.member))
        } else {
            (largest.value + rest, format!("sum of {} members of {axis}", list.len()))
        };
        if best.as_ref().map(|b| result.0.abs() > b.0.abs()).unwrap_or(true) {
            best = Some(result);
        }
    }
    best
}

/// "us-gaap_Assets" -> "total_assets", for each tag in the registry.
pub fn variable_of_tag(registry: &Registry) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for variable in &registry.variables {
        for tag in &variable.tags {
            out.insert(format!("{}_{}", tag.namespace, tag.name), variable.name.clone());
        }
    }
    out
}

fn statement_kind(long_name: &str) -> Option<&'static str> {
    let text = long_name.to_lowercase();
    if text.contains("parenthetical") || text.contains("comprehensive") {
        None
    } else if text.contains("cash flow") {
        Some("cash")
    } else if text.contains("balance sheet") || text.contains("financial position") || text.contains("financial condition") {
        Some("balance")
    } else if text.contains("equity") || text.contains("capital") {
        None
    } else if text.contains("income") || text.contains("operations") || text.contains("earnings") {
        Some("income")
    } else {
        None
    }
}

/// The values of one fiscal year in the instance document.
#[derive(Debug, Clone, Default)]
pub struct YearValues {
    pub end: String,
    /// Consolidated values (facts with no segment): tag -> value.
    pub values: HashMap<String, f64>,
    pub members: Members,
}

pub struct InstanceValues {
    /// The fiscal year of the filing first, then the earlier fiscal years (latest first).
    pub years: Vec<YearValues>,
    pub fiscal_year_end: Option<String>,
    pub facts_total: usize,
    pub facts_with_segment: usize,
}

struct Context {
    start: Option<String>,
    end: String,
    /// Number of segment members.
    segments: usize,
    /// (axis, member) if the context has one explicit segment member.
    member: Option<(String, String)>,
}

impl Context {
    /// True for a balance date, and for a flow that covers a full fiscal year.
    fn annual_or_instant(&self) -> bool {
        match &self.start {
            None => true,
            Some(start) => match (dates::parse(&self.end), dates::parse(start)) {
                (Some(e), Some(s)) => (350..=380).contains(&(e - s)),
                _ => false,
            },
        }
    }
}

/// Read the instance document. `tag_of` maps (namespace URI, local name) to the tag
/// key of MetaLinks.json. For a flow, the value is the fact that covers the full
/// fiscal year; for a balance, the fact on the fiscal year end.
pub fn parse_instance(xml: &str, tag_of: &HashMap<(String, String), String>) -> Result<InstanceValues, String> {
    let options = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(xml, options).map_err(|e| format!("instance document: {e}"))?;
    let root = doc.root_element();

    let mut contexts: HashMap<&str, Context> = HashMap::new();
    for ctx in root.children().filter(|n| n.has_tag_name((XBRLI, "context"))) {
        let Some(id) = ctx.attribute("id") else { continue };
        let Some(period) = ctx.children().find(|n| n.has_tag_name((XBRLI, "period"))) else { continue };
        let text_of = |name: &str| {
            period
                .children()
                .find(|n| n.has_tag_name((XBRLI, name)))
                .and_then(|n| n.text())
                .map(|t| t.trim().to_string())
        };
        let instant = text_of("instant");
        let start = text_of("startDate");
        let Some(end) = instant.clone().or_else(|| text_of("endDate")) else { continue };
        let members: Vec<roxmltree::Node> = ctx
            .children()
            .filter(|n| n.has_tag_name((XBRLI, "entity")))
            .flat_map(|entity| entity.children().filter(|n| n.has_tag_name((XBRLI, "segment"))))
            .flat_map(|segment| segment.children().filter(|n| n.is_element()))
            .collect();
        let member = match members.as_slice() {
            [one] if one.has_tag_name((XBRLDI, "explicitMember")) => match (one.attribute("dimension"), one.text()) {
                (Some(axis), Some(member)) => Some((axis.trim().to_string(), member.trim().to_string())),
                _ => None,
            },
            _ => None,
        };
        contexts.insert(id, Context { start: if instant.is_some() { None } else { start }, end, segments: members.len(), member });
    }

    let mut fiscal_year_end: Option<String> = None;
    let mut raw: Vec<(&String, &Context, f64)> = Vec::new();
    let (mut total, mut with_segment) = (0usize, 0usize);
    for element in root.children().filter(|n| n.is_element()) {
        let Some(reference) = element.attribute("contextRef") else { continue };
        let Some(context) = contexts.get(reference) else { continue };
        let Some(uri) = element.tag_name().namespace() else { continue };
        let name = element.tag_name().name();
        let text = element.text().map(str::trim).unwrap_or("");
        if name == "DocumentPeriodEndDate" && !text.is_empty() {
            if dates::parse(text).is_some() {
                fiscal_year_end = Some(text.to_string());
            }
            continue;
        }
        if element.attribute("unitRef").is_none() || text.is_empty() {
            continue;
        }
        let Ok(value) = text.parse::<f64>() else { continue };
        total += 1;
        if context.segments > 0 {
            with_segment += 1;
        }
        if context.segments > 0 && context.member.is_none() {
            continue;
        }
        if !context.annual_or_instant() {
            continue;
        }
        if let Some(tag) = tag_of.get(&(uri.to_string(), name.to_string())) {
            raw.push((tag, context, value));
        }
    }

    if fiscal_year_end.is_none() {
        fiscal_year_end = contexts.values().filter(|c| c.segments == 0).map(|c| c.end.clone()).max();
    }
    let mut years: Vec<YearValues> = Vec::new();
    if let Some(end) = &fiscal_year_end {
        // The fiscal years in the filing: the ends of the flows with no segment.
        let mut ends: Vec<&String> = contexts
            .values()
            .filter(|c| c.segments == 0 && c.start.is_some() && c.annual_or_instant() && c.end < *end)
            .map(|c| &c.end)
            .collect();
        ends.sort();
        ends.dedup();
        ends.reverse();
        years.push(YearValues { end: end.clone(), ..Default::default() });
        years.extend(ends.into_iter().map(|e| YearValues { end: e.clone(), ..Default::default() }));
        for (tag, context, value) in raw {
            let Some(year) = years.iter_mut().find(|y| y.end == context.end) else { continue };
            match &context.member {
                None => {
                    year.values.entry(tag.clone()).or_insert(value);
                }
                Some((axis, member)) => {
                    let list = year.members.entry(tag.clone()).or_default();
                    if !list.iter().any(|f| f.axis == *axis && f.member == *member) {
                        list.push(MemberFact { axis: axis.clone(), member: member.clone(), value });
                    }
                }
            }
        }
    }
    Ok(InstanceValues { years, fiscal_year_end, facts_total: total, facts_with_segment: with_segment })
}

/// The income statement, the balance sheet and the cash-flow statement as trees.
pub fn build_statements(meta: &Value, values: &HashMap<String, f64>) -> Vec<Statement> {
    let Some(instance) = meta.get("instance").and_then(Value::as_object).and_then(|m| m.values().next()) else {
        return Vec::new();
    };
    let empty = serde_json::Map::new();
    let tags = instance.get("tag").and_then(Value::as_object).unwrap_or(&empty);
    let reports = instance.get("report").and_then(Value::as_object).unwrap_or(&empty);
    let label = |tag: &str| -> String {
        let roles = tags.get(tag).and_then(|t| t.pointer("/lang/en-us/role"));
        roles
            .and_then(|r| r.get("terseLabel").or_else(|| r.get("label")))
            .and_then(Value::as_str)
            .unwrap_or(tag)
            .to_string()
    };

    // The reports in the sequence of the filing: "R2", "R3", ...
    let mut ordered: Vec<(u64, &Value)> = reports
        .iter()
        .map(|(key, report)| (key.trim_start_matches('R').parse().unwrap_or(u64::MAX), report))
        .collect();
    ordered.sort_by_key(|r| r.0);

    let mut statements = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    // Second pass: a filer with one statement for income and comprehensive income.
    for (_, report, combined) in ordered.iter().map(|(n, r)| (n, *r, false)).chain(ordered.iter().map(|(n, r)| (n, *r, true))) {
        if report.get("groupType").and_then(Value::as_str) != Some("statement") {
            continue;
        }
        if !report.get("subGroupType").and_then(Value::as_str).unwrap_or("").is_empty() {
            continue;
        }
        let long_name = report.get("longName").and_then(Value::as_str).unwrap_or("");
        let kind = if combined {
            let text = long_name.to_lowercase();
            if text.contains("comprehensive") && !text.contains("parenthetical") { Some("income") } else { None }
        } else {
            statement_kind(long_name)
        };
        let Some(kind) = kind else { continue };
        if seen.contains(kind) {
            continue;
        }
        let Some(role) = report.get("role").and_then(Value::as_str) else { continue };

        // parent tag (None = root) -> [(order, tag, weight)]
        let mut children: HashMap<Option<&str>, Vec<(f64, &str, f64)>> = HashMap::new();
        for (tag, info) in tags {
            let Some(arc) = info.get("calculation").and_then(|c| c.get(role)) else { continue };
            let parent = arc.get("parentTag").and_then(Value::as_str);
            let order = arc.get("order").and_then(Value::as_f64).unwrap_or(0.0);
            let weight = arc.get("weight").and_then(Value::as_f64).unwrap_or(1.0);
            children.entry(parent).or_default().push((order, tag.as_str(), weight));
        }
        for list in children.values_mut() {
            list.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(b.1)));
        }
        fn node(
            tag: &str,
            weight: f64,
            children: &HashMap<Option<&str>, Vec<(f64, &str, f64)>>,
            values: &HashMap<String, f64>,
            label: &dyn Fn(&str) -> String,
            depth: usize,
        ) -> Line {
            let mut line = Line { tag: tag.to_string(), label: label(tag), weight, value: values.get(tag).copied(), children: Vec::new() };
            if depth < 40 {
                if let Some(list) = children.get(&Some(tag)) {
                    for (_, child, child_weight) in list {
                        line.children.push(node(child, *child_weight, children, values, label, depth + 1));
                    }
                }
            }
            line
        }
        let roots: Vec<Line> = children
            .get(&None)
            .map(|list| list.iter().map(|(_, tag, _)| node(tag, 1.0, &children, values, &label, 0)).collect())
            .unwrap_or_default();
        if roots.is_empty() {
            continue;
        }
        let name = long_name.splitn(3, " - ").last().unwrap_or(long_name).to_string();
        statements.push(Statement { kind: kind.to_string(), name, role: role.to_string(), roots });
        seen.insert(kind);
    }
    statements
}

/// Read the filing in a cache folder (MetaLinks.json and the instance document).
pub fn load_filing(ticker: &str, folder: &Path) -> Result<Filing, String> {
    let meta_path = folder.join("MetaLinks.json");
    let instance_path = crate::sec::instance_path(folder);
    let (true, Some(instance_path)) = (meta_path.is_file(), instance_path) else {
        return Err(format!("{}: no MetaLinks.json or no instance document", folder.display()));
    };
    let meta: Value = serde_json::from_slice(&std::fs::read(&meta_path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("{}: {e}", meta_path.display()))?;
    let mut tag_of = HashMap::new();
    if let Some(tags) = meta.get("instance").and_then(Value::as_object).and_then(|m| m.values().next()).and_then(|i| i.get("tag")).and_then(Value::as_object) {
        for (tag, info) in tags {
            if let (Some(uri), Some(name)) = (info.get("nsuri").and_then(Value::as_str), info.get("localname").and_then(Value::as_str)) {
                tag_of.insert((uri.to_string(), name.to_string()), tag.clone());
            }
        }
    }
    let xml = std::fs::read_to_string(&instance_path).map_err(|e| format!("{}: {e}", instance_path.display()))?;
    let instance = parse_instance(&xml, &tag_of)?;
    let accession = folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    Ok(make_filing(ticker, accession, &meta, instance))
}

pub fn make_filing(ticker: &str, accession: String, meta: &Value, instance: InstanceValues) -> Filing {
    let mut years = instance.years.into_iter();
    let first = years.next().unwrap_or_default();
    Filing {
        ticker: ticker.to_uppercase(),
        accession,
        fiscal_year_end: instance.fiscal_year_end,
        statements: build_statements(meta, &first.values),
        values: first.values,
        members: first.members,
        earlier: years
            .map(|y| FilingYear { statements: build_statements(meta, &y.values), end: y.end, values: y.values, members: y.members })
            .collect(),
        facts_total: instance.facts_total,
        facts_with_segment: instance.facts_with_segment,
    }
}

/// The top lines with a value whose tag matches the pattern. `under`: only lines
/// below one of these tags. A line below a line that matches is not in the result:
/// the parent already contains it, and a sum of the two counts the amount two times.
pub fn lines_matching<'a>(statement: Option<&'a Statement>, pattern: &Regex, under: &[&str], exclude: Option<&Regex>) -> Vec<&'a Line> {
    lines_matching_label(statement, pattern, None, under, exclude)
}

/// The same, and a line whose label matches `label` is also in the result.
pub fn lines_matching_label<'a>(statement: Option<&'a Statement>, pattern: &Regex, label: Option<&Regex>, under: &[&str], exclude: Option<&Regex>) -> Vec<&'a Line> {
    let Some(statement) = statement else { return Vec::new() };
    let pattern = &(pattern, label);
    fn visit<'a>(line: &'a Line, pattern: &(&Regex, Option<&Regex>), exclude: Option<&Regex>, seen: &mut HashSet<&'a str>, out: &mut Vec<&'a Line>) {
        let name = local(&line.tag);
        let matches = pattern.0.is_match(name) || pattern.1.map(|l| l.is_match(&line.label)).unwrap_or(false);
        if line.value.is_some() && matches && !exclude.map(|e| e.is_match(name)).unwrap_or(false) {
            if seen.insert(line.tag.as_str()) {
                out.push(line);
            }
            return;
        }
        for child in &line.children {
            visit(child, pattern, exclude, seen, out);
        }
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    if under.is_empty() {
        for root in &statement.roots {
            visit(root, pattern, exclude, &mut seen, &mut out);
        }
    } else {
        for top in statement.lines().into_iter().filter(|l| under.contains(&l.tag.as_str())) {
            for child in &top.children {
                visit(child, pattern, exclude, &mut seen, &mut out);
            }
        }
    }
    out
}

pub fn capex_lines(filing: &Filing) -> Vec<&Line> {
    capex_lines_in(filing.statement("cash"))
}

/// The capital expenditure lines in the investing section of a cash-flow statement.
pub fn capex_lines_in(cash: Option<&Statement>) -> Vec<&Line> {
    let p = patterns();
    lines_matching_label(cash, &p.capex, Some(&p.capex_label), &INVESTING_TOTALS, Some(&p.not_a_payment))
}

/// True if the cash-flow statement has an investing section with lines that have values.
pub fn has_investing_lines(cash: Option<&Statement>) -> bool {
    let Some(cash) = cash else { return false };
    cash.lines()
        .into_iter()
        .filter(|l| INVESTING_TOTALS.contains(&l.tag.as_str()))
        .any(|total| total.children.iter().any(|c| c.value.is_some()))
}

pub fn intangible_purchase_lines(filing: &Filing) -> Vec<&Line> {
    let p = patterns();
    let capex: HashSet<&str> = capex_lines(filing).into_iter().map(|l| l.tag.as_str()).collect();
    lines_matching(filing.statement("cash"), &p.intangibles, &INVESTING_TOTALS, Some(&p.not_a_payment))
        .into_iter()
        .filter(|l| !capex.contains(l.tag.as_str()))
        .collect()
}

pub fn depreciation_lines(filing: &Filing) -> Vec<&Line> {
    let p = patterns();
    lines_matching(filing.statement("cash"), &p.da, &OPERATING_TOTALS, Some(&p.not_da))
}

pub fn debt_lines(filing: &Filing) -> Vec<&Line> {
    let p = patterns();
    lines_matching(filing.statement("balance"), &p.debt, &LIABILITY_TOTALS, Some(&p.not_debt))
}

/// Three inputs from the statements of the filing (None: no line found). The values
/// use the sign of the standard variables: capex is negative (cash out). Capex is
/// equipment and software (PLANNING/RULES.md R2).
#[derive(Debug, Clone, PartialEq)]
pub struct FilingEstimates {
    pub capex: Option<f64>,
    pub capex_with_intangibles: Option<f64>,
    pub da_total: Option<f64>,
    pub total_debt: Option<f64>,
}

pub fn estimates(filing: &Filing) -> FilingEstimates {
    // weight * value is the effect of a line on its section total (negative = cash out)
    let cash_effect = |lines: &[&Line]| -> Option<f64> {
        if lines.is_empty() {
            None
        } else {
            Some(lines.iter().map(|l| l.weight * l.value.unwrap_or(0.0)).sum())
        }
    };
    let total = |lines: &[&Line]| -> Option<f64> {
        if lines.is_empty() {
            None
        } else {
            Some(lines.iter().map(|l| l.value.unwrap_or(0.0)).sum())
        }
    };
    let capex = cash_effect(&capex_lines(filing));
    let intangibles = cash_effect(&intangible_purchase_lines(filing));
    FilingEstimates {
        capex,
        capex_with_intangibles: capex.map(|c| c + intangibles.unwrap_or(0.0)),
        da_total: total(&depreciation_lines(filing)),
        total_debt: total(&debt_lines(filing)),
    }
}

/// The text report of a filing.
pub fn report(registry: &Registry, filing: &Filing, statement: Option<&str>, focus: bool) -> String {
    use std::fmt::Write;
    let variables = variable_of_tag(registry);
    let value = |line: &Line| match line.value {
        Some(v) => format!("{:>12.3}", v / 1e9),
        None => format!("{:>12}", "-"),
    };
    let kind = |line: &Line| match line.variable(&variables) {
        Some(name) => format!("[{name}]"),
        None if line.company_tag() => "[company tag]".to_string(),
        None => "[no variable]".to_string(),
    };
    let mut out = String::new();
    let share = if filing.facts_total > 0 { 100.0 * filing.facts_with_segment as f64 / filing.facts_total as f64 } else { 0.0 };
    let _ = writeln!(out, "\n{}", "=".repeat(110));
    let _ = writeln!(
        out,
        "{}  10-K {}  fiscal year end {}  |  {} numeric facts, {:.0}% with a segment  |  values in $B",
        filing.ticker,
        filing.accession,
        filing.fiscal_year_end.as_deref().unwrap_or("-"),
        filing.facts_total,
        share
    );
    fn print_line(out: &mut String, line: &Line, depth: usize, value: &dyn Fn(&Line) -> String, kind: &dyn Fn(&Line) -> String) {
        use std::fmt::Write;
        let sign = if line.weight >= 0.0 { "+" } else { "-" };
        let note = match (line.sum_agrees(), line.parts_sum()) {
            (Some(false), Some(sum)) => format!("  parts differ: {:.3}", sum / 1e9),
            _ => String::new(),
        };
        let name: String = format!("{}{} {}", "  ".repeat(depth), sign, line.label).chars().take(62).collect();
        let _ = writeln!(out, "  {:62} {}  {} {}{}", name, value(line), kind(line), line.tag, note);
        for child in &line.children {
            print_line(out, child, depth + 1, value, kind);
        }
    }
    if !focus {
        for st in &filing.statements {
            if statement.map(|s| s != st.kind).unwrap_or(false) {
                continue;
            }
            let lines = st.lines();
            let company = lines.iter().filter(|l| l.company_tag()).count();
            let unknown = lines.iter().filter(|l| !l.company_tag() && l.variable(&variables).is_none()).count();
            let _ = writeln!(
                out,
                "\n  --- {}: {} ({} lines; {} company tags; {} standard tags with no variable) ---",
                st.kind,
                st.name,
                lines.len(),
                company,
                unknown
            );
            for root in &st.roots {
                print_line(&mut out, root, 0, &value, &kind);
            }
        }
    }
    if focus || statement.is_none() {
        let sections: [(&str, Vec<&Line>); 4] = [
            ("Capital expenditure lines (in the investing section)", capex_lines(filing)),
            ("Purchases of intangible assets (in the investing section)", intangible_purchase_lines(filing)),
            ("Depreciation and amortization lines (in the operating section)", depreciation_lines(filing)),
            ("Debt lines (balance sheet)", debt_lines(filing)),
        ];
        for (title, lines) in sections {
            let _ = writeln!(out, "\n  {title}:");
            if lines.is_empty() {
                let _ = writeln!(out, "    none");
            }
            for line in lines {
                let label: String = line.label.chars().take(58).collect();
                let _ = writeln!(out, "    {:58} {}  {} {}", label, value(line), kind(line), line.tag);
            }
        }
        let e = estimates(filing);
        let b = |v: Option<f64>| v.map(|v| format!("{:.3}", v / 1e9)).unwrap_or_else(|| "no line found".to_string());
        let _ = writeln!(
            out,
            "\n  From these lines: capex {} | capex with purchased intangibles {} | depreciation and amortization {} | total debt {}",
            b(e.capex),
            b(e.capex_with_intangibles),
            b(e.da_total),
            b(e.total_debt)
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const GAAP: &str = "http://fasb.org/us-gaap/2025";
    const ACME: &str = "http://www.acme.com/20241231";
    const BS: &str = "http://acme/role/BalanceSheet";
    const CF: &str = "http://acme/role/CashFlow";
    const IS: &str = "http://acme/role/Income";

    fn tag(prefix: &str, name: &str, label: &str, role: &str, parent: Option<&str>, weight: f64, order: f64) -> (String, Value) {
        let arc = match parent {
            Some(p) => json!({"parentTag": p, "weight": weight, "order": order}),
            None => json!({"parentTag": null, "weight": null, "order": null, "root": true}),
        };
        let uri = if prefix == "us-gaap" { GAAP } else { ACME };
        (
            format!("{prefix}_{name}"),
            json!({"nsuri": uri, "localname": name, "lang": {"en-us": {"role": {"terseLabel": label}}}, "calculation": {role: arc}}),
        )
    }

    fn meta() -> Value {
        let inv = Some("us-gaap_NetCashProvidedByUsedInInvestingActivities");
        let op = Some("us-gaap_NetCashProvidedByUsedInOperatingActivities");
        let liab = Some("us-gaap_Liabilities");
        let tags: serde_json::Map<String, Value> = vec![
            tag("us-gaap", "LiabilitiesAndStockholdersEquity", "Total liabilities and equity", BS, None, 1.0, 0.0),
            tag("us-gaap", "Liabilities", "Total liabilities", BS, Some("us-gaap_LiabilitiesAndStockholdersEquity"), 1.0, 1.0),
            tag("us-gaap", "StockholdersEquity", "Equity", BS, Some("us-gaap_LiabilitiesAndStockholdersEquity"), 1.0, 2.0),
            tag("us-gaap", "ShortTermBorrowings", "Short-term debt", BS, liab, 1.0, 1.0),
            tag("us-gaap", "LongTermDebtNoncurrent", "Long-term debt", BS, liab, 1.0, 2.0),
            tag("us-gaap", "OperatingLeaseLiabilityNoncurrent", "Lease liabilities", BS, liab, 1.0, 3.0),
            tag("acme", "OtherObligations", "Other obligations", BS, liab, 1.0, 4.0),
            tag("us-gaap", "CashPeriodIncreaseDecrease", "Change in cash", CF, None, 1.0, 0.0),
            tag("us-gaap", "NetCashProvidedByUsedInOperatingActivities", "Operating cash flow", CF, Some("us-gaap_CashPeriodIncreaseDecrease"), 1.0, 1.0),
            tag("us-gaap", "NetCashProvidedByUsedInInvestingActivities", "Investing cash flow", CF, Some("us-gaap_CashPeriodIncreaseDecrease"), 1.0, 2.0),
            tag("us-gaap", "NetIncomeLoss", "Net income", CF, op, 1.0, 1.0),
            tag("us-gaap", "Depreciation", "Depreciation", CF, op, 1.0, 2.0),
            tag("us-gaap", "AdjustmentForAmortization", "Amortization", CF, op, 1.0, 3.0),
            tag("us-gaap", "AmortizationOfDebtDiscountPremium", "Debt discount", CF, op, 1.0, 4.0),
            tag("us-gaap", "PaymentsToAcquirePropertyPlantAndEquipment", "Purchases of equipment", CF, inv, -1.0, 1.0),
            tag("acme", "PaymentsToAcquireAndDevelopSoftware", "Software", CF, inv, -1.0, 2.0),
            tag("us-gaap", "PaymentsToAcquireIntangibleAssets", "Licenses", CF, inv, -1.0, 3.0),
            tag("us-gaap", "ProceedsFromSaleOfPropertyPlantAndEquipment", "Sale of equipment", CF, inv, 1.0, 4.0),
            tag("us-gaap", "PaymentsToAcquireBusinessesNetOfCashAcquired", "Acquisitions", CF, inv, -1.0, 5.0),
            tag("us-gaap", "GrossProfit", "Gross profit", IS, None, 1.0, 0.0),
            tag("us-gaap", "Revenues", "Revenue", IS, Some("us-gaap_GrossProfit"), 1.0, 1.0),
            tag("us-gaap", "CostOfRevenue", "Cost of revenue", IS, Some("us-gaap_GrossProfit"), -1.0, 2.0),
        ]
        .into_iter()
        .collect();
        json!({"instance": {"acme-20241231.htm": {"tag": tags, "report": {
            "R1": {"groupType": "document", "subGroupType": "", "longName": "0001 - Document - Cover", "role": "http://acme/role/Cover"},
            "R2": {"groupType": "statement", "subGroupType": "", "longName": "0002 - Statement - Consolidated Statements of Income", "role": IS},
            "R3": {"groupType": "statement", "subGroupType": "", "longName": "0003 - Statement - Consolidated Statements of Comprehensive Income", "role": "http://acme/role/OCI"},
            "R4": {"groupType": "statement", "subGroupType": "", "longName": "0004 - Statement - Consolidated Balance Sheets", "role": BS},
            "R5": {"groupType": "statement", "subGroupType": "parenthetical", "longName": "0005 - Statement - Consolidated Balance Sheets (Parenthetical)", "role": "http://acme/role/BSP"},
            "R6": {"groupType": "statement", "subGroupType": "", "longName": "0006 - Statement - Consolidated Statements of Cash Flows", "role": CF}
        }}}})
    }

    fn instance() -> String {
        let fact = |prefix: &str, name: &str, ctx: &str, value: f64| {
            format!("<{prefix}:{name} contextRef=\"{ctx}\" unitRef=\"usd\" decimals=\"-6\">{value}</{prefix}:{name}>")
        };
        let facts: String = [
            fact("us-gaap", "LiabilitiesAndStockholdersEquity", "now", 1500.0),
            fact("us-gaap", "Liabilities", "now", 600.0),
            fact("us-gaap", "StockholdersEquity", "now", 900.0),
            fact("us-gaap", "ShortTermBorrowings", "now", 64.0),
            fact("us-gaap", "LongTermDebtNoncurrent", "now", 400.0),
            fact("us-gaap", "OperatingLeaseLiabilityNoncurrent", "now", 36.0),
            fact("acme", "OtherObligations", "now", 100.0),
            fact("us-gaap", "LongTermDebtNoncurrent", "before", 380.0),
            fact("us-gaap", "NetCashProvidedByUsedInOperatingActivities", "fy", 200.0),
            fact("us-gaap", "NetCashProvidedByUsedInInvestingActivities", "fy", -95.0),
            fact("us-gaap", "NetIncomeLoss", "fy", 100.0),
            fact("us-gaap", "Depreciation", "fy", 40.0),
            fact("us-gaap", "AdjustmentForAmortization", "fy", 25.0),
            fact("us-gaap", "AmortizationOfDebtDiscountPremium", "fy", 2.0),
            fact("us-gaap", "PaymentsToAcquirePropertyPlantAndEquipment", "fy", 50.0),
            fact("acme", "PaymentsToAcquireAndDevelopSoftware", "fy", 30.0),
            fact("us-gaap", "PaymentsToAcquireIntangibleAssets", "fy", 10.0),
            fact("us-gaap", "ProceedsFromSaleOfPropertyPlantAndEquipment", "fy", 5.0),
            fact("us-gaap", "PaymentsToAcquireBusinessesNetOfCashAcquired", "fy", 10.0),
            fact("us-gaap", "Revenues", "fy", 1000.0),
            fact("us-gaap", "Revenues", "q4", 270.0),
            fact("us-gaap", "Revenues", "segment", 600.0),
            fact("us-gaap", "CostOfRevenue", "two", 300.0),
            fact("us-gaap", "Revenues", "prior", 900.0),
            fact("us-gaap", "PaymentsToAcquirePropertyPlantAndEquipment", "prior", 45.0),
            fact("us-gaap", "NetCashProvidedByUsedInInvestingActivities", "prior", -45.0),
            fact("us-gaap", "CostOfRevenue", "fy", 560.0),
            fact("us-gaap", "GrossProfit", "fy", 440.0),
        ]
        .concat();
        let entity = "<entity><identifier scheme=\"http://www.sec.gov/CIK\">1</identifier></entity>";
        let segment = "<entity><identifier scheme=\"http://www.sec.gov/CIK\">1</identifier><segment><xbrldi:explicitMember dimension=\"us-gaap:StatementBusinessSegmentsAxis\">acme:CloudMember</xbrldi:explicitMember></segment></entity>";
        let two = segment.replace("</segment>", "<xbrldi:explicitMember dimension=\"srt:StatementGeographicalAxis\">country:US</xbrldi:explicitMember></segment>");
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><xbrl xmlns=\"http://www.xbrl.org/2003/instance\" xmlns:us-gaap=\"{GAAP}\" xmlns:acme=\"{ACME}\" xmlns:dei=\"http://xbrl.sec.gov/dei/2025\" xmlns:xbrldi=\"http://xbrl.org/2006/xbrldi\">\
             <context id=\"fy\">{entity}<period><startDate>2024-01-01</startDate><endDate>2024-12-31</endDate></period></context>\
             <context id=\"q4\">{entity}<period><startDate>2024-10-01</startDate><endDate>2024-12-31</endDate></period></context>\
             <context id=\"segment\">{segment}<period><startDate>2024-01-01</startDate><endDate>2024-12-31</endDate></period></context>\
             <context id=\"two\">{two}<period><startDate>2024-01-01</startDate><endDate>2024-12-31</endDate></period></context>\
             <context id=\"prior\">{entity}<period><startDate>2023-01-01</startDate><endDate>2023-12-31</endDate></period></context>\
             <context id=\"now\">{entity}<period><instant>2024-12-31</instant></period></context>\
             <context id=\"before\">{entity}<period><instant>2023-12-31</instant></period></context>\
             <dei:DocumentPeriodEndDate contextRef=\"fy\">2024-12-31</dei:DocumentPeriodEndDate>{facts}</xbrl>"
        )
    }

    fn filing() -> Filing {
        let meta = meta();
        let mut tag_of = HashMap::new();
        for (tag, info) in meta["instance"]["acme-20241231.htm"]["tag"].as_object().unwrap() {
            tag_of.insert((info["nsuri"].as_str().unwrap().to_string(), info["localname"].as_str().unwrap().to_string()), tag.clone());
        }
        let instance = parse_instance(&instance(), &tag_of).unwrap();
        assert_eq!(instance.fiscal_year_end.as_deref(), Some("2024-12-31"));
        let year = &instance.years[0];
        assert_eq!(year.values["us-gaap_Revenues"], 1000.0); // not the quarter, not the segment
        assert_eq!(year.values["us-gaap_LongTermDebtNoncurrent"], 400.0); // not the prior year
        assert_eq!(year.values["acme_PaymentsToAcquireAndDevelopSoftware"], 30.0);
        assert_eq!((instance.facts_total, instance.facts_with_segment), (28, 2));
        make_filing("ACME", "0000000001-25-000001".into(), &meta, instance)
    }

    #[test]
    fn earlier_fiscal_year_and_member_facts_are_kept() {
        let f = filing();
        assert_eq!(f.earlier.len(), 1);
        assert_eq!(f.earlier[0].end, "2023-12-31");
        let year = f.year("2023-12-31").unwrap();
        let cash = year.statements.iter().find(|s| s.kind == "cash").unwrap();
        assert_eq!(cash.find("us-gaap_PaymentsToAcquirePropertyPlantAndEquipment").unwrap().value, Some(45.0));
        // The fiscal year end of the company facts can be some days from the filing date.
        assert!(f.year("2024-12-28").is_some() && f.year("2022-12-31").is_none());
        let members = f.year("2024-12-31").unwrap().members;
        assert_eq!(members["us-gaap_Revenues"], vec![MemberFact { axis: "us-gaap:StatementBusinessSegmentsAxis".into(), member: "acme:CloudMember".into(), value: 600.0 }]);
        // A fact with two segment members is not kept.
        assert!(!members.contains_key("us-gaap_CostOfRevenue"));
    }

    fn member(axis: &str, name: &str, value: f64) -> MemberFact {
        MemberFact { axis: axis.into(), member: name.into(), value }
    }

    #[test]
    fn member_value_is_a_sum_a_total_member_or_the_largest_member() {
        // The members of one axis are the parts: the value is the sum.
        let parts = [member("a:TypeAxis", "x:Ongoing", 3824.0), member("a:TypeAxis", "x:Plant", 812.0)];
        assert_eq!(member_value(&parts, false).unwrap().0, 4636.0);
        // One member is the total of the other members: it is counted one time.
        let nested = [member("a:ProductAxis", "x:All", 8920.0), member("a:ProductAxis", "x:Made", 7229.0), member("a:ProductAxis", "x:Bought", 1691.0)];
        let (value, text) = member_value(&nested, false).unwrap();
        assert_eq!(value, 8920.0);
        assert!(text.contains("x:All") && text.contains("total of the other members"));
        // Share classes: the largest class, not the sum.
        let classes = [member("a:ClassAxis", "x:A", 1907.0), member("a:ClassAxis", "x:B", 73.0)];
        assert_eq!(member_value(&classes, true).unwrap().0, 1907.0);
        // A subsidiary that files in the same document is not the group.
        let entities = [member("dei:LegalEntityAxis", "x:Subsidiary", 90.0)];
        assert!(member_value(&entities, false).is_none());
        // Two axes: the axis with the full breakdown.
        let two = [member("a:ProductAxis", "x:P1", 60.0), member("a:ProductAxis", "x:P2", 40.0), member("a:PartnerAxis", "x:K", 5.0)];
        assert_eq!(member_value(&two, false).unwrap().0, 100.0);
        assert!(member_value(&[], false).is_none());
    }

    fn tags(lines: Vec<&Line>) -> Vec<&str> {
        lines.into_iter().map(|l| l.tag.as_str()).collect()
    }

    #[test]
    fn the_three_statements_are_found_and_others_are_not() {
        let f = filing();
        assert_eq!(f.statements.iter().map(|s| s.kind.as_str()).collect::<Vec<_>>(), vec!["income", "balance", "cash"]);
        assert_eq!(f.statement("balance").unwrap().name, "Consolidated Balance Sheets");
    }

    #[test]
    fn tree_has_parents_weights_and_values() {
        let f = filing();
        let root = &f.statement("income").unwrap().roots[0];
        assert_eq!((root.tag.as_str(), root.value), ("us-gaap_GrossProfit", Some(440.0)));
        let parts: Vec<(&str, f64, Option<f64>)> = root.children.iter().map(|c| (c.tag.as_str(), c.weight, c.value)).collect();
        assert_eq!(parts, vec![("us-gaap_Revenues", 1.0, Some(1000.0)), ("us-gaap_CostOfRevenue", -1.0, Some(560.0))]);
        assert_eq!(root.parts_sum(), Some(440.0));
        assert_eq!(root.sum_agrees(), Some(true));
    }

    #[test]
    fn line_shows_its_standard_variable_or_that_it_is_a_company_tag() {
        let registry = Registry::embedded().unwrap();
        let variables = variable_of_tag(&registry);
        let f = filing();
        let balance = f.statement("balance").unwrap();
        assert_eq!(balance.find("us-gaap_LongTermDebtNoncurrent").unwrap().variable(&variables), Some("long_term_debt"));
        let other = balance.find("acme_OtherObligations").unwrap();
        assert!(other.company_tag() && other.variable(&variables).is_none());
        assert!(!balance.find("us-gaap_Liabilities").unwrap().company_tag());
    }

    #[test]
    fn parts_that_do_not_sum_are_found() {
        let f = filing();
        let mut liabilities = f.statement("balance").unwrap().find("us-gaap_Liabilities").unwrap().clone();
        assert_eq!(liabilities.sum_agrees(), Some(true));
        liabilities.children[0].value = Some(164.0);
        assert_eq!(liabilities.sum_agrees(), Some(false));
    }

    #[test]
    fn capex_lines_include_a_company_tag_and_no_proceeds_and_no_acquisitions() {
        let f = filing();
        assert_eq!(tags(capex_lines(&f)), vec!["us-gaap_PaymentsToAcquirePropertyPlantAndEquipment", "acme_PaymentsToAcquireAndDevelopSoftware"]);
        assert_eq!(tags(intangible_purchase_lines(&f)), vec!["us-gaap_PaymentsToAcquireIntangibleAssets"]);
    }

    #[test]
    fn depreciation_lines_do_not_include_debt_discount_and_debt_lines_do_not_include_leases() {
        let f = filing();
        assert_eq!(tags(depreciation_lines(&f)), vec!["us-gaap_Depreciation", "us-gaap_AdjustmentForAmortization"]);
        assert_eq!(tags(debt_lines(&f)), vec!["us-gaap_ShortTermBorrowings", "us-gaap_LongTermDebtNoncurrent"]);
    }

    #[test]
    fn estimates_from_the_filing_lines() {
        let e = estimates(&filing());
        assert_eq!(e, FilingEstimates { capex: Some(-80.0), capex_with_intangibles: Some(-90.0), da_total: Some(65.0), total_debt: Some(464.0) });
    }
}
