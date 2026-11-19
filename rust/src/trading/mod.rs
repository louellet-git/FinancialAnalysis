//! Backtester for the SPY/QQQ option bots (moved from the Python package `trading/`).
//!
//! It reads the CSV cache of market data, runs each bot on its own account, and prints a
//! leaderboard. All option prices are MODEL prices (see `pricing`). The code sends no order
//! to a broker.

// The code keeps the shape of the Python expressions (float operation order, NaN behavior), so a
// few style lints are off on purpose.
#![allow(
    clippy::manual_range_contains,
    clippy::manual_clamp,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::too_many_arguments,
    clippy::manual_is_multiple_of
)]

pub mod bots;
pub mod calendar;
pub mod cli;
pub mod data;
pub mod engine;
pub mod metrics;
pub mod pricing;
pub mod stats;
pub mod strategy;
pub mod structures;
