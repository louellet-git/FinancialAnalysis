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
    assert!(c.resid_ln.abs() < 1e-6, "{}", c.resid_ln);
    assert!(!a.new_facts);
    // the percent parts add to the change of the price
    let sum: f64 = c.pct().iter().sum();
    assert!((sum - (p2 / p1 - 1.0)).abs() < 1e-12);
    // the order of rates and premium does not matter when the rates do not move
    assert!((c.rev_free_ln - c.free_ln).abs() < 1e-9 && c.rev_rates_ln.abs() < 1e-9);
    // the growth view: the same price move is a growth shift, and the parts add to the change as well
    let g = grow(&a);
    assert!(g.facts_ln.abs() < 1e-12 && g.rates_ln.abs() < 1e-12 && g.resid_ln.abs() < 1e-6, "{g:?}");
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
    assert!(c.resid_ln.abs() < 1e-6);
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
    assert!(c.rates_ln.abs() < 1e-12 && c.free_ln.abs() < 1e-6 && c.resid_ln.abs() < 1e-6, "{c:?}");
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
    assert!((c.free_ln - (3.0f64 / 1.2).ln()).abs() < 1e-6 && c.resid_ln.abs() < 1e-6, "{c:?}");
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

fn chain(ln: f64, which: usize, x1: f64) -> Chain {
    let f = |x: f64| Found { x, bound: Bound::Exact, monotone: true };
    let mut c = Chain { x1: f(x1), x2: f(0.05), facts_ln: 0.0, time_ln: 0.0, rates_ln: 0.0, free_ln: 0.0, resid_ln: 0.0, rev_free_ln: 0.0, rev_rates_ln: 0.0 };
    if which == 3 {
        c.free_ln = ln;
        c.rev_free_ln = ln;
    } else {
        c.rates_ln = ln;
        c.rev_rates_ln = ln;
    }
    c
}

fn attribution(p1: f64, p2: f64, c: Chain, shares: f64) -> Attribution {
    Attribution {
        p1,
        p2,
        model_premium: 0.05,
        prem: Ok(c.clone()),
        grow: Ok(c),
        cagr1: 0.1,
        cagr2: 0.1,
        growth_at_x1: None,
        cagr_at_x1: f64::NAN,
        shares1: shares,
        size: "all".into(),
        new_facts: false,
        curve1: "2026-01-01".into(),
        curve2: "2026-01-01".into(),
        date1: "2026-01-01".into(),
        date2: "2026-01-02".into(),
    }
}

/// `which` 3: all of the change is in the plug; 2: all in the rates.
fn row(name: &str, p1: f64, p2: f64, which: usize, shares: f64, x1: f64) -> AttrRow {
    let c = chain((p2 / p1).ln(), which, x1);
    AttrRow { ticker: name.into(), sector: "S".into(), industry: String::new(), prices: Some((p1, p2)), shares: Some(shares), shares_note: None, cik: None, weight_of: None, outcome: Ok(attribution(p1, p2, c, shares)) }
}

#[test]
fn the_sums_have_the_hand_calculated_answers() {
    // A: market value 100 (price 10, 10 shares) falls 20%, all in the plug; B: market value 300 falls 10%, all in the rates.
    // Whole list: (100 x -0.2 + 300 x -0.1) / 400 = -0.125. A refused bank C (market value 600, price 60) falls 5%: it is in the
    // whole list and in no view. Whole: (-20 - 30 - 30) / 1000 = -0.08.
    let mut rows = vec![row("A", 10.0, 8.0, 3, 10.0, 0.05), row("B", 30.0, 27.0, 2, 10.0, 0.05)];
    rows.push(AttrRow {
        ticker: "C".into(),
        sector: "Money".into(),
        industry: String::new(),
        prices: Some((60.0, 57.0)),
        shares: Some(10.0),
        shares_note: None,
        cik: None,
        weight_of: None,
        outcome: Err(("template bank: no model".into(), String::new())),
    });
    let whole = whole_list(&rows).unwrap();
    assert_eq!(whole.n, 3);
    assert!((whole.value1 - 1000.0).abs() < 1e-9 && (whole.change + 0.08).abs() < 1e-12, "{whole:?}");
    assert_eq!(whole.top[0].0, "C");
    let w = set_sum(&rows, View::Premium, SetKind::Own, &whole, false).unwrap();
    assert_eq!(w.n, 2);
    assert!((w.value1 - 400.0).abs() < 1e-9 && (w.share_of_list - 0.4).abs() < 1e-12);
    assert!((w.change + 0.125).abs() < 1e-12, "{w:?}");
    // the plug (index 3) is -0.05 and the rates (index 2) are -0.075 of the market value of the set
    assert!((w.parts[3] + 0.05).abs() < 1e-12 && (w.parts[2] + 0.075).abs() < 1e-12, "{w:?}");
    assert!(w.parts[0].abs() < 1e-12 && w.parts[1].abs() < 1e-12 && w.parts[4].abs() < 1e-12);
    assert!((w.parts.iter().sum::<f64>() - w.change).abs() < 1e-12);
    // the part of the list outside the set: C only, -5%
    assert!((w.change_excluded + 0.05).abs() < 1e-12, "{w:?}");
    // the largest contributor to the plug is A (-5 points)
    assert_eq!(w.top_parts[3][0].0, "A");
    assert!((w.top_parts[3][0].1 + 0.05).abs() < 1e-12);
    // the common set is the same here (both views exist for A and B)
    let c = set_sum(&rows, View::Growth, SetKind::Common, &whole, false).unwrap();
    assert_eq!(c.n, 2);
    // a ticker with no premium view is left out of the premium set and the common set
    let mut rows2 = rows.clone();
    if let Ok(a) = &mut rows2[1].outcome {
        a.prem = Err("none".into());
    }
    let w2 = set_sum(&rows2, View::Premium, SetKind::Own, &whole, false).unwrap();
    assert_eq!(w2.n, 1);
    assert!((w2.change + 0.2).abs() < 1e-12 && (w2.value1 - 100.0).abs() < 1e-9);
    assert_eq!(set_sum(&rows2, View::Premium, SetKind::Common, &whole, false).unwrap().n, 1);
    assert_eq!(set_sum(&rows2, View::Growth, SetKind::Common, &whole, false).unwrap().n, 1);
    assert_eq!(set_sum(&rows2, View::Growth, SetKind::Own, &whole, false).unwrap().n, 2);
    // the medians of the group: two values
    let g = group_medians(&rows, View::Premium);
    assert_eq!(g[0].name, "all");
    assert!((g[0].total + 0.15).abs() < 1e-12, "{g:?}");
    assert!(g.iter().any(|l| l.name == "sector S"));
}

#[test]
fn a_flagged_ticker_is_listed_and_left_out_of_the_sum_without_the_flagged() {
    // A has an implied premium of 0.5% at D1 (below 1%): flagged in the premium view only. B has a part of 30% of P1: flagged
    // in both views. D is clean.
    let rows = vec![row("A", 10.0, 9.0, 3, 10.0, 0.005), row("B", 30.0, 39.0, 2, 10.0, 0.05), row("D", 20.0, 19.0, 3, 10.0, 0.04)];
    let whole = whole_list(&rows).unwrap();
    let fp: Vec<String> = flagged_list(&rows, View::Premium).into_iter().map(|x| x.0).collect();
    assert_eq!(fp, vec!["A", "B"]);
    let fg: Vec<String> = flagged_list(&rows, View::Growth).into_iter().map(|x| x.0).collect();
    assert_eq!(fg, vec!["B"]);
    let clean = set_sum(&rows, View::Premium, SetKind::Own, &whole, true).unwrap();
    assert_eq!(clean.n, 1);
    assert!((clean.change + 0.05).abs() < 1e-12, "{clean:?}");
}

#[test]
fn a_share_count_that_is_a_thousand_times_too_large_is_replaced_and_flagged() {
    let mut inp = inputs();
    // the 10-Q of WAT: the weighted diluted count is 98,204,000,000, the count at the end of the period is 59.5M
    let b = inp.balance.as_mut().unwrap();
    b.shares = Some(98.204e9);
    b.shares_outstanding = Some(59.5e6);
    let (s, note) = guarded_shares(&inp, Some(98.204e9));
    assert_eq!(s, Some(59.5e6));
    assert!(note.unwrap().starts_with("replaced: shares 98204.0M -> 59.5M"));
    // a count within 0.5 to 2 times the reference stays
    let (s, note) = guarded_shares(&inp, Some(70e6));
    assert_eq!((s, note), (Some(70e6), None));
    // a split after the reference date (25 for 1): the count stays and has a note
    let (s, note) = guarded_shares(&inp, Some(25.0 * 59.5e6));
    assert_eq!(s, Some(25.0 * 59.5e6));
    assert!(note.unwrap().starts_with("differs"));
    // no count at the end of the period: the count of the year before is the reference
    let mut inp2 = inputs();
    inp2.balance.as_mut().unwrap().shares_outstanding = None;
    inp2.balance.as_mut().unwrap().shares = Some(5e10);
    let (s, note) = guarded_shares(&inp2, None);
    assert_eq!(s, Some(1e8), "the year before has 100M shares");
    assert!(note.is_some());
    // the weight of the whole list does not follow the wrong count: with the guard A is 59.5M x 100, without it 98B x 100
    let guarded = AttrRow { shares: Some(59.5e6), shares_note: Some("replaced: x".into()), ..row("WAT", 100.0, 90.0, 3, 59.5e6, 0.05) };
    let other = row("O", 100.0, 100.0, 3, 59.5e6, 0.05);
    let w = whole_list(&[guarded, other]).unwrap();
    assert!((w.change + 0.05).abs() < 1e-12 && w.guarded == vec!["WAT"], "{w:?}");
}

#[test]
fn the_parts_in_sequence_add_to_the_change() {
    let p = sequential(&[0.1, -0.3, 0.05, 0.02, 0.01]);
    let sum: f64 = p.iter().sum();
    assert!((sum - ((0.1f64 - 0.3 + 0.05 + 0.02 + 0.01).exp() - 1.0)).abs() < 1e-12);
    assert!((p[0] - (0.1f64.exp() - 1.0)).abs() < 1e-12);
}

#[test]
fn two_classes_of_one_company_have_one_weight() {
    // GOOGL and GOOG: the same company, the same share count of the company (10), two prices. The first ticker carries the
    // weight. X is another company. Without the rule the company would be counted two times.
    let mut rows = vec![row("GOOGL", 100.0, 90.0, 3, 10.0, 0.05), row("GOOG", 101.0, 90.9, 3, 10.0, 0.05), row("X", 50.0, 50.0, 3, 10.0, 0.05)];
    rows[0].cik = Some("1652044".into());
    rows[1].cik = Some("1652044".into());
    rows[2].cik = Some("9".into());
    one_weight_for_one_company(&mut rows);
    assert_eq!(rows[1].weight_of.as_deref(), Some("GOOGL"));
    assert!(rows[0].weight_of.is_none() && rows[2].weight_of.is_none());
    let w = whole_list(&rows).unwrap();
    // GOOGL: 1000 falls 10%; X: 500 flat. (-100) / 1500
    assert_eq!(w.n, 2);
    assert!((w.value1 - 1500.0).abs() < 1e-9 && (w.change + 100.0 / 1500.0).abs() < 1e-12, "{w:?}");
    assert_eq!(w.dropped, vec![("GOOG".to_string(), "GOOGL".to_string())]);
    // the set sums do not count GOOG either, and the rows keep both tickers (medians over rows)
    let p = set_sum(&rows, View::Premium, SetKind::Own, &w, false).unwrap();
    assert_eq!(p.n, 2);
    assert_eq!(group_medians(&rows, View::Premium)[0].n, 3);
    // with no rule: three tickers
    let raw = whole_where(&rows, false, &|_| true).unwrap();
    assert_eq!(raw.n, 3);
    assert!((raw.value1 - (1000.0 + 1010.0 + 500.0)).abs() < 1e-9);
}
