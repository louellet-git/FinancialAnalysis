//! The commands `fa estimates get|show` and `fa earnings calendar|dates`.

use super::consensus::{self, forward_pe, ConsensusEstimates};
use super::earnings_dates::{self, Flag, NextDate};
use super::estimates_cache::{self, Options, Status};
use super::fmp::{self, Fmp};
use super::nasdaq_est::{self, Client, Endpoint, SurpriseRow};
use super::price_cli::{cache_dir, collect_tickers, parse};
use super::{parse_date, price_cache, today_iso};
use std::path::Path;
use std::time::{Duration, Instant};

pub const ESTIMATES_USAGE: &str = "\
fa estimates - analyst consensus, price targets and earnings surprises (Nasdaq site API, data of Zacks; no key)

USAGE:
  fa estimates get TICKER... [--tickers-file F] [--endpoints LIST] [--momentum] [--refresh]
                              get the consensus of today into data/raw/<TICKER>/ (one request each 2 seconds).
                              LIST is a comma list of: forecast (EPS consensus by quarter and year), surprise (the
                              last 4 reported quarters with the report dates), targets (price target and ratings),
                              momentum (the consensus 1 week and 1 month ago; optional). Default:
                              forecast,surprise,targets = 3 requests for each ticker. --momentum adds the fourth.
                              A file of today is not asked again (--refresh asks again). The snapshots of the days
                              stay: they are the history of the consensus. A stopped run continues when you run it again.
                              With FMP_API_KEY in the environment the revenue estimates of FMP go into the snapshot.
  fa estimates show TICKER [--price P]
                              the consensus EPS by quarter and year with range and number of analysts, the revisions
                              between the snapshots, the forward P/E, the price target against the last close of the
                              price cache, the ratings, the last 4 surprises and the next earnings date

The source is not a published API: it can change or stop. There are no revenue estimates in it.
The tickers-file has one ticker at the start of each line; # starts a comment (rust/mapcheck/universe/sp500.txt).";

pub const EARNINGS_USAGE: &str = "\
fa earnings - earnings dates (Nasdaq earnings calendar; the SEC filing list as a check of the past dates)

USAGE:
  fa earnings calendar [TICKER...] [--tickers-file F] [--from D] [--days N] [--refresh]
                              get the Nasdaq calendar of each weekday from D (default: today) for N calendar days
                              (default: 60), 1 request for each day, into data/raw/_calendar/. A day that the cache
                              has is not asked again (a past day is final; a future day is asked again each day).
                              Then a table of the next report date of each ticker of the list, written into
                              data/raw/<TICKER>/earnings_dates.json.
  fa earnings dates TICKER... [--tolerance N]
                              the past report dates of the Nasdaq surprise table against the SEC filing list (form 8-K
                              with item 2.02), and the next date. Exit 1 when two dates are more than N days apart
                              (default: 1). The SEC file comes from: mapcheck fetch --submissions TICKER...

The calendar covers a few weeks. A company with no date in the window has no next date in the table.";

fn fmt(v: Option<f64>, decimals: usize) -> String {
    v.map_or("-".to_string(), |x| format!("{x:.decimals$}"))
}

fn pct(v: Option<f64>) -> String {
    v.map_or("-".to_string(), |x| format!("{:+.1}%", x * 100.0))
}

fn fmt_money(v: Option<f64>) -> String {
    v.map_or("-".to_string(), |x| if x.abs() >= 1e9 { format!("{:.2}B", x / 1e9) } else { format!("{x:.0}") })
}

/// One line for a next report date. An estimate says that it is a guess.
fn describe_next(n: &NextDate) -> String {
    if n.source.starts_with("estimate") {
        return format!("{} (a guess: {}; the calendar of the vendor has no date yet)", n.date, n.source.trim_start_matches("estimate: "));
    }
    format!("{} ({}), fiscal quarter {}, EPS forecast {} from {} estimate(s) [{}]", n.date, n.time, n.fiscal_quarter, fmt(n.eps_forecast, 2), fmt(n.estimates, 0), n.source)
}

// ---------------------------------------------------------------- estimates get

fn estimates_get(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let mut p = parse(rest, &["--tickers-file", "--endpoints"], &["--refresh", "--momentum"])?;
    collect_tickers(&mut p)?;
    let mut endpoints: Vec<Endpoint> = Endpoint::DEFAULT.to_vec();
    if let Some(list) = p.flag("--endpoints") {
        endpoints = list.split(',').map(|n| Endpoint::from_name(n.trim()).ok_or_else(|| format!("unknown endpoint '{n}'; the endpoints are forecast, surprise, targets, momentum"))).collect::<Result<_, _>>()?;
    }
    if p.switches.iter().any(|s| s == "--momentum") && !endpoints.contains(&Endpoint::Momentum) {
        endpoints.push(Endpoint::Momentum);
    }
    let refresh = p.switches.iter().any(|s| s == "--refresh");
    let opt = Options { today: today_iso(), refresh, endpoints: endpoints.clone() };
    let cache = cache_dir(repo);
    let names: Vec<&str> = endpoints.iter().map(|e| e.name()).collect();
    let n = p.tickers.len();
    eprintln!("fa: estimates for {n} ticker(s), endpoints {} ({} request(s) for each ticker), into {}", names.join(","), endpoints.len(), cache.display());

    let get = |url: &str| super::download(url);
    let mut client = Client::new(&get, Duration::from_secs(60));
    // The revenue estimates need a key (FMP). Without a key there is no revenue source and no request.
    let key = fmp::api_key(repo);
    let http = key.as_deref().map(fmp::http_fetch);
    let fmp_client = match (&key, &http) {
        (Some(k), Some(h)) => Some(Fmp::new(k, h)),
        _ => None,
    };
    let mut fmp_failed: Option<String> = None;

    let started = Instant::now();
    let (mut updated, mut current, mut nodata) = (0usize, 0usize, Vec::<String>::new());
    let mut failed: Vec<(String, String)> = Vec::new();
    let mut stopped_at = None;
    let mut revenue_added = 0usize;
    for (i, t) in p.tickers.iter().enumerate() {
        if client.blocked.is_some() {
            stopped_at = Some(i);
            break;
        }
        let status = estimates_cache::update_ticker(&cache, t, &opt, &mut client);
        match &status {
            Status::Current(note) => {
                eprintln!("[{}/{n}] {t}: current ({note})", i + 1);
                current += 1;
            }
            Status::Updated(note) => {
                eprintln!("[{}/{n}] {t}: {note}", i + 1);
                updated += 1;
            }
            Status::NoData(note) => {
                eprintln!("[{}/{n}] {t}: no data ({note})", i + 1);
                nodata.push(t.clone());
            }
            Status::Failed(why) => {
                eprintln!("[{}/{n}] {t}: FAILED {why}", i + 1);
                failed.push((t.clone(), why.clone()));
            }
            Status::Blocked(why) => {
                eprintln!("[{}/{n}] {t}: BLOCKED {why}", i + 1);
                failed.push((t.clone(), format!("blocked: {why}")));
                stopped_at = Some(i + 1);
                break;
            }
        }
        if let (Some(f), None, true) = (&fmp_client, &fmp_failed, matches!(status, Status::Updated(_) | Status::Current(_))) {
            match add_fmp_revenue(&cache, t, &opt.today, refresh, f) {
                Ok(0) => {}
                Ok(k) => {
                    revenue_added += 1;
                    eprintln!("    {t}: revenue estimates of FMP in {k} period(s)");
                }
                Err(e) => {
                    eprintln!("fa: FMP revenue estimates stop for this run: {e}");
                    fmp_failed = Some(e);
                }
            }
        }
    }
    let not_tried = stopped_at.map_or(0, |i| n - i);
    println!("estimates: {updated} updated, {current} current, {} no data, {} failed, {not_tried} not tried; {} request(s), {:.0} s", nodata.len(), failed.len(), client.requests, started.elapsed().as_secs_f64());
    if key.is_some() {
        println!("  revenue estimates (FMP): {revenue_added} ticker(s){}", fmp_failed.map_or(String::new(), |e| format!("; stopped: {e}")));
    } else {
        println!("  revenue estimates: none (the Nasdaq source has none; set FMP_API_KEY for FMP)");
    }
    if !nodata.is_empty() {
        println!("  no data from the source: {}", nodata.iter().take(40).cloned().collect::<Vec<_>>().join(" "));
    }
    for (t, why) in failed.iter().take(40) {
        println!("  failed {t}: {why}");
    }
    if let Some(why) = &client.blocked {
        let next = stopped_at.and_then(|i| p.tickers.get(i)).map_or(String::new(), |t| format!(" Run the same command later to continue from {t}."));
        println!("  stopped: the source blocked the program ({why}). The program does not get around a block.{next}");
    }
    Ok(if failed.is_empty() && client.blocked.is_none() { 0 } else { 1 })
}

/// The revenue estimates of FMP into today's snapshot of the ticker. Returns the number of periods with revenue.
fn add_fmp_revenue(cache: &Path, ticker: &str, today: &str, refresh: bool, f: &Fmp) -> Result<usize, String> {
    let Ok(mut snap) = consensus::load_snapshot(cache, ticker, Some(today)) else { return Ok(0) };
    if snap.source.as_deref() != Some(nasdaq_est::SOURCE) || (!refresh && snap.endpoints.iter().any(|e| e == "fmp-revenue")) {
        return Ok(0);
    }
    let estimates = fmp::load_estimates(cache, ticker, refresh, Some(f)).map_err(|e| e.to_string())?;
    let k = consensus::merge_revenue(&mut snap, &estimates);
    if k > 0 {
        consensus::save_snapshot(&snap, cache)?;
    }
    Ok(k)
}

// ---------------------------------------------------------------- estimates show

/// What `show` prints, from the data (no file access, so a test can run it).
pub struct ShowInput<'a> {
    /// The snapshots of the ticker, oldest first (at least one).
    pub snapshots: &'a [ConsensusEstimates],
    pub price: Option<(f64, String)>,
    pub surprises: &'a [SurpriseRow],
    pub next: Option<NextDate>,
}

fn revisions(snaps: &[ConsensusEstimates]) -> String {
    let latest = snaps.last().expect("one snapshot");
    if snaps.len() < 2 {
        return "revisions: one snapshot only. Run `fa estimates get` on later days; the snapshots of the days give the revisions.\n".to_string();
    }
    let first = &snaps[0];
    let mut out = format!("revisions of the mean EPS ({} snapshots, {} to {}):\n", snaps.len(), first.snapshot_date, latest.snapshot_date);
    out += &format!("  {:<8}{:>12}{:>12}{:>10}\n", "period", first.snapshot_date, latest.snapshot_date, "change");
    for period in ["0q", "+1q", "0y", "+1y"] {
        // The same fiscal period in both snapshots (the label `0y` moves when a year is reported).
        let end = |s: &ConsensusEstimates| s.period_ends.0.iter().find(|(p, _)| p == period).map(|(_, e)| e.clone());
        let (a, b) = (first.eps(period, "avg"), latest.eps(period, "avg"));
        let same = end(first).is_some() && end(first) == end(latest);
        let change = match (a, b) {
            (Some(a), Some(b)) if same && a != 0.0 => pct(Some(b / a.abs() - 1.0)),
            _ if !same => "new period".to_string(),
            _ => "-".to_string(),
        };
        if a.is_some() || b.is_some() {
            out += &format!("  {period:<8}{:>12}{:>12}{:>10}\n", fmt(a, 2), fmt(b, 2), change);
        }
    }
    out
}

/// The text of `fa estimates show`.
pub fn render_show(input: &ShowInput) -> String {
    let s = input.snapshots.last().expect("one snapshot");
    let mut out = format!("{}: consensus of {} (source {}; endpoints {})\n", s.ticker, s.snapshot_date, s.source.as_deref().unwrap_or("python-era file"), if s.endpoints.is_empty() { "-".to_string() } else { s.endpoints.join(",") });
    match &input.price {
        Some((p, note)) => out += &format!("last close {p:.2} ({note})\n"),
        None => out += "no price in the cache: run `fa prices get` or give --price P\n",
    }
    let price = input.price.as_ref().map(|(p, _)| *p);
    out += &format!("\n{:<7}{:<9}{:>9}{:>9}{:>9}{:>9}{:>13}{:>9}{:>9}\n", "period", "end", "mean EPS", "low", "high", "analysts", "revisions", "growth", "fwd P/E");
    let years = s.year_periods();
    for (period, metrics) in &s.earnings_estimate.0 {
        let get = |m: &str| metrics.0.iter().find(|(k, _)| k == m).and_then(|(_, v)| *v);
        let end = s.period_ends.0.iter().find(|(p, _)| p == period).map(|(_, e)| e.as_str()).unwrap_or("-");
        let revisions = match (get("up"), get("down")) {
            (Some(u), Some(d)) => format!("{u:.0} up/{d:.0} dn"),
            _ => "-".to_string(),
        };
        let growth = years.iter().position(|y| y == period).filter(|i| *i > 0).and_then(|_| s.eps_growth().into_iter().find(|(p, _)| p == period)).map(|(_, g)| g);
        let pe = if period.ends_with('y') { price.and_then(|p| forward_pe(p, get("avg"))) } else { None };
        out += &format!("{period:<7}{end:<9}{:>9}{:>9}{:>9}{:>9}{:>13}{:>9}{:>9}\n", fmt(get("avg"), 2), fmt(get("low"), 2), fmt(get("high"), 2), fmt(get("numberOfAnalysts"), 0), revisions, pct(growth), fmt(pe, 1));
    }
    if s.earnings_estimate.is_empty() {
        out += "(no EPS consensus in this snapshot)\n";
    }
    if let (Some(p), Some(ntm)) = (price, s.eps_next_four_quarters()) {
        out += &format!("next four quarters: EPS {ntm:.2}, forward P/E {}\n", fmt(forward_pe(p, Some(ntm)), 1));
    }
    out += "(growth: the consensus EPS of the year over the consensus of the year before; the DCF growth start of `fa dcf auto` is not shown here)\n";
    if let Some(g) = s.consensus_change.0.iter().find(|(k, _)| k == "yr_now") {
        let v = |k: &str| s.consensus_change.0.iter().find(|(n, _)| n == k).and_then(|(_, v)| *v);
        out += &format!("consensus change of the year EPS: now {}, 1 week ago {}, 1 month ago {}\n", fmt(g.1, 2), fmt(v("yr_1w"), 2), fmt(v("yr_1m"), 2));
    }
    out += "\n";
    out += &revisions(input.snapshots);

    out += "\n";
    if s.revenue_estimate.is_empty() {
        out += "revenue estimates: none (the Nasdaq source has none; FMP_API_KEY gives them)\n";
    } else {
        out += "revenue estimates (FMP):";
        for (period, m) in &s.revenue_estimate.0 {
            out += &format!(" {period} {}", fmt_money(m.0.iter().find(|(k, _)| k == "avg").and_then(|(_, v)| *v)));
        }
        out += "\n";
    }
    let t = |k: &str| s.analyst_price_targets.0.iter().find(|(n, _)| n == k).and_then(|(_, v)| *v);
    if s.analyst_price_targets.is_empty() {
        out += "price target: none in this snapshot\n";
    } else {
        let up = |x: Option<f64>| match (x, price) {
            (Some(x), Some(p)) if p > 0.0 => format!(" ({})", pct(Some(x / p - 1.0))),
            _ => String::new(),
        };
        out += &format!("price target: mean {}{}, low {}{}, high {}{}\n", fmt(t("mean"), 2), up(t("mean")), fmt(t("low"), 2), up(t("low")), fmt(t("high"), 2), up(t("high")));
    }
    let r = |k: &str| s.ratings.0.iter().find(|(n, _)| n == k).and_then(|(_, v)| *v);
    if !s.ratings.is_empty() {
        out += &format!("ratings: {} buy, {} hold, {} sell\n", fmt(r("buy"), 0), fmt(r("hold"), 0), fmt(r("sell"), 0));
    }

    out += "\nlast surprises:\n";
    if input.surprises.is_empty() {
        out += "  none in the cache (run `fa estimates get` with the endpoint surprise)\n";
    }
    for r in input.surprises.iter().take(4) {
        out += &format!("  {} quarter {:<8} EPS {:>6} consensus {:>6} surprise {:>7}\n", r.date, r.fiscal_qtr_end, fmt(r.eps, 2), fmt(r.consensus, 2), r.pct.map_or("-".to_string(), |p| format!("{p:+.2}%")));
    }
    match &input.next {
        Some(n) => out += &format!("\nnext earnings: {}\n", describe_next(n)),
        None => out += "\nnext earnings: not known (run `fa earnings calendar TICKER`)\n",
    }
    out
}

fn estimates_show(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let p = parse(rest, &["--price"], &[])?;
    let [t] = p.tickers.as_slice() else { return Err("show takes one ticker".into()) };
    let cache = cache_dir(repo);
    let dates = consensus::list_snapshots(&cache, t);
    if dates.is_empty() {
        println!("{t}: no estimates snapshot in the cache. Run: fa estimates get {t}. If that says \"no data\", the source has no estimates for this ticker (a new listing, or a share class that the vendor does not cover).");
        return Ok(1);
    }
    let snapshots: Vec<ConsensusEstimates> = dates.iter().filter_map(|d| consensus::load_snapshot(&cache, t, Some(d)).ok()).collect();
    if snapshots.is_empty() {
        return Err(format!("the snapshot files of {t} cannot be read"));
    }
    let price = match p.flag("--price") {
        Some(v) => Some((v.parse::<f64>().map_err(|_| format!("bad --price '{v}'"))?, "given by --price".to_string())),
        None => price_cache::last_close(&cache, t).map(|l| (l.close, format!("{}, {}", l.date, l.source))),
    };
    if let Some(w) = crate::sync::old_data_warning(repo, t, &[crate::sync::dataset::DataSet::Estimates, crate::sync::dataset::DataSet::Prices]) {
        eprintln!("fa: {w}");
    }
    let today = today_iso();
    let surprises = estimates_cache::load_surprises(&cache, t).map(|f| f.rows).unwrap_or_default();
    let next = earnings_dates::resolve_next(&cache, t, &today);
    use std::io::Write;
    let text = render_show(&ShowInput { snapshots: &snapshots, price, surprises: &surprises, next });
    let _ = std::io::stdout().write_all(text.as_bytes()); // a closed pipe (| head) is not an error
    Ok(0)
}

pub fn run_estimates(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{ESTIMATES_USAGE}");
            Ok(0)
        }
        Some("get") => estimates_get(repo, &args[1..]),
        Some("show") => estimates_show(repo, &args[1..]),
        Some(other) => Err(format!("unknown estimates command '{other}'; see fa estimates --help")),
    }
}

// ---------------------------------------------------------------- earnings

fn earnings_calendar(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let mut p = parse(rest, &["--tickers-file", "--from", "--days"], &["--refresh"])?;
    let listed = !p.tickers.is_empty() || p.flag("--tickers-file").is_some();
    if listed {
        collect_tickers(&mut p)?;
    }
    let today = today_iso();
    let from = p.flag("--from").map_or(today.clone(), |d| if d == "today" { today.clone() } else { d.to_string() });
    if parse_date(&from).is_none() {
        return Err(format!("bad --from '{from}' (YYYY-MM-DD)"));
    }
    let days: i64 = p.flag("--days").map_or(Ok(60), |d| d.parse().map_err(|_| format!("bad --days '{d}'")))?;
    if !(1..=400).contains(&days) {
        return Err("--days must be from 1 to 400".into());
    }
    let refresh = p.switches.iter().any(|s| s == "--refresh");
    let cache = cache_dir(repo);
    let dates = earnings_dates::scan_dates(&from, days);
    eprintln!("fa: earnings calendar, {} weekday(s) from {from} ({days} calendar days), into {}", dates.len(), cache.display());
    let get = |url: &str| super::download(url);
    let mut client = Client::new(&get, Duration::from_secs(60));
    let started = Instant::now();
    let total = dates.len();
    let st = earnings_dates::scan(&cache, &dates, &today, refresh, &mut client, &|i, d, what| eprintln!("[{}/{total}] {d}: {what}", i + 1));
    println!("calendar: {} weekday(s), {} request(s), {} from the cache, {} failed; {} company rows; {:.0} s", st.days, st.requests, st.cached, st.failed.len(), st.rows, started.elapsed().as_secs_f64());
    for (d, why) in st.failed.iter().take(10) {
        println!("  failed {d}: {why}");
    }
    // The dates that the scan covered (a stop at a block cuts the window).
    let covered: Vec<String> = match &st.stopped_at {
        Some(stop) => dates.iter().take_while(|d| *d != stop).cloned().collect(),
        None => dates.clone(),
    };
    if let Some(stop) = &st.stopped_at {
        println!("  stopped at {stop}: the source blocked the program ({}). The program does not get around a block; run the same command later.", client.blocked.clone().unwrap_or_default());
    }
    let next = earnings_dates::next_from_cache(&cache, &covered);
    println!("  {} symbols have a report date in the window", next.len());
    if !listed {
        println!("  give TICKER... or --tickers-file F for the table of the next date of each ticker");
        return Ok(if st.stopped_at.is_some() { 1 } else { 0 });
    }
    let through = covered.last().cloned().unwrap_or_else(|| from.clone());
    println!("{:<8}{:<12}{:<14}{:<9}{:>10}{:>6}  source", "ticker", "next date", "time", "quarter", "EPS fcst", "ests");
    let (mut found, mut estimated, mut none) = (0, 0, Vec::new());
    for n in earnings_dates::apply_calendar(&cache, &p.tickers, &covered, &from, &today) {
        let t = &n.ticker;
        match (&n.calendar, &n.value) {
            (Some((d, r)), _) => {
                found += 1;
                println!("{t:<8}{d:<12}{:<14}{:<9}{:>10}{:>6}  calendar", r.time, r.fiscal_quarter, fmt(r.eps_forecast, 2), fmt(r.estimates, 0));
            }
            (None, Some(e)) => {
                estimated += 1;
                println!("{t:<8}{:<12}{:<14}{:<9}{:>10}{:>6}  estimate (a year ago + 364 days)", e.date, "-", "-", "-", "-");
            }
            (None, None) => none.push(t.clone()),
        }
    }
    println!("{found} of {} ticker(s) have a date in the calendar window to {through}; {estimated} more have an estimate from the past dates; {} have none{}", p.tickers.len(), none.len(), if none.is_empty() { String::new() } else { format!(": {}", none.iter().take(30).cloned().collect::<Vec<_>>().join(" ")) });
    Ok(if st.stopped_at.is_some() || !st.failed.is_empty() { 1 } else { 0 })
}

fn earnings_dates_cmd(repo: &Path, rest: &[String]) -> Result<i32, String> {
    let mut p = parse(rest, &["--tickers-file", "--tolerance"], &[])?;
    collect_tickers(&mut p)?;
    let tolerance: i64 = p.flag("--tolerance").map_or(Ok(1), |t| t.parse().map_err(|_| format!("bad --tolerance '{t}'")))?;
    let cache = cache_dir(repo);
    let today = today_iso();
    let mut differences = Vec::new();
    for t in &p.tickers {
        println!("{t}");
        let file = earnings_dates::load_dates(&cache, t);
        let nasdaq: Vec<String> = file.as_ref().map(|f| f.past.iter().map(|p| p.date.clone()).collect()).unwrap_or_default();
        match earnings_dates::load_sec(&cache, t) {
            None if nasdaq.is_empty() => println!("  no past dates: run `fa estimates get {t}` (Nasdaq) and `mapcheck fetch --submissions {t}` (SEC)"),
            None => {
                println!("  past dates (Nasdaq; no SEC file: run `mapcheck fetch --submissions {t}` to check them):");
                for d in &nasdaq {
                    println!("    {d}");
                }
            }
            Some(sec) => {
                println!("  {:<12}{:<12}{:>6}  check", "Nasdaq", "SEC 8-K 2.02", "days");
                // The SEC list is long; the comparison starts at the first Nasdaq date (4 quarters and more).
                let rows = earnings_dates::compare_dates(&nasdaq, &sec, tolerance);
                let recent: Vec<&earnings_dates::DateRow> = if nasdaq.is_empty() { rows.iter().take(8).collect() } else { rows.iter().collect() };
                for r in recent {
                    let flag = match r.flag {
                        Flag::Same => "same",
                        Flag::Differs => "DIFFERS",
                        Flag::NasdaqOnly => "Nasdaq only (no SEC 8-K 2.02 within 10 days)",
                        Flag::SecOnly => "SEC only (Nasdaq has no date within 10 days)",
                    };
                    println!("  {:<12}{:<12}{:>6}  {flag}", r.nasdaq.as_deref().unwrap_or("-"), r.sec.as_deref().unwrap_or("-"), r.days.map_or("-".to_string(), |d| d.to_string()));
                    if r.flag == Flag::Differs {
                        differences.push((t.clone(), r.nasdaq.clone().unwrap_or_default(), r.sec.clone().unwrap_or_default(), r.days.unwrap_or(0)));
                    }
                }
            }
        }
        let next = earnings_dates::resolve_next(&cache, t, &today);
        match next {
            Some(n) => println!("  next: {}", describe_next(&n)),
            None => println!("  next: not known (run `fa earnings calendar {t}`)"),
        }
    }
    if p.tickers.len() > 1 || !differences.is_empty() {
        println!("dates more than {tolerance} day(s) apart: {}", differences.len());
        for (t, n, s, d) in &differences {
            println!("  {t}: Nasdaq {n}, SEC {s}, {d} days");
        }
    }
    Ok(if differences.is_empty() { 0 } else { 1 })
}

pub fn run_earnings(repo: &Path, args: &[String]) -> Result<i32, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{EARNINGS_USAGE}");
            Ok(0)
        }
        Some("calendar") => earnings_calendar(repo, &args[1..]),
        Some("dates") => earnings_dates_cmd(repo, &args[1..]),
        Some(other) => Err(format!("unknown earnings command '{other}'; see fa earnings --help")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::nasdaq_est as nq;

    fn snapshot(date: &str, scale: f64) -> ConsensusEstimates {
        let f = nq::parse_forecast(include_bytes!("../../tests/fixtures/market/nasdaq/forecast_MSFT.json")).unwrap();
        let mut f = f;
        for r in f.quarterly.iter_mut().chain(f.yearly.iter_mut()) {
            r.consensus = r.consensus.map(|c| c * scale);
        }
        let mut s = ConsensusEstimates::empty("MSFT", date, "nasdaq");
        nq::apply_forecast(&mut s, &f);
        nq::apply_targets(&mut s, &nq::parse_targets(include_bytes!("../../tests/fixtures/market/nasdaq/targetprice_MSFT.json")).unwrap());
        s
    }

    #[test]
    fn show_has_the_consensus_the_target_against_the_price_and_the_revisions() {
        let snaps = vec![snapshot("2026-11-05", 0.98), snapshot("2026-11-19", 1.0)];
        let surprises = nq::parse_surprises(include_bytes!("../../tests/fixtures/market/nasdaq/surprise_MSFT.json")).unwrap();
        let next = Some(NextDate { date: "2027-01-27".into(), time: "after-hours".into(), fiscal_quarter: "2026-12".into(), eps_forecast: Some(4.86), estimates: Some(14.0), source: "nasdaq calendar".into() });
        let text = render_show(&ShowInput { snapshots: &snaps, price: Some((400.0, "2026-11-18, nasdaq".into())), surprises: &surprises, next });
        assert!(text.contains("MSFT: consensus of 2026-11-19 (source nasdaq"), "{text}");
        // Year 0y: mean 19.91, forward P/E 400 / 19.91 = 20.1, no growth for the first year; +1y: growth 19.6%.
        assert!(text.contains("19.91") && text.contains("20.1"), "{text}");
        assert!(text.contains("+19.6%"), "{text}");
        // Revisions between the two snapshots: 19.91 against 19.91 * 0.98 is +2.0%.
        assert!(text.contains("revisions of the mean EPS (2 snapshots, 2026-11-05 to 2026-11-19)") && text.contains("+2.0%"), "{text}");
        // Price target 632.48 against 400: +58.1%. Ratings from the targets endpoint.
        assert!(text.contains("mean 632.48 (+58.1%)") && text.contains("34 buy, 4 hold, 0 sell"), "{text}");
        assert!(text.contains("revenue estimates: none"), "{text}");
        assert!(text.contains("2026-10-28 quarter 2026-09  EPS   4.89 consensus   4.71 surprise  +3.82%"), "{text}");
        assert!(text.contains("next earnings: 2027-01-27 (after-hours)"), "{text}");
        // Without a price and with one snapshot the text says so and does not invent numbers.
        let one = render_show(&ShowInput { snapshots: &snaps[1..], price: None, surprises: &[], next: None });
        assert!(one.contains("no price in the cache") && one.contains("one snapshot only") && one.contains("next earnings: not known") && one.contains("none in the cache"), "{one}");
        assert!(!one.contains("fwd P/E 2"), "{one}");
    }

    #[test]
    fn arguments_of_get() {
        let dir = std::env::temp_dir().join(format!("fa-estcli-{}", std::process::id()));
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(estimates_get(&dir, &args(&["MSFT", "--endpoints", "nope"])).unwrap_err().contains("unknown endpoint"));
        assert!(estimates_get(&dir, &args(&[])).unwrap_err().contains("at least one ticker"));
        assert_eq!(estimates_show(&dir, &args(&["MSFT"])), Ok(1));
        assert!(earnings_calendar(&dir, &args(&["--days", "0"])).is_err());
        assert!(earnings_calendar(&dir, &args(&["--from", "2026-13-01"])).is_err());
        assert!(run_estimates(&dir, &args(&["bogus"])).is_err() && run_earnings(&dir, &args(&["bogus"])).is_err());
    }

    #[test]
    fn revenue_of_fmp_goes_to_the_period_with_the_same_month() {
        use crate::market::fmp::EarningsEstimate;
        let mut s = snapshot("2026-11-19", 1.0);
        let est = |end: &str, kind: &str, revenue: f64| EarningsEstimate { period_end: end.into(), period_type: kind.into(), estimated_eps: None, estimated_revenue: Some(revenue), number_of_analysts: Some(20.0) };
        let n = consensus::merge_revenue(&mut s, &[est("2027-06-30", "annual", 3.2e11), est("2026-12-31", "quarterly", 8.1e10), est("2028-06-30", "annual", 3.6e11), est("2027-12-31", "annual", 1.0)]);
        assert_eq!(n, 3);
        let text = render_show(&ShowInput { snapshots: std::slice::from_ref(&s), price: None, surprises: &[], next: None });
        assert!(text.contains("revenue estimates (FMP): 0q 81.00B 0y 320.00B +1y 360.00B"), "{text}");
        assert!(s.endpoints.contains(&"fmp-revenue".to_string()));
    }
}
