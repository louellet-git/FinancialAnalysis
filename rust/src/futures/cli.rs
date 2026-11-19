//! The `fa futures` commands.

use super::{cftc, check, facts, mpt, Result};
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;


fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).expect("serializable"));
}

/// The arguments of one command: a positional text and the flags that the command knows.
#[derive(Debug)]
struct Args {
    positional: Option<String>,
    flags: Vec<(String, String)>,
}

impl Args {
    fn new(rest: &[String], allowed: &[&str], needs_positional: Option<&str>) -> Result<Args> {
        let mut positional = None;
        let mut flags = Vec::new();
        let mut i = 0;
        while i < rest.len() {
            let a = &rest[i];
            if a.starts_with("--") {
                if !allowed.contains(&a.as_str()) {
                    return Err(format!("unknown option {a}; this command takes: {}", allowed.join(" ")));
                }
                let v = rest.get(i + 1).filter(|v| !v.starts_with("--")).ok_or_else(|| format!("{a} needs a value"))?;
                flags.push((a.clone(), v.clone()));
                i += 2;
            } else if positional.is_none() {
                positional = Some(a.clone());
                i += 1;
            } else {
                return Err(format!("unexpected argument '{a}'"));
            }
        }
        if let (Some(name), None) = (needs_positional, &positional) {
            return Err(format!("the command needs {name}"));
        }
        Ok(Args { positional, flags })
    }

    fn flag(&self, name: &str) -> Option<&str> {
        self.flags.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    fn pos(&self) -> &str {
        self.positional.as_deref().unwrap_or("")
    }
}

fn dispatch(repo: &Path, rest: &[String]) -> Result<ExitCode> {
    let sub = rest.first().map(String::as_str).ok_or("fa futures needs a command")?;
    let tail = &rest[1..];
    match sub {
        "roots" => {
            Args::new(tail, &[], None)?;
            print_json(&facts::ROOTS);
        }
        "quote" => {
            let a = Args::new(tail, &["--date"], Some("a symbol, for example ESZ26"))?;
            print_json(&super::quote(repo, a.pos(), a.flag("--date"))?);
        }
        "history" => {
            let a = Args::new(tail, &["--start", "--end"], Some("a symbol, for example ESZ26"))?;
            let start = a.flag("--start").ok_or("history needs --start D")?;
            print_json(&super::history(repo, a.pos(), start, a.flag("--end"))?);
        }
        "curve" => {
            let a = Args::new(tail, &["--date"], Some("a root, for example SR3"))?;
            print_json(&super::curve(repo, a.pos(), a.flag("--date"))?);
        }
        "positions" => {
            let a = Args::new(tail, &["--weeks"], Some("a root, for example ES"))?;
            let weeks: u32 = a.flag("--weeks").map_or(Ok(4), |w| w.parse().map_err(|_| format!("bad --weeks '{w}'")))?;
            if !(1..=520).contains(&weeks) {
                return Err("--weeks is from 1 to 520".into());
            }
            print_json(&cftc::positions(repo, facts::root_or_error(a.pos())?, weeks)?);
        }
        "policy-path" => {
            let a = Args::new(tail, &["--date"], None)?;
            print_json(&mpt::policy_path(repo, a.flag("--date"))?);
        }
        "check" => {
            let a = Args::new(tail, &["--date"], None)?;
            let results = check::run(repo, a.flag("--date"))?;
            print_json(&results);
            if results.iter().any(|r| !r.mismatches.is_empty()) {
                return Ok(ExitCode::from(1));
            }
        }
        other => return Err(format!("unknown futures command '{other}'; see fa help")),
    }
    Ok(ExitCode::SUCCESS)
}

pub fn run(repo: &Path, rest: &[String]) -> ExitCode {
    dispatch(repo, rest).unwrap_or_else(|e| {
        eprintln!("fa: {e}");
        ExitCode::from(2)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn arguments() {
        let a = Args::new(&v(&["ESZ26", "--date", "2026-11-18"]), &["--date"], Some("x")).unwrap();
        assert_eq!((a.pos(), a.flag("--date")), ("ESZ26", Some("2026-11-18")));
        assert!(Args::new(&v(&[]), &["--date"], Some("a symbol")).unwrap_err().contains("needs a symbol"));
        assert!(Args::new(&v(&["ES", "--bad", "1"]), &["--date"], None).unwrap_err().contains("unknown option"));
        assert!(Args::new(&v(&["ES", "--date"]), &["--date"], None).unwrap_err().contains("needs a value"));
        assert!(Args::new(&v(&["ES", "NQ"]), &[], None).unwrap_err().contains("unexpected"));
    }
}
