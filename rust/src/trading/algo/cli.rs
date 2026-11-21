//! `fa algo run` and `fa algo replay`. The same engine and the same strategy run in both; only the source differs.
//!
//! Files, in `<repo>/data/processed/algo/<date>/`: `<strategy>-inputs.jsonl` (each input that the live run
//! read), `<strategy>-journal.jsonl` and `<strategy>-summary.json`. A replay writes `<strategy>-replay-journal.jsonl`
//! and `<strategy>-replay-summary.json`, so it never overwrites the live files. The kill file is
//! `<repo>/data/processed/algo/STOP`.

use super::demo::Demo;
use super::engine::{Config, Engine, Summary};
use super::source::{read_recorded, read_stored_bars, AlpacaQuotes, DataClient, Live, Next, Replay, Source};
use super::Strategy;
use crate::market::alpaca::{feed_from_env, ny_day, now_utc, Client, Keys};
use crate::trading::data::Store;
use std::io::Write;
use std::path::{Path, PathBuf};

const USAGE: &str = "usage:
  fa algo run --strategy demo [--symbol SPY] [--poll-secs N] [--stale-secs N] [--max-pos N] [--max-loss USD] [--spread-share X] [--latency-ms N] [--fee F] [--until HH:MM]
  fa algo replay --strategy demo --date YYYY-MM-DD [--symbol SPY] [--max-pos N] [--max-loss USD] [--spread-share X] [--latency-ms N] [--fee F] [--until HH:MM]
The simulator fills in memory. The command sends no order to a broker. Stop a run: create the file data/processed/algo/STOP
(in the repo folder; the run prints the full path). The loss stop is approximate: it checks each input, and the position closes at the next price.
Run one Alpaca command at a time: the rate guard (at most 120 requests a minute) is for this process only, and the plan allows 200 a minute for the whole key.
A restart on the same day resets the max loss to 0. `fa algo` is outside the trading sleeves: a pipe test, no result is used for a decision.";

/// The free Alpaca plan allows 200 requests a minute for the whole account (a daily save task uses the same key).
pub const DEFAULT_POLL_SECS: f64 = 1.0;
pub const MAX_REQUESTS_PER_MIN: f64 = 120.0;

#[derive(Debug, Clone)]
pub struct Opts {
    pub strategy: String,
    pub symbol: String,
    pub date: Option<String>,
    pub poll_secs: f64,
    pub stale_secs: i64,
    pub max_pos: i64,
    pub max_loss: f64,
    pub spread_share: f64,
    pub latency_ms: i64,
    pub fee: f64,
    pub until: String,
}

/// Requests each minute of the live source (one request for each symbol and poll).
pub fn requests_per_min(poll_secs: f64, symbols: usize) -> f64 {
    60.0 / poll_secs * symbols as f64
}

pub fn parse_opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts { strategy: String::new(), symbol: "SPY".into(), date: None, poll_secs: DEFAULT_POLL_SECS, stale_secs: super::source::STALE_SECS, max_pos: 100, max_loss: 500.0, spread_share: 1.0, latency_ms: 250, fee: 0.005, until: "15:55".into() };
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let mut val = || it.next().cloned().ok_or_else(|| format!("{flag} needs a value"));
        let num = |s: String| s.parse::<f64>().ok().filter(|x| x.is_finite()).ok_or_else(|| format!("{flag}: '{s}' is not a number"));
        match flag.as_str() {
            "--strategy" => o.strategy = val()?,
            "--symbol" => o.symbol = val()?.to_uppercase(),
            "--date" => o.date = Some(val()?),
            "--poll-secs" => o.poll_secs = num(val()?)?,
            "--stale-secs" => o.stale_secs = num(val()?)? as i64,
            "--max-pos" => o.max_pos = num(val()?)? as i64,
            "--max-loss" => o.max_loss = num(val()?)?,
            "--spread-share" => o.spread_share = num(val()?)?,
            "--latency-ms" => o.latency_ms = num(val()?)? as i64,
            "--fee" => o.fee = num(val()?)?,
            "--until" => o.until = val()?,
            other => return Err(format!("unknown option {other}\n{USAGE}")),
        }
    }
    if o.strategy != "demo" {
        return Err(format!("--strategy demo is the only strategy so far\n{USAGE}"));
    }
    if o.poll_secs <= 0.0 || requests_per_min(o.poll_secs, 1) > MAX_REQUESTS_PER_MIN {
        return Err(format!("--poll-secs {} gives {:.0} requests a minute; the limit is {MAX_REQUESTS_PER_MIN:.0} (the plan allows 200 for the whole account)", o.poll_secs, requests_per_min(o.poll_secs, 1)));
    }
    if o.max_pos <= 0 || o.max_loss <= 0.0 || !(0.0..=1.0).contains(&o.spread_share) || o.latency_ms < 0 || o.stale_secs <= 0 || o.fee < 0.0 {
        return Err("need --max-pos > 0, --max-loss > 0, --spread-share from 0 to 1, --latency-ms >= 0, --stale-secs > 0, --fee >= 0".into());
    }
    until_secs(&o.until)?;
    Ok(o)
}

fn until_secs(text: &str) -> Result<i64, String> {
    let (h, m) = text.split_once(':').ok_or_else(|| format!("--until '{text}': use HH:MM"))?;
    match (h.parse::<i64>(), m.parse::<i64>()) {
        (Ok(h), Ok(m)) if (0..24).contains(&h) && (0..60).contains(&m) => Ok(h * 3600 + m * 60),
        _ => Err(format!("--until '{text}': use HH:MM")),
    }
}

pub fn algo_dir(repo: &Path) -> PathBuf {
    repo.join("data/processed/algo")
}

fn config(o: &Opts, repo: &Path, feed: &str) -> Result<Config, String> {
    Ok(Config { max_pos: o.max_pos, max_loss: o.max_loss, spread_share: o.spread_share, latency_ms: o.latency_ms, fee: o.fee, until_secs: until_secs(&o.until)?, stop_file: algo_dir(repo).join("STOP"), feed: feed.to_string() })
}

fn describe(mode: &str, o: &Opts, s: &dyn Strategy, feed: &str) -> String {
    format!(
        "algo {mode}: strategy {} on {}\n  rules: {}\n  feed: {feed}\n  simulator: spread-share {} (1 = fill at the ask or bid), latency {} ms, fee {} USD a share, max position {} shares, max loss {} USD (approximate), stop at {} New York\n  {}",
        s.name(), o.symbol, s.params(), o.spread_share, o.latency_ms, o.fee, o.max_pos, o.max_loss, o.until, super::LIMITS
    )
}

/// A file that exists moves to `<name>.<unix secs>` so that a new run does not mix with an old one.
fn open_new(path: &Path) -> Result<std::fs::File, String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    if path.exists() {
        let old = PathBuf::from(format!("{}.{}", path.display(), now_utc()));
        std::fs::rename(path, &old).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The one loop for both modes. `record` gets each input before the engine does (live only).
pub fn drive<S: Strategy>(src: &mut dyn Source, engine: &mut Engine<S>, mut record: Option<&mut dyn Write>) -> Summary {
    loop {
        match src.next() {
            Next::End(_) => break,
            Next::Input(inp) => {
                if let Some(w) = record.as_mut() {
                    let _ = writeln!(w, "{}", serde_json::to_string(&inp).unwrap_or_default());
                    let _ = w.flush();
                }
                if !engine.on_input(&inp) {
                    break;
                }
            }
        }
    }
    engine.finish()
}

/// Where a later change connects the results of `fa algo` to the research harness (the hypothesis file and the
/// add-only run log of `rust/src/portfolio/harness.rs`). It does nothing now. A demo run is a pipe test, not a result.
pub fn report_to_harness(_mode: &str, _date: &str, _opts: &Opts, _summary: &Summary) {}

fn save_summary(path: &Path, s: &Summary, extra: serde_json::Value) -> Result<(), String> {
    let mut v = s.to_json();
    if let (Some(m), Some(e)) = (v.as_object_mut(), extra.as_object()) {
        for (k, x) in e {
            m.insert(k.clone(), x.clone());
        }
    }
    std::fs::write(path, serde_json::to_string_pretty(&v).unwrap_or_default() + "\n").map_err(|e| format!("{}: {e}", path.display()))
}

pub fn command(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        Some("run") => run(repo, &args[1..]),
        Some("replay") => replay(repo, &args[1..]),
        Some("--help") | Some("-h") | Some("help") => {
            println!("{USAGE}");
            Ok(0)
        }
        _ => Err(format!("algo needs a command: run or replay\n{USAGE}")),
    }
}

fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    let o = parse_opts(args)?;
    let keys = Keys::read(repo).ok_or("no Alpaca keys: set APCA_API_KEY_ID and APCA_API_SECRET_KEY (environment or .env)")?;
    let cfg = config(&o, repo, "")?;
    let date = ny_day(now_utc()).0;
    let dir = algo_dir(repo).join(&date);
    let journal = open_new(&dir.join(format!("{}-journal.jsonl", o.strategy)))?;
    let mut inputs = open_new(&dir.join(format!("{}-inputs.jsonl", o.strategy)))?;
    let demo = Demo::new();
    let feed_name = format!("alpaca {} quotes, polled every {} s (not the NBBO)", feed_from_env(), o.poll_secs);
    let cfg = Config { feed: feed_name.clone(), ..cfg };
    println!("{}", describe("run (live quotes, simulated fills)", &o, &demo, &feed_name));
    println!("  kill file (create it to stop and flatten): {}", absolute(&cfg.stop_file).display());
    println!("  a restart on the same day resets the max loss to 0 (the engine does not read the earlier journal)");
    println!("  quotes older than {} s, or older than the open, are skipped", o.stale_secs);
    println!("  request rate: {:.0} a minute for {} symbol(s) (limit {MAX_REQUESTS_PER_MIN:.0}; the plan allows 200 for the whole account)", requests_per_min(o.poll_secs, 1), 1);
    println!("  files: {}", dir.display());
    if cfg.stop_file.exists() {
        return Err(format!("the kill file {} exists: remove it to start", cfg.stop_file.display()));
    }
    let feed = AlpacaQuotes::new(DataClient::new(Client::live(keys)), &o.symbol, &feed_from_env());
    let mut live = Live::new(feed, &o.symbol, (o.poll_secs * 1000.0) as u64, cfg.until_secs, o.stale_secs, cfg.stop_file.clone(), Box::new(now_ms), Box::new(|ms| std::thread::sleep(std::time::Duration::from_millis(ms))));
    let started = std::time::Instant::now();
    let mut engine = Engine::new(cfg, demo, Some(Box::new(journal)));
    let s = drive(&mut live, &mut engine, Some(&mut inputs));
    let minutes = started.elapsed().as_secs_f64() / 60.0;
    let reqs = live.requests();
    println!("{}", s.text());
    println!("requests: {reqs} in {minutes:.1} minutes = {:.1} a minute", if minutes > 0.0 { reqs as f64 / minutes } else { 0.0 });
    save_summary(&dir.join(format!("{}-summary.json", o.strategy)), &s, serde_json::json!({"mode": "live", "date": date, "requests": reqs, "requests_per_min": if minutes > 0.0 { reqs as f64 / minutes } else { 0.0 }}))?;
    report_to_harness("run", &date, &o, &s);
    Ok(0)
}

/// An absolute path for the print (no `canonicalize`: the file need not exist).
fn absolute(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().map(|d| d.join(p)).unwrap_or_else(|_| p.to_path_buf())
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn replay(repo: &Path, args: &[String]) -> Result<i32, String> {
    let o = parse_opts(args)?;
    let date = o.date.clone().ok_or("fa algo replay needs --date YYYY-MM-DD")?;
    let dir = algo_dir(repo).join(&date);
    let recorded = dir.join(format!("{}-inputs.jsonl", o.strategy));
    let (inputs, source, feed, bar_minutes) = if recorded.exists() {
        (read_recorded(&recorded)?, format!("recorded inputs {}", recorded.display()), "recorded live quotes (alpaca, not the NBBO)".to_string(), None)
    } else {
        let store = Store::from_env(repo);
        let (v, name, minutes) = read_stored_bars(store.dir(), &o.symbol, &date)?;
        (v, format!("stored bars {name} in {}", store.dir().display()), format!("stored {minutes}-minute bars {name} (the spread of a bar is the assumed {} USD)", super::source::BAR_SPREAD), Some(minutes))
    };
    let cfg = config(&o, repo, &feed)?;
    let demo = Demo::new();
    println!("{}", describe("replay", &o, &demo, &feed));
    println!("  source: {source} ({} inputs)", inputs.len());
    if let Some(m) = bar_minutes {
        println!("  bar mode: the strategy decides on a bar close and an order fills at the OPEN of the next bar. The time of the fill after the decision is the gap between the bars (0 s for bars that follow each other), not {} ms.", o.latency_ms);
        println!("  the bars are {m} minute(s) long: the demo window of 20 closes is {} minutes", 20 * m);
        if o.latency_ms > 0 {
            println!("  WARNING: --latency-ms {} is not applied in bar mode: a bar has no time inside it, and the real latency is below the bar length ({} s)", o.latency_ms, m * 60);
        }
    }
    if cfg.stop_file.exists() {
        println!("  note: the kill file {} exists: the replay stops at its first input", absolute(&cfg.stop_file).display());
    }
    let journal = open_new(&dir.join(format!("{}-replay-journal.jsonl", o.strategy)))?;
    let mut engine = Engine::new(cfg, demo, Some(Box::new(journal)));
    let s = drive(&mut Replay::new(inputs), &mut engine, None);
    println!("{}", s.text());
    save_summary(&dir.join(format!("{}-replay-summary.json", o.strategy)), &s, serde_json::json!({"mode": "replay", "date": date, "source": source}))?;
    report_to_harness("replay", &date, &o, &s);
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::algo::source::Replay;
    use crate::trading::algo::Input;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn poll_rate_is_limited() {
        assert!((requests_per_min(DEFAULT_POLL_SECS, 1) - 60.0).abs() < 1e-9);
        assert!(parse_opts(&args("--strategy demo")).is_ok());
        assert!(parse_opts(&args("--strategy demo --poll-secs 0.5")).is_ok()); // 120 a minute
        let e = parse_opts(&args("--strategy demo --poll-secs 0.4")).unwrap_err();
        assert!(e.contains("150 requests"), "{e}");
        assert!(parse_opts(&args("--strategy other")).is_err());
        assert!(parse_opts(&args("--strategy demo --until 25:00")).is_err());
    }

    /// A day of fixed inputs: a quiet random-like walk and then moves that make the demo trade.
    fn fixture() -> Vec<Input> {
        let t0 = 1_790_000_000_000i64 - 13 * 60_000 - 20_000; // 10:00:00 New York
        (0..120)
            .flat_map(|m| {
                let px = 100.0 + if m < 40 { (m % 3) as f64 * 0.02 } else if m < 50 { -1.0 } else if m < 70 { 0.0 } else if m < 80 { 1.2 } else { (m % 3) as f64 * 0.02 };
                (0..3).map(move |k| {
                    let ms = t0 + m * 60_000 + k * 20_000;
                    Input { quote_ms: ms - 30, read_ms: ms + 10, symbol: "SPY".into(), bid: px - 0.01 + k as f64 * 0.001, ask: px + 0.01 + k as f64 * 0.001, kind: "quote".into() }
                })
            })
            .collect()
    }

    fn run_day(inputs: Vec<Input>) -> (Vec<super::super::engine::Decision>, Summary) {
        let c = Config { max_pos: 100, max_loss: 500.0, spread_share: 1.0, latency_ms: 250, fee: 0.005, until_secs: 15 * 3600 + 55 * 60, stop_file: std::env::temp_dir().join("fa-algo-no-such-stop"), feed: "test".into() };
        let mut e = Engine::new(c, Demo::new(), None);
        let s = drive(&mut Replay::new(inputs), &mut e, None);
        (e.decisions().to_vec(), s)
    }

    #[test]
    fn replay_of_a_recorded_file_gives_the_same_decisions() {
        let dir = std::env::temp_dir().join(format!("fa-algo-parity-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("demo-inputs.jsonl");
        // Record the fixture as the live run does, then read it back.
        let mut w: Vec<u8> = vec![];
        for i in fixture() {
            writeln!(w, "{}", serde_json::to_string(&i).unwrap()).unwrap();
        }
        std::fs::write(&path, w).unwrap();
        let back = read_recorded(&path).unwrap();
        assert_eq!(back, fixture());
        let (a, sa) = run_day(back.clone());
        let (b, sb) = run_day(back);
        assert!(!a.is_empty(), "the fixture must make the demo trade");
        assert_eq!(a, b);
        assert_eq!(a, run_day(fixture()).0);
        assert!((sa.pnl_net - sb.pnl_net).abs() < 1e-12);
        assert_eq!(sa.config["feed"], "test");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A signal uses data known at its time: the decisions on a prefix of the day equal the prefix of the decisions of the full day.
    #[test]
    fn truncation_gives_the_prefix_of_the_decisions() {
        let full = fixture();
        let (all, _) = run_day(full.clone());
        assert!(all.len() >= 2, "the fixture must make the demo trade");
        for cut in [full[full.len() / 3].read_ms, full[full.len() / 2].read_ms, full[full.len() * 2 / 3].read_ms] {
            let prefix: Vec<Input> = full.iter().filter(|i| i.read_ms <= cut).cloned().collect();
            let (got, _) = run_day(prefix);
            let want: Vec<_> = all.iter().filter(|d| d.decision_ms <= cut).cloned().collect();
            assert_eq!(got, want, "cut at {cut}");
        }
        // Every input of the fixture is known at its read time.
        assert!(full.iter().all(|i| i.check().is_ok()));
    }
}
