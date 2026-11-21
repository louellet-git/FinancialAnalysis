//! `fa vol fit heston|bates|sabr --symbol T --date D [--time HHMM] [--rate R] [--expiry-max DAYS] [--max-spread V] [--starts N] [--seed S] [--beta B]`

use super::base::{iv_errors, CharModel, QuoteSet};
use super::calib::{at_bound, fit, FitOptions, FitResult, LmStatus};
use super::chain::{build, Counts, Filters};
use super::models::{Bates, Heston};
use crate::market::option_history::index_info;
use crate::market::options::load_snapshot_at;
use crate::market::options_cli::rate_fn;
use crate::market::price_cli::cache_dir;
use std::path::Path;
use std::time::Instant;

pub const USAGE: &str = "\
fa vol - volatility models (Fourier pricing, calibration)

USAGE:
  fa vol fit sabr --symbol T --date D [--time HHMM] [--rate R] [--beta B] [--expiry-max DAYS] [--max-spread V]
                          fit SABR (Hagan et al. 2002 with the Obloj 2008 correction) to each series: alpha, rho, nu with beta fixed
                          (default 1). Loss (model vol - mid vol) / spread in vol. Prints, for each series, days, forward, the
                          parameters, the fit status, the number of quotes, the error in vol points, and the time. One smile for
                          each expiry: SABR has no dynamics between expiries. The parameters are calibrated values, not measurements.
  fa vol fit heston|bates --symbol T --date D [--time HHMM] [--rate R] [--expiry-max DAYS] [--max-spread V] [--starts N] [--seed S]
                          fit the model to the last stored snapshot of the date (data/raw/<T>/options/<D>.json or <D>T<hhmm>.json;
                          real bid and ask). --time HHMM (UTC) takes the file of that minute.
                          Series = root and expiry; forward from put-call parity; out-of-the-money quotes with
                          bid > 0, ask > bid and a spread of at most 1 vol point (5 for VIX); loss weight 1 / spread.
                          Prints the parameters, the Feller ratio, the error by expiry in vol points, and the time.
                          --rate R is one flat rate (decimal); default: the Treasury par curve of the date.
                          --max-spread V is the largest spread in volatility (0.01 = 1 vol point).
                          Stock and ETF options are American: the fit is an approximation for them.
                          --rate R missing: the Treasury par curve of the date is read from the cache, and can be downloaded (network).
                          The fitted parameters are calibrated values of option prices. They are not measurements of the company.
                          Do not use them as an input of a DCF or of a forecast.
                          A parameter marked \"at bound\" (within 0.1% of its range) means that the fit is not identified.";

fn parse_args(rest: &[String]) -> Result<(String, Vec<(String, String)>), String> {
    let model = rest.first().filter(|m| !m.starts_with("--")).cloned().ok_or("give a model: heston or bates")?;
    let mut flags = Vec::new();
    let mut i = 1;
    while i < rest.len() {
        let k = &rest[i];
        if !["--symbol", "--date", "--time", "--rate", "--expiry-max", "--max-spread", "--starts", "--seed", "--beta"].contains(&k.as_str()) {
            return Err(format!("unknown argument {k}; see fa vol --help"));
        }
        let v = rest.get(i + 1).filter(|v| !v.starts_with("--")).ok_or_else(|| format!("{k} needs a value"))?;
        flags.push((k.clone(), v.clone()));
        i += 2;
    }
    Ok((model, flags))
}

fn num<T: std::str::FromStr>(flags: &[(String, String)], name: &str) -> Result<Option<T>, String> {
    flags.iter().find(|(k, _)| k == name).map(|(_, v)| v.parse::<T>().map_err(|_| format!("bad {name} '{v}'"))).transpose()
}

pub fn run(repo: &Path, rest: &[String]) -> Result<i32, String> {
    if rest.is_empty() || rest.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    if rest[0] != "fit" {
        return Err(format!("unknown vol command '{}'; see fa vol --help", rest[0]));
    }
    let (model, flags) = parse_args(&rest[1..])?;
    if !["heston", "bates", "sabr"].contains(&model.as_str()) {
        return Err(format!("unknown model '{model}': heston, bates or sabr"));
    }
    let get = |n: &str| flags.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    let beta = num::<f64>(&flags, "--beta")?;
    if let Some(b) = beta {
        if !(0.0..=1.0).contains(&b) {
            return Err(format!("bad --beta '{}': 0 to 1", get("--beta").unwrap_or_default()));
        }
    }
    if model == "sabr" {
        for n in ["--starts", "--seed"] {
            if get(n).is_some() {
                println!("warning: {n} has no effect with sabr (fixed starts)");
            }
        }
    } else if beta.is_some() {
        println!("warning: --beta has no effect with {model} (it is for sabr)");
    }
    let symbol = get("--symbol").ok_or("give --symbol T")?.to_uppercase();
    let date = get("--date").ok_or("give --date D")?;
    let flat = get("--rate").map(|r| crate::market::options_cli::parse_rate(&r)).transpose()?;
    let expiry_max: Option<i64> = num(&flags, "--expiry-max")?;
    let mut filters = Filters::for_symbol(&symbol, expiry_max);
    if let Some(v) = num::<f64>(&flags, "--max-spread")? {
        if !v.is_finite() || v < 0.0 {
            return Err(format!("bad --max-spread '{}'", get("--max-spread").unwrap_or_default()));
        }
        filters.max_spread = v;
    }
    let mut opts = FitOptions::default();
    if let Some(n) = num::<usize>(&flags, "--starts")? {
        opts.starts = n;
    }
    if let Some(s) = num::<u64>(&flags, "--seed")? {
        opts.seed = s;
    }
    let snap = load_snapshot_at(&cache_dir(repo), &symbol, Some(&date), get("--time").as_deref())?;
    let mut rate = rate_fn(repo, flat);
    let (set, counts) = build(&snap, &mut rate, &filters)?;
    println!("file {}", crate::market::options::stem_of(&cache_dir(repo), &symbol, &date, get("--time").as_deref())?);
    println!("fa vol fit {model} {symbol} {date}: snapshot of {} quotes, quote kind {}", snap.quotes.len(), snap.quote_kind.as_deref().unwrap_or("bid and ask"));
    print_counts(&counts, &filters);
    if index_info(&symbol).is_none() {
        println!("note: {symbol} options are American. The model is European, so the fit is an approximation (early exercise, dividends).");
    }
    if set.series.is_empty() {
        return Err("no series left after the filters".into());
    }
    let start = Instant::now();
    if model == "sabr" {
        report_sabr(&set, beta.unwrap_or(1.0), start);
    } else if model == "heston" {
        let r = fit::<Heston>(&set, &opts)?;
        report(&set, &r, start, &opts);
    } else {
        let r = fit::<Bates>(&set, &opts)?;
        report(&set, &r, start, &opts);
    }
    Ok(0)
}

fn report_sabr(set: &QuoteSet, beta: f64, start: Instant) {
    let fits = super::sabr::fit_all(&set.series, beta, 100);
    let secs = start.elapsed().as_secs_f64();
    println!("\nSABR, beta = {beta} fixed, alpha rho nu fitted for each series (loss: model vol minus mid vol over the spread; error in vol points):");
    println!("  {:<6} {:<11} {:>5} {:>10} {:>9} {:>8} {:>8} {:>5} {:>8} {:>8}  status", "root", "expiry", "days", "forward", "alpha", "rho", "nu", "n", "rms", "max");
    let (mut s2, mut n, mut mx, mut not_conv, mut bound, mut failed, mut poor) = (0.0, 0usize, 0.0f64, 0usize, 0usize, 0usize, 0usize);
    for f in &fits {
        match f {
            Ok(f) => {
                let marks: Vec<&str> = super::sabr::PARAM_NAMES.iter().zip(&f.at_bound).filter(|(_, b)| **b).map(|(n, _)| *n).collect();
                let mut status = f.status.text().to_string();
                if f.status == LmStatus::Converged && f.poor {
                    status = "converged, poor fit".into();
                }
                if !marks.is_empty() {
                    status += &format!("; at bound: {}", marks.join(", "));
                }
                println!("  {:<6} {:<11} {:>5} {:>10.2} {:>9.5} {:>8.4} {:>8.4} {:>5} {:>8.3} {:>8.3}  {status}", f.root, f.expiry, f.days, f.forward, f.model.alpha, f.model.rho, f.model.nu, f.n, f.rms_points, f.max_points);
                s2 += f.rms_points * f.rms_points * f.n as f64;
                n += f.n;
                mx = mx.max(f.max_points);
                not_conv += (f.status != LmStatus::Converged) as usize;
                poor += (f.status == LmStatus::Converged && f.poor) as usize;
                bound += f.at_bound.iter().any(|b| *b) as usize;
            }
            Err(e) => {
                failed += 1;
                println!("  FAILED: {e}");
            }
        }
    }
    println!("  all    {:<11} {:>5} {:>10} {:>9} {:>8} {:>8} {:>5} {:>8.3} {:>8.3}", "", "", "", "", "", "", n, (s2 / n.max(1) as f64).sqrt(), mx);
    println!("series: {} fitted, {} not converged, {} \"converged, poor fit\" (largest error above {} times the median spread), {} with a parameter at a bound (not identified: do not read it), {} failed", fits.len() - failed, not_conv, poor, super::sabr::POOR_FACTOR, bound, failed);
    println!("note: the error is a model vol against a mid vol of an indicative or stored quote: a measurement of the fit, not a tradable price. No dynamics between expiries.");
    println!("\ntime: {secs:.3} s (fit of {} series, {} quotes, {} threads)", set.series.len(), set.n_quotes(), rayon::current_num_threads());
}

fn print_counts(c: &Counts, f: &Filters) {
    println!("series: {} in the file, {} kept (forward from put-call parity, median of the 5 strikes nearest to the parity strike; time = calendar days / 365; the snapshot has no clock time)", c.series_total, c.series_kept);
    println!("quotes: {} total, {} kept, {} dropped:", c.total, c.kept, c.dropped());
    for (n, why) in [
        (c.expired_or_today, "expiry today or earlier".to_string()),
        (c.beyond_expiry_max, "expiry beyond --expiry-max".to_string()),
        (c.no_forward, "series with no forward from parity".to_string()),
        (c.no_bid_or_ask, "no bid or no ask (bid > 0 needed)".to_string()),
        (c.ask_not_above_bid, "ask not above bid".to_string()),
        (c.in_the_money, "in the money (out-of-the-money quotes only)".to_string()),
        (c.no_iv, "no implied volatility for bid, mid or ask".to_string()),
        (c.spread_too_wide, format!("spread above {} vol points", 100.0 * f.max_spread)),
        (c.short_series, format!("quotes of series with fewer than {} quotes left", super::chain::MIN_QUOTES)),
    ] {
        println!("  {n:>7}  {why}");
    }
}

fn report<M: CharModel>(set: &QuoteSet, r: &FitResult<M>, start: Instant, opts: &FitOptions) {
    let (m, loss, iterations) = (&r.model, r.loss, r.iterations);
    let secs = start.elapsed().as_secs_f64();
    let p = m.to_vec();
    println!("\nparameters ({} starts, seed {}, LM iterations {}, loss {:.3} in squared spreads for {} quotes):", opts.starts, opts.seed, iterations, loss, set.n_quotes());
    let marks = at_bound(&p, &M::bounds());
    for ((n, v), b) in M::names().iter().zip(&p).zip(&marks) {
        println!("  {n:<8} {v:>10.5}{}", if *b { "  at bound" } else { "" });
    }
    println!("  status of the best start (final): {}", r.status.text());
    let (lo, hi) = r.refined_losses.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, l| (a.0.min(*l), a.1.max(*l)));
    println!("  refined starts (stage 1, after {} iterations each, not final losses; only the best one goes on): {} (losses {lo:.3} to {hi:.3}; stage 1 status: {})", super::calib::STAGE1_ITER, r.refined_losses.len(), r.statuses.iter().map(|s| s.text().split(':').next().unwrap_or("").to_string()).collect::<Vec<_>>().join(", "));
    println!("  uniqueness: not checked in this run (only the best start goes to the end); compare the final loss of runs with other --seed values");
    if marks.iter().any(|b| *b) {
        println!("  WARNING: a parameter is at a bound. The fit is not identified by these quotes; do not read the parameters.");
    }
    if M::names().len() >= 5 {
        let feller = 2.0 * p[2] * p[1] / (p[3] * p[3]);
        println!("  Feller ratio 2 kappa theta / xi^2 = {feller:.3}{}", if feller < 1.0 { " (under 1: v can reach 0)" } else { "" });
    }
    let errs = iv_errors(m, set);
    println!("\nerror by series, model implied volatility minus market mid (vol points):");
    println!("  {:<6} {:<11} {:>5} {:>5} {:>9} {:>9}", "root", "expiry", "days", "n", "rms", "max");
    let (mut s2, mut n, mut mx) = (0.0, 0usize, 0.0f64);
    for e in &errs {
        let note = if e.no_iv > 0 { format!("  ({} model prices with no volatility)", e.no_iv) } else { String::new() };
        println!("  {:<6} {:<11} {:>5} {:>5} {:>9.3} {:>9.3}{note}", e.root, e.expiry, e.days, e.n, e.rms_points, e.max_points);
        if e.rms_points.is_finite() {
            let k = e.n - e.no_iv;
            s2 += e.rms_points * e.rms_points * k as f64;
            n += k;
        }
        mx = mx.max(e.max_points);
    }
    println!("  all    {:<11} {:>5} {:>5} {:>9.3} {:>9.3}", "", "", n, (s2 / n.max(1) as f64).sqrt(), mx);
    println!("\ntime: {secs:.2} s (fit of {} series, {} quotes, {} threads)", set.series.len(), set.n_quotes(), rayon::current_num_threads());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: &[&str]) -> Vec<String> {
        x.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn vol_fit_lines_are_parsed_as_the_help_says() {
        let (m, f) = parse_args(&v(&["heston", "--symbol", "IBM", "--date", "2026-11-19", "--rate", "0.04", "--starts", "3"])).unwrap();
        assert_eq!(m, "heston");
        assert_eq!((num::<f64>(&f, "--rate").unwrap(), num::<usize>(&f, "--starts").unwrap(), num::<u64>(&f, "--seed").unwrap()), (Some(0.04), Some(3), None));
        assert!(parse_args(&v(&[])).is_err(), "no model");
        assert!(parse_args(&v(&["heston", "--bogus", "1"])).unwrap_err().contains("unknown argument"));
        assert!(parse_args(&v(&["heston", "--date"])).unwrap_err().contains("needs a value"));
        assert!(parse_args(&v(&["heston", "--date", "--rate", "0.04"])).unwrap_err().contains("--date needs a value"));
        assert!(parse_args(&v(&["--symbol", "IBM"])).unwrap_err().contains("give a model"));
        let none = Path::new("/nonexistent");
        let run = |x: &[&str]| run(none, &v(x)).unwrap_err();
        assert!(run(&["fit", "heston", "--date", "2026-11-19"]).contains("--symbol"));
        assert!(run(&["fit", "heston", "--symbol", "IBM"]).contains("--date"));
        assert!(run(&["fit", "heston", "--symbol", "IBM", "--date", "2026-11-19", "--rate", "abc"]).contains("bad --rate"));
        assert!(run(&["fit", "heston", "--symbol", "IBM", "--date", "2026-11-19", "--starts", "x"]).contains("bad --starts"));
        assert!(run(&["fit", "heston", "--symbol", "IBM", "--date", "2026-11-19", "--rate", "inf"]).contains("bad --rate"));
        assert!(run(&["fit", "heston", "--symbol", "IBM", "--date", "2026-11-19", "--max-spread", "nan"]).contains("bad --max-spread"));
        assert!(run(&["fit", "heston", "--symbol", "IBM", "--date", "2026-11-19", "--max-spread", "nan"]).contains("'nan'"), "the raw text");
        assert!(run(&["fit", "heston", "--symbol", "IBM", "--date", "2026-11-19", "--max-spread", "-1"]).contains("bad --max-spread"), "below 0");
        assert!(run(&["fit", "sabr", "--symbol", "IBM", "--date", "2026-11-19", "--beta", "1.5"]).contains("bad --beta"), "checked before the snapshot loads");
        assert!(run(&["fit", "sabr", "--symbol", "IBM", "--date", "2026-11-19", "--beta", "nan"]).contains("bad --beta"));
        assert!(run(&["fit", "merton", "--symbol", "IBM", "--date", "2026-11-19"]).contains("unknown model"));
    }
}
