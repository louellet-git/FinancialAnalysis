//! Tests of the command with no network: the plan, `--status`, `--dry-run`, the summary, a stop and a second run. The runner is
//! a fake that writes the cache files that the real job would write and records each call.

use super::exec::{JobOutcome, Runner, Status};
use super::fresh::tests::{temp, write_prices};
use super::plan::{KeyInfo, PlanItem};
use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// A runner that writes the cache of a ticker for prices (as the real job does) and counts its calls. It can set the stop
/// flag after some calls (a Ctrl+C), fail some keys, and block a host.
struct CachingFake {
    cache: std::path::PathBuf,
    today: String,
    calls: Mutex<Vec<(DataSet, String)>>,
    count: AtomicUsize,
    stop_after: Option<usize>,
    stop: &'static AtomicBool,
    fail: Vec<(DataSet, String, String)>,
    block: Option<(String, DataSet)>,
    not_yet: Vec<(DataSet, String)>,
    /// (data set, N): the Nth call of the data set fails and stops its host, as the gate of `hostnet` does.
    unreachable: Option<(DataSet, usize)>,
}

struct Shared(Arc<CachingFake>);

impl Runner for Shared {
    fn run(&self, item: &PlanItem) -> JobOutcome {
        let f = &self.0;
        f.calls.lock().unwrap().push((item.set, item.key.clone()));
        let n = f.count.fetch_add(1, Ordering::SeqCst) + 1;
        if f.stop_after == Some(n) {
            f.stop.store(true, Ordering::SeqCst);
        }
        if let Some((host, set)) = &f.block {
            if item.set == *set {
                return JobOutcome::new(Status::Blocked, "HTTP 429").blocked_host(host, "HTTP 429 for https://x");
            }
        }
        if f.not_yet.iter().any(|(s, k)| *s == item.set && *k == item.key) {
            return JobOutcome::new(Status::NotYet, "FedInvest has no end-of-day prices yet").requests(4);
        }
        if let Some((set, n)) = &f.unreachable {
            if item.set == *set {
                let seen = f.calls.lock().unwrap().iter().filter(|(s, _)| s == set).count();
                let host = item.hosts[0].clone();
                let why = "unreachable: 2 connection failures in a row (connection closed)";
                return if seen >= *n { JobOutcome::new(Status::Blocked, "fred: unreachable").blocked_host(&host, why) } else { JobOutcome::new(Status::Failed, "connection closed") };
            }
        }
        if let Some((_, _, why)) = f.fail.iter().find(|(s, k, _)| *s == item.set && *k == item.key) {
            return JobOutcome::new(Status::Failed, why.clone());
        }
        if item.set == DataSet::Prices {
            write_prices(&f.cache, &item.key, &f.today, &f.today);
        }
        if item.set == DataSet::Gsw {
            let p = f.cache.join("rates").join("gsw_feds200628.csv");
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "x").unwrap();
        }
        JobOutcome::new(Status::Updated, "fake").requests(1)
    }
}

struct Bench {
    repo: std::path::PathBuf,
    ctx: Ctx,
    stop: &'static AtomicBool,
}

impl Bench {
    fn new(name: &str, today: &str) -> Bench {
        let repo = temp(name);
        let mut ctx = Ctx::new(&repo);
        ctx.today = mapcheck::dates::parse(today).unwrap();
        // The flag lives as long as the program: a test leaks one bool.
        let stop: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));
        Bench { repo, ctx, stop }
    }

    fn setup(&self, env: EnvInfo, cfg: SyncConfig) -> RunSetup<'_> {
        RunSetup { repo: &self.repo, ctx: self.ctx.clone(), cfg, env, stop: self.stop, job_gap: BTreeMap::new() }
    }

    fn fake(&self) -> CachingFake {
        CachingFake { cache: self.ctx.cache.clone(), today: self.ctx.today_text(), calls: Mutex::new(vec![]), count: AtomicUsize::new(0), stop_after: None, stop: self.stop, fail: vec![], block: None, not_yet: vec![], unreachable: None }
    }

    /// Run one command line with a fake. Gives the text, the exit code and the fake.
    fn run(&self, line: &[&str], env: EnvInfo, fake: CachingFake) -> (String, i32, Arc<CachingFake>) {
        let args = parse_args(&v(line)).unwrap();
        let fake = Arc::new(fake);
        let f2 = fake.clone();
        let make = move |_: &[String], _: &Ledger| -> Box<dyn Runner> { Box::new(Shared(f2.clone())) };
        let mut text = String::new();
        let code = run_with(&args, &self.setup(env, SyncConfig::default()), &make, &mut |t| text.push_str(t), &|_| {}).unwrap();
        (norm(&text), code, fake)
    }
}

/// The text with each run of spaces cut to one space (the columns of the tables do not matter for a test).
fn norm(text: &str) -> String {
    text.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n")
}

fn env(sec: bool, keys: bool) -> EnvInfo {
    EnvInfo { sec_contact: if sec { Ok("file sec_contact.toml".to_string()) } else { Err("no SEC contact found (test)".to_string()) }, keys: KeyInfo { tiingo: keys, alpaca: keys, fred: keys, fmp: keys } }
}

#[test]
fn arguments() {
    let a = parse_args(&v(&["ibm", "MSFT", "--only", "prices,estimates", "--skip", "estimates", "--force", "--max-minutes", "2.5", "--tickers-file", "x.txt", "--option-symbols", "spy,qqq"])).unwrap();
    assert_eq!(a.tickers, ["IBM", "MSFT"]);
    assert_eq!(selected_sets(&a), [DataSet::Prices]);
    assert!(a.force && !a.dry_run && !a.status);
    assert_eq!(a.max_minutes, Some(2.5));
    assert_eq!(a.option_symbols, Some(v(&["SPY", "QQQ"])));
    assert_eq!(a.echo, ["IBM", "MSFT", "--tickers-file", "x.txt", "--option-symbols", "spy,qqq"]);
    let a = parse_args(&v(&["--only", "bonds", "--bond-days", "5"])).unwrap();
    assert_eq!(a.bond_days, Some(5));
    assert_eq!(a.echo, ["--bond-days", "5"]);
    assert_eq!(parse_args(&v(&["--force"])).unwrap().bond_days, None);
    for bad in [&["--bond-days"][..], &["--bond-days", "0"], &["--bond-days", "x"], &["--bond-days", "501"], &["--bond-days", "-1"]] {
        assert!(parse_args(&v(bad)).is_err(), "{bad:?}");
    }
    // Skip alone: all data sets but those.
    let a = parse_args(&v(&["--skip", "options,intraday"])).unwrap();
    assert_eq!(selected_sets(&a).len(), ALL.len() - 2);
    for bad in [&["--only", "nope"][..], &["--bogus"], &["--only"], &["--max-minutes", "0"], &["--max-minutes", "x"], &["../x"], &["--status", "--force"], &["--only", "prices", "--skip", "prices"]] {
        assert!(parse_args(&v(bad)).is_err(), "{bad:?}");
    }
}

#[test]
fn the_ticker_list_is_the_named_tickers_then_the_file_then_the_default_file_of_the_config() {
    let b = Bench::new("tickers", "2026-11-19");
    std::fs::write(b.repo.join("list.txt"), "# list\nAAA 1 # x\nBBB 2\nIBM 3\n").unwrap();
    let cfg = SyncConfig::default();
    let t = |line: &[&str], cfg: &SyncConfig| resolve_tickers(&b.repo, &parse_args(&v(line)).unwrap(), cfg).unwrap();
    assert_eq!(t(&["IBM", "--tickers-file", "list.txt"], &cfg), ["IBM", "AAA", "BBB"]);
    assert!(t(&[], &cfg).is_empty());
    let with_default = SyncConfig::parse("tickers_file = \"list.txt\"").unwrap();
    assert_eq!(t(&[], &with_default), ["AAA", "BBB", "IBM"]);
    assert_eq!(t(&["MSFT"], &with_default), ["MSFT"], "named tickers replace the default file");
    assert!(resolve_tickers(&b.repo, &parse_args(&v(&["--tickers-file", "missing.txt"])).unwrap(), &cfg).is_err());
}

#[test]
fn status_and_dry_run_make_no_request() {
    let b = Bench::new("status", "2026-11-21"); // a Saturday
    write_prices(&b.ctx.cache, "IBM", "2026-11-20", "2026-11-20");
    write_prices(&b.ctx.cache, "MSFT", "2026-11-12", "2026-11-12");
    let built = Arc::new(AtomicUsize::new(0));
    for line in [&["--status", "IBM", "MSFT", "NVDA", "--only", "prices,estimates"][..], &["--dry-run", "IBM", "MSFT", "NVDA", "--only", "prices,estimates"]] {
        let args = parse_args(&v(line)).unwrap();
        let b2 = built.clone();
        let make = move |_: &[String], _: &Ledger| -> Box<dyn Runner> {
            b2.fetch_add(1, Ordering::SeqCst);
            panic!("no runner may be built for a status or a dry run");
        };
        let mut text = String::new();
        let code = run_with(&args, &b.setup(env(true, false), SyncConfig::default()), &make, &mut |t| text.push_str(t), &|_| panic!("no progress line")).unwrap();
        assert_eq!(code, 0);
        let text = norm(&text);
        assert!(text.contains("prices"), "{text}");
        if args.status {
            // IBM (Friday data on Saturday) is current; MSFT is old; NVDA has no data. Estimates: all three have no data.
            assert!(text.contains("prices 1 current, 1 old, 1 no data"), "{text}");
            assert!(text.contains("estimates 0 current, 0 old, 3 no data (max age 7 days"), "{text}");
            assert!(text.contains("max age 1 market day"), "{text}");
            assert!(text.contains("oldest record 2026-11-12"), "{text}");
        } else {
            assert!(text.contains("dry run: no request is made"), "{text}");
            assert!(text.contains("nasdaq") && text.contains("request(s)") && text.contains("estimated time of the run"), "{text}");
            // prices: MSFT (cache of nasdaq) 1 request and NVDA (yahoo first) 1 request; estimates: 3 tickers x 3 requests on nasdaq.
            assert!(text.contains("nasdaq 10 request(s)"), "{text}");
            assert!(text.contains("yahoo 1 request(s)"), "{text}");
        }
    }
    assert_eq!(built.load(Ordering::SeqCst), 0);
}

#[test]
fn a_run_updates_the_old_items_and_a_second_run_asks_for_nothing() {
    let b = Bench::new("second", "2026-11-19");
    write_prices(&b.ctx.cache, "IBM", "2026-11-19", "2026-11-19"); // current
    write_prices(&b.ctx.cache, "MSFT", "2026-11-10", "2026-11-10"); // old
    let (text, code, fake) = b.run(&["IBM", "MSFT", "NVDA", "--only", "prices"], env(true, false), b.fake());
    assert_eq!(code, 0, "{text}");
    let mut calls: Vec<String> = fake.calls.lock().unwrap().iter().map(|(_, k)| k.clone()).collect();
    calls.sort();
    assert_eq!(calls, ["MSFT", "NVDA"], "IBM was current");
    assert!(text.contains("prices 2 updated, 1 current, 0 no data, 0 failed, 0 blocked, 0 not tried"), "{text}");
    assert!(text.contains("hosts:") && text.contains("nasdaq"), "{text}");
    assert!(!text.contains("stays old"), "{text}");
    // The second run: no item, so no call.
    let (text2, code2, fake2) = b.run(&["IBM", "MSFT", "NVDA", "--only", "prices"], env(true, false), b.fake());
    assert_eq!(code2, 0);
    assert!(fake2.calls.lock().unwrap().is_empty(), "the second run made a request: {:?}", fake2.calls.lock().unwrap());
    assert!(text2.contains("0 updated, 3 current"), "{text2}");
    // --force asks for all three again.
    let (_, _, fake3) = b.run(&["IBM", "MSFT", "NVDA", "--only", "prices", "--force"], env(true, false), b.fake());
    assert_eq!(fake3.calls.lock().unwrap().len(), 3);
}

#[test]
fn a_stop_leaves_the_rest_old_and_the_next_run_continues() {
    let b = Bench::new("stop", "2026-11-19");
    let tickers = ["AAA", "BBB", "CCC", "DDD", "EEE"];
    let mut line = tickers.to_vec();
    line.extend(["--only", "prices"]);
    let mut fake = b.fake();
    fake.stop_after = Some(2); // a Ctrl+C after the second item
    let (text, code, fake) = b.run(&line, env(true, false), fake);
    assert_eq!(code, 0, "a stop is no failure: {text}");
    assert_eq!(fake.calls.lock().unwrap().len(), 2);
    assert!(text.contains("2 updated, 0 current, 0 no data, 0 failed, 0 blocked, 3 not tried"), "{text}");
    assert!(text.contains("stopped by Ctrl+C") && text.contains("stays old") && text.contains("CCC DDD EEE"), "{text}");
    assert!(text.contains("run again: fa sync AAA BBB CCC DDD EEE --only prices"), "{text}");
    // The next run (the flag cleared) does the three that are left and no more.
    b.stop.store(false, Ordering::SeqCst);
    let (_, _, fake2) = b.run(&line, env(true, false), b.fake());
    let calls: Vec<String> = fake2.calls.lock().unwrap().iter().map(|(_, k)| k.clone()).collect();
    assert_eq!(calls, ["CCC", "DDD", "EEE"]);
}

#[test]
fn a_failed_ticker_is_in_the_summary_and_the_exit_code_depends_on_the_whole_data_set() {
    let b = Bench::new("fail", "2026-11-19");
    let mut fake = b.fake();
    fake.fail.push((DataSet::Prices, "BAD".into(), "HTTP 404 for https://x/BAD".into()));
    let (text, code, _) = b.run(&["IBM", "BAD", "MSFT", "--only", "prices"], env(true, false), fake);
    assert_eq!(code, 0, "one failed ticker does not fail the data set: {text}");
    assert!(text.contains("2 updated, 0 current, 0 no data, 1 failed (HTTP 404), 0 blocked"), "{text}");
    assert!(text.contains("stays old:\nprices BAD"), "{text}");
    // All items fail: the data set failed, exit code 1.
    let mut fake = b.fake();
    fake.fail.push((DataSet::Prices, "A1".into(), "HTTP 500".into()));
    fake.fail.push((DataSet::Prices, "A2".into(), "HTTP 500".into()));
    let (text, code, _) = b.run(&["A1", "A2", "--only", "prices"], env(true, false), fake);
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("failed data set(s): prices"), "{text}");
}

#[test]
fn a_block_of_one_host_does_not_fail_the_other_data_sets() {
    let b = Bench::new("block", "2026-11-19");
    let mut fake = b.fake();
    fake.block = Some(("yahoo".into(), DataSet::Intraday));
    let (text, code, _) = b.run(&["IBM", "--only", "prices,intraday,gsw"], env(true, false), fake);
    // The intraday data set is blocked as a whole (its only source stopped): that is a failure of that data set.
    assert_eq!(code, 1, "{text}");
    assert!(text.contains("prices 1 updated"), "{text}");
    assert!(text.contains("gsw 1 updated"), "{text}");
    assert!(text.contains("intraday") && text.contains("blocked"), "{text}");
    assert!(text.contains("yahoo") && text.contains("BLOCKED for this run"), "{text}");
}

#[test]
fn without_the_sec_contact_the_sec_sets_are_skipped_with_a_line_and_the_rest_runs() {
    let b = Bench::new("nosec", "2026-11-19");
    let (text, code, fake) = b.run(&["IBM", "--only", "sec,filings,prices"], env(false, false), b.fake());
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("sec skipped: no SEC contact found (test)"), "{text}");
    assert!(text.contains("filings skipped: no SEC contact found (test)"), "{text}");
    assert!(text.contains("prices 1 updated"), "{text}");
    let calls = fake.calls.lock().unwrap();
    assert!(calls.iter().all(|(s, _)| *s == DataSet::Prices));
    // With the contact the sec data set is planned.
    let (_, _, fake2) = b.run(&["IBM", "--only", "sec"], env(true, false), b.fake());
    assert_eq!(fake2.calls.lock().unwrap().len(), 1);
}

#[test]
fn no_key_says_which_key_adds_which_source_and_a_key_removes_the_line() {
    let b = Bench::new("keys", "2026-11-19");
    let (text, _, _) = b.run(&["IBM", "--only", "prices,fred,estimates"], env(true, false), b.fake());
    assert!(text.contains("key: APCA_API_KEY_ID and APCA_API_SECRET_KEY would add Alpaca"), "{text}");
    assert!(text.contains("key: TIINGO_API_KEY would add Tiingo") && text.contains("key: FRED_API_KEY would add the FRED API"), "{text}");
    let (text2, _, _) = b.run(&["IBM", "--only", "prices,fred,estimates", "--force"], env(true, true), b.fake());
    assert!(!text2.contains("key: "), "{text2}");
    // The text never has a value of a key: it has only the names.
    assert!(!text.contains("secret") || text.contains("APCA_API_SECRET_KEY"));
}

#[test]
fn the_config_changes_the_age() {
    let b = Bench::new("cfg", "2026-11-23"); // a Monday
    write_prices(&b.ctx.cache, "IBM", "2026-11-20", "2026-11-20"); // Friday data: one market day old on Monday
    let status = |cfg: SyncConfig| {
        let args = parse_args(&v(&["--status", "IBM", "--only", "prices"])).unwrap();
        let mut text = String::new();
        run_with(&args, &b.setup(env(true, false), cfg), &|_: &[String], _: &Ledger| -> Box<dyn Runner> { unreachable!() }, &mut |t| text.push_str(t), &|_| {}).unwrap();
        text
    };
    assert!(norm(&status(SyncConfig::default())).contains("0 current, 1 old"));
    let longer = SyncConfig::parse("[max_age_days]\nprices = 2").unwrap();
    let text = norm(&status(longer));
    assert!(text.contains("1 current, 0 old") && text.contains("max age 2 market days"), "{text}");
}


#[test]
fn the_bond_days_flag_wins_over_the_config_file() {
    let b = Bench::new("bonddays-flag", "2026-11-23"); // a Monday; no FedInvest file in the cache
    let status = |extra: &[&str], cfg: SyncConfig| {
        let mut a = vec!["--status", "--only", "bonds"];
        a.extend_from_slice(extra);
        let args = parse_args(&v(&a)).unwrap();
        let mut text = String::new();
        run_with(&args, &b.setup(env(true, false), cfg), &|_: &[String], _: &Ledger| -> Box<dyn Runner> { unreachable!() }, &mut |t| text.push_str(t), &|_| {}).unwrap();
        norm(&text)
    };
    let file = || SyncConfig::parse("bond_days = 3").unwrap();
    assert!(status(&[], SyncConfig::default()).contains("1 no data"));
    let text = status(&[], file());
    assert!(text.contains("3 no data"), "the file gives 3 days: {text}");
    let text = status(&["--bond-days", "5"], file());
    assert!(text.contains("5 no data"), "the flag wins over the file: {text}");
}
#[test]
fn the_rates_data_set_of_a_weekend_is_current_after_a_friday_run() {
    let b = Bench::new("weekend", "2026-11-21");
    let p = b.ctx.cache.join("rates").join("gsw_feds200628.csv");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "x").unwrap();
    // The file has the date of today of the machine (the test cannot set the date of a file): the ledger gives the date.
    let mut ledger = Ledger::load(&b.ctx.cache);
    ledger.set("gsw", "gsw", "2026-11-20");
    ledger.save().unwrap();
    let (_, _, fake) = b.run(&["--only", "gsw"], env(true, false), b.fake());
    assert!(fake.calls.lock().unwrap().is_empty(), "Friday data is current on a Saturday");
}

#[test]
fn a_not_available_yet_item_is_no_failure_and_does_not_set_the_exit_code() {
    let b = Bench::new("notyet", "2026-11-19");
    let mut fake = b.fake();
    fake.not_yet.push((DataSet::Bonds, "2026-11-18".into()));
    let (text, code, _) = b.run(&["--only", "bonds"], env(true, false), fake);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("bonds 0 updated, 0 current, 0 no data, 0 failed, 0 blocked, 0 not tried, 1 not available yet"), "{text}");
    assert!(text.contains("stays old:\nbonds 2026-11-18"), "{text}");
    assert!(!text.contains("failed data set"), "{text}");
}

#[test]
fn a_host_that_fails_in_the_same_way_in_sequence_is_reported_one_time_and_its_items_are_blocked() {
    let b = Bench::new("unreach", "2026-11-19");
    let mut fake = b.fake();
    // The fake answers like the real runner: the second failure of the host stops it; the later items are blocked.
    fake.unreachable = Some((DataSet::Fred, 2));
    let (text, code, fake) = b.run(&["--only", "fred,gsw"], env(true, false), fake);
    assert_eq!(code, 1, "{text}");
    let fred_calls = fake.calls.lock().unwrap().iter().filter(|(s, _)| *s == DataSet::Fred).count();
    assert_eq!(fred_calls, 2, "no request after the second failure");
    assert!(text.contains("gsw 1 updated"), "the other host finishes: {text}");
    assert!(text.contains("fred 0 updated") && text.contains("failed data set(s): fred"), "{text}");
    assert!(text.contains("BLOCKED for this run (unreachable") , "{text}");
    assert!(!text.contains("https://"), "the summary has no URL: {text}");
}

#[test]
fn the_status_line_names_the_place_of_the_sec_contact_and_never_the_contact() {
    let line = env(true, false).describe();
    assert!(line.starts_with("SEC contact: file sec_contact.toml;"), "{line}");
    let variable = EnvInfo { sec_contact: Ok("SEC_USER_AGENT".to_string()), keys: KeyInfo::default() }.describe();
    assert!(variable.starts_with("SEC contact: SEC_USER_AGENT;"), "{variable}");
    assert!(!variable.contains('@'));
    assert!(env(false, false).describe().starts_with("SEC contact: none;"));
}
