//! The `fa prices` commands: get, show, last, check.

use super::price_cache::{self, Options, Run, Status};
use super::price_check;
use super::price_source::{self, FetchError, PriceSource};
use super::today_iso;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const USAGE: &str = "\
fa prices - daily price history of stocks and ETFs, cached in data/raw/<TICKER>/prices.json

USAGE:
  fa prices get TICKER... [--tickers-file F] [--source S] [--full]
                              get or update the history (one request each 2 seconds for each site);
                              S is tiingo (needs TIINGO_API_KEY), alpaca (needs the two Alpaca keys; many tickers in one
                              request), yahoo or nasdaq; without --source: the source of the cache first, else tiingo and
                              alpaca (with keys), yahoo, nasdaq.
                              --full gets the whole history again. A stopped run continues when you run it again.
  fa prices intraday SYMBOL [--interval 1m|5m] [--from D] [--to D] [--feed iex|sip]
                              1-minute or 5-minute bars of Alpaca (needs APCA_API_KEY_ID and APCA_API_SECRET_KEY) into
                              data/raw/trading/<SYMBOL>_<interval>.csv; a stopped run continues when you run it again
  fa prices show TICKER [--from D] [--to D]       rows of the cache (default: the last 15)
  fa prices last TICKER...                        last close, its date and source; warns when older than 5 weekdays
  fa prices check TICKER... [--sources A,B] [--tolerance T]
                              compare the split adjusted closes of two sources on their common dates
                              (default: yahoo,nasdaq; T = 0.001 is 0.1%). Exit 1 when a date is above T.

Files: prices.json (date, open, high, low, close, volume) and prices.meta.json (source, adjusted, updated).
The tickers-file has one ticker at the start of each line; # starts a comment (rust/mapcheck/universe/sp500.txt).";

pub fn cache_dir(repo: &Path) -> PathBuf {
    repo.join("data").join("raw")
}

/// The price of a command that takes `--price` when the person gives none: the last close of the
/// cache. Gives the price and the text that says which price and date the command used.
pub fn cached_price(repo: &Path, ticker: &str) -> Result<(f64, String), String> {
    let t = ticker.to_uppercase();
    let l = price_cache::last_close(&cache_dir(repo), &t).ok_or_else(|| format!("no price for {t}: give --price P, or run: fa prices get {t}"))?;
    let age = price_cache::weekdays_between(&l.date, &today_iso()).unwrap_or(0);
    let warn = if age > price_cache::STALE_WEEKDAYS { format!(" WARNING: {age} weekdays old; run: fa prices get {t}") } else { String::new() };
    Ok((l.close, format!("price ${:.2}: last close of {} ({}, data/raw/{t}/prices.json){warn}", l.close, l.date, l.source)))
}

#[derive(Debug)]
struct Parsed {
    tickers: Vec<String>,
    flags: Vec<(String, String)>,
    switches: Vec<String>,
}

fn parse(rest: &[String], values: &[&str], switches: &[&str]) -> Result<Parsed, String> {
    let mut p = Parsed { tickers: Vec::new(), flags: Vec::new(), switches: Vec::new() };
    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        if values.contains(&a.as_str()) {
            let v = rest.get(i + 1).filter(|v| !v.starts_with("--")).ok_or_else(|| format!("{a} needs a value"))?;
            p.flags.push((a.clone(), v.clone()));
            i += 2;
        } else if switches.contains(&a.as_str()) {
            p.switches.push(a.clone());
            i += 1;
        } else if a.starts_with("--") {
            return Err(format!("unknown option {a}; see fa prices --help"));
        } else {
            let t = a.trim().to_uppercase();
            if !t.is_empty() && !p.tickers.contains(&t) {
                p.tickers.push(t);
            }
            i += 1;
        }
    }
    Ok(p)
}

impl Parsed {
    fn flag(&self, name: &str) -> Option<&str> {
        self.flags.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
}

/// The tickers of a file: the first word of each line that is not empty and not a comment.
pub fn tickers_from_text(text: &str) -> Vec<String> {
    text.lines().filter_map(|l| l.split('#').next()).filter_map(|l| l.split_whitespace().next()).map(|t| t.to_uppercase()).collect()
}

fn valid_symbol(t: &str) -> bool {
    !t.is_empty() && t.len() <= 12 && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '^')
}

fn collect_tickers(p: &mut Parsed) -> Result<(), String> {
    if let Some(f) = p.flag("--tickers-file") {
        let text = std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?;
        for t in tickers_from_text(&text) {
            if !p.tickers.contains(&t) {
                p.tickers.push(t);
            }
        }
    }
    if p.tickers.is_empty() {
        return Err("give at least one ticker (or --tickers-file F)".into());
    }
    if let Some(bad) = p.tickers.iter().find(|t| !valid_symbol(t)) {
        return Err(format!("'{bad}' is not a ticker symbol"));
    }
    Ok(())
}

fn get(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let mut p = parse(rest, &["--tickers-file", "--source"], &["--full"])?;
    collect_tickers(&mut p)?;
    let sources = price_source::sources(repo, p.flag("--source"))?;
    let opt = Options { today: today_iso(), full: p.switches.iter().any(|s| s == "--full"), forced: p.flag("--source").is_some(), retry_wait: Duration::from_secs(60) };
    let cache = cache_dir(repo);
    let names: Vec<&str> = sources.iter().map(|s| s.name()).collect();
    eprintln!("fa: prices for {} ticker(s) from {} into {}", p.tickers.len(), names.join(", "), cache.display());
    let (mut run, started) = (Run::default(), Instant::now());
    let (mut updated, mut current, mut failed): (Vec<String>, Vec<String>, Vec<(String, String)>) = (vec![], vec![], vec![]);
    let n = p.tickers.len();
    let mut stopped_at = None;
    let mut handle = |i: usize, t: &str, status: &Status| match status {
        Status::Current => {
            eprintln!("[{}/{n}] {t}: current", i + 1);
            current.push(t.to_string());
        }
        Status::Updated { source, added, full, note } => {
            eprintln!("[{}/{n}] {t}: {source} {} +{added} rows ({note})", i + 1, if *full { "full history" } else { "update" });
            updated.push(t.to_string());
        }
        Status::Failed(why) => {
            eprintln!("[{}/{n}] {t}: FAILED {why}", i + 1);
            failed.push((t.to_string(), why.clone()));
        }
    };
    if sources.len() == 1 && sources[0].batch_size() > 1 {
        // A source with multi-symbol requests (Alpaca): the tickers go in groups, one request for each group and page.
        price_cache::update_many(&cache, &p.tickers, sources[0].as_ref(), &opt, &mut run, |i, st| handle(i, &p.tickers[i], st));
    } else {
        for (i, t) in p.tickers.iter().enumerate() {
            if run.blocked.len() >= sources.len() {
                stopped_at = Some(i);
                break;
            }
            let status = price_cache::update_ticker(&cache, t, &sources, &opt, &mut run);
            handle(i, t, &status);
        }
    }
    println!("prices: {} updated, {} current, {} failed, {} not tried; {:.0} s", updated.len(), current.len(), failed.len(), stopped_at.map_or(0, |i| n - i), started.elapsed().as_secs_f64());
    for (t, why) in failed.iter().take(40) {
        println!("  failed {t}: {why}");
    }
    if failed.len() > 40 {
        println!("  ... {} more failures", failed.len() - 40);
    }
    if let Some(i) = stopped_at {
        println!("  stopped: {} answered HTTP 429 twice. Run the same command later to continue from {}.", run.blocked.iter().cloned().collect::<Vec<_>>().join(", "), p.tickers[i]);
    }
    Ok(if failed.is_empty() && stopped_at.is_none() { 0 } else { 1 })
}

fn show(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let p = parse(rest, &["--from", "--to"], &[])?;
    let [t] = p.tickers.as_slice() else { return Err("show takes one ticker".into()) };
    let cache = cache_dir(repo);
    let rows = price_cache::history(&cache, t).ok_or_else(|| format!("no price cache for {t}: run fa prices get {t}"))?;
    let mut out = String::new();
    let meta = std::fs::read_to_string(price_cache::meta_path(&cache, t)).ok().and_then(|m| serde_json::from_str::<price_cache::Meta>(&m).ok());
    match price_cache::last_close(&cache, t) {
        Some(l) => {
            out += &format!("{t}: {} rows, last close {} on {}, source {}", rows.len(), l.close, l.date, l.source);
            if let Some(m) = meta.filter(|m| m.last_date == l.date) {
                out += &format!(", adjusted for {}, updated {}", m.adjusted, m.updated);
            }
            out.push('\n');
        }
        None => out += &format!("{t}: no valid close\n"),
    }
    let (from, to) = (p.flag("--from"), p.flag("--to"));
    let mut sel: Vec<&super::prices::PricePoint> = rows.iter().filter(|r| from.is_none_or(|f| r.date.as_str() >= f) && to.is_none_or(|x| r.date.as_str() <= x)).collect();
    if from.is_none() && to.is_none() && sel.len() > 15 {
        sel = sel.split_off(sel.len() - 15);
    }
    out += &format!("{:<12}{:>12}{:>12}{:>12}{:>12}{:>14}\n", "date", "open", "high", "low", "close", "volume");
    for r in sel {
        out += &format!("{:<12}{:>12.4}{:>12.4}{:>12.4}{:>12.4}{:>14}\n", r.date, r.open, r.high, r.low, r.close, r.volume);
    }
    use std::io::Write;
    let _ = std::io::stdout().write_all(out.as_bytes()); // a closed pipe (| head) is not an error
    Ok(0)
}

fn last(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let mut p = parse(rest, &["--tickers-file"], &[])?;
    collect_tickers(&mut p)?;
    let (cache, today) = (cache_dir(repo), today_iso());
    let mut missing = 0;
    for t in &p.tickers {
        match price_cache::last_close(&cache, t) {
            Some(l) => {
                let age = price_cache::weekdays_between(&l.date, &today).unwrap_or(0);
                let warn = if age > price_cache::STALE_WEEKDAYS { format!("  WARNING: {age} weekdays old") } else { String::new() };
                println!("{t:<8} {:>12.4}  {}  {}{warn}", l.close, l.date, l.source);
            }
            None => {
                missing += 1;
                println!("{t:<8} no price in the cache");
            }
        }
    }
    Ok(if missing > 0 { 1 } else { 0 })
}

fn check(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let mut p = parse(rest, &["--tickers-file", "--sources", "--tolerance"], &[])?;
    collect_tickers(&mut p)?;
    let tolerance: f64 = p.flag("--tolerance").map_or(Ok(0.001), |t| t.parse().map_err(|_| format!("bad --tolerance '{t}'")))?;
    let names: Vec<&str> = p.flag("--sources").unwrap_or("yahoo,nasdaq").split(',').collect();
    let [a_name, b_name] = names.as_slice() else { return Err("--sources takes two names, for example yahoo,nasdaq".into()) };
    if a_name == b_name {
        return Err("--sources needs two different sources".into());
    }
    let mut pair: Vec<Box<dyn PriceSource>> = Vec::new();
    for n in [a_name, b_name] {
        pair.extend(price_source::sources(repo, Some(n))?);
    }
    let (a, b) = (&pair[0], &pair[1]);
    let today = today_iso();
    let mut bad = 0;
    for t in &p.tickers {
        let mut rows = Vec::new();
        for s in [a, b] {
            match s.fetch(t, None, &today) {
                Ok(r) if !r.is_empty() => rows.push(r),
                Ok(_) => return report_gap(t, s.name(), "no rows", bad),
                Err(FetchError::RateLimited(m)) => {
                    println!("{t}: {} answered HTTP 429 ({m}). The check stops; it does not get around the limit.", s.name());
                    return Ok(2);
                }
                Err(FetchError::Failed(m)) => return report_gap(t, s.name(), &m, bad),
            }
        }
        let c = price_check::compare(a.name(), &rows[0], b.name(), &rows[1], tolerance);
        let events = vec![(a.name().to_string(), price_check::check_events(&rows[0])), (b.name().to_string(), price_check::check_events(&rows[1]))];
        println!("{t}\n{}", price_check::render(&c, &events));
        if !c.above.is_empty() {
            bad += 1;
        }
    }
    Ok(if bad > 0 { 1 } else { 0 })
}

fn report_gap(t: &str, source: &str, why: &str, bad: i32) -> Result<i32, String> {
    println!("{t}: {source}: {why}");
    Ok(if bad > 0 { 1 } else { 2 })
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(0)
        }
        Some("get") => get(repo, &args[1..]),
        Some("show") => show(repo, &args[1..]),
        Some("last") => last(repo, &args[1..]),
        Some("check") => check(repo, &args[1..]),
        Some("intraday") => super::intraday::command(repo, &args[1..]),
        Some(other) => Err(format!("unknown prices command '{other}'; see fa prices --help")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn ticker_file_format_of_the_universe() {
        let text = "# S&P 500\n# source\nBRK-B 1067983  # Financials\n\nAAPL 320193 # IT\n";
        assert_eq!(tickers_from_text(text), ["BRK-B", "AAPL"]);
    }

    #[test]
    fn arguments() {
        let p = parse(&v(&["ibm", "--source", "nasdaq", "msft", "--full", "IBM"]), &["--source"], &["--full"]).unwrap();
        assert_eq!(p.tickers, ["IBM", "MSFT"]);
        assert_eq!(p.flag("--source"), Some("nasdaq"));
        assert_eq!(p.switches, ["--full"]);
        assert!(parse(&v(&["IBM", "--bad"]), &[], &[]).is_err());
        assert!(parse(&v(&["IBM", "--source"]), &["--source"], &[]).unwrap_err().contains("needs a value"));
        assert!(!valid_symbol("a b") && valid_symbol("BRK-B") && !valid_symbol("../x"));
    }

    #[test]
    fn price_from_the_cache_says_which_price_it_used() {
        let dir = std::env::temp_dir().join(format!("fa_pcli_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(cached_price(&dir, "IBM").unwrap_err().contains("fa prices get IBM"));
        let raw = dir.join("data/raw/IBM");
        std::fs::create_dir_all(&raw).unwrap();
        std::fs::write(raw.join("prices.json"), r#"[{"date":"2020-01-02","open":1,"high":1,"low":1,"close":1.5,"volume":1}]"#).unwrap();
        let (p, note) = cached_price(&dir, "ibm").unwrap();
        assert_eq!(p, 1.5);
        assert!(note.contains("last close of 2020-01-02") && note.contains("WARNING"), "{note}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
