//! The text summary of a run over many tickers.

use crate::checks::Severity;
use crate::registry::Registry;
use crate::run::{Status, TickerReport};
use std::collections::BTreeMap;
use std::fmt::Write;

fn median(values: &mut Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(values[values.len() / 2])
}

pub fn summary(registry: &Registry, reports: &[TickerReport], seconds: f64, threads: usize, top: usize) -> String {
    let mut out = String::new();
    let n = reports.len();
    let rate = if seconds > 0.0 { n as f64 / seconds } else { 0.0 };
    let _ = writeln!(out, "mapcheck: {n} tickers in {seconds:.1} s ({rate:.0} tickers/s, {threads} threads)");

    let count = |s: Status| reports.iter().filter(|r| r.status == s).count();
    let _ = writeln!(
        out,
        "status: PASS {} | WARN {} | FAIL {} | N/A (foreign filer, not USD, no 10-K yet, or no revenue) {} | ERROR (not us-gaap) {}",
        count(Status::Pass), count(Status::Warn), count(Status::Fail), count(Status::Template), count(Status::Error)
    );

    let judged: Vec<&TickerReport> = reports.iter().filter(|r| matches!(r.status, Status::Pass | Status::Warn | Status::Fail)).collect();
    if judged.is_empty() {
        return out;
    }

    // Company types.
    let mut types: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    for r in &judged {
        let entry = types.entry(r.template.as_str()).or_default();
        match r.status {
            Status::Pass => entry[0] += 1,
            Status::Warn => entry[1] += 1,
            _ => entry[2] += 1,
        }
    }
    let _ = writeln!(out, "by company type (PASS / WARN / FAIL):");
    for (name, c) in &types {
        let _ = writeln!(out, "  {name:10}{:>5} /{:>5} /{:>5}", c[0], c[1], c[2]);
    }

    // Core inputs.
    let standard: Vec<&&TickerReport> = judged.iter().filter(|r| r.template == "standard").collect();
    let _ = writeln!(out, "\nCore DCF inputs with a value, latest fiscal year, {} companies with the standard template:", standard.len());
    let with_year: Vec<&&TickerReport> = standard.iter().copied().filter(|r| r.fiscal_year_end.is_some()).collect();
    let mut line = String::new();
    for i in registry.core() {
        let name = &registry.variables[i].name;
        let filled = with_year.iter().filter(|r| !r.latest.core_missing.contains(name)).count();
        let share = if with_year.is_empty() { 0.0 } else { 100.0 * filled as f64 / with_year.len() as f64 };
        let _ = write!(line, "  {name} {share:.0}%");
        if line.len() > 90 {
            let _ = writeln!(out, "{line}");
            line.clear();
        }
    }
    if !line.is_empty() {
        let _ = writeln!(out, "{line}");
    }

    // Rollups.
    let _ = writeln!(out, "\nRollups (sum of the parts as a share of the total), latest fiscal year:");
    let _ = writeln!(out, "  {:24}{:>10}{:>10}{:>14}{:>16}", "rollup", "companies", "median", "overlap", "remainder > 10%");
    for rollup in &registry.rollups {
        if rollup.signed {
            continue;
        }
        let mut shares: Vec<f64> = judged
            .iter()
            .filter_map(|r| r.latest.rollups.iter().find(|x| x.name == rollup.name).and_then(|x| x.explained))
            .collect();
        let companies = shares.len();
        let over = shares.iter().filter(|s| **s > 1.02).count();
        let under = shares.iter().filter(|s| **s < 0.90).count();
        if let Some(mid) = median(&mut shares) {
            let _ = writeln!(out, "  {:24}{:>10}{:>9.1}%{:>14}{:>16}", rollup.name, companies, mid * 100.0, over, under);
        }
    }

    // Details that are inside other lines.
    let mut nested: BTreeMap<&str, usize> = BTreeMap::new();
    for r in &judged {
        for (_, variable) in &r.latest.nested {
            *nested.entry(variable.as_str()).or_default() += 1;
        }
    }
    if !nested.is_empty() {
        let mut rows: Vec<(&str, usize)> = nested.into_iter().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let text: Vec<String> = rows.iter().map(|(name, count)| format!("{name} {count}")).collect();
        let _ = writeln!(out, "\nVariables that are a detail inside a different line (number of companies; do not add them to the other lines):\n  {}", text.join(", "));
    }

    // Findings by code.
    let mut by_code: BTreeMap<(std::cmp::Reverse<Severity>, String), Vec<&str>> = BTreeMap::new();
    for r in &judged {
        let mut seen: Vec<&str> = Vec::new();
        for f in &r.findings {
            if !seen.contains(&f.code.as_str()) {
                seen.push(&f.code);
                by_code.entry((std::cmp::Reverse(f.severity), f.code.clone())).or_default().push(&r.ticker);
            }
        }
    }
    let _ = writeln!(out, "\nFindings (number of companies):");
    let mut rows: Vec<(&(std::cmp::Reverse<Severity>, String), &Vec<&str>)> = by_code.iter().collect();
    rows.sort_by(|a, b| a.0 .0.cmp(&b.0 .0).then(b.1.len().cmp(&a.1.len())));
    for ((severity, code), tickers) in rows {
        let sample: Vec<&str> = tickers.iter().take(8).copied().collect();
        let more = if tickers.len() > 8 { " ..." } else { "" };
        let _ = writeln!(out, "  {:4} {:38}{:>5}  {}{}", severity.0.label(), code, tickers.len(), sample.join(" "), more);
    }

    // Tags with no variable.
    let mut tags: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for r in &judged {
        for (tag, _) in &r.latest.unmapped {
            tags.entry(tag.as_str()).or_default().push(&r.ticker);
        }
    }
    let mut tag_rows: Vec<(&str, Vec<&str>)> = tags.into_iter().collect();
    tag_rows.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    if !tag_rows.is_empty() {
        let _ = writeln!(out, "\nMaterial tags with no variable and no exclusion (number of companies), top {top}:");
        for (tag, tickers) in tag_rows.iter().take(top) {
            let sample: Vec<&str> = tickers.iter().take(6).copied().collect();
            let _ = writeln!(out, "  {:>4}  {:72} {}", tickers.len(), tag.trim_start_matches("us-gaap/"), sample.join(" "));
        }
    }

    let errors: Vec<&TickerReport> = reports.iter().filter(|r| r.status == Status::Error).collect();
    if !errors.is_empty() {
        let _ = writeln!(out, "\nNot read ({}):", errors.len());
        for r in errors.iter().take(top) {
            let _ = writeln!(out, "  {:8} {}", r.ticker, r.error.as_deref().unwrap_or(""));
        }
    }
    out
}

/// One line for each ticker.
pub fn listing(reports: &[TickerReport]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{:8}{:6}{:10}{:12}{:>7}{:>6}  findings (FAIL and WARN)", "ticker", "status", "template", "FY end", "annual", "core");
    for r in reports {
        let codes: Vec<String> = r
            .findings
            .iter()
            .filter(|f| f.severity >= Severity::Warn)
            .map(|f| format!("{}{}", if f.severity == Severity::Fail { "!" } else { "" }, f.code))
            .collect();
        let detail = if r.status == Status::Error { r.error.clone().unwrap_or_default() } else { codes.join(" ") };
        let _ = writeln!(
            out,
            "{:8}{:6}{:10}{:12}{:>7}{:>6}  {}",
            r.ticker, r.status.label(), r.template, r.fiscal_year_end.as_deref().unwrap_or("-"), r.annual_rows, r.core_filled, detail
        );
    }
    out
}

/// The full detail for one ticker.
pub fn detail(registry: &Registry, r: &TickerReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n{} {} | {} | template {} | FY end {} | {} annual rows, {} quarterly rows",
        r.ticker, r.name, r.status.label(), r.template, r.fiscal_year_end.as_deref().unwrap_or("-"), r.annual_rows, r.quarterly_rows);
    if let Some(e) = &r.error {
        let _ = writeln!(out, "  ERROR {e}");
    }
    for f in &r.findings {
        let _ = writeln!(out, "  {:4} {:36} {}", f.severity.label(), f.code, f.detail);
    }
    for x in &r.latest.rollups {
        if let Some(e) = x.explained {
            let _ = writeln!(out, "  rollup {:24} parts are {:.1}% of the total", x.name, e * 100.0);
        }
    }
    let _ = registry;
    out
}

/// Each standard variable of the latest fiscal year with its value and its source.
/// `*` marks a core DCF input.
pub fn values_listing(registry: &Registry, r: &TickerReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n{}", "=".repeat(100));
    let _ = writeln!(out, "{}  {}  |  fiscal year end {}  |  template: {}  |  {} annual rows, {} quarterly rows",
        r.ticker, r.name, r.fiscal_year_end.as_deref().unwrap_or("-"), r.template, r.annual_rows, r.quarterly_rows);
    if let Some(e) = &r.error {
        let _ = writeln!(out, "  ERROR {e}");
        return out;
    }
    let Some(period) = r.company.as_ref().and_then(|c| c.annual.last()) else {
        let _ = writeln!(out, "  no annual period");
        return out;
    };
    let mut statement = "";
    let mut filled = 0;
    let mut from_formula = 0;
    for (i, variable) in registry.variables.iter().enumerate() {
        if variable.statement != statement {
            statement = &variable.statement;
            let _ = writeln!(out, "\n  --- {statement} ---");
        }
        let mark = if variable.core { "*" } else { " " };
        let Some(value) = period.values[i] else {
            if variable.core {
                let _ = writeln!(out, "  {mark} {:40} {:>14}", variable.name, "NO VALUE");
            }
            continue;
        };
        filled += 1;
        let source = period.sources[i].as_deref().unwrap_or("");
        if source.starts_with('=') {
            from_formula += 1;
        }
        let shown = match variable.unit.as_str() {
            "USD" => format!("{:>12.3} B", value / 1e9),
            "shares" => format!("{:>12.1} M", value / 1e6),
            _ => format!("{:>12.2}  ", value),
        };
        let _ = writeln!(out, "  {mark} {:40} {shown}  {source}", variable.name);
    }
    let core = registry.core().len();
    let _ = writeln!(out, "\n  Variables with a value: {filled} of {} ({from_formula} from a formula).  Core inputs (*): {} of {core}.",
        registry.variables.len(), r.core_filled);
    for f in &r.findings {
        let _ = writeln!(out, "  {:4} {:36} {}", f.severity.label(), f.code, f.detail);
    }
    out
}
