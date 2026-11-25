//! The paper broker of `fa algo`: LIMIT orders to the Alpaca PAPER account, and nothing else.
//!
//! This is the only file of the repo with order code (the tripwire `no_order_code_in_the_repo` checks it). The rules
//! are the controls 0 to 8 of `docs/QUALITY-STANDARDS.md` section 8:
//!
//! 1. Endpoint: `PAPER_HOST` is a constant. `paper_url` builds each URL from it, `host_is_paper` compares the parsed
//!    host exactly, and `RealTransport` follows no redirect. No variable and no option changes the host.
//! 2. Keys: only `ALPACA_PAPER_KEY_ID` and `ALPACA_PAPER_SECRET_KEY`. The account check runs before the first order.
//!    The Alpaca documentation (account object, read 2026-11-25) has no field that says "paper". The check is the
//!    host constant (a live key does not work on the paper host) plus a successful account call whose status is
//!    `ACTIVE` or `PAPER_ONLY`, with no block. No key, token or account number goes to a file or the output.
//! 3. Limits: `LIMITS`. A limit that cannot be read or checked refuses the order (`check_order`).
//! 4. Stops: the kill file, the loss stops and the end time cancel all open orders, KEEP the position, write the log line and send no new order.
//!    A person closes a position. The simulator flattens its own position for its P&L only.
//! 5. `client_order_id`: unique, at most 128 characters, kept in the journal; an id of the journal is never sent again.
//! 6. Caps: day orders, limit orders, no margin, no option, at most `LIMITS.orders_per_min` orders a minute.
//! 7. The order journal holds the order, the broker reply and the simulator fill of the same decision.
//! 8. Long only in step A: a sell that would make the position negative is refused.
//!
//! Without `--send` the hook has NO transport (`Mode::DryRun`): it cannot make a request.

use super::engine::{DecisionCtx, OrderHook, Stop};
use super::{Input, Side};
use crate::market::keys::{read_key, scrub_all};
use serde_json::{json, Value};
use std::collections::{HashSet, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The order host. A constant: no variable, option or file changes it.
pub const PAPER_HOST: &str = "https://paper-api.alpaca.markets";
/// The only variables that the order code reads for the keys.
pub const KEY_ID_VAR: &str = "ALPACA_PAPER_KEY_ID";
pub const SECRET_VAR: &str = "ALPACA_PAPER_SECRET_KEY";
/// Step A: one symbol. Options are not allowed (control 6).
pub const SYMBOLS: [&str; 1] = ["SPY"];

/// The paper limits. Source: `chiefs/louellet-git/quality/proposal-sleeve-limits.md` (table "Proposed limits", the PAPER table, not the
/// live pilot), approved by the owner 2026-11-21; the order caps are `docs/QUALITY-STANDARDS.md` section 8 control 6 (5% and 10 orders
/// a day); the rate cap and the size cap of step A are set by the ticket `T-20261121-algo-paper-orders` (2026-11-25).
/// The sleeve capital is the equity of the paper account (one account for each sleeve owner), read before the run.
pub struct Limits {
    /// Gross exposure of the account at most this percent of the capital (no leverage).
    pub gross_pct: f64,
    /// One position at entry, percent of the capital.
    pub position_entry_pct: f64,
    /// One position at market, percent of the capital (the value that a held position can reach).
    pub position_market_pct: f64,
    /// One issuer in the whole book (one account: the account is the book).
    pub issuer_pct: f64,
    /// A position at most this percent of the 20-day average daily dollar volume.
    pub adv_pct: f64,
    /// The loss of one New York day, percent of the capital: the stop.
    pub day_loss_pct: f64,
    /// The fall from the peak of the sleeve, percent: no run starts below it.
    pub sleeve_stop_pct: f64,
    /// One order at most this percent of the capital.
    pub order_pct: f64,
    pub orders_per_day: usize,
    /// At most this many orders in 60 seconds. One order each 10 seconds is far above what a one-minute rule makes: a hit is a bug.
    pub orders_per_min: usize,
    /// Step A: at most this many shares in an order and in the position.
    pub max_qty: i64,
}

pub const LIMITS: Limits = Limits {
    gross_pct: 100.0,
    position_entry_pct: 10.0,
    position_market_pct: 15.0,
    issuer_pct: 15.0,
    adv_pct: 1.0,
    day_loss_pct: 2.0,
    sleeve_stop_pct: 10.0,
    order_pct: 5.0,
    orders_per_day: 10,
    orders_per_min: 6,
    max_qty: 1,
};

/// An order that is open this long (ms) is cancelled: a marketable limit fills at once; a stale order must not fill later.
pub const ORDER_TTL_MS: i64 = 30_000;
/// The pause between two reads of an open order (ms).
pub const POLL_EVERY_MS: i64 = 2_000;
/// API errors in a row that end the run (control 5).
pub const MAX_API_ERRORS: usize = 3;

// ---------------------------------------------------------------- transport

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Delete,
}

impl Method {
    pub fn name(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Delete => "DELETE",
        }
    }
}

#[derive(Debug)]
pub struct Reply {
    pub status: u16,
    pub body: String,
}

/// The HTTP calls of the broker. The real one is `RealTransport`; the tests use a mock. A transport gets a full URL, and the
/// keys are its own business: the broker never sees a key.
pub trait Transport {
    fn send(&mut self, method: Method, url: &str, body: Option<&str>) -> Result<Reply, String>;
}

/// The exact host of a URL, or `None` for a URL that is not a plain `https://<host>/...`: no user part, no port,
/// no backslash, no white space.
fn exact_host(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let bad = |c: char| c.is_whitespace() || c.is_control() || matches!(c, '@' | ':' | '\\' | '%');
    if host.is_empty() || host.chars().any(bad) || url.chars().any(|c| c.is_whitespace() || c.is_control() || c == '\\') {
        return None;
    }
    Some(host)
}

/// Is the URL on the paper host? The parsed host must equal the host of `PAPER_HOST`, exactly.
pub fn host_is_paper(url: &str) -> bool {
    let want = PAPER_HOST.strip_prefix("https://").unwrap_or("");
    exact_host(url) == Some(want)
}

/// The URL of an API path. The host is the constant. The path must be a `/v2/` path with no second host in it.
pub fn paper_url(path: &str) -> Result<String, String> {
    if !path.starts_with("/v2/") || path.contains("://") || path.contains("//") || path.contains('\\') || path.chars().any(|c| c.is_whitespace()) {
        return Err(format!("refused: '{path}' is not a /v2/ path of the paper API"));
    }
    let url = format!("{PAPER_HOST}{path}");
    if !host_is_paper(&url) {
        return Err("refused: the URL is not on the paper host".into());
    }
    Ok(url)
}

/// The keys of the paper account. No `Debug`: a key never goes to a log by a format string.
pub struct PaperKeys {
    id: String,
    secret: String,
}

impl PaperKeys {
    /// Keys from values that a caller already has (a test).
    pub fn new(id: &str, secret: &str) -> PaperKeys {
        PaperKeys { id: id.to_string(), secret: secret.to_string() }
    }
    /// Reads `ALPACA_PAPER_KEY_ID` and `ALPACA_PAPER_SECRET_KEY` (environment, else `<repo>/.env`). No other name.
    pub fn read(repo: &Path) -> Option<PaperKeys> {
        Some(PaperKeys { id: read_key(repo, KEY_ID_VAR)?, secret: read_key(repo, SECRET_VAR)? })
    }
    fn secrets(&self) -> Vec<String> {
        vec![self.id.clone(), self.secret.clone()]
    }
}

/// The real HTTPS transport: the paper host only, the keys in headers, no redirect.
pub struct RealTransport {
    keys: PaperKeys,
    client: mapcheck::http::Client,
}

impl RealTransport {
    pub fn new(keys: PaperKeys) -> RealTransport {
        RealTransport { keys, client: mapcheck::http::Client::no_redirect(crate::hostnet::USER_AGENT, 30) }
    }
}

impl Transport for RealTransport {
    fn send(&mut self, method: Method, url: &str, body: Option<&str>) -> Result<Reply, String> {
        if !host_is_paper(url) {
            return Err("refused: the URL is not on the paper host".into());
        }
        let headers = [("APCA-API-KEY-ID", self.keys.id.as_str()), ("APCA-API-SECRET-KEY", self.keys.secret.as_str())];
        match self.client.request(method.name(), url, &headers, body) {
            Ok((status, bytes)) => Ok(Reply { status, body: String::from_utf8_lossy(&bytes).to_string() }),
            Err(e) => Err(scrub_all(&e.to_string(), &[&self.keys.id, &self.keys.secret])),
        }
    }
}

// ---------------------------------------------------------------- mode

pub enum Mode {
    /// No transport: the hook cannot make a request.
    DryRun,
    Send(Box<dyn Transport>, Vec<String>),
}

/// Choose the mode. Without `--send` the keys are not used and no transport is made. With `--send` and no keys the run is refused.
pub fn choose_mode(send: bool, keys: Option<PaperKeys>, make: impl FnOnce(PaperKeys) -> Box<dyn Transport>) -> Result<Mode, String> {
    if !send {
        return Ok(Mode::DryRun);
    }
    let keys = keys.ok_or_else(|| format!("--send needs {KEY_ID_VAR} and {SECRET_VAR} (the paper account keys; no other variable is used). No request was made"))?;
    let secrets = keys.secrets();
    Ok(Mode::Send(make(keys), secrets))
}

// ---------------------------------------------------------------- limits and checks

pub struct OrderReq {
    pub symbol: String,
    pub side: Side,
    pub qty: i64,
    pub limit: f64,
}

/// The facts that the checks need. `None` is "could not read": the order is refused (fail closed) in a real run.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub capital: Option<f64>,
    pub cash: Option<f64>,
    pub gross: Option<f64>,
    pub position_qty: Option<i64>,
    pub open_buy_qty: i64,
    pub open_sell_qty: i64,
    pub adv: Option<f64>,
    pub orders_today: usize,
    pub orders_last_min: usize,
    pub budget_left: usize,
}

/// A stock symbol: capital letters and a dot only. An option symbol (the OCC form has digits) is not one.
pub fn is_stock_symbol(s: &str) -> bool {
    !s.is_empty() && s.len() <= 6 && s.chars().all(|c| c.is_ascii_uppercase() || c == '.')
}

/// The checks of one order. Returns `(refusals, not_evaluated)`. With `strict` (a real run) a missing fact is a refusal;
/// in a dry run it is listed as not evaluated.
pub fn check_order(req: &OrderReq, f: &Facts, strict: bool) -> (Vec<String>, Vec<String>) {
    let (mut no, mut skipped) = (vec![], vec![]);
    let mut need = |name: &str, missing: bool, no: &mut Vec<String>| {
        if missing {
            if strict {
                no.push(format!("cannot check: {name} is not known"));
            } else {
                skipped.push(name.to_string());
            }
        }
        missing
    };
    if !is_stock_symbol(&req.symbol) {
        no.push(format!("symbol '{}' is not a stock symbol (no option in step A)", req.symbol));
    }
    if !SYMBOLS.contains(&req.symbol.as_str()) {
        no.push(format!("symbol '{}' is not allowed in step A (only {})", req.symbol, SYMBOLS.join(", ")));
    }
    if req.qty < 1 || req.qty > LIMITS.max_qty {
        no.push(format!("quantity {} is outside 1..={} (the cap of step A)", req.qty, LIMITS.max_qty));
    }
    if !(req.limit.is_finite() && req.limit > 0.0) {
        no.push(format!("limit price {} is not valid", req.limit));
    }
        if f.orders_today >= LIMITS.orders_per_day {
            no.push(format!("{} orders today: the cap is {}", f.orders_today, LIMITS.orders_per_day));
        }
        if f.orders_last_min >= LIMITS.orders_per_min {
            no.push(format!("{} orders in the last minute: the cap is {}", f.orders_last_min, LIMITS.orders_per_min));
        }
    if f.budget_left == 0 {
        no.push("the request budget of the minute is used".into());
    }
    let value = req.qty as f64 * req.limit;
    match req.side {
        Side::Sell => {
            if !need("the position", f.position_qty.is_none(), &mut no) {
                let pos = f.position_qty.unwrap_or(0);
                if pos - f.open_sell_qty - req.qty < 0 {
                    no.push(format!("a sell of {} would make the position negative (position {pos}, open sells {}): long only in step A, no short", req.qty, f.open_sell_qty));
                }
            }
        }
        Side::Buy => {
            let pos = f.position_qty;
            if !need("the position", pos.is_none(), &mut no) {
                let after = pos.unwrap_or(0) + f.open_buy_qty + req.qty;
                if after > LIMITS.max_qty {
                    no.push(format!("the position would be {after} shares: the cap of step A is {}", LIMITS.max_qty));
                }
                let after_value = after as f64 * req.limit;
                if !need("the capital", f.capital.is_none(), &mut no) {
                    let cap = f.capital.unwrap_or(0.0);
                    let over = |name: &str, v: f64, pct: f64, no: &mut Vec<String>| {
                        if v > cap * pct / 100.0 {
                            no.push(format!("{name}: {v:.2} USD is over {pct}% of the capital {cap:.2} USD"));
                        }
                    };
                    over("one order", value, LIMITS.order_pct, &mut no);
                    over("one position at entry", after_value, LIMITS.position_entry_pct, &mut no);
                    over("one position at market", after_value * 1.0, LIMITS.position_market_pct, &mut no);
                    over("one issuer", after_value, LIMITS.issuer_pct, &mut no);
                    if !need("the gross exposure", f.gross.is_none(), &mut no) && f.gross.unwrap_or(0.0) + value > cap * LIMITS.gross_pct / 100.0 {
                        no.push(format!("gross exposure {:.2} USD plus {value:.2} USD is over {}% of the capital", f.gross.unwrap_or(0.0), LIMITS.gross_pct));
                    }
                }
                if !need("the 20-day dollar volume", f.adv.is_none(), &mut no) {
                    let adv = f.adv.unwrap_or(0.0);
                    if after_value > adv * LIMITS.adv_pct / 100.0 {
                        no.push(format!("liquidity: position {after_value:.2} USD is over {}% of the 20-day dollar volume {adv:.0} USD", LIMITS.adv_pct));
                    }
                }
            }
            // No margin: the order value must be inside the cash.
            if !need("the cash", f.cash.is_none(), &mut no) && value > f.cash.unwrap_or(0.0) {
                no.push(format!("no margin: the order value {value:.2} USD is over the cash {:.2} USD", f.cash.unwrap_or(0.0)));
            }
        }
    }
    (no, skipped)
}

// ---------------------------------------------------------------- the request budget

/// One Alpaca request budget for the process: the trading requests count inside it. `limit` is the share that the trading host
/// gets (the whole limit of `fa algo` minus the requests of the data poll).
pub struct Budget {
    limit: usize,
    times: VecDeque<i64>,
}

impl Budget {
    pub fn new(limit: usize) -> Budget {
        Budget { limit, times: VecDeque::new() }
    }
    fn prune(&mut self, now: i64) {
        while self.times.front().is_some_and(|t| *t <= now - 60_000) {
            self.times.pop_front();
        }
    }
    pub fn left(&mut self, now: i64) -> usize {
        self.prune(now);
        self.limit.saturating_sub(self.times.len())
    }
    pub fn take(&mut self, now: i64) -> bool {
        if self.left(now) == 0 {
            return false;
        }
        self.times.push_back(now);
        true
    }
}

// ---------------------------------------------------------------- account and positions (parsers)

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub equity: f64,
    pub cash: f64,
}

fn num(v: &Value) -> Option<f64> {
    v.as_str().and_then(|s| s.trim().parse::<f64>().ok()).or_else(|| v.as_f64()).filter(|x| x.is_finite())
}

/// The account check. It keeps no account number: only the equity and the cash leave this function.
pub fn parse_paper_account(v: &Value) -> Result<Account, String> {
    let status = v["status"].as_str().unwrap_or("");
    if !matches!(status, "ACTIVE" | "PAPER_ONLY") {
        return Err(format!("the account status is '{status}', not ACTIVE or PAPER_ONLY"));
    }
    if v["trading_blocked"].as_bool() != Some(false) || v["account_blocked"].as_bool() != Some(false) {
        return Err("the account is blocked, or the block flags are missing".into());
    }
    if let Some(c) = v["currency"].as_str() {
        if c != "USD" {
            return Err(format!("the account currency is {c}, not USD"));
        }
    }
    let (Some(equity), Some(cash)) = (num(&v["equity"]), num(&v["cash"])) else {
        return Err("the account has no readable equity and cash".into());
    };
    if equity <= 0.0 {
        return Err("the account equity is not positive".into());
    }
    Ok(Account { equity, cash })
}

/// `(quantity of the symbol, gross exposure of all positions)` from `GET /v2/positions`.
pub fn parse_positions(v: &Value, symbol: &str) -> Result<(i64, f64), String> {
    let list = v.as_array().ok_or("the positions reply is not a list")?;
    let (mut qty, mut gross) = (0i64, 0.0);
    for p in list {
        let mv = num(&p["market_value"]).ok_or("a position has no readable market value")?;
        gross += mv.abs();
        if p["symbol"].as_str() == Some(symbol) {
            let q = num(&p["qty"]).ok_or("the position has no readable quantity")?;
            if q.fract() != 0.0 {
                return Err("a fractional position is not handled".into());
            }
            qty = q as i64;
        }
    }
    Ok((qty, gross))
}

/// The average entry price of the symbol from `GET /v2/positions`.
fn avg_entry(v: &Value, symbol: &str) -> Option<f64> {
    v.as_array()?.iter().find(|p| p["symbol"].as_str() == Some(symbol)).and_then(|p| num(&p["avg_entry_price"]))
}

/// The 20-day average daily dollar volume from daily bars `(close, volume)`, oldest first. Needs 20 bars.
pub fn adv_dollars(bars: &[(f64, f64)]) -> Result<f64, String> {
    if bars.len() < 20 {
        return Err(format!("only {} daily bars, need 20", bars.len()));
    }
    let last = &bars[bars.len() - 20..];
    if last.iter().any(|(c, v)| !(c.is_finite() && v.is_finite() && *c > 0.0 && *v >= 0.0)) {
        return Err("a daily bar is not valid".into());
    }
    Ok(last.iter().map(|(c, v)| c * v).sum::<f64>() / 20.0)
}

/// The sleeve stop: the peak of the account equity is kept in a file; no run starts 10% below it.
pub fn sleeve_peak(path: &Path, equity: f64) -> Result<f64, String> {
    let old = match std::fs::read_to_string(path) {
        Ok(t) => serde_json::from_str::<Value>(&t).ok().and_then(|v| v["peak_equity"].as_f64()).filter(|x| x.is_finite() && *x > 0.0).ok_or_else(|| format!("{} is not readable: remove it only after a person checks the sleeve", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0.0,
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let peak = old.max(equity);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, serde_json::to_string(&json!({"peak_equity": peak})).unwrap_or_default() + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
    if equity <= peak * (1.0 - LIMITS.sleeve_stop_pct / 100.0) {
        return Err(format!("the sleeve stop: equity {equity:.2} USD is {}% or more below the peak {peak:.2} USD", LIMITS.sleeve_stop_pct));
    }
    Ok(peak)
}

/// The name of this machine, for the run log. No shell command: the environment, else the kernel file.
pub fn host_name() -> String {
    for var in ["HOSTNAME", "COMPUTERNAME"] {
        if let Ok(v) = std::env::var(var) {
            if !v.trim().is_empty() {
                return v.trim().to_string();
            }
        }
    }
    for f in ["/proc/sys/kernel/hostname", "/etc/hostname"] {
        if let Ok(t) = std::fs::read_to_string(f) {
            if !t.trim().is_empty() {
                return t.trim().to_string();
            }
        }
    }
    "unknown".into()
}

fn order_view(v: &Value) -> Value {
    let keep = ["id", "client_order_id", "status", "symbol", "side", "qty", "type", "time_in_force", "limit_price", "filled_qty", "filled_avg_price", "created_at", "submitted_at", "filled_at", "updated_at", "canceled_at", "expired_at", "failed_at"];
    let mut m = serde_json::Map::new();
    for k in keep {
        m.insert(k.to_string(), v[k].clone());
    }
    Value::Object(m)
}

fn is_terminal(status: &str) -> bool {
    matches!(status, "filled" | "canceled" | "expired" | "rejected" | "done_for_day" | "replaced" | "stopped")
}

fn cents(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

// ---------------------------------------------------------------- the hook

struct Tracked {
    decision_id: Option<usize>,
    client_order_id: String,
    side: Side,
    qty: i64,
    limit: f64,
    bid: f64,
    ask: f64,
    decision_ms: i64,
    request: Value,
    broker_id: Option<String>,
    broker: Option<Value>,
    status: String,
    filled_qty: i64,
    filled_avg: f64,
    refused: Vec<String>,
    dry: bool,
    sim: Option<Value>,
    sent_ms: i64,
    cancel_sent: bool,
    written: bool,
}

impl Tracked {
    fn open(&self) -> bool {
        !self.dry && self.refused.is_empty() && self.broker_id.is_some() && !is_terminal(&self.status)
    }
    fn done(&self) -> bool {
        self.dry || !self.refused.is_empty() || is_terminal(&self.status) || self.broker_id.is_none()
    }
}

pub struct PaperHook {
    tr: Option<Box<dyn Transport>>,
    secrets: Vec<String>,
    symbol: String,
    strategy: String,
    stop_file: PathBuf,
    journal: Box<dyn Write>,
    seen_ids: HashSet<String>,
    counter: usize,
    sent_ms: VecDeque<i64>,
    orders_today: usize,
    budget: Budget,
    facts: Facts,
    tracked: Vec<Tracked>,
    now_ms: i64,
    api_errors: usize,
    stopped: bool,
    failed: Option<String>,
    broker_pos: i64,
    cash_flow: f64,
    last_mid: f64,
    day_loss_limit: f64,
    last_poll_ms: i64,
    pub paper_check: String,
}

impl PaperHook {
    /// `prior` is the text of the order journal of the day (a restart): its ids are never sent again, and its orders count.
    pub fn new(mode: Mode, symbol: &str, strategy: &str, stop_file: PathBuf, journal: Box<dyn Write>, prior: &str, budget: usize) -> PaperHook {
        let (tr, secrets) = match mode {
            Mode::DryRun => (None, vec![]),
            Mode::Send(t, s) => (Some(t), s),
        };
        let (mut seen, mut counter, mut today, mut sent) = (HashSet::new(), 0usize, 0usize, VecDeque::new());
        for l in prior.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(v) = serde_json::from_str::<Value>(l) else { continue };
            if let Some(id) = v["client_order_id"].as_str() {
                seen.insert(id.to_string());
            }
            if v["kind"] == "paper_intent" {
                counter += 1;
                if v["dry_run"] == false {
                    today += 1;
                    if let Some(t) = v["t_ms"].as_i64() {
                        sent.push_back(t);
                    }
                }
            }
        }
        // A restart keeps the P&L of the day: the fills of the journal are the first cash flow.
        let mut latest: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
        for l in prior.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(v) = serde_json::from_str::<Value>(l) else { continue };
            if v["kind"] == "paper_order" || v["kind"] == "paper_status" {
                let (Some(cid), Some(fq), Some(avg)) = (v["client_order_id"].as_str(), num(&v["broker"]["filled_qty"]), num(&v["broker"]["filled_avg_price"])) else { continue };
                let sign = if v["broker"]["side"] == "buy" { 1.0 } else { -1.0 };
                latest.insert(cid.to_string(), -sign * fq * avg);
            }
        }
        let prior_cash_flow: f64 = latest.values().sum();
        PaperHook {
            tr,
            secrets,
            symbol: symbol.to_string(),
            strategy: strategy.to_string(),
            stop_file,
            journal,
            seen_ids: seen,
            counter,
            sent_ms: sent,
            orders_today: today,
            budget: Budget::new(budget),
            facts: Facts::default(),
            tracked: vec![],
            now_ms: 0,
            api_errors: 0,
            stopped: false,
            failed: None,
            broker_pos: 0,
            cash_flow: prior_cash_flow,
            last_mid: 0.0,
            day_loss_limit: f64::INFINITY,
            last_poll_ms: 0,
            paper_check: "not checked (dry run)".into(),
        }
    }

    pub fn is_send(&self) -> bool {
        self.tr.is_some()
    }
    /// The daily loss limit in USD (2% of the capital). Infinite before `start` and in a dry run.
    pub fn day_loss_limit(&self) -> f64 {
        self.day_loss_limit
    }

    fn clean(&self, text: &str) -> String {
        let refs: Vec<&str> = self.secrets.iter().map(String::as_str).collect();
        scrub_all(&text.chars().take(300).collect::<String>(), &refs)
    }

    /// Append one line to the order journal, with the time. A failure is an error: the caller refuses to send.
    fn log(&mut self, mut v: Value) -> Result<(), String> {
        if let Some(m) = v.as_object_mut() {
            m.insert("t_ms".into(), json!(self.now_ms));
        }
        writeln!(self.journal, "{v}").and_then(|_| self.journal.flush()).map_err(|e| format!("order journal: {e}"))
    }

    fn log_soft(&mut self, v: Value) {
        if let Err(e) = self.log(v) {
            self.failed.get_or_insert(e);
        }
    }

    /// One call to the paper API. The budget and the error count are here.
    fn call(&mut self, method: Method, path: &str, body: Option<&str>) -> Result<Value, String> {
        self.call_with(method, path, body, true, true)
    }

    /// A read of an order by its client id after a failed send: it uses the budget, but it does not count as a second API error.
    fn call_lookup(&mut self, method: Method, path: &str, body: Option<&str>) -> Result<Value, String> {
        self.call_with(method, path, body, true, false)
    }

    /// A call at a stop: a cancel must not wait for the request budget, and it does not count in the error count.
    fn call_critical(&mut self, method: Method, path: &str, body: Option<&str>) -> Result<Value, String> {
        self.call_with(method, path, body, false, false)
    }

    fn call_with(&mut self, method: Method, path: &str, body: Option<&str>, use_budget: bool, count_errors: bool) -> Result<Value, String> {
        if use_budget && !self.budget.take(self.now_ms) {
            return Err("the request budget of the minute is used".into());
        }
        let url = paper_url(path)?;
        let result = match self.tr.as_mut() {
            Some(t) => t.send(method, &url, body),
            None => return Err("dry run: no request is made".into()),
        };
        let out = match result {
            Err(e) => Err(format!("transport: {}", self.clean(&e))),
            Ok(r) if (200..300).contains(&r.status) => {
                if r.body.trim().is_empty() {
                    Ok(Value::Null)
                } else {
                    serde_json::from_str(&r.body).map_err(|_| "the reply is not JSON".to_string())
                }
            }
            Ok(r) => Err(format!("HTTP {}: {}", r.status, self.clean(&r.body))),
        };
        if !count_errors {
            // no change of the error count
        } else if out.is_err() {
            self.api_errors += 1;
            if self.api_errors >= MAX_API_ERRORS {
                self.failed.get_or_insert(format!("{MAX_API_ERRORS} API errors in a row"));
            }
        } else {
            self.api_errors = 0;
        }
        out
    }

    /// Dry run: nothing to read. Returns the lines for the run log.
    pub fn start_dry(&mut self) -> Vec<String> {
        self.paper_check = "not checked (dry run)".into();
        vec![
            "DRY RUN: no order request and no request to the trading host is made. Use --send to send orders to the paper account.".into(),
            "  the limits that need the account (capital, cash, gross, liquidity) are not evaluated in a dry run".into(),
        ]
    }

    /// Real run: the account check, the sleeve stop, the positions and the open orders. Any failure refuses the run.
    pub fn start(&mut self, now_ms: i64, adv: Result<f64, String>, peak_file: &Path) -> Result<Vec<String>, String> {
        self.now_ms = now_ms;
        let acct = match self.call(Method::Get, "/v2/account", None).and_then(|v| parse_paper_account(&v)) {
            Ok(a) => a,
            Err(e) => {
                self.paper_check = "no".into();
                return Err(format!("account check failed (paper: no): {e}. No order was sent"));
            }
        };
        self.paper_check = "yes".into();
        let peak = sleeve_peak(peak_file, acct.equity)?;
        let (qty, gross) = self.read_positions().map_err(|e| format!("positions: {e}"))?;
        if qty != 0 {
            return Err(format!("the account holds {qty} {} already: close it by hand first (the engine starts flat)", self.symbol));
        }
        let open = self.call(Method::Get, &format!("/v2/orders?status=open&symbols={}&limit=50", self.symbol), None).map_err(|e| format!("open orders: {e}"))?;
        if open.as_array().map(|a| !a.is_empty()).unwrap_or(true) {
            return Err(format!("the account has open orders for {}: cancel them first (a restart reads the open orders before it sends)", self.symbol));
        }
        let adv = adv.map_err(|e| format!("the 20-day dollar volume is not known ({e}): the liquidity limit cannot be checked"))?;
        self.facts = Facts { capital: Some(acct.equity), cash: Some(acct.cash), gross: Some(gross), position_qty: Some(0), adv: Some(adv), ..Facts::default() };
        self.day_loss_limit = acct.equity * LIMITS.day_loss_pct / 100.0;
        self.log(json!({"kind": "paper_start", "host_machine": host_name(), "paper": "yes", "capital": acct.equity, "cash": acct.cash, "peak_equity": peak, "day_loss_limit": self.day_loss_limit, "adv_20d_dollars": adv}))?;
        Ok(vec![
            "account check: paper: yes (host constant, status ACTIVE or PAPER_ONLY, no block; the account number is never printed or saved)".into(),
            format!("  capital (equity) {:.2} USD, cash {:.2} USD, sleeve peak {peak:.2} USD, daily loss limit {:.2} USD ({}% of the capital)", acct.equity, acct.cash, self.day_loss_limit, LIMITS.day_loss_pct),
            format!("  20-day dollar volume {adv:.0} USD (IEX volume only, so the liquidity check is stricter than the real market)"),
            "  the account holds no position and no open order for the symbol".into(),
        ])
    }

    fn read_positions(&mut self) -> Result<(i64, f64), String> {
        let v = self.call(Method::Get, "/v2/positions", None)?;
        parse_positions(&v, &self.symbol)
    }

    /// Fresh account facts for one order. A failed read leaves the fact `None`, so the order is refused.
    fn refresh(&mut self) {
        self.facts.capital = None;
        self.facts.cash = None;
        self.facts.gross = None;
        self.facts.position_qty = None;
        if !self.day_loss_limit.is_finite() {
            return; // not started: nothing is read
        }
        if let Ok(a) = self.call(Method::Get, "/v2/account", None).and_then(|v| parse_paper_account(&v)) {
            self.facts.capital = Some(a.equity);
            self.facts.cash = Some(a.cash);
        }
        if let Ok((q, g)) = self.read_positions() {
            self.facts.position_qty = Some(q);
            self.facts.gross = Some(g);
        }
    }

    fn open_qty(&self, side: Side) -> i64 {
        self.tracked.iter().filter(|t| t.open() && t.side == side).map(|t| t.qty - t.filled_qty).sum()
    }

    fn next_id(&mut self, date: &str) -> Result<String, String> {
        self.counter += 1;
        let id = format!("fa-{}-{}-{}-{:04}", date, self.strategy, self.symbol, self.counter);
        if id.len() > 128 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.') {
            return Err(format!("client_order_id '{id}' is not valid (at most 128 characters)"));
        }
        if self.seen_ids.contains(&id) {
            return Err(format!("client_order_id {id} is in the journal already: not sent again"));
        }
        Ok(id)
    }

    /// Everything after the decision: checks, the journal, and the request (only in a real run).
    fn submit(&mut self, mut t: Tracked) {
        let date = crate::market::alpaca::ny_day(self.now_ms.div_euclid(1000)).0;
        let req = OrderReq { symbol: self.symbol.clone(), side: t.side, qty: t.qty, limit: t.limit };
        if self.stopped {
            t.refused.push("the run is stopped: no new order".into());
            self.finish_refused(t);
            return;
        }
        let id = match self.next_id(&date) {
            Ok(id) => id,
            Err(e) => {
                t.refused.push(e);
                self.finish_refused(t);
                return;
            }
        };
        self.seen_ids.insert(id.clone());
        t.client_order_id = id.clone();
        t.request = json!({"symbol": self.symbol, "qty": t.qty.to_string(), "side": format!("{:?}", t.side).to_lowercase(), "type": "limit", "time_in_force": "day", "limit_price": format!("{:.2}", t.limit), "client_order_id": id});
        if self.stop_file.exists() {
            t.refused.push(format!("the kill file {} exists", self.stop_file.display()));
        }
        let strict = self.tr.is_some();
        if strict {
            self.refresh();
        } else {
            // A dry run keeps the position that the simulator holds, and checks what it can.
            self.facts.position_qty = Some(self.broker_pos);
        }
        self.sent_ms.retain(|m| *m > self.now_ms - 60_000);
        self.facts.orders_last_min = self.sent_ms.len();
        self.facts.orders_today = self.orders_today;
        self.facts.budget_left = if strict { self.budget.left(self.now_ms) } else { 1 };
        self.facts.open_buy_qty = self.open_qty(Side::Buy);
        self.facts.open_sell_qty = self.open_qty(Side::Sell);
        let (mut refusals, skipped) = check_order(&req, &self.facts, strict);
        t.refused.append(&mut refusals);
        if !t.refused.is_empty() {
            self.finish_refused(t);
            return;
        }
        let intent = json!({"kind": "paper_intent", "dry_run": !strict, "client_order_id": id, "decision_id": t.decision_id, "request": t.request, "bid": t.bid, "ask": t.ask, "host": PAPER_HOST, "not_evaluated": skipped});
        if let Err(e) = self.log(intent) {
            t.refused.push(e);
            self.finish_refused(t);
            return;
        }
        if !strict {
            t.dry = true;
            println!("  DRY RUN would send: {} {} {} limit {:.2} day (client_order_id {id}); not sent", t.side_text(), t.qty, self.symbol, t.limit);
            self.log_soft(json!({"kind": "paper_would_send", "client_order_id": id, "request": t.request}));
            if t.side == Side::Buy {
                self.broker_pos += t.qty;
            } else {
                self.broker_pos -= t.qty;
            }
            self.tracked.push(t);
            return;
        }
        self.orders_today += 1;
        self.sent_ms.push_back(self.now_ms);
        t.sent_ms = self.now_ms;
        let body = t.request.to_string();
        match self.call(Method::Post, "/v2/orders", Some(&body)) {
            Ok(v) => self.accept_reply(&mut t, &v),
            Err(e) => {
                self.log_soft(json!({"kind": "paper_error", "client_order_id": id, "stage": "send", "error": e}));
                // The order may exist (a timeout): read it by its client id, and never send it again with a new id.
                match self.call_lookup(Method::Get, &format!("/v2/orders:by_client_order_id?client_order_id={id}"), None) {
                    Ok(v) if v["client_order_id"].as_str() == Some(id.as_str()) => {
                        self.log_soft(json!({"kind": "paper_note", "client_order_id": id, "note": "the order exists at the broker although the send call failed"}));
                        self.accept_reply(&mut t, &v);
                    }
                    Err(e2) if e2.starts_with("HTTP 404") => t.refused.push(format!("the send failed and the broker has no such order: {e}")),
                    _ => {
                        t.refused.push(format!("the send failed and the state of the order is not known: {e}"));
                        self.failed.get_or_insert("the state of an order is not known".into());
                    }
                }
            }
        }
        if !t.refused.is_empty() {
            self.finish_refused(t);
            return;
        }
        println!("  PAPER ORDER sent: {} {} {} limit {:.2} day, status {} (client_order_id {id})", t.side_text(), t.qty, self.symbol, t.limit, t.status);
        self.tracked.push(t);
    }

    /// Accept the reply to a send (or the read by client id) only when it matches the request. A reply with no id is read again by the
    /// client id, so that an open order is never left untracked. Any mismatch marks the order failed and stops the run.
    fn accept_reply(&mut self, t: &mut Tracked, v: &Value) {
        let mut v = v.clone();
        if v["id"].as_str().is_none_or(str::is_empty) {
            let path = format!("/v2/orders:by_client_order_id?client_order_id={}", t.client_order_id);
            match self.call_lookup(Method::Get, &path, None) {
                Ok(r) => v = r,
                Err(e) => {
                    self.mark_failed(t, format!("the reply has no order id and the read by client id failed: {e}"));
                    return;
                }
            }
        }
        if let Err(m) = verify_order(&self.symbol, t, &v) {
            self.mark_failed(t, m);
            return;
        }
        let view = order_view(&v);
        t.broker_id = v["id"].as_str().map(String::from);
        t.status = v["status"].as_str().unwrap_or("unknown").to_string();
        if let Err(m) = self.apply_fill(t, &v) {
            self.mark_failed(t, m);
            return;
        }
        self.log_soft(json!({"kind": "paper_order", "client_order_id": t.client_order_id, "broker": view}));
        t.broker = Some(view);
    }

    fn mark_failed(&mut self, t: &mut Tracked, why: String) {
        t.refused.push(why.clone());
        t.status = "failed".into();
        self.failed.get_or_insert(why.clone());
        self.log_soft(json!({"kind": "paper_error", "client_order_id": t.client_order_id, "stage": "reply check", "error": why}));
    }

    /// Update the fills of an order from a reply: the cash flow and the broker position follow the filled quantity.
    /// The filled quantity must be a whole number from 0 to the order quantity, and it must not fall.
    fn apply_fill(&mut self, t: &mut Tracked, v: &Value) -> Result<(), String> {
        let raw = &v["filled_qty"];
        let fq = if raw.is_null() { 0.0 } else { num(raw).ok_or("the filled quantity is not a number")? };
        if fq < 0.0 || fq.fract() != 0.0 {
            return Err(format!("the filled quantity {fq} is negative or not a whole number"));
        }
        if fq > t.qty as f64 {
            return Err(format!("the filled quantity {fq} is above the order quantity {}", t.qty));
        }
        let fq = fq as i64;
        if fq < t.filled_qty {
            return Err(format!("the filled quantity fell from {} to {fq}", t.filled_qty));
        }
        if fq > t.filled_qty {
            let avg = num(&v["filled_avg_price"]).filter(|a| *a > 0.0).ok_or("the order has fills and no valid filled average price")?;
            let sign = t.side.sign() as f64;
            self.cash_flow -= sign * (fq as f64 * avg - t.filled_qty as f64 * t.filled_avg);
            self.broker_pos += t.side.sign() * (fq - t.filled_qty);
            t.filled_qty = fq;
            t.filled_avg = avg;
        }
        Ok(())
    }

    fn finish_refused(&mut self, mut t: Tracked) {
        let id = t.client_order_id.clone();
        println!("  PAPER ORDER REFUSED ({} {} {}): {}", t.side_text(), t.qty, self.symbol, t.refused.join("; "));
        self.log_soft(json!({"kind": "paper_refused", "client_order_id": id, "decision_id": t.decision_id, "request": t.request, "reasons": t.refused}));
        t.status = "refused".into();
        self.tracked.push(t);
    }

    fn write_pairs(&mut self, all: bool) {
        for i in 0..self.tracked.len() {
            let t = &self.tracked[i];
            if t.written || !(t.done() || all) || (t.sim.is_none() && !all) {
                continue;
            }
            let pair = json!({
                "kind": "paper_pair", "client_order_id": t.client_order_id, "decision_id": t.decision_id, "dry_run": t.dry,
                "decision": {"side": t.side_text(), "qty": t.qty, "decision_ms": t.decision_ms, "bid": t.bid, "ask": t.ask, "limit": t.limit},
                "request": t.request, "refused": t.refused,
                "broker": t.broker, "broker_status": t.status, "broker_filled_qty": t.filled_qty, "broker_filled_avg_price": if t.filled_qty > 0 { json!(t.filled_avg) } else { Value::Null },
                "simulator": t.sim,
            });
            self.tracked[i].written = true;
            self.log_soft(pair);
        }
    }

    fn poll_orders(&mut self) {
        if self.tr.is_none() || self.failed.is_some() {
            return;
        }
        let open: Vec<usize> = (0..self.tracked.len()).filter(|&i| self.tracked[i].open()).collect();
        if open.is_empty() || self.now_ms - self.last_poll_ms < POLL_EVERY_MS {
            return;
        }
        self.last_poll_ms = self.now_ms;
        for i in open {
            let Some(bid) = self.tracked[i].broker_id.clone() else { continue };
            if self.budget.left(self.now_ms) == 0 {
                return;
            }
            let Ok(v) = self.call(Method::Get, &format!("/v2/orders/{bid}"), None) else { continue };
            self.update_from(i, &v);
            let t = &self.tracked[i];
            if t.open() && !t.cancel_sent && self.now_ms - t.sent_ms >= ORDER_TTL_MS {
                self.tracked[i].cancel_sent = true;
                let cid = self.tracked[i].client_order_id.clone();
                let r = self.call(Method::Delete, &format!("/v2/orders/{bid}"), None);
                self.log_soft(json!({"kind": "paper_cancel", "client_order_id": cid, "reason": "not filled within the time limit", "ok": r.is_ok()}));
            }
        }
        self.write_pairs(false);
    }

    fn update_from(&mut self, i: usize, v: &Value) {
        let (want_id, want_cid) = (self.tracked[i].broker_id.clone(), self.tracked[i].client_order_id.clone());
        if v["id"].as_str() != want_id.as_deref() || v["client_order_id"].as_str() != Some(want_cid.as_str()) {
            let why = "the reply to a read of an order is for another order".to_string();
            self.failed.get_or_insert(why.clone());
            self.log_soft(json!({"kind": "paper_error", "client_order_id": want_cid, "stage": "status check", "error": why}));
            return;
        }
        let mut t = std::mem::replace(&mut self.tracked[i], Tracked::blank());
        let (old_status, old_fq) = (t.status.clone(), t.filled_qty);
        if let Err(why) = self.apply_fill(&mut t, v) {
            self.failed.get_or_insert(why.clone());
            self.log_soft(json!({"kind": "paper_error", "client_order_id": t.client_order_id, "stage": "status check", "error": why}));
            self.tracked[i] = t;
            return;
        }
        t.status = v["status"].as_str().unwrap_or(&t.status).to_string();
        if t.status != old_status || t.filled_qty != old_fq {
            let view = order_view(v);
            self.log_soft(json!({"kind": "paper_status", "client_order_id": t.client_order_id, "broker": view}));
            t.broker = Some(view);
        }
        self.tracked[i] = t;
    }

    /// Cancel every open order of the symbol at the broker. It skips the request budget and the error count.
    fn cancel_all(&mut self, why: &str) -> usize {
        let mut cancelled = 0;
        match self.call_critical(Method::Get, &format!("/v2/orders?status=open&symbols={}&limit=50", self.symbol), None) {
            Ok(Value::Array(list)) => {
                for o in list {
                    if let Some(id) = o["id"].as_str() {
                        let r = self.call_critical(Method::Delete, &format!("/v2/orders/{id}"), None);
                        cancelled += usize::from(r.is_ok());
                        self.log_soft(json!({"kind": "paper_cancel", "reason": why, "broker_order_id": id, "ok": r.is_ok()}));
                    }
                }
            }
            other => self.log_soft(json!({"kind": "paper_error", "stage": "list open orders at the stop", "error": format!("{other:?}").chars().take(200).collect::<String>()})),
        }
        cancelled
    }

    /// The broker day P&L: cash flow of the fills plus the position at the mid.
    pub fn day_pnl(&self) -> f64 {
        self.cash_flow + self.broker_pos as f64 * self.last_mid
    }
}

impl Tracked {
    fn side_text(&self) -> String {
        format!("{:?}", self.side).to_lowercase()
    }
    fn blank() -> Tracked {
        Tracked { decision_id: None, client_order_id: String::new(), side: Side::Buy, qty: 0, limit: 0.0, bid: 0.0, ask: 0.0, decision_ms: 0, request: Value::Null, broker_id: None, broker: None, status: String::new(), filled_qty: 0, filled_avg: 0.0, refused: vec![], dry: false, sim: None, sent_ms: 0, cancel_sent: false, written: true }
    }
}

impl OrderHook for PaperHook {
    fn poll(&mut self, input: &Input) {
        self.now_ms = input.read_ms;
        self.last_mid = input.mid();
        self.poll_orders();
        if self.tr.is_some() && self.failed.is_none() && self.day_pnl() <= -self.day_loss_limit {
            self.failed = Some(format!("paper daily loss stop: broker P&L {:.2} USD is at or below -{:.2} USD ({}% of the capital)", self.day_pnl(), self.day_loss_limit, LIMITS.day_loss_pct));
        }
    }

    fn decision(&mut self, ctx: &DecisionCtx) {
        self.now_ms = ctx.input.read_ms;
        let (bid, ask) = (ctx.input.bid, ctx.input.ask);
        // A limit order at the touch: a buy at the ask, a sell at the bid (the NBBO is at least as good as the IEX quote: its ask is not above the IEX ask, its bid is not below the IEX bid, so a limit at the IEX touch is marketable).
        let touch = if ctx.side == Side::Buy { ask } else { bid };
        let limit = match (ctx.limit, ctx.side) {
            (Some(l), Side::Buy) => touch.min(l),
            (Some(l), Side::Sell) => touch.max(l),
            (None, _) => touch,
        };
        let t = Tracked {
            decision_id: Some(ctx.id),
            side: ctx.side,
            qty: ctx.qty.min(LIMITS.max_qty).max(0),
            limit: cents(limit),
            bid,
            ask,
            decision_ms: ctx.input.read_ms,
            written: false,
            ..Tracked::blank()
        };
        if ctx.qty > LIMITS.max_qty {
            self.log_soft(json!({"kind": "paper_note", "decision_id": ctx.id, "note": format!("the size {} is cut to {} share (step A)", ctx.qty, LIMITS.max_qty)}));
        }
        self.submit(t);
    }

    fn sim_result(&mut self, id: usize, record: &Value) {
        if let Some(t) = self.tracked.iter_mut().find(|t| t.decision_id == Some(id)) {
            t.sim = Some(record.clone());
        }
        self.write_pairs(false);
    }

    /// Control 4: cancel all open orders, KEEP the position, write the log line, send no new order. A person closes a position.
    /// The simulator closes its own position for its P&L (`sim` is that record); the broker position is not touched.
    fn stop(&mut self, why: Stop, input: &Input, sim: Option<&Value>) {
        self.now_ms = input.read_ms;
        self.stopped = true;
        if self.tr.is_none() {
            println!("  DRY RUN stop ({}): would cancel the open orders, keep the position ({} shares), send no new order", why.label(), self.broker_pos);
            self.log_soft(json!({"kind": "paper_stop", "reason": why.label(), "dry_run": true, "position_kept_qty": self.broker_pos, "simulator_close": sim}));
            self.write_pairs(true);
            return;
        }
        println!("  STOP ({}): cancel all open orders, keep the position, send no new order", why.label());
        let cancelled = self.cancel_all(why.label());
        for i in 0..self.tracked.len() {
            if self.tracked[i].open() {
                if let Some(bid) = self.tracked[i].broker_id.clone() {
                    if let Ok(v) = self.call_critical(Method::Get, &format!("/v2/orders/{bid}"), None) {
                        self.update_from(i, &v);
                    }
                }
            }
        }
        // The position that is kept: quantity and average entry price (no account number).
        let kept = self.call_critical(Method::Get, "/v2/positions", None).and_then(|v| parse_positions(&v, &self.symbol).map(|(q, _)| (q, avg_entry(&v, &self.symbol))));
        match kept {
            Ok((q, avg)) => {
                self.broker_pos = q;
                self.log_soft(json!({"kind": "paper_stop", "reason": why.label(), "dry_run": false, "cancelled": cancelled, "position_kept_qty": q, "position_kept_avg_entry_price": avg, "simulator_close": sim}));
                if q != 0 {
                    println!("  position kept: {q} shares of {} (average entry {}); a person closes it", self.symbol, avg.map(|a| format!("{a:.2}")).unwrap_or_else(|| "not known".into()));
                } else {
                    println!("  position kept: 0 shares");
                }
            }
            Err(e) => {
                self.log_soft(json!({"kind": "paper_stop", "reason": why.label(), "dry_run": false, "cancelled": cancelled, "position_kept_qty": Value::Null, "error": e, "simulator_close": sim}));
                println!("  WARNING: the position at the broker is not known: check the paper account by hand");
            }
        }
        self.write_pairs(true);
    }

    fn stop_request(&self) -> Option<String> {
        self.failed.clone()
    }

    fn finish(&mut self) {
        self.write_pairs(true);
        if self.tr.is_some() {
            println!("  position kept: {} shares; a person closes it (the engine and the broker close nothing)", self.broker_pos);
        }
        let n = self.tracked.iter().filter(|t| !t.dry && t.refused.is_empty()).count();
        self.log_soft(json!({"kind": "paper_end", "orders_sent": n, "orders_today": self.orders_today, "trading_requests_api_errors_in_a_row": self.api_errors, "broker_pos": self.broker_pos, "broker_day_pnl": self.day_pnl()}));
    }
}

impl Drop for PaperHook {
    /// An exit with no stop (an error path, a panic that unwinds) still cancels the open orders. The position is kept.
    fn drop(&mut self) {
        if self.tr.is_some() && !self.stopped {
            self.stopped = true;
            let n = self.cancel_all("abnormal exit");
            self.log_soft(json!({"kind": "paper_stop", "reason": "abnormal exit", "dry_run": false, "cancelled": n}));
            println!("  abnormal exit: {n} open order(s) cancelled; the position is kept; a person checks the paper account");
        }
    }
}

/// The path of the order journal. It is under `data/processed/algo/`, which git ignores.
pub fn journal_path(algo_dir: &Path, date: &str, strategy: &str) -> PathBuf {
    algo_dir.join(date).join(format!("{strategy}-orders.jsonl"))
}

pub fn peak_path(algo_dir: &Path) -> PathBuf {
    algo_dir.join("paper-sleeve-peak.json")
}

/// The reply to an order call must be the order that we asked for: id, client id, symbol, side, quantity, type, day and limit price.
fn verify_order(symbol: &str, t: &Tracked, v: &Value) -> Result<(), String> {
    let bad = |name: &str| Err(format!("the broker reply does not match the request: {name}"));
    if v["id"].as_str().is_none_or(str::is_empty) {
        return bad("id");
    }
    if v["client_order_id"].as_str() != Some(t.client_order_id.as_str()) {
        return bad("client_order_id");
    }
    if v["symbol"].as_str() != Some(symbol) {
        return bad("symbol");
    }
    if v["side"].as_str() != Some(t.side_text().as_str()) {
        return bad("side");
    }
    if num(&v["qty"]) != Some(t.qty as f64) {
        return bad("qty");
    }
    if v["type"].as_str() != Some("limit") || v["time_in_force"].as_str() != Some("day") {
        return bad("type or time_in_force");
    }
    if !num(&v["limit_price"]).is_some_and(|p| (p - t.limit).abs() < 1e-9) {
        return bad("limit_price");
    }
    Ok(())
}

// ---------------------------------------------------------------- where an order run may start

/// The facts about the machine and the session, as data. Tests give their own; `from_process` reads the real ones.
pub struct OrderEnv {
    /// The names of the environment variables (the values are not needed).
    pub vars: Vec<String>,
    /// The order host file, outside the repo: `$HOME/.config/fa/order-host` (Windows: `%APPDATA%\fa\order-host`).
    pub host_file: Option<PathBuf>,
    pub hostname: String,
    pub windows: bool,
}

impl OrderEnv {
    pub fn from_process() -> OrderEnv {
        let windows = cfg!(windows);
        let host_file = if windows {
            std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("fa").join("order-host"))
        } else {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("fa").join("order-host"))
        };
        OrderEnv { vars: std::env::vars_os().map(|(k, _)| k.to_string_lossy().to_string()).collect(), host_file, hostname: host_name(), windows }
    }
}

/// The one command that makes the order host file on this machine.
pub fn order_host_command(env: &OrderEnv) -> String {
    if env.windows {
        format!("mkdir %APPDATA%\\fa & echo {}> %APPDATA%\\fa\\order-host", env.hostname)
    } else {
        format!("mkdir -p ~/.config/fa && echo {} > ~/.config/fa/order-host", env.hostname)
    }
}

/// `--send` is allowed only on the laptop or the order host, never in a Claude Code or cloud session: a session can hold real paper keys.
/// (a) no variable `CLAUDECODE` or `CCR_*`; (b) the order host file, outside the repo, whose first line is the host name of this machine.
pub fn check_order_host(env: &OrderEnv, repo: &Path) -> Result<(), String> {
    if env.vars.iter().any(|v| v == "CLAUDECODE" || v.starts_with("CCR_")) {
        return Err("orders are sent only from the laptop or the order host, not from a Claude Code or cloud session".into());
    }
    let help = format!("--send needs the order host file; make it on this machine with: {}", order_host_command(env));
    let Some(file) = &env.host_file else {
        return Err(format!("no home folder is known. {help}"));
    };
    if file.starts_with(repo) {
        return Err(format!("the order host file {} is inside the repo: it must be outside. {help}", file.display()));
    }
    let text = std::fs::read_to_string(file).map_err(|_| format!("the order host file {} is missing. {help}", file.display()))?;
    if text.lines().next().map(str::trim) != Some(env.hostname.as_str()) {
        return Err(format!("the first line of {} is not the host name of this machine. {help}", file.display()));
    }
    Ok(())
}

/// The start gate without the paper keys (no request is possible): the New York hours rule, weekdays 09:30 to 16:00.
/// A holiday is not known without the clock. `Some(text)` means closed.
pub fn ny_hours_gate(utc_secs: i64) -> Option<String> {
    let (date, tod) = crate::market::alpaca::ny_day(utc_secs);
    let weekend = crate::trading::calendar::parse_iso(&date).is_some_and(|d| crate::trading::calendar::weekday(d) >= 5);
    if weekend || !(9 * 3600 + 30 * 60..16 * 3600).contains(&tod) {
        return Some(format!("market closed (New York hours rule, weekdays 09:30 to 16:00; holidays are not known without the clock): it is {date} {:02}:{:02} New York", tod / 3600, tod % 3600 / 60));
    }
    None
}

// ---------------------------------------------------------------- the market clock (read only)

/// The result of `GET /v2/clock` on the paper host constant.
#[derive(Debug, Clone, PartialEq)]
pub enum Clock {
    Open,
    Closed { next_open: String },
}

/// Reads the market clock (a read, no order). It fails closed: any error is an `Err`, and the caller refuses to start.
pub fn read_clock(t: &mut dyn Transport) -> Result<Clock, String> {
    let url = paper_url("/v2/clock")?;
    let r = t.send(Method::Get, &url, None).map_err(|e| format!("the market clock call failed: {e}"))?;
    if !(200..300).contains(&r.status) {
        return Err(format!("the market clock call gave HTTP {}", r.status));
    }
    let v: Value = serde_json::from_str(&r.body).map_err(|_| "the market clock reply is not JSON".to_string())?;
    match v["is_open"].as_bool() {
        Some(true) => Ok(Clock::Open),
        Some(false) => Ok(Clock::Closed { next_open: v["next_open"].as_str().unwrap_or("unknown").to_string() }),
        None => Err("the market clock reply has no is_open field".into()),
    }
}

/// The start gate of `fa algo run`: `Ok(None)` the market is open, `Ok(Some(text))` it is closed (print the text, exit 0, no loop),
/// `Err` the clock could not be read (refuse to start).
pub fn market_gate(t: &mut dyn Transport) -> Result<Option<String>, String> {
    Ok(match read_clock(t)? {
        Clock::Open => None,
        Clock::Closed { next_open } => Some(format!("market closed (Alpaca clock): next open {next_open}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::algo::engine::{Config, Engine};
    use crate::trading::algo::{Intent, Strategy};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    const T0: i64 = 1_790_000_000_000; // 2026-09-21 10:13:20 New York, a Monday
    const ACCOUNT_NUMBER: &str = "PASECRETNUMBER999";
    const KEY: &str = "KEYIDVALUE123";
    const SECRET: &str = "SECRETVALUE456";

    #[derive(Clone, Debug)]
    struct Req {
        method: Method,
        url: String,
        body: Option<String>,
    }

    struct Ord1 {
        id: String,
        cid: String,
        side: String,
        qty: i64,
        limit: String,
        status: String,
    }

    struct World {
        pos: i64,
        cash: f64,
        equity: f64,
        status: &'static str,
        orders: Vec<Ord1>,
        /// A new order fills at once.
        fill: bool,
        /// An open order fills at its next read.
        fill_on_get: bool,
        post_override: Option<Result<(u16, String), String>>,
        /// The body of every read of one order (a wrong reply).
        get_override: Option<String>,
        fail_all: bool,
        clock_open: bool,
    }

    fn world() -> Rc<RefCell<World>> {
        Rc::new(RefCell::new(World { pos: 0, cash: 100_000.0, equity: 100_000.0, status: "ACTIVE", orders: vec![], fill: true, fill_on_get: false, post_override: None, get_override: None, fail_all: false, clock_open: true }))
    }

    struct Mock {
        log: Rc<RefCell<Vec<Req>>>,
        w: Rc<RefCell<World>>,
    }

    fn order_json(o: &Ord1) -> String {
        let filled = o.status == "filled";
        json!({"id": o.id, "client_order_id": o.cid, "status": o.status, "symbol": "SPY", "side": o.side, "qty": o.qty.to_string(), "type": "limit", "time_in_force": "day", "limit_price": o.limit,
               "filled_qty": if filled { o.qty.to_string() } else { "0".into() }, "filled_avg_price": if filled { json!(o.limit) } else { Value::Null }, "submitted_at": "2026-09-21T14:13:20Z", "account_number": ACCOUNT_NUMBER})
        .to_string()
    }

    impl Transport for Mock {
        fn send(&mut self, method: Method, url: &str, body: Option<&str>) -> Result<Reply, String> {
            self.log.borrow_mut().push(Req { method, url: url.to_string(), body: body.map(String::from) });
            let mut w = self.w.borrow_mut();
            if w.fail_all {
                return Err(format!("connection reset (headers {KEY} {SECRET})"));
            }
            let path = url.strip_prefix(PAPER_HOST).unwrap_or("NOT-PAPER").to_string();
            let ok = |b: String| Ok(Reply { status: 200, body: b });
            match (method, path.as_str()) {
                (Method::Get, "/v2/clock") => ok(json!({"is_open": w.clock_open, "next_open": "2026-09-22T09:30:00-04:00"}).to_string()),
                (Method::Get, "/v2/account") => ok(json!({"status": w.status, "trading_blocked": false, "account_blocked": false, "currency": "USD", "equity": w.equity.to_string(), "cash": w.cash.to_string(), "account_number": ACCOUNT_NUMBER}).to_string()),
                (Method::Get, "/v2/positions") => {
                    let list = if w.pos == 0 { vec![] } else { vec![json!({"symbol": "SPY", "qty": w.pos.to_string(), "market_value": (w.pos as f64 * 700.0).to_string()})] };
                    ok(Value::Array(list).to_string())
                }
                (Method::Get, p) if p.starts_with("/v2/orders?status=open") => ok(Value::Array(w.orders.iter().filter(|o| o.status == "accepted").map(|o| serde_json::from_str(&order_json(o)).unwrap()).collect()).to_string()),
                (Method::Post, "/v2/orders") => {
                    if let Some(r) = w.post_override.clone() {
                        return r.map(|(status, body)| Reply { status, body });
                    }
                    let b: Value = serde_json::from_str(body.unwrap_or("{}")).unwrap();
                    let n = w.orders.len() + 1;
                    let (qty, side, fill) = (b["qty"].as_str().unwrap().parse::<i64>().unwrap(), b["side"].as_str().unwrap().to_string(), w.fill);
                    let o = Ord1 { id: format!("ord-{n}"), cid: b["client_order_id"].as_str().unwrap().into(), side: side.clone(), qty, limit: b["limit_price"].as_str().unwrap().into(), status: if fill { "filled".into() } else { "accepted".into() } };
                    if fill {
                        w.pos += if side == "buy" { qty } else { -qty };
                    }
                    let out = order_json(&o);
                    w.orders.push(o);
                    ok(out)
                }
                (Method::Get, p) if p.starts_with("/v2/orders:by_client_order_id") => {
                    let cid = p.split("client_order_id=").nth(1).unwrap_or("").to_string();
                    match w.orders.iter().find(|o| o.cid == cid) {
                        Some(o) => ok(order_json(o)),
                        None => Ok(Reply { status: 404, body: "{\"message\":\"order not found\"}".into() }),
                    }
                }
                (Method::Get, p) if p.starts_with("/v2/orders/") => {
                    if let Some(b) = w.get_override.clone() {
                        return ok(b);
                    }
                    let id = p.trim_start_matches("/v2/orders/").to_string();
                    let fill_on_get = w.fill_on_get;
                    let mut pos_change = 0;
                    let Some(o) = w.orders.iter_mut().find(|o| o.id == id) else { return Ok(Reply { status: 404, body: "{}".into() }) };
                    if fill_on_get && o.status == "accepted" {
                        o.status = "filled".into();
                        pos_change = if o.side == "buy" { o.qty } else { -o.qty };
                    }
                    let out = order_json(o);
                    w.pos += pos_change;
                    ok(out)
                }
                (Method::Delete, p) if p.starts_with("/v2/orders/") => {
                    let id = p.trim_start_matches("/v2/orders/").to_string();
                    if let Some(o) = w.orders.iter_mut().find(|o| o.id == id && o.status == "accepted") {
                        o.status = "canceled".into();
                    }
                    Ok(Reply { status: 204, body: String::new() })
                }
                _ => Ok(Reply { status: 404, body: format!("no route {path}") }),
            }
        }
    }

    #[derive(Clone, Default)]
    struct Buf(Rc<RefCell<Vec<u8>>>);
    impl Write for Buf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Buf {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.borrow()).to_string()
        }
        fn lines(&self) -> Vec<Value> {
            self.text().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
        }
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fa-paper-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn inp(ms: i64, bid: f64, ask: f64) -> Input {
        Input { quote_ms: ms - 50, read_ms: ms, symbol: "SPY".into(), bid, ask, kind: "quote".into() }
    }

    struct Rig {
        h: PaperHook,
        buf: Buf,
        log: Rc<RefCell<Vec<Req>>>,
        w: Rc<RefCell<World>>,
        dir: PathBuf,
    }

    fn rig_with(name: &str, prior: &str, budget: usize, start: bool) -> Rig {
        let (w, log, buf, dir) = (world(), Rc::new(RefCell::new(vec![])), Buf::default(), tmp(name));
        let mode = Mode::Send(Box::new(Mock { log: log.clone(), w: w.clone() }), vec![KEY.into(), SECRET.into()]);
        let mut h = PaperHook::new(mode, "SPY", "demo", dir.join("STOP"), Box::new(buf.clone()), prior, budget);
        if start {
            h.start(T0, Ok(2.0e9), &dir.join("peak.json")).unwrap();
        }
        Rig { h, buf, log, w, dir }
    }

    fn rig(name: &str) -> Rig {
        rig_with(name, "", 60, true)
    }

    fn decide(h: &mut PaperHook, id: usize, side: Side, qty: i64, i: &Input) {
        h.decision(&DecisionCtx { id, side, qty, limit: None, pos_before: 0, input: i });
    }

    fn posts(r: &Rig) -> Vec<Req> {
        r.log.borrow().iter().filter(|q| q.method == Method::Post).cloned().collect()
    }

    fn live_host() -> String {
        let parts = ["api", "alpaca", "markets"];
        format!("https://{}", parts.join("."))
    }

    // ---- control 1: endpoint

    #[test]
    fn host_is_compared_exactly_and_the_url_is_built_from_the_constant() {
        assert_eq!(PAPER_HOST, "https://paper-api.alpaca.markets");
        assert!(host_is_paper("https://paper-api.alpaca.markets/v2/account"));
        assert!(host_is_paper("https://paper-api.alpaca.markets"));
        let live = live_host();
        for bad in [
            format!("{live}/v2/account"),
            "https://paper-api.alpaca.markets.evil.com/v2/account".to_string(),
            "https://paper-api.alpaca-markets.com/v2/account".to_string(),
            "https://paper-api.alpaca.markets@evil.com/v2/account".to_string(),
            "https://evil.com/paper-api.alpaca.markets".to_string(),
            "http://paper-api.alpaca.markets/v2/account".to_string(),
            "https://paper-api.alpaca.markets:8443/v2/account".to_string(),
            "https://PAPER-API.alpaca.markets/v2/account".to_string(),
            "https://paper-api.alpaca.markets\\@evil.com/".to_string(),
            "https://paper-api.alpaca.markets /v2".to_string(),
            "//paper-api.alpaca.markets/v2".to_string(),
        ] {
            assert!(!host_is_paper(&bad), "{bad}");
        }
        assert_eq!(paper_url("/v2/account").unwrap(), "https://paper-api.alpaca.markets/v2/account");
        for bad in ["//evil.com/v2/x", "/v1/x", "https://evil.com/v2/x", "/v2/x://evil", "v2/account", "/v2/a b"] {
            assert!(paper_url(bad).is_err(), "{bad}");
        }
        // The real transport refuses any other host before it makes a call (no network here).
        let mut t = RealTransport::new(PaperKeys { id: KEY.into(), secret: SECRET.into() });
        assert!(t.send(Method::Get, &format!("{live}/v2/account"), None).unwrap_err().contains("not on the paper host"));
    }

    #[test]
    fn a_base_url_setting_never_changes_the_order_host() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let var = crate::market::alpaca::TRADING_BASE_ENV;
        let old = std::env::var(var).ok();
        for (k, setting) in [("live", live_host()), ("lookalike", "https://paper-api.alpaca-markets.com".to_string())] {
            std::env::set_var(var, &setting);
            let mut r = rig(&format!("env-{k}"));
            decide(&mut r.h, 0, Side::Buy, 50, &inp(T0, 100.0, 100.1));
            let log = r.log.borrow();
            assert!(!log.is_empty() && log.iter().any(|q| q.method == Method::Post), "the order is sent");
            assert!(log.iter().all(|q| q.url.starts_with("https://paper-api.alpaca.markets/v2/")), "{log:?}");
            assert!(log.iter().all(|q| !q.url.contains(&setting) && !q.url.contains("evil")), "{log:?}");
        }
        match old {
            Some(v) => std::env::set_var(var, v),
            None => std::env::remove_var(var),
        }
    }

    #[test]
    fn a_redirect_is_not_followed_and_the_order_is_not_sent_again() {
        let mut r = rig("redirect");
        r.w.borrow_mut().post_override = Some(Ok((307, String::new())));
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert_eq!(posts(&r).len(), 1, "one POST only");
        assert!(r.log.borrow().iter().all(|q| q.url.starts_with(PAPER_HOST)));
        let l = r.buf.lines();
        assert!(l.iter().any(|v| v["kind"] == "paper_error" && v["error"].as_str().unwrap().contains("307")), "{l:?}");
        assert!(l.iter().any(|v| v["kind"] == "paper_refused"));
    }

    // ---- control 2: keys

    #[test]
    fn only_the_paper_variables_are_read() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("keys");
        let (a, b) = (std::env::var(KEY_ID_VAR).ok(), std::env::var(SECRET_VAR).ok());
        std::env::remove_var(KEY_ID_VAR);
        std::env::remove_var(SECRET_VAR);
        // The data key variables (the cloud proxy key of a person) may be set: they are not used.
        assert!(PaperKeys::read(&dir).is_none());
        std::env::set_var(KEY_ID_VAR, "x-id-1234");
        assert!(PaperKeys::read(&dir).is_none(), "one variable is not enough");
        std::env::set_var(SECRET_VAR, "x-secret-1234");
        assert!(PaperKeys::read(&dir).is_some());
        for (k, v) in [(KEY_ID_VAR, a), (SECRET_VAR, b)] {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }

    #[test]
    fn without_send_or_without_keys_no_request_is_possible() {
        let boom = |_k: PaperKeys| -> Box<dyn Transport> { panic!("a transport was made") };
        // No --send: the keys are not used, the mode has no transport.
        let m = choose_mode(false, Some(PaperKeys { id: KEY.into(), secret: SECRET.into() }), boom).unwrap();
        assert!(matches!(m, Mode::DryRun));
        // --send without the keys: refused, no transport.
        let e = choose_mode(true, None, boom).err().unwrap();
        assert!(e.contains(KEY_ID_VAR) && e.contains("No request was made"), "{e}");
        // A dry run over decisions and a stop: no transport, so no request; the order is journaled as "would send".
        let buf = Buf::default();
        let dir = tmp("dry");
        let mut h = PaperHook::new(Mode::DryRun, "SPY", "demo", dir.join("STOP"), Box::new(buf.clone()), "", 60);
        assert!(!h.is_send());
        assert!(h.start_dry()[0].contains("DRY RUN"));
        decide(&mut h, 0, Side::Buy, 50, &inp(T0, 100.0, 100.1));
        h.stop(Stop::Until, &inp(T0 + 5000, 100.2, 100.3), None);
        h.finish();
        let l = buf.lines();
        assert!(l.iter().any(|v| v["kind"] == "paper_would_send" && v["request"]["limit_price"] == "100.10" && v["request"]["qty"] == "1"));
        assert!(l.iter().filter(|v| v["kind"] == "paper_intent").all(|v| v["dry_run"] == true));
        assert!(l.iter().any(|v| v["kind"] == "paper_stop" && v["dry_run"] == true && v["position_kept_qty"] == 1), "the stop keeps the position");
        assert_eq!(l.iter().filter(|v| v["kind"] == "paper_would_send").count(), 1, "no close order");
        assert!(!buf.text().contains(PAPER_HOST) || buf.text().contains("\"host\""), "only the host note");
    }

    #[test]
    fn a_send_run_with_the_kill_file_makes_no_post() {
        let mut r = rig("kill-no-post");
        std::fs::write(r.dir.join("STOP"), "stop").unwrap();
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert!(posts(&r).is_empty());
        assert!(r.buf.lines().iter().any(|v| v["kind"] == "paper_refused" && v["reasons"][0].as_str().unwrap().contains("kill file")));
    }

    #[test]
    fn the_account_check_refuses_a_bad_account_before_any_order() {
        for status in ["ACCOUNT_CLOSED", "SUBMITTED", ""] {
            let mut r = rig_with("acct-bad", "", 60, false);
            r.w.borrow_mut().status = Box::leak(status.to_string().into_boxed_str());
            let e = r.h.start(T0, Ok(1e9), &r.dir.join("peak.json")).unwrap_err();
            assert!(e.contains("paper: no"), "{e}");
            assert_eq!(r.h.paper_check, "no");
            assert!(posts(&r).is_empty());
        }
        // The account must also answer: an API error is "paper: no".
        let mut r = rig_with("acct-down", "", 60, false);
        r.w.borrow_mut().fail_all = true;
        assert!(r.h.start(T0, Ok(1e9), &r.dir.join("peak.json")).is_err());
        assert_eq!(r.h.paper_check, "no");
        // A good account: yes, and no account number or key in the lines, the journal or the peak file.
        let mut r = rig_with("acct-ok", "", 60, false);
        let lines = r.h.start(T0, Ok(2e9), &r.dir.join("peak.json")).unwrap();
        assert_eq!(r.h.paper_check, "yes");
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        r.h.finish();
        let all = format!("{}{}{}", lines.join("\n"), r.buf.text(), std::fs::read_to_string(r.dir.join("peak.json")).unwrap());
        for secret in [ACCOUNT_NUMBER, KEY, SECRET] {
            assert!(!all.contains(secret), "{secret} leaked");
        }
    }

    #[test]
    fn a_secret_in_an_error_text_is_taken_out() {
        let mut r = rig("scrub");
        r.w.borrow_mut().post_override = Some(Err(format!("connection reset (headers {KEY} {SECRET})")));
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        let t = r.buf.text();
        assert!(t.contains("paper_error") && t.contains("***"));
        assert!(!t.contains(KEY) && !t.contains(SECRET));
    }

    #[test]
    fn start_refuses_a_held_position_open_orders_and_a_sleeve_stop() {
        let mut r = rig_with("held", "", 60, false);
        r.w.borrow_mut().pos = 3;
        assert!(r.h.start(T0, Ok(1e9), &r.dir.join("peak.json")).unwrap_err().contains("holds 3"));
        let mut r = rig_with("openord", "", 60, false);
        r.w.borrow_mut().orders.push(Ord1 { id: "o9".into(), cid: "x".into(), side: "buy".into(), qty: 1, limit: "1.00".into(), status: "accepted".into() });
        assert!(r.h.start(T0, Ok(1e9), &r.dir.join("peak.json")).unwrap_err().contains("open orders"));
        // No liquidity figure: the limit cannot be checked, so the run is refused (fail closed).
        let mut r = rig_with("noadv", "", 60, false);
        assert!(r.h.start(T0, Err("no bars".into()), &r.dir.join("peak.json")).unwrap_err().contains("liquidity"));
        // The sleeve stop: a peak file with a peak 12% above the equity.
        let dir = tmp("peak");
        let p = dir.join("peak.json");
        std::fs::write(&p, "{\"peak_equity\": 113636.4}").unwrap();
        assert!(sleeve_peak(&p, 100_000.0).unwrap_err().contains("sleeve stop"));
        assert!(sleeve_peak(&p, 105_000.0).is_ok());
        std::fs::write(&p, "garbage").unwrap();
        assert!(sleeve_peak(&p, 105_000.0).is_err(), "an unreadable file refuses");
        assert_eq!(sleeve_peak(&dir.join("new.json"), 50_000.0).unwrap(), 50_000.0);
    }

    // ---- control 3: limits

    fn good_facts() -> Facts {
        Facts { capital: Some(100_000.0), cash: Some(100_000.0), gross: Some(0.0), position_qty: Some(0), adv: Some(1e9), budget_left: 50, ..Facts::default() }
    }
    fn buy1(px: f64) -> OrderReq {
        OrderReq { symbol: "SPY".into(), side: Side::Buy, qty: 1, limit: px }
    }

    #[test]
    fn the_limit_constant_is_the_paper_table() {
        assert_eq!((LIMITS.gross_pct, LIMITS.position_entry_pct, LIMITS.position_market_pct, LIMITS.issuer_pct, LIMITS.adv_pct), (100.0, 10.0, 15.0, 15.0, 1.0));
        assert_eq!((LIMITS.day_loss_pct, LIMITS.sleeve_stop_pct, LIMITS.order_pct), (2.0, 10.0, 5.0));
        assert_eq!((LIMITS.orders_per_day, LIMITS.orders_per_min, LIMITS.max_qty), (10, 6, 1));
    }

    #[test]
    fn each_limit_refuses_and_a_missing_fact_refuses() {
        assert!(check_order(&buy1(700.0), &good_facts(), true).0.is_empty());
        let refuses = |f: Facts, req: OrderReq, word: &str| {
            let (no, _) = check_order(&req, &f, true);
            assert!(no.iter().any(|m| m.contains(word)), "{word}: {no:?}");
        };
        // 5% of the capital for one order, 10% at entry (a small capital), gross, liquidity, cash.
        refuses(Facts { capital: Some(10_000.0), ..good_facts() }, buy1(700.0), "one order");
        refuses(Facts { capital: Some(1_000.0), cash: Some(1e6), ..good_facts() }, buy1(700.0), "one position at entry");
        refuses(Facts { gross: Some(100_000.0), ..good_facts() }, buy1(700.0), "gross exposure");
        refuses(Facts { adv: Some(50_000.0), ..good_facts() }, buy1(700.0), "liquidity");
        // A missing fact refuses in a real run.
        refuses(Facts { capital: None, ..good_facts() }, buy1(700.0), "capital is not known");
        refuses(Facts { cash: None, ..good_facts() }, buy1(700.0), "cash is not known");
        refuses(Facts { gross: None, ..good_facts() }, buy1(700.0), "gross exposure is not known");
        refuses(Facts { adv: None, ..good_facts() }, buy1(700.0), "dollar volume is not known");
        refuses(Facts { position_qty: None, ..good_facts() }, buy1(700.0), "position is not known");
        // In a dry run a missing fact is listed, not a refusal.
        let (no, skipped) = check_order(&buy1(700.0), &Facts { capital: None, cash: None, gross: None, adv: None, budget_left: 1, position_qty: Some(0), ..Facts::default() }, false);
        assert!(no.is_empty() && skipped.len() >= 3, "{no:?} {skipped:?}");
        // The size cap of step A.
        refuses(good_facts(), OrderReq { qty: 2, ..buy1(700.0) }, "outside 1..=1");
        refuses(Facts { position_qty: Some(1), ..good_facts() }, buy1(700.0), "position would be 2");
    }

    #[test]
    fn the_hook_reads_the_account_for_each_order_and_refuses_when_it_cannot() {
        let mut r = rig("failclosed");
        // The account call fails after the start: capital is unknown, the order is refused and no POST is made.
        r.w.borrow_mut().fail_all = true;
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert!(posts(&r).is_empty());
        let l = r.buf.lines();
        let refused = l.iter().find(|v| v["kind"] == "paper_refused").expect("refused");
        assert!(refused["reasons"].to_string().contains("cannot check"), "{refused}");
    }

    // ---- control 4: stops

    #[test]
    fn the_stop_cancels_open_orders_keeps_the_position_and_sends_no_new_order() {
        let mut r = rig("stop");
        r.w.borrow_mut().fill = false; // the entry stays open
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert_eq!(posts(&r).len(), 1);
        // The position is 1 share at the broker (an earlier fill).
        r.w.borrow_mut().pos = 1;
        let before = r.log.borrow().len();
        r.h.stop(Stop::MaxLoss, &inp(T0 + 5000, 99.0, 99.1), None);
        {
            let log = r.log.borrow();
            assert!(log[before..].iter().any(|q| q.method == Method::Delete && q.url.ends_with("/v2/orders/ord-1")), "the open order is cancelled");
            assert!(log[before..].iter().all(|q| q.method != Method::Post), "no new POST after the stop");
        }
        assert_eq!(r.w.borrow().orders[0].status, "canceled");
        assert_eq!(r.w.borrow().pos, 1, "the position is kept");
        // The log line: the position that is kept, no account number.
        let l = r.buf.lines();
        let stop = l.iter().find(|v| v["kind"] == "paper_stop").expect("stop line");
        assert_eq!((stop["reason"].as_str(), stop["position_kept_qty"].as_i64(), stop["cancelled"].as_i64()), (Some("max loss"), Some(1), Some(1)));
        assert!(stop.get("position_kept_avg_entry_price").is_some());
        assert!(!r.buf.text().contains(ACCOUNT_NUMBER));
        // After the stop no decision sends an order, whatever the facts.
        decide(&mut r.h, 1, Side::Sell, 1, &inp(T0 + 6000, 99.0, 99.1));
        assert!(posts(&r).len() == 1 && r.buf.text().contains("the run is stopped"));
    }

    #[test]
    fn a_loss_stop_through_the_engine_keeps_the_broker_position() {
        struct Once;
        impl Strategy for Once {
            fn name(&self) -> &'static str {
                "demo"
            }
            fn params(&self) -> String {
                String::new()
            }
            fn on_input(&mut self, i: &Input, _pos: i64) -> Vec<Intent> {
                if i.read_ms == T0 { vec![Intent { side: Side::Buy, qty: 50, limit: None }] } else { vec![] }
            }
        }
        let r = rig("engine");
        let (w, log, buf) = (r.w.clone(), r.log.clone(), r.buf.clone());
        let cfg = Config { max_pos: 1, long_only: true, max_loss: 0.05, spread_share: 1.0, latency_ms: 0, fee: 0.0, until_secs: 15 * 3600 + 55 * 60, stop_file: r.dir.join("STOP"), feed: "test".into() };
        let mut e = Engine::new(cfg, Once, None).with_hook(Box::new(r.h));
        assert!(e.on_input(&inp(T0, 100.0, 100.1)));
        assert!(e.on_input(&inp(T0 + 1000, 100.0, 100.1))); // the simulator fills 1 share (not 50)
        assert_eq!(w.borrow().pos, 1);
        assert!(!e.on_input(&inp(T0 + 2000, 99.0, 99.1))); // the simulator loss stop
        let s = e.finish();
        assert_eq!(s.stop.label(), "max loss");
        assert_eq!(w.borrow().pos, 1, "the position at the broker is kept: a person closes it");
        let posts: Vec<Req> = log.borrow().iter().filter(|q| q.method == Method::Post).cloned().collect();
        assert_eq!(posts.len(), 1, "the entry only: no close order at the stop");
        assert!(posts[0].body.as_ref().unwrap().contains("\"qty\":\"1\""), "the size 50 is cut to 1");
        // The journal has the pair of the entry: the order, the broker reply and the simulator fill side by side.
        let pair = buf.lines().into_iter().find(|v| v["kind"] == "paper_pair").expect("pair");
        assert_eq!(pair["broker"]["status"], "filled");
        assert_eq!(pair["simulator"]["kind"], "fill");
        assert!(pair["request"]["client_order_id"].is_string() && pair["decision"]["bid"] == 100.0);
        // The stop line has the position that is kept and the simulator close (the simulator flattens its own position for its P&L).
        assert!(buf.lines().iter().any(|v| v["kind"] == "paper_stop" && v["position_kept_qty"] == 1 && v["simulator_close"]["why"] == "max loss"));
    }

    #[test]
    fn the_broker_loss_stop_asks_the_engine_to_stop() {
        let mut r = rig("lossstop");
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert!(r.h.stop_request().is_none());
        r.h.day_loss_limit = 0.5; // the fill was at 100.10
        r.h.poll(&inp(T0 + 1000, 99.7, 99.9)); // mid 99.8: -0.30
        assert!(r.h.stop_request().is_none());
        r.h.poll(&inp(T0 + 2000, 99.4, 99.6)); // mid 99.5: -0.60
        assert!(r.h.stop_request().unwrap().contains("daily loss stop"));
    }

    // ---- control 5: ids

    #[test]
    fn client_order_ids_are_unique_short_journaled_and_never_repeated() {
        let mut r = rig("ids");
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        decide(&mut r.h, 1, Side::Sell, 1, &inp(T0 + 1000, 100.2, 100.3));
        let ids: Vec<String> = r.buf.lines().iter().filter(|v| v["kind"] == "paper_intent").map(|v| v["client_order_id"].as_str().unwrap().to_string()).collect();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
        assert!(ids.iter().all(|i| i.len() <= 128 && i.starts_with("fa-2026-09-21-demo-SPY-")), "{ids:?}");
        // A restart: the journal of the day is the prior text. The counter goes on and the old ids stay in the set.
        let mut r2 = rig_with("ids2", &r.buf.text(), 60, true);
        decide(&mut r2.h, 0, Side::Buy, 1, &inp(T0 + 60_000, 100.0, 100.1));
        let new_id = r2.buf.lines().iter().find(|v| v["kind"] == "paper_intent").unwrap()["client_order_id"].as_str().unwrap().to_string();
        assert!(!ids.contains(&new_id), "{new_id}");
        // If the counter were reset, the id would be in the journal already: refused, not sent.
        let mut r3 = rig_with("ids3", &r.buf.text(), 60, true);
        r3.h.counter = 0;
        decide(&mut r3.h, 0, Side::Buy, 1, &inp(T0 + 60_000, 100.0, 100.1));
        assert!(posts(&r3).is_empty());
        assert!(r3.buf.lines().iter().any(|v| v["kind"] == "paper_refused" && v["reasons"].to_string().contains("in the journal already")));
    }

    #[test]
    fn a_failed_send_reads_the_order_by_its_client_id_and_never_retries() {
        let mut r = rig("timeout");
        r.w.borrow_mut().post_override = Some(Err("timeout".into()));
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert_eq!(posts(&r).len(), 1);
        assert!(r.log.borrow().iter().any(|q| q.url.contains("orders:by_client_order_id?client_order_id=fa-2026-09-21-demo-SPY-0001")));
    }

    #[test]
    fn three_api_errors_in_a_row_end_the_run() {
        let mut r = rig("errors");
        r.w.borrow_mut().fail_all = true;
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        decide(&mut r.h, 1, Side::Buy, 1, &inp(T0 + 1000, 100.0, 100.1)); // each order reads the account and the positions: two errors each
        assert!(r.h.stop_request().unwrap().contains("3 API errors in a row"));
    }

    // ---- control 6: caps

    #[test]
    fn orders_are_day_limit_orders_with_the_quote_as_the_limit() {
        let mut r = rig("shape");
        decide(&mut r.h, 0, Side::Buy, 50, &inp(T0, 100.0, 100.1));
        let b: Value = serde_json::from_str(posts(&r)[0].body.as_ref().unwrap()).unwrap();
        assert_eq!((b["type"].as_str(), b["time_in_force"].as_str(), b["qty"].as_str(), b["limit_price"].as_str(), b["side"].as_str(), b["symbol"].as_str()), (Some("limit"), Some("day"), Some("1"), Some("100.10"), Some("buy"), Some("SPY")));
        assert!(b.get("order_class").is_none() && b.get("extended_hours").is_none() && b.get("stop_price").is_none());
        // The sell goes at the bid.
        decide(&mut r.h, 1, Side::Sell, 1, &inp(T0 + 5000, 100.2, 100.3));
        let s: Value = serde_json::from_str(posts(&r)[1].body.as_ref().unwrap()).unwrap();
        assert_eq!((s["side"].as_str(), s["limit_price"].as_str()), (Some("sell"), Some("100.20")));
    }

    #[test]
    fn no_option_and_no_other_symbol() {
        for s in ["SPY261218C00600000", "SPY 261218C00600000", "SPY251219P00600000", "spy", "QQQ", "SPY1", ""] {
            let (no, _) = check_order(&OrderReq { symbol: s.into(), ..buy1(700.0) }, &good_facts(), true);
            assert!(!no.is_empty(), "'{s}' was not refused");
        }
        assert!(is_stock_symbol("SPY") && is_stock_symbol("BRK.B") && !is_stock_symbol("SPY261218C00600000"));
        let a = |s: &str| crate::trading::algo::cli::parse_opts(&s.split_whitespace().map(String::from).collect::<Vec<_>>());
        assert!(a("--strategy demo --broker alpaca-paper --symbol SPY261218C00600000").is_err());
        assert!(a("--strategy demo --broker alpaca-paper --symbol QQQ").is_err());
        assert!(a("--strategy demo --broker alpaca-paper --symbol SPY").is_ok());
        assert!(a("--strategy demo --broker alpaca-live").is_err());
        assert!(a("--strategy demo --send").is_err(), "--send needs the broker");
    }

    #[test]
    fn no_margin_the_order_value_must_be_inside_the_cash() {
        let (no, _) = check_order(&buy1(700.0), &Facts { cash: Some(699.99), ..good_facts() }, true);
        assert!(no.iter().any(|m| m.contains("no margin")), "{no:?}");
        assert!(check_order(&buy1(700.0), &Facts { cash: Some(700.0), ..good_facts() }, true).0.is_empty());
        let mut r = rig("cash");
        r.w.borrow_mut().cash = 50.0;
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert!(posts(&r).is_empty());
    }

    #[test]
    fn at_most_six_orders_a_minute_and_ten_a_day_and_the_budget() {
        let f = |t: usize, m: usize| Facts { orders_today: t, orders_last_min: m, ..good_facts() };
        assert!(check_order(&buy1(700.0), &f(9, 5), true).0.is_empty());
        assert!(check_order(&buy1(700.0), &f(10, 0), true).0.iter().any(|m| m.contains("orders today")));
        assert!(check_order(&buy1(700.0), &f(0, 6), true).0.iter().any(|m| m.contains("last minute")));
        let sell = OrderReq { side: Side::Sell, ..buy1(700.0) };
        let held = Facts { position_qty: Some(1), ..f(10, 6) };
        assert!(check_order(&sell, &held, true).0.len() == 2, "no exception for any order");
        // Through the hook: ten real orders in the journal of the day.
        let prior: String = (1..=10).map(|n| format!("{{\"kind\":\"paper_intent\",\"dry_run\":false,\"client_order_id\":\"old-{n}\",\"t_ms\":{}}}\n", T0 - 3_600_000)).collect();
        let mut r = rig_with("daycap", &prior, 60, true);
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert!(posts(&r).is_empty());
        assert!(r.buf.text().contains("orders today"));
        // Six orders in the last minute from the journal.
        let prior: String = (1..=6).map(|n| format!("{{\"kind\":\"paper_intent\",\"dry_run\":false,\"client_order_id\":\"old-{n}\",\"t_ms\":{}}}\n", T0 - 10_000)).collect();
        let mut r = rig_with("mincap", &prior, 60, true);
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert!(posts(&r).is_empty() && r.buf.text().contains("last minute"));
        // The request budget: the shared budget of the minute.
        let mut b = Budget::new(2);
        assert!(b.take(T0) && b.take(T0 + 1) && !b.take(T0 + 2) && b.take(T0 + 61_000));
        let mut r = rig_with("budget", "", 3, true); // the start used 3 requests
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0 + 1000, 100.0, 100.1));
        assert!(posts(&r).is_empty());
    }

    // ---- control 7: journal

    #[test]
    fn the_order_journal_is_under_the_ignored_folder_and_each_line_has_the_time() {
        let p = journal_path(Path::new("/repo/data/processed/algo"), "2026-09-21", "demo");
        assert_eq!(p, Path::new("/repo/data/processed/algo/2026-09-21/demo-orders.jsonl"));
        let gi = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.gitignore")).unwrap();
        assert!(gi.lines().any(|l| l.trim() == "data/processed/"), "data/processed/ is not ignored");
        let mut r = rig("journal");
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        r.h.finish();
        let l = r.buf.lines();
        assert!(l.iter().all(|v| v["t_ms"].is_i64()));
        let pos = |k: &str| l.iter().position(|v| v["kind"] == k).unwrap();
        assert!(pos("paper_intent") < pos("paper_order"), "the intent is written before the call");
        assert_eq!(l.iter().find(|v| v["kind"] == "paper_order").unwrap()["broker"]["filled_avg_price"], "100.10");
    }

    // ---- control 8: long only

    #[test]
    fn a_sell_that_makes_the_position_negative_is_refused() {
        let sell = OrderReq { side: Side::Sell, ..buy1(700.0) };
        let (no, _) = check_order(&sell, &Facts { position_qty: Some(0), ..good_facts() }, true);
        assert!(no.iter().any(|m| m.contains("negative")), "{no:?}");
        assert!(check_order(&sell, &Facts { position_qty: Some(1), open_sell_qty: 1, ..good_facts() }, true).0.iter().any(|m| m.contains("negative")));
        assert!(check_order(&sell, &Facts { position_qty: Some(1), ..good_facts() }, true).0.is_empty());
        let mut r = rig("short");
        decide(&mut r.h, 0, Side::Sell, 50, &inp(T0, 100.0, 100.1));
        assert!(posts(&r).is_empty());
        assert!(r.buf.text().contains("negative"));
    }

    // ---- the clock gate

    #[test]
    fn the_market_clock_gate_open_closed_and_failed() {
        let mk = |open: bool| {
            let r = rig_with("clock", "", 60, false);
            r.w.borrow_mut().clock_open = open;
            (Mock { log: r.log.clone(), w: r.w.clone() }, r)
        };
        let (mut t, r) = mk(true);
        assert_eq!(market_gate(&mut t).unwrap(), None);
        assert_eq!(r.log.borrow().len(), 1);
        assert_eq!(r.log.borrow()[0].url, "https://paper-api.alpaca.markets/v2/clock");
        assert_eq!(r.log.borrow()[0].method, Method::Get);
        let (mut t, _r) = mk(false);
        assert_eq!(market_gate(&mut t).unwrap().unwrap(), "market closed (Alpaca clock): next open 2026-09-22T09:30:00-04:00");
        // A failing call, an HTTP error and a reply with no is_open all refuse (fail closed).
        let (mut t, r) = mk(true);
        r.w.borrow_mut().fail_all = true;
        assert!(market_gate(&mut t).unwrap_err().contains("clock call failed"));
        struct Fixed(u16, &'static str);
        impl Transport for Fixed {
            fn send(&mut self, _m: Method, _u: &str, _b: Option<&str>) -> Result<Reply, String> {
                Ok(Reply { status: self.0, body: self.1.into() })
            }
        }
        assert!(market_gate(&mut Fixed(401, "{}")).unwrap_err().contains("HTTP 401"));
        assert!(market_gate(&mut Fixed(200, "{\"next_open\": \"x\"}")).unwrap_err().contains("no is_open"));
        assert!(market_gate(&mut Fixed(200, "not json")).is_err());
    }

    // ---- round 2: the replies of the server are checked

    fn good_reply() -> Value {
        json!({"id": "x1", "client_order_id": "fa-2026-09-21-demo-SPY-0001", "symbol": "SPY", "side": "buy", "qty": "1", "type": "limit", "time_in_force": "day", "limit_price": "100.10", "status": "accepted", "filled_qty": "0", "filled_avg_price": null})
    }

    fn send_with_reply(name: &str, reply: Value) -> Rig {
        let mut r = rig(name);
        r.w.borrow_mut().post_override = Some(Ok((200, reply.to_string())));
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        r
    }

    #[test]
    fn a_reply_that_does_not_match_the_request_marks_the_order_failed_and_stops_the_run() {
        let r = send_with_reply("reply-good", good_reply());
        assert!(r.h.stop_request().is_none(), "the good reply is accepted");
        assert_eq!(r.h.tracked[0].broker_id.as_deref(), Some("x1"));
        for (field, bad) in [("client_order_id", json!("other")), ("symbol", json!("QQQ")), ("side", json!("sell")), ("qty", json!("2")), ("limit_price", json!("100.20")), ("type", json!("market")), ("time_in_force", json!("gtc")), ("id", json!(""))] {
            let mut v = good_reply();
            v[field] = bad;
            // A reply with no id is read again by the client id: the broker has no such order, so it fails too.
            let r = send_with_reply(&format!("reply-{field}"), v);
            let why = r.h.stop_request().unwrap_or_else(|| panic!("{field}: the run was not stopped"));
            assert!(why.contains("does not match") || why.contains("no order id"), "{field}: {why}");
            assert_eq!(r.h.tracked[0].status, "refused", "{field}");
            assert!(r.buf.lines().iter().any(|v| v["kind"] == "paper_error" && v["stage"] == "reply check"), "{field}");
        }
    }

    #[test]
    fn a_filled_quantity_above_the_order_or_negative_or_not_whole_is_refused() {
        let filled = |fq: Value, avg: Value| {
            let mut v = good_reply();
            v["filled_qty"] = fq;
            v["filled_avg_price"] = avg;
            v["status"] = json!("filled");
            v
        };
        let ok = send_with_reply("fq-ok", filled(json!("1"), json!("100.10")));
        assert!(ok.h.stop_request().is_none());
        assert_eq!(ok.h.broker_pos, 1);
        for (name, v) in [("above", filled(json!("2"), json!("100.10"))), ("fraction", filled(json!("0.5"), json!("100.10"))), ("negative", filled(json!("-1"), json!("100.10"))), ("text", filled(json!("many"), json!("100.10"))), ("no price", filled(json!("1"), Value::Null)), ("zero price", filled(json!("1"), json!("0")))] {
            let r = send_with_reply(&format!("fq-{name}"), v);
            assert!(r.h.stop_request().is_some(), "{name}: not refused");
            assert_eq!(r.h.broker_pos, 0, "{name}: the position must not move");
        }
    }

    #[test]
    fn a_reply_with_no_id_is_read_by_the_client_id_so_no_open_order_is_untracked() {
        let mut r = rig("noid");
        r.w.borrow_mut().orders.push(Ord1 { id: "pre-1".into(), cid: "fa-2026-09-21-demo-SPY-0001".into(), side: "buy".into(), qty: 1, limit: "100.10".into(), status: "accepted".into() });
        let mut v = good_reply();
        v.as_object_mut().unwrap().remove("id");
        r.w.borrow_mut().post_override = Some(Ok((200, v.to_string())));
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert!(r.h.stop_request().is_none());
        assert_eq!(r.h.tracked[0].broker_id.as_deref(), Some("pre-1"), "the open order is tracked");
        assert!(r.log.borrow().iter().any(|q| q.url.contains("orders:by_client_order_id")));
    }

    #[test]
    fn a_status_read_for_another_order_or_with_a_bad_fill_stops_the_run() {
        for (name, body) in [("other id", {
            let mut v = good_reply();
            v["id"] = json!("ord-9");
            v
        }), ("other client id", {
            let mut v = good_reply();
            v["id"] = json!("ord-1");
            v["client_order_id"] = json!("zzz");
            v
        }), ("fill above", {
            let mut v = good_reply();
            v["id"] = json!("ord-1");
            v["filled_qty"] = json!("5");
            v["filled_avg_price"] = json!("100.1");
            v
        })] {
            let mut r = rig(&format!("status-{name}"));
            r.w.borrow_mut().fill = false;
            decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
            assert!(r.h.stop_request().is_none());
            r.w.borrow_mut().get_override = Some(body.to_string());
            r.h.poll(&inp(T0 + 3000, 100.0, 100.1));
            assert!(r.h.stop_request().is_some(), "{name}");
            assert_eq!(r.h.broker_pos, 0, "{name}");
        }
    }

    #[test]
    fn the_cancels_at_a_stop_skip_the_budget_and_the_error_count() {
        // The start used the whole budget of 3 requests: a normal call would be refused.
        let mut r = rig_with("critical", "", 3, true);
        r.w.borrow_mut().orders.push(Ord1 { id: "o9".into(), cid: "x".into(), side: "buy".into(), qty: 1, limit: "1.00".into(), status: "accepted".into() });
        r.h.stop(Stop::Until, &inp(T0 + 1000, 100.0, 100.1), None);
        assert!(r.log.borrow().iter().any(|q| q.method == Method::Delete && q.url.ends_with("/v2/orders/o9")), "the cancel was sent");
        assert_eq!(r.w.borrow().orders[0].status, "canceled");
        // Failing calls at the stop do not count as API errors.
        let mut r = rig("critical-errors");
        r.w.borrow_mut().fail_all = true;
        r.h.stop(Stop::Until, &inp(T0 + 1000, 100.0, 100.1), None);
        assert_eq!(r.h.api_errors, 0);
        assert!(r.h.stop_request().is_none());
    }

    #[test]
    fn an_exit_with_no_stop_cancels_the_open_orders() {
        let mut r = rig("abnormal");
        r.w.borrow_mut().fill = false;
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        let (h, log, w, buf) = (r.h, r.log.clone(), r.w.clone(), r.buf.clone());
        drop(h); // an Err path or an unwind: no stop() was called
        assert!(log.borrow().iter().any(|q| q.method == Method::Delete && q.url.ends_with("/v2/orders/ord-1")));
        assert_eq!(w.borrow().orders[0].status, "canceled");
        assert!(buf.lines().iter().any(|v| v["kind"] == "paper_stop" && v["reason"] == "abnormal exit"));
        // After a normal stop, the drop sends nothing more.
        let mut r = rig("normal-drop");
        r.h.stop(Stop::Until, &inp(T0 + 1000, 100.0, 100.1), None);
        let n = r.log.borrow().len();
        let log = r.log.clone();
        drop(r.h);
        assert_eq!(log.borrow().len(), n);
    }

    #[test]
    fn a_restart_keeps_the_day_pnl_from_the_journal() {
        let line = |kind: &str, cid: &str, side: &str, avg: &str| json!({"kind": kind, "client_order_id": cid, "broker": {"side": side, "filled_qty": "1", "filled_avg_price": avg}}).to_string();
        // A buy at 100 (seen twice: the later line replaces the first) and a sell at 98.5: -1.5 USD.
        let prior = [line("paper_order", "a", "buy", "100"), line("paper_status", "a", "buy", "100"), line("paper_status", "b", "sell", "98.5")].join("\n");
        let mut r = rig_with("restart-pnl", &prior, 60, true);
        assert!((r.h.day_pnl() - -1.5).abs() < 1e-9, "{}", r.h.day_pnl());
        r.h.day_loss_limit = 1.0;
        r.h.poll(&inp(T0, 100.0, 100.1));
        assert!(r.h.stop_request().unwrap().contains("daily loss stop"));
    }

    #[test]
    fn one_failed_send_counts_once_in_the_error_count() {
        let mut r = rig("count-once");
        r.w.borrow_mut().post_override = Some(Err("timeout".into()));
        decide(&mut r.h, 0, Side::Buy, 1, &inp(T0, 100.0, 100.1));
        assert_eq!(r.h.api_errors, 1, "the POST counts once; the read by client id (404) does not count");
    }

    // ---- round 2: where an order run may start

    fn host_env(name: &str, file_text: Option<&str>, vars: &[&str]) -> (OrderEnv, PathBuf) {
        let dir = tmp(name);
        let file = dir.join("home").join(".config").join("fa").join("order-host");
        if let Some(t) = file_text {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, t).unwrap();
        }
        (OrderEnv { vars: vars.iter().map(|s| s.to_string()).collect(), host_file: Some(file), hostname: "laptop-7".into(), windows: false }, dir.join("repo"))
    }

    #[test]
    fn send_is_refused_in_a_claude_code_or_cloud_session() {
        for var in ["CLAUDECODE", "CCR_SESSION_PROFILE", "CCR_X"] {
            // The host file is right: the session check refuses anyway.
            let (env, repo) = host_env(&format!("cloud-{var}"), Some("laptop-7\n"), &["PATH", var]);
            let e = check_order_host(&env, &repo).unwrap_err();
            assert_eq!(e, "orders are sent only from the laptop or the order host, not from a Claude Code or cloud session");
        }
        let (env, repo) = host_env("not-cloud", Some("laptop-7\n"), &["PATH", "CLAUDE_CODE_X", "CCRX"]);
        assert!(check_order_host(&env, &repo).is_ok(), "only CLAUDECODE and CCR_* count");
    }

    #[test]
    fn send_needs_the_order_host_file_with_this_host_name() {
        let (env, repo) = host_env("hostfile-missing", None, &["PATH"]);
        let e = check_order_host(&env, &repo).unwrap_err();
        assert!(e.contains("is missing") && e.contains("mkdir -p ~/.config/fa && echo laptop-7 > ~/.config/fa/order-host"), "{e}");
        let (env, repo) = host_env("hostfile-other", Some("another-box\n"), &["PATH"]);
        assert!(check_order_host(&env, &repo).unwrap_err().contains("not the host name of this machine"));
        let (env, repo) = host_env("hostfile-empty", Some(""), &["PATH"]);
        assert!(check_order_host(&env, &repo).is_err());
        let (env, repo) = host_env("hostfile-ok", Some("laptop-7\nsecond line is ignored\n"), &["PATH"]);
        assert!(check_order_host(&env, &repo).is_ok());
        // The file must be outside the repo.
        let (mut env, _) = host_env("hostfile-inside", Some("laptop-7\n"), &["PATH"]);
        let repo = env.host_file.clone().unwrap().parent().unwrap().to_path_buf();
        assert!(check_order_host(&env, &repo).unwrap_err().contains("inside the repo"));
        env.host_file = None;
        assert!(check_order_host(&env, Path::new("/x")).is_err());
        // The Windows command.
        let (mut env, _) = host_env("hostfile-win", None, &[]);
        env.windows = true;
        assert!(order_host_command(&env).contains("%APPDATA%"));
    }

    #[test]
    fn without_the_paper_keys_the_start_gate_uses_the_new_york_hours_and_no_request() {
        assert_eq!(ny_hours_gate(T0 / 1000), None, "Monday 10:13 New York");
        let sat = ny_hours_gate(T0 / 1000 + 5 * 86_400).unwrap();
        assert!(sat.contains("holidays are not known without the clock") && sat.contains("2026-09-26"), "{sat}");
        assert!(ny_hours_gate(T0 / 1000 - 44 * 60 - 20).is_some(), "09:29:00 is before the open");
        assert!(ny_hours_gate(T0 / 1000 - 43 * 60 - 20).is_none(), "09:30:00 is open");
        assert!(ny_hours_gate(T0 / 1000 + 5 * 3600 + 46 * 60 + 39).is_none(), "15:59:59 is open");
        assert!(ny_hours_gate(T0 / 1000 + 5 * 3600 + 46 * 60 + 40).is_some(), "16:00:00 is closed");
    }

    // ---- small parts

    #[test]
    fn parsers_and_helpers_fixed_answers() {
        let acct = json!({"status": "ACTIVE", "trading_blocked": false, "account_blocked": false, "currency": "USD", "equity": "100000.5", "cash": "90000"});
        assert_eq!(parse_paper_account(&acct).unwrap(), Account { equity: 100000.5, cash: 90000.0 });
        assert!(parse_paper_account(&json!({"status": "ACTIVE", "equity": "1", "cash": "1"})).is_err(), "no block flags");
        assert!(parse_paper_account(&json!({"status": "ACTIVE", "trading_blocked": true, "account_blocked": false, "equity": "1", "cash": "1"})).is_err());
        let pos = json!([{"symbol": "SPY", "qty": "2", "market_value": "1400"}, {"symbol": "QQQ", "qty": "-1", "market_value": "-500"}]);
        assert_eq!(parse_positions(&pos, "SPY").unwrap(), (2, 1900.0));
        assert!(parse_positions(&json!({"x": 1}), "SPY").is_err());
        let bars: Vec<(f64, f64)> = (0..25).map(|i| (100.0 + i as f64, 1000.0)).collect();
        // The last 20 closes are 105..=124: mean 114.5, times 1000.
        assert!((adv_dollars(&bars).unwrap() - 114_500.0).abs() < 1e-6);
        assert!(adv_dollars(&bars[..19]).is_err());
        assert!(!host_name().is_empty());
        assert_eq!(cents(100.104), 100.1);
    }
}
