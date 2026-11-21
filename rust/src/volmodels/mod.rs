//! Volatility models priced by Fourier methods: Heston and Bates, and their calibration to a quote set.
//!
//! - `cx`: a small complex type.
//! - `base`: the model trait `CharModel`, the quote set (`Quote`, `Series`, `QuoteSet`), the loss, the error in vol points.
//! - `fourier`: the Lewis integral (adaptive Gauss-Legendre) and the Carr-Madan FFT.
//! - `models`: `Heston`, `Bates` (characteristic function in the stable form of Albrecher et al. 2007).
//! - `calib`: Levenberg-Marquardt with a numerical Jacobian and random starts.
//! - `chain`: a stored option snapshot as a quote set, with the live filters and their counts.
//! - `cli`: `fa vol fit`.
//!
//! A model price is not a market price. The fit is to mids of bid and ask of a snapshot.

pub mod base;
pub mod calib;
pub mod chain;
pub mod cli;
pub mod cx;
pub mod fourier;
pub mod models;

#[cfg(test)]
mod tests;
