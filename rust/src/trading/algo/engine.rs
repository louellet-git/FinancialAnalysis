//! The engine: the paper simulator, the risk stops, the journal and the daily summary.
//!
//! Simulator rules (stated, all in memory):
//! - A buy fills at `mid + spread_share * half spread` (1.0 = the ask), a sell at `mid - ...` (1.0 = the bid).
//! - An order fills on the first LATER input whose `read_ms` is at least `decision_ms + latency_ms`, at the bid and
//!   ask of THAT input (never at the input of the decision). A limit that the fill price breaks cancels the order.
//!   No partial fills, no queue, no market impact.
//! - Bar mode: the strategy decides on a bar close (kind `bar`) and the order fills at the open of the next bar
//!   (kind `bar_open`). A bar has no time inside it, so `latency_ms` does not apply: the real latency is the time
//!   from the close to the next open (0 for bars that follow each other). The `bar_open` inputs only fill, mark
//!   and check the stops: the strategy does not see them.
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
    /// The name of the data source, printed in the summary (for example `alpaca iex quotes`).
    pub feed: String,
}

impl Config {
    pub fn to_json(&self) -> Value {
        json!({"max_pos": self.max_pos, "max_loss": self.max_loss, "spread_share": self.spread_share, "latency_ms": self.latency_ms,
               "fee": self.fee, "until_secs_new_york": self.until_secs, "stop_file": self.stop_file.display().to_string(), "feed": self.feed})
    }
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
    /// The sequence number of the input of the decision: an order never fills on it.
    seq: usize,
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
    pub config: Value,
    pub feed: String,
}

impl Summary {
    pub fn to_json(&self) -> Value {
        json!({
            "strategy": self.strategy, "inputs": self.inputs, "decisions": self.decisions, "rejected": self.rejected,
            "fills": self.fills, "pnl_gross": self.pnl_gross, "fees": self.fees, "pnl_net": self.pnl_net,
            "max_drawdown": self.max_drawdown, "final_pos": self.final_pos,
            "decision_latency_median_ms": self.compute_median_ms, "decision_latency_p99_ms": self.compute_p99_ms,
            "stop": self.stop.label(), "note": "simulated fills, model prices, a pipe test", "feed": self.feed, "config": self.config,
            "limits": super::LIMITS, "loss_stop": "approximate"
        })
    }
    pub fn text(&self) -> String {
        format!(
            "summary {} (feed: {}): inputs {}, decisions {}, rejected {}, fills {}, P&L gross (simulated) {:.2} USD, fees {:.2} USD, P&L net (simulated) {:.2} USD, max drawdown (simulated) {:.2} USD, final position {}, decision latency median {:.3} ms p99 {:.3} ms, stop: {}\n{}",
            self.strategy, self.feed, self.inputs, self.decisions, self.rejected, self.fills, self.pnl_gross, self.fees, self.pnl_net, self.max_drawdown, self.final_pos, self.compute_median_ms, self.compute_p99_ms, self.stop.label(), super::LIMITS
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
            "kind": "fill", "why": why, "input_kind": inp.kind, "quote_ms": inp.quote_ms, "decision_ms": decision_ms, "fill_ms": inp.read_ms,
            "latency_ms": inp.read_ms - decision_ms, "side": side_text, "qty": qty, "fill_price": price,
            "bid": inp.bid, "ask": inp.ask, "fee": fee, "pos": self.pos, "cash": self.cash, "pnl": pnl
        }));
    }

    fn fill_due(&mut self, inp: &Input) {
        let (latency, seq) = (self.cfg.latency_ms, self.inputs);
        // A bar close never fills an order. A bar open fills it at once (a bar has no time inside).
        let eligible = |p: &Pending| p.seq < seq && inp.kind != "bar" && (inp.kind == "bar_open" || p.decision_ms + latency <= inp.read_ms);
        let (due, rest): (Vec<Pending>, Vec<Pending>) = std::mem::take(&mut self.pending).into_iter().partition(|p| eligible(p));
        self.pending = rest;
        for p in due {
            self.fill(p.side, p.qty, p.limit, p.decision_ms, inp, "order");
        }
    }

    fn flatten(&mut self, inp: &Input, why: Stop) {
        if !self.pending.is_empty() {
            let n = self.pending.len();
            self.pending.clear();
            self.write(json!({"kind": "cancel", "reason": "stop", "cancelled": n, "quote_ms": inp.quote_ms}));
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
        if inp.kind == "bar_open" {
            return true;
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
            self.pending.push(Pending { side, qty: allowed, limit, decision_ms: inp.read_ms, seq: self.inputs });
        }
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
            config: self.cfg.to_json(),
            feed: self.cfg.feed.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A strategy with a fixed script: (read_ms, intent).
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
        Config { max_pos: 100, max_loss: 1000.0, spread_share: 1.0, latency_ms: 0, fee: 0.01, until_secs: 15 * 3600 + 55 * 60, stop_file: std::env::temp_dir().join("fa-algo-no-such-stop-file"), feed: "test".into() }
    }
    fn buy(q: i64) -> Intent {
        Intent { side: Side::Buy, qty: q, limit: None }
    }
    fn sell(q: i64) -> Intent {
        Intent { side: Side::Sell, qty: q, limit: None }
    }

    #[test]
    fn buy_at_ask_sell_at_bid_with_fees_and_never_at_the_deciding_input() {
        let mut e = Engine::new(cfg(), Script(vec![(T0, buy(10)), (T0 + 1000, sell(10))]), None);
        e.on_input(&inp(T0, 99.90, 100.10)); // decision, latency 0: still no fill on this input
        assert_eq!(e.position(), 0);
        e.on_input(&inp(T0 + 500, 99.90, 100.10)); // buy 10 at 100.10, fee 0.10
        assert_eq!(e.position(), 10);
        e.on_input(&inp(T0 + 1000, 100.40, 100.60)); // decision to sell
        e.on_input(&inp(T0 + 1500, 100.40, 100.60)); // sell 10 at 100.40, fee 0.10
        assert_eq!(e.position(), 0);
        let s = e.finish();
        // net = -1001.0 - 0.1 + 1004.0 - 0.1 = 2.8 ; gross = 3.0
        assert!((s.pnl_net - 2.8).abs() < 1e-9, "{}", s.pnl_net);
        assert!((s.pnl_gross - 3.0).abs() < 1e-9);
        assert!((s.fees - 0.2).abs() < 1e-9);
        assert_eq!((s.fills, s.decisions), (2, 2));
        assert!(s.text().contains("(simulated)") && s.text().contains("not the NBBO") && s.text().contains("feed: test"));
        assert_eq!(s.to_json()["config"]["max_pos"], 100);
    }

    #[test]
    fn spread_share_half_fills_between_mid_and_ask() {
        let mut c = cfg();
        c.spread_share = 0.5;
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        e.on_input(&inp(T0, 99.90, 100.10));
        e.on_input(&inp(T0 + 100, 99.90, 100.10));
        let fill = e.journal.iter().find(|v| v["kind"] == "fill").unwrap();
        assert!((fill["fill_price"].as_f64().unwrap() - 100.05).abs() < 1e-9);
    }

    #[test]
    fn latency_fills_on_a_later_input() {
        let mut c = cfg();
        c.latency_ms = 500;
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        e.on_input(&inp(T0, 99.90, 100.10));
        e.on_input(&inp(T0 + 400, 99.90, 100.10));
        assert_eq!(e.position(), 0);
        e.on_input(&inp(T0 + 600, 100.90, 101.10)); // fills at the ask of THIS input
        assert_eq!(e.position(), 10);
        let fill = e.journal.iter().find(|v| v["kind"] == "fill").unwrap();
        assert_eq!(fill["latency_ms"], 600);
        assert!((fill["fill_price"].as_f64().unwrap() - 101.10).abs() < 1e-9);
    }

    fn bar(ms: i64, px: f64, kind: &str) -> Input {
        Input { quote_ms: ms, read_ms: ms, symbol: "SPY".into(), bid: px - 0.005, ask: px + 0.005, kind: kind.into() }
    }

    #[test]
    fn bar_mode_fills_at_the_next_bar_open_for_any_latency() {
        let t = T0;
        let day = [bar(t, 100.0, "bar"), bar(t, 100.8, "bar_open"), bar(t + 300_000, 101.0, "bar")];
        for latency in [0, 250, 5000] {
            let mut c = cfg();
            c.latency_ms = latency;
            c.fee = 0.0;
            let mut e = Engine::new(c, Script(vec![(t, buy(10))]), None);
            e.on_input(&day[0]);
            assert_eq!(e.position(), 0, "a bar close never fills (latency {latency})");
            e.on_input(&day[1]);
            assert_eq!(e.position(), 10);
            let fill = e.journal.iter().find(|v| v["kind"] == "fill").unwrap();
            assert!((fill["fill_price"].as_f64().unwrap() - 100.805).abs() < 1e-9, "the ask of the next open");
            assert_eq!((fill["input_kind"].as_str().unwrap(), fill["latency_ms"].as_i64().unwrap()), ("bar_open", 0));
            // The strategy did not see the open input: only one decision exists.
            e.on_input(&day[2]);
            assert_eq!(e.finish().decisions, 1);
        }
    }

    #[test]
    fn limit_cancels_when_broken() {
        let mut e = Engine::new(cfg(), Script(vec![(T0, Intent { side: Side::Buy, qty: 5, limit: Some(100.0) })]), None);
        e.on_input(&inp(T0, 99.90, 100.10));
        e.on_input(&inp(T0 + 100, 99.90, 100.10));
        assert_eq!(e.position(), 0);
        assert!(e.journal.iter().any(|v| v["kind"] == "cancel"));
    }

    #[test]
    fn max_position_clips_and_rejects() {
        let mut e = Engine::new(cfg(), Script(vec![(T0, buy(80)), (T0 + 1000, buy(80)), (T0 + 2000, buy(10))]), None);
        e.on_input(&inp(T0, 99.9, 100.1));
        e.on_input(&inp(T0 + 1000, 99.9, 100.1)); // the first order fills; the second is clipped to 20
        assert_eq!(e.position(), 80);
        e.on_input(&inp(T0 + 2000, 99.9, 100.1)); // the clipped order fills (100); a buy of 10 is rejected
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
        assert!(e.on_input(&inp(T0, 99.9, 100.1)));
        assert!(e.on_input(&inp(T0 + 1000, 99.9, 100.1))); // long 100 at 100.10, mid 100.0: -10
        assert!(e.on_input(&inp(T0 + 2000, 99.7, 99.9))); // mid 99.8: -30
        assert!(!e.on_input(&inp(T0 + 3000, 99.5, 99.7))); // mid 99.6: -50 -> stop
        assert_eq!(e.position(), 0);
        assert_eq!(e.stopped, Some(Stop::MaxLoss));
        assert!(!e.on_input(&inp(T0 + 4000, 99.5, 99.7)));
        let s = e.finish();
        assert!((s.pnl_net - -60.0).abs() < 1e-6, "{}", s.pnl_net); // sold at the bid 99.5
        assert!(s.text().contains("approximate"));
    }

    #[test]
    fn until_stops_and_flattens() {
        let mut c = cfg();
        // T0 is 10:13:20 New York (EDT); stop at 10:14.
        c.until_secs = 10 * 3600 + 14 * 60;
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        assert!(e.on_input(&inp(T0, 99.9, 100.1)));
        assert!(e.on_input(&inp(T0 + 1000, 99.9, 100.1)));
        assert_eq!(e.position(), 10);
        assert!(!e.on_input(&inp(T0 + 41_000, 99.9, 100.1))); // 10:14:01
        assert_eq!((e.position(), e.stopped), (0, Some(Stop::Until)));
    }

    /// The 15:55 New York stop across the two time changes of 2026 (EST is UTC-5, EDT is UTC-4).
    #[test]
    fn the_end_time_follows_new_york_time_across_dst() {
        use crate::trading::calendar::days_from_civil;
        // (date, UTC hour of 15:55 New York)
        for (y, m, d, utc_hour) in [(2026, 3, 6, 20), (2026, 3, 9, 19), (2026, 10, 30, 19), (2026, 11, 2, 20)] {
            let t = days_from_civil(y, m, d) as i64 * 86_400_000 + utc_hour * 3_600_000 + 55 * 60_000;
            let mut e = Engine::new(cfg(), Script(vec![]), None);
            assert!(e.on_input(&inp(t - 1000, 99.9, 100.1)), "{y}-{m}-{d}: 15:54:59 runs");
            assert!(!e.on_input(&inp(t, 99.9, 100.1)), "{y}-{m}-{d}: 15:55:00 stops");
            assert_eq!(e.stopped, Some(Stop::Until));
        }
    }

    #[test]
    fn kill_file_flattens_and_stops() {
        let path = std::env::temp_dir().join(format!("fa-algo-stop-{}", std::process::id()));
        let mut c = cfg();
        c.stop_file = path.clone();
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        assert!(e.on_input(&inp(T0, 99.9, 100.1)));
        assert!(e.on_input(&inp(T0 + 1000, 99.9, 100.1)));
        assert_eq!(e.position(), 10);
        std::fs::write(&path, "stop").unwrap();
        let go = e.on_input(&inp(T0 + 2000, 99.9, 100.1));
        std::fs::remove_file(&path).unwrap();
        assert!(!go);
        assert_eq!((e.position(), e.stopped), (0, Some(Stop::Kill)));
    }

    #[test]
    fn drawdown_and_latency_summary() {
        let mut c = cfg();
        c.fee = 0.0;
        let mut e = Engine::new(c, Script(vec![(T0, buy(10))]), None);
        e.on_input(&inp(T0, 100.0, 100.0));
        e.on_input(&inp(T0 + 1000, 100.0, 100.0)); // long 10 at 100
        e.on_input(&inp(T0 + 2000, 102.0, 102.0)); // +20
        e.on_input(&inp(T0 + 3000, 99.0, 99.0)); // -10, drawdown 30
        let s = e.finish();
        assert!((s.max_drawdown - 30.0).abs() < 1e-9);
        assert!(s.compute_p99_ms >= s.compute_median_ms);
    }
}
