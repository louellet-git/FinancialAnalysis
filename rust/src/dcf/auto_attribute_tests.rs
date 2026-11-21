//! Tests of `fa dcf attribute` on small data made by hand (fixed answers). No network and no cache.
//!
//! The price enters the builder through the equity weight, so the tests set `equity_weight_one`: a price is then the value of
//! the model at a premium and the parts have closed answers.

use super::super::auto::tests::{inputs, market, plain_rules, synthetic, table};
use super::*;

fn rules_w1() -> Rules {
    Rules { equity_weight_one: true, ..plain_rules() }
}

fn cfg<'a>(t: &'a super::super::calibrate::ProcessTable) -> ImpliedConfig<'a> {
    ImpliedConfig { table: t, fade: &[], rules: rules_w1(), cache: None }
}

fn curve(zero10: f64, beta1: f64) -> CurveNumbers {
    CurveNumbers { date: "2026-01-01".into(), beta0: 0.0, beta1, beta2: 5.0, beta3: 17.0, tau1: 1.5, tau2: 6.0, zero10 }
}

fn mk(price: f64, curve: Option<CurveNumbers>) -> Market {
    Market { price: Some(price), price_source: "test".into(), curve, ..market() }
}

/// The value at the means of a model at a premium (a price is a value in these tests).
fn value(inp: &AutoInputs, m: &Market, erp: f64) -> f64 {
    let t = table();
    value_at(inp, &cfg(&t), m, erp).unwrap()
}

fn prem(a: &Attribution) -> &Chain {
    a.prem.as_ref().unwrap_or_else(|x| panic!("{x}"))
}

fn grow(a: &Attribution) -> &Chain {
    a.grow.as_ref().unwrap_or_else(|x| panic!("{x}"))
}

#[test]
fn only_the_price_moves_the_whole_change_is_the_premium() {
    let t = table();
    let model = rules_w1().erp.0;
    let (p1, p2) = (value(&inputs(), &mk(1.0, None), model), value(&inputs(), &mk(1.0, None), 0.03));
    assert!(p2 > p1 && p1 > 1.0);
    let a = attribute_models(&inputs(), &mk(p1, None), &inputs(), &mk(p2, None), &cfg(&t)).unwrap();
    let c = prem(&a);
    // the premium that each price implies: the model premium and 3% (closed: the price is the value at that premium)
    assert!((c.x1.x - model).abs() < 1e-9 && (c.x2.x - 0.03).abs() < 1e-9, "{c:?}");
    assert!(c.x1.bound == Bound::Exact && c.x2.bound == Bound::Exact);
    assert!(c.facts_ln.abs() < 1e-12 && c.rates_ln.abs() < 1e-12 && c.time_ln.abs() < 1e-12, "{c:?}");
    assert!((c.free_ln - (p2 / p1).ln()).abs() < 1e-6, "{} {}", c.free_ln, (p2 / p1).ln());
    assert!(c.rest_ln.abs() < 1e-6, "{}", c.rest_ln);
    assert!(!a.new_facts);
    // the percent parts add to the change of the price
    let sum: f64 = c.pct().iter().sum();
    assert!((sum - (p2 / p1 - 1.0)).abs() < 1e-12);
    // the order of rates and premium does not matter when the rates do not move
    assert!((c.rev_free_ln - c.free_ln).abs() < 1e-9 && c.rev_rates_ln.abs() < 1e-9);
    // the growth view: the same price move is a growth shift, and the parts add to the change as well
    let g = grow(&a);
    assert!(g.facts_ln.abs() < 1e-12 && g.rates_ln.abs() < 1e-12 && g.rest_ln.abs() < 1e-6, "{g:?}");
    assert!((g.free_ln - (p2 / p1).ln()).abs() < 1e-6);
    assert!(g.x2.x > g.x1.x && g.x1.x.abs() < 1e-9, "the price at the model premium needs no shift at D1: {g:?}");
}

#[test]
fn only_the_curve_moves_the_whole_change_is_the_rates() {
    let t = table();
    let model = rules_w1().erp.0;
    let (c1, c2) = (curve(0.042, 4.0), curve(0.047, 4.6));
    let p1 = value(&inputs(), &mk(1.0, Some(c1.clone())), model);
    let p2 = value(&inputs(), &mk(1.0, Some(c2.clone())), model);
    assert!((p2 / p1).ln().abs() > 1e-3, "the curve must move the value: {p1} {p2}");
    let a = attribute_models(&inputs(), &mk(p1, Some(c1)), &inputs(), &mk(p2, Some(c2)), &cfg(&t)).unwrap();
    let c = prem(&a);
    assert!((c.x1.x - model).abs() < 1e-9 && (c.x2.x - model).abs() < 1e-9);
    assert!(c.facts_ln.abs() < 1e-12 && c.free_ln.abs() < 1e-6, "{c:?}");
    assert!((c.rates_ln - (p2 / p1).ln()).abs() < 1e-6, "{} {}", c.rates_ln, (p2 / p1).ln());
    assert!(c.rest_ln.abs() < 1e-6);
    // the reverse order: the premium does not move, so the same rates part
    assert!((c.rev_rates_ln - c.rates_ln).abs() < 1e-6 && c.rev_free_ln.abs() < 1e-6);
    // the dates of the curve
    assert_eq!(a.curve1, "2026-01-01");
}

#[test]
fn only_the_facts_move_the_whole_change_is_the_new_facts() {
    let t = table();
    let model = rules_w1().erp.0;
    let inp1 = inputs();
    // a later filing: the revenue of every year is 15% higher
    let mut inp2 = inputs();
    for y in inp2.years.iter_mut() {
        y.revenue = y.revenue.map(|r| r * 1.15);
        y.operating_cash_flow = y.operating_cash_flow.map(|r| r * 1.15);
        y.capex = y.capex.map(|r| r * 1.15);
    }
    inp2.balance = Some(BalanceData::from_year(inp2.years.last().unwrap()));
    let m = mk(1.0, None);
    let (p1, p2) = (value(&inp1, &m, model), value(&inp2, &m, model));
    assert!(p2 > p1);
    let a = attribute_models(&inp1, &mk(p1, None), &inp2, &mk(p2, None), &cfg(&t)).unwrap();
    assert!(a.new_facts);
    let c = prem(&a);
    assert!((c.facts_ln - (p2 / p1).ln()).abs() < 1e-6, "{} {}", c.facts_ln, (p2 / p1).ln());
    assert!(c.rates_ln.abs() < 1e-12 && c.free_ln.abs() < 1e-6 && c.rest_ln.abs() < 1e-6, "{c:?}");
    assert!((c.x2.x - model).abs() < 1e-9);
    assert!((grow(&a).facts_ln - (p2 / p1).ln()).abs() < 1e-6);
}

#[test]
fn a_price_far_above_the_value_moves_the_growth_view_by_the_closed_answer() {
    let t = table();
    // the price rises by a factor 2.5 from 1.2 to 3.0 times the value at the model premium: all of it is the growth shift
    let model_value = value(&inputs(), &mk(1.0, None), rules_w1().erp.0);
    let a = attribute_models(&inputs(), &mk(model_value * 1.2, None), &inputs(), &mk(model_value * 3.0, None), &cfg(&t)).unwrap();
    assert!(a.grow.is_ok());
    let c = grow(&a);
    assert!((c.free_ln - (3.0f64 / 1.2).ln()).abs() < 1e-6 && c.rest_ln.abs() < 1e-6, "{c:?}");
}

#[test]
fn one_and_four_threads_give_the_same_rows() {
    let t = table();
    let list = synthetic(12);
    let m1 = |_: &str| mk(30.0, Some(curve(0.042, 4.0)));
    let m2 = |_: &str| mk(27.0, Some(curve(0.045, 4.3)));
    let one = attribute_list(&list, &list, &m1, &m2, &HashMap::new(), &cfg(&t), Some(1)).unwrap();
    let four = attribute_list(&list, &list, &m1, &m2, &HashMap::new(), &cfg(&t), Some(4)).unwrap();
    // the rows hold NaN (no solution), so compare the text of the rows
    assert_eq!(format!("{one:?}"), format!("{four:?}"));
    assert_eq!(csv_text(&one), csv_text(&four));
    assert!(one.iter().filter(|r| r.outcome.is_ok()).count() >= 6);
    // every row of the CSV has the cells of the header
    let n = CSV_HEADER.split(',').count();
    assert!(csv_text(&one).lines().all(|l| l.split(',').count() == n));
}

fn chain(ln: f64, which: usize) -> Chain {
    let f = |x: f64| Found { x, bound: Bound::Exact, monotone: true };
    let mut c = Chain { x1: f(0.05), x2: f(0.05), facts_ln: 0.0, rates_ln: 0.0, free_ln: 0.0, time_ln: 0.0, rest_ln: 0.0, rev_free_ln: 0.0, rev_rates_ln: 0.0, rev_rest_ln: 0.0, total_ln: ln };
    if which == 2 {
        c.free_ln = ln;
        c.rev_free_ln = ln;
    } else {
        c.rates_ln = ln;
        c.rev_rates_ln = ln;
    }
    c
}

#[test]
fn the_weighted_sum_has_the_hand_calculated_answer() {
    // two tickers: market value 100 (price 10, 10 shares) falls 20%, all in the premium; market value 300 (price 30, 10
    // shares) falls 10%, all in the rates. The sum: (100 x -0.2 + 300 x -0.1) / 400 = -0.125 = premium -0.05, rates -0.075.
    let row = |name: &str, p1: f64, p2: f64, which: usize| {
        let c = chain((p2 / p1).ln(), which);
        AttrRow {
            ticker: name.into(),
            sector: "S".into(),
            industry: String::new(),
            outcome: Ok(Attribution {
                p1,
                p2,
                model_premium: 0.05,
                prem: Ok(c.clone()),
                grow: Ok(c),
                cagr1: 0.1,
                cagr2: 0.1,
                growth_at_x1: None,
                cagr_at_x1: f64::NAN,
                shares1: 10.0,
                size: "all".into(),
                new_facts: false,
                curve1: "d".into(),
                curve2: "d".into(),
            }),
        }
    };
    let rows = vec![row("A", 10.0, 8.0, 2), row("B", 30.0, 27.0, 1)];
    let w = weighted_sum(&rows, View::Premium).unwrap();
    assert_eq!((w.n, w.n_without), (2, 0));
    assert!((w.value1 - 400.0).abs() < 1e-9);
    assert!((w.total + 0.125).abs() < 1e-12, "{w:?}");
    assert!((w.parts[2] + 0.05).abs() < 1e-12 && (w.parts[1] + 0.075).abs() < 1e-12, "{w:?}");
    assert!(w.parts[0].abs() < 1e-12 && w.parts[3].abs() < 1e-12);
    assert!((w.parts.iter().sum::<f64>() - w.total).abs() < 1e-12);
    // the median of the group: two values, the mean of the two middle ones
    let g = group_medians(&rows, View::Premium);
    assert_eq!(g[0].name, "all");
    assert!((g[0].total + 0.15).abs() < 1e-12, "{g:?}");
    assert!(g.iter().any(|l| l.name == "sector S"));
    // a ticker with no premium view is left out of the premium sum and counted
    let mut rows2 = rows.clone();
    if let Ok(a) = &mut rows2[1].outcome {
        a.prem = Err("none".into());
    }
    let w2 = weighted_sum(&rows2, View::Premium).unwrap();
    assert_eq!((w2.n, w2.n_without), (1, 1));
    assert!((w2.total + 0.2).abs() < 1e-12 && (w2.value1 - 100.0).abs() < 1e-9 && (w2.value1_all - 400.0).abs() < 1e-9);
}

#[test]
fn the_parts_in_sequence_add_to_the_change() {
    let p = sequential(&[0.1, -0.3, 0.05, 0.02]);
    let sum: f64 = p.iter().sum();
    assert!((sum - ((0.1f64 - 0.3 + 0.05 + 0.02).exp() - 1.0)).abs() < 1e-12);
    assert!((p[0] - (0.1f64.exp() - 1.0)).abs() < 1e-12);
}
