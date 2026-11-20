//! The ledger of `fa sync`: for each item (data set and key) the date of the last run that ended with no failure.
//!
//! The caches have their own dates (the `updated` field of `prices.meta.json`, the date of a snapshot file, the date of a
//! file). Some items have no record in their cache: a ticker that the source does not know (no file is written), a file
//! that a successful run did not change (no new bar on a weekend). For those items the ledger keeps the date of the check,
//! so the next run does not ask again. The age of an item is the age of the newest of the two records.
//!
//! The file is `data/raw/_sync/checked.json`. It is a record of the caches, not a second copy of them: it can be deleted
//! at any time (the sync then reads the dates of the caches only).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ledger {
    path: Option<PathBuf>,
    map: BTreeMap<String, String>,
}

pub fn ledger_path(cache: &Path) -> PathBuf {
    cache.join("_sync").join("checked.json")
}

fn key(set: &str, item: &str) -> String {
    format!("{set}/{item}")
}

impl Ledger {
    /// A ledger that is not saved (a test, or `--status` that must not write).
    pub fn in_memory() -> Ledger {
        Ledger::default()
    }

    /// The ledger of a cache folder. A missing or damaged file gives an empty ledger.
    pub fn load(cache: &Path) -> Ledger {
        let path = ledger_path(cache);
        let map = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<BTreeMap<String, String>>(&t).ok())
            .unwrap_or_default();
        Ledger { path: Some(path), map }
    }

    pub fn get(&self, set: &str, item: &str) -> Option<&str> {
        self.map.get(&key(set, item)).map(String::as_str)
    }

    pub fn set(&mut self, set: &str, item: &str, date: &str) {
        self.map.insert(key(set, item), date.to_string());
    }

    /// Write the file in one step (the old file stays when the write fails).
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else { return Ok(()) };
        let text = serde_json::to_string_pretty(&self.map).map_err(|e| e.to_string())?;
        crate::market::write_atomic(path, &text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_and_save() {
        let dir = std::env::temp_dir().join(format!("fa-sync-ledger-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut l = Ledger::load(&dir);
        assert_eq!(l.get("prices", "IBM"), None);
        l.set("prices", "IBM", "2026-11-19");
        l.save().unwrap();
        let again = Ledger::load(&dir);
        assert_eq!(again.get("prices", "IBM"), Some("2026-11-19"));
        std::fs::write(ledger_path(&dir), "not json").unwrap();
        assert_eq!(Ledger::load(&dir).get("prices", "IBM"), None, "a damaged file is an empty ledger");
        assert!(Ledger::in_memory().save().is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
