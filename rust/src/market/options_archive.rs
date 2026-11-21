//! `fa options archive [SYMBOLS] [--keep-days N] [--min-share X] [--force] [--rate R]`: copy the live option chains of the day with the time.
//!
//! Alpaca gives past closes only, never a past bid and ask. A chain that nobody copied on its day has no bid and no ask for
//! ever (`PLANNING/board/T-20261120-vol-7-chain-copy.md`). For each symbol (default SPX, SPY, VIX) the command:
//!
//! 1. gets the live Alpaca chain (`options_cli::fetch_alpaca`, the same code as `fa options snapshot --source alpaca`);
//! 2. checks the size: a chain with less than the share (default 80%) of the quotes of the last live file of the symbol is a half chain
//!    (a failed page, a closed market) and is NOT written; the line says so;
//! 3. writes `<date>T<hhmm>.json` (the time is UTC, to the minute, and is in the file too) and the gzip copy
//!    `<date>T<hhmm>.json.gz` of the same text. The JSON stays: all readers and the old programs read it. The gzip file is the
//!    small copy (about a tenth) that a person moves to another machine. The readers read a `.json.gz` file when no `.json`
//!    file of the same name exists, so `--keep-days N` can remove the old JSON files and the day stays readable.
//!
//! Then the Cboe VX settlement file is fetched as `fa futures curve VX` fetches it (`futures::curve`, file in
//! `data/raw/futures/`). Cboe gives the file of a date before today (UTC) only, so a run on the day gives the settlement of
//! the trade date before.

use super::alpaca;
use super::options::{self, OptionChainSnapshot};
use super::options_cli::{fetch_alpaca, show_opt};
use crate::trading::calendar::parse_iso;
use std::path::{Path, PathBuf};

pub const DEFAULT_SYMBOLS: [&str; 3] = ["SPX", "SPY", "VIX"];
/// A chain with fewer quotes than this share (in percent) of the last live file is not written.
/// The default of `--min-share`.
pub const MIN_SHARE: f64 = 0.8;

#[derive(Debug, PartialEq)]
pub struct Stored {
    pub json: PathBuf,
    pub gz: PathBuf,
    pub quotes: usize,
    pub json_bytes: u64,
    pub gz_bytes: u64,
    /// A file of the same minute existed and was replaced.
    pub replaced: bool,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Written(Stored),
    /// Not written: the quotes of the chain, the quotes of the last live file and its name.
    TooSmall { quotes: usize, last_quotes: usize, last_stem: String },
}

/// The last file of the symbol from Alpaca with live quotes, with its stem and its number of quotes. A Cboe file (no source)
/// and a file of closes are no base: their chains have another size.
pub fn last_live(cache: &Path, ticker: &str) -> Option<(String, usize)> {
    for (stem, path) in options::snapshot_files(cache, ticker).into_iter().rev() {
        let Ok(text) = options::read_text(&path) else { continue };
        let Ok(snap) = serde_json::from_str::<OptionChainSnapshot>(&super::lenient_json(&text)) else { continue };
        if snap.source.as_deref() == Some("alpaca") && snap.quote_kind.as_deref() != Some("close") {
            return Some((stem, snap.quotes.len()));
        }
    }
    None
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

fn gzip(text: &str) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    enc.finish().map_err(|e| e.to_string())
}

/// Write the JSON file and the gzip copy of a live snapshot with a time, after the 80% check. The gzip file is read back and
/// compared with the JSON text; a copy that differs is removed and is an error.
pub fn store(cache: &Path, snap: &OptionChainSnapshot, min_share: f64, force: bool) -> Result<Outcome, String> {
    let hhmm = snap.snapshot_time.as_deref().and_then(options::hhmm_of_stamp).ok_or_else(|| format!("the snapshot of {} {} has no time (YYYY-MM-DDTHH:MMZ)", snap.ticker, snap.snapshot_date))?;
    if let Some((last_stem, last_quotes)) = last_live(cache, &snap.ticker).filter(|_| !force) {
        if (snap.quotes.len() as f64) < last_quotes as f64 * min_share {
            return Ok(Outcome::TooSmall { quotes: snap.quotes.len(), last_quotes, last_stem });
        }
    }
    let text = serde_json::to_string_pretty(snap).map_err(|e| e.to_string())?;
    let json = options::timed_path(cache, &snap.ticker, &snap.snapshot_date, &hhmm);
    let gz = json.with_extension("json.gz");
    let replaced = json.is_file();
    super::write_atomic(&json, &text)?;
    write_bytes(&gz, &gzip(&text)?)?;
    if options::read_text(&gz)? != text {
        let _ = std::fs::remove_file(&gz);
        return Err(format!("{}: the gzip copy differs from the JSON file; removed", gz.display()));
    }
    let size = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    Ok(Outcome::Written(Stored { quotes: snap.quotes.len(), json_bytes: size(&json), gz_bytes: size(&gz), json, gz, replaced }))
}

/// Remove the JSON file of each timed snapshot that is older than `keep_days` before `today` (days since 1970), when the gzip
/// copy has the same text. A file with no gzip copy, or with another text, stays. A file with no time (Cboe, closes) stays.
/// Gives the number of files removed and the files that could not be checked (kept).
pub fn prune(cache: &Path, ticker: &str, today: i64, keep_days: i64) -> (usize, Vec<String>) {
    let (mut removed, mut broken) = (0, Vec::new());
    let dir = options::options_dir(cache, ticker);
    for stem in options::snapshot_files(cache, ticker).into_keys() {
        let Some((date, _)) = stem.split_once('T') else { continue };
        let Some(day) = parse_iso(date) else { continue };
        if i64::from(day) >= today - keep_days {
            continue;
        }
        let (json, gz) = (dir.join(format!("{stem}.json")), dir.join(format!("{stem}.json.gz")));
        if json.is_file() && gz.is_file() {
            match (options::read_text(&json), options::read_text(&gz)) {
                (Ok(a), Ok(b)) if a == b => match std::fs::remove_file(&json) {
                    Ok(()) => removed += 1,
                    Err(e) => broken.push(format!("{}: {e}", json.display())),
                },
                (Ok(_), Ok(_)) => broken.push(format!("{}: the gzip copy differs; JSON kept", gz.display())),
                (_, Err(e)) | (Err(e), _) => broken.push(format!("{e}; JSON kept")),
            }
        }
    }
    (removed, broken)
}

fn mb(bytes: u64) -> String {
    format!("{:.2} MB", bytes as f64 / 1e6)
}

/// The output line of one symbol. It holds counts, times, sizes and paths only.
pub fn line(symbol: &str, stamp: &str, requests: usize, spot: Option<f64>, outcome: &Outcome) -> String {
    match outcome {
        Outcome::Written(s) => format!(
            "archive {symbol} {stamp}: {} quotes, {requests} requests, spot {}, json {}, gzip {}, {} {}",
            s.quotes,
            show_opt(spot),
            mb(s.json_bytes),
            mb(s.gz_bytes),
            if s.replaced { "REPLACED the file of the same minute:" } else { "written to" },
            s.json.display()
        ),
        Outcome::TooSmall { quotes, last_quotes, last_stem } => format!(
            "archive {symbol} {stamp}: NOT written, {quotes} quotes are less than the minimum share of the {last_quotes} quotes of {last_stem} ({requests} requests). If the chain really became smaller, take it as the new base: fa options archive {symbol} --force (or --min-share X, for example 0.6)"
        ),
    }
}

/// The Cboe VX settlement file, as `fa futures curve VX` gets it. The line, and whether it worked.
fn vx(repo: &Path) -> (String, bool) {
    crate::futures::ignore_cache_age(true);
    match crate::futures::curve(repo, "VX", None) {
        Ok(c) => (format!("archive VX settlement of {}: {} points, Cboe file {}", c.date, c.points.len(), crate::futures::cache_dir(repo).join(format!("cboe_settle_{}.csv", c.date)).display()), true),
        Err(e) => (format!("archive VX settlement: not copied ({e})"), false),
    }
}

struct Parsed {
    symbols: Vec<String>,
    keep_days: Option<i64>,
    rate: Option<f64>,
    min_share: f64,
    force: bool,
}

fn parse(rest: &[String]) -> Result<Parsed, String> {
    let mut p = Parsed { symbols: Vec::new(), keep_days: None, rate: None, min_share: MIN_SHARE, force: false };
    let mut i = 0;
    while i < rest.len() {
        let x = rest[i].as_str();
        match x {
            "--force" => {
                p.force = true;
                i += 1;
            }
            "--keep-days" | "--rate" | "--min-share" => {
                let v = rest.get(i + 1).filter(|v| !v.starts_with("--")).ok_or_else(|| format!("{x} needs a value"))?;
                if x == "--keep-days" {
                    p.keep_days = Some(v.parse::<i64>().ok().filter(|n| *n >= 0).ok_or_else(|| format!("bad --keep-days '{v}'"))?);
                } else if x == "--min-share" {
                    p.min_share = v.parse::<f64>().ok().filter(|n| (0.0..=1.0).contains(n)).ok_or_else(|| format!("bad --min-share '{v}' (0 to 1)"))?;
                } else {
                    p.rate = Some(super::options_cli::parse_rate(v)?);
                }
                i += 2;
            }
            _ if x.starts_with("--") => return Err(format!("unknown option {x}; see fa options --help")),
            _ => {
                p.symbols.extend(x.split(',').filter(|s| !s.is_empty()).map(|s| s.to_uppercase()));
                i += 1;
            }
        }
    }
    if p.symbols.is_empty() {
        p.symbols = DEFAULT_SYMBOLS.iter().map(|s| s.to_string()).collect();
    }
    Ok(p)
}

pub fn run(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let p = parse(rest)?;
    let cache = super::price_cli::cache_dir(repo);
    // One clock for the whole run: the date and the time stamp are the start of the run.
    let now = alpaca::now_utc();
    let (stamp, _) = options::utc_stamp(now);
    let today = stamp[..10].to_string();
    let mut failed = false;
    for sym in &p.symbols {
        match fetch_alpaca(repo, sym, &today, None, p.rate, &stamp).and_then(|f| store(&cache, &f.snap, p.min_share, p.force).map(|o| (f, o))) {
            Ok((f, outcome)) => {
                failed |= matches!(outcome, Outcome::TooSmall { .. });
                println!("{}", line(sym, &stamp, f.requests, f.snap.spot_price, &outcome));
            }
            Err(e) => {
                failed = true;
                println!("archive {sym} {stamp}: NOT written, {e}");
            }
        }
        if let Some(n) = p.keep_days {
            let (k, broken) = prune(&cache, sym, now.div_euclid(86_400), n);
            if k > 0 {
                println!("archive {sym}: {k} old JSON files removed (the gzip copies stay, {n} days kept)");
            }
            for b in broken {
                println!("archive {sym}: prune kept a file: {b}");
            }
        }
    }
    let (vx_line, ok) = vx(repo);
    println!("{vx_line}");
    failed |= !ok;
    Ok(i32::from(failed))
}

#[cfg(test)]
mod tests {
    use super::super::options::OptionQuote;
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fa_arch_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn quote(i: usize) -> OptionQuote {
        OptionQuote {
            contract_symbol: format!("SPY261120C{:08}", 500_000 + i * 1000),
            option_type: "call".into(),
            expiry: "2026-11-20".into(),
            strike: 500.0 + i as f64,
            last_price: Some(1.0),
            bid: Some(1.5),
            ask: Some(1.6),
            volume: Some(3),
            open_interest: None,
            implied_volatility: Some(0.2),
            in_the_money: Some(false),
        }
    }

    fn snap(n: usize, stamp: Option<&str>) -> OptionChainSnapshot {
        OptionChainSnapshot {
            ticker: "SPY".into(),
            snapshot_date: "2026-11-20".into(),
            spot_price: Some(560.0),
            quotes: (0..n).map(quote).collect(),
            source: Some("alpaca".into()),
            feed: Some("indicative".into()),
            quote_kind: Some("indicative".into()),
            note: None,
            snapshot_time: stamp.map(String::from),
        }
    }

    // Test 2 of the ticket: the file name and the time round-trip; an old file without the time is still read.
    #[test]
    fn the_time_and_the_file_name_round_trip() {
        // 2026-11-20 15:42:37 UTC
        let secs = i64::from(parse_iso("2026-11-20").unwrap()) * 86_400 + 15 * 3600 + 42 * 60 + 37;
        let (stamp, hhmm) = options::utc_stamp(secs);
        assert_eq!((stamp.as_str(), hhmm.as_str()), ("2026-11-20T15:42Z", "1542"));
        assert_eq!(options::hhmm_of_stamp(&stamp).as_deref(), Some("1542"));
        assert_eq!(options::hhmm_of_stamp("2026-11-20"), None);
        assert_eq!(options::hhmm_of_stamp("2026-11-20T15:42"), None);

        let d = dir("name");
        let s = snap(4, Some(&stamp));
        let path = options::save_snapshot(&s, &d).unwrap();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "2026-11-20T1542.json");
        assert_eq!(options::load_snapshot_at(&d, "SPY", Some("2026-11-20"), Some("1542")).unwrap(), s);
        assert_eq!(options::load_snapshot(&d, "SPY", Some("2026-11-20")).unwrap().snapshot_time.as_deref(), Some("2026-11-20T15:42Z"));
        assert!(options::load_snapshot_at(&d, "SPY", Some("2026-11-20"), Some("1543")).unwrap_err().contains("1543"));
        assert!(options::load_snapshot_at(&d, "SPY", Some("2026-11-20"), Some("25:00")).unwrap_err().contains("bad --time"));

        // An old file (name <date>.json, no time field) is read, and it does not get the field when it is written again.
        let old = r#"{"ticker":"SPY","snapshot_date":"2026-11-19","spot_price":1.0,"quotes":[]}"#;
        std::fs::write(options::snapshot_path(&d, "SPY", "2026-11-19"), old).unwrap();
        let o = options::load_snapshot(&d, "SPY", Some("2026-11-19")).unwrap();
        assert!(o.snapshot_time.is_none());
        assert!(!serde_json::to_string(&o).unwrap().contains("snapshot_time"));
        assert_eq!(options::list_snapshots(&d, "SPY"), ["2026-11-19", "2026-11-20"]);

        // Two files of one date: the date alone gives the LAST one; the old name comes before the timed names.
        std::fs::write(options::snapshot_path(&d, "SPY", "2026-11-20"), old.replace("11-19", "11-20")).unwrap();
        let mut early = snap(2, Some("2026-11-20T09:30Z"));
        early.spot_price = Some(1.0);
        options::save_snapshot(&early, &d).unwrap();
        assert_eq!(options::stems_of_date(&d, "SPY", "2026-11-20"), ["2026-11-20", "2026-11-20T0930", "2026-11-20T1542"]);
        assert_eq!(options::load_snapshot(&d, "SPY", Some("2026-11-20")).unwrap().quotes.len(), 4, "the 15:42 file");
        assert_eq!(options::load_snapshot_at(&d, "SPY", Some("2026-11-20"), Some("0930")).unwrap().quotes.len(), 2);
        assert_eq!(options::list_snapshots(&d, "SPY"), ["2026-11-19", "2026-11-20"], "a date once");
        let _ = std::fs::remove_dir_all(&d);
    }

    // The rule of #240 holds with timed files: closes never replace a live file, whatever its name.
    #[test]
    fn closes_never_replace_a_timed_live_file() {
        let d = dir("rule240");
        options::save_snapshot(&snap(3, Some("2026-11-20T15:42Z")), &d).unwrap();
        assert!(!super::super::options_cli::may_write(&d, "SPY", "2026-11-20", true).unwrap());
        assert!(super::super::options_cli::may_write(&d, "SPY", "2026-11-21", true).unwrap());
        let _ = std::fs::remove_dir_all(&d);
    }

    // Test 3: a chain with less than the share (default 80%) of the last quotes is not written.
    #[test]
    fn a_half_chain_is_not_written() {
        let d = dir("share");
        assert!(matches!(store(&d, &snap(100, Some("2026-11-20T15:00Z")), MIN_SHARE, false).unwrap(), Outcome::Written(_)), "no earlier file: written");
        // A file of closes with a long list is not the base.
        let mut closes = snap(1000, None);
        closes.quote_kind = Some("close".into());
        closes.snapshot_date = "2026-11-19".into();
        options::save_snapshot(&closes, &d).unwrap();
        assert_eq!(last_live(&d, "SPY"), Some(("2026-11-20T1500".to_string(), 100)));

        let small = snap(79, Some("2026-11-20T15:10Z"));
        let out = store(&d, &small, MIN_SHARE, false).unwrap();
        assert_eq!(out, Outcome::TooSmall { quotes: 79, last_quotes: 100, last_stem: "2026-11-20T1500".into() });
        assert!(!options::timed_path(&d, "SPY", "2026-11-20", "1510").exists(), "no JSON");
        assert!(!options::timed_path(&d, "SPY", "2026-11-20", "1510").with_extension("json.gz").exists(), "no gzip");
        let text = line("SPY", "2026-11-20T15:10Z", 14, Some(560.0), &out);
        assert!(text.contains("NOT written") && text.contains("79 quotes are less than the minimum share of the 100 quotes of 2026-11-20T1500"), "{text}");

        // Exactly 80% is written (the rule is "less than").
        assert!(matches!(store(&d, &snap(80, Some("2026-11-20T15:11Z")), MIN_SHARE, false).unwrap(), Outcome::Written(_)));
        // A snapshot without a time is refused.
        assert!(store(&d, &snap(100, None), MIN_SHARE, false).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    // Test 4: the gzip file has the same quotes as the JSON file; the readers read a day that has only the gzip file.
    #[test]
    fn the_gzip_copy_has_the_quotes_of_the_json_file() {
        let d = dir("gz");
        let s = snap(50, Some("2026-11-20T15:42Z"));
        let Outcome::Written(st) = store(&d, &s, MIN_SHARE, false).unwrap() else { panic!("not written") };
        assert_eq!(st.quotes, 50);
        let from_json: OptionChainSnapshot = serde_json::from_str(&std::fs::read_to_string(&st.json).unwrap()).unwrap();
        let from_gz: OptionChainSnapshot = serde_json::from_str(&options::read_text(&st.gz).unwrap()).unwrap();
        assert_eq!(from_json.quotes, from_gz.quotes);
        assert_eq!(from_gz, s);
        assert!(st.gz_bytes < st.json_bytes, "{} < {}", st.gz_bytes, st.json_bytes);

        // Remove the JSON file of an old day: the reader takes the gzip file.
        let today_far = i64::from(parse_iso("2026-12-31").unwrap());
        assert_eq!(prune(&d, "SPY", today_far, 7), (1, vec![]));
        assert!(!st.json.exists() && st.gz.exists());
        assert_eq!(options::load_snapshot(&d, "SPY", Some("2026-11-20")).unwrap(), s);
        assert_eq!(options::list_snapshots(&d, "SPY"), ["2026-11-20"]);
        // A day inside the keep window stays; a gzip file that differs keeps its JSON file.
        let s2 = snap(60, Some("2026-12-30T15:42Z"));
        let mut s2 = s2;
        s2.snapshot_date = "2026-12-30".into();
        let Outcome::Written(st2) = store(&d, &s2, MIN_SHARE, false).unwrap() else { panic!("not written") };
        assert_eq!(prune(&d, "SPY", today_far, 7).0, 0, "2 days old, 7 kept");
        let other = gzip("{}").unwrap();
        std::fs::write(&st2.gz, other).unwrap();
        let (n, broken) = prune(&d, "SPY", today_far + 30, 7);
        assert_eq!((n, broken.len()), (0, 1), "the copy differs: kept and reported");
        assert!(st2.json.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    // Test 5: no key is in a file or in a line of output.
    #[test]
    fn no_key_is_in_a_file_or_a_line() {
        const ID: &str = "PKTESTKEYID123456";
        const SECRET: &str = "sEcReTvAlUe-do-not-print-9876";
        let keys = alpaca::Keys { id: ID.into(), secret: SECRET.into() };
        // The scrub of an error text (the path that a failed request takes).
        let err = keys.scrub(&format!("HTTP 403 for header {ID} and {SECRET}"));
        assert!(!err.contains(ID) && !err.contains(SECRET), "{err}");

        let d = dir("key");
        let s = snap(30, Some("2026-11-20T15:42Z"));
        let out = store(&d, &s, MIN_SHARE, false).unwrap();
        let mut texts = vec![line("SPY", "2026-11-20T15:42Z", 14, s.spot_price, &out), line("SPY", "t", 1, None, &Outcome::TooSmall { quotes: 1, last_quotes: 9, last_stem: "x".into() })];
        for entry in std::fs::read_dir(options::options_dir(&d, "SPY")).unwrap() {
            let path = entry.unwrap().path();
            texts.push(options::read_text(&path).unwrap());
            let raw = std::fs::read(&path).unwrap();
            assert!(!String::from_utf8_lossy(&raw).contains(ID));
        }
        for t in &texts {
            assert!(!t.contains(ID) && !t.contains(SECRET) && !t.contains("APCA-API"), "key text in {t:.200}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    // Review F1: the base is an Alpaca file; a real shrink needs --force; the next normal run uses the new base.
    #[test]
    fn a_real_shrink_is_taken_with_force_and_becomes_the_base() {
        let d = dir("force");
        // A big Cboe file of the same symbol is no base.
        let mut cboe = snap(500, None);
        cboe.source = None;
        cboe.quote_kind = None;
        cboe.snapshot_date = "2026-11-19".into();
        options::save_snapshot(&cboe, &d).unwrap();
        assert!(matches!(store(&d, &snap(100, Some("2026-11-20T15:00Z")), MIN_SHARE, false).unwrap(), Outcome::Written(_)), "Cboe file is no base");
        let shrunk = snap(70, Some("2026-11-20T15:10Z"));
        let out = store(&d, &shrunk, MIN_SHARE, false).unwrap();
        assert!(matches!(out, Outcome::TooSmall { .. }), "30% shrink refused");
        let text = line("SPY", "t", 1, None, &out);
        assert!(text.contains("fa options archive SPY --force"), "{text}");
        assert!(matches!(store(&d, &shrunk, 0.6, false).unwrap(), Outcome::Written(_)), "--min-share 0.6 takes it");
        let _ = std::fs::remove_dir_all(&d);
        let d = dir("force2");
        store(&d, &snap(100, Some("2026-11-20T15:00Z")), MIN_SHARE, false).unwrap();
        assert!(matches!(store(&d, &shrunk, MIN_SHARE, true).unwrap(), Outcome::Written(_)), "--force writes");
        assert_eq!(last_live(&d, "SPY"), Some(("2026-11-20T1510".to_string(), 70)));
        assert!(matches!(store(&d, &snap(65, Some("2026-11-20T15:20Z")), MIN_SHARE, false).unwrap(), Outcome::Written(_)), "next run: base 70, 65 is above 56");
        let _ = std::fs::remove_dir_all(&d);
    }

    // Review MIN 1, 4, 6.
    #[test]
    fn guard_broken_gzip_and_same_minute() {
        let d = dir("misc");
        let s = snap(10, Some("2026-11-20T15:42Z"));
        assert!(!super::super::options_cli::has_live_alpaca(&d, "SPY", "2026-11-20"));
        let Outcome::Written(st) = store(&d, &s, MIN_SHARE, false).unwrap() else { panic!() };
        assert!(!st.replaced);
        assert!(super::super::options_cli::has_live_alpaca(&d, "SPY", "2026-11-20"));
        assert!(!super::super::options_cli::has_live_alpaca(&d, "SPY", "2026-11-21"));
        let Outcome::Written(again) = store(&d, &s, MIN_SHARE, false).unwrap() else { panic!() };
        assert!(again.replaced && line("SPY", "t", 1, None, &Outcome::Written(again)).contains("REPLACED"));
        std::fs::write(&st.gz, b"not gzip").unwrap();
        let far = i64::from(parse_iso("2026-12-31").unwrap());
        let (n, broken) = prune(&d, "SPY", far, 7);
        assert_eq!((n, broken.len()), (0, 1));
        assert!(st.json.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn arguments() {
        let v = |x: &[&str]| x.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let p = parse(&v(&[])).unwrap();
        assert_eq!(p.symbols, ["SPX", "SPY", "VIX"]);
        let p = parse(&v(&["spy,qqq", "--keep-days", "14", "--rate", "0.04"])).unwrap();
        assert_eq!((p.symbols, p.keep_days, p.rate), (vec!["SPY".to_string(), "QQQ".to_string()], Some(14), Some(0.04)));
        assert!(parse(&v(&["--keep-days", "x"])).is_err());
        assert!(parse(&v(&["--bad"])).is_err());
        let p = parse(&v(&["--force", "--min-share", "0.5"])).unwrap();
        assert!(p.force && p.min_share == 0.5);
        assert!(parse(&v(&["--min-share", "2"])).is_err());
    }
}
