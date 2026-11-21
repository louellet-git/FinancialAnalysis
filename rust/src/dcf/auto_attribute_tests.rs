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
    let mut c = Chain { x1: f(x1), x2: f(0.05), facts_ln: 0.0, time_ln: 0.0, rates_ln: 0.0, free_ln: 0.0, resid_ln: 0.0, rev_free_ln: 0.0, rev_rates_ln: 0.0, split: None };
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
        growth2_bound: Bound::Exact,
        growth_at_x1: None,
        cagr_at_x1: f64::NAN,
        shares1: shares,
        size: "all".into(),
        new_facts: false,
        curve1: "2026-01-01".into(),
        curve2: "2026-01-01".into(),
        beta1: None,
        beta2: None,
        dur_premium1: None,
        date1: "2026-01-01".into(),
        date2: "2026-01-02".into(),
    }
}

/// `which` 3: all of the change is in the plug; 2: all in the rates.
fn row(name: &str, p1: f64, p2: f64, which: usize, shares: f64, x1: f64) -> AttrRow {
    let c = chain((p2 / p1).ln(), which, x1);
    AttrRow { ticker: name.into(), sector: "S".into(), industry: String::new(), prices: Some((p1, p2)), shares: Some(shares), shares_note: None, cik: None, weight_of: None, outcome: Ok(attribution(p1, p2, c, shares)), earnings_8k: String::new(), stale: None }
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
        earnings_8k: String::new(),
        stale: None,
    });
    let whole = whole_list(&rows).unwrap();
    assert_eq!(whole.n, 3);
    assert!((whole.value1 - 1000.0).abs() < 1e-9 && (whole.change + 0.08).abs() < 1e-12, "{whole:?}");
    assert_eq!(whole.top[0].0, "C");
    let w = set_sum(&rows, View::Premium, SetKind::Own, &whole).unwrap();
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
    let c = set_sum(&rows, View::Growth, SetKind::Common, &whole).unwrap();
    assert_eq!(c.n, 2);
    // a ticker with no premium view is left out of the premium set and the common set
    let mut rows2 = rows.clone();
    if let Ok(a) = &mut rows2[1].outcome {
        a.prem = Err("none".into());
    }
    let w2 = set_sum(&rows2, View::Premium, SetKind::Own, &whole).unwrap();
    assert_eq!(w2.n, 1);
    assert!((w2.change + 0.2).abs() < 1e-12 && (w2.value1 - 100.0).abs() < 1e-9);
    assert_eq!(set_sum(&rows2, View::Premium, SetKind::Common, &whole).unwrap().n, 1);
    assert_eq!(set_sum(&rows2, View::Growth, SetKind::Common, &whole).unwrap().n, 1);
    assert_eq!(set_sum(&rows2, View::Growth, SetKind::Own, &whole).unwrap().n, 2);
    // the medians of the group: two values
    let g = group_medians(&rows, View::Premium);
    assert_eq!(g[0].name, "all");
    assert!((g[0].total + 0.15).abs() < 1e-12, "{g:?}");
    assert!(g.iter().any(|l| l.name == "sector S"));
}

#[test]
fn a_flagged_ticker_is_listed() {
    // A has an implied premium of 0.5% at D1 (below 1%): flagged in the premium view only. B has a part of 30% of P1: flagged
    // in both views. D is clean.
    let rows = vec![row("A", 10.0, 9.0, 3, 10.0, 0.005), row("B", 30.0, 39.0, 2, 10.0, 0.05), row("D", 20.0, 19.0, 3, 10.0, 0.04)];
    let fp: Vec<String> = flagged_list(&rows, View::Premium).into_iter().map(|x| x.0).collect();
    assert_eq!(fp, vec!["A", "B"]);
    let fg: Vec<String> = flagged_list(&rows, View::Growth).into_iter().map(|x| x.0).collect();
    assert_eq!(fg, vec!["B"]);
}

#[test]
fn a_scale_error_of_1650_times_is_replaced_and_a_split_is_kept_with_a_note() {
    let mut inp = inputs();
    // the 10-Q of WAT: the weighted diluted count is 98,204,000,000, the count at the end of the period is 59.5M
    let b = inp.balance.as_mut().unwrap();
    b.shares = Some(98.204e9);
    b.shares_outstanding = Some(59.5e6);
    let (s, note) = guarded_shares(&inp, Some(98.204e9));
    assert_eq!(s, Some(59.5e6));
    assert!(note.unwrap().starts_with("replaced: shares 98204.0M -> 59.5M"));
    // the edges: exactly 100 times and exactly 0.01 times stay (with a note); a little beyond them is replaced
    let mut edge = inputs();
    edge.balance.as_mut().unwrap().shares_outstanding = Some(1e6);
    assert_eq!(guarded_shares(&edge, Some(1e8)).0, Some(1e8));
    assert!(guarded_shares(&edge, Some(1e8)).1.unwrap().starts_with("differs"));
    assert_eq!(guarded_shares(&edge, Some(1.0001e8)).0, Some(1e6));
    assert_eq!(guarded_shares(&edge, Some(1e4)).0, Some(1e4));
    assert_eq!(guarded_shares(&edge, Some(0.99e4)).0, Some(1e6));
    // equal values: no note
    assert_eq!(guarded_shares(&edge, Some(1e6)), (Some(1e6), None));
    // a reference of 0 is no reference: the count stays, with no note
    edge.balance.as_mut().unwrap().shares_outstanding = Some(0.0);
    for y in edge.years.iter_mut() {
        y.shares = Some(0.0);
    }
    assert_eq!(guarded_shares(&edge, Some(7e6)), (Some(7e6), None));
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
    let p = set_sum(&rows, View::Premium, SetKind::Own, &w).unwrap();
    assert_eq!(p.n, 2);
    assert_eq!(group_medians(&rows, View::Premium)[0].n, 3);
    // with no rule: three tickers
    let raw = whole_where(&rows, false, &|_| true).unwrap();
    assert_eq!(raw.n, 3);
    assert!((raw.value1 - (1000.0 + 1010.0 + 500.0)).abs() < 1e-9);
}

#[test]
fn the_curve_age_flag_has_the_edge_at_three_days() {
    let with = |c1: &str, d1: &str, c2: &str, d2: &str| {
        let mut a = attribution(10.0, 9.0, chain(-0.1, 3, 0.05), 10.0);
        a.curve1 = c1.into();
        a.date1 = d1.into();
        a.curve2 = c2.into();
        a.date2 = d2.into();
        a
    };
    // 3 days: not flagged on either side
    let a = with("2026-11-10", "2026-11-13", "2026-11-10", "2026-11-13");
    assert_eq!(a.curve_lag(), (None, None));
    assert!(!a.flags().contains("curve"));
    // 4 days at D1 only
    let a = with("2026-11-09", "2026-11-13", "2026-11-13", "2026-11-13");
    assert_eq!(a.curve_lag(), (Some(4), None));
    assert!(a.flags().contains("curve1-old") && !a.flags().contains("curve2-old"));
    // 4 days at D2 only
    let a = with("2026-11-13", "2026-11-13", "2026-11-09", "2026-11-13");
    assert_eq!(a.curve_lag(), (None, Some(4)));
    assert!(a.flags().contains("curve2-old") && !a.flags().contains("curve1-old"));
    // a missing curve
    let a = with("", "2026-11-13", "2026-11-13", "2026-11-13");
    assert!(a.flags().contains("curve1-none"));
}

#[test]
fn the_list_text_names_the_tickers_with_no_model_value_and_a_missing_cik() {
    let mut rows = vec![row("A", 10.0, 8.0, 3, 10.0, 0.05)];
    rows[0].cik = Some("1".into());
    rows.push(AttrRow {
        ticker: "V".into(),
        sector: "Money".into(),
        industry: String::new(),
        prices: Some((100.0, 99.0)),
        shares: Some(10.0),
        shares_note: None,
        cik: None,
        weight_of: None,
        outcome: Err(("template bank: no model".into(), String::new())),
        earnings_8k: String::new(),
        stale: None,
    });
    let w = whole_list(&rows).unwrap();
    assert_eq!(w.no_model.len(), 1);
    assert_eq!(w.no_model[0].0, "V");
    assert!((w.no_model[0].1 - 1000.0 / 1100.0).abs() < 1e-12);
    assert_eq!(w.no_cik, vec!["V"]);
    let text = list_text(&rows);
    assert!(text.contains("no CIK in the cache") && text.contains("no model value") && text.contains("V 90.91%"), "{text}");
    // a set with no tickers outside it prints "none", not NaN
    assert!(!text.contains("NaN"), "{text}");
}

#[test]
fn the_full_bridge_puts_each_ticker_in_once_and_adds_to_the_whole_change() {
    // A (value 100, -20%, all in the plug), B (value 300, -10%, all in the rates), C a bank (value 600, -5%, no model).
    // Whole: (-20 - 30 - 30) / 1000 = -8 points. Premium first: premium -2, rates -3, no model -3.
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
        earnings_8k: String::new(),
        stale: None,
    });
    let b = full_bridge(&rows, View::Premium).unwrap();
    let p = b.all.parts;
    assert!((p[3] + 0.02).abs() < 1e-12 && (p[2] + 0.03).abs() < 1e-12 && (p[6] + 0.03).abs() < 1e-12 && p[4].abs() < 1e-12, "{p:?}");
    assert!((p.iter().sum::<f64>() + 0.08).abs() < 1e-12);
    assert_eq!((b.n_view, b.all.n), ([2, 0, 1], 3));
    assert!((b.share_view[2] - 0.6).abs() < 1e-12);
    // A has no premium view: with the premium view first it is in the growth line, once.
    if let Ok(a) = &mut rows[0].outcome {
        a.prem = Err("none".into());
    }
    let b = full_bridge(&rows, View::Premium).unwrap();
    assert!((b.all.parts[4] + 0.02).abs() < 1e-12 && b.all.parts[3].abs() < 1e-12, "{:?}", b.all.parts);
    assert_eq!(b.n_view, [1, 1, 1]);
    // the sectors add to the whole list, and the largest negative contribution comes first
    let mut sum = [0.0; N_BRIDGE];
    for l in &b.sectors {
        for (s, p) in sum.iter_mut().zip(l.parts) {
            *s += p;
        }
    }
    assert!((0..N_BRIDGE).all(|k| (sum[k] - b.all.parts[k]).abs() < 1e-12));
    // B and C both give -3 points; A gives -2 points and comes last
    let names: Vec<&str> = b.top.iter().map(|t| t.0.as_str()).collect();
    assert!(names[..2].contains(&"B") && names[..2].contains(&"C") && names[2] == "A", "{names:?}");
    assert!((b.top[0].3 + 0.03).abs() < 1e-12);
    // a second class of one company carries no weight and is not in the bridge
    let mut d = row("D", 10.0, 5.0, 3, 10.0, 0.05);
    d.weight_of = Some("A".into());
    rows.push(d);
    let b = full_bridge(&rows, View::Premium).unwrap();
    assert_eq!(b.all.n, 3);
    // one line for each weighted ticker; the lines add to the whole list, part by part
    assert_eq!(b.tickers.len(), 3);
    assert!((0..N_BRIDGE).all(|k| (b.tickers.iter().map(|l| l.parts[k]).sum::<f64>() - b.all.parts[k]).abs() < 1e-12));
    let a = b.tickers.iter().find(|l| l.name == "A").unwrap();
    assert!((a.share - 0.1).abs() < 1e-12 && (a.change + 0.2).abs() < 1e-12 && (a.parts[4] + 0.02).abs() < 1e-12, "{a:?}");
    assert!(list_text(&rows).contains("\n  A    "), "a short list prints one line for each ticker");
}

#[test]
fn a_close_older_than_the_index_close_takes_the_row_out_of_every_sum() {
    // SPY has closes on 11-13 and 11-20. A has both; B stops at 11-18, so its price for 11-20 is of an earlier day.
    let series = |t: &str| -> Option<Vec<(String, f64)>> {
        let s = |v: &[(&str, f64)]| v.iter().map(|(d, c)| (d.to_string(), *c)).collect::<Vec<_>>();
        match t {
            "SPY" => Some(s(&[("2026-11-13", 750.0), ("2026-11-20", 690.0)])),
            "A" => Some(s(&[("2026-11-13", 10.0), ("2026-11-20", 8.0)])),
            "B" => Some(s(&[("2026-11-13", 30.0), ("2026-11-18", 27.0)])),
            _ => None,
        }
    };
    let mut rows = vec![row("A", 10.0, 8.0, 3, 10.0, 0.05), row("B", 30.0, 27.0, 2, 10.0, 0.05), row("C", 5.0, 4.0, 3, 10.0, 0.05)];
    assert_eq!(mark_stale(&mut rows, &series, "2026-11-13", "2026-11-20"), 1);
    assert_eq!(rows[1].stale.as_deref(), Some("close of 2026-11-18 for 2026-11-20 (SPY 2026-11-20)"));
    assert!(rows[0].stale.is_none() && rows[2].stale.is_none(), "a ticker with no series is not marked");
    assert!(rows[1].weight().is_none() && rows[1].weight_raw().is_none());
    // the whole list and the bridge leave B out: A (value 100, -20%) and C (value 50, -20%) only
    let w = whole_list(&rows).unwrap();
    assert_eq!(w.n, 2);
    assert!((w.change + 0.2).abs() < 1e-12, "{w:?}");
    assert_eq!(full_bridge(&rows, View::Premium).unwrap().all.n, 2);
    let text = list_text(&rows);
    assert!(text.contains("STALE PRICE (1 tickers") && text.contains("own parts of these rows use the older price; each is out of every sum: B ("), "{text}");
    assert!(text.contains("  left out of the sums (1 tickers): B (stale price)\n"), "{text}");
    assert!(csv_line(&rows[1]).contains("price-stale(close of 2026-11-18 for 2026-11-20 (SPY 2026-11-20))"));
}

#[test]
fn the_whole_list_names_the_tickers_left_out_of_the_sums_with_the_reason() {
    let mut rows = vec![row("A", 10.0, 8.0, 3, 10.0, 0.05), row("BRK-B", 10.0, 8.0, 3, 10.0, 0.05), row("N", 10.0, 8.0, 3, 10.0, 0.05), row("G2", 10.0, 8.0, 3, 10.0, 0.05)];
    rows[1].shares = None;
    rows[2].prices = None;
    rows[3].weight_of = Some("A".into());
    let text = list_text(&rows);
    // the second class of one company has its own line already; it is not named here
    assert!(text.contains("  left out of the sums (2 tickers): BRK-B (no share count), N (no close at a date)\n"), "{text}");
    // the line is in the WHOLE LIST block, before the first set
    assert!(text.find("WHOLE LIST").unwrap() < text.find("left out of the sums").unwrap() && text.find("left out of the sums").unwrap() < text.find("(2) COMMON SET").unwrap());
    // a list with every ticker in has no such line
    assert!(!list_text(&[row("A", 10.0, 8.0, 3, 10.0, 0.05)]).contains("left out of the sums"));
}

#[test]
fn the_curve_text_is_the_old_text_for_gsw_and_names_the_fit_for_the_others() {
    assert_eq!(curve_sentence("gsw"), "");
    assert!(curve_sentence("fedinvest").contains("FedInvest") && !curve_sentence("fedinvest").contains("par row"));
    assert!(curve_sentence("par").contains("Treasury par row") && curve_sentence("par").contains("beyond 30 years"));
    let c = CurveNumbers { date: "2025-12-31".into(), zero10: 0.0412345, ..curve(0.0, 1.0) };
    assert_eq!(curve_line("par", &c, Some(0.4249)), "curve (par fit) 2025-12-31: 10-year zero 4.1235%, fit error 0.42 bp (root mean square over the 8 par bonds; limit 5 bp)");
    assert_eq!(curve_line("fedinvest", &c, None), "curve (fedinvest fit) 2025-12-31: 10-year zero 4.1235%");
}

#[test]
fn an_unknown_curve_value_is_an_error() {
    let args: Vec<String> = ["--curve", "ois", "--from", "2026-11-13", "--to", "2026-11-20"].iter().map(|s| s.to_string()).collect();
    let x = command_inner(Path::new("."), &args).unwrap_err();
    assert!(x.0.contains("--curve 'ois': use auto, gsw, fedinvest or par"), "{}", x.0);
}

fn dates_of(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn gsw_of(v: &[Option<&str>]) -> Vec<Option<String>> {
    v.iter().map(|s| s.map(|x| x.to_string())).collect()
}

#[test]
fn auto_picks_gsw_when_both_rows_are_fresh_and_par_when_one_is_four_days_old() {
    let dates = dates_of(&["2026-11-13", "2026-11-20"]);
    // both rows on the date, or 3 days older: gsw
    let (s, why) = auto_curve_source(&gsw_of(&[Some("2026-11-13"), Some("2026-11-17")]), &dates);
    assert_eq!(s, "gsw");
    assert!(why.starts_with("gsw, because") && why.contains("2026-11-17, 3 days older"), "{why}");
    // the row of D2 is 4 days old: par at both dates, and the text names the date and the lag
    let (s, why) = auto_curve_source(&gsw_of(&[Some("2026-11-13"), Some("2026-11-16")]), &dates);
    assert_eq!(s, "par");
    assert!(why.starts_with("par, because") && why.contains("2026-11-20: GSW row 2026-11-16, 4 days older") && why.contains("one source is used at all dates"), "{why}");
    // the file ends at D1 (the real case): the lag at D2 is 7 days
    assert_eq!(auto_curve_source(&gsw_of(&[Some("2026-11-13"), Some("2026-11-13")]), &dates).0, "par");
    // no row at a date: par
    let (s, why) = auto_curve_source(&gsw_of(&[None, Some("2026-11-20")]), &dates);
    assert_eq!((s, why.contains("2026-11-13: no GSW row")), ("par", true));
}

#[test]
fn auto_with_fresh_rows_is_the_run_of_curve_gsw_and_an_explicit_source_stays() {
    let dates = dates_of(&["2026-11-13", "2026-11-20"]);
    let fresh = gsw_of(&[Some("2026-11-12"), Some("2026-11-20")]);
    let (auto, note) = effective_source("auto", &fresh, &dates);
    let (gsw, none) = effective_source("gsw", &fresh, &dates);
    assert_eq!(auto, gsw, "the same source gives the same run");
    assert_eq!(auto, "gsw");
    // --curve gsw has no new text; auto names its choice
    assert_eq!(none, None);
    assert!(note.is_some_and(|n| n.starts_with("CURVE SOURCE (--curve auto): gsw, because")));
    // an explicit source is not changed by old rows
    let old = gsw_of(&[Some("2026-11-01"), Some("2026-11-02")]);
    assert_eq!(effective_source("gsw", &old, &dates).0, "gsw");
    assert_eq!(effective_source("fedinvest", &old, &dates).0, "fedinvest");
    assert_eq!(effective_source("auto", &old, &dates).0, "par");
}

fn spy_days(v: &[&str]) -> Vec<(String, f64)> {
    v.iter().map(|d| (d.to_string(), 100.0)).collect()
}

#[test]
fn d0_is_d1_less_the_days_of_the_window_and_moves_back_to_a_trading_day() {
    // window 2026-11-13 to 2026-11-20 is 7 days: D0 = 2026-11-06 (a Friday with a close)
    let spy = spy_days(&["2026-11-05", "2026-11-06", "2026-11-13", "2026-11-20"]);
    assert_eq!(prior_date(&spy, "2026-11-13", "2026-11-20").unwrap(), "2026-11-06");
    // Good Friday 2026-04-03 has no close: D1 2026-04-10, D2 2026-04-17, D0 = 2026-04-03 moves to Thursday 2026-04-02
    let spy = spy_days(&["2026-04-01", "2026-04-02", "2026-04-06", "2026-04-10", "2026-04-17"]);
    assert_eq!(prior_date(&spy, "2026-04-10", "2026-04-17").unwrap(), "2026-04-02");
    // a window of 3 days: D1 Monday 2026-11-09 to D2 Thursday 2026-11-12: D0 Friday 2026-11-06
    let spy = spy_days(&["2026-11-05", "2026-11-06", "2026-11-09", "2026-11-12"]);
    assert_eq!(prior_date(&spy, "2026-11-09", "2026-11-12").unwrap(), "2026-11-06");
    // a SPY series that is too short is an error
    assert!(prior_date(&spy_days(&["2026-11-13"]), "2026-11-13", "2026-11-20").is_err());
}

#[test]
fn the_prior_window_lines_chain_for_a_hand_made_list() {
    // two companies, 10 shares each. Prices at D0, D1, D2: A 10, 11, 12.1 (+10% twice); B 20, 19, 17.1 (-5%, -10%).
    // Value at D0 = 100 + 200 = 300; D1 = 110 + 190 = 300; D2 = 121 + 171 = 292.
    // D0 to D1: 0%; D1 to D2: 292 / 300 - 1 = -2.6667%; D0 to D2: 292 / 300 - 1 = -2.6667% (the same: D1 = D0 in value).
    let w01 = vec![row("A", 10.0, 11.0, 3, 10.0, 0.05), row("B", 20.0, 19.0, 3, 10.0, 0.05)];
    let w12 = vec![row("A", 11.0, 12.1, 3, 10.0, 0.05), row("B", 19.0, 17.1, 3, 10.0, 0.05)];
    let w02 = vec![row("A", 10.0, 12.1, 3, 10.0, 0.05), row("B", 20.0, 17.1, 3, 10.0, 0.05)];
    let l = [prior_line("2026-11-06", "2026-11-13", &w01), prior_line("2026-11-13", "2026-11-20", &w12), prior_line("2026-11-06", "2026-11-20", &w02)];
    let c = |i: usize| l[i].as_ref().unwrap().change;
    assert!(c(0).abs() < 1e-12, "{}", c(0));
    assert!((c(1) - (292.0 / 300.0 - 1.0)).abs() < 1e-12);
    // D0 to D2 equals the chained change of the two windows
    assert!(((1.0 + c(0)) * (1.0 + c(1)) - 1.0 - c(2)).abs() < 1e-12, "{} {} {}", c(0), c(1), c(2));
    // the whole change is in the plug of the premium view (which = 3): the premium part equals the change of each window
    for x in l.iter().map(|x| x.as_ref().unwrap()) {
        assert!((x.parts[3] - x.change).abs() < 1e-12 && x.parts[0].abs() < 1e-12 && x.parts[2].abs() < 1e-12, "{x:?}");
        assert!((x.parts.iter().sum::<f64>() - x.change).abs() < 1e-12);
    }
    let text = prior_window_text("2026-11-06", "2026-11-13", "2026-11-20", &l);
    assert!(text.contains("(6) PRIOR WINDOW"));
    let (a, b, d) = (text.find("2026-11-06 to 2026-11-13 "), text.find("2026-11-13 to 2026-11-20 "), text.find("2026-11-06 to 2026-11-20 "));
    assert!(a.is_some() && a < b && b < d, "the three lines in the order D0 to D1, D1 to D2, D0 to D2:\n{text}");
    assert!(text.contains("2026-11-06 to 2026-11-13, then 2026-11-13 to 2026-11-20, chained: -2.67%"), "{text}");
    assert!(text.contains("-2.67% |"), "{text}");
    // the same inputs give the same text
    assert_eq!(text, prior_window_text("2026-11-06", "2026-11-13", "2026-11-20", &l));
}

#[test]
fn the_common_change_of_a_prior_line_is_the_premium_common_change() {
    let mut rows = vec![row("A", 10.0, 8.0, 3, 10.0, 0.05), row("B", 30.0, 27.0, 3, 10.0, 0.05)];
    for r in rows.iter_mut() {
        if let Ok(a) = r.outcome.as_mut() {
            if let Ok(c) = a.prem.as_mut() {
                c.split = Some(Split { c: -0.00581, label: "list median".into(), common_ln: -0.01, company_ln: 0.0, rev_common_ln: -0.01, rev_company_ln: 0.0, none: false, covered: None });
            }
        }
    }
    let l = prior_line("2026-11-13", "2026-11-20", &rows).unwrap();
    assert_eq!(l.common, Some((-0.00581, "list median".to_string())));
    let text = prior_window_text("2026-11-06", "2026-11-13", "2026-11-20", &[None, Some(l), None]);
    assert!(text.contains("-0.581"), "{text}");
    assert!(text.contains("source of c: list median."), "{text}");
    assert!(text.contains("2026-11-06 to 2026-11-13: no ticker has a weight"), "{text}");
}

#[test]
fn the_two_readings_of_the_full_bridge_give_the_same_total() {
    let rows = vec![row("A", 10.0, 8.0, 3, 10.0, 0.05), row("B", 30.0, 27.0, 2, 10.0, 0.05)];
    // whole change: (100 x -0.2 + 300 x -0.1) / 400 = -0.125
    let whole = whole_list(&rows).unwrap().change;
    assert!((whole + 0.125).abs() < 1e-12);
    for first in [View::Premium, View::Growth] {
        let b = full_bridge(&rows, first).unwrap();
        assert!((b.all.parts.iter().sum::<f64>() - whole).abs() < 1e-12, "{first:?}: {:?}", b.all.parts);
    }
}

#[test]
fn a_list_of_21_weighted_tickers_prints_no_ticker_lines_and_a_list_of_20_does() {
    let rows_of = |n: usize| -> Vec<AttrRow> { (0..n).map(|i| row(&format!("T{i:02}"), 10.0, 8.0, 3, 10.0, 0.05)).collect() };
    let text20 = bridge_text(&full_bridge(&rows_of(BRIDGE_TICKER_LINES), View::Premium).unwrap());
    let text21 = bridge_text(&full_bridge(&rows_of(BRIDGE_TICKER_LINES + 1), View::Premium).unwrap());
    assert_eq!(BRIDGE_TICKER_LINES, 20);
    assert!(text20.contains("\n  T00 ") && text20.contains("\n  T19 "), "{text20}");
    assert!(!text21.contains("\n  T00 ") && !text21.contains("\n  T20 "), "{text21}");
}

#[test]
fn the_premium_duration_is_the_central_difference_of_ln_value() {
    let t = table();
    let model = rules_w1().erp.0;
    let (h, x) = (DURATION_STEP, 0.04);
    let m = mk(1.0, None);
    // by hand: D = -(ln V(x + h) - ln V(x - h)) / (2 h), with the two values of the same engine at x +- h
    let (up, down) = (value(&inputs(), &m, x + h), value(&inputs(), &m, x - h));
    let by_hand = -(up.ln() - down.ln()) / (2.0 * h);
    let d = premium_duration(&inputs(), &cfg(&t), &m, x, h).unwrap();
    assert!((d - by_hand).abs() < 1e-12, "{d} {by_hand}");
    assert!(d > 0.0, "the value falls with the premium: {d}");
    // a price that is the value at the model premium: the attribution gives the duration at that premium
    let p1 = value(&inputs(), &m, model);
    let a = attribute_models(&inputs(), &mk(p1, None), &inputs(), &mk(p1 * 0.9, None), &cfg(&t)).unwrap();
    let by_hand = -(value(&inputs(), &m, model + h).ln() - value(&inputs(), &m, model - h).ln()) / (2.0 * h);
    assert!((a.dur_premium1.unwrap() - by_hand).abs() < 1e-6, "{:?} {by_hand}", a.dur_premium1);
    // a step far outside the range has no value: no duration
    assert!(premium_duration(&inputs(), &cfg(&t), &m, -5.0, h).is_none() || premium_duration(&inputs(), &cfg(&t), &m, 50.0, h).is_none());
}

#[test]
fn the_betas_of_the_two_dates_are_the_adjusted_betas_or_empty() {
    let t = table();
    let model = rules_w1().erp.0;
    let p1 = value(&inputs(), &mk(1.0, None), model);
    let fit = |adjusted: f64| BetaFit { raw: 0.0, adjusted, weeks: 260, first: "a".into(), last: "b".into(), market: "SPY".into() };
    let (mut m1, mut m2) = (mk(p1, None), mk(p1, None));
    let a = attribute_models(&inputs(), &m1, &inputs(), &m2, &cfg(&t)).unwrap();
    assert!(a.beta1.is_none() && a.beta2.is_none());
    m1.beta = Some(fit(1.25));
    m2.beta = Some(fit(1.5));
    let a = attribute_models(&inputs(), &m1, &inputs(), &m2, &cfg(&t)).unwrap();
    assert_eq!((a.beta1, a.beta2), (Some(1.25), Some(1.5)));
}

fn cache_with_8k(name: &str, filed: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fa_attr_8k_{}_{name}", std::process::id()));
    std::fs::create_dir_all(dir.join("ACME")).unwrap();
    let list: Vec<String> = filed.iter().map(|(d, i)| format!(r#"{{"filed":"{d}","items":"{i}"}}"#)).collect();
    std::fs::write(dir.join("ACME/sec_8k_earnings.json"), format!(r#"{{"filings":[{}]}}"#, list.join(","))).unwrap();
    dir
}

#[test]
fn the_8k_field_has_the_filings_after_d1_and_on_or_before_d2() {
    let dir = cache_with_8k(
        "window",
        &[("2026-11-13", "2.02,9.01"), ("2026-11-17", "2.02,9.01"), ("2026-11-20", "2.02"), ("2026-11-21", "2.02"), ("2026-11-18", "5.02"), ("2026-11-19", "12.02")],
    );
    // D1 itself is out; the inside and D2 are in; after D2 is out; an item other than 2.02 is out (12.02 is not 2.02)
    assert_eq!(earnings_8k_between(&dir, "ACME", "2026-11-13", "2026-11-20"), "2026-11-17;2026-11-20");
    assert_eq!(earnings_8k_between(&dir, "ACME", "2026-11-20", "2026-11-21"), "2026-11-21");
    assert_eq!(earnings_8k_between(&dir, "ACME", "2026-11-21", "2026-11-30"), "");
    assert_eq!(earnings_8k_between(&dir, "NONE", "2026-11-13", "2026-11-20"), "");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn the_csv_header_ends_with_the_four_new_columns_and_the_old_ones_keep_their_places() {
    assert!(CSV_HEADER.contains(",refusal,beta1,beta2,dur_premium1,earnings_8k,"));
    assert!(CSV_HEADER.ends_with(",earnings_8k,c_premium,c_growth,p_premium_common_pct,p_premium_company_pct,g_growth_common_pct,g_growth_company_pct"));
    assert!(CSV_HEADER.starts_with("ticker,size,sector,industry,price1,price2,change_pct,"));
    assert_eq!(CSV_HEADER.split(',').position(|h| h == "flags"), Some(34));
    let mut r = row("A", 10.0, 8.0, 3, 10.0, 0.05);
    r.earnings_8k = "2026-11-17;2026-11-19".into();
    if let Ok(a) = r.outcome.as_mut() {
        a.beta1 = Some(1.1);
        a.dur_premium1 = Some(21.5);
    }
    let line = csv_line(&r);
    let cells: Vec<&str> = line.split(',').collect();
    assert_eq!(cells.len(), CSV_HEADER.split(',').count());
    let k = CSV_HEADER.split(',').position(|h| h == "beta1").unwrap();
    assert_eq!(&cells[k..k + 4], ["1.1000", "", "21.500", "2026-11-17;2026-11-19"]);
    // no split yet: the six new cells are empty
    assert!(cells[cells.len() - 6..].iter().all(|c| c.is_empty()));
}

#[test]
fn the_premium_series_csv_header_is_unchanged() {
    assert_eq!(super::super::auto_premium::HEADER, "friday,anchor,curve_source,curve_date,zero10,s1,n_value,n_list,ret_w,n_ret,s2_facts,s2_time,s2_rates,s2_premium,s2_growth,s2_residual,s2_nomodel,s2_change");
}

// ---------------------------------------------------------------------------------------------
// The common change and the company part
// ---------------------------------------------------------------------------------------------

fn engine_row(a: Attribution) -> AttrRow {
    AttrRow { ticker: "A".into(), sector: "S".into(), industry: String::new(), prices: Some((a.p1, a.p2)), shares: Some(10.0), shares_note: None, cik: None, weight_of: None, outcome: Ok(a), earnings_8k: String::new(), stale: None }
}

fn change(c: Option<f64>) -> CommonChange {
    CommonChange { c, label: "list median".into() }
}

/// A price that moves from the value at the model premium to the value at 3%, with the engine of the tests.
fn moved_row(t: &super::super::calibrate::ProcessTable) -> (Vec<AttrRow>, f64) {
    let model = rules_w1().erp.0;
    let (p1, p2) = (value(&inputs(), &mk(1.0, None), model), value(&inputs(), &mk(1.0, None), 0.03));
    let a = attribute_models(&inputs(), &mk(p1, None), &inputs(), &mk(p2, None), &cfg(t)).unwrap();
    (vec![engine_row(a)], model)
}

#[test]
fn the_common_step_and_the_company_part_add_to_the_plug_with_closed_answers() {
    let t = table();
    let (mut rows, model) = moved_row(&t);
    let inp = vec![inputs()];
    let (m1, m2) = (|_: &str| mk(1.0, None), |_: &str| mk(1.0, None));
    let c = -0.01;
    let changes = [change(Some(c)), change(Some(0.01))];
    add_common(&mut rows[..], &inp, &m1, &m2, &cfg(&t), &changes, Some(1)).unwrap();
    let a = rows[0].outcome.as_ref().unwrap();
    let ch = prem(a);
    let sp = ch.split.as_ref().unwrap();
    // premium view: the common step is ln V(x1 + c) - ln V(x1) of the same engine (closed: the price is the value at x1)
    let by_hand = (value(&inputs(), &mk(1.0, None), model + c) / value(&inputs(), &mk(1.0, None), model)).ln();
    assert!((sp.common_ln - by_hand).abs() < 1e-6 && sp.common_ln > 0.0, "{sp:?} {by_hand}");
    // the company step goes from x1 + c to x2 = 3%
    let company = (value(&inputs(), &mk(1.0, None), 0.03) / value(&inputs(), &mk(1.0, None), model + c)).ln();
    assert!((sp.company_ln - company).abs() < 1e-6, "{sp:?} {company}");
    assert!((sp.common_ln + sp.company_ln - ch.free_ln).abs() < 1e-12);
    assert!(!sp.none && ch.flags("premium").iter().all(|f| !f.contains("common-step")));
    // the rates and the curve do not move, so the reverse order gives the same two parts
    assert!((sp.rev_common_ln - sp.common_ln).abs() < 1e-9 && (sp.rev_common_ln + sp.rev_company_ln - ch.rev_free_ln).abs() < 1e-12);
    // the percent parts: the two plug parts add to the plug of `pct`, in both orders
    let (p, q) = (ch.pct_split(), ch.pct_rev_split());
    assert!((p[3] + p[4] - ch.pct()[3]).abs() < 1e-12 && (q[3] + q[4] - ch.pct_rev()[3]).abs() < 1e-12);
    assert!((p.iter().sum::<f64>() - (a.p2 / a.p1 - 1.0)).abs() < 1e-12);
    // the growth view: the same with the growth shift
    let g = grow(a);
    let gs = g.split.as_ref().unwrap();
    let ct = cfg(&t);
    let v = |x: f64| value_with_growth(&inputs(), &ct, &mk(1.0, None), x).unwrap();
    assert!((gs.common_ln - (v(g.x1.x + 0.01) / v(g.x1.x)).ln()).abs() < 1e-9, "{gs:?}");
    assert!((gs.common_ln + gs.company_ln - g.free_ln).abs() < 1e-12);
    // the CSV has the parts, and they add to the old columns
    let line = csv_line(&rows[0]);
    let cell = |name: &str| -> f64 { line.split(',').nth(CSV_HEADER.split(',').position(|h| h == name).unwrap()).unwrap().parse().unwrap() };
    assert!((cell("p_premium_common_pct") + cell("p_premium_company_pct") - cell("p_premium_pct")).abs() < 2e-3);
    assert!((cell("g_growth_common_pct") + cell("g_growth_company_pct") - cell("g_growth_pct")).abs() < 2e-3);
    assert_eq!((cell("c_premium"), cell("c_growth")), (-0.01, 0.01));
}

#[test]
fn with_c_zero_the_company_part_is_the_plug_and_no_c_is_zero() {
    let t = table();
    let (mut rows, _) = moved_row(&t);
    let inp = vec![inputs()];
    let (m1, m2) = (|_: &str| mk(1.0, None), |_: &str| mk(1.0, None));
    add_common(&mut rows[..], &inp, &m1, &m2, &cfg(&t), &no_common("one ticker and no --common-list"), None).unwrap();
    let a = rows[0].outcome.as_ref().unwrap();
    for ch in [prem(a), grow(a)] {
        let sp = ch.split.as_ref().unwrap();
        assert_eq!((sp.c, sp.common_ln, sp.rev_common_ln), (0.0, 0.0, 0.0));
        assert_eq!((sp.company_ln, sp.rev_company_ln), (ch.free_ln, ch.rev_free_ln));
        assert!(sp.label.contains("c = 0") && !sp.none);
    }
    // an empty text of the source says so on the headline
    assert!(common_label(&CommonChange::none("x")).starts_with("none, c = 0"));
}

#[test]
fn a_value_that_the_engine_cannot_give_at_x1_plus_c_puts_the_whole_plug_in_the_company_part() {
    let t = table();
    let (mut rows, _) = moved_row(&t);
    let inp = vec![inputs()];
    let (m1, m2) = (|_: &str| mk(1.0, None), |_: &str| mk(1.0, None));
    add_common(&mut rows[..], &inp, &m1, &m2, &cfg(&t), &[change(Some(-5.0)), change(Some(0.0))], Some(1)).unwrap();
    let a = rows[0].outcome.as_ref().unwrap();
    let ch = prem(a);
    let sp = ch.split.as_ref().unwrap();
    assert!(sp.none && sp.common_ln == 0.0 && sp.company_ln == ch.free_ln, "{sp:?}");
    assert!(a.flags().contains("premium:common-step:none"), "{}", a.flags());
}

#[test]
fn the_median_and_the_value_weighted_c_of_a_hand_made_list() {
    // x2 is 5% in each row; x1 is 4%, 3% and 5%: x2 - x1 is 1, 2 and 0 points. Median: 1 point.
    // Weights (price x shares): A 10 x 10 = 100, B 10 x 30 = 300, C 10 x 10 = 100. Value-weighted: (100 + 600 + 0) / 500 = 1.4 points.
    let rows = vec![row("A", 10.0, 8.0, 3, 10.0, 0.04), row("B", 10.0, 8.0, 3, 30.0, 0.03), row("C", 10.0, 8.0, 3, 10.0, 0.05)];
    let m = common_from_rows(&rows, CommonMode::Median);
    assert!((m[0].c.unwrap() - 0.01).abs() < 1e-12 && m[0].label == "list median");
    let v = common_from_rows(&rows, CommonMode::ValueWeighted);
    assert!((v[1].c.unwrap() - 0.014).abs() < 1e-12 && v[1].label == "value-weighted");
    // a stale row is left out: without B the median of 1 and 0 is 0.5 points
    let mut stale = rows.clone();
    stale[1].stale = Some("old".into());
    assert!((common_from_rows(&stale, CommonMode::Median)[0].c.unwrap() - 0.005).abs() < 1e-12);
    // no row, no c
    assert!(common_from_rows(&[], CommonMode::Median)[0].c.is_none());
}

#[test]
fn the_file_source_gives_the_row_of_d2_and_an_empty_field_is_no_information() {
    let text = "date,erp_change,growth_change,source\n2026-11-13,0.001,0.002,old\n2026-11-20,-0.0025,,economics v1\n";
    let c = econ_from_text(text, "2026-11-13", "2026-11-20").unwrap();
    assert_eq!((c.erp, c.growth, c.source.as_str()), (Some(-0.0025), None, "economics v1"));
    assert!(c.label(true).starts_with("economics view, economics v1") && c.label(true).contains("erp_change only"), "{}", c.label(true));
    assert!(econ_from_text(text, "2026-11-13", "2026-11-19").is_err());
    assert!(econ_from_text("date,erp\n", "2026-11-13", "2026-11-20").is_err());
    assert!(econ_from_text("date,erp_change,growth_change,source\n2026-11-20,abc,,x\n", "2026-11-13", "2026-11-20").is_err());
    let empty = econ_from_text("date,erp_change,growth_change,source\n2026-11-20,,,test only\n", "2026-11-13", "2026-11-20").unwrap();
    assert_eq!((empty.erp, empty.growth), (None, None));
    assert!(empty.label(true).contains("not covered"), "{}", empty.label(true));
    // the file of `fa macro view`: more columns, in a different order; with a column d1 the row must also match D1
    let view = "date,d1,erp_change,growth_change,source,growth_parts\n2026-11-20,2026-11-06,0.009,0.001,spf,other window\n2026-11-20,2026-11-13,0.004,-0.001,spf,RGDP10 2.0 + CPI10 2.3\n2026-03-20,2026-03-13,,,no release,\n";
    let v = econ_from_text(view, "2026-11-13", "2026-11-20").unwrap();
    assert_eq!((v.erp, v.growth, v.source.as_str()), (Some(0.004), Some(-0.001), "spf"));
    let none = econ_from_text(view, "2026-03-13", "2026-03-20").unwrap();
    assert_eq!((none.erp, none.growth, none.source.as_str()), (None, None, "no release"));
    assert!(econ_from_text(view, "2026-11-12", "2026-11-20").is_err(), "a row of another D1 is not the row of the window");
}

fn econ(erp: Option<f64>, growth: Option<f64>) -> EconView {
    EconView { erp, growth, source: "test only".into() }
}

/// The value at the means for a premium, a growth shift and a G, at the plain market (the default rate 4.2% or a curve).
fn v_rules(m: &Market, erp: f64, shift: f64, g: f64) -> f64 {
    let t = table();
    value_at_rules(&inputs(), &cfg(&t), m, erp, shift, Some(g)).unwrap()
}

#[test]
fn a_change_of_g_moves_the_value_only_where_g_binds() {
    let model = rules_w1().erp.0;
    // no curve: the default rate is 4.2%, so G = 4.0% is below it: L = G, and a lower G gives a lower value
    let m_g = mk(1.0, None);
    let (hi, lo) = (v_rules(&m_g, model, 0.0, 0.040), v_rules(&m_g, model, 0.0, 0.035));
    assert!((lo / hi).ln() < -1e-4, "{lo} {hi}");
    // a 10-year zero of 3.0% is below G: L = the yield, and G from 3.5% to 4.5% does not move the value
    let m_y = mk(1.0, Some(curve(0.03, 4.0)));
    let base = v_rules(&m_y, model, 0.0, 0.040);
    for g in [0.035, 0.045] {
        assert!((v_rules(&m_y, model, 0.0, g) / base - 1.0).abs() < 1e-12, "G {g}");
    }
}

#[test]
fn both_changes_together_equal_the_direct_value_in_both_views_and_the_parts_add() {
    let t = table();
    let ct = cfg(&t);
    let (mut rows, model) = moved_row(&t);
    let inp = vec![inputs()];
    let (m1, m2) = (|_: &str| mk(1.0, None), |_: &str| mk(1.0, None));
    let (d_erp, d_g) = (0.004, -0.001);
    add_common_economics(&mut rows[..], &inp, &m1, &m2, &ct, &econ(Some(d_erp), Some(d_g)), Some(1)).unwrap();
    let a = rows[0].outcome.as_ref().unwrap();
    let m = mk(1.0, None);
    let g0 = ct.rules.long_run.unwrap();
    // premium view: from x1, the premium + erp_change and G + growth_change together (G binds: 4.0% < 4.2%)
    let (p, g) = (prem(a), grow(a));
    let sp = p.split.as_ref().unwrap();
    let direct = (v_rules(&m, p.x1.x + d_erp, 0.0, g0 + d_g) / v_rules(&m, p.x1.x, 0.0, g0)).ln();
    let erp_only = (v_rules(&m, p.x1.x + d_erp, 0.0, g0) / v_rules(&m, p.x1.x, 0.0, g0)).ln();
    assert!((sp.common_ln - direct).abs() < 1e-9, "{} {direct}", sp.common_ln);
    assert!((sp.common_ln - erp_only).abs() > 1e-4, "G must move the common step: {} {erp_only}", sp.common_ln);
    assert_eq!((sp.c, sp.covered, sp.none), (d_erp, Some(true), false));
    assert!(sp.label.starts_with("economics view, test only"), "{}", sp.label);
    // growth view: the model premium moves by erp_change, the growth shift stays at x1 of D1
    let gs = g.split.as_ref().unwrap();
    let direct_g = (v_rules(&m, model + d_erp, g.x1.x, g0 + d_g) / v_rules(&m, model, g.x1.x, g0)).ln();
    assert!((gs.common_ln - direct_g).abs() < 1e-9, "{} {direct_g}", gs.common_ln);
    assert_eq!(gs.c, d_g);
    // common + company = plug, in both views and both orders (the rates do not move, so the orders agree)
    for ch in [p, g] {
        let s = ch.split.as_ref().unwrap();
        assert!((s.common_ln + s.company_ln - ch.free_ln).abs() < 1e-12);
        assert!((s.rev_common_ln + s.rev_company_ln - ch.rev_free_ln).abs() < 1e-12);
        let (u, w) = (ch.pct_split(), ch.pct_rev_split());
        assert!((u[3] + u[4] - ch.pct()[3]).abs() < 1e-12 && (w[3] + w[4] - ch.pct_rev()[3]).abs() < 1e-12);
        assert!((u.iter().sum::<f64>() - (a.p2 / a.p1 - 1.0)).abs() < 1e-12);
    }
}

#[test]
fn a_change_of_g_has_no_effect_in_the_common_step_where_the_yield_binds() {
    let t = table();
    let ct = cfg(&t);
    // the curve gives a 10-year zero of 3.0% at both dates: G does not bind, so growth_change adds nothing to erp_change
    let m = |_: &str| mk(1.0, Some(curve(0.03, 4.0)));
    let (p1, p2) = (v_rules(&m(""), rules_w1().erp.0, 0.0, 0.04), v_rules(&m(""), 0.03, 0.0, 0.04));
    let a = attribute_models(&inputs(), &mk(p1, Some(curve(0.03, 4.0))), &inputs(), &mk(p2, Some(curve(0.03, 4.0))), &ct).unwrap();
    let mut rows = [engine_row(a)];
    add_common_economics(&mut rows[..], &[inputs()], &m, &m, &ct, &econ(Some(0.004), Some(-0.003)), Some(1)).unwrap();
    let s = prem(rows[0].outcome.as_ref().unwrap()).split.as_ref().unwrap().clone();
    let x1 = prem(rows[0].outcome.as_ref().unwrap()).x1.x;
    let erp_only = (v_rules(&m(""), x1 + 0.004, 0.0, 0.04) / v_rules(&m(""), x1, 0.0, 0.04)).ln();
    assert!((s.common_ln - erp_only).abs() < 1e-12, "{} {erp_only}", s.common_ln);
}

#[test]
fn empty_fields_are_not_covered_and_the_company_part_is_the_whole_plug() {
    let t = table();
    let ct = cfg(&t);
    let (mut rows, _) = moved_row(&t);
    let (mut second, _) = moved_row(&t);
    second[0].ticker = "B".into();
    rows.append(&mut second);
    let inp = vec![inputs(), inputs()];
    let (m1, m2) = (|_: &str| mk(1.0, None), |_: &str| mk(1.0, None));
    add_common_economics(&mut rows[..], &inp, &m1, &m2, &ct, &econ(None, None), Some(1)).unwrap();
    for r in &rows {
        let a = r.outcome.as_ref().unwrap();
        for (ch, name) in [(prem(a), "premium"), (grow(a), "growth")] {
            let s = ch.split.as_ref().unwrap();
            assert_eq!((s.common_ln, s.rev_common_ln, s.covered, s.none), (0.0, 0.0, Some(false), false));
            assert_eq!((s.company_ln, s.rev_company_ln), (ch.free_ln, ch.rev_free_ln));
            assert!(s.label.contains("not covered") && s.label.starts_with("economics view, test only"), "{}", s.label);
            assert!(ch.flags(name).iter().any(|f| f.contains("not-covered-by-economics-view")));
        }
    }
    assert_eq!(economics_cover(rows.iter()), Some((0, 2)));
    let text = list_text(&rows);
    assert!(text.contains("economics view: 0 tickers covered, 2 tickers not covered"), "{text}");
    assert!(econ_line(&econ(None, None), true, economics_cover(rows.iter())).contains("not covered (company part = the whole plug): 2 tickers"));
    // a median run has no economics view: no count
    assert_eq!(economics_cover(std::iter::empty()), None);
}

#[test]
fn a_file_with_only_erp_change_applies_it_and_says_so() {
    let t = table();
    let ct = cfg(&t);
    let (mut rows, _) = moved_row(&t);
    let inp = vec![inputs()];
    let (m1, m2) = (|_: &str| mk(1.0, None), |_: &str| mk(1.0, None));
    add_common_economics(&mut rows[..], &inp, &m1, &m2, &ct, &econ(Some(0.004), None), Some(1)).unwrap();
    let a = rows[0].outcome.as_ref().unwrap();
    let (p, g) = (prem(a), grow(a));
    let m = mk(1.0, None);
    let g0 = ct.rules.long_run.unwrap();
    let sp = p.split.as_ref().unwrap();
    // G stays: the common step is the premium step alone
    assert!((sp.common_ln - (v_rules(&m, p.x1.x + 0.004, 0.0, g0) / v_rules(&m, p.x1.x, 0.0, g0)).ln()).abs() < 1e-9);
    assert_eq!(sp.covered, Some(true));
    assert!(sp.label.contains("erp_change only") && sp.label.contains("growth_change empty"), "{}", sp.label);
    // the growth view: the premium of the model + erp_change at the shift x1; its own field (growth_change) is empty, so c = 0
    let gs = g.split.as_ref().unwrap();
    let model = ct.rules.erp.0;
    assert!((gs.common_ln - (v_rules(&m, model + 0.004, g.x1.x, g0) / v_rules(&m, model, g.x1.x, g0)).ln()).abs() < 1e-9);
    assert_eq!(gs.c, 0.0);
    assert!((gs.common_ln + gs.company_ln - g.free_ln).abs() < 1e-12);
    // a file with only growth_change: G applies, the premium does not move
    let (mut rows2, _) = moved_row(&t);
    add_common_economics(&mut rows2[..], &inp, &m1, &m2, &ct, &econ(None, Some(-0.001)), Some(1)).unwrap();
    let a2 = rows2[0].outcome.as_ref().unwrap();
    let s2 = prem(a2).split.as_ref().unwrap();
    let x1 = prem(a2).x1.x;
    assert!((s2.common_ln - (v_rules(&m, x1, 0.0, g0 - 0.001) / v_rules(&m, x1, 0.0, g0)).ln()).abs() < 1e-9 && s2.common_ln.abs() > 1e-5);
    assert!(s2.label.contains("growth_change only"), "{}", s2.label);
}

#[test]
fn the_8k_group_line_is_the_sum_of_the_lines_of_its_tickers() {
    let mut rows = vec![row("A", 10.0, 8.0, 3, 10.0, 0.05), row("B", 30.0, 27.0, 2, 10.0, 0.05), row("C", 20.0, 22.0, 3, 5.0, 0.05)];
    rows[0].earnings_8k = "2026-11-17".into();
    rows[2].earnings_8k = "2026-11-19".into();
    for (i, r) in rows.iter_mut().enumerate() {
        if let Ok(a) = r.outcome.as_mut() {
            if let Ok(c) = a.prem.as_mut() {
                let (common, company) = (c.free_ln * 0.25, c.free_ln * 0.75);
                let (rc, rcc) = (c.rev_free_ln * 0.25, c.rev_free_ln * 0.75);
                c.split = Some(Split { c: 0.001 * (i as f64 + 1.0), label: "list median".into(), common_ln: common, company_ln: company, rev_common_ln: rc, rev_company_ln: rcc, none: false, covered: None });
            }
        }
    }
    let b = full_bridge(&rows, View::Premium).unwrap();
    let g = b.earnings.as_ref().unwrap();
    assert_eq!((g.name.as_str(), g.n, b.earnings_tickers.clone()), (EARNINGS_GROUP, 2, vec!["A".to_string(), "C".to_string()]));
    for k in 0..N_BRIDGE {
        let sum: f64 = ["A", "C"].iter().map(|n| b.tickers.iter().find(|l| l.name == *n).unwrap().parts[k]).sum();
        assert!((g.parts[k] - sum).abs() < 1e-12, "{k}");
    }
    for k in 0..4 {
        let sum: f64 = ["A", "C"].iter().map(|n| b.tickers.iter().find(|l| l.name == *n).unwrap().split[k]).sum();
        assert!((g.split[k] - sum).abs() < 1e-12, "{k}");
    }
    // the split columns add to the old plug column, on each line and on the whole list; the group is not added to the whole
    assert!((b.all.split[0] + b.all.split[1] - b.all.parts[3]).abs() < 1e-12);
    assert!(b.all.split[0].abs() > 1e-6 && b.all.split[2] == 0.0 && b.all.split[3] == 0.0);
    assert_eq!(b.all.n, 3);
    assert_eq!(b.common[0].as_ref().map(|x| x.1.as_str()), Some("list median"));
    let text = list_text(&rows);
    assert!(text.contains("premium-common") && text.contains("earnings 8-K in window") && text.contains("common change c of the premium view"), "{text}");
    // no 8-K, no group line
    rows[0].earnings_8k.clear();
    rows[2].earnings_8k.clear();
    assert!(full_bridge(&rows, View::Premium).unwrap().earnings.is_none());
}

#[test]
fn one_and_four_threads_give_the_same_rows_after_the_common_pass() {
    let t = table();
    let list = synthetic(12);
    let m1 = |_: &str| mk(30.0, Some(curve(0.042, 4.0)));
    let m2 = |_: &str| mk(27.0, Some(curve(0.045, 4.3)));
    let run = |jobs: usize| {
        let mut rows = attribute_list(&list, &list, &m1, &m2, &HashMap::new(), &cfg(&t), Some(jobs)).unwrap();
        let c = common_from_rows(&rows, CommonMode::Median);
        add_common(&mut rows, &list, &m1, &m2, &cfg(&t), &c, Some(jobs)).unwrap();
        rows
    };
    let (one, four) = (run(1), run(4));
    assert_eq!(format!("{one:?}"), format!("{four:?}"));
    assert_eq!(csv_text(&one), csv_text(&four));
    assert!(one.iter().filter_map(|r| r.outcome.as_ref().ok()).any(|a| a.prem.as_ref().is_ok_and(|c| c.split.is_some())));
}

/// A row with a CIK and a market value `price x 10`.
fn class_row(name: &str, price: f64, cik: &str) -> AttrRow {
    let mut r = row(name, price, price * 0.9, 3, 10.0, 0.05);
    r.cik = Some(cik.into());
    r
}

#[test]
fn top_keeps_the_largest_companies_and_both_classes_of_one_company() {
    // GOOG (market value 1000) and GOOGL (980, same company) carry one weight; MSFT 500; AAA 300; BBB 100
    let mut rows = vec![class_row("GOOG", 100.0, "7"), class_row("GOOGL", 98.0, "7"), class_row("MSFT", 50.0, "2"), class_row("AAA", 30.0, "3"), class_row("BBB", 10.0, "4")];
    one_weight_for_one_company(&mut rows);
    assert_eq!(rows[1].weight_of.as_deref(), Some("GOOG"));
    // the top 2 companies are GOOG and MSFT: three tickers, because the class GOOGL stays
    let keep = keep_top(&rows, 2);
    assert_eq!(keep, vec![0, 1, 2]);
    let kept: Vec<AttrRow> = keep.iter().map(|i| rows[*i].clone()).collect();
    let w = whole_list(&kept).unwrap();
    // 2 companies in the sum (the class has no weight), market value 1000 + 500
    assert_eq!(w.n, 2);
    assert!((w.value1 - 1500.0).abs() < 1e-9, "{w:?}");
    assert_eq!(w.dropped, vec![("GOOGL".to_string(), "GOOG".to_string())]);
    // the top 1 is GOOG with its class; a number above the count keeps all
    assert_eq!(keep_top(&rows, 1), vec![0, 1]);
    assert_eq!(keep_top(&rows, 10), vec![0, 1, 2, 3, 4]);
    // a stale row has no weight: it is not ranked, even when it is the largest
    rows[3].stale = Some("close of 2026-11-12 for 2026-11-13 (SPY 2026-11-13)".into());
    rows[3].prices = Some((500.0, 450.0));
    assert_eq!(keep_top(&rows, 3), vec![0, 1, 2, 4]);
}

#[test]
fn the_common_change_with_top_is_the_common_change_of_the_full_list() {
    let t = table();
    let list = synthetic(12);
    let price = |tk: &str, day: usize| -> f64 {
        let k: f64 = tk[1..].parse().unwrap();
        // D1 price grows with the index (so the largest are the last); the move to D2 differs from one ticker to the next
        (20.0 + 3.0 * k) * if day == 2 { 0.80 + 0.04 * k } else { 1.0 }
    };
    let m1 = |tk: &str| mk(price(tk, 1), Some(curve(0.042, 4.0)));
    let m2 = |tk: &str| mk(price(tk, 2), Some(curve(0.045, 4.3)));
    let mut rows = attribute_list(&list, &list, &m1, &m2, &HashMap::new(), &cfg(&t), Some(1)).unwrap();
    let full = common_from_rows(&rows, CommonMode::Median);
    add_common(&mut rows, &list, &m1, &m2, &cfg(&t), &full, Some(1)).unwrap();
    let keep = keep_top(&rows, 3);
    assert_eq!(keep.len(), 3);
    let kept: Vec<AttrRow> = keep.iter().map(|i| rows[*i].clone()).collect();
    // the kept rows hold the c of the full list in both views ...
    let c_of = |r: &AttrRow, v: View| r.outcome.as_ref().ok().and_then(|a| a.chain(v)).and_then(|c| c.split.as_ref()).map(|s| s.c);
    let mut seen = 0;
    for r in &kept {
        for (k, v) in [View::Premium, View::Growth].into_iter().enumerate() {
            if let Some(c) = c_of(r, v) {
                assert_eq!(Some(c), full[k].c, "{} {v:?}", r.ticker);
                seen += 1;
            }
        }
    }
    assert!(seen >= 3, "the kept rows must have a split");
    // ... and it is not the c of the cut list: the cut before the common change would change it
    let cut = common_from_rows(&kept, CommonMode::Median);
    assert!(full[0].c.is_some() && cut[0].c.is_some() && (full[0].c.unwrap() - cut[0].c.unwrap()).abs() > 1e-6, "full {:?} cut {:?}", full[0].c, cut[0].c);
}

#[test]
fn the_csv_with_the_reality_columns_has_the_old_columns_in_their_places() {
    let r = row("A", 10.0, 8.0, 3, 10.0, 0.05);
    let plain = csv_text(std::slice::from_ref(&r));
    let tail = "x1,x2";
    let cells = ["1,2".to_string()];
    let with = csv_text_with(std::slice::from_ref(&r), Some((tail, &cells)));
    let (p, w): (Vec<&str>, Vec<&str>) = (plain.lines().collect(), with.lines().collect());
    assert_eq!(w[0], format!("{CSV_HEADER},{tail}"));
    assert_eq!(w[1], format!("{},1,2", p[1]));
    assert_eq!(p[0], CSV_HEADER);
}
