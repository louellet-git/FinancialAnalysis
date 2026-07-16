"""IBM Monte Carlo DCF — bear/base/bull scenarios with equity-adjusted per-share values.

Prerequisite: python -m scripts.analyze IBM  (populates data/processed/IBM.json)

Usage (from repo root):
    python -m analysis.IBM.dcf_scenarios
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import numpy as np

from financial_analysis import store
from financial_analysis.dcf import DCFAssumptions, StochasticDCF

# FY2025 10-K: $61.3B total debt + finance leases (incl. current) - $13.6B cash - $0.8B securities
NET_DEBT = 46.9e9
SHARES = 936_952_129  # FY2025 10-K shares outstanding

SCENARIOS = {
    "Bear  (AI disruption is real, growth stalls)": DCFAssumptions(
        growth_rate_mean=0.01, growth_rate_std=0.02,
        wacc_mean=0.095, wacc_std=0.01,
        terminal_growth_mean=0.015, terminal_growth_std=0.005,
        seed=7,
    ),
    "Base  (mature grower, historical trend)": DCFAssumptions(
        growth_rate_mean=0.04, growth_rate_std=0.02,
        wacc_mean=0.085, wacc_std=0.01,
        terminal_growth_mean=0.025, terminal_growth_std=0.005,
        seed=7,
    ),
    "Bull  (Q2 was timing; software/AI momentum resumes)": DCFAssumptions(
        growth_rate_mean=0.06, growth_rate_std=0.02,
        wacc_mean=0.08, wacc_std=0.01,
        terminal_growth_mean=0.03, terminal_growth_std=0.005,
        seed=7,
    ),
}


def main() -> None:
    company = store.load("IBM")

    print("IBM Monte Carlo DCF — 10,000 sims, 10y horizon, FCF margin derived from history")
    print(f"Net debt ${NET_DEBT / 1e9:.1f}B | shares {SHARES / 1e9:.3f}B\n")
    header = (f"{'Scenario':<52}{'EV P10':>9}{'EV P50':>9}{'EV P90':>9}"
              f"{'Eq/sh P10':>11}{'Eq/sh P50':>11}{'Eq/sh P90':>11}")
    print(header)
    print("-" * len(header))
    for name, assumptions in SCENARIOS.items():
        r = StochasticDCF(company, assumptions).run()
        per_share = (r.simulations - NET_DEBT) / SHARES
        p10, p50, p90 = (float(np.percentile(per_share, q)) for q in (10, 50, 90))
        print(f"{name:<52}"
              f"{r.p10_value / 1e9:>8.0f}B{r.median_value / 1e9:>8.0f}B{r.p90_value / 1e9:>8.0f}B"
              f"{p10:>10.0f}${p50:>10.0f}${p90:>10.0f}$")


if __name__ == "__main__":
    main()
