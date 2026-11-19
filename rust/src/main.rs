//! `fa`: command-line access to the FinancialAnalysis pipeline from Rust.
//! Typed commands print pretty JSON; `fa run` passes any Python entry point through.

use financial_analysis::{
    CheckResult, CreditSnapshot, Curve, Fa, HqmCurve, Inflation, RateObservation, Snapshot, Verdict,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::process::ExitCode;

mod serve;

const USAGE: &str = "\
fa - Rust interface to the FinancialAnalysis Python pipeline

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
  fa verdict TICKER --price P [--rf-date D|today] [--rf-base-date D] [--rating A-]
             [--simulations N] [--seed S]
  fa trade fetch [--symbol S]... [--interval I]...  download the bars of the option bots (Yahoo) into the CSV cache
  fa trade backtest [--bot ID]... [--max-risk N] [--out FILE] [-v]   backtest the option bots (fa trade backtest --help)
  fa trade paper [BOT]... [--no-refresh] [--no-live]   forward paper trading, live Cboe option chain (fa trade paper --help)
  fa run MODULE [ARGS...]                         any entry point, e.g. fa run scripts.analyze IBM
  fa serve [--port N] [--no-open]                 local viewer in the browser (http://127.0.0.1:8765/)

D is YYYY-MM-DD or 'today'. Rates are decimals (0.0507 = 5.07%).

ENVIRONMENT:
  FA_REPO    repository root (default: the repository that holds this crate)
  FA_PYTHON  Python interpreter (default: <repo>/.venv/bin/python, else python3;
             Windows: <repo>\\.venv\\Scripts\\python.exe, else python)";

const RATES: &str = "financial_analysis.rates";
const BONDS: &str = "financial_analysis.bonds";

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

fn bonds(fa: &Fa, rest: &[String]) -> financial_analysis::Result<ExitCode> {
    let (args, sub) = with_default_sub(rest);
    match sub.as_str() {
        "snapshot" => print_json(&fa.call::<CreditSnapshot>(BONDS, &args)?),
        "spread" | "yield" | "over10y" | "moodys" => print_json(&fa.call::<RateObservation>(BONDS, &args)?),
        "hqm" => print_json(&fa.call::<HqmCurve>(BONDS, &args)?),
        "check" => {
            let results = fa.call::<Vec<CheckResult>>(BONDS, &args)?;
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
    match sub.as_str() {
        "snapshot" => print_json(&fa.call::<Snapshot>(RATES, &args)?),
        "yield" | "nyfed" => print_json(&fa.call::<RateObservation>(RATES, &args)?),
        "curve" => print_json(&fa.call::<Curve>(RATES, &args)?),
        "history" | "fred" => print_json(&fa.call::<BTreeMap<String, f64>>(RATES, &args)?),
        "inflation" => print_json(&fa.call::<Inflation>(RATES, &args)?),
        "check" => {
            let results = fa.call::<Vec<CheckResult>>(RATES, &args)?;
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

fn dispatch(fa: &Fa, args: &[String]) -> financial_analysis::Result<ExitCode> {
    match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help") => {
            println!("{USAGE}\n\nMODELS (fa models ...):\n{}", financial_analysis::ratemodels::cli::USAGE);
            Ok(ExitCode::SUCCESS)
        }
        Some("rates") => rates(fa, &args[1..]),
        Some("bonds") => bonds(fa, &args[1..]),
        Some("models") => financial_analysis::ratemodels::cli::run(&fa.repo, &args[1..]).map(|_| ExitCode::SUCCESS).or_else(|e| {
            eprintln!("fa: {e}");
            Ok(ExitCode::from(2))
        }),
        Some("forwards") => Ok(forwards_cmd(fa, &args[1..])),
        Some("futures") => Ok(financial_analysis::futures::cli::run(&fa.repo, &args[1..])),
        Some("verdict") => {
            print_json(&fa.call::<Verdict>("scripts.verdict", &args[1..])?);
            Ok(ExitCode::SUCCESS)
        }
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
        Some("serve") => {
            if args[1..].iter().any(|a| a == "-h" || a == "--help") {
                println!("{}", serve::USAGE);
                return Ok(ExitCode::SUCCESS);
            }
            Ok(serve::serve(fa.clone(), &args[1..]))
        }
        Some("run") => match args.get(1) {
            None => Ok(usage_error("run needs a module, e.g. scripts.analyze")),
            Some(module) => {
                let status = fa.run(module, &args[2..])?;
                Ok(ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8))
            }
        },
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
