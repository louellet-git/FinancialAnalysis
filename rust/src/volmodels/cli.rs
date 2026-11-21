//! `fa vol fit heston|bates --symbol T --date D [--time HHMM] [--rate R] [--expiry-max DAYS] [--max-spread V] [--starts N] [--seed S]`

use super::base::{iv_errors, CharModel, QuoteSet};
use super::calib::{at_bound, fit, FitOptions, FitResult};
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
    let model = rest.first().cloned().ok_or("give a model: heston or bates")?;
    let mut flags = Vec::new();
    let mut i = 1;
    while i < rest.len() {
        let k = &rest[i];
        if !["--symbol", "--date", "--time", "--rate", "--expiry-max", "--max-spread", "--starts", "--seed"].contains(&k.as_str()) {
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
    if model != "heston" && model != "bates" {
        return Err(format!("unknown model '{model}': heston or bates"));
    }
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
    if model == "heston" {
        let r = fit::<Heston>(&set, &opts)?;
        report(&set, &r, start, &opts);
    } else {
        let r = fit::<Bates>(&set, &opts)?;
        report(&set, &r, start, &opts);
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
    println!("  status of the best start: {}", r.status.text());
    let (lo, hi) = r.refined_losses.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, l| (a.0.min(*l), a.1.max(*l)));
    println!("  refined starts: {} (after the first iterations, losses {lo:.3} to {hi:.3}; {})", r.refined_losses.len(), r.statuses.iter().map(|s| s.text().split(':').next().unwrap_or("").to_string()).collect::<Vec<_>>().join(", "));
    if marks.iter().any(|b| *b) {
        println!("  WARNING: a parameter is at a bound. The fit is not identified by these quotes; do not read the parameters.");
    }
    if hi - lo > 0.01 * lo.abs().max(1e-9) {
        println!("  WARNING: the refined starts end at different losses. The fit is not unique; try --seed or --starts.");
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
