"""JNJ driver-based DCF model — FIRST PASS (2026-10-16), NOT a vetted driver build.

Built in one session to test the IBM template on a new ticker (Luc's review on
PR #16: "test using an entirely different stock"). No driver, formula or
correlation review fan-out has been run; every rationale below is a first-pass
argument awaiting the same per-driver review IBM received (DRIVER-REVIEW.md).
Findings and sources: analysis/JNJ/FIRST-PASS.md.

Contract (same as analysis/IBM/driver_model.py):
    build() -> {"model", "segments", "net_debt", "shares", "segment_history"}
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from financial_analysis.driver_dcf import Driver, DriverModel, SegmentSpec

# Segment sales, continuing operations (Kenvue recast), $ — FY2025 10-K
# (2025/2024/2023) and FY2023 10-K (2022/2021) segment tables.
SEGMENT_HISTORY = {
    "Innovative Medicine": {2021: 51_680e6, 2022: 52_563e6, 2023: 54_759e6, 2024: 56_964e6, 2025: 60_401e6},
    "MedTech":             {2021: 27_060e6, 2022: 27_427e6, 2023: 30_400e6, 2024: 31_857e6, 2025: 33_792e6},
}

# Net-debt bridge, stock-vs-flow (IBM F15 convention), $ — Q2-2026 10-Q unless noted:
#   debt 49.04B (short-term borrowings 11.69 + long-term debt 37.34)
# + pension/OPEB liability 6.96B (FY2025 10-K) x (1 - 21%) = 5.50B, liability only
# + talc reserves ~9.2B pre-tax (~3.7B PV per the Q2-2026 10-Q + 5.52B Q3-2026
#   litigation special item for the proposed ovarian settlement) x (1 - 21%) = 7.27B
# - cash & marketable securities 20.76B (20.42 + 0.34)
# Operating leases (1.4B) excluded: their flows already sit in the margin.
NET_DEBT = 49.04e9 + 5.50e9 + 7.27e9 - 20.76e9   # = 41.05B
SHARES = 2_440_100_000   # Q2-2026 diluted weighted average, held flat (SBC charged in margin)

PROJECTION_YEARS = 10

#: M&A recurrence f — single source of truth, paired with the growth credit
#: exactly as in the IBM register (charge f x M&A in margin, credit f x acquired
#: growth in the segment means).
MA_RECURRENCE = 0.50
ACQUIRED_G = 0.020  # FY2025: operational 5.3% vs adjusted operational 4.2% (~1.1pp), rounded up for lumpy deal years

IM_ORGANIC_G = 0.040
MEDTECH_ORGANIC_G = 0.035


def _linear_schedule(start: float, end: float, years: int = PROJECTION_YEARS) -> tuple:
    step = (end - start) / (years - 1)
    return tuple(start + step * i for i in range(years))


DRIVERS = [
    Driver("g_im", stochastic=True, mean=IM_ORGANIC_G + MA_RECURRENCE * ACQUIRED_G, std=0.030,
           dist="truncnorm", hi=0.12,
           rationale="Innovative Medicine, 10y average from year 2. FY22-25 reported growth "
                     "+1.7/+4.2/+4.0/+6.0% while absorbing the STELARA loss of exclusivity "
                     "(~690bp drag in Q3-26); organic ~4% plus f x 2pp acquired. Std 3% "
                     "(above IBM software's 2.5%): patent cliffs and trial readouts make the "
                     "decade outcome wider. hi 12%: no decade precedent above that at $60B scale."),
    Driver("g_medtech", stochastic=True, mean=MEDTECH_ORGANIC_G + MA_RECURRENCE * ACQUIRED_G, std=0.020,
           dist="truncnorm", hi=0.09,
           rationale="MedTech, 10y average from year 2. FY22-25 +1.4/+10.8 (Abiomed)/+4.8/"
                     "+6.1% (Shockwave); organic ~3.5% plus f x 2pp acquired. The planned "
                     "Orthopaedics separation is NOT modeled (segment kept whole)."),
    Driver("core_margin", stochastic=True, mean=0.208, std=0.020,
           rationale="Pre-SBC, pre-M&A FCFF margin at the FY2025 level: (OCF 24.53 - capex "
                     "4.83 - after-tax net interest 0.07) / revenue 94.19 = 20.8%; FY2024 "
                     "21.8%. Endpoint, not mean (IBM F19 rule). FY2022-23 excluded: their OCF "
                     "includes Kenvue while revenue is recast (same landmine as IBM FY2019-20)."),
    Driver("margin_mix_drift", stochastic=False, mean=0.0, schedule=_linear_schedule(0.0, 0.004),
           rationale="DETERMINISTIC 0 -> +0.4pp by y10: Innovative Medicine (FY2025 pre-tax "
                     "segment margin 36.9%) grows faster than MedTech (12.2%) at the means."),
    Driver("sbc_ratio", stochastic=False, mean=0.0143,
           rationale="DETERMINISTIC 1.43% (FY2025 1.35B / 94.19B); flat since FY2021 (1.45%), "
                     "so no ramp, unlike IBM."),
    Driver("ma_ratio", stochastic=True, mean=0.116, std=0.040, dist="truncnorm", lo=0.0,
           rationale="Acquisition spend % of revenue FY21-25: 0.1/22.1/0.0/17.1/18.6% "
                     "(Abiomed, Shockwave, Intra-Cellular), mean 11.6%; FY16-25 ~12.7% with "
                     "Actelion. Std 4%: one draw per path, so decade-average dispersion of a "
                     "very lumpy series (annual sigma ~14% / sqrt(10)). lo 0 at 2.9 sigma."),
    Driver("ma_recurrence", stochastic=False, mean=MA_RECURRENCE,
           rationale="DETERMINISTIC f=0.50, the IBM headline convention, paired with the "
                     "growth credit. First-pass caveat: in pharma, acquired pipeline revenue "
                     "turns 'organic' after one year, so the 2pp acquired-growth credit likely "
                     "understates what M&A buys; the f-locus is published."),
    Driver("rf", stochastic=True, mean=0.051, std=0.005, dist="truncnorm", lo=0.01,
           rationale="10y Treasury 5.10% (^TNX close 2026-10-16); std 0.5% = permanent-"
                     "component uncertainty (IBM convention)."),
    Driver("beta", stochastic=False, mean=0.65,
           rationale="DETERMINISTIC judgment 0.65. Regression betas are unusable: 0.22 "
                     "(5y monthly) and ~0 (2y weekly) vs the S&P 500; Blume-adjusted 0.48. "
                     "0.65 sits between that and a pharma/medtech industry beta; locus "
                     "0.45 / 0.65 / 0.85 published."),
    Driver("erp", stochastic=True, mean=0.042, std=0.0075, dist="truncnorm", lo=0.02,
           rationale="Damodaran implied ERP 4.20% (Oct 1, 2026); std 0.75% (IBM convention)."),
    Driver("kd_spread", stochastic=False, mean=0.0040,
           rationale="DETERMINISTIC +40bp over rf: AAA-rated issuer (first-pass estimate, "
                     "not calibrated to the debt ladder)."),
    Driver("tax_rate", stochastic=False, mean=0.21, rationale="DETERMINISTIC statutory 21%."),
    Driver("equity_weight", stochastic=False, mean=0.93,
           rationale="DETERMINISTIC E/V 0.93: market cap ~$616B (255.56 x 2.41B) vs net debt "
                     "$41.1B. Not iterated to the price-value fixed point (first pass)."),
    Driver("terminal_growth", stochastic=True, mean=0.0225, std=0.0075,
           rationale="Long-run nominal growth 2.25%, std 0.75% (IBM convention)."),
]

assert next(d for d in DRIVERS if d.name == "ma_recurrence").mean == MA_RECURRENCE

CORR = {
    ("g_im", "g_medtech"): 0.30,          # shared healthcare demand and pricing
    ("g_im", "core_margin"): 0.30,        # pharma mix lifts margin
    ("g_medtech", "core_margin"): 0.15,
    ("g_im", "ma_ratio"): 0.25,           # M&A concentrated in pharma pipeline
    ("g_medtech", "ma_ratio"): 0.15,
    ("rf", "erp"): -0.30,
    ("rf", "g_im"): 0.10,
    ("rf", "g_medtech"): 0.10,
    ("ma_ratio", "rf"): -0.15,
    ("erp", "g_im"): -0.15,               # defensive sector: weaker demand link than IBM
    ("erp", "g_medtech"): -0.15,
    ("terminal_growth", "g_im"): 0.35,
    ("terminal_growth", "g_medtech"): 0.25,
    ("terminal_growth", "rf"): 0.25,
    ("terminal_growth", "erp"): -0.20,
    ("terminal_growth", "ma_ratio"): 0.15,
}


def build() -> dict:
    # Year-1 pins = Oct-2026 guidance ($101.2B reported) split by the nine-month
    # run rate: Innovative Medicine +8.8% YTD, MedTech +4.9% YTD (Q3-26 8-K).
    segments = [
        SegmentSpec("Innovative Medicine", SEGMENT_HISTORY["Innovative Medicine"][2025], "g_im",
                    year1_growth=0.086),
        SegmentSpec("MedTech", SEGMENT_HISTORY["MedTech"][2025], "g_medtech",
                    year1_growth=0.050),
    ]
    return {
        "model": DriverModel(drivers=list(DRIVERS), corr=dict(CORR)),
        "segments": segments,
        "net_debt": NET_DEBT,
        "shares": SHARES,
        "segment_history": SEGMENT_HISTORY,
    }
