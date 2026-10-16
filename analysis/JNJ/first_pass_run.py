"""JNJ first-pass runner (seed 7, 10k sims). Run from the repo root:

    python -m analysis.JNJ.first_pass_run

Prints the headline, the engine-vs-hand-DCF cross-check at driver means, and
the published loci (f, acquired-growth credit, beta, rf, one stacked case).
Findings: analysis/JNJ/FIRST-PASS.md.
"""
import sys
from dataclasses import replace
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import numpy as np

from analysis.JNJ import driver_model as dm
from financial_analysis.driver_dcf import DriverDCF

PRICE = 255.56  # close 2026-10-16
SEED, N = 7, 10_000


def _run(overrides: dict = None):
    b = dm.build()
    model = b["model"]
    if overrides:
        model = replace(model, drivers=[
            replace(d, mean=overrides[d.name]) if d.name in overrides else d for d in model.drivers
        ])
    engine = DriverDCF(model, b["segments"], net_debt=b["net_debt"], shares=b["shares"])
    return engine, engine.run(N, SEED)


def _f(f: float) -> dict:
    return {"ma_recurrence": f,
            "g_im": dm.IM_ORGANIC_G + f * dm.ACQUIRED_G,
            "g_medtech": dm.MEDTECH_ORGANIC_G + f * dm.ACQUIRED_G}


def _hand_dcf_at_means() -> float:
    """Independent closed-form DCF with every driver at its mean."""
    b = dm.build()
    d = {x.name: x for x in b["model"].drivers}
    m = lambda name: d[name].mean
    wacc = (m("equity_weight") * (m("rf") + m("beta") * m("erp"))
            + (1 - m("equity_weight")) * (m("rf") + m("kd_spread")) * (1 - m("tax_rate")))
    growth = {"Innovative Medicine": m("g_im"), "MedTech": m("g_medtech")}
    pv = 0.0
    for t in range(1, dm.PROJECTION_YEARS + 1):
        revenue = sum(s.base_revenue * (1 + s.year1_growth) * (1 + growth[s.name]) ** (t - 1)
                      for s in b["segments"])
        margin = (m("core_margin") + d["margin_mix_drift"].schedule[t - 1]
                  - m("sbc_ratio") - m("ma_recurrence") * m("ma_ratio"))
        fcff = revenue * margin
        pv += fcff / (1 + wacc) ** t
    tg = min(m("terminal_growth"), wacc - 0.02)
    tv = fcff * (1 + tg) / (wacc - tg) / (1 + wacc) ** dm.PROJECTION_YEARS
    return (pv + tv - b["net_debt"]) / b["shares"]


def main() -> None:
    engine, r = _run()
    paths = r.per_share_paths
    q = np.percentile(paths, [10, 25, 50, 75, 90])
    print(f"JNJ first pass - seed {SEED}, {N:,} sims, price ${PRICE}")
    print("  P10 {:.2f} | P25 {:.2f} | P50 {:.2f} | P75 {:.2f} | P90 {:.2f}".format(*q))
    print(f"  chance the price is too high {np.mean(paths < PRICE):.1%}; TV/EV at means "
          f"{r.tv_ev_share_at_means:.1%}; tg pinned {r.tg_pinned_fraction:.2%}")

    engine_mean = float(np.ravel(engine._valuation(engine._mean_draws())[0])[0])
    print(f"\n  Cross-check at means: engine ${engine_mean:.4f} vs hand DCF ${_hand_dcf_at_means():.4f}")

    print("\n  f-locus (growth credit moves with f):")
    for f in (0.0, 0.25, 0.5, 0.75, 1.0):
        print(f"    f={f:.2f}: P50 {_run(_f(f))[1].per_share_p50:7.2f}")
    print("  Acquired-growth credit at f=0.5:")
    for ag in (0.02, 0.04, 0.06):
        ov = {"g_im": dm.IM_ORGANIC_G + 0.5 * ag, "g_medtech": dm.MEDTECH_ORGANIC_G + 0.5 * ag}
        print(f"    {ag:.0%}: P50 {_run(ov)[1].per_share_p50:7.2f}")
    print("  Beta locus:")
    for beta in (0.45, 0.65, 0.85):
        print(f"    beta {beta:.2f}: P50 {_run({'beta': beta})[1].per_share_p50:7.2f}")
    print(f"  rf 4.6% (July-2026 level): P50 {_run({'rf': 0.046})[1].per_share_p50:7.2f}")
    _, s = _run({**_f(0.0), "beta": 0.45, "rf": 0.046})
    print(f"  Stacked (f=0, beta 0.45, rf 4.6%): P50 {s.per_share_p50:.2f}, "
          f"chance too high {np.mean(s.per_share_paths < PRICE):.1%}")


if __name__ == "__main__":
    main()
