//! The store file `data/processed/<TICKER>.json`.
//!
//! The valuation code (`fa`: engines, backtest, report) reads this file. Rust is
//! the only program that makes the standard variables; it writes them here.

use crate::mapper::Company;
use crate::registry::Registry;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// The store content for one company: all annual and quarterly periods, oldest first.
pub fn store_json(registry: &Registry, ticker: &str, cik: &str, company: &Company) -> Value {
    let mut periods: Vec<(String, String, Value)> = Vec::new();
    for period in company.annual.iter().chain(company.quarterly.iter()) {
        let mut entry = Map::new();
        entry.insert("ticker".into(), json!(ticker));
        entry.insert("period_end".into(), json!(period.end));
        entry.insert("form".into(), json!(period.form_label()));
        entry.insert("filed_date".into(), json!(period.filed.clone().unwrap_or_else(|| period.end.clone())));
        entry.insert("currency".into(), json!(company.currency));
        let mut sources = Map::new();
        for (i, variable) in registry.variables.iter().enumerate() {
            entry.insert(variable.name.clone(), json!(period.values[i]));
            if let Some(source) = &period.sources[i] {
                sources.insert(variable.name.clone(), json!(source));
            }
        }
        entry.insert("segment_revenue".into(), Value::Null);
        entry.insert("sources".into(), Value::Object(sources));
        entry.insert("raw_extras".into(), json!(period.raw_extras));
        periods.push((period.end.clone(), period.form_label().to_string(), Value::Object(entry)));
    }
    periods.sort_by(|a, b| (a.0.as_str(), a.1.as_str()).cmp(&(b.0.as_str(), b.1.as_str())));
    json!({
        "ticker": ticker,
        "cik": cik,
        "company_name": company.name,
        "periods": periods.into_iter().map(|p| p.2).collect::<Vec<_>>(),
    })
}

pub fn write_store(dir: &Path, registry: &Registry, ticker: &str, cik: &str, company: &Company) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{ticker}.json"));
    let text = serde_json::to_string_pretty(&store_json(registry, ticker, cik, company)).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::testing::{company, fact};
    use crate::mapper::map_company;

    #[test]
    fn store_has_each_period_with_values_sources_and_raw_tags() {
        let registry = Registry::embedded().unwrap();
        let facts = company(vec![
            ("Revenues", vec![
                fact(Some("2024-01-01"), "2024-12-31", 100.0, "10-K", "2025-02-01"),
                fact(Some("2024-04-01"), "2024-06-30", 26.0, "10-Q", "2024-08-01"),
            ]),
            ("CostOfRevenue", vec![fact(Some("2024-01-01"), "2024-12-31", 60.0, "10-K", "2025-02-01")]),
            ("SomeCompanyDetail", vec![fact(Some("2024-01-01"), "2024-12-31", 3.0, "10-K", "2025-02-01")]),
        ]);
        let mapped = map_company(&registry, &facts, 12);
        let store = store_json(&registry, "ACME", "1", &mapped);
        let periods = store["periods"].as_array().unwrap();
        assert_eq!(periods.len(), 2);
        assert_eq!(periods[0]["period_end"], "2024-06-30");       // oldest first
        let annual = &periods[1];
        assert_eq!(annual["form"], "10-K");
        assert_eq!(annual["filed_date"], "2025-02-01");
        assert_eq!(annual["revenue"], 100.0);
        assert_eq!(annual["gross_profit"], 40.0);
        assert_eq!(annual["sources"]["gross_profit"], "= revenue - cogs_total");
        assert!(annual["total_debt"].is_null());
        assert_eq!(annual["raw_extras"]["us-gaap/SomeCompanyDetail"], 3.0);
        assert_eq!(store["company_name"], "ACME");
    }
}
