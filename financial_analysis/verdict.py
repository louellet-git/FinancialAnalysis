"""Investor verdict over the driver engine's simulated values.

A read-only consumer of :mod:`financial_analysis.driver_dcf`: it takes the
per-share value paths a run already produced, plus a market price, and
changes no model input. Plan and sources:
PLANNING/proposals/2026-10-15-investor-verdict-layer.md.

The deciding number is the chance the price is too high: the share of paths
valued below the price. The 25% / 75% outer cutoffs mirror Morningstar's
bull/bear calibration (25% probability beyond each case); the 40% / 60%
middle cutoffs are judgment, to be tuned by backtest.
"""
import numpy as np


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
