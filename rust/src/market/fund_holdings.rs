//! `fa fund holdings TICKER`: the holdings of an ETF from its newest SEC N-PORT report (form NPORT-P).
//!
//! Steps (3 requests for a new ticker, at most 5 in the worst case):
//! 1. The SEC file of fund tickers (`company_tickers_mf.json`: cik, seriesId, classId, symbol) gives the trust and the
//!    series id. It is cached in `<cache>/_sec/`.
//! 2. The EDGAR filing list of the SERIES (the series id is a filer key) gives the newest NPORT-P filings. A trust
//!    files one NPORT-P for each series, so the list of the trust would need many reads.
//! 3. The `primary_doc.xml` of the filing is read and kept in `<cache>/<TICKER>/nport/<accession>.xml`.
//!
//! The SEC publishes a N-PORT report about 60 days after the quarter end, for the third month of the quarter only.
//! The data is public domain.

use super::{today_iso, write_atomic};
use mapcheck::http::Client;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
fa fund holdings - the holdings of an ETF from its newest SEC N-PORT report

USAGE:
  fa fund holdings TICKER [--cache DIR] [--top N]
    --cache DIR   default data/raw. The ticker file is DIR/_sec/company_tickers_mf.json, the report is
                  DIR/TICKER/nport/ACCESSION.xml.
    --top N       the number of holdings shown (default 25), sorted by the signed pctVal (a short position is last)
Source: SEC N-PORT (public domain). The newest public report is about 60 days old.";

const TICKERS_URL: &str = "https://www.sec.gov/files/company_tickers_mf.json";
/// Seconds between two requests (the SEC limit is 10 requests in one second).
const PAUSE: f64 = 0.2;
/// The number of filings of the series that are read before the program stops.
const MAX_FILINGS: usize = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub cik: u64,
    pub series_id: String,
    pub class_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filing {
    pub accession: String,
    pub form: String,
    pub filed: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Holding {
    pub name: String,
    pub cusip: String,
    pub isin: String,
    pub val_usd: f64,
    pub pct_val: f64,
    pub balance: f64,
    pub units: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub series_id: String,
    pub report_date: String,
    pub holdings: Vec<Holding>,
    /// The count of values (pctVal, valUSD, balance) that are missing or not a number (they are 0.0 in the rows).
    pub bad_numbers: usize,
}

/// The series of a ticker in the text of `company_tickers_mf.json`.
pub fn series_of(json: &str, ticker: &str) -> Result<Series, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("company_tickers_mf.json: {e}"))?;
    let rows = v["data"].as_array().ok_or("company_tickers_mf.json: no data array")?;
    for r in rows {
        if r[3].as_str().is_some_and(|s| s.eq_ignore_ascii_case(ticker)) {
            return Ok(Series {
                cik: r[0].as_u64().ok_or("company_tickers_mf.json: bad cik")?,
                series_id: r[1].as_str().unwrap_or("").to_string(),
                class_id: r[2].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Err(format!("{ticker}: not in the SEC file of fund tickers (an ETF or a mutual fund is needed)"))
}

/// The text of the first `<tag>` in `s` (a tag with no prefix), trimmed.
fn tag<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let a = s.find(&open)? + open.len();
    let b = s[a..].find(&format!("</{name}>"))? + a;
    Some(s[a..b].trim())
}

/// The value of the first `<tag attr="..."` in `s`.
fn attr<'a>(s: &'a str, name: &str, attr: &str) -> Option<&'a str> {
    let a = s.find(&format!("<{name} "))?;
    let end = s[a..].find('>')? + a;
    let el = &s[a..end];
    let k = el.find(&format!("{attr}=\""))? + attr.len() + 2;
    let e = el[k..].find('"')? + k;
    Some(&el[k..e])
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';').filter(|e| *e <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "amp" => Some('&'),
            _ => ent
                .strip_prefix("#x")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// An accession number is 10 digits, 2 digits, 6 digits (it goes into a path and a URL).
pub fn valid_accession(a: &str) -> bool {
    let p: Vec<&str> = a.split('-').collect();
    p.len() == 3
        && [10, 2, 6].iter().zip(&p).all(|(n, x)| x.len() == *n && x.bytes().all(|b| b.is_ascii_digit()))
}

/// A ticker is letters, digits, `.` and `-` only.
pub fn valid_ticker(t: &str) -> bool {
    t.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric) && t.len() <= 12 && t.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
}

/// The filings in the Atom answer of EDGAR, in the order of the answer. The type filter of EDGAR is a prefix match:
/// the caller keeps form `NPORT-P` only (an amendment is `NPORT-P/A`).
pub fn parse_filings(atom: &str) -> Vec<Filing> {
    atom.split("<entry>")
        .skip(1)
        .filter_map(|e| {
            Some(Filing {
                accession: tag(e, "accession-number")?.to_string(),
                form: tag(e, "filing-type").unwrap_or("").to_string(),
                filed: tag(e, "filing-date")?.to_string(),
            })
        })
        .collect()
}

/// The report in the text of `primary_doc.xml`.
pub fn parse_report(xml: &str) -> Result<Report, String> {
    let gen = tag(xml, "genInfo").ok_or("no genInfo in the report")?;
    let series_id = tag(gen, "seriesId").ok_or("no seriesId")?.to_string();
    let report_date = tag(xml, "repPdDate").ok_or("no repPdDate")?.to_string();
    let mut bad_numbers = 0usize;
    let mut num = |s: &str, t: &str| match tag(s, t).and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite()) {
        Some(v) => v,
        None => {
            bad_numbers += 1;
            0.0
        }
    };
    let mut holdings = Vec::new();
    for part in xml.split("<invstOrSec>").skip(1) {
        let item = part.split("</invstOrSec>").next().unwrap_or(part);
        holdings.push(Holding {
            name: unescape(tag(item, "name").unwrap_or("")),
            cusip: tag(item, "cusip").unwrap_or("").to_string(),
            isin: attr(item, "isin", "value").unwrap_or("").to_string(),
            val_usd: num(item, "valUSD"),
            pct_val: num(item, "pctVal"),
            balance: num(item, "balance"),
            units: tag(item, "units").unwrap_or("").to_string(),
        });
    }
    Ok(Report { series_id, report_date, holdings, bad_numbers })
}

fn get(client: &mut Client, url: &str, count: &mut usize) -> Result<String, String> {
    *count += 1;
    let b = client.get(url).map_err(|e| e.to_string())?;
    String::from_utf8(b).map_err(|e| format!("{url}: not UTF-8: {e}"))
}

/// What the command line asks for. Nothing is sent before this is checked.
#[derive(Debug, PartialEq)]
pub enum Cmd {
    Help,
    Holdings { ticker: String, cache: Option<PathBuf>, top: usize },
}

pub fn parse_args(args: &[String]) -> Result<Cmd, String> {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => return Ok(Cmd::Help),
        Some("holdings") => {}
        Some(o) => return Err(format!("fa fund: unknown command {o}\n{USAGE}")),
    }
    let t = args.get(1).ok_or("fa fund holdings TICKER")?;
    if t.starts_with('-') {
        return if matches!(t.as_str(), "-h" | "--help") { Ok(Cmd::Help) } else { Err(format!("unknown option {t}\n{USAGE}")) };
    }
    let ticker = t.to_uppercase();
    if !valid_ticker(&ticker) {
        return Err(format!("{t}: not a ticker (letters, digits, . and - only)"));
    }
    let (mut cache, mut top) = (None, 25usize);
    let mut it = args[2..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => return Ok(Cmd::Help),
            "--cache" => cache = Some(PathBuf::from(it.next().ok_or("--cache needs a folder")?)),
            "--top" => top = it.next().and_then(|v| v.parse().ok()).ok_or("--top needs a number")?,
            o => return Err(format!("unknown option {o}")),
        }
    }
    Ok(Cmd::Holdings { ticker, cache, top })
}

fn read_report(client: &mut Client, path: &Path, series: &Series, f: &Filing, requests: &mut usize) -> Result<(Report, bool), String> {
    if let Ok(t) = std::fs::read_to_string(path) {
        if let Ok(r) = parse_report(&t) {
            return Ok((r, true));
        }
    }
    let url = format!("https://www.sec.gov/Archives/edgar/data/{}/{}/primary_doc.xml", series.cik, f.accession.replace('-', ""));
    let t = get(client, &url, requests)?;
    let r = parse_report(&t).map_err(|e| format!("{}: {e}", f.accession))?;
    if r.series_id == series.series_id {
        write_atomic(path, &t)?;
    }
    Ok((r, false))
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    let (ticker, cache, top) = match parse_args(args)? {
        Cmd::Help => {
            println!("{USAGE}");
            return Ok(0);
        }
        Cmd::Holdings { ticker, cache, top } => (ticker, cache.unwrap_or_else(|| repo.join("data").join("raw")), top),
    };
    let mut client = Client::sec(PAUSE)?;
    let mut requests = 0usize;

    let map_path = cache.join("_sec").join("company_tickers_mf.json");
    let map_text = match std::fs::read_to_string(&map_path) {
        Ok(t) if series_of(&t, &ticker).is_ok() => t,
        _ => {
            let t = get(&mut client, TICKERS_URL, &mut requests)?;
            series_of(&t, &ticker)?;
            write_atomic(&map_path, &t)?;
            t
        }
    };
    let series = series_of(&map_text, &ticker)?;

    let list_url = format!(
        "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={}&type=NPORT-P&dateb=&owner=include&count=10&output=atom",
        series.series_id
    );
    let atom = get(&mut client, &list_url, &mut requests)?;
    let trust = tag(&atom, "conformed-name").map(unescape).unwrap_or_default();
    // The filter is a prefix match: keep the form NPORT-P only, with a valid accession number.
    let filings: Vec<Filing> = parse_filings(&atom).into_iter().filter(|f| f.form == "NPORT-P" && valid_accession(&f.accession)).collect();

    // The newest report date among the filings read (the order of the Atom list is not trusted).
    let mut best: Option<(Report, Filing, bool)> = None;
    for f in filings.iter().take(MAX_FILINGS) {
        let path = cache.join(&ticker).join("nport").join(format!("{}.xml", f.accession));
        let (rep, cached) = read_report(&mut client, &path, &series, f, &mut requests)?;
        if rep.series_id != series.series_id {
            continue;
        }
        let newer = best.as_ref().is_none_or(|(b, bf, _)| (&rep.report_date, &f.filed) > (&b.report_date, &bf.filed));
        if newer {
            best = Some((rep, f.clone(), cached));
        }
    }
    let Some((mut rep, f, cached)) = best else {
        return Err(format!("{ticker}: no NPORT-P of series {} found in the newest {MAX_FILINGS} filings", series.series_id));
    };
    rep.holdings.sort_by(|a, b| b.pct_val.total_cmp(&a.pct_val));
    println!("{ticker}  trust {trust} (CIK {})  series {}  report date {}  accession {}  filed {}", series.cik, rep.series_id, rep.report_date, f.accession, f.filed);
    println!(
        "source: SEC N-PORT ({}), {}, {requests} {}",
        today_iso(),
        if cached { "XML from the cache" } else { "XML fetched" },
        if requests == 1 { "request" } else { "requests" }
    );
    if rep.bad_numbers > 0 {
        println!("warning: {} values (pctVal, valUSD, balance) are missing or not numbers; they show as 0", rep.bad_numbers);
    }
    println!("{:<44} {:>10} {:>16}  cusip / isin", "name", "pctVal", "valUSD");
    let mut shown = 0.0;
    for h in rep.holdings.iter().take(top) {
        shown += h.pct_val;
        let id = if h.cusip.is_empty() || h.cusip == "N/A" { &h.isin } else { &h.cusip };
        println!("{:<44} {:>10.4} {:>16.0}  {id}", h.name.chars().take(44).collect::<String>(), h.pct_val, h.val_usd);
    }
    println!("shown {} of {} holdings, sum of pctVal shown {shown:.4}", rep.holdings.len().min(top), rep.holdings.len());
    eprintln!("fa: fund holdings: {requests} requests");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = r#"{"fields":["cik","seriesId","classId","symbol"],"data":[[1,"S1","C1","AAA"],[1432353,"S000029442","C000090393","URA"]]}"#;
    const XML: &str = r#"<edgarSubmission><formData><genInfo><regName>X Funds</regName><seriesId>S000029442</seriesId></genInfo>
<genInfo2><repPdDate>2026-08-31</repPdDate></genInfo2><invstOrSecs>
<invstOrSec><name>Cameco &amp; Co</name><cusip>13321L108</cusip><balance>100.00</balance><units>NS</units><valUSD>2000.50</valUSD><pctVal>20.5</pctVal></invstOrSec>
<invstOrSec><name>Kazatomprom</name><cusip>N/A</cusip><identifiers><isin value="US48581R2058"/></identifiers><balance>5</balance><units>NS</units><valUSD>500</valUSD><pctVal>5.25</pctVal></invstOrSec>
<invstOrSec><name>Nexgen</name><cusip>65340P106</cusip><balance>7</balance><units>NS</units><valUSD>900</valUSD><pctVal>9</pctVal></invstOrSec>
</invstOrSecs></formData></edgarSubmission>"#;

    #[test]
    fn ticker_to_series() {
        let s = series_of(MAP, "ura").unwrap();
        assert_eq!((s.cik, s.series_id.as_str(), s.class_id.as_str()), (1432353, "S000029442", "C000090393"));
        assert!(series_of(MAP, "ZZZ").is_err());
    }

    #[test]
    fn parse_xml() {
        let r = parse_report(XML).unwrap();
        assert_eq!((r.series_id.as_str(), r.report_date.as_str(), r.holdings.len()), ("S000029442", "2026-08-31", 3));
        assert_eq!(r.holdings[0].name, "Cameco & Co");
        assert_eq!(r.holdings[0].pct_val, 20.5);
        assert_eq!(r.holdings[0].val_usd, 2000.5);
        assert_eq!(r.holdings[1].isin, "US48581R2058");
        assert_eq!(r.holdings.iter().map(|h| h.pct_val).sum::<f64>(), 34.75);
    }

    #[test]
    fn parse_atom() {
        let a = "<feed><entry><content><accession-number>0001-26-000002</accession-number><filing-date>2026-09-29</filing-date></content></entry><entry><content><accession-number>0001-26-000001</accession-number><filing-date>2026-06-29</filing-date></content></entry></feed>";
        let f = parse_filings(a);
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].accession.as_str(), f[0].filed.as_str()), ("0001-26-000002", "2026-09-29"));
    }

    #[test]
    fn atom_with_amendment_first_is_filtered_by_form() {
        let a = "<feed><entry><content><accession-number>0001234567-26-000009</accession-number><filing-date>2026-10-05</filing-date><filing-type>NPORT-P/A</filing-type></content></entry><entry><content><accession-number>0001234567-26-000002</accession-number><filing-date>2026-09-29</filing-date><filing-type>NPORT-P</filing-type></content></entry></feed>";
        let f: Vec<Filing> = parse_filings(a).into_iter().filter(|f| f.form == "NPORT-P").collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].accession, "0001234567-26-000002");
    }

    #[test]
    fn bad_numbers_are_counted_and_entities_decoded() {
        let x = XML.replace("<pctVal>9</pctVal>", "<pctVal>x</pctVal>").replace("Nexgen", "It&#39;s &#x27;A&#x27;");
        let r = parse_report(&x).unwrap();
        assert_eq!(r.bad_numbers, 1);
        assert_eq!(r.holdings[2].name, "It's 'A'");
    }

    #[test]
    fn accession_and_ticker_checks() {
        assert!(valid_accession("0002048251-26-008623"));
        assert!(!valid_accession("../../etc-26-008623"));
        assert!(!valid_accession("0002048251-26-00862"));
        assert!(valid_ticker("BRK-B") && valid_ticker("URA"));
        assert!(!valid_ticker("--HELP") && !valid_ticker("../x") && !valid_ticker(""));
    }

    #[test]
    fn bad_arguments_fail_before_any_request() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // `run` checks the arguments first; a refused call never reaches `Client::sec` (no cache folder is made).
        let dir = std::env::temp_dir().join("fa_fund_no_request_test");
        let _ = std::fs::remove_dir_all(&dir);
        let d = dir.to_str().unwrap();
        for a in [v(&["holdings", "--HELP", "--cache", d]), v(&["holdings", "../x", "--cache", d]), v(&["holdings", "URA", "--bogus"])] {
            assert!(run(Path::new("."), &a).is_err(), "{a:?}");
        }
        assert!(!dir.exists());
        assert_eq!(parse_args(&v(&["holdings", "--help"])).unwrap(), Cmd::Help);
        assert_eq!(run(Path::new("."), &v(&["holdings", "-h"])).unwrap(), 0);
    }
}
