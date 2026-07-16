"""IBM Monte Carlo DCF — bear/base/bull scenarios with equity-adjusted per-share values.

Thin wrapper: the scenario presets and IBM capital-structure override live in
financial_analysis/scenarios.py; this script preserves the original console report.

Prerequisite: python -m scripts.analyze IBM  (populates data/processed/IBM.json)

Usage (from repo root):
    python -m analysis.IBM.dcf_scenarios
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import numpy as np

from financial_analysis import store
from financial_analysis.dcf import StochasticDCF
from financial_analysis.scenarios import CAPITAL_OVERRIDES, scenarios_for

_CAPITAL = CAPITAL_OVERRIDES["IBM"]
NET_DEBT = _CAPITAL.net_debt
SHARES = int(_CAPITAL.shares)

SCENARIOS = {s.label: s.assumptions for s in scenarios_for("IBM")}


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
