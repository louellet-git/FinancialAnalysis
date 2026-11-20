//! The optional file `sync.toml` in the root of the repo: maximum ages, the default list of tickers, the symbols of
//! the option chains and of the intraday bars. A missing file means the defaults of the code.
//!
//! ```toml
//! tickers_file = "rust/mapcheck/universe/sp500.txt"
//! option_symbols = ["SPY"]
//! bond_days = 1
//! intraday_symbols = ["SPY", "QQQ"]
//!
//! [max_age_days]
//! prices = 1
//! estimates = 7
//! ```

use super::dataset::{DataSet, ALL};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

pub const FILE_NAME: &str = "sync.toml";
/// The most FedInvest days that one run asks for.
pub const MAX_BOND_DAYS: u32 = 500;
pub const DEFAULT_OPTION_SYMBOLS: [&str; 1] = ["SPY"];
/// The symbols of the intraday bars that the option bots use. The symbols of `--intraday-symbols` come after them.
pub const DEFAULT_INTRADAY_SYMBOLS: [&str; 2] = ["SPY", "QQQ"];

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    tickers_file: Option<String>,
    option_symbols: Option<Vec<String>>,
    intraday_symbols: Option<Vec<String>>,
    bond_days: Option<u32>,
    max_age_days: Option<BTreeMap<String, u32>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncConfig {
    pub tickers_file: Option<String>,
    pub option_symbols: Vec<String>,
    pub intraday_symbols: Vec<String>,
    /// The number of market days of FedInvest prices that the bonds data set asks for (1 = the last market day).
    pub bond_days: u32,
    /// The maximum age of each data set, in the unit of the data set (see `DataSet::age_unit`).
    pub max_age: BTreeMap<DataSet, u32>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        SyncConfig {
            tickers_file: None,
            option_symbols: DEFAULT_OPTION_SYMBOLS.iter().map(|s| s.to_string()).collect(),
            intraday_symbols: DEFAULT_INTRADAY_SYMBOLS.iter().map(|s| s.to_string()).collect(),
            bond_days: 1,
            max_age: ALL.iter().map(|d| (*d, d.default_max_age())).collect(),
        }
    }
}

fn symbols(list: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in list {
        let s = s.trim().to_uppercase();
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// The number of FedInvest days must be from 1 to `MAX_BOND_DAYS`.
pub fn check_bond_days(n: u32, name: &str) -> Result<u32, String> {
    if n == 0 || n > MAX_BOND_DAYS {
        return Err(format!("{name} must be from 1 to {MAX_BOND_DAYS}"));
    }
    Ok(n)
}

impl SyncConfig {
    /// The config of the text of a `sync.toml`. An unknown key or an unknown data set name is an error (a typo must not
    /// pass in silence).
    pub fn parse(text: &str) -> Result<SyncConfig, String> {
        let raw: RawConfig = toml::from_str(text).map_err(|e| format!("{FILE_NAME}: {e}"))?;
        let mut cfg = SyncConfig::default();
        if let Some(f) = raw.tickers_file.filter(|f| !f.trim().is_empty()) {
            cfg.tickers_file = Some(f);
        }
        if let Some(o) = raw.option_symbols {
            cfg.option_symbols = symbols(o);
        }
        if let Some(i) = raw.intraday_symbols {
            cfg.intraday_symbols = symbols(i);
        }
        if let Some(n) = raw.bond_days {
            cfg.bond_days = check_bond_days(n, &format!("{FILE_NAME}: bond_days"))?;
        }
        for (name, days) in raw.max_age_days.unwrap_or_default() {
            let set = DataSet::from_name(&name).ok_or_else(|| format!("{FILE_NAME}: max_age_days has an unknown data set '{name}'"))?;
            if days == 0 {
                return Err(format!("{FILE_NAME}: max_age_days.{name} must be 1 or more"));
            }
            cfg.max_age.insert(set, days);
        }
        Ok(cfg)
    }

    /// The config of the repo: `<repo>/sync.toml`, or the defaults when the file is missing.
    pub fn load(repo: &Path) -> Result<SyncConfig, String> {
        let path = repo.join(FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(text) => SyncConfig::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SyncConfig::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    pub fn age_of(&self, set: DataSet) -> u32 {
        self.max_age.get(&set).copied().unwrap_or_else(|| set.default_max_age())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_file_gives_the_defaults() {
        let dir = std::env::temp_dir().join(format!("fa-sync-cfg-none-{}", std::process::id()));
        let cfg = SyncConfig::load(&dir).unwrap();
        assert_eq!(cfg, SyncConfig::default());
        assert_eq!(cfg.age_of(DataSet::Prices), 1);
        assert_eq!(cfg.age_of(DataSet::Estimates), 7);
        assert_eq!(cfg.option_symbols, ["SPY"]);
        assert_eq!(cfg.intraday_symbols, ["SPY", "QQQ"]);
        assert_eq!(cfg.tickers_file, None);
        assert_eq!(cfg.bond_days, 1);
    }

    #[test]
    fn the_file_changes_ages_lists_and_the_ticker_file() {
        let cfg = SyncConfig::parse(
            "tickers_file = \"my/list.txt\"\noption_symbols = [\"spy\", \"QQQ\", \"spy\"]\nintraday_symbols = [\"IWM\"]\n[max_age_days]\nprices = 2\nestimates = 14\n",
        )
        .unwrap();
        assert_eq!(cfg.tickers_file.as_deref(), Some("my/list.txt"));
        assert_eq!(cfg.option_symbols, ["SPY", "QQQ"]);
        assert_eq!(cfg.intraday_symbols, ["IWM"]);
        assert_eq!(cfg.age_of(DataSet::Prices), 2);
        assert_eq!(cfg.age_of(DataSet::Estimates), 14);
        assert_eq!(cfg.age_of(DataSet::Rates), 1, "a data set that the file does not name keeps its default");
    }

    #[test]
    fn bond_days_is_read_and_checked() {
        assert_eq!(SyncConfig::parse("bond_days = 60\n").unwrap().bond_days, 60);
        assert_eq!(SyncConfig::parse("").unwrap().bond_days, 1);
        assert!(SyncConfig::parse("bond_days = 0").unwrap_err().contains("from 1 to 500"));
        assert!(SyncConfig::parse("bond_days = 501").is_err());
        assert!(SyncConfig::parse("bond_days = \"a\"").is_err());
    }

    #[test]
    fn a_typo_is_an_error() {
        assert!(SyncConfig::parse("tickers_flie = \"x\"").unwrap_err().contains("sync.toml"));
        assert!(SyncConfig::parse("[max_age_days]\nprise = 1").unwrap_err().contains("unknown data set 'prise'"));
        assert!(SyncConfig::parse("[max_age_days]\nprices = 0").unwrap_err().contains("1 or more"));
        assert!(SyncConfig::parse("[max_age_days]\nprices = \"a\"").is_err());
    }

    #[test]
    fn the_file_of_the_repo_is_read() {
        let dir = std::env::temp_dir().join(format!("fa-sync-cfg-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE_NAME), "[max_age_days]\nrates = 3\n").unwrap();
        assert_eq!(SyncConfig::load(&dir).unwrap().age_of(DataSet::Rates), 3);
        std::fs::write(dir.join(FILE_NAME), "this is not toml [").unwrap();
        assert!(SyncConfig::load(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
