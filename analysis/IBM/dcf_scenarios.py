"""IBM Monte Carlo DCF v3 — bridge/flow-consistent FCFF, paired (growth, M&A-charge) scenarios.

v3 framework (see REVIEW.md for the adversarial review that produced it):
  FCFF = OCF + (1-21%) x (interest expense - interest income) - capex - SBC
  Net debt = total debt + after-tax pension deficit - cash & securities
             (no lease add: rent stays in OCF; no financing-receivables credit:
              the captive's income stays in the flows)
  Charging fraction f of avg M&A spend credits growth g = organic 1.65% + f x 1.0pp.

Prerequisite: python -m scripts.analyze IBM

Usage (from repo root):
    python -m analysis.IBM.dcf_scenarios
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import numpy as np

from financial_analysis import store
from financial_analysis.dcf import DCFAssumptions, StochasticDCF

# FY2025 10-K / Q4-25 8-K. Bridge is stock-vs-flow consistent (REVIEW.md F15).
NET_DEBT = (61.3 + 0.79 * 9.0 - 14.4) * 1e9   # debt + after-tax pension - cash&securities = $54.0B
SHARES = 948_675_228                           # diluted; SBC charged as cash cost, count held flat
TAX_SHIELD = 0.79                              # 21% statutory (choice; FY2025 cash tax ~0)
MA_AVG_SPEND_PCT = 4.46 / 67.5                 # avg 2021-25 acquisitions, % of FY2025 revenue (~6.6%)
ORGANIC_G = 0.0165                             # all-in 2019-base CAGR 2.65% - ~1.0pp acquired contribution
ACQUIRED_G = 0.010                             # $4.46B/yr at 6-9x software sales = ~1.0pp revenue growth
BASE_WACC = 0.075                              # bottom-up 7.3%, top of band (README)


def fcff_v3_margins(company) -> tuple[float, float, list[tuple[int, float]]]:
    """Per-year FCFF v3 margin over FY2021+ annuals; returns (mean, sampling std, per-year)."""
    annuals = sorted(
        (p for p in company.periods if p.form in ("10-K", "10-K/A")),
        key=lambda p: p.period_end,
    )
    rows = []
    for p in annuals:
        if p.period_end.year < 2021 or p.revenue is None or p.operating_cash_flow is None:
            continue
        int_exp = p.interest_expense or 0
        int_inc = (p.raw_extras or {}).get("us-gaap/InterestIncomeOther", 0) or 0
        fcff = (p.operating_cash_flow + TAX_SHIELD * (int_exp - int_inc)
                - abs(p.capex or 0) - (p.sbc_total or 0))
        rows.append((p.period_end.year, fcff / p.revenue))
    arr = np.array([m for _, m in rows])
    return float(arr.mean()), max(float(arr.std()), 0.02), rows


def paired_growth(f: float) -> float:
    return ORGANIC_G + f * ACQUIRED_G


def scenario(g, wacc, tg, margin) -> DCFAssumptions:
    return DCFAssumptions(
        growth_rate_mean=g, growth_rate_std=0.02,
        wacc_mean=wacc, wacc_std=0.0075,
        terminal_growth_mean=tg, terminal_growth_std=0.005,
        fcf_margin_mean=margin, fcf_margin_std=0.02,
        seed=7,
    )


def per_share_percentiles(company, a: DCFAssumptions) -> tuple[float, float, float]:
    r = StochasticDCF(company, a).run()
    ps = (r.simulations - NET_DEBT) / SHARES
    return tuple(float(np.percentile(ps, q)) for q in (10, 50, 90))


def tv_share(a: DCFAssumptions) -> float:
    years = np.arange(1, 11)
    rev = 67.535e9 * (1 + a.growth_rate_mean) ** years
    fcfs = rev * a.fcf_margin_mean
    pv_f = (fcfs / (1 + a.wacc_mean) ** years).sum()
    tv = (fcfs[-1] * (1 + a.terminal_growth_mean)
          / (a.wacc_mean - a.terminal_growth_mean) / (1 + a.wacc_mean) ** 10)
    return tv / (pv_f + tv)


def main() -> None:
    company = store.load("IBM")
    m_mean, m_std, rows = fcff_v3_margins(company)

    print("IBM Monte Carlo DCF v3 — 10,000 sims, 10y horizon, seed 7")
    print("FCFF v3 per-year margins: "
          + "  ".join(f"{y}: {m:.1%}" for y, m in rows)
          + f"  ->  mean {m_mean:.1%} (sampling std {m_std:.1%})")
    print(f"Net debt ${NET_DEBT / 1e9:.1f}B (bridge in README) | diluted shares {SHARES / 1e9:.3f}B")
    print("Tail caveat: inputs sampled independently, growth autocorrelated per path —")
    print("P10/P90 understate left-tail risk; P50s are robust (REVIEW.md F6/F12).\n")

    scenarios = {
        "Bear  (disruption: g 1%, 100% M&A charged, 2pp erosion)":
            scenario(0.01, 0.0825, 0.015, m_mean - 0.02 - MA_AVG_SPEND_PCT),
        "Base  (organic 1.65% + half-paid acquired growth, f=50%)":
            scenario(paired_growth(0.5), BASE_WACC, 0.0225, m_mean - 0.5 * MA_AVG_SPEND_PCT),
        "Bull  (guidance-anchored recovery: g 4.5%, f=25%)":
            scenario(0.045, 0.0725, 0.025, m_mean - 0.25 * MA_AVG_SPEND_PCT),
    }
    hdr = f"{'Scenario':<60}{'g':>7}{'margin':>8}{'WACC':>7}{'tg':>6}{'P10':>7}{'P50':>7}{'P90':>7}{'TV/EV':>7}"
    print(hdr)
    print("-" * len(hdr))
    for name, a in scenarios.items():
        p10, p50, p90 = per_share_percentiles(company, a)
        print(f"{name:<60}{a.growth_rate_mean:>7.2%}{a.fcf_margin_mean:>8.1%}"
              f"{a.wacc_mean:>7.2%}{a.terminal_growth_mean:>6.1%}"
              f"{p10:>6.0f}${p50:>6.0f}${p90:>6.0f}${tv_share(a):>7.0%}")

    print("\nM&A-recurrence locus (paired growth, base WACC):")
    for f in (0.0, 0.25, 0.5, 0.75, 1.0):
        a = scenario(paired_growth(f), BASE_WACC, 0.0225, m_mean - f * MA_AVG_SPEND_PCT)
        _, p50, _ = per_share_percentiles(company, a)
        print(f"  f={f:>4.0%}: g {paired_growth(f):.2%}, margin {m_mean - f * MA_AVG_SPEND_PCT:.1%} -> P50 ${p50:.0f}")

    print("\nWACC grid (base pairing f=50%):")
    for w in (0.0625, 0.065, 0.0675, 0.07, 0.0725, 0.075):
        a = scenario(paired_growth(0.5), w, 0.0225, m_mean - 0.5 * MA_AVG_SPEND_PCT)
        _, p50, _ = per_share_percentiles(company, a)
        print(f"  WACC {w:.2%} -> P50 ${p50:.0f}")

    print("\nF19 margin-level check (FY2025 margin instead of 5y mean, f=50%):")
    latest_margin = rows[-1][1]
    a = scenario(paired_growth(0.5), BASE_WACC, 0.0225, latest_margin - 0.5 * MA_AVG_SPEND_PCT)
    p10, p50, p90 = per_share_percentiles(company, a)
    print(f"  margin {latest_margin - 0.5 * MA_AVG_SPEND_PCT:.1%} -> P50 ${p50:.0f} [{p10:.0f}, {p90:.0f}]")


if __name__ == "__main__":
    main()
