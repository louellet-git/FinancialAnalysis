//! The plotly figures of the report, as JSON objects (`data` and `layout`). The page draws them
//! with plotly.js. Each figure has the same data and the same layout keys as the figure that the
//! Python script made (the tests compare them with fixtures), except the plotly Python template,
//! which the page does not carry.

use super::fmt::*;
use crate::curve::{civil_from_days, days_from_civil, parse_iso};
use crate::dcf::backtest::BacktestResult;
use crate::dcf::classic::{DcfAssumptions, DcfResult, Period};
use crate::dcf::driver::{DriverDcfResult, DriverModel};
use crate::dcf::scenarios::{CapitalStructure, Scenario};
use crate::dcf::stats::{mean, percentile, sorted_copy, std_pop};
use crate::market::options::{ChainMetrics, OptionChainSnapshot};
use crate::market::prices::PricePoint;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

// Chart chrome: the "Ratings-desk blueprint" tokens (light mode).
pub const PAGE: &str = "#f4f4f5";
pub const SURFACE: &str = "#f4f4f5";
pub const RAISED: &str = "#eceef0";
pub const INK: &str = "#1d1f20";
pub const INK_SECONDARY: &str = "#4a5158";
pub const INK_MUTED: &str = "#6b7480";
pub const ACCENT: &str = "#5980a6";
pub const ACCENT_TINT: &str = "#e9eef4";
pub const HAIRLINE: &str = "#c9ced4";
pub const GRID: &str = "#dde2e8";
pub const AXIS: &str = "#aab2bb";
pub const FONT: &str = "\"Avenir Next\",\"Segoe UI\",system-ui,sans-serif";
pub const CATEGORICAL: [&str; 8] = ["#35618f", "#b45309", "#216e5c", "#8f4d9f", "#857f29", "#882255", "#667585", "#0a5e75"];
pub const COOL: &str = "#35618f";
pub const WARM: &str = "#a04a17";
pub const DIVERGING_MID: &str = "#c6cbd1";
pub const SEQ_STEEL: [(f64, &str); 8] = [
    (0.0, "#dfe6ee"),
    (0.14, "#c4d1de"),
    (0.3, "#a8bccd"),
    (0.45, "#8ca7bd"),
    (0.6, "#7092ac"),
    (0.75, "#557d99"),
    (0.9, "#3c6484"),
    (1.0, "#274a66"),
];
pub const DRIVER_COLOR: &str = "#8f4d9f";
pub const SPAGHETTI_TARGET: usize = 300;
pub const PARCOORDS_TARGET: usize = 2_000;
pub const PATH_SAMPLE_SEED: u64 = 7;
/// Expiries closer to the snapshot than this many days have no skew in the chart.
pub const SKEW_MIN_DTE: i64 = 2;

pub fn scenario_color(key: &str) -> &'static str {
    match key {
        "bear" => "#9e3a26",
        "base" => "#35618f",
        _ => "#23755f",
    }
}

/// A figure: its data traces and its layout. `height` is the height of its box in pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Fig {
    pub height: u32,
    pub data: Value,
    pub layout: Value,
}

pub fn title_case(key: &str) -> String {
    let mut c = key.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn rgba(hex: &str, alpha: f64) -> String {
    let c = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0);
    format!("rgba({},{},{},{})", c(1), c(3), c(5), alpha)
}

fn num(x: f64) -> Value {
    json!(x)
}

fn nums(v: &[f64]) -> Value {
    Value::Array(v.iter().map(|x| num(*x)).collect())
}

fn opts(v: &[Option<f64>]) -> Value {
    Value::Array(v.iter().map(|x| x.map_or(Value::Null, num)).collect())
}

fn strs<S: AsRef<str>>(v: &[S]) -> Value {
    Value::Array(v.iter().map(|s| Value::String(s.as_ref().to_string())).collect())
}

fn font(size: u32, color: &str) -> Value {
    json!({"size": size, "color": color})
}

/// `fig.update_layout` / `update_xaxes`: merge `src` into `dst` (objects are merged, others are replaced).
pub fn merge(dst: &mut Value, src: Value) {
    match (dst, src) {
        (Value::Object(d), Value::Object(s)) => {
            for (k, v) in s {
                match d.get_mut(&k) {
                    Some(old) if old.is_object() && v.is_object() => merge(old, v),
                    _ => {
                        d.insert(k, v);
                    }
                }
            }
        }
        (d, s) => *d = s,
    }
}

/// Set one value by a dotted path (`"xaxis.range"`), made with objects on the way.
pub fn put(layout: &mut Value, path: &str, value: Value) {
    let mut cur = layout;
    let parts: Vec<&str> = path.split('.').collect();
    for (i, p) in parts.iter().enumerate() {
        if !cur.is_object() {
            *cur = Value::Object(Map::new());
        }
        let m = cur.as_object_mut().expect("object");
        if i + 1 == parts.len() {
            m.insert(p.to_string(), value);
            return;
        }
        cur = m.entry(p.to_string()).or_insert_with(|| Value::Object(Map::new()));
    }
}

fn axis(title: &str) -> Value {
    json!({
        "title": {"text": title, "font": {"size": 11, "color": INK_MUTED}, "standoff": 15},
        "gridcolor": GRID, "gridwidth": 1, "linecolor": AXIS,
        "zeroline": false, "zerolinecolor": AXIS, "zerolinewidth": 1,
        "tickfont": {"size": 11, "color": INK_MUTED},
        // from the plotly Python template, which the page does not carry
        "automargin": true, "ticks": "",
    })
}

/// The shared chrome: transparent backgrounds, hairline grids, ink hover labels.
pub fn chrome(x_title: &str, y_title: &str, show_legend: bool) -> Value {
    json!({
        "paper_bgcolor": "rgba(0,0,0,0)", "plot_bgcolor": "rgba(0,0,0,0)",
        // The plotly Python template sets this: a year as text ("2026") stays text on a category axis.
        "autotypenumbers": "strict",
        "font": {"family": FONT, "size": 12, "color": INK_SECONDARY},
        "colorway": CATEGORICAL,
        "margin": {"l": 46, "r": 12, "t": 12, "b": 38},
        "showlegend": show_legend,
        "legend": {"orientation": "h", "yanchor": "bottom", "y": 1.0, "x": 0,
                   "font": {"size": 11, "color": INK_SECONDARY}, "bgcolor": "rgba(0,0,0,0)", "borderwidth": 0},
        "hoverlabel": {"bgcolor": INK, "bordercolor": INK, "align": "left",
                       "font": {"family": FONT, "size": 12, "color": PAGE}},
        "xaxis": axis(x_title),
        "yaxis": axis(y_title),
    })
}

fn margin(l: u32, r: u32, t: u32, b: u32) -> Value {
    json!({"l": l, "r": r, "t": t, "b": b})
}

fn push(layout: &mut Value, key: &str, item: Value) {
    let m = layout.as_object_mut().expect("layout object");
    m.entry(key.to_string()).or_insert_with(|| Value::Array(Vec::new())).as_array_mut().expect("array").push(item);
}

/// `fig.add_hline`.
fn hline(layout: &mut Value, y: f64, color: &str, width: u32) {
    push(layout, "shapes", json!({"type": "line", "x0": 0, "x1": 1, "xref": "x domain", "y0": y, "y1": y, "yref": "y",
                                    "line": {"color": color, "width": width}}));
}

/// `fig.add_vline`; `x` is a number or a date text.
fn vline(layout: &mut Value, x: Value, color: &str, width: u32) {
    push(layout, "shapes", json!({"type": "line", "x0": x.clone(), "x1": x, "xref": "x", "y0": 0, "y1": 1, "yref": "y domain",
                                    "line": {"color": color, "width": width}}));
}

fn annotation(layout: &mut Value, a: Value) {
    push(layout, "annotations", a);
}

/// Range-window buttons (label, months; `None` = all).
fn rangeselector(buttons: &[(&str, Option<u32>)]) -> Value {
    let b: Vec<Value> = buttons
        .iter()
        .map(|(label, months)| match months {
            None => json!({"label": label, "step": "all"}),
            Some(m) => json!({"label": label, "count": m, "step": "month", "stepmode": "backward"}),
        })
        .collect();
    json!({"x": 0, "xanchor": "left", "y": 1.18, "yanchor": "top", "bgcolor": RAISED, "activecolor": ACCENT_TINT,
           "bordercolor": HAIRLINE, "borderwidth": 1, "font": {"size": 11, "color": INK_SECONDARY}, "buttons": b})
}

fn fig(data: Vec<Value>, layout: Value, height: u32) -> Fig {
    Fig { height, data: Value::Array(data), layout }
}

// ---------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------

fn days(date: &str) -> i64 {
    parse_iso(date).unwrap_or(0)
}

pub fn add_days(date: &str, n: i64) -> String {
    let (y, m, d) = civil_from_days(days(date) + n);
    format!("{y:04}-{m:02}-{d:02}")
}

/// (ISO year, ISO week) of a date.
pub fn iso_week_of(date: &str) -> (i64, i64) {
    let d = days(date);
    let weekday = (d + 3).rem_euclid(7) + 1; // 1970-01-01 was a Thursday: Monday = 1
    let thursday = d - weekday + 4;
    let (y, _, _) = civil_from_days(thursday);
    let week = (thursday - days_from_civil(y, 1, 1)) / 7 + 1;
    (y, week)
}

/// `d.strftime("%d %b %y")`.
pub fn expiry_label(date: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, d) = civil_from_days(days(date));
    format!("{d:02} {} {:02}", MONTHS[(m - 1) as usize], y.rem_euclid(100))
}

/// Calendar days from `a` to `b` (`(b - a).days`).
pub fn days_between(a: &str, b: &str) -> i64 {
    days(b) - days(a)
}

// ---------------------------------------------------------------------------
// Price and valuation
// ---------------------------------------------------------------------------

/// The last point of each ISO week.
fn weekly<'a>(points: &[&'a PricePoint]) -> Vec<&'a PricePoint> {
    let mut out = Vec::new();
    for (i, p) in points.iter().enumerate() {
        let last_of_week = match points.get(i + 1) {
            None => true,
            Some(n) => iso_week_of(&n.date) != iso_week_of(&p.date),
        };
        if last_of_week {
            out.push(*p);
        }
    }
    out
}

/// More points than this: weekly before the trailing year.
pub const WEEKLY_LIMIT: usize = 2600;

pub fn price_history_figure(points: &[PricePoint]) -> Fig {
    let last = &points[points.len() - 1].date;
    let pts: Vec<&PricePoint> = if points.len() > WEEKLY_LIMIT {
        let cut = add_days(last, -365);
        let old: Vec<&PricePoint> = points.iter().filter(|p| p.date < cut).collect();
        let mut v = weekly(&old);
        v.extend(points.iter().filter(|p| p.date >= cut));
        v
    } else {
        points.iter().collect()
    };
    let x: Vec<&str> = pts.iter().map(|p| p.date.as_str()).collect();
    let y: Vec<f64> = pts.iter().map(|p| p.close).collect();
    let trace = json!({
        "type": "scatter", "x": strs(&x), "y": nums(&y), "mode": "lines",
        "line": {"color": CATEGORICAL[0], "width": 2}, "name": "Close",
        "hovertemplate": "%{x|%Y-%m-%d}: %{y:$,.2f}<extra></extra>",
    });
    let mut layout = chrome("", "Close ($)", false);
    annotation(
        &mut layout,
        json!({"x": x[x.len() - 1], "y": y[y.len() - 1], "text": format!("${}", fixed_c(y[y.len() - 1], 0)), "showarrow": false,
               "xanchor": "left", "xshift": 6, "font": font(11, INK_SECONDARY)}),
    );
    merge(&mut layout, json!({
        "margin": margin(46, 56, 30, 38),
        "updatemenus": [{
            "type": "buttons", "direction": "right", "x": 1.0, "xanchor": "right", "y": 1.18, "yanchor": "top",
            "showactive": true, "bgcolor": RAISED, "bordercolor": HAIRLINE, "borderwidth": 1,
            "font": font(11, INK_SECONDARY), "pad": {"l": 4, "r": 4, "t": 2, "b": 2},
            "buttons": [
                {"label": "Linear", "method": "relayout", "args": [{"yaxis.type": "linear"}]},
                {"label": "Log", "method": "relayout", "args": [{"yaxis.type": "log"}]},
            ],
        }],
    }));
    put(&mut layout, "xaxis.rangeselector", rangeselector(&[("1M", Some(1)), ("6M", Some(6)), ("1Y", Some(12)), ("5Y", Some(60)), ("Max", None)]));
    fig(vec![trace], layout, 380)
}

pub type Stats = Vec<(&'static str, [f64; 3])>;

pub fn stat<'a>(stats: &'a Stats, key: &str) -> Option<&'a [f64; 3]> {
    stats.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
}

pub fn valuation_zoom_figure(points: &[PricePoint], scenarios: &[Scenario], stats: &Stats) -> Fig {
    let last = &points[points.len() - 1].date;
    let cutoff = add_days(last, -730);
    let mut recent: Vec<&PricePoint> = points.iter().filter(|p| p.date >= cutoff).collect();
    if recent.is_empty() {
        recent = points.iter().collect();
    }
    let x: Vec<&str> = recent.iter().map(|p| p.date.as_str()).collect();
    let y: Vec<f64> = recent.iter().map(|p| p.close).collect();
    let x_last = x[x.len() - 1];

    let mut layout = chrome("", "Price / value per share ($)", false);
    if let Some([b10, _, b90]) = stat(stats, "base") {
        push(&mut layout, "shapes", json!({"type": "rect", "x0": 0, "x1": 1, "xref": "x domain", "y0": b10, "y1": b90, "yref": "y",
                                           "fillcolor": rgba(scenario_color("base"), 0.10), "line": {"width": 0}}));
        annotation(&mut layout, json!({"xref": "paper", "x": 0.01, "y": b90, "text": "Base P10\u{2013}P90", "showarrow": false,
                                        "xanchor": "left", "yshift": 10, "bgcolor": PAGE, "font": font(11, INK_MUTED)}));
    }
    for sc in scenarios {
        let p50 = stat(stats, sc.key).expect("scenario stats")[1];
        hline(&mut layout, p50, scenario_color(sc.key), 2);
        annotation(&mut layout, json!({"x": x_last, "y": p50, "text": format!("{} P50 ${}", title_case(sc.key), fixed_c(p50, 0)),
                                        "showarrow": false, "xanchor": "left", "xshift": 6, "bgcolor": PAGE, "font": font(11, INK_SECONDARY)}));
    }
    let trace = json!({
        "type": "scatter", "x": strs(&x), "y": nums(&y), "mode": "lines", "line": {"color": INK, "width": 2}, "name": "Price",
        "hovertemplate": "%{x|%Y-%m-%d}: %{y:$,.2f}<extra>Price</extra>",
    });
    let p50s: Vec<f64> = scenarios.iter().map(|sc| stat(stats, sc.key).expect("scenario stats")[1]).collect();
    let mut lo = y.iter().cloned().fold(f64::INFINITY, f64::min).min(p50s.iter().cloned().fold(f64::INFINITY, f64::min));
    let mut hi = y.iter().cloned().fold(f64::NEG_INFINITY, f64::max).max(p50s.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
    if let Some([b10, _, b90]) = stat(stats, "base") {
        lo = lo.min(*b10);
        hi = hi.max(*b90);
    }
    let pad = if (hi - lo) * 0.06 != 0.0 { (hi - lo) * 0.06 } else { 1.0 };
    put(&mut layout, "yaxis.range", json!([lo - pad, hi + pad]));
    put(&mut layout, "xaxis.range", json!([add_days(x_last, -183), x_last]));
    put(&mut layout, "xaxis.rangeselector", rangeselector(&[("1M", Some(1)), ("3M", Some(3)), ("6M", Some(6)), ("1Y", Some(12)), ("2Y", None)]));
    put(&mut layout, "margin", margin(46, 130, 30, 38));
    fig(vec![trace], layout, 380)
}

// ---------------------------------------------------------------------------
// Fundamentals
// ---------------------------------------------------------------------------

pub fn fundamentals_figure(annuals: &[&Period]) -> Fig {
    let years: Vec<String> = annuals.iter().map(|p| p.period_end[..4].to_string()).collect();
    let revenue: Vec<f64> = annuals.iter().map(|p| p.revenue.unwrap_or(f64::NAN) / 1e9).collect();
    let fcf: Vec<Option<f64>> = annuals.iter().map(|p| p.free_cash_flow().map(|v| v / 1e9)).collect();
    let margin_v: Vec<Option<f64>> = annuals.iter().map(|p| p.fcf_margin()).collect();
    let marker = |c: &str| json!({"size": 8, "color": c, "line": {"color": SURFACE, "width": 2}});
    let data = vec![
        json!({"type": "bar", "x": strs(&years), "y": nums(&revenue), "name": "Revenue", "width": 0.55, "marker": {"color": CATEGORICAL[0]},
               "hovertemplate": "FY%{x}: %{y:$,.1f}B<extra>Revenue</extra>", "xaxis": "x", "yaxis": "y"}),
        json!({"type": "scatter", "x": strs(&years), "y": opts(&fcf), "name": "Free cash flow", "mode": "lines+markers",
               "line": {"color": CATEGORICAL[1], "width": 2}, "marker": marker(CATEGORICAL[1]),
               "hovertemplate": "FY%{x}: %{y:$,.1f}B<extra>FCF</extra>", "xaxis": "x", "yaxis": "y"}),
        json!({"type": "scatter", "x": strs(&years), "y": opts(&margin_v), "name": "FCF margin", "mode": "lines+markers", "showlegend": false,
               "line": {"color": CATEGORICAL[1], "width": 2}, "marker": marker(CATEGORICAL[1]),
               "hovertemplate": "FY%{x}: %{y:.1%}<extra>FCF margin</extra>", "xaxis": "x2", "yaxis": "y2"}),
    ];
    let mut layout = chrome("", "", true);
    // Two rows with a shared x axis (`make_subplots(rows=2, row_heights=[0.66, 0.34], vertical_spacing=0.10)`).
    let total = 1.0 - 0.10;
    let (h1, h2) = (0.66 * total, 0.34 * total);
    let ax = |title: &str| {
        let mut a = axis(title);
        merge(&mut a, json!({"title": {"text": title}}));
        a
    };
    let mut xaxis = ax("");
    merge(&mut xaxis, json!({"anchor": "y", "domain": [0.0, 1.0], "matches": "x2", "showticklabels": false, "type": "category"}));
    let mut yaxis = ax("$B");
    merge(&mut yaxis, json!({"anchor": "x", "domain": [1.0 - h1, 1.0]}));
    let mut xaxis2 = ax("");
    merge(&mut xaxis2, json!({"anchor": "y2", "domain": [0.0, 1.0], "type": "category"}));
    let mut yaxis2 = ax("FCF margin");
    merge(&mut yaxis2, json!({"anchor": "x2", "domain": [0.0, h2], "tickformat": ".0%"}));
    merge(&mut layout, json!({"xaxis": xaxis, "yaxis": yaxis, "xaxis2": xaxis2, "yaxis2": yaxis2}));
    // The category axes take the index of the category for an annotation (a text like "2025" would be read as a date).
    let last_x = years.len() as f64 - 1.0;
    let ann = |x: f64, y: f64, text: String, extra: Value, xr: &str, yr: &str| {
        let mut a = json!({"x": x, "y": y, "text": text, "showarrow": false, "xref": xr, "yref": yr, "font": font(11, INK_SECONDARY)});
        merge(&mut a, extra);
        a
    };
    let rev_last = revenue[revenue.len() - 1];
    annotation(&mut layout, ann(last_x, rev_last, format!("${}B", fixed_c(rev_last, 0)), json!({"yshift": 10}), "x", "y"));
    if let Some(v) = fcf.iter().rev().flatten().next() {
        annotation(&mut layout, ann(last_x, *v, format!("FCF ${}B", fixed_c(*v, 1)), json!({"xanchor": "left", "xshift": 8}), "x", "y"));
    }
    if let Some(v) = margin_v.iter().rev().flatten().next() {
        annotation(&mut layout, ann(last_x, *v, pct(*v, 1), json!({"xanchor": "left", "xshift": 8}), "x2", "y2"));
    }
    merge(&mut layout, json!({"bargap": 0.4, "margin": margin(46, 72, 12, 38), "hovermode": "x unified"}));
    fig(data, layout, 420)
}

// ---------------------------------------------------------------------------
// Aggregate DCF scenarios
// ---------------------------------------------------------------------------

/// A scenario with its run.
pub struct Run<'a> {
    pub sc: &'a Scenario,
    pub res: DcfResult,
}

pub fn per_share(res: &DcfResult, cap: &CapitalStructure) -> Vec<f64> {
    res.simulations.iter().map(|v| (v - cap.net_debt) / cap.shares).collect()
}

/// `np.percentile(a, q, axis=1)` for the year-major matrix `a`.
fn year_percentile(a: &[f64], n: usize, year: usize, q: f64) -> f64 {
    percentile(&sorted_copy(&a[year * n..(year + 1) * n]), q)
}

pub fn fan_figure(runs: &[Run], base_year: i32) -> Fig {
    let mut data = Vec::new();
    let mut layout = chrome("Projection year", "Free cash flow ($B)", true);
    merge(&mut layout, json!({"hovermode": "x unified"}));
    let mut years: Vec<i32> = Vec::new();
    for r in runs {
        let (n, ny) = (r.res.n, r.res.years);
        years = (0..ny as i32).map(|i| base_year + 1 + i).collect();
        let col = |q: f64| -> Vec<f64> { (0..ny).map(|t| year_percentile(&r.res.fcfs, n, t, q) / 1e9).collect() };
        let (p10, p50, p90) = (col(10.0), col(50.0), col(90.0));
        let color = scenario_color(r.sc.key);
        let xs: Vec<i32> = years.iter().cloned().chain(years.iter().rev().cloned()).collect();
        let ys: Vec<f64> = p90.iter().cloned().chain(p10.iter().rev().cloned()).collect();
        data.push(json!({"type": "scatter", "x": xs, "y": nums(&ys), "fill": "toself", "fillcolor": rgba(color, 0.10), "line": {"width": 0},
                         "hoverinfo": "skip", "legendgroup": r.sc.key, "showlegend": false, "name": format!("{} P10\u{2013}P90", r.sc.key)}));
        data.push(json!({"type": "scatter", "x": years, "y": nums(&p50), "mode": "lines", "line": {"color": color, "width": 2},
                         "legendgroup": r.sc.key, "name": title_case(r.sc.key),
                         "hovertemplate": format!("%{{y:$.1f}}B<extra>{}</extra>", title_case(r.sc.key))}));
        annotation(&mut layout, json!({"x": years[years.len() - 1], "y": p50[p50.len() - 1], "text": title_case(r.sc.key), "xanchor": "left",
                                        "showarrow": false, "xshift": 6, "font": font(11, INK_SECONDARY)}));
    }
    put(&mut layout, "margin", margin(46, 56, 12, 38));
    if !years.is_empty() {
        put(&mut layout, "xaxis.tickmode", json!("array"));
        put(&mut layout, "xaxis.tickvals", json!(years));
    }
    fig(data, layout, 380)
}

pub fn distribution_figure(runs: &[Run], cap: &CapitalStructure, price: Option<f64>) -> Fig {
    let ps: Vec<(&'static str, Vec<f64>)> = runs.iter().map(|r| (r.sc.key, per_share(&r.res, cap))).collect();
    let mut lo = ps.iter().map(|(_, v)| v.iter().cloned().fold(f64::INFINITY, f64::min)).fold(f64::INFINITY, f64::min);
    let mut view_hi = ps.iter().map(|(_, v)| percentile(&sorted_copy(v), 99.0)).fold(f64::NEG_INFINITY, f64::max);
    if let Some(p) = price {
        lo = lo.min(p);
        view_hi = view_hi.max(p * 1.1);
    }
    let size = if (view_hi - lo) / 60.0 != 0.0 { (view_hi - lo) / 60.0 } else { 1.0 };
    let data_hi = ps.iter().map(|(_, v)| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max)).fold(f64::NEG_INFINITY, f64::max);
    let data: Vec<Value> = ps
        .iter()
        .map(|(key, v)| {
            json!({"type": "histogram", "x": nums(v), "name": title_case(key), "xbins": {"start": lo, "end": data_hi + size, "size": size},
                   "marker": {"color": scenario_color(key)}, "opacity": 0.55,
                   "hovertemplate": format!("%{{y:,}} sims at %{{x:$,.0f}}<extra>{}</extra>", title_case(key))})
        })
        .collect();
    let mut layout = chrome("Equity value per share ($)", "Simulations", true);
    merge(&mut layout, json!({"barmode": "overlay"}));
    if let Some(p) = price {
        vline(&mut layout, num(p), INK, 2);
        annotation(&mut layout, json!({"x": p, "yref": "paper", "y": 0.94, "yanchor": "top", "xanchor": "left", "xshift": 6,
                                        "text": format!("Price ${}", fixed_c(p, 2)), "showarrow": false, "font": font(11, INK)}));
    }
    put(&mut layout, "xaxis.range", json!([lo, view_hi]));
    fig(data, layout, 380)
}

pub fn waterfall_figure(base: &DcfResult, cap: &CapitalStructure, base_year: i32) -> Fig {
    let (n, ny) = (base.n, base.years);
    let med_pv: Vec<f64> = (0..ny).map(|t| year_percentile(&base.pv_fcfs, n, t, 50.0) / 1e9).collect();
    let med_terminal = percentile(&sorted_copy(&base.pv_terminal), 50.0) / 1e9;
    let net_debt = cap.net_debt / 1e9;
    let sum: f64 = med_pv.iter().sum();
    let equity = sum + med_terminal - net_debt;
    let per_share_v = equity * 1e9 / cap.shares;
    let terminal_pct = med_terminal / (sum + med_terminal);
    let mut labels: Vec<String> = (0..ny).map(|i| (base_year + 1 + i as i32).to_string()).collect();
    labels.extend(["Terminal value".to_string(), "Net debt".to_string(), "Equity value".to_string()]);
    let mut values = med_pv.clone();
    values.extend([med_terminal, -net_debt, 0.0]);
    let mut measure: Vec<&str> = vec!["relative"; ny + 2];
    measure.push("total");
    let data = vec![json!({
        "type": "waterfall", "x": strs(&labels), "y": nums(&values), "measure": measure,
        "increasing": {"marker": {"color": COOL}}, "decreasing": {"marker": {"color": WARM}}, "totals": {"marker": {"color": INK_SECONDARY}},
        "connector": {"line": {"color": GRID, "width": 1}}, "hovertemplate": "%{x}: %{delta:$,.1f}B<extra></extra>",
    })];
    let mut layout = chrome("", "Present value ($B)", false);
    annotation(&mut layout, json!({"x": "Terminal value", "y": sum + med_terminal, "text": format!("{} of gross PV", pct(terminal_pct, 0)),
                                    "showarrow": false, "yshift": 14, "font": font(11, INK_SECONDARY)}));
    annotation(&mut layout, json!({"x": "Equity value", "y": sum + med_terminal, "text": format!("${}/share", fixed_c(per_share_v, 0)),
                                    "showarrow": false, "yshift": 36, "font": font(11, INK)}));
    put(&mut layout, "xaxis.tickangle", json!(-45));
    put(&mut layout, "yaxis.rangemode", json!("tozero"));
    fig(data, layout, 380)
}

/// A row of a tornado: the label, the change of the P50 for the low input and for the high input.
pub type TornadoRow = (String, f64, f64);

/// The tornado bars; the caller sorts the rows.
pub fn tornado_bars(base_ps: f64, rows: &[TornadoRow]) -> Fig {
    let labels: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    let lows: Vec<f64> = rows.iter().map(|r| r.1).collect();
    let highs: Vec<f64> = rows.iter().map(|r| r.2).collect();
    let mut data = Vec::new();
    for (name, deltas, color) in [("Input \u{2212}1\u{3c3}", &lows, COOL), ("Input +1\u{3c3}", &highs, WARM)] {
        let text: Vec<String> = deltas.iter().map(|d| signed_c(*d, 0)).collect();
        data.push(json!({
            "type": "bar", "y": strs(&labels), "x": nums(deltas), "base": base_ps, "orientation": "h", "name": name,
            "marker": {"color": color}, "width": 0.5, "text": strs(&text), "textposition": "outside", "cliponaxis": false,
            "textfont": font(11, INK_SECONDARY), "hovertemplate": format!("%{{y}}: %{{text}} vs base P50<extra>{name}</extra>"),
        }));
    }
    let mut layout = chrome("P50 equity value per share ($)", "", true);
    merge(&mut layout, json!({"barmode": "overlay"}));
    vline(&mut layout, num(base_ps), AXIS, 1);
    annotation(&mut layout, json!({"x": base_ps, "yref": "paper", "y": 1.0, "yanchor": "bottom", "text": format!("Base P50 ${}", fixed_c(base_ps, 0)),
                                    "showarrow": false, "font": font(11, INK_MUTED)}));
    let span = lows.iter().chain(highs.iter()).fold(0.0f64, |a, d| a.max(d.abs()));
    let span = if span != 0.0 { span } else { 1.0 };
    put(&mut layout, "xaxis.range", json!([base_ps - span * 1.35, base_ps + span * 1.35]));
    let rev: Vec<&str> = labels.iter().rev().cloned().collect();
    put(&mut layout, "yaxis.categoryorder", json!("array"));
    put(&mut layout, "yaxis.categoryarray", strs(&rev));
    put(&mut layout, "yaxis.gridcolor", json!(SURFACE));
    fig(data, layout, 380)
}

/// Sort the tornado rows by the sum of their absolute deltas, largest first (a stable sort).
pub fn sort_tornado(rows: &mut [TornadoRow]) {
    rows.sort_by(|a, b| (b.1.abs() + b.2.abs()).total_cmp(&(a.1.abs() + a.2.abs())));
}

/// The four inputs of the classic tornado: (label, how to read and write the mean and the std).
pub type TornadoInput = (&'static str, fn(&DcfAssumptions) -> (f64, f64), fn(&mut DcfAssumptions, f64));

pub fn tornado_inputs() -> [TornadoInput; 4] {
    [
        ("Revenue growth", |a| (a.growth_rate_mean, a.growth_rate_std), |a, v| a.growth_rate_mean = v),
        ("WACC", |a| (a.wacc_mean, a.wacc_std), |a, v| a.wacc_mean = v),
        ("Terminal growth", |a| (a.terminal_growth_mean, a.terminal_growth_std), |a, v| a.terminal_growth_mean = v),
        ("FCF margin", |a| (a.fcf_margin_mean.unwrap_or(0.0), a.fcf_margin_std.unwrap_or(0.0)), |a, v| a.fcf_margin_mean = Some(v)),
    ]
}

// ---------------------------------------------------------------------------
// Driver model
// ---------------------------------------------------------------------------

pub fn segment_history_figure(segment_names: &[String], history: &BTreeMap<String, BTreeMap<String, f64>>) -> Fig {
    let mut names: Vec<&String> = segment_names.iter().filter(|n| history.contains_key(*n)).collect();
    for n in history.keys() {
        if !names.contains(&n) {
            names.push(n);
        }
    }
    let mut years: Vec<&String> = history.values().flat_map(|h| h.keys()).collect();
    years.sort();
    years.dedup();
    let data: Vec<Value> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let h = &history[*name];
            let y: Vec<f64> = years.iter().map(|yr| h.get(*yr).copied().unwrap_or(0.0) / 1e9).collect();
            json!({"type": "bar", "x": strs(&years), "y": nums(&y), "name": name, "width": 0.55,
                   "marker": {"color": CATEGORICAL[i % CATEGORICAL.len()], "line": {"color": SURFACE, "width": 2}},
                   "hovertemplate": format!("FY%{{x}}: %{{y:$,.1f}}B<extra>{name}</extra>")})
        })
        .collect();
    let mut layout = chrome("", "Segment revenue ($B)", true);
    merge(&mut layout, json!({"barmode": "stack", "bargap": 0.4}));
    put(&mut layout, "xaxis.type", json!("category"));
    fig(data, layout, 380)
}

pub fn corr_heatmap_figure(names: &[String], mat: &[Vec<f64>]) -> Fig {
    let k = names.len();
    let z: Vec<Vec<f64>> = mat.iter().rev().cloned().collect();
    let text: Vec<Vec<String>> = (0..k).rev().map(|i| (0..k).map(|j| if i == j { String::new() } else { fixed(mat[i][j], 2) }).collect()).collect();
    let rev: Vec<&String> = names.iter().rev().collect();
    let scale = json!([[0.0, WARM], [0.5, DIVERGING_MID], [1.0, COOL]]);
    let data = vec![json!({
        "type": "heatmap", "z": z, "x": strs(names), "y": strs(&rev), "zmin": -1, "zmax": 1, "colorscale": scale,
        "text": text, "texttemplate": "%{text}", "textfont": font(10, INK_SECONDARY), "showscale": false, "xgap": 2, "ygap": 2,
        "hovertemplate": "%{y} \u{d7} %{x}: %{z:.2f}<extra></extra>",
    })];
    let mut layout = chrome("", "", false);
    put(&mut layout, "xaxis.tickangle", json!(-35));
    put(&mut layout, "xaxis.showgrid", json!(false));
    put(&mut layout, "yaxis.showgrid", json!(false));
    fig(data, layout, 380)
}

pub fn driver_compare_figure(driver_stats: [f64; 3], base_stats: [f64; 3], price: Option<f64>) -> Fig {
    let rows = [("Aggregate base", base_stats, scenario_color("base"), 18), ("Driver model", driver_stats, DRIVER_COLOR, -24)];
    let mut data = Vec::new();
    let mut layout = chrome("Equity value per share ($)", "", false);
    for (label, [p10, p50, p90], color, shift) in rows {
        data.push(json!({"type": "scatter", "x": [p10, p90], "y": [label, label], "mode": "lines",
                         "line": {"color": rgba(color, 0.35), "width": 6}, "hoverinfo": "skip", "showlegend": false}));
        data.push(json!({"type": "scatter", "x": [p10, p50, p90], "y": [label, label, label], "mode": "markers", "name": label,
                         "marker": {"size": [9, 13, 9], "color": color, "line": {"color": SURFACE, "width": 2}},
                         "hovertemplate": format!("%{{x:$,.0f}}<extra>{label}</extra>")}));
        for (v, tag) in [(p10, "P10"), (p50, "P50"), (p90, "P90")] {
            annotation(&mut layout, json!({"x": v, "y": label, "text": format!("{tag} ${}", fixed_c(v, 0)), "showarrow": false, "yshift": shift,
                                            "bgcolor": PAGE, "font": font(11, INK_SECONDARY)}));
        }
    }
    if let Some(p) = price {
        vline(&mut layout, num(p), INK, 2);
        annotation(&mut layout, json!({"x": p, "yref": "paper", "y": 1.0, "yanchor": "bottom", "text": format!("Price ${}", fixed_c(p, 2)),
                                        "showarrow": false, "font": font(11, INK)}));
    }
    put(&mut layout, "margin", margin(46, 12, 28, 38));
    put(&mut layout, "yaxis.showgrid", json!(false));
    fig(data, layout, 380)
}

pub fn driver_tornado_figure(result: &DriverDcfResult) -> Fig {
    let mut rows: Vec<TornadoRow> =
        result.attribution.iter().map(|a| (a.driver.clone(), a.p50_minus_1sd, a.p50_plus_1sd)).collect();
    sort_tornado(&mut rows);
    tornado_bars(result.per_share_p50, &rows)
}

// ---------------------------------------------------------------------------
// Path explorer
// ---------------------------------------------------------------------------

/// All indices below P1 and above P99 of `per_share`, plus a seeded random sample of the middle band
/// that fills the total up to `target`. The middle indices are sorted.
/// The indices of the bottom tail, the middle sample and the top tail.
pub type PathSample = (Vec<usize>, Vec<usize>, Vec<usize>);

pub fn stratified_path_sample(per_share: &[f64], target: usize, seed: u64) -> Result<PathSample, String> {
    let sorted = sorted_copy(per_share);
    let (p1, p99) = (percentile(&sorted, 1.0), percentile(&sorted, 99.0));
    let bottom: Vec<usize> = (0..per_share.len()).filter(|i| per_share[*i] < p1).collect();
    let top: Vec<usize> = (0..per_share.len()).filter(|i| per_share[*i] > p99).collect();
    let mut middle: Vec<usize> = (0..per_share.len()).filter(|i| per_share[*i] >= p1 && per_share[*i] <= p99).collect();
    let n_mid = target.saturating_sub(bottom.len() + top.len());
    if n_mid < middle.len() {
        let mut rng = crate::dcf::nprandom::Pcg64::new(seed);
        let picked = rng.choice_set(middle.len(), n_mid).ok_or("the path sample needs a larger numpy sampling case")?;
        middle = picked.into_iter().map(|i| middle[i]).collect();
        middle.sort_unstable();
    }
    Ok((bottom, middle, top))
}

fn driver_short(name: &str) -> String {
    match name {
        "g_software" => "g_sw",
        "g_consulting" => "g_cons",
        "g_infrastructure" => "g_infra",
        "g_financing" => "g_fin",
        "core_margin" => "margin",
        "ma_ratio" => "ma",
        "rf" => "rf",
        "erp" => "erp",
        "terminal_growth" => "tg",
        "sbc_ratio" => "sbc",
        "beta" => "beta",
        "kd_spread" => "kd",
        "tax_rate" => "tax",
        "equity_weight" => "e/v",
        "margin_mix_drift" => "mix",
        "ma_recurrence" => "ma_rec",
        other => return if other.chars().count() <= 8 { other.to_string() } else { other.chars().take(8).collect() },
    }
    .to_string()
}

pub fn short_name(name: &str) -> String {
    driver_short(name)
}

/// Percent format? True for growth, margin and rate drivers: every draw is inside (-1.5, 1.5), except beta.
pub fn rate_like(name: &str, values: &[f64]) -> bool {
    name != "beta" && values.iter().fold(0.0f64, |a, v| a.max(v.abs())) < 1.5
}

fn path_hover(stoch: &[(String, Vec<f64>)], group: &str) -> String {
    let parts: Vec<String> = stoch
        .iter()
        .enumerate()
        .map(|(i, (name, vals))| {
            let f = if rate_like(name, vals) { ".1%" } else { ".2f" };
            format!("{} %{{customdata[{}]:{f}}}", driver_short(name), i + 2)
        })
        .collect();
    let lines: Vec<String> = parts.chunks(4).map(|c| c.join(" \u{b7} ")).collect();
    format!("Path %{{customdata[0]:.0f}} \u{b7} %{{customdata[1]:$,.2f}}/sh<br>{}<extra>{group}</extra>", lines.join("<br>"))
}

/// The FCFF path "spaghetti". Returns the figure and the number of distinct paths drawn.
pub fn spaghetti_figure(result: &DriverDcfResult, base_year: i32) -> Result<(Fig, usize), String> {
    let per_share_v = &result.per_share_paths;
    let v = &result.valuation;
    let (n, ny) = (v.n, v.years);
    let years: Vec<i32> = (0..ny as i32).map(|i| base_year + 1 + i).collect();
    let (bottom, middle, top) = stratified_path_sample(per_share_v, SPAGHETTI_TARGET, PATH_SAMPLE_SEED)?;
    let stoch = &result.driver_draws;
    let fcff_b = |t: usize, i: usize| v.fcff[t * n + i] / 1e9;
    let custom_row = |i: usize| -> Value {
        let mut row = vec![num(i as f64), num(pyround(per_share_v[i], 2))];
        row.extend(stoch.iter().map(|(_, d)| num(pyround(d[i], 5))));
        Value::Array(row)
    };
    let group_trace = |idx: &[usize], name: &str, color: String, width: f64| -> Value {
        let (mut xs, mut ys, mut cd) = (Vec::new(), Vec::new(), Vec::new());
        for &i in idx {
            let row = custom_row(i);
            for (t, yr) in years.iter().enumerate() {
                xs.push(json!(yr));
                ys.push(num(pyround(fcff_b(t, i), 3)));
                cd.push(row.clone());
            }
            xs.push(Value::Null);
            ys.push(Value::Null);
            cd.push(row.clone());
        }
        json!({"type": "scatter", "x": xs, "y": ys, "customdata": cd, "mode": "lines", "name": name,
               "line": {"color": color, "width": width}, "hovertemplate": path_hover(stoch, name)})
    };
    let mut data = vec![
        group_trace(&middle, "Middle (P1\u{2013}P99)", rgba(INK_MUTED, 0.12), 1.0),
        group_trace(&bottom, "Bottom 1%", rgba(scenario_color("bear"), 0.6), 1.5),
        group_trace(&top, "Top 1%", rgba(scenario_color("bull"), 0.6), 1.5),
    ];
    let p50 = percentile(&sorted_copy(per_share_v), 50.0);
    let med = (0..per_share_v.len()).fold((0usize, f64::INFINITY), |best, i| {
        let d = (per_share_v[i] - p50).abs();
        if d < best.1 { (i, d) } else { best }
    }).0;
    let med_row = custom_row(med);
    data.push(json!({
        "type": "scatter", "x": years, "y": (0..ny).map(|t| num(pyround(fcff_b(t, med), 3))).collect::<Vec<_>>(),
        "customdata": vec![med_row; ny], "mode": "lines", "name": "Median path", "line": {"color": INK, "width": 2.5},
        "hovertemplate": path_hover(stoch, "Median path"),
    }));
    let mut layout = chrome("Projection year", "FCFF ($B)", true);
    annotation(&mut layout, json!({"x": years[ny - 1], "y": fcff_b(ny - 1, med), "text": "Median path", "xanchor": "left", "xshift": 6,
                                    "showarrow": false, "font": font(11, INK)}));
    put(&mut layout, "margin", margin(46, 76, 12, 38));
    put(&mut layout, "xaxis.tickmode", json!("array"));
    put(&mut layout, "xaxis.tickvals", json!(years));
    let mut shown: Vec<usize> = bottom.iter().chain(middle.iter()).chain(top.iter()).cloned().collect();
    shown.push(med);
    shown.sort_unstable();
    shown.dedup();
    Ok((fig(data, layout, 380), shown.len()))
}

pub fn parcoords_figure(result: &DriverDcfResult) -> Result<Fig, String> {
    let per_share_v = &result.per_share_paths;
    let (bottom, middle, top) = stratified_path_sample(per_share_v, PARCOORDS_TARGET, PATH_SAMPLE_SEED)?;
    let idx: Vec<usize> = bottom.into_iter().chain(middle).chain(top).collect();
    let mut dims = Vec::new();
    for (name, draws) in &result.driver_draws {
        let rate = rate_like(name, draws);
        let vals: Vec<f64> = idx.iter().map(|i| npround(if rate { draws[*i] * 100.0 } else { draws[*i] }, 3)).collect();
        dims.push(json!({"label": driver_short(name), "values": nums(&vals)}));
    }
    let ps: Vec<f64> = idx.iter().map(|i| npround(per_share_v[*i], 2)).collect();
    dims.push(json!({"label": "$/sh", "values": nums(&ps)}));
    let scale: Vec<Value> = SEQ_STEEL.iter().map(|(p, c)| json!([p, c])).collect();
    let data = vec![json!({
        "type": "parcoords", "dimensions": dims,
        "line": {"color": nums(&ps), "colorscale": scale, "showscale": false},
        "labelfont": {"family": FONT, "size": 12, "color": INK_SECONDARY},
        "tickfont": {"family": FONT, "size": 10, "color": INK_MUTED},
        "rangefont": {"family": FONT, "size": 9, "color": INK_MUTED},
    })];
    let layout = json!({
        "paper_bgcolor": "rgba(0,0,0,0)", "plot_bgcolor": "rgba(0,0,0,0)",
        "font": {"family": FONT, "size": 12, "color": INK_SECONDARY},
        "margin": margin(48, 48, 44, 24),
    });
    Ok(fig(data, layout, 420))
}

/// Mean and population std of a driver's draws, for the z-scores of the outlier table.
pub fn draw_stats(values: &[f64]) -> (f64, f64) {
    (mean(values), std_pop(values))
}

// ---------------------------------------------------------------------------
// Backtest
// ---------------------------------------------------------------------------

/// Actuals as an ink line; each vintage's P50 projection as a spur off its base-year actual, with the P10-P90 whiskers.
pub fn hedgehog_figure(result: &BacktestResult, actuals: &BTreeMap<i32, f64>, label: &str) -> Fig {
    let spur = rgba(ACCENT, 0.55);
    let whisker = rgba(ACCENT, 0.35);
    let mut data = Vec::new();
    for v in &result.vintages {
        let proj = &v.projection;
        let bands = if label == "Revenue" { &proj.rev } else { &proj.fcf };
        let (p10s, p50s, p90s) = (&bands[0], &bands[1], &bands[2]);
        let base_fy = v.params.base_fiscal_year;
        let mut xs: Vec<i32> = proj.target_fiscal_years.clone();
        let mut ys: Vec<f64> = p50s.iter().map(|p| p / 1e9).collect();
        let mut up: Vec<f64> = p90s.iter().zip(p50s).map(|(hi, mid)| (hi - mid) / 1e9).collect();
        let mut dn: Vec<f64> = p50s.iter().zip(p10s).map(|(mid, lo)| (mid - lo) / 1e9).collect();
        if let Some(origin) = actuals.get(&base_fy) {
            xs.insert(0, base_fy);
            ys.insert(0, origin / 1e9);
            up.insert(0, 0.0);
            dn.insert(0, 0.0);
        }
        let vlabel = v.params.as_of.clone().unwrap_or_else(|| format!("FY{base_fy}+lag"));
        data.push(json!({
            "type": "scatter", "x": xs, "y": nums(&ys), "mode": "lines+markers", "line": {"color": spur, "width": 1.4},
            "marker": {"size": 4, "color": spur},
            "error_y": {"type": "data", "array": nums(&up), "arrayminus": nums(&dn), "color": whisker, "thickness": 1, "width": 3},
            "name": format!("v{base_fy}"), "hovertemplate": format!("FY%{{x}} P50 $%{{y:,.1f}}B<extra>vintage {vlabel}</extra>"),
        }));
    }
    let yrs: Vec<i32> = actuals.keys().cloned().collect();
    let ay: Vec<f64> = actuals.values().map(|v| v / 1e9).collect();
    data.push(json!({
        "type": "scatter", "x": yrs, "y": nums(&ay), "mode": "lines+markers", "line": {"color": INK, "width": 2.2},
        "marker": {"size": 5, "color": INK}, "name": "Actual", "hovertemplate": "FY%{x} actual $%{y:,.1f}B<extra></extra>",
    }));
    let mut layout = chrome("Fiscal year", &format!("{label} ($B)"), false);
    put(&mut layout, "xaxis.dtick", json!(1));
    put(&mut layout, "xaxis.tickformat", json!("d"));
    fig(data, layout, 380)
}

// ---------------------------------------------------------------------------
// Options market
// ---------------------------------------------------------------------------

pub fn iv_term_figure(metrics: &ChainMetrics, next_earnings: Option<&str>) -> Option<Fig> {
    let ems: Vec<_> = metrics.expiries.iter().filter(|e| e.atm_iv.is_some()).collect();
    if ems.is_empty() {
        return None;
    }
    let x: Vec<&str> = ems.iter().map(|e| e.expiry.as_str()).collect();
    let y: Vec<f64> = ems.iter().map(|e| e.atm_iv.unwrap_or(0.0) * 100.0).collect();
    let data = vec![json!({
        "type": "scatter", "x": strs(&x), "y": nums(&y), "mode": "lines+markers", "name": "ATM IV",
        "line": {"color": CATEGORICAL[0], "width": 2}, "marker": {"size": 8, "color": CATEGORICAL[0], "line": {"color": SURFACE, "width": 2}},
        "hovertemplate": "%{x|%Y-%m-%d}: %{y:.1f}%<extra>ATM IV</extra>",
    })];
    let mut layout = chrome("Expiry", "ATM implied vol (%)", false);
    if let Some(ne) = next_earnings {
        vline(&mut layout, json!(ne), AXIS, 1);
        annotation(&mut layout, json!({"x": ne, "yref": "paper", "y": 0.02, "yanchor": "bottom", "xanchor": "left", "xshift": 6,
                                        "text": format!("Earnings {ne}"), "showarrow": false, "font": font(11, INK_MUTED)}));
        if let Some(h) = ems.iter().find(|e| e.expiry.as_str() >= ne) {
            let iv = h.atm_iv.unwrap_or(0.0) * 100.0;
            annotation(&mut layout, json!({"x": h.expiry, "y": iv, "text": format!("Earnings hump \u{2014} {}%", fixed(iv, 1)), "showarrow": true,
                                            "arrowhead": 2, "arrowcolor": INK_SECONDARY, "ax": 40, "ay": -32, "font": font(11, INK_SECONDARY)}));
        }
    }
    Some(fig(data, layout, 380))
}

pub fn implied_move_figure(metrics: &ChainMetrics) -> Option<Fig> {
    let ems: Vec<_> = metrics.expiries.iter().filter(|e| e.implied_move.is_some()).collect();
    if ems.is_empty() {
        return None;
    }
    let x: Vec<String> = ems.iter().map(|e| expiry_label(&e.expiry)).collect();
    let y: Vec<f64> = ems.iter().map(|e| e.implied_move.unwrap_or(0.0) * 100.0).collect();
    let mut trace = json!({
        "type": "bar", "x": strs(&x), "y": nums(&y), "width": 0.55, "marker": {"color": CATEGORICAL[0]},
        "textposition": "outside", "cliponaxis": false, "textfont": font(10, INK_SECONDARY),
        "hovertemplate": "%{x}: \u{b1}%{y:.1f}%<extra>Implied move</extra>",
    });
    if y.len() <= 8 {
        let labels: Vec<String> = y.iter().map(|v| format!("{}%", fixed(*v, 1))).collect();
        trace["text"] = strs(&labels);
    }
    let mut layout = chrome("Expiry", "Implied move (\u{b1}%)", false);
    merge(&mut layout, json!({"bargap": 0.4}));
    put(&mut layout, "xaxis.tickangle", json!(-35));
    Some(fig(vec![trace], layout, 380))
}

pub fn skew_figure(metrics: &ChainMetrics) -> Option<Fig> {
    let ems: Vec<_> = metrics
        .expiries
        .iter()
        .filter(|e| e.skew.is_some() && days_between(&metrics.snapshot_date, &e.expiry) >= SKEW_MIN_DTE)
        .collect();
    if ems.is_empty() {
        return None;
    }
    let x: Vec<String> = ems.iter().map(|e| expiry_label(&e.expiry)).collect();
    let y: Vec<f64> = ems.iter().map(|e| e.skew.unwrap_or(0.0) * 100.0).collect();
    let colors: Vec<&str> = y.iter().map(|v| if *v >= 0.0 { COOL } else { WARM }).collect();
    let mut trace = json!({
        "type": "bar", "x": strs(&x), "y": nums(&y), "width": 0.55, "marker": {"color": colors},
        "textposition": "outside", "cliponaxis": false, "textfont": font(10, INK_SECONDARY),
        "hovertemplate": "%{x}: %{y:+.1f} IV pts<extra>Skew</extra>",
    });
    if y.len() <= 8 {
        let labels: Vec<String> = y.iter().map(|v| signed(*v, 1)).collect();
        trace["text"] = strs(&labels);
    }
    let mut layout = chrome("Expiry", "Skew (90% put IV \u{2212} 110% call IV, pts)", false);
    hline(&mut layout, 0.0, AXIS, 1);
    merge(&mut layout, json!({"bargap": 0.4}));
    put(&mut layout, "xaxis.tickangle", json!(-35));
    Some(fig(vec![trace], layout, 380))
}

pub fn oi_heatmap_figure(snapshot: &OptionChainSnapshot, spot: Option<f64>) -> Option<Fig> {
    let mut expiries: Vec<&str> = snapshot.quotes.iter().map(|q| q.expiry.as_str()).collect();
    expiries.sort_unstable();
    expiries.dedup();
    expiries.truncate(4);
    if expiries.is_empty() {
        return None;
    }
    let window = spot.filter(|s| *s != 0.0).map(|s| (s * 0.75, s * 1.25));
    let mut oi: Vec<BTreeMap<u64, (f64, i64)>> = vec![BTreeMap::new(); expiries.len()];
    // The key keeps the strikes in numeric order (all strikes are positive).
    for q in &snapshot.quotes {
        let Some(e) = expiries.iter().position(|x| *x == q.expiry) else { continue };
        let open = q.open_interest.unwrap_or(0);
        if open == 0 {
            continue;
        }
        if let Some((lo, hi)) = window {
            if !(lo <= q.strike && q.strike <= hi) {
                continue;
            }
        }
        oi[e].entry(q.strike.to_bits()).or_insert((q.strike, 0)).1 += open;
    }
    let mut strikes: Vec<f64> = oi.iter().flat_map(|m| m.values().map(|(s, _)| *s)).collect();
    strikes.sort_by(f64::total_cmp);
    strikes.dedup();
    if strikes.is_empty() {
        return None;
    }
    let z: Vec<Vec<Value>> = oi.iter().map(|row| strikes.iter().map(|s| row.get(&s.to_bits()).map_or(Value::Null, |(_, n)| json!(n))).collect()).collect();
    let labels: Vec<String> = expiries.iter().map(|e| expiry_label(e)).collect();
    let scale: Vec<Value> = SEQ_STEEL.iter().map(|(p, c)| json!([p, c])).collect();
    let data = vec![json!({
        "type": "heatmap", "z": z, "x": nums(&strikes), "y": strs(&labels), "colorscale": scale, "xgap": 2, "ygap": 2, "zmin": 0,
        "colorbar": {"title": {"text": "OI", "font": {"color": INK_MUTED}}, "thickness": 12, "outlinewidth": 0, "tickfont": {"color": INK_MUTED}},
        "hovertemplate": "%{y} \u{b7} $%{x}: %{z:,} contracts<extra></extra>",
    })];
    let mut layout = chrome("Strike ($)", "", false);
    if let Some(s) = spot {
        vline(&mut layout, num(s), INK, 2);
        annotation(&mut layout, json!({"x": s, "yref": "paper", "y": 1.0, "yanchor": "bottom", "text": format!("Spot ${}", fixed_c(s, 2)),
                                        "showarrow": false, "font": font(11, INK)}));
    }
    put(&mut layout, "margin", margin(46, 12, 28, 38));
    put(&mut layout, "xaxis.showgrid", json!(false));
    put(&mut layout, "yaxis.showgrid", json!(false));
    put(&mut layout, "yaxis.autorange", json!("reversed"));
    Some(fig(data, layout, 380))
}

/// The correlation matrix of the stochastic drivers (for the heatmap).
pub fn correlation(model: &DriverModel) -> Result<(Vec<String>, Vec<Vec<f64>>), String> {
    let names = model.stochastic_drivers().iter().map(|d| d.name.clone()).collect();
    let mat = model.correlation_matrix().map_err(|e| e.0)?;
    Ok((names, mat))
}
