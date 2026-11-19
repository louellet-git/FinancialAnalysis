//! `fa recon rich-cheap`: the difference of each Treasury security from a curve that is
//! fitted to the securities themselves (`fit.rs`), the cost, the group, and the list of
//! rich and cheap candidates.
//!
//! SIGN. `res_bp = observed yield - fitted yield`. Positive = the price is below the curve:
//! CHEAP. Negative = the price is above the curve: RICH.
//!
//! The command gives candidates. It does not give arbitrage. See `LIMITS`.

use crate::curve::{self, Bond, Day};
use crate::fit::{self, Excluded, Nss, Sec};
use crate::prices::Quote;
use crate::recon::{bucket, BUCKET_ORDER};
use std::fmt::Write as _;

/// One security with its difference from the fitted curve.
#[derive(Debug, Clone)]
pub struct Row {
    pub id: String,
    pub kind: &'static str,
    pub coupon: Option<f64>,
    pub maturity: Day,
    pub years: f64,
    pub obs_clean: f64,
    pub buy: Option<f64>,
    pub sell: Option<f64>,
    pub model_clean: f64,
    pub y_obs: f64,
    pub y_model: f64,
    pub d_mod: f64,
    /// Observed minus fitted yield (bp). Positive = cheap.
    pub res_bp: f64,
    pub z: f64,
    pub in_fit: bool,
    pub ba_bp: f64,
    pub ba_source: &'static str,
    /// The cost used: the larger of the bid-ask and one price step of the source (bp).
    pub cost_bp: f64,
    /// The fit error of the day (RMSE of the fit set, bp): a residual below this is within the model error.
    pub noise_bp: f64,
    pub group: &'static str,
    pub near_coupon: bool,
}

impl Row {
    pub fn price_res(&self) -> f64 {
        self.obs_clean - self.model_clean
    }
    /// A candidate: the residual is larger than the cost and larger than the fit error of the day,
    /// the z-score is 2 or more, and the life is 3 months or more.
    pub fn survives(&self) -> bool {
        self.res_bp.abs() > self.cost_bp.max(self.noise_bp) && self.z.abs() >= 2.0 && self.years >= 0.25
    }
    pub fn side(&self) -> &'static str {
        if self.res_bp > 0.0 { "cheap" } else { "rich" }
    }
}

pub struct Analysis {
    pub date: String,
    pub settle: Day,
    pub nss: Nss,
    pub excluded: Excluded,
    pub n_fit: usize,
    pub rows: Vec<Row>,
    pub rmse_all: f64,
}

pub const GROUPS: [&str; 8] = [
    "bill",
    "under 3 months",
    "recent issue",
    "old 30-year bond",
    "high coupon",
    "low coupon",
    "near coupon date",
    "other",
];

/// What the data cannot show, for each group.
pub fn limits(group: &str) -> &'static str {
    match group {
        "bill" => "Bills are not in the fit and the fitted curve is extrapolated below 3 months. The gap to coupon securities is a known effect (money-market demand, tax, different holders). No repo rate, no balance-sheet cost.",
        "under 3 months" => "Outside the fit. One price step is large in yield. Pull-to-par and cash-like demand; the file has no financing rate.",
        "recent issue" => "On-the-run and recent securities are rich because they are the ones that are financed and shorted; the file has no repo or special-repo rate, so the premium cannot be tested.",
        "old 30-year bond" => "Original 30-year bonds (maturity before 2040) against notes of the same date: STRIPS demand, coupon and tax effects, futures deliverable status and who holds them are not in the file. The gap to a note of the same maturity is a coupon or issue-type effect that the curve does not model; it is shared by the whole group, so it is a group effect and not one mispriced security.",
        "high coupon" => "A high coupon has a different cash-flow shape and a different tax treatment (premium, ordinary income). The data has no investor tax rate and no repo rate.",
        "low coupon" => "A low coupon trades at a deep discount: market-discount tax rules and financing differ. The data cannot show tax or repo.",
        "near coupon date" => "Close to a coupon date the accrued interest is a large part of the cash price and the price step of the source is the same. The data has no ex-coupon or settlement-fail information.",
        _ => "No known cause from the file alone. Possible: scarcity (repo special), stale price, odd size. The file has no volume, no repo rate and no short-sale cost.",
    }
}

pub const LIMITS_ALL: &str = "\
Not in the data: repo and special-repo rates (the cost of financing a long and the gain on a short), the cost of a short sale,
balance-sheet and capital cost, tax, trade size and market impact, intraday time of the price, and fees. The fitted curve is
a model: a residual is the sum of a real price difference and the model error. A candidate is a security to study; it is
not a trade.";

fn first_coupon_flag(bond: &Bond, settle: Day) -> bool {
    let s = bond.schedule(settle);
    s.next[0] - settle <= 10
}

/// Recent issue: the remaining life is within 0.35 year below a nominal term of the issue cycle
/// AND the coupon is within 0.5 percentage point of the yield (a new issue has a coupon near the
/// market yield; an old 10-year note with 3 years left does not). The file has no issue date.
fn recent_issue(kind: &str, years: f64, coupon: f64, y_obs: f64) -> bool {
    const TERMS: [(f64, &str); 7] = [(2.0, "note"), (3.0, "note"), (5.0, "note"), (7.0, "note"), (10.0, "note"), (20.0, "bond"), (30.0, "bond")];
    (coupon - y_obs).abs() <= 0.005 && TERMS.iter().any(|(t, k)| *k == kind && years >= t - 0.35 && years <= t + 0.05)
}

/// A "bond" that matures before 2040 is an original 30-year bond (Treasury issued 20-year bonds
/// again only from 2020, which mature from 2040).
fn old_thirty_year(kind: &str, maturity: Day) -> bool {
    kind == "bond" && maturity < curve::days_from_civil(2040, 1, 1)
}

/// Median coupon and (median, scaled MAD) of the residuals of the `k` securities nearest in maturity, self left out.
fn neighbours(pool: &[(f64, f64, f64)], i: usize, k: usize) -> (f64, f64, f64) {
    // pool = (years, res_bp, coupon)
    let mut idx: Vec<usize> = (0..pool.len()).filter(|&j| j != i).collect();
    idx.sort_by(|&a, &b| (pool[a].0 - pool[i].0).abs().total_cmp(&(pool[b].0 - pool[i].0).abs()));
    idx.truncate(k);
    let r: Vec<f64> = idx.iter().map(|&j| pool[j].1).collect();
    let mut c: Vec<f64> = idx.iter().map(|&j| pool[j].2).collect();
    let med = fit::median(&mut r.clone());
    let mut dev: Vec<f64> = r.iter().map(|x| (x - med).abs()).collect();
    let mad = 1.4826 * fit::median(&mut dev);
    (med, mad, fit::median(&mut c))
}

fn nonzero(x: f64) -> Option<f64> {
    if x > 0.0 { Some(x) } else { None }
}

/// The bid-ask of a quote in price per 100: BUY minus SELL, when both exist.
/// In the file the SELL column is the bid side (it is equal to END OF DAY for most notes), BUY is above it.
fn file_spread(q: &Quote) -> Option<f64> {
    (q.buy > 0.0 && q.sell > 0.0 && q.buy > q.sell).then_some(q.buy - q.sell)
}

/// Fit the curve to the date and give the difference of each security.
pub fn analyse(date: &str, settle: Day, quotes: &[Quote]) -> Result<Analysis, String> {
    let (mut secs, mut ex) = fit::select(settle, quotes, 0.25);
    let (nss, _) = fit::fit_robust(&mut secs, &mut ex).ok_or("too few notes and bonds to fit a curve")?;
    let n_fit = secs.iter().filter(|s| s.in_fit).count();

    let mut rows: Vec<Row> = Vec::new();
    // Notes and bonds of 3 months or more (already in `secs`).
    for s in &secs {
        rows.push(coupon_row(s, &nss, settle));
    }
    // Notes and bonds of less than 3 months, and bills: priced on the curve, not in the fit.
    for q in quotes {
        if q.call_date.is_some() || q.end_of_day <= 0.0 || q.maturity - settle < 7 {
            continue;
        }
        match q.security_type.as_str() {
            "MARKET BASED BILL" => rows.push(bill_row(q, &nss, settle)),
            "MARKET BASED NOTE" | "MARKET BASED BOND" if q.coupon > 0.0 && ((q.maturity - settle) as f64 / 365.0) < 0.25 => {
                let bond = Bond { coupon: q.coupon, maturity: q.maturity };
                if let Some((inst, y, d)) = fit::make_inst(&bond, settle, q.end_of_day) {
                    let s = Sec { quote: q.clone(), bond, inst, y_obs: y, d_mod: d, years: (q.maturity - settle) as f64 / 365.0, in_fit: false };
                    rows.push(coupon_row(&s, &nss, settle));
                }
            }
            _ => {}
        }
    }
    finish_rows(&mut rows);
    let in_fit: Vec<f64> = rows.iter().filter(|r| r.in_fit).map(|r| r.res_bp).collect();
    let rmse_all = (in_fit.iter().map(|x| x * x).sum::<f64>() / in_fit.len().max(1) as f64).sqrt();
    for r in rows.iter_mut() {
        r.noise_bp = rmse_all;
    }
    Ok(Analysis { date: date.to_string(), settle, nss, excluded: ex, n_fit, rows, rmse_all })
}

/// Observed minus fitted yield (bp) of a quote on a given curve: for a security that was not
/// in the fit. `None` if the quote has no usable price.
pub fn residual_bp(nss: &Nss, settle: Day, q: &Quote) -> Option<f64> {
    let bond = Bond { coupon: q.coupon, maturity: q.maturity };
    let (inst, y, _) = fit::make_inst(&bond, settle, q.end_of_day)?;
    let y_model = bond.yield_from_clean(inst.model_clean(nss), settle)?;
    Some((y - y_model) * 1e4)
}

fn tick_bp(bond: &Bond, settle: Day, price: f64) -> f64 {
    // The source gives notes and bonds in 64ths: the yield change of one step.
    match (bond.yield_from_clean(price - 1.0 / 128.0, settle), bond.yield_from_clean(price + 1.0 / 128.0, settle)) {
        (Some(a), Some(b)) => (a - b).abs() * 1e4,
        _ => f64::NAN,
    }
}

fn coupon_row(s: &Sec, nss: &Nss, settle: Day) -> Row {
    let q = &s.quote;
    let model_clean = s.inst.model_clean(nss);
    let y_model = s.bond.yield_from_clean(model_clean, settle).unwrap_or(f64::NAN);
    let ba_price = file_spread(q);
    let dirty = q.end_of_day + s.bond.accrued(settle);
    let ba_bp = ba_price.map_or(f64::NAN, |b| b / (dirty * s.d_mod) * 1e4);
    let kind = if q.security_type.ends_with("BOND") { "bond" } else { "note" };
    Row {
        id: q.cusip.clone(),
        kind,
        coupon: Some(q.coupon),
        maturity: q.maturity,
        years: s.years,
        obs_clean: q.end_of_day,
        buy: nonzero(q.buy),
        sell: nonzero(q.sell),
        model_clean,
        y_obs: s.y_obs,
        y_model,
        d_mod: s.d_mod,
        res_bp: (s.y_obs - y_model) * 1e4,
        z: f64::NAN,
        in_fit: s.in_fit,
        ba_bp,
        ba_source: if ba_price.is_some() { "file" } else { "assumed" },
        cost_bp: tick_bp(&s.bond, settle, q.end_of_day),
        noise_bp: 0.0,
        group: "other",
        near_coupon: first_coupon_flag(&s.bond, settle),
    }
}

fn bill_row(q: &Quote, nss: &Nss, settle: Day) -> Row {
    let days = q.maturity - settle;
    let t = days as f64 / 365.0;
    let model = 100.0 * nss.df(t);
    let y_obs = curve::bill_yield(q.end_of_day, days);
    let y_model = curve::bill_yield(model, days);
    let ba_price = file_spread(q).or_else(|| {
        // Bills: BUY and SELL are quotes on the price date. Use the absolute difference when both exist.
        (q.buy > 0.0 && q.sell > 0.0).then(|| (q.buy - q.sell).abs())
    });
    let ba_bp = ba_price.map_or(f64::NAN, |b| b / q.end_of_day * 365.0 / days as f64 * 1e4);
    let tick = (curve::bill_yield(q.end_of_day - 5e-7, days) - curve::bill_yield(q.end_of_day + 5e-7, days)).abs() * 1e4;
    Row {
        id: q.cusip.clone(),
        kind: "bill",
        coupon: None,
        maturity: q.maturity,
        years: t,
        obs_clean: q.end_of_day,
        buy: nonzero(q.buy),
        sell: nonzero(q.sell),
        model_clean: model,
        y_obs,
        y_model,
        d_mod: t / (1.0 + y_obs * t),
        res_bp: (y_obs - y_model) * 1e4,
        z: f64::NAN,
        in_fit: false,
        ba_bp,
        ba_source: if ba_price.is_some() { "file" } else { "assumed" },
        // The EOD price of a bill is a discount rate in steps of 1 bp: that step is the resolution.
        cost_bp: tick.max(1.0),
        noise_bp: 0.0,
        group: "bill",
        near_coupon: false,
    }
}

/// z-scores, the cost, the group. Needs all rows.
fn finish_rows(rows: &mut [Row]) {
    // z-score against the 20 securities nearest in maturity (bills: among bills).
    for bills in [false, true] {
        let idx: Vec<usize> = (0..rows.len()).filter(|&i| (rows[i].kind == "bill") == bills).collect();
        let pool: Vec<(f64, f64, f64)> = idx.iter().map(|&i| (rows[i].years, rows[i].res_bp, rows[i].coupon.unwrap_or(0.0))).collect();
        for (k, &i) in idx.iter().enumerate() {
            let (med, mad, med_coupon) = neighbours(&pool, k, 20);
            rows[i].z = (rows[i].res_bp - med) / mad.max(0.3);
            if !bills {
                let c = rows[i].coupon.unwrap_or(0.0);
                rows[i].group = classify(&rows[i], c - med_coupon);
            }
        }
    }
    // Bid-ask: the file where it exists, else the median of the file values in the same maturity bucket.
    let mut by_bucket: std::collections::BTreeMap<&str, Vec<f64>> = std::collections::BTreeMap::new();
    for r in rows.iter().filter(|r| r.ba_source == "file" && r.ba_bp.is_finite()) {
        by_bucket.entry(bucket(r.years)).or_default().push(r.ba_bp);
    }
    let all: Vec<f64> = by_bucket.values().flatten().copied().collect();
    let global = fit::median(&mut all.clone());
    let medians: std::collections::BTreeMap<&str, f64> =
        by_bucket.iter_mut().map(|(k, v)| (*k, fit::median(v))).collect();
    for r in rows.iter_mut() {
        if r.ba_source == "assumed" {
            r.ba_bp = medians.get(bucket(r.years)).copied().unwrap_or(global);
        }
        if !r.ba_bp.is_finite() {
            r.ba_bp = 0.0;
        }
        r.cost_bp = r.cost_bp.max(r.ba_bp);
    }
}

/// The group of a coupon security, by the first rule that fits.
pub fn classify(r: &Row, coupon_minus_neighbours: f64) -> &'static str {
    if r.years < 0.25 {
        "under 3 months"
    } else if recent_issue(r.kind, r.years, r.coupon.unwrap_or(0.0), r.y_obs) {
        "recent issue"
    } else if old_thirty_year(r.kind, r.maturity) {
        "old 30-year bond"
    } else if coupon_minus_neighbours >= 0.0125 {
        "high coupon"
    } else if coupon_minus_neighbours <= -0.0125 {
        "low coupon"
    } else if r.near_coupon {
        "near coupon date"
    } else {
        "other"
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

pub const CSV_HEADER: &str = "id,type,coupon,maturity,years,price_date,settle_date,obs_clean,buy,sell,model_clean,price_res,obs_yield,model_yield,res_bp,z,duration,in_fit,bidask_bp,bidask_source,cost_bp,noise_bp,group,near_coupon,side,candidate";

pub fn csv_row(r: &Row, date: &str, settle: Day) -> String {
    let m = |v: Option<f64>| v.map_or(String::new(), |x| format!("{x:.6}"));
    format!(
        "{},{},{},{},{:.4},{},{},{:.6},{},{},{:.6},{:.6},{:.6},{:.6},{:.3},{:.2},{:.3},{},{:.3},{},{:.3},{:.3},{},{},{},{}",
        r.id,
        r.kind,
        r.coupon.map_or(String::new(), |c| format!("{c:.6}")),
        curve::iso(r.maturity),
        r.years,
        date,
        curve::iso(settle),
        r.obs_clean,
        m(r.buy),
        m(r.sell),
        r.model_clean,
        r.price_res(),
        r.y_obs,
        r.y_model,
        r.res_bp,
        r.z,
        r.d_mod,
        r.in_fit,
        r.ba_bp,
        r.ba_source,
        r.cost_bp,
        r.noise_bp,
        r.group.replace(',', ";"),
        r.near_coupon,
        r.side(),
        r.survives()
    )
}

pub fn write_csv(path: &std::path::Path, a: &Analysis) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut text = String::from(CSV_HEADER);
    text.push('\n');
    let mut rows: Vec<&Row> = a.rows.iter().collect();
    rows.sort_by(|x, y| x.maturity.cmp(&y.maturity).then(x.id.cmp(&y.id)));
    for r in rows {
        text.push_str(&csv_row(r, &a.date, a.settle));
        text.push('\n');
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn rms(v: &[f64]) -> f64 {
    (v.iter().map(|x| x * x).sum::<f64>() / v.len() as f64).sqrt()
}

/// The text report of one date. `gsw` has (years, GSW zero, our zero) for the comparison table.
pub fn report(a: &Analysis, gsw: &[(u32, f64, f64)], gsw_date: Option<&str>) -> String {
    let mut o = String::new();
    let n = &a.nss;
    let _ = writeln!(o, "Price date {} | settlement {} (T+1) | prices: FedInvest END OF DAY (bid side), clean, per 100", a.date, curve::iso(a.settle));
    let _ = writeln!(
        o,
        "Fitted curve: Nelson-Siegel-Svensson, weights 1/(dirty x duration). b0={:.4}% b1={:.4}% b2={:.4}% b3={:.4}% tau1={:.2}y tau2={:.2}y",
        n.b0 * 100.0,
        n.b1 * 100.0,
        n.b2 * 100.0,
        n.b3 * 100.0,
        n.tau1,
        n.tau2
    );
    let e = &a.excluded;
    let _ = writeln!(
        o,
        "Fit set: {} notes and bonds. Left out of the fit: {} bills, {} under 3 months, {} TIPS/FRN, {} callable, {} no price, {} zero coupon, {} far from the curve (outliers).",
        a.n_fit, e.bills, e.short_life, e.tips_frn, e.callable, e.no_price, e.zero_coupon, e.outliers
    );
    let _ = writeln!(o, "Fit error (yield bp, fit set): RMSE {:.2}\n", a.rmse_all);
    let _ = writeln!(o, "FIT ERROR BY MATURITY (fit set; bp of yield)\n{:<8} {:>4} {:>7} {:>8}", "bucket", "n", "RMSE", "mean|e|");
    for b in BUCKET_ORDER {
        let v: Vec<f64> = a.rows.iter().filter(|r| r.in_fit && bucket(r.years) == b).map(|r| r.res_bp).collect();
        if !v.is_empty() {
            let mabs = v.iter().map(|x| x.abs()).sum::<f64>() / v.len() as f64;
            let _ = writeln!(o, "{b:<8} {:>4} {:>7.2} {:>8.2}", v.len(), rms(&v), mabs);
        }
    }
    if !gsw.is_empty() {
        let _ = writeln!(
            o,
            "\nCOMPARISON WITH THE FEDERAL RESERVE BOARD CURVE (GSW, zero-coupon yield, continuous; GSW row {}; our curve is fitted to the prices of the same day)\n{:<6} {:>9} {:>9} {:>8}",
            gsw_date.unwrap_or("?"),
            "years",
            "GSW %",
            "NSS %",
            "NSS-GSW bp"
        );
        for (y, g, f) in gsw {
            let _ = writeln!(o, "{y:<6} {:>9.3} {:>9.3} {:>+8.1}", g * 100.0, f * 100.0, (f - g) * 1e4);
        }
    }
    let _ = writeln!(o, "\nBY GROUP (res_bp = observed - fitted yield; + = cheap, - = rich)");
    let _ = writeln!(o, "{:<22} {:>4} {:>8} {:>8} {:>6} {:>6}", "group", "n", "mean bp", "mean|bp|", "cheap", "rich");
    for grp in GROUPS {
        let g: Vec<&Row> = a.rows.iter().filter(|r| r.group == grp).collect();
        if g.is_empty() {
            continue;
        }
        let mean = g.iter().map(|r| r.res_bp).sum::<f64>() / g.len() as f64;
        let mabs = g.iter().map(|r| r.res_bp.abs()).sum::<f64>() / g.len() as f64;
        let cheap = g.iter().filter(|r| r.survives() && r.res_bp > 0.0).count();
        let rich = g.iter().filter(|r| r.survives() && r.res_bp < 0.0).count();
        let _ = writeln!(o, "{grp:<22} {:>4} {:>+8.2} {:>8.2} {:>6} {:>6}", g.len(), mean, mabs, cheap, rich);
    }
    let total = a.rows.len();
    let surv = a.rows.iter().filter(|r| r.survives()).count();
    let _ = writeln!(
        o,
        "\nCandidates: |res_bp| > cost and > fit RMSE, |z| >= 2, life >= 3 months: {surv} of {total} securities ({} cheap, {} rich).",
        a.rows.iter().filter(|r| r.survives() && r.res_bp > 0.0).count(),
        a.rows.iter().filter(|r| r.survives() && r.res_bp < 0.0).count()
    );
    let _ = writeln!(
        o,
        "Cost = larger of (bid-ask: BUY - SELL of the file, else the median of the file in the same maturity bucket) and one price step (1/64) of the source, one leg. A residual below the fit RMSE is within the model error."
    );
    for (title, cheap) in [("CHEAP CANDIDATES (price below the curve)", true), ("RICH CANDIDATES (price above the curve)", false)] {
        let mut v: Vec<&Row> = a.rows.iter().filter(|r| r.survives() && (r.res_bp > 0.0) == cheap).collect();
        v.sort_by(|x, y| y.res_bp.abs().total_cmp(&x.res_bp.abs()));
        let _ = writeln!(o, "\n{title}: top {} of {}", v.len().min(10), v.len());
        let _ = writeln!(
            o,
            "{:<10} {:<5} {:>7} {:<10} {:>9} {:>8} {:>6} {:>6} {:>5}  {}",
            "cusip", "type", "coupon", "maturity", "price", "res bp", "z", "cost", "ba", "group"
        );
        for r in v.iter().take(10) {
            let _ = writeln!(
                o,
                "{:<10} {:<5} {:>7} {:<10} {:>9.4} {:>+8.1} {:>+6.1} {:>6.1} {:>5}  {}",
                r.id,
                r.kind,
                r.coupon.map_or("-".into(), |c| format!("{:.3}%", c * 100.0)),
                curve::iso(r.maturity),
                r.obs_clean,
                r.res_bp,
                r.z,
                r.cost_bp,
                if r.ba_source == "file" { "file" } else { "est." },
                r.group
            );
        }
    }
    let _ = writeln!(o, "\nWHAT THE DATA CANNOT SHOW (by group)");
    for grp in GROUPS {
        if a.rows.iter().any(|r| r.group == grp) {
            let _ = writeln!(o, "- {grp}: {}", limits(grp));
        }
    }
    let _ = writeln!(o, "\n{LIMITS_ALL}");
    o
}

// ---------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------

pub const HELP: &str = "\
fa recon rich-cheap [--date YYYY-MM-DD] [--out FILE]

Fit a Nelson-Siegel-Svensson discount curve to the market prices of the Treasury notes
and bonds of the date (FedInvest END OF DAY file). List each security with its difference
from the curve (price and yield), z-score, cost and group, and the rich and cheap
candidates whose difference is larger than the cost. Compare the curve with the GSW curve.

  --date  A business day that has a cached price file. Default: the latest cached day.
  --out   The CSV file. Default: data/recon/richcheap_<date>.csv in the repository.

The price files are in data/raw/prices/fedinvest_<date>.csv (see hidden assumption 69).";

/// The latest day that has a cached price file with end-of-day prices.
pub fn latest_cached(cache: &std::path::Path) -> Option<String> {
    let mut days: Vec<String> = std::fs::read_dir(cache.join("prices"))
        .ok()?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|n| n.strip_prefix("fedinvest_")?.strip_suffix(".csv").map(str::to_string))
        .filter(|d| curve::parse_iso(d).is_some())
        .collect();
    days.sort();
    days.pop()
}

/// (years, GSW zero, our zero) for the tenors that both have.
pub fn gsw_table(nss: &Nss, ds: &crate::forwards::Dataset, date: &str, day: Day, settle: Day) -> Option<(Vec<(u32, f64, f64)>, String)> {
    let f = crate::forwards::forwards_on(ds, date, day).ok()?;
    let table = [1u32, 2, 3, 5, 7, 10, 15, 20, 25, 30]
        .iter()
        .filter_map(|n| {
            let g = f.zero_yields.iter().find(|p| p.years == *n)?.rate;
            let mat = curve::add_months(day, 12 * *n as i64, false);
            Some((*n, g, nss.zero((mat - settle) as f64 / 365.0)))
        })
        .collect();
    Some((table, f.date))
}

/// The GSW table for the last GSW row on or before `date`. The Board publishes with a delay of
/// some days. If the row is for an earlier day, our curve of that day is fitted from the cached
/// price file of that day (so that the two curves are for the same day); without such a file, no table.
fn gsw_for(ds: &crate::forwards::Dataset, cache: &std::path::Path, date: &str, a: &Analysis) -> Option<(Vec<(u32, f64, f64)>, Option<String>)> {
    let day = curve::parse_iso(date)?;
    let g = crate::forwards::forwards_on(ds, date, day).ok()?;
    let gday = curve::parse_iso(&g.date)?;
    let nss = if g.date == date {
        a.nss
    } else {
        let quotes = cached_quotes(cache, &g.date).ok()?;
        analyse(&g.date, curve::add_business_days(gday, 1), &quotes).ok()?.nss
    };
    let (table, gdate) = gsw_table(&nss, ds, &g.date, gday, curve::add_business_days(gday, 1))?;
    Some((table, Some(gdate)))
}

fn parse_args(args: &[String]) -> Result<(Option<String>, Option<std::path::PathBuf>), String> {
    let (mut date, mut out) = (None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--date" => date = Some(it.next().ok_or("--date needs a value")?.clone()),
            "--out" => out = Some(std::path::PathBuf::from(it.next().ok_or("--out needs a value")?)),
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    if let Some(d) = &date {
        if curve::parse_iso(d).is_none() {
            return Err(format!("'{d}' is not a date (YYYY-MM-DD)"));
        }
    }
    Ok((date, out))
}

/// The quotes of a cached day. A missing file is an error with the hint for the cache.
pub fn cached_quotes(cache: &std::path::Path, date: &str) -> Result<Vec<Quote>, String> {
    let path = cache.join("prices").join(format!("fedinvest_{date}.csv"));
    let text = std::fs::read_to_string(&path).map_err(|_| {
        format!(
            "no price file {} (get it with: fa recon bonds --date <date>; see hidden assumption 69)",
            path.display()
        )
    })?;
    let quotes = crate::prices::parse_csv(&text);
    if !crate::prices::has_end_of_day(&quotes) {
        return Err(format!("{} has no end-of-day prices", path.display()));
    }
    Ok(quotes)
}

pub fn run_rich_cheap(fa: &crate::Fa, args: &[String]) -> Result<(), String> {
    let (date, out) = parse_args(args)?;
    let cache = fa.repo.join("data/raw");
    let date = match date {
        Some(d) => d,
        None => latest_cached(&cache).ok_or("no cached price file in data/raw/prices")?,
    };
    let day = curve::parse_iso(&date).ok_or("bad date")?;
    let settle = curve::add_business_days(day, 1);
    let quotes = cached_quotes(&cache, &date)?;
    let a = analyse(&date, settle, &quotes)?;
    let (table, gsw_date) = match crate::forwards::load(&fa.repo) {
        Ok(ds) => gsw_for(&ds, &cache, &date, &a).unwrap_or_else(|| (Vec::new(), None)),
        Err(e) => {
            eprintln!("fa recon: GSW comparison skipped: {e}");
            (Vec::new(), None)
        }
    };
    print!("{}", report(&a, &table, gsw_date.as_deref()));
    let path = out.unwrap_or_else(|| fa.repo.join("data/recon").join(format!("richcheap_{date}.csv")));
    write_csv(&path, &a)?;
    println!("\nWrote {} rows to {}", a.rows.len(), path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::{add_business_days, parse_iso};

    fn row(kind: &'static str, years: f64) -> Row {
        Row {
            id: "X".into(),
            kind,
            coupon: Some(0.04),
            maturity: curve::days_from_civil(2030, 1, 1),
            years,
            obs_clean: 100.0,
            buy: None,
            sell: None,
            model_clean: 100.0,
            y_obs: 0.04,
            y_model: 0.04,
            d_mod: 5.0,
            res_bp: 0.0,
            z: 0.0,
            in_fit: true,
            ba_bp: 1.0,
            ba_source: "file",
            cost_bp: 1.0,
            noise_bp: 0.0,
            group: "other",
            near_coupon: false,
        }
    }

    #[test]
    fn groups_follow_the_rule_order() {
        assert_eq!(classify(&row("note", 0.2), 0.0), "under 3 months");
        assert_eq!(classify(&row("note", 9.8), 0.0), "recent issue"); // 10-year term, 0.2 y old
        assert_eq!(classify(&row("note", 9.5), 0.0), "other");
        assert_eq!(classify(&row("bond", 29.9), 0.0), "recent issue");
        // The same life with a coupon far from the yield is an old security, not a new issue.
        let mut old = row("note", 3.0);
        old.coupon = Some(0.0175);
        assert_eq!(classify(&old, -0.02), "low coupon");
        let mut thirty = row("bond", 12.0);
        thirty.maturity = curve::days_from_civil(2038, 2, 15);
        assert_eq!(classify(&thirty, 0.0), "old 30-year bond");
        thirty.maturity = curve::days_from_civil(2046, 2, 15); // a 20-year bond of 2026 would mature in 2046 or later
        assert_eq!(classify(&thirty, 0.0), "other");
        assert_eq!(classify(&row("note", 4.0), 0.02), "high coupon");
        assert_eq!(classify(&row("note", 4.0), -0.02), "low coupon");
        let mut r = row("note", 4.0);
        r.near_coupon = true;
        assert_eq!(classify(&r, 0.0), "near coupon date");
        // A note is not a recent issue at the 20-year term.
        assert_eq!(classify(&row("note", 19.9), 0.0), "other");
    }

    #[test]
    fn survives_needs_size_z_and_life() {
        let mut r = row("note", 5.0);
        r.res_bp = 3.0;
        r.cost_bp = 1.0;
        r.z = 2.5;
        assert!(r.survives());
        r.cost_bp = 3.5;
        assert!(!r.survives()); // the cost is larger
        r.cost_bp = 1.0;
        r.noise_bp = 3.2;
        assert!(!r.survives()); // the fit error of the day is larger
        r.noise_bp = 0.0;
        r.cost_bp = 1.0;
        r.z = 1.9;
        assert!(!r.survives()); // not far from the neighbours
        r.z = -2.5;
        r.res_bp = -3.0;
        assert!(r.survives());
        assert_eq!(r.side(), "rich");
        r.years = 0.2;
        assert!(!r.survives());
    }

    #[test]
    fn file_spread_is_buy_minus_sell() {
        let mut q = Quote {
            cusip: "A".into(),
            security_type: "MARKET BASED NOTE".into(),
            coupon: 0.04,
            maturity: 1,
            call_date: None,
            buy: 100.03125,
            sell: 100.0,
            end_of_day: 100.0,
        };
        assert_eq!(file_spread(&q), Some(0.03125));
        q.buy = 0.0;
        assert_eq!(file_spread(&q), None);
        q.buy = 99.9; // buy below sell: not a bid-ask
        assert_eq!(file_spread(&q), None);
    }

    /// A market on a known curve with one security 2 points rich and one 1 point cheap: the
    /// analysis finds them, with the right signs, and the costs and groups are set.
    #[test]
    fn finds_planted_rich_and_cheap_securities() {
        let settle = add_business_days(parse_iso("2026-11-17").unwrap(), 1);
        let truth = Nss { b0: 0.05, b1: -0.01, b2: 0.01, b3: 0.0, tau1: 2.0, tau2: 8.0 };
        let mut text = String::new();
        for i in 0..120_i64 {
            let maturity = curve::add_months(settle, 9 + i * 3, false);
            let coupon = 0.03 + 0.0005 * (i % 8) as f64;
            let bond = Bond { coupon, maturity };
            let s = bond.schedule(settle);
            let last = s.next.len() - 1;
            let dirty: f64 = s
                .next
                .iter()
                .enumerate()
                .map(|(k, d)| (100.0 * coupon / 2.0 + if k == last { 100.0 } else { 0.0 }) * truth.df((*d - settle) as f64 / 365.0))
                .sum();
            let mut clean = dirty - bond.accrued(settle);
            if i == 50 {
                clean += 0.9; // rich
            }
            if i == 80 {
                clean -= 0.9; // cheap
            }
            let (y, m, d) = curve::civil_from_days(maturity);
            let _ = writeln!(text, "9128{i:05},MARKET BASED NOTE,{coupon},{m:02}/{d:02}/{y},,{:.6},{:.6},{:.6}", clean + 0.0156, clean, clean);
        }
        let quotes = crate::prices::parse_csv(&text);
        let a = analyse("2026-11-17", settle, &quotes).unwrap();
        let rich = a.rows.iter().find(|r| r.id == "912800050").unwrap();
        let cheap = a.rows.iter().find(|r| r.id == "912800080").unwrap();
        assert!(rich.res_bp < -3.0 && rich.survives() && rich.side() == "rich", "{:?}", rich.res_bp);
        assert!(cheap.res_bp > 3.0 && cheap.survives() && cheap.side() == "cheap", "{:?}", cheap.res_bp);
        assert!(!rich.in_fit && !cheap.in_fit);
        // The other securities sit on the curve (to the size of the 1/128 rounding).
        let others = a.rows.iter().filter(|r| r.id != rich.id && r.id != cheap.id);
        assert!(others.clone().all(|r| r.res_bp.abs() < 1.0), "max {}", others.map(|r| r.res_bp.abs()).fold(0.0, f64::max));
        assert_eq!(a.rows.iter().filter(|r| r.survives()).count(), 2);
        // The file gave BUY - SELL = 0.0156 for each row.
        assert!(a.rows.iter().all(|r| r.ba_source == "file"));
        let text = report(&a, &[], None);
        assert!(text.contains("CHEAP CANDIDATES") && text.contains("912800080"));
        assert_eq!(csv_row(rich, "2026-11-17", settle).split(',').count(), CSV_HEADER.split(',').count());
    }
}
