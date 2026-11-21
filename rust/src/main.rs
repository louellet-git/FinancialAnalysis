//! `fa`: command-line access to FinancialAnalysis. Rates, bonds, forwards, futures, models,
//! DCF, backtest, verdict, report, recon and trade are Rust. Typed commands print pretty JSON.

use financial_analysis::{CheckResult, Fa};
use serde::Serialize;
use std::process::ExitCode;

mod serve;

const USAGE: &str = "\
fa - FinancialAnalysis in Rust

USAGE:
  fa rates [snapshot] [--date D]                  curves, SOFR, EFFR, FOMC range, inflation
  fa rates yield --tenor 10Y [--real] [--date D]  one Treasury par yield
  fa rates curve [--real] [--date D]              one full Treasury curve
  fa rates history --tenor 10Y --start D [--end D] [--real]
  fa rates nyfed --rate SOFR|EFFR|OBFR|TGCR|BGCR [--date D]
  fa rates inflation [--date D]                   breakevens, 5y5y forward, CPI (BLS)
  fa rates fred --series ID [--start D] [--raw]   any FRED series
  fa rates check [--days N]                       cross-check against FRED (exit 1 on mismatch)
  fa bonds [snapshot] [--date D]                  every rating, Moody's Aaa/Baa, HQM corporate curve
  fa bonds spread|yield|over10y --rating A- [--date D]
                                                  ICE OAS, index yield, or yield minus UST 10Y
  fa bonds moodys --grade Aaa|Baa [--date D]      Moody's seasoned corporate yield
  fa bonds hqm [--date D]                         Treasury HQM corporate spot curve (monthly)
  fa bonds check [--days N]                       Moody's - 10Y vs FRED (exit 1 on mismatch)
  fa forwards [--date D]                          GSW zero-coupon yields and forward rates (Federal Reserve Board)
  fa forwards check [--days N]                    GSW par yields against Treasury (exit 1 on mismatch)
  fa futures roots                                contract facts of each root
  fa futures quote SYMBOL [--date D]              last daily value; SYMBOL is a root (ES) or a contract (ESZ26)
  fa futures history SYMBOL --start D [--end D]   daily values of a contract or of the Yahoo series of a root
  fa futures curve ROOT [--date D]                all listed contracts of one root; rate futures give the implied rate
  fa futures positions ROOT [--weeks N]           CFTC Commitments of Traders (financial futures)
  fa futures policy-path [--date D]               policy-rate path from SOFR options (Atlanta Fed)
  fa futures check [--date D]                     cross-checks of two sources (exit 1 on mismatch)
  fa models calibrate|rolling|frequencies|compare|simulate ...
                                                  stochastic rate models (Rust); see below
  fa vol fit heston|bates --symbol T --date D [--rate R] [--expiry-max DAYS]
                                                  fit Heston or Bates to a stored option snapshot (fa vol --help)
  fa dcf drivers|classic|scenarios|models ...     Monte Carlo DCF in Rust (fa dcf --help)
  fa backtest TICKER [--horizon Y] [--simulations N] [--seed S]
                                                  point-in-time vintage backtest of revenue and FCF projections (Rust)
  fa verdict TICKER [--price P] [--rf-date D|today] [--rf-base-date D] [--rating A-]
             [--simulations N] [--seed S]
                                                  Monte Carlo verdict; without --price: the last close of the price cache;
                                                  --rf-date takes the Treasury 10Y as rf,
                                                  --rating takes the market kd spread of the rating (network)
  fa prices get TICKER... [--tickers-file F] [--source S] [--full]   daily price history into data/raw (fa prices --help)
  fa prices intraday SYMBOL [--interval 1m|5m] --from D [--to D]   1- and 5-minute bars of Alpaca (free key) into data/raw/trading
  fa options snapshot SYMBOL [--source cboe|alpaca] | history SYMBOL --date D | --from D --to D
                                                  option chain of today, or the surface of past dates from daily closes (fa options --help)
  fa prices show TICKER [--from D] [--to D] | last TICKER... | check TICKER... [--sources A,B]
                                                  rows of the cache, last close, or a cross-check of two sources
  fa estimates get TICKER... [--tickers-file F] [--endpoints LIST] [--momentum] [--refresh]
                                                  analyst consensus EPS, price targets, surprises into data/raw (Nasdaq site, no key)
  fa estimates show TICKER [--price P]            consensus by period, revisions, forward P/E, target against the last close
  fa earnings calendar [TICKER...] [--tickers-file F] [--from D] [--days N]
                                                  Nasdaq earnings calendar scan, then the next report date of each ticker
  fa earnings dates TICKER... [--tolerance N]     past report dates against the SEC filing list (8-K item 2.02), next date
  fa macro premium-test --series FILE [--cache DIR] [--local DIR] [--out FILE] [--exclude-from D --exclude-to D] [--list sp500|sp100]
                                                  frozen test of the weekly change of the implied premium (fa macro premium-test --help)
  fa recon bonds [--date D] [--out FILE]          bond values from our rates vs market prices (see: fa recon help)
  fa recon bills [--date D]                       tests of each cause of the bill difference of recon bonds
  fa recon rich-cheap [--date D] [--out FILE]     fitted NSS curve, difference of each security, rich and cheap candidates
  fa recon history [--from D] [--to D] [--out F]  the same difference over the cached dates: how long it lasts
  fa portfolio backtest --strategy value-rank|momentum --start D --end D [--rebalance monthly|quarterly] [--cost-bp N] [--tickers-file F] [--cache DIR]
                                                  portfolio backtest with costs, point-in-time, against SPY (fa portfolio --help)
  fa portfolio calibrate --start D [--block-weeks N] [--ranks-out F] [--entries-out F] [--stress-file F]
                                                  the weekly rule against the 13-week outcome: tests T2 to T4 (fa portfolio --help)
  fa trade fetch [--symbol S]... [--interval I]...  download the bars of the option bots (Yahoo) into the CSV cache
  fa trade backtest [--bot ID]... [--max-risk N] [--out FILE] [-v]   backtest the option bots (fa trade backtest --help)
  fa trade paper [BOT]... [--no-refresh] [--no-live]   forward paper trading, live Cboe option chain (fa trade paper --help)
  fa sync [TICKER...] [--tickers-file F] [--only A,B] [--skip A,B] [--force] [--dry-run] [--max-minutes N]
                                                  bring all cached data sets up to date, only the old ones, one queue for each host (fa sync --help)
  fa sync --status [TICKER...]                    age of each data set from the cache files, no request
  fa import TICKER... [--refresh] [--periods N]   SEC data to the standard variables: data/processed/<TICKER>.json
  fa analyze TICKER [--refresh] [--out FILE] [--no-market] [--no-options] [--no-estimates]
                                                  import, market data, and the nine-sheet xlsx file (fa analyze --help)
  fa report TICKER [--price P] [--out FILE]       single-page HTML company report: price, fundamentals, DCF, driver model,
                                                  backtest, options (fa report --help)
  fa serve [--port N] [--no-open]                 local viewer in the browser (http://127.0.0.1:8765/)

D is YYYY-MM-DD or 'today'. Rates are decimals (0.0507 = 5.07%).

ENVIRONMENT:
  FA_REPO    repository root (default: the repository that holds this crate)";

/// Default subcommand "snapshot" when none (or only a flag) is given.
fn with_default_sub(rest: &[String]) -> (Vec<String>, String) {
    let mut args = rest.to_vec();
    match rest.first() {
        Some(s) if !s.starts_with('-') => (args, s.clone()),
        _ => {
            args.insert(0, "snapshot".into());
            (args, "snapshot".into())
        }
    }
}

fn exit_on_mismatch(results: &[CheckResult]) -> ExitCode {
    if results.iter().any(|r| !r.mismatches.is_empty()) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `--name value` in the arguments after the command word.
fn flag(rest: &[String], name: &str) -> Option<String> {
    rest.iter().position(|a| a == name).and_then(|i| rest.get(i + 1)).cloned()
}

fn has(rest: &[String], name: &str) -> bool {
    rest.iter().any(|a| a == name)
}

fn days_flag(rest: &[String], default: u32) -> Result<u32, String> {
    flag(rest, "--days").map_or(Ok(default), |d| d.parse::<u32>().map_err(|_| format!("bad --days '{d}'")))
}

fn bonds(fa: &Fa, rest: &[String]) -> financial_analysis::Result<ExitCode> {
    let (args, sub) = with_default_sub(rest);
    let date = flag(&args, "--date");
    let date = date.as_deref();
    let rating = flag(&args, "--rating").unwrap_or_else(|| "A".into());
    match sub.as_str() {
        "snapshot" => print_json(&fa.credit_snapshot(date)?),
        "spread" => print_json(&fa.credit_spread(&rating, date)?),
        "yield" => print_json(&fa.corporate_yield(&rating, date)?),
        "over10y" => print_json(&fa.spread_over_10y(&rating, date)?),
        "moodys" => {
            let grade = flag(&args, "--grade").unwrap_or_else(|| "Baa".into());
            print_json(&fa.moodys(&grade, date)?)
        }
        "hqm" => print_json(&fa.hqm_curve(date)?),
        "check" => {
            let results = match days_flag(&args, 90) {
                Ok(d) => fa.check_bonds(d)?,
                Err(m) => return Ok(usage_error(&m)),
            };
            print_json(&results);
            return Ok(exit_on_mismatch(&results));
        }
        other => return Ok(usage_error(&format!("unknown bonds command '{other}'"))),
    }
    Ok(ExitCode::SUCCESS)
}

fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).expect("serializable"));
}

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("fa: {msg}\n\n{USAGE}");
    ExitCode::from(64)
}

fn rates(fa: &Fa, rest: &[String]) -> financial_analysis::Result<ExitCode> {
    let (args, sub) = with_default_sub(rest);
    let date = flag(&args, "--date");
    let date = date.as_deref();
    let real = has(&args, "--real");
    match sub.as_str() {
        "snapshot" => print_json(&fa.snapshot(date)?),
        "yield" => print_json(&fa.treasury_yield(&flag(&args, "--tenor").unwrap_or_else(|| "10Y".into()), date, real)?),
        "nyfed" => print_json(&fa.nyfed(&flag(&args, "--rate").unwrap_or_else(|| "SOFR".into()), date)?),
        "curve" => print_json(&fa.curve(date, real)?),
        "history" => {
            let Some(start) = flag(&args, "--start") else { return Ok(usage_error("rates history needs --start D")) };
            let tenor = flag(&args, "--tenor").unwrap_or_else(|| "10Y".into());
            print_json(&fa.history(&tenor, &start, flag(&args, "--end").as_deref(), real)?)
        }
        "fred" => {
            let Some(series) = flag(&args, "--series") else { return Ok(usage_error("rates fred needs --series ID")) };
            print_json(&fa.fred(&series, flag(&args, "--start").as_deref(), has(&args, "--raw"))?)
        }
        "inflation" => print_json(&fa.inflation(date)?),
        "check" => {
            let results = match days_flag(&args, 30) {
                Ok(d) => fa.check(d)?,
                Err(m) => return Ok(usage_error(&m)),
            };
            print_json(&results);
            return Ok(exit_on_mismatch(&results));
        }
        other => return Ok(usage_error(&format!("unknown rates command '{other}'"))),
    }
    Ok(ExitCode::SUCCESS)
}

fn forwards_cmd(fa: &Fa, rest: &[String]) -> ExitCode {
    let flag = |name: &str| rest.iter().position(|a| a == name).and_then(|i| rest.get(i + 1)).cloned();
    let result = if rest.first().map(String::as_str) == Some("check") {
        let days = flag("--days").map_or(Ok(30), |d| d.parse::<u32>().map_err(|_| format!("bad --days '{d}'")));
        days.and_then(|d| financial_analysis::forwards::check(&fa.repo, d)).map(|r| {
            print_json(&r);
            exit_on_mismatch(&r)
        })
    } else {
        financial_analysis::forwards::forwards(&fa.repo, flag("--date").as_deref()).map(|f| {
            print_json(&f);
            ExitCode::SUCCESS
        })
    };
    result.unwrap_or_else(|e| {
        eprintln!("fa: {e}");
        ExitCode::from(2)
    })
}

/// `fa verdict TICKER --price P ...`
fn verdict_cmd(fa: &Fa, rest: &[String]) -> financial_analysis::Result<ExitCode> {
    let flag = |name: &str| flag(rest, name);
    let Some(ticker) = rest.first().filter(|t| !t.starts_with('-')) else {
        return Ok(usage_error("verdict needs a ticker"));
    };
    let price = match flag("--price") {
        Some(p) => match p.parse::<f64>() {
            Ok(p) => p,
            Err(_) => return Ok(usage_error("verdict needs --price P (a number)")),
        },
        None => match financial_analysis::market::price_cli::cached_price(&fa.repo, ticker) {
            Ok((p, note)) => {
                eprintln!("fa: {note}");
                p
            }
            Err(e) => return Ok(usage_error(&e)),
        },
    };
    if flag("--price").is_none() {
        if let Some(w) = financial_analysis::sync::old_data_warning(&fa.repo, ticker, &[financial_analysis::sync::dataset::DataSet::Prices]) {
            eprintln!("fa: {w}");
        }
    }
    let parse = |name: &str| -> Result<Option<i64>, String> {
        flag(name).map_or(Ok(None), |v| v.parse::<i64>().map(Some).map_err(|_| format!("bad {name} '{v}'")))
    };
    let (simulations, seed) = match (parse("--simulations"), parse("--seed")) {
        (Ok(s), Ok(d)) => (s, d),
        (Err(m), _) | (_, Err(m)) => return Ok(usage_error(&m)),
    };
    let opts = financial_analysis::VerdictOptions {
        simulations: simulations.map(|s| s.max(0) as u32),
        seed,
        rf_date: flag("--rf-date"),
        rf_base_date: flag("--rf-base-date"),
        rating: flag("--rating"),
    };
    print_json(&fa.verdict(ticker, price, &opts)?);
    Ok(ExitCode::SUCCESS)
}

fn dispatch(fa: &Fa, args: &[String]) -> financial_analysis::Result<ExitCode> {
    match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help") => {
            println!("{USAGE}\n\nMODELS (fa models ...):\n{}", financial_analysis::ratemodels::cli::USAGE);
            Ok(ExitCode::SUCCESS)
        }
        Some("rates") => rates(fa, &args[1..]),
        Some("bonds") => bonds(fa, &args[1..]),
        Some("recon") => Ok(financial_analysis::recon::run(fa, &args[1..])),
        Some("models") => financial_analysis::ratemodels::cli::run(&fa.repo, &args[1..]).map(|_| ExitCode::SUCCESS).or_else(|e| {
            eprintln!("fa: {e}");
            Ok(ExitCode::from(2))
        }),
        Some("forwards") => Ok(forwards_cmd(fa, &args[1..])),
        Some("futures") => Ok(financial_analysis::futures::cli::run(&fa.repo, &args[1..])),
        Some("portfolio") => Ok(ExitCode::from(financial_analysis::portfolio::cli::run(&fa.repo, &args[1..]))),
        Some("backtest") => Ok(ExitCode::from(financial_analysis::dcf::cli::backtest(&fa.repo, &args[1..]))),
        Some("dcf") => Ok(ExitCode::from(financial_analysis::dcf::cli::run(&fa.repo, &args[1..]))),
        Some("options") => match financial_analysis::market::options_cli::run(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("prices") => match financial_analysis::market::price_cli::run(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("splits") => match financial_analysis::market::splits_cli::run(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("estimates") => match financial_analysis::market::estimates_cli::run_estimates(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("earnings") => match financial_analysis::market::estimates_cli::run_earnings(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("vol") => match financial_analysis::volmodels::cli::run(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("macro") => match financial_analysis::macro_test::command(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("verdict") => verdict_cmd(fa, &args[1..]),
        Some("trade") => match args.get(1).map(String::as_str) {
            Some("backtest") => match financial_analysis::trading::cli::backtest(&fa.repo, &args[2..]) {
                Ok(code) => Ok(ExitCode::from(code as u8)),
                Err(e) => Ok(usage_error(&e)),
            },
            Some("fetch") => match financial_analysis::trading::fetch::command(financial_analysis::trading::data::Store::from_env(&fa.repo).dir(), &args[2..]) {
                Ok(code) => Ok(ExitCode::from(code as u8)),
                Err(e) => Ok(usage_error(&e)),
            },
            Some("paper") => match financial_analysis::trading::paper::command(&fa.repo, &args[2..]) {
                Ok(code) => Ok(ExitCode::from(code as u8)),
                Err(e) => Ok(usage_error(&e)),
            },
            _ => Ok(usage_error("trade needs a command: backtest, fetch or paper")),
        },
        Some("sync") => match financial_analysis::sync::command(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("import") => match financial_analysis::company::import_command(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("analyze") => match financial_analysis::analyze::command(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("report") => match financial_analysis::report::command(&fa.repo, &args[1..]) {
            Ok(code) => Ok(ExitCode::from(code as u8)),
            Err(e) => Ok(usage_error(&e)),
        },
        Some("serve") => {
            if args[1..].iter().any(|a| a == "-h" || a == "--help") {
                println!("{}", serve::USAGE);
                return Ok(ExitCode::SUCCESS);
            }
            Ok(serve::serve(fa.clone(), &args[1..]))
        }
        Some(other) => Ok(usage_error(&format!("unknown command '{other}'"))),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&Fa::from_env(), &args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("fa: {e}");
            ExitCode::from(2)
        }
    }
}
