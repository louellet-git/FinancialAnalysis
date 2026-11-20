//! `fa serve`: a local web viewer. One HTML page (inside the program) and JSON
//! endpoints that call the same functions as the other `fa` commands.
//!
//! The server binds to 127.0.0.1 only. Each request runs in its own thread, so a
//! slow call (Monte Carlo, a data site that does not answer) does not stop the page.

use financial_analysis::{Fa, FaError, VerdictOptions};
use mapcheck::registry::Registry;
use mapcheck::run::{check_cached, facts_path, report_json, values_json, TickerReport};
use mapcheck::{dates, sec};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Arc;

const PAGE: &str = include_str!("viewer.html");
const DEFAULT_PORT: u16 = 8765;
/// Ports to try after the default port when it is in use (only without `--port`).
const PORT_TRIES: u16 = 20;
/// Fiscal years that the mapping keeps (the same default as `mapcheck values`).
const PERIODS: usize = 12;
const NYFED_RATES: [&str; 5] = ["SOFR", "EFFR", "OBFR", "TGCR", "BGCR"];

pub const USAGE: &str = "\
fa serve [--port N] [--no-open]
  Start the local viewer on http://127.0.0.1:N/ (default port 8765).
  The program opens the page in the default browser. Use --no-open to stop this.
  Push Ctrl+C to stop the server.";

/// Shared state of the server.
pub struct Ctx {
    pub fa: Fa,
    /// The SEC cache of mapcheck (`<repo>/data/raw`).
    pub cache: PathBuf,
    pub registry: Registry,
    /// Gets a web address (the price history of a report, when the cache has none). Tests set a function that does not use the network.
    pub fetch: fn(&str) -> Result<Vec<u8>, String>,
}

impl Ctx {
    pub fn new(fa: Fa) -> Result<Ctx, String> {
        let cache = fa.repo.join("data").join("raw");
        Ok(Ctx { fa, cache, registry: Registry::embedded()?, fetch: financial_analysis::market::download })
    }
}

#[derive(Debug)]
pub struct Reply {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
    /// A file that the server sends in place of `body` (it reads the file in parts).
    pub file: Option<PathBuf>,
}

impl Reply {
    fn json<T: Serialize>(value: &T) -> Reply {
        match serde_json::to_string(value) {
            Ok(body) => Reply { status: 200, content_type: "application/json; charset=utf-8", body, file: None },
            Err(e) => Reply::error(500, &format!("cannot write JSON: {e}"), ""),
        }
    }

    fn error(status: u16, message: &str, hint: &str) -> Reply {
        Reply {
            status,
            content_type: "application/json; charset=utf-8",
            body: json!({"error": message, "hint": hint}).to_string(),
            file: None,
        }
    }
}

type Answer = std::result::Result<Reply, Reply>;

// ---------------------------------------------------------------------------
// Routing (no network in this part)
// ---------------------------------------------------------------------------

/// Decode `%XX` and `+` in a query value.
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                match hex {
                    Some(b) => {
                        out.push(b);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Split a request URL into the path and the query values.
pub fn split_url(url: &str) -> (String, HashMap<String, String>) {
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let mut values = HashMap::new();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        values.insert(percent_decode(k), percent_decode(v));
    }
    (path.to_string(), values)
}

/// A ticker in upper case: 1 to 10 letters, digits, '.' or '-', and a letter or a
/// digit first. This also prevents a path out of the cache folder and an option for
/// a command.
pub fn valid_ticker(text: Option<&String>) -> Result<String, Reply> {
    let t = text.map(|s| s.trim().to_uppercase()).unwrap_or_default();
    let ok = !t.is_empty()
        && t.len() <= 10
        && t.chars().next().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false)
        && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    if ok {
        Ok(t)
    } else {
        Err(Reply::error(400, "The ticker is not valid.", "Type a ticker, for example IBM. Use letters, digits, '.' or '-'."))
    }
}

/// The Host header must name this computer. This stops a web site that uses a DNS
/// name for 127.0.0.1 (DNS rebinding) from reading the data.
pub fn host_allowed(host: Option<&str>, port: u16) -> bool {
    match host {
        None => true,
        Some(h) => ["127.0.0.1", "localhost"].iter().any(|name| h == *name || h == format!("{name}:{port}")),
    }
}

fn number<T: std::str::FromStr>(q: &HashMap<String, String>, key: &str, default: T) -> Result<T, Reply> {
    match q.get(key).map(|s| s.trim()).filter(|s| !s.is_empty()) {
        None => Ok(default),
        Some(s) => s.parse().map_err(|_| Reply::error(400, &format!("'{key}' must be a number, not '{s}'."), "")),
    }
}

pub fn route(ctx: &Ctx, method: &str, url: &str) -> Reply {
    if method != "GET" && method != "HEAD" {
        return Reply::error(405, "Only GET is permitted.", "");
    }
    let (path, q) = split_url(url);
    let answer: Answer = match path.as_str() {
        "/" | "/index.html" => Ok(Reply { status: 200, content_type: "text/html; charset=utf-8", body: PAGE.to_string(), file: None }),
        "/api/report/info" => report_info(ctx, &q),
        "/api/report/make" => report_make(ctx, &q),
        "/report/file" => report_file(ctx, &q),
        "/api/tickers" => tickers(&ctx.cache),
        "/api/stock" => stock(ctx, &q),
        "/api/models" => Ok(Reply::json(&models(&ctx.fa.repo))),
        "/api/verdict" => verdict(ctx, &q),
        "/api/rates/curve" => {
            let real = q.get("real").map(|v| v == "1" || v == "true").unwrap_or(false);
            ctx.fa.curve(None, real).map_err(fa_error).map(|c| {
                Reply::json(&json!({"curve": &c, "ordered": c.ordered()}))
            })
        }
        "/api/rates/nyfed" => {
            let rate = q.get("rate").map(|r| r.to_uppercase()).unwrap_or_else(|| "SOFR".into());
            if !NYFED_RATES.contains(&rate.as_str()) {
                Err(Reply::error(400, &format!("'{rate}' is not a NY Fed rate."), "Use SOFR, EFFR, OBFR, TGCR or BGCR."))
            } else {
                ctx.fa.nyfed(&rate, None).map_err(fa_error).map(|o| Reply::json(&o))
            }
        }
        "/api/rates/inflation" => ctx.fa.inflation(None).map_err(fa_error).map(|i| Reply::json(&i)),
        "/api/bonds" => ctx.fa.credit_snapshot(None).map_err(fa_error).map(|s| {
            Reply::json(&json!({"snapshot": &s, "hqm_ordered": s.hqm_curve.ordered()}))
        }),
        "/api/leaderboard" => leaderboard(&ctx.fa.repo),
        _ => Err(Reply::error(404, &format!("No page or endpoint '{path}'."), "")),
    };
    answer.unwrap_or_else(|e| e)
}

// ---------------------------------------------------------------------------
// Endpoints
// ---------------------------------------------------------------------------

/// The tickers that have company facts in the cache, sorted.
pub fn tickers(cache: &Path) -> Answer {
    let entries = fs::read_dir(cache).map_err(|e| {
        Reply::error(
            500,
            &format!("Cannot read the cache {}: {e}", cache.display()),
            "Get the SEC data first with mapcheck fetch (see rust/mapcheck/README.md).",
        )
    })?;
    let mut out: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| !name.starts_with('_') && facts_path(cache, name).is_some())
        .collect();
    out.sort();
    Ok(Reply::json(&out))
}

/// Tickers with a driver model file: `analysis/<TICKER>/driver_model.toml`. The Monte Carlo DCF and the
/// company report use it.
pub fn models(repo: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(repo.join("analysis"))
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().join("driver_model.toml").is_file())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn stock(ctx: &Ctx, q: &HashMap<String, String>) -> Answer {
    let ticker = valid_ticker(q.get("ticker"))?;
    let years: usize = number(q, "years", 3)?;
    let years = years.clamp(1, PERIODS);
    let facts = sec::load_or_fetch_facts(&ctx.cache, &ticker, 0.15).map_err(|e| {
        let hint = if e.contains("SEC contact") || e.contains("sec_contact.toml") || e.contains("SEC_USER_AGENT") {
            "The ticker is not in the cache, so the program must get it from the SEC. \
             The SEC contact comes from the file sec_contact.toml in the repo root; the environment variable \
             SEC_USER_AGENT is optional and replaces it. Make sure that the file is there and that its key \
             user_agent has a name and an address. Then start the server again."
        } else if e.contains("not in the SEC ticker list") {
            "Make sure that the ticker is correct. The SEC list has only U.S. registrants."
        } else {
            "The ticker is not in the cache and the SEC request failed. Make sure that the computer has a network connection."
        };
        Reply::error(502, &e, hint)
    })?;
    let report = check_cached(&ctx.registry, &ctx.cache, &ticker, &facts, PERIODS, dates::today(), true);
    Ok(Reply::json(&stock_json(&ctx.registry, &report, years)))
}

/// The status, the findings, and each standard variable of the latest `years`
/// fiscal years (oldest first) with its value and its source.
pub fn stock_json(registry: &Registry, report: &TickerReport, years: usize) -> Value {
    let mut out = report_json(report);
    if let Some(map) = out.as_object_mut() {
        map.remove("unmapped");
    }
    let annual = report.company.as_ref().map(|c| c.annual.len()).unwrap_or(0);
    let lines = values_json(registry, report);
    let periods: Vec<&Value> = lines[..annual.min(lines.len())].iter().rev().take(years).rev().collect();
    let variables: Vec<Value> = registry
        .variables
        .iter()
        .map(|v| {
            let values: Vec<Value> = periods.iter().map(|p| p["values"][&v.name].clone()).collect();
            let sources: Vec<Value> = periods.iter().map(|p| p["sources"][&v.name].clone()).collect();
            json!({"name": v.name, "statement": v.statement, "unit": v.unit, "core": v.core,
                   "memo": v.memo, "values": values, "sources": sources})
        })
        .collect();
    out["periods"] = json!(periods
        .iter()
        .map(|p| json!({"period_end": p["period_end"], "form": p["form"], "filed_date": p["filed_date"]}))
        .collect::<Vec<_>>());
    out["variables"] = json!(variables);
    out["core_total"] = json!(registry.core().len());
    out
}

/// `today` or YYYY-MM-DD.
fn valid_date(text: &str) -> bool {
    text == "today" || (text.len() == 10 && dates::parse(text).is_some())
}

/// Check the verdict query and make the options. Nothing runs here.
pub fn verdict_request(repo: &Path, q: &HashMap<String, String>) -> Result<(String, f64, VerdictOptions), Reply> {
    let ticker = valid_ticker(q.get("ticker"))?;
    let available = models(repo);
    if !available.contains(&ticker) {
        return Err(Reply::error(
            400,
            &format!("{ticker} has no driver model (analysis/{ticker}/driver_model.toml)."),
            &format!("The Monte Carlo DCF can run for: {}.", available.join(", ")),
        ));
    }
    let price: f64 = number(q, "price", f64::NAN)?;
    if !(price.is_finite() && price > 0.0) {
        return Err(Reply::error(400, "Type a market price that is more than 0.", ""));
    }
    let simulations: u32 = number(q, "simulations", 10_000)?;
    if !(100..=200_000).contains(&simulations) {
        return Err(Reply::error(400, "The number of simulations must be from 100 to 200000.", ""));
    }
    let seed: i64 = number(q, "seed", 7)?;
    let rf_date = q.get("rf_date").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(d) = &rf_date {
        if !valid_date(d) {
            return Err(Reply::error(400, &format!("The rf date '{d}' is not 'today' or YYYY-MM-DD."), ""));
        }
    }
    let rating = q.get("rating").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(r) = &rating {
        if r.len() > 5 || !r.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-') {
            return Err(Reply::error(400, &format!("The rating '{r}' is not valid."), "Use an S&P rating (A-) or a Moody's rating (A3)."));
        }
        if rf_date.is_none() {
            return Err(Reply::error(400, "A rating needs the market rates.", "Select 'Use the market rates of today'."));
        }
    }
    Ok((ticker, price, VerdictOptions { simulations: Some(simulations), seed: Some(seed), rf_date, rf_base_date: None, rating }))
}

fn verdict(ctx: &Ctx, q: &HashMap<String, String>) -> Answer {
    let (ticker, price, opts) = verdict_request(&ctx.fa.repo, q)?;
    ctx.fa.verdict(&ticker, price, &opts).map(|v| Reply::json(&v)).map_err(fa_error)
}

// ---------------------------------------------------------------------------
// Company report (`fa report`)
// ---------------------------------------------------------------------------

/// The cache folder of the company reports, under `data/` (not in git).
const REPORT_DIR: &str = "data/reports";
/// The text that the report has when it has no price.
const NO_PRICE_TEXT: &str = "current price unavailable";

/// The file name of a report: `IBM_228.53.html` (price from the user) or
/// `IBM_auto.html` (the script gets the price). Only a checked ticker and a number
/// that the program formats itself are in the name, so no text of the user is in it.
pub fn report_name(ticker: &str, price: Option<f64>) -> String {
    match price {
        Some(p) => format!("{ticker}_{p:.2}.html"),
        None => format!("{ticker}_auto.html"),
    }
}

/// The rule for a name in the cache folder: letters, digits, '.', '-' and '_' only,
/// no '..', and it ends with `.html`.
pub fn report_name_ok(name: &str) -> bool {
    name.ends_with(".html")
        && !name.contains("..")
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Check the report query. The price is rounded to the cent, so the name and the
/// report use the same number. Nothing is made here.
pub fn report_request(repo: &Path, q: &HashMap<String, String>) -> Result<(String, Option<f64>, PathBuf), Reply> {
    let ticker = valid_ticker(q.get("ticker"))?;
    let price = match q.get("price").map(|s| s.trim()).filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => {
            let p: f64 = s.parse().map_err(|_| Reply::error(400, &format!("The price '{s}' is not a number."), ""))?;
            if !(p.is_finite() && (0.005..=10_000_000.0).contains(&p)) {
                return Err(Reply::error(400, "Type a market price that is more than 0, or leave the price empty.", ""));
            }
            Some(format!("{p:.2}").parse::<f64>().unwrap_or(p))
        }
    };
    let name = report_name(&ticker, price);
    if !report_name_ok(&name) {
        return Err(Reply::error(400, "The file name of the report is not valid.", ""));
    }
    let path = repo.join(REPORT_DIR).join(name);
    Ok((ticker, price, path))
}

/// The state of a report file for the page.
pub fn report_state(repo: &Path, ticker: &str, price: Option<f64>, path: &Path, made_now: bool) -> Value {
    let meta = fs::metadata(path).ok().filter(|m| m.is_file());
    let made_at = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    // Without a price from the user, the report takes the last close of the price history.
    let price_missing = meta.is_some()
        && price.is_none()
        && fs::read(path).map(|b| b.windows(NO_PRICE_TEXT.len()).any(|w| w == NO_PRICE_TEXT.as_bytes())).unwrap_or(false);
    let mut url = format!("/report/file?ticker={ticker}");
    if let Some(p) = price {
        url.push_str(&format!("&price={p:.2}"));
    }
    json!({
        "ticker": ticker,
        "price": price,
        "exists": meta.is_some(),
        "made_at": made_at,
        "size": meta.as_ref().map(|m| m.len()),
        "made_now": made_now,
        "driver_model": models(repo).iter().any(|m| m == ticker),
        "price_missing": price_missing,
        "url": url,
    })
}

fn report_info(ctx: &Ctx, q: &HashMap<String, String>) -> Answer {
    let (ticker, price, path) = report_request(&ctx.fa.repo, q)?;
    Ok(Reply::json(&report_state(&ctx.fa.repo, &ticker, price, &path, false)))
}

/// The report file, for the iframe and for a new tab. The file is sent in parts.
fn report_file(ctx: &Ctx, q: &HashMap<String, String>) -> Answer {
    let (ticker, _, path) = report_request(&ctx.fa.repo, q)?;
    if !path.is_file() {
        return Err(Reply::error(
            404,
            &format!("There is no report file for {ticker} with this price."),
            "Push 'Show the report' in the Company report view to make it.",
        ));
    }
    Ok(Reply { status: 200, content_type: "text/html; charset=utf-8", body: String::new(), file: Some(path) })
}

/// A failed report, with a hint.
fn report_error(text: &str) -> Reply {
    if text.contains("No saved data") {
        let ticker = text.split_whitespace().skip_while(|w| *w != "for").nth(1).unwrap_or("TICKER").trim_end_matches(['.', ',']);
        return Reply::error(404, text, &format!("Import the company data first: fa import {ticker}"));
    }
    if ["Yahoo", "HTTP", "timed out", "connection"].iter().any(|w| text.contains(w)) {
        return Reply::error(502, text, "A data site did not answer or refused the request. Try again later.");
    }
    Reply::error(502, &format!("The report cannot be made: {text}"), "")
}

/// Make the report when no file exists or when the user asks (`force=1`), then
/// give the state of the file. The file is written in one step (a temporary file, then a
/// rename), so a failed run does not destroy an earlier report.
fn report_make(ctx: &Ctx, q: &HashMap<String, String>) -> Answer {
    let (ticker, price, path) = report_request(&ctx.fa.repo, q)?;
    let processed = ctx.fa.repo.join("data").join("processed").join(format!("{ticker}.json"));
    if !processed.is_file() {
        return Err(Reply::error(
            404,
            &format!("There is no saved company data for {ticker}."),
            &format!("Run: fa import {ticker}"),
        ));
    }
    let force = q.get("force").map(|v| v == "1" || v == "true").unwrap_or(false);
    let made_now = force || !path.is_file();
    if made_now {
        let built = financial_analysis::report::make_report(&ctx.fa.repo, &ticker, price, &financial_analysis::market::today_iso(), &ctx.fetch)
            .map_err(|e| report_error(&e))?;
        financial_analysis::market::write_atomic(&path, &built.html())
            .map_err(|e| Reply::error(500, &format!("Cannot save the report {}: {e}", path.display()), ""))?;
    }
    Ok(Reply::json(&report_state(&ctx.fa.repo, &ticker, price, &path, made_now)))
}

fn leaderboard(repo: &Path) -> Answer {
    let path = repo.join("trading").join("research").join("LEADERBOARD.md");
    fs::read_to_string(&path)
        .map(|text| Reply::json(&json!({"path": "trading/research/LEADERBOARD.md", "text": text})))
        .map_err(|e| Reply::error(500, &format!("Cannot read {}: {e}", path.display()), "The file is part of the repository (trading/research/LEADERBOARD.md)."))
}

/// An error of a typed call, with a hint that tells the user what to do.
pub fn fa_error(e: FaError) -> Reply {
    let text = e.to_string();
    let lower = text.to_lowercase();
    let hint = match &e {
        FaError::Json { .. } => "A reply was not the expected JSON. Try again later.",
        FaError::Data(_)
            if ["fetch failed", "request failed", "connection", "timed out", "timeout", "threshold", "http 4", "http 5", "tls", "dns"]
                .iter()
                .any(|w| lower.contains(w)) =>
        {
            "A data site did not answer or refused the request. Make sure that the computer has a network connection. Try again later."
        }
        FaError::Data(_) => "",
    };
    Reply::error(502, &text, hint)
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

fn open_browser(url: &str) {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    match cmd.spawn() {
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => eprintln!("fa serve: the browser did not open ({e}). Open {url} in your browser."),
    }
}

fn respond(ctx: &Ctx, port: u16, request: tiny_http::Request) {
    let host = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Host"))
        .map(|h| h.value.as_str().to_string());
    let reply = if host_allowed(host.as_deref(), port) {
        route(ctx, request.method().as_str(), request.url())
    } else {
        Reply::error(403, "The Host header is not this computer.", "")
    };
    let headers = [("Content-Type", reply.content_type), ("Cache-Control", "no-store"), ("X-Content-Type-Options", "nosniff")];
    if let Some(path) = &reply.file {
        // A large file: the server reads it in parts and does not keep it in memory.
        match fs::File::open(path) {
            Ok(file) => {
                let mut response = tiny_http::Response::from_file(file).with_status_code(reply.status);
                for (k, v) in headers {
                    if let Ok(h) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                        response.add_header(h);
                    }
                }
                let _ = request.respond(response);
            }
            Err(e) => {
                let r = Reply::error(500, &format!("Cannot read {}: {e}", path.display()), "");
                let _ = request.respond(tiny_http::Response::from_string(r.body).with_status_code(r.status));
            }
        }
        return;
    }
    let mut response = tiny_http::Response::from_string(reply.body).with_status_code(reply.status);
    for (k, v) in headers {
        if let Ok(h) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
            response.add_header(h);
        }
    }
    let _ = request.respond(response);
}

fn parse_args(args: &[String]) -> Result<(Option<u16>, bool), String> {
    let mut port = None;
    let mut open = true;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                let v = args.get(i + 1).ok_or("--port needs a number")?;
                port = Some(v.parse().map_err(|_| format!("--port '{v}' is not a port number"))?);
                i += 1;
            }
            "--no-open" => open = false,
            other => return Err(format!("unknown serve option '{other}'")),
        }
        i += 1;
    }
    Ok((port, open))
}

pub fn serve(fa: Fa, args: &[String]) -> ExitCode {
    let (port, open) = match parse_args(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("fa serve: {e}\n\n{USAGE}");
            return ExitCode::from(64);
        }
    };
    let ctx = match Ctx::new(fa) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("fa serve: {e}");
            return ExitCode::from(2);
        }
    };
    let candidates: Vec<u16> = match port {
        Some(p) => vec![p],
        None => (DEFAULT_PORT..DEFAULT_PORT + PORT_TRIES).collect(),
    };
    let mut bound = None;
    let mut last_error = String::new();
    for p in candidates {
        match tiny_http::Server::http(("127.0.0.1", p)) {
            Ok(server) => {
                bound = Some((server, p));
                break;
            }
            Err(e) => last_error = format!("port {p}: {e}"),
        }
    }
    let Some((server, port)) = bound else {
        eprintln!("fa serve: cannot start the server ({last_error}). Use --port with a different number.");
        return ExitCode::from(2);
    };
    let url = format!("http://127.0.0.1:{port}/");
    println!("FinancialAnalysis viewer: {url}");
    println!("Repository: {}", ctx.fa.repo.display());
    println!("Push Ctrl+C to stop the server.");
    if open {
        open_browser(&url);
    }
    for request in server.incoming_requests() {
        let ctx = Arc::clone(&ctx);
        std::thread::spawn(move || respond(&ctx, port, request));
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temporary repository: a cache with one ticker, one driver model, and a leaderboard.
    fn fixture(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("fa-serve-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data/raw/TEST")).unwrap();
        fs::create_dir_all(root.join("data/raw/_sec")).unwrap();
        fs::create_dir_all(root.join("analysis/TEST")).unwrap();
        fs::create_dir_all(root.join("analysis/NOMODEL")).unwrap();
        fs::write(root.join("analysis/TEST/driver_model.toml"), "").unwrap();
        fs::create_dir_all(root.join("trading/research")).unwrap();
        fs::write(root.join("trading/research/LEADERBOARD.md"), "# Leaderboard\n").unwrap();
        let fact = |y: i32, val: f64| {
            json!({"start": format!("{y}-01-01"), "end": format!("{y}-12-31"), "val": val,
                   "form": "10-K", "filed": format!("{}-02-15", y + 1)})
        };
        let facts = json!({"cik": 1, "entityName": "Test Company", "facts": {"us-gaap": {
            "Revenues": {"units": {"USD": [fact(2022, 100.0e9), fact(2023, 110.0e9), fact(2024, 120.0e9), fact(2025, 130.0e9)]}}
        }}});
        fs::write(root.join("data/raw/TEST/company_facts.json"), facts.to_string()).unwrap();
        root
    }

    fn offline(_: &str) -> Result<Vec<u8>, String> {
        Err("offline".to_string())
    }

    fn ctx(root: &Path) -> Ctx {
        let mut c = Ctx::new(Fa::new(root)).unwrap();
        c.fetch = offline;
        c
    }

    fn body(r: &Reply) -> Value {
        serde_json::from_str(&r.body).unwrap()
    }

    #[test]
    fn query_values_are_decoded() {
        let (path, q) = split_url("/api/stock?ticker=brk.b&x=a%20b+c&empty=&bad=%zz");
        assert_eq!(path, "/api/stock");
        assert_eq!(q["ticker"], "brk.b");
        assert_eq!(q["x"], "a b c");
        assert_eq!(q["empty"], "");
        assert_eq!(q["bad"], "%zz");
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn tickers_are_checked() {
        let ok = |s: &str| valid_ticker(Some(&s.to_string())).ok();
        assert_eq!(ok(" ibm "), Some("IBM".into()));
        assert_eq!(ok("BRK-B"), Some("BRK-B".into()));
        assert_eq!(ok("../etc"), None);
        assert_eq!(ok("-h"), None);
        assert_eq!(ok("A/B"), None);
        assert_eq!(ok(""), None);
        assert!(valid_ticker(None).is_err());
    }

    #[test]
    fn host_must_be_this_computer() {
        assert!(host_allowed(Some("127.0.0.1:8765"), 8765));
        assert!(host_allowed(Some("localhost:8765"), 8765));
        assert!(host_allowed(None, 8765));
        assert!(!host_allowed(Some("evil.example:8765"), 8765));
        assert!(!host_allowed(Some("127.0.0.1:9999"), 8765));
    }

    #[test]
    fn page_and_unknown_path() {
        let root = fixture("page");
        let c = ctx(&root);
        let r = route(&c, "GET", "/");
        assert_eq!((r.status, r.content_type), (200, "text/html; charset=utf-8"));
        assert!(r.body.contains("<title>"));
        assert_eq!(route(&c, "GET", "/nothing").status, 404);
        assert_eq!(route(&c, "POST", "/").status, 405);
    }

    #[test]
    fn tickers_models_and_leaderboard() {
        let root = fixture("lists");
        let c = ctx(&root);
        assert_eq!(body(&route(&c, "GET", "/api/tickers")), json!(["TEST"]));
        assert_eq!(body(&route(&c, "GET", "/api/models")), json!(["TEST"]));
        let l = body(&route(&c, "GET", "/api/leaderboard"));
        assert_eq!(l["text"], "# Leaderboard\n");
    }

    #[test]
    fn stock_shows_variables_of_the_latest_years() {
        let root = fixture("stock");
        let c = ctx(&root);
        let r = route(&c, "GET", "/api/stock?ticker=test&years=2");
        assert_eq!(r.status, 200, "{}", r.body);
        let j = body(&r);
        assert_eq!(j["ticker"], "TEST");
        assert_eq!(j["name"], "Test Company");
        assert!(["PASS", "WARN", "FAIL", "N/A", "ERROR"].contains(&j["status"].as_str().unwrap()));
        let ends: Vec<&str> = j["periods"].as_array().unwrap().iter().map(|p| p["period_end"].as_str().unwrap()).collect();
        assert_eq!(ends, vec!["2024-12-31", "2025-12-31"]);
        let revenue = j["variables"].as_array().unwrap().iter().find(|v| v["name"] == "revenue").unwrap();
        assert_eq!(revenue["values"], json!([120.0e9, 130.0e9]));
        assert!(revenue["sources"][1].as_str().unwrap().contains("Revenues"));
        assert!(j["findings"].as_array().unwrap().iter().any(|f| f["code"] == "core_missing_critical"));
        assert!(j.get("unmapped").is_none());
    }

    #[test]
    fn stock_errors() {
        let root = fixture("stockerr");
        let c = ctx(&root);
        assert_eq!(route(&c, "GET", "/api/stock?ticker=../x").status, 400);
        assert_eq!(route(&c, "GET", "/api/stock?ticker=TEST&years=x").status, 400);
    }

    #[test]
    fn verdict_query_is_checked_before_the_run() {
        let root = fixture("verdict");
        let q = |url: &str| split_url(url).1;
        let (t, p, o) = verdict_request(&root, &q("/?ticker=test&price=228.53")).unwrap();
        assert_eq!((t.as_str(), p, o.simulations, o.seed), ("TEST", 228.53, Some(10_000), Some(7)));
        let err = |url: &str| verdict_request(&root, &q(url)).unwrap_err().status;
        assert_eq!(err("/?ticker=NOMODEL&price=1"), 400);
        assert_eq!(err("/?ticker=TEST"), 400);
        assert_eq!(err("/?ticker=TEST&price=-3"), 400);
        assert_eq!(err("/?ticker=TEST&price=1&simulations=5"), 400);
        assert_eq!(err("/?ticker=TEST&price=1&rf_date=yesterday"), 400);
        assert_eq!(err("/?ticker=TEST&price=1&rating=A-"), 400);
        let (_, _, o) = verdict_request(&root, &q("/?ticker=TEST&price=1&rf_date=today&rating=A-")).unwrap();
        assert_eq!((o.rf_date.as_deref(), o.rating.as_deref()), (Some("today"), Some("A-")));
    }

    #[test]
    fn a_broken_model_file_gives_an_error_and_a_data_error_gives_a_hint() {
        let root = fixture("brokenmodel");
        let c = ctx(&root);
        // the model file of TEST is empty: the run stops with the reason, and no program is started
        let r = route(&c, "GET", "/api/verdict?ticker=TEST&price=10");
        assert_eq!(r.status, 502);
        assert!(body(&r)["error"].as_str().unwrap().contains("model file"), "{}", r.body);
        let net = fa_error(FaError::Data("request failed: connection reset".into()));
        assert!(body(&net)["hint"].as_str().unwrap().contains("network"));
        assert_eq!(body(&fa_error(FaError::Data("bad input".into())))["hint"], "");
        assert_eq!(route(&c, "GET", "/api/rates/nyfed?rate=XYZ").status, 400);
    }

    #[test]
    fn report_file_names_follow_the_rule() {
        assert_eq!(report_name("IBM", Some(228.5)), "IBM_228.50.html");
        assert_eq!(report_name("BRK-B", None), "BRK-B_auto.html");
        assert!(report_name_ok("IBM_228.50.html"));
        assert!(report_name_ok("BRK-B_auto.html"));
        for bad in ["../x.html", "a/b.html", "a\\b.html", "..html", ".hidden.html", "x.txt", "a b.html", "a..b.html", ""] {
            assert!(!report_name_ok(bad), "{bad}");
        }
    }

    #[test]
    fn report_query_is_checked_before_the_run() {
        let root = fixture("reportq");
        let q = |url: &str| split_url(url).1;
        let (t, p, path) = report_request(&root, &q("/?ticker=test&price=228.534")).unwrap();
        assert_eq!((t.as_str(), p), ("TEST", Some(228.53)));
        assert!(path.ends_with("data/reports/TEST_228.53.html"));
        let (_, p, path) = report_request(&root, &q("/?ticker=TEST&price=")).unwrap();
        assert_eq!(p, None);
        assert!(path.ends_with("TEST_auto.html"));
        let err = |url: &str| report_request(&root, &q(url)).unwrap_err().status;
        assert_eq!(err("/?ticker=../x"), 400);
        assert_eq!(err("/?ticker=a%2Fb"), 400);
        assert_eq!(err("/?ticker=-h"), 400);
        assert_eq!(err("/?ticker=TEST&price=abc"), 400);
        assert_eq!(err("/?ticker=TEST&price=0"), 400);
        assert_eq!(err("/?ticker=TEST&price=-1"), 400);
        assert_eq!(err("/?ticker=TEST&price=NaN"), 400);
        assert_eq!(err("/"), 400);
    }

    #[test]
    fn report_file_is_served_from_the_cache_folder_only() {
        let root = fixture("reportfile");
        let c = ctx(&root);
        let missing = route(&c, "GET", "/report/file?ticker=TEST&price=10");
        assert_eq!(missing.status, 404);
        assert!(body(&missing)["hint"].as_str().unwrap().contains("Show the report"));
        assert_eq!(route(&c, "GET", "/report/file?ticker=../../etc/passwd").status, 400);
        // A file name from the user is not used.
        assert_eq!(route(&c, "GET", "/report/file?ticker=TEST&file=x.html").status, 404);
        let dir = root.join("data/reports");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("TEST_10.00.html"), "<html>report</html>").unwrap();
        let ok = route(&c, "GET", "/report/file?ticker=test&price=10");
        assert_eq!((ok.status, ok.content_type), (200, "text/html; charset=utf-8"));
        assert_eq!(ok.file.as_deref(), Some(dir.join("TEST_10.00.html").as_path()));
        assert!(ok.body.is_empty());
        // A file of another price is a different file.
        assert_eq!(route(&c, "GET", "/report/file?ticker=TEST&price=11").status, 404);
    }

    #[test]
    fn report_info_shows_the_file_state() {
        let root = fixture("reportinfo");
        let c = ctx(&root);
        let none = body(&route(&c, "GET", "/api/report/info?ticker=TEST&price=10"));
        assert_eq!((none["exists"].clone(), none["driver_model"].clone()), (json!(false), json!(true)));
        assert!(none["made_at"].is_null());
        let dir = root.join("data/reports");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("TEST_auto.html"), "<p>price: current price unavailable (offline?)</p>").unwrap();
        fs::write(dir.join("TEST_10.00.html"), "<p>current price unavailable</p>").unwrap();
        let auto = body(&route(&c, "GET", "/api/report/info?ticker=TEST"));
        assert_eq!((auto["exists"].clone(), auto["price_missing"].clone()), (json!(true), json!(true)));
        assert!(auto["made_at"].as_u64().unwrap() > 1_600_000_000);
        assert_eq!(auto["url"], "/report/file?ticker=TEST");
        // With a price from the user, the missing-price text does not matter.
        let priced = body(&route(&c, "GET", "/api/report/info?ticker=TEST&price=10"));
        assert_eq!(priced["price_missing"], json!(false));
        assert_eq!(priced["url"], "/report/file?ticker=TEST&price=10.00");
        let nomodel = body(&route(&c, "GET", "/api/report/info?ticker=NOMODEL"));
        assert_eq!(nomodel["driver_model"], json!(false));
        assert_eq!(route(&c, "GET", "/api/report/info?ticker=%2e%2e").status, 400);
    }

    /// A company with ten annual periods: enough for the report.
    fn write_company(root: &Path) {
        let periods: Vec<Value> = (2016..=2025)
            .map(|y| {
                let r = 100.0e9 * 1.05f64.powi(y - 2016);
                json!({"form": "10-K", "period_end": format!("{y}-12-31"), "filed_date": format!("{}-02-15", y + 1), "revenue": r,
                       "operating_cash_flow": r * 0.22, "capex": -r * 0.02, "total_debt": 40.0e9, "cash_and_equivalents": 10.0e9,
                       "shares_outstanding": 1.0e9})
            })
            .collect();
        fs::create_dir_all(root.join("data/processed")).unwrap();
        fs::write(root.join("data/processed/TEST.json"), json!({"ticker": "TEST", "cik": "1", "company_name": "Test Company", "periods": periods}).to_string())
            .unwrap();
    }

    #[test]
    fn report_make_checks_first_reuses_the_file_and_keeps_it_when_a_run_fails() {
        let root = fixture("reportmake");
        let c = ctx(&root);
        assert_eq!(route(&c, "GET", "/api/report/make?ticker=../x").status, 400);
        // No saved company data: a clear error with the import command.
        let r = route(&c, "GET", "/api/report/make?ticker=TEST&price=10");
        assert_eq!(r.status, 404);
        assert!(body(&r)["hint"].as_str().unwrap().contains("fa import"));
        // A company file without annual periods: the report stops with the reason, and no file stays.
        fs::create_dir_all(root.join("data/processed")).unwrap();
        fs::write(root.join("data/processed/TEST.json"), "{}").unwrap();
        let r = route(&c, "GET", "/api/report/make?ticker=TEST&price=10");
        assert_eq!(r.status, 502);
        assert!(body(&r)["error"].as_str().unwrap().contains("no annual"), "{}", r.body);
        assert!(!root.join("data/reports").exists() || fs::read_dir(root.join("data/reports")).unwrap().count() == 0);
        // A file exists and the user does not ask again: the report is not made again.
        fs::create_dir_all(root.join("data/reports")).unwrap();
        fs::write(root.join("data/reports/TEST_10.00.html"), "<p>old</p>").unwrap();
        let r = route(&c, "GET", "/api/report/make?ticker=TEST&price=10");
        assert_eq!(r.status, 200, "{}", r.body);
        let j = body(&r);
        assert_eq!((j["exists"].clone(), j["made_now"].clone()), (json!(true), json!(false)));
        // With force the report is made again; here it fails, and the old file stays.
        assert_eq!(route(&c, "GET", "/api/report/make?ticker=TEST&price=10&force=1").status, 502);
        assert_eq!(fs::read_to_string(root.join("data/reports/TEST_10.00.html")).unwrap(), "<p>old</p>");
    }

    #[test]
    fn report_make_writes_the_page_with_the_fields_that_the_viewer_reads() {
        let root = fixture("reportwrite");
        write_company(&root);
        let c = ctx(&root);
        let r = route(&c, "GET", "/api/report/make?ticker=test&price=10");
        assert_eq!(r.status, 200, "{}", r.body);
        let j = body(&r);
        for k in ["ticker", "price", "exists", "made_at", "size", "made_now", "driver_model", "price_missing", "url"] {
            assert!(j.get(k).is_some(), "{k}");
        }
        assert_eq!((j["ticker"].clone(), j["exists"].clone(), j["made_now"].clone(), j["price_missing"].clone()), (json!("TEST"), json!(true), json!(true), json!(false)));
        assert_eq!(j["url"], "/report/file?ticker=TEST&price=10.00");
        let page = fs::read_to_string(root.join("data/reports/TEST_10.00.html")).unwrap();
        assert!(page.contains("<title>TEST \u{2014} Company report</title>") && page.contains("cdn.plot.ly/plotly-4.1.1.min.js"));
        // the model file of the fixture is empty: the section says so and the report goes on
        assert!(page.contains("Driver model could not be loaded"));
        assert!(page.contains("Price history not available"));
        // without a price from the user and without a price history, the page says so and the viewer shows the warning
        let r = route(&c, "GET", "/api/report/make?ticker=TEST");
        assert_eq!(body(&r)["price_missing"], json!(true), "{}", r.body);
        // a second call reuses the file
        let again = body(&route(&c, "GET", "/api/report/make?ticker=TEST&price=10"));
        assert_eq!(again["made_now"], json!(false));
    }

    #[test]
    fn report_errors_have_hints() {
        let nodata = report_error("No saved data for ZZZ at /x/ZZZ.json. Run: fa import ZZZ");
        assert_eq!(nodata.status, 404);
        assert!(body(&nodata)["hint"].as_str().unwrap().contains("fa import ZZZ"));
        let net = report_error("Yahoo answered HTTP 429 (too many requests)");
        assert!(body(&net)["hint"].as_str().unwrap().contains("Try again later"));
        let other = report_error("TEST: no annual (10-K) periods available.");
        assert!(body(&other)["error"].as_str().unwrap().starts_with("The report cannot be made"));
    }

    #[test]
    fn serve_options() {
        let s = |v: &[&str]| parse_args(&v.iter().map(|x| x.to_string()).collect::<Vec<_>>());
        assert_eq!(s(&[]).unwrap(), (None, true));
        assert_eq!(s(&["--port", "9000", "--no-open"]).unwrap(), (Some(9000), false));
        assert!(s(&["--port", "x"]).is_err());
        assert!(s(&["--bad"]).is_err());
    }
}
