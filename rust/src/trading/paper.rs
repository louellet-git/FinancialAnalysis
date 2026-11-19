//! Forward paper trading: `fa trade paper`. It never sends an order to a broker.
//!
//! Each bot has a paper account that starts on its first step (the start date is saved). On each
//! step the program refreshes the data (the cache merges, so the intraday history keeps growing),
//! runs the bot again from a warm-up window with entries blocked before the paper start, and
//! shows:
//!
//! - the paper account: equity, open positions, trades since the start. The fills are model fills
//!   on data that the bot did not see when it was designed, so this is a forward test;
//! - the new signals that the bot queued on the last bar, priced on the LIVE option chain (real bid
//!   and ask, delayed: Cboe delayed quotes; model price for a leg that the chain does not list).
//!
//! State file of each bot: `data/processed/trading/paper/<bot>.json` (not in git). The format is
//! the one of the earlier Python code: `paper_start`, and `snapshots` (the last 500).

use super::bots;
use super::calendar::{civil_from_days, days_from_civil, iso, ny_offset, offset_text, parse_iso, Date};
use super::chain::{Chain, Chains};
use super::data::{load_market, Interval, MarketData, Store};
use super::engine::{run_from, BacktestResult, RiskConfig, Time};
use super::fetch;
use super::pricing::{Quote, SyntheticPricer, VolSnapshot};
use super::strategy::Strategy;
use super::structures::MULTIPLIER;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa trade paper - forward paper trading of the option bots (no orders to a broker, ever)

USAGE:
  fa trade paper [BOT]... [--start D] [--no-refresh] [--no-live] [--equity N] [--max-risk N] [--risk F]

  BOT           the bots to step (default: all bots)
  --start D     paper start date for a bot with no saved state (default: today, New York)
  --no-refresh  use the cached data only (no download of bars)
  --no-live     price the signals with the model, not with the live option chain
  --equity N    the starting equity (default 2000)
  --max-risk N  the maximum loss of one trade is N dollars; it replaces --risk
  --risk F      the maximum loss of one trade as a fraction of equity (default 0.03)

Run it on a schedule during the market hours (every 5 minutes for the 5-minute bots, every hour for
the hourly bots, after the close for the daily bots). The state of each bot is in
data/processed/trading/paper/<bot>.json. Bars: Yahoo chart endpoint (`fa trade fetch`). Live
option chain: Cboe delayed quotes (about 15 minutes late; before the open the quotes can be stale).";

/// The clock of one run: the New York date and minute of the day, and the UTC second.
#[derive(Debug, Clone, Copy)]
pub struct Now {
    pub date: Date,
    pub minute: i32,
    pub utc: i64,
}

impl Now {
    pub fn from_utc(utc: i64) -> Now {
        let local = utc + ny_offset(utc);
        Now { date: local.div_euclid(86_400) as Date, minute: (local.rem_euclid(86_400) / 60) as i32, utc }
    }

    pub fn system() -> Now {
        Now::from_utc(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0))
    }

    /// `2026-11-19T09:30:12-05:00`
    pub fn iso(&self) -> String {
        let local = self.utc + ny_offset(self.utc);
        let s = local.rem_euclid(86_400);
        format!("{}T{:02}:{:02}:{:02}{}", iso(self.date), s / 3600, s % 3600 / 60, s % 60, offset_text(ny_offset(self.utc)))
    }
}

#[derive(Debug, Default)]
struct Args {
    bots: Vec<String>,
    start: Option<String>,
    no_refresh: bool,
    no_live: bool,
    equity: Option<f64>,
    max_risk: Option<f64>,
    risk: Option<f64>,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = args.iter();
    let num = |name: &str, v: Option<&String>| -> Result<f64, String> {
        let v = v.ok_or_else(|| format!("{name} needs a value"))?;
        v.parse::<f64>().map_err(|_| format!("bad number for {name}: '{v}'"))
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--start" => a.start = Some(it.next().ok_or("--start needs a date")?.clone()),
            "--no-refresh" => a.no_refresh = true,
            "--no-live" => a.no_live = true,
            "--equity" => a.equity = Some(num(arg, it.next())?),
            "--max-risk" => a.max_risk = Some(num(arg, it.next())?),
            "--risk" => a.risk = Some(num(arg, it.next())?),
            s if s.starts_with('-') => return Err(format!("unknown option '{s}'")),
            bot => a.bots.push(bot.to_string()),
        }
    }
    Ok(a)
}

pub fn state_dir(repo: &Path) -> PathBuf {
    repo.join("data/processed/trading/paper")
}

/// `fa trade paper ...`. Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> Result<i32, String> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let a = parse_args(args)?;
    let known = bots::names(true);
    let names: Vec<&str> = if a.bots.is_empty() { bots::names(false) } else { a.bots.iter().map(String::as_str).collect() };
    if let Some(bad) = names.iter().find(|n| !known.contains(n)) {
        eprintln!("unknown bot '{bad}'\navailable: {known:?}");
        return Ok(2);
    }
    let equity = a.equity.unwrap_or(2000.0);
    let risk = match a.max_risk {
        Some(m) if m != 0.0 => RiskConfig::with_max_risk(m, equity, RiskConfig::default().commission_per_contract),
        _ => RiskConfig { starting_equity: equity, risk_per_trade: a.risk.unwrap_or(0.03), ..RiskConfig::default() },
    };
    let start = match &a.start {
        Some(s) => Some(parse_iso(s).ok_or_else(|| format!("bad date for --start: '{s}' (use YYYY-MM-DD)"))?),
        None => None,
    };
    let data_dir = Store::from_env(repo).dir().to_path_buf();
    let dir = state_dir(repo);
    let mut chains = Chains::default();
    let mut refreshed: HashSet<(String, &'static str)> = HashSet::new();
    let mut rc = 0;
    for name in names {
        let result = (|| -> Result<Report, String> {
            let mut bot = bots::make(name).ok_or("unknown bot")?;
            if !a.no_refresh {
                let todo: Vec<_> = fetch::files_needed(bot.symbols(), bot.interval()).into_iter().filter(|(s, iv)| refreshed.insert((s.clone(), iv.label()))).collect();
                for o in fetch::refresh(&data_dir, &todo) {
                    if let Err(e) = o.result {
                        eprintln!("fa: refresh of {} {} failed ({e}); using the cache", o.symbol, o.interval.label());
                    }
                }
            }
            let store = Store::new(data_dir.clone());
            step(bot.as_mut(), &store, &dir, &risk, start, if a.no_live { None } else { Some(&mut chains) }, Now::system())
        })();
        match result {
            Ok(r) => println!("{}", r.text),
            Err(e) => {
                eprintln!("== {name}: FAILED {e}");
                rc = 1;
            }
        }
    }
    Ok(rc)
}

/// The days of the warm-up window before the paper start.
fn warmup_start(start: Date, interval: Interval) -> Date {
    match interval {
        // DateOffset(years=2): the same month and day, or the last day of a shorter month.
        Interval::Daily => {
            let (y, m, d) = civil_from_days(start);
            let first = days_from_civil(y - 2, m, 1);
            let next = if m == 12 { days_from_civil(y - 1, 1, 1) } else { days_from_civil(y - 2, m + 1, 1) };
            first + (d as i32 - 1).min(next - first - 1)
        }
        Interval::Hourly => start - 120,
        Interval::FiveMinute => start - 30,
    }
}

#[derive(Debug)]
pub struct Signal {
    pub structure: String,
    pub entry: f64,
    pub tag: String,
    pub source: String,
    pub max_loss_per_unit: f64,
}

#[derive(Debug)]
pub struct Report {
    pub bot: &'static str,
    pub paper_start: Date,
    pub signals: Vec<Signal>,
    pub text: String,
}

/// `1,234.50`, with a sign when `plus` is set or the number is negative.
fn money(x: f64, decimals: usize, plus: bool) -> String {
    let s = format!("{:.*}", decimals, x.abs());
    let (int, frac) = s.split_once('.').map_or((s.as_str(), None), |(i, f)| (i, Some(f)));
    let mut out = String::new();
    for (k, c) in int.chars().enumerate() {
        if k > 0 && (int.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    let sign = if x < 0.0 && s.chars().any(|c| c.is_ascii_digit() && c != '0') {
        "-"
    } else if plus {
        "+"
    } else {
        ""
    };
    format!("{sign}{out}")
}

fn clock(t: Time) -> String {
    let (_, m, d) = civil_from_days(t.date);
    format!("{m:02}-{d:02} {:02}:{:02}", t.minute / 60, t.minute % 60)
}

fn stamp_text(utc: i64) -> String {
    let off = ny_offset(utc);
    let local = utc + off;
    let s = local.rem_euclid(86_400);
    format!("{} {:02}:{:02}:{:02}{}", iso(local.div_euclid(86_400) as Date), s / 3600, s % 3600 / 60, s % 60, offset_text(off))
}

/// The paper start of a bot: from its state file; the first call saves it.
fn paper_start_for(path: &Path, start: Option<Date>, now: Now) -> Result<Date, String> {
    if path.exists() {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let s = v["paper_start"].as_str().ok_or_else(|| format!("{}: no paper_start", path.display()))?;
        return parse_iso(s).ok_or_else(|| format!("{}: bad paper_start '{s}'", path.display()));
    }
    let date = start.unwrap_or(now.date);
    let off = offset_text(ny_offset(date as i64 * 86_400 + 5 * 3600));
    write_state(path, &json!({"paper_start": format!("{}T00:00:00{off}", iso(date)), "snapshots": []}))?;
    Ok(date)
}

fn write_state(path: &Path, v: &Value) -> Result<(), String> {
    use serde::Serialize;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, serde_json::ser::PrettyFormatter::with_indent(b" "));
    v.serialize(&mut ser).map_err(|e| e.to_string())?;
    std::fs::write(path, buf).map_err(|e| format!("{}: {e}", path.display()))
}

/// Entry price of the signals, per unit and share: long legs at the ask, short legs at the bid.
/// With a chain, a leg that the chain lists is priced from it; the other legs get the model price at
/// `now`. Without a chain, the model price at the time of the last bar close.
fn price_signals(result: &BacktestResult, md: &MarketData, chain_of: &mut dyn FnMut(&str) -> Option<std::sync::Arc<Chain>>, live: bool, now: Now) -> Vec<Signal> {
    let last = md.len() - 1;
    let mut out = Vec::new();
    for (s, tag) in &result.pending {
        let si = md.sym_index(s.symbol);
        let spot = md.bars[si].close[last];
        let vol = VolSnapshot { atm: md.vol[si].atm_close[last], short: md.vol[si].short_close[last], rate: md.rate[last] };
        let chain = if live { chain_of(s.symbol) } else { None };
        let (mut entry, mut live_legs) = (0.0, 0);
        for leg in &s.legs {
            let q: Quote = match chain.as_ref().and_then(|c| c.quote(leg.call, leg.strike, s.expiry)) {
                Some(q) => {
                    live_legs += 1;
                    q
                }
                None if live => SyntheticPricer::quote(leg.call, leg.strike, s.expiry, now.date, now.minute, spot, &vol),
                None => SyntheticPricer::quote(leg.call, leg.strike, s.expiry, md.date[last], md.interval.close_minute(md.minute[last]), spot, &vol),
            };
            entry += leg.qty as f64 * if leg.qty > 0 { q.ask } else { q.bid };
        }
        let source = match (&chain, live_legs) {
            (Some(c), n) if n == s.legs.len() => format!("live chain, Cboe delayed: {}", c.age()),
            (Some(_), 0) => "synthetic (no leg in the chain)".to_string(),
            (Some(c), n) => format!("mixed: {n} of {} legs from the Cboe chain, {}", s.legs.len(), c.age()),
            (None, _) if live => "synthetic (no chain)".to_string(),
            (None, _) => "synthetic".to_string(),
        };
        out.push(Signal { structure: s.describe(), entry, tag: tag.clone(), source, max_loss_per_unit: s.max_loss(entry) * MULTIPLIER });
    }
    out
}

/// One paper step of one bot.
pub fn step(
    strat: &mut dyn Strategy,
    store: &Store,
    state_dir: &Path,
    risk: &RiskConfig,
    start: Option<Date>,
    chains: Option<&mut Chains>,
    now: Now,
) -> Result<Report, String> {
    let name = strat.name();
    let path = state_dir.join(format!("{name}.json"));
    let paper_start = paper_start_for(&path, start, now)?;
    let mut data_start = warmup_start(paper_start, strat.interval());
    if let Some(s) = strat.start().and_then(parse_iso) {
        data_start = data_start.max(s);
    }
    let md = load_market(store, strat.symbols(), strat.interval(), Some(data_start), None)?;
    if md.is_empty() {
        return Err(format!("no bars for {name} from {}", iso(data_start)));
    }
    let start_time = Time { date: paper_start, minute: 0 };
    let result = run_from(strat, &md, risk.clone(), Some(start_time))?;

    let live = chains.is_some();
    let mut chains = chains;
    let mut warned = HashSet::new();
    let mut chain_of = |symbol: &str| -> Option<std::sync::Arc<Chain>> {
        match chains.as_mut()?.get(symbol) {
            Ok(c) => Some(c),
            Err(e) => {
                if warned.insert(symbol.to_string()) {
                    eprintln!("fa: no live chain for {symbol} ({e}); the signals use the model price");
                }
                None
            }
        }
    };
    let signals = price_signals(&result, &md, &mut chain_of, live, now);

    let last = md.len() - 1;
    let as_of = stamp_text(md.utc[last]);
    let eq = *result.equity.last().expect("at least one bar");
    let start_eq = result.risk.starting_equity;
    let trades: Vec<_> = result.trades.iter().filter(|t| t.entry_time >= start_time).collect();
    let mut lines = vec![
        format!("== {name}  (paper since {}, data through {as_of})", iso(paper_start)),
        format!(
            "   equity ${}  ({}%)  closed trades {}  realized ${}",
            money(eq, 2, false),
            money((eq / start_eq - 1.0) * 100.0, 2, true),
            trades.len(),
            money(trades.iter().map(|t| t.pnl).sum::<f64>(), 2, true)
        ),
    ];
    for p in &result.open_positions {
        lines.push(format!(
            "   OPEN  {} x{}  entry {}  mark {}  uP&L ${}",
            p.structure.describe(),
            p.units,
            money(p.entry_net, 2, true),
            money(p.mark, 2, true),
            money(p.pnl_at(p.mark), 2, true)
        ));
    }
    for t in &trades[trades.len().saturating_sub(5)..] {
        lines.push(format!("   DONE  {} {} x{} {}→{} ${} ({})", t.symbol, t.structure, t.units, clock(t.entry_time), clock(t.exit_time), money(t.pnl, 2, true), t.exit_reason));
    }
    for s in &signals {
        lines.push(format!("   SIGNAL (next bar) {}  live entry {}/unit (max loss ${}/unit; {})", s.structure, money(s.entry, 2, true), money(s.max_loss_per_unit, 0, false), s.source));
    }
    if !result.rejections.is_empty() {
        let items: Vec<String> = result.rejections.iter().map(|(k, v)| format!("'{k}': {v}")).collect();
        lines.push(format!("   rejections: {{{}}}", items.join(", ")));
    }

    // Save a snapshot (the last 500).
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut state: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let snapshots = state["snapshots"].as_array_mut().ok_or_else(|| format!("{}: no snapshots list", path.display()))?;
    snapshots.push(json!({
        "run_at": now.iso(),
        "as_of": as_of,
        "equity": eq,
        "open": result.open_positions.iter().map(|p| p.structure.describe()).collect::<Vec<_>>(),
        "signals": signals.iter().map(|s| json!({"structure": s.structure, "entry": s.entry, "tag": s.tag, "source": s.source, "max_loss_per_unit": s.max_loss_per_unit})).collect::<Vec<_>>(),
    }));
    let drop = snapshots.len().saturating_sub(500);
    snapshots.drain(..drop);
    write_state(&path, &state)?;

    Ok(Report { bot: name, paper_start, signals, text: lines.join("\n") })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::calendar::busday_offset;
    use crate::trading::chain::tests::fixture as _chain_fixture;
    use crate::trading::engine::Ctx;
    use crate::trading::strategy::ExitRules;
    use crate::trading::structures::vertical;

    /// The strike (long leg) of the test spread: near the money of the fixture, whose last close is 514.80.
    const K: f64 = 515.0;

    /// Queues one call spread (6 sessions out, K and K+1) on the bar with this index.
    struct QueueAt(usize);

    impl Strategy for QueueAt {
        fn name(&self) -> &'static str {
            "t_queue_at"
        }

        fn on_bar(&mut self, ctx: &mut Ctx) -> Result<(), String> {
            if ctx.i == self.0 {
                let s = vertical("SPY", ctx.expiry(6), true, K, K + 1.0)?;
                ctx.open(s, ExitRules::default(), "test", None);
            }
            Ok(())
        }
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fa-paper-{tag}-{}", std::process::id()));
        std::fs::remove_dir_all(&d).ok();
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A chain with the two strikes of the test bot at the expiry that the bot asks for.
    fn chain_for(expiry: Date, long_ask: f64, short_bid: f64) -> Chain {
        let name = |k: u32| format!("SPY{:02}{:02}{:02}C{:08}", civil_from_days(expiry).0 % 100, civil_from_days(expiry).1, civil_from_days(expiry).2, k);
        let text = format!(
            r#"{{"timestamp":"2026-11-19 15:10:00","data":{{"symbol":"SPY","current_price":735.5,"options":[
            {{"option":"{}","bid":{},"ask":{long_ask}}},{{"option":"{}","bid":{short_bid},"ask":{}}}]}}}}"#,
            name(K as u32 * 1000),
            long_ask - 0.1,
            name((K as u32 + 1) * 1000),
            short_bid + 0.1
        );
        Chain::parse(text.as_bytes(), "SPY").unwrap()
    }

    #[test]
    fn money_format_has_python_commas_and_signs() {
        assert_eq!(money(1234567.891, 2, false), "1,234,567.89");
        assert_eq!(money(-1234.5, 2, true), "-1,234.50");
        assert_eq!(money(12.0, 2, true), "+12.00");
        assert_eq!(money(999.5, 0, false), "1,000");
        assert_eq!(money(0.0, 2, true), "+0.00");
    }

    #[test]
    fn warmup_windows() {
        let d = days_from_civil(2026, 11, 19);
        assert_eq!(iso(warmup_start(d, Interval::Daily)), "2024-11-19");
        assert_eq!(iso(warmup_start(d, Interval::Hourly)), "2026-07-22");
        assert_eq!(iso(warmup_start(d, Interval::FiveMinute)), "2026-10-20");
        assert_eq!(iso(warmup_start(days_from_civil(2028, 2, 29), Interval::Daily)), "2026-02-28");
    }

    #[test]
    fn clock_is_new_york_time() {
        // 2026-11-19 14:30:00 UTC is 09:30 EST. 2026-07-01 13:30:00 UTC is 09:30 EDT.
        let n = Now::from_utc(days_from_civil(2026, 11, 19) as i64 * 86_400 + 14 * 3600 + 30 * 60);
        assert_eq!((iso(n.date), n.minute, n.iso()), ("2026-11-19".to_string(), 570, "2026-11-19T09:30:00-05:00".to_string()));
        let s = Now::from_utc(days_from_civil(2026, 7, 1) as i64 * 86_400 + 13 * 3600 + 30 * 60);
        assert_eq!((s.minute, s.iso()), (570, "2026-07-01T09:30:00-04:00".to_string()));
        // Before 05:00 UTC it is still the evening of the day before.
        assert_eq!(iso(Now::from_utc(days_from_civil(2026, 11, 19) as i64 * 86_400 + 3600).date), "2026-11-18");
    }

    #[test]
    fn a_step_with_a_fixture_chain() {
        let dir = temp("step");
        crate::trading::cli::tests::write_fixture(&dir);
        let store = Store::new(dir.clone());
        let state = dir.join("paper");
        let n = 150;
        let first = days_from_civil(2024, 3, 11);
        let last_date = busday_offset(first, n as i32 - 1, true);
        let expiry = busday_offset(last_date, 6, true);
        let now = Now::from_utc(last_date as i64 * 86_400 + 21 * 3600);
        let start_date = busday_offset(last_date, -20, false);
        let start = Some(start_date);
        let risk = RiskConfig::default();

        // Live chain: the entry is the ask of the long leg minus the bid of the short leg.
        let mut chains = Chains::default();
        chains.insert(chain_for(expiry, 3.40, 2.60));
        let r = step(&mut QueueAt(n - 1), &store, &state, &risk, start, Some(&mut chains), now).unwrap();
        assert_eq!(r.signals.len(), 1);
        let s = &r.signals[0];
        assert!((s.entry - (3.40 - 2.60)).abs() < 1e-12, "{}", s.entry);
        assert!((s.max_loss_per_unit - 0.80 * 100.0).abs() < 1e-9);
        assert!(s.source.starts_with("live chain, Cboe delayed: file 2026-11-19 15:10:00 UTC, underlying last trade"), "{}", s.source);
        assert!(r.text.contains("SIGNAL (next bar) SPY") && r.text.contains("live entry +0.80/unit (max loss $80/unit;"), "{}", r.text);
        assert!(r.text.contains("equity $2,000.00  (+0.00%)  closed trades 0  realized $+0.00"), "{}", r.text);

        // The state file: the start date is saved, one snapshot.
        let v: Value = serde_json::from_str(&std::fs::read_to_string(state.join("t_queue_at.json")).unwrap()).unwrap();
        assert_eq!(v["paper_start"].as_str().unwrap()[..10], iso(start_date));
        assert_eq!(v["snapshots"].as_array().unwrap().len(), 1);
        assert_eq!(v["snapshots"][0]["signals"][0]["tag"], "test");
        assert_eq!(v["snapshots"][0]["as_of"].as_str().unwrap()[..10], iso(last_date));

        // A second step with no live data: the model price, the same start date, two snapshots.
        let r2 = step(&mut QueueAt(n - 1), &store, &state, &risk, Some(first), None, now).unwrap();
        assert_eq!(r2.paper_start, start_date);
        assert_eq!(r2.signals[0].source, "synthetic");
        assert!(r2.signals[0].entry > 0.0 && (r2.signals[0].entry - 0.80).abs() > 1e-6);
        let v: Value = serde_json::from_str(&std::fs::read_to_string(state.join("t_queue_at.json")).unwrap()).unwrap();
        assert_eq!(v["snapshots"].as_array().unwrap().len(), 2);

        // A chain that does not list the legs: the signal falls back to the model price and says so.
        let mut other = Chains::default();
        other.insert(Chain::parse(_chain_fixture().as_bytes(), "SPY").unwrap());
        let afternoon = Now::from_utc(last_date as i64 * 86_400 + 18 * 3600); // 13:00 New York
        let r3 = step(&mut QueueAt(n - 1), &store, &state, &risk, None, Some(&mut other), afternoon).unwrap();
        assert_eq!(r3.signals[0].source, "synthetic (no leg in the chain)");
        assert!((r3.signals[0].entry - r2.signals[0].entry).abs() > 0.0); // priced at `now`, not at the bar close

        // An entry that would fill before the paper start is dropped: a bot that queues on bar 5 trades nothing.
        let r4 = step(&mut QueueAt(5), &store, &state, &risk, None, None, now).unwrap();
        assert!(r4.text.contains("closed trades 0") && r4.signals.is_empty());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn arguments() {
        let a = parse_args(&["s05_ic_0dte".into(), "--no-live".into(), "--max-risk".into(), "250".into(), "--start".into(), "2026-11-19".into()]).unwrap();
        assert_eq!((a.bots.as_slice(), a.no_live, a.max_risk, a.start.as_deref()), (&["s05_ic_0dte".to_string()][..], true, Some(250.0), Some("2026-11-19")));
        assert!(parse_args(&["--bogus".into()]).is_err());
        assert!(parse_args(&["--equity".into()]).is_err());
        assert_eq!(command(Path::new("/nonexistent"), &["nope".into()]).unwrap(), 2);
    }
}
