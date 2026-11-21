//! `fa macro get`: the FRED macro series, from the public FRED graph CSV, into the LOCAL folder
//! (`data/local/macro/fred/<ID>.csv`). They never go into the data store `fa-data`: the FRED terms do not let us
//! store or publish FRED content (`docs/DATA-SOURCES.md`). `get` refuses a target inside a work tree with a
//! `fa-data` remote.
//!
//! The file form is the form of the FRED graph CSV: `observation_date,<ID>` and one row for each date.
//! The request goes through `Rates::fred_text` (the one FRED client: honest client name, one request at a time,
//! a pause for the host in `hostnet`). It asks for the full history.
//! NOTE: `guard_not_store` is a copy of the guard of `fa members get` (branch claude/sp500-members). Join the two
//! when both are on main.

use super::{today_iso, write_atomic};
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa macro - FRED macro series into the local folder (the FRED terms do not allow the public data store)

USAGE:
  fa macro get [--cache DIR] [--force]   download each series of the table into DIR/macro/fred/<ID>.csv and write
                                         DIR/macro/fred/SOURCE.md (citations). A file that exists is kept unless --force.
                                         Default --cache data/local (never data/raw: it is the clone of the public store).
  fa macro list                          the series, the FRED terms and the citations (no request).

Source: the FRED graph CSV (no key). Read the terms of each series on its FRED page before you add one.";

/// (series id, terms on FRED, citation). The list of `macro/fred/SOURCE.md` of the data store, from 2026-11-21.
const SERIES: [(&str, &str, &str); 25] = [
    ("TB3MS", "Public Domain: Citation Requested", "Board of Governors of the Federal Reserve System (US), 3-Month Treasury Bill Secondary Market Rate, Discount Basis [TB3MS], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("M13002US35620M156NNBR", "Copyrighted: Citation Required", "National Bureau of Economic Research, Commercial Paper Rates for New York, NY [M13002US35620M156NNBR], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("PAYEMS", "Public Domain: Citation Requested", "U.S. Bureau of Labor Statistics, All Employees, Total Nonfarm [PAYEMS], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("UNRATE", "Public Domain: Citation Requested", "U.S. Bureau of Labor Statistics, Unemployment Rate [UNRATE], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("ICSA", "Public Domain: Citation Requested", "U.S. Employment and Training Administration, Initial Claims [ICSA], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("CES0500000003", "Public Domain: Citation Requested", "U.S. Bureau of Labor Statistics, Average Hourly Earnings of All Employees, Total Private [CES0500000003], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("GDPC1", "Public Domain: Citation Requested", "U.S. Bureau of Economic Analysis, Real Gross Domestic Product [GDPC1], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("PCEPI", "Public Domain: Citation Requested", "U.S. Bureau of Economic Analysis, Personal Consumption Expenditures: Chain-type Price Index [PCEPI], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("PCEPILFE", "Public Domain: Citation Requested", "U.S. Bureau of Economic Analysis, Personal Consumption Expenditures Excluding Food and Energy (Chain-Type Price Index) [PCEPILFE], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("PCEC96", "Public Domain: Citation Requested", "U.S. Bureau of Economic Analysis, Real Personal Consumption Expenditures [PCEC96], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("RSAFS", "Public Domain: Citation Requested", "U.S. Census Bureau, Advance Retail Sales: Retail Trade and Food Services [RSAFS], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("NFCI", "Copyrighted: Citation Required", "Federal Reserve Bank of Chicago, Chicago Fed National Financial Conditions Index [NFCI], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("STLFSI4", "Copyrighted: Citation Required", "Federal Reserve Bank of St. Louis, St. Louis Fed Financial Stress Index [STLFSI4], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("SAHMREALTIME", "Public Domain: Citation Requested", "Sahm, Claudia, Real-time Sahm Rule Recession Indicator [SAHMREALTIME], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("T10Y3M", "Copyrighted: Citation Required", "Federal Reserve Bank of St. Louis, 10-Year Treasury Constant Maturity Minus 3-Month Treasury Constant Maturity [T10Y3M], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("BAA", "Copyrighted: Citation Required", "Moody’s, Moody's Seasoned Baa Corporate Bond Yield [BAA], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("AAA", "Copyrighted: Citation Required", "Moody’s, Moody's Seasoned Aaa Corporate Bond Yield [AAA], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("GS10", "Public Domain: Citation Requested", "Board of Governors of the Federal Reserve System (US), Market Yield on U.S. Treasury Securities at 10-Year Constant Maturity, Quoted on an Investment Basis [GS10], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("CPIAUCNS", "Public Domain: Citation Requested", "U.S. Bureau of Labor Statistics, Consumer Price Index for All Urban Consumers: All Items in U.S. City Average [CPIAUCNS], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("A0292AUSA380NNBR", "Copyrighted: Citation Required", "National Bureau of Economic Research, Private Public Utility Construction, Value in Constant Prices for United States [A0292AUSA380NNBR], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("A02082USA374NNBR", "Copyrighted: Citation Required", "National Bureau of Economic Research, Increases in Railroad Mileage for United States [A02082USA374NNBR], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("C275RC1A027NBEA", "Public Domain: Citation Requested", "U.S. Bureau of Economic Analysis, Private fixed investment in equipment and software: Nonresidential: Information processing equipment and software: Communication equipment [C275RC1A027NBEA], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("GDPA", "Public Domain: Citation Requested", "U.S. Bureau of Economic Analysis, Gross Domestic Product [GDPA], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("DFII10", "Public Domain: Citation Requested", "Board of Governors of the Federal Reserve System (US), Market Yield on U.S. Treasury Securities at 10-Year Constant Maturity, Quoted on an Investment Basis, Inflation-Indexed [DFII10], retrieved from FRED, Federal Reserve Bank of St. Louis"),
    ("T10YIE", "Copyrighted: Citation Required", "Federal Reserve Bank of St. Louis, 10-Year Breakeven Inflation Rate [T10YIE], retrieved from FRED, Federal Reserve Bank of St. Louis"),
];

/// A date that FRED answers with the first observation of each series.
const START: &str = "1700-01-01";

pub fn series_ids() -> Vec<&'static str> {
    SERIES.iter().map(|s| s.0).collect()
}

/// The default root of the local files: `data/local` (git-ignored, outside the store clone).
pub fn local_dir(repo: &Path) -> PathBuf {
    repo.join("data").join("local")
}

pub fn series_path(cache: &Path, id: &str) -> PathBuf {
    cache.join("macro").join("fred").join(format!("{id}.csv"))
}

/// The remote URLs of the git config text.
fn remote_urls(config: &str) -> Vec<String> {
    config.lines().filter_map(|l| l.trim().strip_prefix("url").and_then(|r| r.trim_start().strip_prefix('=')).map(|u| u.trim().to_string())).collect()
}

/// The git config file of the repo whose `.git` entry is `dot_git` (a folder, or a file of a linked work tree).
fn git_config_of(dot_git: &Path) -> Option<PathBuf> {
    if dot_git.is_dir() {
        return Some(dot_git.join("config"));
    }
    let text = std::fs::read_to_string(dot_git).ok()?;
    let gitdir = PathBuf::from(text.trim().strip_prefix("gitdir:")?.trim());
    let gitdir = if gitdir.is_absolute() { gitdir } else { dot_git.parent()?.join(gitdir) };
    let common = std::fs::read_to_string(gitdir.join("commondir")).ok().map(|c| gitdir.join(c.trim())).unwrap_or(gitdir);
    Some(common.join("config"))
}

/// Refuse a target folder inside a git work tree whose remote is `fa-data` (the public store).
pub fn guard_not_store(dir: &Path) -> Result<(), String> {
    let mut cur = Some(if dir.is_absolute() { dir.to_path_buf() } else { std::env::current_dir().map_err(|e| e.to_string())?.join(dir) });
    while let Some(d) = cur {
        let dot_git = d.join(".git");
        if dot_git.exists() {
            if let Some(cfg) = git_config_of(&dot_git).and_then(|c| std::fs::read_to_string(c).ok()) {
                if remote_urls(&cfg).iter().any(|u| u.contains("fa-data")) {
                    return Err(format!("{} is inside the work tree {} of the public store fa-data; a file here would be published. Use data/local (the default).", dir.display(), d.display()));
                }
            }
        }
        cur = d.parent().map(Path::to_path_buf);
    }
    Ok(())
}

/// The CSV text of the answer when it is a FRED CSV of this series with at least one value, else the reason.
pub fn validate(id: &str, text: &str) -> Result<usize, String> {
    let header = text.trim_start_matches('\u{feff}').lines().next().unwrap_or("").trim().to_string();
    if header.split(',').next_back() != Some(id) {
        let head: String = header.chars().take(60).collect();
        return Err(format!("not a FRED CSV for {id} (header: '{head}')"));
    }
    let n = crate::rates::parse_fred_csv(text, false)?.len();
    if n == 0 {
        return Err(format!("{id}: no value in the answer"));
    }
    Ok(n)
}

/// The text of `SOURCE.md` of the local folder: the citations and the terms of each series.
pub fn source_md(run_date: &str) -> String {
    let mut s = format!("# FRED series (downloaded {run_date} by `fa macro get` from the FRED graph CSV)\n\nLocal only. The FRED terms do not allow us to store or publish FRED content in the public data store.\n\n| File | Terms on FRED | Citation |\n|---|---|---|\n");
    for (id, terms, cit) in SERIES {
        s.push_str(&format!("| `{id}.csv` | {terms} | {cit} |\n"));
    }
    s
}

/// (written, kept, failed: id and reason).
pub type GetResult = (usize, usize, Vec<(String, String)>);

/// Download each series. `fetch(id)` gives the CSV text. Returns (written, kept, failed ids).
pub fn get_all(cache: &Path, force: bool, fetch: &mut dyn FnMut(&str) -> Result<String, String>) -> Result<GetResult, String> {
    guard_not_store(&cache.join("macro").join("fred"))?;
    let (mut written, mut kept, mut failed) = (0, 0, Vec::new());
    let mut in_a_row = 0;
    for (id, _, _) in SERIES {
        let path = series_path(cache, id);
        if path.exists() && !force {
            kept += 1;
            continue;
        }
        match fetch(id).and_then(|t| validate(id, &t).map(|n| (t, n))) {
            Ok((t, n)) => {
                write_atomic(&path, &t)?;
                println!("{id}: {n} values");
                written += 1;
                in_a_row = 0;
            }
            Err(e) => {
                eprintln!("{id}: FAILED: {e}");
                failed.push((id.to_string(), e));
                in_a_row += 1;
                if in_a_row >= 2 {
                    eprintln!("fa macro: two failures in a row; the host is stopped for this run");
                    break;
                }
            }
        }
    }
    write_atomic(&cache.join("macro").join("fred").join("SOURCE.md"), &source_md(&today_iso()))?;
    Ok((written, kept, failed))
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(0)
        }
        Some("list") => {
            for (id, terms, cit) in SERIES {
                println!("{id}\t{terms}\t{cit}");
            }
            Ok(0)
        }
        Some("get") => {
            let mut cache = local_dir(repo);
            let mut force = false;
            let mut it = args[1..].iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--cache" => cache = PathBuf::from(it.next().ok_or("--cache needs a folder")?),
                    "--force" => force = true,
                    o => return Err(format!("fa macro get: unknown option '{o}'")),
                }
            }
            let mut rates = crate::rates::Rates::live(repo);
            let (w, k, failed) = get_all(&cache, force, &mut |id| rates.fred_text(id, START))?;
            println!("macro: {w} written, {k} kept, {} failed, into {}", failed.len(), cache.join("macro").join("fred").display());
            Ok(if failed.is_empty() { 0 } else { 1 })
        }
        Some(o) => Err(format!("fa macro: unknown command '{o}'\n\n{USAGE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STORE_IDS: [&str; 25] = [
        "A02082USA374NNBR", "A0292AUSA380NNBR", "AAA", "BAA", "C275RC1A027NBEA", "CES0500000003", "CPIAUCNS", "DFII10", "GDPA", "GDPC1", "GS10", "ICSA",
        "M13002US35620M156NNBR", "NFCI", "PAYEMS", "PCEC96", "PCEPI", "PCEPILFE", "RSAFS", "SAHMREALTIME", "STLFSI4", "T10Y3M", "T10YIE", "TB3MS", "UNRATE",
    ];

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fa_macro_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_table_is_the_list_of_the_store_source_file() {
        let mut ids = series_ids();
        ids.sort();
        assert_eq!(ids, STORE_IDS.to_vec());
        // When a store clone is near, its SOURCE.md and its files give the same list.
        for root in ["data/raw", "../data/raw", "../../fa-data"] {
            let dir = Path::new(root).join("macro").join("fred");
            if let Ok(rd) = std::fs::read_dir(&dir) {
                let mut files: Vec<String> = rd.filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".csv")).map(String::from)).collect();
                files.sort();
                assert_eq!(files, STORE_IDS.to_vec(), "{}", dir.display());
            }
        }
    }

    #[test]
    fn the_writer_output_round_trips_through_the_fred_parser() {
        let text = "observation_date,TB3MS\n1934-01-01,0.72\n1934-02-01,0.62\n1934-03-01,.\n";
        assert_eq!(validate("TB3MS", text).unwrap(), 2);
        let dir = tmp("rt");
        // Only TB3MS answers with a right header; the next two answers fail and stop the run.
        let (w, k, f) = get_all(&dir, false, &mut |id| Ok(if id == "TB3MS" { text.to_string() } else { "x".to_string() })).unwrap();
        assert_eq!((w, k), (1, 0), "{f:?}");
        let back = std::fs::read_to_string(series_path(&dir, "TB3MS")).unwrap();
        assert_eq!(back, text);
        let m = crate::rates::parse_fred_csv(&back, false).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(*m.values().next().unwrap(), 0.72);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_wrong_answer_is_a_failure_and_two_in_a_row_stop_the_run() {
        let dir = tmp("fail");
        let mut calls = Vec::new();
        let (w, k, f) = get_all(&dir, false, &mut |id| {
            calls.push(id.to_string());
            Ok("<html>blocked</html>".to_string())
        })
        .unwrap();
        assert_eq!((w, k, f.len()), (0, 0, 2));
        assert_eq!(calls.len(), 2);
        assert!(!series_path(&dir, "TB3MS").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_exists_is_kept_unless_force() {
        let dir = tmp("keep");
        let p = series_path(&dir, "AAA");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "old").unwrap();
        let asked = std::cell::RefCell::new(Vec::new());
        let mut fake = |id: &str| {
            asked.borrow_mut().push(id.to_string());
            Ok(format!("observation_date,{id}\n2020-01-01,1.5\n"))
        };
        let (w, k, f) = get_all(&dir, false, &mut fake).unwrap();
        assert_eq!((w, k, f.len()), (24, 1, 0));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "old");
        assert!(!asked.borrow().contains(&"AAA".to_string()));
        let (w, k, _) = get_all(&dir, true, &mut fake).unwrap();
        assert_eq!((w, k), (25, 0));
        assert!(std::fs::read_to_string(&p).unwrap().starts_with("observation_date,AAA"));
        assert!(std::fs::read_to_string(dir.join("macro/fred/SOURCE.md")).unwrap().contains("[TB3MS]"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_guard_refuses_a_store_work_tree() {
        let store = tmp("store");
        std::fs::create_dir_all(store.join(".git")).unwrap();
        std::fs::write(store.join(".git/config"), "[remote \"origin\"]\n\turl = https://github.com/someone/fa-data\n").unwrap();
        let e = guard_not_store(&store.join("macro/fred")).unwrap_err();
        assert!(e.contains("public store"), "{e}");
        let e = get_all(&store, false, &mut |_| Ok(String::new())).unwrap_err();
        assert!(e.contains("fa-data"), "{e}");
        assert!(!store.join("macro").exists());
        let other = tmp("other");
        std::fs::create_dir_all(other.join(".git")).unwrap();
        std::fs::write(other.join(".git/config"), "[remote \"origin\"]\n\turl = https://github.com/someone/FinancialAnalysis\n").unwrap();
        assert!(guard_not_store(&other.join("data/local/macro/fred")).is_ok());
        let _ = std::fs::remove_dir_all(&store);
        let _ = std::fs::remove_dir_all(&other);
    }
}
