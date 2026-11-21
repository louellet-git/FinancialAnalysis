//! Tests of the automatic model on small data made by hand (fixed answers). No network and no cache.

use super::super::auto_cli::*;
use super::super::auto_pit::*;
use super::super::calibrate::{Ar1Params, FadeCell, FadeFile, FadeStep, ProcessPriors, ProcessTable, TableMeta};
use super::super::paths::{Mods, PathEngine};
use super::super::process::{ParamSpec, PathsSpec, StartSpec};
use super::*;

fn ar1(phi: f64, mean: f64, tau: f64, sigma: f64) -> Ar1Params {
    Ar1Params {
        phi,
        phi_se: 0.02,
        half_life: 1.0,
        mean,
        mean_se: 0.01,
        mean_sd_units: tau,
        sigma,
        n_obs: 1000,
        n_units: 100,
        phi_pooled: phi,
        phi_pooled_se: 0.0,
        phi_fe: phi,
        phi_fe_se: 0.0,
        phi_ah: None,
        phi_ah_se: None,
        sigma_pooled: sigma,
        mean_pooled: mean,
    }
}

fn cell(size: &str, template: &str, g: Ar1Params, m: Ar1Params) -> ProcessPriors {
    ProcessPriors {
        size: size.into(),
        template: template.into(),
        growth: Some(g),
        growth_excess: None,
        operating_margin: None,
        fcf_margin: Some(m),
        corr_growth_operating_margin: None,
        corr_growth_operating_margin_se: None,
        corr_growth_fcf_margin: None,
        corr_growth_fcf_margin_se: None,
    }
}

/// A table with one cell for the size bucket 1-10B and one for all: growth (phi 0.2, mean 8%, tau 6%, sigma 12%), margin
/// (phi 0.5, mean 10%, tau 8%, sigma 9%).
pub(crate) fn table() -> ProcessTable {
    let meta = TableMeta { date: "test".into(), command: String::new(), companies: 0, company_years: 0, growth_observations: 0, note: String::new() };
    let cells = vec![
        cell("1-10B", "standard", ar1(0.2, 0.08, 0.06, 0.12), ar1(0.5, 0.10, 0.08, 0.09)),
        cell("all", "all", ar1(0.2, 0.08, 0.06, 0.12), ar1(0.5, 0.10, 0.08, 0.09)),
    ];
    ProcessTable { meta, cell: cells }
}

/// A rule set with the table shock size for everyone and no level rule: the hand calculation of the tests.
/// The rules of the fixed-answer tests: no own spread, no level pull, and the past spend as the charge base
/// (`acq_persist` off), so that a test of the charge reads the spend that it sets.
pub(crate) fn plain_rules() -> Rules {
    Rules { own_sigma_nu: 0.0, level_kappa: 0.0, acq_persist: None, ..Rules::default() }
}

/// Six fiscal years 2020 to 2025: revenue +10% each year from $1B, operating cash flow 20% of revenue, capex 5%, interest 2%,
/// SBC 3%. Debt $200M, cash $50M, 100M shares.
fn years() -> Vec<YearData> {
    (0..6)
        .map(|k| {
            let rev = 1e9 * 1.1f64.powi(k);
            YearData {
                end: format!("{}-12-31", 2020 + k),
                filed: format!("{}-02-15", 2021 + k),
                revenue: Some(rev),
                operating_cash_flow: Some(0.20 * rev),
                capex: Some(0.05 * rev),
                sbc: Some(0.03 * rev),
                interest: Some(0.02 * rev),
                total_debt: Some(2e8),
                cash: Some(5e7),
                shares: Some(1e8),
                ..Default::default()
            }
        })
        .collect()
}

pub(crate) fn inputs() -> AutoInputs {
    let y = years();
    AutoInputs {
        ticker: "ACME".into(),
        name: "Acme Corp".into(),
        template: "standard".into(),
        status: "PASS".into(),
        balance: Some(BalanceData::from_year(y.last().unwrap())),
        years: y,
        ..Default::default()
    }
}

pub(crate) fn market() -> Market {
    Market { as_of: "2026-11-19".into(), ..Default::default() }
}

fn spec_of(m: &AutoModel) -> PathsSpec {
    PathsSpec::from_toml(&m.toml).unwrap_or_else(|e| panic!("{e}\n{}", m.toml))
}

fn draw_mean(s: &StartSpec) -> f64 {
    match s {
        StartSpec::Value(p) => p.mean(),
        StartSpec::Tied(_) => panic!("tied"),
    }
}

#[test]
fn the_ar1_fit_has_the_hand_calculated_numbers() {
    // phi 0.2, tau 6%, sigma 12%, mean 8%; four growth values of 10%.
    // mvf = (1 + 2 (0.75*0.2 + 0.5*0.04 + 0.25*0.008)) / (0.96 * 4) = 0.35; noise = 0.0144 * 0.35 = 0.00504
    // w = 0.0036 / (0.0036 + 0.00504) = 0.41667; mu = w 0.10 + (1 - w) 0.08 = 0.088333
    let f = ar1_fit(&[0.1; 4], Some(0.1), &ar1(0.2, 0.08, 0.06, 0.12));
    assert!((f.w - 0.416667).abs() < 1e-5, "{f:?}");
    assert!((f.mu - 0.0883333).abs() < 1e-6);
    assert!((f.var_post - 0.58333 * 0.0036).abs() < 1e-6);
    assert!((f.start_mean - (0.0883333 + 0.2 * (0.1 - 0.0883333))).abs() < 1e-6);
    assert!((f.start_sd - (0.64 * 0.0021 + 0.0144f64).sqrt()).abs() < 1e-6);
    // no history: the cell mean, with all of the cell variance
    let g = ar1_fit(&[], None, &ar1(0.2, 0.08, 0.06, 0.12));
    assert_eq!((g.w, g.mu, g.start_mean), (0.0, 0.08, 0.08));
}

#[test]
fn a_long_history_gives_more_weight_to_the_company_and_a_short_one_less() {
    let p = ar1(0.2, 0.08, 0.06, 0.12);
    let short = ar1_fit(&[0.3, 0.3], Some(0.3), &p);
    let long = ar1_fit(&[0.3; 8], Some(0.3), &p);
    assert!(short.w < long.w && long.mu > short.mu && long.mu < 0.3 && short.mu > 0.08);
}

#[test]
fn the_company_shock_size_uses_the_residuals_of_its_history() {
    let p = ar1(0.2, 0.08, 0.06, 0.12);
    // a constant history has no residual: sqrt(nu0 sigma^2 / (nu0 + 3))
    let s = own_sigma(&[0.1; 4], &p, 4.0);
    assert!((s - (4.0 * 0.0144 / 7.0f64).sqrt()).abs() < 1e-12);
    // nu0 = 0, or fewer than 3 values: the table value
    assert_eq!(own_sigma(&[0.1; 4], &p, 0.0), 0.12);
    assert_eq!(own_sigma(&[0.1, 0.2], &p, 4.0), 0.12);
    // a wild history gives a larger value than the table
    assert!(own_sigma(&[0.5, -0.4, 0.6, -0.5], &p, 4.0) > 0.12);
}

#[test]
fn the_half_life_and_the_linear_ramp_follow_their_formulas() {
    assert!((half_life_of(0.5) - 1.0).abs() < 1e-12);
    // phi is cut to [0.02, 0.95]
    assert!((half_life_of(0.0) - half_life_of(0.02)).abs() < 1e-12);
    // a half-life of 4 years: share 1 - 2^(-9/4) = 0.7896; N = 1 + 9 / 0.7896 = 12.4 -> 12
    assert_eq!(fade_ramp_years(4.0, 10), 12);
    // a very fast fade: the ramp is the shortest one
    assert_eq!(fade_ramp_years(0.1, 10), 10);
}

#[test]
fn the_tax_rate_is_the_effective_rate_only_when_it_is_stable() {
    let mut y = years();
    for (i, v) in y.iter_mut().enumerate() {
        v.pretax_income = Some(100.0);
        v.income_tax = Some(16.0 + i as f64);
    }
    // the last 5 years: 17..21 on 100: 17% to 21%, median 19%
    let (t, why) = tax_rate(&y, 0.21);
    assert!((t - 0.19).abs() < 1e-12, "{why}");
    // one year with a 60% rate: not stable
    y[5].income_tax = Some(60.0);
    assert_eq!(tax_rate(&y, 0.21).0, 0.21);
    // too few values
    assert_eq!(tax_rate(&years(), 0.21).0, 0.21);
}

/// Weekly closes on Fridays from a start day: `n` weeks of a series whose weekly return is `f(week)`.
fn weekly_series(start_day: i64, n: usize, f: &dyn Fn(usize) -> f64) -> Vec<(String, f64)> {
    let mut p = 100.0;
    (0..n)
        .map(|k| {
            if k > 0 {
                p *= 1.0 + f(k);
            }
            (mapcheck::dates::format(start_day + 7 * k as i64), p)
        })
        .collect()
}

#[test]
fn the_regression_beta_recovers_a_known_beta_and_shrinks_it() {
    let friday = mapcheck::dates::parse("2020-01-03").unwrap();
    // a market return that moves between -3% and +3% (a fixed pattern); the stock return is 1.5 times it
    let mr = |k: usize| ((k * 37 % 61) as f64 - 30.0) / 1000.0;
    let market = weekly_series(friday, 300, &mr);
    let stock = weekly_series(friday, 300, &|k| 1.5 * mr(k));
    let as_of = mapcheck::dates::format(friday + 7 * 299);
    let b = regression_beta(&stock, &market, &as_of, 260, 2.0 / 3.0, "test").unwrap();
    assert_eq!(b.weeks, 260);
    assert!((b.raw - 1.5).abs() < 1e-9, "{b:?}");
    assert!((b.adjusted - (2.0 / 3.0 * 1.5 + 1.0 / 3.0)).abs() < 1e-9);
    // fewer than 104 weeks: no beta
    assert!(regression_beta(&stock[..100], &market[..100], &as_of, 260, 2.0 / 3.0, "test").is_none());
    // the window ends at the as-of date: a later week does not count
    let early = mapcheck::dates::format(friday + 7 * 150);
    assert_eq!(regression_beta(&stock, &market, &early, 260, 2.0 / 3.0, "t").unwrap().weeks, 150);
    // a bar later in the same week as the date does not count: the Monday of the last week as the date, and a wild close on
    // the Wednesday after it, in both series, give the beta of the series without that bar
    let last_friday = friday + 7 * 299;
    let monday = last_friday - 4;
    let mut stock2 = stock.clone();
    let mut market2 = market.clone();
    stock2.insert(299, (mapcheck::dates::format(monday), stock[298].1 * 1.01));
    market2.insert(299, (mapcheck::dates::format(monday), market[298].1 * 1.01));
    let date = mapcheck::dates::format(monday);
    let with_bar = regression_beta(&stock2, &market2, &date, 260, 2.0 / 3.0, "t").unwrap();
    let mut stock3 = stock2.clone();
    stock3[300].1 *= 5.0; // the Friday close after the date
    let poisoned = regression_beta(&stock3, &market2, &date, 260, 2.0 / 3.0, "t").unwrap();
    assert_eq!(with_bar.raw, poisoned.raw);
}

#[test]
fn the_model_of_a_small_company_has_the_hand_calculated_numbers() {
    let m = build(&inputs(), &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    let s = spec_of(&m);
    // base revenue: 1.1^5 of $1B
    assert!((s.segments[0].base_revenue - 1.61051e9).abs() < 1.0);
    // net debt: 200M - 50M; shares: 100M
    assert_eq!((s.net_debt, s.shares), (1.5e8, 1e8));
    // growth: company mean 8.8333%, year 1 9.0667%, std sqrt(0.64 * 0.0021 + 0.0144)
    assert!((draw_mean(&s.process["growth"].start) - 0.0906667).abs() < 1e-6);
    assert!((s.process["growth"].target.mean() - 0.0883333).abs() < 1e-6);
    assert!((s.process["growth"].target.std() - (0.0021f64).sqrt()).abs() < 1e-6);
    // L = min(default rate 4.2%, 4.0%) = 4.0% (the default long_run): the company mean is above it, so the growth fades:
    // drift end = 4.0% - 8.8333% (before the decision of 2026-11-20 the level was the rate 4.2%)
    let d = s.process["growth"].drift.as_ref().unwrap();
    assert!((d.end - (0.040 - 0.0883333)).abs() < 1e-6 && d.start == 0.0);
    assert_eq!(d.ramp_years, 12, "the default half-life 4 years (no fade cells)");
    // margin: shrunk as cash flow after interest, plus 79% of the 2% interest ratio
    // mvf(0.5, 5) = 0.593333, noise = 0.0081 * 0.593333, w = 0.0064 / (0.0064 + noise) = 0.571122, mu = 0.128556
    assert!((s.process["margin"].target.mean() - (0.128556 + 0.0158)).abs() < 1e-5);
    assert!((draw_mean(&s.process["margin"].start) - (0.139278 + 0.0158)).abs() < 1e-5);
    assert!((s.process["margin"].half_life.as_ref().unwrap().mean() - 1.0).abs() < 1e-9);
    // SBC 3%
    assert!((draw_mean(&s.process["sbc"].start) - 0.03).abs() < 1e-12);
    // no price, no market history, no curve: stated defaults, each with a warning
    assert_eq!((s.discount.beta.mean(), s.discount.equity_weight, s.tax.rate), (1.0, 1.0, 0.21));
    let w = m.warnings.join("|");
    assert!(w.contains("no GSW yield curve") && w.contains("no price") && w.contains("beta is 1.0"), "{w}");
    assert_eq!(s.rates.model, "fixed");
    // the model runs in the engine
    let sim = PathEngine::new(&s).unwrap().simulate(2000, 7, &Mods::default(), None, false).unwrap();
    assert!(sim.p50().is_finite() && sim.p50() > 0.0);
    // every number has a row with a source and a rule
    assert!(m.rows.len() >= 16 && m.rows.iter().all(|r| !r.source.is_empty() && !r.rule.is_empty() && !r.value.is_empty()));
}

#[test]
fn a_price_and_a_beta_set_the_equity_weight_and_the_discount_rate() {
    let mk = Market {
        price: Some(20.0),
        price_source: "--price".into(),
        beta: Some(BetaFit { raw: 1.2, adjusted: 1.13, weeks: 260, first: "a".into(), last: "b".into(), market: "m".into() }),
        curve: Some(CurveNumbers { date: "2026-11-13".into(), beta0: 0.0, beta1: 4.1, beta2: 4.9, beta3: 16.6, tau1: 1.8, tau2: 15.7, zero10: 0.047 }),
        ..market()
    };
    let m = build(&inputs(), &table(), &[], &mk, &plain_rules(), BuildOptions::default()).unwrap();
    let s = spec_of(&m);
    // equity 20 * 100M = 2B; debt 200M: weight 2000 / 2200
    assert!((s.discount.equity_weight - 2000.0 / 2200.0).abs() < 1e-12);
    assert_eq!(s.discount.beta.mean(), 1.13);
    assert_eq!(s.rates.model, "hullwhite");
    // the level L is min(10-year zero yield of the curve 4.7%, 4.0%) = 4.0%; with long_run off it is the yield 4.7%
    let d = s.process["growth"].drift.as_ref().unwrap();
    assert!((d.end - (0.040 - 0.0883333)).abs() < 1e-6);
    let old = build(&inputs(), &table(), &[], &mk, &Rules { long_run: None, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert!((spec_of(&old).process["growth"].drift.as_ref().unwrap().end - (0.047 - 0.0883333)).abs() < 1e-6);
    assert!(!m.warnings.join("|").contains("no price"));
}

#[test]
fn a_company_below_the_cap_has_no_fade() {
    let mut y = years();
    for (k, v) in y.iter_mut().enumerate() {
        v.revenue = Some(1e9 * 0.98f64.powi(k as i32));
    }
    let mut i = inputs();
    // the test is about the cap, not about the floor: a decline of 2% each year with the floor would raise the company mean above the cap
    i.years = y;
    let m = build(&i, &table(), &[], &market(), &Rules { last_floor: None, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert!(spec_of(&m).process["growth"].drift.is_none());
}

#[test]
fn a_jump_with_no_cash_for_businesses_stays_in_the_history_and_with_cash_it_goes() {
    // revenue 2B, 2B, 2B, 2B, 5B (+150%), 5.5B
    let mut y = years();
    for (k, v) in y.iter_mut().enumerate() {
        v.revenue = Some(if k < 4 { 2e9 } else if k == 4 { 5e9 } else { 5.5e9 });
        v.operating_cash_flow = Some(0.2 * v.revenue.unwrap());
        v.capex = Some(0.05 * v.revenue.unwrap());
    }
    let mut i = inputs();
    i.years = y.clone();
    let organic = build(&i, &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    let r = Rules { history_organic: false, ..plain_rules() };
    let clean = build(&i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
    // the organic history has the +150%, so the company mean is higher
    assert!(spec_of(&organic).process["growth"].target.mean() > spec_of(&clean).process["growth"].target.mean() + 0.05);
    // the same jump with cash for businesses: left out of the history
    y[4].acquisitions = Some(2.5e9);
    i.years = y;
    let acq = build(&i, &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    assert!((spec_of(&acq).process["growth"].target.mean() - spec_of(&clean).process["growth"].target.mean()).abs() < 1e-9);
}

#[test]
fn a_margin_cell_with_a_large_standard_error_takes_the_phi_of_all_sizes() {
    let mut t = table();
    // the size cell: phi 0.9 with a standard error 0.14; the cell of all sizes for the template is ("all", "standard")
    t.cell[0].fcf_margin = Some(Ar1Params { phi_se: 0.14, ..ar1(0.9, 0.10, 0.08, 0.09) });
    t.cell.push(cell("all", "standard", ar1(0.2, 0.08, 0.06, 0.12), ar1(0.4, 0.11, 0.1, 0.1)));
    let (p, note) = margin_params(&t, "1-10B", "standard", 0.08).unwrap();
    assert_eq!((p.phi, p.mean, p.mean_sd_units), (0.4, 0.10, 0.08));
    assert!(note.contains("replaced"), "{note}");
    // switched off: the phi of the size cell
    assert_eq!(margin_params(&t, "1-10B", "standard", 0.0).unwrap().0.phi, 0.9);
}

fn fade_cells_for_tests() -> Vec<FadeCell> {
    let step = |k, p10, p50, p90| FadeStep { k, n: 100, p10, p50, p90 };
    vec![FadeCell {
        size: "all".into(),
        threshold: 0.3,
        n_start: 1000,
        start_mean: 0.6,
        start_median: 0.45,
        group_mean: 0.10,
        // k = 3: 0.20; k = 10: 0.12: the gap to the mean 0.10 shrinks from 0.10 to 0.02: r = 0.2
        step: vec![step(1, -0.1, 0.25, 0.65), step(3, -0.1, 0.20, 0.5), step(10, -0.1, 0.12, 0.4)],
    }]
}

#[test]
fn the_fade_table_gives_the_half_life_and_the_spread_ratio() {
    let cells = fade_cells_for_tests();
    let h = slow_half_life(&cells).unwrap();
    assert!((h - std::f64::consts::LN_2 / -(0.2f64.powf(1.0 / 7.0)).ln()).abs() < 1e-9);
    // (0.65 + 0.1) / (2 * 1.28155) / 0.6
    let k = level_spread_ratio(&cells).unwrap();
    assert!((k - 0.75 / (2.0 * 1.2815515655446004) / 0.6).abs() < 1e-9);
    // no fade cells: none
    assert!(slow_half_life(&[]).is_none() && level_spread_ratio(&[]).is_none());
    // the fade cells make a table file and a table file without them still reads as a table
    let text = format!("{}\n{}", table().to_toml().unwrap(), FadeFile { fade: cells.clone() }.to_toml().unwrap());
    assert_eq!(FadeFile::from_toml(&text).unwrap().fade, cells);
    assert_eq!(ProcessTable::from_toml(&text).unwrap().cell.len(), 2);
    assert!(FadeFile::from_toml(&table().to_toml().unwrap()).unwrap().fade.is_empty());
}

#[test]
fn fade_cells_change_the_ramp_and_the_shock() {
    let cells = fade_cells_for_tests();
    let r = Rules { own_sigma_nu: 0.0, ..Rules::default() };
    let m = build(&inputs(), &table(), &cells, &market(), &r, BuildOptions::default()).unwrap();
    let s = spec_of(&m);
    let h = slow_half_life(&cells).unwrap();
    assert_eq!(s.process["growth"].drift.as_ref().unwrap().ramp_years, fade_ramp_years(h, 10));
    let kappa = level_spread_ratio(&cells).unwrap();
    assert!((s.process["growth"].sigma_level - kappa).abs() < 1e-12);
    assert!(!m.warnings.join("|").contains("fade"));
}

#[test]
fn refusals_name_the_reason() {
    let run = |f: &dyn Fn(&mut AutoInputs)| {
        let mut i = inputs();
        f(&mut i);
        build(&i, &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap_err()
    };
    for t in ["bank", "insurer", "reit", "fund", "foreign", "new"] {
        let e = run(&|i| i.template = t.into());
        assert_eq!(e, Refusal::Template(t.into()));
        assert!(e.to_string().contains("no model for this template") && e.key().contains(t));
    }
    let e = run(&|i| {
        i.status = "FAIL".into();
        i.findings.push(FindingText { severity: "FAIL".into(), code: "sign".into(), detail: "capex".into() });
    });
    assert!(matches!(&e, Refusal::Fail(m) if m.contains("sign (capex)")));
    let e = run(&|i| {
        i.status = "ERROR".into();
        i.error = Some("no file".into());
    });
    assert_eq!(e, Refusal::NoData("no file".into()));
    assert_eq!(run(&|i| i.years.last_mut().unwrap().capex = None), Refusal::Missing(vec!["capex".into()]));
    assert_eq!(run(&|i| i.years.last_mut().unwrap().operating_cash_flow = None), Refusal::Missing(vec!["operating_cash_flow".into()]));
    let e = run(&|i| i.balance.as_mut().unwrap().shares = None);
    assert_eq!(e, Refusal::Missing(vec!["shares_diluted".into()]));
    assert!(matches!(run(&|i| i.years.truncate(2)), Refusal::ShortHistory(_)));
    assert!(matches!(run(&|i| i.balance = None), Refusal::Missing(_)));
    // a table with no cell: no priors
    let e = build(&inputs(), &ProcessTable { meta: table().meta, cell: vec![] }, &[], &market(), &plain_rules(), BuildOptions::default()).unwrap_err();
    assert!(matches!(e, Refusal::NoPriors(_)));
}

#[test]
fn a_mapping_warning_on_a_model_input_is_printed_and_another_is_not() {
    let mut i = inputs();
    i.status = "WARN".into();
    i.findings = vec![
        FindingText { severity: "WARN".into(), code: "total_debt_estimates".into(), detail: "two estimates".into() },
        FindingText { severity: "WARN".into(), code: "odd_tag".into(), detail: "a tag of a note".into() },
    ];
    let m = build(&i, &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    let w = m.warnings.join("|");
    assert!(w.contains("total_debt_estimates") && !w.contains("odd_tag"), "{w}");
}

#[test]
fn a_missing_debt_is_zero_with_a_warning() {
    let mut i = inputs();
    i.balance.as_mut().unwrap().total_debt = None;
    let m = build(&i, &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    assert_eq!(spec_of(&m).net_debt, -5e7);
    assert!(m.warnings.join("|").contains("total_debt has no value"));
}

#[test]
fn a_jump_in_the_latest_year_with_cash_for_businesses_gets_a_warning() {
    let mut y = years();
    y[5].revenue = Some(y[4].revenue.unwrap() * 2.0);
    y[5].acquisitions = Some(2e9);
    let mut i = inputs();
    i.years = y;
    let m = build(&i, &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    assert!(m.warnings.join("|").contains("is a jump"));
}

#[test]
fn the_implied_target_margin_gives_back_the_margin_of_a_known_price() {
    let m = build(&inputs(), &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    let spec = spec_of(&m);
    // the P50 of the model with the target margin 0.22; the price implies 0.22 again
    let mut s2 = spec.clone();
    let g = s2.process.get_mut("margin").unwrap();
    if let ParamSpec::Draw(d) = &mut g.target {
        d.mean = 0.22;
    }
    let price = PathEngine::new(&s2).unwrap().simulate_serial(3000, 7, &Mods::default()).unwrap().p50();
    let x = implied_target_margin(&spec, price, 3000, 7, true).unwrap().unwrap();
    assert!((x - 0.22).abs() < 1e-4, "{x}");
    // a price that no margin in the range gives: none
    assert!(implied_target_margin(&spec, 1e6, 3000, 7, true).unwrap().is_none());
}

#[test]
fn the_implied_long_run_growth_gives_back_the_end_of_the_fade() {
    let m = build(&inputs(), &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    let spec = spec_of(&m);
    let mu = spec.process["growth"].target.mean();
    // the model has a fade to 4.2%; the price of a model that fades to 6%
    let mut s2 = spec.clone();
    s2.process.get_mut("growth").unwrap().drift.as_mut().unwrap().end = 0.06 - mu;
    let price = PathEngine::new(&s2).unwrap().simulate_serial(3000, 7, &Mods::default()).unwrap().p50();
    let x = implied_long_run_growth(&spec, price, 3000, 7, true).unwrap().unwrap();
    assert!((x - 0.06).abs() < 1e-4, "{x}");
    // the half-life of the fade is found from the ramp
    let h = implied_fade_half_life(&spec, price, 3000, 7, true).unwrap();
    assert!(h.is_some());
}

#[test]
fn no_later_value_enters_the_model_of_a_base_year() {
    // nine fiscal years; the model of the end of the sixth year (index 5) must not depend on the years after it
    let mut y: Vec<YearData> = (0..9)
        .map(|k| {
            let rev = 1e9 * 1.08f64.powi(k);
            YearData {
                end: format!("{}-12-31", 2015 + k),
                filed: format!("{}-02-15", 2016 + k),
                revenue: Some(rev),
                operating_cash_flow: Some(0.2 * rev),
                capex: Some(0.05 * rev),
                sbc: Some(0.02 * rev),
                interest: Some(0.01 * rev),
                ..Default::default()
            }
        })
        .collect();
    let rules = Rules::default();
    let a = model_at("ACME", &y, 5, &table(), &[], &rules).unwrap();
    // change every later year: revenue, cash flow, capex, SBC, interest, tax
    for v in &mut y[6..] {
        v.revenue = Some(v.revenue.unwrap() * 3.0);
        v.operating_cash_flow = Some(-1e9);
        v.capex = Some(5e9);
        v.sbc = Some(7e8);
        v.interest = Some(1e9);
        v.income_tax = Some(1e9);
        v.pretax_income = Some(1e9);
        v.acquisitions = Some(9e9);
    }
    let b = model_at("ACME", &y, 5, &table(), &[], &rules).unwrap();
    assert_eq!(a.toml, b.toml);
    assert_eq!(a.summary, b.summary);
    // the model of a later base year does change (the test would be empty otherwise)
    let c = model_at("ACME", &y, 6, &table(), &[], &rules).unwrap();
    assert_ne!(a.toml, c.toml);
    // the real values of the observation come from the later years only
    let obs = company_observations("ACME", &y, &table(), &[], &PitParams { rules: &rules, paths: 200, seed: 7, min_year: None });
    assert_eq!(obs.len(), 2, "the base years are the fifth and the sixth year");
    let o = &obs[1];
    assert_eq!(o.base, y[5].revenue.unwrap());
    assert_eq!(o.rev_act[0], y[6].revenue.unwrap());
    assert_eq!(o.rev_act[2], y[8].revenue.unwrap());
    // the default charges the acquisitions (acq_w): the real free cash flow after acquisitions has the spend of 9e9 out
    assert!((o.fcff_act[0].unwrap() - (-1e9 - 5e9 + 0.79 * 1e9 - 7e8 - 9e9)).abs() < 1.0);
    // the band of the first base year is made from the first five years only: a change of the sixth year does not move it
    let mut y2 = y.clone();
    y2[5].revenue = Some(y2[5].revenue.unwrap() * 2.0);
    let again = company_observations("ACME", &y2, &table(), &[], &PitParams { rules: &rules, paths: 200, seed: 7, min_year: None });
    assert_eq!(obs[0].rev_q, again[0].rev_q);
    assert_eq!(obs[0].fcff_q, again[0].fcff_q);
}

#[test]
fn the_base_years_need_five_years_up_to_t_and_three_after_in_sequence() {
    let y: Vec<YearData> = (0..9).map(|k| YearData { end: format!("{}-12-31", 2015 + k), revenue: Some(1e8 * (1.0 + k as f64)), ..Default::default() }).collect();
    // nine years: T can be the fifth year to the sixth year (indices 4 and 5): 5 up to T, 3 after
    let b: Vec<usize> = base_years(&y).into_iter().map(|x| x.0).collect();
    assert_eq!(b, vec![4, 5]);
    // a missing year in the middle breaks the sequence
    let mut gap = y.clone();
    gap.remove(4);
    assert!(base_years(&gap).is_empty());
    // fewer than eight years: none
    assert!(base_years(&y[..7]).is_empty());
}

#[test]
fn the_coverage_counts_below_inside_and_above() {
    let obs = |actual: f64| PitObs {
        ticker: "T".into(),
        year: 2020,
        size: 1,
        base: 100.0,
        g_last: Some(0.1),
        margin_range: 0.02,
        has_jump: false,
        g_start: 0.05,
        g_mu: 0.05,
        acq_past: 0.0,
        acq_next: 0.0,
        rev_q: [[90.0, 100.0, 110.0]; HORIZON],
        rev_act: [actual; HORIZON],
        fcff_q: [[1.0, 2.0, 3.0]; HORIZON],
        fcff_act: [Some(actual / 50.0); HORIZON],
    };
    let o = vec![obs(80.0), obs(95.0), obs(100.0), obs(105.0), obs(120.0)];
    let c = coverage(&o, Series::Revenue, 0, &|_| true);
    assert_eq!((c.n, c.below, c.inside, c.above, c.below_p50), (5, 1, 3, 1, 2));
    assert_eq!(c.shares(), [20.0, 60.0, 20.0]);
    // the free cash flow: 1.6, 1.9, 2.0, 2.1, 2.4: none below 1.0, none above 3.0
    let f = coverage(&o, Series::Fcff, 1, &|_| true);
    assert_eq!((f.below, f.inside, f.above), (0, 5, 0));
    // the median error of the revenue is the real value over the P50 less 1: -20%, -5%, 0, +5%, +20% -> 0
    assert_eq!(median_error(&o, Series::Revenue, 0, &|_| true), Some(0.0));
    // a subset
    assert_eq!(coverage(&o, Series::Revenue, 0, &|x| x.rev_act[0] > 100.0).n, 2);
    // the text has the table
    let t = render(&o, "HEADER");
    assert!(t.contains("HEADER") && t.contains("REVENUE by size bucket") && t.contains("1-10B"));
}

#[test]
fn the_rules_text_changes_a_rule_and_refuses_an_unknown_one() {
    let r = parse_rules("own_sigma_nu=0,level_kappa=0.3,history_organic=0").unwrap();
    assert_eq!((r.own_sigma_nu, r.level_kappa, r.history_organic), (0.0, 0.3, false));
    assert!(parse_rules("nope=1").is_err() && parse_rules("level_kappa=x").is_err());
    assert_eq!(parse_rules("").unwrap(), Rules::default());
}

pub(crate) fn synthetic(n: usize) -> Vec<AutoInputs> {
    (0..n)
        .map(|k| {
            let mut i = inputs();
            i.ticker = format!("T{k:02}");
            let g = 1.0 + 0.02 * (k % 9) as f64;
            for (j, y) in i.years.iter_mut().enumerate() {
                let rev = 1e9 * g.powi(j as i32) * (1.0 + 0.3 * k as f64);
                y.revenue = Some(rev);
                y.operating_cash_flow = Some((0.10 + 0.01 * (k % 7) as f64) * rev);
                y.capex = Some(0.04 * rev);
            }
            i.balance = Some(BalanceData::from_year(i.years.last().unwrap()));
            if k % 5 == 3 {
                i.template = "bank".into();
            }
            if k % 7 == 6 {
                i.years.last_mut().unwrap().capex = None;
            }
            i
        })
        .collect()
}

#[test]
fn the_screen_gives_the_same_rows_on_one_and_on_four_threads() {
    let inputs = synthetic(24);
    let t = table();
    let cfg = ScreenConfig { table: &t, fade: &[], rules: Rules::default(), sims: 500, seed: 7, implied_rate: false };
    let market_of = |tk: &str| Market { price: if tk == "T01" { Some(30.0) } else { None }, price_source: "test".into(), ..market() };
    let (a, _) = screen_models(&inputs, &market_of, &cfg, Some(1)).unwrap();
    let (b, _) = screen_models(&inputs, &market_of, &cfg, Some(4)).unwrap();
    assert_eq!(a, b);
    // the rows are in the order of the input; the refusals have their reason; the others have a value
    assert_eq!(a.iter().map(|r| r.ticker.clone()).collect::<Vec<_>>(), inputs.iter().map(|i| i.ticker.clone()).collect::<Vec<_>>());
    let refused: Vec<String> = a.iter().filter_map(|r| r.outcome.as_ref().err().map(|e| e.0.clone())).collect();
    assert!(refused.iter().any(|k| k == "template bank: no model") && refused.iter().any(|k| k == "missing capex"));
    let with_value = a.iter().filter(|r| r.outcome.is_ok()).count();
    assert_eq!(with_value + refused.len(), 24);
    assert!(with_value > 10);
    // the ticker with a price has the share below it
    let t01 = a.iter().find(|r| r.ticker == "T01").unwrap().outcome.as_ref().unwrap();
    assert!(t01.share_below.is_some() && t01.price == Some(30.0));
    let t02 = a.iter().find(|r| r.ticker == "T02").unwrap().outcome.as_ref().unwrap();
    assert!(t02.share_below.is_none() && t02.p10 < t02.p50 && t02.p50 < t02.p90);
    // the same run again gives the same rows (the seed is fixed)
    let (c, _) = screen_models(&inputs, &market_of, &cfg, Some(2)).unwrap();
    assert_eq!(a, c);
    // a CSV line for a row with a value and for a refusal
    let line = csv_line(&a[0]);
    assert_eq!(line.split(',').count(), CSV_HEADER.split(',').count(), "{line}");
    // the column `mean` is the last one (the order of the other columns did not change) and holds the mean of the paths
    assert!(CSV_HEADER.ends_with(",acq_ratio,mean"));
    let v0 = a[0].outcome.as_ref().unwrap();
    assert_eq!(line.rsplit(',').next().unwrap(), format!("{:.2}", v0.mean));
    assert!(v0.mean.is_finite() && v0.mean > 0.0);
    let bank = a.iter().find(|r| r.ticker == "T03").unwrap();
    assert_eq!(csv_line(bank).split(',').count(), CSV_HEADER.split(',').count());
    assert!(csv_line(bank).contains(",template bank: no model,"));
}

#[test]
fn a_price_file_gives_prices_and_refuses_a_bad_line() {
    let dir = std::env::temp_dir().join(format!("auto-prices-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ok = dir.join("ok.csv");
    std::fs::write(&ok, "ticker,price\nibm, 228.5\n# a comment\nJNJ,272.7\n").unwrap();
    let p = read_prices(ok.to_str().unwrap()).unwrap();
    assert_eq!((p["IBM"], p["JNJ"], p.len()), (228.5, 272.7, 2));
    let bad = dir.join("bad.csv");
    std::fs::write(&bad, "IBM,228.5\nJNJ,abc\n").unwrap();
    assert!(read_prices(bad.to_str().unwrap()).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- Tests of the investigation of 2026-11-20 (`analysis/priors/dcf_investigation_2026-11-20.md`)

fn ev_at_means(m: &AutoModel) -> f64 {
    let sim = PathEngine::new(&spec_of(m)).unwrap().deterministic().unwrap();
    sim.pv_explicit[0] + sim.pv_terminal[0]
}

#[test]
fn the_timing_of_the_discount_follows_the_model_date_and_the_balance_sheet() {
    // base year 2025-12-31, so year 1 ends 2026-12-31; the 10-Q of 2026-09-30; the model date 2026-11-19
    let (stub, share) = stub_and_share("2025-12-31", "2026-09-30", "2026-11-19").unwrap();
    assert!((stub - 42.0 / 365.0).abs() < 1e-12 && (share - 92.0 / 365.0).abs() < 1e-12, "{stub} {share}");
    // year 1 is already over (its 10-K is not filed yet): no discount for it; a balance sheet of the base year: all of year 1
    assert_eq!(stub_and_share("2025-06-30", "2025-06-30", "2026-11-19").unwrap(), (0.0, 1.0));
    // the engine at a fixed rate (4.2% + 1.0 x 4.41%): a stub of half a year moves every flow half a year nearer
    let rules = Rules { stub: false, ..plain_rules() };
    let base = build(&inputs(), &table(), &[], &market(), &rules, BuildOptions::default()).unwrap();
    let k: f64 = 0.042 + 0.0441;
    let with = |stub: f64, share: f64, mid: bool| {
        let mut t = base.toml.replace("equity_weight = 1.0\n", &format!("equity_weight = 1.0\nstub_years = {stub:?}\nyear1_share = {share:?}\n"));
        if mid {
            t = t.replace("equity_weight = 1.0\n", "equity_weight = 1.0\nmid_year = true\n");
        }
        let sim = PathEngine::new(&PathsSpec::from_toml(&t).unwrap()).unwrap().deterministic().unwrap();
        (sim.pv_explicit[0] + sim.pv_terminal[0], sim)
    };
    let ev0 = ev_at_means(&base);
    let (ev1, _) = with(1.0, 1.0, false);
    assert_eq!(ev1, ev0, "stub 1 and share 1 are the old rule");
    let (ev_half, _) = with(0.5, 1.0, false);
    assert!((ev_half / ev0 - (1.0 + k).sqrt()).abs() < 1e-9, "{}", ev_half / ev0);
    // the share of year 1 after the balance sheet: a quarter of the flow of year 1 stays
    let det = PathEngine::new(&spec_of(&base)).unwrap().deterministic().unwrap();
    let fcf1 = det.column(0, crate::dcf::paths::F_FCFF)[0];
    let (ev_q, _) = with(1.0, 0.25, false);
    assert!((ev0 - ev_q - 0.75 * fcf1 / (1.0 + k)).abs() < 1e-6 * ev0, "{} {}", ev0 - ev_q, 0.75 * fcf1 / (1.0 + k));
    // the middle of the year: every flow and the terminal value come half a year earlier
    let (ev_mid, _) = with(1.0, 1.0, true);
    assert!((ev_mid / ev0 - (1.0 + k).sqrt()).abs() < 1e-9, "{}", ev_mid / ev0);
    // a bad value is refused
    assert!(PathEngine::new(&PathsSpec::from_toml(&base.toml.replace("equity_weight = 1.0\n", "equity_weight = 1.0\nstub_years = 1.5\n")).unwrap()).is_err());
}

#[test]
fn the_default_model_starts_the_discount_on_the_model_date() {
    // the balance sheet is a 10-Q of 2026-09-30; the base fiscal year ends 2025-12-31; the model date is 2026-11-19
    let mut i = inputs();
    i.balance.as_mut().unwrap().end = "2026-09-30".into();
    let m = build(&i, &table(), &[], &market(), &Rules::default(), BuildOptions::default()).unwrap();
    let d = &spec_of(&m).discount;
    assert!((d.stub_years.unwrap() - 42.0 / 365.0).abs() < 1e-12, "{:?}", d.stub_years);
    assert!((d.year1_share.unwrap() - 92.0 / 365.0).abs() < 1e-12);
    assert!(!d.mid_year, "the middle of the year is an option, not the default");
    // the point-in-time model (no valuation) has no timing
    let p = build(&i, &table(), &[], &market(), &Rules::default(), BuildOptions { valuation: false }).unwrap();
    assert!(spec_of(&p).discount.stub_years.is_none());
}

#[test]
fn interest_income_leaves_the_margin_and_the_investments_enter_the_bridge() {
    // interest income 1% of revenue, long-term investments $100M
    let mut i = inputs();
    for y in i.years.iter_mut() {
        y.interest_income = Some(0.01 * y.revenue.unwrap());
    }
    i.balance.as_mut().unwrap().long_term_investments = Some(1e8);
    let m = build(&i, &table(), &[], &market(), &Rules { stub: false, ..plain_rules() }, BuildOptions::default()).unwrap();
    let s = spec_of(&m);
    // the margin: the after-tax interest ratio is (1 - 0.21)(2% - 1%) = 0.79%, not 1.58%
    assert!((s.process["margin"].target.mean() - (0.128556 + 0.0079)).abs() < 1e-5, "{}", s.process["margin"].target.mean());
    // the bridge: debt 200M - cash 50M - investments 100M
    assert_eq!(s.net_debt, 5e7);
    // the old rule with both parts off
    let old = build(&i, &table(), &[], &market(), &Rules { stub: false, no_interest_income: false, bridge_lti: false, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert_eq!(spec_of(&old).net_debt, 1.5e8);
    // the point-in-time free cash flow follows the same rule
    let y = &i.years[5];
    let a = fcff_of_rules(y, 0.21, &Rules::default()).unwrap();
    let b = fcff_of(y, 0.21).unwrap();
    assert!((b - a - 0.79 * y.interest_income.unwrap()).abs() < 1e-3);
}

#[test]
fn the_share_count_is_the_count_at_the_end_plus_the_dilution() {
    // weighted diluted 100M, weighted basic 98M, 95M at the end of the period (buybacks): 95M + 2M
    let mut i = inputs();
    let b = i.balance.as_mut().unwrap();
    b.shares_outstanding = Some(9.5e7);
    b.shares_basic = Some(9.8e7);
    let m = build(&i, &table(), &[], &market(), &Rules { stub: false, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert_eq!(spec_of(&m).shares, 9.7e7);
    // a count outside 85% to 110% of the weighted count (one class of two, a unit error) is not used, with a warning
    i.balance.as_mut().unwrap().shares_outstanding = Some(4e7);
    let m = build(&i, &table(), &[], &market(), &Rules { stub: false, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert_eq!(spec_of(&m).shares, 1e8);
    assert!(m.warnings.join("|").contains("shares_outstanding"), "{:?}", m.warnings);
}

#[test]
fn the_acquisition_charge_is_an_option_with_the_mean_ratio() {
    let mut i = inputs();
    for y in i.years.iter_mut() {
        y.acquisitions = Some(0.02 * y.revenue.unwrap());
    }
    let off = build(&i, &table(), &[], &market(), &Rules { acq_w: false, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert!(!spec_of(&off).process.contains_key("charge"));
    // acq_charge wins over the default acq_w: the full mean ratio
    let on = build(&i, &table(), &[], &market(), &Rules { acq_charge: true, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert!((draw_mean(&spec_of(&on).process["charge"].start) - 0.02).abs() < 1e-12);
    assert!(ev_at_means(&on) < ev_at_means(&off));
}

#[test]
fn the_paired_acquisition_rule_charges_a_share_and_takes_out_the_rest_of_the_bought_growth() {
    let mut i = inputs();
    for y in i.years.iter_mut() {
        y.acquisitions = Some(0.02 * y.revenue.unwrap());
    }
    let run = |r: Rules| build(&i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
    let off = run(plain_rules());
    // f = 1: the charge is the mean ratio and the growth history stays as it is (the same as `acq_charge`)
    let one = run(Rules { acq_f: Some(1.0), acq_mult: 2.0, ..plain_rules() });
    assert!((one.summary.acq_ratio - 0.02).abs() < 1e-12);
    assert!((one.summary.growth_company_mean - off.summary.growth_company_mean).abs() < 1e-12);
    // f = 0.5, 2 dollars of revenue for each dollar: half the charge, and the growth of each year is lower by
    // (1 - f) m acquisitions / revenue of the year before = 0.5 x 2 x 0.02 x 1.1 = 2.2 points
    let half = run(Rules { acq_f: Some(0.5), acq_mult: 2.0, ..plain_rules() });
    assert!((half.summary.acq_ratio - 0.01).abs() < 1e-12);
    assert!(half.summary.growth_company_mean < off.summary.growth_company_mean);
    assert!(half.summary.growth_start < off.summary.growth_start - 0.01);
    // f = 0: organic growth only, no charge
    let zero = run(Rules { acq_f: Some(0.0), acq_mult: 2.0, ..plain_rules() });
    assert_eq!(zero.summary.acq_ratio, 0.0);
    assert!(zero.summary.growth_start < half.summary.growth_start);
    // off by default: the option changes nothing when it is not given
    assert!(Rules::default().acq_f.is_none());
    let r = parse_rules("acq_f=0.5,acq_mult=1.5").unwrap();
    assert_eq!((r.acq_f, r.acq_mult), (Some(0.5), 1.5));
}

#[test]
fn the_charge_can_follow_the_weight_of_the_growth_history() {
    let mut i = inputs();
    for y in i.years.iter_mut() {
        y.acquisitions = Some(0.02 * y.revenue.unwrap());
    }
    let off = build(&i, &table(), &[], &market(), &Rules { acq_w: false, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert_eq!(off.summary.acq_ratio, 0.0);
    // the default is on
    let on = build(&i, &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    // the weight of the history in the growth mean is printed in the rule text of the row "growth: company mean"
    let rule = &on.rows.iter().find(|r| r.input == "growth: company mean").unwrap().rule;
    let w: f64 = rule.split("= ").nth(2).unwrap().split(';').next().unwrap().trim().parse().unwrap();
    assert!(w > 0.0 && w < 1.0, "{rule}");
    assert!((on.summary.acq_ratio - 0.02 * w).abs() < 2e-5 /* w is printed with three decimals */, "{} {}", on.summary.acq_ratio, w);
    // the growth history stays as it is
    assert!((on.summary.growth_company_mean - off.summary.growth_company_mean).abs() < 1e-12);
    assert!(Rules::default().acq_w && parse_rules("").unwrap().acq_w);
    assert!(!parse_rules("acq_w=0").unwrap().acq_w);
}

#[test]
fn one_long_run_growth_is_the_lower_of_the_yield_and_the_number() {
    let i = inputs();
    // the market has no curve: the 10-year zero yield is the default 4.2%
    let tg = |r: Rules| {
        let m = build(&i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
        let t = m.toml.split("[terminal]").nth(1).unwrap().to_string();
        t.split("mean = ").nth(1).unwrap().split(',').next().unwrap().trim().parse::<f64>().unwrap()
    };
    let off = Rules { long_run: None, ..plain_rules() };
    // default (decision of 2026-11-20): L = min(yield 4.2%, 4.0%) = 4.0%
    assert!((tg(plain_rules()) - 0.040).abs() < 1e-12);
    // off: the terminal growth is the rule value
    assert!((tg(off.clone()) - 0.0225).abs() < 1e-12);
    // on: the number when it is below the yield, the yield when it is below the number
    assert!((tg(Rules { long_run: Some(0.038), ..plain_rules() }) - 0.038).abs() < 1e-12);
    assert!((tg(Rules { long_run: Some(0.05), ..plain_rules() }) - DEFAULT_RF).abs() < 1e-12);
    // the explicit rule wins
    assert!((tg(Rules { long_run: Some(0.03), terminal_rf: true, ..plain_rules() }) - DEFAULT_RF).abs() < 1e-12);
    assert_eq!(parse_rules("long_run=0.038").unwrap().long_run, Some(0.038));
    assert_eq!(Rules::default().long_run, Some(0.040));
    // the old model on request: `off` or 0; an explicit terminal_growth is the old model for that number
    assert_eq!(parse_rules("long_run=off").unwrap().long_run, None);
    assert_eq!(parse_rules("long_run=0").unwrap().long_run, None);
    let r = parse_rules("terminal_growth=0.03").unwrap();
    assert_eq!(r.long_run, None);
    assert!((tg(Rules { own_sigma_nu: 0.0, level_kappa: 0.0, ..r }) - 0.03).abs() < 1e-12);
    // long_cap and terminal_rf keep their priority over the default
    assert!((tg(Rules { terminal_rf: true, ..plain_rules() }) - DEFAULT_RF).abs() < 1e-12);
    let cap = |r: Rules| {
        let m = build(&i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
        spec_of(&m).process["growth"].drift.as_ref().unwrap().end + m.summary.growth_company_mean
    };
    // the fade level: L = 4.0% by default (hand: drift end + mean = level), the yield 4.2% when off, long_cap first
    assert!((cap(plain_rules()) - 0.040).abs() < 1e-9);
    assert!((cap(off) - 0.042).abs() < 1e-9);
    assert!((cap(Rules { long_cap: Some(0.03), ..plain_rules() }) - 0.03).abs() < 1e-9);
}

#[test]
fn the_new_rules_have_names_in_the_rules_text() {
    let r = parse_rules("rf=0.04,erp=0.05,terminal_rf=1,years=20,long_cap=0.0225,fade_years=15,growth_w=1,margin_last=1,sbc_cost=0,acq_charge=1,stub=0,mid_year=1,shares_latest=0,bridge_lti=0,no_interest_income=0,bridge_leases=1,equity_weight_one=1,beta_weight=1,terminal_growth=0.03,growth_phi=0.5,margin_w=0.5").unwrap();
    assert_eq!((r.rf_fixed, r.erp.0, r.terminal_rf, r.years, r.long_cap, r.fade_years), (Some(0.04), 0.05, true, 20, Some(0.0225), Some(15)));
    assert!(r.margin_last && !r.sbc_cost && r.acq_charge && !r.stub && r.mid_year && !r.shares_latest && !r.bridge_lti && !r.no_interest_income && r.bridge_leases && r.equity_weight_one);
    assert_eq!((r.beta_weight, r.terminal_growth.0, r.growth_phi, r.margin_w, r.growth_w), (1.0, 0.03, Some(0.5), Some(0.5), Some(1.0)));
}

#[test]
fn the_return_test_helpers_have_known_answers() {
    use super::super::auto_returns::*;
    assert_eq!(ranks(&[3.0, 1.0, 2.0, 2.0]), vec![4.0, 1.0, 2.5, 2.5]);
    assert!((spearman(&[1.0, 2.0, 3.0, 4.0, 5.0], &[2.0, 4.0, 6.0, 8.0, 10.0]).unwrap() - 1.0).abs() < 1e-12);
    assert!((spearman(&[1.0, 2.0, 3.0, 4.0, 5.0], &[5.0, 4.0, 3.0, 2.0, 1.0]).unwrap() + 1.0).abs() < 1e-12);
    let (m, se) = mean_se(&[1.0, 2.0, 3.0]);
    assert!((m - 2.0).abs() < 1e-12 && (se - (1.0f64 / 3.0).sqrt()).abs() < 1e-12);
    let p = vec![(10, 1.0), (12, 2.0), (20, 3.0)];
    assert_eq!(close_on_or_after(&p, 11, 7), Some((12, 2.0)));
    assert_eq!(close_on_or_after(&p, 13, 5), None);
}

#[test]
fn a_decline_in_the_base_year_enters_the_growth_mean_and_the_first_step_as_zero() {
    // revenue +10% in the years 2021 to 2024, then -10% in 2025: the growth history is 10%, 10%, 10%, -10%.
    let mut i = inputs();
    let prev = i.years[4].revenue.unwrap();
    i.years[5].revenue = Some(0.9 * prev);
    i.balance = Some(BalanceData::from_year(i.years.last().unwrap()));
    let on = build(&i, &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    let off = build(&i, &table(), &[], &market(), &Rules { last_floor: None, ..plain_rules() }, BuildOptions::default()).unwrap();
    // hand calculation, cell phi 0.2, mean 8%, tau 6%, sigma 12%, n = 4:
    // mean variance factor = (1 + 2 (3/4) 0.2 + 2 (1/2) 0.04 + 2 (1/4) 0.008) / ((1 - 0.04) 4) = 1.344 / 3.84 = 0.35
    // weight of the history w = 0.0036 / (0.0036 + 0.0144 x 0.35) = 5/12
    // floor on (-10% -> 0): history mean 7.5%; company mean = 5/12 x 7.5% + 7/12 x 8% = 7.7917%; start = mu + 0.2 (0 - mu) = 0.8 mu
    let mu_on = 5.0 / 12.0 * 0.075 + 7.0 / 12.0 * 0.08;
    assert!((on.summary.growth_company_mean - mu_on).abs() < 1e-9, "{}", on.summary.growth_company_mean);
    assert!((on.summary.growth_start - 0.8 * mu_on).abs() < 1e-9, "{}", on.summary.growth_start);
    // the old rule: history mean 5%; company mean 6.75%; start = 6.75% + 0.2 (-10% - 6.75%) = 3.4%
    let mu_off = 5.0 / 12.0 * 0.05 + 7.0 / 12.0 * 0.08;
    assert!((off.summary.growth_company_mean - mu_off).abs() < 1e-9);
    assert!((off.summary.growth_start - (mu_off + 0.2 * (-0.10 - mu_off))).abs() < 1e-9);
    // a base year with growth above the floor is not changed
    let up = build(&inputs(), &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    let up_off = build(&inputs(), &table(), &[], &market(), &Rules { last_floor: None, ..plain_rules() }, BuildOptions::default()).unwrap();
    assert_eq!(up.summary.growth_start, up_off.summary.growth_start);
    // the rule is reachable: `last_floor=-1` is off, `last_floor=0.02` floors at 2%
    assert_eq!(parse_rules("last_floor=-1").unwrap().last_floor, None);
    assert_eq!(parse_rules("last_floor=0.02").unwrap().last_floor, Some(0.02));
    assert_eq!(Rules::default().last_floor, Some(0.0));
}

#[test]
fn the_margin_has_its_own_weight_for_the_table_shock() {
    // the margin history is constant (5 values, 4 residuals of 0): the shock size is sqrt(nu / (nu + 4)) x the table 9%
    let sigma_of = |r: Rules| -> f64 {
        let m = build(&inputs(), &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
        let sec = m.toml.split("[process.margin]").nth(1).unwrap();
        let line = sec.lines().find(|l| l.starts_with("sigma = ")).unwrap();
        line.trim_start_matches("sigma = ").parse().unwrap()
    };
    let base = Rules { own_sigma_nu: 4.0, level_kappa: 0.0, ..Rules::default() };
    // by default the margin follows `own_sigma_nu` = 4: sqrt(4 / 8) x 0.09 = 0.063640
    assert!((sigma_of(base.clone()) - 0.09 * 0.5f64.sqrt()).abs() < 1e-5);
    // `margin_sigma_nu` = 1: sqrt(1 / 5) x 0.09 = 0.040249; the growth keeps its own weight
    assert!((sigma_of(Rules { margin_sigma_nu: Some(1.0), ..base.clone() }) - 0.09 * 0.2f64.sqrt()).abs() < 1e-5);
    assert_eq!(parse_rules("margin_sigma_nu=1").unwrap().margin_sigma_nu, Some(1.0));
    assert_eq!(Rules::default().margin_sigma_nu, None);
}

#[test]
fn the_halves_of_the_point_in_time_test_split_the_tickers_by_a_fixed_rule() {
    // FNV-1a 64 bit: each step is `h = (h xor byte) x prime` with an odd prime, so the lowest bit of the result is the xor of
    // the lowest bit of the offset basis (1) and of the lowest bits of the bytes: A (0x41) -> 0; B (0x42) -> 1; IBM (0x49, 0x42, 0x4d) -> 1 ^ 1 ^ 0 ^ 1 = 1
    assert_eq!((ticker_half("A"), ticker_half("B"), ticker_half("IBM")), (0, 1, 1));
}

#[test]
fn the_floor_of_the_base_year_has_clear_edges() {
    let run = |i: &AutoInputs, floor: Option<f64>| build(i, &table(), &[], &market(), &Rules { last_floor: floor, ..plain_rules() }, BuildOptions::default());
    let with_last = |f: &dyn Fn(f64) -> f64| {
        let mut i = inputs();
        let prev = i.years[4].revenue.unwrap();
        i.years[5].revenue = Some(f(prev));
        i.balance = Some(BalanceData::from_year(i.years.last().unwrap()));
        i
    };
    // growth exactly at the floor (0%): nothing changes
    let flat = with_last(&|p| p);
    let (a, b) = (run(&flat, Some(0.0)).unwrap(), run(&flat, None).unwrap());
    assert_eq!((a.summary.growth_start, a.summary.growth_company_mean), (b.summary.growth_start, b.summary.growth_company_mean));
    // a base year with no growth value (a jump of +200% with revenue above $1B is left out of the history): nothing changes
    let jump = with_last(&|p| 3.0 * p);
    let (a, b) = (run(&jump, Some(0.0)).unwrap(), run(&jump, None).unwrap());
    assert_eq!((a.summary.growth_start, a.summary.growth_company_mean), (b.summary.growth_start, b.summary.growth_company_mean));
    // a negative revenue in the base year: the floor does not change the answer (an error or the same numbers)
    let neg = with_last(&|p| -p);
    match (run(&neg, Some(0.0)), run(&neg, None)) {
        (Ok(a), Ok(b)) => assert_eq!(a.summary.growth_start, b.summary.growth_start),
        (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string()),
        _ => panic!("the floor changes whether a model is made for a negative revenue"),
    }
    // the parser: a floor must be a finite number; a weight of the margin shock must be 0 or more
    assert!(parse_rules("last_floor=nan").is_err() && parse_rules("last_floor=inf").is_err());
    assert!(parse_rules("margin_sigma_nu=-1").is_err() && parse_rules("margin_sigma_nu=nan").is_err());
    assert_eq!(parse_rules("margin_sigma_nu=0").unwrap().margin_sigma_nu, Some(0.0));
}

/// The charge of each year of the model at the means: (FCFF without the charge - FCFF with it) / revenue.
fn charge_path(with_charge: &AutoModel, without: &AutoModel) -> Vec<f64> {
    use crate::dcf::paths::{F_FCFF, F_REV};
    let a = PathEngine::new(&spec_of(with_charge)).unwrap().deterministic().unwrap();
    let b = PathEngine::new(&spec_of(without)).unwrap().deterministic().unwrap();
    (0..10).map(|t| (b.at(0, t, F_FCFF) - a.at(0, t, F_FCFF)) / a.at(0, t, F_REV)).collect()
}

fn inputs_with_spend() -> AutoInputs {
    let mut i = inputs();
    for y in i.years.iter_mut() {
        y.acquisitions = Some(0.02 * y.revenue.unwrap());
    }
    i
}

#[test]
fn the_charge_fades_with_the_growth_that_it_pays_for() {
    let i = inputs_with_spend();
    let run = |r: Rules| build(&i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
    let off = run(Rules { acq_w: false, ..plain_rules() });
    let on = run(plain_rules());
    let c0 = on.summary.acq_ratio;
    assert!(c0 > 0.0 && c0 < 0.02);
    // the company mean (8.83%) is above L (4.0%): the drift ramps over 12 years (no fade cells: half-life 4 years); the charge
    // is the process "ramp" from c0 in year 1 to 0 in year 12: c(t) = c0 (1 - (t - 1) / 11), the same timing as the drift
    let s = spec_of(&on);
    let d = s.process["growth"].drift.as_ref().unwrap();
    let ch = &s.process["charge"];
    assert_eq!((d.ramp_years, ch.ramp_years, ch.shape.as_str()), (12, Some(12), "ramp"));
    assert!((draw_mean(&ch.start) - c0).abs() < 1e-12 && ch.target.mean() == 0.0);
    let path = charge_path(&on, &off);
    for t in [1usize, 6, 10] {
        let want = c0 * (1.0 - (t as f64 - 1.0) / 11.0);
        assert!((path[t - 1] - want).abs() < 1e-9, "year {t}: {} {want}", path[t - 1]);
    }
    // the same timing as the drift of the growth: the share of the excess left in year t is the same for both
    let sched = crate::dcf::driver::Ramp { start: d.start, end: d.end, ramp_years: d.ramp_years }.schedule(10);
    for t in 0..10 {
        assert!(((1.0 - path[t] / c0) - sched[t] / d.end).abs() < 1e-9, "year {}", t + 1);
    }
    // acq_fade=0: the constant charge
    let flat = run(Rules { acq_fade: false, ..plain_rules() });
    assert!(charge_path(&flat, &off).iter().all(|c| (c - c0).abs() < 1e-9));
    assert!(ev_at_means(&on) > ev_at_means(&flat) && ev_at_means(&on) < ev_at_means(&off));
    // a company mean at or below the level (no drift): the charge is constant
    let low = run(Rules { long_cap: Some(0.20), ..plain_rules() });
    let low_off = run(Rules { long_cap: Some(0.20), acq_w: false, ..plain_rules() });
    assert!(spec_of(&low).process["growth"].drift.is_none());
    assert!(charge_path(&low, &low_off).iter().all(|c| (c - low.summary.acq_ratio).abs() < 1e-9 && *c > 0.0));
    // acq_charge and acq_f are never faded
    let full = run(Rules { acq_charge: true, ..plain_rules() });
    assert!(charge_path(&full, &off).iter().all(|c| (c - 0.02).abs() < 1e-9));
    assert!(!parse_rules("acq_fade=0").unwrap().acq_fade && Rules::default().acq_fade);
}

#[test]
fn the_explicit_charge_rules_win_over_acq_w_off_and_zero_spend_gives_no_charge() {
    let i = inputs_with_spend();
    let run = |r: Rules| build(&i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
    // acq_w=0 does not turn off acq_charge or acq_f
    let m = run(parse_rules("acq_w=0,acq_charge=1,acq_persist=off,own_sigma_nu=0,level_kappa=0").unwrap());
    assert!((m.summary.acq_ratio - 0.02).abs() < 1e-12);
    let m = run(parse_rules("acq_w=0,acq_f=0.5,acq_mult=2,acq_persist=off,own_sigma_nu=0,level_kappa=0").unwrap());
    assert!((m.summary.acq_ratio - 0.01).abs() < 1e-12);
    // no spend with the default on: no charge, no process
    let none = build(&inputs(), &table(), &[], &market(), &plain_rules(), BuildOptions::default()).unwrap();
    assert_eq!(none.summary.acq_ratio, 0.0);
    assert!(!spec_of(&none).process.contains_key("charge"));
}

#[test]
fn off_is_allowed_for_long_run_and_the_flags_only_and_the_old_model_runs_from_the_parser() {
    assert_eq!(parse_rules("long_run=off").unwrap().long_run, None);
    assert!(!parse_rules("acq_w=off").unwrap().acq_w && !parse_rules("acq_fade=off").unwrap().acq_fade && !parse_rules("stub=off").unwrap().stub);
    for bad in ["erp=off", "terminal_growth=off", "rf=off", "years=off", "long_cap=off", "acq_f=off", "growth_w=off"] {
        assert!(parse_rules(bad).is_err(), "{bad}");
    }
    // `long_run=-1` and `long_run=off` give the old model (terminal mean 2.25%, fade to the yield) in a model
    let i = inputs();
    for text in ["long_run=-1", "long_run=off", "long_run=0"] {
        let r = Rules { own_sigma_nu: 0.0, level_kappa: 0.0, ..parse_rules(text).unwrap() };
        let m = build(&i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
        let t = m.toml.split("[terminal]").nth(1).unwrap().to_string();
        let tg: f64 = t.split("mean = ").nth(1).unwrap().split(',').next().unwrap().trim().parse().unwrap();
        assert!((tg - 0.0225).abs() < 1e-12, "{text}");
        let d = spec_of(&m).process["growth"].drift.clone().unwrap();
        assert!((d.end - (DEFAULT_RF - 0.0883333)).abs() < 1e-6, "{text}");
    }
    // the order of terminal_growth and long_run matters
    assert_eq!(parse_rules("terminal_growth=0.03,long_run=0.04").unwrap().long_run, Some(0.04));
    assert_eq!(parse_rules("long_run=0.04,terminal_growth=0.03").unwrap().long_run, None);
}

#[test]
fn terminal_rf_sets_the_fade_level_and_the_terminal_mean_to_the_yield() {
    let i = inputs();
    let m = build(&i, &table(), &[], &market(), &Rules { terminal_rf: true, ..plain_rules() }, BuildOptions::default()).unwrap();
    // no curve: the yield is the default 4.2%; it wins over long_run = 4.0% for both numbers
    let t = m.toml.split("[terminal]").nth(1).unwrap().to_string();
    let tg: f64 = t.split("mean = ").nth(1).unwrap().split(',').next().unwrap().trim().parse().unwrap();
    assert!((tg - DEFAULT_RF).abs() < 1e-12);
    assert!((spec_of(&m).process["growth"].drift.as_ref().unwrap().end - (DEFAULT_RF - 0.0883333)).abs() < 1e-6);
    // long_cap keeps its priority for the fade level
    let c = build(&i, &table(), &[], &market(), &Rules { terminal_rf: true, long_cap: Some(0.03), ..plain_rules() }, BuildOptions::default()).unwrap();
    assert!((spec_of(&c).process["growth"].drift.as_ref().unwrap().end - (0.03 - 0.0883333)).abs() < 1e-6);
}

#[test]
fn the_margin_scan_needs_all_five_links_of_the_six_years() {
    use super::super::panel::{CompanyPanel, PanelRow};
    // 7 years of one company; the margin of each year is 10% (fcf 100 on revenue 1,000M). The cases are the targets t = 5 and t = 6.
    // A case needs 5 margins in sequence before the target and the target: the rows t-5 .. t, 5 links (`prev_ok` of the rows t-4 .. t).
    // Row 1 has `prev_ok = false` (a gap between the row 0 and the row 1). The case t = 5 uses the rows 0 to 5: its first
    // link is the row 1, so it is out. The case t = 6 uses the rows 1 to 6: its links are the rows 2 to 6, so it is in.
    // Before the correction the window of the case t = 5 left out the row 1 (4 of 5 links), so the scan had 2 cases.
    let row = |k: usize| PanelRow {
        end: format!("{}-12-31", 2015 + k),
        year: 2015 + k as i32,
        revenue: 1e9,
        operating_income: None,
        operating_cash_flow: None,
        capex: None,
        fcf: Some(1e8),
        sbc: None,
        acquisitions: None,
        prev_ok: k != 1,
        long_year: false,
        growth: Some(0.0),
        jump: false,
        acq_likely: false,
        glitch: false,
    };
    let c = CompanyPanel { ticker: "T".into(), name: "T".into(), template: "standard".into(), rows: (0..7).map(row).collect() };
    let scan = sigma_nu_scan(&[c.clone()], &table(), &plain_rules(), None, &[4.0]);
    assert_eq!(scan[0][3].0, 1);
    // with no gap both cases count
    let mut full = c;
    full.rows[1].prev_ok = true;
    assert_eq!(sigma_nu_scan(&[full], &table(), &plain_rules(), None, &[4.0])[0][3].0, 2);
}

#[test]
fn the_second_acquisition_line_is_added_only_when_it_differs_or_the_first_has_none() {
    let year = |a: Option<f64>, b: Option<f64>| YearData { acquisitions: a, acquisitions_second: b, ..Default::default() };
    // different value: added
    assert_eq!(year(Some(1.5e9), Some(13.0e9)).acq_spend(true), 14.5e9);
    // same value (a tag renamed between 10-Ks, EQT FY2021): not added
    assert_eq!(year(Some(1_030_239_000.0), Some(1_030_239_000.0)).acq_spend(true), 1_030_239_000.0);
    // first line missing: the second is used (ZBH, MATX, LUMN, WT)
    assert_eq!(year(None, Some(2.0e8)).acq_spend(true), 2.0e8);
    // no second line, no line: as before
    assert_eq!(year(Some(3.0e8), None).acq_spend(true), 3.0e8);
    assert_eq!(year(None, None).acq_spend(true), 0.0);
    // acq_second=0: the first line only (the old behaviour)
    assert_eq!(year(Some(1.5e9), Some(13.0e9)).acq_spend(false), 1.5e9);
    assert_eq!(year(None, Some(2.0e8)).acq_spend(false), 0.0);
    assert!(Rules::default().acq_second && !parse_rules("acq_second=0").unwrap().acq_second);
}

#[test]
fn the_charge_reads_the_second_line_and_acq_second_off_gives_the_first_line_only() {
    let mut i = inputs();
    for y in i.years.iter_mut() {
        y.acquisitions = Some(0.01 * y.revenue.unwrap());
        y.acquisitions_second = Some(0.03 * y.revenue.unwrap());
    }
    let run = |i: &AutoInputs, r: Rules| build(i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
    let both = run(&i, Rules { acq_charge: true, ..plain_rules() });
    let first = run(&i, Rules { acq_charge: true, acq_second: false, ..plain_rules() });
    assert!((both.summary.acq_ratio - 0.04).abs() < 1e-12, "{}", both.summary.acq_ratio);
    assert!((first.summary.acq_ratio - 0.01).abs() < 1e-12);
    // the same value in both lines (a renamed tag): one purchase
    for y in i.years.iter_mut() {
        y.acquisitions_second = y.acquisitions;
    }
    let same = run(&i, Rules { acq_charge: true, ..plain_rules() });
    assert!((same.summary.acq_ratio - 0.01).abs() < 1e-12);
}

#[test]
fn the_kept_share_of_the_charge_has_no_step_at_a_company_mean_equal_to_l() {
    let i = inputs_with_spend();
    let run = |r: Rules| build(&i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap();
    let off = run(Rules { acq_w: false, ..plain_rules() });
    let base = run(Rules { acq_keep: true, ..plain_rules() });
    let mu = base.summary.growth_company_mean;
    let c0 = base.summary.acq_ratio;
    assert!(mu > 0.08 && c0 > 0.0);
    // closed form: with L = 4%, the charge falls from c0 in year 1 to c0 L / mu in year 12 (ramp of 12 years)
    let path = charge_path(&base, &off);
    let kept = 0.04 / mu;
    for t in [1usize, 6, 10] {
        let want = c0 * (1.0 - (1.0 - kept) * (t as f64 - 1.0) / 11.0);
        assert!((path[t - 1] - want).abs() < 1e-9, "year {t}: {} {want}", path[t - 1]);
    }
    // the old rule: to zero
    let old = charge_path(&run(plain_rules()), &off);
    assert!(old[9] < path[9] && (old[9] - c0 * (1.0 - 9.0 / 11.0)).abs() < 1e-9);
    // no step at mu = L: L just below and just above the company mean give the same charge path (within 1e-5 of c0)
    let below = |lc: f64| {
        let on = run(Rules { acq_keep: true, long_cap: Some(lc), ..plain_rules() });
        let no = run(Rules { acq_keep: true, long_cap: Some(lc), acq_w: false, ..plain_rules() });
        (charge_path(&on, &no), on.summary.acq_ratio)
    };
    let (lo, c_lo) = below(mu + 1e-7);
    let (hi, c_hi) = below(mu - 1e-7);
    assert!((c_lo - c_hi).abs() < 1e-12);
    for t in 0..10 {
        assert!((lo[t] - hi[t]).abs() < 1e-5 * c0, "year {}: {} {}", t + 1, lo[t], hi[t]);
    }
    // without acq_keep the same two paths differ by the whole charge in year 10 (the step)
    let step = |lc: f64| {
        let on = run(Rules { long_cap: Some(lc), ..plain_rules() });
        let no = run(Rules { long_cap: Some(lc), acq_w: false, ..plain_rules() });
        charge_path(&on, &no)
    };
    assert!((step(mu + 1e-7)[9] - step(mu - 1e-7)[9]).abs() > 0.5 * c0 * (1.0 - 9.0 / 11.0) * 0.5);
    assert!(!Rules::default().acq_keep && parse_rules("acq_keep=1").unwrap().acq_keep);
}

#[test]
fn the_charge_base_can_be_the_persistence_fit_of_the_past_spend() {
    let run = |spend: f64, r: Rules| {
        let mut i = inputs();
        for y in i.years.iter_mut() {
            y.acquisitions = Some(spend * y.revenue.unwrap());
        }
        build(&i, &table(), &[], &market(), &r, BuildOptions::default()).unwrap()
    };
    // the paired charge (`acq_w`; w = 1 here, no fade) with the fit a + b x past
    let paired = |p: Option<(f64, f64)>| Rules { acq_w: true, acq_fade: false, acq_persist: p, ..plain_rules() };
    let w = run(0.02, paired(None)).summary.acq_ratio / 0.02;
    assert!(w > 0.0 && w <= 1.0);
    // off: w x the past spend
    assert!((run(0.02, paired(None)).summary.acq_ratio - w * 0.02).abs() < 1e-12);
    // on: w x (a + b x past) = w x (0.0175 + 0.3925 x 0.02)
    let on = paired(Some((0.0175, 0.3925)));
    assert!((run(0.02, on.clone()).summary.acq_ratio - w * (0.0175 + 0.3925 * 0.02)).abs() < 1e-12);
    // the past spend is capped at 25% of revenue
    assert!((run(0.40, on.clone()).summary.acq_ratio - w * (0.0175 + 0.3925 * 0.25)).abs() < 1e-12);
    // the default is the fit through the origin (record A-002, 2026-11-21): a past spend of 0 gives a charge of 0
    assert_eq!(Rules::default().acq_persist, Some((0.0, 0.5226)));
    assert_eq!(run(0.0, paired(Some(ACQ_PERSIST_DEFAULT))).summary.acq_ratio, 0.0);
    assert!((run(0.02, paired(Some(ACQ_PERSIST_DEFAULT))).summary.acq_ratio - w * 0.5226 * 0.02).abs() < 1e-12);
    // the old intercept rule at a spend of 0 charges 1.75% x w
    assert!((run(0.0, on.clone()).summary.acq_ratio - w * 0.0175).abs() < 1e-12);
    // acq_charge and acq_f use the past spend, not the fit
    let explicit = Rules { acq_charge: true, acq_persist: Some((0.0175, 0.3925)), ..plain_rules() };
    assert!((run(0.02, explicit).summary.acq_ratio - 0.02).abs() < 1e-12);
    let f = Rules { acq_f: Some(0.5), acq_persist: Some((0.0175, 0.3925)), ..plain_rules() };
    assert!((run(0.02, f).summary.acq_ratio - 0.5 * 0.02).abs() < 1e-12);
    // the rule keys change one number of the default, in any order; after `off`, one number set takes the default of
    // the other (0.0 or 0.5226, not 1.0); `acq_persist=off` gives the past spend, and any other value of that key is an error
    assert_eq!(parse_rules("acq_persist_b=0.4,acq_persist_a=0.01").unwrap().acq_persist, Some((0.01, 0.4)));
    assert_eq!(parse_rules("acq_persist_b=0.4").unwrap().acq_persist, Some((0.0, 0.4)));
    assert_eq!(parse_rules("acq_persist_a=0.0175").unwrap().acq_persist, Some((0.0175, 0.5226)));
    assert!(parse_rules("acq_persist=off").unwrap().acq_persist.is_none());
    assert_eq!(parse_rules("acq_persist=off,acq_persist_a=0.01").unwrap().acq_persist, Some((0.01, 0.5226)));
    assert_eq!(parse_rules("acq_persist=off,acq_persist_b=0.4").unwrap().acq_persist, Some((0.0, 0.4)));
    assert!(parse_rules("acq_persist=1").is_err());
}

#[test]
fn the_persistence_diagnosis_gives_the_fit_of_a_known_line() {
    use crate::dcf::auto_pit::{ols, render_acq_persistence};
    let line: Vec<(f64, f64)> = (0..10).map(|k| (k as f64 * 0.01, 0.01 + 0.5 * k as f64 * 0.01)).collect();
    let ((a, b), _) = ols(&line).unwrap();
    assert!((a - 0.01).abs() < 1e-12 && (b - 0.5).abs() < 1e-12);
    let origin: Vec<(f64, f64)> = (1..10).map(|k| (k as f64 * 0.01, 3.0 * k as f64 * 0.01)).collect();
    assert!((ols(&origin).unwrap().1 - 3.0).abs() < 1e-12);
    assert!(ols(&[(0.0, 1.0), (0.0, 2.0), (0.0, 3.0)]).is_none());
    assert!(render_acq_persistence(&[]).contains("PERSISTENCE OF THE ACQUISITION SPEND"));
}

#[test]
fn the_price_of_a_past_date_is_the_last_close_on_or_before_it() {
    let s: Vec<(String, f64)> = [("2026-11-02", 10.0), ("2026-11-03", 11.0), ("2026-11-06", 12.0), ("2026-11-09", 13.0)].iter().map(|(d, c)| (d.to_string(), *c)).collect();
    assert_eq!(close_on_or_before(&s, "2026-11-03"), Some(("2026-11-03".into(), 11.0)));
    // a Saturday and a Sunday: the Friday close
    assert_eq!(close_on_or_before(&s, "2026-11-07"), Some(("2026-11-06".into(), 12.0)));
    assert_eq!(close_on_or_before(&s, "2026-11-08"), Some(("2026-11-06".into(), 12.0)));
    // a date after the last close: that close, while it is not more than 10 days old
    assert_eq!(close_on_or_before(&s, "2026-11-19"), Some(("2026-11-09".into(), 13.0)));
    assert_eq!(close_on_or_before(&s, "2026-11-20"), None);
    // a date before the first close
    assert_eq!(close_on_or_before(&s, "2026-11-01"), None);
    // a series that is not sorted gives the same answers
    let u: Vec<(String, f64)> = [s[3].clone(), s[0].clone(), s[2].clone(), s[1].clone()].to_vec();
    assert_eq!(close_on_or_before(&u, "2026-11-07"), Some(("2026-11-06".into(), 12.0)));
    assert_eq!(close_on_or_before(&u, "2026-11-19"), Some(("2026-11-09".into(), 13.0)));
}

#[test]
fn market_at_takes_the_close_of_the_price_date_and_market_from_cache_the_last_close() {
    let dir = std::env::temp_dir().join(format!("fa_market_at_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("ZZ")).unwrap();
    let rows: Vec<String> = [("2026-11-02", 10.0), ("2026-11-03", 11.0), ("2026-11-06", 12.0)]
        .iter()
        .map(|(d, c)| format!(r#"{{"date":"{d}","open":{c},"high":{c},"low":{c},"close":{c},"volume":1}}"#))
        .collect();
    std::fs::write(dir.join("ZZ").join("prices.json"), format!("[{}]", rows.join(","))).unwrap();
    let rules = plain_rules();
    let past = market_at(&dir, "ZZ", "2026-11-04", None, Some("2026-11-04"), &rules, None, None);
    assert_eq!(past.price, Some(11.0));
    assert!(past.price_source.contains("2026-11-03"), "{}", past.price_source);
    assert_eq!(past.as_of, "2026-11-04");
    // a price of the caller wins; no price date: the last close
    assert_eq!(market_at(&dir, "ZZ", "2026-11-04", Some(5.0), Some("2026-11-04"), &rules, None, None).price, Some(5.0));
    assert_eq!(market_from_cache(&dir, "ZZ", "2026-11-04", None, &rules, None, None).price, Some(12.0));
    std::fs::remove_dir_all(&dir).ok();
}

/// The SEC facts of a company with six fiscal years 2020 to 2025 (10-K filed on 15 Feb of the next year) and 100M shares.
fn live_facts() -> mapcheck::facts::CompanyFacts {
    let dur = |tag: &str, f: &dyn Fn(i32) -> f64| -> String {
        let rows: Vec<String> = (0..6).map(|k| format!(r#"{{"start":"{y}-01-01","end":"{y}-12-31","val":{v},"form":"10-K","filed":"{f}-02-15","fy":{y},"fp":"FY"}}"#, y = 2020 + k, v = f(k), f = 2021 + k)).collect();
        format!(r#""{tag}":{{"units":{{"USD":[{}]}}}}"#, rows.join(","))
    };
    let inst = |tag: &str, unit: &str, v: f64| -> String {
        let rows: Vec<String> = (0..6).map(|k| format!(r#"{{"end":"{y}-12-31","val":{v},"form":"10-K","filed":"{f}-02-15","fy":{y},"fp":"FY"}}"#, y = 2020 + k, f = 2021 + k)).collect();
        format!(r#""{tag}":{{"units":{{"{unit}":[{}]}}}}"#, rows.join(","))
    };
    let rev = |k: i32| 1e9 * 1.1f64.powi(k);
    let shares_w: Vec<String> = (0..6).map(|k| format!(r#"{{"start":"{y}-01-01","end":"{y}-12-31","val":1e8,"form":"10-K","filed":"{f}-02-15","fy":{y},"fp":"FY"}}"#, y = 2020 + k, f = 2021 + k)).collect();
    let text = format!(
        r#"{{"facts":{{"us-gaap":{{{},{},{},{},{},{},{},{}, "WeightedAverageNumberOfDilutedSharesOutstanding":{{"units":{{"shares":[{}]}}}}}}}}}}"#,
        dur("Revenues", &rev),
        dur("NetCashProvidedByUsedInOperatingActivities", &|k| 0.2 * rev(k)),
        dur("PaymentsToAcquirePropertyPlantAndEquipment", &|k| 0.05 * rev(k)),
        inst("CashAndCashEquivalentsAtCarryingValue", "USD", 5e7),
        inst("LongTermDebt", "USD", 2e8),
        inst("CommonStockSharesOutstanding", "shares", 1e8),
        dur("ShareBasedCompensation", &|k| 0.03 * rev(k)),
        inst("Assets", "USD", 2e9),
        shares_w.join(",")
    );
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"))
}

/// The live case: a 25:1 split on 2026-04-06, after the last 10-K (period end 2025-12-31). The share counts of the 10-K
/// are before the split; the close of the store is split-adjusted. With the split table the model gives the same
/// P50 / price as a company with no split (the price divided by 25 and no split row would give a value 25 times too high).
#[test]
fn a_split_after_the_last_10k_does_not_change_the_value_over_the_price() {
    use super::super::auto_implied::deterministic_value;
    use crate::market::splits::Split;
    let reg = Registry::embedded().unwrap();
    let day = mapcheck::dates::parse("2026-11-21").unwrap();
    let report = mapcheck::run::check_facts(&reg, "LIVE", &live_facts(), 8, day, true);
    let split = [Split { ticker: "LIVE".into(), ex_date: "2026-04-06".into(), old_rate: 1.0, new_rate: 25.0, factor: 25.0, kind: "forward".into(), sec_check: "match".into() }];
    let none = inputs_from_report(&reg, &report, &[], "2026-11-21");
    let with = inputs_from_report(&reg, &report, &split, "2026-11-21");
    assert_eq!(none.years.len(), 6, "{:?}", none.findings);
    let (b0, b1) = (none.balance.as_ref().unwrap(), with.balance.as_ref().unwrap());
    assert_eq!((b0.shares, b0.shares_outstanding), (Some(1e8), Some(1e8)));
    assert_eq!((b1.shares, b1.shares_outstanding), (Some(2.5e9), Some(2.5e9)));
    assert!(with.years.iter().all(|y| y.shares == Some(2.5e9)));
    let value = |i: &AutoInputs, price: f64| -> f64 {
        let m = Market { as_of: "2026-11-21".into(), price: Some(price), ..Default::default() };
        let model = build(i, &table(), &[], &m, &plain_rules(), BuildOptions::default()).unwrap_or_else(|e| panic!("{e:?}"));
        deterministic_value(&spec_of(&model)).unwrap().0
    };
    let p = 20.0;
    let ratio_no_split = value(&none, p) / p;
    // the store shows the adjusted close p / 25; the shares of the model are on the same basis
    let ratio_split = value(&with, p / 25.0) / (p / 25.0);
    assert!((ratio_split / ratio_no_split - 1.0).abs() < 1e-9, "{ratio_split} {ratio_no_split}");
    // the defect: the adjusted close with the share count of the 10-K
    let ratio_defect = value(&none, p / 25.0) / (p / 25.0);
    assert!(ratio_defect / ratio_no_split > 20.0, "{ratio_defect} {ratio_no_split}");
}

/// The SNDK case: a 1:8 split with the ex-date 2026-11-20, the price file ends on 2026-11-19 and is not adjusted for it.
/// The `to` date is the last date of the price file, so the split is not counted and the P50 / price is the same as with
/// no split. With `to` = today (the defect) the shares are 8 times too high and the P50 / price 8 times too low.
#[test]
fn a_split_after_the_last_price_date_does_not_change_the_value_over_the_price() {
    use super::super::auto_implied::deterministic_value;
    use super::super::share_basis::basis_to;
    use crate::market::splits::Split;
    let dir = std::env::temp_dir().join(format!("fa_sndk_like_{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join("LIVE")).unwrap();
    let row = |d: &str| format!(r#"{{"date":"{d}","open":20,"high":20,"low":20,"close":20,"volume":1}}"#);
    std::fs::write(dir.join("LIVE").join("prices.json"), format!("[{},{}]", row("2026-11-18"), row("2026-11-19"))).unwrap();
    std::fs::write(dir.join("LIVE").join("prices.meta.json"), r#"{"adjusted":"splits","last_date":"2026-11-19"}"#).unwrap();
    let to = basis_to(&dir, "LIVE", false);
    assert_eq!(to, "2026-11-19");

    let reg = Registry::embedded().unwrap();
    let day = mapcheck::dates::parse("2026-11-21").unwrap();
    let report = mapcheck::run::check_facts(&reg, "LIVE", &live_facts(), 8, day, true);
    let split = [Split { ticker: "LIVE".into(), ex_date: "2026-11-20".into(), old_rate: 1.0, new_rate: 8.0, factor: 8.0, kind: "forward".into(), sec_check: "match".into() }];
    let none = inputs_from_report(&reg, &report, &[], &to);
    let with = inputs_from_report(&reg, &report, &split, &to);
    assert_eq!(none.years, with.years);
    assert_eq!(none.balance, with.balance);
    let value = |i: &AutoInputs, price: f64| -> f64 {
        let m = Market { as_of: "2026-11-21".into(), price: Some(price), ..Default::default() };
        let model = build(i, &table(), &[], &m, &plain_rules(), BuildOptions::default()).unwrap_or_else(|e| panic!("{e:?}"));
        deterministic_value(&spec_of(&model)).unwrap().0
    };
    let p = 20.0;
    assert_eq!(value(&none, p) / p, value(&with, p) / p);
    // the defect: `to` = today counts the split that is not in the closes
    let wrong = inputs_from_report(&reg, &report, &split, "2026-11-21");
    assert_eq!(wrong.balance.as_ref().unwrap().shares, Some(8e8));
    let ratio_wrong = value(&wrong, p) / p;
    let ratio_right = value(&with, p) / p;
    assert!(ratio_wrong / ratio_right < 1.0 / 7.0, "{ratio_wrong} {ratio_right}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A truncated SPY file with fewer than 520 rows gives NO index: the beta is dropped (beta 1) and every score moves. A test
/// that cuts SPY at a date must keep 520 rows or more, or it measures this switch and not a look-ahead (the 2018-11-23 cut
/// of the store has 507 rows from 2016-11-18).
#[test]
fn a_short_spy_file_gives_no_index() {
    let dir = std::env::temp_dir().join(format!("fa_short_spy_{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join("SPY")).unwrap();
    let write = |n: usize| {
        let rows: Vec<String> = (0..n).map(|i| format!(r#"{{"date":"{}","open":1,"high":1,"low":1,"close":{},"volume":1}}"#, mapcheck::dates::format(mapcheck::dates::parse("2016-11-18").unwrap() + i as i64), 100.0 + i as f64)).collect();
        std::fs::write(dir.join("SPY").join("prices.json"), format!("[{}]", rows.join(","))).unwrap();
    };
    write(507);
    assert!(index_from_cache(&dir).is_none());
    write(520);
    assert_eq!(index_from_cache(&dir).map(|s| s.len()), Some(520));
    std::fs::remove_dir_all(&dir).ok();
}
