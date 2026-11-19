//! The Svensson zero-coupon curve of the Federal Reserve Board (GSW), for any maturity.
//!
//! The Board publishes six parameters for each day. The yield (continuously compounded,
//! decimal) at maturity T years is
//! `y(T) = b0 + b1 g1 + b2 (g1 - e1) + b3 (g2 - e2)` with `e_i = exp(-T/tau_i)`,
//! `g_i = (1 - e_i) / (T/tau_i)`. The instantaneous forward rate is
//! `f(T) = b0 + b1 e1 + b2 (T/tau1) e1 + b3 (T/tau2) e2`.
//! Before 1980 the file has no b3 and tau2 (a three-factor curve): the code sets b3 = 0.

use crate::forwards::Forwards;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZeroCurve {
    pub beta0: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub beta3: f64,
    pub tau1: f64,
    pub tau2: f64,
}

impl ZeroCurve {
    /// A flat curve: yield `y` (continuously compounded) at all maturities.
    pub fn flat(y: f64) -> ZeroCurve {
        ZeroCurve { beta0: y, beta1: 0.0, beta2: 0.0, beta3: 0.0, tau1: 1.0, tau2: 1.0 }
    }

    /// From the parameters of one day of the GSW file (BETA in percent, TAU in years).
    pub fn from_forwards(f: &Forwards) -> Result<ZeroCurve, String> {
        let get = |n: &str| f.parameters.get(n).copied();
        let need = |n: &str| get(n).ok_or_else(|| format!("GSW row {} has no {n}", f.date));
        let tau1 = need("TAU1")?;
        if tau1 <= 0.0 {
            return Err(format!("GSW row {} has TAU1 = {tau1}", f.date));
        }
        Ok(ZeroCurve {
            beta0: need("BETA0")? / 100.0,
            beta1: need("BETA1")? / 100.0,
            beta2: need("BETA2")? / 100.0,
            beta3: get("BETA3").unwrap_or(0.0) / 100.0,
            tau1,
            tau2: get("TAU2").filter(|t| *t > 0.0).unwrap_or(1.0),
        })
    }

    /// Zero-coupon yield at maturity `t` years (the limit at t = 0 is the short rate).
    pub fn zero_yield(&self, t: f64) -> f64 {
        if t < 1e-8 {
            return self.beta0 + self.beta1;
        }
        let (x1, x2) = (t / self.tau1, t / self.tau2);
        let (e1, e2) = ((-x1).exp(), (-x2).exp());
        let (g1, g2) = ((1.0 - e1) / x1, (1.0 - e2) / x2);
        self.beta0 + self.beta1 * g1 + self.beta2 * (g1 - e1) + self.beta3 * (g2 - e2)
    }

    /// Instantaneous forward rate at horizon `t`.
    pub fn forward(&self, t: f64) -> f64 {
        let (x1, x2) = (t / self.tau1, t / self.tau2);
        self.beta0 + self.beta1 * (-x1).exp() + self.beta2 * x1 * (-x1).exp() + self.beta3 * x2 * (-x2).exp()
    }

    /// Discount factor P(0, t).
    pub fn discount(&self, t: f64) -> f64 {
        (-t * self.zero_yield(t)).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ZeroCurve {
        // The size of a typical 2020s curve (percent / 100).
        ZeroCurve { beta0: 0.045, beta1: -0.02, beta2: 0.01, beta3: -0.015, tau1: 2.0, tau2: 8.0 }
    }

    #[test]
    fn forward_is_the_derivative_of_total_yield() {
        let c = sample();
        for t in [0.5, 1.0, 5.0, 20.0] {
            let h = 1e-5;
            let d = ((t + h) * c.zero_yield(t + h) - (t - h) * c.zero_yield(t - h)) / (2.0 * h);
            assert!((d - c.forward(t)).abs() < 1e-8, "t={t}");
        }
    }

    #[test]
    fn short_end_and_flat() {
        let c = sample();
        assert!((c.zero_yield(0.0) - 0.025).abs() < 1e-15);
        assert!((c.zero_yield(1e-6) - 0.025).abs() < 1e-6);
        assert!((c.forward(0.0) - 0.025).abs() < 1e-15);
        let f = ZeroCurve::flat(0.03);
        assert!((f.discount(10.0) - (-0.3f64).exp()).abs() < 1e-14);
    }
}
