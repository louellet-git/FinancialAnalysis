//! IMF Primary Commodity Prices (PCPS): the uranium series, monthly, from the official IMF data API (SDMX 2.1).
//!
//! `fa commodity get uranium` makes ONE request for the one series `G001.PURAN.USD.M` (world, uranium, US dollars, monthly)
//! and keeps the answer (XML) with a record. `fa commodity show uranium` prints the months.
//! Terms (copyright page of the IMF, read by the Head of Staff on 2026-11-26, section "Special Terms for Statistical
//! Data"): "Primary Commodity Prices" is Data; users may download, copy and use it; the data must appear accurately
//! with attribution to the IMF. The IMF prohibits bulk download by automated technology: this loader asks for one
//! series only, never for the whole dataflow.
//! Definition (structure of the dataflow, read 2026-11-26): "Uranium, NUEXCO, Restricted Price, Nuexco exchange spot,
//! US$ per pound"; monthly averages converted from weekly or daily values.

use super::{today_iso, write_atomic};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The one address that the program asks. The host is fixed in the code, not read from an answer.
pub const URL: &str = "https://api.imf.org/external/sdmx/2.1/data/PCPS/G001.PURAN.USD.M";
pub const PAUSE: f64 = 3.0;
pub const SOURCE: &str = "International Monetary Fund, Primary Commodity Price System (PCPS), series G001.PURAN.USD.M";
pub const ATTRIBUTION: &str = "Source: International Monetary Fund, Primary Commodity Prices, https://data.imf.org/en/datasets//IMF.RES:PCPS";
pub const DEFINITION: &str = "Uranium, NUEXCO, Restricted Price, Nuexco exchange spot, US$ per pound (monthly average of weekly or daily values; the IMF names NUEXCO as the price source)";
pub const TERMS: &str = "IMF copyright page, section Special Terms for Statistical Data: download, copy and use permitted with attribution to the IMF; no bulk download by automated technology (https://www.imf.org/en/About/copyright-and-terms)";
const STEM: &str = "PCPS_G001.PURAN.USD.M";

pub fn dir(cache: &Path) -> PathBuf {
    cache.join("commodity").join("imf")
}

#[derive(Debug, PartialEq)]
pub struct Parsed {
    /// "YYYY-MM" to US dollars per pound.
    pub months: BTreeMap<String, f64>,
    /// The `UPDATE_DATE` of the data set (the IMF publication time).
    pub updated: String,
}

fn xml_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    crate::futures::mpt::attr(tag, name)
}

/// "2026-M08" to "2026-08".
pub fn month_of(p: &str) -> Option<String> {
    let (y, m) = p.split_once("-M")?;
    let (yn, mn) = (y.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (y.len() == 4 && m.len() == 2 && (1..=12).contains(&mn)).then(|| format!("{yn:04}-{mn:02}"))
}

/// Read the answer. The function checks the form: one series with the four expected dimension values.
pub fn parse(xml: &str) -> Result<Parsed, String> {
    let ds = xml.find("<message:DataSet ").ok_or("the answer has no DataSet (not an SDMX data answer)")?;
    let ds_tag = &xml[ds..ds + xml[ds..].find('>').unwrap_or(0)];
    let updated = xml_attr(ds_tag, "UPDATE_DATE").unwrap_or("").to_string();
    let mut series = xml[ds..].split("<Series ").skip(1);
    let first = series.next().ok_or("the answer has no series: the key did not match (HTTP 200 with an empty data set)")?;
    if series.next().is_some() {
        return Err("the answer has more than one series: the request key is wrong".into());
    }
    let tag = &first[..first.find('>').unwrap_or(0)];
    for (k, v) in [("COUNTRY", "G001"), ("INDICATOR", "PURAN"), ("DATA_TRANSFORMATION", "USD"), ("FREQUENCY", "M")] {
        if xml_attr(tag, k) != Some(v) {
            return Err(format!("the series has {k}={:?}, not {v}: the form of the answer changed", xml_attr(tag, k)));
        }
    }
    let mut months = BTreeMap::new();
    for obs in first.split("<Obs ").skip(1) {
        let t = &obs[..obs.find('>').unwrap_or(obs.len())];
        let (Some(p), Some(v)) = (xml_attr(t, "TIME_PERIOD"), xml_attr(t, "OBS_VALUE")) else { continue };
        let m = month_of(p).ok_or_else(|| format!("TIME_PERIOD {p}: not a month"))?;
        months.insert(m, v.parse::<f64>().map_err(|_| format!("OBS_VALUE {v}: not a number"))?);
    }
    if months.is_empty() {
        return Err("the series has no observations".into());
    }
    Ok(Parsed { months, updated })
}

pub fn format_rows(p: &Parsed, from: Option<&str>) -> String {
    let mut s = format!("{:<8} {:>10}\n", "month", "USD/lb");
    for (m, v) in p.months.iter().filter(|(m, _)| from.is_none_or(|f| m.as_str() >= f)) {
        s += &format!("{m:<8} {v:>10.2}\n");
    }
    s
}

pub fn get(cache: &Path) -> Result<i32, String> {
    let d = dir(cache);
    let day = today_iso();
    let body = crate::hostnet::get(URL, &[("Accept", "application/xml")], PAUSE)?;
    let text = String::from_utf8(body.clone()).map_err(|_| format!("the IMF answer is not UTF-8 ({} bytes, starts with {:02x?})", body.len(), &body[..body.len().min(8)]))?;
    let parsed = parse(&text).map_err(|e| format!("{e} (answer: {} bytes, starts with {:?})", body.len(), text.chars().take(200).collect::<String>()))?;
    let (first, last) = (parsed.months.keys().next().cloned().unwrap_or_default(), parsed.months.keys().next_back().cloned().unwrap_or_default());
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    let name = format!("{STEM}_{day}.xml");
    std::fs::write(d.join(&name), &body).map_err(|e| format!("{}: {e}", d.join(&name).display()))?;
    let sha = crate::portfolio::harness::sha256_hex(&body);
    let meta = serde_json::json!({
        "source": SOURCE, "address": URL, "date": day, "bytes": body.len(), "sha256": sha,
        "terms": TERMS, "attribution": ATTRIBUTION, "definition": DEFINITION,
        "series": "uranium", "months": parsed.months.len(), "first": first, "last": last, "imf_update_date": parsed.updated,
        "note": "private cache; a later download can revise old values",
    });
    write_atomic(&d.join(format!("{STEM}_{day}.meta.json")), &(serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())? + "\n"))?;
    println!("saved {name} ({} bytes, sha256 {sha}); uranium: {} months, {first} to {last}; IMF update {}", body.len(), parsed.months.len(), parsed.updated);
    println!("requests: 1");
    Ok(0)
}

pub fn show(cache: &Path, file: Option<&str>, from: Option<&str>) -> Result<i32, String> {
    let path = match file {
        Some(f) => PathBuf::from(f),
        None => super::spf::newest(&dir(cache), &format!("{STEM}_"), ".xml").ok_or_else(|| format!("no {STEM}_*.xml in {}: run fa commodity get uranium", dir(cache).display()))?,
    };
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let sha = crate::portfolio::harness::sha256_hex(&bytes);
    let meta_path = path.with_extension("meta.json");
    match std::fs::read_to_string(&meta_path).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
        Some(m) if m["sha256"].as_str() == Some(sha.as_str()) => {}
        Some(_) => eprintln!("fa: WARNING the SHA-256 of {} differs from its record: the file changed after the download", path.display()),
        None => eprintln!("fa: no record file {}: the origin of the copy is not known", meta_path.display()),
    }
    let parsed = parse(&String::from_utf8(bytes).map_err(|_| "the file is not UTF-8")?)?;
    eprintln!("fa: {} ({} months; IMF update {})", path.file_name().and_then(|n| n.to_str()).unwrap_or(""), parsed.months.len(), parsed.updated);
    eprintln!("fa: definition: {DEFINITION}");
    eprintln!("fa: {ATTRIBUTION}");
    print!("{}", format_rows(&parsed, from));
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The answer of the IMF API for `G001.PURAN.USD.M?lastNObservations=3` (2026-11-26), with the long text attributes cut.
    const ANSWER: &str = "<?xml version='1.0' encoding='UTF-8'?><message:StructureSpecificData xmlns:message=\"x\"><message:Header><message:ID>DS1</message:ID></message:Header><message:DataSet ss:dataScope=\"DataStructure\" action=\"Replace\" LANGUAGE=\"EN\" PUBLISHER=\"IMF\" UPDATE_DATE=\"2026-11-10T18:30:55.319378200Z\" LICENSE=\"&#169; International Monetary Fund Copyright. All Rights Reserved. https://www.imf.org/external/terms.htm\"><Series COUNTRY=\"G001\" INDICATOR=\"PURAN\" DATA_TRANSFORMATION=\"USD\" FREQUENCY=\"M\" SCALE=\"0\"><Obs TIME_PERIOD=\"2026-M08\" OBS_VALUE=\"71.09474744380952\" DERIVATION_TYPE=\"R\" ACCESS_SHARING_LEVEL=\"PUBLIC_OPEN\" SECURITY_CLASSIFICATION=\"PUB\"/><Obs TIME_PERIOD=\"2026-M09\" OBS_VALUE=\"72.47442565545455\" DERIVATION_TYPE=\"R\" ACCESS_SHARING_LEVEL=\"PUBLIC_OPEN\" SECURITY_CLASSIFICATION=\"PUB\"/><Obs TIME_PERIOD=\"2026-M10\" OBS_VALUE=\"72.887620785\" DERIVATION_TYPE=\"R\" ACCESS_SHARING_LEVEL=\"PUBLIC_OPEN\" SECURITY_CLASSIFICATION=\"PUB\"/></Series></message:DataSet></message:StructureSpecificData>";

    /// A key that matches no series: HTTP 200, a data set with no Series (real answer, 2026-11-26, cut).
    const EMPTY: &str = "<message:StructureSpecificData><message:DataSet UPDATE_DATE=\"2026-11-10T18:30:55.319378200Z\" LICENSE=\"x\"/></message:StructureSpecificData>";

    #[test]
    fn the_real_answer_gives_three_months() {
        let p = parse(ANSWER).unwrap();
        assert_eq!(p.months.len(), 3);
        assert_eq!(p.months["2026-08"], 71.09474744380952);
        assert_eq!(p.months["2026-09"], 72.47442565545455);
        assert_eq!(p.months["2026-10"], 72.887620785);
        assert_eq!(p.updated, "2026-11-10T18:30:55.319378200Z");
        assert_eq!(format_rows(&p, Some("2026-09")), "month        USD/lb\n2026-09       72.47\n2026-10       72.89\n");
    }

    #[test]
    fn an_empty_or_changed_answer_is_an_error() {
        assert!(parse(EMPTY).unwrap_err().contains("no series"));
        assert!(parse("<html>Access Denied</html>").is_err());
        assert!(parse(&ANSWER.replace("INDICATOR=\"PURAN\"", "INDICATOR=\"PGOLD\"")).unwrap_err().contains("INDICATOR"));
        assert!(parse(&ANSWER.replace("DATA_TRANSFORMATION=\"USD\"", "DATA_TRANSFORMATION=\"INDEX\"")).unwrap_err().contains("DATA_TRANSFORMATION"));
        assert!(parse(&ANSWER.replace("2026-M08", "2026-Q3")).unwrap_err().contains("not a month"));
    }

    #[test]
    fn the_address_is_the_imf_api_host_with_one_series_key() {
        assert_eq!(crate::hostnet::group_of_url(URL), "imf");
        assert!(URL.starts_with("https://api.imf.org/external/sdmx/2.1/data/PCPS/G001.PURAN."));
        assert_eq!(month_of("1990-M01").as_deref(), Some("1990-01"));
        assert_eq!(month_of("1990-M13"), None);
        assert!(ATTRIBUTION.starts_with("Source: International Monetary Fund, Primary Commodity Prices, https://"));
    }
}
