//! The algo engine: one engine for replay and live, an own paper simulator, no order to a broker.
//!
//! - `Strategy` gets each input (a quote or a bar close) and returns order intents.
//! - `Source` gives the inputs: `Replay` (a recorded file or stored bars) or `Live` (polls the read
//!   functions of the Alpaca data API). Only the source changes between the two modes.
//! - `engine::Engine` holds the simulator (fills at the bid or ask in memory), the risk stops and the journal.
//!
//! The code sends no order. The live source type has one read function (`QuoteFeed::read`) and no
//! function that writes to a broker; a test checks the source text for order endpoints.
//! All fills are simulated: a result is a pipe test, not a claim of profit.

pub mod cli;
pub mod demo;
pub mod engine;
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
    /// `quote` or `bar`.
    pub kind: String,
}

impl Input {
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
