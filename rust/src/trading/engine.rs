//! Event-driven options backtester with account risk rules.
//!
//! One bot is one isolated account. For each bar:
//!   1. fill the entries queued on the previous bar at THIS bar open (sized by the risk rules)
//!   2. mark open positions at close; fire the `ExitRules` exits (pattern-day-trader check)
//!   3. settle positions that expire today at the last bar of the session (intrinsic value)
//!   4. call `strategy.on_bar(ctx)`
//!   5. record the equity (mid marks)
//!
//! The risk rules default to the paper account of the owner: $2,000, 3% of equity maximum
//! loss for each trade, defined-risk structures only, pattern-day-trader limit (below
//! $25,000: at most 3 same-day round trips in 5 sessions; a blocked exit is tried again on each
//! bar, and otherwise the position stays to expiry, because expiration is not a day trade).

use super::calendar::{busday_count, busday_offset, Date};
use super::data::{DailyBars, MarketData};
use super::pricing::{Quote, SyntheticPricer, VolSnapshot, CLOSE_MINUTE};
use super::strategy::{ExitRules, Strategy};
use super::structures::{Structure, MULTIPLIER};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub struct RiskConfig {
    pub starting_equity: f64,
    /// Maximum loss for one trade as a fraction of equity.
    pub risk_per_trade: f64,
    /// A hard cap in dollars on top of the fraction.
    pub max_risk_dollars: Option<f64>,
    pub max_open_positions: usize,
    /// Sum of the open maximum losses divided by equity.
    pub max_total_risk: f64,
    pub daily_loss_limit: f64,
    pub max_units: i64,
    pub commission_per_contract: f64,
    pub pdt: bool,
    pub pdt_equity_threshold: f64,
    /// For each rolling 5 sessions.
    pub pdt_max_day_trades: usize,
}

impl Default for RiskConfig {
    fn default() -> Self {
        RiskConfig {
            starting_equity: 2_000.0,
            risk_per_trade: 0.03,
            max_risk_dollars: None,
            max_open_positions: 3,
            max_total_risk: 0.12,
            daily_loss_limit: 0.06,
            max_units: 20,
            commission_per_contract: 0.65,
            pdt: true,
            pdt_equity_threshold: 25_000.0,
            pdt_max_day_trades: 3,
        }
    }
}

impl RiskConfig {
    /// A maximum loss of `dollars` for each trade at the starting equity. It goes down with the
    /// equity and never up past `dollars`. The total open risk allows about 3 such trades (50% at most).
    pub fn with_max_risk(dollars: f64, starting_equity: f64, commission_per_contract: f64) -> Self {
        let frac = dollars / starting_equity;
        RiskConfig {
            starting_equity,
            risk_per_trade: frac,
            max_risk_dollars: Some(dollars),
            max_total_risk: (3.0 * frac).min(0.5),
            commission_per_contract,
            ..RiskConfig::default()
        }
    }
}

/// A time on a bar: the date and the minute of the day (New York wall time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    pub date: Date,
    pub minute: i32,
}

impl Time {
    /// A number that sorts like the time.
    pub fn key(self) -> i64 {
        self.date as i64 * 1440 + self.minute as i64
    }
}

#[derive(Debug, Clone)]
pub struct Position {
    pub id: u64,
    pub structure: Structure,
    pub units: i64,
    pub entry_time: Time,
    /// Per share for one unit: +debit paid, -credit received.
    pub entry_net: f64,
    /// Dollars for the whole position, with commissions.
    pub max_loss: f64,
    pub exits: ExitRules,
    pub tag: String,
    pub bars_held: i32,
    /// Mid net value per share for one unit.
    pub mark: f64,
}

impl Position {
    pub fn premium(&self) -> f64 {
        self.entry_net.abs()
    }

    /// Dollars of profit and loss before the exit commission if closed at `net_value`.
    pub fn pnl_at(&self, net_value: f64) -> f64 {
        (net_value - self.entry_net) * MULTIPLIER * self.units as f64
    }
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub id: u64,
    pub strategy: &'static str,
    pub symbol: &'static str,
    pub structure: &'static str,
    /// `Structure::describe()`: expiry and strikes.
    pub description: String,
    pub tag: String,
    pub units: i64,
    pub entry_time: Time,
    pub exit_time: Time,
    pub entry_net: f64,
    pub exit_net: f64,
    pub max_loss: f64,
    /// Dollars, net of all commissions.
    pub pnl: f64,
    pub exit_reason: String,
}

#[derive(Debug, Clone)]
struct Pending {
    structure: Structure,
    exits: ExitRules,
    tag: String,
    max_units: Option<i64>,
}

/// Net prices for one share of one unit: entry (pay the ask, receive the bid), mid, liquidation.
#[derive(Debug, Clone, Copy)]
pub struct Prices {
    pub entry: f64,
    pub mid: f64,
    pub liquidation: f64,
}

/// Bars of one symbol up to a bar (a view into the arrays; no copy).
#[derive(Debug, Clone, Copy)]
pub struct BarView<'a> {
    pub open: &'a [f64],
    pub high: &'a [f64],
    pub low: &'a [f64],
    pub close: &'a [f64],
    pub volume: &'a [f64],
    /// Minute of the day when each bar starts.
    pub minute: &'a [i32],
}

impl<'a> BarView<'a> {
    pub fn len(&self) -> usize {
        self.close.len()
    }

    pub fn is_empty(&self) -> bool {
        self.close.is_empty()
    }
}

/// Python `a // b` for floats.
fn py_floor_div(a: f64, b: f64) -> f64 {
    let m = a % b;
    let mut div = (a - m) / b;
    if m != 0.0 && ((b < 0.0) != (m < 0.0)) {
        div -= 1.0;
    }
    if div != 0.0 {
        let f = div.floor();
        if div - f > 0.5 { f + 1.0 } else { f }
    } else {
        0.0
    }
}

/// The account, the data up to the current bar, and the orders: what a bot sees.
pub struct Ctx<'a> {
    md: &'a MarketData,
    risk: RiskConfig,
    name: &'static str,
    warmup: usize,
    /// Index of the current bar.
    pub i: usize,
    cash: f64,
    positions: Vec<Position>,
    pending: Vec<Pending>,
    trades: Vec<Trade>,
    equity_curve: Vec<f64>,
    rejections: Vec<(&'static str, u32)>,
    pdt_blocks: u32,
    day_trades: VecDeque<Date>,
    next_id: u64,
    day_start_equity: f64,
    first_of_day: Vec<bool>,
    last_of_day: Vec<bool>,
    day_start: Vec<usize>,
}

impl<'a> Ctx<'a> {
    fn new(name: &'static str, warmup: usize, md: &'a MarketData, risk: RiskConfig) -> Self {
        let n = md.len();
        let first_of_day: Vec<bool> = (0..n).map(|i| i == 0 || md.date[i] != md.date[i - 1]).collect();
        let last_of_day: Vec<bool> = (0..n).map(|i| i + 1 == n || md.date[i] != md.date[i + 1]).collect();
        let mut day_start = vec![0usize; n];
        let mut cur = 0;
        for i in 0..n {
            if first_of_day[i] {
                cur = i;
            }
            day_start[i] = cur;
        }
        Ctx {
            md,
            name,
            warmup,
            i: 0,
            cash: risk.starting_equity,
            day_start_equity: risk.starting_equity,
            risk,
            positions: Vec::new(),
            pending: Vec::new(),
            trades: Vec::new(),
            equity_curve: Vec::with_capacity(n),
            rejections: Vec::new(),
            pdt_blocks: 0,
            day_trades: VecDeque::new(),
            next_id: 1,
            first_of_day,
            last_of_day,
            day_start,
        }
    }

    // --- time -----------------------------------------------------------------
    /// The date of the current bar.
    pub fn today(&self) -> Date {
        self.md.date[self.i]
    }

    /// The date of the bar `j` (an index up to the current bar).
    pub fn date_at(&self, j: usize) -> Date {
        self.md.date[j]
    }

    /// Minute of the day when the current bar starts.
    pub fn bar_minute(&self) -> i32 {
        self.md.minute[self.i]
    }

    /// Minute of the day when the current bar ends (the time of the decision).
    pub fn now_minute(&self) -> i32 {
        self.md.interval.close_minute(self.md.minute[self.i])
    }

    pub fn is_first_bar_of_day(&self) -> bool {
        self.first_of_day[self.i]
    }

    pub fn is_last_bar_of_day(&self) -> bool {
        self.last_of_day[self.i]
    }

    pub fn minutes_to_close(&self) -> f64 {
        (CLOSE_MINUTE - self.now_minute()) as f64
    }

    // --- market data (no lookahead) ------------------------------------------------
    fn sym(&self, symbol: Option<&str>) -> usize {
        symbol.map_or(0, |s| self.md.sym_index(s))
    }

    fn view(&self, s: usize, from: usize) -> BarView<'a> {
        let md: &'a MarketData = self.md;
        let b = &md.bars[s];
        let to = self.i + 1;
        BarView { open: &b.open[from..to], high: &b.high[from..to], low: &b.low[from..to], close: &b.close[from..to], volume: &b.volume[from..to], minute: &md.minute[from..to] }
    }

    /// Bars up to and including the current one.
    pub fn bars(&self, symbol: &str) -> BarView<'a> {
        self.view(self.sym(Some(symbol)), 0)
    }

    /// Bars of the current session up to and including the current one.
    pub fn today_bars(&self, symbol: &str) -> BarView<'a> {
        self.view(self.sym(Some(symbol)), self.day_start[self.i])
    }

    /// Completed daily bars strictly before today.
    pub fn daily(&self, symbol: &str) -> DailyView<'a> {
        let md: &'a MarketData = self.md;
        let d: &'a DailyBars = &md.daily[self.sym(Some(symbol))];
        let n = d.date.partition_point(|&x| x < self.today());
        DailyView { open: &d.px.open[..n], high: &d.px.high[..n], low: &d.px.low[..n], close: &d.px.close[..n] }
    }

    pub fn spot(&self, symbol: &str) -> f64 {
        self.md.bars[self.sym(Some(symbol))].close[self.i]
    }

    /// The 30-day vol index (decimal) at each bar close up to now.
    pub fn atm_close_history(&self, symbol: &str) -> &'a [f64] {
        let md: &'a MarketData = self.md;
        &md.vol[self.sym(Some(symbol))].atm_close[..=self.i]
    }

    pub fn vol(&self, symbol: &str) -> VolSnapshot {
        self.vol_at(self.sym(Some(symbol)), self.i, false)
    }

    fn vol_at(&self, s: usize, i: usize, open: bool) -> VolSnapshot {
        let v = &self.md.vol[s];
        if open {
            VolSnapshot { atm: v.atm_open[i], short: v.short_open[i], rate: self.md.rate[i] }
        } else {
            VolSnapshot { atm: v.atm_close[i], short: v.short_close[i], rate: self.md.rate[i] }
        }
    }

    // --- options helpers -----------------------------------------------------------
    /// The expiry date `sessions_ahead` trading days from today (0 is 0DTE). Daily expirations
    /// are assumed, and no holiday exists in the calendar.
    pub fn expiry(&self, sessions_ahead: i32) -> Date {
        busday_offset(self.today(), sessions_ahead, true)
    }

    pub fn quote(&self, symbol: &str, call: bool, strike: f64, expiry: Date) -> Quote {
        let s = self.sym(Some(symbol));
        SyntheticPricer::quote(call, strike, expiry, self.today(), self.now_minute(), self.md.bars[s].close[self.i], &self.vol_at(s, self.i, false))
    }

    /// The strike with |delta| closest to `target_delta`. |delta| is monotone in the strike, so a
    /// bisection finds it; the three strikes near the result are compared.
    pub fn strike_for_delta(&self, symbol: &str, call: bool, target_delta: f64, expiry: Date, increment: f64) -> f64 {
        self.bisect_strike(symbol, call, target_delta, expiry, (0.5, 1.5), 22, increment)
    }

    /// Bisection with other bounds and steps (the iron condor bot uses fewer steps).
    pub fn bisect_strike(&self, symbol: &str, call: bool, target: f64, expiry: Date, bounds: (f64, f64), steps: usize, increment: f64) -> f64 {
        let spot = self.spot(symbol);
        let d = |k: f64| self.quote(symbol, call, k, expiry).delta.abs();
        let (mut lo, mut hi) = (spot * bounds.0, spot * bounds.1);
        for _ in 0..steps {
            let m = 0.5 * (lo + hi);
            if (d(m) > target) == call {
                lo = m;
            } else {
                hi = m;
            }
        }
        let k0 = (0.5 * (lo + hi) / increment).round_ties_even() * increment;
        let mut best: Option<(f64, f64)> = None;
        for j in [-1.0, 0.0, 1.0] {
            let k = k0 + j * increment;
            if k > 0.0 {
                let k = (k * 100.0).round_ties_even() / 100.0;
                let score = (d(k) - target).abs();
                if best.is_none_or(|(_, s)| score < s) {
                    best = Some((k, score));
                }
            }
        }
        best.expect("a candidate strike exists").0
    }

    /// Net per-share prices now: entry (pay the ask, receive the bid), mid, liquidation.
    pub fn price(&self, structure: &Structure) -> Prices {
        self.prices_at(structure, self.i, false)
    }

    fn prices_at(&self, structure: &Structure, i: usize, open: bool) -> Prices {
        let s = self.md.sym_index(structure.symbol);
        let spot = if open { self.md.bars[s].open[i] } else { self.md.bars[s].close[i] };
        let vol = self.vol_at(s, i, open);
        let bar_min = self.md.minute[i];
        let minute = if open { self.md.interval.open_minute(bar_min) } else { self.md.interval.close_minute(bar_min) };
        let (mut buy, mut mid, mut sell) = (0.0, 0.0, 0.0);
        for leg in &structure.legs {
            let q = SyntheticPricer::quote(leg.call, leg.strike, structure.expiry, self.md.date[i], minute, spot, &vol);
            let qty = leg.qty as f64;
            mid += qty * q.mid;
            buy += qty * if leg.qty > 0 { q.ask } else { q.bid };
            sell += qty * if leg.qty > 0 { q.bid } else { q.ask };
        }
        Prices { entry: buy, mid, liquidation: sell }
    }

    // --- account ---------------------------------------------------------------------
    /// The open positions (a copy), oldest first.
    pub fn positions(&self) -> Vec<Position> {
        self.positions.clone()
    }

    pub fn open_position_count(&self) -> usize {
        self.positions.len()
    }

    pub fn equity(&self) -> f64 {
        self.equity_now()
    }

    fn equity_now(&self) -> f64 {
        self.cash + self.positions.iter().fold(0.0, |a, p| a + p.mark * MULTIPLIER * p.units as f64)
    }

    /// The maximum dollar loss that the engine allows for a new trade now (with commissions).
    pub fn risk_budget(&self) -> f64 {
        let budget = self.equity_now() * self.risk.risk_per_trade;
        match self.risk.max_risk_dollars {
            Some(cap) => budget.min(cap),
            None => budget,
        }
    }

    pub fn commission_per_contract(&self) -> f64 {
        self.risk.commission_per_contract
    }

    pub fn pending_entries(&self) -> usize {
        self.pending.len()
    }

    // --- orders ------------------------------------------------------------------------
    /// Queue an entry for the next bar open. The engine does the sizing.
    pub fn open(&mut self, structure: Structure, exits: ExitRules, tag: &str, max_units: Option<i64>) {
        self.pending.push(Pending { structure, exits, tag: tag.to_string(), max_units });
    }

    /// Close at this bar close. Returns false if the pattern-day-trader rule blocks it.
    pub fn close(&mut self, position: &Position, reason: &str) -> bool {
        self.try_close(position.id, reason)
    }

    // --- internals ---------------------------------------------------------------------------
    fn commission(&self, structure: &Structure, units: i64) -> f64 {
        self.risk.commission_per_contract * structure.contracts_per_unit() as f64 * units as f64
    }

    fn reject(&mut self, why: &'static str) {
        match self.rejections.iter_mut().find(|(w, _)| *w == why) {
            Some(r) => r.1 += 1,
            None => self.rejections.push((why, 1)),
        }
    }

    fn pdt_ok(&mut self, idx: usize) -> bool {
        let today = self.today();
        if !self.risk.pdt || self.positions[idx].entry_time.date != today {
            return true;
        }
        if self.equity_now() >= self.risk.pdt_equity_threshold {
            return true;
        }
        let window_start = busday_offset(today, -4, false);
        while self.day_trades.front().is_some_and(|&d| d < window_start) {
            self.day_trades.pop_front();
        }
        self.day_trades.len() < self.risk.pdt_max_day_trades
    }

    fn close_at(&mut self, idx: usize, net_value: f64, reason: &str, commission: bool) {
        let pos = self.positions.remove(idx);
        let fee = if commission { self.commission(&pos.structure, pos.units) } else { 0.0 };
        self.cash += net_value * MULTIPLIER * pos.units as f64 - fee;
        let entry_fee = self.commission(&pos.structure, pos.units);
        let pnl = pos.pnl_at(net_value) - fee - entry_fee;
        let today = self.today();
        if pos.entry_time.date == today && reason != "expiry" {
            self.day_trades.push_back(today);
        }
        let exit_time = Time { date: today, minute: self.now_minute() };
        self.trades.push(Trade {
            id: pos.id,
            strategy: self.name,
            symbol: pos.structure.symbol,
            structure: pos.structure.name,
            description: pos.structure.describe(),
            tag: pos.tag,
            units: pos.units,
            entry_time: pos.entry_time,
            exit_time,
            entry_net: pos.entry_net,
            exit_net: net_value,
            max_loss: pos.max_loss,
            pnl,
            exit_reason: reason.to_string(),
        });
    }

    fn try_close(&mut self, id: u64, reason: &str) -> bool {
        let Some(idx) = self.positions.iter().position(|p| p.id == id) else {
            return false;
        };
        if !self.pdt_ok(idx) {
            self.pdt_blocks += 1;
            return false;
        }
        let liq = self.prices_at(&self.positions[idx].structure, self.i, false).liquidation;
        self.close_at(idx, liq, reason, true);
        true
    }

    fn fill_pending(&mut self) {
        let orders = std::mem::take(&mut self.pending);
        if self.i < self.warmup {
            return;
        }
        let today = self.today();
        let open_minute = self.md.interval.open_minute(self.md.minute[self.i]);
        for o in orders {
            let s = &o.structure;
            if s.expiry < today || super::pricing::year_fraction(today, open_minute, s.expiry) <= 0.0 {
                self.reject("expired_before_fill");
                continue;
            }
            let equity = self.equity_now();
            if equity < self.day_start_equity * (1.0 - self.risk.daily_loss_limit) {
                self.reject("daily_loss_limit");
                continue;
            }
            if self.positions.len() >= self.risk.max_open_positions {
                self.reject("max_open_positions");
                continue;
            }
            let px = self.prices_at(s, self.i, true);
            let entry = px.entry;
            let loss_per_unit = s.max_loss(entry) * MULTIPLIER + 2.0 * self.commission(s, 1);
            if loss_per_unit <= 0.0 {
                self.reject("nonpositive_max_loss"); // an arbitrage-looking quote: skip it
                continue;
            }
            let mut budget = equity * self.risk.risk_per_trade;
            if let Some(cap) = self.risk.max_risk_dollars {
                budget = budget.min(cap);
            }
            let by_budget = py_floor_div(budget, loss_per_unit) as i64;
            let cap = match o.max_units {
                Some(m) if m != 0 => m,
                _ => 1_000_000_000,
            };
            let units = by_budget.min(self.risk.max_units).min(cap);
            if units < 1 {
                self.reject("too_risky_for_account");
                continue;
            }
            let open_risk = self.positions.iter().fold(0.0, |a, p| a + p.max_loss);
            if open_risk + units as f64 * loss_per_unit > equity * self.risk.max_total_risk {
                self.reject("max_total_risk");
                continue;
            }
            let cost = entry * MULTIPLIER * units as f64 + self.commission(s, units);
            if entry > 0.0 && cost > self.cash {
                self.reject("insufficient_cash");
                continue;
            }
            self.cash -= cost;
            let pos = Position {
                id: self.next_id,
                structure: o.structure,
                units,
                entry_time: Time { date: today, minute: open_minute },
                entry_net: entry,
                max_loss: units as f64 * loss_per_unit,
                exits: o.exits,
                tag: o.tag,
                bars_held: 0,
                mark: px.mid,
            };
            self.positions.push(pos);
            self.next_id += 1;
        }
    }

    fn check_exits(&mut self) {
        let today = self.today();
        let now = self.now_minute();
        let ids: Vec<u64> = self.positions.iter().map(|p| p.id).collect();
        for id in ids {
            let idx = self.positions.iter().position(|p| p.id == id).expect("position is open");
            let px = self.prices_at(&self.positions[idx].structure, self.i, false);
            let pos = &mut self.positions[idx];
            pos.mark = px.mid;
            pos.bars_held += 1;
            if pos.structure.expiry <= today && self.last_of_day[self.i] {
                continue; // settles below
            }
            let ex = pos.exits;
            let gain = if pos.premium() > 1e-9 { (px.liquidation - pos.entry_net) / pos.premium() } else { 0.0 };
            let expiry = pos.structure.expiry;
            let reason = if ex.take_profit.is_some_and(|t| gain >= t) {
                Some("take_profit")
            } else if ex.stop_loss.is_some_and(|s| gain <= -s) {
                Some("stop_loss")
            } else if ex.max_hold_bars.is_some_and(|m| pos.bars_held >= m) {
                Some("max_hold")
            } else if ex.exit_dte.is_some_and(|d| busday_count(today, expiry) <= d) {
                Some("dte_exit")
            } else if ex.minutes_before_close.is_some_and(|m| expiry == today && ((CLOSE_MINUTE - now) as f64) <= m as f64) {
                Some("time_exit")
            } else {
                None
            };
            if let Some(r) = reason {
                self.try_close(id, r);
            }
        }
    }

    fn settle_expiries(&mut self) {
        if !self.last_of_day[self.i] {
            return;
        }
        let today = self.today();
        let ids: Vec<u64> = self.positions.iter().filter(|p| p.structure.expiry <= today).map(|p| p.id).collect();
        for id in ids {
            let idx = self.positions.iter().position(|p| p.id == id).expect("position is open");
            let s = self.md.sym_index(self.positions[idx].structure.symbol);
            let spot = self.md.bars[s].close[self.i];
            let value = self.positions[idx].structure.payoff(spot);
            self.close_at(idx, value, "expiry", false);
        }
    }
}

/// Completed daily bars before today.
#[derive(Debug, Clone, Copy)]
pub struct DailyView<'a> {
    pub open: &'a [f64],
    pub high: &'a [f64],
    pub low: &'a [f64],
    pub close: &'a [f64],
}

impl<'a> DailyView<'a> {
    pub fn len(&self) -> usize {
        self.close.len()
    }

    pub fn is_empty(&self) -> bool {
        self.close.is_empty()
    }
}

#[derive(Debug)]
pub struct BacktestResult {
    pub strategy: &'static str,
    pub equity: Vec<f64>,
    /// The date, the minute and the UTC second of each bar (the index of `equity`).
    pub date: Vec<Date>,
    pub minute: Vec<i32>,
    pub utc: Vec<i64>,
    pub trades: Vec<Trade>,
    pub open_positions: Vec<Position>,
    /// Reason and count, in the order of the first rejection.
    pub rejections: Vec<(&'static str, u32)>,
    pub pdt_blocks: u32,
    pub risk: RiskConfig,
}

/// Run one bot on the market data.
pub fn run(strategy: &mut dyn Strategy, md: &MarketData, risk: RiskConfig) -> Result<BacktestResult, String> {
    let mut ctx = Ctx::new(strategy.name(), strategy.warmup_bars(), md, risk);
    for i in 0..md.len() {
        ctx.i = i;
        if ctx.first_of_day[i] {
            ctx.day_start_equity = ctx.equity_now();
        }
        ctx.fill_pending();
        ctx.check_exits();
        ctx.settle_expiries();
        if i >= ctx.warmup {
            strategy.on_bar(&mut ctx).map_err(|e| format!("{} at bar {i}: {e}", strategy.name()))?;
        }
        let eq = ctx.equity_now();
        ctx.equity_curve.push(eq);
    }
    Ok(BacktestResult {
        strategy: strategy.name(),
        equity: ctx.equity_curve,
        date: md.date.clone(),
        minute: md.minute.clone(),
        utc: md.utc.clone(),
        trades: ctx.trades,
        open_positions: ctx.positions,
        rejections: ctx.rejections,
        pdt_blocks: ctx.pdt_blocks,
        risk: ctx.risk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::calendar::{busday_offset, days_from_civil, iso};
    use crate::trading::data::{Interval, Ohlcv, VolFrame};
    use crate::trading::structures::vertical;

    /// Flat-vol market for SPY: daily bars from 2024-01-02, or 5-minute bars from 09:30 on that day.
    fn market(closes: &[f64], interval: Interval) -> MarketData {
        let n = closes.len();
        let first = days_from_civil(2024, 1, 2);
        let (date, minute): (Vec<i32>, Vec<i32>) = match interval {
            Interval::Daily => (0..n).map(|k| (busday_offset(first, k as i32, true), 0)).unzip(),
            _ => (0..n).map(|k| (first, 570 + 5 * k as i32)).unzip(),
        };
        let utc = date.iter().zip(&minute).map(|(&d, &m)| d as i64 * 86_400 + m as i64 * 60 + 5 * 3600).collect();
        let px = Ohlcv { open: closes.to_vec(), high: closes.to_vec(), low: closes.to_vec(), close: closes.to_vec(), volume: vec![1.0; n] };
        let v = vec![0.15; n];
        let vol = VolFrame { atm_open: v.clone(), atm_close: v.clone(), short_open: v.clone(), short_close: v };
        let daily = crate::trading::data::DailyBars { date: date.clone(), px: px.clone() };
        MarketData { interval, symbols: vec!["SPY"], date, minute, utc, bars: vec![px], vol: vec![vol], rate: vec![0.04; n], daily: vec![daily] }
    }

    /// A bot that queues the structure of `make` on its first bar.
    struct BuyOnce {
        make: Box<dyn Fn(&Ctx) -> Structure + Send>,
        exits: ExitRules,
        copies: usize,
        done: bool,
    }

    impl BuyOnce {
        fn new(make: impl Fn(&Ctx) -> Structure + Send + 'static) -> Self {
            BuyOnce { make: Box::new(make), exits: ExitRules::default(), copies: 1, done: false }
        }
    }

    impl Strategy for BuyOnce {
        fn name(&self) -> &'static str {
            "t_buy_once"
        }

        fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
            if !self.done {
                for _ in 0..self.copies {
                    let s = (self.make)(ctx);
                    ctx.open(s, self.exits, "", None);
                }
                self.done = true;
            }
            Ok(())
        }
    }

    fn call_spread(ctx: &Ctx, sessions: i32, long: f64, short: f64) -> Structure {
        vertical("SPY", ctx.expiry(sessions), true, long, short).unwrap()
    }

    #[test]
    fn python_float_floor_division() {
        assert_eq!(py_floor_div(60.0, 22.6), 2.0);
        assert_eq!(py_floor_div(0.3, 0.1), 2.0); // Python: 0.3 // 0.1 == 2.0
        assert_eq!(py_floor_div(1.0, 0.1), 9.0); // Python: 1.0 // 0.1 == 9.0
        assert_eq!(py_floor_div(-1.0, 3.0), -1.0);
    }

    #[test]
    fn sizes_by_risk_and_settles_at_expiry() {
        let md = market(&[500.0, 500.0, 500.0, 520.0, 520.0, 520.0, 520.0, 520.0, 520.0, 520.0], Interval::Daily);
        let mut bot = BuyOnce::new(|c| call_spread(c, 4, 503.0, 504.0));
        let r = run(&mut bot, &md, RiskConfig::default()).unwrap();
        assert_eq!(r.trades.len(), 1);
        let t = &r.trades[0];
        assert_eq!(t.exit_reason, "expiry");
        assert!((t.exit_net - 1.0).abs() < 1e-12);
        // Sized so that the worst case (the debit and both commissions) is at most 3% of $2,000.
        assert!(t.max_loss <= 60.0 + 1e-9 && t.units >= 1);
        assert!((r.equity[r.equity.len() - 1] - (2000.0 + t.pnl)).abs() < 1e-9);
        // The expiry settlement pays no exit commission: pnl = (1 - entry) * 100 * units - entry commission.
        let entry_fee = 0.65 * 2.0 * t.units as f64;
        assert!((t.pnl - ((1.0 - t.entry_net) * 100.0 * t.units as f64 - entry_fee)).abs() < 1e-9);
    }

    #[test]
    fn entry_fills_at_the_next_bar_open() {
        let md = market(&[500.0, 510.0, 510.0, 510.0], Interval::Daily);
        let mut bot = BuyOnce::new(|c| call_spread(c, 3, 515.0, 516.0));
        let r = run(&mut bot, &md, RiskConfig::default()).unwrap();
        assert_eq!(iso(r.trades[0].entry_time.date), iso(md.date[1]));
        assert_eq!(r.trades[0].entry_time.minute, 570); // 09:30
        assert_eq!(r.trades[0].exit_time.minute, 960); // 16:00
    }

    #[test]
    fn max_risk_dollars_allows_bigger_trades() {
        let md = market(&[500.0; 5], Interval::Daily);
        let wide = |c: &Ctx| call_spread(c, 3, 500.0, 503.0); // about $150 maximum loss
        assert!(run(&mut BuyOnce::new(wide), &md, RiskConfig::default()).unwrap().trades.is_empty());
        let r = run(&mut BuyOnce::new(wide), &md, RiskConfig::with_max_risk(250.0, 2000.0, 0.65)).unwrap();
        assert_eq!(r.trades.len(), 1);
        assert!(r.trades[0].max_loss <= 250.0);
        // Total open risk is capped at 3 such trades or 50% of equity.
        assert_eq!(RiskConfig::with_max_risk(250.0, 2000.0, 0.65).max_total_risk, 0.375);
        assert_eq!(RiskConfig::with_max_risk(800.0, 2000.0, 0.65).max_total_risk, 0.5);
    }

    #[test]
    fn too_risky_trade_is_rejected() {
        let md = market(&[500.0; 5], Interval::Daily);
        let mut bot = BuyOnce::new(|c| call_spread(c, 3, 480.0, 500.0)); // $20 wide
        let r = run(&mut bot, &md, RiskConfig::default()).unwrap();
        assert!(r.trades.is_empty());
        assert_eq!(r.rejections, vec![("too_risky_for_account", 1)]);
    }

    #[test]
    fn open_position_limit() {
        let md = market(&[500.0; 6], Interval::Daily);
        let mut bot = BuyOnce::new(|c| call_spread(c, 4, 503.0, 504.0));
        bot.copies = 5;
        let r = run(&mut bot, &md, RiskConfig::default()).unwrap();
        assert_eq!(r.trades.len(), 3);
        assert_eq!(r.rejections, vec![("max_open_positions", 2)]);
    }

    #[test]
    fn exit_rules_fire_in_order() {
        // The spot falls: a call debit spread loses more than 50% of its debit at the first mark.
        let md = market(&[500.0, 500.0, 490.0, 490.0, 490.0, 490.0], Interval::Daily);
        let mut bot = BuyOnce::new(|c| call_spread(c, 4, 500.0, 501.0));
        bot.exits = ExitRules { take_profit: Some(5.0), stop_loss: Some(0.5), ..Default::default() };
        let r = run(&mut bot, &md, RiskConfig::default()).unwrap();
        assert_eq!(r.trades.len(), 1);
        assert_eq!(r.trades[0].exit_reason, "stop_loss");
        assert_eq!(iso(r.trades[0].exit_time.date), iso(md.date[2]));
        assert!(r.trades[0].pnl < 0.0);
        // max_hold_bars: 2 bars after the fill.
        let md = market(&[500.0; 8], Interval::Daily);
        let mut bot = BuyOnce::new(|c| call_spread(c, 6, 505.0, 506.0));
        bot.exits = ExitRules { max_hold_bars: Some(2), ..Default::default() };
        let r = run(&mut bot, &md, RiskConfig::default()).unwrap();
        assert_eq!(r.trades[0].exit_reason, "max_hold");
        assert_eq!(iso(r.trades[0].exit_time.date), iso(md.date[2])); // filled on bar 1 (bars_held 1), exit on bar 2
    }

    /// Opens a 0DTE spread each time that nothing is open; each exits after one bar.
    struct DayTrader;

    impl Strategy for DayTrader {
        fn name(&self) -> &'static str {
            "t_day_trader"
        }

        fn interval(&self) -> Interval {
            Interval::FiveMinute
        }

        fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
            if ctx.open_position_count() == 0 && ctx.pending_entries() == 0 {
                let s = vertical("SPY", ctx.expiry(0), true, 500.0, 501.0)?;
                ctx.open(s, ExitRules { max_hold_bars: Some(1), ..Default::default() }, "", None);
            }
            Ok(())
        }
    }

    #[test]
    fn pdt_limits_same_day_round_trips() {
        let md = market(&[500.0; 40], Interval::FiveMinute);
        let r = run(&mut DayTrader, &md, RiskConfig::default()).unwrap();
        let day_trades = r.trades.iter().filter(|t| t.exit_reason != "expiry").count();
        assert_eq!(day_trades, 3);
        assert!(r.pdt_blocks > 0);
        assert_eq!(r.trades[r.trades.len() - 1].exit_reason, "expiry");
        // With the rule off, the bot trades each bar pair.
        let free = run(&mut DayTrader, &md, RiskConfig { pdt: false, ..RiskConfig::default() }).unwrap();
        assert!(free.trades.iter().filter(|t| t.exit_reason != "expiry").count() > 3);
        assert_eq!(free.pdt_blocks, 0);
    }

    struct Peeker;

    impl Strategy for Peeker {
        fn name(&self) -> &'static str {
            "t_peeker"
        }

        fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
            let b = ctx.bars("SPY");
            if b.len() != ctx.i + 1 || b.close[ctx.i] != 400.0 + ctx.i as f64 {
                return Err("the bars do not end at the current bar".into());
            }
            if ctx.vol("SPY").atm != 0.15 || ctx.atm_close_history("SPY").len() != ctx.i + 1 {
                return Err("vol history is wrong".into());
            }
            // The daily bars are strictly before today.
            if ctx.daily("SPY").len() != ctx.i {
                return Err("daily bars include today".into());
            }
            Ok(())
        }
    }

    #[test]
    fn context_has_no_lookahead() {
        let closes: Vec<f64> = (0..20).map(|k| 400.0 + k as f64).collect();
        run(&mut Peeker, &market(&closes, Interval::Daily), RiskConfig::default()).unwrap();
    }

    #[test]
    fn a_bot_error_stops_the_run() {
        struct Broken;
        impl Strategy for Broken {
            fn name(&self) -> &'static str {
                "t_broken"
            }
            fn on_bar(&mut self, _: &mut Ctx) -> Result<(), String> {
                Err("boom".into())
            }
        }
        let e = run(&mut Broken, &market(&[500.0; 3], Interval::Daily), RiskConfig::default()).unwrap_err();
        assert!(e.contains("boom"));
    }

    #[test]
    fn strike_for_delta_finds_the_target() {
        let md = market(&[500.0; 3], Interval::Daily);
        struct Probe(std::sync::Arc<std::sync::Mutex<Vec<(f64, f64)>>>);
        impl Strategy for Probe {
            fn name(&self) -> &'static str {
                "t_probe"
            }
            fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
                let exp = ctx.expiry(10);
                for (call, target) in [(true, 0.30), (false, 0.30), (true, 0.50)] {
                    let k = ctx.strike_for_delta("SPY", call, target, exp, 1.0);
                    let d = ctx.quote("SPY", call, k, exp).delta.abs();
                    self.0.lock().unwrap().push((k, d));
                }
                Ok(())
            }
        }
        let out = Default::default();
        run(&mut Probe(std::sync::Arc::clone(&out)), &md, RiskConfig::default()).unwrap();
        let v = out.lock().unwrap();
        let (call30, put30, atm) = (v[0], v[1], v[2]);
        assert!(call30.0 > 500.0 && put30.0 < 500.0);
        assert!((call30.1 - 0.30).abs() < 0.05 && (put30.1 - 0.30).abs() < 0.05);
        assert!((atm.1 - 0.50).abs() < 0.05, "{atm:?}");
        assert_eq!(call30.0.fract(), 0.0); // strikes sit on the $1 grid
    }
}
