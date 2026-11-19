//! Observed prices of U.S. Treasury securities: the end-of-day prices of the
//! Federal Investments Program ("FedInvest"), published by the Bureau of the Fiscal
//! Service on TreasuryDirect.
//!
//! Page: https://www.treasurydirect.gov/GA-FI/FedInvest/selectSecurityPriceDate
//!
//! The file has one row for each market-based security: CUSIP, type, coupon rate
//! (decimal), maturity (MM/DD/YYYY), call date, BUY, SELL, END OF DAY. Prices are per
//! 100 of face value. They are clean prices (no accrued interest): a check in
//! `docs/ARCHITECTURE.md` explains this. A price of 0 means "no price".
//! END OF DAY is not yet filled in on the evening of the business day: the file of
//! that date has 0 in every row. Use the next day.
//!
//! The form needs a session: a GET gives a cookie and a CSRF token, a POST with the
//! date selects the date, a GET of the detail page gives the CSV form (with a new
//! token), and a POST of that form gives the CSV file.

use crate::curve::{parse_us, Day};
use mapcheck::http::Client;
use std::path::Path;

const BASE: &str = "https://www.treasurydirect.gov/GA-FI/FedInvest";
const USER_AGENT: &str = "FinancialAnalysis-recon/0.1 (research; one file for each date)";

#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub cusip: String,
    /// "MARKET BASED BILL", "MARKET BASED NOTE", "MARKET BASED BOND", "MARKET BASED FRN" or "TIPS".
    pub security_type: String,
    pub coupon: f64,
    pub maturity: Day,
    pub call_date: Option<Day>,
    pub buy: f64,
    pub sell: f64,
    pub end_of_day: f64,
}

/// Parse the CSV file. A line that does not have 8 fields, or a rate or a date that does
/// not parse, is skipped (the real file has no header line).
pub fn parse_csv(text: &str) -> Vec<Quote> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.trim().split(',').collect();
            if f.len() < 8 {
                return None;
            }
            let call = f[4].trim();
            Some(Quote {
                cusip: f[0].trim().to_string(),
                security_type: f[1].trim().to_string(),
                coupon: f[2].trim().parse().ok()?,
                maturity: parse_us(f[3])?,
                call_date: if call.is_empty() { None } else { parse_us(call) },
                buy: f[5].trim().parse().ok()?,
                sell: f[6].trim().parse().ok()?,
                end_of_day: f[7].trim().parse().ok()?,
            })
        })
        .collect()
}

/// True when at least one row has an end-of-day price.
pub fn has_end_of_day(quotes: &[Quote]) -> bool {
    quotes.iter().any(|q| q.end_of_day > 0.0)
}

/// The value of a hidden `_csrf` input in an HTML page.
pub fn csrf_token(html: &str) -> Option<String> {
    let at = html.find("name=\"_csrf\"")?;
    let rest = &html[at..];
    let v = rest.find("value=\"")? + "value=\"".len();
    let end = rest[v..].find('"')?;
    Some(rest[v..v + end].to_string())
}

fn text(bytes: Vec<u8>) -> String {
    // The file is ASCII; the HTML pages are ISO-8859-1.
    bytes.iter().map(|b| *b as char).collect()
}

/// Download the file for one date ("YYYY-MM-DD") from TreasuryDirect.
pub fn download(date: &str) -> Result<String, String> {
    let mut client = Client::session(USER_AGENT, 1.0);
    let e = |x: mapcheck::http::HttpError| x.to_string();
    let form = text(client.get(&format!("{BASE}/selectSecurityPriceDate")).map_err(e)?);
    let token = csrf_token(&form).ok_or("no CSRF token on the FedInvest date page")?;
    client
        .post_form(
            &format!("{BASE}/selectSecurityPriceDate"),
            &[("priceDate", date), ("submit", "Show Prices"), ("_csrf", &token)],
        )
        .map_err(e)?;
    let detail = text(client.get(&format!("{BASE}/securityPriceDetail")).map_err(e)?);
    let token = csrf_token(&detail).ok_or("no CSRF token on the FedInvest price page")?;
    let (y, m, d) = (&date[0..4], date[5..7].trim_start_matches('0'), date[8..10].trim_start_matches('0'));
    let body = client
        .post_form(
            &format!("{BASE}/securityPriceDetail"),
            &[
                ("priceDateDay", d),
                ("priceDateMonth", m),
                ("priceDateYear", y),
                ("fileType", "csv"),
                ("csv", "CSV FORMAT"),
                ("_csrf", &token),
            ],
        )
        .map_err(e)?;
    Ok(text(body))
}

/// The quotes of one date. The cache file `<cache>/prices/fedinvest_<date>.csv` is used
/// when it exists. A download is cached only if it has end-of-day prices.
pub fn load(cache_dir: &Path, date: &str) -> Result<Vec<Quote>, String> {
    let path = cache_dir.join("prices").join(format!("fedinvest_{date}.csv"));
    if let Ok(t) = std::fs::read_to_string(&path) {
        return Ok(parse_csv(&t));
    }
    let t = download(date)?;
    let quotes = parse_csv(&t);
    if quotes.is_empty() {
        return Err(format!("FedInvest gave no price rows for {date}"));
    }
    if has_end_of_day(&quotes) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, &t);
    }
    Ok(quotes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::parse_iso;

    // The real format: no header; the first two rows are bills, then notes and a TIPS row.
    const SAMPLE: &str = "\
912797UZ8,MARKET BASED BILL,0.0,11/19/2026,,0.000000,99.978889,99.989472
912797VY0,MARKET BASED BILL,0.0,11/24/2026,,99.925139,99.924944,99.935667
91282CLY5,MARKET BASED NOTE,0.0425,11/30/2026,,0.000000,100.000000,100.000000
912810XX1,MARKET BASED BOND,0.0625,05/15/2030,05/15/2025,0.000000,101.0,101.0
91282CXX9,TIPS,0.00125,01/15/2027,,0.000000,99.0,99.5
bad line
";

    #[test]
    fn parses_real_format() {
        let q = parse_csv(SAMPLE);
        assert_eq!(q.len(), 5);
        assert_eq!(q[0].cusip, "912797UZ8");
        assert_eq!(q[0].maturity, parse_iso("2026-11-19").unwrap());
        assert_eq!(q[0].end_of_day, 99.989472);
        assert_eq!(q[2].coupon, 0.0425);
        assert_eq!(q[2].security_type, "MARKET BASED NOTE");
        assert_eq!(q[3].call_date, parse_iso("2025-05-15"));
        assert!(q[0].call_date.is_none());
        assert!(has_end_of_day(&q));
    }

    #[test]
    fn empty_end_of_day_is_detected() {
        let q = parse_csv("912797UZ8,MARKET BASED BILL,0.0,11/19/2026,,0.000000,99.989500,0.000000\n");
        assert_eq!(q.len(), 1);
        assert!(!has_end_of_day(&q));
    }

    #[test]
    fn finds_csrf_token() {
        let html = r#"<form><input type="hidden" name="_csrf" value="abc-DEF_123" /></form>"#;
        assert_eq!(csrf_token(html).as_deref(), Some("abc-DEF_123"));
        assert_eq!(csrf_token("<html></html>"), None);
    }
}
