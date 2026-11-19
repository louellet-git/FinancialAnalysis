//! The printed numbers of the three Python runners, made again with the generic features of
//! `analysis.rs` and `Overrides`. The expected text is the recorded output of
//! `analysis/IBM/dcf_drivers_run.py`, `analysis/JNJ/first_pass_run.py` and
//! `analysis/ANT/first_pass_run.py` (`rust/tests/fixtures/dcf/runner_*.txt`). Each Rust line
//! must be a line of that text, character for character.

use super::analysis::{at_means, full, implied_mean, p50_with_mean, quick};
use super::driver::Ramp;
use super::model::{hand_dcf_at_means, ModelSpec, Overrides};
use super::stats::{percentile, percentiles, share_below, sorted_copy};

const N: usize = 10_000;
const SEED: u64 = 7;

fn text(t: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/dcf/runner_{t}.txt", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn model(t: &str) -> ModelSpec {
    ModelSpec::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".."), t).unwrap().unwrap()
}

#[track_caller]
fn has(doc: &str, line: &str) {
    assert!(doc.lines().any(|l| l == line), "line not in the Python output:\n{line}");
}

fn mean(name: &str, v: f64) -> Overrides {
    Overrides::mean(name, v)
}

// ---------------------------------------------------------------------------
// IBM: analysis/IBM/dcf_drivers_run.py
// ---------------------------------------------------------------------------

fn ibm_row(label: &str, q: &super::analysis::Quick) -> String {
    format!("  {label:<28} P10 ${:7.2}   P50 ${:7.2}   P90 ${:7.2}", q.p10, q.p50, q.p90)
}

#[test]
fn ibm_runner_headline_and_attribution() {
    let doc = text("IBM");
    let m = model("IBM");
    let r = full(&m, &Overrides::default(), N, SEED).unwrap();
    has(&doc, &format!("  {:<28} P10 ${:7.2}   P50 ${:7.2}   P90 ${:7.2}", "Base", r.per_share_p10, r.per_share_p50, r.per_share_p90));
    has(&doc, &format!("  TV/EV at means: {:.1}%   tg guard pinned fraction: {:.4}", r.tv_ev_share_at_means * 100.0, r.tg_pinned_fraction));
    let mut ranked = r.attribution.clone();
    ranked.sort_by(|a, b| {
        let k = |x: &super::driver::DriverAttribution| x.p50_minus_1sd.abs().max(x.p50_plus_1sd.abs());
        k(b).partial_cmp(&k(a)).unwrap()
    });
    for a in &ranked {
        has(&doc, &format!("  {:<18} {:+9.2} {:+9.2}", a.driver, a.p50_minus_1sd, a.p50_plus_1sd));
    }
    assert_eq!(ranked.len(), 8);
}

#[test]
fn ibm_runner_ev_fixed_point() {
    let doc = text("IBM");
    let m = model("IBM");
    let mut w = 0.765;
    let mut last = 0.0;
    for i in 1..=20 {
        let ps = at_means(&m, &mean("equity_weight", w)).unwrap();
        let equity = ps * m.shares;
        let w_new = equity / (equity + m.net_debt);
        has(&doc, &format!("  iter {i}: E/V in {w:.4} -> value/share ${ps:7.2} -> E/V out {w_new:.4}"));
        last = w_new;
        if (w_new - w).abs() < 5e-4 {
            break;
        }
        w = w_new;
    }
    has(&doc, &format!("  Fixed point E/V ~ {last:.3} (model uses 0.72 per the ledger)."));
}

#[test]
fn ibm_runner_loci_and_disclosure_runs() {
    let doc = text("IBM");
    let m = model("IBM");
    // ma_recurrence locus: the g_software mean moves with f
    for f in [0.0, 0.25, 0.50, 0.75, 1.00] {
        let g = 0.040 + f * 0.020;
        let o = Overrides { means: vec![("ma_recurrence".into(), f), ("g_software".into(), g)], ..Overrides::default() };
        has(&doc, &ibm_row(&format!("f={f:.2} (g_sw mean {g:.3})"), &quick(&m, &o, N, SEED).unwrap()));
    }
    // SBC locus
    let flat = Overrides { flat: vec![("sbc_ratio".into(), 0.0254)], ..Overrides::default() };
    has(&doc, &ibm_row("flat 2.54%", &quick(&m, &flat, N, SEED).unwrap()));
    has(&doc, &ibm_row("ramp 2.54% -> 3.5% (base)", &quick(&m, &Overrides::default(), N, SEED).unwrap()));
    let ramp = Overrides { ramps: vec![("sbc_ratio".into(), Ramp { start: 0.0254, end: 0.045, ramp_years: 10 })], ..Overrides::default() };
    has(&doc, &ibm_row("ramp 2.54% -> 4.5%", &quick(&m, &ramp, N, SEED).unwrap()));
    // beta locus
    for (b, label) in [(0.68, "0.68 (trailing post-crash)"), (0.85, "0.85 (base)"), (1.00, "1.00")] {
        has(&doc, &ibm_row(&format!("beta {label}"), &quick(&m, &mean("beta", b), N, SEED).unwrap()));
    }
    // disclosure runs
    let supply = Overrides {
        corr: vec![
            ("rf".into(), "g_software".into(), -0.15),
            ("rf".into(), "g_consulting".into(), -0.15),
            ("rf".into(), "g_infrastructure".into(), -0.15),
        ],
        ..Overrides::default()
    };
    has(&doc, &ibm_row("supply-shock corr (rf x g = -0.15)", &quick(&m, &supply, N, SEED).unwrap()));
    has(&doc, &ibm_row("tau = 0 (no debt tax shield)", &quick(&m, &mean("tax_rate", 0.0), N, SEED).unwrap()));
}

#[test]
fn ibm_runner_implied_reinvestment_check() {
    let doc = text("IBM");
    let m = model("IBM");
    let d = |n: &str| m.driver(n).unwrap();
    let f = 0.50;
    let (core, ma, tg) = (d("core_margin").mean, d("ma_ratio").mean, d("terminal_growth").mean);
    let drift_t = *d("margin_mix_drift").schedule.as_ref().unwrap().last().unwrap();
    let sbc_t = *d("sbc_ratio").schedule.as_ref().unwrap().last().unwrap();
    let reinv = f * ma;
    let fcff_t = core + drift_t - sbc_t - reinv;
    let nopat = fcff_t + reinv;
    let rr = reinv / nopat;
    let roic = tg / rr;
    has(
        &doc,
        &format!(
            "  Terminal reinvestment = f x ma_ratio = {f:.2} x {ma:.3} = {:.2}% of revenue; terminal FCFF margin {:.2}%; RR = {:.1}% of pre-reinvestment cash earnings ({:.2}%).",
            reinv * 100.0,
            fcff_t * 100.0,
            rr * 100.0,
            nopat * 100.0
        ),
    );
    has(
        &doc,
        &format!(
            "  One line: sustaining tg = {:.2}% on that reinvestment implies ROIC = tg/RR = {:.1}% -- a plausible (not heroic) incremental return, so the terminal no longer carries cost-without-growth.",
            tg * 100.0,
            roic * 100.0
        ),
    );
}

// ---------------------------------------------------------------------------
// JNJ: analysis/JNJ/first_pass_run.py
// ---------------------------------------------------------------------------

const JNJ_PRICE: f64 = 255.56;
const IM_ORGANIC_G: f64 = 0.04;
const MEDTECH_ORGANIC_G: f64 = 0.035;
const ACQUIRED_G: f64 = 0.02;

fn jnj_f(f: f64, acquired: f64) -> Overrides {
    Overrides {
        means: vec![
            ("ma_recurrence".into(), f),
            ("g_im".into(), IM_ORGANIC_G + f * acquired),
            ("g_medtech".into(), MEDTECH_ORGANIC_G + f * acquired),
        ],
        ..Overrides::default()
    }
}

#[test]
fn jnj_runner_all_numbers() {
    let doc = text("JNJ");
    let m = model("JNJ");
    let r = full(&m, &Overrides::default(), N, SEED).unwrap();
    let q = percentiles(&r.per_share_paths, &[10.0, 25.0, 50.0, 75.0, 90.0]);
    has(&doc, &format!("JNJ first pass - seed {SEED}, 10,000 sims, price ${JNJ_PRICE}"));
    has(&doc, &format!("  P10 {:.2} | P25 {:.2} | P50 {:.2} | P75 {:.2} | P90 {:.2}", q[0], q[1], q[2], q[3], q[4]));
    has(
        &doc,
        &format!(
            "  chance the price is too high {:.1}%; TV/EV at means {:.1}%; tg pinned {:.2}%",
            share_below(&r.per_share_paths, JNJ_PRICE) * 100.0,
            r.tv_ev_share_at_means * 100.0,
            r.tg_pinned_fraction * 100.0
        ),
    );
    let (hand, _, _) = hand_dcf_at_means(&m).unwrap();
    has(&doc, &format!("  Cross-check at means: engine ${:.4} vs hand DCF ${:.4}", r.value_at_means, hand));
    let p50 = |o: &Overrides| quick(&m, o, N, SEED).unwrap().p50;
    for f in [0.0, 0.25, 0.5, 0.75, 1.0] {
        has(&doc, &format!("    f={f:.2}: P50 {:7.2}", p50(&jnj_f(f, ACQUIRED_G))));
    }
    for ag in [0.02, 0.04, 0.06] {
        let o = Overrides {
            means: vec![("g_im".into(), IM_ORGANIC_G + 0.5 * ag), ("g_medtech".into(), MEDTECH_ORGANIC_G + 0.5 * ag)],
            ..Overrides::default()
        };
        has(&doc, &format!("    {:.0}%: P50 {:7.2}", ag * 100.0, p50(&o)));
    }
    for beta in [0.45, 0.65, 0.85] {
        has(&doc, &format!("    beta {beta:.2}: P50 {:7.2}", p50(&mean("beta", beta))));
    }
    has(&doc, &format!("  rf 4.6% (July-2026 level): P50 {:7.2}", p50(&mean("rf", 0.046))));
    let mut stacked = jnj_f(0.0, ACQUIRED_G);
    stacked.means.push(("beta".into(), 0.45));
    stacked.means.push(("rf".into(), 0.046));
    let s = full(&m, &stacked, N, SEED).unwrap();
    has(
        &doc,
        &format!(
            "  Stacked (f=0, beta 0.45, rf 4.6%): P50 {:.2}, chance too high {:.1}%",
            s.per_share_p50,
            share_below(&s.per_share_paths, JNJ_PRICE) * 100.0
        ),
    );
}

// ---------------------------------------------------------------------------
// ANT: analysis/ANT/first_pass_run.py
// ---------------------------------------------------------------------------

const ANT_BASE_REVENUE: f64 = 35_282e6;
const ANT_YEAR1: f64 = 105e9;
const SBC_MATURE: f64 = 0.06;
const B: f64 = 1e9;

/// Overrides of the ANT runner for `core_mean`: the margin level, its bounds and the drift ramp.
fn ant_core(cm: f64, ramp_years: usize) -> Overrides {
    Overrides {
        means: vec![("core_margin".into(), cm)],
        shift_bounds: true,
        ramps: vec![("margin_mix_drift".into(), Ramp { start: -0.07 - cm, end: 0.0, ramp_years })],
        ..Overrides::default()
    }
}

/// `_run(mean=...)` of the ANT runner: means move together with the bounds.
fn ant_mean(name: &str, v: f64) -> Overrides {
    Overrides { means: vec![(name.into(), v)], shift_bounds: true, ..Overrides::default() }
}

fn merge(mut a: Overrides, b: Overrides) -> Overrides {
    a.means.extend(b.means);
    a.shift_bounds |= b.shift_bounds;
    a.flat.extend(b.flat);
    a.ramps.retain(|(n, _)| !b.ramps.iter().any(|(m, _)| m == n));
    a.ramps.extend(b.ramps);
    a.corr.extend(b.corr);
    a.net_debt = b.net_debt.or(a.net_debt);
    a.shares = b.shares.or(a.shares);
    a.years = b.years.or(a.years);
    a.year1_revenue.extend(b.year1_revenue);
    a
}

fn year10_revenue(g: f64) -> f64 {
    ANT_YEAR1 * (1.0 + g).powi(9)
}

#[test]
fn ant_runner_headline_check_and_attribution() {
    let doc = text("ANT");
    let m = model("ANT");
    let r = full(&m, &Overrides::default(), N, SEED).unwrap();
    let q = percentiles(&r.per_share_paths, &[10.0, 25.0, 50.0, 75.0, 90.0]);
    has(&doc, &format!("  P10 {:.2} | P25 {:.2} | P50 {:.2} | P75 {:.2} | P90 {:.2}", q[0], q[1], q[2], q[3], q[4]));
    has(
        &doc,
        &format!(
            "  paths with value below $83.00: {:.1}%; below $105.00: {:.1}%",
            share_below(&r.per_share_paths, 83.0) * 100.0,
            share_below(&r.per_share_paths, 105.0) * 100.0
        ),
    );
    let d = |n: &str| m.driver(n).unwrap().mean;
    let wacc = d("equity_weight") * (d("rf") + d("beta") * d("erp")) + (1.0 - d("equity_weight")) * (d("rf") + d("kd_spread")) * (1.0 - d("tax_rate"));
    has(
        &doc,
        &format!(
            "  WACC at means {:.2}% (rf {:.2}% + beta {:.2} x ERP {:.2}%, equity weight {:.2}); terminal growth pinned on {:.2}% of paths",
            wacc * 100.0,
            d("rf") * 100.0,
            d("beta"),
            d("erp") * 100.0,
            d("equity_weight"),
            r.tg_pinned_fraction * 100.0
        ),
    );
    has(&doc, &format!("  terminal value share of EV at means {:.1}%", r.tv_ev_share_at_means * 100.0));
    // year by year at the means
    let eng = m.engine().unwrap();
    let at = eng.valuation(&eng.mean_draws(), 1).unwrap();
    let y1 = m.segments[0].year1_growth.unwrap();
    let mut level = m.segments[0].base_revenue * (1.0 + y1);
    let sched = |n: &str, t: usize| {
        let dr = m.driver(n).unwrap();
        let _ = dr;
        eng.model.driver(n).unwrap().schedule.as_ref().map_or(eng.model.driver(n).unwrap().mean, |s| s[t - 1])
    };
    for t in 1..=10usize {
        if t > 1 {
            level *= 1.0 + d("g_total");
        }
        let margin = sched("core_margin", t) + sched("margin_mix_drift", t) - sched("sbc_ratio", t) - sched("ma_recurrence", t) * sched("ma_ratio", t);
        if [1, 3, 5, 10].contains(&t) {
            has(
                &doc,
                &format!(
                    "  year {t:>2} at means: revenue ${:7.1}B, FCFF margin after SBC {}, FCFF ${:7.1}B",
                    level / B,
                    pct_signed(margin, 6),
                    level * margin / B
                ),
            );
        }
        // the engine gives the same FCFF at the means
        assert!((at.fcff[t - 1] - level * margin).abs() <= 1e-9 * (level * margin).abs().max(1.0));
    }
    let g = &r.driver_draws.iter().find(|(k, _)| k == "g_total").unwrap().1;
    let gp = percentiles(g, &[10.0, 50.0, 90.0]);
    has(&doc, &format!("  year-10 revenue: P10 ${:.0}B | P50 ${:.0}B | P90 ${:.0}B", year10_revenue(gp[0]) / B, year10_revenue(gp[1]) / B, year10_revenue(gp[2]) / B));
    let equity_p50 = q[2] * m.shares;
    let ev_p50 = equity_p50 + m.net_debt;
    has(
        &doc,
        &format!(
            "  equity value at P50 ${:.0}B on {}M shares; net cash ${:.1}B (${:.2} a share); EV at P50 ${:.0}B = {:.1} x trailing revenue",
            equity_p50 / B,
            commas(m.shares / 1e6, 1),
            -m.net_debt / B,
            -m.net_debt / m.shares,
            ev_p50 / B,
            ev_p50 / ANT_BASE_REVENUE
        ),
    );
    // header lines and the market values
    let seg = &m.segments[0];
    let year1 = seg.base_revenue * (1.0 + seg.year1_growth.unwrap());
    has(
        &doc,
        &format!(
            "  base revenue ${:.3}B (12 months to 2026-09-30); year 1 pinned at ${:.1}B ({:+.0}%)",
            seg.base_revenue / B,
            year1 / B,
            (year1 / seg.base_revenue - 1.0) * 100.0
        ),
    );
    for price in [83.0, 105.0] {
        has(
            &doc,
            &format!(
                "  market value at ${price:.2}: ${}B; EV {:.0} x trailing revenue",
                group(price * m.shares / B),
                (price * m.shares + m.net_debt) / seg.base_revenue
            ),
        );
    }
    // engine check against the hand DCF
    let (hand, ev, tv) = hand_dcf_at_means(&m).unwrap();
    has(
        &doc,
        &format!("  engine ${:.4} | hand DCF ${:.4} | EV ${:.1}B, of which terminal value ${:.1}B", r.value_at_means, hand, ev / B, tv / B),
    );
    // attribution ranked by the width of the spread
    let mut ranked = r.attribution.clone();
    ranked.sort_by(|a, b| (b.p50_plus_1sd - b.p50_minus_1sd).abs().partial_cmp(&(a.p50_plus_1sd - a.p50_minus_1sd).abs()).unwrap());
    for a in &ranked {
        let sd = m.driver(&a.driver).unwrap().std;
        has(&doc, &format!("  {:16} {:+7.2} / {:+7.2} (std {:.2}%)", a.driver, a.p50_minus_1sd, a.p50_plus_1sd, sd * 100.0));
    }
}

#[test]
fn ant_runner_sensitivities() {
    let doc = text("ANT");
    let m = model("ANT");
    let p50 = |o: &Overrides| quick(&m, o, N, SEED).unwrap().p50;
    for g in [0.10, 0.15, 0.25, 0.30] {
        has(&doc, &format!("  growth mean {:.0}% (year-10 revenue ${}B): {:.2}", g * 100.0, group(year10_revenue(g) / B), p50(&ant_mean("g_total", g))));
    }
    for cm in [0.16, 0.28, 0.34] {
        has(&doc, &format!("  mature margin after SBC {:.0}% (year 1 held): {:.2}", (cm - SBC_MATURE) * 100.0, p50(&ant_core(cm, 8))));
    }
    for (y1, why) in [(80.6e9, "run rate stays at $81B"), (125e9, "run rate adds as in 2026")] {
        let o = Overrides { year1_revenue: vec![("Total".into(), y1)], ..Overrides::default() };
        has(&doc, &format!("  year-1 revenue ${:.1}B ({why}): {:.2}", y1 / B, p50(&o)));
    }
    for ramp in [6usize, 10] {
        let o = Overrides {
            ramps: vec![
                ("margin_mix_drift".into(), Ramp { start: -0.07 - 0.22, end: 0.0, ramp_years: ramp }),
                ("sbc_ratio".into(), Ramp { start: 0.25, end: 0.06, ramp_years: ramp }),
            ],
            ..Overrides::default()
        };
        has(&doc, &format!("  mature margin and SBC reached in year {ramp}: {:.2}", p50(&o)));
    }
    let o = Overrides { ramps: vec![("sbc_ratio".into(), Ramp { start: 0.15, end: 0.06, ramp_years: 8 })], ..Overrides::default() };
    has(&doc, &format!("  SBC year 1 at 15% (expense basis): {:.2}", p50(&o)));
    let (rf0, erp0) = (m.driver("rf").unwrap().mean, m.driver("erp").unwrap().mean);
    for beta in [1.10, 1.70] {
        has(&doc, &format!("  beta {beta:.2} (cost of equity {:.2}%): {:.2}", (rf0 + beta * erp0) * 100.0, p50(&ant_mean("beta", beta))));
    }
    for rf in [0.0407, 0.0607] {
        has(&doc, &format!("  rf {:.2}%: {:.2}", rf * 100.0, p50(&ant_mean("rf", rf))));
    }
    let o15 = Overrides { years: Some(15), ..Overrides::default() };
    has(
        &doc,
        &format!("  horizon 15 years, growth mean 20% (year-15 revenue ${}B, no precedent): {:.2}", group(ANT_YEAR1 * 1.2f64.powi(14) / B), p50(&o15)),
    );
    let y15 = year10_revenue(0.20) * 1.17 * 1.14 * 1.11 * 1.08 * 1.05;
    let g15 = (y15 / ANT_YEAR1).powf(1.0 / 14.0) - 1.0;
    has(
        &doc,
        &format!("  horizon 15 years, growth mean {:.1}% (year-15 revenue ${}B): {:.2}", g15 * 100.0, group(y15 / B), p50(&merge(ant_mean("g_total", g15), o15.clone()))),
    );
    let no_fund = m.shares - 846_840_960.0;
    let o = Overrides { shares: Some(no_fund), ..Overrides::default() };
    has(&doc, &format!("  shares without the 846.8M that the subsidiary holds ({}M): {:.2}", commas(no_fund / 1e6, 1), p50(&o)));
    let outstanding = 7_583_103_996.0 + 4_789_697_318.0;
    let o = Overrides { shares: Some(outstanding), net_debt: Some(m.net_debt + 109_742e6), ..Overrides::default() };
    has(
        &doc,
        &format!(
            "  shares outstanding only ({}M), notes as a liability of $109.7B; no options, no RSUs, thus not a correct count: {:.2}",
            commas(outstanding / 1e6, 1),
            p50(&o)
        ),
    );
    let (d, wacc) = {
        let g = |n: &str| m.driver(n).unwrap().mean;
        (g("rf") + g("kd_spread"), g("equity_weight") * (g("rf") + g("beta") * g("erp")) + (1.0 - g("equity_weight")) * (g("rf") + g("kd_spread")) * (1.0 - g("tax_rate")))
    };
    let kd = d;
    let pay = 451.2e9 / 10.0;
    let extra: f64 = (2..=11).map(|t| pay / (1.0 + kd).powi(t) - pay / (1.0 + wacc).powi(t)).sum();
    let o = Overrides { net_debt: Some(m.net_debt + extra), ..Overrides::default() };
    has(
        &doc,
        &format!(
            "  leases not started ($451.2B) as debt at {:.2}% and out of the margin (net debt +${:.1}B): {:.2}",
            kd * 100.0,
            extra / B,
            p50(&o)
        ),
    );
    let o = Overrides { net_debt: Some(m.net_debt + 472.3e9), ..Overrides::default() };
    has(&doc, &format!("  Damodaran debt $472.3B added, margin not changed (counts the cost two times): {:.2}", p50(&o)));
}

/// 1,833 style: thousands separator, no decimals (Python `{:,.0f}`).
fn group(x: f64) -> String {
    commas(x, 0)
}

/// Python `{:,.Nf}`.
fn commas(x: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, x);
    let (s, frac) = match s.split_once('.') {
        Some((a, b)) => (a.to_string(), format!(".{b}")),
        None => (s, String::new()),
    };
    let s = s.as_str();
    let (sign, digits) = s.strip_prefix('-').map_or(("", s), |d| ("-", d));
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{sign}{out}{frac}")
}

/// Python `{x:+6.1%}`: sign, one decimal, percent sign, width 6 including the sign and the percent sign.
fn pct_signed(x: f64, width: usize) -> String {
    format!("{:>width$}", format!("{:+.1}%", x * 100.0))
}

#[test]
fn ant_runner_price_implied_values() {
    let doc = text("ANT");
    let m = model("ANT");
    let (rf0, erp0) = (m.driver("rf").unwrap().mean, m.driver("erp").unwrap().mean);
    let shares = m.shares;
    for (price, label) in [(83.0, "IPO price"), (105.0, "opening indication (press), no trade yet")] {
        has(&doc, &format!("  ${price:.2} ({label}); equity ${}B", group(price * shares / B)));
        // (a) growth from year 2
        let g = implied_mean(&m, &ant_mean("g_total", 0.0), "g_total", 0.0, 0.80, price, N, SEED).unwrap().unwrap();
        has(&doc, &format!("    (a) growth from year 2: {:.1}% a year for 9 years; year-10 revenue ${}B", g * 100.0, group(year10_revenue(g) / B)));
        // (b) mature margin before SBC: core mean, bounds and drift move together
        let cm = crate::dcf::model::bisect(|x| Ok(quick(&m, &ant_core(x, 8), N, SEED).unwrap().p50), 0.10, 1.00, price).unwrap().unwrap();
        has(&doc, &format!("    (b) mature FCFF margin after SBC: {:.1}% ({:.1}% before SBC)", (cm - SBC_MATURE) * 100.0, cm * 100.0));
        // (c) beta (the P50 falls when beta rises)
        let beta = implied_mean(&m, &Overrides { shift_bounds: true, ..Overrides::default() }, "beta", 0.0, 3.0, price, N, SEED).unwrap().unwrap();
        has(&doc, &format!("    (c) beta: {beta:.2}; cost of equity {:.2}%", (rf0 + beta * erp0) * 100.0));
        // joint: beta 1.10 and mature margin before SBC 25%: growth that gives the price
        let joint = |x: f64| {
            let o = merge(ant_mean("beta", 1.10), merge(ant_mean("g_total", x), ant_core(0.25, 8)));
            Ok(quick(&m, &o, N, SEED).unwrap().p50)
        };
        let gj = crate::dcf::model::bisect(joint, 0.0, 0.80, price).unwrap().unwrap();
        has(
            &doc,
            &format!(
                "    joint: growth {:.1}% (year-10 revenue ${}B) with mature margin 19% after SBC and beta 1.10 (cost of equity {:.2}%)",
                gj * 100.0,
                group(year10_revenue(gj) / B),
                (rf0 + 1.10 * erp0) * 100.0
            ),
        );
    }
    let g_dam = (1_011e9 / ANT_YEAR1).powf(1.0 / 9.0) - 1.0;
    let o = merge(ant_mean("g_total", g_dam), ant_core(0.25, 8));
    has(
        &doc,
        &format!(
            "  Damodaran year-10 revenue $1,011B (growth {:.1}%) and margin 19% after SBC, repo discount rate: P50 {:.2}",
            g_dam * 100.0,
            quick(&m, &o, N, SEED).unwrap().p50
        ),
    );
    has(&doc, &format!("  Damodaran equity value $1,475B on this share count: ${:.2} a share", 1_475e9 / shares));
    // keep the helper in use for a plain percentile check of the base run
    let base = quick(&m, &Overrides::default(), N, SEED).unwrap();
    let r = full(&m, &Overrides::default(), N, SEED).unwrap();
    assert_eq!(base.p50, percentile(&sorted_copy(&r.per_share_paths), 50.0));
    let _ = p50_with_mean;
}

/// Revenue and margin after SBC per year at the ANT means; `growth_by_year` replaces the growth
/// of years 2..N (hand calculation for the fade).
fn ant_path_at_means(m: &ModelSpec, growth_by_year: Option<&[f64]>) -> (Vec<f64>, Vec<f64>) {
    let eng = m.engine().unwrap();
    let dv = |n: &str, t: usize| {
        let d = eng.model.driver(n).unwrap();
        d.schedule.as_ref().map_or(d.mean, |s| s[t - 1])
    };
    let seg = &m.segments[0];
    let mut level = seg.base_revenue * (1.0 + seg.year1_growth.unwrap());
    let (mut rev, mut margin) = (vec![], vec![]);
    for t in 1..=m.projection_years {
        if t > 1 {
            level *= 1.0 + growth_by_year.map_or(m.driver("g_total").unwrap().mean, |g| g[t - 2]);
        }
        rev.push(level);
        margin.push(dv("core_margin", t) + dv("margin_mix_drift", t) - dv("sbc_ratio", t) - dv("ma_recurrence", t) * dv("ma_ratio", t));
    }
    (rev, margin)
}

fn ant_hand_dcf(m: &ModelSpec, growth_by_year: Option<&[f64]>) -> f64 {
    let d = |n: &str| m.driver(n).unwrap().mean;
    let wacc = d("equity_weight") * (d("rf") + d("beta") * d("erp")) + (1.0 - d("equity_weight")) * (d("rf") + d("kd_spread")) * (1.0 - d("tax_rate"));
    let (rev, margin) = ant_path_at_means(m, growth_by_year);
    let pv: f64 = rev.iter().zip(&margin).enumerate().map(|(i, (r, mg))| r * mg / (1.0 + wacc).powi(i as i32 + 1)).sum();
    let tg = d("terminal_growth").min(wacc - 0.02);
    let n = rev.len();
    let tv = rev[n - 1] * margin[n - 1] * (1.0 + tg) / (wacc - tg) / (1.0 + wacc).powi(n as i32);
    (pv + tv - m.net_debt) / m.shares
}

#[test]
fn ant_runner_growth_limit_and_commitment_check() {
    let doc = text("ANT");
    let m = model("ANT");
    let hand = ant_hand_dcf(&m, None);
    let (rev, margin) = ant_path_at_means(&m, None);
    // fade: growth in year 2 goes down linearly to 2.25% in year 10, same year-10 revenue
    let n = m.projection_years - 1;
    let end = 0.0225;
    let g_mean = m.driver("g_total").unwrap().mean;
    let target = (1.0 + g_mean).powi(n as i32);
    let path = |g2: f64| (0..n).map(|i| g2 + (end - g2) * i as f64 / (n - 1) as f64).collect::<Vec<f64>>();
    let g2 = crate::dcf::model::bisect(|x| Ok(path(x).iter().map(|g| 1.0 + g).product::<f64>()), end, 2.0, target).unwrap().unwrap();
    let fade = path(g2);
    let fade_value = ant_hand_dcf(&m, Some(&fade));
    let (fade_rev, _) = ant_path_at_means(&m, Some(&fade));
    has(&doc, &format!("  constant {:.0}% from year 2: ${:.2}; year-5 revenue ${:.0}B", g_mean * 100.0, hand, rev[4] / B));
    has(
        &doc,
        &format!(
            "  fade from {:.1}% in year 2 to {:.2}% in year 10: ${:.2}; year-5 revenue ${:.0}B; difference ${:+.2}",
            fade[0] * 100.0,
            fade[n - 1] * 100.0,
            fade_value,
            fade_rev[4] / B,
            fade_value - hand
        ),
    );
    // commitment check
    let r = full(&m, &Overrides::default(), N, SEED).unwrap();
    let cum = rev[..4].iter().sum::<f64>() + 0.25 * rev[4];
    let g = &r.driver_draws.iter().find(|(k, _)| k == "g_total").unwrap().1;
    let g10 = percentile(&sorted_copy(g), 10.0);
    let low: Vec<f64> = (0..5).map(|t| ANT_YEAR1 * (1.0 + g10).powi(t)).collect();
    let cum_low = low[..4].iter().sum::<f64>() + 0.25 * low[4];
    has(
        &doc,
        &format!("  next 12 months: committed $70.7B = {:.0}% of year-1 revenue (compute and infrastructure was 57% of 9M2026 revenue)", 70.7e9 / rev[0] * 100.0),
    );
    has(
        &doc,
        &format!(
            "  to 2030-12-31: committed $451.6B = {:.0}% of revenue at mean growth (${:.0}B), {:.0}% at P10 growth {:.1}% (${:.0}B)",
            451.6e9 / cum * 100.0,
            cum / B,
            451.6e9 / cum_low * 100.0,
            g10 * 100.0,
            cum_low / B
        ),
    );
    let spend = [70.7e9, 380.9e9 / 3.25, 380.9e9 / 3.25, 380.9e9 / 3.25];
    let eng = m.engine().unwrap();
    let sbc = eng.model.driver("sbc_ratio").unwrap().schedule.clone().unwrap();
    let pre_sbc: Vec<f64> = margin.iter().zip(&sbc).map(|(a, b)| a + b).collect();
    let implied: Vec<f64> = (0..4).map(|i| 1.0 - spend[i] / rev[i] - 0.31).collect();
    let fmt = |v: &[f64]| v.iter().map(|x| format!("{:+.0}%", x * 100.0)).collect::<Vec<_>>().join(" / ");
    has(&doc, &format!("  FCFF margin before SBC, years 1 to 4: model {}; from the committed spend {}", fmt(&pre_sbc[..4]), fmt(&implied)));
    let d = |n: &str| m.driver(n).unwrap().mean;
    let wacc = d("equity_weight") * (d("rf") + d("beta") * d("erp")) + (1.0 - d("equity_weight")) * (d("rf") + d("kd_spread")) * (1.0 - d("tax_rate"));
    let gap: f64 = (0..4).map(|i| (implied[i] - pre_sbc[i]).min(0.0) * rev[i] / (1.0 + wacc).powi(i as i32 + 1)).sum();
    has(
        &doc,
        &format!(
            "  where the committed spend gives a lower margin (years 2 to 4), the effect at means is ${:+.2} a share; this is NOT in the base",
            gap / m.shares
        ),
    );
    has(&doc, &format!("  base rate: AMZN 27.4% a year for 10 years from this base = ${:.0}B in year 10", ANT_BASE_REVENUE * 1.274f64.powi(10) / B));
}
