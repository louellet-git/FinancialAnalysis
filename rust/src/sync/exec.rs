//! The executor: one thread and one queue for each host, all hosts at the same time.
//!
//! The executor knows nothing about the network. It takes a `Runner` (the real one calls the existing data commands; a
//! test gives a fake that records the calls). The rules:
//!
//! - the items of one host run one after the other, in the order of the plan (a stable order);
//! - two hosts run at the same time;
//! - a block (HTTP 429, 403, a block page) stops the host for the run: its items are not run, they are counted as blocked;
//!   the other hosts go on; a failure of one item does not stop the others;
//! - the run stops at the stop flag (Ctrl+C) or at the time limit: a running item ends at its next request, the items
//!   that did not run are counted as not tried.

use super::dataset::DataSet;
use super::plan::{Plan, PlanItem};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// New data went into the cache.
    Updated,
    /// The cache had the data (the look at the source found nothing new, or the cache was young at the look).
    Current,
    /// The source has no data for this key (an ETF has no SEC facts; the symbol is unknown).
    NoData,
    Failed,
    /// The source has not published the data yet (a normal state, not a failure; for example FedInvest at the end of the day).
    NotYet,
    /// All sources of the item are stopped by a block.
    Blocked,
    /// The item did not run (stop, time limit).
    NotTried,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JobOutcome {
    pub status: Status,
    /// The source and the numbers for the progress line; the reason of a failure.
    pub note: String,
    /// The requests that the job made.
    pub requests: usize,
    /// The hosts that this job found blocked, with the reason.
    pub blocked: Vec<(String, String)>,
}

impl JobOutcome {
    pub fn new(status: Status, note: impl Into<String>) -> JobOutcome {
        JobOutcome { status, note: note.into(), requests: 0, blocked: Vec::new() }
    }
    pub fn requests(mut self, n: usize) -> JobOutcome {
        self.requests = n;
        self
    }
    pub fn blocked_host(mut self, host: &str, why: &str) -> JobOutcome {
        self.blocked.push((host.to_string(), why.to_string()));
        self
    }
}

pub trait Runner: Sync {
    fn run(&self, item: &PlanItem) -> JobOutcome;
    /// The hosts that are stopped for the run, with the reason (the real runner asks the request queues).
    fn blocked_hosts(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    /// The requests that a host got in the run, when the runner counts them itself (else the sum of the jobs is used).
    fn host_requests(&self, _host: &str) -> Option<usize> {
        None
    }
    /// The time limit has passed: a running item must end at its next request.
    fn request_stop(&self) {}
}

#[derive(Debug, Clone, Default)]
pub struct ExecOptions {
    /// Stop starting items after this time.
    pub time_limit: Option<Duration>,
    /// The seconds between the end of one item and the start of the next item of the same host. The real clients keep
    /// the pause between their own requests; this gap is for a host that makes a new client for each item (the SEC).
    pub job_gap: BTreeMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct ItemResult {
    pub item: PlanItem,
    pub outcome: JobOutcome,
    /// The start and the end of the item, from the start of the run. `None` for an item that did not run.
    pub span: Option<(Duration, Duration)>,
}

#[derive(Debug, Clone, Default)]
pub struct HostReport {
    pub items_run: usize,
    pub requests: usize,
    /// The time that the host thread spent in items.
    pub busy: Duration,
    pub blocked: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    Interrupted,
    TimeLimit,
}

#[derive(Debug, Clone)]
pub struct RunReport {
    pub results: Vec<ItemResult>,
    pub hosts: BTreeMap<String, HostReport>,
    pub stopped: Option<StopReason>,
    pub elapsed: Duration,
}

fn all_blocked(hosts: &[String], blocked: &BTreeMap<String, String>) -> Option<(String, String)> {
    let mut first = None;
    for h in hosts {
        let why = blocked.get(h)?;
        first.get_or_insert((h.clone(), why.clone()));
    }
    first
}

/// Run the plan. `stop` is the flag of Ctrl+C. `progress` gets one text for each finished item.
pub fn execute(plan: &Plan, runner: &dyn Runner, opts: &ExecOptions, stop: &AtomicBool, progress: &(dyn Fn(&str) + Sync)) -> RunReport {
    let start = Instant::now();
    // The queues: the items of each host in the order of the plan.
    let mut queues: BTreeMap<String, Vec<(usize, &PlanItem)>> = BTreeMap::new();
    for (i, it) in plan.items.iter().enumerate() {
        queues.entry(it.hosts[0].clone()).or_default().push((i, it));
    }
    let total = plan.items.len();
    let done = AtomicUsize::new(0);
    let blocked: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());
    let results: Mutex<Vec<(usize, ItemResult)>> = Mutex::new(Vec::new());
    let host_reports: Mutex<BTreeMap<String, HostReport>> = Mutex::new(BTreeMap::new());
    let stop_reason: Mutex<Option<StopReason>> = Mutex::new(None);

    let note_stop = |why: StopReason| {
        let mut s = stop_reason.lock().unwrap_or_else(|e| e.into_inner());
        if s.is_none() {
            *s = Some(why);
        }
    };

    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (host, queue) in &queues {
            let (blocked, results, host_reports, done) = (&blocked, &results, &host_reports, &done);
            let (note_stop, stop) = (&note_stop, stop);
            handles.push(scope.spawn(move || {
                let gap = opts.job_gap.get(host).copied().unwrap_or(0.0);
                let mut report = HostReport::default();
                let mut last_end: Option<Instant> = None;
                for (idx, item) in queue {
                    let not_tried = |note: &str| ItemResult { item: (*item).clone(), outcome: JobOutcome::new(Status::NotTried, note), span: None };
                    // Stop, time limit, blocked hosts: the item does not run.
                    let skip: Option<(Status, String)> = if stop.load(Ordering::SeqCst) {
                        note_stop(StopReason::Interrupted);
                        Some((Status::NotTried, "stopped".into()))
                    } else if opts.time_limit.is_some_and(|l| start.elapsed() >= l) {
                        note_stop(StopReason::TimeLimit);
                        Some((Status::NotTried, "time limit".into()))
                    } else {
                        let b = blocked.lock().unwrap_or_else(|e| e.into_inner());
                        all_blocked(&item.hosts, &b).map(|(h, why)| (Status::Blocked, format!("{h}: {}", crate::hostnet::reason(&why))))
                    };
                    if let Some((status, note)) = skip {
                        let mut r = not_tried(&note);
                        r.outcome.status = status;
                        results.lock().unwrap_or_else(|e| e.into_inner()).push((*idx, r));
                        continue;
                    }
                    if let (Some(end), true) = (last_end, gap > 0.0) {
                        let wait = Duration::from_secs_f64(gap).saturating_sub(end.elapsed());
                        if !wait.is_zero() {
                            std::thread::sleep(wait);
                        }
                    }
                    let t0 = Instant::now();
                    let mut outcome = runner.run(item);
                    let t1 = Instant::now();
                    last_end = Some(t1);
                    // A job that ended because of the stop flag is not a failure.
                    if outcome.status == Status::Failed && stop.load(Ordering::SeqCst) && outcome.note.starts_with("stopped") {
                        outcome.status = Status::NotTried;
                        note_stop(StopReason::Interrupted);
                    }
                    {
                        let mut b = blocked.lock().unwrap_or_else(|e| e.into_inner());
                        for (h, why) in outcome.blocked.iter().cloned().chain(runner.blocked_hosts()) {
                            b.entry(h).or_insert(why);
                        }
                    }
                    report.items_run += 1;
                    report.requests += outcome.requests;
                    report.busy += t1 - t0;
                    let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                    progress(&format!("[{n}/{total}] {} {}: {}{}", item.set.name(), item.key, status_word(outcome.status), if outcome.note.is_empty() { String::new() } else { format!(" ({})", outcome.note) }));
                    results.lock().unwrap_or_else(|e| e.into_inner()).push((*idx, ItemResult { item: (*item).clone(), outcome, span: Some((t0 - start, t1 - start)) }));
                }
                host_reports.lock().unwrap_or_else(|e| e.into_inner()).insert(host.clone(), report);
            }));
        }
        // The time limit also stops a running item (it ends at its next request) through the stop flag of the caller.
        while !handles.iter().all(|h| h.is_finished()) {
            if opts.time_limit.is_some_and(|l| start.elapsed() >= l) {
                runner.request_stop();
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    });

    let mut results = results.into_inner().unwrap_or_default();
    results.sort_by_key(|(i, _)| *i);
    let blocked = blocked.into_inner().unwrap_or_default();
    let mut hosts = host_reports.into_inner().unwrap_or_default();
    for host in queues.keys() {
        hosts.entry(host.clone()).or_default();
    }
    for (h, why) in &blocked {
        hosts.entry(h.clone()).or_default().blocked = Some(why.clone());
    }
    for (h, rep) in hosts.iter_mut() {
        if let Some(n) = runner.host_requests(h) {
            rep.requests = n;
        }
    }
    let stopped = stop_reason.into_inner().unwrap_or(None);
    RunReport { results: results.into_iter().map(|(_, r)| r).collect(), hosts, stopped, elapsed: start.elapsed() }
}

pub fn status_word(s: Status) -> &'static str {
    match s {
        Status::Updated => "updated",
        Status::Current => "current",
        Status::NoData => "no data",
        Status::Failed => "FAILED",
        Status::NotYet => "not available yet",
        Status::Blocked => "BLOCKED",
        Status::NotTried => "not tried",
    }
}

/// The result of one data set.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SetSummary {
    pub updated: usize,
    pub current_before: usize,
    pub current_run: usize,
    pub no_data: usize,
    pub failed: usize,
    pub blocked: usize,
    pub not_tried: usize,
    /// Items that the source has not published yet. They are not failures and not counted for the exit code.
    pub not_yet: usize,
    /// The main reason of the failures, with the count.
    pub reason: Option<(String, usize)>,
    /// The main reason of the blocks, with the count.
    pub blocked_reason: Option<(String, usize)>,
}

impl SetSummary {
    /// Did the data set fail as a whole: items ran, none gave data or current or no-data, and some failed or were blocked?
    pub fn failed_fully(&self) -> bool {
        self.updated + self.current_run + self.no_data == 0 && self.failed + self.blocked > 0
    }
}

/// The summary of each data set of the plan: the items that were current before the run (not in the plan) and the results.
pub fn summarize(plan: &Plan, report: &RunReport) -> BTreeMap<DataSet, SetSummary> {
    let mut out: BTreeMap<DataSet, SetSummary> = BTreeMap::new();
    for (set, c) in &plan.counts {
        out.entry(*set).or_default().current_before = c.current;
    }
    let mut reasons: BTreeMap<DataSet, BTreeMap<String, usize>> = BTreeMap::new();
    let mut block_reasons: BTreeMap<DataSet, BTreeMap<String, usize>> = BTreeMap::new();
    for r in &report.results {
        let s = out.entry(r.item.set).or_default();
        match r.outcome.status {
            Status::Updated => s.updated += 1,
            Status::Current => s.current_run += 1,
            Status::NoData => s.no_data += 1,
            Status::Failed => {
                s.failed += 1;
                *reasons.entry(r.item.set).or_default().entry(short_reason(&r.outcome.note)).or_default() += 1;
            }
            Status::Blocked => {
                s.blocked += 1;
                *block_reasons.entry(r.item.set).or_default().entry(short_reason(&r.outcome.note)).or_default() += 1;
            }
            Status::NotTried => s.not_tried += 1,
            Status::NotYet => s.not_yet += 1,
        }
    }
    let main = |map: BTreeMap<String, usize>| map.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)));
    for (set, map) in reasons {
        out.entry(set).or_default().reason = main(map);
    }
    for (set, map) in block_reasons {
        out.entry(set).or_default().blocked_reason = main(map);
    }
    out
}

/// A failure text without the parts that differ between items (the URL and the ticker), for the count of the main reason.
pub fn short_reason(note: &str) -> String {
    let first: Vec<&str> = note.split_whitespace().collect();
    // `HTTP 404 for https://...` becomes `HTTP 404`; a text of the form `host: reason` keeps the host; the URLs are cut out.
    if first.len() >= 2 && first[0] == "HTTP" && first[1].chars().all(|c| c.is_ascii_digit()) {
        return format!("HTTP {}", first[1]);
    }
    crate::hostnet::plain(note)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::sync::plan::PlanItem;
    use std::collections::HashMap;

    /// One call that the fake saw.
    #[derive(Debug, Clone)]
    pub struct Call {
        pub key: String,
        pub host: String,
        pub start: Instant,
        pub end: Instant,
    }

    /// A runner with no network: it records each call (the time and the order) and answers from a script.
    pub struct Fake {
        pub calls: Mutex<Vec<Call>>,
        /// The answer for (data set, key); the default is `Updated`.
        pub script: HashMap<(DataSet, String), JobOutcome>,
        pub duration: Duration,
        pub blocked: Mutex<Vec<(String, String)>>,
    }

    impl Fake {
        pub fn new(ms: u64) -> Fake {
            Fake { calls: Mutex::new(Vec::new()), script: HashMap::new(), duration: Duration::from_millis(ms), blocked: Mutex::new(Vec::new()) }
        }
        pub fn answer(mut self, set: DataSet, key: &str, o: JobOutcome) -> Fake {
            self.script.insert((set, key.to_string()), o);
            self
        }
        pub fn calls_of(&self, host: &str) -> Vec<Call> {
            self.calls.lock().unwrap().iter().filter(|c| c.host == host).cloned().collect()
        }
    }

    impl Runner for Fake {
        fn run(&self, item: &PlanItem) -> JobOutcome {
            let start = Instant::now();
            std::thread::sleep(self.duration);
            let end = Instant::now();
            self.calls.lock().unwrap().push(Call { key: item.key.clone(), host: item.hosts[0].clone(), start, end });
            let out = self.script.get(&(item.set, item.key.clone())).cloned().unwrap_or_else(|| JobOutcome::new(Status::Updated, "fake").requests(item.requests.max(1)));
            for (h, why) in &out.blocked {
                self.blocked.lock().unwrap().push((h.clone(), why.clone()));
            }
            out
        }
        fn blocked_hosts(&self) -> Vec<(String, String)> {
            self.blocked.lock().unwrap().clone()
        }
    }

    pub fn item(set: DataSet, key: &str, host: &str) -> PlanItem {
        PlanItem { set, key: key.into(), hosts: vec![host.into()], requests: 1, why: "old".into() }
    }

    pub fn plan_of(items: Vec<PlanItem>) -> Plan {
        Plan { items, ..Default::default() }
    }

    fn quiet(_: &str) {}

    #[test]
    fn hosts_run_at_the_same_time_and_one_host_runs_one_item_at_a_time() {
        let mut items = Vec::new();
        for t in ["A", "B", "C"] {
            items.push(item(DataSet::Estimates, t, "nasdaq"));
            items.push(item(DataSet::Rates, t, "treasury"));
            items.push(item(DataSet::Fred, t, "fred"));
        }
        let fake = Fake::new(40);
        let rep = execute(&plan_of(items), &fake, &ExecOptions::default(), &AtomicBool::new(false), &quiet);
        assert_eq!(rep.results.len(), 9);
        // 9 items of 40 ms in 3 queues: about 120 ms, not 360 ms.
        assert!(rep.elapsed < Duration::from_millis(300), "{:?}", rep.elapsed);
        for host in ["nasdaq", "treasury", "fred"] {
            let calls = fake.calls_of(host);
            assert_eq!(calls.len(), 3);
            for pair in calls.windows(2) {
                assert!(pair[1].start >= pair[0].end, "two items of {host} at the same time");
            }
            // The order is the order of the plan.
            assert_eq!(calls.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(), ["A", "B", "C"]);
        }
        // The three hosts overlap in time.
        let first_end = ["nasdaq", "treasury", "fred"].iter().map(|h| fake.calls_of(h)[0].end).min().unwrap();
        let last_start = ["nasdaq", "treasury", "fred"].iter().map(|h| fake.calls_of(h)[0].start).max().unwrap();
        assert!(last_start < first_end, "the first items of the three hosts overlap");
        assert_eq!(rep.hosts["nasdaq"].items_run, 3);
        assert!(rep.stopped.is_none());
    }

    #[test]
    fn a_block_stops_that_host_and_the_other_hosts_finish() {
        let mut items = Vec::new();
        for t in ["A", "B", "C", "D"] {
            items.push(item(DataSet::Prices, t, "yahoo"));
        }
        for t in ["A", "B", "C"] {
            items.push(item(DataSet::Estimates, t, "nasdaq"));
        }
        let fake = Fake::new(5).answer(DataSet::Prices, "B", JobOutcome::new(Status::Blocked, "HTTP 429").blocked_host("yahoo", "HTTP 429 for https://query1.finance.yahoo.com"));
        let rep = execute(&plan_of(items), &fake, &ExecOptions::default(), &AtomicBool::new(false), &quiet);
        let s: Vec<(String, Status)> = rep.results.iter().map(|r| (format!("{} {}", r.item.set.name(), r.item.key), r.outcome.status)).collect();
        assert_eq!(s[0].1, Status::Updated, "A ran before the block");
        assert_eq!(s[1].1, Status::Blocked, "B met the block");
        assert_eq!((s[2].1, s[3].1), (Status::Blocked, Status::Blocked), "C and D were not run");
        assert_eq!(fake.calls_of("yahoo").len(), 2, "no request after the block");
        assert!(s[4..].iter().all(|(_, st)| *st == Status::Updated), "the Nasdaq items all ran");
        assert_eq!(rep.hosts["yahoo"].blocked.as_deref().map(|b| b.starts_with("HTTP 429")), Some(true));
        assert!(rep.hosts["nasdaq"].blocked.is_none());
    }

    #[test]
    fn an_item_with_a_fallback_host_runs_while_only_its_first_host_is_blocked() {
        let mut a = item(DataSet::Prices, "A", "yahoo");
        a.hosts = vec!["yahoo".into(), "nasdaq".into()];
        let mut b = a.clone();
        b.key = "B".into();
        let mut c = a.clone();
        c.key = "C".into();
        let fake = Fake::new(1).answer(DataSet::Prices, "A", JobOutcome::new(Status::Updated, "nasdaq").blocked_host("yahoo", "HTTP 429"));
        let rep = execute(&plan_of(vec![a, b, c]), &fake, &ExecOptions::default(), &AtomicBool::new(false), &quiet);
        assert!(rep.results.iter().all(|r| r.outcome.status == Status::Updated), "the fallback is not a failure");
        assert_eq!(fake.calls_of("yahoo").len(), 3);
        // When both hosts of the item are blocked, the item does not run.
        let mut d = item(DataSet::Prices, "D", "yahoo");
        d.hosts = vec!["yahoo".into(), "nasdaq".into()];
        let mut e = d.clone();
        e.key = "E".into();
        let fake = Fake::new(1).answer(DataSet::Prices, "D", JobOutcome::new(Status::Blocked, "x").blocked_host("yahoo", "HTTP 429").blocked_host("nasdaq", "HTTP 403"));
        let rep = execute(&plan_of(vec![d, e]), &fake, &ExecOptions::default(), &AtomicBool::new(false), &quiet);
        assert_eq!(rep.results[1].outcome.status, Status::Blocked);
        assert_eq!(fake.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_stopped_host_does_not_delay_the_other_hosts_and_a_not_yet_item_is_no_failure() {
        // The yahoo queue stops at its first item; the nasdaq queue of 4 items of 30 ms goes on at the same time.
        let mut items: Vec<PlanItem> = ["A", "B", "C"].iter().map(|t| item(DataSet::Prices, t, "yahoo")).collect();
        items.extend(["A", "B", "C", "D"].iter().map(|t| item(DataSet::Estimates, t, "nasdaq")));
        items.push(item(DataSet::Bonds, "2026-11-19", "treasurydirect"));
        let fake = Fake::new(30)
            .answer(DataSet::Prices, "A", JobOutcome::new(Status::Blocked, "yahoo: HTTP 429").blocked_host("yahoo", "HTTP 429 for https://x"))
            .answer(DataSet::Bonds, "2026-11-19", JobOutcome::new(Status::NotYet, "FedInvest has no end-of-day prices for 2026-11-19 yet"));
        let rep = execute(&plan_of(items.clone()), &fake, &ExecOptions::default(), &AtomicBool::new(false), &quiet);
        assert!(rep.elapsed < Duration::from_millis(250), "{:?}", rep.elapsed);
        assert_eq!(fake.calls_of("nasdaq").len(), 4);
        assert_eq!(fake.calls_of("yahoo").len(), 1);
        let sum = summarize(&plan_of(items), &rep);
        let bonds = &sum[&DataSet::Bonds];
        assert_eq!((bonds.not_yet, bonds.failed, bonds.blocked), (1, 0, 0));
        assert!(!bonds.failed_fully(), "not available yet is not a failure");
        assert!(sum[&DataSet::Prices].failed_fully());
        assert_eq!(status_word(Status::NotYet), "not available yet");
    }

    #[test]
    fn a_failure_of_one_ticker_does_not_stop_the_others() {
        let items: Vec<PlanItem> = ["A", "B", "C"].iter().map(|t| item(DataSet::Prices, t, "nasdaq")).collect();
        let fake = Fake::new(1).answer(DataSet::Prices, "B", JobOutcome::new(Status::Failed, "HTTP 404 for https://x/B"));
        let rep = execute(&plan_of(items), &fake, &ExecOptions::default(), &AtomicBool::new(false), &quiet);
        let st: Vec<Status> = rep.results.iter().map(|r| r.outcome.status).collect();
        assert_eq!(st, [Status::Updated, Status::Failed, Status::Updated]);
        assert!(rep.hosts["nasdaq"].blocked.is_none());
    }

    #[test]
    fn the_job_gap_of_a_host_is_kept() {
        let items: Vec<PlanItem> = ["A", "B", "C"].iter().map(|t| item(DataSet::Sec, t, "sec")).collect();
        let fake = Fake::new(1);
        let mut opts = ExecOptions::default();
        opts.job_gap.insert("sec".into(), 0.05);
        execute(&plan_of(items), &fake, &opts, &AtomicBool::new(false), &quiet);
        let calls = fake.calls_of("sec");
        for pair in calls.windows(2) {
            assert!(pair[1].start - pair[0].end >= Duration::from_millis(45), "{:?}", pair[1].start - pair[0].end);
        }
    }

    #[test]
    fn the_stop_flag_and_the_time_limit_leave_the_rest_not_tried() {
        let items: Vec<PlanItem> = ["A", "B", "C", "D", "E"].iter().map(|t| item(DataSet::Prices, t, "nasdaq")).collect();
        // Time limit: each item takes 30 ms, the limit is 50 ms: two or three items run.
        let fake = Fake::new(30);
        let opts = ExecOptions { time_limit: Some(Duration::from_millis(50)), ..Default::default() };
        let rep = execute(&plan_of(items.clone()), &fake, &opts, &AtomicBool::new(false), &quiet);
        let ran = rep.results.iter().filter(|r| r.outcome.status == Status::Updated).count();
        assert!((1..=3).contains(&ran), "{ran} ran");
        assert_eq!(rep.results.iter().filter(|r| r.outcome.status == Status::NotTried).count(), 5 - ran);
        assert_eq!(rep.stopped, Some(StopReason::TimeLimit));
        // Stop flag set before the run: nothing runs.
        let fake = Fake::new(1);
        let rep = execute(&plan_of(items), &fake, &ExecOptions::default(), &AtomicBool::new(true), &quiet);
        assert!(rep.results.iter().all(|r| r.outcome.status == Status::NotTried));
        assert_eq!(rep.stopped, Some(StopReason::Interrupted));
        assert!(fake.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn summary_counts_and_the_main_reason() {
        let items: Vec<PlanItem> = ["A", "B", "C", "D", "E"].iter().map(|t| item(DataSet::Prices, t, "nasdaq")).collect();
        let fake = Fake::new(0)
            .answer(DataSet::Prices, "B", JobOutcome::new(Status::Failed, "HTTP 404 for https://x/B"))
            .answer(DataSet::Prices, "C", JobOutcome::new(Status::Failed, "HTTP 404 for https://x/C"))
            .answer(DataSet::Prices, "D", JobOutcome::new(Status::NoData, "unknown symbol"))
            .answer(DataSet::Prices, "E", JobOutcome::new(Status::Current, ""));
        let mut plan = plan_of(items);
        plan.counts.push((DataSet::Prices, crate::sync::fresh::Counts { current: 10, old: 4, missing: 1 }));
        let rep = execute(&plan, &fake, &ExecOptions::default(), &AtomicBool::new(false), &quiet);
        let sum = summarize(&plan, &rep);
        let s = &sum[&DataSet::Prices];
        assert_eq!((s.updated, s.current_before, s.current_run, s.no_data, s.failed, s.blocked, s.not_tried), (1, 10, 1, 1, 2, 0, 0));
        assert_eq!(s.reason, Some(("HTTP 404".to_string(), 2)));
        assert!(!s.failed_fully());
        // A set where each item failed or was blocked is a failure; a set with one update is not.
        assert!(SetSummary { failed: 2, ..Default::default() }.failed_fully());
        assert!(SetSummary { blocked: 3, ..Default::default() }.failed_fully());
        assert!(!SetSummary { blocked: 3, updated: 1, ..Default::default() }.failed_fully());
        assert!(!SetSummary::default().failed_fully());
        assert_eq!(short_reason("HTTP 429 for https://a/b?x=1"), "HTTP 429");
    }

    #[test]
    fn progress_has_one_line_for_each_item() {
        let lines: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let items: Vec<PlanItem> = ["A", "B"].iter().map(|t| item(DataSet::Gsw, t, "federalreserve")).collect();
        let fake = Fake::new(0);
        execute(&plan_of(items), &fake, &ExecOptions::default(), &AtomicBool::new(false), &|l: &str| lines.lock().unwrap().push(l.to_string()));
        let l = lines.into_inner().unwrap();
        assert_eq!(l, ["[1/2] gsw A: updated (fake)", "[2/2] gsw B: updated (fake)"]);
    }
}
