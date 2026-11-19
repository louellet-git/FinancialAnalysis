//! The import of one company: SEC company facts (cache, or the SEC), the standard
//! variables, and the store file `<store>/<TICKER>.json`.
//!
//! `mapcheck import` and the `fa` program (`fa import`, `fa analyze`) both call
//! `import_ticker`, so the import has one implementation.

use crate::facts::CompanyFacts;
use crate::http::Client;
use crate::mapper::Company;
use crate::registry::Registry;
use crate::run::map_ticker;
use crate::{sec, store};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Why an import failed. `Unsupported` is a filer with no us-gaap facts (IFRS).
#[derive(Debug)]
pub enum ImportError {
    Unsupported(String),
    Failed(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Unsupported(m) | ImportError::Failed(m) => write!(f, "{m}"),
        }
    }
}

impl From<String> for ImportError {
    fn from(message: String) -> Self {
        ImportError::Failed(message)
    }
}

#[derive(Debug)]
pub struct Imported {
    pub cik: String,
    pub company: Company,
    pub store_path: PathBuf,
    /// The store content that was written (the same text as the file).
    pub store: Value,
}

/// Get the company facts from the SEC again (`refresh`), then map and write the store file.
pub fn import_ticker(
    registry: &Registry,
    cache: &Path,
    store_dir: &Path,
    ticker: &str,
    periods: usize,
    delay: f64,
    refresh: bool,
) -> Result<Imported, ImportError> {
    if refresh {
        let mut client = Client::sec(delay)?;
        let list = sec::ticker_list(&mut client, cache, true)?;
        let cik = list
            .iter()
            .find(|(t, _)| t == ticker)
            .map(|(_, c)| c.clone())
            .ok_or_else(|| format!("{ticker} is not in the SEC ticker list"))?;
        sec::fetch_facts(&mut client, cache, ticker, &cik)?;
    }
    let facts = sec::load_or_fetch_facts(cache, ticker, delay)?;
    import_facts(registry, cache, store_dir, ticker, periods, &facts)
}

/// The same, for facts that are loaded (a test has no SEC and no cache file).
pub fn import_facts(
    registry: &Registry,
    cache: &Path,
    store_dir: &Path,
    ticker: &str,
    periods: usize,
    facts: &CompanyFacts,
) -> Result<Imported, ImportError> {
    if !facts.has_us_gaap() {
        let namespaces: Vec<&String> = facts.facts.keys().collect();
        return Err(ImportError::Unsupported(format!(
            "{ticker}: does not report under us-gaap (namespaces found: {namespaces:?}). IFRS and non-US-GAAP filers are not supported."
        )));
    }
    let company = map_ticker(registry, cache, ticker, facts, periods);
    let cik = match &facts.cik {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => String::new(),
    };
    let store_path = store::write_store(store_dir, registry, ticker, &cik, &company)?;
    let store = store::store_json(registry, ticker, &cik, &company);
    Ok(Imported { cik, company, store_path, store })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::testing::{company, fact};

    #[test]
    fn import_writes_the_store_file_and_returns_its_content() {
        let registry = Registry::embedded().unwrap();
        let facts = company(vec![("Revenues", vec![fact(Some("2024-01-01"), "2024-12-31", 100.0, "10-K", "2025-02-01")])]);
        let dir = std::env::temp_dir().join(format!("mapcheck-import-{}", std::process::id()));
        let done = import_facts(&registry, &dir.join("raw"), &dir, "ACME", 12, &facts).unwrap();
        assert!(done.store_path.is_file());
        let text = std::fs::read_to_string(&done.store_path).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), done.store);
        assert_eq!(done.store["periods"][0]["revenue"], 100.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_filer_without_us_gaap_is_unsupported() {
        let registry = Registry::embedded().unwrap();
        let mut facts = company(vec![]);
        facts.facts.clear();
        let dir = std::env::temp_dir().join(format!("mapcheck-import-ifrs-{}", std::process::id()));
        let e = import_facts(&registry, &dir.join("raw"), &dir, "ACME", 12, &facts).unwrap_err();
        assert!(matches!(e, ImportError::Unsupported(m) if m.contains("does not report under us-gaap")));
    }
}
