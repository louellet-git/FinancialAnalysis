//! The checks on the mapped data of one company.
//!
//! Each check answers one question about the mapping:
//! - Did a value get lost?            core_missing, unmapped_lines, material_unmapped
//! - Was a value counted two times?   exceeds_total, formula_not_used, overlap, nested, equal_parts,
//!                                    total_debt_estimates, capex_software_overlap
//! - Do the statements agree?         balance_sheet_identity, rollup_mismatch,
//!                                    change_in_cash, net_income_identity
//! - Are the periods correct?         fiscal_year_spacing, quarters_exceed_year, stale_annual
//! - Can the DCF use the inputs?      core_missing_critical, filled_from_filing, sign, short_history,
//!                                    core_approximate, source_switch, revenue_small,
//!                                    share_count_identity, share_count_scaled
//!
//! Severity: Fail = the mapping is incorrect or the DCF cannot use it. Warn = a person
//! must examine it. Info = a record with no decision.

use crate::dates;
use crate::mapper::{Company, PeriodData};
use crate::registry::Registry;

pub const IDENTITY_TOLERANCE: f64 = 0.005; // share of the total
pub const DOUBLE_COUNT_TOLERANCE: f64 = 0.02; // the parts can exceed the total by this share
pub const UNMAPPED_LINES_SHARE: f64 = 0.10; // remainder of a total with no variable
pub const SIGNED_UNEXPLAINED_SHARE: f64 = 0.25; // same, for cash-flow rollups
pub const MATERIAL_SHARE: f64 = 0.05; // of the smaller of revenue and total assets
pub const TOTAL_DEBT_TOLERANCE: f64 = 0.02;
pub const STALE_DAYS: i64 = 500; // from the latest fiscal year end to the as-of date
pub const SOURCE_SWITCH_CHANGE: f64 = 0.30;
pub const MIN_FCF_YEARS: usize = 4;
pub const SHARE_COUNT_TOLERANCE: f64 = 0.10; // EPS is a rounded number

/// Without these inputs the DCF does not run (a company with the standard template).
pub const CRITICAL: [&str; 6] = [
    "revenue",
    "operating_cash_flow",
    "capex",
    "cash_and_equivalents",
    "total_assets",
    "shares_diluted",
];
/// A bank or an insurer has no capital expenditure and no operating cash flow of the
/// usual meaning; a free-cash-flow DCF does not fit. These inputs must have a value.
pub const CRITICAL_FINANCIAL: [&str; 5] = ["revenue", "net_income", "total_assets", "equity", "shares_diluted"];
/// A REIT buys properties; the capex tags of other companies do not fit.
pub const CRITICAL_REIT: [&str; 5] = ["revenue", "net_income", "operating_cash_flow", "total_assets", "shares_diluted"];

pub fn critical_for(template: &str) -> &'static [&'static str] {
    match template {
        "bank" | "insurer" | "fund" => &CRITICAL_FINANCIAL,
        "reit" => &CRITICAL_REIT,
        _ => &CRITICAL,
    }
}

// A company type is decided from the size of its special lines, not from the
// presence of a tag (a car-parts maker can have a small "deposits" line).
pub const BANK_DEPOSITS_SHARE: f64 = 0.20; // deposits / total assets
pub const INSURER_PREMIUMS_SHARE: f64 = 0.20; // premiums / revenue
pub const INSURER_POLICY_SHARE: f64 = 0.30; // policyholder liabilities / total assets
pub const REIT_PROPERTY_SHARE: f64 = 0.30; // investment property / total assets
pub const LENDER_LOANS_SHARE: f64 = 0.50; // loans and finance receivables / total assets (no current-assets total)
pub const DEPOSIT_TAKER_LOANS_SHARE: f64 = 0.40; // loans / total assets, with an interest expense on deposits

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warn,
    Fail,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Warn => "WARN",
            Severity::Fail => "FAIL",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub code: String,
    pub detail: String,
}

fn finding(severity: Severity, code: impl Into<String>, detail: impl Into<String>) -> Finding {
    Finding { severity, code: code.into(), detail: detail.into() }
}

#[derive(Debug, Clone)]
pub struct RollupResult {
    pub name: String,
    /// Sum of the parts as a share of the total (None for cash-flow rollups).
    pub explained: Option<f64>,
    /// Share of the gross flows with no variable (cash-flow rollups only).
    pub unexplained: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct PeriodChecks {
    pub findings: Vec<Finding>,
    pub rollups: Vec<RollupResult>,
    /// (rollup, variable): the variable is a detail inside a different line of this filer.
    pub nested: Vec<(String, String)>,
    pub core_missing: Vec<String>,
    /// Material tags with no variable and no exclusion: (tag, value), largest first.
    pub unmapped: Vec<(String, f64)>,
}

/// "standard", or "bank" / "insurer" / "fund" / "reit": a business with a different statement
/// structure. The decision uses the values of the fiscal year:
/// - bank: deposits are 20% or more of total assets;
/// - insurer: premiums are 20% or more of revenue, or the policyholder liabilities
///   are 30% or more of total assets; and the balance sheet has no current-assets
///   total (a health insurer has one, and is "standard");
/// - fund: revenue is the total investment income of an investment company, and the
///   balance sheet has no current-assets total;
/// - reit: investment property is 30% or more of total assets.
///
/// Added 2026-11-20 (ticket `T-20261120-dcf-investigation`; banks and REITs that passed as standard):
/// - bank: the deposits can also be `DepositsDomestic` or the interest-bearing plus the non-interest-bearing
///   deposit lines; a company with an interest expense on deposits and loans of 40% or more of total assets is a
///   bank; a lender with no current-assets total and loans or finance receivables of 50% or more of total assets
///   is a bank (its debt is the material of its business, not a financing choice);
/// - reit: the investment property can also be the gross real estate less its accumulated depreciation, and the
///   net investment in sales-type leases of real estate counts as property.
pub fn filer_template(registry: &Registry, p: &PeriodData) -> &'static str {
    let raw = |name: &str| p.raw_extras.get(&format!("us-gaap/{name}")).copied();
    let share = |part: Option<f64>, total: Option<f64>| match (part, total) {
        (Some(part), Some(total)) if total > 0.0 => part / total,
        _ => 0.0,
    };
    let assets = p.get(registry, "total_assets");
    let revenue = p.get(registry, "revenue");
    let split_deposits = match (raw("InterestBearingDepositLiabilities"), raw("NoninterestBearingDepositLiabilities")) {
        (None, None) => None,
        (a, b) => Some(a.unwrap_or(0.0) + b.unwrap_or(0.0)),
    };
    let deposits = [raw("Deposits"), raw("DepositsDomestic"), split_deposits].into_iter().flatten().fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))));
    if share(deposits, assets) >= BANK_DEPOSITS_SHARE {
        return "bank";
    }
    let unclassified = p.get(registry, "total_current_assets").is_none();
    let loans = ["FinancingReceivableExcludingAccruedInterestAfterAllowanceForCreditLoss", "LoansAndLeasesReceivableNetReportedAmount", "NotesReceivableNet", "LoansReceivableNet"]
        .iter()
        .filter_map(|t| raw(t))
        .fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))));
    if raw("InterestExpenseDeposits").is_some_and(|x| x > 0.0) && share(loans, assets) >= DEPOSIT_TAKER_LOANS_SHARE {
        return "bank";
    }
    if unclassified && share(loans, assets) >= LENDER_LOANS_SHARE {
        return "bank";
    }
    if share(raw("PremiumsEarnedNet"), revenue) >= INSURER_PREMIUMS_SHARE && unclassified {
        return "insurer";
    }
    // A life or annuity insurer: its income is fees and investment income, and its
    // liabilities are the accounts of the policyholders.
    let policies: f64 = ["PolicyholderAccountBalance", "LiabilityForFuturePolicyBenefits", "SeparateAccountsLiability"].iter().filter_map(|t| raw(t)).sum();
    if share(Some(policies), assets) >= INSURER_POLICY_SHARE && unclassified {
        return "insurer";
    }
    // An investment company (a business development company): its top line is the
    // total investment income, and it has no capital expenditure.
    if p.source(registry, "revenue") == "= investment_income_operating" && unclassified {
        return "fund";
    }
    let gross_less_depreciation = raw("RealEstateGrossAtCarryingValue").map(|g| g - raw("RealEstateInvestmentPropertyAccumulatedDepreciation").unwrap_or(0.0));
    let property = raw("RealEstateInvestmentPropertyNet").or_else(|| raw("RealEstateInvestmentPropertyAtCost")).or(gross_less_depreciation);
    let leased_property = raw("NetInvestmentInLeaseExcludingAccruedInterestAfterAllowanceForCreditLoss").or_else(|| raw("NetInvestmentInLease"));
    let property = match (property, leased_property) {
        (None, None) => None,
        (a, b) => Some(a.unwrap_or(0.0) + b.unwrap_or(0.0)),
    };
    if share(property, assets) >= REIT_PROPERTY_SHARE {
        return "reit";
    }
    "standard"
}

fn billions(value: f64) -> String {
    format!("{:.3}B", value / 1e9)
}

/// The ways to calculate total debt from the standard variables.
pub fn total_debt_alternatives(registry: &Registry, p: &PeriodData) -> Vec<(&'static str, f64)> {
    let mut out = Vec::new();
    let total_debt = registry.var("total_debt");
    if p.from_tag(total_debt) {
        if let Some(v) = p.values[total_debt] {
            out.push(("tag", v));
        }
    }
    if let Some(ltd) = p.get(registry, "long_term_debt") {
        out.push(("debt_current + long_term_debt", p.get(registry, "debt_current").unwrap_or(0.0) + ltd));
    }
    if let Some(total) = p.get(registry, "long_term_debt_total") {
        out.push((
            "short_term_borrowings + long_term_debt_total",
            p.get(registry, "short_term_borrowings").unwrap_or(0.0) + total,
        ));
    }
    out
}

/// The parts of a rollup that have a value, after the one-line substitutions.
fn rollup_parts(p: &PeriodData, rollup: &crate::registry::Rollup) -> Vec<usize> {
    let mut parts = rollup.parts.clone();
    for (one_line, replaces) in &rollup.combined {
        // The filer shows one line: use it in place of the parts that have no value.
        let all_parts_present = replaces.iter().all(|&i| p.values[i].is_some());
        if p.values[*one_line].is_some() && p.from_tag(*one_line) && !all_parts_present {
            parts.retain(|i| !replaces.contains(i));
            parts.push(*one_line);
        }
    }
    parts.retain(|&i| p.values[i].is_some());
    parts
}

/// All checks on one annual period of a company with the standard template.
pub fn check_period(registry: &Registry, p: &PeriodData) -> PeriodChecks {
    check_period_as(registry, p, "standard")
}

/// All checks on one annual period. `template` selects the inputs that must have a value.
pub fn check_period_as(registry: &Registry, p: &PeriodData, template: &str) -> PeriodChecks {
    let mut out = PeriodChecks::default();
    let get = |name: &str| p.get(registry, name);
    let critical = critical_for(template);

    // --- Can the DCF use the inputs? ---
    let mut critical_missing: Vec<&str> = critical.iter().copied().filter(|n| get(n).is_none()).collect();
    if get("shares_diluted").is_none() && get("shares_outstanding").is_some() {
        critical_missing.retain(|n| *n != "shares_diluted");
    }
    if !critical_missing.is_empty() {
        out.findings.push(finding(Severity::Fail, "core_missing_critical", critical_missing.join(", ")));
    }
    // A formula value that is larger than its total is not used (mapper.rs).
    for (variable, source) in registry.variables.iter().zip(&p.sources) {
        if let Some(source) = source.as_deref().filter(|s| s.contains(crate::mapper::NOT_USED)) {
            out.findings.push(finding(Severity::Warn, "formula_not_used", format!("{} has no value [{}]", variable.name, source)));
        }
    }
    // A value from the 10-K is an estimate from the lines or the segments of the filing.
    for (name, source) in p.from_filing(registry) {
        let value = get(name).unwrap_or(0.0);
        let shown = if registry.try_var(name).map(|i| registry.variables[i].unit == "shares").unwrap_or(false) {
            format!("{:.1}M", value / 1e6)
        } else {
            billions(value)
        };
        out.findings.push(finding(Severity::Warn, "filled_from_filing", format!("{name} {shown} [{source}]")));
    }
    for i in registry.core() {
        if p.values[i].is_none() {
            out.core_missing.push(registry.variables[i].name.clone());
        }
    }
    let other_missing: Vec<&str> = out
        .core_missing
        .iter()
        .map(String::as_str)
        .filter(|n| !critical.contains(n))
        .collect();
    // The other core inputs are those of a free-cash-flow DCF: only for the standard template.
    if template == "standard" && !other_missing.is_empty() {
        out.findings.push(finding(Severity::Warn, "core_missing", other_missing.join(", ")));
    }
    let approximate: Vec<String> = registry
        .core()
        .into_iter()
        .filter(|&i| match &p.sources[i] {
            // A formula with one term is a fallback to a different number; a formula
            // with an optional term that has no value can be too small.
            // Contract revenue is the full revenue of most companies: not a fallback.
            Some(s) => s.starts_with('=') && s != "= revenue_from_contracts" && (s.contains("no value:") || !(s.contains(" + ") || s.contains(" - "))),
            None => false,
        })
        .map(|i| format!("{} [{}]", registry.variables[i].name, p.sources[i].as_deref().unwrap_or("")))
        .collect();
    if !approximate.is_empty() {
        out.findings.push(finding(Severity::Info, "core_approximate", approximate.join("; ")));
    }

    // Revenue from the lease-income tag or the investment-income tag is correct only
    // for a landlord or an investment company. For other companies it is a small part.
    if let (Some(revenue), Some(income)) = (get("revenue"), get("net_income")) {
        let source = p.source(registry, "revenue");
        if (source == "= lease_income" || source == "= investment_income_operating") && revenue.abs() < income.abs() {
            out.findings.push(finding(
                Severity::Warn,
                "revenue_small",
                format!("revenue {} [{}] is smaller than net income {}: it can be a part of the revenue", billions(revenue), source, billions(income)),
            ));
        }
    }

    let mut bad_signs = Vec::new();
    for name in ["total_assets", "shares_diluted"] {
        if let Some(v) = get(name) {
            if v <= 0.0 {
                bad_signs.push(format!("{name} = {v}"));
            }
        }
    }
    if let Some(v) = get("revenue") {
        if v < 0.0 {
            bad_signs.push(format!("revenue = {}", billions(v)));
        } else if v == 0.0 {
            out.findings.push(finding(Severity::Warn, "no_revenue", "revenue = 0 (a company with no sales: the DCF has no revenue base)"));
        }
    }
    for name in ["cash_and_equivalents", "total_debt", "sbc_total"] {
        if let Some(v) = get(name) {
            if v < 0.0 {
                bad_signs.push(format!("{name} = {}", billions(v)));
            }
        }
    }
    // For a financial company the depreciation tag can be a net number that includes
    // the accretion of discounts on investments, and it can be negative.
    if let Some(v) = get("da_total") {
        if v < 0.0 {
            out.findings.push(finding(
                Severity::Warn,
                "sign_unusual",
                format!("da_total = {} [{}]: not depreciation of equipment only", billions(v), p.source(registry, "da_total")),
            ));
        }
    }
    if !bad_signs.is_empty() {
        out.findings.push(finding(Severity::Fail, "sign", bad_signs.join(", ")));
    }
    if let Some(v) = get("interest_expense") {
        if v < 0.0 {
            out.findings.push(finding(
                Severity::Warn,
                "sign_unusual",
                format!("interest_expense = {} (the filer reports the expense as a negative number)", billions(v)),
            ));
        }
    }

    // Capital expenditure for free cash flow is capex + capitalized software
    // (PLANNING/RULES.md R2). A "productive assets" tag can already include software.
    if let (Some(software), Some(capex)) = (get("capitalized_software"), get("capex")) {
        if software != 0.0 && p.source(registry, "capex").contains("ProductiveAssets") {
            out.findings.push(finding(
                Severity::Warn,
                "capex_software_overlap",
                format!(
                    "capex {} [{}] can already include the capitalized software {}; capex_total adds the two",
                    billions(capex),
                    p.source(registry, "capex"),
                    billions(software)
                ),
            ));
        }
    }

    // The share count against net income / EPS.
    if let Some(i) = registry.try_var("shares_diluted") {
        if let Some(source) = &p.sources[i] {
            if source.contains("scale from net income / EPS") {
                out.findings.push(finding(Severity::Info, "share_count_scaled", source.clone()));
            }
        }
        if let (Some(shares), Some(implied)) = (p.values[i], crate::mapper::implied_share_count(registry, &p.values, "eps_diluted")) {
            if shares > 0.0 && ((shares - implied) / shares).abs() > SHARE_COUNT_TOLERANCE {
                out.findings.push(finding(
                    Severity::Warn,
                    "share_count_identity",
                    format!("shares_diluted {:.1}M != net income / eps_diluted {:.1}M", shares / 1e6, implied / 1e6),
                ));
            }
        }
    }

    // --- Do the statements agree? ---
    if let (Some(assets), Some(le)) = (get("total_assets"), get("total_liabilities_and_equity")) {
        if assets != 0.0 && ((assets - le) / assets).abs() > IDENTITY_TOLERANCE {
            out.findings.push(finding(
                Severity::Fail,
                "balance_sheet_identity",
                format!("total_assets {} != total_liabilities_and_equity {}", billions(assets), billions(le)),
            ));
        }
    }

    // --- Rollups: was a value lost, or counted two times? ---
    for rollup in &registry.rollups {
        let Some(total) = p.values[rollup.total] else { continue };
        let parts = rollup_parts(p, rollup);
        if parts.is_empty() || total == 0.0 {
            continue;
        }
        let sum: f64 = parts.iter().map(|&i| p.values[i].unwrap()).sum();

        // Two different lines with the same value: one item with two tags?
        let lines: Vec<usize> = parts
            .iter()
            .copied()
            .chain(rollup.details.iter().copied().filter(|&i| p.values[i].is_some()))
            .collect();
        for (a, &i) in lines.iter().enumerate() {
            for &j in &lines[a + 1..] {
                let (x, y) = (p.values[i].unwrap(), p.values[j].unwrap());
                if x != 0.0 && x == y && p.from_tag(i) && p.from_tag(j) {
                    out.findings.push(finding(
                        Severity::Warn,
                        format!("equal_parts:{}", rollup.name),
                        format!("{} and {} have the same value {}", registry.variables[i].name, registry.variables[j].name, billions(x)),
                    ));
                }
            }
        }

        if rollup.signed {
            let residual = total - sum;
            let gross: f64 = parts.iter().map(|&i| p.values[i].unwrap().abs()).sum::<f64>() + residual.abs();
            let unexplained = if gross > 0.0 { residual.abs() / gross } else { 0.0 };
            out.rollups.push(RollupResult { name: rollup.name.clone(), explained: None, unexplained: Some(unexplained) });
            if rollup.exact {
                let base = parts.iter().map(|&i| p.values[i].unwrap().abs()).fold(total.abs(), f64::max);
                if base > 0.0 && residual.abs() / base > IDENTITY_TOLERANCE * 2.0 {
                    out.findings.push(finding(
                        Severity::Warn,
                        rollup.name.clone(),
                        format!("total {} != sum of the parts {} (difference {})", billions(total), billions(sum), billions(residual)),
                    ));
                }
            } else if unexplained > SIGNED_UNEXPLAINED_SHARE {
                out.findings.push(finding(
                    Severity::Warn,
                    format!("unmapped_lines:{}", rollup.name),
                    format!("{:.0}% of the gross flows have no variable (remainder {})", unexplained * 100.0, billions(residual)),
                ));
            }
            continue;
        }

        if rollup.exact {
            let explained = sum / total;
            out.rollups.push(RollupResult { name: rollup.name.clone(), explained: Some(explained), unexplained: None });
            if (explained - 1.0).abs() > IDENTITY_TOLERANCE {
                // The totals can agree (see balance_sheet_identity) while a line between
                // them has no variable, or one part has an incorrect fact.
                out.findings.push(finding(
                    Severity::Warn,
                    format!("rollup_mismatch:{}", rollup.name),
                    format!("total {} != sum of the parts {} (difference {})", billions(total), billions(sum), billions(total - sum)),
                ));
            }
            continue;
        }

        let limit = total * (1.0 + DOUBLE_COUNT_TOLERANCE);
        let mut details: Vec<usize> = rollup.details.iter().copied().filter(|&i| p.values[i].is_some()).collect();

        // One line larger than its total: the mapping is incorrect.
        if total > 0.0 {
            for &i in parts.iter().chain(details.iter()) {
                let v = p.values[i].unwrap();
                if v > limit {
                    out.findings.push(finding(
                        Severity::Fail,
                        format!("exceeds_total:{}", rollup.name),
                        format!("{} {} > {} {}", registry.variables[i].name, billions(v), registry.variables[rollup.total].name, billions(total)),
                    ));
                }
            }
        }

        let mut explained = sum / total;
        if total > 0.0 && sum > limit {
            // Lines that are usually separate are together larger than the total:
            // two of them overlap for this filer.
            let names: Vec<String> = parts
                .iter()
                .map(|&i| format!("{} {}", registry.variables[i].name, billions(p.values[i].unwrap())))
                .collect();
            out.findings.push(finding(
                Severity::Warn,
                format!("overlap:{}", rollup.name),
                format!("the parts are {:.1}% of the total {}: {}", explained * 100.0, billions(total), names.join(", ")),
            ));
        } else if total > 0.0 {
            // Add each detail only if the sum stays in the total (largest first).
            details.sort_by(|a, b| p.values[*b].partial_cmp(&p.values[*a]).unwrap_or(std::cmp::Ordering::Equal));
            let mut running = sum;
            let mut nested: Vec<&str> = Vec::new();
            for i in details {
                let v = p.values[i].unwrap();
                if running + v <= limit {
                    running += v;
                } else {
                    nested.push(registry.variables[i].name.as_str());
                    out.nested.push((rollup.name.clone(), registry.variables[i].name.clone()));
                }
            }
            if !nested.is_empty() {
                out.findings.push(finding(
                    Severity::Info,
                    format!("nested:{}", rollup.name),
                    format!("{} are inside other lines of this filer; do not add them to the other lines", nested.join(", ")),
                ));
            }
            explained = running / total;
            if explained < 1.0 - UNMAPPED_LINES_SHARE {
                out.findings.push(finding(
                    Severity::Warn,
                    format!("unmapped_lines:{}", rollup.name),
                    format!("{:.0}% of the total {} has no variable", (1.0 - explained) * 100.0, billions(total)),
                ));
            }
        }
        out.rollups.push(RollupResult { name: rollup.name.clone(), explained: Some(explained), unexplained: None });
    }

    // A variable larger than a total that contains it.
    for &(part, total) in &registry.within {
        if let (Some(v), Some(t)) = (p.values[part], p.values[total]) {
            if t > 0.0 && v > t * (1.0 + DOUBLE_COUNT_TOLERANCE) {
                out.findings.push(finding(
                    Severity::Fail,
                    "exceeds_total",
                    format!("{} {} > {} {} [{}]", registry.variables[part].name, billions(v), registry.variables[total].name, billions(t), p.sources[part].as_deref().unwrap_or("")),
                ));
            }
        }
    }

    let alternatives = total_debt_alternatives(registry, p);
    if alternatives.len() > 1 {
        let low = alternatives.iter().map(|a| a.1).fold(f64::INFINITY, f64::min);
        let high = alternatives.iter().map(|a| a.1).fold(f64::NEG_INFINITY, f64::max);
        if high > 0.0 && (high - low) / high > TOTAL_DEBT_TOLERANCE {
            let text: Vec<String> = alternatives.iter().map(|(k, v)| format!("{k}: {}", billions(*v))).collect();
            out.findings.push(finding(Severity::Warn, "total_debt_estimates", text.join(" | ")));
        }
    }

    let nci = registry.var("net_income_incl_nci");
    if p.from_tag(nci) {
        if let (Some(ni), Some(pretax), Some(tax), Some(revenue)) =
            (p.values[nci], get("income_before_tax"), get("income_tax_expense"), get("revenue"))
        {
            let before_equity_method = p
                .source(registry, "income_before_tax")
                .contains("MinorityInterestAndIncomeLossFromEquityMethodInvestments");
            let expected = pretax - tax
                + if before_equity_method { get("equity_method_income").unwrap_or(0.0) } else { 0.0 }
                + get("discontinued_operations_income").unwrap_or(0.0);
            if revenue != 0.0 && ((ni - expected) / revenue).abs() > IDENTITY_TOLERANCE {
                out.findings.push(finding(
                    Severity::Warn,
                    "net_income_identity",
                    format!("net_income_incl_nci {} != pre-tax income less tax {}", billions(ni), billions(expected)),
                ));
            }
        }
    }

    // --- Material tags with no variable ---
    let bases: Vec<f64> = [get("revenue"), get("total_assets")].iter().flatten().map(|v| v.abs()).filter(|v| *v > 0.0).collect();
    if let Some(base) = bases.iter().cloned().reduce(f64::min) {
        let threshold = MATERIAL_SHARE * base;
        for (tag, value) in &p.raw_extras {
            if value.abs() >= threshold && registry.exclusion_reason(tag).is_none() {
                out.unmapped.push((tag.clone(), *value));
            }
        }
        out.unmapped.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
        if !out.unmapped.is_empty() {
            let top: Vec<String> = out.unmapped.iter().take(3).map(|(t, v)| format!("{} {}", t.trim_start_matches("us-gaap/"), billions(*v))).collect();
            out.findings.push(finding(
                Severity::Info,
                "material_unmapped",
                format!("{} tags; largest: {}", out.unmapped.len(), top.join(", ")),
            ));
        }
    }
    out
}

/// Checks on the company as a whole (period sequence, history, freshness).
pub fn check_company(registry: &Registry, company: &Company, as_of: i64) -> Vec<Finding> {
    check_company_as(registry, company, as_of, "standard")
}

pub fn check_company_as(registry: &Registry, company: &Company, as_of: i64, template: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let Some(latest) = company.annual.last() else {
        out.push(finding(Severity::Fail, "no_annual_period", "no fiscal year with an anchor fact in a 10-K"));
        return out;
    };

    if let Some(end) = dates::parse(&latest.end) {
        if as_of - end > STALE_DAYS {
            out.push(finding(
                Severity::Warn,
                "stale_annual",
                format!("the latest fiscal year ends {} ({} days before the as-of date)", latest.end, as_of - end),
            ));
        }
    }

    let mut gaps = Vec::new();
    for pair in company.annual.windows(2) {
        if let (Some(a), Some(b)) = (dates::parse(&pair[0].end), dates::parse(&pair[1].end)) {
            if !(350..=380).contains(&(b - a)) {
                gaps.push(format!("{} -> {} ({} days)", pair[0].end, pair[1].end, b - a));
            }
        }
    }
    if !gaps.is_empty() {
        out.push(finding(Severity::Warn, "fiscal_year_spacing", gaps.join(", ")));
    }

    let fcf_years = company
        .annual
        .iter()
        .filter(|p| p.get(registry, "revenue").is_some() && p.get(registry, "operating_cash_flow").is_some() && p.get(registry, "capex").is_some())
        .count();
    if template == "standard" && fcf_years < MIN_FCF_YEARS {
        out.push(finding(
            Severity::Warn,
            "short_history",
            format!("{fcf_years} fiscal years have revenue, operating cash flow and capex (the classic engine needs {MIN_FCF_YEARS})"),
        ));
    }

    // The source tag of revenue changes and the value jumps: possibly a tag with a different scope.
    if company.annual.len() >= 2 {
        let prev = &company.annual[company.annual.len() - 2];
        let revenue = registry.var("revenue");
        if let (Some(now), Some(before)) = (latest.values[revenue], prev.values[revenue]) {
            let switched = latest.sources[revenue] != prev.sources[revenue];
            if switched && before != 0.0 && ((now - before) / before).abs() > SOURCE_SWITCH_CHANGE {
                out.push(finding(
                    Severity::Warn,
                    "source_switch",
                    format!(
                        "revenue {} [{}] after {} [{}]",
                        billions(now),
                        latest.sources[revenue].as_deref().unwrap_or(""),
                        billions(before),
                        prev.sources[revenue].as_deref().unwrap_or("")
                    ),
                ));
            }
        }
    }

    // The quarters in the latest fiscal year must not be larger than the year.
    if let (Some(fy_end), Some(fy_revenue)) = (dates::parse(&latest.end), latest.get(registry, "revenue")) {
        let in_year: Vec<f64> = company
            .quarterly
            .iter()
            .filter(|q| dates::parse(&q.end).map(|d| d < fy_end && fy_end - d < 300).unwrap_or(false))
            .filter_map(|q| q.get(registry, "revenue"))
            .collect();
        let sum: f64 = in_year.iter().sum();
        if in_year.len() == 3 && fy_revenue > 0.0 && sum > fy_revenue * 1.02 {
            out.push(finding(
                Severity::Fail,
                "quarters_exceed_year",
                format!("revenue of 3 quarters {} > revenue of the fiscal year {}", billions(sum), billions(fy_revenue)),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::testing::{company, fact};
    use crate::facts::Forms;
    use crate::mapper::{map_company, resolve_period};

    const END: &str = "2024-12-31";

    fn flow(val: f64) -> Vec<crate::facts::Fact> {
        vec![fact(Some("2024-01-01"), END, val, "10-K", "2025-02-01")]
    }

    fn balance(val: f64) -> Vec<crate::facts::Fact> {
        vec![fact(None, END, val, "10-K", "2025-02-01")]
    }

    fn with_shares(mut facts: crate::facts::CompanyFacts) -> crate::facts::CompanyFacts {
        // Shares are in the unit "shares", not USD.
        let mut units = std::collections::HashMap::new();
        units.insert("shares".to_string(), flow(100.0));
        facts.facts.get_mut("us-gaap").unwrap().insert(
            "WeightedAverageNumberOfDilutedSharesOutstanding".to_string(),
            crate::facts::Concept { units },
        );
        facts
    }

    fn checks(concepts: Vec<(&str, Vec<crate::facts::Fact>)>) -> PeriodChecks {
        let registry = Registry::embedded().unwrap();
        let facts = with_shares(company(concepts));
        check_period(&registry, &resolve_period(&registry, &facts, END, Forms::Annual))
    }

    fn codes(c: &PeriodChecks, severity: Severity) -> Vec<String> {
        c.findings.iter().filter(|f| f.severity == severity).map(|f| f.code.clone()).collect()
    }

    /// A complete, consistent company: no Fail and no Warn.
    fn good() -> Vec<(&'static str, Vec<crate::facts::Fact>)> {
        vec![
            ("Revenues", flow(1000.0)),
            ("NetCashProvidedByUsedInOperatingActivities", flow(200.0)),
            ("PaymentsToAcquirePropertyPlantAndEquipment", flow(50.0)),
            ("InterestExpense", flow(10.0)),
            ("IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest", flow(150.0)),
            ("IncomeTaxExpenseBenefit", flow(30.0)),
            ("NetIncomeLoss", flow(120.0)),
            ("DepreciationDepletionAndAmortization", flow(40.0)),
            ("ShareBasedCompensation", flow(20.0)),
            ("CashAndCashEquivalentsAtCarryingValue", balance(300.0)),
            ("AccountsReceivableNetCurrent", balance(200.0)),
            ("AssetsCurrent", balance(500.0)),
            ("PropertyPlantAndEquipmentNet", balance(1000.0)),
            ("Assets", balance(1500.0)),
            ("AccountsPayableCurrent", balance(150.0)),
            ("LongTermDebtCurrent", balance(50.0)),
            ("LiabilitiesCurrent", balance(200.0)),
            ("LongTermDebtNoncurrent", balance(400.0)),
            ("Liabilities", balance(600.0)),
            ("StockholdersEquity", balance(900.0)),
            ("LiabilitiesAndStockholdersEquity", balance(1500.0)),
        ]
    }

    fn with(extra: Vec<(&'static str, Vec<crate::facts::Fact>)>) -> Vec<(&'static str, Vec<crate::facts::Fact>)> {
        let mut all: Vec<_> = good().into_iter().filter(|(n, _)| !extra.iter().any(|(m, _)| m == n)).collect();
        all.extend(extra);
        all
    }

    #[test]
    fn consistent_company_has_no_fail_and_no_warn() {
        let c = checks(good());
        assert_eq!(codes(&c, Severity::Fail), Vec::<String>::new(), "{:?}", c.findings);
        assert_eq!(codes(&c, Severity::Warn), Vec::<String>::new(), "{:?}", c.findings);
        assert!(c.core_missing.is_empty());
    }

    #[test]
    fn missing_critical_input_is_a_fail() {
        let all: Vec<_> = good().into_iter().filter(|(n, _)| *n != "NetCashProvidedByUsedInOperatingActivities").collect();
        let c = checks(all);
        let fails = c.findings.iter().find(|f| f.code == "core_missing_critical").unwrap();
        assert!(fails.detail.contains("operating_cash_flow"));
    }

    #[test]
    fn one_line_larger_than_its_total_is_a_fail() {
        let c = checks(with(vec![("InventoryNet", balance(600.0))]));
        assert!(codes(&c, Severity::Fail).contains(&"exceeds_total:current_assets".to_string()), "{:?}", c.findings);
    }

    #[test]
    fn separate_lines_larger_than_the_total_are_an_overlap() {
        // cash 300 + receivables 200 + short-term investments 100 > current assets 500
        let c = checks(with(vec![("ShortTermInvestments", balance(100.0))]));
        assert!(codes(&c, Severity::Warn).contains(&"overlap:current_assets".to_string()), "{:?}", c.findings);
        assert!(codes(&c, Severity::Fail).is_empty(), "{:?}", c.findings);
    }

    #[test]
    fn detail_that_does_not_fit_is_nested_and_is_not_a_fail() {
        // cash 300 + receivables 200 = current assets 500: "prepaid and other" 150 is inside a line.
        let c = checks(with(vec![("PrepaidExpenseAndOtherAssetsCurrent", balance(150.0))]));
        assert_eq!(c.nested, vec![("current_assets".to_string(), "prepaid_and_other_current_assets".to_string())]);
        assert!(codes(&c, Severity::Fail).is_empty() && codes(&c, Severity::Warn).is_empty(), "{:?}", c.findings);
    }

    #[test]
    fn variable_larger_than_a_total_that_contains_it_is_a_fail() {
        // total debt 50 + 700 = 750 > total liabilities 600
        let c = checks(with(vec![("LongTermDebtNoncurrent", balance(700.0))]));
        assert!(codes(&c, Severity::Fail).iter().any(|code| code.starts_with("exceeds_total")), "{:?}", c.findings);
    }

    #[test]
    fn large_remainder_shows_lines_with_no_variable() {
        let c = checks(with(vec![("AssetsCurrent", balance(900.0)), ("Assets", balance(1900.0)), ("StockholdersEquity", balance(1300.0)), ("LiabilitiesAndStockholdersEquity", balance(1900.0))]));
        assert!(codes(&c, Severity::Warn).contains(&"unmapped_lines:current_assets".to_string()), "{:?}", c.findings);
    }

    #[test]
    fn balance_sheet_that_does_not_balance_is_a_fail() {
        let c = checks(with(vec![("LiabilitiesAndStockholdersEquity", balance(1400.0))]));
        assert!(codes(&c, Severity::Fail).contains(&"balance_sheet_identity".to_string()), "{:?}", c.findings);
        assert!(codes(&c, Severity::Warn).contains(&"rollup_mismatch:liabilities_and_equity".to_string()), "{:?}", c.findings);
    }

    #[test]
    fn two_parts_with_the_same_value_are_reported() {
        let c = checks(with(vec![("InventoryNet", balance(200.0)), ("AssetsCurrent", balance(700.0)), ("Assets", balance(1700.0)), ("StockholdersEquity", balance(1100.0)), ("LiabilitiesAndStockholdersEquity", balance(1700.0))]));
        assert!(codes(&c, Severity::Warn).iter().any(|code| code == "equal_parts:current_assets"), "{:?}", c.findings);
    }

    #[test]
    fn total_debt_estimates_that_disagree_are_reported() {
        let c = checks(with(vec![("DebtCurrent", balance(150.0)), ("LongTermDebt", balance(450.0))]));
        assert!(codes(&c, Severity::Warn).contains(&"total_debt_estimates".to_string()), "{:?}", c.findings);
    }

    #[test]
    fn value_from_the_filing_and_formula_not_used_give_warnings() {
        let registry = Registry::embedded().unwrap();
        let facts = company(vec![
            ("Revenues", flow(1000.0)),
            ("CashAndCashEquivalentsAtCarryingValue", balance(2336.0)),
            ("ShortTermInvestments", balance(2000.0)),
            ("AssetsCurrent", balance(2459.0)),
        ]);
        let fill = |_: &str, name: &str| if name == "capex" { Some((0.0, "= 0 (filing 0000000001-25-000001: no line)".to_string())) } else { None };
        let p = crate::mapper::resolve_period_with(&registry, &facts, "2024-12-31", crate::facts::Forms::Annual, Some(&fill));
        let warnings = codes(&check_period(&registry, &p), Severity::Warn);
        assert!(warnings.contains(&"filled_from_filing".to_string()), "{warnings:?}");
        assert!(warnings.contains(&"formula_not_used".to_string()), "{warnings:?}");
        assert!(!codes(&check_period(&registry, &p), Severity::Fail).contains(&"exceeds_total".to_string()));
    }

    #[test]
    fn investment_company_is_a_fund_and_capex_is_not_critical() {
        let registry = Registry::embedded().unwrap();
        let p = resolve_period(&registry, &company(vec![("GrossInvestmentIncomeOperating", flow(3052.0)), ("Assets", balance(30000.0))]), "2024-12-31", crate::facts::Forms::Annual);
        assert_eq!(filer_template(&registry, &p), "fund");
        assert!(!critical_for("fund").contains(&"capex"));
    }

    #[test]
    fn capex_total_adds_software_and_a_broad_capex_tag_gives_a_warning() {
        let registry = Registry::embedded().unwrap();
        let facts = with_shares(company(with(vec![("PaymentsToDevelopSoftware", flow(30.0))])));
        let p = resolve_period(&registry, &facts, END, Forms::Annual);
        assert_eq!(p.get(&registry, "capex_total"), Some(-80.0));     // equipment 50 + software 30
        assert!(!codes(&check_period(&registry, &p), Severity::Warn).contains(&"capex_software_overlap".to_string()));

        let broad: Vec<_> = with(vec![("PaymentsToDevelopSoftware", flow(30.0)), ("PaymentsToAcquireProductiveAssets", flow(50.0))])
            .into_iter()
            .filter(|(n, _)| *n != "PaymentsToAcquirePropertyPlantAndEquipment")
            .collect();
        let c = checks(broad);
        assert!(codes(&c, Severity::Warn).contains(&"capex_software_overlap".to_string()), "{:?}", c.findings);
    }

    #[test]
    fn material_tag_with_no_variable_is_listed_and_an_excluded_tag_is_not() {
        let c = checks(with(vec![("AssetRetirementObligation", balance(300.0)), ("ComprehensiveIncomeNetOfTax", flow(400.0))]));
        assert_eq!(c.unmapped, vec![("us-gaap/AssetRetirementObligation".to_string(), 300.0)]);
    }

    #[test]
    fn cash_flow_statement_that_does_not_sum_is_reported() {
        let c = checks(with(vec![
            ("NetCashProvidedByUsedInInvestingActivities", flow(-50.0)),
            ("NetCashProvidedByUsedInFinancingActivities", flow(-100.0)),
            ("CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalentsPeriodIncreaseDecreaseIncludingExchangeRateEffect", flow(10.0)),
        ]));
        assert!(codes(&c, Severity::Warn).contains(&"change_in_cash".to_string()), "{:?}", c.findings);
    }

    #[test]
    fn company_checks_find_a_missing_year_and_a_stale_year() {
        let registry = Registry::embedded().unwrap();
        let revenue = vec![
            fact(Some("2020-01-01"), "2020-12-31", 1.0, "10-K", "2021-02-01"),
            fact(Some("2022-01-01"), "2022-12-31", 1.0, "10-K", "2023-02-01"),
        ];
        let mapped = map_company(&registry, &company(vec![("Revenues", revenue)]), 12);
        let found = check_company(&registry, &mapped, dates::parse("2026-11-19").unwrap());
        let codes: Vec<&str> = found.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"fiscal_year_spacing"), "{codes:?}");
        assert!(codes.contains(&"stale_annual"), "{codes:?}");
        assert!(codes.contains(&"short_history"), "{codes:?}");
    }

    #[test]
    fn quarters_larger_than_the_year_are_a_fail() {
        let registry = Registry::embedded().unwrap();
        let revenue = vec![
            fact(Some("2024-01-01"), END, 100.0, "10-K", "2025-02-01"),
            fact(Some("2024-01-01"), "2024-03-31", 40.0, "10-Q", "2024-05-01"),
            fact(Some("2024-04-01"), "2024-06-30", 40.0, "10-Q", "2024-08-01"),
            fact(Some("2024-07-01"), "2024-09-30", 40.0, "10-Q", "2024-11-01"),
        ];
        let mapped = map_company(&registry, &company(vec![("Revenues", revenue)]), 12);
        let found = check_company(&registry, &mapped, dates::parse("2025-03-01").unwrap());
        assert!(found.iter().any(|f| f.code == "quarters_exceed_year" && f.severity == Severity::Fail));
    }

    #[test]
    fn company_type_comes_from_the_size_of_the_special_lines() {
        let registry = Registry::embedded().unwrap();
        let template = |concepts: Vec<(&'static str, Vec<crate::facts::Fact>)>| {
            let facts = with_shares(company(concepts));
            filer_template(&registry, &resolve_period(&registry, &facts, END, Forms::Annual))
        };
        // A small deposits line (a car-parts maker): standard.
        assert_eq!(template(with(vec![("Deposits", balance(30.0))])), "standard");
        // Deposits are 60% of total assets: a bank.
        assert_eq!(template(with(vec![("Deposits", balance(900.0))])), "bank");
        // Premiums are most of revenue, but the balance sheet has current assets (a health insurer): standard.
        assert_eq!(template(with(vec![("PremiumsEarnedNet", flow(800.0))])), "standard");
        // The same with no current-assets total: an insurer.
        let unclassified: Vec<_> = with(vec![("PremiumsEarnedNet", flow(800.0))]).into_iter().filter(|(n, _)| *n != "AssetsCurrent").collect();
        assert_eq!(template(unclassified), "insurer");
        assert_eq!(template(with(vec![("RealEstateInvestmentPropertyNet", balance(900.0))])), "reit");
    }

    #[test]
    fn banks_lenders_and_reits_with_other_tags_are_not_standard() {
        let registry = Registry::embedded().unwrap();
        let template = |concepts: Vec<(&'static str, Vec<crate::facts::Fact>)>| {
            let facts = with_shares(company(concepts));
            filer_template(&registry, &resolve_period(&registry, &facts, END, Forms::Annual))
        };
        let unclassified = |v: Vec<(&'static str, Vec<crate::facts::Fact>)>| -> Vec<(&'static str, Vec<crate::facts::Fact>)> { with(v).into_iter().filter(|(n, _)| *n != "AssetsCurrent").collect() };
        // domestic deposits (PNFP) and split deposit lines
        assert_eq!(template(with(vec![("DepositsDomestic", balance(900.0))])), "bank");
        assert_eq!(template(with(vec![("InterestBearingDepositLiabilities", balance(500.0)), ("NoninterestBearingDepositLiabilities", balance(200.0))])), "bank");
        // an interest expense on deposits and a loan book of 40% of assets, no deposits line (GBCI)
        assert_eq!(template(with(vec![("InterestExpenseDeposits", flow(20.0)), ("FinancingReceivableExcludingAccruedInterestAfterAllowanceForCreditLoss", balance(700.0))])), "bank");
        // a lender with no current-assets total (OMF); the same loans with current assets (a maker with a finance arm) stay standard
        assert_eq!(template(unclassified(vec![("NotesReceivableNet", balance(900.0))])), "bank");
        assert_eq!(template(with(vec![("NotesReceivableNet", balance(900.0))])), "standard");
        // real estate as gross value less depreciation (CPT) and as sales-type leases (VICI)
        assert_eq!(template(with(vec![("RealEstateGrossAtCarryingValue", balance(1400.0)), ("RealEstateInvestmentPropertyAccumulatedDepreciation", balance(500.0))])), "reit");
        assert_eq!(template(with(vec![("NetInvestmentInLeaseExcludingAccruedInterestAfterAllowanceForCreditLoss", balance(800.0))])), "reit");
        // small lines stay standard
        assert_eq!(template(with(vec![("DepositsDomestic", balance(30.0)), ("NetInvestmentInLease", balance(100.0))])), "standard");
    }

    #[test]
    fn a_bank_is_judged_on_its_own_inputs() {
        let registry = Registry::embedded().unwrap();
        // No capex and no operating cash flow, but revenue, net income, assets, equity and shares.
        let facts = with_shares(company(vec![
            ("InterestIncomeExpenseNet", flow(60.0)),
            ("NoninterestIncome", flow(40.0)),
            ("NetIncomeLoss", flow(20.0)),
            ("Assets", balance(1500.0)),
            ("StockholdersEquity", balance(150.0)),
            ("Deposits", balance(1000.0)),
        ]));
        let p = resolve_period(&registry, &facts, END, Forms::Annual);
        assert_eq!(p.get(&registry, "revenue"), Some(100.0)); // net interest income + noninterest income
        assert_eq!(filer_template(&registry, &p), "bank");
        assert!(codes(&check_period_as(&registry, &p, "bank"), Severity::Fail).is_empty());
        assert!(codes(&check_period_as(&registry, &p, "standard"), Severity::Fail).contains(&"core_missing_critical".to_string()));
    }
}
