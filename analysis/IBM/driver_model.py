"""IBM driver-based DCF model — drivers, correlations, and capital structure.

Every parameter here is a REVIEWABLE ASSUMPTION: see DRIVERS.md for the full
rationale table (per-driver stochastic-vs-deterministic reasoning, parameter
sources, and the per-entry correlation rationale) and DRIVER-REVIEW.md for the
round-1 verdicts and dispositions this revision implements. Numbers derive
from the FY2021-25 segment history in the Q4 8-Ks (citations in DRIVERS.md)
and the v3 valuation framework (analysis/IBM/README.md, REVIEW.md).

Contract consumed by the v4 analysis scripts (analysis/IBM/dcf_drivers_run.py):
    build() -> {"model": DriverModel, "segments": [SegmentSpec...],
                "net_debt": float, "shares": float,
                "segment_history": {segment: {fy: revenue_dollars}}}
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from financial_analysis.driver_dcf import Driver, DriverModel, SegmentSpec

# FY2025 base revenue per segment, $M (Jan 28, 2026 8-K; "Other" $63M dropped —
# 0.1% of revenue, below materiality; documented in DRIVERS.md)
SEGMENT_HISTORY = {
    "Software":       {2021: 24_141e6, 2022: 25_037e6, 2023: 26_308e6, 2024: 27_085e6, 2025: 29_962e6},
    "Consulting":     {2021: 17_844e6, 2022: 19_107e6, 2023: 19_985e6, 2024: 20_692e6, 2025: 21_055e6},
    "Infrastructure": {2021: 14_188e6, 2022: 15_288e6, 2023: 14_593e6, 2024: 14_020e6, 2025: 15_718e6},
    "Financing":      {2021: 774e6,    2022: 645e6,    2023: 741e6,    2024: 713e6,    2025: 737e6},
}

NET_DEBT = 54.0e9        # v3 stock-vs-flow bridge (README): debt + after-tax pension - cash&securities
SHARES = 948_675_228     # diluted, held flat (SBC charged in margin)

PROJECTION_YEARS = 10

#: The M&A recurrence judgment f — the SINGLE source of truth (DRIVER-REVIEW:
#: it was hardcoded in two places, a silent-drift risk). Used by the
#: ma_recurrence driver AND by the g_software mean, which credits f × acquired
#: growth because f × M&A cost is charged in the margin. An assert below ties
#: the driver to this constant. The headline valuation is CONDITIONAL ON
#: f = 0.50; the f-locus (0 → 1, with the paired g_software shift) is
#: published by dcf_drivers_run.py.
MA_RECURRENCE = 0.50


def _linear_schedule(start: float, end: float, years: int = PROJECTION_YEARS) -> tuple:
    """Per-year linear ramp from start (year 1) to end (year `years`)."""
    step = (end - start) / (years - 1)
    return tuple(start + step * i for i in range(years))


# Infrastructure base: FY24/FY25 average — FY2025 ($15,718M, +12%, FX tailwind)
# is a mainframe-cycle peak; averaging with FY2024 removes the cycle-peak
# haircut the revenue-formula review demanded.
INFRA_BASE = (SEGMENT_HISTORY["Infrastructure"][2024]
              + SEGMENT_HISTORY["Infrastructure"][2025]) / 2.0  # = 14_869e6

# Software growth pairing (consolidation reopen: the f-credit must be DERIVED from
# MA_RECURRENCE, not hardcoded on one leg): mean = organic + f x acquired.
SOFTWARE_ORGANIC_G = 0.040   # F16 decomposition: organic ~3.75-4%
SOFTWARE_ACQUIRED_G = 0.020  # ~2-2.5pp all-in acquired growth at historical spend
G_SOFTWARE_MEAN = SOFTWARE_ORGANIC_G + MA_RECURRENCE * SOFTWARE_ACQUIRED_G  # = 0.050

# Infrastructure year-1 pin (consolidation reopen: -7% applied to the FY24/25-average
# base double-counted the cycle correction). The pin targets the warning-implied LEVEL,
# FY2025 x (1 - 7%) = $14,618M, expressed as growth on INFRA_BASE:
INFRA_Y1_TARGET = SEGMENT_HISTORY["Infrastructure"][2025] * (1 - 0.07)
INFRA_Y1_GROWTH = INFRA_Y1_TARGET / INFRA_BASE - 1.0  # = -1.69%

DRIVERS = [
    # --- revenue drivers (10y-horizon growth means, charged basis: f=MA_RECURRENCE
    #     of M&A is paid for in the margin, so f x acquired growth is credited on
    #     top of organic) ---
    Driver("g_software", stochastic=True, mean=G_SOFTWARE_MEAN, std=0.025,
           dist="truncnorm", hi=0.11,
           rationale=f"Largest segment (44%). Organic growth ~3.75-4% (F16 decomposition); "
                     f"acquired growth ~2-2.5pp of which f=MA_RECURRENCE={MA_RECURRENCE:.2f} "
                     f"is paid for in the margin, crediting ~1pp: mean 5.0% all-in on the "
                     f"charged basis. History footnote (corrected): headline FY22-25 growth "
                     f"of 5-11% mixes recast bases and includes HashiCorp — it is NOT an "
                     f"organic run-rate; guide ~10% and Q2-26 warning +5% bracket the near "
                     f"term. hi=11%: ten straight years at 11%+ on a $30B base has no "
                     f"precedent — the truncation caps that tail. Stochastic: demand-driven, "
                     f"the main uncertainty."),
    Driver("g_consulting", stochastic=True, mean=0.0175, std=0.025,
           rationale="FY22-25: +7.1/+5/-1/+2% (recast footnote: Jan-2024 segment change "
                     "moved Security Services Software->Consulting; growth across that "
                     "boundary is not like-for-like); guide low-to-mid singles. GenAI is "
                     "two-sided but the cannibalization left tail (AI does the work) is "
                     "the heavier side: mean 1.75% and std 2.5% proxy the left-skewed "
                     "mixture the reviewer derived (implied mean ~1.75%); a true mixture "
                     "marginal is an engine ticket. Stochastic: cyclical + secular "
                     "uncertainty."),
    Driver("g_infrastructure", stochastic=True, mean=-0.010, std=0.025,
           dist="truncnorm", hi=0.03,
           rationale="Mainframe-cycle sawtooth (+7.8/-5/-4/+12); guide down low singles; "
                     "Q2-26 -7%. Because the engine holds one draw constant per path "
                     "(T-DCF-1 embed, disclosed), std must be the dispersion of the "
                     "10-YEAR CAGR, not annual volatility: annual sigma ~8.4% with "
                     "cycle mean-reversion compresses to ~1.5-2.7% at the decade "
                     "horizon — std 2.5% sits in that band. hi=+3%: a hardware segment "
                     "compounding >=+3%/yr for a decade has never happened. Mean -1% "
                     "kept: slightly negative through the cycle."),
    Driver("g_financing", stochastic=False, mean=0.000,
           rationale="DETERMINISTIC flat: 1.1% of revenue; a +/-50% error moves total "
                     "revenue <0.6% — materiality does not justify a sampled process. "
                     "Disclosure: the terminal value grows this flat segment at tg like "
                     "the others (second-order at 1.1% weight). Disclosure: financing "
                     "volume is not linked to the infrastructure cycle here — that "
                     "volume link is severed by the flat treatment."),
    # --- margin drivers ---
    Driver("core_margin", stochastic=True, mean=0.194, std=0.020,
           rationale="Pre-SBC pre-M&A FCFF margin at the FY2025 LEVEL (19.4%): F19 shows "
                     "the margin declining three straight years, so a 5y mean overstates "
                     "the starting point — the endpoint is the honest base. The offsetting "
                     "software-mix improvement is NOT loaded into this mean; it is the "
                     "separate deterministic margin_mix_drift schedule (implemented "
                     "together, per the core_margin review). Stochastic: the operational "
                     "uncertainty core. Std from the v3 per-year dispersion."),
    Driver("margin_mix_drift", stochastic=False, mean=0.0,
           schedule=_linear_schedule(0.0, 0.012),
           rationale="DETERMINISTIC schedule, linear 0 -> +1.2pp by year 10: software-mix "
                     "margin drift (per the core_margin review: holding mix flat while "
                     "software compounds faster than the company contradicts the model's "
                     "own revenue mix — by y10 the software share implies ~+1.2pp of "
                     "margin). Deterministic: it is a mechanical consequence of the "
                     "segment growth means, not an independent random process; the "
                     "randomness already lives in the growth draws."),
    Driver("sbc_ratio", stochastic=False, mean=0.0254,
           schedule=_linear_schedule(0.0254, 0.035),
           rationale="DETERMINISTIC schedule, linear 2.54% (FY2025 actual, no flattering "
                     "rounding) -> 3.5% by year 10: the ratio rose 1.74% -> 2.54% since "
                     "2021 and is accelerating — a constant embeds an unexplained trend "
                     "break. 3.5% is a deliberate partial-persistence choice (full "
                     "historical pace reaches ~4.5%; MSFT sits at 4.3%): management-set, "
                     "slow-moving policy, so a schedule not a sampled process. The "
                     "flat-2.54% / ramp-3.5% / ramp-4.5% locus is published by "
                     "dcf_drivers_run.py."),
    Driver("ma_ratio", stochastic=True, mean=0.071, std=0.024, dist="truncnorm", lo=0.0,
           rationale="Acquisition spend % of revenue: 5.7/3.9/8.2/5.2/12.3% FY21-25 — "
                     "simple average 7.06%, so mean 7.1% (the old 6.6% was flattering-"
                     "backward; the endpoint rule applied to sbc_ratio must not be "
                     "selectively skipped here). Std 2.4%: one draw is held for all 10 "
                     "years, so dispersion is DECADE-AVERAGE scale — the margin "
                     "consolidation ruled that at high spend persistence (AR(1) rho~0.8, "
                     "supported by the accelerating 12.3% endpoint) the decade-average "
                     "std is ~2.4%, and the prior 1.8% made the FY2025 endpoint a 2.9-"
                     "sigma event. The fat right tail lives in the lognormal-marginal "
                     "engine ticket. Truncated at 0: spend cannot be negative (lo now "
                     "3.0 sigma out)."),
    Driver("ma_recurrence", stochastic=False, mean=MA_RECURRENCE,
           rationale=f"DETERMINISTIC f={MA_RECURRENCE:.2f}. The correct defense is "
                     f"per-path PAIRING COHERENCE, not 'fake precision' (the old "
                     f"rationale was backwards — sampling would widen the distribution "
                     f"honestly): g_software's mean credits f x acquired growth because "
                     f"f x M&A cost is charged here; sampling f alone would break that "
                     f"pairing path-by-path (a path with f=0.2 would still carry the "
                     f"f=0.5 growth credit). f is a judgment (v3 F2/F16), explored via "
                     f"the published locus in which the growth credit moves WITH f. "
                     f"Headline results are conditional on f=0.50. Single module-level "
                     f"MA_RECURRENCE constant (was hardcoded in two places)."),
    # --- WACC drivers ---
    Driver("rf", stochastic=True, mean=0.046, std=0.005, dist="truncnorm", lo=0.01,
           rationale="10y Treasury ~4.56-4.60% spot (Jul 2026): mean 4.6%. Framing "
                     "(rewritten): one draw held per path is NOT 'rates are stochastic' "
                     "year-to-year — it is uncertainty about the PERMANENT COMPONENT of "
                     "the discount rate over the decade; std 0.5% is calibrated to that "
                     "permanent-component reading, not annual rate vol. lo=1% is a "
                     "formal guard only (4.6% is 7.2 sigma away), not a modeling claim."),
    Driver("beta", stochastic=False, mean=0.85,
           rationale="DETERMINISTIC 0.85 on EPISTEMIC grounds: beta is an estimation "
                     "constant, not a year-to-year random process — sampling it would "
                     "model our ignorance as world randomness. (Corrected: the old "
                     "'sampling beta double-counts ERP vol' claim was technically wrong — "
                     "a random loading and a random price of risk compound legitimately; "
                     "the deterministic choice stands on epistemic grounds alone.) Regime "
                     "risk is real: trailing beta ~0.68 post-crash vs arguably higher "
                     "forward beta for an AI-capex-levered IBM — probed via the published "
                     "beta locus (0.68/0.85/1.00) in dcf_drivers_run.py, not sampled."),
    Driver("erp", stochastic=True, mean=0.045, std=0.0075, dist="truncnorm", lo=0.02,
           rationale="Equity risk premium: mean 4.5% per Damodaran implied ERP (~4.45% "
                     "Jul-2026; 4.23% Jan-2026) — the old 4.75% was unsourced. Stochastic "
                     "because the ERP is a STATE VARIABLE of risk appetite, not an "
                     "estimation constant like beta: it comoves with the demand and rate "
                     "drivers (see the erp rows of the correlation matrix — that "
                     "comovement is the point of sampling it). Std 0.75% spans the "
                     "post-2008 implied-ERP range."),
    Driver("kd_spread", stochastic=False, mean=-0.0010,
           rationale="DETERMINISTIC -10bp spread over the rf draw; after-tax kd is "
                     "DERIVED per path as (rf + spread) x (1 - tax_rate), so kd floats "
                     "with rates and negative-credit-spread paths (3.8% under the old "
                     "fixed 4.1%) cannot occur. Calibration (adjudicated conflict #2): "
                     "fair value of the debt stack — blended ~7y ladder yields ~4.45% "
                     "pre-tax at rf 4.56%, i.e. spread ~ -10bp (IBM's ladder trades "
                     "through the 10y point because its duration is shorter). The old "
                     "4.1% 'after-tax' level failed provenance (it was a pre-tax-looking "
                     "number) and is retired with the position."),
    Driver("tax_rate", stochastic=False, mean=0.21,
           rationale="DETERMINISTIC 21% statutory federal rate on the debt shield — a "
                     "disclosed modeling choice (IBM's effective cash tax rate runs "
                     "lower, which cuts the shield's value). The tau=0 disclosure "
                     "scenario is run by dcf_drivers_run.py, not by changing this "
                     "driver."),
    Driver("equity_weight", stochastic=False, mean=0.72,
           rationale="DETERMINISTIC E/V = 0.72: ITERATED price-value fixed point at the "
                     "driver means (disclosure != treatment — v3 disclosed the "
                     "circularity of weighting at the $212 market price but kept it; "
                     "this revision iterates E = model value x shares, V = E + net debt "
                     "until E/V converges; the iteration itself is shown in "
                     "dcf_drivers_run.py). Core-debt swap rejected as cherry-picking. "
                     "Non-stationary weights as debt grows + Harris-Pringle relever "
                     "remain ticketed."),
    # --- terminal ---
    Driver("terminal_growth", stochastic=True, mean=0.0225, std=0.0075,
           rationale="Long-run nominal growth, mean 2.25% (survives the reviewer's own "
                     "mix-weighted-fade math). Stochastic while beta is not — the "
                     "asymmetry is deliberate and defended: tg is a WORLD-STATE OUTCOME "
                     "(which long-run demand world was realized), correlated with the "
                     "realized growth draws (the F6 tail-fattening links), whereas beta "
                     "is an estimation constant about which we are merely uncertain. "
                     "Std 0.75% (was 0.5% — tightest-to-impact input, widened per "
                     "review). Engine enforces tg < WACC - 2% jointly per path "
                     "(T-DCF-2) and now reports the pinned fraction; deriving tg per "
                     "path from faded segment draws is ticket T-DCF-3."),
]

# The ma_recurrence driver and the g_software growth credit must move together
# (pairing coherence) — both read MA_RECURRENCE, and this assert keeps any
# future edit from silently drifting one without the other.
_ma_rec = next(d for d in DRIVERS if d.name == "ma_recurrence")
assert _ma_rec.mean == MA_RECURRENCE, (
    "ma_recurrence driver mean must equal MA_RECURRENCE — the g_software mean "
    "credits f x acquired growth against this same f."
)

# Full correlation matrix over the 8 stochastic drivers — every entry (incl. the
# zeros) is an assumption with a rationale row in DRIVERS.md. PSD-validated by the
# engine at construction.
CORR = {
    ("g_software", "g_consulting"): 0.40,        # shared enterprise IT demand cycle
    ("g_software", "g_infrastructure"): 0.35,    # Z stack coupling — Q2-26 warning: TP software fell WITH the hardware shift
    ("g_consulting", "g_infrastructure"): 0.20,  # weaker demand link
    ("g_software", "core_margin"): 0.30,         # software mix lifts margin
    ("g_consulting", "core_margin"): 0.10,       # low-margin segment, weak link
    ("g_infrastructure", "core_margin"): 0.15,   # operating leverage on hardware cycle
    ("g_software", "ma_ratio"): 0.30,            # M&A concentrated in software; buys its growth
    ("g_consulting", "ma_ratio"): 0.10,
    ("rf", "erp"): -0.30,                        # risk-off: rates down, premium up
    ("rf", "g_software"): 0.15,                  # strong nominal economy: rates and demand up together
    ("rf", "g_consulting"): 0.15,                # (demand-regime sign; supply-shock regime -0.15 published as sensitivity)
    ("rf", "g_infrastructure"): 0.10,
    ("ma_ratio", "rf"): -0.15,                   # cheap money fuels deals; was the least defensible zero
    ("erp", "g_software"): -0.25,                # risk aversion rises in weak-demand worlds
    ("erp", "g_consulting"): -0.25,
    ("erp", "g_infrastructure"): -0.15,
    ("erp", "core_margin"): -0.15,
    ("erp", "ma_ratio"): -0.10,                  # M&A slows when risk premia blow out
    ("terminal_growth", "g_software"): 0.40,     # long-run growth ties to realized growth (F6: this is the tail-fattening link)
    ("terminal_growth", "g_consulting"): 0.30,
    ("terminal_growth", "g_infrastructure"): 0.20,
    ("terminal_growth", "core_margin"): 0.15,
    ("terminal_growth", "rf"): 0.25,             # shared inflation component
    ("terminal_growth", "erp"): -0.20,
    ("terminal_growth", "ma_ratio"): 0.15,       # high-tg worlds keep buying growth: repairs cost-without-growth in the terminal (TV review)
    # Explicit zeros (assumptions too): ma_ratio-{g_infrastructure, core_margin},
    # core_margin-rf — rationales in DRIVERS.md.
}


def build() -> dict:
    model = DriverModel(drivers=list(DRIVERS), corr=dict(CORR))
    # Year-1 growth pinned to the Jul-2026 warning actuals (Q2-26 8-K):
    # Software +5%, Consulting flat, Infrastructure -7%, Financing flat.
    # Draws apply from year 2, compounding from the pinned year-1 level.
    segments = [
        SegmentSpec("Software", SEGMENT_HISTORY["Software"][2025], "g_software",
                    year1_growth=0.05),
        SegmentSpec("Consulting", SEGMENT_HISTORY["Consulting"][2025], "g_consulting",
                    year1_growth=0.0),
        SegmentSpec("Infrastructure", INFRA_BASE, "g_infrastructure",
                    year1_growth=INFRA_Y1_GROWTH),
        SegmentSpec("Financing", SEGMENT_HISTORY["Financing"][2025], "g_financing",
                    year1_growth=0.0),
    ]
    return {
        "model": model,
        "segments": segments,
        "net_debt": NET_DEBT,
        "shares": SHARES,
        "segment_history": SEGMENT_HISTORY,
    }
