//! `fa analyze TICKER`: import the company, add the market data, write a nine-sheet xlsx file
//! (the Rust form of `scripts/analyze.py`).
//!
//! The sheets, the columns, the sequence of the rows and the values are those of the Python
//! command. The cells have the format "General" (no number format), as the openpyxl file had. These
//! are the differences, on purpose:
//!
//! - A source text that starts with `=` (a formula of the registry, for example
//!   `= revenue - cogs_total`) is a text cell. openpyxl made a formula of it, and Excel shows `#NAME?`.
//! - The source text of the two derived rows (`free_cash_flow`, `fcf_margin`) does not name a Python file.
//! - A value that is not a number (NaN) is an empty cell (openpyxl did the same).
//!
//! The data sources are in `market`; the SEC import is `mapcheck::import`.

use crate::company::Company;
use crate::market::consensus::{self, ConsensusEstimates};
use crate::market::estimates_cache;
use crate::market::fmp::{self, EarningsEstimate, EarningsSurprise, Fmp, FmpError};
use crate::market::options::{self, compute_chain_metrics, OptionChainSnapshot};
use crate::market::prices::{self, PricePoint};
use mapcheck::import::{import_ticker, ImportError};
use mapcheck::registry::Registry;
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};

pub const COLUMNS: [&str; 11] = ["ticker", "cik", "company_name", "period_end", "form_type", "filed_date", "statement", "metric", "value", "unit", "source"];
pub const PRICE_COLUMNS: [&str; 7] = ["ticker", "date", "open", "high", "low", "close", "volume"];
pub const ESTIMATE_COLUMNS: [&str; 6] = ["ticker", "period_end", "period_type", "metric", "value", "unit"];
pub const ESTIMATES_FWD_COLUMNS: [&str; 6] = ["ticker", "snapshot_date", "kind", "period", "metric", "value"];
pub const OPTION_COLUMNS: [&str; 13] = [
    "ticker", "snapshot_date", "expiry", "option_type", "contract_symbol", "strike", "last_price", "bid", "ask", "volume", "open_interest", "implied_volatility", "in_the_money",
];
pub const OPTION_SUMMARY_COLUMNS: [&str; 15] = [
    "ticker", "snapshot_date", "spot_price", "expiry", "atm_strike", "atm_iv", "straddle_mid", "implied_move", "skew", "call_volume", "put_volume", "call_open_interest",
    "put_open_interest", "put_call_volume_ratio", "put_call_oi_ratio",
];

pub const SHEETS: [&str; 9] = ["Income Statement", "Balance Sheet", "Cash Flow", "Raw", "Prices", "Estimates", "Estimates Fwd", "Options", "Options Summary"];

/// Free cash flow and its margin are not variables of the registry; they are derived rows of the Cash Flow sheet.
const DERIVED_SOURCE: &str = "= derived (free_cash_flow = operating_cash_flow - abs(capex_total, else capex); fcf_margin = free_cash_flow / revenue)";

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    Str(String),
    Num(f64),
    Bool(bool),
}

impl Cell {
    /// An empty text and a number that is not finite are empty cells.
    pub fn text(s: &str) -> Cell {
        if s.is_empty() {
            Cell::Empty
        } else {
            Cell::Str(s.to_string())
        }
    }
    pub fn num(x: f64) -> Cell {
        if x.is_finite() {
            Cell::Num(x)
        } else {
            Cell::Empty
        }
    }
    pub fn opt(x: Option<f64>) -> Cell {
        x.map_or(Cell::Empty, Cell::num)
    }
    pub fn int(x: i64) -> Cell {
        Cell::Num(x as f64)
    }
    pub fn opt_int(x: Option<i64>) -> Cell {
        x.map_or(Cell::Empty, Cell::int)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub name: &'static str,
    /// The first row is the header.
    pub rows: Vec<Vec<Cell>>,
}

/// The market data of one company.
#[derive(Debug, Clone, Default)]
pub struct Market {
    pub prices: Vec<PricePoint>,
    pub estimates: Vec<EarningsEstimate>,
    pub surprises: Vec<EarningsSurprise>,
    pub consensus: Option<ConsensusEstimates>,
    pub option_chain: Option<OptionChainSnapshot>,
}

/// The unit text of a variable: `USD`, except for the per-share and the share lines and the margin.
pub fn unit_of(metric: &str) -> &'static str {
    match metric {
        "eps_basic" | "eps_diluted" => "USD/share",
        "shares_basic" | "shares_diluted" | "shares_outstanding" => "shares",
        "fcf_margin" => "ratio",
        _ => "USD",
    }
}

/// Income statement, balance sheet and cash flow have a sheet each. A disclosure (a note, for example
/// the remaining performance obligation) is a line of the Balance Sheet sheet; its `statement` column says `disclosure`.
fn sheet_for(statement: &str) -> usize {
    match statement {
        "income_statement" => 0,
        "cash_flow" => 2,
        _ => 1,
    }
}

fn header(columns: &[&str]) -> Vec<Cell> {
    columns.iter().map(|c| Cell::text(c)).collect()
}

/// All sheets of the file, from the company and the market data.
pub fn flatten(registry: &Registry, company: &Company, market: &Market) -> Vec<Sheet> {
    let mut sheets: Vec<Sheet> = SHEETS
        .iter()
        .map(|name| {
            let columns: &[&str] = match *name {
                "Prices" => &PRICE_COLUMNS,
                "Estimates" => &ESTIMATE_COLUMNS,
                "Estimates Fwd" => &ESTIMATES_FWD_COLUMNS,
                "Options" => &OPTION_COLUMNS,
                "Options Summary" => &OPTION_SUMMARY_COLUMNS,
                _ => &COLUMNS,
            };
            Sheet { name, rows: vec![header(columns)] }
        })
        .collect();

    for p in &company.periods {
        let line = |statement: &str, metric: &str, value: f64, unit: &str, source: &str| -> Vec<Cell> {
            vec![
                Cell::text(&p.ticker),
                Cell::text(&company.cik),
                Cell::text(&company.company_name),
                Cell::text(&p.period_end),
                Cell::text(&p.form),
                Cell::text(&p.filed_date),
                Cell::text(statement),
                Cell::text(metric),
                Cell::num(value),
                Cell::text(unit),
                Cell::text(source),
            ]
        };
        for (i, v) in registry.variables.iter().enumerate() {
            if let Some(value) = p.values[i] {
                let source = p.sources.get(&v.name).map_or("", String::as_str);
                sheets[sheet_for(&v.statement)].rows.push(line(&v.statement, &v.name, value, unit_of(&v.name), source));
            }
        }
        for (name, value) in [("free_cash_flow", p.free_cash_flow(registry)), ("fcf_margin", p.fcf_margin(registry))] {
            if let Some(value) = value {
                sheets[2].rows.push(line("cash_flow", name, value, unit_of(name), DERIVED_SOURCE));
            }
        }
        for (concept, value) in &p.raw_extras {
            sheets[3].rows.push(line("raw", concept, *value, "USD", "no standard variable"));
        }
    }

    for pp in &market.prices {
        sheets[4].rows.push(vec![
            Cell::text(&company.ticker),
            Cell::text(&pp.date),
            Cell::num(pp.open),
            Cell::num(pp.high),
            Cell::num(pp.low),
            Cell::num(pp.close),
            Cell::int(pp.volume),
        ]);
    }

    let estimates = &mut sheets[5].rows;
    for e in &market.estimates {
        let row = |metric: &str, value: f64, unit: &str| {
            vec![Cell::text(&company.ticker), Cell::text(&e.period_end), Cell::text(&e.period_type), Cell::text(metric), Cell::num(value), Cell::text(unit)]
        };
        if let Some(v) = e.estimated_eps {
            estimates.push(row("estimated_eps", v, "USD/share"));
        }
        if let Some(v) = e.estimated_revenue {
            estimates.push(row("estimated_revenue", v, "USD"));
        }
        if let Some(v) = e.number_of_analysts {
            estimates.push(row("number_of_analysts", v, "count"));
        }
    }
    for s in &market.surprises {
        for (metric, value) in [("actual_eps", s.actual_eps), ("estimated_eps", s.estimated_eps), ("surprise", s.surprise), ("surprise_pct", s.surprise_pct)] {
            let unit = if metric.contains("eps") || metric == "surprise" { "USD/share" } else { "ratio" };
            estimates.push(vec![
                Cell::text(&company.ticker),
                Cell::text(&s.date),
                Cell::text("surprise"),
                Cell::text(metric),
                Cell::num(value),
                Cell::text(unit),
            ]);
        }
    }

    if let Some(c) = &market.consensus {
        let rows = &mut sheets[6].rows;
        let row = |kind: &str, period: &str, metric: &str, value: f64| {
            vec![Cell::text(&c.ticker), Cell::text(&c.snapshot_date), Cell::text(kind), Cell::text(period), Cell::text(metric), Cell::num(value)]
        };
        for (kind, periods) in [("revenue_estimate", &c.revenue_estimate), ("earnings_estimate", &c.earnings_estimate)] {
            for (period, metrics) in &periods.0 {
                for (metric, value) in &metrics.0 {
                    if let Some(v) = value {
                        rows.push(row(kind, period, metric, *v));
                    }
                }
            }
        }
        for (metric, value) in &c.analyst_price_targets.0 {
            if let Some(v) = value {
                rows.push(row("price_target", "current", metric, *v));
            }
        }
        // Fields of the Nasdaq snapshot (the snapshots of the Python code have none of them).
        for (metric, value) in &c.ratings.0 {
            if let Some(v) = value {
                rows.push(row("ratings", "current", metric, *v));
            }
        }
        for (key, value) in &c.consensus_change.0 {
            if let (Some(v), Some((scope, when))) = (value, key.split_once('_')) {
                rows.push(row("consensus_change", if scope == "qtr" { "0q" } else { "0y" }, when, *v));
            }
        }
    }

    if let Some(snap) = &market.option_chain {
        for q in &snap.quotes {
            sheets[7].rows.push(vec![
                Cell::text(&snap.ticker),
                Cell::text(&snap.snapshot_date),
                Cell::text(&q.expiry),
                Cell::text(&q.option_type),
                Cell::text(&q.contract_symbol),
                Cell::num(q.strike),
                Cell::opt(q.last_price),
                Cell::opt(q.bid),
                Cell::opt(q.ask),
                Cell::opt_int(q.volume),
                Cell::opt_int(q.open_interest),
                Cell::opt(q.implied_volatility),
                q.in_the_money.map_or(Cell::Empty, Cell::Bool),
            ]);
        }
        let metrics = compute_chain_metrics(snap);
        let rows = &mut sheets[8].rows;
        let lead = |expiry: &str| vec![Cell::text(&snap.ticker), Cell::text(&snap.snapshot_date), Cell::opt(snap.spot_price), Cell::text(expiry)];
        for em in &metrics.expiries {
            let mut r = lead(&em.expiry);
            r.extend([
                Cell::opt(em.atm_strike),
                Cell::opt(em.atm_iv),
                Cell::opt(em.straddle_mid),
                Cell::opt(em.implied_move),
                Cell::opt(em.skew),
                Cell::int(em.call_volume),
                Cell::int(em.put_volume),
                Cell::int(em.call_open_interest),
                Cell::int(em.put_open_interest),
                Cell::opt(em.put_call_volume_ratio()),
                Cell::opt(em.put_call_oi_ratio()),
            ]);
            rows.push(r);
        }
        if !metrics.expiries.is_empty() {
            let sum = |f: fn(&options::ExpiryMetrics) -> i64| metrics.expiries.iter().map(f).sum::<i64>();
            let mut r = lead("ALL");
            r.extend([
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::int(sum(|e| e.call_volume)),
                Cell::int(sum(|e| e.put_volume)),
                Cell::int(sum(|e| e.call_open_interest)),
                Cell::int(sum(|e| e.put_open_interest)),
                Cell::opt(metrics.put_call_volume_ratio),
                Cell::opt(metrics.put_call_oi_ratio),
            ]);
            rows.push(r);
        }
    }
    sheets
}

/// The number of rows that are not header rows.
pub fn data_rows(sheets: &[Sheet]) -> usize {
    sheets.iter().map(|s| s.rows.len() - 1).sum()
}

/// Write the file. The folder is made if it does not exist. Returns the number of data rows.
pub fn write_xlsx(sheets: &[Sheet], path: &Path) -> Result<usize, String> {
    let mut workbook = Workbook::new();
    for sheet in sheets {
        let ws = workbook.add_worksheet();
        ws.set_name(sheet.name).map_err(|e| e.to_string())?;
        for (r, row) in sheet.rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let (r, c) = (r as u32, c as u16);
                match cell {
                    Cell::Empty => continue,
                    Cell::Str(s) => ws.write_string(r, c, s),
                    Cell::Num(x) => ws.write_number(r, c, *x),
                    Cell::Bool(b) => ws.write_boolean(r, c, *b),
                }
                .map_err(|e| format!("{} row {}: {e}", sheet.name, r + 1))?;
            }
        }
    }
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    workbook.save(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(data_rows(sheets))
}

#[derive(Debug, Default, PartialEq)]
pub struct Options {
    pub refresh: bool,
    pub out: Option<PathBuf>,
    pub no_market: bool,
    pub no_options: bool,
    pub no_estimates: bool,
}

pub const USAGE: &str = "\
fa analyze TICKER [--refresh] [--out FILE] [--no-market] [--no-options] [--no-estimates]

Import the company (SEC company facts, cache first), add the market data, and write the nine-sheet
xlsx file (default: data/processed/<TICKER>_financials.xlsx).

  --refresh       get the SEC company facts, the prices, the option chain and the FMP data again
  --out FILE      the output file
  --no-market     skip prices, estimates, surprises and options
  --no-options    skip the option chain
  --no-estimates  skip the forward consensus snapshot

Sources (one request for each source, 2 seconds apart): prices from the Yahoo chart endpoint, the option
chain from the Cboe delayed quotes, the revenue estimates from Financial Modeling Prep (key: FMP_API_KEY
in the environment or in .env). The forward consensus (EPS, price targets, ratings), the earnings surprises
and the earnings dates are read from the cache that `fa estimates get` and `fa earnings calendar` write
(Nasdaq site API, no key); this command makes no request to that source. SEC requests use the contact in the file sec_contact.toml of the repo (the environment variable SEC_USER_AGENT replaces it).";

pub fn parse_args(args: &[String]) -> Result<(String, Options), String> {
    let mut ticker = None;
    let mut o = Options::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--refresh" => o.refresh = true,
            "--no-market" => o.no_market = true,
            "--no-options" => o.no_options = true,
            "--no-estimates" => o.no_estimates = true,
            "--out" => o.out = Some(PathBuf::from(it.next().ok_or("--out needs a file name")?)),
            s if s.starts_with('-') => return Err(format!("unknown option '{s}'")),
            s => {
                if ticker.replace(s.to_uppercase()).is_some() {
                    return Err("give one ticker".to_string());
                }
            }
        }
    }
    Ok((ticker.ok_or("analyze needs a ticker")?, o))
}

/// Where the market data comes from. The real sources, or (in a test) functions.
pub struct Sources<'a> {
    pub today: String,
    pub yahoo: &'a dyn Fn(&str) -> Result<Vec<u8>, String>,
    pub cboe: &'a dyn Fn(&str) -> Result<Vec<u8>, String>,
    pub fmp: Option<Fmp<'a>>,
}

/// Get the market data and print one line for each step (the steps of the Python command).
pub fn load_market(cache: &Path, ticker: &str, o: &Options, src: &Sources) -> Market {
    let mut m = Market::default();
    if o.no_market {
        return m;
    }
    println!("Fetching stock prices (Yahoo chart endpoint)...");
    match prices::load_prices(cache, ticker, o.refresh, src.yahoo) {
        Ok((p, source)) => {
            if let prices::Source::CacheAfterFailure(reason) = &source {
                println!("  Warning: refresh failed ({reason}); the cache file stays and is used");
            }
            println!("  {} price points loaded", p.len());
            m.prices = p;
        }
        Err(e) => println!("  Warning: price fetch failed: {e}"),
    }

    // The earnings surprises: the Nasdaq file (`fa estimates get`) first, then the cache file of the Python code.
    println!("Reading earnings surprises (cache: fa estimates get; the older Python cache file is the second choice)...");
    let nasdaq_surprises = estimates_cache::surprises_for_sheet(cache, ticker, &src.today).filter(|s| !s.is_empty());
    match nasdaq_surprises {
        Some(s) => {
            println!("  {} earnings surprise records loaded (Nasdaq file)", s.len());
            m.surprises = s;
        }
        None => match consensus::cached_surprises(cache, ticker) {
            Some(s) => {
                println!("  {} earnings surprise records loaded (Python cache file)", s.len());
                m.surprises = s;
            }
            None => println!("  no earnings cache for {ticker}; run: fa estimates get {ticker}"),
        },
    }

    println!("Fetching analyst estimates (FMP, optional; the revenue estimates)...");
    let estimates = fmp::load_estimates(cache, ticker, o.refresh, src.fmp.as_ref());
    match estimates {
        Ok(e) => {
            // FMP surprises only when the cache has no surprises of Nasdaq or of the Python code.
            if m.surprises.is_empty() {
                match fmp::load_surprises(cache, ticker, o.refresh, src.fmp.as_ref()) {
                    Ok(s) if !s.is_empty() => m.surprises = s,
                    Ok(_) => {}
                    Err(e) => println!("  Warning: FMP surprises failed: {e}"),
                }
            }
            println!("  {} FMP estimate records loaded", e.len());
            m.estimates = e;
        }
        Err(FmpError::NoKey) => println!("  FMP unavailable ({}): no revenue estimates", FmpError::NoKey),
        Err(e) => println!("  FMP unavailable ({e}): no revenue estimates"),
    }

    if !o.no_estimates {
        println!("Reading forward consensus estimates (snapshot files; fa estimates get makes them)...");
        let (mut snapshot, text) = consensus::snapshot_for_report(cache, ticker, &src.today);
        println!("  {text}");
        // The Nasdaq snapshot has no revenue: the FMP records fill it in memory (the file stays as it is).
        if let Some(s) = snapshot.as_mut().filter(|s| s.revenue_estimate.is_empty()) {
            let n = consensus::merge_revenue(s, &m.estimates);
            if n > 0 {
                println!("  revenue estimates of FMP for {n} period(s)");
            }
        }
        m.consensus = snapshot;
    }

    if !o.no_options {
        println!("Fetching option chain (Cboe delayed quotes)...");
        match options::fetch_option_chain(cache, ticker, o.refresh, &src.today, src.cboe) {
            Ok((snapshot, _)) => {
                let n_expiries = snapshot.quotes.iter().map(|q| q.expiry.as_str()).collect::<std::collections::BTreeSet<_>>().len();
                println!("  {} option contracts across {n_expiries} expiries loaded", snapshot.quotes.len());
                m.option_chain = Some(snapshot);
            }
            Err(e) => println!("  Warning: option-chain fetch failed: {}", prices::explain(&e)),
        }
    }
    m
}

/// `fa analyze`. Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> Result<i32, String> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let (ticker, o) = parse_args(args)?;
    let cache = repo.join("data").join("raw");
    let store_dir = repo.join("data").join("processed");
    let out = o.out.clone().unwrap_or_else(|| store_dir.join(format!("{ticker}_financials.xlsx")));

    println!("Importing {ticker}...");
    let registry = Registry::embedded()?;
    let imported = match import_ticker(&registry, &cache, &store_dir, &ticker, 12, 0.15, o.refresh) {
        Ok(i) => i,
        Err(ImportError::Unsupported(e)) => {
            println!("XBRL import failed ({e}). IFRS filers are not supported. Exiting.");
            return Ok(1);
        }
        Err(ImportError::Failed(e)) => {
            println!("SEC error: {e}");
            return Ok(1);
        }
    };
    let company = Company::from_store(&registry, &imported.store)?;

    let key = if o.no_market { None } else { fmp::api_key(repo) };
    let yahoo = |url: &str| crate::market::download(url);
    let cboe = |url: &str| crate::market::download(url);
    let http = key.as_deref().map(fmp::http_fetch);
    let fmp = match (&key, &http) {
        (Some(k), Some(h)) => Some(Fmp::new(k, h)),
        _ => None,
    };
    let sources = Sources { today: crate::market::today_iso(), yahoo: &yahoo, cboe: &cboe, fmp };
    let market = load_market(&cache, &ticker, &o, &sources);

    println!("Writing {} ...", out.display());
    let sheets = flatten(&registry, &company, &market);
    let total = write_xlsx(&sheets, &out)?;
    println!("Done: {} ({total} rows across {} tabs)", out.display(), sheets.len());
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../tests/fixtures/analyze/expected_sheets.json");

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).unwrap()
    }

    /// The consensus snapshot of the fixture, with the key order of the file (a `Value` would sort the keys).
    fn ordered_consensus() -> ConsensusEstimates {
        #[derive(serde::Deserialize)]
        struct Input {
            consensus: ConsensusEstimates,
        }
        #[derive(serde::Deserialize)]
        struct Wrapper {
            input: Input,
        }
        serde_json::from_str::<Wrapper>(FIXTURE).unwrap().input.consensus
    }

    fn company() -> (Registry, Company) {
        let registry = Registry::embedded().unwrap();
        let path = format!("{}/tests/fixtures/analyze/company_store.json", env!("CARGO_MANIFEST_DIR"));
        let store: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let c = Company::from_store(&registry, &store).unwrap();
        (registry, c)
    }

    fn market(input: &Value) -> Market {
        let f = |v: &Value| v.as_f64();
        let nan = |v: &Value| v.as_f64().unwrap_or(f64::NAN);
        Market {
            prices: input["prices"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| PricePoint {
                    date: p["date"].as_str().unwrap().into(),
                    open: f(&p["open"]).unwrap(),
                    high: f(&p["high"]).unwrap(),
                    low: f(&p["low"]).unwrap(),
                    close: f(&p["close"]).unwrap(),
                    volume: p["volume"].as_i64().unwrap(),
                })
                .collect(),
            estimates: input["estimates"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| EarningsEstimate {
                    period_end: e["period_end"].as_str().unwrap().into(),
                    period_type: e["period_type"].as_str().unwrap().into(),
                    estimated_eps: f(&e["estimated_eps"]),
                    estimated_revenue: f(&e["estimated_revenue"]),
                    number_of_analysts: f(&e["number_of_analysts"]),
                })
                .collect(),
            surprises: input["surprises"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| EarningsSurprise {
                    date: s["date"].as_str().unwrap().into(),
                    actual_eps: nan(&s["actual_eps"]),
                    estimated_eps: nan(&s["estimated_eps"]),
                    surprise: nan(&s["surprise"]),
                    surprise_pct: nan(&s["surprise_pct"]),
                })
                .collect(),
            consensus: Some(ordered_consensus()),
            option_chain: Some(serde_json::from_value(input["option_chain"].clone()).unwrap()),
        }
    }

    fn cell(v: &Value) -> Cell {
        match v {
            Value::Null => Cell::Empty,
            Value::String(s) => Cell::Str(s.clone()),
            Value::Bool(b) => Cell::Bool(*b),
            Value::Number(n) => Cell::Num(n.as_f64().unwrap()),
            other => panic!("{other}"),
        }
    }

    /// The Python code (`_flatten` and the column lists of `write_xlsx`) made `expected_sheets.json` from the same
    /// data. Each cell must be the same; the two differences of the module text are replaced here.
    #[test]
    fn sheets_match_the_python_cells() {
        let (registry, company) = company();
        let f = fixture();
        let sheets = flatten(&registry, &company, &market(&f["input"]));
        let expected = f["sheets"].as_array().unwrap();
        assert_eq!(sheets.len(), expected.len());
        let (mut cells, mut derived) = (0, 0);
        for (sheet, want) in sheets.iter().zip(expected) {
            assert_eq!(sheet.name, want["name"].as_str().unwrap());
            let rows = want["rows"].as_array().unwrap();
            assert_eq!(sheet.rows.len(), rows.len(), "{} row count", sheet.name);
            for (r, (got_row, want_row)) in sheet.rows.iter().zip(rows).enumerate() {
                let want_row = want_row.as_array().unwrap();
                assert_eq!(got_row.len(), want_row.len(), "{} row {r}", sheet.name);
                for (c, (got, want)) in got_row.iter().zip(want_row).enumerate() {
                    cells += 1;
                    let want = cell(want);
                    if matches!(&want, Cell::Str(s) if s == "= derived (financial_analysis/schema.py)") {
                        assert_eq!(got, &Cell::Str(DERIVED_SOURCE.to_string()));
                        derived += 1;
                        continue;
                    }
                    assert_eq!(got, &want, "{} row {r} column {c}", sheet.name);
                }
            }
        }
        assert!(cells > 1000, "{cells}");
        assert_eq!(derived, 4, "two derived rows for each of the two periods");
        let free_cash_flow = sheets[2].rows.iter().filter(|r| r[7] == Cell::text("free_cash_flow")).count();
        assert_eq!(free_cash_flow, 2);
    }

    #[test]
    fn sheet_names_and_unit_rules() {
        let (registry, company) = company();
        let sheets = flatten(&registry, &company, &Market::default());
        assert_eq!(sheets.iter().map(|s| s.name).collect::<Vec<_>>(), SHEETS);
        // No market data: the market sheets have the header row only.
        for s in &sheets[4..] {
            assert_eq!(s.rows.len(), 1, "{}", s.name);
        }
        assert_eq!((unit_of("eps_basic"), unit_of("shares_outstanding"), unit_of("fcf_margin"), unit_of("revenue")), ("USD/share", "shares", "ratio", "USD"));
    }

    #[test]
    fn text_cells_stay_text_and_nan_is_empty() {
        assert_eq!(Cell::text(""), Cell::Empty);
        assert_eq!(Cell::num(f64::NAN), Cell::Empty);
        assert_eq!(Cell::opt(None), Cell::Empty);
        let dir = std::env::temp_dir().join(format!("fa-xlsx-{}", std::process::id()));
        let sheets = vec![Sheet { name: "Raw", rows: vec![vec![Cell::text("a"), Cell::Num(1.5), Cell::Bool(true), Cell::Empty, Cell::text("= revenue - cogs_total")]] }];
        let path = dir.join("t.xlsx");
        assert_eq!(write_xlsx(&sheets, &path).unwrap(), 0);
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..2], b"PK", "an xlsx file is a zip file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn arguments() {
        let a = |s: &[&str]| parse_args(&s.iter().map(|x| x.to_string()).collect::<Vec<_>>());
        let (t, o) = a(&["ibm", "--refresh", "--no-options", "--out", "x.xlsx"]).unwrap();
        assert_eq!(t, "IBM");
        assert_eq!(o, Options { refresh: true, out: Some("x.xlsx".into()), no_market: false, no_options: true, no_estimates: false });
        assert!(a(&[]).is_err() && a(&["A", "B"]).is_err() && a(&["A", "--bogus"]).is_err() && a(&["A", "--out"]).is_err());
    }

    #[test]
    fn market_steps_use_caches_and_say_when_there_is_no_source() {
        let dir = std::env::temp_dir().join(format!("fa-market-{}", std::process::id()));
        let chart = std::fs::read_to_string(format!("{}/tests/fixtures/market/yahoo_prices.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let chart: Value = serde_json::from_str(&chart).unwrap();
        let reply = chart["chart"].to_string().into_bytes();
        let yahoo = |_: &str| -> Result<Vec<u8>, String> { Ok(reply.clone()) };
        let cboe = |u: &str| -> Result<Vec<u8>, String> { Err(format!("HTTP 404 for {u}")) };
        let sources = Sources { today: "2026-11-19".into(), yahoo: &yahoo, cboe: &cboe, fmp: None };
        let m = load_market(&dir, "TEST", &Options::default(), &sources);
        assert_eq!(m.prices.len(), 5);
        assert!(m.estimates.is_empty() && m.surprises.is_empty() && m.consensus.is_none() && m.option_chain.is_none());
        assert!(prices::cache_path(&dir, "TEST").is_file(), "the prices are cached");
        // --no-market: nothing is read, nothing is asked.
        let never = |_: &str| -> Result<Vec<u8>, String> { panic!("no request") };
        let s2 = Sources { today: "2026-11-19".into(), yahoo: &never, cboe: &never, fmp: None };
        let m2 = load_market(&dir, "TEST", &Options { no_market: true, ..Default::default() }, &s2);
        assert!(m2.prices.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
