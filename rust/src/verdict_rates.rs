//! Market rates in a verdict (the rates path of `scripts/verdict.py`, in Rust).
//!
//! `--rf-date D`: the `rf` driver mean becomes the Treasury 10Y par yield on that date. The
//! output shows the inflation split of the rate (real rate and breakeven) and a check of the
//! terminal growth against the 5y5y forward inflation. Only `rf` changes: the growth drivers
//! are nominal and keep the model values.
//!
//! `--rating R` (needs `--rf-date`): the `kd_spread` driver mean becomes the cost of debt of
//! that rating minus the UST 10Y on the same date. The cost of debt is the Treasury HQM 10Y
//! spot yield plus the fixed step of the rating bucket (`bonds.rs`, records A-200 to A-203),
//! so the mean pre-tax cost of debt is that cost of debt.

use crate::dcf::model::ModelSpec;
use crate::rates::{obs_text, parse_on, round6, Rates, Res};
use crate::{BaseSplit, CreditContext, RateObservation, RatesContext};

fn replace_driver(model: &ModelSpec, name: &str, mean: f64, rationale: String) -> Res<ModelSpec> {
    let mut m = model.clone();
    let d = m.drivers.iter_mut().find(|d| d.name == name).ok_or_else(|| format!("Unknown driver {name:?}."))?;
    d.mean = mean;
    d.rationale = rationale;
    Ok(m)
}

/// A copy of the model whose `rf` driver mean is the value of `obs`. Std, bounds, distribution and
/// correlations stay. The rationale records the source, the date and the replaced mean.
pub fn with_risk_free(model: &ModelSpec, obs: &RateObservation) -> Res<ModelSpec> {
    let rf = model.driver("rf").map_err(|e| e.0)?;
    let rationale = format!("{}; replaces the model mean {:.2}%. Model rationale: {}", obs_text(obs), rf.mean * 100.0, rf.rationale);
    replace_driver(model, "rf", obs.value, rationale)
}

/// A copy of the model whose `kd_spread` driver mean is the value of `obs`.
pub fn with_kd_spread(model: &ModelSpec, obs: &RateObservation) -> Res<ModelSpec> {
    let kd = model.driver("kd_spread").map_err(|e| e.0)?;
    let rationale = format!("{}; replaces the model mean {:+.2}%. Model rationale: {}", obs_text(obs), kd.mean * 100.0, kd.rationale);
    replace_driver(model, "kd_spread", obs.value, rationale)
}

/// (model with the Treasury `rf`, the rate and inflation facts for the output).
pub fn rates_context(model: &ModelSpec, rates: &mut Rates, rf_date: &str, base_date: Option<&str>) -> Res<(ModelSpec, RatesContext)> {
    let on = parse_on(Some(rf_date))?;
    let obs = rates.treasury_yield("10Y", on, false)?;
    let obs_day = mapcheck::dates::parse(&obs.date).ok_or("bad date")?;
    let model_rf = model.driver("rf").map_err(|e| e.0)?.mean;
    let infl = rates.inflation_snapshot(Some(obs_day))?;
    let tg = model.driver("terminal_growth").map_err(|e| e.0)?.mean;
    let fwd = infl.forward_5y5y.value;
    let (real_10y, breakeven_10y) = (infl.real_10y.value, infl.breakeven_10y.value);
    let base = match base_date {
        None => None,
        Some(b) => {
            let b_nom = rates.treasury_yield("10Y", parse_on(Some(b))?, false)?;
            let b_day = mapcheck::dates::parse(&b_nom.date).ok_or("bad date")?;
            let b_infl = rates.inflation_snapshot(Some(b_day))?;
            Some(BaseSplit {
                date: b_nom.date.clone(),
                nominal_10y: b_nom.value,
                nominal_change: round6(obs.value - b_nom.value),
                real_change: round6(real_10y - b_infl.real_10y.value),
                breakeven_change: round6(breakeven_10y - b_infl.breakeven_10y.value),
            })
        }
    };
    let ctx = RatesContext {
        rf: obs.clone(),
        model_rf_mean: model_rf,
        rf_change_vs_model: round6(obs.value - model_rf),
        real_10y,
        breakeven_10y,
        forward_5y5y: fwd,
        cpi_12m: infl.cpi_12m.clone(),
        core_cpi_12m: infl.core_cpi_12m.clone(),
        terminal_growth_mean: tg,
        terminal_growth_real_vs_5y5y: round6(tg - fwd),
        base,
    };
    Ok((with_risk_free(model, &obs)?, ctx))
}

/// (model with the market `kd_spread` for `rating`, the credit facts for the output).
pub fn credit_context(model: &ModelSpec, rates: &mut Rates, rating: &str, rf_date: &str, rf: f64) -> Res<(ModelSpec, CreditContext)> {
    let on = parse_on(Some(rf_date))?;
    let spread = rates.spread_over_10y(rating, on)?;
    let hqm = rates.hqm_spot(crate::bonds::DEFAULT_MATURITY, on)?;
    let bucket = crate::bonds::rating_bucket(rating)?;
    let model_spread = model.driver("kd_spread").map_err(|e| e.0)?.mean;
    let tax = model.driver("tax_rate").map_err(|e| e.0)?.mean;
    let kd = round6(rf + spread.value);
    let ctx = CreditContext {
        rating: rating.to_string(),
        index: bucket.to_string(),
        kd_spread: spread.clone(),
        model_kd_spread: model_spread,
        kd_spread_change_vs_model: round6(spread.value - model_spread),
        hqm_spot: hqm.value,
        step: crate::bonds::rating_step(bucket),
        kd_pretax: kd,
        kd_after_tax: round6(kd * (1.0 - tax)),
        tax_rate: tax,
    };
    Ok((with_kd_spread(model, &spread)?, ctx))
}
