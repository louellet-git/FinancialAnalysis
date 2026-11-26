//! One request queue for each host.
//!
//! Each outside site (Nasdaq, Yahoo, Cboe, Treasury, FRED and so on) has one slot here. A request to a host
//! locks the slot of that host, waits until the pause of the caller has passed since the last request to the
//! same host, makes the request, and writes down the time. So:
//!
//! - two requests to one host never run at the same time, whatever the code that asks;
//! - the pause is a pause for each host, not one pause for all hosts: two hosts can run at the same time, in two
//!   threads (this is what `fa sync` does);
//! - the pause of one host is the same as before (each caller keeps the pause that it had).
//!
//! The slot counts the requests and the time of each host. In a run with the gate on (`enable_gate`, set by
//! `fa sync`) the slot also stops a host for the rest of the run after an HTTP 429, an HTTP 403 or a block page:
//! the next request to that host makes no network call and gives the same error text. The gate is off for all
//! other commands, so their behavior has not changed.

use mapcheck::http::Client;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The name that outside sites see.
pub const USER_AGENT: &str = "FinancialAnalysis fa (personal research)";

/// The host groups: (group name, host names). A host that is not here is its own group.
pub const GROUPS: [(&str, &[&str]); 16] = [
    ("sec", &["data.sec.gov", "www.sec.gov", "sec.gov"]),
    ("nasdaq", &["api.nasdaq.com"]),
    ("alpaca", &["data.alpaca.markets", "paper-api.alpaca.markets", "api.alpaca.markets"]),
    ("tiingo", &["api.tiingo.com"]),
    ("yahoo", &["query1.finance.yahoo.com", "query2.finance.yahoo.com"]),
    ("cboe", &["cdn.cboe.com", "www.cboe.com"]),
    ("treasury", &["home.treasury.gov"]),
    ("nyfed", &["markets.newyorkfed.org"]),
    ("fred", &["api.stlouisfed.org", "fred.stlouisfed.org"]),
    ("federalreserve", &["www.federalreserve.gov"]),
    ("bls", &["api.bls.gov"]),
    ("treasurydirect", &["www.treasurydirect.gov", "treasurydirect.gov"]),
    ("fiscaldata", &["api.fiscaldata.treasury.gov"]),
    ("cftc", &["www.cftc.gov", "publicreporting.cftc.gov"]),
    ("philadelphiafed", &["www.philadelphiafed.org"]),
    ("worldbank", &["www.worldbank.org", "thedocs.worldbank.org"]),
];

/// The group of a host name (`api.nasdaq.com` is `nasdaq`).
pub fn group_of(host: &str) -> String {
    let host = host.to_lowercase();
    for (group, hosts) in GROUPS {
        if hosts.contains(&host.as_str()) {
            return group.to_string();
        }
    }
    if host.ends_with("atlantafed.org") {
        return "atlantafed".to_string();
    }
    host
}

/// The host name of a URL (`https://api.nasdaq.com/api/x` gives `api.nasdaq.com`).
pub fn host_of_url(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    host.rsplit('@').next().unwrap_or(host).split(':').next().unwrap_or("").to_string()
}

/// The group of the host of a URL.
pub fn group_of_url(url: &str) -> String {
    group_of(&host_of_url(url))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostStat {
    pub requests: usize,
    /// The time inside the slot: the wait for the pause and the request.
    pub busy: Duration,
    /// The reason, when the gate stopped the host: the code and the text.
    pub blocked: Option<(u16, String)>,
    /// The text of the last error of a request to the host (the keys are taken out).
    pub last_error: Option<String>,
    /// The number of requests that ended with an error.
    pub errors: usize,
}

#[derive(Default)]
struct SlotState {
    client: Option<Client>,
    last: Option<Instant>,
    stat: HostStat,
    /// The connection failures (timeout, closed connection, DNS) in a row since the last answer of the host.
    fail_run: usize,
}

/// With the gate on, a host that fails in this way this many times in a row is stopped for the run, like a block.
pub const FAIL_LIMIT: usize = 2;

/// The time limit of one request, in seconds, for each host group. A host that sends small JSON answers has a short limit;
/// a host that sends a large file (a spreadsheet, a ZIP, a full history) has the old limit of 60 seconds.
pub fn timeout_of(group: &str) -> u64 {
    match group {
        // The Atlanta Fed spreadsheet took 67 seconds in the cloud container (2026-11-20).
        "atlantafed" => 90,
        "federalreserve" | "philadelphiafed" | "worldbank" | "cftc" | "treasury" | "treasurydirect" | "cboe" => 60,
        _ => 30,
    }
}

/// Is the error text a failure of the connection (not an answer of the host)? A timeout, a closed connection, a DNS or a TLS error.
pub fn is_connection_error(error: &str) -> bool {
    if error.starts_with("HTTP ") {
        return false;
    }
    let e = error.to_lowercase();
    ["timeout", "timed out", "connection", "peer closed", "dns", "tls", "unexpected-eof", "refused", "reset", "unreachable", "io:", "proxy"].iter().any(|w| e.contains(w))
}

/// A short reason for a line of the summary: the HTTP code, or the kind of the connection error, with no URL.
pub fn reason(text: &str) -> String {
    let t = text.trim();
    if let Some(i) = t.find("HTTP ") {
        let code: String = t[i + 5..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if !code.is_empty() {
            return format!("HTTP {code}");
        }
    }
    let low = t.to_lowercase();
    let kinds: [(&[&str], &str); 6] = [
        (&["unreachable"], "unreachable"),
        (&["timeout", "timed out"], "timeout"),
        (&["peer closed", "connection reset", "unexpected-eof"], "connection closed"),
        (&["refused"], "connection refused"),
        (&["dns", "resolve"], "DNS failure"),
        (&["tls", "certificate"], "TLS error"),
    ];
    for (words, name) in kinds {
        if words.iter().any(|w| low.contains(w)) {
            return name.to_string();
        }
    }
    plain(t)
}

/// A text with no URL, at most 80 characters (for a note of a progress line).
pub fn plain(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().filter(|w| !w.starts_with("http://") && !w.starts_with("https://")).collect();
    let joined = words.join(" ");
    let cut: String = joined.chars().take(80).collect();
    if joined.chars().count() > 80 { format!("{cut}...") } else { cut }
}

#[derive(Default)]
struct Slot {
    state: Mutex<SlotState>,
}

static GATE: AtomicBool = AtomicBool::new(false);
static STOP: AtomicBool = AtomicBool::new(false);

/// Ask all running and new requests to stop (Ctrl+C or the time limit of `fa sync`). A request that has not started makes no
/// network call and gives an error that starts with `stopped`.
pub fn request_stop() {
    STOP.store(true, Ordering::SeqCst);
}

pub fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

/// Clear the stop request (a new run, a test).
pub fn clear_stop() {
    STOP.store(false, Ordering::SeqCst);
}

fn slots() -> &'static Mutex<BTreeMap<String, Arc<Slot>>> {
    static SLOTS: OnceLock<Mutex<BTreeMap<String, Arc<Slot>>>> = OnceLock::new();
    SLOTS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn slot(group: &str) -> Arc<Slot> {
    let mut all = slots().lock().unwrap_or_else(|e| e.into_inner());
    all.entry(group.to_string()).or_default().clone()
}

fn lock(slot: &Slot) -> std::sync::MutexGuard<'_, SlotState> {
    slot.state.lock().unwrap_or_else(|e| e.into_inner())
}

/// Turn the gate on or off. With the gate on, a block stops the host for the run.
pub fn enable_gate(on: bool) {
    GATE.store(on, Ordering::SeqCst);
}

/// The counts and the block of each host since the start (or since `reset`).
pub fn stats() -> BTreeMap<String, HostStat> {
    let all = slots().lock().unwrap_or_else(|e| e.into_inner());
    all.iter().map(|(k, s)| (k.clone(), lock(s).stat.clone())).collect()
}

/// Clear the counts and the blocks (a new run).
pub fn reset() {
    let all = slots().lock().unwrap_or_else(|e| e.into_inner());
    for s in all.values() {
        let mut st = lock(s);
        st.stat = HostStat::default();
        st.fail_run = 0;
    }
}

/// Stop a host for the run (the code is the HTTP code, 429 or 403).
pub fn block(group: &str, code: u16, reason: &str) {
    let s = slot(group);
    let mut st = lock(&s);
    if st.stat.blocked.is_none() {
        st.stat.blocked = Some((code, reason.to_string()));
    }
}

/// The reason when the host is stopped for the run.
pub fn blocked(group: &str) -> Option<(u16, String)> {
    lock(&slot(group)).stat.blocked.clone()
}

/// Add requests that the slot did not see (a client that has its own network code, for example the SEC client).
pub fn note_requests(group: &str, n: usize) {
    lock(&slot(group)).stat.requests += n;
}

/// The code of an error text of the form `HTTP 429 for <url>`.
pub fn block_code(error: &str) -> Option<u16> {
    let code: u16 = error.strip_prefix("HTTP ")?.split(' ').next()?.parse().ok()?;
    matches!(code, 429 | 403).then_some(code)
}

/// A page of a protection layer (not data): HTML with a typical text. Only the start of the body is read.
pub fn looks_like_block_page(body: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&body[..body.len().min(4096)]).to_lowercase();
    let head = head.trim_start();
    if !(head.starts_with('<') || head.starts_with("\u{feff}<")) {
        return false;
    }
    ["cloudflare", "just a moment", "access denied", "request blocked", "attention required", "captcha", "unusual traffic"].iter().any(|w| head.contains(w))
}

/// Run `f` as the next request to the host group `group`: lock the slot, wait for the pause, run, and count.
/// `f` gets the shared client of the host (no delay of its own). With the gate on, a stopped host gives its error
/// at once, and a block in the answer stops the host.
pub fn paced<R>(group: &str, pause: f64, url_for_errors: &str, f: impl FnOnce(&mut Client) -> Result<R, String>) -> Result<R, String> {
    let s = slot(group);
    let mut st = lock(&s);
    if STOP.load(Ordering::SeqCst) {
        return Err(format!("stopped: the run was interrupted before the request to {url_for_errors}"));
    }
    let gate = GATE.load(Ordering::SeqCst);
    if gate {
        if let Some((code, why)) = &st.stat.blocked {
            if *code == 0 {
                return Err(format!("{why} (the host {group} is stopped for this run; {url_for_errors} not asked)"));
            }
            return Err(format!("HTTP {code} for {url_for_errors} (the host {group} is stopped for this run: {why})"));
        }
    }
    let started = Instant::now();
    if let Some(last) = st.last {
        let wait = Duration::from_secs_f64(pause.max(0.0)).saturating_sub(last.elapsed());
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
    }
    let client = st.client.get_or_insert_with(|| Client::with_timeout(USER_AGENT, 0.0, timeout_of(group)));
    let result = f(client);
    st.last = Some(Instant::now());
    st.stat.requests += 1;
    st.stat.busy += started.elapsed();
    if let Err(e) = &result {
        st.stat.last_error = Some(crate::market::keys::scrub_api_key_param(e));
        st.stat.errors += 1;
    }
    match &result {
        Ok(_) => st.fail_run = 0,
        Err(e) if is_connection_error(e) => st.fail_run += 1,
        // An answer of the host (HTTP 404 and so on) shows that the host is reachable.
        Err(_) => st.fail_run = 0,
    }
    if gate {
        if let Err(e) = &result {
            if let Some(code) = block_code(e) {
                st.stat.blocked = Some((code, crate::market::keys::scrub_api_key_param(e)));
            } else if st.fail_run >= FAIL_LIMIT && st.stat.blocked.is_none() {
                st.stat.blocked = Some((0, format!("unreachable: {} connection failures in a row ({})", st.fail_run, reason(e))));
            }
        }
    }
    result
}

/// GET a URL as the next request to its host. The pause is the pause of the caller.
pub fn get(url: &str, headers: &[(&str, &str)], pause: f64) -> Result<Vec<u8>, String> {
    let group = group_of_url(url);
    let body = paced(&group, pause, url, |c| c.get_with_headers(url, headers).map_err(|e| e.to_string()))?;
    check_block_page(&group, url, body)
}

/// POST a JSON text as the next request to its host.
pub fn post_json(url: &str, body: &str, pause: f64) -> Result<Vec<u8>, String> {
    let group = group_of_url(url);
    let answer = paced(&group, pause, url, |c| c.post_json(url, body).map_err(|e| e.to_string()))?;
    check_block_page(&group, url, answer)
}

fn check_block_page(group: &str, url: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
    if GATE.load(Ordering::SeqCst) && looks_like_block_page(&body) {
        let why = format!("HTTP 403 for {url} (the answer is a block page, not data)");
        block(group, 403, &why);
        return Err(why);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_have_groups() {
        assert_eq!(host_of_url("https://api.nasdaq.com/api/quote/IBM/info?assetclass=stocks"), "api.nasdaq.com");
        assert_eq!(group_of_url("https://api.nasdaq.com/api/x"), "nasdaq");
        assert_eq!(group_of_url("https://query1.finance.yahoo.com/v8/finance/chart/IBM"), "yahoo");
        assert_eq!(group_of_url("https://api.stlouisfed.org/fred/series/observations?api_key=K"), "fred");
        assert_eq!(group_of_url("https://fred.stlouisfed.org/graph/fredgraph.csv?id=X"), "fred");
        assert_eq!(group_of_url("https://www.atlantafed.org/-/media/x.xlsx"), "atlantafed");
        assert_eq!(group_of_url("https://example.org:8080/x"), "example.org");
        assert_eq!(host_of_url("https://user@example.org/x"), "example.org");
    }

    #[test]
    fn block_codes_and_block_pages() {
        assert_eq!(block_code("HTTP 429 for https://x"), Some(429));
        assert_eq!(block_code("HTTP 403 for https://x"), Some(403));
        assert_eq!(block_code("HTTP 404 for https://x"), None);
        assert_eq!(block_code("connection reset"), None);
        assert!(looks_like_block_page(b"<!DOCTYPE html><html><title>Just a moment...</title>Cloudflare"));
        assert!(!looks_like_block_page(b"{\"data\":1}"));
        assert!(!looks_like_block_page(b"<html><body>a normal page</body></html>"));
    }

    #[test]
    fn short_reasons_have_no_url() {
        assert_eq!(reason("HTTP 403 for https://fred.stlouisfed.org/graph/fredgraph.csv?id=AAA10Y"), "HTTP 403");
        assert_eq!(reason("Cboe: HTTP 429 for https://x"), "HTTP 429");
        assert_eq!(reason("https://fred.stlouisfed.org/graph/fredgraph.csv?id=AAA10Y: io: peer closed connection without sending TLS close_notify: https://docs.rs/rustls/x#unexpected-eof"), "connection closed");
        assert_eq!(reason("https://x/y: timeout: global"), "timeout");
        assert_eq!(reason("https://x/y: Dns Failed: resolve dns name 'x'"), "DNS failure");
        assert_eq!(reason("unreachable: 2 connection failures in a row (timeout)"), "unreachable");
        assert_eq!(reason("the file has no header DATE https://x/y"), "the file has no header DATE");
        assert!(reason(&"word ".repeat(40)).chars().count() <= 83);
        assert!(is_connection_error("https://x: timeout: global"));
        assert!(!is_connection_error("HTTP 404 for https://x"));
        assert!(!is_connection_error("no CSRF token on the page"));
    }

    #[test]
    fn each_host_has_a_time_limit_and_a_big_file_host_has_more() {
        assert_eq!(timeout_of("fred"), 30);
        assert_eq!(timeout_of("nasdaq"), 30);
        assert!(timeout_of("cftc") >= 60 && timeout_of("federalreserve") >= 60);
        assert!(["fred", "nasdaq", "yahoo", "cftc", "other"].iter().all(|h| timeout_of(h) <= 60));
        assert_eq!(timeout_of("atlantafed"), 90);
        assert_eq!(group_of_url("https://cdn.cboe.com/api/global/us_indices/daily_prices/VIX_History.csv"), "cboe");
    }

    #[test]
    fn the_pause_is_for_each_host_and_hosts_run_at_the_same_time() {
        let pause = 0.08;
        let t0 = Instant::now();
        let work = |host: &'static str| {
            std::thread::spawn(move || {
                let mut starts = Vec::new();
                for _ in 0..3 {
                    paced(host, pause, "u", |_| {
                        starts.push(t0.elapsed());
                        std::thread::sleep(Duration::from_millis(10));
                        Ok(())
                    })
                    .unwrap();
                }
                starts
            })
        };
        let (a, b) = (work("test-pause-a"), work("test-pause-b"));
        let (sa, sb) = (a.join().unwrap(), b.join().unwrap());
        for s in [&sa, &sb] {
            // The pause counts from the end of the last request: start gap >= pause + request time of the last.
            assert!(s[1] - s[0] >= Duration::from_millis(85), "gap {:?}", s[1] - s[0]);
            assert!(s[2] - s[1] >= Duration::from_millis(85), "gap {:?}", s[2] - s[1]);
        }
        // Two hosts at the same time: 3 requests with a pause of 80 ms take about 0.2 s, not 0.4 s.
        let total = t0.elapsed();
        assert!(total < Duration::from_millis(340), "{total:?}");
        // The first requests of the two hosts started at nearly the same time.
        let diff = sa[0].abs_diff(sb[0]);
        assert!(diff < Duration::from_millis(50), "{diff:?}");
        let st = stats();
        assert_eq!(st["test-pause-a"].requests, 3);
        assert!(st["test-pause-a"].busy >= Duration::from_millis(30));
    }

    #[test]
    fn two_requests_to_one_host_never_overlap() {
        use std::sync::atomic::AtomicUsize;
        let inside = Arc::new(AtomicUsize::new(0));
        let worst = Arc::new(AtomicUsize::new(0));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let (inside, worst) = (inside.clone(), worst.clone());
                std::thread::spawn(move || {
                    for _ in 0..3 {
                        paced("test-overlap", 0.0, "u", |_| {
                            let n = inside.fetch_add(1, Ordering::SeqCst) + 1;
                            worst.fetch_max(n, Ordering::SeqCst);
                            std::thread::sleep(Duration::from_millis(3));
                            inside.fetch_sub(1, Ordering::SeqCst);
                            Ok(())
                        })
                        .unwrap();
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(worst.load(Ordering::SeqCst), 1);
        assert_eq!(stats()["test-overlap"].requests, 12);
    }

    #[test]
    fn the_gate_stops_a_host_after_a_block_and_not_before() {
        // The gate is global: only this test turns it on, with host names of its own.
        enable_gate(true);
        let calls = std::cell::Cell::new(0);
        let first: Result<(), String> = paced("test-gate", 0.0, "https://x/y", |_| {
            calls.set(calls.get() + 1);
            Err("HTTP 429 for https://x/y?api_key=SECRET1234".to_string())
        });
        assert!(first.is_err());
        let second: Result<(), String> = paced("test-gate", 0.0, "https://x/z", |_| {
            calls.set(calls.get() + 1);
            Ok(())
        });
        let text = second.unwrap_err();
        assert!(text.starts_with("HTTP 429 for https://x/z"), "{text}");
        assert!(!text.contains("SECRET1234"), "{text}");
        assert_eq!(calls.get(), 1, "the second request made no call");
        assert_eq!(blocked("test-gate").map(|b| b.0), Some(429));
        // Another host is not stopped.
        assert!(paced("test-gate-other", 0.0, "u", |_| Ok(5)).is_ok());
        // A failure that is not a block does not stop the host.
        let _ = paced("test-gate-404", 0.0, "u", |_| Err::<(), _>("HTTP 404 for u".to_string()));
        assert!(paced("test-gate-404", 0.0, "u", |_| Ok(1)).is_ok());
        // N connection failures in a row stop the host for the run (a block of the same kind).
        let conn = "https://x/s?id=1: io: peer closed connection without sending TLS close_notify";
        let calls = std::cell::Cell::new(0);
        let ask = |host: &str, r: Result<(), String>| {
            let out = paced(host, 0.0, "https://x/s", |_| {
                calls.set(calls.get() + 1);
                r
            });
            out.unwrap_err()
        };
        ask("test-unreach", Err(conn.into()));
        assert!(blocked("test-unreach").is_none(), "one failure does not stop the host");
        ask("test-unreach", Err(conn.into()));
        let (code, why) = blocked("test-unreach").expect("two failures in a row stop the host");
        assert_eq!(code, 0);
        assert!(why.starts_with("unreachable: 2 connection failures in a row"), "{why}");
        let third = ask("test-unreach", Err("never".into()));
        assert!(third.contains("stopped for this run"), "{third}");
        assert_eq!(calls.get(), 2, "the third request made no call");
        assert_eq!(stats()["test-unreach"].errors, 2, "the errors are counted for the check of a function that falls back to an old file");
        // An answer of the host between two failures resets the count; so does a success.
        ask("test-reset", Err(conn.into()));
        ask("test-reset", Err("HTTP 404 for https://x/s".into()));
        ask("test-reset", Err(conn.into()));
        assert!(blocked("test-reset").is_none());
        assert!(paced("test-reset", 0.0, "u", |_| Ok(1)).is_ok());
        ask("test-reset", Err(conn.into()));
        assert!(blocked("test-reset").is_none());
        // An error that is not about the connection (a parse text) does not count.
        ask("test-text", Err("no CSRF token on the page".into()));
        ask("test-text", Err("no CSRF token on the page".into()));
        assert!(blocked("test-text").is_none());
        enable_gate(false);
        // With the gate off the slot does not keep a block (the old behavior of all other commands).
        let _ = paced("test-nogate", 0.0, "u", |_| Err::<(), _>("HTTP 429 for u".to_string()));
        assert!(paced("test-nogate", 0.0, "u", |_| Ok(1)).is_ok());
    }
}
