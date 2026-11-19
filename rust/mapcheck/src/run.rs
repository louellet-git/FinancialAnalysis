//! Run the mapping and the checks for one ticker, and for a list of tickers.

use crate::checks::{check_company_as, check_period_as, filer_template, Finding, PeriodChecks, Severity};
use crate::facts::CompanyFacts;
use crate::mapper::{map_company_with, Company, Fill};
use crate::supplement::Supplement;
use std::cell::OnceCell;
use crate::registry::Registry;
use flate2::read::GzDecoder;
use rayon::prelude::*;
use serde_json::{json, Map, Value};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Pass,
    Warn,
    Fail,
    /// Not judged: a foreign filer with annual reports on form 20-F or 40-F, a filer
    /// with a reporting currency that is not USD, a new registrant that has no 10-K,
    /// or a company that has no revenue (the DCF does not fit).
    Template,
    /// The data cannot be read (no file, not us-gaap, not JSON).
    Error,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Pass => "PASS",
            Status::Warn => "WARN",
            Status::Fail => "FAIL",
            Status::Template => "N/A",
            Status::Error => "ERROR",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TickerReport {
    pub ticker: String,
    pub name: String,
    pub status: Status,
    pub template: String,
    pub error: Option<String>,
    pub fiscal_year_end: Option<String>,
    pub annual_rows: usize,
    pub quarterly_rows: usize,
    /// Findings for the company and for the latest fiscal year.
    pub findings: Vec<Finding>,
    /// Checks of the latest fiscal year.
    pub latest: PeriodChecks,
    /// Number of core inputs with a value in the latest fiscal year.
    pub core_filled: usize,
    pub seconds: f64,
    pub company: Option<Company>,
}

/// The companyfacts file for a ticker: `<cache>/<TICKER>/company_facts.json[.gz]`.
pub fn facts_path(cache: &Path, ticker: &str) -> Option<PathBuf> {
    let dir = cache.join(ticker);
    for name in ["company_facts.json", "company_facts.json.gz"] {
        let path = dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// The bytes of a cache file (a `.gz` file is decompressed).
pub fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    if path.extension().map(|e| e == "gz").unwrap_or(false) {
        GzDecoder::new(BufReader::new(file)).read_to_end(&mut bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    } else {
        BufReader::new(file).read_to_end(&mut bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(bytes)
}

pub fn read_facts(path: &Path) -> Result<CompanyFacts, String> {
    serde_json::from_slice(&read_bytes(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn error_report(ticker: &str, message: String, seconds: f64) -> TickerReport {
    TickerReport {
        ticker: ticker.to_string(),
        name: String::new(),
        status: Status::Error,
        template: String::new(),
        error: Some(message),
        fiscal_year_end: None,
        annual_rows: 0,
        quarterly_rows: 0,
        findings: Vec::new(),
        latest: PeriodChecks::default(),
        core_filled: 0,
        seconds,
        company: None,
    }
}

/// Map one company. A 10-K of the ticker in the cache gives the critical inputs that
/// the company facts do not have. The filing is read only if an input has no value.
pub fn map_ticker(registry: &Registry, cache: &Path, ticker: &str, facts: &CompanyFacts, n_periods: usize) -> Company {
    let filing: OnceCell<Option<Supplement>> = OnceCell::new();
    let fill = |end: &str, name: &str| {
        filing.get_or_init(|| Supplement::load(registry, cache, ticker)).as_ref().and_then(|s| s.fill(registry, end, name))
    };
    map_company_with(registry, facts, n_periods, Some(&fill))
}

/// Map and check one company from its facts (no filing).
pub fn check_facts(registry: &Registry, ticker: &str, facts: &CompanyFacts, n_periods: usize, as_of: i64, keep_values: bool) -> TickerReport {
    check_facts_with(registry, ticker, facts, n_periods, as_of, keep_values, None)
}

/// Map and check one company from its facts, and from the 10-K in the cache.
pub fn check_cached(registry: &Registry, cache: &Path, ticker: &str, facts: &CompanyFacts, n_periods: usize, as_of: i64, keep_values: bool) -> TickerReport {
    let filing: OnceCell<Option<Supplement>> = OnceCell::new();
    let fill = |end: &str, name: &str| {
        filing.get_or_init(|| Supplement::load(registry, cache, ticker)).as_ref().and_then(|s| s.fill(registry, end, name))
    };
    check_facts_with(registry, ticker, facts, n_periods, as_of, keep_values, Some(&fill))
}

fn check_facts_with(registry: &Registry, ticker: &str, facts: &CompanyFacts, n_periods: usize, as_of: i64, keep_values: bool, fill: Option<Fill>) -> TickerReport {
    let start = Instant::now();
    if !facts.has_us_gaap() {
        let namespaces: Vec<&String> = facts.facts.keys().collect();
        return error_report(ticker, format!("not a us-gaap filer (namespaces: {namespaces:?})"), start.elapsed().as_secs_f64());
    }
    let company = map_company_with(registry, facts, n_periods, fill);
    let mut template = company.annual.last().map(|p| filer_template(registry, p)).unwrap_or("standard");
    let mut findings = check_company_as(registry, &company, as_of, template);
    let mut latest = PeriodChecks::default();
    let mut earlier_failures = 0usize;
    if let Some(period) = company.annual.last() {
        latest = check_period_as(registry, period, template);
        findings.extend(latest.findings.iter().cloned());
        // Earlier fiscal years: count the years that have a Fail.
        let mut codes: Vec<String> = Vec::new();
        for earlier in &company.annual[..company.annual.len() - 1] {
            let checks = check_period_as(registry, earlier, template);
            let fails: Vec<&Finding> = checks.findings.iter().filter(|f| f.severity == Severity::Fail).collect();
            if !fails.is_empty() {
                earlier_failures += 1;
                for f in fails {
                    if !codes.contains(&f.code) {
                        codes.push(f.code.clone());
                    }
                }
            }
        }
        if earlier_failures > 0 {
            findings.push(Finding {
                severity: Severity::Warn,
                code: "earlier_years_fail".to_string(),
                detail: format!("{earlier_failures} of {} earlier fiscal years have a Fail: {}", company.annual.len() - 1, codes.join(", ")),
            });
        }
    }
    // Not judged: a foreign filer, a currency that is not USD, a registrant that has
    // no 10-K (a new company), or a company with no revenue.
    let mut judged = true;
    if company.annual.is_empty() {
        if has_form(facts, &["20-F", "20-F/A", "40-F", "40-F/A"]) {
            template = "foreign";
            judged = false;
        } else if !has_form(facts, &["10-K", "10-K/A"]) {
            template = "new";
            judged = false;
        } else if let Some(currency) = other_currency(facts) {
            findings.push(Finding { severity: Severity::Info, code: "currency".to_string(), detail: format!("the filer reports in {currency}, not in USD") });
            template = "non-usd";
            judged = false;
        }
    } else if template == "standard" && no_revenue(registry, &company) {
        template = "no-revenue";
        judged = false;
    }
    let worst = findings.iter().map(|f| f.severity).max().unwrap_or(Severity::Info);
    let status = if !judged {
        Status::Template
    } else {
        match worst {
            Severity::Fail => Status::Fail,
            Severity::Warn => Status::Warn,
            Severity::Info => Status::Pass,
        }
    };
    let core_total = registry.core().len();
    TickerReport {
        ticker: ticker.to_string(),
        name: company.name.clone(),
        status,
        template: template.to_string(),
        error: None,
        fiscal_year_end: company.annual.last().map(|p| p.end.clone()),
        annual_rows: company.annual.len(),
        quarterly_rows: company.quarterly.len(),
        findings,
        core_filled: if company.annual.is_empty() { 0 } else { core_total - latest.core_missing.len() },
        latest,
        seconds: start.elapsed().as_secs_f64(),
        company: if keep_values { Some(company) } else { None },
    }
}

/// The reporting currency, if the total assets have no value in USD.
fn other_currency(facts: &CompanyFacts) -> Option<String> {
    let units = &facts.facts.get("us-gaap")?.get("Assets")?.units;
    if units.contains_key("USD") {
        return None;
    }
    units.keys().find(|u| u.len() == 3 && u.chars().all(|c| c.is_ascii_uppercase())).cloned()
}

/// True for a company that has no revenue now (for example a drug developer before
/// its first product): the latest three fiscal years have no revenue, the latest year
/// has a loss, and no revenue tag without a variable has a value. A free-cash-flow
/// DCF does not fit.
fn no_revenue(registry: &Registry, company: &Company) -> bool {
    let Some(latest) = company.annual.last() else { return false };
    let revenue_tag = |tag: &str| tag.starts_with("us-gaap/Revenue") || tag.contains("Revenues") || tag.contains("SalesRevenue");
    company.annual.iter().rev().take(3).all(|p| p.get(registry, "revenue").unwrap_or(0.0) == 0.0)
        && latest.get(registry, "net_income").map(|v| v < 0.0).unwrap_or(false)
        && !latest.raw_extras.iter().any(|(tag, value)| revenue_tag(tag) && *value > 0.0)
}

/// True if the filer has a fact from one of these forms.
fn has_form(facts: &CompanyFacts, forms: &[&str]) -> bool {
    facts.facts.values().any(|namespace| {
        namespace.values().any(|concept| {
            concept
                .units
                .values()
                .any(|entries| entries.iter().any(|e| e.form.as_deref().map(|f| forms.contains(&f)).unwrap_or(false)))
        })
    })
}

pub fn check_ticker(registry: &Registry, cache: &Path, ticker: &str, n_periods: usize, as_of: i64, keep_values: bool) -> TickerReport {
    let start = Instant::now();
    let Some(path) = facts_path(cache, ticker) else {
        return error_report(ticker, format!("no company_facts file in {}", cache.join(ticker).display()), 0.0);
    };
    match read_facts(&path) {
        Ok(facts) => {
            let mut report = check_cached(registry, cache, ticker, &facts, n_periods, as_of, keep_values);
            report.seconds = start.elapsed().as_secs_f64();
            report
        }
        Err(message) => error_report(ticker, message, start.elapsed().as_secs_f64()),
    }
}

/// All tickers, in parallel. The result has the sequence of the input.
pub fn check_all(registry: &Registry, cache: &Path, tickers: &[String], n_periods: usize, as_of: i64, keep_values: bool) -> Vec<TickerReport> {
    tickers
        .par_iter()
        .map(|t| check_ticker(registry, cache, t, n_periods, as_of, keep_values))
        .collect()
}

/// One JSON line for the report file.
pub fn report_json(report: &TickerReport) -> Value {
    let findings: Vec<Value> = report
        .findings
        .iter()
        .map(|f| json!({"severity": f.severity.label(), "code": f.code, "detail": f.detail}))
        .collect();
    let rollups: Map<String, Value> = report
        .latest
        .rollups
        .iter()
        .map(|r| (r.name.clone(), json!(r.explained.or(r.unexplained.map(|u| 1.0 - u)))))
        .collect();
    json!({
        "ticker": report.ticker,
        "name": report.name,
        "status": report.status.label(),
        "template": report.template,
        "error": report.error,
        "fiscal_year_end": report.fiscal_year_end,
        "annual_rows": report.annual_rows,
        "quarterly_rows": report.quarterly_rows,
        "core_filled": report.core_filled,
        "core_missing": report.latest.core_missing,
        "rollups": rollups,
        "unmapped": report.latest.unmapped.iter().map(|(t, v)| json!([t, v])).collect::<Vec<_>>(),
        "findings": findings,
    })
}

/// The mapped values of all periods, one JSON line for each period (for a comparison
/// of two runs).
pub fn values_json(registry: &Registry, report: &TickerReport) -> Vec<Value> {
    let Some(company) = &report.company else { return Vec::new() };
    let mut lines = Vec::new();
    for period in company.annual.iter().chain(company.quarterly.iter()) {
        let mut values = Map::new();
        let mut sources = Map::new();
        for (i, variable) in registry.variables.iter().enumerate() {
            if let Some(v) = period.values[i] {
                values.insert(variable.name.clone(), json!(v));
            }
            if let Some(s) = &period.sources[i] {
                sources.insert(variable.name.clone(), json!(s));
            }
        }
        lines.push(json!({
            "ticker": report.ticker,
            "form": period.forms.primary(),
            "period_end": period.end,
            "filed_date": period.filed.clone().unwrap_or_else(|| period.end.clone()),
            "values": values,
            "sources": sources,
            "raw_extras": period.raw_extras,
        }));
    }
    lines
}
