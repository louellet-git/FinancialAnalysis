//! mapcheck command line. See rust/mapcheck/README.md.

use mapcheck::dates;
use mapcheck::registry::Registry;
use mapcheck::run::{check_all, report_json, values_json, Status};
use mapcheck::summary::{detail, listing, summary};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

const HELP: &str = "\
mapcheck: test the mapping from XBRL tags to the standard DCF input variables.

USAGE
    mapcheck [OPTIONS] [TICKER ...]

Reads <cache>/<TICKER>/company_facts.json (or .json.gz). No network.
With no ticker and no --tickers-file, it runs all tickers in the cache.

OPTIONS
    --cache DIR          cache directory (default: data/raw)
    --tickers-file FILE  one ticker for each line; '#' starts a comment
    --periods N          fiscal years and quarters for each ticker (default: 12)
    --as-of YYYY-MM-DD   date for the freshness check (default: today)
    --jobs N             threads (default: all processors)
    --list               print one line for each ticker
    --detail             print all findings for each ticker
    --top N              rows in the tag table (default: 25)
    --out FILE           write the report, one JSON line for each ticker
    --emit FILE          write the mapped values, one JSON line for each period
                         (for scripts/mapping_parity.py)
    --strict             exit code 1 if a ticker has status FAIL or ERROR
    --registry FILE      use this registry file, not the built-in one
    -h, --help           print this text
";

struct Options {
    cache: PathBuf,
    tickers: Vec<String>,
    tickers_file: Option<PathBuf>,
    periods: usize,
    as_of: i64,
    jobs: Option<usize>,
    list: bool,
    detail: bool,
    top: usize,
    out: Option<PathBuf>,
    emit: Option<PathBuf>,
    strict: bool,
    registry: Option<PathBuf>,
}

fn parse_args() -> Result<Options, String> {
    let mut o = Options {
        cache: PathBuf::from("data/raw"),
        tickers: Vec::new(),
        tickers_file: None,
        periods: 12,
        as_of: dates::today(),
        jobs: None,
        list: false,
        detail: false,
        top: 25,
        out: None,
        emit: None,
        strict: false,
        registry: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "-h" | "--help" => return Err(String::new()),
            "--cache" => o.cache = PathBuf::from(value("--cache")?),
            "--tickers-file" => o.tickers_file = Some(PathBuf::from(value("--tickers-file")?)),
            "--periods" => o.periods = value("--periods")?.parse().map_err(|_| "--periods needs a number".to_string())?,
            "--as-of" => {
                let text = value("--as-of")?;
                o.as_of = dates::parse(&text).ok_or_else(|| format!("--as-of '{text}' is not YYYY-MM-DD"))?;
            }
            "--jobs" => o.jobs = Some(value("--jobs")?.parse().map_err(|_| "--jobs needs a number".to_string())?),
            "--list" => o.list = true,
            "--detail" => o.detail = true,
            "--top" => o.top = value("--top")?.parse().map_err(|_| "--top needs a number".to_string())?,
            "--out" => o.out = Some(PathBuf::from(value("--out")?)),
            "--emit" => o.emit = Some(PathBuf::from(value("--emit")?)),
            "--strict" => o.strict = true,
            "--registry" => o.registry = Some(PathBuf::from(value("--registry")?)),
            other if other.starts_with('-') => return Err(format!("unknown option '{other}'")),
            ticker => o.tickers.push(ticker.to_uppercase()),
        }
    }
    Ok(o)
}

fn tickers_from_file(path: &PathBuf) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim().to_uppercase())
        .filter(|l| !l.is_empty())
        .collect())
}

fn tickers_in_cache(cache: &PathBuf) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(cache).map_err(|e| format!("{}: {e}", cache.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if mapcheck::run::facts_path(cache, &name).is_some() {
            out.push(name);
        }
    }
    out.sort();
    Ok(out)
}

fn run() -> Result<bool, String> {
    let o = match parse_args() {
        Ok(o) => o,
        Err(message) if message.is_empty() => {
            print!("{HELP}");
            return Ok(true);
        }
        Err(message) => return Err(format!("{message}\n\n{HELP}")),
    };
    let registry = match &o.registry {
        Some(path) => Registry::load(&fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)?,
        None => Registry::embedded()?,
    };
    let mut tickers = o.tickers.clone();
    if let Some(path) = &o.tickers_file {
        tickers.extend(tickers_from_file(path)?);
    }
    if tickers.is_empty() {
        tickers = tickers_in_cache(&o.cache)?;
    }
    let mut seen = std::collections::HashSet::new();
    tickers.retain(|t| seen.insert(t.clone()));
    if tickers.is_empty() {
        return Err(format!("no tickers (the cache {} is empty)", o.cache.display()));
    }
    if let Some(jobs) = o.jobs {
        rayon::ThreadPoolBuilder::new().num_threads(jobs).build_global().map_err(|e| e.to_string())?;
    }

    let start = Instant::now();
    let reports = check_all(&registry, &o.cache, &tickers, o.periods, o.as_of, o.emit.is_some());
    let seconds = start.elapsed().as_secs_f64();

    if o.list {
        print!("{}", listing(&reports));
        println!();
    }
    if o.detail {
        for r in &reports {
            print!("{}", detail(&registry, r));
        }
        println!();
    }
    print!("{}", summary(&registry, &reports, seconds, rayon::current_num_threads(), o.top));

    if let Some(path) = &o.out {
        let mut w = BufWriter::new(fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?);
        for r in &reports {
            writeln!(w, "{}", report_json(r)).map_err(|e| e.to_string())?;
        }
        println!("\nReport: {}", path.display());
    }
    if let Some(path) = &o.emit {
        let mut w = BufWriter::new(fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?);
        for r in &reports {
            for line in values_json(&registry, r) {
                writeln!(w, "{line}").map_err(|e| e.to_string())?;
            }
        }
        println!("Values: {}", path.display());
    }
    let bad = reports.iter().any(|r| matches!(r.status, Status::Fail | Status::Error));
    Ok(!(o.strict && bad))
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(message) => {
            eprintln!("mapcheck: {message}");
            ExitCode::from(2)
        }
    }
}
