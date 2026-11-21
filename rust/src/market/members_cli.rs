//! `fa members get`: the point-in-time S&P 500 member history, from one file of a pinned commit of a public
//! GitHub repo, into the LOCAL folder (`data/local/_members/sp500_history.csv`). It never goes into the data store:
//! `data/raw` is the work tree of the public store `fa-data`, so `get` refuses a target inside a work tree with a
//! `fa-data` remote.
//!
//! The file has the columns `date,tickers`. A row is the full member list from its date until the next row.
//! The SHA-256 of the answer must match `EXPECTED_SHA256`, or nothing is written.

use super::{today_iso, write_atomic};
use crate::portfolio::harness::sha256_hex;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa members - the S&P 500 members on a past date (a history file of a pinned commit, local cache only)

USAGE:
  fa members get [--cache DIR]    fetch the file, check its SHA-256, write DIR/_members/sp500_history.csv
                                  and sp500_history.meta.json. Default --cache data/local (never data/raw: it is the clone of the public store).
  fa members check [--cache DIR]  read the local file again: sorted and unique dates, 400 to 520 names a date,
                                  the last date. No request.
  fa members on DATE [--cache DIR]  the members on DATE (YYYY-MM-DD), one ticker on each line (class shares as BRK-B).
                                  A date after the last row is refused (survivor-bias risk).
Each read checks the file against the pinned SHA-256 and writes the commit and the hash to stderr.
The default folder is under the repository root: the environment variable FA_REPO (else the repo of the build).

Source: github.com/fja05680/sp500, a pinned commit. The file is not in the data store (licence: docs/DATA-SOURCES.md).";

pub const REPO: &str = "fja05680/sp500";
pub const COMMIT: &str = "a2430f2af0c79ddf0748e91de11bdeb1616ab5a7";
pub const FILE_NAME: &str = "S&P 500 Historical Components & Changes (Updated).csv";
pub const EXPECTED_SHA256: &str = "36326709d46d6cd25834de5df457b16f5f96fad3a06b9beac28f7b88aa0b0d54";
/// Seconds between two requests to the host (one request in a run).
const PAUSE: f64 = 2.0;
pub const MIN_NAMES: usize = 400;
pub const MAX_NAMES: usize = 520;

/// The percent-encoded form of a path part: letters, digits and `-._~` stay, `/` stays, all else is `%XX`.
pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn url() -> String {
    format!("https://raw.githubusercontent.com/{REPO}/{COMMIT}/{}", url_encode(FILE_NAME))
}

pub fn history_path(cache: &Path) -> PathBuf {
    cache.join("_members").join("sp500_history.csv")
}

fn meta_path(cache: &Path) -> PathBuf {
    cache.join("_members").join("sp500_history.meta.json")
}

/// Check the SHA-256 of the bytes against the pinned value. Returns the actual hash.
pub fn verify_sha256(bytes: &[u8], expected: &str) -> Result<String, String> {
    let actual = sha256_hex(bytes);
    if actual == expected {
        Ok(actual)
    } else {
        Err(format!("SHA-256 mismatch: expected {expected}, actual {actual}; nothing written"))
    }
}

/// True for a real calendar date written YYYY-MM-DD.
pub fn valid_date(s: &str) -> bool {
    use crate::trading::calendar::{iso, parse_iso};
    s.len() == 10 && parse_iso(s).is_some_and(|d| iso(d) == s)
}

/// The rows of the file: (date, sorted tickers; a class share `BRK.B` is written `BRK-B`, the form of the repo and the store), in file order. A header line is skipped.
pub fn parse_history(text: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || (i == 0 && line.starts_with("date")) {
            continue;
        }
        let (date, rest) = line.split_once(',').ok_or_else(|| format!("line {}: no comma", i + 1))?;
        if !valid_date(date) {
            return Err(format!("line {}: bad date '{date}'", i + 1));
        }
        let mut names: Vec<String> = rest.trim().trim_matches('"').split(',').map(|t| t.trim().replace('.', "-")).filter(|t| !t.is_empty()).collect();
        names.sort();
        rows.push((date.to_string(), names));
    }
    if rows.is_empty() {
        return Err("no rows".into());
    }
    Ok(rows)
}

/// The members on `date`: the tickers of the latest row with row date <= `date`.
pub fn members_on(rows: &[(String, Vec<String>)], date: &str) -> Result<Vec<String>, String> {
    if !valid_date(date) {
        return Err(format!("'{date}' is not a date in the form YYYY-MM-DD"));
    }
    let last = &rows[rows.len() - 1].0;
    if date > last.as_str() {
        return Err(format!("{date} is after the last row of the member history ({last}); the file has no list for it. Today's list would carry survivor bias: use it only with that label"));
    }
    let n = rows.partition_point(|(d, _)| d.as_str() <= date);
    if n == 0 {
        return Err(format!("{date} is before the first row of the member history ({})", rows[0].0));
    }
    Ok(rows[n - 1].1.clone())
}

/// Read the cache file, check it against `expected` (SHA-256), log the commit and the hash to stderr.
fn read_rows_with(cache: &Path, expected: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    let p = history_path(cache);
    let bytes = std::fs::read(&p).map_err(|_| format!("{} is missing; run `fa members get` first", p.display()))?;
    let sha = sha256_hex(&bytes);
    if sha != expected {
        return Err(format!("{} does not match the pinned SHA-256 (expected {expected}, actual {sha}); the file was changed. Run `fa members get` again", p.display()));
    }
    eprintln!("fa: members: {REPO} commit {COMMIT}, SHA-256 {sha}");
    let text = String::from_utf8(bytes).map_err(|e| format!("not UTF-8: {e}"))?;
    parse_history(&text)
}

fn read_rows(cache: &Path) -> Result<Vec<(String, Vec<String>)>, String> {
    read_rows_with(cache, EXPECTED_SHA256)
}

/// The S&P 500 members on `date` (YYYY-MM-DD) from the local cache file.
pub fn sp500_members_on(cache: &Path, date: &str) -> Result<Vec<String>, String> {
    members_on(&read_rows(cache)?, date)
}

pub struct Check {
    pub first: String,
    pub last: String,
    pub rows: usize,
    pub min_names: usize,
    pub max_names: usize,
    pub warnings: Vec<String>,
}

pub fn check_rows(rows: &[(String, Vec<String>)]) -> Check {
    let mut warnings = Vec::new();
    for w in rows.windows(2) {
        if w[0].0 >= w[1].0 {
            warnings.push(format!("dates not sorted and unique: {} then {}", w[0].0, w[1].0));
        }
    }
    let counts = rows.iter().map(|r| r.1.len());
    let (min_names, max_names) = (counts.clone().min().unwrap_or(0), counts.max().unwrap_or(0));
    for (d, t) in rows {
        if t.len() < MIN_NAMES || t.len() > MAX_NAMES {
            warnings.push(format!("{d}: {} names, outside {MIN_NAMES} to {MAX_NAMES}", t.len()));
        }
    }
    Check { first: rows[0].0.clone(), last: rows[rows.len() - 1].0.clone(), rows: rows.len(), min_names, max_names, warnings }
}

fn report(c: &Check) {
    println!("members: {} dates, {} to {}, {} to {} names a date", c.rows, c.first, c.last, c.min_names, c.max_names);
    for w in &c.warnings {
        println!("WARN: {w}");
    }
    println!("note: the last date is {}. A run that needs a later date uses today's list and must carry the survivor-bias label.", c.last);
}

fn meta_json(sha: &str, run_date: &str, c: &Check) -> serde_json::Value {
    serde_json::json!({
        "source_repo": REPO,
        "commit": COMMIT,
        "file_name": FILE_NAME,
        "url": url(),
        "sha256": sha,
        "fetched": run_date,
        "client": "FinancialAnalysis fa (personal research)",
        "first_date": c.first,
        "last_date": c.last,
        "dates": c.rows,
        "names_min": c.min_names,
        "names_max": c.max_names,
        "note": "local cache only; not in the data store (licence: docs/DATA-SOURCES.md)",
    })
}

/// Check the bytes, write both files. The file is not written if the hash or the format is wrong.
pub fn store(cache: &Path, bytes: &[u8], run_date: &str) -> Result<Check, String> {
    let sha = verify_sha256(bytes, EXPECTED_SHA256)?;
    let text = String::from_utf8(bytes.to_vec()).map_err(|e| format!("not UTF-8: {e}"))?;
    let check = check_rows(&parse_history(&text)?);
    // meta first: a file without a record is never left
    write_atomic(&meta_path(cache), &(serde_json::to_string_pretty(&meta_json(&sha, run_date, &check)).map_err(|e| e.to_string())? + "\n"))?;
    write_atomic(&history_path(cache), &text)?;
    Ok(check)
}

/// The default root of the local files: `data/local` (git-ignored, outside the store clone).
pub fn local_dir(repo: &Path) -> PathBuf {
    repo.join("data").join("local")
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

/// The absolute path with the nearest existing ancestor resolved (symlinks followed).
fn resolve(dir: &Path) -> Result<PathBuf, String> {
    let abs = if dir.is_absolute() { dir.to_path_buf() } else { std::env::current_dir().map_err(|e| e.to_string())?.join(dir) };
    let mut tail = Vec::new();
    let mut cur = abs.as_path();
    loop {
        if let Ok(c) = cur.canonicalize() {
            let mut out = c;
            for part in tail.iter().rev() {
                out.push(part);
            }
            return Ok(out);
        }
        match (cur.file_name(), cur.parent()) {
            (Some(n), Some(p)) => {
                tail.push(n.to_os_string());
                cur = p;
            }
            _ => return Err(format!("cannot resolve {}", dir.display())),
        }
    }
}

/// Refuse a target folder inside a git work tree whose remote is `fa-data` (the shared data store).
/// The path is resolved first (symlinks), and a `.git` entry that cannot be read refuses the target.
pub fn guard_not_store(dir: &Path) -> Result<(), String> {
    let mut cur = Some(resolve(dir)?);
    while let Some(d) = cur {
        let dot_git = d.join(".git");
        if dot_git.exists() {
            let cfg = git_config_of(&dot_git)
                .and_then(|c| std::fs::read_to_string(c).ok())
                .ok_or_else(|| format!("cannot read the git config of {}; refused (it may be the public store fa-data)", d.display()))?;
            if remote_urls(&cfg).iter().any(|u| u.contains("fa-data")) {
                return Err(format!("{} is inside the work tree {} of the public store fa-data; a file here would be published. Use data/local (the default).", dir.display(), d.display()));
            }
        }
        cur = d.parent().map(Path::to_path_buf);
    }
    Ok(())
}

fn cache_of(repo: &Path, rest: &[String]) -> Result<PathBuf, String> {
    let mut cache = None;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        if a == "--cache" {
            cache = Some(PathBuf::from(it.next().ok_or("--cache needs a folder")?));
        }
    }
    Ok(cache.unwrap_or_else(|| local_dir(repo)))
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(0)
        }
        Some("get") => {
            let cache = cache_of(repo, &args[1..])?;
            guard_not_store(&cache.join("_members"))?;
            let u = url();
            eprintln!("fa: members: GET {u}");
            let bytes = crate::hostnet::get(&u, &[], PAUSE)?;
            let c = store(&cache, &bytes, &today_iso())?;
            println!("members: written {} (commit {COMMIT}, SHA-256 {EXPECTED_SHA256})", history_path(&cache).display());
            report(&c);
            Ok(0)
        }
        Some("check") => {
            let c = check_rows(&read_rows(&cache_of(repo, &args[1..])?)?);
            report(&c);
            Ok(if c.warnings.is_empty() { 0 } else { 1 })
        }
        Some("on") => {
            let date = args.get(1).ok_or("fa members on DATE")?;
            let rows = read_rows(&cache_of(repo, &args[2..])?)?;
            let c = check_rows(&rows);
            for w in &c.warnings {
                eprintln!("WARN: {w}");
            }
            for t in members_on(&rows, date)? {
                println!("{t}");
            }
            Ok(0)
        }
        Some(other) => Err(format!("unknown members command '{other}'; see fa members --help")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "date,tickers\n2020-12-14,\"AAPL,MSFT,TSLA_NOT,XOM\"\n2020-12-18,\"AAPL,MSFT,XOM\"\n2020-12-21,\"AAPL,MSFT,TSLA,XOM\"\n";

    #[test]
    fn reader_cases() {
        let rows = parse_history(FIXTURE).unwrap();
        assert_eq!(rows.len(), 3);
        // between two rows: the earlier row
        assert_eq!(members_on(&rows, "2020-12-20").unwrap(), ["AAPL", "MSFT", "XOM"]);
        // on a row date
        assert_eq!(members_on(&rows, "2020-12-14").unwrap(), ["AAPL", "MSFT", "TSLA_NOT", "XOM"]);
        // on the last row date: allowed; after it: refused, with the last date named
        assert_eq!(members_on(&rows, "2020-12-21").unwrap().len(), 4);
        let e = members_on(&rows, "2030-01-01").unwrap_err();
        assert!(e.contains("after the last row") && e.contains("2020-12-21") && e.contains("survivor bias"));
        // before the first row
        assert!(members_on(&rows, "2020-12-13").unwrap_err().contains("before the first row"));
        // TSLA out on 2020-12-18, in on 2020-12-21
        assert!(!members_on(&rows, "2020-12-18").unwrap().contains(&"TSLA".to_string()));
        assert!(members_on(&rows, "2020-12-21").unwrap().contains(&"TSLA".to_string()));
    }

    #[test]
    fn bad_date_strings_are_refused() {
        let rows = parse_history(FIXTURE).unwrap();
        for bad in ["garbage", "20201215", "2020-1-5", "2020-02-30", "2020-13-45", "", "2020-12-1x", "+020-12-15"] {
            assert!(members_on(&rows, bad).unwrap_err().contains("YYYY-MM-DD"), "{bad}");
        }
        assert!(parse_history("2020-02-30,\"A\"\n").is_err());
    }

    #[test]
    fn class_share_dot_becomes_dash() {
        let rows = parse_history("2020-01-02,\"BRK.B,AAPL,BF.B\"\n").unwrap();
        assert_eq!(rows[0].1, ["AAPL", "BF-B", "BRK-B"]);
    }

    #[test]
    fn missing_file_tells_the_command() {
        let e = sp500_members_on(Path::new("/nonexistent/fa-members-test"), "2020-01-01").unwrap_err();
        assert!(e.contains("fa members get"));
    }

    #[test]
    fn sha_check() {
        // SHA-256 of "abc" (FIPS 180 test vector)
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(verify_sha256(b"abc", abc).unwrap(), abc);
        let e = verify_sha256(b"abd", abc).unwrap_err();
        assert!(e.contains(abc) && e.contains("actual"));
    }

    #[test]
    fn wrong_hash_writes_nothing() {
        let dir = std::env::temp_dir().join(format!("fa-members-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(store(&dir, FIXTURE.as_bytes(), "2026-11-21").is_err());
        assert!(!history_path(&dir).exists());
        assert!(!meta_path(&dir).exists());
    }

    #[test]
    fn an_edited_file_is_refused_on_read() {
        let dir = std::env::temp_dir().join(format!("fa-members-edit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_atomic(&history_path(&dir), FIXTURE).unwrap();
        // the right pin passes; one changed byte does not
        let pin = sha256_hex(FIXTURE.as_bytes());
        assert!(read_rows_with(&dir, &pin).is_ok());
        write_atomic(&history_path(&dir), &FIXTURE.replace("XOM", "XOX")).unwrap();
        assert!(read_rows_with(&dir, &pin).unwrap_err().contains("does not match"));
        // the public reader uses the real pin, so the fixture is refused
        assert!(sp500_members_on(&dir, "2020-12-15").unwrap_err().contains("SHA-256"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn count_rule_boundaries() {
        let mk = |n: usize| {
            let names: Vec<String> = (0..n).map(|i| format!("T{i}")).collect();
            vec![("2020-01-02".to_string(), names)]
        };
        assert_eq!(check_rows(&mk(399)).warnings.len(), 1);
        assert!(check_rows(&mk(400)).warnings.is_empty());
        assert!(check_rows(&mk(520)).warnings.is_empty());
        assert_eq!(check_rows(&mk(521)).warnings.len(), 1);
    }

    #[test]
    fn reader_reads_the_cache_file() {
        let dir = std::env::temp_dir().join(format!("fa-members-read-{}", std::process::id()));
        write_atomic(&history_path(&dir), FIXTURE).unwrap();
        let pin = sha256_hex(FIXTURE.as_bytes());
        assert!(members_on(&read_rows_with(&dir, &pin).unwrap(), "2020-12-21").unwrap().contains(&"TSLA".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_warns_and_unsorted() {
        let c = check_rows(&parse_history(FIXTURE).unwrap());
        assert_eq!(c.warnings.len(), 3); // 3 or 4 names: outside 400..520
        let bad = parse_history("2020-02-01,\"A\"\n2020-01-01,\"B\"\n").unwrap();
        assert!(check_rows(&bad).warnings.iter().any(|w| w.contains("not sorted")));
    }

    #[test]
    fn guard_refuses_the_store_clone() {
        let base = std::env::temp_dir().join(format!("fa-members-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let store = base.join("raw");
        std::fs::create_dir_all(store.join(".git")).unwrap();
        std::fs::write(store.join(".git/config"), "[remote \"origin\"]\n\turl = https://github.com/louellet-git/fa-data\n").unwrap();
        let e = guard_not_store(&store.join("_members")).unwrap_err();
        assert!(e.contains("fa-data") && e.contains("data/local"));
        let other = base.join("other");
        std::fs::create_dir_all(other.join(".git")).unwrap();
        std::fs::write(other.join(".git/config"), "[remote \"origin\"]\n\turl = https://github.com/x/FinancialAnalysis\n").unwrap();
        assert!(guard_not_store(&other.join("local/_members")).is_ok());
        let plain = base.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        assert!(guard_not_store(&plain.join("_members")).is_ok());
        // a symlink to a subfolder of the store clone is refused
        std::fs::create_dir_all(store.join("sub")).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(store.join("sub"), plain.join("link")).unwrap();
            assert!(guard_not_store(&plain.join("link/_members")).is_err());
        }
        // a .git file that cannot be read fails closed
        let broken = base.join("broken");
        std::fs::create_dir_all(&broken).unwrap();
        std::fs::write(broken.join(".git"), "gitdir: /nonexistent/x\n").unwrap();
        assert!(guard_not_store(&broken.join("_members")).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn url_is_encoded() {
        let u = url();
        assert!(u.ends_with("S%26P%20500%20Historical%20Components%20%26%20Changes%20%28Updated%29.csv"));
        assert!(u.contains(COMMIT));
    }
}
