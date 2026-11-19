//! s20 - Walk-forward L2-logistic model of the next 5-day direction, high confidence only (SPY, daily bars).
//!
//! About every 21 sessions the model is fitted again on samples whose 5-day outcome was already
//! known at that bar (sample t is used only when t + 5 <= now). The standardization uses the
//! training window only. The bot buys a 5-DTE $1-wide debit spread in the predicted direction when
//! P(up) >= 0.62 (call spread) or <= 0.38 (put spread).
//!
//! The fit is Newton's method on the penalized log-likelihood (the intercept is not penalized). It
//! uses plain loops, so its sums add in another order than numpy/BLAS: the weights agree to about 1e-12.

use super::tail;
use crate::trading::data::Interval;
use crate::trading::engine::Ctx;
use crate::trading::stats;
use crate::trading::strategy::{ExitRules, Strategy};
use crate::trading::structures::vertical;

const HORIZON: usize = 5;
const LAMBDA: f64 = 300.0;
const RETRAIN: usize = 21;
const MIN_TRAIN: usize = 750;
const P_HI: f64 = 0.62;
const P_LO: f64 = 0.38;
const DTE: i32 = 5;
const DELTA: f64 = 0.50;
const WIDTH: f64 = 1.0;
const D: usize = 14;
const SPY: &str = "SPY";

type Row = [f64; D];

/// The feature row for the history that ends at the LAST element (no future data); `None` if too short.
pub fn make_features(close: &[f64], vix: &[f64], dow: u32) -> Option<Row> {
    if close.len() < 201 || vix.len() < 6 {
        return None;
    }
    let c = close[close.len() - 1];
    let w = tail(close, 22);
    let lr: Vec<f64> = w.windows(2).map(|p| p[1].ln() - p[0].ln()).collect();
    let rv10 = stats::std(tail(&lr, 10), 1) * 252f64.sqrt();
    let rv20 = stats::std(tail(&lr, 20), 1) * 252f64.sqrt();
    let ret = |k: usize| (c / close[close.len() - 1 - k]).ln();
    let sma = |k: usize| stats::mean(tail(close, k));
    let v = vix[vix.len() - 1];
    Some([
        ret(1),
        ret(3),
        ret(5),
        ret(10),
        ret(21),
        c / sma(10) - 1.0,
        c / sma(50) - 1.0,
        c / sma(200) - 1.0,
        rv10,
        v.ln(),
        (v / vix[vix.len() - 6]).ln(),
        v - rv20,
        if dow == 0 { 1.0 } else { 0.0 },
        if dow == 4 { 1.0 } else { 0.0 },
    ])
}

/// Solve `a x = b` by Gaussian elimination with partial pivoting (LAPACK `gesv` does the same).
#[allow(clippy::needless_range_loop)]
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for col in 0..n {
        let piv = (col..n).max_by(|&i, &j| a[i][col].abs().partial_cmp(&a[j][col].abs()).expect("numbers")).expect("rows");
        a.swap(col, piv);
        b.swap(col, piv);
        for r in col + 1..n {
            let f = a[r][col] / a[col][col];
            for k in col..n {
                a[r][k] -= f * a[col][k];
            }
            b[r] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let s: f64 = (r + 1..n).map(|k| a[r][k] * x[k]).sum();
        x[r] = (b[r] - s) / a[r][r];
    }
    x
}

/// L2 logistic regression by Newton steps; the intercept is not penalized. `x` is standardized.
#[allow(clippy::needless_range_loop)]
fn fit_logistic(x: &[Row], y: &[f64], lam: f64, iters: usize) -> Vec<f64> {
    let n = x.len();
    let d = D + 1;
    let mut w = vec![0.0; d];
    let pen: Vec<f64> = (0..d).map(|k| if k == 0 { 0.0 } else { lam }).collect();
    for _ in 0..iters {
        let mut g = vec![0.0; d];
        let mut h = vec![vec![0.0; d]; d];
        for r in 0..n {
            let mut z = w[0];
            for k in 0..D {
                z += x[r][k] * w[k + 1];
            }
            let z = z.max(-30.0).min(30.0);
            let p = 1.0 / (1.0 + (-z).exp());
            let e = p - y[r];
            let s = p * (1.0 - p);
            g[0] += e;
            h[0][0] += s;
            for a in 0..D {
                g[a + 1] += x[r][a] * e;
                let xa = x[r][a] * s;
                h[0][a + 1] += xa;
                for b in a..D {
                    h[a + 1][b + 1] += xa * x[r][b];
                }
            }
        }
        for a in 0..d {
            for b in 0..a {
                h[a][b] = h[b][a];
            }
            g[a] += pen[a] * w[a];
            h[a][a] += pen[a] + 1e-9;
        }
        let step = solve(h, g);
        for k in 0..d {
            w[k] -= step[k];
        }
    }
    w
}

/// Expanding-window walk-forward fitter. It uses no future data.
struct WalkForwardModel {
    feats: Vec<Option<Row>>, // by bar index
    closes: Vec<f64>,
    w: Option<Vec<f64>>,
    mu: Row,
    sd: Row,
    last_fit: Option<usize>,
}

impl WalkForwardModel {
    fn observe(&mut self, x: Option<Row>, close: f64) {
        self.closes.push(close);
        self.feats.push(x);
    }

    fn maybe_fit(&mut self, i: usize) {
        if let Some(last) = self.last_fit {
            if i - last < RETRAIN {
                return;
            }
        }
        // Only the samples whose outcome is already known at bar i.
        let idx: Vec<usize> = (0..self.feats.len()).filter(|&t| self.feats[t].is_some() && t + HORIZON <= i).collect();
        if idx.len() < MIN_TRAIN {
            return;
        }
        let rows: Vec<Row> = idx.iter().map(|&t| self.feats[t].expect("feature row")).collect();
        let y: Vec<f64> = idx.iter().map(|&t| if self.closes[t + HORIZON] > self.closes[t] { 1.0 } else { 0.0 }).collect();
        let n = rows.len() as f64;
        let mut mu = [0.0; D];
        for r in &rows {
            for k in 0..D {
                mu[k] += r[k];
            }
        }
        for m in mu.iter_mut() {
            *m /= n;
        }
        let mut var = [0.0; D];
        for r in &rows {
            for k in 0..D {
                let dx = r[k] - mu[k];
                var[k] += dx * dx;
            }
        }
        let mut sd = [0.0; D];
        for k in 0..D {
            sd[k] = (var[k] / n).sqrt() + 1e-12;
        }
        let z: Vec<Row> = rows
            .iter()
            .map(|r| {
                let mut o = [0.0; D];
                for k in 0..D {
                    o[k] = (r[k] - mu[k]) / sd[k];
                }
                o
            })
            .collect();
        self.mu = mu;
        self.sd = sd;
        self.w = Some(fit_logistic(&z, &y, LAMBDA, 12));
        self.last_fit = Some(i);
    }

    fn predict(&self, x: Option<&Row>) -> Option<f64> {
        let (w, x) = (self.w.as_ref()?, x?);
        let mut z = w[0];
        for k in 0..D {
            z += (x[k] - self.mu[k]) / self.sd[k] * w[k + 1];
        }
        Some(1.0 / (1.0 + (-z).exp()))
    }
}

pub struct WalkForwardDirection {
    model: WalkForwardModel,
}

impl Default for WalkForwardDirection {
    fn default() -> Self {
        WalkForwardDirection { model: WalkForwardModel { feats: Vec::new(), closes: Vec::new(), w: None, mu: [0.0; D], sd: [1.0; D], last_fit: None } }
    }
}

impl Strategy for WalkForwardDirection {
    fn name(&self) -> &'static str {
        "s20_walkforward_model"
    }

    fn interval(&self) -> Interval {
        Interval::Daily
    }

    fn start(&self) -> Option<&'static str> {
        Some("2005-01-03")
    }

    fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        let close = tail(ctx.bars(SPY).close, 260);
        let vix = tail(ctx.atm_close_history(SPY), 10);
        let x = make_features(close, vix, crate::trading::calendar::weekday(ctx.today()));
        self.model.observe(x, close[close.len() - 1]);
        self.model.maybe_fit(ctx.i);
        let Some(p) = self.model.predict(x.as_ref()) else { return Ok(()) };
        if ctx.open_position_count() > 0 || ctx.pending_entries() > 0 {
            return Ok(());
        }
        let call = if p >= P_HI {
            true
        } else if p <= P_LO {
            false
        } else {
            return Ok(());
        };
        let exp = ctx.expiry(DTE + 1); // fills next session
        let k = ctx.strike_for_delta(SPY, call, DELTA, exp, 1.0);
        let other = if call { k + WIDTH } else { k - WIDTH };
        ctx.open(vertical(SPY, exp, call, k, other)?, ExitRules { exit_dte: Some(0), ..Default::default() }, &format!("{}@{:.2}", if call { "C" } else { "P" }, p), None);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_solver_hand_values() {
        // 2x + y = 5, x + 3y = 10 -> x = 1, y = 3.
        let x = solve(vec![vec![2.0, 1.0], vec![1.0, 3.0]], vec![5.0, 10.0]);
        assert!((x[0] - 1.0).abs() < 1e-12 && (x[1] - 3.0).abs() < 1e-12);
        // A zero pivot needs a row swap.
        let x = solve(vec![vec![0.0, 1.0], vec![1.0, 1.0]], vec![2.0, 3.0]);
        assert!((x[0] - 1.0).abs() < 1e-12 && (x[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn logistic_fit_finds_the_sign_of_the_signal() {
        // One feature decides the outcome; the weight of that feature must be positive and the others small.
        let mut rows = Vec::new();
        let mut y = Vec::new();
        for t in 0..400 {
            let mut r = [0.0; D];
            let s = ((t * 37 % 101) as f64 - 50.0) / 29.0;
            r[0] = s;
            r[1] = ((t * 53 % 89) as f64 - 44.0) / 25.0;
            rows.push(r);
            y.push(if s + 0.3 * r[1] > 0.0 { 1.0 } else { 0.0 });
        }
        let w = fit_logistic(&rows, &y, 1.0, 12);
        assert!(w[1] > 1.0, "{w:?}");
        assert!(w[1] > w[2].abs());
        assert!(w[0].abs() < 0.5);
    }

    #[test]
    fn features_need_enough_history() {
        let close: Vec<f64> = (0..300).map(|k| 100.0 + k as f64 * 0.1).collect();
        let vix = [0.2, 0.21, 0.2, 0.22, 0.2, 0.19, 0.2];
        assert!(make_features(&close[..200], &vix, 0).is_none());
        let f = make_features(&close, &vix, 4).unwrap();
        assert_eq!(f[13], 1.0); // Friday
        assert_eq!(f[12], 0.0);
        assert!((f[0] - (close[299] / close[298]).ln()).abs() < 1e-15);
    }
}
