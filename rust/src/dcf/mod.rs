//! The DCF engines in Rust: the classic engine (`classic`), the driver engine (`driver`),
//! the scenario presets (`scenarios`), the verdict (`verdict`) and the driver models as
//! data (`model`).
//!
//! The random numbers are the same as those of numpy (`nprandom`), so each number that
//! the Python engines published is the same here (tests: `tests.rs`, fixtures in
//! `rust/tests/fixtures/dcf/`).

use std::fmt;

pub mod analysis;
pub mod auto;
pub mod auto_cli;
pub mod auto_pit;
pub mod assumptions;
pub mod auto_returns;
pub mod backtest;
pub mod calib_bands;
pub mod calib_report;
pub mod calibrate;
pub mod classic;
pub mod cli;
pub mod driver;
pub mod model;
pub mod nprandom;
pub mod pathrate;
pub mod paths;
pub mod paths_cli;
pub mod process;
pub mod panel;
pub mod scenarios;
pub mod stats;
pub mod verdict;
mod ziggurat_tables;

#[cfg(test)]
mod paths_tests;
mod calibrate_tests;
#[cfg(test)]
mod runner_tests;
#[cfg(test)]
mod tests;

/// An error of an engine: the input is not valid, or a result is not defined.
#[derive(Debug, Clone, PartialEq)]
pub struct DcfError(pub String);

impl fmt::Display for DcfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DcfError {}

/// A seed from the clock, for runs without a seed.
pub(crate) fn clock_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    (n as u64 ^ (n >> 64) as u64) & 0xFFFF_FFFF
}
