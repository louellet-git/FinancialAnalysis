"""Point-in-time vintage backtest of mechanical operating projections.

Walk-forward harness: at each historical "vintage" date, rebuild the company
from only the annual periods that had been *filed* by that date, derive
projection parameters mechanically (no judgment inputs), project revenue and
FCF forward, then score the projections against fiscal years that have since
been reported.

Honesty mechanisms, in order of importance:

1. **Point-in-time estimates.** ``vintage_periods`` filters on
   ``PeriodFinancials.filed_date`` — a period is visible to a vintage only if
   its filing date is on or before the as-of date. Filtering on ``period_end``
   instead would leak restated/late-filed figures into history the model could
   never have seen. ``filed_date`` is the point-in-time key.

   Caveat inherited from the importer: for facts sourced from the SEC
   companyfacts API, ``filed_date`` is the date of the filing the figure was
   drawn from — often a *later* 10-K's comparative column, not the fiscal
   year's own 10-K. Vintages built from such a store are therefore
   conservative (they may see fewer periods than a real-time observer did),
   never optimistic. The leakage direction is the safe one.

2. **Asymmetric actuals, disclosed.** Realized actuals are joined from the
   full store *as currently reported* — they may reflect restatements the
   vintage could not foresee. Estimates are point-in-time; actuals are
   as-currently-reported. This asymmetry is deliberate (it is how a forecast
   is judged in practice) and is documented rather than hidden.

3. **Data flags, not data fixes.** Known input defects (e.g. IBM's FY2019-20
   revenue/OCF recast mismatch) are *surfaced* on any vintage whose parameter
   window touches a flagged year (``VintageParams.data_flags``). The backtest
   does not secretly repair its inputs; it marks them.

Scope: operating projections only (revenue, FCF). No WACC, no terminal value,
no valuation — this module deliberately does not import StochasticDCF's
valuation machinery. It backtests the projection engine, not a price.
"""
from __future__ import annotations
from dataclasses import dataclass, field
from datetime import date, timedelta
from typing import Dict, List, Optional, Tuple

import numpy as np

from financial_analysis.schema import CompanyFinancials, PeriodFinancials

_ANNUAL_FORMS = {"10-K", "10-K/A"}
# Mirror of dcf._MARGIN_LOOKBACK: parameters derive from up to the last 5
# clean annual periods available at the vintage date.
_PARAM_LOOKBACK = 5
_MIN_USABLE_PERIODS = 3
# Mirror of dcf.derive_fcf_margin's floor, applied to both derived stds so a
# too-smooth history cannot collapse the projection bands to a point.
_STD_FLOOR = 0.01
# Vintages sit at fiscal-year-end + this lag, i.e. after the 10-K is filed.
FILING_LAG_DAYS = 90

# Known input defects by ticker -> {fiscal_year: description}. A vintage whose
# parameter window includes a flagged year carries the flag in its output.
# Sourced from analysis data notes (see analysis/IBM/README.md, REVIEW.md F13).
DATA_FLAGS: Dict[str, Dict[int, str]] = {
    "IBM": {
        2019: ("FY2019-20 recast mismatch: continuing-ops revenue vs "
               "unrecast consolidated OCF (margin/growth distorted)"),
        2020: ("FY2019-20 recast mismatch: continuing-ops revenue vs "
               "unrecast consolidated OCF (margin/growth distorted)"),
    },
}


# ---------------------------------------------------------------------------
# Period selection
# ---------------------------------------------------------------------------

def _clean_annuals(company: CompanyFinancials) -> List[PeriodFinancials]:
    """Annual (10-K) periods usable by the backtest, sorted by period_end.

    Requires both ``revenue`` and ``operating_cash_flow`` to be present. This
    filters the phantom "10-K" rows the importer can emit — stray rows with
    only 1-2 fields (e.g. IBM's 2012-06-30, 2018-01-01, 2019-01-01 rows) —
    without touching legitimate periods. Duplicate fiscal years (10-K plus a
    10-K/A) collapse to the latest-filed row.
    """
    rows = [
        p for p in company.periods
        if p.form in _ANNUAL_FORMS
        and p.revenue is not None
        and p.operating_cash_flow is not None
    ]
    by_fy: Dict[int, PeriodFinancials] = {}
    for p in sorted(rows, key=lambda p: (p.period_end, p.filed_date)):
        by_fy[p.period_end.year] = p
    return [by_fy[y] for y in sorted(by_fy)]


def vintage_periods(company: CompanyFinancials, as_of: date) -> List[PeriodFinancials]:
    """Clean annual periods with ``filed_date <= as_of`` — TRUE point-in-time.

    This is the backtest's honesty mechanism: a vintage sees only what had
    been *filed* by its as-of date. A period whose fiscal year ended before
    ``as_of`` but whose filing (or, for companyfacts-sourced stores, the
    filing its figures were drawn from) came later is excluded, so restated
    or late-arriving figures cannot leak into the estimate side.
    """
    return [p for p in _clean_annuals(company) if p.filed_date <= as_of]


# ---------------------------------------------------------------------------
# Mechanical parameters
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class VintageParams:
    """Projection inputs derived mechanically from one vintage's periods.

    Every field is computed from the vintage's own history — no judgment
    inputs, no forward information.
    """
    ticker: str
    as_of: Optional[date]
    base_fiscal_year: int          # fiscal year of the latest visible period
    base_revenue: float            # latest visible annual revenue
    growth: float                  # trailing revenue CAGR over the window
    growth_std: float              # std of YoY growth in the window (floored)
    fcf_margin_mean: float         # mean FCF margin over the window
    fcf_margin_std: float          # std of FCF margin (floored)
    window_years: Tuple[int, ...]  # fiscal years the parameters derive from
    data_flags: Tuple[str, ...]    # known input defects touching the window


def mechanical_params(
    periods: List[PeriodFinancials],
    as_of: Optional[date] = None,
) -> VintageParams:
    """Derive projection parameters mechanically from a vintage's periods.

    - growth: trailing revenue CAGR over up to the last _PARAM_LOOKBACK clean
      annual periods available; growth_std is the std of year-over-year growth
      rates within that window (floored at _STD_FLOOR).
    - FCF margin mean/std: mirror of dcf.derive_fcf_margin applied to the
      vintage's own window (mean of fcf_margin over up to the last 5 annuals,
      std floored at 0.01).
    - base revenue: the latest visible annual revenue.

    Raises ValueError when fewer than _MIN_USABLE_PERIODS usable annual
    periods are available — a vintage with less history than that cannot
    support a mechanical derivation.
    """
    if len(periods) < _MIN_USABLE_PERIODS:
        raise ValueError(
            f"only {len(periods)} usable annual period(s) available — at "
            f"least {_MIN_USABLE_PERIODS} are required to derive mechanical "
            "backtest parameters (revenue CAGR and FCF margin distribution)."
        )
    window = sorted(periods, key=lambda p: p.period_end)[-_PARAM_LOOKBACK:]
    first, last = window[0], window[-1]

    span = last.period_end.year - first.period_end.year
    if span <= 0 or not first.revenue or first.revenue <= 0:
        raise ValueError(
            f"{last.ticker}: cannot derive trailing revenue CAGR — window "
            f"spans {span} year(s) with first revenue {first.revenue!r}."
        )
    growth = (last.revenue / first.revenue) ** (1.0 / span) - 1.0

    yoy = [
        b.revenue / a.revenue - 1.0
        for a, b in zip(window, window[1:])
        if a.revenue and b.revenue is not None
    ]
    growth_std = max(float(np.std(yoy)), _STD_FLOOR) if yoy else _STD_FLOOR

    margins = [p.fcf_margin for p in window if p.fcf_margin is not None]
    if not margins:
        raise ValueError(
            f"{last.ticker}: no annual period in the vintage window has both "
            "FCF and revenue — FCF margin cannot be derived."
        )
    marr = np.array(margins)
    fcf_margin_mean = float(marr.mean())
    fcf_margin_std = max(float(marr.std()), _STD_FLOOR)

    ticker = last.ticker.upper()
    flag_map = DATA_FLAGS.get(ticker, {})
    window_years = tuple(p.period_end.year for p in window)
    data_flags = tuple(sorted({flag_map[y] for y in window_years if y in flag_map}))

    return VintageParams(
        ticker=ticker,
        as_of=as_of,
        base_fiscal_year=last.period_end.year,
        base_revenue=float(last.revenue),
        growth=float(growth),
        growth_std=growth_std,
        fcf_margin_mean=fcf_margin_mean,
        fcf_margin_std=fcf_margin_std,
        window_years=window_years,
        data_flags=data_flags,
    )


# ---------------------------------------------------------------------------
# Projection
# ---------------------------------------------------------------------------

@dataclass
class VintageProjection:
    """Monte Carlo revenue/FCF paths for years +1..+horizon from a vintage.

    Percentile lists are indexed by horizon year (index 0 = year +1). Same
    parameter-MC structure as the aggregate engine (StochasticDCF): one
    normal growth draw and one normal margin draw per path, held constant
    across the path; margins clipped to [0, 1]. No WACC, no terminal value —
    operating projections only.
    """
    params: VintageParams
    horizon: int
    n: int
    seed: int
    target_fiscal_years: List[int]
    rev_p10: List[float]
    rev_p50: List[float]
    rev_p90: List[float]
    fcf_p10: List[float]
    fcf_p50: List[float]
    fcf_p90: List[float]


def project(
    params: VintageParams,
    horizon: int = 3,
    n: int = 10_000,
    seed: int = 7,
) -> VintageProjection:
    """Project revenue and FCF for years +1..+horizon from the vintage base.

    One growth draw and one margin draw per path (normal, seeded), mirroring
    StochasticDCF's sampling structure; revenue_t = base * (1+g)^t and
    fcf_t = revenue_t * margin. Records P10/P50/P90 per year for both metrics.
    """
    rng = np.random.default_rng(seed)
    growth = rng.normal(params.growth, params.growth_std, n)
    margins = np.clip(rng.normal(params.fcf_margin_mean, params.fcf_margin_std, n), 0.0, 1.0)

    years = np.arange(1, horizon + 1).reshape(-1, 1)  # (horizon, 1)
    revenues = params.base_revenue * (1.0 + growth) ** years  # (horizon, n)
    fcfs = revenues * margins

    def _pct(a: np.ndarray, q: float) -> List[float]:
        return [float(v) for v in np.percentile(a, q, axis=1)]

    return VintageProjection(
        params=params,
        horizon=horizon,
        n=n,
        seed=seed,
        target_fiscal_years=[params.base_fiscal_year + t for t in range(1, horizon + 1)],
        rev_p10=_pct(revenues, 10), rev_p50=_pct(revenues, 50), rev_p90=_pct(revenues, 90),
        fcf_p10=_pct(fcfs, 10), fcf_p50=_pct(fcfs, 50), fcf_p90=_pct(fcfs, 90),
    )


# ---------------------------------------------------------------------------
# Backtest
# ---------------------------------------------------------------------------

@dataclass
class VintageOutcome:
    """One vintage x target-fiscal-year x metric scored row.

    actual / error_pct / in_band are None where the target fiscal year has
    not (yet) been reported. error_pct = (actual / P50 - 1) * 100: positive
    means the realized actual came in above the vintage's median forecast.
    """
    target_fiscal_year: int
    metric: str  # "revenue" | "fcf"
    p10: float
    p50: float
    p90: float
    actual: Optional[float]
    error_pct: Optional[float]
    in_band: Optional[bool]


@dataclass
class Vintage:
    params: VintageParams
    projection: VintageProjection
    outcomes: List[VintageOutcome]


@dataclass
class BacktestResult:
    ticker: str
    horizon: int
    vintages: List[Vintage]
    # (as_of, reason) for fiscal-year-end vintage dates with too little history
    skipped: List[Tuple[date, str]] = field(default_factory=list)

    def table(self) -> List[dict]:
        """Flat rows: vintage, base/target FY, metric, P10/P50/P90, actual,
        error_pct, in_band (None where unrealized), data_flags."""
        rows = []
        for v in self.vintages:
            for o in v.outcomes:
                rows.append({
                    "vintage": v.params.as_of,
                    "base_fy": v.params.base_fiscal_year,
                    "target_fy": o.target_fiscal_year,
                    "metric": o.metric,
                    "p10": o.p10,
                    "p50": o.p50,
                    "p90": o.p90,
                    "actual": o.actual,
                    "error_pct": o.error_pct,
                    "in_band": o.in_band,
                    "data_flags": list(v.params.data_flags),
                })
        return rows

    def coverage(self) -> Tuple[int, int]:
        """(realized outcomes inside [P10, P90], total realized outcomes)."""
        realized = [r for r in self.table() if r["in_band"] is not None]
        return sum(1 for r in realized if r["in_band"]), len(realized)

    def bias_by_horizon(self) -> Dict[int, Dict[str, Tuple[float, int]]]:
        """{horizon_year: {metric: (mean signed error %, n realized)}}.

        Positive mean error: actuals came in above the P50 forecasts (the
        mechanical model underestimates); negative: it overestimates.
        """
        out: Dict[int, Dict[str, Tuple[float, int]]] = {}
        buckets: Dict[Tuple[int, str], List[float]] = {}
        for r in self.table():
            if r["error_pct"] is None:
                continue
            h = r["target_fy"] - r["base_fy"]
            buckets.setdefault((h, r["metric"]), []).append(r["error_pct"])
        for (h, metric), errs in sorted(buckets.items()):
            out.setdefault(h, {})[metric] = (float(np.mean(errs)), len(errs))
        return out


def run_backtest(
    company: CompanyFinancials,
    horizon: int = 3,
    n: int = 10_000,
    seed: int = 7,
) -> BacktestResult:
    """Walk-forward backtest: one vintage per clean fiscal-year-end.

    Vintage dates sit at fiscal-year-end + FILING_LAG_DAYS (~90d, i.e. after
    the year's 10-K is normally filed). Each vintage derives mechanical
    parameters from its point-in-time periods and projects +1..+horizon.
    Realized actuals are joined from the full store as currently reported
    (restatements included — see module docstring on this asymmetry). Vintage
    dates with fewer than the minimum usable periods are recorded in
    ``skipped`` with the reason, not silently dropped.
    """
    clean = _clean_annuals(company)
    actuals_by_fy = {p.period_end.year: p for p in clean}

    vintages: List[Vintage] = []
    skipped: List[Tuple[date, str]] = []
    for anchor in clean:
        as_of = anchor.period_end + timedelta(days=FILING_LAG_DAYS)
        visible = vintage_periods(company, as_of)
        try:
            params = mechanical_params(visible, as_of=as_of)
        except ValueError as e:
            skipped.append((as_of, str(e)))
            continue

        proj = project(params, horizon=horizon, n=n, seed=seed)
        outcomes: List[VintageOutcome] = []
        for i, target_fy in enumerate(proj.target_fiscal_years):
            actual_period = actuals_by_fy.get(target_fy)
            for metric, p10s, p50s, p90s in (
                ("revenue", proj.rev_p10, proj.rev_p50, proj.rev_p90),
                ("fcf", proj.fcf_p10, proj.fcf_p50, proj.fcf_p90),
            ):
                actual: Optional[float] = None
                if actual_period is not None:
                    actual = (
                        actual_period.revenue if metric == "revenue"
                        else actual_period.free_cash_flow
                    )
                p10, p50, p90 = p10s[i], p50s[i], p90s[i]
                error_pct = (
                    (actual / p50 - 1.0) * 100.0
                    if actual is not None and p50 else None
                )
                in_band = (p10 <= actual <= p90) if actual is not None else None
                outcomes.append(VintageOutcome(
                    target_fiscal_year=target_fy, metric=metric,
                    p10=p10, p50=p50, p90=p90,
                    actual=actual, error_pct=error_pct, in_band=in_band,
                ))
        vintages.append(Vintage(params=params, projection=proj, outcomes=outcomes))

    return BacktestResult(
        ticker=company.ticker.upper(),
        horizon=horizon,
        vintages=vintages,
        skipped=skipped,
    )
