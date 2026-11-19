//! The bots. A bot is a struct that implements `Strategy`. The list below is the registry.

use super::strategy::Strategy;

pub mod baseline;
pub mod s01_orb_0dte;
pub mod s02_vwap_revert_0dte;
pub mod s03_gap_fade;
pub mod s04_gap_and_go;
pub mod s05_ic_0dte;
pub mod s06_last_hour_momentum;
pub mod s07_overnight_drift;
pub mod s08_trend_regime;
pub mod s09_rsi2_pullback;
pub mod s10_vix_regime;
pub mod s11_vrp_condor;
pub mod s12_turn_of_month;
pub mod s13_fomc_drift;
pub mod s14_spy_qqq_rotation;
pub mod s15_squeeze_breakout;
pub mod s16_donchian_breakout;
pub mod s17_capitulation_bounce;
pub mod s18_pin_butterfly;
pub mod s19_opex_cycle;
pub mod s20_walkforward_model;
pub mod s21_bollinger_fade;

type Maker = fn() -> Box<dyn Strategy>;

/// (name, is a template, constructor), sorted by name. Templates run only with `--include-templates`.
const BOTS: &[(&str, bool, Maker)] = &[
    ("baseline_weekly_call_spread", true, || Box::new(baseline::WeeklyCallSpreadBaseline)),
    ("s01_orb_0dte", false, || Box::<s01_orb_0dte::OrbZeroDte>::default()),
    ("s02_vwap_revert_0dte", false, || Box::<s02_vwap_revert_0dte::VwapRevert0Dte>::default()),
    ("s03_gap_fade", false, || Box::<s03_gap_fade::GapFade>::default()),
    ("s04_gap_and_go", false, || Box::<s04_gap_and_go::GapAndGo>::default()),
    ("s05_ic_0dte", false, || Box::<s05_ic_0dte::IcZeroDte>::default()),
    ("s06_last_hour_momentum", false, || Box::<s06_last_hour_momentum::LastHourMomentum>::default()),
    ("s07_overnight_drift", false, || Box::<s07_overnight_drift::OvernightDrift>::default()),
    ("s08_trend_regime", false, || Box::<s08_trend_regime::TrendRegime>::default()),
    ("s09_rsi2_pullback", false, || Box::<s09_rsi2_pullback::Rsi2Pullback>::default()),
    ("s10_vix_regime", false, || Box::<s10_vix_regime::VixRegime>::default()),
    ("s11_vrp_condor", false, || Box::<s11_vrp_condor::VrpCondor>::default()),
    ("s12_turn_of_month", false, || Box::<s12_turn_of_month::TurnOfMonth>::default()),
    ("s13_fomc_drift", false, || Box::<s13_fomc_drift::PreFomcDrift>::default()),
    ("s14_spy_qqq_rotation", false, || Box::new(s14_spy_qqq_rotation::SpyQqqRotation::rotation())),
    ("s14b_spy_trend_calls", false, || Box::new(s14_spy_qqq_rotation::SpyQqqRotation::spy_only())),
    ("s15_squeeze_breakout", false, || Box::<s15_squeeze_breakout::SqueezeBreakout>::default()),
    ("s16_donchian_breakout", false, || Box::<s16_donchian_breakout::DonchianBreakoutSpreads>::default()),
    ("s17_capitulation_bounce", false, || Box::<s17_capitulation_bounce::CapitulationBounce>::default()),
    ("s18_pin_butterfly", false, || Box::<s18_pin_butterfly::PinButterfly>::default()),
    ("s19_opex_cycle", false, || Box::<s19_opex_cycle::OpexCycle>::default()),
    ("s20_walkforward_model", false, || Box::<s20_walkforward_model::WalkForwardDirection>::default()),
    ("s21_bollinger_fade", false, || Box::<s21_bollinger_fade::BollingerFade>::default()),
];

/// Bot names in order; templates only if `include_templates`.
pub fn names(include_templates: bool) -> Vec<&'static str> {
    BOTS.iter().filter(|(_, template, _)| include_templates || !template).map(|(n, _, _)| *n).collect()
}

/// A fresh bot (a bot keeps state, so each run needs its own).
pub fn make(name: &str) -> Option<Box<dyn Strategy>> {
    BOTS.iter().find(|(n, _, _)| *n == name).map(|(_, _, make)| make())
}

/// The last `n` items of `a` (all items if there are fewer), like the Python slice `a[-n:]`.
pub fn tail<T>(a: &[T], n: usize) -> &[T] {
    &a[a.len().saturating_sub(n)..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_are_unique_and_sorted() {
        let all = names(true);
        let mut sorted = all.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(all, sorted);
        for n in &all {
            assert_eq!(make(n).unwrap().name(), *n);
        }
        assert!(!names(false).contains(&"baseline_weekly_call_spread"));
    }
}
