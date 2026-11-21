//! `fa arb run`: the register checks on one stored option snapshot. Cache only: no network, no download.
//!
//! The command calls `option_arb::check_chain`, maps each check to its register id (`arbitrage/register/<ID>.md`) and
//! prints one table. A relation whose sold leg has no bid is "not tested", not "pass". A break is a measurement of one
//! stored file; on indicative Alpaca quotes it is not a tradable price.

use super::option_arb::{self, CheckOpts, Dropped, Report, CHECK_IDS};
use super::options::{self, OptionChainSnapshot};
use crate::rates::parse_treasury_csv;
use std::collections::BTreeMap;
use std::path::Path;

pub const USAGE: &str = "\
fa arb run --symbol T --date D [--time HHMM] [--rate R] [--out PATH]
  Runs the active relations of arbitrage/register/ on one stored snapshot (data/raw/T/options/D.json or DTHHMM.json).
  The rate is --rate R (flat, decimal) or the Treasury par curve file in data/raw/rates (read only). It never downloads.
  Prints id, class, status, tested, pass, break, not tested, and the dropped legs by reason; writes the same as JSON
  (default data/processed/arb/D/T-<HHMM or close>.json). Exit 0 also with breaks (a break is a measurement).
  A break on indicative quotes is not a tradable price.";

/// Class of each check (same order as `CHECK_IDS`): hard = inside one chain copy.
const CLASS: [&str; 7] = ["hard"; 7];
/// The checks that run for American options only (calendar, parity, intrinsic, bound).
const AMERICAN_ONLY: [usize; 4] = [3, 4, 5, 6];

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub id: &'static str,
    pub class: &'static str,
    pub status: &'static str,
    pub tested: usize,
    pub pass: usize,
    pub breaks: usize,
    pub not_tested: usize,
}

/// One line for each check, summed over the series of the report.
pub fn lines(rep: &Report, american: bool) -> Vec<Line> {
    (0..CHECK_IDS.len())
        .map(|i| {
            let tests: usize = rep.rows.iter().map(|r| r.tests[i]).sum();
            let not_tested: usize = rep.rows.iter().map(|r| r.not_tested[i]).sum();
            let breaks: usize = rep.rows.iter().map(|r| r.breaks[i]).sum();
            let tested = tests - not_tested;
            let status = if !american && AMERICAN_ONLY.contains(&i) {
                "n/a European"
            } else if breaks > 0 {
                "break"
            } else if tested > 0 {
                "pass"
            } else {
                "not tested"
            };
            Line { id: CHECK_IDS[i], class: CLASS[i], status, tested, pass: tested.saturating_sub(breaks), breaks, not_tested }
        })
        .collect()
}

fn dropped_list(d: &Dropped) -> [(&'static str, usize); 7] {
    [
        ("no bid and no ask above 0", d.no_bid),
        ("ask missing", d.no_ask),
        ("ask below bid", d.ask_below_bid),
        ("bad symbol", d.bad_symbol),
        ("bad expiry", d.bad_expiry),
        ("expiry on or before the date", d.past_expiry),
        ("type not call or put", d.bad_type),
    ]
}

pub fn render(label: &str, rate_text: &str, rep: &Report, ls: &[Line]) -> String {
    let mut s = format!("arb run {label}: {} quotes, {} used, rate {rate_text}\n", rep.quotes_total, rep.quotes_used);
    s.push_str(&format!("{:<18} {:<5} {:<13} {:>8} {:>8} {:>8} {:>11}\n", "id", "class", "status", "tested", "pass", "break", "not tested"));
    for l in ls {
        s.push_str(&format!("{:<18} {:<5} {:<13} {:>8} {:>8} {:>8} {:>11}\n", l.id, l.class, l.status, l.tested, l.pass, l.breaks, l.not_tested));
    }
    let parts: Vec<String> = dropped_list(&rep.dropped).iter().filter(|(_, n)| *n > 0).map(|(r, n)| format!("{r}: {n}")).collect();
    s.push_str(&format!("dropped legs: {} ({})\n", rep.dropped.total(), if parts.is_empty() { "none".into() } else { parts.join(", ") }));
    s.push_str(&format!("buy-only legs (bid 0, ask above 0; never a sold leg): {}; bid = ask (flagged, still used): {}; removed by the strike filter: {}\n", rep.buy_only, rep.locked, rep.otm_removed));
    s.push_str("limits: a break is a measurement of one stored file, not a tradable price; indicative Alpaca quotes are modified, not OPRA.\n");
    s
}

pub fn to_json(label: &str, snap: &OptionChainSnapshot, rate_text: &str, rep: &Report, ls: &[Line]) -> serde_json::Value {
    let dropped: BTreeMap<&str, usize> = dropped_list(&rep.dropped).into_iter().collect();
    serde_json::json!({
        "run": label, "snapshot_date": snap.snapshot_date, "quote_kind": snap.quote_kind, "source": snap.source,
        "rate": rate_text, "quotes_total": rep.quotes_total, "quotes_used": rep.quotes_used,
        "dropped_total": rep.dropped.total(), "dropped": dropped, "buy_only": rep.buy_only, "bid_equals_ask": rep.locked,
        "strike_filter_removed": rep.otm_removed,
        "relations": ls.iter().map(|l| serde_json::json!({"id": l.id, "class": l.class, "status": l.status, "tested": l.tested, "pass": l.pass, "break": l.breaks, "not_tested": l.not_tested})).collect::<Vec<_>>(),
        "limits": "a break is a measurement of one stored file, not a tradable price; indicative Alpaca quotes are modified, not OPRA",
    })
}

/// The rate function: one flat rate, or the Treasury par curve file of the cache. Reads files only. A missing file is an error.
pub fn cached_rate_fn(cache: &Path, flat: Option<f64>) -> impl FnMut(&str, f64) -> Result<f64, String> + '_ {
    let mut memo: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    move |date: &str, t: f64| {
        if let Some(r) = flat {
            return Ok(r);
        }
        if !memo.contains_key(date) {
            let on = super::parse_date(date).ok_or_else(|| format!("bad date {date}"))?;
            let year: i64 = date[..4].parse().map_err(|_| format!("bad date {date}"))?;
            let mut found = None;
            for y in [year, year - 1] {
                let path = cache.join("rates").join(format!("treasury_nominal_{y}.csv"));
                let Ok(text) = std::fs::read_to_string(&path) else { continue };
                if let Some((_, c)) = parse_treasury_csv(&text)?.range(..=on).next_back() {
                    found = Some(c.clone());
                    break;
                }
            }
            let curve = found.ok_or_else(|| format!("no Treasury par curve file for {date} in {}/rates (treasury_nominal_{year}.csv); give --rate R. This command does not download", cache.display()))?;
            memo.insert(date.to_string(), curve);
        }
        options_interp(&memo[date], t).ok_or_else(|| format!("no Treasury rate for {date}; give --rate R"))
    }
}

fn options_interp(c: &BTreeMap<String, f64>, t: f64) -> Option<f64> {
    super::options_cli::interpolate_curve(c, t)
}

/// The register checks on a snapshot (the default options of `fa options check`, no size floor).
pub fn run_chain(snap: &OptionChainSnapshot, ticker: &str, rate: &mut dyn FnMut(&str, f64) -> Result<f64, String>) -> Result<(Report, Vec<Line>), String> {
    let american = !option_arb::is_european(ticker);
    let opts = CheckOpts { min_size: 0.0, iv_mark: 0.0, otm: !american };
    let rep = option_arb::check_chain(snap, american, rate, &opts)?;
    let ls = lines(&rep, american);
    Ok((rep, ls))
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    if matches!(args.first().map(String::as_str), None | Some("-h" | "--help" | "help")) {
        println!("{USAGE}");
        return Ok(0);
    }
    if args[0] != "run" {
        return Err(format!("unknown arb command '{}'; see fa arb --help", args[0]));
    }
    let mut f: BTreeMap<&str, String> = BTreeMap::new();
    let mut i = 1;
    while i < args.len() {
        let k = ["--symbol", "--date", "--time", "--rate", "--out"].into_iter().find(|k| *k == args[i]).ok_or_else(|| format!("unknown option {}; see fa arb --help", args[i]))?;
        let v = args.get(i + 1).filter(|v| !v.starts_with("--")).ok_or_else(|| format!("{k} needs a value"))?;
        f.insert(k, v.clone());
        i += 2;
    }
    let ticker = f.get("--symbol").ok_or("give --symbol T")?.to_uppercase();
    let date = f.get("--date").ok_or("give --date D")?.clone();
    let time = f.get("--time").map(String::as_str);
    let flat = f.get("--rate").map(|r| super::options_cli::parse_rate(r)).transpose()?;
    let cache = super::price_cli::cache_dir(repo);
    let snap = options::load_snapshot_at(&cache, &ticker, Some(&date), time)?;
    let mut rate = cached_rate_fn(&cache, flat);
    let (rep, ls) = run_chain(&snap, &ticker, &mut rate)?;
    let rate_text = flat.map_or("Treasury par curve (cache)".to_string(), |r| format!("flat {r}"));
    let stem = options::stem_of(&cache, &ticker, &date, time)?;
    let label = format!("{ticker} {date} (file {stem})");
    print!("{}", render(&label, &rate_text, &rep, &ls));
    let out = match f.get("--out") {
        Some(p) => std::path::PathBuf::from(p),
        None => repo.join("data").join("processed").join("arb").join(&date).join(format!("{ticker}-{}.json", time.unwrap_or("close"))),
    };
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(&to_json(&label, &snap, &rate_text, &rep, &ls)).map_err(|e| e.to_string())?;
    std::fs::write(&out, json).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("written {}", out.display());
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::options::OptionQuote;

    fn q(call: bool, strike: f64, bid: Option<f64>, ask: Option<f64>) -> OptionQuote {
        let sym = format!("XYZ260116{}{:08}", if call { 'C' } else { 'P' }, (strike * 1000.0).round() as i64);
        OptionQuote { contract_symbol: sym, option_type: if call { "call" } else { "put" }.into(), expiry: "2026-01-16".into(), strike, last_price: None, bid, ask, volume: None, open_interest: None, implied_volatility: None, in_the_money: None }
    }
    fn snap(quotes: Vec<OptionQuote>) -> OptionChainSnapshot {
        OptionChainSnapshot { ticker: "XYZ".into(), snapshot_date: "2026-01-02".into(), spot_price: Some(100.0), quotes, source: None, feed: None, quote_kind: None, note: None, snapshot_time: None }
    }
    fn zero(_: &str, _: f64) -> Result<f64, String> {
        Ok(0.0)
    }
    fn get<'a>(ls: &'a [Line], id: &str) -> &'a Line {
        ls.iter().find(|l| l.id == id).unwrap()
    }

    /// Calls 90/100/110 with a butterfly break (bid 7.0 > 0.5 x 11.0 + 0.5 x 1.1 = 6.05) and a crossed quote at 120.
    #[test]
    fn exact_counts_for_one_butterfly_break_and_one_crossed_quote() {
        let s = snap(vec![q(true, 90.0, Some(10.9), Some(11.0)), q(true, 100.0, Some(7.0), Some(7.1)), q(true, 110.0, Some(1.0), Some(1.1)), q(true, 120.0, Some(2.0), Some(1.0))]);
        let (rep, ls) = run_chain(&s, "XYZ", &mut zero).unwrap();
        assert_eq!((rep.quotes_total, rep.quotes_used, rep.dropped.ask_below_bid, rep.dropped.total()), (4, 3, 1, 1));
        let want = [("OPT-MONOTONY", 3, 3, 0, 0, "pass"), ("OPT-SLOPE", 3, 3, 0, 0, "pass"), ("OPT-BUTTERFLY", 1, 0, 1, 0, "break"), ("OPT-CALENDAR", 0, 0, 0, 0, "not tested"), ("OPT-PARITY-UPPER", 0, 0, 0, 0, "not tested"), ("OPT-INTRINSIC", 3, 3, 0, 0, "pass"), ("OPT-BOUND", 3, 3, 0, 0, "pass")];
        for (id, tested, pass, brk, nt, st) in want {
            let l = get(&ls, id);
            assert_eq!((l.tested, l.pass, l.breaks, l.not_tested, l.status), (tested, pass, brk, nt, st), "{id}");
        }
    }

    #[test]
    fn a_buy_only_sold_leg_is_not_tested_and_not_pass() {
        let s = snap(vec![q(true, 90.0, Some(10.9), Some(11.0)), q(true, 100.0, Some(0.0), Some(7.1)), q(true, 110.0, Some(1.0), Some(1.1))]);
        let (_, ls) = run_chain(&s, "XYZ", &mut zero).unwrap();
        let b = get(&ls, "OPT-BUTTERFLY");
        assert_eq!((b.tested, b.pass, b.breaks, b.not_tested, b.status), (0, 0, 0, 1, "not tested"));
    }

    #[test]
    fn locked_quote_is_flagged_and_still_used() {
        let s = snap(vec![q(true, 90.0, Some(11.0), Some(11.0)), q(true, 100.0, Some(7.0), Some(7.1))]);
        let (rep, _) = run_chain(&s, "XYZ", &mut zero).unwrap();
        assert_eq!((rep.locked, rep.quotes_used), (1, 2));
    }

    /// The register folder and the code agree: each active file names a check id of the code, each id of the code has an active file.
    #[test]
    fn register_and_code_agree() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("arbitrage").join("register");
        let (mut active, mut planned) = (Vec::new(), Vec::new());
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.file_name().unwrap() == "README.md" || p.extension().is_none_or(|x| x != "md") {
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap();
            let field = |k: &str| text.lines().skip(1).take_while(|l| *l != "---").find_map(|l| l.strip_prefix(&format!("{k}: "))).map(|v| v.trim().to_string());
            let (id, status) = (field("id").unwrap_or_default(), field("status").unwrap_or_default());
            assert_eq!(p.file_stem().unwrap().to_str().unwrap(), id, "file name = id");
            assert!(["hard", "soft"].contains(&field("class").unwrap_or_default().as_str()), "{id}: class");
            match status.as_str() {
                "active" => active.push(id),
                "planned" => planned.push(id),
                s => panic!("{id}: bad status {s}"),
            }
        }
        for id in &active {
            assert!(CHECK_IDS.contains(&id.as_str()), "active {id} is not a check id of the code");
        }
        for id in CHECK_IDS {
            assert!(active.iter().any(|a| a == id), "check id {id} has no active register file");
        }
        for id in &planned {
            assert!(!CHECK_IDS.contains(&id.as_str()), "planned {id} is in the code");
        }
        assert!(planned.len() >= 3);
    }

    #[test]
    fn no_rate_source_stops_and_never_uses_the_network() {
        let dir = std::env::temp_dir().join(format!("fa-arb-norate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut r = cached_rate_fn(&dir, None);
        let e = r("2026-11-20", 0.1).unwrap_err();
        assert!(e.contains("give --rate R") && e.contains("does not download"), "{e}");
        assert_eq!(cached_rate_fn(&dir, Some(0.04))("2026-11-20", 0.1), Ok(0.04));
        // code path: this file calls neither the live rate source nor the downloading rate function
        let src = include_str!("arb_run.rs");
        let code = src.split("#[cfg(test)]").next().unwrap();
        for bad in [["Rates", "::live"].concat(), ["rate", "_fn(repo"].concat(), ["hostnet", "::"].concat()] {
            assert!(!code.contains(&bad), "found {bad}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
