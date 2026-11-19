//! Typed access to the company data from `mapcheck` (the Rust form of `schema.py`, `store.py`
//! and the read part of `importer.py`).
//!
//! A company has periods; a period has one value for each standard variable of the registry
//! (`financial_analysis/standard_variables.toml`), in the sequence of the registry, with its source text
//! and the unmapped tags (`raw_extras`). There is no second list of the variable names: the registry is the
//! only definition, so the check "the field order of `schema.py` equals the registry" is not necessary.
//!
//! Two ways to get a company:
//! - `import` (`mapcheck::import::import_ticker`) maps the SEC data, writes the store file
//!   `data/processed/<T>.json` (the file that `fa dcf classic` reads) and gives the same content as
//!   a `Company`: `Company::from_store(&registry, &imported.store)`.
//! - `Company::load` reads a store file.

use mapcheck::registry::Registry;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct Period {
    pub ticker: String,
    pub period_end: String,
    pub form: String,
    pub filed_date: String,
    pub currency: String,
    /// One entry for each variable of the registry, in the sequence of the registry.
    pub values: Vec<Option<f64>>,
    /// Variable name -> the tag, or `= formula`.
    pub sources: BTreeMap<String, String>,
    /// Tags that have no standard variable: tag -> value.
    pub raw_extras: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Company {
    pub ticker: String,
    pub cik: String,
    pub company_name: String,
    pub periods: Vec<Period>,
}

impl Period {
    pub fn get(&self, registry: &Registry, name: &str) -> Option<f64> {
        registry.try_var(name).and_then(|i| self.values[i])
    }

    /// Operating cash flow less capital expenditure. The capital expenditure is `capex_total` (equipment
    /// and capitalized software, `PLANNING/RULES.md` R2); a period with no `capex_total` uses `capex`. An
    /// outflow of either sign gives the same result.
    pub fn free_cash_flow(&self, registry: &Registry) -> Option<f64> {
        let capex = self.get(registry, "capex_total").or_else(|| self.get(registry, "capex"))?;
        Some(self.get(registry, "operating_cash_flow")? - capex.abs())
    }

    /// Free cash flow divided by revenue (`None` when the revenue is missing or zero).
    pub fn fcf_margin(&self, registry: &Registry) -> Option<f64> {
        let fcf = self.free_cash_flow(registry)?;
        match self.get(registry, "revenue") {
            Some(r) if r != 0.0 => Some(fcf / r),
            _ => None,
        }
    }
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

impl Company {
    /// The company of the content of a store file (`mapcheck::store::store_json`).
    pub fn from_store(registry: &Registry, store: &Value) -> Result<Company, String> {
        let ticker = text(store, "ticker").ok_or("store: no ticker")?;
        let periods = store.get("periods").and_then(Value::as_array).ok_or("store: no periods")?;
        let mut out = Vec::new();
        for p in periods {
            let period_end = text(p, "period_end").ok_or("store: a period has no period_end")?;
            let filed_date = text(p, "filed_date").ok_or_else(|| format!("store: period {period_end} has no filed_date"))?;
            let values = registry.variables.iter().map(|v| p.get(&v.name).and_then(Value::as_f64)).collect();
            let sources = p
                .get("sources")
                .and_then(Value::as_object)
                .map(|m| m.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
                .unwrap_or_default();
            let raw_extras = p
                .get("raw_extras")
                .and_then(Value::as_object)
                .map(|m| m.iter().filter_map(|(k, v)| v.as_f64().map(|x| (k.clone(), x))).collect())
                .unwrap_or_default();
            out.push(Period {
                ticker: text(p, "ticker").unwrap_or_else(|| ticker.clone()),
                period_end,
                form: text(p, "form").unwrap_or_default(),
                filed_date,
                currency: text(p, "currency").unwrap_or_else(|| "USD".to_string()),
                values,
                sources,
                raw_extras,
            });
        }
        Ok(Company {
            cik: text(store, "cik").ok_or("store: no cik")?,
            company_name: text(store, "company_name").ok_or("store: no company_name")?,
            ticker,
            periods: out,
        })
    }

    /// Read `<dir>/<TICKER>.json`.
    pub fn load(registry: &Registry, dir: &Path, ticker: &str) -> Result<Company, String> {
        let path = dir.join(format!("{}.json", ticker.to_uppercase()));
        if !path.is_file() {
            return Err(format!("No saved data for {} at {}. Run: fa import {}", ticker.to_uppercase(), path.display(), ticker.to_uppercase()));
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let value: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Company::from_store(registry, &value).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn annual(&self) -> impl Iterator<Item = &Period> {
        self.periods.iter().filter(|p| p.form == "10-K")
    }

    pub fn quarterly(&self) -> impl Iterator<Item = &Period> {
        self.periods.iter().filter(|p| p.form == "10-Q")
    }

    /// The values of one metric, oldest period first (`store.get_series` of the Python code). A metric is a
    /// variable of the registry, or one of the derived values `free_cash_flow` and `fcf_margin`.
    pub fn series(&self, registry: &Registry, name: &str) -> Result<Vec<(String, Option<f64>)>, String> {
        let value = |p: &Period| -> Result<Option<f64>, String> {
            match name {
                "free_cash_flow" => Ok(p.free_cash_flow(registry)),
                "fcf_margin" => Ok(p.fcf_margin(registry)),
                _ => registry.try_var(name).map(|i| p.values[i]).ok_or_else(|| format!("'{name}' is not a valid metric")),
            }
        };
        let mut out = self.periods.iter().map(|p| Ok((p.period_end.clone(), value(p)?))).collect::<Result<Vec<_>, String>>()?;
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }
}

pub const IMPORT_USAGE: &str = "\
fa import TICKER... [--refresh] [--periods N]

Map the SEC company facts to the standard variables and write data/processed/<TICKER>.json (the file
that `fa dcf classic` reads). The company facts come from the cache data/raw/<TICKER>/ first; the SEC
is asked if they are not there, or with --refresh (set SEC_USER_AGENT for that).
  --periods N   fiscal years and quarters for each ticker (default: 12)";

/// `fa import`. Returns the exit code.
pub fn import_command(repo: &Path, args: &[String]) -> Result<i32, String> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{IMPORT_USAGE}");
        return Ok(0);
    }
    let (mut tickers, mut refresh, mut periods) = (Vec::new(), false, 12usize);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--refresh" => refresh = true,
            "--periods" => periods = it.next().and_then(|v| v.parse().ok()).ok_or("--periods needs a number")?,
            s if s.starts_with('-') => return Err(format!("unknown option '{s}'")),
            s => tickers.push(s.to_uppercase()),
        }
    }
    if tickers.is_empty() {
        return Err("import needs a ticker".to_string());
    }
    let registry = Registry::embedded()?;
    let (cache, store_dir) = (repo.join("data").join("raw"), repo.join("data").join("processed"));
    let mut code = 0;
    for ticker in &tickers {
        println!("Importing {ticker}...");
        match mapcheck::import::import_ticker(&registry, &cache, &store_dir, ticker, periods, 0.15, refresh) {
            Ok(done) => {
                let company = Company::from_store(&registry, &done.store)?;
                println!("  Company:   {}", company.company_name);
                println!("  CIK:       {}", company.cik);
                println!("  Annual periods (10-K):    {}", company.annual().count());
                println!("  Quarterly periods (10-Q): {}", company.quarterly().count());
                println!("  Saved: {}", done.store_path.display());
            }
            Err(e) => {
                println!("Error: {e}");
                code = 1;
            }
        }
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mapcheck::facts::CompanyFacts;
    use mapcheck::store::store_json;

    fn registry() -> Registry {
        Registry::embedded().unwrap()
    }

    #[test]
    fn mapped_data_gives_typed_periods_with_values_sources_and_extras() {
        let registry = registry();
        let tag = |name: &str, value: f64| {
            format!(r#""{name}": {{"units": {{"USD": [{{"start": "2024-01-01", "end": "2024-12-31", "val": {value}, "form": "10-K", "filed": "2025-02-01"}}]}}}}"#)
        };
        let json = format!(
            r#"{{"cik": 1, "entityName": "ACME", "facts": {{"us-gaap": {{{}, {}, {}, {}, {}}}}}}}"#,
            tag("Revenues", 100.0),
            tag("CostOfRevenue", 60.0),
            tag("NetCashProvidedByUsedInOperatingActivities", 30.0),
            tag("PaymentsToAcquirePropertyPlantAndEquipment", 8.0),
            tag("SomeCompanyDetail", 3.0)
        );
        let facts: CompanyFacts = serde_json::from_str(&json).unwrap();
        let mapped = mapcheck::mapper::map_company(&registry, &facts, 12);
        let c = Company::from_store(&registry, &store_json(&registry, "ACME", "1", &mapped)).unwrap();
        assert_eq!((c.ticker.as_str(), c.cik.as_str(), c.company_name.as_str()), ("ACME", "1", "ACME"));
        let p = &c.periods[0];
        assert_eq!((p.form.as_str(), p.period_end.as_str(), p.filed_date.as_str(), p.currency.as_str()), ("10-K", "2024-12-31", "2025-02-01", "USD"));
        assert_eq!(p.values.len(), registry.variables.len());
        assert_eq!(p.get(&registry, "revenue"), Some(100.0));
        assert_eq!(p.get(&registry, "gross_profit"), Some(40.0));
        assert_eq!(p.sources["gross_profit"], "= revenue - cogs_total");
        assert_eq!(p.raw_extras["us-gaap/SomeCompanyDetail"], 3.0);
        assert_eq!(p.get(&registry, "total_debt"), None);
        assert_eq!(c.annual().count(), 1);
        assert_eq!(c.series(&registry, "revenue").unwrap(), vec![("2024-12-31".to_string(), Some(100.0))]);
        assert!(c.series(&registry, "not_a_variable").unwrap_err().contains("not a valid metric"));
    }

    /// The tests of `store.get_series`: oldest first, and the derived metrics.
    #[test]
    fn series_are_sorted_and_have_the_derived_metrics() {
        let registry = registry();
        let mk = |end: &str, rev: f64, ocf: f64, capex: f64| {
            let mut p = Period {
                ticker: "TEST".into(),
                period_end: end.into(),
                form: "10-K".into(),
                filed_date: "2024-02-14".into(),
                currency: "USD".into(),
                values: vec![None; registry.variables.len()],
                sources: BTreeMap::new(),
                raw_extras: BTreeMap::new(),
            };
            p.values[registry.var("revenue")] = Some(rev);
            p.values[registry.var("operating_cash_flow")] = Some(ocf);
            p.values[registry.var("capex")] = Some(capex);
            p
        };
        // the newer period first: the series are sorted
        let c = Company {
            ticker: "TEST".into(),
            cik: "1".into(),
            company_name: "Test Corp".into(),
            periods: vec![mk("2023-12-31", 1.2e9, 250e6, 60e6), mk("2022-12-31", 1.0e9, 200e6, 50e6)],
        };
        let dates: Vec<String> = c.series(&registry, "revenue").unwrap().into_iter().map(|x| x.0).collect();
        assert_eq!(dates, ["2022-12-31", "2023-12-31"]);
        let fcf = c.series(&registry, "free_cash_flow").unwrap();
        assert_eq!((fcf[0].1, fcf[1].1), (Some(150e6), Some(190e6)));
        let margin = c.series(&registry, "fcf_margin").unwrap();
        assert!((margin[0].1.unwrap() - 0.15).abs() < 1e-9);
    }

    /// The tests of `store.save` and `store.load`: the folder is made, the JSON is indented, the dates are ISO
    /// texts, and a file loads again with every value, in any case of the ticker.
    #[test]
    fn a_written_store_file_is_pretty_json_and_loads_again_in_any_case() {
        let registry = registry();
        let tag = |name: &str, value: f64| {
            format!(r#""{name}": {{"units": {{"USD": [{{"start": "2024-01-01", "end": "2024-12-31", "val": {value}, "form": "10-K", "filed": "2025-02-01"}}]}}}}"#)
        };
        let json = format!(
            r#"{{"cik": 1, "entityName": "ACME", "facts": {{"us-gaap": {{{}, {}, {}}}}}}}"#,
            tag("Revenues", 100.0),
            tag("CashAndCashEquivalentsAtCarryingValue", 7.0),
            tag("SomeCompanyDetail", 999.0)
        );
        let facts: mapcheck::facts::CompanyFacts = serde_json::from_str(&json).unwrap();
        let mapped = mapcheck::mapper::map_company(&registry, &facts, 12);
        let dir = std::env::temp_dir().join(format!("fa-store-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = mapcheck::store::write_store(&dir.join("not/yet/there"), &registry, "TEST", "0001", &mapped).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("{\n  \"cik\": \"0001\","), "{}", &text[..40]);
        assert!(text.contains("\"period_end\": \"2024-12-31\"") && text.contains("\"filed_date\": \"2025-02-01\""));
        let c = Company::load(&registry, &dir.join("not/yet/there"), "test").unwrap();
        assert_eq!((c.ticker.as_str(), c.cik.as_str()), ("TEST", "0001"));
        assert_eq!(c.periods[0].get(&registry, "revenue"), Some(100.0));
        assert_eq!(c.periods[0].get(&registry, "cash_and_equivalents"), Some(7.0));
        assert_eq!(c.periods[0].raw_extras["us-gaap/SomeCompanyDetail"], 999.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The store file of the fixtures is a real period of IBM. Each registry variable is read, and each value
    /// that is in the file arrives at the place of its variable.
    #[test]
    fn every_variable_of_the_registry_is_read_from_a_store_file() {
        let registry = registry();
        let path = format!("{}/tests/fixtures/analyze/company_store.json", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(path).unwrap();
        let store: Value = serde_json::from_str(&text).unwrap();
        let c = Company::from_store(&registry, &store).unwrap();
        assert_eq!(c.periods.len(), 2);
        for (period, json) in c.periods.iter().zip(store["periods"].as_array().unwrap()) {
            for (i, v) in registry.variables.iter().enumerate() {
                assert!(json.get(&v.name).is_some(), "the store file has the variable {}", v.name);
                assert_eq!(period.values[i], json[&v.name].as_f64(), "{}", v.name);
            }
        }
    }

    #[test]
    fn free_cash_flow_and_margin_follow_the_python_property() {
        let registry = registry();
        let mut p = Period {
            ticker: "T".into(),
            period_end: "2024-12-31".into(),
            form: "10-K".into(),
            filed_date: "2025-02-01".into(),
            currency: "USD".into(),
            values: vec![None; registry.variables.len()],
            sources: BTreeMap::new(),
            raw_extras: BTreeMap::new(),
        };
        let set = |p: &mut Period, name: &str, v: f64| p.values[registry.var(name)] = Some(v);
        assert_eq!((p.free_cash_flow(&registry), p.fcf_margin(&registry)), (None, None));
        set(&mut p, "operating_cash_flow", 100.0);
        assert_eq!(p.free_cash_flow(&registry), None, "no capex");
        set(&mut p, "capex", -30.0);
        assert_eq!(p.free_cash_flow(&registry), Some(70.0), "an outflow of either sign");
        set(&mut p, "capex_total", 40.0);
        assert_eq!(p.free_cash_flow(&registry), Some(60.0), "capex_total first");
        assert_eq!(p.fcf_margin(&registry), None, "no revenue");
        set(&mut p, "revenue", 0.0);
        assert_eq!(p.fcf_margin(&registry), None, "revenue zero");
        set(&mut p, "revenue", 200.0);
        assert_eq!(p.fcf_margin(&registry), Some(0.3));
    }

    #[test]
    fn a_missing_store_file_gives_the_import_hint() {
        let e = Company::load(&registry(), std::path::Path::new("/nonexistent"), "zzz").unwrap_err();
        assert!(e.contains("No saved data for ZZZ") && e.contains("fa import ZZZ"), "{e}");
    }
}
