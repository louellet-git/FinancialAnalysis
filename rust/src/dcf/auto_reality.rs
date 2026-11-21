//! `fa dcf attribute --reality`: the implied growth and the implied premium against what happened before (item R2 of the ticket
//! `T-20261121-dcf-engine-integration`). A measurement. No input of the model is set from it.
//!
//! * The implied 10-year growth at D2 (the growth view of the attribution, at the model premium) is put in the bucket of the
//!   base revenue of the ticker and against the last windows of the base rates (`auto_baserate`) at D2: its percentile is the
//!   share of those growths that are below it; it is "above every case" when it is above the maximum.
//! * With a history of `fa dcf premium-series`, the premium that the list implies at D1 and at D2 is put against the rows of the
//!   history that are before D1 (the rows of D1 and later are not used).
//! * The path of the 8-K earnings release of the window, when the cache has it.

use super::auto::YearData;
use super::auto_baserate::{bucket_of, last_year, position_in, windows, BaseRates, Position, BUCKETS};
use super::auto_implied::Bound;
use super::DcfError;

type R<T> = Result<T, DcfError>;

/// The years of the own growth of a company (the 5-year growth of its last fiscal year).
pub const OWN_YEARS: usize = 5;

/// The tail of the CSV header of `--reality`. The columns come after the columns of `fa dcf attribute`.
pub const HEADER_TAIL: &str = "base_revenue,base_bucket,growth5_own,growth10_implied,growth10_bound,growth10_pctile,growth10_above_all,list_premium1,list_premium2,list_premium1_pctile,list_premium2_pctile,release_8k_path";

/// The reality numbers of one ticker.
#[derive(Debug, Clone, PartialEq)]
pub struct RealityRow {
    pub ticker: String,
    /// The revenue of the last fiscal year that was public at D2, and its bucket of `auto_baserate::BUCKETS`.
    pub base_revenue: Option<f64>,
    pub bucket: Option<usize>,
    /// The compound annual growth of the last 5 fiscal years (`None`: no window).
    pub growth5: Option<f64>,
    /// The implied 10-year growth at D2 (`None`: no value), and where its search ended (not `Exact`: a floor or a ceiling).
    pub implied: Option<f64>,
    pub bound: Bound,
    /// The place of the implied growth among the last windows of its bucket.
    pub position: Option<Position>,
    /// The path of the 8-K release file (empty: none in the cache).
    pub release: String,
}

/// The reality numbers of one ticker. `implied` is the growth of the growth view at D2 and where its search ended.
pub fn reality_row(ticker: &str, years: &[YearData], d2: &str, implied: Option<(f64, Bound)>, rates: &BaseRates, release: String) -> RealityRow {
    let last = last_year(years, d2);
    let bucket = last.as_ref().map(|(_, r)| bucket_of(*r));
    let growth5 = last.as_ref().and_then(|(end, _)| windows(years, OWN_YEARS, d2).into_iter().find(|w| &w.end == end)).map(|w| w.growth);
    let implied_ok = implied.filter(|(g, _)| g.is_finite());
    let position = bucket.zip(implied_ok).and_then(|(b, (g, _))| position_in(&rates.last_growths(b), g));
    RealityRow {
        ticker: ticker.to_string(),
        base_revenue: last.map(|(_, r)| r),
        bucket,
        growth5,
        implied: implied_ok.map(|x| x.0),
        bound: implied_ok.map_or(Bound::Exact, |x| x.1),
        position,
        release,
    }
}

/// The history of `fa dcf premium-series`: (anchor, s1). A row with no s1 is out.
pub fn parse_history(text: &str) -> R<Vec<(String, f64)>> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let header: Vec<&str> = lines.next().unwrap_or("").split(',').map(str::trim).collect();
    let (Some(ia), Some(is)) = (header.iter().position(|h| *h == "anchor"), header.iter().position(|h| *h == "s1")) else {
        return Err(DcfError("--premium-history: the header needs the columns anchor and s1 (the file of `fa dcf premium-series`)".into()));
    };
    let mut out = Vec::new();
    for l in lines {
        let f: Vec<&str> = l.split(',').map(str::trim).collect();
        let (Some(a), Some(s)) = (f.get(ia), f.get(is)) else { continue };
        if let Ok(x) = s.parse::<f64>() {
            if x.is_finite() && a.len() >= 10 {
                out.push((a[..10].to_string(), x));
            }
        }
    }
    Ok(out)
}

/// The premiums of the rows of the history with an anchor BEFORE `d1` (a row of D1 or later is not used).
pub fn history_before(history: &[(String, f64)], d1: &str) -> Vec<f64> {
    let d1 = d1.get(..10).unwrap_or(d1);
    history.iter().filter(|(a, _)| a.as_str() < d1).map(|(_, x)| *x).collect()
}

/// The premium that the list implies at D1 and at D2, with the place of each in the rows of the history before D1.
#[derive(Debug, Clone, PartialEq)]
pub struct PremiumPosition {
    pub premium1: Option<f64>,
    pub premium2: Option<f64>,
    pub p1: Option<Position>,
    pub p2: Option<Position>,
    /// The rows before D1 and their first and last anchor.
    pub n_before: usize,
    pub first: String,
    pub last: String,
}

pub fn premium_position(history: &[(String, f64)], d1: &str, premium1: Option<f64>, premium2: Option<f64>) -> PremiumPosition {
    let before = history_before(history, d1);
    let d1 = d1.get(..10).unwrap_or(d1);
    let anchors: Vec<&str> = history.iter().map(|(a, _)| a.as_str()).filter(|a| *a < d1).collect();
    PremiumPosition {
        premium1,
        premium2,
        p1: premium1.and_then(|x| position_in(&before, x)),
        p2: premium2.and_then(|x| position_in(&before, x)),
        n_before: before.len(),
        first: anchors.iter().min().map_or(String::new(), |a| a.to_string()),
        last: anchors.iter().max().map_or(String::new(), |a| a.to_string()),
    }
}

fn dec(x: Option<f64>, d: usize) -> String {
    x.filter(|x| x.is_finite()).map_or(String::new(), |x| format!("{x:.d$}"))
}

fn bound_mark(b: Bound) -> &'static str {
    match b {
        Bound::Exact => "",
        Bound::Below => "<",
        Bound::Above => ">",
    }
}

/// The CSV cells of one row, in the order of `HEADER_TAIL`. The premium columns are the same on each row (they are numbers of
/// the list).
pub fn cells(r: &RealityRow, prem: Option<&PremiumPosition>) -> String {
    let flag = r.position.map_or(String::new(), |p| p.above_all.to_string());
    let f = [
        dec(r.base_revenue, 0),
        r.bucket.map_or(String::new(), |b| BUCKETS[b].to_string()),
        dec(r.growth5, 4),
        dec(r.implied, 4),
        bound_mark(r.bound).to_string(),
        dec(r.position.map(|p| p.pctile), 1),
        flag,
        dec(prem.and_then(|p| p.premium1), 4),
        dec(prem.and_then(|p| p.premium2), 4),
        dec(prem.and_then(|p| p.p1).map(|p| p.pctile), 1),
        dec(prem.and_then(|p| p.p2).map(|p| p.pctile), 1),
        r.release.replace(',', ";"),
    ];
    f.join(",")
}

fn pct(x: Option<f64>) -> String {
    x.map_or("-".to_string(), |x| format!("{:.1}%", x * 100.0))
}

/// The block "(5) REALITY CHECK" of the text. `source` names the list of the base rates.
pub fn block_text(rows: &[RealityRow], rates: &BaseRates, source: &str, prem: Option<&PremiumPosition>) -> String {
    let mut s = format!(
        "\n(5) REALITY CHECK: the implied {}-year growth at D2 against the growth that companies had. A measurement, not an input of the model.\n  base rates: {} companies of {source} ({} tickers); the last window of {} fiscal years of each company, filed on or before {}; the bucket is the revenue of the base year (nominal dollars). The percentile is the share of the last windows of the bucket that are below the implied growth.\n",
        rates.years, rates.companies, rates.tickers, rates.years, rates.as_of
    );
    s.push_str(&super::auto_baserate::table_text("LAST WINDOWS by bucket", &rates.last));
    s.push_str(&format!("  {:<8} {:>10} {:<12} {:>9} {:>9} {:>11}  position in the bucket\n", "ticker", "base rev", "bucket", "own 5y", "implied", "bound"));
    for r in rows {
        let place = match (&r.position, r.implied) {
            (Some(p), _) if p.above_all => format!("above every case of {} (n {})", r.bucket.map_or("", |b| BUCKETS[b]), p.n),
            (Some(p), _) => format!("percentile {:.0} of {} (n {})", p.pctile, r.bucket.map_or("", |b| BUCKETS[b]), p.n),
            (None, None) => "no implied growth".to_string(),
            (None, Some(_)) => "no last window in the bucket".to_string(),
        };
        let base = r.base_revenue.map_or("-".to_string(), |x| format!("{:.1}B", x / 1e9));
        let bound = match r.bound {
            Bound::Exact => "exact",
            Bound::Below => "ceiling (<)",
            Bound::Above => "floor (>)",
        };
        s.push_str(&format!("  {:<8} {:>10} {:<12} {:>9} {:>9} {:>11}  {place}\n", r.ticker, base, r.bucket.map_or("-", |b| BUCKETS[b]), pct(r.growth5), pct(r.implied), bound));
    }
    let releases: Vec<String> = rows.iter().filter(|r| !r.release.is_empty()).map(|r| format!("{} {}", r.ticker, r.release)).collect();
    if releases.is_empty() {
        s.push_str("  8-K earnings release in the window: no file in the cache\n");
    } else {
        s.push_str("  8-K earnings release in the window (the file of the cache):\n");
        for l in releases {
            s.push_str(&format!("    {l}\n"));
        }
    }
    if let Some(p) = prem {
        let one = |label: &str, x: Option<f64>, pos: &Option<Position>| match (x, pos) {
            (Some(x), Some(p)) => format!("{label} {:.2}%: {} of the {} rows before D1", x * 100.0, if p.above_all { "above every row".to_string() } else { format!("percentile {:.0}", p.pctile) }, p.n),
            (Some(x), None) => format!("{label} {:.2}%: no row before D1", x * 100.0),
            (None, _) => format!("{label}: no solution in the range"),
        };
        s.push_str(&format!("  list premium (the list of the run, before --top; the premium for which the median of value over price is 1) against the history rows before D1 ({} rows, {} to {}): {}; {}\n", p.n_before, p.first, p.last, one("D1", p.premium1, &p.p1), one("D2", p.premium2, &p.p2)));
    }
    s.push_str("  Limits: the list of the base rates is a list of today's names (survivorship); the windows overlap; acquired revenue is in the growth; the implied growth is the growth of one path at the means, at the model premium and the model margin (a floor or a ceiling where the search has no solution in its range); the base year of a company with no mapped revenue is not patched.\n");
    s
}

#[cfg(test)]
#[path = "auto_reality_tests.rs"]
mod tests;
