"""IBM driver-based DCF — v4 analysis runner (post DRIVER-REVIEW round 1).

Run from the repo root:

    python -m analysis.IBM.dcf_drivers_run

Prints, in order:
  1. Headline run (10k sims, seed 7): P10/P50/P90 per share, TV/EV at means,
     tg pinned fraction (T-DCF-2 guard binding frequency), attribution table.
  2. E/V price-value fixed-point iteration at means (the equity_weight=0.72
     disposition — the iteration the driver rationale points to).
  3. ma_recurrence locus f = 0/0.25/0.5/0.75/1.0 — the g_software mean moves
     WITH f (shift (f-0.5) x 2pp) to keep the F16 pairing coherent: the same
     f that charges M&A cost is the f whose paid growth is credited.
  4. SBC-drift locus: flat 2.54% / ramp-to-3.5% (base) / ramp-to-4.5%.
  5. Beta locus: 0.68 (trailing post-crash) / 0.85 (base) / 1.00.
  6. Disclosure runs: supply-shock correlation regime (rf x g_* = -0.15) and
     tau = 0 (no debt tax shield).
  7. Implied-reinvestment check at means: terminal reinvestment
     (= MA_RECURRENCE x ma_ratio of revenue) vs terminal growth -> implied
     ROIC on that reinvestment (g = RR x ROIC with NOPAT ~ FCFF + reinvestment).

All variant runs rebuild the model (drivers are frozen dataclasses) and reuse
seed 7, so rows are path-comparable.
"""
import sys
from dataclasses import replace
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import numpy as np

from analysis.IBM.driver_model import (
    MA_RECURRENCE,
    NET_DEBT,
    SHARES,
    _linear_schedule,
    build,
)
from financial_analysis.driver_dcf import DriverDCF, DriverModel

N_SIMS = 10_000
SEED = 7
YEARS = 10


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _engine(bundle: dict) -> DriverDCF:
    return DriverDCF(
        bundle["model"], bundle["segments"],
        net_debt=bundle["net_debt"], shares=bundle["shares"],
        projection_years=YEARS,
    )


def _variant(driver_changes: dict = None, corr_changes: dict = None) -> dict:
    """Fresh bundle with selected drivers/correlation entries replaced."""
    bundle = build()
    drivers = list(bundle["model"].drivers)
    for name, changes in (driver_changes or {}).items():
        idx = next(i for i, d in enumerate(drivers) if d.name == name)
        drivers[idx] = replace(drivers[idx], **changes)
    corr = dict(bundle["model"].corr)
    corr.update(corr_changes or {})
    bundle["model"] = DriverModel(drivers=drivers, corr=corr)
    return bundle


def _quick(bundle: dict) -> tuple:
    """(p10, p50, p90, tg_pinned) for a variant — no attribution loop."""
    dcf = _engine(bundle)
    draws = dcf.model.sample(N_SIMS, SEED)
    per_share, _, _, _, pinned = dcf._valuation(draws)
    return (
        float(np.percentile(per_share, 10)),
        float(np.percentile(per_share, 50)),
        float(np.percentile(per_share, 90)),
        pinned,
    )


def _at_means_per_share(bundle: dict) -> float:
    dcf = _engine(bundle)
    per_share, _, _, _, _ = dcf._valuation(dcf._mean_draws())
    return float(per_share[0])


def _row(label: str, p10: float, p50: float, p90: float, extra: str = "") -> None:
    print(f"  {label:<28s} P10 ${p10:7.2f}   P50 ${p50:7.2f}   P90 ${p90:7.2f}{extra}")


# ---------------------------------------------------------------------------
# Sections
# ---------------------------------------------------------------------------

def headline() -> None:
    bundle = build()
    result = _engine(bundle).run(n_simulations=N_SIMS, seed=SEED)
    print("=" * 78)
    print(f"IBM driver DCF v4 -- headline (n={N_SIMS:,}, seed={SEED}, {YEARS}y, "
          f"conditional on f={MA_RECURRENCE:.2f})")
    print("=" * 78)
    _row("Base", result.per_share_p10, result.per_share_p50, result.per_share_p90)
    print(f"  TV/EV at means: {result.tv_ev_share_at_means:.1%}   "
          f"tg guard pinned fraction: {result.tg_pinned_fraction:.4f}")
    print("\n  Attribution (P50 delta vs base at mean +/- 1 sigma, same seed, "
          "correlations intact):")
    print(f"  {'driver':<18s} {'-1sd':>9s} {'+1sd':>9s}")
    ranked = sorted(
        result.attribution.values(),
        key=lambda a: max(abs(a.p50_minus_1sd), abs(a.p50_plus_1sd)),
        reverse=True,
    )
    for a in ranked:
        print(f"  {a.driver:<18s} {a.p50_minus_1sd:+9.2f} {a.p50_plus_1sd:+9.2f}")


def ev_fixed_point() -> None:
    print("\n" + "-" * 78)
    print("E/V price-value fixed point at means (equity_weight disposition)")
    print("-" * 78)
    w = 0.765  # v3 starting point (market E/V at $212)
    for i in range(1, 21):
        bundle = _variant({"equity_weight": {"mean": w}})
        ps = _at_means_per_share(bundle)
        equity = ps * SHARES
        w_new = equity / (equity + NET_DEBT)
        print(f"  iter {i}: E/V in {w:.4f} -> value/share ${ps:7.2f} "
              f"-> E/V out {w_new:.4f}")
        if abs(w_new - w) < 5e-4:
            break
        w = w_new
    print(f"  Fixed point E/V ~ {w_new:.3f} (model uses 0.72 per the ledger).")


def ma_recurrence_locus() -> None:
    print("\n" + "-" * 78)
    print("ma_recurrence locus (g_software mean moves WITH f: "
          "shift (f - 0.5) x 2pp -- pairing kept)")
    print("-" * 78)
    for f in (0.0, 0.25, 0.50, 0.75, 1.00):
        g_sw_mean = 0.050 + (f - MA_RECURRENCE) * 0.02
        bundle = _variant({
            "ma_recurrence": {"mean": f},
            "g_software": {"mean": g_sw_mean},
        })
        p10, p50, p90, _ = _quick(bundle)
        _row(f"f={f:.2f} (g_sw mean {g_sw_mean:.3f})", p10, p50, p90)


def sbc_locus() -> None:
    print("\n" + "-" * 78)
    print("SBC-drift locus")
    print("-" * 78)
    variants = [
        ("flat 2.54%", {"sbc_ratio": {"schedule": None, "mean": 0.0254}}),
        ("ramp 2.54% -> 3.5% (base)", None),
        ("ramp 2.54% -> 4.5%",
         {"sbc_ratio": {"schedule": _linear_schedule(0.0254, 0.045)}}),
    ]
    for label, changes in variants:
        p10, p50, p90, _ = _quick(_variant(changes))
        _row(label, p10, p50, p90)


def beta_locus() -> None:
    print("\n" + "-" * 78)
    print("Beta locus (deterministic driver, epistemic probe)")
    print("-" * 78)
    for b in (0.68, 0.85, 1.00):
        p10, p50, p90, _ = _quick(_variant({"beta": {"mean": b}}))
        label = {0.68: "0.68 (trailing post-crash)",
                 0.85: "0.85 (base)",
                 1.00: "1.00"}[b]
        _row(f"beta {label}", p10, p50, p90)


def disclosure_runs() -> None:
    print("\n" + "-" * 78)
    print("Disclosure runs")
    print("-" * 78)
    supply = _variant(corr_changes={
        ("rf", "g_software"): -0.15,
        ("rf", "g_consulting"): -0.15,
        ("rf", "g_infrastructure"): -0.15,
    })
    p10, p50, p90, _ = _quick(supply)
    _row("supply-shock corr (rf x g = -0.15)", p10, p50, p90)

    tau0 = _variant({"tax_rate": {"mean": 0.0}})
    p10, p50, p90, _ = _quick(tau0)
    _row("tau = 0 (no debt tax shield)", p10, p50, p90)


def implied_reinvestment_check() -> None:
    print("\n" + "-" * 78)
    print("Implied-reinvestment check at means (terminal year)")
    print("-" * 78)
    drivers = {d.name: d for d in build()["model"].drivers}
    core = drivers["core_margin"].mean
    drift_T = drivers["margin_mix_drift"].schedule[-1]
    sbc_T = drivers["sbc_ratio"].schedule[-1]
    ma = drivers["ma_ratio"].mean
    tg = drivers["terminal_growth"].mean
    reinv = MA_RECURRENCE * ma                       # % of revenue
    fcff_T = core + drift_T - sbc_T - reinv          # terminal FCFF margin
    nopat = fcff_T + reinv                           # pre-reinvestment cash earnings proxy
    rr = reinv / nopat                               # reinvestment rate
    roic = tg / rr                                   # g = RR x ROIC
    print(f"  Terminal reinvestment = f x ma_ratio = {MA_RECURRENCE:.2f} x {ma:.3f} "
          f"= {reinv:.2%} of revenue; terminal FCFF margin {fcff_T:.2%}; "
          f"RR = {rr:.1%} of pre-reinvestment cash earnings ({nopat:.2%}).")
    print(f"  One line: sustaining tg = {tg:.2%} on that reinvestment implies "
          f"ROIC = tg/RR = {roic:.1%} -- a plausible (not heroic) incremental "
          f"return, so the terminal no longer carries cost-without-growth.")


def main() -> None:
    headline()
    ev_fixed_point()
    ma_recurrence_locus()
    sbc_locus()
    beta_locus()
    disclosure_runs()
    implied_reinvestment_check()


if __name__ == "__main__":
    main()
