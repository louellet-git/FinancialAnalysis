"""ANT (Anthropic, PBC) driver model for the driver DCF engine. FIRST PASS, 2026-11-19.

Not reviewed. No skeptic, reviewer or auditor examined this model. Do not use it alone
for a trade. Claude models made this model, and Anthropic makes Claude: read each
judgment with that fact in mind.

The prospectus has no XBRL values. A person or an agent typed each value below from
the prospectus text. "Lnnn" is a line of the prospectus text file. The ledger
`analysis/ANT/research-ledger.md` gives the same lines. "Press" marks a number from a
news source: it has lower quality than the prospectus.

Contract (the same as analysis/JNJ/driver_model.py):
    build() -> {"model", "segments", "net_debt", "shares", "segment_history"}
Results and limits: analysis/ANT/FIRST-PASS.md.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from financial_analysis.driver_dcf import Driver, DriverModel, SegmentSpec

# ---------------------------------------------------------------------------
# Revenue. One segment. USD.
# ---------------------------------------------------------------------------
# Fiscal years end on December 31 (L4552).
SEGMENT_HISTORY = {"Total": {2023: 41e6, 2024: 386e6, 2025: 4_585e6}}

# Base: the 12 months to 2026-09-30 = FY2025 4,585 - 9M2025 2,545 + 9M2026 33,242 (L4552).
BASE_REVENUE = (4_585 - 2_545 + 33_242) * 1e6          # 35.282B

# Year 1 = the 12 months to 2027-09-30. Pinned. Evidence:
#   - Quarters of 2026: 4,731 / 11,555 / 16,956 (L2130). Q3 x 4 = 67.8B.
#   - Run-Rate Revenue: about 71B at 2026-09-30 and about 81B at 2026-10-31 (L2683).
#   - Run rate flat at 81B for the full year: 76/12 + 11 x 81/12 = 80.6B (the floor
#     if growth stops now).
#   - Run rate adds 7.2B to 10B each month, as in 2026 (Q3 less Q2 revenue x 4 / 3 =
#     7.2; October = 10): 117B to 131B.
#   - Press, lower quality: Goldman briefing 52B for 2026 and 136B for 2027 (about
#     108B for these 12 months, by interpolation); Damodaran year 1 103B.
#   105B is equal to a run rate that adds 4.8B each month: slower than 2026, because
#   the prospectus says that the growth rate will decline (L1666). JUDGMENT.
YEAR1_REVENUE = 105e9

# ---------------------------------------------------------------------------
# Share count and net debt: ONE treatment (ledger section 11).
# ---------------------------------------------------------------------------
# Shares = the fully diluted count of the prospectus (table L383 to L410, row L410):
#   Class A 7,585,256,149 + Class N 7,766,496,616 + options net 1,712,133,811
#   (treasury stock method at $83.00) + RSUs 1,073,701,297. Class T and Class F have
#   no economic rights and are not in the count.
# The Class N number contains: the Amazon rollover notes as converted
# (1,547,676,309 shares), the Initial Public Benefit Issuance (846,840,960 shares
# that a wholly owned subsidiary holds for donation), and the Vested Matching
# Obligation (553,441,297 shares).
# Because the notes are shares here, the note liability (109,742 at fair value,
# L1517) is NOT in the net debt. The principal is only about 5.6B at conversion
# prices of $2.09 and $5.05 (L2275 to L2277): the notes are equity in substance.
# Because the unvested RSUs and options are shares here, the SBC driver below is the
# cost of NEW grants only. Their unrecognized cost (32.3B, L2443) is not charged again.
SHARES = 7_585_256_149 + 7_766_496_616 + 1_712_133_811 + 1_073_701_297   # 18,137,587,873

# Net debt (stock-or-flow rule, IBM F15), USD:
#   - cash, cash equivalents and short-term investments, pro forma as adjusted
#     232,351 (L1516; after the offering, the private placement and the 4.3B RSU tax)
#   + finance lease liabilities 1,674 (L5278). They are debt: their principal
#     payments are financing flows, not in the margin.
#   Operating leases (2,810, L4514 and L4517) stay out: their flow is in the margin.
#   Restricted cash (2,540, L4505) is not counted. The credit facility is not drawn.
#   Purchase and lease commitments (900.9B, L1655) are future operating costs: they
#   are in the margin, not here. The runner shows a debt treatment as a sensitivity.
CASH_AND_INVESTMENTS = 232_351e6
FINANCE_LEASE_LIABILITIES = 1_674e6
NET_DEBT = FINANCE_LEASE_LIABILITIES - CASH_AND_INVESTMENTS               # -230.677B

PROJECTION_YEARS = 10

#: No M&A model: goodwill is 595 (L4508), acquisitions are small.
MA_RECURRENCE = 0.0

# ---------------------------------------------------------------------------
# Judgment values (each has its reason in the driver rationale).
# ---------------------------------------------------------------------------
G_MEAN, G_STD = 0.20, 0.08            # growth from year 2, one draw for each path
CORE_MARGIN_MEAN, CORE_MARGIN_STD = 0.22, 0.06   # mature FCFF margin before SBC
MARGIN_YEAR1 = -0.07                  # FCFF margin before SBC in year 1
RAMP_YEARS = 8                        # the mature level starts in this year
SBC_YEAR1, SBC_MATURE = 0.25, 0.06    # new grants as a share of revenue
BETA = 1.40


def _ramp(start: float, end: float, ramp_years: int, years: int) -> tuple:
    """Linear from `start` in year 1 to `end` in year `ramp_years`, then constant."""
    return tuple(start + (end - start) * min(i, ramp_years - 1) / (ramp_years - 1)
                 for i in range(years))


def _drivers(years: int = PROJECTION_YEARS, ramp_years: int = RAMP_YEARS,
             core_mean: float = CORE_MARGIN_MEAN, sbc_year1: float = SBC_YEAR1) -> list:
    drift = _ramp(MARGIN_YEAR1 - core_mean, 0.0, ramp_years, years)
    sbc = _ramp(sbc_year1, SBC_MATURE, ramp_years, years)
    return [
        Driver("g_total", stochastic=True, mean=G_MEAN, std=G_STD, dist="truncnorm",
               lo=0.0, hi=0.40,
               rationale="Revenue growth from year 2, one constant draw for each path (the "
                         "engine has no fade). JUDGMENT. Mean 20% gives year-10 revenue of "
                         "542B from 105B. Base rate (market notes 3.4): the best 10-year "
                         "result from a 30B start is AMZN 27.4% a year, equal to 397B from "
                         "this base; from a 100B start the median is 6.7% and the maximum "
                         "is 21.0%. Public forecasts: Damodaran 1,011B in year 10 (28.9% a "
                         "year from his year 1); press: New Street 505B and Goldman more "
                         "than 600B in 2030. The mean is between the base rate and the "
                         "forecasts, nearer to the base rate. Std 8% is the spread of the "
                         "9-year rate: about 240B at P10 and 1,100B at P90 in year 10. Bounds 0% and 40% "
                         "are 2.5 std from the mean."),
        Driver("core_margin", stochastic=True, mean=core_mean, std=CORE_MARGIN_STD,
               dist="truncnorm", lo=core_mean - 0.17, hi=core_mean + 0.18,
               rationale="Mature FCFF margin: after tax, after capex, before the SBC charge. "
                         "JUDGMENT. 22% less 6% SBC = 16% after SBC. Peers, FCF less SBC "
                         "(market notes 3.1 and 3.2): MSFT, GOOGL and META 12% to 16% now, "
                         "while they build AI capacity; 20% to 34% before; AMZN -7% to 5%. "
                         "Damodaran: 19.0% in year 10, 13.9% in the terminal year. Std 6%: "
                         "P10 to P90 is 8% to 24% after SBC. One draw moves all years by "
                         "the same amount (engine limit). Bounds are 2.8 and 3.0 std from "
                         "the mean."),
        Driver("margin_mix_drift", stochastic=False, mean=drift[0], schedule=drift,
               rationale="DETERMINISTIC ramp. Year 1 is -7% before SBC. 9M2026: operating "
                         "cash flow 6,569 - interest income 2,359 - capex 925 - lease "
                         "prepayments 1,955 = 1,330 = 4.0% of revenue (L4692, L447, L4694, "
                         "L4695). Committed spend in the next 12 months is 70.7B (L1655) = "
                         "67% of year-1 revenue; compute and infrastructure was 57% of "
                         "9M2026 revenue (L440): 4.0% - 10.7% = -6.7%. The prospectus says "
                         "that short-term compute will compress the margin (L1617) and that "
                         "operating and investing activities will use cash 'for the "
                         "foreseeable future' (L2249). Linear to the mature level in year "
                         "8. Year 4 is +5.4%; the Goldman briefing (press) has the first "
                         "positive free cash flow in 2030 at about 5.6% of revenue."),
        Driver("sbc_ratio", stochastic=False, mean=sbc[0], schedule=sbc,
               rationale="DETERMINISTIC. Cost of NEW equity grants (IBM F5: SBC is a cash "
                         "cost). The awards that exist are in the share count, thus their "
                         "cost is not here, and the one-time 14.7B IPO charge (L2443) is "
                         "not here. Year 1 25%: the grants after 2026-09-30 were 3.7B in "
                         "7 weeks (L6036), about 27B a year, on 105B of revenue; the 9M2026 "
                         "grants were 542.9M RSUs at $49.38 and 91.2M options at $20.11 = "
                         "28.6B (L5719, L5683, L5691). Mature 6%: GOOGL 6.2%, between MSFT "
                         "3.7% and META 10.2%. Linear to year 8. The donation matching "
                         "charges of future years (2,086 in 9M2026, L2090) are not here."),
        Driver("ma_ratio", stochastic=False, mean=0.0,
               rationale="DETERMINISTIC 0. No evidence of material acquisitions: goodwill "
                         "is 595 on 154,320 of assets."),
        Driver("ma_recurrence", stochastic=False, mean=MA_RECURRENCE,
               rationale="DETERMINISTIC 0, paired with ma_ratio 0. No growth credit."),
        Driver("rf", stochastic=True, mean=0.0507, std=0.005, dist="truncnorm", lo=0.01,
               rationale="10-year Treasury par yield 5.07%, 2026-11-18 (market notes 1.5). "
                         "Std 0.5% (repo convention)."),
        Driver("beta", stochastic=False, mean=BETA,
               rationale="DETERMINISTIC JUDGMENT 1.40. ANT has no price history. Peer "
                         "5-year betas: median 1.38, range 0.98 to 2.25. Damodaran industry "
                         "betas: software 1.28, semiconductor 1.52, internet software 1.69. "
                         "Fixed compute commitments increase the risk of the equity. Locus "
                         "1.10 / 1.40 / 1.70 in the runner."),
        Driver("erp", stochastic=True, mean=0.0441, std=0.0075, dist="truncnorm", lo=0.02,
               rationale="Damodaran implied ERP 4.41%, 2026-11-01. Std 0.75% (repo "
                         "convention)."),
        Driver("kd_spread", stochastic=False, mean=0.02,
               rationale="DETERMINISTIC 2.0% above rf. JUDGMENT for a BB+ rating (Damodaran "
                         "slides). Not calibrated. No effect on WACC at equity weight 1.0; "
                         "the runner uses it for the commitments-as-debt sensitivity."),
        Driver("tax_rate", stochastic=False, mean=0.21,
               rationale="DETERMINISTIC statutory 21%. Used on the debt shield only."),
        Driver("equity_weight", stochastic=False, mean=1.0,
               rationale="DETERMINISTIC 1.0. Debt is 1.7B of finance leases against more "
                         "than 200B of cash. The rollover notes are shares. Cash is not "
                         "negative debt in the weights. WACC = cost of equity."),
        Driver("terminal_growth", stochastic=True, mean=0.0225, std=0.0075,
               rationale="Long-run nominal growth 2.25%, std 0.75% (repo convention)."),
    ]


DRIVERS = _drivers()

assert next(d for d in DRIVERS if d.name == "ma_recurrence").mean == MA_RECURRENCE

CORR = {
    # Fixed compute commitments: low growth leaves capacity that the company must pay
    # for; high growth fills it. JUDGMENT.
    ("g_total", "core_margin"): 0.40,
    ("rf", "erp"): -0.30,                 # IBM and JNJ convention
    ("erp", "g_total"): -0.15,            # weak demand goes with high risk premiums
    ("terminal_growth", "rf"): 0.25,      # nominal rates and nominal growth
    ("terminal_growth", "g_total"): 0.25, # a larger market in the decade and after it
}


def build(years: int = PROJECTION_YEARS, ramp_years: int = RAMP_YEARS,
          core_mean: float = CORE_MARGIN_MEAN, sbc_year1: float = SBC_YEAR1,
          year1_revenue: float = YEAR1_REVENUE) -> dict:
    """Base model with no arguments. The arguments are for the runner sensitivities."""
    segments = [SegmentSpec("Total", BASE_REVENUE, "g_total",
                            year1_growth=year1_revenue / BASE_REVENUE - 1.0)]
    return {
        "model": DriverModel(drivers=_drivers(years, ramp_years, core_mean, sbc_year1),
                             corr=dict(CORR)),
        "segments": segments,
        "net_debt": NET_DEBT,
        "shares": SHARES,
        "segment_history": SEGMENT_HISTORY,
    }
