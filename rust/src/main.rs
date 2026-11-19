//! `fa`: command-line access to the FinancialAnalysis pipeline from Rust.
//! Typed commands print pretty JSON; `fa run` passes any Python entry point through.

use financial_analysis::{CheckResult, Curve, Fa, Inflation, RateObservation, Snapshot, Verdict};
use serde::Serialize;
use std::collections::BTreeMap;
use std::process::ExitCode;

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
  fa verdict TICKER --price P [--rf-date D|today] [--rf-base-date D] [--simulations N] [--seed S]
  fa run MODULE [ARGS...]                         any entry point, e.g. fa run scripts.analyze IBM

D is YYYY-MM-DD or 'today'. Rates are decimals (0.0507 = 5.07%).

ENVIRONMENT:
  FA_REPO    repository root (default: the repository that holds this crate)
  FA_PYTHON  Python interpreter (default: <repo>/.venv/bin/python, else python3)";

const RATES: &str = "financial_analysis.rates";

fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).expect("serializable"));
}

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("fa: {msg}\n\n{USAGE}");
    ExitCode::from(64)
}

fn rates(fa: &Fa, rest: &[String]) -> financial_analysis::Result<ExitCode> {
    let mut args = rest.to_vec();
    let sub = match rest.first().map(String::as_str) {
        None => {
            args.insert(0, "snapshot".into());
            "snapshot"
        }
        Some(s) if s.starts_with('-') => {
            args.insert(0, "snapshot".into());
            "snapshot"
        }
        Some(s) => s,
    };
    match sub {
        "snapshot" => print_json(&fa.call::<Snapshot>(RATES, &args)?),
        "yield" | "nyfed" => print_json(&fa.call::<RateObservation>(RATES, &args)?),
        "curve" => print_json(&fa.call::<Curve>(RATES, &args)?),
        "history" | "fred" => print_json(&fa.call::<BTreeMap<String, f64>>(RATES, &args)?),
        "inflation" => print_json(&fa.call::<Inflation>(RATES, &args)?),
        "check" => {
            let results = fa.call::<Vec<CheckResult>>(RATES, &args)?;
            print_json(&results);
            if results.iter().any(|r| !r.mismatches.is_empty()) {
                return Ok(ExitCode::from(1));
            }
        }
        other => return Ok(usage_error(&format!("unknown rates command '{other}'"))),
    }
    Ok(ExitCode::SUCCESS)
}

fn dispatch(fa: &Fa, args: &[String]) -> financial_analysis::Result<ExitCode> {
    match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help") => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Some("rates") => rates(fa, &args[1..]),
        Some("verdict") => {
            print_json(&fa.call::<Verdict>("scripts.verdict", &args[1..])?);
            Ok(ExitCode::SUCCESS)
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
