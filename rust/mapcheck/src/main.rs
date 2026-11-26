//! mapcheck command line. See rust/mapcheck/README.md.

use mapcheck::cli::Args;
use mapcheck::dates;
use mapcheck::http::Client;
use mapcheck::import::{import_ticker, ImportError};
use mapcheck::registry::Registry;
use mapcheck::run::{check_all, check_cached, facts_path, report_json, values_json, Status};
use mapcheck::summary::{detail, listing, summary, values_listing};
use mapcheck::sec;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

const HELP: &str = "\
mapcheck: from SEC data to the standard DCF input variables.

USAGE
    mapcheck [check] [OPTIONS] [TICKER ...]   map and check the tickers in the cache
    mapcheck fetch   [OPTIONS] [TICKER ...]   get SEC company facts into the cache
    mapcheck fetch --edinet --ticker 8001.T --from D --to D   get a Japanese annual report from EDINET
    mapcheck import  [OPTIONS] TICKER ...     write data/processed/<TICKER>.json
    mapcheck values  [OPTIONS] TICKER ...     each variable with its value and source
    mapcheck filing  [OPTIONS] TICKER ...     the statements of the latest 10-K
    mapcheck crosscheck [OPTIONS] TICKER ...  standard variables against the filing

COMMON OPTIONS
    --cache DIR          cache directory (default: data/raw)
    --tickers-file FILE  one ticker for each line; '#' starts a comment
    --periods N          fiscal years and quarters for each ticker (default: 12)
    -h, --help           print this text

check (no network; with no ticker it runs all tickers in the cache)
    --as-of YYYY-MM-DD   date for the freshness check (default: today)
    --jobs N             threads (default: all processors)
    --list               print one line for each ticker
    --detail             print all findings for each ticker
    --top N              rows in the tag table (default: 25)
    --out FILE           write the report, one JSON line for each ticker
    --emit FILE          write the mapped values, one JSON line for each period
    --strict             exit code 1 if a ticker has status FAIL or ERROR
    --registry FILE      use this registry file, not the built-in one

fetch (network: SEC EDGAR; the contact is in the file sec_contact.toml of the repo;
    the environment variable SEC_USER_AGENT is optional and replaces it)
    --top N              the first N companies of the SEC ticker list
    --write-list FILE    write the tickers that are in the cache after the run
    --delay S            seconds between requests (default 0.15, minimum 0.1)
    --refresh            get a ticker again if it is in the cache
    --filings            get the files of the latest 10-K of each ticker (not the
                         company facts); the checks use them for inputs that the
                         company facts do not have
    --gaps               with --filings: only the tickers that have a critical
                         input with no value
    --prospectus         get the latest prospectus of each ticker (424B4, or S-1/A, or S-1)
    --submissions        get the earnings filings of each ticker (form 8-K with item 2.02) from the
                         SEC filing list, into <ticker>/sec_8k_earnings.json (1 request for each ticker)

fetch --edinet (network: EDINET API v2; the free key is in the environment variable EDINET_API_KEY or in .env;
    a missing key stops the command at once; the key is never printed or saved)
    --edinet             find the annual securities report (type 120) of the ticker in the file dates
                         --from to --to (one request each weekday, 3 seconds apart; stops at the first hit)
    --edinet-doc ID      get this document (EDINET document ID) and make no scan
    --edinet-rebuild     no request: make company_facts.json again from the ZIP files in the cache
    --ticker T           the Tokyo ticker, form 8001.T (the cache folder has this name; amounts stay in JPY)
    --filed D            filing date, if the cover page has none (YYYY-MM-DD)

import (gets the company facts if they are not in the cache)
    --store DIR          output directory (default: data/processed)

filing (network: www.sec.gov for a filing that is not in the cache)
    --statement KIND     income, balance or cash
    --focus              only the capex, depreciation and debt lines
    --refresh            get the filing again

crosscheck (capex, depreciation and debt: the standard variable against the lines
of the latest 10-K; network: www.sec.gov for a filing that is not in the cache)
    --out FILE           one JSON line for each ticker and variable
    --examples N         differences to print for each variable (default: 6)
";

const COMMON_VALUES: [&str; 3] = ["--cache", "--tickers-file", "--periods"];

fn cache_of(args: &Args) -> PathBuf {
    PathBuf::from(args.value("--cache").unwrap_or("data/raw"))
}

fn tickers_in_cache(cache: &Path) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(cache).map_err(|e| format!("{}: {e}", cache.display()))? {
        let name = entry.map_err(|e| e.to_string())?.file_name().to_string_lossy().to_string();
        if facts_path(cache, &name).is_some() {
            out.push(name);
        }
    }
    out.sort();
    Ok(out)
}

fn cmd_check(raw: &[String]) -> Result<bool, String> {
    let mut with_value = COMMON_VALUES.to_vec();
    with_value.extend(["--as-of", "--jobs", "--top", "--out", "--emit", "--registry"]);
    let args = Args::parse(raw, &with_value, &["--list", "--detail", "--strict"])?;
    let cache = cache_of(&args);
    let registry = match args.value("--registry") {
        Some(path) => Registry::load(&fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?)?,
        None => Registry::embedded()?,
    };
    let as_of = match args.value("--as-of") {
        Some(text) => dates::parse(text).ok_or_else(|| format!("--as-of '{text}' is not YYYY-MM-DD"))?,
        None => dates::today(),
    };
    let mut tickers = args.tickers()?;
    if tickers.is_empty() {
        tickers = tickers_in_cache(&cache)?;
    }
    if tickers.is_empty() {
        return Err(format!("no tickers (the cache {} is empty)", cache.display()));
    }
    if let Some(jobs) = args.value("--jobs") {
        let jobs: usize = jobs.parse().map_err(|_| "--jobs needs a number".to_string())?;
        rayon::ThreadPoolBuilder::new().num_threads(jobs).build_global().map_err(|e| e.to_string())?;
    }
    let emit = args.value("--emit");
    let start = Instant::now();
    let reports = check_all(&registry, &cache, &tickers, args.number("--periods", 12)?, as_of, emit.is_some());
    let seconds = start.elapsed().as_secs_f64();

    if args.flag("--list") {
        println!("{}", listing(&reports));
    }
    if args.flag("--detail") {
        for r in &reports {
            print!("{}", detail(&registry, r));
        }
        println!();
    }
    print!("{}", summary(&registry, &reports, seconds, rayon::current_num_threads(), args.number("--top", 25)?));
    if let Some(path) = args.value("--out") {
        let mut w = BufWriter::new(fs::File::create(path).map_err(|e| format!("{path}: {e}"))?);
        for r in &reports {
            writeln!(w, "{}", report_json(r)).map_err(|e| e.to_string())?;
        }
        println!("\nReport: {path}");
    }
    if let Some(path) = emit {
        let mut w = BufWriter::new(fs::File::create(path).map_err(|e| format!("{path}: {e}"))?);
        for r in &reports {
            for line in values_json(&registry, r) {
                writeln!(w, "{line}").map_err(|e| e.to_string())?;
            }
        }
        println!("Values: {path}");
    }
    let bad = reports.iter().any(|r| matches!(r.status, Status::Fail | Status::Error));
    Ok(!(args.flag("--strict") && bad))
}

fn cmd_fetch(raw: &[String]) -> Result<bool, String> {
    let mut with_value = COMMON_VALUES.to_vec();
    with_value.extend(["--top", "--write-list", "--delay", "--edinet-doc", "--from", "--to", "--ticker", "--filed"]);
    let args = Args::parse(raw, &with_value, &["--refresh", "--filings", "--gaps", "--prospectus", "--submissions", "--edinet", "--edinet-rebuild"])?;
    let cache = cache_of(&args);
    if args.flag("--edinet") || args.flag("--edinet-rebuild") || args.value("--edinet-doc").is_some() {
        return cmd_fetch_edinet(&args, &cache);
    }
    let delay: f64 = args.number("--delay", 0.15)?;
    if args.flag("--submissions") {
        // The forms 8-K with item 2.02 of each ticker (the past earnings dates), 1 request for each ticker.
        let mut ok = true;
        for ticker in &args.tickers()? {
            match sec::fetch_earnings_filings(&cache, ticker, delay) {
                Ok((filings, path)) => println!("{ticker}: {} filings 8-K with item 2.02, newest {} | {}", filings.len(), filings.first().map_or("-", |f| f.filed.as_str()), path.display()),
                Err(e) => {
                    println!("{e}");
                    ok = false;
                }
            }
        }
        return Ok(ok);
    }
    if args.flag("--prospectus") {
        // The primary document of the latest prospectus (the company facts must be in the cache).
        let mut ok = true;
        for ticker in &args.tickers()? {
            match sec::fetch_prospectus(&cache, ticker, delay, args.flag("--refresh")) {
                Ok((filing, path)) => println!("{ticker}: {} {} filed {} | {}", filing.form, filing.accession, filing.filed, path.display()),
                Err(e) => {
                    println!("{e}");
                    ok = false;
                }
            }
        }
        return Ok(ok);
    }
    if args.flag("--gaps") && !args.flag("--filings") {
        return Err("--gaps is an option of --filings".to_string());
    }
    if args.flag("--filings") {
        // The files of the latest 10-K of each ticker (the company facts must be in the cache).
        let mut tickers = args.tickers()?;
        if args.flag("--gaps") {
            // Only the tickers that have a critical input with no value.
            let registry = Registry::embedded()?;
            let reports = check_all(&registry, &cache, &tickers, args.number("--periods", 12)?, dates::today(), false);
            tickers = reports
                .into_iter()
                .filter(|r| r.findings.iter().any(|f| f.code == "core_missing_critical"))
                .map(|r| r.ticker)
                .collect();
        }
        let (mut done, mut failed) = (0usize, Vec::new());
        for ticker in &tickers {
            match sec::fetch_filing(&cache, ticker, delay, args.flag("--refresh")) {
                Ok(_) => done += 1,
                Err(e) => failed.push((ticker.clone(), e)),
            }
        }
        println!("Filings: {} tickers | in the cache now {done} | failed {}", tickers.len(), failed.len());
        for (ticker, reason) in &failed {
            println!("  {ticker:8} {reason}");
        }
        return Ok(true);
    }
    let mut client = Client::sec(delay)?;
    let mut tickers = args.tickers()?;
    // A ticker file can give the CIK of each ticker; then the SEC ticker list is not necessary.
    let mut known: Vec<(String, String)> = Vec::new();
    if let Some(path) = args.value("--tickers-file") {
        for (ticker, cik) in mapcheck::cli::read_ticker_ciks(path)? {
            if let Some(cik) = cik {
                known.push((ticker, cik));
            }
        }
    }
    let all_known = !tickers.is_empty() && tickers.iter().all(|t| known.iter().any(|(k, _)| k == t));
    let mut list = if all_known && args.value("--top").is_none() {
        Vec::new()
    } else {
        sec::ticker_list(&mut client, &cache, args.flag("--refresh"))?
    };
    list.splice(0..0, known);
    if let Some(top) = args.value("--top") {
        let top: usize = top.parse().map_err(|_| "--top needs a number".to_string())?;
        for t in sec::top_tickers(&list, top) {
            if !tickers.contains(&t) {
                tickers.push(t);
            }
        }
    }
    if tickers.is_empty() {
        return Err("give tickers, --tickers-file or --top".to_string());
    }
    let (mut fetched, mut cached, mut bytes) = (0usize, 0usize, 0u64);
    let mut failed: Vec<(String, String)> = Vec::new();
    for (number, ticker) in tickers.iter().enumerate() {
        if !args.flag("--refresh") && facts_path(&cache, ticker).is_some() {
            cached += 1;
            continue;
        }
        match list.iter().find(|(t, _)| t == ticker) {
            None => failed.push((ticker.clone(), "not in the SEC ticker list".to_string())),
            Some((_, cik)) => match sec::fetch_facts(&mut client, &cache, ticker, cik) {
                Ok(size) => {
                    fetched += 1;
                    bytes += size;
                }
                Err(e) => failed.push((ticker.clone(), e)),
            },
        }
        if (number + 1) % 50 == 0 {
            println!("  {} of {} ...", number + 1, tickers.len());
        }
    }
    println!(
        "Tickers: {} | fetched {fetched} ({:.0} MB) | in the cache before {cached} | failed {}",
        tickers.len(),
        bytes as f64 / 1e6,
        failed.len()
    );
    for (ticker, reason) in &failed {
        println!("  {ticker:8} {reason}");
    }
    if let Some(path) = args.value("--write-list") {
        let have: Vec<&String> = tickers.iter().filter(|t| facts_path(&cache, t).is_some()).collect();
        let mut text = format!("# {} tickers. Made by: mapcheck fetch", have.len());
        if let Some(top) = args.value("--top") {
            text.push_str(&format!(" --top {top} (the first companies of the SEC ticker list)"));
        }
        text.push_str(".\n");
        for t in have {
            text.push_str(t);
            text.push('\n');
        }
        fs::write(path, text).map_err(|e| format!("{path}: {e}"))?;
        println!("List: {path}");
    }
    Ok(true)
}

/// The EDINET read of one Tokyo ticker: find the annual report (or take the document ID), get the CSV form,
/// save it with its record, make `company_facts.json`, and print the latest year as proof.
fn cmd_fetch_edinet(args: &Args, cache: &Path) -> Result<bool, String> {
    use mapcheck::edinet;
    let ticker = args.value("--ticker").map(str::to_uppercase).or_else(|| args.tickers().ok().and_then(|t| t.into_iter().next())).ok_or("give the ticker (for example 8001.T)")?;
    let sec_code = edinet::security_code(&ticker)?;
    let folder = edinet::folder(cache, &ticker);
    if !args.flag("--edinet-rebuild") {
        let key = edinet::api_key()?;
        let delay: f64 = args.number("--delay", edinet::DEFAULT_DELAY_SECONDS)?;
        let mut session = edinet::Session::new(edinet::Http::new(&key, delay), &key);
        let (doc_id, meta) = match args.value("--edinet-doc") {
            Some(id) => (id.to_string(), None),
            None => {
                let (from, to) = (args.value("--from").ok_or("give --from and --to (file dates, YYYY-MM-DD) or --edinet-doc DOCID")?, args.value("--to").ok_or("give --to")?);
                let found = session.find_annual_report(&sec_code, from, to, &mut |date, n| println!("  {date}: {n} documents"))?;
                match found {
                    Some(meta) => {
                        println!("found {} {} (EDINET code {}), fiscal year {} to {}, submitted {}", meta.doc_id, meta.filer_name, meta.edinet_code, meta.period_start, meta.period_end, meta.submit_date_time);
                        (meta.doc_id.clone(), Some(meta))
                    }
                    None => {
                        println!("{ticker}: no annual securities report (type 120, with a CSV form) of security code {sec_code} in {from} to {to} ({} requests)", session.requests);
                        return Ok(false);
                    }
                }
            }
        };
        let zip = session.csv_zip(&doc_id)?;
        let today = dates::format(dates::today());
        let path = edinet::save_document(&folder, &doc_id, &zip, meta.as_ref(), args.value("--filed"), &today)?;
        println!("saved {} ({} bytes, {} requests)", path.display(), zip.len(), session.requests);
    }
    let (facts_file, lines) = edinet::rebuild_facts(&folder)?;
    for line in &lines {
        println!("{line}");
    }
    println!("facts: {}", facts_file.display());
    // The proof: the standard variables of the latest year, in JPY.
    let registry = Registry::embedded()?;
    let facts = mapcheck::run::read_facts(&facts_file)?;
    let company = mapcheck::mapper::map_company(&registry, &facts, 12);
    println!("{ticker}: {} | currency {} (no conversion) | {} annual periods", company.name, company.currency, company.annual.len());
    if let Some(p) = company.annual.last() {
        println!("latest fiscal year end {}:", p.end);
        for name in ["revenue", "operating_income", "net_income", "operating_cash_flow", "capex", "total_assets", "total_liabilities", "equity", "cash_and_equivalents", "shares_outstanding"] {
            match p.get(&registry, name) {
                Some(v) => println!("  {name:22} {v:>22.0}  {}", p.source(&registry, name)),
                None => println!("  {name:22} {:>22}", "(no value)"),
            }
        }
    }
    Ok(true)
}

fn cmd_import(raw: &[String]) -> Result<bool, String> {
    let mut with_value = COMMON_VALUES.to_vec();
    with_value.extend(["--store", "--delay"]);
    let args = Args::parse(raw, &with_value, &[])?;
    let cache = cache_of(&args);
    let dir = PathBuf::from(args.value("--store").unwrap_or("data/processed"));
    let registry = Registry::embedded()?;
    let tickers = args.tickers()?;
    if tickers.is_empty() {
        return Err("give one or more tickers".to_string());
    }
    let periods: usize = args.number("--periods", 12)?;
    let delay: f64 = args.number("--delay", 0.15)?;
    let mut ok = true;
    for ticker in &tickers {
        match import_ticker(&registry, &cache, &dir, ticker, periods, delay, false) {
            Ok(done) => println!(
                "{ticker}: {} | {} annual periods, {} quarterly periods | {}",
                done.company.name,
                done.company.annual.len(),
                done.company.quarterly.len(),
                done.store_path.display()
            ),
            Err(ImportError::Unsupported(message)) => {
                eprintln!("{message}");
                ok = false;
            }
            Err(ImportError::Failed(message)) => {
                eprintln!("{ticker}: {message}");
                ok = false;
            }
        }
    }
    Ok(ok)
}

fn cmd_values(raw: &[String]) -> Result<bool, String> {
    let args = Args::parse(raw, &COMMON_VALUES, &[])?;
    let cache = cache_of(&args);
    let registry = Registry::embedded()?;
    let tickers = args.tickers()?;
    if tickers.is_empty() {
        return Err("give one or more tickers".to_string());
    }
    for ticker in &tickers {
        match sec::load_or_fetch_facts(&cache, ticker, 0.15) {
            Ok(facts) => {
                let report = check_cached(&registry, &cache, ticker, &facts, args.number("--periods", 12)?, dates::today(), true);
                match sec::company_sic(&cache, ticker) {
                    Some((sic, text)) => println!("\n{ticker}: SIC {sic} {text}, sector {} (Fama-French 12)", sec::ff12_sector(sic)),
                    None => println!("\n{ticker}: no SIC code in the cache (the file is missing or old, or the filer has no code; run: mapcheck fetch --submissions {ticker})"),
                }
                if report.currency != "USD" {
                    println!("{ticker}: ifrs-full facts; all amounts are in {} (no conversion to USD)", report.currency);
                    if report.currency == "unknown" {
                        println!("{ticker}: the currency is not known (no total assets in a 3-letter currency unit): no values");
                    }
                    if facts.other_currency_facts() > 0 {
                        println!("{ticker}: {} total-assets facts in another currency are not used", facts.other_currency_facts());
                    }
                }
                print!("{}", values_listing(&registry, &report));
            }
            Err(e) => println!("\n{ticker}: {e}"),
        }
    }
    Ok(true)
}

fn cmd_filing(raw: &[String]) -> Result<bool, String> {
    let mut with_value = COMMON_VALUES.to_vec();
    with_value.extend(["--statement", "--delay"]);
    let args = Args::parse(raw, &with_value, &["--focus", "--refresh"])?;
    let cache = cache_of(&args);
    let registry = Registry::embedded()?;
    let statement = args.value("--statement");
    if let Some(kind) = statement {
        if !["income", "balance", "cash"].contains(&kind) {
            return Err(format!("--statement '{kind}' is not income, balance or cash"));
        }
    }
    let tickers = args.tickers()?;
    if tickers.is_empty() {
        return Err("give one or more tickers".to_string());
    }
    let mut ok = true;
    for ticker in &tickers {
        let result = sec::fetch_filing(&cache, ticker, args.number("--delay", 0.15)?, args.flag("--refresh"))
            .and_then(|folder| mapcheck::filing::load_filing(ticker, &folder));
        match result {
            Ok(filing) => print!("{}", mapcheck::filing::report(&registry, &filing, statement, args.flag("--focus"))),
            Err(e) => {
                println!("\n{ticker}: {e}");
                ok = false;
            }
        }
    }
    Ok(ok)
}

fn cmd_crosscheck(raw: &[String]) -> Result<bool, String> {
    use mapcheck::crosscheck::{compare, outcome_label, Summary};
    let mut with_value = COMMON_VALUES.to_vec();
    with_value.extend(["--out", "--examples", "--delay"]);
    let args = Args::parse(raw, &with_value, &[])?;
    let cache = cache_of(&args);
    let registry = Registry::embedded()?;
    let tickers = args.tickers()?;
    if tickers.is_empty() {
        return Err("give tickers or --tickers-file".to_string());
    }
    let periods: usize = args.number("--periods", 12)?;
    let delay: f64 = args.number("--delay", 0.15)?;
    let mut summary = Summary::default();
    let mut skipped: Vec<(String, String)> = Vec::new();
    let mut lines: Vec<serde_json::Value> = Vec::new();
    for ticker in &tickers {
        let outcome = (|| -> Result<(String, Vec<mapcheck::crosscheck::Row>), String> {
            let facts = sec::load_or_fetch_facts(&cache, ticker, delay)?;
            if !facts.has_us_gaap() {
                return Err("not a us-gaap filer (the crosscheck reads a 10-K)".to_string());
            }
            let company = mapcheck::mapper::map_company(&registry, &facts, periods);
            let filing = mapcheck::filing::load_filing(ticker, &sec::fetch_filing(&cache, ticker, delay, false)?)?;
            let end = filing.fiscal_year_end.clone().ok_or("the filing has no fiscal year end")?;
            let period = company
                .annual
                .iter()
                .find(|p| p.end == end)
                .ok_or_else(|| format!("no annual row for the fiscal year of the filing ({end})"))?;
            Ok((end, compare(&registry, period, &mapcheck::filing::estimates(&registry, &filing))))
        })();
        match outcome {
            Ok((end, rows)) => {
                summary.add(ticker, &rows);
                for row in &rows {
                    lines.push(serde_json::json!({
                        "ticker": ticker, "fiscal_year_end": end, "variable": row.variable,
                        "outcome": outcome_label(row.outcome), "standard_variable": row.ours, "filing": row.filing, "note": row.note,
                    }));
                }
            }
            Err(e) => skipped.push((ticker.clone(), e)),
        }
    }
    print!("{}", summary.text(&skipped, args.number("--examples", 6)?));
    if let Some(path) = args.value("--out") {
        let mut w = BufWriter::new(fs::File::create(path).map_err(|e| format!("{path}: {e}"))?);
        for line in &lines {
            writeln!(w, "{line}").map_err(|e| e.to_string())?;
        }
        println!("\nReport: {path}");
    }
    Ok(true)
}

fn run() -> Result<bool, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.iter().any(|a| a == "-h" || a == "--help") {
        print!("{HELP}");
        return Ok(true);
    }
    match raw.first().map(String::as_str) {
        Some("check") => cmd_check(&raw[1..]),
        Some("fetch") => cmd_fetch(&raw[1..]),
        Some("import") => cmd_import(&raw[1..]),
        Some("values") => cmd_values(&raw[1..]),
        Some("filing") => cmd_filing(&raw[1..]),
        Some("crosscheck") => cmd_crosscheck(&raw[1..]),
        _ => cmd_check(&raw),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(message) => {
            eprintln!("mapcheck: {message}\n\nRun 'mapcheck --help' for the commands.");
            ExitCode::from(2)
        }
    }
}
