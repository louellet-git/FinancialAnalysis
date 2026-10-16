"""Investor verdict layer over the driver engine's simulated values.

A read-only consumer of :mod:`financial_analysis.driver_dcf`: it takes the
per-share value paths a run already produced, plus a market price, and
changes no model input. Proposal and sources:
PLANNING/proposals/2026-10-15-investor-verdict-layer.md.

- **Chance the price is too high** = share of paths valued below the price.
  Verdict cutoffs at 25% / 75% mirror Morningstar's bull/bear calibration
  (25% probability beyond each case); the 40% / 60% middle cutoffs are
  judgment, to be tuned by backtest.
- **Horizon returns** assume the price closes a fraction ``k`` of the log
  price-to-value gap per year (Capozza & Israelsen 2010: 15-30%), with the
  dividend yield compounded on top. Full convergence by a fixed date would
  overstate both gains and losses.
- **Price-implied means** solve one stochastic driver's mean (all others at
  base, same seed) for the value whose P50 equals the price — a reverse DCF
  per assumption. ``None`` means unreachable inside the driver's own bounds,
  or within ±6σ of its mean where it declares none.
"""
from __future__ import annotations
from dataclasses import replace
from typing import Optional

import numpy as np

from financial_analysis.driver_dcf import DriverDCF

#: Non-positive path values are floored here before taking logs (a path
#: worth nothing converges toward a near-zero price).
_VALUE_FLOOR = 0.01

#: Search half-width, in driver std, for drivers without declared bounds.
_SEARCH_SIGMAS = 6.0


def chance_price_too_high(per_share_paths: np.ndarray, price: float) -> float:
    """Share of simulated per-share values below the market price."""
    return float(np.mean(np.asarray(per_share_paths) < price))


def verdict_band(p_too_high: float) -> str:
    """Map the chance the price is too high to one of five verdicts."""
    if p_too_high < 0.25:
        return "Strong opportunity"
    if p_too_high < 0.40:
        return "Opportunity"
    if p_too_high <= 0.60:
        return "No edge"
    if p_too_high <= 0.75:
        return "Overpriced"
    return "Strongly overpriced"


def total_return(value, price: float, k: float, years: int,
                 dividend_yield: float = 0.0):
    """Total return factor minus 1 after ``years``, the log gap between price
    and ``value`` shrinking by a fraction ``k`` each year, dividends
    compounded at ``dividend_yield``. ``value`` may be a scalar or an array."""
    value = np.maximum(value, _VALUE_FLOOR)
    log_price = np.log(value) + (1.0 - k) ** years * (np.log(price) - np.log(value))
    return np.exp(log_price) / price * (1.0 + dividend_yield) ** years - 1.0


def annual_return(value, price: float, k: float, years: int,
                  dividend_yield: float = 0.0):
    """:func:`total_return`, annualized."""
    return (1.0 + total_return(value, price, k, years, dividend_yield)) ** (1.0 / years) - 1.0


def chance_of_loss(per_share_paths: np.ndarray, price: float, k: float,
                   years: int, dividend_yield: float = 0.0) -> float:
    """Share of paths whose total return over ``years`` is negative — the
    probability a scored track record grades (not the value-vs-price share)."""
    return float(np.mean(total_return(per_share_paths, price, k, years, dividend_yield) < 0.0))


def _p50_at(bundle: dict, name: str, mean: float, n_simulations: int, seed: int) -> float:
    model = bundle["model"]
    shifted = replace(model, drivers=[
        replace(d, mean=mean) if d.name == name else d for d in model.drivers
    ])
    engine = DriverDCF(shifted, bundle["segments"],
                       net_debt=bundle["net_debt"], shares=bundle["shares"])
    return engine.run(n_simulations=n_simulations, seed=seed).per_share_p50


def price_implied_mean(bundle: dict, name: str, price: float, *,
                       n_simulations: int = 4_000, seed: int = 7,
                       iterations: int = 20) -> Optional[float]:
    """Mean of stochastic driver ``name`` at which the P50 equals ``price``,
    by bisection over the driver's bounds (else mean ± 6σ). ``bundle`` is the
    ``build()`` contract of analysis/<TICKER>/driver_model.py."""
    d = bundle["model"].driver(name)
    if not d.stochastic:
        raise ValueError(f"Driver {name!r} is deterministic; only stochastic drivers are solved.")
    lo = d.lo if d.lo is not None else d.mean - _SEARCH_SIGMAS * d.std
    hi = d.hi if d.hi is not None else d.mean + _SEARCH_SIGMAS * d.std
    gap_lo = _p50_at(bundle, name, lo, n_simulations, seed) - price
    gap_hi = _p50_at(bundle, name, hi, n_simulations, seed) - price
    if gap_lo * gap_hi > 0:
        return None
    for _ in range(iterations):
        mid = (lo + hi) / 2.0
        gap_mid = _p50_at(bundle, name, mid, n_simulations, seed) - price
        if gap_mid * gap_lo > 0:
            lo, gap_lo = mid, gap_mid
        else:
            hi = mid
    return (lo + hi) / 2.0
