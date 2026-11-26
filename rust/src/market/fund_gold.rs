//! `fa fund attribution GLD`: layers 1 and 2 of the SPDR Gold Trust from its own SEC 10-Q and 10-K (quarter ends).
//!
//! Inputs, for each quarter end: the ounces of gold, the market value of the gold and the date of the LBMA Gold Price PM
//! that the trust used (the text of the filing: the ounces are a custom item that the SEC companyfacts file does not carry),
//! the accrued fee payable to the sponsor, the net assets, the shares and the NAV per share (the statement of financial
//! condition in the text), and the sponsor fee of the quarter (XBRL companyfacts, tag `SponsorFees`, 3-month facts). The
//! companyfacts values of net assets, shares and NAV per share are a cross-check.
//!
//! The gold price is not a number in the filing: it is the market value divided by the ounces ("implied price"). The filing
//! says that the price is the LBMA Gold Price PM of the date. A price of the LBMA is in whole cents: the program tests this.
//!
//! Layer 1a: ounces per share x gold price - accrued fee per share against the reported NAV per share.
//! Layer 1b: NAV per share against the GLD close at the same date (a premium LEVEL: the share count is known).
//! Layer 2: NAV return = gold price return + fee drag (the change of the ounces per share) + the named rest. The rest is
//! split exactly: the cross term (gold return x fee drag) and the effect of the accrued fee payable; what is left is rounding.
//!
//! Request count: the filing list (1), companyfacts (1) and one document for each filing; a file in the cache costs no request.
//! Model document: `docs/models/fund-attribution.md`.

use super::fund_holdings::unescape;
use super::price_cache;
use super::prices::PricePoint;
use super::{parse_date, today_iso, write_atomic};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The SEC central index key of the SPDR Gold Trust (checked in `company_tickers.json` and in the filing list).
pub const GLD_CIK: u64 = 1222333;
const PAUSE: f64 = 0.2;
/// A 3-month fact has this many days between its start and end (inclusive of both ends).
const QUARTER_DAYS: std::ops::RangeInclusive<i64> = 85..=95;
/// A gap between two quarter-end filings of this many days is a quarter (a missing filing gives a larger gap).
const GAP_DAYS: std::ops::RangeInclusive<i64> = 85..=100;
/// The gold price of the LBMA is in whole cents. The implied price may differ from the cent grid by this much (USD): the
/// ounces are rounded to 0.1 ounce or to 1 ounce (error under 0.00004 USD for 28 to 32 million ounces).
const CENT_GRID_TOLERANCE: f64 = 1e-4;
/// A NAV per share that differs from the NAV per share of the identity by more than this (USD) gets a flag (the
/// reported value has 2 decimals, so half a cent is rounding).
const NAV_DIFF_FLAG: f64 = 0.0051;
/// The sponsor fee rate that the filings state (percent of the daily NAV, a year).
pub const FEE_RATE: f64 = 0.004;
/// The pricing date may be at most this many days before the period end (a period end on a weekend).
const MAX_PRICE_DATE_LAG: i64 = 4;
/// The cached filings are looked at for this many years back from the newest quarter end.
const YEARS_BACK: i64 = 3;

// ---------------------------------------------------------------------------------------------------------------
// The filing list and the text of a filing.

#[derive(Debug, Clone, PartialEq)]
pub struct Filing {
    pub form: String,
    pub period_end: String,
    pub filed: String,
    pub accession: String,
    pub doc: String,
}

fn valid_doc_name(s: &str) -> bool {
    !s.is_empty() && s.len() < 80 && s.ends_with(".htm") && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// The 10-Q and 10-K filings of the filing list (`submissions/CIK##########.json`), newest first as in the list. A 10-K/A is
/// not used. If two filings have the same period end, the one that is listed first is used.
pub fn parse_submissions(json: &str) -> Result<(String, Vec<Filing>), String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("filing list: {e}"))?;
    if v["cik"].as_str().and_then(|c| c.parse::<u64>().ok()) != Some(GLD_CIK) {
        return Err(format!("filing list: the CIK is not {GLD_CIK}"));
    }
    let name = v["name"].as_str().unwrap_or("").to_string();
    let r = &v["filings"]["recent"];
    let col = |k: &str| r[k].as_array().cloned().unwrap_or_default();
    let (forms, ends, filed, acc, docs) = (col("form"), col("reportDate"), col("filingDate"), col("accessionNumber"), col("primaryDocument"));
    let mut out: Vec<Filing> = Vec::new();
    for i in 0..forms.len() {
        let s = |c: &Vec<Value>| c.get(i).and_then(Value::as_str).unwrap_or("").to_string();
        let f = Filing { form: s(&forms), period_end: s(&ends), filed: s(&filed), accession: s(&acc), doc: s(&docs) };
        if !matches!(f.form.as_str(), "10-Q" | "10-K") || parse_date(&f.period_end).is_none() || !super::fund_holdings::valid_accession(&f.accession) || !valid_doc_name(&f.doc) {
            continue;
        }
        if out.iter().any(|o| o.period_end == f.period_end) {
            continue;
        }
        out.push(f);
    }
    Ok((name, out))
}

/// The text of an HTML filing: tags become spaces, entities are decoded, white space (also the no-break space) is one space.
pub fn flat_text(html: &str) -> String {
    let mut s = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => {
                in_tag = true;
                s.push(' ');
            }
            '>' if in_tag => {
                in_tag = false;
                s.push(' ');
            }
            _ if !in_tag => s.push(c),
            _ => {}
        }
    }
    unescape(&s.replace("&nbsp;", " ")).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A number after `marker` (after spaces, `$` and `(`; commas removed). Returns the number and the position after it.
/// A dash (the sign of the statement for "none") is 0.
fn num_after(text: &str, marker: &str, from: usize) -> Option<(f64, usize)> {
    let at = from + text.get(from..)?.find(marker)? + marker.len();
    let rest = &text[at..];
    let skip = rest.len() - rest.trim_start_matches([' ', '$', '(']).len();
    let body = &rest[skip..];
    if let Some(r) = body.strip_prefix(['\u{2014}', '\u{2013}']) {
        return Some((0.0, at + skip + body.len() - r.len()));
    }
    let len = body.find(|c: char| !(c.is_ascii_digit() || c == ',' || c == '.')).unwrap_or(body.len());
    let raw = body[..len].replace(',', "");
    let raw = raw.trim_end_matches('.');
    raw.parse::<f64>().ok().map(|v| (v, at + skip + len))
}

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/// `June 30, 2026` at the start of `text` as `2026-06-30`.
pub fn date_at(text: &str) -> Option<String> {
    let text = text.trim_start();
    let (i, m) = MONTHS.iter().enumerate().find(|(_, m)| text.starts_with(**m))?;
    let rest = text[m.len()..].trim_start();
    let day_end = rest.find(|c: char| !c.is_ascii_digit())?;
    let day: u32 = rest[..day_end].parse().ok()?;
    let rest = rest[day_end..].trim_start_matches([',', ' ']);
    let year: u32 = rest.get(..4)?.parse().ok()?;
    let d = format!("{year:04}-{:02}-{day:02}", i + 1);
    parse_date(&d).map(|_| d)
}

/// What the text of one filing gives.
#[derive(Debug, Clone, PartialEq)]
pub struct DocValues {
    pub ounces: f64,
    /// The market value of the gold in USD (to the dollar) and the date of the LBMA Gold Price PM.
    pub market_value: f64,
    pub price_date: String,
    /// Accounts payable to the sponsor (the accrued fee), net assets, in USD (the statement is in thousands).
    pub payable: f64,
    /// Gold receivable (creations: shares exist, the gold is not yet in the trust) and gold payable (redemptions: the shares are
    /// gone, the gold is still in the trust), in USD (thousands in the statement).
    pub gold_receivable: f64,
    pub gold_payable: f64,
    pub net_assets: f64,
    pub shares: f64,
    pub nav_reported: f64,
}

/// Read the values of the text of a 10-Q or 10-K (the first, current period, of each item).
pub fn parse_doc(text: &str) -> Result<DocValues, String> {
    let m = "the amount of gold owned by the Trust";
    let at = text.find(m).ok_or("the sentence on the gold owned by the Trust is not in the text")?;
    let window = &text[at..(at + 700).min(text.len())];
    let (ounces, p) = num_after(window, " was ", 0).ok_or("no ounces")?;
    if !window[p..].trim_start().starts_with("ounces") {
        return Err("the number after `was` is not followed by `ounces`".into());
    }
    let (market_value, p) = num_after(window, "market value of", p).ok_or("no market value")?;
    let l = "based on the LBMA Gold Price PM on";
    let li = window[p..].find(l).ok_or("the market value is not based on the LBMA Gold Price PM in the sentence")?;
    let price_date = date_at(&window[p + li + l.len()..]).ok_or("no date of the LBMA price")?;
    // The statement of financial condition (amounts in thousands).
    let s = text.find("Statements of Financial Condition").ok_or("no statement of financial condition")?;
    // Gold receivable and gold payable are rows of the statement only when a quarter has them: no row is 0.
    let (payable, p) = num_after(text, "Accounts payable to Sponsor", s).ok_or("no accounts payable to Sponsor")?;
    let gold_receivable = ["Gold receivable", "Gold Receivable"].iter().find_map(|m| num_after(&text[..p], m, s)).map_or(0.0, |v| v.0);
    let (net_assets, p2) = num_after(text, "Net Assets", p).ok_or("no net assets")?;
    let gold_payable = ["Gold payable", "Gold Payable"].iter().find_map(|m| num_after(&text[..p2], m, p)).map_or(0.0, |v| v.0);
    let p = p2;
    let after = text.get(p..).ok_or("text ends")?;
    let sh = after.find("Shares issued and outstanding").ok_or("no shares line")? + "Shares issued and outstanding".len();
    let mut q = p + sh;
    let t = text[q..].trim_start();
    if t.starts_with('(') {
        q = text.len() - t.len() + t.find(')').ok_or("bad footnote mark")? + 1;
    }
    let (shares, p) = num_after(text, "", q).ok_or("no shares")?;
    let (nav_reported, _) = num_after(text, "Net asset value per Share", p).ok_or("no NAV per share")?;
    // A statement in thousands: the first Net Assets number is in thousands of USD.
    Ok(DocValues { ounces, market_value, price_date, payable: payable * 1000.0, gold_receivable: gold_receivable * 1000.0, gold_payable: gold_payable * 1000.0, net_assets: net_assets * 1000.0, shares, nav_reported })
}

// ---------------------------------------------------------------------------------------------------------------
// XBRL companyfacts.

/// The value of an XBRL fact at an instant (the first fact of the tag with this end date and unit).
pub fn fact_at(facts: &Value, tag: &str, unit: &str, end: &str) -> Option<f64> {
    facts["facts"]["us-gaap"][tag]["units"][unit].as_array()?.iter().find(|f| f["end"].as_str() == Some(end) && f["start"].is_null())?["val"].as_f64()
}

/// The sponsor fee of the 3 months that end on `end` (a 3-month fact of `SponsorFees`).
pub fn fee_quarter(facts: &Value, end: &str) -> Option<f64> {
    let e = parse_date(end)?;
    facts["facts"]["us-gaap"]["SponsorFees"]["units"]["USD"].as_array()?.iter().find_map(|f| {
        let (s, fe) = (f["start"].as_str()?, f["end"].as_str()?);
        let days = e - parse_date(s)? + 1;
        (fe == end && QUARTER_DAYS.contains(&days)).then(|| f["val"].as_f64()).flatten()
    })
}

// ---------------------------------------------------------------------------------------------------------------
// The calculation.

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub period_end: String,
    pub form: String,
    pub accession: String,
    pub filed: String,
    pub doc: DocValues,
    /// The sponsor fee of the quarter that ends on the period end (XBRL), if tagged.
    pub fee_q: Option<f64>,
    /// The differences from the XBRL companyfacts (net assets, shares, NAV per share): a text, empty if all agree.
    pub xbrl_note: String,
}

impl Point {
    /// The gold price that the trust used: the market value divided by the ounces.
    pub fn price(&self) -> f64 {
        self.doc.market_value / self.doc.ounces
    }
    /// The USD of the gold in settlement: receivable (creations) less payable (redemptions).
    pub fn settlement(&self) -> f64 {
        self.doc.gold_receivable - self.doc.gold_payable
    }
    /// The ounces per share, settlement-adjusted: the shares of a creation count the gold that is still to come, and the
    /// shares of a redemption are already gone while the gold is still held. Both are valued at the trust's price.
    pub fn oz_per_share(&self) -> f64 {
        (self.doc.ounces + self.settlement() / self.price()) / self.doc.shares
    }
    pub fn fee_per_share(&self) -> f64 {
        self.doc.payable / self.doc.shares
    }
    /// The NAV per share from the net assets and the shares (more digits than the reported NAV).
    pub fn nav(&self) -> f64 {
        self.doc.net_assets / self.doc.shares
    }
    /// Layer 1a: (ounces per share, settlement-adjusted) x gold price - accrued fee per share.
    pub fn nav_identity(&self) -> f64 {
        self.oz_per_share() * self.price() - self.fee_per_share()
    }
    /// The distance of the implied price from the grid of whole cents (USD).
    pub fn cent_grid_error(&self) -> f64 {
        let p = self.price() * 100.0;
        (p - p.round()).abs() / 100.0
    }
}

/// The parts of the NAV return of one quarter (fractions). `nav_ret = g + d + cross + payable + rounding` exactly.
#[derive(Debug, Clone, PartialEq)]
pub struct Decomp {
    pub nav_ret: f64,
    /// The gold price return (implied prices).
    pub g: f64,
    /// The fee drag: the change of the ounces per share.
    pub d: f64,
    /// g x d.
    pub cross: f64,
    /// The effect of the accrued fee payable: (f0 (1+g)(1+d) - f1) / NAV0, f = accrued fee per share.
    pub payable: f64,
    /// What is left: the rounding of the net assets (thousands of USD) and of the shares.
    pub rounding: f64,
}

impl Decomp {
    /// The rest in the formula NAV return = gold return + fee drag + rest.
    pub fn rest(&self) -> f64 {
        self.nav_ret - self.g - self.d
    }
}

/// The identity: NAV = o P - f with o the settlement-adjusted ounces per share, P the price, f the fee per share. Then
/// NAV1/NAV0 - 1 = g + d + g d + (f0 (1+g)(1+d) - f1)/NAV0, with 1+g = P1/P0 and 1+d = o1/o0.
pub fn decompose(a: &Point, b: &Point) -> Decomp {
    let g = b.price() / a.price() - 1.0;
    let d = b.oz_per_share() / a.oz_per_share() - 1.0;
    let nav_ret = b.nav() / a.nav() - 1.0;
    let n0 = a.nav_identity();
    let payable = (a.fee_per_share() * (1.0 + g) * (1.0 + d) - b.fee_per_share()) / n0;
    let cross = g * d;
    let rounding = nav_ret - g - d - cross - payable;
    Decomp { nav_ret, g, d, cross, payable, rounding }
}

/// The premium level: the close of the fund against its NAV per share.
pub fn premium(close: f64, nav: f64) -> f64 {
    close / nav - 1.0
}

/// The last close on or before `date`, at most 5 calendar days before it: the date of the row and the close. The filing names
/// the period end as the price date, also when it is a Saturday or Sunday (the LBMA has no price then): the NAV of such a
/// date is the one of the last business day, and so is the GLD close.
fn close_before(rows: &[PricePoint], date: &str) -> Option<(String, f64)> {
    let i = rows.partition_point(|r| r.date.as_str() <= date);
    let r = rows.get(i.checked_sub(1)?)?;
    let gap = parse_date(date)? - parse_date(&r.date)?;
    (gap <= 5 && r.close.is_finite() && r.close > 0.0).then(|| (r.date.clone(), r.close))
}

fn pp(v: f64) -> String {
    format!("{:+.3}", v * 100.0)
}

/// The limits that apply to GLD (the list of the URA run is in `fund_attribution::LIMITS`).
pub const GLD_LIMITS: [&str; 6] = [
    "1. The gold price is the one that the trust used for its own NAV: the market value of the filing divided by the ounces (the LBMA Gold Price PM of the date in the filing). It is the trust's own number, not a second source: layer 1a tests the arithmetic and the cent grid, not the price.",
    "2. The monthly view is not yet built (it waits for an owner answer). A later monthly view that uses the World Bank monthly average compares an average of the month with a month-end NAV: the two do not match, and the difference is not an error of the trust.",
    "3. The NAV is struck at the LBMA Gold Price PM (3:00 PM London, about 10:00 AM New York); the GLD close is at 4:00 PM New York. The premium level contains the move of gold between the two times (not removed).",
    "4. The fee drag is the change of the ounces per share. It includes the gold that the trust sold to pay the fee and any other change of the ounces per share (none is known). The fee rate (0.40% a year) is a text of the filing, not a test of it.",
    "5. A quarter is the gap between two quarter-end filings (85 to 100 days). The last quarter end can be missing until the filing is public (10-Q about 5 weeks, 10-K about 8 weeks after the end). A 10-K/A is not used.",
    "6. GLD pays no distribution: the price return of the Nasdaq close (split-adjusted) is the total return. A distribution would enter the premium change.",
];

fn gld_cautions() -> [String; 3] {
    [
        "- The ounces, the market value, the payable and the NAV are read from the text of the filing (the layout of the statement is assumed, and tested on the filings of the run). The companyfacts file has no tag for the ounces.".to_string(),
        "- A quarter-end value of a filing is the first of the period in the text; the XBRL companyfacts values of net assets, shares and NAV per share are a cross-check (differences are printed).".to_string(),
        "- A period end on a weekend (for example 2023-12-31) has no LBMA price of its own: the filing names the period end, and the NAV is the one of the last business day. The GLD close is the last close before the date (the date is printed).".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------------------------
// The cache.

fn trust_dir(cache: &Path) -> PathBuf {
    cache.join("GLD").join("trust")
}

fn doc_url(f: &Filing) -> String {
    format!("https://www.sec.gov/Archives/edgar/data/{GLD_CIK}/{}/{}", f.accession.replace('-', ""), f.doc)
}

/// Add or replace one entry of the record file `trust/record.json`: where the file came from.
fn record(dir: &Path, file: &str, url: &str, bytes: usize, note: &str) -> Result<(), String> {
    let path = dir.join("record.json");
    let mut v: Value = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| serde_json::json!({"source": "SEC EDGAR (the SPDR Gold Trust, CIK 1222333)", "files": {}}));
    v["files"][file] = serde_json::json!({"url": url, "fetched": today_iso(), "bytes": bytes, "note": note});
    write_atomic(&path, &serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?)
}

fn http_get(client: &mut mapcheck::http::Client, url: &str, requests: &mut usize) -> Result<String, String> {
    *requests += 1;
    let b = client.get(url).map_err(|e| e.to_string())?;
    String::from_utf8(b).map_err(|e| format!("{url}: not UTF-8: {e}"))
}

/// The year-back limit date: the newest period end with the year reduced by `YEARS_BACK`.
fn start_date(newest: &str) -> Option<String> {
    let y: i64 = newest.get(..4)?.parse().ok()?;
    Some(format!("{:04}{}", y - YEARS_BACK, &newest[4..]))
}

pub struct Loaded {
    pub filings: Vec<Filing>,
    pub name: String,
    pub requests: usize,
}

/// The filing list and the facts, from the cache; fetched only if missing, or if `refresh`. Returns the filings in the range
/// (oldest first) and the number of requests. `fetch` allows the requests; without it a missing file is an error.
fn load(cache: &Path, from: Option<&str>, to: Option<&str>, fetch: bool, refresh: bool) -> Result<(Loaded, Value), String> {
    let dir = trust_dir(cache);
    let sub_path = dir.join("submissions.json");
    let facts_path = dir.join("company_facts.json");
    let mut requests = 0usize;
    let mut client: Option<mapcheck::http::Client> = None;
    let need = |client: &mut Option<mapcheck::http::Client>| -> Result<(), String> {
        if client.is_none() {
            *client = Some(mapcheck::http::Client::sec(PAUSE)?);
        }
        Ok(())
    };
    let missing = |what: &str| format!("GLD: {what} is not in the cache {}. Run: fa fund attribution GLD --fetch", dir.display());
    let sub_text = if sub_path.is_file() && !refresh {
        std::fs::read_to_string(&sub_path).map_err(|e| e.to_string())?
    } else if fetch {
        need(&mut client)?;
        let url = format!("https://data.sec.gov/submissions/CIK{GLD_CIK:010}.json");
        let t = http_get(client.as_mut().ok_or("no client")?, &url, &mut requests)?;
        parse_submissions(&t)?;
        write_atomic(&sub_path, &t)?;
        record(&dir, "submissions.json", &url, t.len(), "the filing list; 10-Q and 10-K are used")?;
        t
    } else {
        return Err(missing("the filing list"));
    };
    let (name, all) = parse_submissions(&sub_text)?;
    let newest = all.iter().map(|f| f.period_end.as_str()).max().ok_or("GLD: no 10-Q or 10-K in the filing list")?.to_string();
    let lo = from.map(str::to_string).or_else(|| start_date(&newest)).unwrap_or_default();
    let mut filings: Vec<Filing> = all.into_iter().filter(|f| f.period_end >= lo && to.is_none_or(|t| f.period_end.as_str() <= t)).collect();
    filings.sort_by(|a, b| a.period_end.cmp(&b.period_end));

    let facts_text = if facts_path.is_file() && !refresh {
        std::fs::read_to_string(&facts_path).map_err(|e| e.to_string())?
    } else if fetch {
        need(&mut client)?;
        let url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{GLD_CIK:010}.json");
        let t = http_get(client.as_mut().ok_or("no client")?, &url, &mut requests)?;
        serde_json::from_str::<Value>(&t).map_err(|e| format!("companyfacts: {e}"))?;
        write_atomic(&facts_path, &t)?;
        record(&dir, "company_facts.json", &url, t.len(), "XBRL companyfacts: SponsorFees (3-month), cross-check of net assets, shares, NAV per share")?;
        t
    } else {
        return Err(missing("the companyfacts file"));
    };
    let facts: Value = serde_json::from_str(&facts_text).map_err(|e| format!("companyfacts: {e}"))?;
    for f in &filings {
        let p = dir.join(format!("{}.htm", f.accession));
        if p.is_file() {
            continue;
        }
        if !fetch {
            return Err(missing(&format!("the filing {} ({} {})", f.accession, f.form, f.period_end)));
        }
        need(&mut client)?;
        let url = doc_url(f);
        let t = http_get(client.as_mut().ok_or("no client")?, &url, &mut requests)?;
        write_atomic(&p, &t)?;
        record(&dir, &format!("{}.htm", f.accession), &url, t.len(), &format!("{} for the period ending {}, filed {}", f.form, f.period_end, f.filed))?;
    }
    Ok((Loaded { filings, name, requests }, facts))
}

/// Read one filing from the text, with the companyfacts cross-check.
pub fn point_of(f: &Filing, text: &str, facts: &Value) -> Result<Point, String> {
    let doc = parse_doc(&flat_text(text)).map_err(|e| format!("{} {}: {e}", f.form, f.period_end))?;
    let gap = parse_date(&f.period_end).zip(parse_date(&doc.price_date)).map(|(a, b)| a - b).unwrap_or(-1);
    if !(0..=MAX_PRICE_DATE_LAG).contains(&gap) {
        return Err(format!("{} {}: the LBMA price date {} is not at the period end", f.form, f.period_end, doc.price_date));
    }
    let mut notes: Vec<String> = Vec::new();
    // Tolerance: the net assets are in thousands of USD in the text, the shares are whole, the NAV has 2 decimals.
    for (tag, unit, mine, tol) in [("AssetsNet", "USD", doc.net_assets, 500.0), ("SharesOutstanding", "shares", doc.shares, 0.5), ("NetAssetValuePerShare", "USD/shares", doc.nav_reported, 0.0051)] {
        match fact_at(facts, tag, unit, &f.period_end) {
            Some(x) if (x - mine).abs() > tol => notes.push(format!("{tag} XBRL {x} text {mine}")),
            Some(_) => {}
            None => notes.push(format!("{tag} not in companyfacts")),
        }
    }
    Ok(Point { period_end: f.period_end.clone(), form: f.form.clone(), accession: f.accession.clone(), filed: f.filed.clone(), doc, fee_q: fee_quarter(facts, &f.period_end), xbrl_note: notes.join("; ") })
}

/// The run of `fa fund attribution GLD`.
pub fn run(repo: &Path, o: &super::fund_attribution::Options) -> Result<i32, String> {
    let cache = o.cache.clone().unwrap_or_else(|| repo.join("data").join("raw"));
    let (loaded, facts) = load(&cache, o.from.as_deref(), o.to.as_deref(), o.fetch, o.refresh)?;
    let dir = trust_dir(&cache);
    let mut points: Vec<Point> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for f in &loaded.filings {
        let text = std::fs::read_to_string(dir.join(format!("{}.htm", f.accession))).map_err(|e| format!("{}: {e}", f.accession))?;
        match point_of(f, &text, &facts) {
            Ok(p) => points.push(p),
            Err(e) => errors.push(e),
        }
    }
    if points.is_empty() {
        return Err(format!("GLD: no filing could be read. {}", errors.join(" | ")));
    }
    let prices = price_cache::history(&cache, "GLD");
    let meta: Option<price_cache::Meta> = std::fs::read_to_string(price_cache::meta_path(&cache, "GLD")).ok().and_then(|t| serde_json::from_str(&t).ok());
    println!(
        "GLD  {} (CIK {GLD_CIK}): {} quarter-end filings, {} to {}; {} request{} ({})",
        loaded.name,
        points.len(),
        points[0].period_end,
        points[points.len() - 1].period_end,
        loaded.requests,
        if loaded.requests == 1 { "" } else { "s" },
        if loaded.requests == 0 { "the cache only" } else { "the others from the cache" }
    );
    for e in &errors {
        println!("NOT READ: {e}");
    }
    match (&prices, &meta) {
        (Some(_), Some(m)) => println!("GLD price: {} closes, adjusted for {} ({} to {}). GLD pays no distribution: the price return is the total return.", m.source, m.adjusted, m.first_date, m.last_date),
        (Some(_), None) => println!("GLD price: no record file."),
        _ => println!("GLD price: no price file (fa prices get GLD --source nasdaq): the premium lines are n/a."),
    }
    println!("Gold price: the LBMA Gold Price PM of the pricing date, as the trust used it. It is read as market value / ounces from the text of the filing (no price number is tagged).");
    println!("Monthly view: not yet (waits for an owner answer).");

    println!("\nLAYER 1a - ounces per share x gold price - accrued fee per share, against the reported NAV per share");
    println!(
        "{:<11} {:<11} {:<5} {:>11} {:>9} {:>10} {:>10} {:>9} {:>9} {:>9} {:>8} {:>8} {:>8}",
        "period end", "price date", "form", "ounces k", "shares m", "price", "oz/share", "fee/sh", "NAV ident", "NAV rep", "diff", "cent err", "settle $m"
    );
    let mut flags: Vec<String> = Vec::new();
    for p in &points {
        let diff = p.nav_identity() - p.doc.nav_reported;
        println!(
            "{:<11} {:<11} {:<5} {:>11.1} {:>9.1} {:>10.2} {:>10.5} {:>9.4} {:>9.4} {:>9.2} {:>+8.4} {:>8.1e} {:>+8.1}",
            p.period_end,
            p.doc.price_date,
            p.form,
            p.doc.ounces / 1e3,
            p.doc.shares / 1e6,
            p.price(),
            p.oz_per_share(),
            p.fee_per_share(),
            p.nav_identity(),
            p.doc.nav_reported,
            diff,
            p.cent_grid_error(),
            p.settlement() / 1e6
        );
        if diff.abs() > NAV_DIFF_FLAG {
            flags.push(format!("FLAG {}: the identity NAV {:.4} differs from the reported NAV {:.2} by {:+.4}", p.period_end, p.nav_identity(), p.doc.nav_reported, diff));
        }
        if p.cent_grid_error() > CENT_GRID_TOLERANCE {
            flags.push(format!("FLAG {}: the implied price {:.6} is {:.4} cent from a whole cent (the rounding of the ounces explains up to {:.3} cent)", p.period_end, p.price(), p.cent_grid_error() * 100.0, CENT_GRID_TOLERANCE * 100.0));
        }
        let by_nav = p.doc.net_assets / p.doc.shares;
        if (by_nav - p.doc.nav_reported).abs() > NAV_DIFF_FLAG {
            flags.push(format!("FLAG {}: net assets / shares {:.4} differs from the reported NAV {:.2}", p.period_end, by_nav, p.doc.nav_reported));
        }
        let gap = p.doc.market_value + p.settlement() - p.doc.payable - p.doc.net_assets;
        if gap.abs() > 1000.0 {
            flags.push(format!("FLAG {}: gold value + receivable - gold payable - fee payable - net assets = {:+.0} USD (the statement is in thousands: up to 500 is rounding)", p.period_end, gap));
        }
        if !p.xbrl_note.is_empty() {
            flags.push(format!("XBRL check {}: {}", p.period_end, p.xbrl_note));
        }
    }
    println!("diff = identity NAV - reported NAV (the reported NAV has 2 decimals). cent err = distance of the implied price from a whole cent (USD).");
    println!("oz/share = (ounces + settle / price) / shares. settle = gold receivable (creations, gold not yet in the trust) - gold payable (redemptions, gold still in the trust).");
    println!("fee/sh = accrued fee payable to the sponsor per share (USD). The statement is in thousands of USD; the gold value is in dollars.");
    if flags.is_empty() {
        println!("Checks: the NAV identity holds to the cent, the implied prices are on the cent grid, net assets / shares agree with the reported NAV, and the XBRL companyfacts values agree with the text, for each filing.");
    }
    for f in &flags {
        println!("{f}");
    }

    println!("\nLAYER 1b - NAV per share against the GLD close at the pricing date (the premium level)");
    println!("{:<11} {:>10} {:<11} {:>10} {:>10}", "price date", "NAV/share", "close date", "GLD close", "premium %");
    let mut levels: Vec<Option<f64>> = Vec::new();
    for p in &points {
        let hit = prices.as_ref().and_then(|r| close_before(r, &p.doc.price_date));
        let close = hit.as_ref().map(|h| h.1);
        let prem = close.map(|c| premium(c, p.nav()));
        levels.push(prem);
        println!("{:<11} {:>10.4} {:<11} {:>10} {:>10}", p.doc.price_date, p.nav(), hit.as_ref().map_or("n/a", |h| h.0.as_str()), close.map_or("n/a".into(), |c| format!("{c:.2}")), prem.map_or("n/a".into(), |v| format!("{:+.3}", v * 100.0)));
    }
    println!("The premium is close / NAV - 1. The NAV uses the 3:00 PM London price; the close is 4:00 PM New York (limit 3).");

    struct Row {
        end: String,
        nav: f64,
        g: f64,
        d: f64,
        rest: f64,
        price_ret: Option<f64>,
    }
    let mut rows: Vec<Row> = Vec::new();
    for (i, w) in points.windows(2).enumerate() {
        let (a, b) = (&w[0], &w[1]);
        let gap = parse_date(&b.period_end).zip(parse_date(&a.period_end)).map(|(x, y)| x - y);
        println!("\n=== quarter {} to {} (pricing dates {} to {}) ===", a.period_end, b.period_end, a.doc.price_date, b.doc.price_date);
        if !gap.is_some_and(|g| GAP_DAYS.contains(&g)) {
            println!("no quarter: the two filings are {} days apart (a filing is missing between them). No attribution.", gap.map_or("?".to_string(), |g| g.to_string()));
            continue;
        }
        let d = decompose(a, b);
        let days = parse_date(&b.doc.price_date).zip(parse_date(&a.doc.price_date)).map_or(0, |(x, y)| x - y);
        println!("LAYER 2: NAV return {} pp = gold price return {} pp + fee drag {} pp + rest {} pp", pp(d.nav_ret), pp(d.g), pp(d.d), pp(d.rest()));
        println!(
            "  gold price {:.2} -> {:.2}; ounces per share {:.5} -> {:.5}; NAV per share {:.4} -> {:.4}",
            a.price(),
            b.price(),
            a.oz_per_share(),
            b.oz_per_share(),
            a.nav(),
            b.nav()
        );
        println!(
            "  rest {} pp = cross term (gold return x fee drag) {} pp + effect of the accrued fee payable {} pp + rounding {} pp   <-- named remainder",
            pp(d.rest()),
            pp(d.cross),
            pp(d.payable),
            format_args!("{:+.5}", d.rounding * 100.0)
        );
        let expect = -FEE_RATE * days as f64 / 365.0;
        let avg_na = (a.doc.net_assets + b.doc.net_assets) / 2.0;
        match b.fee_q {
            Some(fee) => println!(
                "  fee check ({days} days): fee drag {} pp; stated rate {:.2}% x days/365 = {} pp; sponsor fee of the quarter {:.1} USD m / mean net assets = {} pp (the fee follows the daily NAV, not the mean of two dates)",
                pp(d.d),
                FEE_RATE * 100.0,
                pp(expect),
                fee / 1e6,
                pp(-fee / avg_na)
            ),
            None => println!("  fee check ({days} days): fee drag {} pp; stated rate {:.2}% x days/365 = {} pp; the sponsor fee of the quarter is not tagged for this quarter", pp(d.d), FEE_RATE * 100.0, pp(expect)),
        }
        let pr = prices.as_ref().and_then(|r| close_before(r, &a.doc.price_date).zip(close_before(r, &b.doc.price_date))).map(|(x, y)| y.1 / x.1 - 1.0);
        match pr {
            Some(r) => {
                let (diff, ratio) = super::fund_attribution::premium_change(r, d.nav_ret);
                println!("LAYER 2 premium: GLD price return {} pp - NAV return {} pp = change of the premium {} pp (ratio form {} pp)", pp(r), pp(d.nav_ret), pp(diff), pp(ratio));
                if let (Some(Some(l0)), Some(Some(l1))) = (levels.get(i), levels.get(i + 1)) {
                    println!("  premium level {} % -> {} %: ratio change (1+l1)/(1+l0)-1 = {} pp, as the ratio form (difference {:.2e})", pp(*l0), pp(*l1), pp((1.0 + l1) / (1.0 + l0) - 1.0), (1.0 + l1) / (1.0 + l0) - 1.0 - ratio);
                }
            }
            None => println!("LAYER 2 premium: n/a (no GLD close at the two pricing dates)"),
        }
        rows.push(Row { end: b.period_end.clone(), nav: d.nav_ret, g: d.g, d: d.d, rest: d.rest(), price_ret: pr });
    }

    println!("\nSUMMARY (pp of the NAV return for each quarter)");
    println!("{:<11} {:>9} {:>9} {:>9} {:>9} {:>10} {:>10}", "quarter end", "NAV ret", "gold", "fee drag", "rest", "GLD price", "prem chg");
    for r in &rows {
        println!(
            "{:<11} {:>9} {:>9} {:>9} {:>9} {:>10} {:>10}",
            r.end,
            pp(r.nav),
            pp(r.g),
            pp(r.d),
            pp(r.rest),
            r.price_ret.map_or("n/a".into(), pp),
            r.price_ret.map_or("n/a".into(), |p| pp(p - r.nav))
        );
    }
    println!("\nTHE LIMITS");
    for l in GLD_LIMITS {
        println!("{l}");
    }
    println!("CAUTIONS");
    for c in gld_cautions() {
        println!("{c}");
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trimmed text that follows the layout of the 10-Q of 2026-06-30 (the numbers of that filing for the statement; the
    /// sentence on the gold is the real one).
    const TEXT: &str = "Item 1. Financial Statements Statements of Financial Condition At June 30, 2026 (Amounts in 000's of US$) \
        ASSETS Investments in Gold, at fair value $ 130,098,696 $ 124,430,281 Gold receivable $ — $ 140,834 Total Assets $ 130,098,696 $ 124,571,115 \
        LIABILITIES Accounts payable to Sponsor $ 45,418 $ 38,255 Gold payable — — Total Liabilities $ 45,418 $ 38,255 \
        Net Assets $ 130,053,278 $ 124,532,860 Shares issued and outstanding (1) 352,000,000 353,700,000 Net asset value per Share $ 369.47 $ 352.09 \
        (1) Authorized share capital is unlimited. At June 30, 2026, the amount of gold owned by the Trust and held by the Custodians was 32,314,227.7 ounces, \
        100% of which is allocated gold in the form of London Good Delivery gold bars with a market value of $130,098,696,246 , based on the LBMA Gold Price PM on June 30, 2026 (cost— $95,236,108,571).";

    fn doc() -> DocValues {
        parse_doc(&flat_text(TEXT)).unwrap()
    }

    #[test]
    fn the_text_gives_the_values_of_the_filing() {
        let d = doc();
        assert_eq!(d.ounces, 32_314_227.7);
        assert_eq!(d.market_value, 130_098_696_246.0);
        assert_eq!(d.price_date, "2026-06-30");
        assert_eq!((d.payable, d.net_assets, d.shares, d.nav_reported), (45_418_000.0, 130_053_278_000.0, 352_000_000.0, 369.47));
        // The dash is zero: the receivable of the current period is 0 (the 140,834 is the prior period). The gold payable is 0.
        assert_eq!((d.gold_receivable, d.gold_payable), (0.0, 0.0));
        // A receivable in the current period and a gold payable are read in thousands.
        let t = TEXT.replace("Gold receivable $ — $ 140,834", "Gold receivable $ 140,834 $ —").replace("Gold payable — —", "Gold payable 17,000 —");
        let d = parse_doc(&flat_text(&t)).unwrap();
        assert_eq!((d.gold_receivable, d.gold_payable), (140_834_000.0, 17_000_000.0));
        // The label can have capital letters (the 10-Q of 2025-03-31: "Gold Receivable $ 143,639").
        let t = TEXT.replace("Gold receivable $ — $ 140,834", "Gold Receivable $ 143,639 $ —");
        assert_eq!(parse_doc(&flat_text(&t)).unwrap().gold_receivable, 143_639_000.0);
        // The older statements have no such rows: 0.
        let t = TEXT.replace("Gold receivable $ — $ 140,834 ", "").replace("Gold payable — — ", "");
        let d = parse_doc(&flat_text(&t)).unwrap();
        assert_eq!((d.gold_receivable, d.gold_payable, d.net_assets), (0.0, 0.0, 130_053_278_000.0));
    }

    #[test]
    fn html_becomes_text_and_a_bad_text_is_refused() {
        let t = flat_text("<p>Net&nbsp;Assets <b>$</b>\u{a0}1,5&#8217;</p>\n<td>x</td>");
        assert_eq!(t, "Net Assets $ 1,5\u{2019} x");
        assert!(parse_doc("nothing").is_err());
        let bad = TEXT.replace("ounces", "grams");
        assert!(parse_doc(&bad).unwrap_err().contains("ounces"));
        let no_lbma = TEXT.replace("LBMA Gold Price PM", "some price");
        assert!(parse_doc(&no_lbma).is_err());
        assert_eq!(date_at(" September 30, 2025 (cost"), Some("2025-09-30".into()));
        assert_eq!(date_at("Smarch 30, 2025"), None);
    }

    #[test]
    fn identity_and_cent_grid_of_the_real_numbers() {
        let p = Point { period_end: "2026-06-30".into(), form: "10-Q".into(), accession: "x".into(), filed: "".into(), doc: doc(), fee_q: Some(149_764_000.0), xbrl_note: String::new() };
        // market value / ounces = 4026.05 (the price on the cent grid, to the rounding of the ounces).
        assert!((p.price() - 4026.05).abs() < 1e-3, "{}", p.price());
        assert!(p.cent_grid_error() < CENT_GRID_TOLERANCE, "{}", p.cent_grid_error());
        // ounces per share x price - fee per share = (value - payable) / shares = 369.4814 against the reported 369.47.
        let want = (130_098_696_246.0 - 45_418_000.0) / 352_000_000.0;
        assert!((p.nav_identity() - want).abs() < 1e-9);
        assert!((p.nav_identity() - 369.47).abs() < NAV_DIFF_FLAG, "{}", p.nav_identity());
        assert!((p.nav() - 130_053_278_000.0 / 352_000_000.0).abs() < 1e-12);
    }

    /// A constructed pair of points with round numbers (not a filing): 100 ounces for 10 shares at the price 2000,
    /// then 99 ounces (the fee) at the price 2200, fee payable 1000 then 1100, shares 10 both times.
    fn pair() -> (Point, Point) {
        let mk = |oz: f64, price: f64, pay: f64| {
            let mv = oz * price;
            let doc = DocValues { ounces: oz, market_value: mv, price_date: "2026-01-02".into(), payable: pay, gold_receivable: 0.0, gold_payable: 0.0, net_assets: mv - pay, shares: 10.0, nav_reported: 0.0 };
            Point { period_end: "2026-01-02".into(), form: "10-Q".into(), accession: "x".into(), filed: "".into(), doc, fee_q: None, xbrl_note: String::new() }
        };
        (mk(100.0, 2000.0, 1000.0), mk(99.0, 2200.0, 1100.0))
    }

    #[test]
    fn gold_in_settlement_is_part_of_the_ounces_per_share() {
        // 100 ounces held at 2000, 10 shares; a creation of 1 share is booked with 20000 receivable (the gold is not yet in
        // the trust; the share count has it): 11 shares would be wrong without it. Here: shares 10, receivable 20000, payable 1000.
        let (mut a, _) = pair();
        a.doc.gold_receivable = 20_000.0;
        a.doc.net_assets = a.doc.market_value + 20_000.0 - 1_000.0;
        assert!((a.oz_per_share() - 11.0).abs() < 1e-12);
        assert!((a.nav_identity() - 21_900.0).abs() < 1e-9 && (a.nav() - 21_900.0).abs() < 1e-9);
        // A gold payable (redemption: the shares are gone, the gold is still held) lowers the adjusted ounces.
        a.doc.gold_receivable = 0.0;
        a.doc.gold_payable = 40_000.0;
        a.doc.net_assets = a.doc.market_value - 40_000.0 - 1_000.0;
        assert!((a.oz_per_share() - 8.0).abs() < 1e-12 && (a.nav_identity() - a.nav()).abs() < 1e-9);
    }

    #[test]
    fn the_parts_add_to_the_nav_return_with_a_known_split() {
        let (a, b) = pair();
        let d = decompose(&a, &b);
        // NAV0 = (200000 - 1000) / 10 = 19900. NAV1 = (217800 - 1100) / 10 = 21670. Return 21670 / 19900 - 1.
        assert!((d.nav_ret - (21670.0 / 19900.0 - 1.0)).abs() < 1e-14);
        assert!((d.g - 0.1).abs() < 1e-14 && (d.d + 0.01).abs() < 1e-14);
        assert!((d.cross + 0.001).abs() < 1e-14);
        // payable effect = (f0 (1.1)(0.99) - f1) / NAV0 = (100 x 1.089 - 110) / 19900 per share units: f0 = 100, f1 = 110, NAV0 = 19900.
        assert!((d.payable - (100.0 * 1.089 - 110.0) / 19900.0).abs() < 1e-14, "{}", d.payable);
        // The split is exact: nothing is left but the arithmetic.
        assert!(d.rounding.abs() < 1e-14, "{}", d.rounding);
        assert!((d.rest() - (d.cross + d.payable)).abs() < 1e-14);
        // Layer 1: the identity gives the NAV exactly.
        assert!((a.nav_identity() - 19900.0).abs() < 1e-9 && (b.nav_identity() - 21670.0).abs() < 1e-9);
    }

    #[test]
    fn premium_and_its_change_agree() {
        let (l0, l1) = (premium(101.0, 100.0), premium(103.0, 101.0));
        assert!((l0 - 0.01).abs() < 1e-15);
        // The price return 3/101... : price 101 -> 103, NAV 100 -> 101: the ratio form is the change of the level.
        let (diff, ratio) = super::super::fund_attribution::premium_change(103.0 / 101.0 - 1.0, 101.0 / 100.0 - 1.0);
        assert!(((1.0 + l1) / (1.0 + l0) - 1.0 - ratio).abs() < 1e-15);
        assert!((diff - (103.0 / 101.0 - 1.0 - 0.01)).abs() < 1e-15);
    }

    #[test]
    fn the_facts_give_the_quarter_fee_and_the_cross_check() {
        let facts: Value = serde_json::from_str(
            r#"{"facts":{"us-gaap":{
              "SponsorFees":{"units":{"USD":[
                {"start":"2025-10-01","end":"2026-06-30","val":455828000.0},
                {"start":"2026-04-01","end":"2026-06-30","val":149764000.0}]}},
              "AssetsNet":{"units":{"USD":[{"end":"2026-06-30","val":130053278000.0}]}},
              "SharesOutstanding":{"units":{"shares":[{"end":"2026-06-30","val":352000000.0}]}},
              "NetAssetValuePerShare":{"units":{"USD/shares":[{"end":"2026-06-30","val":369.47}]}}}}}"#,
        )
        .unwrap();
        // The 9-month fact is not a quarter.
        assert_eq!(fee_quarter(&facts, "2026-06-30"), Some(149_764_000.0));
        assert_eq!(fee_quarter(&facts, "2026-03-31"), None);
        let html = format!("<html><body><p>{TEXT}</p></body></html>");
        let f = Filing { form: "10-Q".into(), period_end: "2026-06-30".into(), filed: "2026-08-04".into(), accession: "0001437749-26-025680".into(), doc: "x.htm".into() };
        let p = point_of(&f, &html, &facts).unwrap();
        assert_eq!(p.xbrl_note, "");
        assert_eq!(p.fee_q, Some(149_764_000.0));
        // A different XBRL value is named.
        let bad = serde_json::to_string(&facts).unwrap().replace("130053278000.0", "130053279999.0");
        let p = point_of(&f, &html, &serde_json::from_str(&bad).unwrap()).unwrap();
        assert!(p.xbrl_note.contains("AssetsNet"), "{}", p.xbrl_note);
        // A price date far from the period end is refused.
        let f2 = Filing { period_end: "2026-07-31".into(), ..f };
        assert!(point_of(&f2, &html, &facts).is_err());
    }

    #[test]
    fn the_filing_list_keeps_10q_and_10k_only() {
        let j = r#"{"cik":"1222333","name":"SPDR GOLD TRUST","filings":{"recent":{
          "form":["10-K","10-K/A","10-Q","8-K","10-Q"],
          "reportDate":["2025-09-30","2024-09-30","2025-06-30","","2025-06-30"],
          "filingDate":["2025-11-25","2024-12-19","2025-08-06","2025-08-01","2025-08-07"],
          "accessionNumber":["0001437749-25-036305","0001437749-24-037938","0001437749-25-024998","0001437749-25-000001","0001437749-25-000002"],
          "primaryDocument":["gld20250930_10k.htm","a.htm","gld20250630_10q.htm","x.htm","../evil.htm"]}}}"#;
        let (name, v) = parse_submissions(j).unwrap();
        assert_eq!(name, "SPDR GOLD TRUST");
        assert_eq!(v.iter().map(|f| f.accession.as_str()).collect::<Vec<_>>(), ["0001437749-25-036305", "0001437749-25-024998"]);
        assert!(parse_submissions(&j.replace("1222333", "999")).is_err());
        assert_eq!(start_date("2026-09-30").as_deref(), Some("2023-09-30"));
        assert_eq!(doc_url(&v[0]), "https://www.sec.gov/Archives/edgar/data/1222333/000143774925036305/gld20250930_10k.htm");
    }

    #[test]
    fn the_close_is_the_last_one_up_to_five_days_back() {
        let row = |d: &str, c: f64| PricePoint { date: d.into(), open: c, high: c, low: c, close: c, volume: 1 };
        let rows = vec![row("2026-01-02", 10.0), row("2026-01-05", 11.0)];
        assert_eq!(close_before(&rows, "2026-01-05"), Some(("2026-01-05".into(), 11.0)));
        // A Sunday: the close of the Friday before.
        assert_eq!(close_before(&rows, "2026-01-04"), Some(("2026-01-02".into(), 10.0)));
        assert_eq!(close_before(&rows, "2026-01-01"), None);
        assert_eq!(close_before(&rows, "2026-01-11"), None);
    }
}
