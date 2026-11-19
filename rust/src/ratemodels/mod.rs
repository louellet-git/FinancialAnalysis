//! Stochastic short-rate models, their calibration to a rate history, and fast reproducible simulation.
//!
//! - `models`: `RateModel` trait; `Vasicek`, `Cir`, `Ckls`, `HoLee`, `HullWhite`.
//! - `sim`: parallel paths with one random stream for each path (`simulate_paths`, `simulate_map`).
//! - `data`: `Series`, `Freq`, `sample` (resample an interval to a frequency).
//! - `calib`: `calibrate(kind, method, sample, gamma)`.
//! - `rolling`: rolling windows, frequency comparison, model comparison.
//! - `curve`: the GSW (Svensson) zero curve for Ho-Lee and Hull-White.
//! - `cli`: the `fa models` commands.
//!
//! Rates are decimals per year. The series from the GSW file are proxies of the short rate
//! (the 1-year zero yield is not the overnight rate).

pub mod calib;
pub mod cli;
pub mod curve;
pub mod data;
pub mod math;
pub mod models;
pub mod rng;
pub mod rolling;
pub mod sim;

#[cfg(test)]
mod tests;
