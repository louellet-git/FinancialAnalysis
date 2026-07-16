"""IBM driver-based DCF model — drivers, correlations, and capital structure.

Every parameter here is a REVIEWABLE ASSUMPTION: see DRIVERS.md for the full
rationale table (per-driver stochastic-vs-deterministic reasoning, parameter
sources, and the per-entry correlation rationale). Numbers derive from the
FY2021-25 segment history in the Q4 8-Ks (citations in DRIVERS.md) and the
v3 valuation framework (analysis/IBM/README.md, REVIEW.md).

Contract consumed by scripts/visualize_dcf.py and the v4 analysis scripts:
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

DRIVERS = [
    # --- revenue drivers (10y-horizon growth means, charged basis: f=0.5 of M&A is
    #     paid for below, so ~0.5 x acquired growth is credited on top of organic) ---
    Driver("g_software", stochastic=True, mean=0.060, std=0.025,
           rationale="Largest segment (44%). All-in FY22-25 growth 5-11% incl. HashiCorp; "
                     "guide ~10%; Q2-26 warning +5%. Long-run mean below guide (law of large "
                     "numbers on a $30B base, TP shortfall risk), organic ~5% + half-paid "
                     "acquired ~1%. Stochastic: demand-driven, the main uncertainty."),
    Driver("g_consulting", stochastic=True, mean=0.020, std=0.020,
           rationale="FY22-25: +7.1/+5/-1/+2%; guide low-to-mid singles; GenAI both tailwind "
                     "(backlog) and threat (AI does the work). Stochastic: cyclical + secular "
                     "uncertainty."),
    Driver("g_infrastructure", stochastic=True, mean=-0.010, std=0.030,
           rationale="Mainframe-cycle sawtooth (+7.8/-5/-4/+12); guide down low singles; "
                     "Q2-26 -7%. Long-run slightly negative through the cycle; widest std "
                     "of the segments (hardware cyclicality)."),
    Driver("g_financing", stochastic=False, mean=0.000,
           rationale="DETERMINISTIC flat: 1.1% of revenue; a +/-50% error moves total "
                     "revenue <0.6% — materiality does not justify a sampled process."),
    # --- margin drivers ---
    Driver("core_margin", stochastic=True, mean=0.1975, std=0.020,
           rationale="Pre-SBC pre-M&A FCFF margin: v3 FCFF-after-SBC margin (17.8%) plus "
                     "mean SBC ratio (~2.0%) = ~19.75%. Stochastic: the operational "
                     "uncertainty core. Std from the v3 per-year dispersion."),
    Driver("sbc_ratio", stochastic=False, mean=0.025,
           rationale="DETERMINISTIC at the FY2025 level (2.54%, ratios trending 1.7->2.5% "
                     "since 2021): management-set, slow-moving policy, not a random process; "
                     "level uncertainty is epistemic — probe via attribution, not sampling. "
                     "Set at the trend endpoint, not the flattering mean."),
    Driver("ma_ratio", stochastic=True, mean=0.066, std=0.030, dist="truncnorm", lo=0.0,
           rationale="Acquisition spend % of revenue: 5.7/3.9/8.2/5.2/12.3% FY21-25. "
                     "Stochastic AND truncated at 0: lumpy, genuinely random in timing/size."),
    Driver("ma_recurrence", stochastic=False, mean=0.50,
           rationale="DETERMINISTIC 0.5: a JUDGMENT about what fraction of M&A is "
                     "maintenance vs optional — epistemic, contested (v3 review F2/F16), "
                     "and explored via the published locus (0->100%), not sampled. Sampling "
                     "it would launder a judgment into fake precision."),
    # --- WACC drivers ---
    Driver("rf", stochastic=True, mean=0.043, std=0.005, dist="truncnorm", lo=0.01,
           rationale="10y Treasury ~4.3%: genuinely stochastic over a 10y valuation window."),
    Driver("beta", stochastic=False, mean=0.85,
           rationale="DETERMINISTIC: beta is an estimation constant (epistemic uncertainty, "
                     "not year-to-year randomness); sampling it double-counts market vol "
                     "already in ERP. Probed via attribution/scenarios. Post-crash regime "
                     "risk acknowledged in DRIVERS.md open questions."),
    Driver("erp", stochastic=True, mean=0.0475, std=0.0075, dist="truncnorm", lo=0.02,
           rationale="Equity risk premium 4.75%: varies with risk appetite; stochastic."),
    Driver("kd_after_tax", stochastic=False, mean=0.041,
           rationale="DETERMINISTIC: contractual — observable bond yields x (1-21%); "
                     "refinancing drift over 10y is second-order vs its 23.5% weight."),
    Driver("equity_weight", stochastic=False, mean=0.765,
           rationale="DETERMINISTIC market-value E/V at $212 (circularity disclosed in v3); "
                     "sampling weights would conflate price uncertainty with the discount "
                     "rate. Held fixed, probed via scenarios."),
    # --- terminal ---
    Driver("terminal_growth", stochastic=True, mean=0.0225, std=0.005,
           rationale="Long-run nominal growth: stochastic; engine enforces tg < WACC - 2% "
                     "jointly per path (T-DCF-2)."),
]

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
    ("rf", "g_consulting"): 0.15,
    ("rf", "g_infrastructure"): 0.10,
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
    # Explicit zeros (assumptions too): ma_ratio-{g_infrastructure, core_margin, rf,
    # terminal_growth}, core_margin-rf — rationales in DRIVERS.md.
}


def build() -> dict:
    model = DriverModel(drivers=list(DRIVERS), corr=dict(CORR))
    segments = [
        SegmentSpec("Software", SEGMENT_HISTORY["Software"][2025], "g_software"),
        SegmentSpec("Consulting", SEGMENT_HISTORY["Consulting"][2025], "g_consulting"),
        SegmentSpec("Infrastructure", SEGMENT_HISTORY["Infrastructure"][2025], "g_infrastructure"),
        SegmentSpec("Financing", SEGMENT_HISTORY["Financing"][2025], "g_financing"),
    ]
    return {
        "model": model,
        "segments": segments,
        "net_debt": NET_DEBT,
        "shares": SHARES,
        "segment_history": SEGMENT_HISTORY,
    }
