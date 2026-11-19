//! Tests of `rates`, `bonds` and `verdict_rates`. No test uses the network or Python.
//!
//! `parity_with_python`: the expected values in `tests/fixtures/rates/expected.json` come from
//! the Python code (`financial_analysis/rates.py`, `bonds.py`, `scripts/verdict.py`), run on the
//! same input files before the Rust code was written. The Treasury files, the New York Fed
//! files and the BLS file are real saved answers. The FRED series (`fred.json`) are made up:
//! this container gets no answer from FRED, and ICE data may not be redistributed.

use crate::bonds::{latest, rating_bucket};
use crate::dcf::model::ModelSpec;
use crate::rates::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

fn fixture_dir() -> PathBuf {
    PathBuf::from(format!("{}/tests/fixtures/rates", env!("CARGO_MANIFEST_DIR")))
}

fn read(name: &str) -> String {
    let p = fixture_dir().join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The saved answers, found by the URL; every URL is logged.
struct FixtureNet {
    fred: Value,
    log: Rc<RefCell<Vec<String>>>,
}

impl FixtureNet {
    fn new() -> (FixtureNet, Rc<RefCell<Vec<String>>>) {
        let log = Rc::new(RefCell::new(Vec::new()));
        (FixtureNet { fred: serde_json::from_str(&read("fred.json")).unwrap(), log: log.clone() }, log)
    }
}

impl Net for FixtureNet {
    fn get(&mut self, url: &str) -> Res<String> {
        self.log.borrow_mut().push(url.to_string());
        if url.contains("daily-treasury-rates") {
            let kind = if url.contains("real_yield") { "real" } else { "nominal" };
            let year = url.split("csv/").nth(1).and_then(|r| r.split('/').next()).unwrap_or("");
            let p = fixture_dir().join(format!("treasury_{kind}_{year}.csv"));
            return std::fs::read_to_string(&p).map_err(|_| format!("HTTP 404 for {url}"));
        }
        if url.contains("newyorkfed") {
            let rate = url.split("/rates/").nth(1).and_then(|r| r.split('/').nth(1)).unwrap_or("");
            return Ok(read(&format!("nyfed_{rate}.json")));
        }
        if url.contains("fredgraph") {
            let sid = url.split("id=").nth(1).and_then(|r| r.split('&').next()).unwrap_or("");
            return self.fred.get(sid).and_then(|v| v.as_str()).map(str::to_string).ok_or_else(|| format!("HTTP 404 for {url}"));
        }
        Err(format!("unexpected url {url}"))
    }
    fn post_json(&mut self, url: &str, _body: &str) -> Res<String> {
        self.log.borrow_mut().push(format!("POST {url}"));
        Ok(read("bls_cpi.json"))
    }
}

/// A fake that answers from a function.
type Answer = Box<dyn FnMut(&str) -> Res<String>>;
struct FnNet(Answer, Rc<RefCell<usize>>);

impl Net for FnNet {
    fn get(&mut self, url: &str) -> Res<String> {
        *self.1.borrow_mut() += 1;
        (self.0)(url)
    }
    fn post_json(&mut self, url: &str, _body: &str) -> Res<String> {
        *self.1.borrow_mut() += 1;
        (self.0)(url)
    }
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("fa-rates-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

const TODAY: &str = "2026-11-19";

fn day(s: &str) -> Option<i64> {
    Some(mapcheck::dates::parse(s).unwrap())
}

fn fixture_rates(name: &str) -> (Rates, Rc<RefCell<Vec<String>>>) {
    let (net, log) = FixtureNet::new();
    (Rates::with_net(tmp(name), Box::new(net), day(TODAY).unwrap()), log)
}

// ---------------------------------------------------------------------------
// Parity with the Python output
// ---------------------------------------------------------------------------

fn close(a: &Value, b: &Value, path: &str, bad: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()) {
                match (x.get(k), y.get(k)) {
                    (Some(u), Some(v)) => close(u, v, &format!("{path}/{k}"), bad),
                    (u, v) => bad.push(format!("{path}/{k}: rust {u:?} python {v:?}")),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                bad.push(format!("{path}: length rust {} python {}", x.len(), y.len()));
            } else {
                for (i, (u, v)) in x.iter().zip(y).enumerate() {
                    close(u, v, &format!("{path}[{i}]"), bad);
                }
            }
        }
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            if (x - y).abs() > 1e-9 * y.abs().max(1.0) {
                bad.push(format!("{path}: rust {x:?} python {y:?}"));
            }
        }
        _ => {
            if a != b {
                bad.push(format!("{path}: rust {a} python {b}"));
            }
        }
    }
}

fn s(a: &Value, k: &str) -> Option<String> {
    a.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

fn b(a: &Value, k: &str) -> bool {
    a.get(k).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn to_v<T: serde::Serialize>(r: Res<T>) -> Res<Value> {
    r.map(|v| serde_json::to_value(v).unwrap())
}

fn date_map(r: Res<BTreeMap<i64, f64>>) -> Res<Value> {
    r.map(|m| Value::Object(m.into_iter().map(|(d, v)| (mapcheck::dates::format(d), json!(v))).collect()))
}

fn run_case(kind: &str, a: &Value, rates: &mut Rates) -> Res<Value> {
    let on = match s(a, "on") {
        Some(t) => day(&t),
        None => None,
    };
    let tenor = s(a, "tenor").unwrap_or_else(|| "10Y".into());
    let real = b(a, "real");
    match kind {
        "rates_yield" => to_v(rates.treasury_yield(&tenor, on, real)),
        "rates_curve" => to_v(rates.curve(on, real)),
        "rates_history" => date_map(rates.treasury_history(&tenor, day(&s(a, "start").unwrap()), s(a, "end").and_then(|e| day(&e)), real)),
        "rates_nyfed" => to_v(rates.nyfed_rate(&s(a, "rate").unwrap_or_else(|| "SOFR".into()), on)),
        "rates_inflation" => to_v(rates.inflation_snapshot(on)),
        "rates_snapshot" => rates.snapshot(on).map(|snap| {
            let mut v = serde_json::to_value(snap).unwrap();
            // Python gave no `real` field for the two curves; the Rust type has it.
            for c in ["nominal_curve", "real_curve"] {
                v[c].as_object_mut().unwrap().remove("real");
            }
            v
        }),
        "rates_fred" => date_map(rates.fred_series(&s(a, "series").unwrap(), s(a, "start").and_then(|t| day(&t)), !b(a, "raw"))),
        "rates_check" => to_v(rates.check(a["days"].as_i64().unwrap())),
        "bonds_snapshot" => to_v(rates.credit_snapshot(on)),
        "bonds_spread" => to_v(rates.credit_spread(&s(a, "rating").unwrap(), on)),
        "bonds_yield" => to_v(rates.corporate_yield(&s(a, "rating").unwrap(), on)),
        "bonds_over10y" => to_v(rates.spread_over_10y(&s(a, "rating").unwrap(), on)),
        "bonds_moodys" => to_v(rates.moodys_yield(&s(a, "grade").unwrap(), on)),
        "bonds_hqm" => to_v(rates.hqm_curve(on)),
        "bonds_check" => to_v(rates.cross_check_bonds(a["days"].as_i64().unwrap())),
        "verdict" => {
            let repo = PathBuf::from(format!("{}/..", env!("CARGO_MANIFEST_DIR")));
            let ticker = s(a, "ticker").unwrap();
            let mut spec = ModelSpec::load(&repo, &ticker).map_err(|e| e.0)?.ok_or("no model")?;
            let rf_date = s(a, "rf_date").unwrap();
            let (sp, ctx) = crate::verdict_rates::rates_context(&spec, rates, &rf_date, s(a, "base_date").as_deref())?;
            spec = sp;
            let mut credit = None;
            if let Some(r) = s(a, "rating") {
                let (sp, c) = crate::verdict_rates::credit_context(&spec, rates, &r, &rf_date, ctx.rf.value)?;
                spec = sp;
                credit = Some(c);
            }
            let v = crate::dcf::cli::verdict_for_spec(
                &spec,
                &ticker,
                a["price"].as_f64().unwrap(),
                a["sims"].as_u64().unwrap() as u32,
                a["seed"].as_i64().unwrap(),
                Some(ctx),
                credit,
            )
            .map_err(|e| e.0)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        other => panic!("unknown case kind {other}"),
    }
}

#[test]
fn parity_with_python() {
    let exp: Value = serde_json::from_str(&read("expected.json")).unwrap();
    assert_eq!(exp["today"], TODAY);
    let mut bad = Vec::new();
    let mut n_ok = 0;
    for c in exp["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let (mut rates, _) = fixture_rates(name);
        let got = run_case(c["kind"].as_str().unwrap(), &c["args"], &mut rates);
        match (got, c.get("expect"), c.get("error")) {
            (Ok(v), Some(e), None) => {
                let mut local = Vec::new();
                close(&v, e, name, &mut local);
                bad.extend(local);
                n_ok += 1;
            }
            (Err(m), None, Some(e)) => {
                // Python prints the text of a KeyError inside quotes
                let want = e["message"].as_str().unwrap();
                let want = want.strip_prefix('"').and_then(|w| w.strip_suffix('"')).unwrap_or(want);
                if m != want {
                    bad.push(format!("{name}: error rust '{m}' python '{want}'"));
                }
            }
            (got, e, err) => bad.push(format!("{name}: rust {got:?}, python expect {e:?} error {err:?}")),
        }
        let _ = std::fs::remove_dir_all(tmp(name));
    }
    assert!(bad.is_empty(), "{} differences:\n{}", bad.len(), bad.iter().take(25).cloned().collect::<Vec<_>>().join("\n"));
    assert!(n_ok >= 38, "too few cases compared: {n_ok}");
}

// ---------------------------------------------------------------------------
// Parsers (the tests of tests/test_rates.py)
// ---------------------------------------------------------------------------

const NOMINAL_2004: &str = "Date,\"1 Mo\",\"3 Mo\",\"6 Mo\",\"1 Yr\",\"2 Yr\",\"3 Yr\",\"5 Yr\",\"7 Yr\",\"10 Yr\",\"20 Yr\"\n\
12/31/2004,1.89,2.22,2.59,2.75,3.08,3.25,3.63,3.94,4.24,4.85\n";
const REAL: &str = "Date,\"5 YR\",\"7 YR\",\"10 YR\",\"20 YR\",\"30 YR\"\n11/18/2026,2.66,2.76,2.88,3.12,3.25\n";

#[test]
fn tenor_headers_are_normalized() {
    for (h, e) in [("1 Mo", "1M"), ("1.5 Month", "1.5M"), ("4 Mo", "4M"), ("1 Yr", "1Y"), ("10 Yr", "10Y"), ("10 YR", "10Y"), (" 30 Yr ", "30Y")] {
        assert_eq!(normalize_tenor(h).unwrap(), e, "{h}");
    }
    assert!(normalize_tenor("LT COMPOSITE").is_err());
}

#[test]
fn treasury_csv_gives_decimals_and_skips_blank_cells() {
    let c = parse_treasury_csv(&read("treasury_nominal_2026.csv")).unwrap();
    let jan2 = &c[&day("2026-01-02").unwrap()];
    assert_eq!(jan2["10Y"], 0.0419);
    assert_eq!(jan2["1.5M"], 0.0371);
    let old = &parse_treasury_csv(NOMINAL_2004).unwrap()[&day("2004-12-31").unwrap()];
    assert!(!old.contains_key("30Y") && !old.contains_key("2M"));
    assert_eq!(old["20Y"], 0.0485);
    // a byte order mark and the real curve
    let r = &parse_treasury_csv(&format!("\u{feff}{REAL}")).unwrap()[&day("2026-11-18").unwrap()];
    assert_eq!(r.len(), 5);
    assert_eq!((r["5Y"], r["30Y"]), (0.0266, 0.0325));
    assert!(parse_treasury_csv("<html>maintenance</html>").is_err());
}

#[test]
fn csv_lines_with_quotes() {
    assert_eq!(split_csv_line("Date,\"1 Mo\",\"a,b\",\"say \"\"x\"\"\""), vec!["Date", "1 Mo", "a,b", "say \"x\""]);
}

#[test]
fn nyfed_effr_has_the_target_range() {
    let rows = parse_nyfed_json(
        r#"{ "refRates": [ { "effectiveDate": "2026-11-17", "type": "EFFR", "percentRate": 3.88, "targetRateFrom": 3.75, "targetRateTo": 4.00 },
                           { "effectiveDate": "2026-11-16", "type": "EFFR", "percentRate": 3.87, "targetRateFrom": 3.75, "targetRateTo": 4.00 } ] }"#,
    )
    .unwrap();
    assert_eq!(rows.iter().map(|r| r.day).collect::<Vec<_>>(), vec![day("2026-11-16").unwrap(), day("2026-11-17").unwrap()]);
    assert_eq!((rows[1].value, rows[1].target_from, rows[1].target_to), (0.0388, Some(0.0375), Some(0.04)));
}

#[test]
fn fred_csv_skips_blanks_and_dots() {
    let t = "observation_date,DGS10\n2026-11-10,4.98\n2026-11-11,\n2026-11-12,5.10\n2026-11-13,.\n";
    let s = parse_fred_csv(t, true).unwrap();
    assert_eq!(s.len(), 2);
    assert_eq!((s[&day("2026-11-10").unwrap()], s[&day("2026-11-12").unwrap()]), (0.0498, 0.051));
    let raw = parse_fred_csv("observation_date,CPIAUCNS\n2026-10-01,337.511\n", false).unwrap();
    assert_eq!(raw[&day("2026-10-01").unwrap()], 337.511);
}

fn bls(rows: &[(&str, &str, &str)], status: &str) -> String {
    let data: Vec<Value> = rows.iter().map(|(y, p, v)| json!({"year": y, "period": p, "value": v})).collect();
    json!({"status": status, "message": [], "Results": {"series": [
        {"seriesID": "CUUR0000SA0", "data": data}, {"seriesID": "CUUR0000SA0L1E", "data": data}]}})
    .to_string()
}

#[test]
fn bls_skips_the_annual_average_and_missing_months() {
    let t = bls(&[("2026", "M10", "337.511"), ("2026", "M09", "336.940"), ("2025", "M13", "321.943"), ("2025", "M10", "-"), ("2025", "M09", "324.800")], "REQUEST_SUCCEEDED");
    let s = &parse_bls_json(&t).unwrap()["CUUR0000SA0"];
    assert_eq!(s.keys().copied().collect::<Vec<_>>(), vec![day("2025-09-01").unwrap(), day("2026-09-01").unwrap(), day("2026-10-01").unwrap()]);
    let e = parse_bls_json(&bls(&[], "REQUEST_NOT_PROCESSED")).unwrap_err();
    assert!(e.contains("BLS request failed"), "{e}");
}

#[test]
fn cpi_steps_back_over_a_gap_and_says_so() {
    let t = bls(&[("2026", "M10", "337.511"), ("2026", "M09", "336.940"), ("2025", "M13", "321.943"), ("2025", "M10", "-"), ("2025", "M09", "324.800")], "REQUEST_SUCCEEDED");
    let net = FnNet(Box::new(move |_| Ok(t.clone())), Rc::new(RefCell::new(0)));
    let mut r = Rates::with_net(tmp("cpi-gap"), Box::new(net), day(TODAY).unwrap());
    let o = r.cpi_inflation("headline", day("2026-11-18")).unwrap();
    assert_eq!(o.date, "2026-09-01");
    assert!((o.value - (336.940 / 324.800 - 1.0)).abs() < 1e-6);
    assert!(o.note.contains("Oct 2025"), "{}", o.note);
}

#[test]
fn rounding_is_that_of_python() {
    assert_eq!(round6(0.0507 - 0.0288), 0.0219);
    assert_eq!(round_dp(0.012345678, 4), 0.0123);
    assert_eq!(pct_to_decimal(" 4.17 ").unwrap(), Some(0.0417));
    assert_eq!(pct_to_decimal(".").unwrap(), None);
    assert!(pct_to_decimal("x").is_err());
}

// ---------------------------------------------------------------------------
// Cache and failures
// ---------------------------------------------------------------------------

#[test]
fn a_past_year_comes_from_the_cache_after_one_download() {
    let count = Rc::new(RefCell::new(0));
    let net = FnNet(Box::new(|_| Ok(read("treasury_nominal_2025.csv"))), count.clone());
    let mut r = Rates::with_net(tmp("past-year"), Box::new(net), day(TODAY).unwrap());
    r.load_treasury_curve(2025, false, false).unwrap();
    r.load_treasury_curve(2025, false, false).unwrap();
    assert_eq!(*count.borrow(), 1);
    assert!(r.cache.join("rates").join("treasury_nominal_2025.csv").exists());
}

#[test]
fn a_failed_download_gives_the_old_cache() {
    let dir = tmp("fallback");
    let ok = FnNet(Box::new(|_| Ok(read("treasury_nominal_2026.csv"))), Rc::new(RefCell::new(0)));
    Rates::with_net(dir.clone(), Box::new(ok), day(TODAY).unwrap()).load_treasury_curve(2026, false, false).unwrap();
    let down = FnNet(Box::new(|_| Err("down".into())), Rc::new(RefCell::new(0)));
    let curves = Rates::with_net(dir, Box::new(down), day(TODAY).unwrap()).load_treasury_curve(2026, false, true).unwrap();
    assert!(curves.contains_key(&day("2026-01-05").unwrap()));
}

#[test]
fn a_bad_answer_is_not_cached() {
    let net = FnNet(Box::new(|_| Ok("<html>maintenance</html>".into())), Rc::new(RefCell::new(0)));
    let mut r = Rates::with_net(tmp("bad-payload"), Box::new(net), day(TODAY).unwrap());
    assert!(r.load_treasury_curve(2025, false, false).is_err());
    assert!(!r.cache.join("rates").join("treasury_nominal_2025.csv").exists());
}

#[test]
fn a_refusal_is_an_error_and_is_not_retried() {
    let count = Rc::new(RefCell::new(0));
    let net = FnNet(Box::new(|u| Err(format!("HTTP 403 for {u}"))), count.clone());
    let mut r = Rates::with_net(tmp("refusal"), Box::new(net), day(TODAY).unwrap());
    let e = r.load_fred("DGS10", false).unwrap_err();
    assert!(e.contains("HTTP 403"), "{e}");
    assert_eq!(*count.borrow(), 1, "one request, no retry");
}

#[test]
fn a_missing_tenor_lists_the_available_ones() {
    let net = FnNet(Box::new(|_| Ok(NOMINAL_2004.to_string())), Rc::new(RefCell::new(0)));
    let mut r = Rates::with_net(tmp("tenor"), Box::new(net), day("2004-12-31").unwrap());
    let e = r.treasury_yield("30Y", day("2004-12-31"), false).unwrap_err();
    assert!(e.contains("30Y") && e.contains("['1M', '3M', '6M', '1Y'"), "{e}");
}

#[test]
fn a_new_day_year_end_falls_back_to_the_last_day_of_the_year_before() {
    let (mut r, log) = fixture_rates("year-end");
    let o = r.treasury_yield("10Y", day("2026-01-03"), false).unwrap();
    assert_eq!((o.date.as_str(), o.value), ("2026-01-02", 0.0419));
    let o = r.treasury_yield("10Y", day("2026-01-01"), false).unwrap();
    assert_eq!((o.date.as_str(), o.value), ("2025-12-31", 0.0418));
    assert_eq!(log.borrow().iter().filter(|u| u.contains("/2026/")).count(), 1, "the cache serves the second call");
}

#[test]
fn breakeven_and_5y5y_use_the_fred_formula() {
    // FRED on 2026-11-18: T10YIE 2.19, T5YIFR 2.15
    let (mut r, _) = fixture_rates("breakeven");
    let be = r.breakeven_inflation("10Y", day("2026-11-18")).unwrap();
    assert!((be.value - 0.0219).abs() < 1e-12);
    let fwd = r.forward_inflation_5y5y(day("2026-11-18")).unwrap();
    assert_eq!((fwd.value * 10_000.0).round(), 215.0);
}

// ---------------------------------------------------------------------------
// Bonds (the tests of tests/test_bonds.py)
// ---------------------------------------------------------------------------

#[test]
fn rating_buckets() {
    for (r, e) in [
        ("AAA", "AAA"), ("AA+", "AA"), ("A-", "A"), ("BBB+", "BBB"), ("BB-", "BB"), ("B", "B"), ("CCC+", "CCC"), ("CC", "CCC"), ("C", "CCC"),
        ("Aaa", "AAA"), ("Aa2", "AA"), ("A3", "A"), ("Baa1", "BBB"), ("Ba3", "BB"), ("B2", "B"), ("Caa1", "CCC"), (" A- ", "A"),
    ] {
        assert_eq!(rating_bucket(r).unwrap(), e, "{r}");
    }
    for bad in ["", "A++", "aaa", "D", "Baa4", "investment grade"] {
        assert!(rating_bucket(bad).is_err(), "{bad}");
    }
}

#[test]
fn an_old_value_is_refused() {
    let s: BTreeMap<i64, f64> = [(day("2023-11-20").unwrap(), 0.01)].into();
    assert_eq!(latest(&s, day("2023-11-24").unwrap(), 7, "x").unwrap(), (day("2023-11-20").unwrap(), 0.01));
    assert!(latest(&s, day("2023-12-01").unwrap(), 7, "x").unwrap_err().contains("more than 7 days old"));
    assert!(latest(&s, day("2020-03-23").unwrap(), 7, "x").unwrap_err().contains("data starts 2023-11-20"));
}

#[test]
fn fred_files_are_cached_and_checked_for_the_header() {
    let count = Rc::new(RefCell::new(0));
    let net = FnNet(
        Box::new(|u| {
            if u.contains("id=DBAA") {
                Ok("<html>error</html>\n".to_string())
            } else {
                Ok("observation_date,DAAA\n2026-11-16,6.20\n2026-11-17,6.18\n".to_string())
            }
        }),
        count.clone(),
    );
    let dir = tmp("fred-cache");
    let mut r = Rates::with_net(dir.clone(), Box::new(net), day(TODAY).unwrap());
    let s = r.load_fred("DAAA", false).unwrap();
    assert_eq!(s.len(), 2);
    // a second Rates (a new command) reads the file
    let net2 = FnNet(Box::new(|_| Err("no network".into())), Rc::new(RefCell::new(0)));
    let s2 = Rates::with_net(dir.clone(), Box::new(net2), day(TODAY).unwrap()).load_fred("DAAA", false).unwrap();
    assert_eq!(s, s2);
    assert_eq!(*count.borrow(), 1);
    let e = r.load_fred("DBAA", false).unwrap_err();
    assert!(e.contains("not a FRED CSV"), "{e}");
    assert!(!dir.join("bonds").join("fred_DBAA.csv").exists());
}

#[test]
fn the_spread_uses_the_treasury_of_the_same_day() {
    // ICE has 2026-11-17 as the latest day; the 10Y of 11-17 is 5.10%, not the 11-18 value 5.07%.
    let (mut r, _) = fixture_rates("same-day");
    let o = r.spread_over_10y("A-", day("2026-11-18")).unwrap();
    assert_eq!(o.date, "2026-11-17");
    let y = r.corporate_yield("A-", day("2026-11-18")).unwrap();
    assert!((o.value - round6(y.value - 0.051)).abs() < 1e-12);
    assert_eq!(o.note, "A- uses the A index");
}

// ---------------------------------------------------------------------------
// The model changes (the tests of tests/test_verdict_rates.py)
// ---------------------------------------------------------------------------

fn small_model() -> ModelSpec {
    ModelSpec::from_toml(
        r#"
net_debt = 0.0
shares = 1.0
[[drivers]]
name = "rf"
stochastic = true
mean = 0.046
std = 0.005
dist = "truncnorm"
lo = 0.01
rationale = "old"
[[drivers]]
name = "erp"
stochastic = true
mean = 0.045
std = 0.0075
[[drivers]]
name = "kd_spread"
stochastic = false
mean = -0.001
rationale = "ladder"
[[corr]]
a = "rf"
b = "erp"
rho = -0.3
"#,
    )
    .unwrap()
}

fn obs(series: &str, d: &str, v: f64, source: &str) -> crate::RateObservation {
    crate::RateObservation { series: series.into(), date: d.into(), value: v, source: source.into(), note: String::new() }
}

#[test]
fn with_risk_free_changes_only_the_mean_and_the_rationale() {
    use crate::verdict_rates::with_risk_free;
    let m = small_model();
    let new = with_risk_free(&m, &obs("UST 10Y", "2026-11-18", 0.0507, "U.S. Treasury")).unwrap();
    let rf = new.driver("rf").unwrap();
    assert_eq!((rf.mean, rf.std, rf.lo, rf.dist.as_str()), (0.0507, 0.005, Some(0.01), "truncnorm"));
    assert!(rf.rationale.contains("2026-11-18") && rf.rationale.contains("4.60%") && rf.rationale.contains("old"), "{}", rf.rationale);
    assert_eq!(new.driver("erp").unwrap(), m.driver("erp").unwrap());
    assert_eq!(new.corr, m.corr);
    assert_eq!(m.driver("rf").unwrap().mean, 0.046, "the original model does not change");
}

#[test]
fn with_kd_spread_changes_only_the_spread() {
    use crate::verdict_rates::with_kd_spread;
    let m = small_model();
    let new = with_kd_spread(&m, &obs("A yield minus UST 10Y", "2026-11-17", 0.0067, "ICE BofA via FRED")).unwrap();
    let kd = new.driver("kd_spread").unwrap();
    assert_eq!(kd.mean, 0.0067);
    assert!(kd.rationale.contains("-0.10%") && kd.rationale.contains("ladder"), "{}", kd.rationale);
    assert_eq!(m.driver("kd_spread").unwrap().mean, -0.001);
}

#[test]
fn the_observation_text_is_that_of_python() {
    let mut o = obs("UST 10Y", "2026-11-18", 0.0507, "U.S. Treasury");
    assert_eq!(obs_text(&o), "UST 10Y 5.07% on 2026-11-18 (U.S. Treasury)");
    o.note = "gap".into();
    assert_eq!(obs_text(&o), "UST 10Y 5.07% on 2026-11-18 (U.S. Treasury) [gap]");
}
