//! Tests of `fa dcf implied` on small data made by hand (fixed answers). No network and no cache.
//!
//! The price enters the builder through the equity weight (`equity / (equity + debt)`), so a price that is the value of the
//! model is a fixed point only when the weight does not depend on the price. The tests that use a value as the price set
//! `equity_weight_one`.

use super::super::auto::tests::{inputs, market, plain_rules, synthetic, table};
use super::super::paths::PathEngine;
use super::super::process::{ParamSpec, PathsSpec};
use super::*;

fn rules_w1() -> Rules {
    Rules { equity_weight_one: true, ..plain_rules() }
}

fn priced(p: Option<f64>) -> impl Fn(&str) -> Market + Sync {
    move |_| Market { price: p, price_source: "test".into(), ..market() }
}

fn run(inp: &[AutoInputs], rules: Rules, market_of: &(dyn Fn(&str) -> Market + Sync), jobs: Option<usize>) -> (Vec<ImpliedRow>, Option<MarketPremium>) {
    let t = table();
    let cfg = ImpliedConfig { table: &t, fade: &[], rules };
    let (rows, mp, _) = implied_models(inp, market_of, &HashMap::new(), &cfg, jobs).unwrap();
    (rows, mp)
}

fn value_of(r: &ImpliedRow) -> &ImpliedValue {
    r.outcome.as_ref().unwrap_or_else(|x| panic!("{x:?}"))
}

/// The value of the model at the means with no price.
fn value_at_means(rules: &Rules) -> f64 {
    let (rows, _) = run(&[inputs()], rules.clone(), &priced(None), Some(1));
    value_of(&rows[0]).value
}

#[test]
fn a_price_equal_to_the_value_gives_the_model_numbers_and_zero_shifts() {
    let rules = rules_w1();
    let v = value_at_means(&rules);
    assert!(v.is_finite() && v > 1.0, "{v}");
    let (rows, mp) = run(&[inputs()], rules.clone(), &priced(Some(v)), Some(1));
    let r = value_of(&rows[0]);
    let (p, g, m) = (r.premium.unwrap(), r.growth.unwrap(), r.margin.unwrap());
    assert!(p.bound == Bound::Exact && g.bound == Bound::Exact && m.bound == Bound::Exact && p.monotone && g.monotone && m.monotone, "{r:?}");
    assert!((p.x - rules.erp.0).abs() < 1e-9, "premium {} model {}", p.x, rules.erp.0);
    assert!(g.x.abs() < 1e-9 && m.x.abs() < 1e-9, "shifts {} {}", g.x, m.x);
    assert!((r.implied_rate - r.model_rate).abs() < 1e-9);
    assert!((r.implied_cagr - r.model_cagr).abs() < 1e-9);
    assert!((r.implied_margin - r.model_margin).abs() < 1e-9);
    assert!((r.value_over_price().unwrap() - 1.0).abs() < 1e-12);
    assert_eq!(r.flags(), "");
    // a list of one ticker: the premium of the list is the premium of the ticker
    let mp = mp.unwrap();
    assert_eq!(mp.n, 1);
    assert!((mp.found.x - p.x).abs() < 1e-9 && mp.found.bound == Bound::Exact);
    assert!((r.at_market.unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn a_price_that_is_the_value_at_a_premium_of_three_percent_gives_three_percent() {
    let v3 = value_at_means(&Rules { erp: (0.03, 0.0075), ..rules_w1() });
    let v_model = value_at_means(&rules_w1());
    assert!(v3 > v_model, "a lower premium gives a higher value");
    let (rows, _) = run(&[inputs()], rules_w1(), &priced(Some(v3)), Some(1));
    let r = value_of(&rows[0]);
    let p = r.premium.unwrap();
    assert!(p.bound == Bound::Exact && (p.x - 0.03).abs() < 1e-9, "{p:?}");
    // the cost of capital of year 1 at that premium: the equity weight is 1, so rf + beta premium (beta 1, rf 4.2%)
    assert!((r.implied_rate - (DEFAULT_RF + 0.03)).abs() < 1e-12, "{}", r.implied_rate);
    assert!((r.model_rate - (DEFAULT_RF + 0.0441)).abs() < 1e-12);
    // the price is above the value of the model: the growth and the margin that the price implies are higher
    assert!(r.growth.unwrap().x > 0.0 && r.margin.unwrap().x > 0.0);
    assert!(r.growth_diff().unwrap() > 0.0 && r.margin_diff().unwrap() > 0.0 && r.rate_diff().unwrap() < 0.0);
}

#[test]
fn a_price_that_the_range_does_not_reach_gives_the_bound_and_the_flag() {
    let (rows, _) = run(&[inputs()], rules_w1(), &priced(Some(1.0e6)), Some(1));
    let r = value_of(&rows[0]);
    let p = r.premium.unwrap();
    assert_eq!(p.bound, Bound::Below, "{p:?}");
    assert!(p.x >= ERP_RANGE.0 && p.x < 0.0441);
    assert_eq!(r.growth.unwrap(), Found { x: GROWTH_RANGE.1, bound: Bound::Above, monotone: true });
    assert_eq!(r.margin.unwrap(), Found { x: MARGIN_RANGE.1, bound: Bound::Above, monotone: true });
    assert_eq!(r.flags(), "premium< growth> margin>");
    assert!(csv_line(&rows[0]).contains(",premium< growth> margin>,"));
    // a price below any value of the range
    let (rows, _) = run(&[inputs()], rules_w1(), &priced(Some(1.0e-9)), Some(1));
    let r = value_of(&rows[0]);
    assert_eq!(r.premium.unwrap(), Found { x: ERP_RANGE.1, bound: Bound::Above, monotone: true });
    assert!(r.flags().starts_with("premium>"));
}

const HAND: &str = r#"
engine = "paths"
ticker = "HAND"
net_debt = 100.0
shares = 10.0
projection_years = 3

[[segments]]
name = "A"
base_revenue = 1000.0
process = "g"

[process.g]
role = "growth"
start = 0.03
target = 0.03
half_life = 1.0

[process.m]
role = "margin"
start = 0.10
target = 0.10
half_life = 1.0

[process.s]
role = "sbc"
start = 0.02
target = 0.02
shape = "ramp"
ramp_years = 2

[discount]
beta = 1.0
erp = 0.05
kd_spread = 0.02
equity_weight = 1.0

[tax]
rate = 0.25

[rates]
model = "fixed"
rf = 0.04

[terminal]
growth = 0.02
"#;

/// The closed formula of `HAND`: revenue of year t is 1000 * 1.03^t; the free cash flow is revenue (0.10 - 0.02); the cost of
/// capital is rf + beta erp (equity weight 1); the terminal growth is 2% (below the growth process 3% and below k - 2%).
fn hand_value(erp: f64) -> f64 {
    let k = 0.04 + erp;
    let fcf = |t: i32| 1000.0 * 1.03f64.powi(t) * 0.08;
    let pv: f64 = (1..=3).map(|t| fcf(t) / (1.0 + k).powi(t)).sum();
    let tv = fcf(3) * 1.02 / (k - 0.02) / (1.0 + k).powi(3);
    (pv + tv - 100.0) / 10.0
}

fn with_erp(spec: &PathsSpec, x: f64) -> PathsSpec {
    let mut s = spec.clone();
    s.discount.erp = ParamSpec::Fixed(x);
    s
}

#[test]
fn the_value_at_the_means_is_the_hand_calculated_value_and_the_search_finds_the_premium() {
    let spec = PathsSpec::from_toml(HAND).unwrap();
    // premium 5%: k = 9%. Year 1: 82.4, 84.872, 87.41816; PV = 75.5963 + 71.4348 + 67.5037 (to the cent); terminal
    // 87.41816 * 1.02 / 0.07 / 1.09^3 = 983.6... ; the formula in `hand_value` has the same terms.
    let (v, rev, fcff) = deterministic_value(&spec).unwrap();
    assert!((v - hand_value(0.05)).abs() < 1e-9 * hand_value(0.05).abs(), "{v} {}", hand_value(0.05));
    assert!((rev - 1000.0 * 1.03f64.powi(3)).abs() < 1e-9);
    assert!((fcff / rev - 0.08).abs() < 1e-12);
    // the engine gives the same value twice (no random part)
    assert_eq!(deterministic_value(&spec).unwrap().0, v);
    // the price that the formula gives at 7% gives 7%
    let price = hand_value(0.07);
    let found = solve(|x| Ok(deterministic_value(&with_erp(&spec, x)).map_or(f64::MAX, |v| v.0)), ERP_RANGE.0, ERP_RANGE.1, price, false, Some((0.05, v))).unwrap();
    assert_eq!(found.bound, Bound::Exact);
    assert!(found.monotone && (found.x - 0.07).abs() < 1e-9, "{found:?}");
    // the engine and the formula agree on a second premium
    let (v2, _, _) = deterministic_value(&with_erp(&spec, 0.10)).unwrap();
    assert!((v2 - hand_value(0.10)).abs() < 1e-9 * hand_value(0.10).abs());
    // the engine reaches the model without a price of the engine
    assert!(PathEngine::new(&spec).is_ok());
}

#[test]
fn a_function_with_the_wrong_direction_is_flagged_and_gives_no_number() {
    let f = solve(|x| Ok(x * x), -1.0, 2.0, 1.0, false, None).unwrap();
    assert!(!f.monotone && f.x.is_nan());
    // a value that is not monotone inside the range (the ends are in order, the base is not)
    let g = solve(|x| Ok(if x.abs() < 0.5 { 10.0 } else { x }), -1.0, 1.0, 0.3, true, Some((0.0, 10.0))).unwrap();
    assert!(!g.monotone);
    // a jump: the target is in a part that the function does not reach
    let h = solve(|x| Ok(if x < 0.5 { 1.0e12 } else { 100.0 - 100.0 * x }), 0.0, 1.0, 1.0e6, false, None).unwrap();
    assert_eq!(h.bound, Bound::Below);
}

#[allow(clippy::too_many_arguments)]
fn row(ticker: &str, size: &str, sector: &str, industry: &str, price: f64, value: f64, d_rate: f64, d_growth: f64, d_margin: f64, at_market: f64) -> ImpliedRow {
    let found = |x: f64| Some(Found { x, bound: Bound::Exact, monotone: true });
    ImpliedRow {
        ticker: ticker.into(),
        sector: sector.into(),
        industry: industry.into(),
        outcome: Ok(ImpliedValue {
            size: size.into(),
            price: Some(price),
            value,
            model_premium: 0.0441,
            model_rate: 0.08,
            model_cagr: 0.05,
            model_margin: 0.10,
            premium: found(0.05),
            implied_rate: 0.08 + d_rate,
            growth: found(0.0),
            implied_cagr: 0.05 + d_growth,
            margin: found(0.0),
            implied_margin: 0.10 + d_margin,
            at_market: Some(at_market),
            notes: vec![],
        }),
    }
}

#[test]
fn the_group_medians_of_a_list_of_six_rows_are_the_hand_calculated_ones() {
    // value / price: 0.5 0.8 0.9 1.0 1.2 1.5 -> P25 = 0.8 + 0.25 (0.9 - 0.8) = 0.825, P50 = 0.95, P75 = 1.0 + 0.75 (1.2 - 1.0) = 1.15
    let rows = vec![
        row("A1", BUCKET_LABELS[1], "S1", "I1", 100.0, 50.0, 0.01, 0.00, -0.02, 0.6),
        row("A2", BUCKET_LABELS[1], "S1", "I1", 100.0, 80.0, 0.02, 0.01, -0.01, 0.9),
        row("A3", BUCKET_LABELS[1], "S1", "I1", 100.0, 90.0, 0.03, 0.02, 0.00, 1.0),
        row("B1", BUCKET_LABELS[2], "S2", "I1", 100.0, 100.0, -0.01, 0.03, 0.01, 1.1),
        row("B2", BUCKET_LABELS[2], "S2", "I1", 100.0, 120.0, 0.05, 0.04, 0.02, 1.2),
        row("B3", BUCKET_LABELS[2], "S2", "I2", 100.0, 150.0, 0.00, 0.05, 0.03, 1.4),
    ];
    let t = group_table(&rows);
    let names: Vec<&str> = t.iter().map(|g| g.name.as_str()).collect();
    // the size groups in the order of the buckets; the sectors; the industry I1 has 5 tickers, I2 has 1
    assert_eq!(names, vec!["all", "size 1-10B", "size 10-30B", "sector S1", "sector S2", "industry I1"]);
    let all = &t[0];
    assert_eq!(all.n, 6);
    let q = all.measures[0].unwrap();
    assert!((q.p25 - 0.825).abs() < 1e-12 && (q.p50 - 0.95).abs() < 1e-12 && (q.p75 - 1.15).abs() < 1e-12, "{q:?}");
    // rate differences 0.01 0.02 0.03 -0.01 0.05 0.00: sorted -0.01 0 0.01 0.02 0.03 0.05; median (0.01 + 0.02) / 2
    let r = all.measures[1].unwrap();
    assert!((r.p50 - 0.015).abs() < 1e-12 && (r.p25 - 0.0025).abs() < 1e-12 && (r.p75 - 0.0275).abs() < 1e-12, "{r:?}");
    // growth differences 0 .01 .02 .03 .04 .05: median 0.025; margin differences -.02 -.01 0 .01 .02 .03: median 0.005
    assert!((all.measures[2].unwrap().p50 - 0.025).abs() < 1e-12);
    assert!((all.measures[3].unwrap().p50 - 0.005).abs() < 1e-12);
    // value at the list premium over price: 0.6 0.9 1.0 1.1 1.2 1.4 -> median 1.05
    assert!((all.measures[4].unwrap().p50 - 1.05).abs() < 1e-12);
    // size 1-10B: value / price 0.5 0.8 0.9 -> median 0.8; sector S2: 1.0 1.2 1.5 -> median 1.2
    assert_eq!((t[1].n, t[1].measures[0].unwrap().p50), (3, 0.8));
    assert!((t[4].measures[0].unwrap().p50 - 1.2).abs() < 1e-12);
    // industry I1: 0.5 0.8 0.9 1.0 1.2 -> median 0.9
    assert_eq!((t[5].n, t[5].measures[0].unwrap().p50), (5, 0.9));
    // without group data only all and size
    let plain: Vec<ImpliedRow> = rows.iter().cloned().map(|mut r| {
        r.sector.clear();
        r.industry.clear();
        r
    }).collect();
    assert_eq!(group_table(&plain).iter().map(|g| g.name.clone()).collect::<Vec<_>>(), vec!["all", "size 1-10B", "size 10-30B"]);
    assert!(group_text(&t).contains("sector S1"));
}

#[test]
fn a_group_file_gives_the_sector_and_the_industry() {
    let g = parse_groups("ticker,sector,industry\nibm, Technology , IT Services, Consulting\n# a comment\nJNJ,Health Care,Pharma\n", "test").unwrap();
    assert_eq!(groups_of("IBM", &g), ("Technology".to_string(), "IT Services, Consulting".to_string()));
    assert_eq!(groups_of("JNJ", &g).0, "Health Care");
    assert_eq!(groups_of("NONE", &g), (String::new(), String::new()));
    assert!(parse_groups("IBM,Technology\n", "test").is_err());
    // a comma of an industry does not break the CSV
    let r = row("IBM", "1-10B", "Technology", "IT Services, Consulting", 100.0, 100.0, 0.0, 0.0, 0.0, 1.0);
    assert_eq!(csv_line(&r).split(',').count(), CSV_HEADER.split(',').count());
}

#[test]
fn the_premium_of_the_list_gives_a_median_of_one() {
    let inp: Vec<AutoInputs> = synthetic(12).into_iter().filter(|i| i.template == "standard" && i.years.last().unwrap().capex.is_some()).collect();
    assert!(inp.len() >= 6);
    let market_of = |t: &str| -> Market {
        let k: f64 = t[1..].parse().unwrap();
        Market { price: Some(8.0 + 3.0 * k), price_source: "test".into(), ..market() }
    };
    let (rows, mp) = run(&inp, rules_w1(), &market_of, Some(2));
    let mp = mp.unwrap();
    assert_eq!(mp.n, inp.len());
    assert_eq!(mp.found.bound, Bound::Exact, "{mp:?}");
    let ratios: Vec<f64> = rows.iter().map(|r| value_of(r).at_market.unwrap()).collect();
    let med = {
        let s = sorted_copy(&ratios);
        percentile(&s, 50.0)
    };
    assert!((med - 1.0).abs() < 1e-6, "median {med} at the premium {}", mp.found.x);
    // the premium that equalizes the median is between the premiums of the cheapest and the dearest ticker
    let prem: Vec<f64> = rows.iter().map(|r| value_of(r).premium.unwrap().x).collect();
    let (lo, hi) = (prem.iter().cloned().fold(f64::MAX, f64::min), prem.iter().cloned().fold(f64::MIN, f64::max));
    assert!(mp.found.x >= lo - 1e-9 && mp.found.x <= hi + 1e-9, "{} not in [{lo}, {hi}]", mp.found.x);
}

#[test]
fn one_thread_and_four_threads_give_the_same_rows_and_two_runs_give_the_same_bytes() {
    let inp = synthetic(24);
    let market_of = |t: &str| -> Market {
        let k: f64 = t[1..].parse().unwrap();
        Market { price: if k % 3.0 == 0.0 { None } else { Some(5.0 + 2.0 * k) }, price_source: "test".into(), ..market() }
    };
    let rules = Rules::default();
    let (a, ma) = run(&inp, rules.clone(), &market_of, Some(1));
    let (b, mb) = run(&inp, rules.clone(), &market_of, Some(4));
    let (c, _) = run(&inp, rules, &market_of, Some(1));
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    assert_eq!(format!("{ma:?}"), format!("{mb:?}"));
    assert_eq!(csv_text(&a), csv_text(&c));
    assert_eq!(csv_text(&a), csv_text(&b));
    // refusals have their reason and the same number of fields
    let refused: Vec<String> = a.iter().filter_map(|r| r.outcome.as_ref().err().map(|e| e.0.clone())).collect();
    assert!(refused.iter().any(|k| k == "template bank: no model") && refused.iter().any(|k| k == "missing capex"));
    for r in &a {
        assert_eq!(csv_line(r).split(',').count(), CSV_HEADER.split(',').count(), "{}", csv_line(r));
    }
    // a ticker without a price has a value and no search
    let no_price = a.iter().filter_map(|r| r.outcome.as_ref().ok()).find(|v| v.price.is_none()).unwrap();
    assert!(no_price.premium.is_none() && no_price.growth.is_none() && no_price.margin.is_none() && no_price.at_market.is_none());
}

#[test]
fn the_shifts_of_the_builder_move_one_number_and_zero_changes_nothing() {
    let (t, i, m) = (table(), inputs(), market());
    let base = build(&i, &t, &[], &m, &plain_rules(), BuildOptions::default()).unwrap();
    let zero = build(&i, &t, &[], &m, &Rules { growth_shift: 0.0, margin_shift: 0.0, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert_eq!(base, zero);
    assert_eq!(plain_rules().growth_shift, 0.0);
    assert_eq!(Rules::default().margin_shift, 0.0);
    let g = build(&i, &t, &[], &m, &Rules { growth_shift: 0.05, ..plain_rules() }, BuildOptions::default()).unwrap();
    let (b, s) = (&base.summary, &g.summary);
    assert!((s.growth_company_mean - b.growth_company_mean - 0.05).abs() < 1e-12);
    assert!((s.growth_start - b.growth_start - 0.05).abs() < 1e-12);
    assert_eq!((s.margin_start, s.margin_target), (b.margin_start, b.margin_target));
    let mg = build(&i, &t, &[], &m, &Rules { margin_shift: -0.02, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert!((mg.summary.margin_start - b.margin_start + 0.02).abs() < 1e-12 && (mg.summary.margin_target - b.margin_target + 0.02).abs() < 1e-12);
    assert_eq!(mg.summary.growth_start, b.growth_start);
    // the premium below the lower bound of the draw (2%) builds a model
    let low = build(&i, &t, &[], &m, &Rules { erp: (-0.03, 0.0075), ..plain_rules() }, BuildOptions::default()).unwrap();
    assert!(PathsSpec::from_toml(&low.toml).is_ok());
    // the default lower bound stays 2%
    assert!(base.toml.contains("erp = { mean = 0.0441, std = 0.0075, lo = 0.02 }"));
}
