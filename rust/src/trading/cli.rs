//! The command `fa trade backtest`.

use super::bots;
use super::calendar::parse_iso;
use super::data::{load_market, MarketData, Store};
use super::engine::{run, BacktestResult, RiskConfig};
use super::metrics::{format_summary, summarize, Summary};
use super::stats;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

pub const USAGE: &str = "\
fa trade backtest - backtest the SPY/QQQ option bots on the cached market data

USAGE:
  fa trade backtest [--bot ID]... [--max-risk DOLLARS] [--risk FRACTION] [--equity DOLLARS]
                    [--commission DOLLARS] [--start D] [--end D] [--is-fraction F]
                    [--spread-mult X] [--skew-mult X] [--mid-fill] [--trials N] [--sr-var V]
                    [--include-templates] [--out FILE] [--trades-dir DIR] [-v]

  --bot ID          run this bot only (repeat the option for more bots; default: all bots)
  --max-risk N      the maximum loss of one trade is N dollars (for example 250); it replaces --risk
  --risk F          the maximum loss of one trade as a fraction of equity (default 0.03)
  --equity N        the starting equity (default 2000)
  --commission N    dollars for each contract (default 0.65)
  --start, --end    first and last date of the data, YYYY-MM-DD
  --is-fraction F   the first part of the bars that is in-sample (default 0.7)
  --spread-mult X  multiply the model half-spread by X (default 1)
  --skew-mult X    multiply the model skew slope and curve by X (default 1)
  --mid-fill       fill at the mid price (no spread): a better price, a reference only
  --trials N       print the out-of-sample t of the trade P&L and the deflated Sharpe ratio for N trials
  --sr-var V       variance of the daily Sharpe ratios of the trials (default 1/T)
  --out FILE        write the leaderboard as a CSV file (only if all bots run)
  --trades-dir DIR  write the trades and the equity curve of each bot as CSV files
  -v                print the last trades of each bot

The data are the CSV files in $TRADING_DATA_DIR, else in data/raw/trading/ of the repository.
`fa trade fetch` downloads them. All option prices are model prices.";

#[derive(Debug, Default)]
struct Args {
    bots: Vec<String>,
    max_risk: Option<f64>,
    risk: Option<f64>,
    equity: Option<f64>,
    commission: Option<f64>,
    start: Option<String>,
    end: Option<String>,
    is_fraction: Option<f64>,
    spread_mult: Option<f64>,
    skew_mult: Option<f64>,
    mid_fill: bool,
    trials: Option<usize>,
    sr_var: Option<f64>,
    include_templates: bool,
    out: Option<PathBuf>,
    trades_dir: Option<PathBuf>,
    verbose: bool,
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
            "--bot" => a.bots.push(it.next().ok_or("--bot needs a bot name")?.clone()),
            "--max-risk" => a.max_risk = Some(num(arg, it.next())?),
            "--risk" => a.risk = Some(num(arg, it.next())?),
            "--equity" => a.equity = Some(num(arg, it.next())?),
            "--commission" => a.commission = Some(num(arg, it.next())?),
            "--is-fraction" => a.is_fraction = Some(num(arg, it.next())?),
            "--spread-mult" => a.spread_mult = Some(num(arg, it.next())?),
            "--skew-mult" => a.skew_mult = Some(num(arg, it.next())?),
            "--mid-fill" => a.mid_fill = true,
            "--trials" => a.trials = Some(num(arg, it.next())? as usize),
            "--sr-var" => a.sr_var = Some(num(arg, it.next())?),
            "--start" => a.start = Some(it.next().ok_or("--start needs a date")?.clone()),
            "--end" => a.end = Some(it.next().ok_or("--end needs a date")?.clone()),
            "--out" => a.out = Some(PathBuf::from(it.next().ok_or("--out needs a file")?)),
            "--trades-dir" => a.trades_dir = Some(PathBuf::from(it.next().ok_or("--trades-dir needs a directory")?)),
            "--include-templates" => a.include_templates = true,
            "-v" | "--verbose" => a.verbose = true,
            other => return Err(format!("unknown option '{other}'")),
        }
    }
    Ok(a)
}

fn parse_date(name: &str, v: &Option<String>) -> Result<Option<i32>, String> {
    match v {
        None => Ok(None),
        Some(s) => parse_iso(s).map(Some).ok_or_else(|| format!("bad date for {name}: '{s}' (use YYYY-MM-DD)")),
    }
}

struct Done {
    summary: Summary,
    result: BacktestResult,
    secs: f64,
}

fn market_key(symbols: &[&str], interval: &str, start: Option<&str>) -> String {
    format!("{}|{}|{}", symbols.join(","), interval, start.unwrap_or(""))
}

/// Run the command on the cache of the repository. Returns the exit code.
pub fn backtest(repo: &Path, args: &[String]) -> Result<i32, String> {
    run_command(&Store::from_env(repo), args)
}

/// Run the command on a data cache. Returns the exit code.
pub fn run_command(store: &Store, args: &[String]) -> Result<i32, String> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let a = parse_args(args)?;
    let known = bots::names(a.include_templates);
    let names: Vec<String> = if a.bots.is_empty() { known.iter().map(|s| s.to_string()).collect() } else { a.bots.clone() };
    let missing: Vec<&String> = names.iter().filter(|n| !known.contains(&n.as_str())).collect();
    if !missing.is_empty() {
        eprintln!("unknown bots: {missing:?}\navailable: {known:?}");
        return Ok(2);
    }
    let equity = a.equity.unwrap_or(2000.0);
    let commission = a.commission.unwrap_or(0.65);
    let risk = match a.max_risk {
        Some(m) if m != 0.0 => RiskConfig::with_max_risk(m, equity, commission),
        _ => RiskConfig { starting_equity: equity, risk_per_trade: a.risk.unwrap_or(0.03), commission_per_contract: commission, ..RiskConfig::default() },
    };
    parse_date("--start", &a.start)?;
    let cli_end = parse_date("--end", &a.end)?;
    let is_fraction = a.is_fraction.unwrap_or(0.7);
    super::pricing::set_price_knobs(a.spread_mult.unwrap_or(1.0), a.skew_mult.unwrap_or(1.0), a.mid_fill);
    let t_all = Instant::now();

    // One market for each (symbols, interval, start): load it once and share it between bots.
    let mut wanted: BTreeMap<String, (Vec<&'static str>, super::data::Interval, Option<i32>)> = BTreeMap::new();
    for n in &names {
        let b = bots::make(n).expect("bot exists");
        let start_text = a.start.as_deref().or(b.start());
        let start = match start_text {
            Some(s) => Some(parse_iso(s).ok_or_else(|| format!("bad start date '{s}'"))?),
            None => None,
        };
        wanted.entry(market_key(b.symbols(), b.interval().label(), start_text)).or_insert((b.symbols().to_vec(), b.interval(), start));
    }
    let markets: Vec<(String, Result<Arc<MarketData>, String>)> = wanted
        .into_par_iter()
        .map(|(k, (syms, iv, start))| (k, load_market(store, &syms, iv, start, cli_end).map(Arc::new)))
        .collect();
    let markets: BTreeMap<String, Result<Arc<MarketData>, String>> = markets.into_iter().collect();

    let outcomes: Vec<Result<Done, String>> = names
        .par_iter()
        .map(|n| {
            let t0 = Instant::now();
            let mut bot = bots::make(n).expect("bot exists");
            let start_text = a.start.as_deref().or(bot.start());
            let md = markets.get(&market_key(bot.symbols(), bot.interval().label(), start_text)).expect("market was loaded").clone()?;
            let result = run(bot.as_mut(), &md, risk.clone())?;
            let summary = summarize(&result, bot.interval().label(), is_fraction);
            Ok(Done { summary, result, secs: t0.elapsed().as_secs_f64() })
        })
        .collect();

    if let Some(dir) = &a.trades_dir {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tag = match a.max_risk {
        Some(m) if m != 0.0 => format!("_risk{}", m as i64),
        _ => String::new(),
    };
    let mut rows: Vec<Summary> = Vec::new();
    let mut failed = 0;
    for (n, out) in names.iter().zip(outcomes) {
        match out {
            Err(e) => {
                eprintln!("[{n}] FAILED: {e}");
                failed += 1;
            }
            Ok(d) => {
                println!("{}  ({:.1}s)", format_summary(&d.summary), d.secs);
                if !d.result.rejections.is_empty() {
                    let items: Vec<String> = d.result.rejections.iter().map(|(k, v)| format!("'{k}': {v}")).collect();
                    println!("  rejections: {{{}}}", items.join(", "));
                }
                if let Some(dir) = &a.trades_dir {
                    write_trades(dir, n, &tag, &d.result)?;
                }
                if a.verbose {
                    print_trades(&d.result);
                }
                if let Some(nt) = a.trials {
                    print_robust(&d.result, is_fraction, nt, a.sr_var);
                }
                println!();
                rows.push(d.summary);
            }
        }
    }
    if !rows.is_empty() {
        rows.sort_by(|x, y| y.oos.sharpe.partial_cmp(&x.oos.sharpe).unwrap_or(std::cmp::Ordering::Equal));
        if let (Some(out), true) = (&a.out, a.bots.is_empty()) {
            std::fs::write(out, leaderboard_csv(&rows)).map_err(|e| format!("{}: {e}", out.display()))?;
        }
        println!("LEADERBOARD (sorted by out-of-sample Sharpe; option prices are model-priced)");
        print!("{}", leaderboard_table(&rows));
    }
    println!("total time {:.1}s", t_all.elapsed().as_secs_f64());
    Ok(if failed > 0 { 1 } else { 0 })
}

/// Out-of-sample t of the mean trade P&L (dollars, and as a share of the max loss) and the deflated Sharpe ratio.
fn print_robust(r: &BacktestResult, is_fraction: f64, n_trials: usize, sr_var: Option<f64>) {
    let n = r.equity.len();
    let cut = if n == 0 { 0 } else { r.date[(n as f64 * is_fraction) as usize] as i64 * 1440 + r.minute[(n as f64 * is_fraction) as usize] as i64 };
    let oos: Vec<&super::engine::Trade> = r.trades.iter().filter(|t| t.entry_time.key() >= cut).collect();
    let pnl: Vec<f64> = oos.iter().map(|t| t.pnl).collect();
    let all_pnl: Vec<f64> = r.trades.iter().map(|t| t.pnl).collect();
    let rel: Vec<f64> = oos.iter().filter(|t| t.max_loss > 0.0).map(|t| t.pnl / t.max_loss).collect();
    let rets = super::metrics::daily_returns(r, is_fraction, true);
    if rets.len() < 3 {
        println!("  ROBUST: too few days");
        return;
    }
    let sr = stats::mean(&rets) / stats::std(&rets, 1);
    let (sk, ku) = stats::skew_kurt(&rets);
    let var = sr_var.unwrap_or(1.0 / rets.len() as f64);
    let (sr0, p) = stats::deflated_sharpe(sr, rets.len(), sk, ku, n_trials, var);
    println!(
        "  ROBUST (OOS, model prices): trades {}  mean P&L ${:.2}  t(P&L) {:.2}  t(P&L/maxloss) {:.2}  daily SR {:.4} (annual {:.2})  days {}  skew {:.1} kurt {:.1}  trials {}  SR0 {:.4}  DSR {:.3}  | all-trades t(P&L) {:.2} (n {})",
        pnl.len(), if pnl.is_empty() { 0.0 } else { stats::mean(&pnl) }, stats::t_mean(&pnl), stats::t_mean(&rel), sr, sr * 252f64.sqrt(), rets.len(), sk, ku, n_trials, sr0, p, stats::t_mean(&all_pnl), all_pnl.len()
    );
}

fn print_trades(r: &BacktestResult) {
    let from = r.trades.len().saturating_sub(60);
    for t in &r.trades[from..] {
        println!(
            "  {} {}  {:<20} x{:<3} {:>8.2} {:>8.2} {:>9.2}  {}",
            clock(t.entry_time),
            clock(t.exit_time),
            t.structure,
            t.units,
            t.entry_net,
            t.exit_net,
            t.pnl,
            t.exit_reason
        );
    }
}

fn clock(t: super::engine::Time) -> String {
    format!("{} {:02}:{:02}", super::calendar::iso(t.date), t.minute / 60, t.minute % 60)
}

fn write_trades(dir: &Path, name: &str, tag: &str, r: &BacktestResult) -> Result<(), String> {
    let mut t = String::from("id,strategy,symbol,structure,description,tag,units,entry_time,exit_time,entry_net,exit_net,max_loss,pnl,exit_reason\n");
    for x in &r.trades {
        t.push_str(&format!(
            "{},{},{},{},\"{}\",\"{}\",{},{},{},{:?},{:?},{:?},{:?},{}\n",
            x.id, x.strategy, x.symbol, x.structure, x.description, x.tag, x.units, clock(x.entry_time), clock(x.exit_time), x.entry_net, x.exit_net, x.max_loss, x.pnl, x.exit_reason
        ));
    }
    let p = dir.join(format!("{name}{tag}_trades.csv"));
    std::fs::write(&p, t).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut e = String::from("time,equity\n");
    for k in 0..r.equity.len() {
        e.push_str(&format!("{} {:02}:{:02},{:?}\n", super::calendar::iso(r.date[k]), r.minute[k] / 60, r.minute[k] % 60, r.equity[k]));
    }
    let p = dir.join(format!("{name}{tag}_equity.csv"));
    std::fs::write(&p, e).map_err(|e| format!("{}: {e}", p.display()))
}

const COLS: [&str; 19] = [
    "strategy", "interval", "start", "end", "final_equity", "all_return_pct", "all_max_dd_pct", "all_sharpe", "all_trades", "all_win_rate_pct",
    "all_profit_factor", "oos_return_pct", "oos_max_dd_pct", "oos_sharpe", "oos_trades", "oos_win_rate_pct", "oos_profit_factor", "rejected", "pdt_blocks",
];

fn cells(s: &Summary, csv: bool) -> Vec<String> {
    let f = |x: f64| if csv { format!("{x:?}") } else { format!("{x:.2}") };
    vec![
        s.strategy.to_string(), s.interval.to_string(), s.start.clone(), s.end.clone(), f(s.final_equity),
        f(s.all.return_pct), f(s.all.max_dd_pct), f(s.all.sharpe), s.all.trades.to_string(), f(s.all.win_rate_pct), f(s.all.profit_factor),
        f(s.oos.return_pct), f(s.oos.max_dd_pct), f(s.oos.sharpe), s.oos.trades.to_string(), f(s.oos.win_rate_pct), f(s.oos.profit_factor),
        s.rejected.to_string(), s.pdt_blocks.to_string(),
    ]
}

fn leaderboard_csv(rows: &[Summary]) -> String {
    let mut t = COLS.join(",");
    t.push('\n');
    for r in rows {
        t.push_str(&cells(r, true).join(","));
        t.push('\n');
    }
    t
}

fn leaderboard_table(rows: &[Summary]) -> String {
    let body: Vec<Vec<String>> = rows.iter().map(|r| cells(r, false)).collect();
    let widths: Vec<usize> = (0..COLS.len()).map(|c| body.iter().map(|r| r[c].len()).chain([COLS[c].len()]).max().unwrap_or(0)).collect();
    let line = |v: Vec<String>| -> String {
        let parts: Vec<String> = v.iter().enumerate().map(|(c, s)| if c < 2 { format!("{s:<w$}", w = widths[c]) } else { format!("{s:>w$}", w = widths[c]) }).collect();
        parts.join("  ") + "\n"
    };
    let mut out = line(COLS.iter().map(|s| s.to_string()).collect());
    for r in body {
        out.push_str(&line(r));
    }
    out
}


#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::trading::calendar::{busday_offset, days_from_civil, iso};
    use crate::trading::data::Interval;

    fn f2(x: f64) -> f64 {
        format!("{x:.2}").parse().expect("number")
    }

    /// 150 daily bars from 2024-03-11 (all in daylight saving time, offset -04:00) of a smooth price path,
    /// with the vol indexes and the short rate. The expected numbers in the tests below come from the
    /// Python backtester on the same files.
    pub(crate) fn write_fixture(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        let first = days_from_civil(2024, 3, 11);
        let dates: Vec<String> = (0..150).map(|k| iso(busday_offset(first, k, true))).collect();
        let close: Vec<f64> = (0..150).map(|i| f2(480.0 + 0.15 * i as f64 + 15.0 * (i as f64 / 11.0).sin())).collect();
        let file = |name: &str, rows: Vec<[f64; 5]>| {
            let mut t = String::from("Date,open,high,low,close,volume\n");
            for (d, r) in dates.iter().zip(rows) {
                t.push_str(&format!("{d} 00:00:00-04:00,{:.2},{:.2},{:.2},{:.2},{}\n", r[0], r[1], r[2], r[3], r[4]));
            }
            std::fs::write(dir.join(name), t).unwrap();
        };
        let spy = (0..150)
            .map(|i| {
                let o = if i > 0 { f2(close[i - 1] + 0.1) } else { close[0] };
                [o, o.max(close[i]) + 0.5, o.min(close[i]) - 0.5, close[i], 1000.0]
            })
            .collect();
        file("SPY_1d.csv", spy);
        let flat = |f: &dyn Fn(f64) -> f64| (0..150).map(|i| f2(f(i as f64))).map(|v| [v, v, v, v, 0.0]).collect::<Vec<_>>();
        file("_VIX_1d.csv", flat(&|i| 16.0 + 4.0 * (i / 7.0).sin()));
        file("_VIX1D_1d.csv", flat(&|i| 12.0 + 5.0 * (i / 5.0 + 1.0).sin()));
        file("_IRX_1d.csv", flat(&|_| 4.3));
    }

    #[test]
    fn end_to_end_on_a_fixture_cache() {
        let dir = std::env::temp_dir().join(format!("fa-trading-e2e-{}", std::process::id()));
        write_fixture(&dir);
        let store = Store::new(dir.clone());
        let md = load_market(&store, &["SPY"], Interval::Daily, None, None).unwrap();
        assert_eq!(md.len(), 150);
        let mut bot = bots::make("baseline_weekly_call_spread").unwrap();
        let r = run(bot.as_mut(), &md, RiskConfig::default()).unwrap();
        // Python backtester, same files: 29 trades, total P&L -111.59088888062628, final equity 1888.4091111193734.
        assert_eq!(r.trades.len(), 29);
        let total: f64 = r.trades.iter().map(|t| t.pnl).sum();
        assert!((total + 111.590_888_880_626_28).abs() < 1e-8, "{total}");
        assert!((r.equity[149] - 1_888.409_111_119_373_4).abs() < 1e-8);
        let t = &r.trades[0];
        assert_eq!((iso(t.entry_time.date), t.entry_time.minute, iso(t.exit_time.date), t.exit_reason.as_str()), ("2024-03-18".to_string(), 570, "2024-03-22".to_string(), "dte_exit"));
        assert!((t.entry_net - 0.433_227_393_607_641_6).abs() < 1e-12 && (t.exit_net - 0.685_188_919_466_677_4).abs() < 1e-12);
        assert!((t.pnl - 22.596_152_585_903_596).abs() < 1e-9);
        assert_eq!(r.trades[2].exit_reason, "stop_loss");
        assert!(r.rejections.is_empty() && r.pdt_blocks == 0);
        let s = summarize(&r, "1d", 0.7);
        assert!((s.all.return_pct + 5.579_544_444_031_326).abs() < 1e-9);
        assert!((s.all.max_dd_pct + 14.124_627_509_798_05).abs() < 1e-9);
        assert!((s.all.sharpe + 1.634_275_407_771_309_3).abs() < 1e-9);
        assert!((s.all.win_rate_pct - 44.827_586_206_896_55).abs() < 1e-9);
        assert!((s.all.profit_factor - 0.747_525_545_200_433_9).abs() < 1e-9);
        assert!((s.all.cagr_pct + 9.634_158_102_336_343).abs() < 1e-7);

        // The command: one bot, trade files, and the printed leaderboard path.
        let out = dir.join("out");
        let args: Vec<String> = ["--bot", "baseline_weekly_call_spread", "--include-templates", "--trades-dir", out.to_str().unwrap()].iter().map(|s| s.to_string()).collect();
        assert_eq!(run_command(&store, &args).unwrap(), 0);
        let trades = std::fs::read_to_string(out.join("baseline_weekly_call_spread_trades.csv")).unwrap();
        assert_eq!(trades.lines().count(), 30);
        assert!(trades.lines().nth(1).unwrap().contains("SPY 2024-03-"), "{trades}");
        let equity = std::fs::read_to_string(out.join("baseline_weekly_call_spread_equity.csv")).unwrap();
        assert_eq!(equity.lines().count(), 151);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn arguments_and_errors() {
        let a = parse_args(&["--bot".into(), "s05_ic_0dte".into(), "--max-risk".into(), "250".into(), "-v".into()]).unwrap();
        assert_eq!((a.bots.as_slice(), a.max_risk, a.verbose), (&["s05_ic_0dte".to_string()][..], Some(250.0), true));
        assert!(parse_args(&["--bogus".into()]).is_err());
        assert!(parse_args(&["--max-risk".into()]).is_err());
        assert!(parse_args(&["--max-risk".into(), "abc".into()]).is_err());
        assert!(parse_date("--start", &Some("2024-13-1".into())).is_err());
        // An unknown bot gives exit code 2 before any data is read.
        let store = Store::new(std::env::temp_dir().join("fa-no-such-cache"));
        assert_eq!(run_command(&store, &["--bot".into(), "nope".into()]).unwrap(), 2);
        // A missing cache fails the bot with a message, and the exit code is 1.
        assert_eq!(run_command(&store, &["--bot".into(), "s08_trend_regime".into()]).unwrap(), 1);
    }

    #[test]
    fn all_22_bots_are_registered() {
        let names = bots::names(false);
        assert_eq!(names.len(), 22);
        assert!(names.contains(&"s14b_spy_trend_calls") && names.contains(&"s21_bollinger_fade"));
    }
}
