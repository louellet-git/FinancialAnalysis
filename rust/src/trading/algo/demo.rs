//! The demo strategy. A PIPE TEST, not a claim of profit: a mean reversion on 1-minute closes.
//!
//! The strategy keeps the last mid of each minute (a bar close is known only when the next minute
//! starts, so the signal uses closed bars only). With `WINDOW` closes: z = (last close - mean) / sd.
//! Flat and z < -ENTRY_Z: buy `QTY`. Flat and z > ENTRY_Z: sell `QTY` short. Long and z >= 0, or short
//! and z <= 0: close the position. Replay of 5-minute stored bars gives one close each 5 minutes
//! (the window is then 5 times longer in time).

use super::{Input, Intent, Side, Strategy};
use std::collections::VecDeque;

pub const WINDOW: usize = 20;
pub const ENTRY_Z: f64 = 2.0;
pub const QTY: i64 = 50;

#[derive(Default)]
pub struct Demo {
    closes: VecDeque<f64>,
    minute: Option<i64>,
    last_mid: f64,
}

impl Demo {
    pub fn new() -> Demo {
        Demo::default()
    }

    /// The z-score of the last value against the window, `None` when the window is short or flat.
    pub fn zscore(closes: &[f64]) -> Option<f64> {
        if closes.len() < WINDOW {
            return None;
        }
        let w = &closes[closes.len() - WINDOW..];
        let mean = w.iter().sum::<f64>() / WINDOW as f64;
        let var = w.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / WINDOW as f64;
        let sd = var.sqrt();
        if sd < 1e-9 {
            return None;
        }
        Some((w[WINDOW - 1] - mean) / sd)
    }

    fn signal(z: f64, pos: i64) -> Vec<Intent> {
        let one = |side, qty| vec![Intent { side, qty, limit: None }];
        if pos == 0 {
            if z < -ENTRY_Z {
                return one(Side::Buy, QTY);
            }
            if z > ENTRY_Z {
                return one(Side::Sell, QTY);
            }
        } else if pos > 0 && z >= 0.0 {
            return one(Side::Sell, pos);
        } else if pos < 0 && z <= 0.0 {
            return one(Side::Buy, -pos);
        }
        vec![]
    }
}

impl Strategy for Demo {
    fn name(&self) -> &'static str {
        "demo"
    }
    fn params(&self) -> String {
        format!("mean reversion, window {WINDOW} one-minute closes, enter at |z| > {ENTRY_Z}, exit at z crossing 0, {QTY} shares (pipe test, no claim of profit)")
    }
    fn on_input(&mut self, input: &Input, pos: i64) -> Vec<Intent> {
        let minute = input.read_ms.div_euclid(60_000);
        let mid = input.mid();
        let mut out = vec![];
        if let Some(m) = self.minute {
            if minute > m {
                // The minute m is closed: its last mid is a close.
                self.closes.push_back(self.last_mid);
                while self.closes.len() > WINDOW {
                    self.closes.pop_front();
                }
                let v: Vec<f64> = self.closes.iter().copied().collect();
                if let Some(z) = Demo::zscore(&v) {
                    out = Demo::signal(z, pos);
                }
            }
        }
        self.minute = Some(minute);
        self.last_mid = mid;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(min: i64, mid: f64) -> Input {
        Input { quote_ms: min * 60_000, read_ms: min * 60_000, symbol: "SPY".into(), bid: mid - 0.005, ask: mid + 0.005, kind: "bar".into() }
    }

    #[test]
    fn zscore_fixed_answer() {
        let mut v = vec![100.0; 19];
        v.push(110.0);
        // mean 100.5, var = (19*0.25 + 90.25)/20 = 4.75 -> sd 2.17945, z = 9.5/2.17945
        let z = Demo::zscore(&v).unwrap();
        assert!((z - 4.3589).abs() < 1e-3, "{z}");
        assert!(Demo::zscore(&v[..10]).is_none());
        assert!(Demo::zscore(&[5.0; 20]).is_none());
    }

    #[test]
    fn signal_on_a_fixed_series() {
        let mut d = Demo::new();
        let mut pos = 0;
        let mut seen = vec![];
        // 22 flat-ish minutes (alternating 100.00 and 100.02), then a drop to 99 for one minute, then a return.
        for m in 0..22 {
            let p = if m % 2 == 0 { 100.0 } else { 100.02 };
            assert!(d.on_input(&tick(m, p), pos).is_empty());
        }
        seen.extend(d.on_input(&tick(22, 99.0), pos)); // closes minute 21: still normal, no signal
        assert!(seen.is_empty());
        let buy = d.on_input(&tick(23, 99.0), pos); // closes minute 22 at 99.0: z very negative
        assert_eq!(buy, vec![Intent { side: Side::Buy, qty: QTY, limit: None }]);
        pos = QTY;
        // The same minute again gives no second signal.
        assert!(d.on_input(&tick(23, 99.0), pos).is_empty());
        // Minute 24 closes minute 23 at 99.0: the mean moves down, z is still < 0: hold.
        let hold = d.on_input(&tick(24, 100.5), pos);
        assert!(hold.is_empty(), "{hold:?}");
        // Minute 25 closes minute 24 at 100.5: above the mean: exit.
        let sell = d.on_input(&tick(25, 100.5), pos);
        assert_eq!(sell, vec![Intent { side: Side::Sell, qty: QTY, limit: None }]);
    }
}
