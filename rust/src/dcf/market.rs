//! `fa dcf market`: the equity risk premium that the S&P composite price implies, month by month
//! (ticket `T-20261121-dcf-engine-backtest`, plan item 3). Model document: `docs/models/market-level-dcf.md`.
//!
//! A measurement. The price is the input of the search; no model input comes from the price. All values are real, in
//! money of month t-1 (the last CPI that was known at month t). The cash flow is the dividend only.

use super::auto_cli::Opts;
use super::stats::{percentile, sorted_copy};
use super::DcfError;
use std::collections::HashMap;
use std::path::Path;

type R<T> = Result<T, DcfError>;

fn e<T>(m: impl Into<String>) -> R<T> {
    Err(DcfError(m.into()))
}

pub const MARKET_USAGE: &str = "\
fa dcf market --shiller FILE --cpi FILE --out FILE [--long-real G] [--fade-years N]
  The equity risk premium that the S&P composite price implies under a dividend DCF with a linear fade, for each month.
  A measurement: the price is the input of the search, no model input comes from the price. Real terms (CPI).
  --shiller FILE   shiller_monthly.csv of the data store (macro/shiller)
  --cpi FILE       CPIAUCNS.csv of the data store (macro/fred); needed for the rows from 1913 (the command fails without it)
  --out FILE       CSV: month, price_real, d_known_real, g0, rr, erp_implied, erp_gordon, estimate, status
  --long-real G    real growth after the fade (default 0.02, record A-001; the cap at the 10-year yield is not used)
  --fade-years N   years of the linear fade from g0 to G (default 10, at least 2)
  --score FILE     also score the premium against the later real excess return (design 2026-11-21-long-history-design.md,
                   sections 3, 4, 6, 9); needs --tb3ms, --nber, --gs10 FILE (macro/fred); --months-out FILE: CSV by month
  Then a summary by decade on stdout (a description, not a score). Model document: docs/models/market-level-dcf.md";

pub const ERP_LO: f64 = -0.10;
pub const ERP_HI: f64 = 0.30;
pub const G0_MIN: f64 = -0.10;
pub const G0_MAX: f64 = 0.20;
/// From this year on the April rule holds; before it the known dividend is December of Y-2.
pub const LATE_PUBLICATION_END: i32 = 1936;

/// One month of the Shiller file.
#[derive(Debug, Clone)]
pub struct Obs {
    pub year: i32,
    pub month: u32,
    pub price: f64,
    pub dividend: Option<f64>,
    /// Earnings, annual rate (for P2 only).
    pub earnings: Option<f64>,
    pub cpi: f64,
    pub gs10: f64,
    pub estimate: bool,
}

/// One output row.
#[derive(Debug, Clone)]
pub struct MarketRow {
    pub year: i32,
    pub month: u32,
    pub price_real: f64,
    pub d_known_real: f64,
    pub g0: f64,
    pub rr: f64,
    pub erp_implied: Option<f64>,
    pub erp_gordon: f64,
    pub estimate: bool,
    pub status: &'static str,
}

fn opt_f(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("na") {
        None
    } else {
        s.parse().ok()
    }
}

/// The reader of `shiller_monthly.csv`. Columns: month, price, dividend, earnings, cpi, gs10, cape, estimate, note.
pub fn read_shiller(text: &str) -> R<Vec<Obs>> {
    let mut out: Vec<Obs> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.splitn(9, ',').collect();
        if f.len() < 8 {
            return e(format!("line {}: fewer than 8 columns", i + 1));
        }
        let (ys, ms) = f[0].split_once('-').ok_or_else(|| DcfError(format!("line {}: bad month '{}'", i + 1, f[0])))?;
        let year: i32 = ys.parse().map_err(|_| DcfError(format!("line {}: bad year", i + 1)))?;
        let month: u32 = ms.parse().map_err(|_| DcfError(format!("line {}: bad month", i + 1)))?;
        let (Some(price), Some(cpi), Some(gs10)) = (opt_f(f[1]), opt_f(f[4]), opt_f(f[5])) else {
            return e(format!("line {}: price, cpi or gs10 is missing", i + 1));
        };
        if let Some(p) = out.last() {
            if (p.year, p.month) >= (year, month) || (p.year * 12 + p.month as i32 + 1) != year * 12 + month as i32 {
                return e(format!("line {}: months are not consecutive", i + 1));
            }
        }
        out.push(Obs { year, month, price, dividend: opt_f(f[2]), earnings: opt_f(f[3]), cpi, gs10, estimate: f[7].trim() == "true" });
    }
    if out.is_empty() {
        return e("no data rows");
    }
    Ok(out)
}

/// The value of the dividend stream at the real rate `r`: years 1..N with the linear fade, and the terminal at year N.
/// `d0` is the real dividend of the last known year. `g_j = g0 + (l - g0) (j - 1) / (n - 1)` for j = 1..N.
pub fn value(d0: f64, g0: f64, l: f64, n: usize, r: f64) -> f64 {
    let mut d = d0;
    let mut v = 0.0;
    let mut disc = 1.0;
    for j in 1..=n {
        let g = g0 + (l - g0) * (j as f64 - 1.0) / (n as f64 - 1.0);
        d *= 1.0 + g;
        disc *= 1.0 + r;
        v += d / disc;
    }
    v + d * (1.0 + l) / (r - l) / disc
}

/// The premium for which `value` equals `price`, by bisection. `Err(code)` when there is no root in the range.
pub fn solve_erp(d0: f64, g0: f64, l: f64, n: usize, rr: f64, price: f64) -> Result<f64, &'static str> {
    let lo0 = ERP_LO.max(l - rr + 1e-9);
    if lo0 >= ERP_HI {
        return Err("no_root_rate");
    }
    let f = |erp: f64| value(d0, g0, l, n, rr + erp) - price;
    let (mut lo, mut hi) = (lo0, ERP_HI);
    if f(lo) < 0.0 {
        return Err("no_root_low");
    }
    if f(hi) > 0.0 {
        return Err("no_root_high");
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if f(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(0.5 * (lo + hi))
}

/// The CPIAUCNS series of FRED, by month. A blank value is not stored.
pub type Fred = HashMap<(i32, u32), f64>;

/// The reader of a monthly FRED file (observation_date,ID; a blank or "." value is a month without a value). It reads
/// CPIAUCNS, TB3MS, GS10 and the NBER commercial paper rate.
pub fn read_fred_monthly(text: &str) -> R<Fred> {
    let mut out = Fred::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue;
        }
        let (d, v) = line.split_once(',').ok_or_else(|| DcfError(format!("cpi line {}: no comma", i + 1)))?;
        let mut p = d.split('-');
        let (Some(ys), Some(ms)) = (p.next(), p.next()) else {
            return e(format!("cpi line {}: bad date '{d}'", i + 1));
        };
        let (Ok(y), Ok(m)) = (ys.parse::<i32>(), ms.parse::<u32>()) else {
            return e(format!("cpi line {}: bad date '{d}'", i + 1));
        };
        if let Some(x) = opt_f(v) {
            out.insert((y, m), x);
        }
    }
    Ok(out)
}

/// First month for which the CPI comes from FRED.
const FRED_START: (i32, u32) = (1913, 1);

/// Why a CPI is not there: `Skip` (no Shiller month: no row), `NoValue` (FRED has no value for the month: row with
/// status `no_cpi`), `NoFile` (a row needs FRED and `--cpi` was not given: the command fails).
pub(super) enum Miss {
    Skip,
    NoValue,
    NoFile,
}

/// The year of the December dividend that is known at a month: before 1936 always Y-2, from 1936 Y-1 from April.
pub(super) fn known_year(year: i32, month: u32) -> i32 {
    if year >= LATE_PUBLICATION_END && month >= 4 {
        year - 1
    } else {
        year - 2
    }
}

pub(super) struct CpiSource<'a> {
    obs: &'a [Obs],
    idx: HashMap<(i32, u32), usize>,
    fred: Option<&'a Fred>,
}

impl<'a> CpiSource<'a> {
    pub(super) fn new(obs: &'a [Obs], fred: Option<&'a Fred>) -> Self {
        CpiSource { obs, idx: obs.iter().enumerate().map(|(i, o)| ((o.year, o.month), i)).collect(), fred }
    }
    /// The December dividend of year y (the Shiller annual rate), if the file has it.
    pub(super) fn december_dividend(&self, y: i32) -> Option<f64> {
        self.obs[*self.idx.get(&(y, 12))?].dividend
    }
    /// The December earnings of year y (the Shiller annual rate), if the file has it.
    pub(super) fn december_earnings(&self, y: i32) -> Option<f64> {
        self.obs[*self.idx.get(&(y, 12))?].earnings
    }
    fn shiller(&self, y: i32, m: u32) -> Result<f64, Miss> {
        self.idx.get(&(y, m)).map(|i| self.obs[*i].cpi).ok_or(Miss::Skip)
    }
    fn fred_at(&self, y: i32, m: u32) -> Result<f64, Miss> {
        let f = self.fred.ok_or(Miss::NoFile)?;
        f.get(&(y, m)).copied().ok_or(Miss::NoValue)
    }
    /// The CPI of December of year y, for the deflation of its dividend: Shiller before 1913, FRED from 1913.
    pub(super) fn december(&self, y: i32) -> Result<f64, Miss> {
        if (y, 12) < FRED_START {
            self.shiller(y, 12)
        } else {
            self.fred_at(y, 12)
        }
    }
    /// CPIK(month): the CPI known at the month. Before 1913-01 the Shiller December of the last ended year (a step);
    /// at 1913-01 the Shiller December 1912; from 1913-02 the FRED value of the month before.
    pub(super) fn known(&self, y: i32, m: u32) -> Result<f64, Miss> {
        if (y, m) < FRED_START {
            self.shiller(y - 1, 12)
        } else if (y, m) == FRED_START {
            self.shiller(1912, 12)
        } else {
            let (py, pm) = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
            self.fred_at(py, pm)
        }
    }
}

/// The rows of the measurement. Point in time: see the model document. `fred` is CPIAUCNS; without it the call fails
/// if a row needs it (a month from 1913-02, or a December from 1913).
pub fn compute(obs: &[Obs], fred: Option<&Fred>, l: f64, n: usize) -> R<Vec<MarketRow>> {
    let src = CpiSource::new(obs, fred);
    let dec_div = |y: i32| -> Option<f64> { src.december_dividend(y) };
    let mut rows = Vec::new();
    for o in obs.iter() {
        // before 1936 the annual figures were published late: always December of Y-2; from 1936 the April rule
        let yk = known_year(o.year, o.month);
        let (Some(d_k), Some(d_b)) = (dec_div(yk), dec_div(yk - 10)) else {
            continue;
        };
        let (by, bm) = (o.year - 10, o.month);
        let cpis = (|| -> Result<(f64, f64, f64, f64), Miss> {
            Ok((src.known(o.year, o.month)?, src.known(by, bm)?, src.december(yk)?, src.december(yk - 10)?))
        })();
        let (cpik_now, cpik_back, cpi_k, cpi_b) = match cpis {
            Ok(x) => x,
            Err(Miss::Skip) => continue,
            Err(Miss::NoFile) => return e("--cpi FILE (CPIAUCNS) is needed: a row from 1913 uses it"),
            Err(Miss::NoValue) => {
                rows.push(MarketRow { year: o.year, month: o.month, price_real: o.price, d_known_real: f64::NAN, g0: f64::NAN, rr: f64::NAN, erp_implied: None, erp_gordon: f64::NAN, estimate: o.estimate, status: "no_cpi" });
                continue;
            }
        };
        let rr = o.gs10 / 100.0 - ((cpik_now / cpik_back).powf(0.1) - 1.0);
        let d_real = d_k * cpik_now / cpi_k;
        let price = o.price;
        let mk = |g0: f64, d: f64, erp: Option<f64>, gordon: f64, status: &'static str| MarketRow {
            year: o.year,
            month: o.month,
            price_real: price,
            d_known_real: d,
            g0,
            rr,
            erp_implied: erp,
            erp_gordon: gordon,
            estimate: o.estimate,
            status,
        };
        if d_k <= 0.0 || d_b <= 0.0 {
            rows.push(mk(f64::NAN, d_real, None, f64::NAN, "no_dividend"));
            continue;
        }
        let g0 = (((d_k / cpi_k) / (d_b / cpi_b)).powf(0.1) - 1.0).clamp(G0_MIN, G0_MAX);
        let gordon = d_real * (1.0 + l) / price + l - rr;
        match solve_erp(d_real, g0, l, n, rr, price) {
            Ok(x) => rows.push(mk(g0, d_real, Some(x), gordon, "ok")),
            Err(code) => rows.push(mk(g0, d_real, None, gordon, code)),
        }
    }
    Ok(rows)
}

fn num(x: f64) -> String {
    if x.is_nan() {
        String::new()
    } else {
        format!("{x:.6}")
    }
}

pub fn csv_text(rows: &[MarketRow]) -> String {
    let mut s = String::from("month,price_real,d_known_real,g0,rr,erp_implied,erp_gordon,estimate,status\n");
    for r in rows {
        let erp = r.erp_implied.map_or("none".to_string(), num);
        s += &format!("{}-{:02},{},{},{},{},{},{},{},{}\n", r.year, r.month, num(r.price_real), num(r.d_known_real), num(r.g0), num(r.rr), erp, num(r.erp_gordon), r.estimate, r.status);
    }
    s
}

/// The summary by decade: months, months with no root, median and P10 / P90 of the implied premium.
pub fn summary(rows: &[MarketRow]) -> String {
    let mut s = format!("{:<8}{:>8}{:>8}{:>10}{:>10}{:>10}\n", "decade", "months", "no_root", "median", "P10", "P90");
    let mut line = |label: String, sel: &[&MarketRow]| {
        let v: Vec<f64> = sel.iter().filter_map(|r| r.erp_implied).collect();
        let none = sel.len() - v.len();
        if v.is_empty() {
            s += &format!("{label:<8}{:>8}{none:>8}{:>10}{:>10}{:>10}\n", sel.len(), "-", "-", "-");
        } else {
            let so = sorted_copy(&v);
            s += &format!("{label:<8}{:>8}{none:>8}{:>9.2}%{:>9.2}%{:>9.2}%\n", sel.len(), percentile(&so, 50.0) * 100.0, percentile(&so, 10.0) * 100.0, percentile(&so, 90.0) * 100.0);
        }
    };
    let mut decades: Vec<i32> = rows.iter().map(|r| r.year / 10 * 10).collect();
    decades.dedup();
    for d in decades {
        let sel: Vec<&MarketRow> = rows.iter().filter(|r| r.year / 10 * 10 == d).collect();
        line(format!("{d}s"), &sel);
    }
    let all: Vec<&MarketRow> = rows.iter().collect();
    line("all".to_string(), &all);
    s
}

pub fn command(_repo: &Path, args: &[String]) -> u8 {
    match command_inner(args) {
        Ok(()) => 0,
        Err(x) => {
            eprintln!("fa: {x}");
            2
        }
    }
}

fn command_inner(args: &[String]) -> R<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{MARKET_USAGE}");
        return Ok(());
    }
    let o = Opts::parse(args, &["--shiller", "--cpi", "--out", "--long-real", "--fade-years", "--tb3ms", "--nber", "--gs10", "--score", "--months-out"], &[])?;
    let scoring = o.get("--score").is_some();
    if scoring && (o.get("--long-real").is_some() || o.get("--fade-years").is_some()) {
        return e("--score uses the fixed design (L 2%, N 10 and the four variants): do not give --long-real or --fade-years");
    }
    let file = o.get("--shiller").ok_or_else(|| DcfError("--shiller FILE is needed".into()))?;
    let out = o.get("--out").ok_or_else(|| DcfError("--out FILE is needed".into()))?;
    let l: f64 = o.num("--long-real", 0.02)?;
    let n: usize = o.num("--fade-years", 10)?;
    if n < 2 {
        return e("--fade-years must be at least 2");
    }
    let text = std::fs::read_to_string(file).map_err(|x| DcfError(format!("{file}: {x}")))?;
    let obs = read_shiller(&text)?;
    let fred = match o.get("--cpi") {
        Some(p) => Some(read_fred_monthly(&std::fs::read_to_string(p).map_err(|x| DcfError(format!("{p}: {x}")))?)?),
        None => None,
    };
    let rows = compute(&obs, fred.as_ref(), l, n)?;
    std::fs::write(out, csv_text(&rows)).map_err(|x| DcfError(format!("{out}: {x}")))?;
    println!("MARKET level: dividend DCF, real terms, L = {:.2}%, fade {n} years, {} months, {file}. A measurement: the price is the input of the search.", l * 100.0, rows.len());
    print!("{}", summary(&rows));
    if let Some(score) = o.get("--score") {
        let read = |opt: &str| -> R<Fred> {
            let p = o.get(opt).ok_or_else(|| DcfError(format!("--score needs {opt} FILE")))?;
            read_fred_monthly(&std::fs::read_to_string(p).map_err(|x| DcfError(format!("{p}: {x}")))?)
        };
        let cpi = fred.as_ref().ok_or_else(|| DcfError("--score needs --cpi FILE".into()))?;
        let inputs = super::market_score::Inputs { cpi, tb3ms: &read("--tb3ms")?, nber: &read("--nber")?, gs10: &read("--gs10")? };
        let report = super::market_score::run(&obs, &inputs)?;
        std::fs::write(score, &report.score_text).map_err(|x| DcfError(format!("{score}: {x}")))?;
        if let Some(m) = o.get("--months-out") {
            std::fs::write(m, &report.months_csv).map_err(|x| DcfError(format!("{m}: {x}")))?;
        }
        print!("{}", report.score_text);
    }
    Ok(())
}

#[cfg(test)]
#[path = "market_tests.rs"]
mod tests;
