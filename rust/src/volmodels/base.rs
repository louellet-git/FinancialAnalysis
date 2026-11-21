//! The base of the volatility models: the model trait, the quote set and the loss.
//!
//! A model gives the characteristic function of `X = ln(F_T / F_0)` under the forward measure
//! (`E[F_T] = F_0`). So rate and dividend enter only through the forward `F` and the discount factor `df`
//! of each series. A series is one root and one expiry.

use super::cx::Cx;
use super::fourier;
use crate::market::option_iv::{black76, implied_vol};

use rayon::prelude::*;

/// A model with a characteristic function and a parameter vector.
pub trait CharModel: Sync + Clone {
    /// `E[exp(i z X)]` for `X = ln(F_T / F_0)`, `t` years. `z` is complex.
    fn log_cf(&self, z: Cx, t: f64) -> Cx;
    fn names() -> &'static [&'static str];
    /// Lower and upper bound of each parameter.
    fn bounds() -> Vec<(f64, f64)>;
    fn from_vec(p: &[f64]) -> Self;
    fn to_vec(&self) -> Vec<f64>;
    /// A start vector from the ATM volatility (decimal) and the skew (d IV / d ln K).
    fn start(atm_vol: f64, skew: f64) -> Vec<f64>;
}

/// One out-of-the-money quote of a series.
#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub strike: f64,
    pub call: bool,
    /// Mid of bid and ask (price).
    pub mid: f64,
    pub iv_mid: f64,
    pub iv_bid: f64,
    pub iv_ask: f64,
}

impl Quote {
    /// Ask volatility minus bid volatility (decimal; 0.01 is one vol point).
    pub fn iv_spread(&self) -> f64 {
        self.iv_ask - self.iv_bid
    }
}

/// One root and one expiry.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub root: String,
    pub expiry: String,
    pub days: i64,
    /// Years.
    pub t: f64,
    pub forward: f64,
    pub df: f64,
    pub quotes: Vec<Quote>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct QuoteSet {
    pub series: Vec<Series>,
}

/// The lowest spread that the loss uses (vol, decimal): 0.05 vol points. It stops one very tight quote from taking all the weight.
pub const MIN_SPREAD: f64 = 0.0005;

impl QuoteSet {
    pub fn n_quotes(&self) -> usize {
        self.series.iter().map(|s| s.quotes.len()).sum()
    }
}

impl Series {
    /// Model prices (currency) of the quotes: calls and puts by the same integral.
    pub fn model_prices<M: CharModel>(&self, m: &M) -> Vec<f64> {
        self.model_prices_acc(m, fourier::Accuracy::TIGHT)
    }

    /// `model_prices` with a chosen accuracy of the integral.
    pub fn model_prices_acc<M: CharModel>(&self, m: &M, acc: fourier::Accuracy) -> Vec<f64> {
        let ks: Vec<f64> = self.quotes.iter().map(|q| (self.forward / q.strike).ln()).collect();
        let j = fourier::lewis_j_acc(m, self.t, &ks, acc);
        self.quotes
            .iter()
            .zip(&j)
            .map(|(q, j)| {
                let root = (self.forward * q.strike).sqrt();
                self.df * if q.call { self.forward - root * j } else { q.strike - root * j }
            })
            .collect()
    }

    /// Black-76 vega (price per unit of volatility) at the mid volatility of a quote.
    fn vega(&self, q: &Quote) -> f64 {
        let sq = q.iv_mid * self.t.sqrt();
        let d1 = ((self.forward / q.strike).ln() + 0.5 * sq * sq) / sq;
        self.df * self.forward * (-0.5 * d1 * d1).exp() * 0.398_942_280_401_432_7 * self.t.sqrt()
    }

    /// Residuals of the loss: (model price - mid) / (vega * spread). It is the error in units of the quoted spread.
    pub fn residuals<M: CharModel>(&self, m: &M) -> Vec<f64> {
        let p = self.model_prices_acc(m, fourier::Accuracy::FIT);
        self.quotes.iter().zip(&p).map(|(q, p)| (p - q.mid) / (self.vega(q).max(1e-12) * q.iv_spread().max(MIN_SPREAD))).collect()
    }
}

/// All residuals of the set (series in parallel).
pub fn residuals<M: CharModel>(m: &M, set: &QuoteSet) -> Vec<f64> {
    set.series.par_iter().map(|s| s.residuals(m)).collect::<Vec<_>>().concat()
}

/// The loss: the sum of squares of the residuals.
pub fn loss<M: CharModel>(m: &M, set: &QuoteSet) -> f64 {
    residuals(m, set).iter().map(|r| r * r).sum()
}

/// The error of one series in volatility points (1 point = 0.01 of volatility).
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesError {
    pub root: String,
    pub expiry: String,
    pub days: i64,
    pub n: usize,
    /// Quotes whose model price has no implied volatility. They are not in the two errors.
    pub no_iv: usize,
    pub rms_points: f64,
    pub max_points: f64,
}

/// Model implied volatility minus market mid volatility, in vol points, for each series.
pub fn iv_errors<M: CharModel>(m: &M, set: &QuoteSet) -> Vec<SeriesError> {
    set.series
        .par_iter()
        .map(|s| {
            let prices = s.model_prices(m);
            let (mut sum2, mut max, mut n, mut no_iv) = (0.0, 0.0f64, 0usize, 0usize);
            for (q, p) in s.quotes.iter().zip(&prices) {
                match implied_vol(q.call, *p, s.forward, q.strike, s.t, s.df) {
                    Some(iv) => {
                        let e = 100.0 * (iv - q.iv_mid);
                        sum2 += e * e;
                        max = max.max(e.abs());
                        n += 1;
                    }
                    None => no_iv += 1,
                }
            }
            SeriesError { root: s.root.clone(), expiry: s.expiry.clone(), days: s.days, n: s.quotes.len(), no_iv, rms_points: if n > 0 { (sum2 / n as f64).sqrt() } else { f64::NAN }, max_points: max }
        })
        .collect()
}

/// A quote from a price: the volatility of the mid, bid and ask (`None` when one has no volatility).
pub fn quote_from_prices(strike: f64, call: bool, bid: f64, ask: f64, forward: f64, t: f64, df: f64) -> Option<Quote> {
    let mid = 0.5 * (bid + ask);
    Some(Quote { strike, call, mid, iv_mid: implied_vol(call, mid, forward, strike, t, df)?, iv_bid: implied_vol(call, bid, forward, strike, t, df)?, iv_ask: implied_vol(call, ask, forward, strike, t, df)? })
}

/// Black-76 price of a quote at a volatility (used by the synthetic tests).
pub fn bs_price(call: bool, forward: f64, strike: f64, t: f64, df: f64, vol: f64) -> f64 {
    black76(call, forward, strike, t, df, vol)
}
