//! Generic analyses over a driver model: runs with overrides, one-at-a-time sensitivities
//! and the driver value that a price implies. One feature set for all models.

use super::driver::DriverDcfResult;
use super::model::{bisect, ModelSpec, Overrides};
use super::stats::{percentile, sorted_copy};
use super::DcfError;
use serde::Serialize;

/// Quantiles of a run without the attribution loop (`_quick` of the Python runners).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Quick {
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub tg_pinned: f64,
}

/// Run `spec` with `overrides`: same seed, no attribution.
pub fn quick(spec: &ModelSpec, overrides: &Overrides, n: usize, seed: u64) -> Result<Quick, DcfError> {
    let eng = spec.with_overrides(overrides)?.engine()?;
    let v = eng.run_paths(n, seed, &[])?;
    let s = sorted_copy(&v.per_share);
    Ok(Quick {
        p10: percentile(&s, 10.0),
        p25: percentile(&s, 25.0),
        p50: percentile(&s, 50.0),
        p75: percentile(&s, 75.0),
        p90: percentile(&s, 90.0),
        tg_pinned: v.tg_pinned_fraction,
    })
}

/// The full run (with attribution) of `spec` with `overrides`.
pub fn full(spec: &ModelSpec, overrides: &Overrides, n: usize, seed: u64) -> Result<DriverDcfResult, DcfError> {
    spec.with_overrides(overrides)?.engine()?.run(n, Some(seed))
}

/// Value per share with every driver at its mean.
pub fn at_means(spec: &ModelSpec, overrides: &Overrides) -> Result<f64, DcfError> {
    spec.with_overrides(overrides)?.engine()?.value_at_means()
}

/// P50 per share when the mean of `driver` is `value` (the other overrides in `base` stay).
pub fn p50_with_mean(spec: &ModelSpec, base: &Overrides, driver: &str, value: f64, n: usize, seed: u64) -> Result<f64, DcfError> {
    let mut o = base.clone();
    o.means.retain(|(d, _)| d != driver);
    o.means.push((driver.to_string(), value));
    Ok(quick(spec, &o, n, seed)?.p50)
}

/// The mean of `driver` on `[lo, hi]` for which the P50 per share is `price`. `None` when
/// the price is out of range. The P50 may rise or fall with the driver.
#[allow(clippy::too_many_arguments)]
pub fn implied_mean(
    spec: &ModelSpec,
    base: &Overrides,
    driver: &str,
    lo: f64,
    hi: f64,
    price: f64,
    n: usize,
    seed: u64,
) -> Result<Option<f64>, DcfError> {
    bisect(|x| p50_with_mean(spec, base, driver, x, n, seed), lo, hi, price)
}

/// One-at-a-time sensitivity: P50 for each value of one driver mean.
pub fn sensitivity(
    spec: &ModelSpec,
    base: &Overrides,
    driver: &str,
    values: &[f64],
    n: usize,
    seed: u64,
) -> Result<Vec<(f64, f64)>, DcfError> {
    values.iter().map(|v| Ok((*v, p50_with_mean(spec, base, driver, *v, n, seed)?))).collect()
}

// ---------------------------------------------------------------------------
// Checks at the driver means: the fade of growth and the committed spend
// ---------------------------------------------------------------------------

/// Revenue and margin after SBC and the M&A charge for each year at the driver means, for a
/// model with one segment. `growth_by_year` replaces the constant growth of years 2..N.
pub fn path_at_means(spec: &ModelSpec, growth_by_year: Option<&[f64]>) -> Result<(Vec<f64>, Vec<f64>), DcfError> {
    if spec.segments.len() != 1 {
        return Err(DcfError("this check needs a model with one revenue segment.".into()));
    }
    let eng = spec.engine()?;
    let at = |name: &str, t: usize| -> Result<f64, DcfError> {
        let d = eng.model.driver(name)?;
        Ok(d.schedule.as_ref().map_or(d.mean, |s| s[t - 1]))
    };
    let seg = &spec.segments[0];
    let growth = spec.driver(&seg.growth_driver)?.mean;
    let mut level = match seg.year1_growth {
        Some(y1) => seg.base_revenue * (1.0 + y1),
        None => seg.base_revenue * (1.0 + growth),
    };
    let (mut rev, mut margin) = (Vec::new(), Vec::new());
    for t in 1..=spec.projection_years {
        if t > 1 {
            level *= 1.0 + growth_by_year.map_or(growth, |g| g[t - 2]);
        }
        rev.push(level);
        margin.push(at("core_margin", t)? + at("margin_mix_drift", t)? - at("sbc_ratio", t)? - at("ma_recurrence", t)? * at("ma_ratio", t)?);
    }
    Ok((rev, margin))
}

/// Cost of capital with every driver at its mean.
pub fn wacc_at_means(spec: &ModelSpec) -> Result<f64, DcfError> {
    let m = |n: &str| -> Result<f64, DcfError> { Ok(spec.driver(n)?.mean) };
    Ok(m("equity_weight")? * (m("rf")? + m("beta")? * m("erp")?) + (1.0 - m("equity_weight")?) * (m("rf")? + m("kd_spread")?) * (1.0 - m("tax_rate")?))
}

/// Closed-form value per share at the driver means, with the growth path of years 2..N given.
pub fn hand_dcf_with_path(spec: &ModelSpec, growth_by_year: Option<&[f64]>) -> Result<f64, DcfError> {
    let wacc = wacc_at_means(spec)?;
    let (rev, margin) = path_at_means(spec, growth_by_year)?;
    let pv: f64 = rev.iter().zip(&margin).enumerate().map(|(i, (r, mg))| r * mg / (1.0 + wacc).powi(i as i32 + 1)).sum();
    let tg = spec.driver("terminal_growth")?.mean.min(wacc - super::driver::TG_WACC_GAP);
    let n = rev.len();
    let tv = rev[n - 1] * margin[n - 1] * (1.0 + tg) / (wacc - tg) / (1.0 + wacc).powi(n as i32);
    Ok((pv + tv - spec.net_debt) / spec.shares)
}

/// The fade check: growth falls linearly from a high value in year 2 to the terminal growth
/// mean in year N, and the revenue of year N stays the same as with the constant growth.
#[derive(Debug, Clone, Serialize)]
pub struct Fade {
    pub constant_growth: f64,
    pub constant_value: f64,
    pub constant_revenue_year5: f64,
    pub growth: Vec<f64>,
    pub value: f64,
    pub revenue_year5: f64,
}

pub fn fade_check(spec: &ModelSpec) -> Result<Fade, DcfError> {
    let g_name = &spec.segments.first().ok_or_else(|| DcfError("the model has no segment".into()))?.growth_driver;
    let g_mean = spec.driver(g_name)?.mean;
    let n = spec.projection_years - 1;
    if n < 2 {
        return Err(DcfError("the fade check needs a horizon of 3 years or more.".into()));
    }
    let end = spec.driver("terminal_growth")?.mean;
    let target = (1.0 + g_mean).powi(n as i32);
    let path = |g2: f64| (0..n).map(|i| g2 + (end - g2) * i as f64 / (n - 1) as f64).collect::<Vec<f64>>();
    let g2 = bisect(|x| Ok(path(x).iter().map(|g| 1.0 + g).product::<f64>()), end, 2.0, target)?
        .ok_or_else(|| DcfError("no fade path gives the same year-N revenue.".into()))?;
    let fade = path(g2);
    let (rev, _) = path_at_means(spec, None)?;
    let (fade_rev, _) = path_at_means(spec, Some(&fade))?;
    Ok(Fade {
        constant_growth: g_mean,
        constant_value: hand_dcf_with_path(spec, None)?,
        constant_revenue_year5: rev[4.min(rev.len() - 1)],
        value: hand_dcf_with_path(spec, Some(&fade))?,
        revenue_year5: fade_rev[4.min(fade_rev.len() - 1)],
        growth: fade,
    })
}

/// Committed spend against revenue. `first` is the spend of year 1, `total` the spend to the end
/// of `years` model years (for example 4.25); the rest is spent in equal parts in the whole years
/// 2..; `other_cost_share` is the share of revenue for all other cash costs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Commitments {
    pub first: f64,
    pub total: f64,
    pub years: f64,
    pub other_cost_share: f64,
}

impl Commitments {
    /// `FIRST:TOTAL:YEARS:OTHER`, for example `70.7e9:451.6e9:4.25:0.31`.
    pub fn parse(text: &str) -> Result<Commitments, DcfError> {
        let p: Vec<f64> = text.split(':').map(|x| x.parse::<f64>().map_err(|_| DcfError(format!("--commitments: bad number '{x}'")))).collect::<Result<_, _>>()?;
        match p[..] {
            [first, total, years, other] if years > 1.0 && total > first && first > 0.0 => Ok(Commitments { first, total, years, other_cost_share: other }),
            _ => Err(DcfError("--commitments: expected FIRST:TOTAL:YEARS:OTHER with YEARS above 1 and TOTAL above FIRST".into())),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitmentCheck {
    pub first_share_of_year1_revenue: f64,
    pub total_share_at_mean_growth: f64,
    pub cumulative_revenue_at_mean: f64,
    pub total_share_at_p10_growth: f64,
    pub cumulative_revenue_at_p10: f64,
    pub p10_growth: f64,
    pub margin_before_sbc_model: Vec<f64>,
    pub margin_before_sbc_committed: Vec<f64>,
    /// Effect on the value per share at the means where the committed spend gives the lower margin.
    pub gap_per_share: f64,
}

/// `g_draws` are the draws of the growth driver (for the P10 growth).
pub fn commitment_check(spec: &ModelSpec, c: &Commitments, g_draws: &[f64]) -> Result<CommitmentCheck, DcfError> {
    let (rev, margin) = path_at_means(spec, None)?;
    let eng = spec.engine()?;
    let whole = c.years.floor() as usize;
    let frac = c.years - whole as f64;
    if whole + 1 > rev.len() || whole < 2 {
        return Err(DcfError("--commitments: YEARS does not fit the horizon of the model.".into()));
    }
    let cumulate = |r: &[f64]| r[..whole].iter().sum::<f64>() + frac * r[whole];
    let cum = cumulate(&rev);
    let g10 = percentile(&sorted_copy(g_draws), 10.0);
    let low: Vec<f64> = (0..=whole).map(|t| rev[0] * (1.0 + g10).powi(t as i32)).collect();
    let cum_low = cumulate(&low);
    let rest = (c.total - c.first) / (c.years - 1.0);
    let spend: Vec<f64> = (0..whole).map(|i| if i == 0 { c.first } else { rest }).collect();
    let sbc = eng.model.driver("sbc_ratio")?;
    let sbc_t = |t: usize| sbc.schedule.as_ref().map_or(sbc.mean, |s| s[t]);
    let pre_sbc: Vec<f64> = (0..whole).map(|i| margin[i] + sbc_t(i)).collect();
    let implied: Vec<f64> = (0..whole).map(|i| 1.0 - spend[i] / rev[i] - c.other_cost_share).collect();
    let wacc = wacc_at_means(spec)?;
    let gap: f64 = (0..whole).map(|i| (implied[i] - pre_sbc[i]).min(0.0) * rev[i] / (1.0 + wacc).powi(i as i32 + 1)).sum();
    Ok(CommitmentCheck {
        first_share_of_year1_revenue: c.first / rev[0],
        total_share_at_mean_growth: c.total / cum,
        cumulative_revenue_at_mean: cum,
        total_share_at_p10_growth: c.total / cum_low,
        cumulative_revenue_at_p10: cum_low,
        p10_growth: g10,
        margin_before_sbc_model: pre_sbc,
        margin_before_sbc_committed: implied,
        gap_per_share: gap / spec.shares,
    })
}

/// The price-value fixed point of the equity weight at the driver means: with the weight `w`
/// the value per share gives a market equity value, and that gives a new weight
/// `E / (E + net debt)`. The lines are the text of the output (at most 20 steps).
pub fn ev_fixed_point_lines(spec: &ModelSpec, start: f64) -> Result<Vec<String>, DcfError> {
    let mut lines = Vec::new();
    let (mut w, mut last) = (start, start);
    for i in 1..=20 {
        let ps = at_means(spec, &Overrides::mean("equity_weight", w))?;
        let equity = ps * spec.shares;
        let w_new = equity / (equity + spec.net_debt);
        lines.push(format!("  iter {i}: E/V in {w:.4} -> value/share ${ps:7.2} -> E/V out {w_new:.4}"));
        last = w_new;
        if (w_new - w).abs() < 5e-4 {
            break;
        }
        w = w_new;
    }
    lines.push(format!("  Fixed point E/V ~ {last:.3} (model mean {:.2}).", spec.driver("equity_weight")?.mean));
    Ok(lines)
}

/// The terminal reinvestment at the driver means (the M&A charge of the last year), and the return
/// on that reinvestment that supports the terminal growth: `g = RR x ROIC`.
pub fn reinvestment_lines(spec: &ModelSpec) -> Result<Vec<String>, DcfError> {
    let eng = spec.engine()?;
    let last = |n: &str| -> Result<f64, DcfError> {
        let d = eng.model.driver(n)?;
        Ok(d.schedule.as_ref().map_or(d.mean, |s| s[s.len() - 1]))
    };
    let f = spec.driver("ma_recurrence")?.mean;
    let (core, ma, tg) = (spec.driver("core_margin")?.mean, spec.driver("ma_ratio")?.mean, spec.driver("terminal_growth")?.mean);
    let (drift_t, sbc_t) = (last("margin_mix_drift")?, last("sbc_ratio")?);
    let reinv = f * ma;
    let fcff_t = core + drift_t - sbc_t - reinv;
    let nopat = fcff_t + reinv;
    let rr = reinv / nopat;
    let roic = tg / rr;
    Ok(vec![
        format!(
            "  Terminal reinvestment = f x ma_ratio = {f:.2} x {ma:.3} = {:.2}% of revenue; terminal FCFF margin {:.2}%; RR = {:.1}% of pre-reinvestment cash earnings ({:.2}%).",
            reinv * 100.0,
            fcff_t * 100.0,
            rr * 100.0,
            nopat * 100.0
        ),
        format!("  Sustaining terminal growth {:.2}% on that reinvestment implies ROIC = tg/RR = {:.1}%.", tg * 100.0, roic * 100.0),
    ])
}
