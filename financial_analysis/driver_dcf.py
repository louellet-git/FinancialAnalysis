"""Driver-based Monte Carlo DCF engine.

Sibling of :class:`financial_analysis.dcf.StochasticDCF`, not a replacement.
Where StochasticDCF samples top-level valuation items (growth, margin, WACC,
terminal growth) directly, this engine never samples an item: every top-level
item is a deterministic, vetted formula over underlying *drivers*.

Design contract:

- **Drivers** are the only stochastic objects. Each :class:`Driver` declares —
  explicitly, with a written rationale reviewers can challenge — whether it is
  stochastic or deterministic. Deterministic drivers become constant arrays.
- **Stochastic drivers are sampled jointly** from correlated standard normals
  (full pairwise correlation matrix, Cholesky factorization), then transformed
  to each driver's marginal as ``mean + std * z``.
- **Truncation is clip-after-correlation.** Drivers declaring
  ``dist="truncnorm"`` have their draws clipped to ``[lo, hi]`` *after* the
  correlated transform. This is deliberate and disclosed: clipping piles
  probability mass on the bounds (it is not a true truncated normal) and
  slightly distorts both the clipped marginal and its realized correlations
  with other drivers. The alternative — resampling or a Gaussian-copula
  inverse-CDF — needs scipy or rejection loops; for the mild truncations this
  engine is meant for (keeping rates positive, margins in [0, 1]) the clip
  bias is second-order and the mechanics stay auditable. Reviewers should
  check that no driver's bounds sit within ~2σ of its mean; beyond that the
  distortion stops being negligible.
- **Top-level items** (revenue, FCFF margin, WACC, terminal value) are pure
  functions in this module — the vetted formulas. Each is unit-tested against
  hand-computed values and registered in :data:`ITEMS` with a plain-language
  description for reviewers. :class:`DriverDCF` hard-codes the model shape by
  composing exactly these functions; it introduces no formula of its own.
- **Terminal value carries a hard joint guard** (implements the deferred
  ticket T-DCF-2 from analysis/IBM/REVIEW.md): effective terminal growth per
  path is ``min(tg_draw, wacc_path - 0.02)``, so the Gordon denominator is at
  least 2% on every path — no 0.001-clip absurdities. Per the same ticket,
  results expose percentiles only: no mean/std (fat-tailed DCF outputs make
  them misleading).

Known limitation, disclosed: each segment's growth is one draw held constant
across all projection years of a path (no mean reversion / AR(1) dynamics —
that is ticket T-DCF-1 territory).
"""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Callable, Dict, List, Optional, Sequence, Tuple

import numpy as np

#: Gordon-denominator floor: effective terminal growth is capped at
#: wacc - _TG_WACC_GAP on every path (T-DCF-2).
_TG_WACC_GAP = 0.02

_ALLOWED_DISTS = ("normal", "truncnorm")


# ---------------------------------------------------------------------------
# Driver / Item declarations
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class Driver:
    """One underlying model input.

    ``stochastic`` is an explicit, reviewable choice: set ``rationale`` to why
    the driver is (or is not) sampled and where its parameters come from.
    Deterministic drivers must leave ``std`` at 0 and carry no bounds —
    ambiguity is rejected, not repaired.
    """

    name: str                     # unique key, e.g. "g_software"
    stochastic: bool
    mean: float                   # deterministic value when stochastic=False
    std: float = 0.0              # required > 0 when stochastic
    dist: str = "normal"          # "normal" | "truncnorm"
    lo: Optional[float] = None    # clip bounds, truncnorm only (see module doc)
    hi: Optional[float] = None
    rationale: str = ""           # why stochastic/deterministic + parameter source

    def __post_init__(self) -> None:
        if not self.name:
            raise ValueError("Driver name must be non-empty.")
        if self.dist not in _ALLOWED_DISTS:
            raise ValueError(
                f"Driver {self.name!r}: dist must be one of {_ALLOWED_DISTS}, "
                f"got {self.dist!r}."
            )
        if self.stochastic:
            if self.std <= 0.0:
                raise ValueError(
                    f"Driver {self.name!r} is stochastic but std={self.std} — "
                    "stochastic drivers require std > 0."
                )
        else:
            if self.std != 0.0:
                raise ValueError(
                    f"Driver {self.name!r} is deterministic but std={self.std} — "
                    "set std=0 (or mark the driver stochastic)."
                )
            if self.lo is not None or self.hi is not None:
                raise ValueError(
                    f"Driver {self.name!r} is deterministic but declares bounds — "
                    "bounds only apply to stochastic truncnorm drivers."
                )
        if self.dist == "truncnorm":
            if self.lo is None and self.hi is None:
                raise ValueError(
                    f"Driver {self.name!r}: dist='truncnorm' requires lo and/or hi."
                )
        elif self.lo is not None or self.hi is not None:
            raise ValueError(
                f"Driver {self.name!r}: lo/hi set but dist='normal' — declare "
                "dist='truncnorm' to make the truncation explicit."
            )
        if self.lo is not None and self.hi is not None and self.lo >= self.hi:
            raise ValueError(
                f"Driver {self.name!r}: lo={self.lo} must be < hi={self.hi}."
            )
        if self.lo is not None and self.mean < self.lo:
            raise ValueError(
                f"Driver {self.name!r}: mean={self.mean} below lo={self.lo}."
            )
        if self.hi is not None and self.mean > self.hi:
            raise ValueError(
                f"Driver {self.name!r}: mean={self.mean} above hi={self.hi}."
            )


@dataclass(frozen=True)
class Item:
    """A top-level valuation item: a named, documented, deterministic formula.

    ``formula`` receives driver arrays of shape (n,) (plus any structural
    context such as segment bases or projection years) and returns arrays.
    Items are never sampled — all randomness lives in the drivers.
    """

    name: str   # e.g. "revenue"
    doc: str    # the formula in words, for reviewers
    formula: Callable[..., np.ndarray]


# ---------------------------------------------------------------------------
# Driver model: correlation matrix + joint sampling
# ---------------------------------------------------------------------------

@dataclass
class DriverModel:
    """The full set of drivers plus pairwise correlations between the
    stochastic ones.

    ``corr`` keys are unordered driver-name pairs; a missing pair means 0.0.
    The assembled matrix (stochastic drivers, declaration order) is validated
    symmetric with unit diagonal and positive definite via Cholesky — an
    inconsistent or non-PSD specification raises ValueError naming the
    offending entry/eigenvalue. It is never silently repaired.
    """

    drivers: List[Driver]
    corr: Dict[Tuple[str, str], float] = field(default_factory=dict)

    def __post_init__(self) -> None:
        names = [d.name for d in self.drivers]
        dupes = {n for n in names if names.count(n) > 1}
        if dupes:
            raise ValueError(f"Duplicate driver names: {sorted(dupes)}.")

    # -- lookups ------------------------------------------------------------

    def driver(self, name: str) -> Driver:
        for d in self.drivers:
            if d.name == name:
                return d
        raise ValueError(f"Unknown driver {name!r}.")

    @property
    def stochastic_drivers(self) -> List[Driver]:
        """Stochastic drivers in declaration order (the matrix ordering)."""
        return [d for d in self.drivers if d.stochastic]

    # -- correlation matrix ---------------------------------------------------

    def correlation_matrix(self) -> np.ndarray:
        """Correlation matrix over stochastic drivers, declaration order.

        Deterministic drivers are excluded — correlating a constant is
        meaningless, and declaring such a pair is rejected as a likely error.
        """
        stoch = self.stochastic_drivers
        index = {d.name: i for i, d in enumerate(stoch)}
        det_names = {d.name for d in self.drivers if not d.stochastic}

        mat = np.eye(len(stoch))
        seen: Dict[Tuple[str, str], float] = {}
        for (a, b), rho in self.corr.items():
            for name in (a, b):
                if name in det_names:
                    raise ValueError(
                        f"Correlation ({a!r}, {b!r}) references deterministic "
                        f"driver {name!r} — only stochastic drivers correlate."
                    )
                if name not in index:
                    raise ValueError(
                        f"Correlation ({a!r}, {b!r}) references unknown "
                        f"driver {name!r}."
                    )
            if a == b:
                raise ValueError(
                    f"Self-correlation ({a!r}, {a!r}) is implicit (1.0) — "
                    "remove it."
                )
            if not -1.0 <= rho <= 1.0:
                raise ValueError(
                    f"Correlation ({a!r}, {b!r}) = {rho} outside [-1, 1]."
                )
            key = (a, b) if a < b else (b, a)
            if key in seen and seen[key] != rho:
                raise ValueError(
                    f"Conflicting correlations for pair {key}: "
                    f"{seen[key]} vs {rho} — the matrix must be symmetric."
                )
            seen[key] = rho
            i, j = index[a], index[b]
            mat[i, j] = mat[j, i] = rho

        try:
            np.linalg.cholesky(mat)
        except np.linalg.LinAlgError:
            min_eig = float(np.linalg.eigvalsh(mat).min())
            raise ValueError(
                "Driver correlation matrix is not positive definite: minimum "
                f"eigenvalue {min_eig:.6g} (must be > 0). Revise the pairwise "
                "correlations — the matrix is not silently repaired."
            ) from None
        return mat

    # -- sampling ---------------------------------------------------------

    def sample(
        self,
        n: int,
        seed: Optional[int] = None,
        mean_overrides: Optional[Dict[str, float]] = None,
    ) -> Dict[str, np.ndarray]:
        """Draw n joint paths for every driver.

        Stochastic drivers: correlated standard normals via Cholesky
        (``np.random.default_rng(seed)``), transformed to ``mean + std * z``,
        then clipped to [lo, hi] where declared (see module docstring for the
        disclosed clip-after-correlation distortion). Deterministic drivers:
        constant arrays.

        ``mean_overrides`` shifts the named drivers' means while keeping the
        same z-draws for a given seed — this is what makes ±1σ attribution
        runs comparable path-by-path with all correlations intact.
        """
        if n <= 0:
            raise ValueError(f"n must be positive, got {n}.")
        overrides = dict(mean_overrides or {})
        known = {d.name for d in self.drivers}
        unknown = set(overrides) - known
        if unknown:
            raise ValueError(f"mean_overrides for unknown drivers: {sorted(unknown)}.")

        stoch = self.stochastic_drivers
        chol = np.linalg.cholesky(self.correlation_matrix())
        rng = np.random.default_rng(seed)
        z = chol @ rng.standard_normal((len(stoch), n))

        draws: Dict[str, np.ndarray] = {}
        for i, d in enumerate(stoch):
            mean = overrides.get(d.name, d.mean)
            x = mean + d.std * z[i]
            if d.lo is not None or d.hi is not None:
                x = np.clip(x, d.lo, d.hi)
            draws[d.name] = x
        for d in self.drivers:
            if not d.stochastic:
                draws[d.name] = np.full(n, overrides.get(d.name, d.mean))
        return draws


# ---------------------------------------------------------------------------
# Vetted item formulas (pure functions — each unit-tested against hand values)
# ---------------------------------------------------------------------------

def revenue_paths(
    segments: Dict[str, float],
    growth_draws: Dict[str, np.ndarray],
    years: int,
) -> np.ndarray:
    """Revenue per year: each segment compounds at its own drawn growth.

    revenue_t = Σ_s base_s * (1 + g_s)^t for t = 1..years, shape (years, n).

    Limitation (disclosed): g_s is a single draw held constant over all years
    of a path — no mean reversion. This widens far-year dispersion relative
    to an AR(1) growth process (ticket T-DCF-1).
    """
    if years <= 0:
        raise ValueError(f"years must be positive, got {years}.")
    if not segments:
        raise ValueError("At least one revenue segment is required.")
    missing = set(segments) - set(growth_draws)
    if missing:
        raise ValueError(f"No growth draws for segment(s): {sorted(missing)}.")
    t = np.arange(1, years + 1).reshape(-1, 1)
    return sum(
        base * (1.0 + growth_draws[name]) ** t for name, base in segments.items()
    )


def fcff_margin(
    core_margin: np.ndarray,
    sbc_ratio: np.ndarray,
    ma_ratio: np.ndarray,
    ma_recurrence: np.ndarray,
) -> np.ndarray:
    """FCFF margin = core_margin − sbc_ratio − ma_recurrence × ma_ratio.

    SBC is treated as a full cash-equivalent cost; the recurring share of
    M&A spend (ma_recurrence ∈ [0, 1]) charges acquisition outlay against
    margin. All inputs are (n,) driver arrays.
    """
    return core_margin - sbc_ratio - ma_recurrence * ma_ratio


def wacc_capm(
    rf: np.ndarray,
    beta: np.ndarray,
    erp: np.ndarray,
    kd_after_tax: np.ndarray,
    equity_weight: np.ndarray,
) -> np.ndarray:
    """WACC = E/V × (rf + β × ERP) + (1 − E/V) × Kd_after_tax (CAPM cost of
    equity, after-tax cost of debt, market-value weights)."""
    return equity_weight * (rf + beta * erp) + (1.0 - equity_weight) * kd_after_tax


def effective_terminal_growth(tg: np.ndarray, wacc: np.ndarray) -> np.ndarray:
    """Hard joint guard on terminal growth (implements ticket T-DCF-2).

    Effective tg per path = min(tg_draw, wacc_path − 0.02): the Gordon
    denominator wacc − tg is at least 2% on every path, jointly enforced —
    replaces the aggregate model's 0.001 denominator clip, which let
    near-degenerate paths dominate the tails.
    """
    return np.minimum(tg, wacc - _TG_WACC_GAP)


def terminal_value(
    fcff_final: np.ndarray,
    tg: np.ndarray,
    wacc: np.ndarray,
    years: int,
) -> np.ndarray:
    """Present value of the Gordon terminal value at the end of year `years`.

    TV = fcff_final × (1 + tg_eff) / (wacc − tg_eff), discounted by
    (1 + wacc)^years, with tg_eff from :func:`effective_terminal_growth`
    (T-DCF-2 joint guard) — the denominator is ≥ 2% by construction, so no
    additional clipping is applied or needed.
    """
    tg_eff = effective_terminal_growth(tg, wacc)
    tv = fcff_final * (1.0 + tg_eff) / (wacc - tg_eff)
    return tv / (1.0 + wacc) ** years


#: Reviewer-facing registry of the vetted top-level item formulas.
ITEMS: Tuple[Item, ...] = (
    Item(
        name="revenue",
        doc=(
            "Sum over segments of base revenue compounded at that segment's "
            "drawn growth rate (constant per path)."
        ),
        formula=revenue_paths,
    ),
    Item(
        name="fcff_margin",
        doc=(
            "Core operating FCFF margin minus SBC ratio minus the recurring "
            "share of the M&A-to-revenue ratio."
        ),
        formula=fcff_margin,
    ),
    Item(
        name="wacc",
        doc=(
            "CAPM: equity weight times (risk-free + beta × equity risk "
            "premium) plus debt weight times after-tax cost of debt."
        ),
        formula=wacc_capm,
    ),
    Item(
        name="terminal_value",
        doc=(
            "Gordon growth on final-year FCFF, terminal growth jointly capped "
            "at WACC − 2% per path (T-DCF-2), discounted to present."
        ),
        formula=terminal_value,
    ),
)


# ---------------------------------------------------------------------------
# Engine
# ---------------------------------------------------------------------------

#: Drivers DriverDCF requires by exact name (beyond per-segment growth drivers).
ROLE_DRIVERS: Tuple[str, ...] = (
    "core_margin", "sbc_ratio", "ma_ratio", "ma_recurrence",
    "rf", "beta", "erp", "kd_after_tax", "equity_weight",
    "terminal_growth",
)


@dataclass(frozen=True)
class SegmentSpec:
    """One revenue segment: last-annual base revenue and the name of the
    growth driver that compounds it."""

    name: str
    base_revenue: float
    growth_driver: str

    def __post_init__(self) -> None:
        if self.base_revenue <= 0:
            raise ValueError(
                f"Segment {self.name!r}: base_revenue must be positive, "
                f"got {self.base_revenue}."
            )


@dataclass(frozen=True)
class DriverAttribution:
    """P50 per-share deltas (vs the base run) when one stochastic driver's
    mean is shifted ±1σ with the same seed — same z-draws, correlations
    intact. Feeds the tornado chart and per-driver review docs."""

    driver: str
    p50_minus_1sd: float
    p50_plus_1sd: float


@dataclass
class DriverDCFResult:
    """Per-share equity valuation distribution. Percentiles only — mean/std
    are deliberately absent (T-DCF-2: fat-tailed DCF outputs make moment
    statistics misleading)."""

    per_share_p10: float
    per_share_p50: float
    per_share_p90: float
    tv_ev_share_at_means: float                 # PV(terminal)/EV, all drivers at mean
    attribution: Dict[str, DriverAttribution]   # keyed by stochastic driver name
    per_share_paths: np.ndarray = field(repr=False)  # (n,)
    fcff: np.ndarray = field(repr=False)             # (years, n)
    pv_fcff: np.ndarray = field(repr=False)          # (years, n)
    pv_terminal: np.ndarray = field(repr=False)      # (n,)


class DriverDCF:
    """Driver-based Monte Carlo FCFF DCF.

    Hard-coded model shape, composed exclusively from the vetted formulas in
    this module:

        revenue_t = revenue_paths(segments, growth draws)
        fcff_t    = revenue_t × fcff_margin(core_margin, sbc, m&a, recurrence)
        wacc      = wacc_capm(rf, beta, erp, kd_after_tax, equity_weight)
        EV        = Σ fcff_t/(1+wacc)^t + terminal_value(fcff_T, tg, wacc)
        per share = (EV − net_debt) / shares

    The model must supply drivers named exactly as in :data:`ROLE_DRIVERS`
    plus one growth driver per segment (any name, referenced by the
    SegmentSpec). Missing drivers raise ValueError at construction.
    """

    def __init__(
        self,
        model: DriverModel,
        segments: Sequence[SegmentSpec],
        *,
        net_debt: float,
        shares: float,
        projection_years: int = 10,
    ) -> None:
        if not segments:
            raise ValueError("At least one revenue segment is required.")
        if shares <= 0:
            raise ValueError(f"shares must be positive, got {shares}.")
        if projection_years <= 0:
            raise ValueError(
                f"projection_years must be positive, got {projection_years}."
            )
        seg_names = [s.name for s in segments]
        if len(set(seg_names)) != len(seg_names):
            raise ValueError(f"Duplicate segment names: {seg_names}.")

        available = {d.name for d in model.drivers}
        required = list(ROLE_DRIVERS) + [s.growth_driver for s in segments]
        missing = [name for name in required if name not in available]
        if missing:
            raise ValueError(
                f"DriverModel is missing required driver(s): {sorted(set(missing))}. "
                f"Roles {ROLE_DRIVERS} plus each segment's growth driver must exist."
            )

        self.model = model
        self.segments = list(segments)
        self.net_debt = net_debt
        self.shares = shares
        self.projection_years = projection_years

    # -- internals ---------------------------------------------------------

    def _valuation(
        self, draws: Dict[str, np.ndarray]
    ) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
        """(per_share, fcff, pv_fcff, pv_terminal) from a full draws dict."""
        years = self.projection_years
        growth = {s.name: draws[s.growth_driver] for s in self.segments}
        bases = {s.name: s.base_revenue for s in self.segments}
        revenue = revenue_paths(bases, growth, years)

        margin = fcff_margin(
            draws["core_margin"], draws["sbc_ratio"],
            draws["ma_ratio"], draws["ma_recurrence"],
        )
        fcff = revenue * margin

        wacc = wacc_capm(
            draws["rf"], draws["beta"], draws["erp"],
            draws["kd_after_tax"], draws["equity_weight"],
        )
        if (wacc <= 0.0).any():
            raise ValueError(
                "WACC is non-positive on at least one path — bound the CAPM "
                "drivers (dist='truncnorm') so discounting stays meaningful."
            )

        t = np.arange(1, years + 1).reshape(-1, 1)
        pv_fcff = fcff / (1.0 + wacc) ** t
        pv_term = terminal_value(fcff[-1], draws["terminal_growth"], wacc, years)
        ev = pv_fcff.sum(axis=0) + pv_term
        per_share = (ev - self.net_debt) / self.shares
        return per_share, fcff, pv_fcff, pv_term

    def _mean_draws(self) -> Dict[str, np.ndarray]:
        """Every driver pinned at its mean — a single deterministic path."""
        return {d.name: np.full(1, d.mean) for d in self.model.drivers}

    @staticmethod
    def _p50(per_share: np.ndarray) -> float:
        return float(np.percentile(per_share, 50))

    # -- public API ---------------------------------------------------------

    def run(self, n_simulations: int = 10_000, seed: Optional[int] = None) -> DriverDCFResult:
        """Run the simulation and the ±1σ per-driver attribution.

        Attribution re-runs the full valuation once per stochastic driver and
        direction with that driver's mean shifted by ±1 std, the same seed
        (identical z-draws, correlations intact), and reports the resulting
        P50 per-share delta vs the base run. When ``seed`` is None one is
        drawn once and reused across the base and attribution runs so the
        deltas remain path-comparable.
        """
        if seed is None:
            seed = int(np.random.default_rng().integers(0, 2**32))

        draws = self.model.sample(n_simulations, seed)
        per_share, fcff, pv_fcff, pv_term = self._valuation(draws)
        base_p50 = self._p50(per_share)

        attribution: Dict[str, DriverAttribution] = {}
        for d in self.model.stochastic_drivers:
            deltas = []
            for direction in (-1.0, +1.0):
                shifted = self.model.sample(
                    n_simulations, seed,
                    mean_overrides={d.name: d.mean + direction * d.std},
                )
                shifted_per_share, _, _, _ = self._valuation(shifted)
                deltas.append(self._p50(shifted_per_share) - base_p50)
            attribution[d.name] = DriverAttribution(
                driver=d.name, p50_minus_1sd=deltas[0], p50_plus_1sd=deltas[1],
            )

        _, _, mean_pv_fcff, mean_pv_term = self._valuation(self._mean_draws())
        mean_ev = float(mean_pv_fcff.sum(axis=0)[0] + mean_pv_term[0])
        tv_ev = float(mean_pv_term[0]) / mean_ev

        return DriverDCFResult(
            per_share_p10=float(np.percentile(per_share, 10)),
            per_share_p50=base_p50,
            per_share_p90=float(np.percentile(per_share, 90)),
            tv_ev_share_at_means=tv_ev,
            attribution=attribution,
            per_share_paths=per_share,
            fcff=fcff,
            pv_fcff=pv_fcff,
            pv_terminal=pv_term,
        )
