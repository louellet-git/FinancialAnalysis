//! `fa recon bonds`: compare the value that our rates give with the observed market
//! value of simple bonds, for one date.
//!
//! Steps: (1) get the Treasury par curve for the date through the Python interface,
//! (2) bootstrap it to a zero curve (`curve.rs`), (3) get the observed end-of-day
//! prices of Treasury bills, notes and bonds (`prices.rs`), (4) price each instrument
//! on the curve, (5) list the differences. Difference = observed minus theoretical.
//!
//! The command lists differences. It does not explain them.

use crate::curve::{self, Bond, Bootstrapped, Day};
use crate::prices::{self, Quote};
use crate::{Fa, FaError};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

/// One instrument and its difference.
#[derive(Debug, Clone)]
pub struct Row {
    pub id: String,
    pub kind: &'static str,
    pub coupon: Option<f64>,
    pub maturity: Day,
    pub years: f64,
    pub obs_price: Option<f64>,
    pub obs_yield: f64,
    pub theo_price: Option<f64>,
    pub theo_yield: f64,
    pub accrued: Option<f64>,
    pub df_settle: f64,
    /// The yield change (bp) that one price step of the source makes: the resolution of `diff_bp`.
    pub res_bp: f64,
    pub buy: Option<f64>,
    pub sell: Option<f64>,
    pub note: String,
}

impl Row {
    pub fn diff_price(&self) -> Option<f64> {
        Some(self.obs_price? - self.theo_price?)
    }
    /// Basis points.
    pub fn diff_bp(&self) -> f64 {
        (self.obs_yield - self.theo_yield) * 1e4
    }
}

/// What the filter did.
#[derive(Debug, Default, PartialEq)]
pub struct Skipped {
    pub no_price: usize,
    pub matured: usize,
    /// Fewer than 7 days to maturity: the price step is larger than any real yield information.
    pub too_short: usize,
    pub tips_frn: usize,
    pub callable: usize,
    pub no_yield: usize,
}

fn nonzero(x: f64) -> Option<f64> {
    if x > 0.0 { Some(x) } else { None }
}

/// Price each bill, note and bond of `quotes` on the curve.
pub fn build_rows(boot: &Bootstrapped, settle: Day, quotes: &[Quote]) -> (Vec<Row>, Skipped) {
    let curve = &boot.curve;
    let mut rows = Vec::new();
    let mut skip = Skipped::default();
    for q in quotes {
        let kind = match q.security_type.as_str() {
            "MARKET BASED BILL" => "bill",
            "MARKET BASED NOTE" => "note",
            "MARKET BASED BOND" => "bond",
            _ => {
                skip.tips_frn += 1;
                continue;
            }
        };
        if q.call_date.is_some() {
            skip.callable += 1;
            continue;
        }
        if q.end_of_day <= 0.0 {
            skip.no_price += 1;
            continue;
        }
        if q.maturity <= settle {
            skip.matured += 1;
            continue;
        }
        if q.maturity - settle < 7 {
            skip.too_short += 1;
            continue;
        }
        let years = (q.maturity - settle) as f64 / 365.25;
        let df_settle = curve.df_on(settle);
        if kind == "bill" {
            let days = q.maturity - settle;
            let theo = curve::bill_price(curve, settle, q.maturity);
            rows.push(Row {
                id: q.cusip.clone(),
                kind,
                coupon: None,
                maturity: q.maturity,
                years,
                obs_price: Some(q.end_of_day),
                obs_yield: curve::bill_yield(q.end_of_day, days),
                theo_price: Some(theo),
                theo_yield: curve::bill_yield(theo, days),
                accrued: None,
                df_settle,
                res_bp: (curve::bill_yield(q.end_of_day - 5e-7, days) - curve::bill_yield(q.end_of_day + 5e-7, days)).abs() * 1e4,
                buy: nonzero(q.buy),
                sell: nonzero(q.sell),
                note: String::new(),
            });
        } else {
            if q.coupon <= 0.0 {
                skip.tips_frn += 1;
                continue;
            }
            let bond = Bond { coupon: q.coupon, maturity: q.maturity };
            let theo = bond.clean_from_curve(curve, settle);
            let (Some(oy), Some(ty)) = (bond.yield_from_clean(q.end_of_day, settle), bond.yield_from_clean(theo, settle)) else {
                skip.no_yield += 1;
                continue;
            };
            rows.push(Row {
                id: q.cusip.clone(),
                kind,
                coupon: Some(q.coupon),
                maturity: q.maturity,
                years,
                obs_price: Some(q.end_of_day),
                obs_yield: oy,
                theo_price: Some(theo),
                theo_yield: ty,
                accrued: Some(bond.accrued(settle)),
                df_settle,
                // The source gives notes and bonds in 64ths of a point.
                res_bp: match (bond.yield_from_clean(q.end_of_day - 1.0 / 128.0, settle), bond.yield_from_clean(q.end_of_day + 1.0 / 128.0, settle)) {
                    (Some(a), Some(b)) => (a - b).abs() * 1e4,
                    _ => f64::NAN,
                },
                buy: nonzero(q.buy),
                sell: nonzero(q.sell),
                note: String::new(),
            });
        }
    }
    (rows, skip)
}

/// The maturity buckets of the summary.
pub fn bucket(years: f64) -> &'static str {
    const B: [(f64, &str); 9] = [
        (0.25, "<3M"),
        (0.5, "3-6M"),
        (1.0, "6-12M"),
        (2.0, "1-2Y"),
        (3.0, "2-3Y"),
        (5.0, "3-5Y"),
        (7.0, "5-7Y"),
        (10.0, "7-10Y"),
        (20.0, "10-20Y"),
    ];
    B.iter().find(|(limit, _)| years < *limit).map(|(_, l)| *l).unwrap_or(">=20Y")
}

pub const BUCKET_ORDER: [&str; 10] = ["<3M", "3-6M", "6-12M", "1-2Y", "2-3Y", "3-5Y", "5-7Y", "7-10Y", "10-20Y", ">=20Y"];

#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub n: usize,
    pub mean_price: Option<f64>,
    pub max_abs_price: Option<(f64, String)>,
    pub mean_bp: f64,
    pub mean_abs_bp: f64,
    pub max_abs_bp: (f64, String),
}

pub fn stats(rows: &[&Row]) -> Option<Stats> {
    if rows.is_empty() {
        return None;
    }
    let n = rows.len() as f64;
    let prices: Vec<(f64, &str)> = rows.iter().filter_map(|r| Some((r.diff_price()?, r.id.as_str()))).collect();
    let max_p = prices
        .iter()
        .max_by(|a, b| a.0.abs().total_cmp(&b.0.abs()))
        .map(|(v, id)| (*v, id.to_string()));
    let max_b = rows
        .iter()
        .max_by(|a, b| a.diff_bp().abs().total_cmp(&b.diff_bp().abs()))
        .map(|r| (r.diff_bp(), r.id.clone()))
        .unwrap();
    Some(Stats {
        n: rows.len(),
        mean_price: if prices.is_empty() { None } else { Some(prices.iter().map(|p| p.0).sum::<f64>() / prices.len() as f64) },
        max_abs_price: max_p,
        mean_bp: rows.iter().map(|r| r.diff_bp()).sum::<f64>() / n,
        mean_abs_bp: rows.iter().map(|r| r.diff_bp().abs()).sum::<f64>() / n,
        max_abs_bp: max_b,
    })
}

fn stats_line(label: &str, s: &Stats) -> String {
    let p = |v: Option<f64>| v.map_or("      -".to_string(), |x| format!("{x:+7.3}"));
    let mp = s.max_abs_price.as_ref().map_or("      -".to_string(), |(v, _)| format!("{v:+7.3}"));
    format!(
        "{label:<10} {:>4}  {}  {}  {:+8.2}  {:8.2}  {:+8.2} {}",
        s.n,
        p(s.mean_price),
        mp,
        s.mean_bp,
        s.mean_abs_bp,
        s.max_abs_bp.0,
        s.max_abs_bp.1
    )
}

/// The summary text: by type and by maturity bucket.
pub fn summary(rows: &[Row]) -> String {
    let mut out = String::new();
    let head = format!(
        "{:<10} {:>4}  {:>7}  {:>7}  {:>8}  {:>8}  {:>8} {}\n",
        "group", "n", "mean dP", "max dP", "mean bp", "mean|bp|", "max bp", "at"
    );
    let _ = writeln!(out, "Difference = observed - theoretical. dP per 100 face (clean price); bp = yield, basis points.");
    let _ = writeln!(out, "BY TYPE\n{head}");
    for kind in ["bill", "note", "bond", "hqm"] {
        let g: Vec<&Row> = rows.iter().filter(|r| r.kind == kind).collect();
        if let Some(s) = stats(&g) {
            let _ = writeln!(out, "{}", stats_line(kind, &s));
        }
    }
    let _ = writeln!(out, "\nBY MATURITY (bills, notes and bonds; years from settlement)\n{head}");
    let treasuries: Vec<&Row> = rows.iter().filter(|r| r.kind != "hqm").collect();
    for b in BUCKET_ORDER {
        let g: Vec<&Row> = treasuries.iter().copied().filter(|r| bucket(r.years) == b).collect();
        if let Some(s) = stats(&g) {
            let _ = writeln!(out, "{}", stats_line(b, &s));
        }
    }
    if let Some(s) = stats(&treasuries) {
        let _ = writeln!(out, "{}", stats_line("all", &s));
    }
    out
}

fn money(v: Option<f64>) -> String {
    v.map_or(String::new(), |x| format!("{x:.6}"))
}

pub const CSV_HEADER: &str = "id,type,coupon,maturity,years,price_date,settle_date,curve_date,obs_clean,obs_yield,\
theo_clean,theo_yield,diff_price,diff_yield_bp,resolution_bp,accrued,df_settle,buy,sell,note";

pub fn csv_row(r: &Row, price_date: &str, settle: Day, curve_date: Day) -> String {
    format!(
        "{},{},{},{},{:.4},{},{},{},{},{:.6},{},{:.6},{},{:.3},{:.2},{},{:.8},{},{},{}",
        r.id,
        r.kind,
        r.coupon.map_or(String::new(), |c| format!("{c:.6}")),
        curve::iso(r.maturity),
        r.years,
        price_date,
        curve::iso(settle),
        curve::iso(curve_date),
        money(r.obs_price),
        r.obs_yield,
        money(r.theo_price),
        r.theo_yield,
        money(r.diff_price()),
        r.diff_bp(),
        r.res_bp,
        money(r.accrued),
        r.df_settle,
        money(r.buy),
        money(r.sell),
        r.note.replace(',', ";"),
    )
}

/// The curve-level comparison with the corporate rates. Each row compares the HQM spot
/// rate of a maturity with (our Treasury zero rate + the mean ICE OAS of AAA, AA, A).
pub fn hqm_rows(boot: &Bootstrapped, fa: &Fa, date: &str) -> Result<Vec<Row>, FaError> {
    let hqm = fa.hqm_curve(Some(date))?;
    let mut oas = Vec::new();
    for grade in ["AAA", "AA", "A"] {
        oas.push((grade, fa.credit_spread(grade, Some(date))?));
    }
    let mean_oas = oas.iter().map(|(_, o)| o.value).sum::<f64>() / oas.len() as f64;
    let oas_text: Vec<String> = oas.iter().map(|(g, o)| format!("{g}={:.4}@{}", o.value, o.date)).collect();
    let mut rows = Vec::new();
    for (label, spot) in hqm.ordered() {
        let Some(years) = label.trim_end_matches('Y').parse::<i64>().ok() else { continue };
        let maturity = curve::add_months(boot.curve.date, 12 * years, false);
        let t = curve::year_fraction(boot.curve.date, maturity);
        let zero_sa = curve::continuous_to_semi(boot.curve.zero(t));
        rows.push(Row {
            id: format!("HQM-{label}"),
            kind: "hqm",
            coupon: None,
            maturity,
            years: years as f64,
            obs_price: None,
            obs_yield: spot,
            theo_price: None,
            theo_yield: zero_sa + mean_oas,
            accrued: None,
            df_settle: boot.curve.df_on(boot.curve.date),
            res_bp: 0.5, // HQM is published with 2 decimals of a percent: 1 bp steps
            buy: None,
            sell: None,
            note: format!(
                "hqm_date={} (monthly average); ust_zero_semi={:.6}; implied_spread_bp={:.1}; mean_oas={:.4} of {}",
                hqm.date,
                zero_sa,
                (spot - zero_sa) * 1e4,
                mean_oas,
                oas_text.join(" ")
            ),
        });
    }
    Ok(rows)
}

pub const HELP: &str = "\
fa recon bonds [--date YYYY-MM-DD] [--out FILE]

Compare the value that our rates give with the market value of simple bonds.
It gets the Treasury par curve of the date, builds a zero curve, and prices each
Treasury bill, note and bond that has an end-of-day price (TreasuryDirect FedInvest).
It lists observed minus theoretical, in price (per 100) and in yield (bp).
It also compares the HQM corporate spot curve with our zero curve plus the ICE spread.

  --date  A business day. Default: the latest day that has curve and end-of-day prices.
  --out   The CSV file. Default: data/recon/bonds_<date>.csv in the repository.
          A second file <name>_curve.csv has the curve nodes.";

fn parse_args(args: &[String]) -> Result<(Option<String>, Option<PathBuf>), String> {
    let (mut date, mut out) = (None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--date" => date = Some(it.next().ok_or("--date needs a value")?.clone()),
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a value")?)),
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

/// The par curve and the prices of one date. Without `date`, step back from the latest curve.
fn gather(fa: &Fa, date: Option<&str>, cache: &std::path::Path) -> Result<(crate::Curve, Vec<Quote>), String> {
    let e = |x: FaError| x.to_string();
    let mut curve = fa.curve(date, false).map_err(e)?;
    for _ in 0..6 {
        if let Some(d) = date {
            if curve.date != d {
                return Err(format!("no Treasury curve for {d} (nearest earlier: {})", curve.date));
            }
        }
        let quotes = prices::load(cache, &curve.date)?;
        if prices::has_end_of_day(&quotes) {
            return Ok((curve, quotes));
        }
        if date.is_some() {
            return Err(format!("FedInvest has no end-of-day prices for {} yet", curve.date));
        }
        let prev = curve::civil_from_days(curve::parse_iso(&curve.date).unwrap() - 1);
        let prev = format!("{:04}-{:02}-{:02}", prev.0, prev.1, prev.2);
        curve = fa.curve(Some(&prev), false).map_err(e)?;
    }
    Err("no day with end-of-day prices in the last 6 days".into())
}

pub fn run(fa: &Fa, args: &[String]) -> ExitCode {
    if matches!(args.first().map(String::as_str), None | Some("help" | "-h" | "--help")) {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if args[0] != "bonds" {
        eprintln!("fa: unknown recon command '{}'\n\n{HELP}", args[0]);
        return ExitCode::from(64);
    }
    match run_bonds(fa, &args[1..]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(m) => {
            eprintln!("fa recon: {m}");
            ExitCode::from(2)
        }
    }
}

fn run_bonds(fa: &Fa, args: &[String]) -> Result<(), String> {
    let (date, out) = parse_args(args)?;
    let cache = fa.repo.join("data/raw");
    let (par, quotes) = gather(fa, date.as_deref(), &cache)?;
    let day = curve::parse_iso(&par.date).ok_or("bad curve date")?;
    let settle = curve::add_business_days(day, 1);
    let ordered: Vec<(String, f64)> = par.ordered().into_iter().map(|(t, y)| (t.to_string(), y)).collect();
    let boot = curve::bootstrap(day, &ordered).map_err(|e| e.to_string())?;

    let (mut rows, skip) = build_rows(&boot, settle, &quotes);
    let mut warnings = Vec::new();
    match hqm_rows(&boot, fa, &par.date) {
        Ok(h) => rows.extend(h),
        Err(e) => warnings.push(format!("corporate curve comparison skipped: {e}")),
    }

    println!("Price date {} | curve {} ({}) | settlement {} (T+1)", par.date, par.date, par.source, curve::iso(settle));
    println!("Prices: TreasuryDirect FedInvest END OF DAY, clean, per 100. Curve: Treasury par curve, monotone convex.");
    println!(
        "Instruments: {} priced. Skipped: {} no price, {} matured, {} under 7 days, {} TIPS/FRN, {} callable, {} no yield.\n",
        rows.iter().filter(|r| r.kind != "hqm").count(),
        skip.no_price,
        skip.matured,
        skip.too_short,
        skip.tips_frn,
        skip.callable,
        skip.no_yield
    );
    println!("{}", summary(&rows));
    let mut top: Vec<&Row> = rows.iter().filter(|r| r.kind != "hqm").collect();
    top.sort_by(|a, b| b.diff_bp().abs().total_cmp(&a.diff_bp().abs()));
    println!("LARGEST 12 YIELD DIFFERENCES (all rows are in the file)");
    println!("{:<10} {:<5} {:>7} {:<10} {:>9} {:>9} {:>8} {:>8} {:>6}", "id", "type", "coupon", "maturity", "obs", "theo", "dP", "bp", "step bp");
    for r in top.iter().take(12) {
        println!(
            "{:<10} {:<5} {:>7} {:<10} {:>9.4} {:>9.4} {:>+8.4} {:>+8.2} {:>6.1}",
            r.id,
            r.kind,
            r.coupon.map_or("-".into(), |c| format!("{:.3}%", c * 100.0)),
            curve::iso(r.maturity),
            r.obs_price.unwrap_or(f64::NAN),
            r.theo_price.unwrap_or(f64::NAN),
            r.diff_price().unwrap_or(f64::NAN),
            r.diff_bp(),
            r.res_bp
        );
    }
    let hqm: Vec<&Row> = rows.iter().filter(|r| r.kind == "hqm").collect();
    if !hqm.is_empty() {
        println!("\nCORPORATE CURVE: HQM spot (semi-annual) vs our Treasury zero (semi-annual) + mean ICE OAS of AAA, AA, A");
        println!("{:<9} {:>8} {:>10} {:>10} {:>10} {:>8}", "maturity", "HQM", "UST zero", "spread bp", "theo", "bp");
        for r in &hqm {
            let zero = r.theo_yield - mean_oas_of(r);
            println!(
                "{:<9} {:>7.3}% {:>9.3}% {:>10.1} {:>9.3}% {:>+8.1}",
                r.id,
                r.obs_yield * 100.0,
                zero * 100.0,
                (r.obs_yield - zero) * 1e4,
                r.theo_yield * 100.0,
                r.diff_bp()
            );
        }
        println!("HQM is a monthly average ({}). Our zero rate is for one day.", hqm[0].note.split(' ').next().unwrap_or(""));
    }
    for w in &warnings {
        println!("\nWARNING: {w}");
    }

    let path = out.unwrap_or_else(|| fa.repo.join("data/recon").join(format!("bonds_{}.csv", par.date)));
    write_files(&path, &rows, &par.date, settle, day, &boot)?;
    println!("\nWrote {} rows to {}", rows.len(), path.display());
    println!("Wrote the curve nodes to {}", curve_path(&path).display());
    Ok(())
}

fn mean_oas_of(r: &Row) -> f64 {
    r.note
        .split("mean_oas=")
        .nth(1)
        .and_then(|s| s.split(' ').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

fn curve_path(path: &std::path::Path) -> PathBuf {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("recon");
    path.with_file_name(format!("{stem}_curve.csv"))
}

fn write_files(path: &std::path::Path, rows: &[Row], price_date: &str, settle: Day, day: Day, boot: &Bootstrapped) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut text = String::from(CSV_HEADER);
    text.push('\n');
    for r in rows {
        text.push_str(&csv_row(r, price_date, settle, day));
        text.push('\n');
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut nodes = String::from("tenor,par_yield,time_years,zero_continuous,zero_semi,discount_factor\n");
    for n in &boot.nodes {
        let _ = writeln!(
            nodes,
            "{},{:.6},{:.6},{:.6},{:.6},{:.8}",
            n.label,
            n.par,
            n.t,
            boot.curve.zero(n.t),
            curve::continuous_to_semi(boot.curve.zero(n.t)),
            boot.curve.df(n.t)
        );
    }
    let cp = curve_path(path);
    std::fs::write(&cp, nodes).map_err(|e| format!("{}: {e}", cp.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::parse_iso;
    use crate::prices::parse_csv;

    fn flat_boot(date: &str, y: f64) -> Bootstrapped {
        let par: Vec<(String, f64)> =
            ["1M", "3M", "6M", "1Y", "2Y", "5Y", "10Y", "30Y"].iter().map(|t| (t.to_string(), y)).collect();
        curve::bootstrap(parse_iso(date).unwrap(), &par).unwrap()
    }

    const SAMPLE: &str = "\
912797AA1,MARKET BASED BILL,0.0,02/18/2027,,0.000000,98.900000,98.900000
912828BB2,MARKET BASED NOTE,0.04,11/15/2030,,0.000000,100.000000,100.000000
912828CC3,MARKET BASED BOND,0.05,11/15/2040,11/15/2035,0.000000,101.000000,101.000000
912828DD4,MARKET BASED NOTE,0.03,11/15/2020,,0.000000,100.000000,100.000000
912828EE5,TIPS,0.01,11/15/2030,,0.000000,100.000000,100.000000
912828FF6,MARKET BASED NOTE,0.03,11/15/2029,,0.000000,0.000000,0.000000
";

    #[test]
    fn filters_and_prices() {
        let boot = flat_boot("2026-11-17", 0.05);
        let settle = curve::add_business_days(boot.curve.date, 1);
        let (rows, skip) = build_rows(&boot, settle, &parse_csv(SAMPLE));
        assert_eq!(rows.len(), 2);
        assert_eq!(skip, Skipped { no_price: 1, matured: 1, too_short: 0, tips_frn: 1, callable: 1, no_yield: 0 });
        let bill = &rows[0];
        assert_eq!(bill.kind, "bill");
        // 92 days: observed yield by hand.
        assert!((bill.obs_yield - (100.0 - 98.9) / 98.9 * 365.0 / 92.0).abs() < 1e-12);
        let note = &rows[1];
        assert_eq!(note.kind, "note");
        // Observed 100.0 at a 4% coupon: the yield is a little above 4% (accrued interest is
        // inside the clean price, so it is not equal to the coupon).
        assert!(note.obs_yield > 0.039 && note.obs_yield < 0.041);
        assert!(note.accrued.unwrap() > 0.0);
        // A priced instrument has the identity: theoretical yield re-gives the theoretical price.
        let bond = Bond { coupon: 0.04, maturity: parse_iso("2030-11-15").unwrap() };
        assert!((bond.clean_from_yield(note.theo_yield, settle) - note.theo_price.unwrap()).abs() < 1e-8);
        assert!((note.diff_price().unwrap() - (100.0 - note.theo_price.unwrap())).abs() < 1e-12);
    }

    #[test]
    fn a_bond_on_its_own_curve_has_no_difference() {
        // Price a par bond from the curve, call that price "observed": the difference is 0.
        let boot = flat_boot("2026-11-17", 0.05);
        let settle = curve::add_business_days(boot.curve.date, 1);
        let bond = Bond { coupon: 0.047, maturity: parse_iso("2033-05-15").unwrap() };
        let clean = bond.clean_from_curve(&boot.curve, settle);
        let q = format!("912828ZZ9,MARKET BASED NOTE,0.047,05/15/2033,,0.0,{clean},{clean}\n");
        let (rows, _) = build_rows(&boot, settle, &parse_csv(&q));
        assert!(rows[0].diff_price().unwrap().abs() < 1e-5); // 6 decimals of the print only
        assert!(rows[0].diff_bp().abs() < 1e-3);
    }

    #[test]
    fn buckets_and_summary() {
        assert_eq!(bucket(0.1), "<3M");
        assert_eq!(bucket(0.25), "3-6M");
        assert_eq!(bucket(4.0), "3-5Y");
        assert_eq!(bucket(25.0), ">=20Y");
        let boot = flat_boot("2026-11-17", 0.05);
        let settle = curve::add_business_days(boot.curve.date, 1);
        let (rows, _) = build_rows(&boot, settle, &parse_csv(SAMPLE));
        let s = summary(&rows);
        assert!(s.contains("bill") && s.contains("note") && s.contains("3-6M") && s.contains("3-5Y"));
        let refs: Vec<&Row> = rows.iter().collect();
        let st = stats(&refs).unwrap();
        assert_eq!(st.n, 2);
        assert!(st.max_abs_bp.0.abs() >= st.mean_abs_bp);
    }

    #[test]
    fn csv_row_has_one_field_for_each_header_column() {
        let boot = flat_boot("2026-11-17", 0.05);
        let settle = curve::add_business_days(boot.curve.date, 1);
        let (rows, _) = build_rows(&boot, settle, &parse_csv(SAMPLE));
        let n = CSV_HEADER.split(',').count();
        for r in &rows {
            assert_eq!(csv_row(r, "2026-11-17", settle, boot.curve.date).split(',').count(), n);
        }
    }

    #[test]
    fn rejects_bad_arguments() {
        assert!(parse_args(&["--date".into(), "2026-13-01".into()]).is_err());
        assert!(parse_args(&["--bogus".into()]).is_err());
        assert!(parse_args(&["--date".into()]).is_err());
        let (d, o) = parse_args(&["--date".into(), "2026-11-17".into(), "--out".into(), "x.csv".into()]).unwrap();
        assert_eq!(d.as_deref(), Some("2026-11-17"));
        assert_eq!(o, Some(PathBuf::from("x.csv")));
    }
}
