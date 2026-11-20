//! `fa vol fit heston|bates --symbol T --date D [--rate R] [--expiry-max DAYS] [--max-spread V] [--starts N] [--seed S]`

use super::base::{iv_errors, CharModel, QuoteSet};
use super::calib::{fit, FitOptions};
use super::chain::{build, Counts, Filters};
use super::models::{Bates, Heston};
use crate::market::option_history::index_info;
use crate::market::options::load_snapshot;
use crate::market::options_cli::rate_fn;
use crate::market::price_cli::cache_dir;
use std::path::Path;
use std::time::Instant;

pub const USAGE: &str = "\
fa vol - volatility models (Fourier pricing, calibration)

USAGE:
  fa vol fit heston|bates --symbol T --date D [--rate R] [--expiry-max DAYS] [--max-spread V] [--starts N] [--seed S]
                          fit the model to the stored snapshot data/raw/<T>/options/<D>.json (real bid and ask).
                          Series = root and expiry; forward from put-call parity; out-of-the-money quotes with
                          bid > 0, ask > bid and a spread of at most 1 vol point (5 for VIX); loss weight 1 / spread.
                          Prints the parameters, the Feller ratio, the error by expiry in vol points, and the time.
                          --rate R is one flat rate (decimal); default: the Treasury par curve of the date.
                          --max-spread V is the largest spread in volatility (0.01 = 1 vol point).
                          Stock and ETF options are American: the fit is an approximation for them.";

fn parse_args(rest: &[String]) -> Result<(String, Vec<(String, String)>), String> {
    let model = rest.first().cloned().ok_or("give a model: heston or bates")?;
    let mut flags = Vec::new();
    let mut i = 1;
    while i < rest.len() {
        let k = &rest[i];
        if !["--symbol", "--date", "--rate", "--expiry-max", "--max-spread", "--starts", "--seed"].contains(&k.as_str()) {
            return Err(format!("unknown argument {k}; see fa vol --help"));
        }
        let v = rest.get(i + 1).ok_or_else(|| format!("{k} needs a value"))?;
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
    let get = |n: &str| flags.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    let symbol = get("--symbol").ok_or("give --symbol T")?.to_uppercase();
    let date = get("--date").ok_or("give --date D")?;
    let flat: Option<f64> = num(&flags, "--rate")?;
    let expiry_max: Option<i64> = num(&flags, "--expiry-max")?;
    let mut filters = Filters::for_symbol(&symbol, expiry_max);
    if let Some(v) = num::<f64>(&flags, "--max-spread")? {
        filters.max_spread = v;
    }
    let mut opts = FitOptions::default();
    if let Some(n) = num::<usize>(&flags, "--starts")? {
        opts.starts = n;
    }
    if let Some(s) = num::<u64>(&flags, "--seed")? {
        opts.seed = s;
    }
    let snap = load_snapshot(&cache_dir(repo), &symbol, Some(&date))?;
    let mut rate = rate_fn(repo, flat);
    let (set, counts) = build(&snap, &mut rate, &filters)?;
    println!("fa vol fit {model} {symbol} {date}: snapshot of {} quotes, quote kind {}", snap.quotes.len(), snap.quote_kind.as_deref().unwrap_or("bid and ask"));
    print_counts(&counts, &filters);
    if index_info(&symbol).is_none() {
        println!("note: {symbol} options are American. The model is European, so the fit is an approximation (early exercise, dividends).");
    }
    if set.series.is_empty() {
        return Err("no series left after the filters".into());
    }
    let start = Instant::now();
    match model.as_str() {
        "heston" => {
            let r = fit::<Heston>(&set, &opts);
            report(&set, &r.model, r.loss, r.iterations, start, &opts);
        }
        "bates" => {
            let r = fit::<Bates>(&set, &opts);
            report(&set, &r.model, r.loss, r.iterations, start, &opts);
        }
        other => return Err(format!("unknown model '{other}': heston or bates")),
    }
    Ok(0)
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
        (c.short_series, format!("series with fewer than {} quotes left", super::chain::MIN_QUOTES)),
    ] {
        println!("  {n:>7}  {why}");
    }
}

fn report<M: CharModel>(set: &QuoteSet, m: &M, loss: f64, iterations: usize, start: Instant, opts: &FitOptions) {
    let secs = start.elapsed().as_secs_f64();
    let p = m.to_vec();
    println!("\nparameters ({} starts, seed {}, LM iterations {}, loss {:.3} in squared spreads for {} quotes):", opts.starts, opts.seed, iterations, loss, set.n_quotes());
    for (n, v) in M::names().iter().zip(&p) {
        println!("  {n:<8} {v:>10.5}");
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
