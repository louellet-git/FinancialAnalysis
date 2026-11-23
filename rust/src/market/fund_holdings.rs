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
    --top N       the number of holdings shown (default 25)
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
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// The filings in the Atom answer of EDGAR, newest first.
pub fn parse_filings(atom: &str) -> Vec<Filing> {
    atom.split("<entry>")
        .skip(1)
        .filter_map(|e| {
            Some(Filing { accession: tag(e, "accession-number")?.to_string(), filed: tag(e, "filing-date")?.to_string() })
        })
        .collect()
}

/// The report in the text of `primary_doc.xml`.
pub fn parse_report(xml: &str) -> Result<Report, String> {
    let gen = tag(xml, "genInfo").ok_or("no genInfo in the report")?;
    let series_id = tag(gen, "seriesId").ok_or("no seriesId")?.to_string();
    let report_date = tag(xml, "repPdDate").ok_or("no repPdDate")?.to_string();
    let num = |s: &str, t: &str| tag(s, t).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
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
    Ok(Report { series_id, report_date, holdings })
}

fn get(client: &mut Client, url: &str, count: &mut usize) -> Result<String, String> {
    *count += 1;
    let b = client.get(url).map_err(|e| e.to_string())?;
    String::from_utf8(b).map_err(|e| format!("{url}: not UTF-8: {e}"))
}

pub fn run(repo: &Path, args: &[String]) -> Result<i32, String> {
    if matches!(args.first().map(String::as_str), None | Some("-h" | "--help" | "help")) {
        println!("{USAGE}");
        return Ok(0);
    }
    if args[0] != "holdings" {
        return Err(format!("fa fund: unknown command {}\n{USAGE}", args[0]));
    }
    let ticker = args.get(1).ok_or("fa fund holdings TICKER")?.to_uppercase();
    let (mut cache, mut top) = (repo.join("data").join("raw"), 25usize);
    let mut it = args[2..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--cache" => cache = PathBuf::from(it.next().ok_or("--cache needs a folder")?),
            "--top" => top = it.next().and_then(|v| v.parse().ok()).ok_or("--top needs a number")?,
            o => return Err(format!("unknown option {o}")),
        }
    }
    let mut client = Client::sec(PAUSE)?;
    let mut requests = 0usize;

    let map_path = cache.join("_sec").join("company_tickers_mf.json");
    let map_text = match std::fs::read_to_string(&map_path) {
        Ok(t) if series_of(&t, &ticker).is_ok() => t,
        _ => {
            let t = get(&mut client, TICKERS_URL, &mut requests)?;
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
    let filings = parse_filings(&atom);
    let trust = tag(&atom, "conformed-name").map(unescape).unwrap_or_default();

    for f in filings.iter().take(MAX_FILINGS) {
        let path = cache.join(&ticker).join("nport").join(format!("{}.xml", f.accession));
        let xml = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                let url = format!(
                    "https://www.sec.gov/Archives/edgar/data/{}/{}/primary_doc.xml",
                    series.cik,
                    f.accession.replace('-', "")
                );
                let t = get(&mut client, &url, &mut requests)?;
                if parse_report(&t).is_ok_and(|r| r.series_id == series.series_id) {
                    write_atomic(&path, &t)?;
                }
                t
            }
        };
        let mut rep = parse_report(&xml)?;
        if rep.series_id != series.series_id {
            continue;
        }
        rep.holdings.sort_by(|a, b| b.pct_val.total_cmp(&a.pct_val));
        println!("{ticker}  trust {trust} (CIK {})  series {}  report date {}  accession {}  filed {}", series.cik, rep.series_id, rep.report_date, f.accession, f.filed);
        println!("source: SEC N-PORT, fetched {} ({requests} requests)", today_iso());
        println!("{:<44} {:>10} {:>16}  cusip / isin", "name", "pctVal", "valUSD");
        let mut shown = 0.0;
        for h in rep.holdings.iter().take(top) {
            shown += h.pct_val;
            let id = if h.cusip.is_empty() || h.cusip == "N/A" { &h.isin } else { &h.cusip };
            println!("{:<44} {:>10.4} {:>16.0}  {id}", h.name.chars().take(44).collect::<String>(), h.pct_val, h.val_usd);
        }
        println!("shown {} of {} holdings, sum of pctVal shown {shown:.4}", rep.holdings.len().min(top), rep.holdings.len());
        eprintln!("fa: fund holdings: {requests} requests");
        return Ok(0);
    }
    Err(format!("{ticker}: no NPORT-P of series {} found in the newest {MAX_FILINGS} filings", series.series_id))
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
}
