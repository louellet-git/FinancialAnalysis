//! The algo engine: one engine for replay and live, an own paper simulator, no order to a broker.
//!
//! - `Strategy` gets each input (a quote or a bar close) and returns order intents.
//! - `Source` gives the inputs: `Replay` (a recorded file or stored bars) or `Live` (polls the read
//!   functions of the Alpaca data API). Only the source changes between the two modes.
//! - `engine::Engine` holds the simulator (fills at the bid or ask in memory), the risk stops and the journal.
//!
//! The simulator sends no order. The only order code is `paper.rs`: limit orders to the Alpaca PAPER account, with `--broker alpaca-paper --send`.
//! The live source type has one read function (`QuoteFeed::read`) and no
//! function that writes to a broker; a test checks the source text for order endpoints.
//! All fills are simulated: a result is a pipe test, not a claim of profit.
//!
//! STATUS: `fa algo` is OUTSIDE the trading sleeves. It is a pipe test: no result of it is used for a decision,
//! until limits in percent of a stated paper capital exist.
//!
//! LIMITS of the simulator (printed at each start and in each summary, see `LIMITS`): no market impact, no
//! queue position, no partial fills; IEX quotes are not the NBBO; a fill at the quote is not a real fill; the
//! spread of a stored bar is assumed.

pub mod cli;
pub mod demo;
pub mod engine;
pub mod paper;
pub mod source;

use serde::{Deserialize, Serialize};

/// One input of the engine. A quote has its own bid and ask. A bar close has the close as the mid and a
/// fixed assumed spread (`source::BAR_SPREAD`). `read_ms` is the engine clock: the time that the code
/// read the input (live), or the time that the input became known (a bar close, in replay).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Input {
    pub quote_ms: i64,
    pub read_ms: i64,
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    /// `quote`, `bar` (the close of a bar: the strategy decides on it) or `bar_open` (the open of the next bar: an order fills on it).
    pub kind: String,
}

/// The limits of the simulator, printed at the start and in the summary.
pub const LIMITS: &str = "limits: simulated fills at the quote, no market impact, no queue position, no partial fills; IEX quotes are not the NBBO and a fill at the quote is not a real fill; the spread of a stored bar is assumed; the loss stop is approximate (it checks each input, the position closes at the next price)";

impl Input {
    /// A look-ahead guard: the data of an input must be known at its read time.
    pub fn check(&self) -> Result<(), String> {
        if !self.valid() {
            return Err(format!("bad price (bid {}, ask {})", self.bid, self.ask));
        }
        if self.read_ms < self.quote_ms {
            return Err(format!("the data time {} is after the read time {}", self.quote_ms, self.read_ms));
        }
        Ok(())
    }
    pub fn mid(&self) -> f64 {
        (self.bid + self.ask) / 2.0
    }
    pub fn valid(&self) -> bool {
        self.bid.is_finite() && self.ask.is_finite() && self.bid > 0.0 && self.ask >= self.bid
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn sign(self) -> i64 {
        match self {
            Side::Buy => 1,
            Side::Sell => -1,
        }
    }
}

/// An order intent in shares. With a `limit` the simulator fills only if the fill price is not worse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intent {
    pub side: Side,
    pub qty: i64,
    pub limit: Option<f64>,
}

pub trait Strategy {
    fn name(&self) -> &'static str;
    /// The fixed parameters as text (printed at the start).
    fn params(&self) -> String;
    /// `pos` is the position after the pending orders. The strategy sees the present input and the past only.
    fn on_input(&mut self, input: &Input, pos: i64) -> Vec<Intent>;
}
