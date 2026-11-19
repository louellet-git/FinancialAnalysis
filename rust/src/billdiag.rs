//! `fa recon bills`: test each candidate cause of the bill difference of `fa recon bonds`.
//!
//! Each row of the table is one change to the comparison. The column "mean bp" is the mean of
//! observed minus theoretical bill yield. The change that explains the difference makes the
//! number go to 0. A change that does not explain it leaves the number as it is.
//!
//! The Treasury also publishes the "Daily Treasury Bill Rates" (bank discount and coupon
//! equivalent of the on-the-run 4, 6, 8, 13, 17, 26 and 52 week bills). The table is the
//! independent check of the bill inputs of the par curve.

use crate::curve::{self, BillBasis, Bootstrapped, Day};
use crate::prices::Quote;
use crate::recon::{self, Row};
use crate::Fa;
use std::fmt::Write as _;

pub const BILL_URL: &str = "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/daily-treasury-rates.csv";

/// The bill tenors of the par curve and their columns in the Treasury bill-rate file.
pub const BILL_NODES: [(&str, &str, i64); 6] = [
    ("1M", "4 WEEKS", 28),
    ("1.5M", "6 WEEKS", 42),
    ("2M", "8 WEEKS", 56),
    ("3M", "13 WEEKS", 91),
    ("4M", "17 WEEKS", 119),
    ("6M", "26 WEEKS", 182),
];

/// One day of the Treasury bill-rate file: (column prefix, bank discount, coupon equivalent), decimals.
#[derive(Debug, Clone, PartialEq)]
pub struct BillRates {
    pub date: Day,
    pub rates: Vec<(String, f64, f64)>,
}

/// Parse the file. The header has `"4 WEEKS BANK DISCOUNT","4 WEEKS COUPON EQUIVALENT",...`.
pub fn parse_bill_rates(text: &str) -> Vec<BillRates> {
    let mut lines = text.trim_start_matches('\u{feff}').lines().filter(|l| !l.trim().is_empty());
    let Some(head) = lines.next() else { return Vec::new() };
    let cols: Vec<String> = head.split(',').map(|c| c.trim().trim_matches('"').to_string()).collect();
    let mut out = Vec::new();
    for line in lines {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != cols.len() {
            continue;
        }
        let Some(date) = curve::parse_us(f[0].trim().trim_matches('"')) else { continue };
        let mut rates = Vec::new();
        for (i, c) in cols.iter().enumerate() {
            if let Some(prefix) = c.strip_suffix(" BANK DISCOUNT") {
                let ce_col = format!("{prefix} COUPON EQUIVALENT");
                let (Some(j), Ok(d)) = (cols.iter().position(|x| *x == ce_col), f[i].trim().parse::<f64>()) else { continue };
                if let Ok(ce) = f[j].trim().parse::<f64>() {
                    rates.push((prefix.to_string(), d / 100.0, ce / 100.0));
                }
            }
        }
        out.push(BillRates { date, rates });
    }
    out
}

/// The file of a year from the cache, or from the Treasury (one request).
pub fn load_bill_rates(cache: &std::path::Path, year: i64) -> Result<Vec<BillRates>, String> {
    let path = cache.join("rates").join(format!("treasury_bill_rates_{year}.csv"));
    if let Ok(t) = std::fs::read_to_string(&path) {
        let v = parse_bill_rates(&t);
        if !v.is_empty() {
            return Ok(v);
        }
    }
    let url = format!("{BILL_URL}/{year}/all?type=daily_treasury_bill_rates&field_tdr_date_value={year}&page&_format=csv");
    let mut client = mapcheck::http::Client::new("FinancialAnalysis fa (personal research)", 1.0);
    let body = client.get(&url).map_err(|e| e.to_string())?;
    let text = String::from_utf8(body).map_err(|_| "bill-rate file is not UTF-8")?;
    let v = parse_bill_rates(&text);
    if v.is_empty() {
        return Err("the bill-rate file has no rows".into());
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, text);
    Ok(v)
}

/// The mean of observed minus theoretical (bp) for the bills in three maturity groups and for all.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub label: String,
    pub n: usize,
    pub up_to_3m: f64,
    pub m3_to_6m: f64,
    pub over_6m: f64,
    pub all: f64,
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { f64::NAN } else { v.iter().sum::<f64>() / v.len() as f64 }
}

/// Summarize the bill rows. `base` is the settlement day (to measure the days).
pub fn line(label: &str, rows: &[Row], base: Day) -> Line {
    let bills: Vec<&Row> = rows.iter().filter(|r| r.kind == "bill").collect();
    let pick = |lo: i64, hi: i64| -> Vec<f64> {
        bills.iter().filter(|r| (r.maturity - base) > lo && (r.maturity - base) <= hi).map(|r| r.diff_bp()).collect()
    };
    let all: Vec<f64> = bills.iter().map(|r| r.diff_bp()).collect();
    Line {
        label: label.to_string(),
        n: bills.len(),
        up_to_3m: mean(&pick(0, 91)),
        m3_to_6m: mean(&pick(91, 182)),
        over_6m: mean(&pick(182, 10_000)),
        all: mean(&all),
    }
}

fn with_price(quotes: &[Quote], field: &str) -> Vec<Quote> {
    quotes
        .iter()
        .map(|q| {
            let mut q = q.clone();
            let p = match field {
                "sell" => q.sell,
                "buy" => q.buy,
                _ => q.end_of_day,
            };
            q.end_of_day = if p > 0.0 { p } else { 0.0 };
            q
        })
        .collect()
}

fn par_list(par: &crate::Curve) -> Vec<(String, f64)> {
    par.ordered().into_iter().map(|(t, y)| (t.to_string(), y)).collect()
}

/// The par list with the bill nodes replaced by the coupon-equivalent bill rates of the Treasury table.
pub fn replace_bill_nodes(par: &[(String, f64)], day: &BillRates) -> Vec<(String, f64)> {
    par.iter()
        .map(|(label, y)| {
            let col = BILL_NODES.iter().find(|n| n.0 == label).map(|n| n.1);
            match col.and_then(|c| day.rates.iter().find(|r| r.0 == c)) {
                Some(r) => (label.clone(), r.2),
                None => (label.clone(), *y),
            }
        })
        .collect()
}

fn boot(day: Day, par: &[(String, f64)], basis: BillBasis) -> Result<Bootstrapped, String> {
    curve::bootstrap_with(day, par, basis).map_err(|e| e.to_string())
}

pub fn run_bills(fa: &Fa, args: &[String]) -> Result<(), String> {
    let mut date = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--date" => date = Some(it.next().ok_or("--date needs a value")?.clone()),
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    let cache = fa.repo.join("data/raw");
    let (par, quotes) = recon::gather(fa, date.as_deref(), &cache)?;
    let day = curve::parse_iso(&par.date).ok_or("bad curve date")?;
    let t1 = curve::add_business_days(day, 1);
    let t2 = curve::add_business_days(day, 2);
    let nodes = par_list(&par);
    let table = load_bill_rates(&cache, curve::civil_from_days(day).0);
    let bill_day = table.as_ref().ok().and_then(|t| t.iter().find(|b| b.date == day).cloned());

    let mut lines: Vec<Line> = Vec::new();
    let mut add = |label: &str, b: &Bootstrapped, settle: Day, q: &[Quote]| {
        let (rows, _) = recon::build_rows(b, settle, q);
        lines.push(line(label, &rows, settle));
    };
    let base = boot(day, &nodes, BillBasis::Bey365)?;
    add("0 baseline: par nodes as BEY/365, T+1, END OF DAY", &base, t1, &quotes);
    for (label, basis) in [
        ("a1 bill nodes as money-market yield (ACT/360)", BillBasis::Mm360),
        ("a2 bill nodes as bank discount rate", BillBasis::Discount360),
        ("a3 bill nodes as semi-annual rate", BillBasis::SemiAnnual),
    ] {
        add(label, &boot(day, &nodes, basis)?, t1, &quotes);
    }
    add("b1 settlement T+0 (price date)", &base, day, &quotes);
    add("b2 settlement T+2", &boot(day, &nodes, BillBasis::Bey365)?, t2, &quotes);
    // The BUY and SELL columns are quotes of the price date (T+0 discount rates in steps of 0.5 bp).
    add("c1 price field SELL (bid side), T+0", &base, day, &with_price(&quotes, "sell"));
    add("c2 price field BUY (ask side), T+0", &base, day, &with_price(&quotes, "buy"));
    let mut node_text = String::new();
    if let Some(bd) = &bill_day {
        let alt = replace_bill_nodes(&nodes, bd);
        add("d bill nodes from the Treasury bill-rate table (coupon equivalent)", &boot(day, &alt, BillBasis::Bey365)?, t1, &quotes);
        let _ = writeln!(node_text, "\nBILL NODES: par curve against the Treasury bill-rate table of the same day (coupon equivalent, bp)");
        let _ = writeln!(node_text, "{:<6} {:>9} {:>11} {:>9}", "node", "par curve", "bill table", "par-table");
        for (label, col, _) in BILL_NODES {
            let p = nodes.iter().find(|n| n.0 == label).map(|n| n.1);
            let b = bd.rates.iter().find(|r| r.0 == col).map(|r| r.2);
            if let (Some(p), Some(b)) = (p, b) {
                let _ = writeln!(node_text, "{label:<6} {:>8.2}% {:>10.2}% {:>+9.1}", p * 100.0, b * 100.0, (p - b) * 1e4);
            }
        }
    }

    println!("Bill difference tests for {} (curve date). Mean of observed minus theoretical bill yield, bp; n = {} bills.", par.date, lines[0].n);
    println!("{:<72} {:>7} {:>7} {:>7} {:>7}", "change", "<=3M", "3-6M", ">6M", "all");
    for l in &lines {
        println!("{:<72} {:>+7.1} {:>+7.1} {:>+7.1} {:>+7.1}", l.label, l.up_to_3m, l.m3_to_6m, l.over_6m, l.all);
    }
    print!("{node_text}");
    match (&table, &bill_day) {
        (Err(e), _) => println!("\nThe Treasury bill-rate table could not be loaded: {e}"),
        (Ok(_), None) => println!("\nThe Treasury bill-rate table has no row for {}.", par.date),
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::parse_iso;

    const SAMPLE: &str = "Date,\"4 WEEKS BANK DISCOUNT\",\"4 WEEKS COUPON EQUIVALENT\",\"13 WEEKS BANK DISCOUNT\",\"13 WEEKS COUPON EQUIVALENT\"\n11/17/2026,3.92,3.99,4.15,4.25\n11/16/2026,3.95,4.02,,\n";

    #[test]
    fn parses_the_bill_rate_file() {
        let v = parse_bill_rates(SAMPLE);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].date, parse_iso("2026-11-17").unwrap());
        assert_eq!(v[0].rates.len(), 2);
        assert_eq!(v[0].rates[0].0, "4 WEEKS");
        assert!((v[0].rates[0].1 - 0.0392).abs() < 1e-12 && (v[0].rates[0].2 - 0.0399).abs() < 1e-12);
        assert!((v[0].rates[1].1 - 0.0415).abs() < 1e-12 && (v[0].rates[1].2 - 0.0425).abs() < 1e-12);
        assert_eq!(v[1].rates.len(), 1); // the empty 13-week fields are skipped
    }

    /// The Treasury's own table is the proof of the bill conventions: a 4-week bank discount rate of
    /// 3.92 % is a coupon equivalent of 3.99 %; a 13-week rate of 4.15 % is 4.25 %.
    #[test]
    fn discount_rate_to_coupon_equivalent_matches_the_treasury_table() {
        let price = curve::bill_price_from_discount(0.0392, 28);
        assert!((price - 99.695111).abs() < 1e-6);
        assert!((curve::bill_yield(price, 28) - 0.0399).abs() < 5e-5);
        let p13 = curve::bill_price_from_discount(0.0415, 91);
        assert!((p13 - 98.950972).abs() < 1e-6);
        assert!((curve::bill_yield(p13, 91) - 0.0425).abs() < 5e-5);
        // FedInvest 2026-11-17, bill 912797WF0: discount 3.92 % for 27 days from the settlement 2026-11-18.
        let p = curve::bill_price_from_discount(0.0392, 27);
        assert!((p - 99.706).abs() < 1e-9);
        assert!((curve::bill_yield(p, 27) - 0.039862).abs() < 5e-6);
    }

    #[test]
    fn bill_basis_changes_the_zero_rate_by_the_expected_amount() {
        // 4 % for 91 days. BEY/365 is the reference.
        let bey = curve::bill_node_zero(0.04, 91, BillBasis::Bey365);
        let mm = curve::bill_node_zero(0.04, 91, BillBasis::Mm360);
        let disc = curve::bill_node_zero(0.04, 91, BillBasis::Discount360);
        let semi = curve::bill_node_zero(0.04, 91, BillBasis::SemiAnnual);
        // ACT/360 gives a zero rate higher by about 4 % x 5/360 = 5.5 bp; a discount rate gives about 10 bp more.
        assert!((mm - bey) * 1e4 > 5.0 && (mm - bey) * 1e4 < 6.0, "{}", (mm - bey) * 1e4);
        assert!((disc - bey) * 1e4 > 8.0 && (disc - bey) * 1e4 < 12.0);
        assert!((semi - 2.0 * (1.02_f64).ln()).abs() < 1e-15 && (semi - bey).abs() < 3e-4);
        // The default bootstrap is the BEY basis.
        let par: Vec<(String, f64)> = vec![("1M".into(), 0.04), ("3M".into(), 0.04)];
        let a = curve::bootstrap(parse_iso("2026-11-17").unwrap(), &par).unwrap();
        let b = curve::bootstrap_with(parse_iso("2026-11-17").unwrap(), &par, BillBasis::Bey365).unwrap();
        assert_eq!(a.nodes[1].zero, b.nodes[1].zero);
    }

    #[test]
    fn replaces_only_the_bill_nodes() {
        let par: Vec<(String, f64)> = vec![("1M".into(), 0.0409), ("3M".into(), 0.0433), ("1Y".into(), 0.046)];
        let day = BillRates { date: 0, rates: vec![("4 WEEKS".into(), 0.0392, 0.0399), ("13 WEEKS".into(), 0.0415, 0.0425)] };
        let alt = replace_bill_nodes(&par, &day);
        assert_eq!(alt, vec![("1M".to_string(), 0.0399), ("3M".to_string(), 0.0425), ("1Y".to_string(), 0.046)]);
    }
}
