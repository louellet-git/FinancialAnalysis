//! `fa report TICKER`: the single-page company report as an HTML file (the Rust form of
//! `scripts/visualize_dcf.py`).
//!
//! Sections, in this order: header band, price and valuation, fundamentals, aggregate DCF
//! scenarios, driver-based model, backtest, options market, footer. A section that has no data
//! shows a note with the reason; it never stops the report.
//!
//! The charts are plotly figures that this program writes as JSON (`figs`). The page draws them
//! with the plotly.js library, which it loads from the official CDN with a fixed version
//! (`PLOTLY_JS_URL`). The page needs the network when a person opens it. The Python page had the
//! library inside the file (about 3.5 MB); a Rust option for an offline page is an open point.
//!
//! The random numbers are the numpy numbers (`dcf::nprandom`), with the same seeds and the same
//! order of draws as the Python script, so each simulated value is the same.

pub mod figs;
pub mod fmt;
#[cfg(test)]
mod tests;

use crate::dcf::backtest::{clean_annuals, run_backtest, BacktestResult};
use crate::dcf::classic::{derive_fcf_margin, run_classic, Company, DcfAssumptions, Period};
use crate::dcf::driver::DriverDcfResult;
use crate::dcf::model::ModelSpec;
use crate::dcf::scenarios::{capital_structure_for, latest_annual, scenarios_for, CapitalStructure, Scenario};
use crate::dcf::stats::{percentile, sorted_copy};
use crate::market::options::{compute_chain_metrics, list_snapshots, load_snapshot, ChainMetrics, ExpiryMetrics, OptionChainSnapshot};
use crate::market::prices::{self, PricePoint};
use figs::*;
use fmt::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The plotly.js version that the page loads. It is the version that the Python library plotly (7.1.0)
/// inlined in the old report.
pub const PLOTLY_VERSION: &str = "4.1.1";
pub const PLOTLY_JS_URL: &str = "https://cdn.plot.ly/plotly-4.1.1.min.js";

pub const DRIVER_SIMS: usize = 10_000;
pub const DRIVER_SEED: u64 = 7;
const BACKTEST_HORIZON: usize = 3;
const BACKTEST_SIMS: usize = 10_000;
const BACKTEST_SEED: u64 = 7;
const ANNUAL_FORMS: [&str; 2] = ["10-K", "10-K/A"];

const STYLE: &str = include_str!("style.css");

pub const SECTION_TITLES: [&str; 6] = [
    "Price & valuation",
    "Fundamentals",
    "Aggregate DCF scenarios",
    "Driver-based model",
    "Backtest \u{2014} mechanical vintages",
    "Options market",
];

pub const USAGE: &str = "\
fa report TICKER [--price P] [--out FILE]

Write the single-page company report as an HTML file (default: data/processed/<TICKER>_dcf.html).
Sections: price and valuation, fundamentals, aggregate DCF scenarios, driver-based model, backtest, options market.
The program reads data/processed/<TICKER>.json (run `fa import TICKER` first), the cache files under
data/raw/<TICKER>/ (prices, option chain, earnings dates) and analysis/<TICKER>/driver_model.toml.

  --price P   the share price of the report (without it: the last close of the price history; the program
              asks the Yahoo chart endpoint when there is no cache file)
  --out FILE  the output file

The charts are drawn with the plotly.js library (version 4.1.1), which the page loads from
https://cdn.plot.ly: the page needs the network when you open it.";

// ---------------------------------------------------------------------------
// HTML parts
// ---------------------------------------------------------------------------

/// The figures of a page, in order. The body keeps a marker `[[FIG height]]` for each figure.
#[derive(Default)]
struct Doc {
    figs: Vec<Fig>,
}

impl Doc {
    fn fig(&mut self, f: Fig) -> String {
        let marker = format!("[[FIG {}]]", f.height);
        self.figs.push(f);
        marker
    }
}

fn table(head: &str, rows: &str) -> String {
    format!("<div class=\"tblwrap\"><table>{head}{rows}</table></div>")
}

fn card(title: &str, sub: &str, body: &str, wide: bool) -> String {
    let cls = if wide { "card wide" } else { "card" };
    format!("<section class=\"{cls}\"><h3>{}</h3><p class=\"sub\">{}</p>{body}</section>", esc(title), esc(sub))
}

fn note(text: &str) -> String {
    format!("<p class=\"note\">{}</p>", esc(text))
}

fn section(title: &str, body: &str) -> String {
    format!("<section class=\"rsec\"><h2 class=\"section-h\">{}</h2><div class=\"grid\">{body}</div></section>", esc(title))
}

fn chip(label: &str, value: &str, sub: &str, value_color: Option<&str>, accent: bool) -> String {
    let style = value_color.map(|c| format!(" style=\"color:{c}\"")).unwrap_or_default();
    let cls = if accent { "chip accent" } else { "chip" };
    let sub_html = if sub.is_empty() { String::new() } else { format!("<div class=\"chip-sub\">{}</div>", esc(sub)) };
    format!("<div class=\"{cls}\"><div class=\"chip-label\">{}</div><div class=\"chip-value\"{style}>{}</div>{sub_html}</div>", esc(label), esc(value))
}

const DELTA_UP: &str = "#1c6b4f";
const DELTA_DOWN: &str = "#a13326";

fn delta_color(v: f64) -> &'static str {
    if v >= 0.0 { DELTA_UP } else { DELTA_DOWN }
}

/// Text of a missing value in a table cell.
const DASH: &str = "\u{2014}";

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

/// What the report reads besides the company file: the paths and the market data.
pub struct Inputs<'a> {
    pub repo: &'a Path,
    pub ticker: &'a str,
    pub price: Option<f64>,
    pub price_note: &'a str,
    /// The date in the header (`YYYY-MM-DD`).
    pub as_of: &'a str,
    /// The price history, if there is one.
    pub prices: Option<&'a [PricePoint]>,
}

/// The page body (header, sections, footer) with a marker for each figure, and the figures.
pub struct Built {
    pub ticker: String,
    pub body: String,
    pub figs: Vec<Fig>,
}

fn cache_dir(repo: &Path) -> PathBuf {
    repo.join("data").join("raw")
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn header_html(company: &Company, cap: &CapitalStructure, scenarios: &[Scenario], stats: &Stats, price: Option<f64>, price_note: &str, as_of: &str) -> String {
    let mut chips = Vec::new();
    match price {
        Some(p) => {
            chips.push(chip("Last price", &format!("${}", fixed_c(p, 2)), "", None, false));
            chips.push(chip("Market cap", &money_b(Some(p * cap.shares), 0), "", None, false));
        }
        None => chips.push(chip("Last price", DASH, "unavailable", None, false)),
    }
    for sc in scenarios {
        let p50 = stat(stats, sc.key).expect("stats")[1];
        chips.push(chip(&format!("{} P50", title_case(sc.key)), &format!("${}", fixed_c(p50, 0)), "per share", None, false));
    }
    if let (Some(p), Some(b)) = (price, stat(stats, "base")) {
        let delta = (b[1] - p) / p;
        chips.push(chip("Base P50 vs price", &pct_signed(delta, 0), "implied up/downside", None, true));
    }
    let meta = format!(
        "As of {as_of} \u{b7} net debt {} \u{b7} {}B shares \u{b7} {}",
        money_b(Some(cap.net_debt), 1),
        fixed(cap.shares / 1e9, 3),
        esc(price_note)
    );
    format!(
        "<header><h1>{} \u{2014} Company report</h1><p class=\"meta\">{}</p><p class=\"meta\">{meta}</p><div class=\"chips\">{}</div></header>",
        esc(&company.ticker.to_uppercase()),
        esc(&company.company_name),
        chips.concat()
    )
}

// ---------------------------------------------------------------------------
// Price and valuation
// ---------------------------------------------------------------------------

fn price_section(doc: &mut Doc, ticker: &str, prices: Option<&[PricePoint]>, scenarios: &[Scenario], stats: &Stats) -> String {
    let points = prices.filter(|p| !p.is_empty());
    let Some(points) = points else {
        return section(
            SECTION_TITLES[0],
            &card(
                "Price history",
                "Daily closes",
                &note(&format!(
                    "Price history not available \u{2014} no cached prices under data/raw/{}/prices.json and Yahoo was not reachable.",
                    ticker.to_uppercase()
                )),
                true,
            ),
        );
    };
    let weekly_note = if points.len() > WEEKLY_LIMIT { " (weekly resolution before the trailing year to keep the file light)" } else { "" };
    let mut cards = card(
        "Price history",
        &format!(
            "Closes {} \u{2192} {}{weekly_note} \u{2014} range buttons top left, linear/log toggle top right",
            points[0].date,
            points[points.len() - 1].date
        ),
        &doc.fig(price_history_figure(points)),
        true,
    );
    cards += &card(
        "Price vs valuation",
        "Closes with each scenario's P50 per-share value and the base P10\u{2013}P90 band \u{2014} defaults to the trailing 6 months, range buttons 1M\u{2013}2Y",
        &doc.fig(valuation_zoom_figure(points, scenarios, stats)),
        true,
    );
    section(SECTION_TITLES[0], &cards)
}

// ---------------------------------------------------------------------------
// Fundamentals
// ---------------------------------------------------------------------------

fn fundamental_annuals(company: &Company, max_years: usize) -> Vec<&Period> {
    let mut annuals: Vec<&Period> = company.periods.iter().filter(|p| ANNUAL_FORMS.contains(&p.form.as_str()) && p.revenue.is_some()).collect();
    annuals.sort_by(|a, b| a.period_end.cmp(&b.period_end));
    let skip = annuals.len().saturating_sub(max_years);
    annuals.split_off(skip)
}

fn fundamentals_table(annuals: &[&Period]) -> String {
    let head = "<tr><th>FY</th><th>Revenue</th><th>Net income</th><th>OCF</th><th>Capex</th><th>SBC</th><th>Acquisitions</th><th>FCF</th><th>FCF margin</th></tr>";
    let mut rows = String::new();
    for p in annuals {
        let margin = p.fcf_margin().map_or(DASH.to_string(), |m| pct(m, 1));
        rows += &format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{margin}</td></tr>",
            &p.period_end[..4],
            money_b(p.revenue, 1),
            money_b(p.net_income, 1),
            money_b(p.operating_cash_flow, 1),
            money_b(p.capex, 1),
            money_b(p.sbc_total, 2),
            money_b(p.acquisitions_net_of_cash, 1),
            money_b(p.free_cash_flow(), 1),
        );
    }
    table(head, &rows)
}

fn fundamentals_section(doc: &mut Doc, company: &Company) -> String {
    let annuals = fundamental_annuals(company, 10);
    if annuals.is_empty() {
        return section(
            SECTION_TITLES[1],
            &card("Annual fundamentals", "From SEC EDGAR 10-K periods", &note("Fundamentals not available \u{2014} no annual periods with revenue in the store."), true),
        );
    }
    let span = format!("FY{}\u{2013}FY{}", &annuals[0].period_end[..4], &annuals[annuals.len() - 1].period_end[..4]);
    let mut cards = card(
        "Revenue, free cash flow & margin",
        &format!("Annual (10-K) periods, {span} \u{2014} margin on its own scale below"),
        &doc.fig(fundamentals_figure(&annuals)),
        true,
    );
    cards += &card("Annual fundamentals", "Values as reported (capex and acquisitions are cash outflows)", &fundamentals_table(&annuals), true);
    section(SECTION_TITLES[1], &cards)
}

// ---------------------------------------------------------------------------
// Aggregate DCF scenarios
// ---------------------------------------------------------------------------

/// The P50 value per share when one input moves, and the rows of the tornado (label, low delta, high delta).
fn tornado_data(company: &Company, base: &DcfAssumptions, cap: &CapitalStructure) -> Result<(f64, Vec<TornadoRow>), String> {
    let mut base = base.clone();
    if base.fcf_margin_mean.is_none() || base.fcf_margin_std.is_none() {
        let (m, s) = derive_fcf_margin(company).map_err(|e| e.0)?;
        base.fcf_margin_mean = Some(m);
        base.fcf_margin_std = Some(s);
    }
    let p50 = |a: &DcfAssumptions| -> Result<f64, String> {
        let r = run_classic(company, a).map_err(|e| e.0)?;
        Ok((r.median_value - cap.net_debt) / cap.shares)
    };
    let base_ps = p50(&base)?;
    let mut rows = Vec::new();
    for (label, read, write) in tornado_inputs() {
        let (mean, std) = read(&base);
        let mut lo = base.clone();
        write(&mut lo, mean - std);
        let mut hi = base.clone();
        write(&mut hi, mean + std);
        rows.push((label.to_string(), p50(&lo)? - base_ps, p50(&hi)? - base_ps));
    }
    sort_tornado(&mut rows);
    Ok((base_ps, rows))
}

fn summary_table(runs: &[Run], cap: &CapitalStructure, price: Option<f64>) -> String {
    let head = format!(
        "<tr><th>Scenario</th><th>Growth</th><th>WACC</th><th>Terminal</th><th>EV P50</th><th>$/sh P10</th><th>$/sh P50</th><th>$/sh P90</th>{}</tr>",
        if price.is_some() { "<th>vs price</th>" } else { "" }
    );
    let mut rows = String::new();
    for r in runs {
        let a = &r.sc.assumptions;
        let sorted = sorted_copy(&per_share(&r.res, cap));
        let (p10, p50, p90) = (percentile(&sorted, 10.0), percentile(&sorted, 50.0), percentile(&sorted, 90.0));
        let mut cells = format!(
            "<td><span class=\"dot\" style=\"background:{}\"></span>{}</td><td>{}</td><td>{}</td><td>{}</td><td>${}B</td><td>${}</td><td>${}</td><td>${}</td>",
            scenario_color(r.sc.key),
            esc(r.sc.label),
            pct(a.growth_rate_mean, 1),
            pct(a.wacc_mean, 1),
            pct(a.terminal_growth_mean, 1),
            fixed_c(r.res.median_value / 1e9, 0),
            fixed_c(p10, 0),
            fixed_c(p50, 0),
            fixed_c(p90, 0)
        );
        if let Some(p) = price {
            let d = (p50 - p) / p;
            cells += &format!("<td style=\"color:{}\">{}</td>", delta_color(d), pct_signed(d, 0));
        }
        rows += &format!("<tr>{cells}</tr>");
    }
    table(&head, &rows)
}

fn aggregate_section(doc: &mut Doc, company: &Company, runs: &[Run], cap: &CapitalStructure, price: Option<f64>, base_year: i32) -> Result<String, String> {
    let base = runs.iter().find(|r| r.sc.key == "base").ok_or("no base scenario")?;
    let (base_ps, rows) = tornado_data(company, &base.sc.assumptions, cap)?;
    let panels = [
        ("FCF projection fan", "Simulated free cash flow \u{2014} P10\u{2013}P90 band, median line, per scenario", fan_figure(runs, base_year)),
        ("Per-share value distribution", "Equity value per share across simulations (EV \u{2212} net debt, per share)", distribution_figure(runs, cap, price)),
        (
            "PV build-up \u{2014} median base case",
            "Median PV of each projection year + terminal value \u{2212} net debt (base case)",
            waterfall_figure(&base.res, cap, base_year),
        ),
        (
            "Sensitivity tornado",
            "P50 per-share impact of shifting one input \u{b1}1\u{3c3}, others held at their means (base case)",
            tornado_bars(base_ps, &rows),
        ),
    ];
    let mut cards = String::new();
    for (title, sub, f) in panels {
        cards += &card(title, sub, &doc.fig(f), false);
    }
    cards += &card("Scenario summary", "Per-share equity values are (EV \u{2212} net debt) / shares at each percentile", &summary_table(runs, cap, price), true);
    Ok(section(SECTION_TITLES[2], &cards))
}

// ---------------------------------------------------------------------------
// Driver model
// ---------------------------------------------------------------------------

fn driver_table(spec: &ModelSpec) -> String {
    let head = "<tr><th>Driver</th><th>Type</th><th>Dist</th><th>Mean</th><th>Std</th><th>Bounds</th><th>Rationale</th></tr>";
    let mut rows = String::new();
    for d in &spec.drivers {
        let badge = if d.stochastic { "<span class=\"badge stoch\">stochastic</span>" } else { "<span class=\"badge det\">deterministic</span>" };
        let bounds = if d.lo.is_some() || d.hi.is_some() {
            let f = |v: Option<f64>| v.map_or(DASH.to_string(), |v| general_c(v, 4));
            format!("[{}, {}]", f(d.lo), f(d.hi))
        } else {
            DASH.to_string()
        };
        let std = if d.stochastic { general_c(d.std, 4) } else { DASH.to_string() };
        let rationale = if d.rationale.is_empty() { DASH } else { d.rationale.as_str() };
        rows += &format!(
            "<tr><td>{}</td><td>{badge}</td><td>{}</td><td>{}</td><td>{std}</td><td>{bounds}</td><td class=\"wrap\">{}</td></tr>",
            esc(&d.name),
            esc(&d.dist),
            general_c(d.mean, 4),
            esc(rationale)
        );
    }
    table(head, &rows)
}

/// Bottom 10 and top 10 paths by per-share value, with each stochastic driver's draw and its z-score (bold from |z| 1.5).
fn outlier_table(result: &DriverDcfResult) -> String {
    let per_share_v = &result.per_share_paths;
    let mut order: Vec<usize> = (0..per_share_v.len()).collect();
    order.sort_by(|a, b| per_share_v[*a].total_cmp(&per_share_v[*b]));
    let stats: Vec<(f64, f64)> = result.driver_draws.iter().map(|(_, d)| draw_stats(d)).collect();
    let cell = |k: usize, i: usize| -> String {
        let (name, draws) = &result.driver_draws[k];
        let v = draws[i];
        let (mean, std) = stats[k];
        let z = if std > 0.0 { (v - mean) / std } else { 0.0 };
        let txt = if rate_like(name, draws) { pct(v, 1) } else { general_c(v, 3) };
        let txt = format!("{txt} ({})", signed(z, 1));
        if z.abs() >= 1.5 { format!("<td><strong>{txt}</strong></td>") } else { format!("<td>{txt}</td>") }
    };
    let n_cols = 2 + result.driver_draws.len();
    let head = format!(
        "<tr><th>Path</th><th>Value $/sh</th>{}</tr>",
        result.driver_draws.iter().map(|(n, _)| format!("<th>{}</th>", esc(&short_name(n)))).collect::<String>()
    );
    let mut rows = String::new();
    let bottom: Vec<usize> = order.iter().take(10).cloned().collect();
    let top: Vec<usize> = order.iter().rev().take(10).cloned().collect();
    for (label, idx) in [("Bottom 10 by per-share value", bottom), ("Top 10 by per-share value", top)] {
        rows += &format!("<tr><th colspan=\"{n_cols}\">{label}</th></tr>");
        for i in idx {
            let v = per_share_v[i];
            let val = if v < 0.0 { format!("-${}", fixed_c(v.abs(), 2)) } else { format!("${}", fixed_c(v, 2)) };
            rows += &format!("<tr><td>{i}</td><td>{val}</td>{}</tr>", (0..result.driver_draws.len()).map(|k| cell(k, i)).collect::<String>());
        }
    }
    table(&head, &rows)
}

fn path_explorer_cards(doc: &mut Doc, result: &DriverDcfResult, base_year: i32) -> Result<String, String> {
    if result.driver_draws.is_empty() {
        return Ok(card(
            "Path explorer",
            "Monte Carlo path sample",
            &note("No stochastic drivers \u{2014} every simulated path is identical, so there is no dispersion to explore."),
            true,
        ));
    }
    let n = result.per_share_paths.len();
    let (spaghetti, n_shown) = spaghetti_figure(result, base_year)?;
    let mut cards = card(
        "Path explorer \u{2014} FCFF path spaghetti",
        &format!("{} of {} paths shown (all beyond P1/P99 + stratified sample); hover a path for its drawn driver values", int_c(n_shown as i64), int_c(n as i64)),
        &doc.fig(spaghetti),
        true,
    );
    let (b, m, t) = stratified_path_sample(&result.per_share_paths, PARCOORDS_TARGET, PATH_SAMPLE_SEED)?;
    let n_pc = b.len() + m.len() + t.len();
    cards += &card(
        "Path explorer \u{2014} parallel coordinates",
        &format!(
            "{} of {} paths (all beyond P1/P99 + stratified sample) across every stochastic driver to the per-share value \u{2014} darker steel = higher value; drag along an axis to brush",
            int_c(n_pc as i64),
            int_c(n as i64)
        ),
        &doc.fig(parcoords_figure(result)?),
        true,
    );
    cards += &card(
        "Path explorer \u{2014} outlier paths",
        "Bottom 10 and top 10 paths by per-share value; each stochastic driver's drawn value with its z-score in parentheses \u{2014} bold where |z| \u{2265} 1.5",
        &outlier_table(result),
        true,
    );
    Ok(cards)
}

fn driver_section(doc: &mut Doc, repo: &Path, ticker: &str, base_stats: Option<[f64; 3]>, price: Option<f64>, base_year: i32) -> String {
    let title = SECTION_TITLES[3];
    let ticker = ticker.to_uppercase();
    let file = format!("analysis/{ticker}/driver_model.toml");
    let one_note = |text: String| section(title, &card("Driver model", &file, &note(&text), true));
    let spec = match ModelSpec::load(repo, &ticker) {
        Ok(Some(s)) => s,
        Ok(None) => return one_note(format!("Driver model not available \u{2014} no {file}.")),
        Err(e) => return one_note(format!("Driver model could not be loaded: {e}")),
    };
    let run = || -> Result<(crate::dcf::driver::DriverDcf, DriverDcfResult), String> {
        let engine = spec.engine().map_err(|e| e.0)?;
        let result = engine.run(DRIVER_SIMS, Some(DRIVER_SEED)).map_err(|e| e.0)?;
        Ok((engine, result))
    };
    let (engine, result) = match run() {
        Ok(x) => x,
        Err(e) => return one_note(format!("Driver model run failed: {e}")),
    };
    let build = |doc: &mut Doc| -> Result<String, String> {
        let mut cards = String::new();
        if !spec.segment_history.is_empty() {
            let names: Vec<String> = spec.segments.iter().map(|s| s.name.clone()).collect();
            cards += &card(
                "Segment revenue history",
                "Reported revenue by segment, stacked per fiscal year",
                &doc.fig(segment_history_figure(&names, &spec.segment_history)),
                true,
            );
        } else {
            cards += &card(
                "Segment revenue history",
                "Reported revenue by segment",
                &note("Segment history not available \u{2014} the driver model declares no segment_history."),
                true,
            );
        }
        cards += &card(
            "Driver assumptions",
            "Every model input, its distribution, and the rationale reviewers signed off on",
            &driver_table(&spec),
            true,
        );
        let stoch: Vec<&crate::dcf::driver::Driver> = spec.drivers.iter().filter(|d| d.stochastic).collect();
        if stoch.len() >= 2 {
            let (names, mat) = correlation(&engine.model)?;
            cards += &card(
                "Driver correlation matrix",
                "Pairwise correlations of the stochastic drivers (sampled jointly via Cholesky)",
                &doc.fig(corr_heatmap_figure(&names, &mat)),
                false,
            );
        } else {
            cards += &card(
                "Driver correlation matrix",
                "Stochastic drivers",
                &note("Fewer than two stochastic drivers \u{2014} no correlation structure to show."),
                false,
            );
        }
        let driver_stats = [result.per_share_p10, result.per_share_p50, result.per_share_p90];
        match base_stats {
            Some(b) => {
                cards += &card(
                    "Driver model vs aggregate base",
                    "P10\u{2013}P90 per-share range with the P50 marked, both engines, same capital structure basis",
                    &doc.fig(driver_compare_figure(driver_stats, b, price)),
                    false,
                )
            }
            None => {
                cards += &card("Driver model vs aggregate base", "Per-share percentiles", &note("Aggregate base scenario unavailable for comparison."), false)
            }
        }
        cards += &card(
            "Driver attribution tornado",
            "P50 per-share delta when one driver's mean shifts \u{b1}1\u{3c3} \u{2014} same seed, correlations intact",
            &doc.fig(driver_tornado_figure(&result)),
            true,
        );
        cards += &path_explorer_cards(doc, &result, base_year)?;
        let meta = format!(
            "DriverDCF: {} simulations \u{b7} seed {DRIVER_SEED} \u{b7} {}y horizon \u{b7} net debt {} \u{b7} {}B shares \u{b7} terminal value {} of EV at driver means \u{b7} terminal growth pinned at WACC\u{2212}2% on {} of paths",
            int_c(DRIVER_SIMS as i64),
            engine.projection_years,
            money_b(Some(spec.net_debt), 1),
            fixed(spec.shares / 1e9, 3),
            pct(result.tv_ev_share_at_means, 0),
            pct(result.tg_pinned_fraction, 1)
        );
        cards += &format!("<p class=\"sub wide-note\">{}</p>", esc(&meta));
        Ok(cards)
    };
    match build(doc) {
        Ok(cards) => section(title, &cards),
        Err(e) => one_note(format!("Driver model run failed: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Backtest
// ---------------------------------------------------------------------------

const BACKTEST_METRICS: [(&str, &str); 2] = [("revenue", "Revenue"), ("fcf", "FCF")];

fn actuals_by_year(company: &Company, metric: &str) -> BTreeMap<i32, f64> {
    let mut out = BTreeMap::new();
    for p in clean_annuals(company) {
        let v = if metric == "revenue" { p.revenue } else { p.free_cash_flow() };
        if let (Some(v), Ok(y)) = (v, p.period_end[..4].parse::<i32>()) {
            out.insert(y, v);
        }
    }
    out
}

fn backtest_chips(result: &BacktestResult) -> String {
    let (hits, total) = result.coverage();
    let mut chips = Vec::new();
    if total > 0 {
        chips.push(chip("P10\u{2013}P90 coverage", &format!("{hits}/{total}"), &format!("{} of realized (nominal 80%)", pct(hits as f64 / total as f64, 0)), None, false));
    } else {
        chips.push(chip("P10\u{2013}P90 coverage", DASH, "no realized outcomes yet", None, false));
    }
    for (h, metrics) in result.bias_by_horizon() {
        for (key, label) in BACKTEST_METRICS {
            if let Some((mean_err, n)) = metrics.get(key) {
                chips.push(chip(&format!("+{h}y {label} bias"), &format!("{}%", signed(*mean_err, 1)), &format!("mean err vs P50 \u{b7} n={n}"), Some(delta_color(*mean_err)), false));
            }
        }
    }
    format!("<div class=\"chips wide-chips\">{}</div>", chips.concat())
}

fn backtest_table(result: &BacktestResult) -> String {
    let head = "<thead><tr><th>Vintage as-of</th><th>Base FY</th><th>Target FY</th><th>Metric</th><th>P10</th><th>P50</th><th>P90</th><th>Actual</th><th>Err vs P50</th><th>In band</th></tr></thead>";
    let mut rows = String::new();
    for v in &result.vintages {
        for o in &v.outcomes {
            let err_html = match o.error_pct {
                None => DASH.to_string(),
                Some(e) => format!("<span style=\"color:{}\">{}%</span>", delta_color(e), signed(e, 1)),
            };
            let in_band = match o.in_band {
                Some(true) => "\u{2713}",
                Some(false) => "\u{2717}",
                None => DASH,
            };
            let flag = if v.params.data_flags.is_empty() { "" } else { " \u{2691}" };
            let vin = v.params.as_of.clone().unwrap_or_else(|| DASH.to_string());
            rows += &format!(
                "<tr><td>{vin}{flag}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{err_html}</td><td>{in_band}</td></tr>",
                v.params.base_fiscal_year,
                o.target_fiscal_year,
                esc(o.metric),
                money_b(Some(o.p10), 1),
                money_b(Some(o.p50), 1),
                money_b(Some(o.p90), 1),
                money_b(o.actual, 1),
            );
        }
    }
    table(head, &format!("<tbody>{rows}</tbody>"))
}

fn backtest_notes(result: &BacktestResult) -> String {
    let mut notes = vec![
        "Mechanical vintages: parameters (trailing revenue CAGR, FCF-margin mean/std over \u{2264}5 annuals) derive from each vintage's point-in-time filings alone \u{2014} no judgment inputs, no forward information. Sampling mirrors the aggregate engine (one growth and one margin draw per path). Operating projections only \u{2014} no WACC, no terminal value.".to_string(),
        "filed_date visibility: a period enters a vintage only once filed; companyfacts-sourced figures can carry a later filing's date, so a vintage may see fewer periods than a real-time observer did (the conservative direction). Actuals are as currently reported \u{2014} restatements included, deliberately asymmetric.".to_string(),
    ];
    let mut flags: Vec<&String> = result.vintages.iter().flat_map(|v| v.params.data_flags.iter()).collect();
    flags.sort();
    flags.dedup();
    if !flags.is_empty() {
        notes.push(format!("\u{2691} data flags surfaced, not repaired: {}", flags.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" \u{b7} ")));
    }
    if !result.skipped.is_empty() {
        notes.push(format!("Skipped vintage dates (insufficient history): {}.", result.skipped.iter().map(|(d, _)| d.as_str()).collect::<Vec<_>>().join(", ")));
    }
    notes.iter().map(|t| note(t)).collect()
}

fn backtest_section(doc: &mut Doc, company: &Company) -> String {
    let title = SECTION_TITLES[4];
    let result = run_backtest(company, BACKTEST_HORIZON, BACKTEST_SIMS, BACKTEST_SEED);
    if result.vintages.is_empty() {
        let reasons = result.skipped.iter().map(|(d, _)| d.as_str()).collect::<Vec<_>>().join("; ");
        let tail = if reasons.is_empty() { String::new() } else { format!(" (candidate dates skipped: {reasons})") };
        return section(
            title,
            &card(
                "Vintage backtest",
                "Walk-forward scoring of mechanical operating projections",
                &note(&format!("No vintage date has the 3 clean annual periods needed to derive mechanical parameters{tail}.")),
                true,
            ),
        );
    }
    let mut cards = backtest_chips(&result);
    for (key, label) in BACKTEST_METRICS {
        cards += &card(
            &format!("{label} hedgehog"),
            "Ink: actuals as currently reported. Each accent spur is one vintage's P50 projection off its base-year actual; whiskers span P10\u{2013}P90.",
            &doc.fig(hedgehog_figure(&result, &actuals_by_year(company, key), label)),
            false,
        );
    }
    cards += &card(
        "Vintage scorecard",
        &format!("One row per vintage \u{d7} target year \u{d7} metric (horizon {}y) \u{2014} \u{2691} marks vintages whose parameter window touches a flagged year", result.horizon),
        &(backtest_table(&result) + &backtest_notes(&result)),
        true,
    );
    section(title, &cards)
}

// ---------------------------------------------------------------------------
// Options market
// ---------------------------------------------------------------------------

/// The next earnings date on or after `after`, from the cache file `yf_earnings.json` (a list of records with a `date`).
fn next_earnings_date(cache: &Path, ticker: &str, after: &str) -> Option<String> {
    let path = cache.join(ticker.to_uppercase()).join("yf_earnings.json");
    let text = std::fs::read_to_string(path).ok()?;
    let raw = crate::market::parse_lenient(&text).ok()?;
    raw.as_array()?
        .iter()
        .filter_map(|r| r.get("date").and_then(|d| d.as_str()))
        .filter(|d| crate::curve::parse_iso(d).is_some() && *d >= after)
        .min()
        .map(str::to_string)
}

fn options_chips(metrics: &ChainMetrics) -> String {
    let total_vol: i64 = metrics.expiries.iter().map(|e| e.call_volume + e.put_volume).sum();
    let total_oi: i64 = metrics.expiries.iter().map(|e| e.call_open_interest + e.put_open_interest).sum();
    let ratio = |v: Option<f64>| v.map_or(DASH.to_string(), |v| fixed(v, 2));
    let chips = [
        chip("Snapshot", &metrics.snapshot_date, "", None, false),
        chip("Spot", &metrics.spot_price.map_or(DASH.to_string(), |s| format!("${}", fixed_c(s, 2))), "", None, false),
        chip("P/C volume", &ratio(metrics.put_call_volume_ratio), "chain-level", None, false),
        chip("P/C open interest", &ratio(metrics.put_call_oi_ratio), "chain-level", None, false),
        chip("Total volume", &int_c(total_vol), "contracts", None, false),
        chip("Total open interest", &int_c(total_oi), "contracts", None, false),
    ];
    format!("<div class=\"chips\">{}</div>", chips.concat())
}

fn chain_table(snapshot: &OptionChainSnapshot, expiry: &str, atm_strike: Option<f64>, width: usize) -> String {
    let quotes: Vec<&crate::market::options::OptionQuote> = snapshot.quotes.iter().filter(|q| q.expiry == expiry).collect();
    let mut strikes: Vec<f64> = quotes.iter().map(|q| q.strike).collect();
    strikes.sort_by(f64::total_cmp);
    strikes.dedup();
    if strikes.is_empty() {
        return note(&format!("No quotes for expiry {expiry}."));
    }
    let atm = atm_strike.unwrap_or(strikes[strikes.len() / 2]);
    let mut idx = 0;
    for (i, s) in strikes.iter().enumerate() {
        if (s - atm).abs() < (strikes[idx] - atm).abs() {
            idx = i;
        }
    }
    let window = &strikes[idx.saturating_sub(width)..(idx + width + 1).min(strikes.len())];
    let mut calls = BTreeMap::new();
    let mut puts = BTreeMap::new();
    for q in &quotes {
        if q.option_type == "call" {
            calls.insert(q.strike.to_bits(), *q);
        } else if q.option_type == "put" {
            puts.insert(q.strike.to_bits(), *q);
        }
    }
    let px = |v: Option<f64>| v.map_or(DASH.to_string(), |v| fixed_c(v, 2));
    let iv = |v: Option<f64>| v.map_or(DASH.to_string(), |v| pct(v, 0));
    let oi = |v: Option<i64>| v.map_or(DASH.to_string(), int_c);
    let head = "<tr><th colspan=\"5\">Calls</th><th>Strike</th><th colspan=\"5\">Puts</th></tr><tr><th>Bid</th><th>Ask</th><th>Last</th><th>IV</th><th>OI</th><th></th><th>Bid</th><th>Ask</th><th>Last</th><th>IV</th><th>OI</th></tr>";
    let mut rows = String::new();
    for s in window {
        let (c, p) = (calls.get(&s.to_bits()), puts.get(&s.to_bits()));
        let cls = if *s == strikes[idx] { " class=\"atm\"" } else { "" };
        rows += &format!(
            "<tr{cls}><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><strong>{}</strong></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            px(c.and_then(|q| q.bid)),
            px(c.and_then(|q| q.ask)),
            px(c.and_then(|q| q.last_price)),
            iv(c.and_then(|q| q.implied_volatility)),
            oi(c.and_then(|q| q.open_interest)),
            g(*s),
            px(p.and_then(|q| q.bid)),
            px(p.and_then(|q| q.ask)),
            px(p.and_then(|q| q.last_price)),
            iv(p.and_then(|q| q.implied_volatility)),
            oi(p.and_then(|q| q.open_interest)),
        );
    }
    table(head, &rows)
}

fn options_section(doc: &mut Doc, cache: &Path, ticker: &str, price: Option<f64>) -> String {
    let title = SECTION_TITLES[5];
    let ticker = ticker.to_uppercase();
    let none = |sub: String, text: String| section(title, &card("Option chain", &sub, &note(&text), true));
    if list_snapshots(cache, &ticker).is_empty() {
        return none(
            format!("data/raw/{ticker}/options/"),
            format!("No option-chain snapshot available \u{2014} run fa analyze {ticker} to capture one."),
        );
    }
    let snapshot = match load_snapshot(cache, &ticker, None) {
        Ok(s) => s,
        Err(e) => return none(format!("data/raw/{ticker}/options/"), format!("The option-chain snapshot could not be read: {e}")),
    };
    if snapshot.quotes.is_empty() {
        return none(format!("Snapshot {}", snapshot.snapshot_date), "The snapshot contains no quotes \u{2014} no listed options found.".to_string());
    }
    let metrics = compute_chain_metrics(&snapshot);
    let next_earnings = next_earnings_date(cache, &ticker, &snapshot.snapshot_date);
    let spot = snapshot.spot_price.or(price);
    let mut cards = format!("<div class=\"wide-chips\">{}</div>", options_chips(&metrics));
    let mut chart = |doc: &mut Doc, title: &str, sub: &str, f: Option<Fig>, missing: &str| {
        let body = match f {
            Some(f) => doc.fig(f),
            None => note(missing),
        };
        cards += &card(title, sub, &body, false);
    };
    chart(
        doc,
        "ATM implied-volatility term structure",
        "Average of ATM call/put IV per expiry \u{2014} the near-dated bump is the earnings premium",
        iv_term_figure(&metrics, next_earnings.as_deref()),
        "ATM IV unavailable \u{2014} no expiry has a solvable ATM implied vol.",
    );
    chart(
        doc,
        "Implied move by expiry",
        "ATM straddle mid \u{f7} spot \u{2014} what the market prices for a move by each expiry",
        implied_move_figure(&metrics),
        "Implied move unavailable \u{2014} no expiry has both ATM legs quoted.",
    );
    chart(
        doc,
        "Volatility skew by expiry",
        "IV of the ~90%-moneyness put minus the ~110%-moneyness call (positive = downside bid) \u{2014} expiries under 2 days out are excluded: wing IVs on dying options are solver noise, not skew",
        skew_figure(&metrics),
        "Skew unavailable \u{2014} wings not quoted with solvable IVs (near-snapshot expiries are excluded as solver noise).",
    );
    chart(
        doc,
        "Open interest by strike",
        "Calls + puts open interest, four nearest expiries, strikes within \u{b1}25% of spot",
        oi_heatmap_figure(&snapshot, spot),
        "Open interest unavailable \u{2014} no contracts carry open interest near spot.",
    );
    let mut expiries: Vec<&str> = snapshot.quotes.iter().map(|q| q.expiry.as_str()).collect();
    expiries.sort_unstable();
    expiries.dedup();
    let mut target: Option<&str> = None;
    let mut target_note = String::new();
    if let Some(ne) = &next_earnings {
        target = expiries.iter().find(|e| **e >= ne.as_str()).copied();
        target_note = format!("first expiry after earnings on {ne}");
    }
    let target = match target {
        Some(t) => t,
        None => {
            target_note = "nearest expiry (no upcoming earnings date on file)".to_string();
            expiries[0]
        }
    };
    let em: Option<&ExpiryMetrics> = metrics.expiries.iter().find(|m| m.expiry == target);
    cards += &card(
        "Chain detail \u{2014} ATM \u{b1}10 strikes",
        &format!("Expiry {target} ({target_note}); ATM row highlighted"),
        &chain_table(&snapshot, target, em.and_then(|m| m.atm_strike), 10),
        true,
    );
    section(title, &cards)
}

// ---------------------------------------------------------------------------
// Footer and page
// ---------------------------------------------------------------------------

fn footer_html(company: &Company, base_year: i32, base: &Scenario, cache: &Path, price_note: &str) -> String {
    let ticker = company.ticker.to_uppercase();
    let n_annual = company.periods.iter().filter(|p| ANNUAL_FORMS.contains(&p.form.as_str())).count();
    let prices_src = if prices::cache_path(cache, &ticker).is_file() {
        format!("cached data/raw/{ticker}/prices.json")
    } else {
        "Yahoo at run time".to_string()
    };
    let snaps = list_snapshots(cache, &ticker);
    let options_src = match snaps.last() {
        Some(s) => format!("snapshot {s}"),
        None => "no snapshot on disk".to_string(),
    };
    let a = &base.assumptions;
    let seed = a.seed.map_or("None".to_string(), |s| s.to_string());
    let lines = [
        format!(
            "Data: SEC EDGAR XBRL 10-K/10-Q \u{2192} data/processed/{ticker}.json ({n_annual} annual periods, latest FY{base_year}) \u{b7} prices: {prices_src} \u{b7} options: {options_src} \u{b7} {}",
            esc(price_note)
        ),
        format!(
            "Engines: aggregate StochasticDCF {} sims \u{d7} {}y (seed {seed}, scenarios from rust/src/dcf/scenarios.rs) \u{b7} driver model {} sims (seed {DRIVER_SEED}) when analysis/{ticker}/driver_model.toml exists",
            int_c(a.n_simulations as i64),
            a.projection_years,
            int_c(DRIVER_SIMS as i64)
        ),
        "Generated by fa report \u{2014} seeded runs, reproducible".to_string(),
    ];
    format!("<footer>{}</footer>", lines.iter().map(|l| format!("<p>{l}</p>")).collect::<String>())
}

/// Build the report: the body with a marker for each figure, and the figures.
pub fn build(company: &Company, inputs: &Inputs) -> Result<Built, String> {
    let cache = cache_dir(inputs.repo);
    let scenarios = scenarios_for(&company.ticker);
    let cap = capital_structure_for(company).map_err(|e| e.0)?;
    let base_year: i32 = latest_annual(company).map_err(|e| e.0)?.period_end[..4].parse().map_err(|_| "bad period_end".to_string())?;
    let base = scenarios.iter().find(|s| s.key == "base").ok_or("no base scenario")?;
    let mut runs = Vec::new();
    for sc in &scenarios {
        runs.push(Run { sc, res: run_classic(company, &sc.assumptions).map_err(|e| e.0)? });
    }
    let mut stats: Stats = Vec::new();
    for r in &runs {
        let sorted = sorted_copy(&per_share(&r.res, &cap));
        stats.push((r.sc.key, [percentile(&sorted, 10.0), percentile(&sorted, 50.0), percentile(&sorted, 90.0)]));
    }
    let mut doc = Doc::default();
    let header = header_html(company, &cap, &scenarios, &stats, inputs.price, inputs.price_note, inputs.as_of);
    let sections = price_section(&mut doc, &company.ticker, inputs.prices, &scenarios, &stats)
        + &fundamentals_section(&mut doc, company)
        + &aggregate_section(&mut doc, company, &runs, &cap, inputs.price, base_year)?
        + &driver_section(&mut doc, inputs.repo, &company.ticker, stat(&stats, "base").copied(), inputs.price, base_year)
        + &backtest_section(&mut doc, company)
        + &options_section(&mut doc, &cache, &company.ticker, inputs.price);
    let footer = footer_html(company, base_year, base, &cache, inputs.price_note);
    Ok(Built { ticker: company.ticker.to_uppercase(), body: format!("{header}\n{sections}\n{footer}"), figs: doc.figs })
}

/// JSON for a `<script>` element: no `<`, `>` or `&` can end the script or start a tag.
fn script_json(v: &serde_json::Value) -> String {
    serde_json::to_string(v).unwrap_or_default().replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026")
}

impl Built {
    /// The complete HTML page.
    pub fn html(&self) -> String {
        let mut body = self.body.clone();
        for (i, f) in self.figs.iter().enumerate() {
            let marker = format!("[[FIG {}]]", f.height);
            let id = format!("fig-{}", i + 1);
            let html = format!(
                "<div style=\"height:{h}px; width:100%;\"><div id=\"{id}\" class=\"plotly-graph-div\" style=\"height:100%; width:100%;\"></div><script>(function(){{var el=document.getElementById(\"{id}\");if(!window.Plotly){{el.textContent=\"The chart needs the plotly.js library from cdn.plot.ly (no network?).\";return;}}Plotly.newPlot(el,{data},{layout},{{\"displayModeBar\":false,\"responsive\":true}});}})();</script></div>",
                h = f.height,
                data = script_json(&f.data),
                layout = script_json(&f.layout)
            );
            body = body.replacen(&marker, &html, 1);
        }
        let ticker = esc(&self.ticker);
        format!(
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{ticker} \u{2014} Company report</title>\n<style>{STYLE}</style>\n<script src=\"{PLOTLY_JS_URL}\" charset=\"utf-8\"></script>\n</head>\n<body>\n{body}\n<script>window.addEventListener(\"load\", () => window.dispatchEvent(new Event(\"resize\")));</script>\n</body>\n</html>"
        )
    }
}

// ---------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------

/// The report file of a ticker. `price` is the market price (`None`: the last close of the price history).
/// `fetch` gives the bytes of a web address (the Yahoo chart endpoint when no price cache exists).
pub fn make_report(
    repo: &Path,
    ticker: &str,
    price: Option<f64>,
    as_of: &str,
    fetch: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<Built, String> {
    let ticker = ticker.to_uppercase();
    let company = Company::load(&repo.join("data").join("processed"), &ticker).map_err(|e| e.0)?;
    let cache = cache_dir(repo);
    let (history, source) = match prices::load_prices(&cache, &ticker, false, fetch) {
        Ok((p, s)) => (Some(p), Some(s)),
        Err(_) => (None, None),
    };
    let history = history.filter(|p| !p.is_empty());
    let (price, note) = match price {
        Some(p) => (Some(p), format!("price ${} (--price override)", fixed_c(p, 2))),
        None => match history.as_deref().and_then(prices::last_close) {
            Some(p) => {
                let from = match source {
                    Some(prices::Source::Cache) => format!("data/raw/{ticker}/prices.json"),
                    _ => "Yahoo chart".to_string(),
                };
                (Some(p), format!("last close ${} ({from})", fixed_c(p, 2)))
            }
            None => (None, "current price unavailable (offline?) \u{2014} price marker omitted".to_string()),
        },
    };
    build(&company, &Inputs { repo, ticker: &ticker, price, price_note: &note, as_of, prices: history.as_deref() })
}

/// `fa report`. Returns the exit code.
pub fn command(repo: &Path, args: &[String]) -> Result<i32, String> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return Ok(0);
    }
    let (mut ticker, mut price, mut out) = (None, None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--price" => {
                let p: f64 = it.next().and_then(|v| v.parse().ok()).ok_or("--price needs a number")?;
                price = Some(p);
            }
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out needs a file name")?)),
            s if s.starts_with('-') => return Err(format!("unknown option '{s}'")),
            s if ticker.is_none() => ticker = Some(s.to_uppercase()),
            s => return Err(format!("unexpected argument '{s}'")),
        }
    }
    let ticker = ticker.ok_or("report needs a ticker")?;
    let out = out.unwrap_or_else(|| repo.join("data").join("processed").join(format!("{ticker}_dcf.html")));
    let built = match make_report(repo, &ticker, price, &crate::market::today_iso(), &crate::market::download) {
        Ok(b) => b,
        Err(e) => {
            println!("Cannot visualize {ticker}: {e}");
            return Ok(1);
        }
    };
    crate::market::write_atomic(&out, &built.html())?;
    println!("Report written: {}", out.display());
    Ok(0)
}
