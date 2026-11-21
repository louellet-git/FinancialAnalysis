//! The engine: the paper simulator, the risk stops, the journal and the daily summary.
//!
//! Simulator rules (stated, all in memory):
//! - A buy fills at `mid + spread_share * half spread` (1.0 = the ask), a sell at `mid - ...` (1.0 = the bid).
//! - An order fills on the first input whose `read_ms` is at least `decision_ms + latency_ms`, at the bid and
//!   ask of THAT input. A limit that the fill price breaks cancels the order. No partial fills, no queue,
//!   no market impact.
//! - Fee: `fee` USD for each share. P&L is marked at the mid. Cash starts at 0, so equity is the net P&L.
//! - A stop (kill file, end time, max loss, end of input) cancels the pending orders and closes the
//!   position at once at the present input, with no latency.
//!
//! Risk order for each input: kill file, end time, fills that are due, mark and max loss, strategy.

use super::{Input, Intent, Side, Strategy};
use crate::market::alpaca::ny_day;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Config {
    pub max_pos: i64,
    pub max_loss: f64,
    pub spread_share: f64,
    pub latency_ms: i64,
    pub fee: f64,
    /// Seconds of the New York day.
    pub until_secs: i64,
    pub stop_file: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    Kill,
    Until,
    MaxLoss,
    EndOfInput,
}

impl Stop {
    pub fn label(self) -> &'static str {
        match self {
            Stop::Kill => "kill file",
            Stop::Until => "end time",
            Stop::MaxLoss => "max loss",
            Stop::EndOfInput => "end of input",
        }
    }
}

struct Pending {
    side: Side,
    qty: i64,
    limit: Option<f64>,
    decision_ms: i64,
}

/// What a replay must reproduce: the decisions, without the measured times.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub decision_ms: i64,
    pub side: Side,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub strategy: String,
    pub inputs: usize,
    pub decisions: usize,
    pub rejected: usize,
    pub fills: usize,
    pub pnl_gross: f64,
    pub fees: f64,
    pub pnl_net: f64,
    pub max_drawdown: f64,
    pub final_pos: i64,
    pub compute_median_ms: f64,
    pub compute_p99_ms: f64,
    pub stop: Stop,
}

impl Summary {
    pub fn to_json(&self) -> Value {
        json!({
            "strategy": self.strategy, "inputs": self.inputs, "decisions": self.decisions, "rejected": self.rejected,
            "fills": self.fills, "pnl_gross": self.pnl_gross, "fees": self.fees, "pnl_net": self.pnl_net,
            "max_drawdown": self.max_drawdown, "final_pos": self.final_pos,
            "decision_latency_median_ms": self.compute_median_ms, "decision_latency_p99_ms": self.compute_p99_ms,
            "stop": self.stop.label(), "note": "simulated fills, model prices, a pipe test"
        })
    }
    pub fn text(&self) -> String {
        format!(
            "summary {}: inputs {}, decisions {}, rejected {}, fills {}, P&L gross {:.2} USD, fees {:.2} USD, P&L net {:.2} USD, max drawdown {:.2} USD, final position {}, decision latency median {:.3} ms p99 {:.3} ms, stop: {}",
            self.strategy, self.inputs, self.decisions, self.rejected, self.fills, self.pnl_gross, self.fees, self.pnl_net, self.max_drawdown, self.final_pos, self.compute_median_ms, self.compute_p99_ms, self.stop.label()
        )
    }
}

pub struct Engine<S: Strategy> {
    cfg: Config,
    strat: S,
    pos: i64,
    cash: f64,
    fees: f64,
    pending: Vec<Pending>,
    inputs: usize,
    fills: usize,
    rejected: usize,
    decisions: Vec<Decision>,
    compute_ms: Vec<f64>,
    peak: f64,
    max_dd: f64,
    last: Option<Input>,
    pub stopped: Option<Stop>,
    pub journal: Vec<Value>,
    sink: Option<Box<dyn Write>>,
}

impl<S: Strategy> Engine<S> {
    pub fn new(cfg: Config, strat: S, sink: Option<Box<dyn Write>>) -> Self {
        Engine { cfg, strat, pos: 0, cash: 0.0, fees: 0.0, pending: vec![], inputs: 0, fills: 0, rejected: 0, decisions: vec![], compute_ms: vec![], peak: 0.0, max_dd: 0.0, last: None, stopped: None, journal: vec![], sink }
    }

    pub fn decisions(&self) -> &[Decision] {
        &self.decisions
    }
    pub fn position(&self) -> i64 {
        self.pos
    }

    fn write(&mut self, v: Value) {
        if let Some(s) = self.sink.as_mut() {
            let _ = writeln!(s, "{v}");
            let _ = s.flush();
        }
        self.journal.push(v);
    }

    fn pnl_at(&self, mid: f64) -> f64 {
        self.cash + self.pos as f64 * mid
    }

    fn mark(&mut self, mid: f64) -> f64 {
        let pnl = self.pnl_at(mid);
        self.peak = self.peak.max(pnl);
        self.max_dd = self.max_dd.max(self.peak - pnl);
        pnl
    }

    fn fill(&mut self, side: Side, qty: i64, limit: Option<f64>, decision_ms: i64, inp: &Input, why: &str) {
        let half = (inp.ask - inp.bid) / 2.0;
        let price = inp.mid() + side.sign() as f64 * self.cfg.spread_share * half;
        let broken = limit.is_some_and(|l| (side == Side::Buy && price > l) || (side == Side::Sell && price < l));
        let side_text = format!("{side:?}").to_lowercase();
        if broken {
            self.write(json!({"kind": "cancel", "reason": "limit", "quote_ms": inp.quote_ms, "decision_ms": decision_ms, "side": side_text, "qty": qty, "limit": limit, "fill_price": price}));
            return;
        }
        let fee = self.cfg.fee * qty as f64;
        self.cash -= side.sign() as f64 * qty as f64 * price + fee;
        self.fees += fee;
        self.pos += side.sign() * qty;
        self.fills += 1;
        let pnl = self.mark(inp.mid());
        self.write(json!({
            "kind": "fill", "why": why, "quote_ms": inp.quote_ms, "decision_ms": decision_ms, "fill_ms": inp.read_ms,
            "latency_ms": inp.read_ms - decision_ms, "side": side_text, "qty": qty, "fill_price": price,
            "bid": inp.bid, "ask": inp.ask, "fee": fee, "pos": self.pos, "cash": self.cash, "pnl": pnl
        }));
    }

    fn fill_due(&mut self, inp: &Input) {
        let (due, rest): (Vec<Pending>, Vec<Pending>) = std::mem::take(&mut self.pending).into_iter().partition(|p| p.decision_ms + self.cfg.latency_ms <= inp.read_ms);
        self.pending = rest;
        for p in due {
            self.fill(p.side, p.qty, p.limit, p.decision_ms, inp, "order");
        }
    }

    fn flatten(&mut self, inp: &Input, why: Stop) {
        if !self.pending.is_empty() {
            let n = self.pending.len();
            self.pending.clear();
            self.write(json!({"kind": "cancel", "reason": "stop", "orders": n, "quote_ms": inp.quote_ms}));
        }
        if self.pos != 0 {
            let side = if self.pos > 0 { Side::Sell } else { Side::Buy };
            self.fill(side, self.pos.abs(), None, inp.read_ms, inp, why.label());
        }
        self.mark(inp.mid());
        self.stopped = Some(why);
        self.write(json!({"kind": "stop", "reason": why.label(), "quote_ms": inp.quote_ms, "pos": self.pos, "pnl": self.pnl_at(inp.mid())}));
    }

    /// Process one input. Returns `false` when the engine has stopped.
    pub fn on_input(&mut self, inp: &Input) -> bool {
        if self.stopped.is_some() {
            return false;
        }
        if !inp.valid() {
            self.write(json!({"kind": "bad_input", "quote_ms": inp.quote_ms, "bid": inp.bid, "ask": inp.ask}));
            return true;
        }
        self.inputs += 1;
        self.last = Some(inp.clone());
        if self.cfg.stop_file.exists() {
            self.flatten(inp, Stop::Kill);
            return false;
        }
        if ny_day(inp.read_ms.div_euclid(1000)).1 >= self.cfg.until_secs {
            self.flatten(inp, Stop::Until);
            return false;
        }
        self.fill_due(inp);
        let pnl = self.mark(inp.mid());
        if pnl <= -self.cfg.max_loss {
            self.flatten(inp, Stop::MaxLoss);
            return false;
        }
        let projected = self.pos + self.pending.iter().map(|p| p.side.sign() * p.qty).sum::<i64>();
        let t0 = Instant::now();
        let intents = self.strat.on_input(inp, projected);
        let compute = t0.elapsed().as_secs_f64() * 1000.0;
        self.compute_ms.push(compute);
        let mut proj = projected;
        for Intent { side, qty, limit } in intents {
            let after = proj + side.sign() * qty;
            // The max position clips an order that raises the size of the position.
            let allowed = if after.abs() > proj.abs() { qty - (after.abs() - self.cfg.max_pos).max(0) } else { qty };
            let side_text = format!("{side:?}").to_lowercase();
            if qty <= 0 || allowed <= 0 {
                self.rejected += 1;
                self.write(json!({"kind": "reject", "reason": "max position", "quote_ms": inp.quote_ms, "decision_ms": inp.read_ms, "side": side_text, "qty": qty, "pos": self.pos}));
                continue;
            }
            proj += side.sign() * allowed;
            self.decisions.push(Decision { decision_ms: inp.read_ms, side, qty: allowed });
            self.write(json!({
                "kind": "decision", "quote_ms": inp.quote_ms, "decision_ms": inp.read_ms, "feed_latency_ms": inp.read_ms - inp.quote_ms,
                "compute_ms": compute, "side": side_text, "qty": allowed, "requested_qty": qty, "limit": limit,
                "bid": inp.bid, "ask": inp.ask, "pos": self.pos, "cash": self.cash, "pnl": pnl
            }));
            self.pending.push(Pending { side, qty: allowed, limit, decision_ms: inp.read_ms });
        }
        self.fill_due(inp);
        true
    }

    /// Close at the end of the input (the engine holds nothing overnight) and make the summary.
    pub fn finish(&mut self) -> Summary {
        if self.stopped.is_none() {
            if let Some(last) = self.last.clone() {
                self.flatten(&last, Stop::EndOfInput);
            } else {
                self.stopped = Some(Stop::EndOfInput);
            }
        }
        let mid = self.last.as_ref().map(|l| l.mid()).unwrap_or(0.0);
        let net = self.pnl_at(mid);
        let mut c = self.compute_ms.clone();
        c.sort_by(|a, b| a.total_cmp(b));
        let pct = |p: f64| if c.is_empty() { 0.0 } else { c[((p * c.len() as f64).ceil() as usize).clamp(1, c.len()) - 1] };
        Summary {
            strategy: self.strat.name().into(),
            inputs: self.inputs,
            decisions: self.decisions.len(),
            rejected: self.rejected,
            fills: self.fills,
            pnl_gross: net + self.fees,
            fees: self.fees,
            pnl_net: net,
            max_drawdown: self.max_dd,
            final_pos: self.pos,
            compute_median_ms: pct(0.5),
            compute_p99_ms: pct(0.99),
            stop: self.stopped.unwrap_or(Stop::EndOfInput),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A strategy with a fixed script: (read_ms minute, intent).
    struct Script(Vec<(i64, Intent)>);
    impl Strategy for Script {
        fn name(&self) -> &'static str {
            "script"
        }
        fn params(&self) -> String {
            String::new()
        }
        fn on_input(&mut self, i: &Input, _pos: i64) -> Vec<Intent> {
            self.0.iter().filter(|(m, _)| *m == i.read_ms).map(|(_, x)| *x).collect()
        }
    }

    const T0: i64 = 1_790_000_000_000; // 2026-09-21 14:13:20 UTC = 10:13:20 New York, inside the session
    fn inp(ms: i64, bid: f64, ask: f64) -> Input {
        Input { quote_ms: ms - 50, read_ms: ms, symbol: "SPY".into(), bid, ask, kind: "quote".into() }
    }
    fn cfg() -> Config {
        Config { max_pos: 100, max_loss: 1000.0, spread_share: 1.0, latency_ms: 0, fee: 0.01, until_secs: 15 * 3600 + 55 * 60, stop_file: std::env::temp_dir().join("fa-algo-no-such-stop-file") }
    }
    fn buy(q: i64) -> Intent {
        Intent { side: Side::Buy, qty: q, limit: None }
    }
    fn sell(q: i64) -> Intent {
        Intent { side: Side::Sell, qty: q, limit: None }
    }

    #[test]
    fn buy_at_ask_sell_at_bid_with_fees() {
        let mut e = Engine::new(cfg(), Script(vec![(T0, buy(10)), (T0 + 1000, sell(10))]), None);
        e.on_input(&inp(T0, 99.90, 100.10)); // buy 10 at 100.10, fee 0.10
        assert_eq!(e.position(), 10);
        e.on_input(&inp(T0 + 1000, 100.40, 100.60)); // sell 10 at 100.40, fee 0.10
        assert_eq!(e.position(), 0);
        let s = e.finish();
        // net = -1001.0 - 0.1 + 1004.0 - 0.1 = 2.8 ; gross = 3.0
        assert!((s.pnl_net - 2.8).abs() < 1e-9, "{}", s.pnl_net);
        assert!((s.pnl_gross - 3.0).abs() < 1e-9);
        assert!((s.fees - 0.2).abs() < 1e-9);
        assert_eq!((s.fills, s.decisions), (2, 2));
    }

    #[test]
    fn spread_share_half_fills_between_mid_and_ask() {
        let mut c = cfg();
        c.spread_share = 0.5;
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        e.on_input(&inp(T0, 99.90, 100.10));
        let fill = e.journal.iter().find(|v| v["kind"] == "fill").unwrap();
        assert!((fill["fill_price"].as_f64().unwrap() - 100.05).abs() < 1e-9);
    }

    #[test]
    fn latency_fills_on_a_later_input() {
        let mut c = cfg();
        c.latency_ms = 500;
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        e.on_input(&inp(T0, 99.90, 100.10));
        assert_eq!(e.position(), 0);
        e.on_input(&inp(T0 + 400, 99.90, 100.10));
        assert_eq!(e.position(), 0);
        e.on_input(&inp(T0 + 600, 100.90, 101.10)); // fills at the ask of THIS input
        assert_eq!(e.position(), 10);
        let fill = e.journal.iter().find(|v| v["kind"] == "fill").unwrap();
        assert_eq!(fill["latency_ms"], 600);
        assert!((fill["fill_price"].as_f64().unwrap() - 101.10).abs() < 1e-9);
    }

    #[test]
    fn limit_cancels_when_broken() {
        let mut e = Engine::new(cfg(), Script(vec![(T0, Intent { side: Side::Buy, qty: 5, limit: Some(100.0) })]), None);
        e.on_input(&inp(T0, 99.90, 100.10));
        assert_eq!(e.position(), 0);
        assert!(e.journal.iter().any(|v| v["kind"] == "cancel"));
    }

    #[test]
    fn max_position_clips_and_rejects() {
        let mut e = Engine::new(cfg(), Script(vec![(T0, buy(80)), (T0 + 1000, buy(80)), (T0 + 2000, buy(10))]), None);
        e.on_input(&inp(T0, 99.9, 100.1));
        e.on_input(&inp(T0 + 1000, 99.9, 100.1)); // clipped to 20
        assert_eq!(e.position(), 100);
        e.on_input(&inp(T0 + 2000, 99.9, 100.1)); // rejected
        assert_eq!(e.position(), 100);
        let s = e.finish();
        assert_eq!(s.rejected, 1);
        assert_eq!(e.decisions()[1].qty, 20);
    }

    #[test]
    fn max_loss_flattens_and_stops() {
        let mut c = cfg();
        c.max_loss = 50.0;
        c.fee = 0.0;
        let mut e = Engine::new(c, Script(vec![(T0, buy(100))]), None);
        assert!(e.on_input(&inp(T0, 99.9, 100.1))); // long 100 at 100.10
        assert!(e.on_input(&inp(T0 + 1000, 99.7, 99.9))); // mid 99.8: pnl -30 (mid vs fill 100.10 = -30)
        assert!(!e.on_input(&inp(T0 + 2000, 99.5, 99.7))); // mid 99.6: pnl -50 -> stop
        assert_eq!(e.position(), 0);
        assert_eq!(e.stopped, Some(Stop::MaxLoss));
        assert!(!e.on_input(&inp(T0 + 3000, 99.5, 99.7)));
        let s = e.finish();
        assert!((s.pnl_net - -60.0).abs() < 1e-6, "{}", s.pnl_net); // sold at the bid 99.5
    }

    #[test]
    fn until_stops_and_flattens() {
        let mut c = cfg();
        // T0 is 10:13:20 New York (EDT); stop at 10:14.
        c.until_secs = 10 * 3600 + 14 * 60;
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        assert!(e.on_input(&inp(T0, 99.9, 100.1)));
        assert!(!e.on_input(&inp(T0 + 41_000, 99.9, 100.1))); // 10:14:01
        assert_eq!((e.position(), e.stopped), (0, Some(Stop::Until)));
    }

    #[test]
    fn kill_file_flattens_and_stops() {
        let path = std::env::temp_dir().join(format!("fa-algo-stop-{}", std::process::id()));
        let mut c = cfg();
        c.stop_file = path.clone();
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        assert!(e.on_input(&inp(T0, 99.9, 100.1)));
        std::fs::write(&path, "stop").unwrap();
        let go = e.on_input(&inp(T0 + 1000, 99.9, 100.1));
        std::fs::remove_file(&path).unwrap();
        assert!(!go);
        assert_eq!((e.position(), e.stopped), (0, Some(Stop::Kill)));
    }

    #[test]
    fn drawdown_and_latency_summary() {
        let mut c = cfg();
        c.fee = 0.0;
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        e.on_input(&inp(T0, 100.0, 100.0)); // long 10 at 100
        e.on_input(&inp(T0 + 1000, 102.0, 102.0)); // +20
        e.on_input(&inp(T0 + 2000, 99.0, 99.0)); // -10, drawdown 30
        let s = e.finish();
        assert!((s.max_drawdown - 30.0).abs() < 1e-9);
        assert!(s.compute_p99_ms >= s.compute_median_ms);
    }
}
