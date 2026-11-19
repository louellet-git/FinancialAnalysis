//! Tests of the report. The expected pages come from the Python script (`scripts/visualize_dcf.py`),
//! recorded before the Rust code was written (`rust/tests/fixtures/report/`):
//!
//! - `<case>.skel.html`: the body of the Python page, with a marker `[[FIG height]]` in place of each plotly
//!   figure. It holds each text, table cell, chip and number as text, so a text difference shows here.
//! - `<case>.figs.json`: each plotly figure of that page (data arrays and layout, without the plotly template).
//!   An array of more than 200 numbers is a digest: the shape, the count of empty values, the sum, the sum of
//!   absolute values, the minimum, the maximum and 40 sampled values.
//! - The inputs (company store, prices as `date,close`, option chain, earnings dates) are small copies of the
//!   cache files that the Python script read.
//!
//! No test needs Python or the network.

use super::*;
use crate::market::options::ChainMetrics;
use serde_json::Value;
use std::io::Read;

const TOL: f64 = 1e-9;

fn dir() -> PathBuf {
    PathBuf::from(format!("{}/tests/fixtures/report", env!("CARGO_MANIFEST_DIR")))
}

fn read(name: &str) -> Vec<u8> {
    let path = dir().join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn read_text(name: &str) -> String {
    String::from_utf8(read(name)).unwrap()
}

fn read_gz(name: &str) -> String {
    let mut out = String::new();
    flate2::read::GzDecoder::new(&read(name)[..]).read_to_string(&mut out).unwrap();
    out
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A repository folder in the temp folder with the inputs of one case. The driver models are the real files of
/// the repository (`analysis/<TICKER>/driver_model.toml`).
struct Repo {
    root: PathBuf,
}

impl Repo {
    fn new(case: &str, ticker: &str, store: &str) -> Repo {
        static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("fa-report-test-{}-{case}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(&root.join("data/processed").join(format!("{ticker}.json")), &read_gz(&format!("{store}.store.json.gz")));
        let model = PathBuf::from(format!("{}/../analysis/{ticker}/driver_model.toml", env!("CARGO_MANIFEST_DIR")));
        if model.is_file() {
            write(&root.join("analysis").join(ticker).join("driver_model.toml"), &std::fs::read_to_string(model).unwrap());
        }
        Repo { root }
    }

    fn raw(&self, ticker: &str) -> PathBuf {
        self.root.join("data/raw").join(ticker)
    }

    /// `prices.csv.gz` (`date,close`) becomes `prices.json` (open, high and low equal the close; the report reads dates and closes).
    fn prices(&self, ticker: &str, name: &str) {
        let rows: Vec<String> = read_gz(name)
            .lines()
            .map(|l| {
                let (d, c) = l.split_once(',').unwrap();
                format!("{{\"date\":\"{d}\",\"open\":{c},\"high\":{c},\"low\":{c},\"close\":{c},\"volume\":0}}")
            })
            .collect();
        write(&self.raw(ticker).join("prices.json"), &format!("[{}]", rows.join(",")));
    }

    fn options(&self, ticker: &str, date: &str, text: &str) {
        write(&self.raw(ticker).join("options").join(format!("{date}.json")), text);
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn offline(_: &str) -> Result<Vec<u8>, String> {
    Err("offline".to_string())
}

/// The inputs of a case, as files in a temp repository: (repository, ticker, price, date of the header).
fn setup(case: &str) -> (Repo, String, f64, String) {
    let cases: Value = serde_json::from_str(&read_text("cases.json")).unwrap();
    let c = &cases[case];
    let ticker = c["ticker"].as_str().unwrap().to_string();
    let repo = match case {
        "IBM" | "JNJ" => {
            let r = Repo::new(case, &ticker, &ticker);
            r.prices(&ticker, &format!("{case}.prices.csv.gz"));
            r.options(&ticker, "2026-11-19", &read_gz(&format!("{case}.options.json.gz")));
            write(&r.raw(&ticker).join("yf_earnings.json"), &read_text(&format!("{case}.earnings.json")));
            r
        }
        "KO" => {
            let r = Repo::new(case, &ticker, "KO");
            r.options(&ticker, "2026-11-19", &read_gz("KO.options.json.gz"));
            r
        }
        "KOS" => {
            let r = Repo::new(case, &ticker, "KO");
            r.prices(&ticker, "KOS.prices.csv.gz");
            r.options(&ticker, "2026-11-19", "{\"ticker\":\"KO\",\"snapshot_date\":\"2026-11-19\",\"spot_price\":70.1,\"quotes\":[]}");
            r
        }
        _ => panic!("unknown case {case}"),
    };
    (repo, ticker, c["price"].as_f64().unwrap(), c["as_of"].as_str().unwrap().to_string())
}

fn build_case(case: &str) -> Built {
    let (repo, ticker, price, as_of) = setup(case);
    make_report(&repo.root, &ticker, Some(price), &as_of, &offline).unwrap()
}

// ---------------------------------------------------------------------------
// Skeleton: each text of the page
// ---------------------------------------------------------------------------

/// The words of the Python page that name a Python file or command. Each one is another word in the Rust page.
const TEXT_CHANGES: [(&str, &str); 6] = [
    ("scenarios from financial_analysis/scenarios.py", "scenarios from rust/src/dcf/scenarios.rs"),
    ("/driver_model.py exposing build()", "/driver_model.toml"),
    ("/driver_model.py", "/driver_model.toml"),
    ("Generated by scripts/visualize_dcf.py", "Generated by fa report"),
    ("yfinance was unreachable", "Yahoo was not reachable"),
    ("yfinance at run time", "Yahoo at run time"),
];

fn expected_skeleton(case: &str) -> String {
    let mut s = read_text(&format!("{case}.skel.html"));
    for (from, to) in TEXT_CHANGES {
        s = s.replace(from, to);
    }
    s
}

fn first_difference(a: &str, b: &str) -> String {
    let i = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    let ctx = |s: &str| {
        let from = (0..=i.saturating_sub(120)).rev().find(|k| s.is_char_boundary(*k)).unwrap_or(0);
        let to = (0..=(i + 200).min(s.len())).rev().find(|k| s.is_char_boundary(*k)).unwrap_or(s.len());
        s[from..to].to_string()
    };
    format!("first difference at byte {i}\n expected: {}\n   actual: {}", ctx(a), ctx(b))
}

fn check_skeleton(case: &str) {
    let built = build_case(case);
    let expected = expected_skeleton(case);
    let actual = built.body.trim().to_string();
    assert!(expected.trim() == actual, "{case}: {}", first_difference(expected.trim(), &actual));
}

#[test]
fn ibm_texts_tables_and_chips_equal_the_python_page() {
    check_skeleton("IBM");
}

#[test]
fn jnj_texts_tables_and_chips_equal_the_python_page() {
    check_skeleton("JNJ");
}

#[test]
fn ko_without_prices_and_driver_model_equals_the_python_page() {
    check_skeleton("KO");
}

#[test]
fn short_price_history_and_empty_option_chain_equal_the_python_page() {
    check_skeleton("KOS");
}

// ---------------------------------------------------------------------------
// Figures: each number within 1e-9
// ---------------------------------------------------------------------------

fn fnv(strings: &[String]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for s in strings {
        for b in s.bytes().chain(std::iter::once(1u8)) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{h:016x}")
}

fn flatten<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    match v {
        Value::Array(a) => a.iter().for_each(|x| flatten(x, out)),
        other => out.push(other),
    }
}

fn flat_len(v: &Value) -> usize {
    let mut f = Vec::new();
    flatten(v, &mut f);
    f.len()
}

/// The digest of a long array (the same steps as `digest` of the script that made the fixtures).
fn digest(v: &Value) -> Value {
    let mut shape = Vec::new();
    let mut y = v;
    while let Some(a) = y.as_array() {
        shape.push(a.len());
        match a.first() {
            Some(f) if f.is_array() => y = f,
            _ => break,
        }
    }
    let mut flat = Vec::new();
    flatten(v, &mut flat);
    let n = flat.len();
    let nums: Vec<f64> = flat.iter().filter_map(|x| x.as_f64()).collect();
    let mut out = serde_json::Map::new();
    out.insert("shape".into(), serde_json::json!(shape));
    out.insert("n".into(), serde_json::json!(n));
    let non_null = flat.iter().filter(|x| !x.is_null()).count();
    if nums.len() == non_null && !nums.is_empty() {
        out.insert("nulls".into(), serde_json::json!(n - nums.len()));
        out.insert("sum".into(), serde_json::json!(nums.iter().fold(0.0, |a, b| a + b)));
        out.insert("sumabs".into(), serde_json::json!(nums.iter().fold(0.0, |a, b| a + b.abs())));
        out.insert("min".into(), serde_json::json!(nums.iter().cloned().fold(f64::INFINITY, f64::min)));
        out.insert("max".into(), serde_json::json!(nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)));
    } else if flat.iter().all(|x| x.is_string() || x.is_null()) {
        out.insert("nulls".into(), serde_json::json!(flat.iter().filter(|x| x.is_null()).count()));
        let texts: Vec<String> = flat.iter().map(|x| x.as_str().map_or("\u{2}".to_string(), str::to_string)).collect();
        out.insert("hash".into(), serde_json::json!(fnv(&texts)));
    }
    let k = 40usize;
    let samples: Vec<Value> = (0..k)
        .map(|i| {
            let idx = (i * (n - 1)) / (k - 1);
            serde_json::json!([idx, flat[idx]])
        })
        .collect();
    out.insert("samples".into(), Value::Array(samples));
    Value::Object(out)
}

struct Compare {
    errors: Vec<String>,
    numbers: usize,
    max_rel: f64,
}

impl Compare {
    fn number(&mut self, e: f64, a: f64, path: &str) {
        self.numbers += 1;
        let d = (e - a).abs();
        let rel = if d == 0.0 { 0.0 } else { d / e.abs().max(a.abs()) };
        if d > 1e-12 {
            self.max_rel = self.max_rel.max(rel);
        }
        if d > TOL * e.abs().max(a.abs()) + 1e-12 {
            self.errors.push(format!("{path}: python {e:?} rust {a:?}"));
        }
    }

    /// Every key of the expected object is in the actual one with the same value; the arrays have the same length.
    fn cmp(&mut self, e: &Value, a: &Value, path: &str) {
        if let Some(d) = e.as_object().filter(|o| o.len() == 1 && o.contains_key("__digest")) {
            let want = &d["__digest"];
            if !a.is_array() || flat_len(a) != want["n"].as_u64().unwrap() as usize {
                let shown: String = a.to_string().chars().take(80).collect();
                self.errors.push(format!("{path}: digest of an array of {} values, rust has {shown}", want["n"]));
                return;
            }
            let got = digest(a);
            for (k, wv) in want.as_object().unwrap() {
                if k == "samples" {
                    for (w, g) in wv.as_array().unwrap().iter().zip(got["samples"].as_array().unwrap()) {
                        assert_eq!(w[0], g[0]);
                        self.cmp(&w[1], &g[1], &format!("{path}.sample{}", w[0]));
                    }
                } else {
                    self.cmp(wv, &got[k], &format!("{path}.{k}"));
                }
            }
            return;
        }
        match (e, a) {
            (Value::Number(x), Value::Number(y)) => self.number(x.as_f64().unwrap(), y.as_f64().unwrap(), path),
            (Value::Array(x), Value::Array(y)) => {
                if x.len() != y.len() {
                    self.errors.push(format!("{path}: python has {} items, rust {}", x.len(), y.len()));
                    return;
                }
                for (i, (p, q)) in x.iter().zip(y).enumerate() {
                    self.cmp(p, q, &format!("{path}[{i}]"));
                }
            }
            (Value::Object(x), Value::Object(y)) => {
                for (k, v) in x {
                    match y.get(k) {
                        Some(w) => self.cmp(v, w, &format!("{path}.{k}")),
                        None => self.errors.push(format!("{path}.{k}: missing in rust")),
                    }
                }
            }
            (x, y) => {
                if x != y {
                    self.errors.push(format!("{path}: python {x} rust {y}"));
                }
            }
        }
    }
}

fn check_figures(case: &str, min_figs: usize) -> Compare {
    let built = build_case(case);
    let expected: Value = serde_json::from_str(&read_text(&format!("{case}.figs.json"))).unwrap();
    let expected = expected.as_array().unwrap();
    assert_eq!(built.figs.len(), expected.len(), "{case}: number of figures");
    assert!(expected.len() >= min_figs);
    let mut c = Compare { errors: Vec::new(), numbers: 0, max_rel: 0.0 };
    for (i, (e, f)) in expected.iter().zip(&built.figs).enumerate() {
        assert_eq!(e["height"].as_u64().unwrap() as u32, f.height, "{case} figure {i}: height");
        c.cmp(&e["data"], &f.data, &format!("fig{i}.data"));
        c.cmp(&e["layout"], &f.layout, &format!("fig{i}.layout"));
    }
    println!("{case}: {} figures, {} numbers compared, largest relative difference {:e}", expected.len(), c.numbers, c.max_rel);
    assert!(
        c.errors.is_empty(),
        "{case}: {} differences, the first ones:\n{}",
        c.errors.len(),
        c.errors.iter().take(15).cloned().collect::<Vec<_>>().join("\n")
    );
    c
}

#[test]
fn ibm_figures_equal_the_python_figures() {
    let c = check_figures("IBM", 19);
    assert!(c.numbers > 1_000);
}

#[test]
fn jnj_figures_equal_the_python_figures() {
    let c = check_figures("JNJ", 19);
    assert!(c.numbers > 1_000);
}

#[test]
fn ko_figures_equal_the_python_figures() {
    check_figures("KO", 11);
}

#[test]
fn short_history_figures_equal_the_python_figures() {
    check_figures("KOS", 9);
}

// ---------------------------------------------------------------------------
// Parts
// ---------------------------------------------------------------------------

#[test]
fn the_path_sample_equals_numpy() {
    let cases: Value = serde_json::from_str(&read_text("choice.json")).unwrap();
    for c in cases.as_array().unwrap() {
        let (pop, size, seed) = (c["pop"].as_u64().unwrap() as usize, c["size"].as_u64().unwrap() as usize, c["seed"].as_u64().unwrap());
        let got = crate::dcf::nprandom::Pcg64::new(seed).choice_set(pop, size).unwrap();
        let want: Vec<usize> = c["sorted"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
        assert_eq!(got, want, "pop {pop} size {size} seed {seed}");
    }
    // numpy takes another way for a big population: the program says so and does not guess
    assert!(crate::dcf::nprandom::Pcg64::new(7).choice_set(20_000, 5_000).is_none());
}

#[test]
fn iso_weeks_and_expiry_labels_equal_python() {
    for (d, want) in [
        ("2021-01-03", (2020, 53)),
        ("2026-11-19", (2026, 47)),
        ("2024-12-30", (2025, 1)),
        ("2020-12-31", (2020, 53)),
        ("2016-01-01", (2015, 53)),
        ("2027-01-03", (2026, 53)),
        ("1970-01-01", (1970, 1)),
        ("2000-01-01", (1999, 52)),
    ] {
        assert_eq!(iso_week_of(d), want, "{d}");
    }
    assert_eq!(expiry_label("2026-11-19"), "19 Nov 26");
    assert_eq!(expiry_label("2027-01-05"), "05 Jan 27");
    assert_eq!(add_days("2026-03-01", -1), "2026-02-28");
    assert_eq!(days_between("2026-11-19", "2026-11-21"), 2);
}

#[test]
fn without_a_price_the_report_uses_the_last_close_and_says_so() {
    let (repo, ticker, _, as_of) = setup("IBM");
    let b = make_report(&repo.root, &ticker, None, &as_of, &offline).unwrap();
    assert!(b.body.contains("last close $228.53 (data/raw/IBM/prices.json)"), "{}", &b.body[..600]);
    assert!(b.body.contains("<div class=\"chip-label\">Base P50 vs price</div>"));
}

#[test]
fn without_any_price_the_report_has_no_price_marker_and_no_crash() {
    let (repo, ticker, _, as_of) = setup("KO");
    let b = make_report(&repo.root, &ticker, None, &as_of, &offline).unwrap();
    assert!(b.body.contains("current price unavailable"));
    assert!(b.body.contains("<div class=\"chip-sub\">unavailable</div>"));
    assert!(!b.body.contains("Base P50 vs price"));
    assert!(!b.body.contains("<th>vs price</th>"));
    assert!(b.html().contains("current price unavailable (offline?)"));
}

#[test]
fn a_missing_company_file_gives_the_import_hint() {
    let root = std::env::temp_dir().join(format!("fa-report-test-{}-empty", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let e = make_report(&root, "ZZZ", Some(10.0), "2026-11-19", &offline).err().unwrap();
    assert!(e.contains("No saved data for ZZZ") && e.contains("fa import ZZZ"), "{e}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_ticker_without_an_option_snapshot_gets_the_fetch_hint() {
    let (repo, ticker, price, as_of) = setup("KO");
    std::fs::remove_dir_all(repo.raw(&ticker).join("options")).unwrap();
    let b = make_report(&repo.root, &ticker, Some(price), &as_of, &offline).unwrap();
    assert!(b.body.contains("No option-chain snapshot available \u{2014} run fa analyze KO to capture one."));
    assert!(b.body.contains("options: no snapshot on disk"));
}

#[test]
fn the_page_loads_plotly_from_the_cdn_with_a_fixed_version_and_has_one_script_for_each_figure() {
    let b = build_case("IBM");
    let html = b.html();
    assert!(html.contains("<script src=\"https://cdn.plot.ly/plotly-4.1.1.min.js\" charset=\"utf-8\"></script>"));
    assert_eq!(html.matches("Plotly.newPlot(el,").count(), b.figs.len());
    assert!(!html.contains("[[FIG"));
    // the figure data cannot end the script element: one script for the library, one for each figure, one at the end
    assert_eq!(html.matches("<script").count(), b.figs.len() + 2);
    assert_eq!(html.matches("</script>").count(), b.figs.len() + 2);
    assert!(html.contains("<title>IBM \u{2014} Company report</title>"));
    assert!(html.len() < 4_000_000, "{}", html.len());
}

#[test]
fn the_style_sheet_is_the_one_of_the_python_page() {
    // the sheet is a copy of the one that the Python page had (the tokens of the "Ratings-desk blueprint")
    assert!(STYLE.contains("--accent-deep:#31506e") && STYLE.contains("section.card::before"));
    assert_eq!(STYLE.len(), 4990);
}

// ---------------------------------------------------------------------------
// Ports of the tests of the Python script that no fixture comparison covers
// ---------------------------------------------------------------------------

fn expiry(date: &str, skew: f64) -> ExpiryMetrics {
    ExpiryMetrics { expiry: date.to_string(), skew: Some(skew), ..Default::default() }
}

fn skew_metrics(expiries: Vec<ExpiryMetrics>) -> ChainMetrics {
    ChainMetrics {
        ticker: "ACME".into(),
        snapshot_date: "2026-07-10".into(),
        spot_price: Some(100.0),
        expiries,
        put_call_volume_ratio: None,
        put_call_oi_ratio: None,
    }
}

#[test]
fn the_skew_figure_drops_expiries_that_are_less_than_two_days_out() {
    // wing IVs of dying options are solver noise (a skew of -106 points was seen); only the expiry 7 days out stays
    let fig = skew_figure(&skew_metrics(vec![expiry("2026-07-10", -1.06), expiry("2026-07-11", 0.55), expiry("2026-07-17", 0.07)])).unwrap();
    assert_eq!(fig.data[0]["x"], serde_json::json!(["17 Jul 26"]));
    assert!((fig.data[0]["y"][0].as_f64().unwrap() - 7.0).abs() < 1e-9);
    assert!(skew_figure(&skew_metrics(vec![expiry("2026-07-10", -1.06)])).is_none());
}

fn period(year: i32, revenue: Option<f64>) -> Period {
    Period {
        form: "10-K".into(),
        period_end: format!("{year}-12-31"),
        filed_date: format!("{}-02-15", year + 1),
        revenue,
        operating_cash_flow: revenue.map(|r| r * 0.2),
        capex: revenue.map(|r| -r * 0.02),
        ..Period::default()
    }
}

#[test]
fn fundamentals_without_revenue_and_a_backtest_without_history_give_notes() {
    let mut doc = Doc::default();
    let no_revenue = Company { ticker: "X".into(), company_name: "X".into(), periods: (2020..2025).map(|y| period(y, None)).collect() };
    assert!(fundamentals_section(&mut doc, &no_revenue).contains("Fundamentals not available"));
    let short = Company { ticker: "X".into(), company_name: "X".into(), periods: (2023..2025).map(|y| period(y, Some(1e9))).collect() };
    let s = backtest_section(&mut doc, &short);
    assert!(s.contains("No vintage date has the 3 clean annual periods"), "{s}");
    assert!(doc.figs.is_empty());
}

#[test]
fn the_path_sampler_keeps_every_tail_path_reaches_the_target_and_is_deterministic() {
    let n = 10_000usize;
    let ps: Vec<f64> = (0..n).map(|i| ((i * 7919) % n) as f64).collect();
    let (b, m, t) = stratified_path_sample(&ps, 300, 7).unwrap();
    // P1 = 99.99 and P99 = 9899.01: the values 0..=99 and 9900..=9999 are the tails, and no tail path is left out
    assert_eq!((b.len(), t.len(), m.len()), (100, 100, 100));
    assert!(b.iter().all(|i| ps[*i] < 99.99) && t.iter().all(|i| ps[*i] > 9899.01));
    assert!(m.iter().all(|i| (99.99..=9899.01).contains(&ps[*i])), "the middle stays inside the band");
    assert!(m.windows(2).all(|w| w[0] < w[1]), "sorted and without a repeat");
    assert_eq!(stratified_path_sample(&ps, 300, 7).unwrap().1, m, "the same seed gives the same sample");
    assert_ne!(stratified_path_sample(&ps, 300, 8).unwrap().1, m);
    // a target below the number of tail paths keeps all tails and takes no middle path
    let (b, m, t) = stratified_path_sample(&ps, 50, 7).unwrap();
    assert_eq!((b.len(), m.len(), t.len()), (100, 0, 100));
}

#[test]
fn the_page_uses_the_blueprint_tokens() {
    // the same tokens as the Python page: ground, cards with square corners, one solid chip
    assert!(STYLE.contains("--bg:#f4f4f5") && STYLE.contains("border-radius:0"));
    assert!(STYLE.contains(".chip.accent{background:var(--accent-deep)"));
}
