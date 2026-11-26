//! `fa fund attribution TICKER`: layers 1 and 2 of an ETF from its SEC N-PORT reports (no request: the reports are
//! read from the cache that `fa fund holdings TICKER --from D` fills).
//!
//! Layer 1, for each report date: the sum of the holdings plus the other assets minus the liabilities against the net
//! assets; and for each quarter: the price return of the fund (Nasdaq closes) against its NAV return (the 3 monthly
//! total returns of N-PORT; rtn1 is the newest month). The difference is the change of the premium or discount, plus the distributions.
//!
//! Layer 2, for each quarter: the weight at the start (valUSD / net assets) times the USD return of the holding over the
//! quarter (price = valUSD / balance at both report dates). The parts are summed by group. The fund NAV return minus the
//! sum is the remainder, named: "trading in the quarter, cash, fee, securities lending, other".
//!
//! The monthly view: the US-listed part from Nasdaq month-end closes with the start-of-quarter weights, against the
//! monthly NAV returns of the fund. The foreign part has a quarterly value only.
//! Model document: `docs/models/fund-attribution.md`.

use super::fund_holdings::{read_cached_reports, select_reports, Loaded, MonthlyReturn, Report};
use super::price_cache;
use super::prices::PricePoint;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa fund attribution - layers 1 and 2 of an ETF: the net assets, the premium, and the parts of the return

USAGE:
  fa fund attribution TICKER [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--cache DIR] [--top N]
    --cache DIR   default data/raw. It reads DIR/TICKER/nport/*.xml (fill it with: fa fund holdings TICKER --from D)
                  and the closes DIR/<T>/prices.json of the fund and of the US-listed holdings (fa prices get T --source nasdaq).
    --top N       the holdings shown for each quarter (default 6), sorted by the size of the part
No request is sent. The US ticker of a holding comes from the file rust/src/market/fund_ticker_map.csv (URA only).";

const MAP_CSV: &str = include_str!("fund_ticker_map.csv");
/// A quarter is the gap between two reports: from 80 to 100 days. Another gap is a missing report.
const GAP_DAYS: std::ops::RangeInclusive<i64> = 80..=100;
/// A price factor outside this range in one quarter gets a flag (a split, a currency change or a real move).
const JUMP_LOW: f64 = 1.0 / 3.0;
const JUMP_HIGH: f64 = 3.0;
/// A close for a date is the last close up to this many calendar days before it.
const MAX_STALE_DAYS: i64 = 5;
/// The remainder above this (in percentage points) gets a warning.
const REMAINDER_WARN_PP: f64 = 1.0;
/// The largest difference of the N-PORT price and the Nasdaq close of a mapped holding that is normal (different close
/// times and currency rates; 4.8% seen). Above it the map row is suspect: a wrong ticker gives a much larger difference.
pub const MAP_PRICE_TOLERANCE: f64 = 0.08;
/// The share splits and consolidations that the program knows (a factor k means k shares for one, or one for k).
const SPLIT_FACTORS: [f64; 8] = [2.0, 3.0, 4.0, 5.0, 10.0, 20.0, 50.0, 100.0];
/// The balance may differ from a split factor by this much (the natural log: 0.45 is x1.57): the fund also trades.
const SPLIT_BALANCE_LOG_TOLERANCE: f64 = 0.45;
/// The price factor times the split factor (the value factor) must be inside this range.
const SPLIT_VALUE_RANGE: std::ops::RangeInclusive<f64> = 0.4..=2.5;

/// The ISIN to US ticker map (`fund_ticker_map.csv`).
pub fn parse_map(csv: &str) -> HashMap<String, String> {
    csv.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut p = l.split(',');
            let (k, t) = (p.next()?.trim(), p.next()?.trim());
            (!k.is_empty() && !t.is_empty() && k != "key").then(|| (k.to_string(), t.to_string()))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Isin,
    Cusip,
    Name,
}

/// One position: the lines of a report with the same identifier, added.
#[derive(Debug, Clone, PartialEq)]
pub struct Pos {
    pub name: String,
    pub isin: String,
    pub cusip: String,
    pub currency: String,
    pub val_usd: f64,
    pub balance: f64,
}

impl Pos {
    pub fn price(&self) -> f64 {
        self.val_usd / self.balance
    }
}

fn clean_id(s: &str) -> &str {
    let s = s.trim();
    if s.eq_ignore_ascii_case("n/a") { "" } else { s }
}

/// The equity positions of a report (long, common equity, units NS, a positive balance and value), and the sum of valUSD of
/// every other line (repo, money market, collateral, derivative, short): those lines have no price from valUSD / balance.
pub fn positions(r: &Report) -> (Vec<Pos>, f64) {
    let mut out: Vec<Pos> = Vec::new();
    let mut other = 0.0;
    for h in &r.holdings {
        let ok = h.asset_cat == "EC" && h.units == "NS" && h.payoff != "Short" && h.balance > 0.0 && h.val_usd > 0.0;
        if !ok {
            other += h.val_usd;
            continue;
        }
        let (isin, cusip) = (clean_id(&h.isin), clean_id(&h.cusip));
        let same = |p: &Pos| {
            if !isin.is_empty() {
                p.isin == isin
            } else if !cusip.is_empty() {
                p.isin.is_empty() && p.cusip == cusip
            } else {
                p.isin.is_empty() && p.cusip.is_empty() && p.name == h.name
            }
        };
        if let Some(p) = out.iter_mut().find(|p| same(p)) {
            p.val_usd += h.val_usd;
            p.balance += h.balance;
        } else {
            out.push(Pos { name: h.name.clone(), isin: isin.to_string(), cusip: cusip.to_string(), currency: h.currency.clone(), val_usd: h.val_usd, balance: h.balance });
        }
    }
    (out, other)
}

/// For each position of `a`, the position of `b` that is the same holding: by ISIN, else by CUSIP, else by name.
/// A position of `b` links once (the first position of `a` that finds it).
pub fn link(a: &[Pos], b: &[Pos]) -> Vec<Option<(usize, Method)>> {
    let mut used = vec![false; b.len()];
    a.iter()
        .map(|p| {
            let find = |m: Method| {
                let hit = |q: &Pos| match m {
                    Method::Isin => !p.isin.is_empty() && q.isin == p.isin,
                    Method::Cusip => !p.cusip.is_empty() && q.cusip == p.cusip,
                    Method::Name => q.name.eq_ignore_ascii_case(&p.name),
                };
                b.iter().enumerate().position(|(i, q)| !used[i] && hit(q))
            };
            for m in [Method::Isin, Method::Cusip, Method::Name] {
                if let Some(i) = find(m) {
                    used[i] = true;
                    return Some((i, m));
                }
            }
            None
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// The share has a US ticker in the map (its prices can come from the Nasdaq files).
    Us,
    /// No US ticker: a quarterly price only.
    Foreign,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub name: String,
    pub group: Group,
    pub ticker: Option<String>,
    pub method: Method,
    /// The weight at the start: valUSD / net assets.
    pub w0: f64,
    pub p0: f64,
    pub p1: f64,
    /// The USD return over the quarter, from the two prices.
    pub ret: f64,
    pub contrib: f64,
    /// The balance at the end divided by the balance at the start.
    pub bal_ratio: f64,
    /// The share factor (balance for one old share) if a split or a consolidation was found and the return is adjusted.
    pub split: Option<f64>,
    /// An estimate of the part of the remainder that comes from the trades in this holding:
    /// w0 x (relative balance change - g) x return / 2, where g is the weighted mean relative balance change of all linked
    /// holdings (a creation of shares buys all holdings in proportion and changes no return). It assumes the trades are at the
    /// mid-quarter price.
    pub trade_est: f64,
    pub flag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Quarter {
    pub d0: String,
    pub d1: String,
    /// The NAV return of the quarter (the product of the 3 monthly returns of the later report).
    pub nav_ret: Option<f64>,
    /// The three monthly NAV returns in percent, in the order of time (the first month of the quarter first).
    pub month_rets: [Option<f64>; 3],
    pub rows: Vec<Row>,
    pub us: f64,
    pub foreign: f64,
    /// The sum of `trade_est` of the rows (an estimate, not a fact).
    pub trade_est: f64,
    /// g: the weighted mean relative balance change of the linked holdings (the pro-rata part: creations and redemptions).
    pub balance_growth: f64,
    /// The NAV return minus the sum of the parts (None if the NAV return is missing).
    pub remainder: Option<f64>,
    /// Positions of the later report with no link to the earlier one: name, weight at the end, US ticker if in the map.
    pub entering: Vec<(String, f64, Option<String>)>,
    /// Positions of the earlier report with no link to the later one: name and weight at the start.
    pub leaving: Vec<(String, f64)>,
    /// The weight at the start of the lines that have no price (repo, money market, collateral, other).
    pub other_w0: f64,
    pub linked_by: [usize; 3],
}

/// The product of the monthly returns (in percent) as one return (a fraction). `None` if one month is missing.
pub fn compound(rtn: &[Option<f64>; 3]) -> Option<f64> {
    let mut f = 1.0;
    for r in rtn {
        f *= 1.0 + (*r)? / 100.0;
    }
    Some(f - 1.0)
}

/// The monthly returns of the class (the first class if the id is not found).
pub fn class_returns<'a>(r: &'a Report, class_id: &str) -> Option<&'a MonthlyReturn> {
    r.monthly.iter().find(|m| m.class_id == class_id).or(r.monthly.first())
}

/// The change of the premium or discount: the price return minus the NAV return (the difference), and the ratio form
/// (1 + price return) / (1 + NAV return) - 1.
pub fn premium_change(price_ret: f64, nav_ret: f64) -> (f64, f64) {
    (price_ret - nav_ret, (1.0 + price_ret) / (1.0 + nav_ret) - 1.0)
}

/// A split or consolidation: the factor `f` (balance at the end for one share at the start) if the price moved far from 1
/// (`q` outside 0.6 to 1.6) and a factor of `SPLIT_FACTORS` (or its inverse) explains it: the balance is near `f`
/// (log difference below `SPLIT_BALANCE_LOG_TOLERANCE`) and `q * f` is in `SPLIT_VALUE_RANGE`. Of several factors the one with
/// the smallest sum of |ln(balance / f)| and |ln(q * f)| is taken. The return of the holding is then `q * f - 1`, not
/// `q - 1`. The factor is inferred, not read from a record of corporate actions; a fund that doubles a position while the
/// price halves looks the same. The flag says "adjusted".
pub fn split_factor(bal_ratio: f64, q: f64) -> Option<f64> {
    if (0.6..=1.6).contains(&q) || !(bal_ratio > 0.0 && q > 0.0) {
        return None;
    }
    SPLIT_FACTORS
        .iter()
        .flat_map(|k| [*k, 1.0 / *k])
        .filter(|f| (bal_ratio / f).ln().abs() < SPLIT_BALANCE_LOG_TOLERANCE && SPLIT_VALUE_RANGE.contains(&(q * f)))
        .min_by(|x, y| ((bal_ratio / x).ln().abs() + (q * x).ln().abs()).total_cmp(&((bal_ratio / y).ln().abs() + (q * y).ln().abs())))
}

fn flag_of(a: &Pos, b: &Pos, split: Option<f64>) -> Option<String> {
    let q = b.price() / a.price();
    let bal = b.balance / a.balance;
    let mut f = Vec::new();
    if !a.currency.is_empty() && !b.currency.is_empty() && a.currency != b.currency {
        f.push(format!("currency {} to {}", a.currency, b.currency));
    }
    if let Some(k) = split {
        let what = if k > 1.0 { format!("{k:.0} new shares for 1") } else { format!("1 new share for {:.0}", 1.0 / k) };
        f.push(format!("split or consolidation ({what}): balance x{bal:.2}, USD price x{q:.2}; the return is adjusted to {:+.1}%", (q * k - 1.0) * 100.0));
    } else if !(JUMP_LOW..=JUMP_HIGH).contains(&q) {
        f.push(format!("USD price x{q:.2} with balance x{bal:.2}: a real move, a split not in the list, or a data error: the return is NOT adjusted"));
    }
    (!f.is_empty()).then(|| f.join("; "))
}

/// Layer 2 for one quarter: `r0` is the report at the start, `r1` the report at the end.
pub fn attribute(r0: &Report, r1: &Report, class_id: &str, map: &HashMap<String, String>) -> Result<Quarter, String> {
    let net0 = r0.net_assets.filter(|n| *n > 0.0).ok_or_else(|| format!("{}: no net assets", r0.report_date))?;
    let (a, other0) = positions(r0);
    let (b, _) = positions(r1);
    let links = link(&a, &b);
    let mut q = Quarter { d0: r0.report_date.clone(), d1: r1.report_date.clone(), other_w0: other0 / net0, ..Quarter::default() };
    // N-PORT: rtn1 is the NEWEST month (the month of the report date) and rtn3 the oldest (checked against the Nasdaq closes of URA,
    // 11 of 13 reports clear, 2 undecided). `month_rets` is in the order of time.
    q.month_rets = class_returns(r1, class_id).map_or([None; 3], |m| [m.rtn[2], m.rtn[1], m.rtn[0]]);
    q.nav_ret = compound(&q.month_rets);
    let mut hit = vec![false; b.len()];
    for (p, l) in a.iter().zip(&links) {
        let w0 = p.val_usd / net0;
        let Some((i, method)) = *l else {
            q.leaving.push((p.name.clone(), w0));
            continue;
        };
        hit[i] = true;
        q.linked_by[match method {
            Method::Isin => 0,
            Method::Cusip => 1,
            Method::Name => 2,
        }] += 1;
        let ticker = map.get(&p.isin).or_else(|| map.get(&p.cusip)).cloned();
        let group = if ticker.is_some() { Group::Us } else { Group::Foreign };
        let qf = b[i].price() / p.price();
        let bal_ratio = b[i].balance / p.balance;
        let split = split_factor(bal_ratio, qf);
        let ret = qf * split.unwrap_or(1.0) - 1.0;
        let contrib = w0 * ret;
        let trade_est = 0.0;
        match group {
            Group::Us => q.us += contrib,
            Group::Foreign => q.foreign += contrib,
        }
        q.rows.push(Row { name: p.name.clone(), group, ticker, method, w0, p0: p.price(), p1: b[i].price(), ret, contrib, bal_ratio, split, trade_est, flag: flag_of(p, &b[i], split) });
    }
    // The pro-rata part of the balance change (creations and redemptions) is not a trade of the fund manager.
    let w_sum: f64 = q.rows.iter().map(|r| r.w0).fold(0.0, |a, b| a + b);
    if w_sum > 0.0 {
        q.balance_growth = q.rows.iter().map(|r| r.w0 * (r.bal_ratio / r.split.unwrap_or(1.0) - 1.0)).fold(0.0, |a, b| a + b) / w_sum;
    }
    let g = q.balance_growth;
    for r in &mut q.rows {
        r.trade_est = r.w0 * (r.bal_ratio / r.split.unwrap_or(1.0) - 1.0 - g) * r.ret / 2.0;
        q.trade_est += r.trade_est;
    }
    let net1 = r1.net_assets.filter(|n| *n > 0.0);
    for (i, p) in b.iter().enumerate() {
        if !hit[i] {
            q.entering.push((p.name.clone(), net1.map_or(f64::NAN, |n| p.val_usd / n), map.get(&p.isin).or_else(|| map.get(&p.cusip)).cloned()));
        }
    }
    q.remainder = q.nav_ret.map(|n| n - q.us - q.foreign);
    Ok(q)
}

/// The last close on or before `date`, at most `MAX_STALE_DAYS` calendar days before it.
pub fn close_on(rows: &[PricePoint], date: &str) -> Option<f64> {
    let i = rows.partition_point(|r| r.date.as_str() <= date);
    let r = rows.get(i.checked_sub(1)?)?;
    let gap = super::parse_date(date)? - super::parse_date(&r.date)?;
    (gap <= MAX_STALE_DAYS && r.close.is_finite() && r.close > 0.0).then_some(r.close)
}

/// The end of the month before the month of `date` (`YYYY-MM-DD`), as a date text.
pub fn prev_month_end(date: &str) -> Option<String> {
    let d = super::parse_date(date)?;
    let first = d - (date[8..10].parse::<i64>().ok()? - 1);
    Some(mapcheck::dates::format(first - 1))
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MonthRow {
    pub date: String,
    pub fund_month: Option<f64>,
    pub fund_cum: Option<f64>,
    /// The US-listed part since the start of the quarter: the sum of w0 x (P_month / P_start - 1), Nasdaq closes.
    pub us_cum: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Monthly {
    pub rows: Vec<MonthRow>,
    /// US-listed rows with every Nasdaq close available (name list not kept: the count and the weight).
    pub priced: usize,
    /// US-listed rows with a missing Nasdaq close at one of the four dates: name and weight at the start.
    pub missing: Vec<(String, f64)>,
    /// The sum of the parts of the priced rows from the N-PORT prices (the same rows, the same weights).
    pub nport_us_priced: f64,
}

/// The monthly view of a quarter. `price(ticker, date)` is a Nasdaq close.
pub fn monthly(q: &Quarter, price: &dyn Fn(&str, &str) -> Option<f64>) -> Monthly {
    let m2 = prev_month_end(&q.d1).unwrap_or_default();
    let m1 = prev_month_end(&m2).unwrap_or_default();
    let dates = [m1, m2, q.d1.clone()];
    let mut m = Monthly::default();
    let mut cum = [0.0f64; 3];
    for r in q.rows.iter().filter(|r| r.group == Group::Us) {
        let t = r.ticker.as_deref().unwrap_or("");
        let p0 = price(t, &q.d0);
        let ps: Vec<Option<f64>> = dates.iter().map(|d| price(t, d)).collect();
        let (Some(p0), true) = (p0, ps.iter().all(Option::is_some)) else {
            m.missing.push((r.name.clone(), r.w0));
            continue;
        };
        m.priced += 1;
        m.nport_us_priced += r.contrib;
        for (c, p) in cum.iter_mut().zip(&ps) {
            *c += r.w0 * (p.unwrap_or(p0) / p0 - 1.0);
        }
    }
    let mut fund = 1.0f64;
    let mut fund_ok = true;
    for i in 0..3 {
        let rn = q.month_rets[i].map(|v| v / 100.0);
        match rn {
            Some(v) if fund_ok => fund *= 1.0 + v,
            _ => fund_ok = false,
        }
        m.rows.push(MonthRow { date: dates[i].clone(), fund_month: rn, fund_cum: fund_ok.then_some(fund - 1.0), us_cum: cum[i] });
    }
    m
}

fn pp(v: f64) -> String {
    format!("{:+.2}", v * 100.0)
}

fn pp_opt(v: Option<f64>) -> String {
    v.map_or("n/a".to_string(), pp)
}

/// The six limits of the method, printed at the end.
pub const LIMITS: [&str; 6] = [
    "1. The foreign part of the fund (about 35% to 58% of the net assets for URA, 2023 to 2026) has a quarterly value only: no permitted monthly price.",
    "2. A trade inside a quarter goes to the remainder, with cash, fee and securities lending (the start weights are held for the quarter).",
    "3. The premium is price return minus NAV return. N-PORT has no share count, so a premium level (price / NAV - 1) is not known.",
    "4. GLD months are approximate (World Bank monthly average). This limit applies to a GLD run only.",
    "5. The look-through of the Sprott trust (uranium in the trust) needs the consent of Sprott: the trust is one line here.",
    "6. The last quarter can be missing: N-PORT is public 60 days after the quarter end, for the third month only.",
];

/// Further cautions, printed after the six limits.
pub const CAUTIONS: [&str; 5] = [
    "- Each quarter uses the report filed about 60 days after the quarter end: this is a look back, not a point-in-time view.",
    "- The ticker map is of 2026; it is used for all quarters. Each row has a price check (MAP SUSPECT above the tolerance).",
    "- The return of a foreign holding is in USD: it includes the change of the currency rate.",
    "- The split rule infers a factor from the balance. It can misfire when a large trade comes with a value factor of 0.4 to 2.5.",
    "- An amendment (NPORT-P/A) is used only when the original is missing; it is not compared with the original.",
];

struct SummaryRow {
    date: String,
    nav: Option<f64>,
    us: f64,
    foreign: f64,
    remainder: Option<f64>,
}

pub struct Options {
    pub ticker: String,
    pub cache: Option<PathBuf>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub top: usize,
}

pub fn parse_args(args: &[String]) -> Result<Option<Options>, String> {
    let t = args.first().ok_or("fa fund attribution TICKER")?;
    if matches!(t.as_str(), "-h" | "--help" | "help") {
        return Ok(None);
    }
    let ticker = t.to_uppercase();
    if !super::fund_holdings::valid_ticker(&ticker) {
        return Err(format!("{t}: not a ticker (letters, digits, . and - only)"));
    }
    let mut o = Options { ticker, cache: None, from: None, to: None, top: 6 };
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        let date = |v: Option<&String>, n: &str| -> Result<String, String> {
            let v = v.ok_or_else(|| format!("{n} needs a date YYYY-MM-DD"))?;
            super::parse_date(v).ok_or_else(|| format!("{n}: {v} is not a date YYYY-MM-DD"))?;
            Ok(v.clone())
        };
        match a.as_str() {
            "-h" | "--help" => return Ok(None),
            "--cache" => o.cache = Some(PathBuf::from(it.next().ok_or("--cache needs a folder")?)),
            "--from" => o.from = Some(date(it.next(), "--from")?),
            "--to" => o.to = Some(date(it.next(), "--to")?),
            "--top" => o.top = it.next().and_then(|v| v.parse().ok()).ok_or("--top needs a number")?,
            x => return Err(format!("unknown option {x}")),
        }
    }
    Ok(Some(o))
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    let Some(o) = parse_args(args)? else {
        println!("{USAGE}");
        return Ok(0);
    };
    let cache = o.cache.clone().unwrap_or_else(|| repo.join("data").join("raw"));
    let map = parse_map(MAP_CSV);
    // The share class of the ticker, from the SEC file of fund tickers if it is in the cache.
    let (series_id, class_id) = std::fs::read_to_string(cache.join("_sec").join("company_tickers_mf.json"))
        .ok()
        .and_then(|t| super::fund_holdings::series_of(&t, &o.ticker).ok())
        .map_or((None, String::new()), |s| (Some(s.series_id), s.class_id));
    let loaded = read_cached_reports(&cache, &o.ticker, series_id.as_deref());
    let (reports, notes) = select_reports(loaded);
    let reports: Vec<Loaded> = reports
        .into_iter()
        .filter(|l| o.from.as_deref().is_none_or(|f| l.report.report_date.as_str() >= f) && o.to.as_deref().is_none_or(|t| l.report.report_date.as_str() <= t))
        .collect();
    if reports.is_empty() {
        return Err(format!("{}: no N-PORT report in {}/{}/nport for this range. Run: fa fund holdings {} --from YYYY-MM-DD", o.ticker, cache.display(), o.ticker, o.ticker));
    }
    let fund_prices = price_cache::history(&cache, &o.ticker);
    let meta: Option<price_cache::Meta> = std::fs::read_to_string(price_cache::meta_path(&cache, &o.ticker)).ok().and_then(|t| serde_json::from_str(&t).ok());
    let mut tick_cache: HashMap<String, Option<Vec<PricePoint>>> = HashMap::new();
    for t in map.values() {
        tick_cache.entry(t.clone()).or_insert_with(|| price_cache::history(&cache, t));
    }

    println!("{}  fund attribution from {} N-PORT reports ({} to {}); no request", o.ticker, reports.len(), reports[0].report.report_date, reports[reports.len() - 1].report.report_date);
    for n in &notes {
        println!("note: {n}");
    }
    match (&fund_prices, &meta) {
        (Some(_), Some(m)) => println!(
            "{} price: {} closes, adjusted for {} ({} to {}). {}",
            o.ticker,
            m.source,
            m.adjusted,
            m.first_date,
            m.last_date,
            if m.adjusted.contains("dividends") { "The price return includes the distributions." } else { "The price return has NO distributions: it is not a total return." }
        ),
        (Some(_), None) => println!("{} price: no record file; it is not known if the closes include the distributions.", o.ticker),
        _ => println!("{} price: no price file (fa prices get {} --source nasdaq): the premium lines are n/a.", o.ticker, o.ticker),
    }

    // Layer 1: the net assets.
    println!("\nLAYER 1a - net assets against the holdings (USD million)");
    println!("{:<11} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>7}", "date", "holdings", "equity", "repo etc", "totAssets", "other", "liabs", "implied", "netAssets", "diff");
    for l in &reports {
        let r = &l.report;
        let sum: f64 = r.holdings.iter().map(|h| h.val_usd).sum();
        let (pos, nonprice) = positions(r);
        let equity: f64 = pos.iter().map(|p| p.val_usd).sum();
        let m = |v: f64| format!("{:.1}", v / 1e6);
        let mo = |v: Option<f64>| v.map_or("n/a".to_string(), m);
        // Other assets are not in N-PORT as a number: they are total assets minus the holdings.
        let other = r.tot_assets.map(|t| t - sum);
        let implied = other.zip(r.tot_liabs).map(|(o, li)| sum + o - li);
        let diff = implied.zip(r.net_assets).map(|(i, n)| i - n);
        println!(
            "{:<11} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>7}",
            r.report_date, m(sum), m(equity), m(nonprice), mo(r.tot_assets), mo(other), mo(r.tot_liabs), mo(implied), mo(r.net_assets), mo(diff)
        );
    }
    println!("other = total assets - holdings (N-PORT gives no other-assets number), so diff is 0 by construction: it only shows that the");
    println!("report adds up (total assets - liabilities = net assets). A negative other means: the repo, money market and collateral lines");
    println!("(cash collateral of securities lending, column repo etc) are in the holdings list but not in total assets. Then compare the");
    println!("equity column with net assets: net assets - equity = cash and other net assets (a few percent). The independent check is the");
    println!("sum of pctVal against holdings / net assets:");
    for l in &reports {
        let r = &l.report;
        let sum: f64 = r.holdings.iter().map(|h| h.val_usd).sum();
        if let Some(n) = r.net_assets {
            let from_val = sum / n * 100.0;
            let pct: f64 = r.holdings.iter().map(|h| h.pct_val).sum();
            let flag = if (from_val - pct).abs() > 0.05 { "  <-- differs" } else { "" };
            println!("  {}: sum pctVal {:.3} against holdings / net assets {:.3} (difference {:+.3} points){flag}", r.report_date, pct, from_val, pct - from_val);
        }
    }

    // Quarters.
    let price_of = |t: &str, d: &str| tick_cache.get(t).and_then(|o| o.as_ref()).and_then(|rows| close_on(rows, d));
    let mut summary: Vec<SummaryRow> = Vec::new();
    for w in reports.windows(2) {
        let (a, b) = (&w[0].report, &w[1].report);
        let gap = super::parse_date(&b.report_date).zip(super::parse_date(&a.report_date)).map(|(x, y)| x - y);
        println!("\n=== quarter {} to {} ===", a.report_date, b.report_date);
        if !gap.is_some_and(|g| GAP_DAYS.contains(&g)) {
            println!("no quarter: the two reports are {} days apart (a report is missing between them). No attribution.", gap.map_or("?".to_string(), |g| g.to_string()));
            continue;
        }
        let q = match attribute(a, b, &class_id, &map) {
            Ok(q) => q,
            Err(e) => {
                println!("no attribution: {e}");
                continue;
            }
        };
        // Layer 1b.
        let pr = fund_prices.as_ref().and_then(|rows| close_on(rows, &q.d0).zip(close_on(rows, &q.d1))).map(|(p0, p1)| p1 / p0 - 1.0);
        match (pr, q.nav_ret) {
            (Some(p), Some(n)) => {
                let (d, ratio) = premium_change(p, n);
                println!(
                    "LAYER 1b: price return {} pp, NAV total return {} pp ({} {} {} per month), difference {} pp (ratio form {} pp)",
                    pp(p),
                    pp(n),
                    pp_opt(q.month_rets[0].map(|v| v / 100.0)),
                    pp_opt(q.month_rets[1].map(|v| v / 100.0)),
                    pp_opt(q.month_rets[2].map(|v| v / 100.0)),
                    pp(d),
                    pp(ratio)
                );
                println!("          price vs NAV, distributions not removed: the difference = change of the premium/discount + the distributions of the quarter (not in the Nasdaq close)");
            }
            _ => println!("LAYER 1b: n/a (price return {}, NAV return {})", pp_opt(pr), pp_opt(q.nav_ret)),
        }
        // Layer 2.
        let (nl, nf) = (q.rows.iter().filter(|r| r.group == Group::Us).count(), q.rows.iter().filter(|r| r.group == Group::Foreign).count());
        println!(
            "LAYER 2: {} holdings linked (by ISIN {}, CUSIP {}, name {}); {} entering, {} leaving; US-listed {nl}, foreign {nf}",
            q.rows.len(),
            q.linked_by[0],
            q.linked_by[1],
            q.linked_by[2],
            q.entering.len(),
            q.leaving.len()
        );
        println!("  US-listed part (start weight x USD return)        {} pp", pp(q.us));
        println!("  foreign part (same rule, quarterly prices)        {} pp", pp(q.foreign));
        println!("  sum of the parts                                  {} pp", pp(q.us + q.foreign));
        println!("  fund NAV return                                   {} pp", pp_opt(q.nav_ret));
        println!("  trading in the quarter, cash, fee, dividends, securities lending, other   {} pp   <-- remainder", pp_opt(q.remainder));
        if q.remainder.is_some_and(|r| r.abs() * 100.0 > REMAINDER_WARN_PP) {
            println!("  WARNING: the remainder is above {REMAINDER_WARN_PP} pp");
        }
        let ew: f64 = q.entering.iter().map(|e| e.1).filter(|v| v.is_finite()).fold(0.0, |a, b| a + b);
        // An estimate for the entering US-listed holdings, if the fund held them from the start of the quarter (a bound, not a fact):
        // the weight at the start is w1 / (1 + r) and the part is that weight times r, with r from the Nasdaq closes.
        let mut est = 0.0;
        let mut est_names = Vec::new();
        for (n, w1, t) in &q.entering {
            if let Some((p0, p1)) = t.as_deref().and_then(|t| price_of(t, &q.d0).zip(price_of(t, &q.d1))) {
                let r = p1 / p0 - 1.0;
                est += w1 / (1.0 + r) * r;
                est_names.push(format!("{} {:+.2} pp", n.chars().take(20).collect::<String>(), w1 / (1.0 + r) * r * 100.0));
            }
        }
        let lw: f64 = q.leaving.iter().map(|e| e.1).fold(0.0, |a, b| a + b);
        println!(
            "  rule: a holding that enters or leaves has no start or end price: it is out of the parts and its return is in the remainder. Entering (weight at the end) {:.2}%: {}; leaving (weight at the start) {:.2}%: {}",
            ew * 100.0,
            names(&q.entering.iter().map(|e| (e.0.clone(), e.1)).collect::<Vec<_>>()),
            lw * 100.0,
            names(&q.leaving)
        );
        if !est_names.is_empty() {
            println!("  bound for entering US-listed holdings, if held for the whole quarter (the fund bought them during the quarter; Nasdaq closes): {} pp ({})", pp(est), est_names.join(", "));
        }
        println!(
            "  estimate for the trades in the linked holdings (relative balance change less the pro-rata change {:+.1}%, x half the return; mid-quarter price): {} pp",
            q.balance_growth * 100.0,
            pp(q.trade_est)
        );
        if let Some(r) = q.remainder {
            println!("  unexplained after the two estimates (cash, fee, dividends, securities lending, rest of the trading, error of the estimates): {} pp", pp(r - q.trade_est - est));
            if r.abs() * 100.0 > REMAINDER_WARN_PP {
                println!("  probable cause (not proved): the holdings that entered or left and the trades in the quarter; the estimates are bounds, not measures; unexplained {} pp", pp(r - q.trade_est - est));
            }
        }
        println!("  lines with no price (repo, money market, collateral, other) hold {:.2}% of the net assets at the start; they are in the remainder", q.other_w0 * 100.0);
        let mut rows: Vec<&Row> = q.rows.iter().collect();
        rows.sort_by(|x, y| y.contrib.abs().total_cmp(&x.contrib.abs()));
        println!("  {:<34} {:>7} {:>8} {:>8} {:>8} {:>5} {:<6}", "largest parts", "w0 %", "ret %", "part pp", "p0->p1", "grp", "ticker");
        for r in rows.iter().take(o.top) {
            println!(
                "  {:<34} {:>7.2} {:>8.1} {:>8.2} {:>8} {:>5} {:<6}",
                r.name.chars().take(34).collect::<String>(),
                r.w0 * 100.0,
                r.ret * 100.0,
                r.contrib * 100.0,
                format!("{:.2}->{:.2}", r.p0, r.p1),
                if r.group == Group::Us { "US" } else { "for" },
                r.ticker.as_deref().unwrap_or("")
            );
        }
        for r in q.rows.iter().filter(|r| r.flag.is_some()) {
            let nas = r.ticker.as_deref().and_then(|t| price_of(t, &q.d0).zip(price_of(t, &q.d1))).map(|(a, b)| format!(" [Nasdaq close x{:.2}]", b / a)).unwrap_or_default();
            println!("  FLAG {} ({:.2}% start weight, part {:+.2} pp): {}{nas}", r.name, r.w0 * 100.0, r.contrib * 100.0, r.flag.as_deref().unwrap_or(""));
        }
        // The price check of the mapping: the N-PORT price against the Nasdaq close at the end date.
        let mut chk: Vec<(f64, &str)> = Vec::new();
        for r in q.rows.iter().filter(|r| r.group == Group::Us) {
            if let Some(c) = r.ticker.as_deref().and_then(|t| price_of(t, &q.d1)) {
                chk.push(((r.p1 / c - 1.0).abs(), r.ticker.as_deref().unwrap_or("")));
            }
        }
        chk.sort_by(|x, y| y.0.total_cmp(&x.0));
        for (d, t) in chk.iter().filter(|(d, _)| *d > MAP_PRICE_TOLERANCE) {
            println!("  MAP SUSPECT {t}: N-PORT price against Nasdaq close differs by {:.2}% (tolerance {:.0}%): check the row of the ticker map", d * 100.0, MAP_PRICE_TOLERANCE * 100.0);
        }
        let unchecked: Vec<&str> = q.rows.iter().filter(|r| r.group == Group::Us).filter_map(|r| r.ticker.as_deref()).filter(|t| price_of(t, &q.d1).is_none()).collect();
        if !unchecked.is_empty() {
            println!("  price check NOT RUN for {} (no Nasdaq close at {}): these map rows are not checked", unchecked.join(", "), q.d1);
        }
        match chk.first() {
            Some((d, t)) => println!("  price check at {}: N-PORT price against Nasdaq close for {} US-listed holdings, largest difference {:.2}% ({t}); tolerance {:.0}%", q.d1, chk.len(), d * 100.0, MAP_PRICE_TOLERANCE * 100.0),
            None => println!("  price check NOT RUN at {}: no Nasdaq close for any US-listed holding", q.d1),
        }
        // Monthly view.
        let mv = monthly(&q, &price_of);
        println!("MONTHLY (cumulative since {}, pp):", q.d0);
        println!("  {:<11} {:>10} {:>10} {:>14} {:>14}", "month end", "fund month", "fund cum", "US-listed cum", "rest of fund");
        for m in &mv.rows {
            let rest = m.fund_cum.map(|c| c - m.us_cum);
            println!("  {:<11} {:>10} {:>10} {:>14} {:>14}", m.date, pp_opt(m.fund_month), pp_opt(m.fund_cum), pp(m.us_cum), pp_opt(rest));
        }
        println!("  foreign holdings, quarterly only (no permitted monthly price): {} pp at {} (months 1 and 2: not available)", pp(q.foreign), q.d1);
        println!("  rest of fund at {} = foreign {} pp + remainder and unpriced {} pp", q.d1, pp(q.foreign), pp_opt(mv.rows.last().and_then(|m| m.fund_cum.map(|c| c - m.us_cum - q.foreign))));
        let n_us = q.rows.iter().filter(|r| r.group == Group::Us).count();
        println!(
            "  US-listed with all Nasdaq closes: {} of {n_us}; Nasdaq part {} pp against N-PORT part {} pp for the same holdings (difference {} pp)",
            mv.priced,
            pp(mv.rows.last().map_or(0.0, |m| m.us_cum)),
            pp(mv.nport_us_priced),
            pp(mv.rows.last().map_or(0.0, |m| m.us_cum) - mv.nport_us_priced)
        );
        if !mv.missing.is_empty() {
            println!("  no Nasdaq price file or close (weight {:.2}%): {}", mv.missing.iter().map(|m| m.1).sum::<f64>() * 100.0, mv.missing.iter().map(|m| m.0.as_str()).collect::<Vec<_>>().join(", "));
        }
        summary.push(SummaryRow { date: q.d1.clone(), nav: q.nav_ret, us: q.us, foreign: q.foreign, remainder: q.remainder });
    }

    println!("\nSUMMARY (pp of the fund NAV return for each quarter)");
    println!("{:<11} {:>9} {:>9} {:>9} {:>10}", "quarter end", "NAV ret", "US-listed", "foreign", "remainder");
    for r in &summary {
        println!("{:<11} {:>9} {:>9} {:>9} {:>10}", r.date, pp_opt(r.nav), pp(r.us), pp(r.foreign), pp_opt(r.remainder));
    }
    println!("\nTHE SIX LIMITS");
    for l in LIMITS {
        println!("{l}");
    }
    println!("CAUTIONS");
    for c in CAUTIONS {
        println!("{c}");
    }
    Ok(0)
}

fn names(v: &[(String, f64)]) -> String {
    if v.is_empty() {
        return "none".to_string();
    }
    let mut s: Vec<String> = v.iter().take(4).map(|(n, w)| format!("{} {:.2}%", n.chars().take(24).collect::<String>(), w * 100.0)).collect();
    if v.len() > 4 {
        s.push(format!("and {} more", v.len() - 4));
    }
    s.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::fund_holdings::Holding;

    fn hold(name: &str, isin: &str, val: f64, bal: f64) -> Holding {
        Holding {
            name: name.into(),
            cusip: "N/A".into(),
            isin: isin.into(),
            val_usd: val,
            pct_val: 0.0,
            balance: bal,
            units: "NS".into(),
            currency: "USD".into(),
            exchange_rt: None,
            ticker: String::new(),
            asset_cat: "EC".into(),
            country: "US".into(),
            payoff: "Long".into(),
        }
    }
    fn repo(val: f64) -> Holding {
        Holding { name: "REPO".into(), units: "PA".into(), asset_cat: "RA".into(), balance: val, ..hold("REPO", "", val, val) }
    }
    fn report(date: &str, net: f64, rtn: [f64; 3], holdings: Vec<Holding>) -> Report {
        Report {
            series_id: "S1".into(),
            report_date: date.into(),
            form: "NPORT-P".into(),
            rep_pd_end: String::new(),
            tot_assets: Some(net),
            tot_liabs: Some(0.0),
            net_assets: Some(net),
            monthly: vec![MonthlyReturn { class_id: "C1".into(), rtn: rtn.map(Some) }],
            flows: [None, None, None],
            holdings,
            bad_numbers: 0,
        }
    }
    fn map() -> HashMap<String, String> {
        parse_map("# c\nkey,ticker,name\nAAA,TA,A Corp\n")
    }
    fn two_quarters() -> (Report, Report) {
        // Start: A 400 (price 4), B 300 (price 6), repo 300; net assets 1000.
        let r0 = report("2026-04-30", 1000.0, [0.0; 3], vec![hold("A Corp", "AAA", 400.0, 100.0), hold("B Corp", "BBB", 300.0, 50.0), repo(300.0)]);
        // End: A +10% (4.4), B -10% (5.4), repo 300; the fund NAV return is 1.5% (rtn1 only: the newest month).
        let r1 = report("2026-07-31", 1015.0, [1.5, 0.0, 0.0], vec![hold("A Corp", "AAA", 440.0, 100.0), hold("B Corp", "BBB", 270.0, 50.0), repo(300.0)]);
        (r0, r1)
    }

    #[test]
    fn parts_add_to_the_fund_return_plus_a_known_remainder() {
        let (r0, r1) = two_quarters();
        let q = attribute(&r0, &r1, "C1", &map()).unwrap();
        // A: 0.4 x 0.10 = +0.04 (US-listed, in the map). B: 0.3 x -0.10 = -0.03 (foreign). Sum +0.01. NAV return +0.015.
        assert!((q.us - 0.04).abs() < 1e-12, "{}", q.us);
        assert!((q.foreign + 0.03).abs() < 1e-12, "{}", q.foreign);
        assert!((q.nav_ret.unwrap() - 0.015).abs() < 1e-12);
        assert!((q.remainder.unwrap() - 0.005).abs() < 1e-12, "{:?}", q.remainder);
        assert!((q.us + q.foreign + q.remainder.unwrap() - q.nav_ret.unwrap()).abs() < 1e-15);
        assert_eq!(q.linked_by, [2, 0, 0]);
        assert!((q.other_w0 - 0.3).abs() < 1e-12);
        assert!(q.rows.iter().all(|r| r.flag.is_none()));
    }

    #[test]
    fn rtn1_is_the_newest_month() {
        let (r0, mut r1) = two_quarters();
        r1.monthly[0].rtn = [Some(3.0), Some(2.0), Some(1.0)];
        let q = attribute(&r0, &r1, "C1", &map()).unwrap();
        // In time order: the oldest month (rtn3 = 1.0) first.
        assert_eq!(q.month_rets, [Some(1.0), Some(2.0), Some(3.0)]);
        let m = monthly(&q, &|_, _| None);
        assert_eq!(m.rows.iter().map(|r| r.fund_month).collect::<Vec<_>>(), [Some(0.01), Some(0.02), Some(0.03)]);
        assert!((m.rows[0].fund_cum.unwrap() - 0.01).abs() < 1e-15);
        assert!((m.rows[2].fund_cum.unwrap() - (1.01 * 1.02 * 1.03 - 1.0)).abs() < 1e-12);
    }

    #[test]
    fn nav_return_is_the_product_of_the_three_months() {
        let r = compound(&[Some(10.0), Some(-10.0), Some(0.0)]).unwrap();
        assert!((r - (1.1 * 0.9 - 1.0)).abs() < 1e-15);
        assert_eq!(compound(&[Some(1.0), None, Some(1.0)]), None);
    }

    #[test]
    fn premium_arithmetic() {
        // The price rose 3.00% and the NAV 1.50%: the premium rose by 1.50 points; ratio form 1.03 / 1.015 - 1.
        let (d, ratio) = premium_change(0.03, 0.015);
        assert!((d - 0.015).abs() < 1e-15);
        assert!((ratio - (1.03 / 1.015 - 1.0)).abs() < 1e-15);
        // Layer 1: the same price and NAV return give no change.
        assert_eq!(premium_change(0.02, 0.02).0, 0.0);
    }

    #[test]
    fn a_holding_that_enters_is_out_of_the_parts() {
        let (r0, mut r1) = two_quarters();
        r1.holdings.push(hold("C Corp", "CCC", 100.0, 10.0));
        r1.net_assets = Some(1115.0);
        let q = attribute(&r0, &r1, "C1", &map()).unwrap();
        assert_eq!(q.entering.len(), 1);
        assert_eq!(q.entering[0].0, "C Corp");
        assert_eq!(q.entering[0].2, None);
        assert!((q.entering[0].1 - 100.0 / 1115.0).abs() < 1e-12);
        assert!(q.leaving.is_empty());
        // The parts do not change; the remainder is the same (the new line is in the remainder by rule).
        assert!((q.us + q.foreign - 0.01).abs() < 1e-12);
        assert!((q.remainder.unwrap() - 0.005).abs() < 1e-12);
        assert_eq!(q.rows.len(), 2);
    }

    #[test]
    fn a_holding_that_leaves_is_named_with_its_start_weight() {
        let (r0, mut r1) = two_quarters();
        r1.holdings.retain(|h| h.name != "B Corp");
        let q = attribute(&r0, &r1, "C1", &map()).unwrap();
        assert_eq!(q.leaving, vec![("B Corp".to_string(), 0.3)]);
        assert!((q.us - 0.04).abs() < 1e-12 && q.foreign == 0.0);
        assert!((q.remainder.unwrap() + 0.025).abs() < 1e-12, "{:?}", q.remainder);
    }

    #[test]
    fn linking_goes_by_isin_then_cusip_then_name() {
        let p = |name: &str, isin: &str, cusip: &str| Pos { name: name.into(), isin: isin.into(), cusip: cusip.into(), currency: "USD".into(), val_usd: 1.0, balance: 1.0 };
        let a = vec![p("One", "I1", "C1"), p("Two", "", "C2"), p("Three", "", ""), p("Four", "I4", "")];
        // `One` has a new name; `Two` has no ISIN, the CUSIP is the same; `Three` has the same name only; `Four` is gone.
        let b = vec![p("Uno", "I1", "C1x"), p("Zwei", "", "C2"), p("three", "", ""), p("Five", "I5", "")];
        let l = link(&a, &b);
        assert_eq!(l, vec![Some((0, Method::Isin)), Some((1, Method::Cusip)), Some((2, Method::Name)), None]);
    }

    #[test]
    fn lines_with_the_same_isin_are_added() {
        let r = report("2026-04-30", 1000.0, [0.0; 3], vec![hold("A Corp", "AAA", 100.0, 10.0), hold("A Corp", "AAA", 200.0, 20.0), repo(50.0)]);
        let (p, other) = positions(&r);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].val_usd, p[0].balance, other), (300.0, 30.0, 50.0));
    }

    #[test]
    fn a_split_and_a_currency_change_get_a_flag() {
        let (r0, mut r1) = two_quarters();
        // A 2 for 1 split: the balance doubles, the price halves, the value is the same.
        r1.holdings[0] = hold("A Corp", "AAA", 400.0, 200.0);
        r1.holdings[1].currency = "CAD".into();
        let q = attribute(&r0, &r1, "C1", &map()).unwrap();
        let a = q.rows.iter().find(|r| r.name == "A Corp").unwrap();
        assert!(a.flag.as_deref().unwrap().contains("split or consolidation (2 new shares for 1)"), "{:?}", a.flag);
        // The value did not change: the adjusted return is 0, not -50%.
        assert!(a.ret.abs() < 1e-12 && a.split == Some(2.0), "{} {:?}", a.ret, a.split);
        let b = q.rows.iter().find(|r| r.name == "B Corp").unwrap();
        assert!(b.flag.as_deref().unwrap().contains("currency USD to CAD"));
        // A price jump alone (x4) is flagged too.
        let (r0, mut r1) = two_quarters();
        r1.holdings[1] = hold("B Corp", "BBB", 1200.0, 50.0);
        let q = attribute(&r0, &r1, "C1", &map()).unwrap();
        assert!(q.rows[1].flag.as_deref().unwrap().contains("USD price x4.00"));
    }

    #[test]
    fn the_trade_estimate_leaves_out_the_pro_rata_change() {
        let (r0, mut r1) = two_quarters();
        // All balances grow 20% (a creation of shares): no trade, estimate 0, the pro-rata growth is 20%.
        for h in r1.holdings.iter_mut().filter(|h| h.asset_cat == "EC") {
            h.balance *= 1.2;
        }
        let q = attribute(&r0, &r1, "C1", &map()).unwrap();
        assert!((q.balance_growth - 0.2).abs() < 1e-12 && q.trade_est.abs() < 1e-12, "{} {}", q.balance_growth, q.trade_est);
        // The fund buys 20% more of A only (A: 0.4 weight, B: 0.3): g = (0.4 x 0.2) / 0.7. A: 0.4 x (0.2 - g) x 0.10 / 2.
        let (r0, mut r1) = two_quarters();
        r1.holdings[0] = hold("A Corp", "AAA", 528.0, 120.0);
        let q = attribute(&r0, &r1, "C1", &map()).unwrap();
        let g = 0.08 / 0.7;
        let want = 0.4 * (0.2 - g) * 0.10 / 2.0 + 0.3 * (0.0 - g) * -0.10 / 2.0;
        assert!((q.trade_est - want).abs() < 1e-12, "{} {}", q.trade_est, want);
        // The part itself is unchanged: it uses the start weight and the two prices only.
        assert!((q.us - 0.04).abs() < 1e-12);
    }

    #[test]
    fn a_consolidation_is_adjusted_and_a_real_move_is_not() {
        // 1 new share for 10 old: the balance x0.1, the price x10.6, the value x1.06: the return is +6%, not +960%.
        let k = split_factor(0.1, 10.6).unwrap();
        assert!((k - 0.1).abs() < 1e-12 && (10.6 * k - 1.0 - 0.06).abs() < 1e-12);
        // The balance changed a little with the split (the fund traded): within the tolerance, the factor stays 1 for 10.
        assert_eq!(split_factor(0.085, 10.6), Some(0.1));
        // Two factors fit (the fund sold half): the sum of the two log differences decides.
        assert_eq!(split_factor(0.05, 11.11), Some(0.05));
        // A price x3 with the same balance is a real move: no adjustment. A balance x2 with the price x1.0 is a trade.
        assert_eq!(split_factor(1.0, 3.0), None);
        assert_eq!(split_factor(2.0, 1.0), None);
        // The balance fell by 10 but the price did not rise: a sale, no split.
        assert_eq!(split_factor(0.1, 1.2), None);
        // No factor of the list fits a balance x1.0 with a price x0.3.
        assert_eq!(split_factor(1.0, 0.3), None);
    }

    #[test]
    fn the_monthly_view_uses_nasdaq_closes_and_names_a_missing_price() {
        let (r0, r1) = two_quarters();
        let q = attribute(&r0, &r1, "C1", &map()).unwrap();
        // Dates: month ends 2026-05-31, 2026-06-30, 2026-07-31. Start 2026-04-30.
        let prices: HashMap<(&str, &str), f64> = [(("TA", "2026-04-30"), 4.0), (("TA", "2026-05-31"), 4.2), (("TA", "2026-06-30"), 3.8), (("TA", "2026-07-31"), 4.4)].into();
        let price = |t: &str, d: &str| prices.get(&(t, d)).copied();
        let m = monthly(&q, &price);
        assert_eq!(m.rows.iter().map(|r| r.date.as_str()).collect::<Vec<_>>(), ["2026-05-31", "2026-06-30", "2026-07-31"]);
        // w0 = 0.4: +5%, -5%, +10% since the start.
        assert!((m.rows[0].us_cum - 0.02).abs() < 1e-12 && (m.rows[1].us_cum + 0.02).abs() < 1e-12 && (m.rows[2].us_cum - 0.04).abs() < 1e-12);
        assert!((m.rows[2].fund_cum.unwrap() - 0.015).abs() < 1e-12);
        assert_eq!((m.priced, m.missing.len()), (1, 0));
        assert!((m.nport_us_priced - 0.04).abs() < 1e-12);
        // One close missing: the holding is named, and its weight is not in the US-listed part.
        let price_missing = |t: &str, d: &str| if d == "2026-06-30" { None } else { price(t, d) };
        let m = monthly(&q, &price_missing);
        assert_eq!(m.priced, 0);
        assert_eq!(m.missing, vec![("A Corp".to_string(), 0.4)]);
        assert_eq!(m.rows[2].us_cum, 0.0);
    }

    #[test]
    fn a_close_is_the_last_one_up_to_five_days_back() {
        let row = |d: &str, c: f64| PricePoint { date: d.into(), open: c, high: c, low: c, close: c, volume: 1 };
        let rows = vec![row("2026-04-28", 10.0), row("2026-04-29", 11.0)];
        assert_eq!(close_on(&rows, "2026-04-30"), Some(11.0));
        assert_eq!(close_on(&rows, "2026-05-04"), Some(11.0));
        assert_eq!(close_on(&rows, "2026-05-05"), None);
        assert_eq!(close_on(&rows, "2026-04-27"), None);
        assert_eq!(prev_month_end("2026-07-31").as_deref(), Some("2026-06-30"));
        assert_eq!(prev_month_end("2026-03-31").as_deref(), Some("2026-02-28"));
    }

    #[test]
    fn the_map_file_parses_and_has_unique_keys() {
        let m = parse_map(MAP_CSV);
        assert_eq!(m.get("CA13321L1085").map(String::as_str), Some("CCJ"));
        let n = MAP_CSV.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#') && !l.starts_with("key,")).count();
        assert_eq!(m.len(), n, "a key is twice in the file");
    }

    #[test]
    fn arguments() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(parse_args(&v(&["--help"])).unwrap().is_none());
        let o = parse_args(&v(&["ura", "--from", "2023-09-30", "--top", "3"])).unwrap().unwrap();
        assert_eq!((o.ticker.as_str(), o.from.as_deref(), o.top), ("URA", Some("2023-09-30"), 3));
        assert!(parse_args(&v(&["URA", "--from", "2023-13-01"])).is_err());
        assert!(parse_args(&v(&["../x"])).is_err());
        assert!(parse_args(&v(&["URA", "--bogus"])).is_err());
    }
}
