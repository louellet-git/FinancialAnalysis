import numpy as np
import pytest

from financial_analysis.driver_dcf import (
    ITEMS,
    Driver,
    DriverAttribution,
    DriverDCF,
    DriverDCFResult,
    DriverModel,
    SegmentSpec,
    effective_terminal_growth,
    fcff_margin,
    revenue_paths,
    terminal_value,
    wacc_capm,
)

SEED = 42


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

def _standard_drivers() -> list[Driver]:
    return [
        Driver("g_software", stochastic=True, mean=0.08, std=0.02,
               rationale="analyst range on software growth"),
        Driver("g_infra", stochastic=True, mean=0.02, std=0.01,
               rationale="GDP-like hardware growth"),
        Driver("core_margin", stochastic=True, mean=0.20, std=0.02,
               dist="truncnorm", lo=0.0, hi=0.35,
               rationale="historical FCFF margin band"),
        Driver("sbc_ratio", stochastic=False, mean=0.03,
               rationale="stable SBC/revenue, policy-set"),
        Driver("ma_ratio", stochastic=False, mean=0.02,
               rationale="programmatic tuck-in M&A cadence"),
        Driver("ma_recurrence", stochastic=False, mean=1.0,
               rationale="treat all M&A as recurring (conservative)"),
        Driver("rf", stochastic=True, mean=0.042, std=0.004,
               rationale="10y yield uncertainty"),
        Driver("beta", stochastic=False, mean=0.9,
               rationale="regression beta, stable"),
        Driver("erp", stochastic=False, mean=0.05,
               rationale="Damodaran implied ERP, pinned"),
        Driver("kd_after_tax", stochastic=False, mean=0.035,
               rationale="observed debt cost x (1-tax)"),
        Driver("equity_weight", stochastic=False, mean=0.85,
               rationale="market-value capital structure"),
        Driver("terminal_growth", stochastic=True, mean=0.025, std=0.005,
               rationale="long-run nominal GDP anchor"),
    ]


def _standard_model() -> DriverModel:
    return DriverModel(
        drivers=_standard_drivers(),
        corr={
            ("g_software", "g_infra"): 0.3,
            ("g_software", "core_margin"): 0.2,
        },
    )


def _standard_engine(**kwargs) -> DriverDCF:
    defaults = dict(net_debt=50e9, shares=1e9, projection_years=10)
    defaults.update(kwargs)
    return DriverDCF(
        _standard_model(),
        [
            SegmentSpec("software", base_revenue=30e9, growth_driver="g_software"),
            SegmentSpec("infra", base_revenue=20e9, growth_driver="g_infra"),
        ],
        **defaults,
    )


# ---------------------------------------------------------------------------
# Driver validation
# ---------------------------------------------------------------------------

def test_stochastic_driver_requires_positive_std():
    with pytest.raises(ValueError, match="std > 0"):
        Driver("g", stochastic=True, mean=0.05)


def test_deterministic_driver_rejects_nonzero_std():
    with pytest.raises(ValueError, match="deterministic"):
        Driver("g", stochastic=False, mean=0.05, std=0.01)


def test_truncnorm_requires_a_bound():
    with pytest.raises(ValueError, match="lo and/or hi"):
        Driver("g", stochastic=True, mean=0.05, std=0.01, dist="truncnorm")


def test_bounds_on_normal_dist_rejected():
    with pytest.raises(ValueError, match="truncnorm"):
        Driver("g", stochastic=True, mean=0.05, std=0.01, lo=0.0)


def test_inverted_bounds_rejected():
    with pytest.raises(ValueError, match="must be <"):
        Driver("g", stochastic=True, mean=0.05, std=0.01,
               dist="truncnorm", lo=0.1, hi=0.0)


def test_mean_outside_bounds_rejected():
    with pytest.raises(ValueError, match="below lo"):
        Driver("g", stochastic=True, mean=-0.5, std=0.01,
               dist="truncnorm", lo=0.0, hi=1.0)


def test_unknown_dist_rejected():
    with pytest.raises(ValueError, match="dist"):
        Driver("g", stochastic=True, mean=0.05, std=0.01, dist="lognormal")


# ---------------------------------------------------------------------------
# Correlation matrix validation
# ---------------------------------------------------------------------------

def _two_driver_model(rho: float) -> DriverModel:
    return DriverModel(
        drivers=[
            Driver("a", stochastic=True, mean=0.0, std=1.0),
            Driver("b", stochastic=True, mean=0.0, std=1.0),
        ],
        corr={("a", "b"): rho},
    )


def test_non_psd_matrix_rejected_with_eigenvalue():
    model = DriverModel(
        drivers=[
            Driver("a", stochastic=True, mean=0.0, std=1.0),
            Driver("b", stochastic=True, mean=0.0, std=1.0),
            Driver("c", stochastic=True, mean=0.0, std=1.0),
        ],
        corr={("a", "b"): 0.9, ("a", "c"): 0.9, ("b", "c"): -0.9},
    )
    with pytest.raises(ValueError, match="minimum eigenvalue -"):
        model.correlation_matrix()


def test_correlation_out_of_range_rejected():
    with pytest.raises(ValueError, match=r"outside \[-1, 1\]"):
        _two_driver_model(1.5).correlation_matrix()


def test_correlation_unknown_driver_rejected():
    model = DriverModel(
        drivers=[Driver("a", stochastic=True, mean=0.0, std=1.0)],
        corr={("a", "zzz"): 0.5},
    )
    with pytest.raises(ValueError, match="unknown"):
        model.correlation_matrix()


def test_correlation_with_deterministic_driver_rejected():
    model = DriverModel(
        drivers=[
            Driver("a", stochastic=True, mean=0.0, std=1.0),
            Driver("d", stochastic=False, mean=1.0),
        ],
        corr={("a", "d"): 0.5},
    )
    with pytest.raises(ValueError, match="deterministic"):
        model.correlation_matrix()


def test_conflicting_symmetric_entries_rejected():
    model = DriverModel(
        drivers=[
            Driver("a", stochastic=True, mean=0.0, std=1.0),
            Driver("b", stochastic=True, mean=0.0, std=1.0),
        ],
        corr={("a", "b"): 0.5, ("b", "a"): 0.3},
    )
    with pytest.raises(ValueError, match="Conflicting"):
        model.correlation_matrix()


def test_self_correlation_rejected():
    model = DriverModel(
        drivers=[Driver("a", stochastic=True, mean=0.0, std=1.0)],
        corr={("a", "a"): 1.0},
    )
    with pytest.raises(ValueError, match="Self-correlation"):
        model.correlation_matrix()


def test_matrix_excludes_deterministic_drivers():
    model = _standard_model()
    n_stochastic = sum(d.stochastic for d in model.drivers)
    mat = model.correlation_matrix()
    assert mat.shape == (n_stochastic, n_stochastic)
    assert np.allclose(np.diag(mat), 1.0)
    assert np.allclose(mat, mat.T)


def test_duplicate_driver_names_rejected():
    with pytest.raises(ValueError, match="Duplicate"):
        DriverModel(drivers=[
            Driver("a", stochastic=False, mean=1.0),
            Driver("a", stochastic=False, mean=2.0),
        ])


# ---------------------------------------------------------------------------
# Sampling
# ---------------------------------------------------------------------------

def test_specified_correlation_is_realized_in_samples():
    rho = 0.6
    draws = _two_driver_model(rho).sample(50_000, seed=SEED)
    realized = np.corrcoef(draws["a"], draws["b"])[0, 1]
    assert realized == pytest.approx(rho, abs=0.05)


def test_deterministic_driver_yields_constant_array():
    draws = _standard_model().sample(1000, seed=SEED)
    assert draws["beta"].shape == (1000,)
    assert (draws["beta"] == 0.9).all()


def test_truncnorm_draws_respect_bounds():
    model = DriverModel(drivers=[
        Driver("m", stochastic=True, mean=0.05, std=0.5,
               dist="truncnorm", lo=0.0, hi=0.1),
    ])
    draws = model.sample(10_000, seed=SEED)["m"]
    assert draws.min() >= 0.0
    assert draws.max() <= 0.1


def test_marginal_mean_and_std_realized():
    draws = _two_driver_model(0.6).sample(50_000, seed=SEED)
    assert draws["a"].mean() == pytest.approx(0.0, abs=0.02)
    assert draws["a"].std() == pytest.approx(1.0, abs=0.02)


def test_mean_override_shifts_only_named_driver():
    model = _two_driver_model(0.6)
    base = model.sample(1000, seed=SEED)
    shifted = model.sample(1000, seed=SEED, mean_overrides={"a": 2.0})
    assert np.allclose(shifted["a"], base["a"] + 2.0)
    assert np.array_equal(shifted["b"], base["b"])


def test_mean_override_unknown_driver_rejected():
    with pytest.raises(ValueError, match="unknown"):
        _two_driver_model(0.0).sample(10, seed=SEED, mean_overrides={"zzz": 1.0})


# ---------------------------------------------------------------------------
# Item formulas vs hand-computed values
# ---------------------------------------------------------------------------

def test_revenue_paths_hand_computed():
    segments = {"a": 100.0, "b": 50.0}
    growth = {"a": np.array([0.10, 0.0]), "b": np.array([0.20, 0.0])}
    rev = revenue_paths(segments, growth, years=2)
    assert rev.shape == (2, 2)
    # year 1: 100*1.1 + 50*1.2 = 170 ; zero-growth path stays at 150
    assert rev[0] == pytest.approx([170.0, 150.0])
    # year 2: 100*1.21 + 50*1.44 = 193
    assert rev[1] == pytest.approx([193.0, 150.0])


def test_revenue_paths_missing_growth_draw_rejected():
    with pytest.raises(ValueError, match="growth draws"):
        revenue_paths({"a": 100.0}, {}, years=3)


def test_fcff_margin_hand_computed():
    m = fcff_margin(
        core_margin=np.array([0.30, 0.30]),
        sbc_ratio=np.array([0.05, 0.05]),
        ma_ratio=np.array([0.02, 0.02]),
        ma_recurrence=np.array([1.0, 0.5]),
    )
    # 0.30 - 0.05 - 1.0*0.02 = 0.23 ; 0.30 - 0.05 - 0.5*0.02 = 0.24
    assert m == pytest.approx([0.23, 0.24])


def test_wacc_capm_hand_computed():
    w = wacc_capm(
        rf=np.array([0.04]), beta=np.array([1.2]), erp=np.array([0.05]),
        kd_after_tax=np.array([0.03]), equity_weight=np.array([0.8]),
    )
    # 0.8*(0.04 + 1.2*0.05) + 0.2*0.03 = 0.8*0.10 + 0.006 = 0.086
    assert w == pytest.approx([0.086])


def test_terminal_value_hand_computed_guard_not_binding():
    pv = terminal_value(
        fcff_final=np.array([100.0]), tg=np.array([0.02]),
        wacc=np.array([0.08]), years=5,
    )
    # TV = 100*1.02/0.06 = 1700 ; PV = 1700 / 1.08^5
    assert pv == pytest.approx([1700.0 / 1.08**5])


def test_terminal_value_hand_computed_guard_binding():
    pv = terminal_value(
        fcff_final=np.array([100.0]), tg=np.array([0.07]),
        wacc=np.array([0.08]), years=5,
    )
    # tg capped at 0.08-0.02=0.06: TV = 100*1.06/0.02 = 5300 ; PV = 5300/1.08^5
    assert pv == pytest.approx([5300.0 / 1.08**5])


def test_effective_terminal_growth_guard_never_violated():
    # tg drawn high on purpose so the cap binds on many paths
    model = DriverModel(drivers=[
        Driver("terminal_growth", stochastic=True, mean=0.06, std=0.02),
        Driver("rf", stochastic=True, mean=0.042, std=0.005),
    ])
    draws = model.sample(50_000, seed=SEED)
    wacc = wacc_capm(
        rf=draws["rf"], beta=np.full(50_000, 0.9), erp=np.full(50_000, 0.05),
        kd_after_tax=np.full(50_000, 0.035), equity_weight=np.full(50_000, 0.85),
    )
    eff = effective_terminal_growth(draws["terminal_growth"], wacc)
    assert (wacc - eff >= 0.02 - 1e-12).all()
    assert (eff <= draws["terminal_growth"]).all()
    # the cap actually binds somewhere (otherwise this test proves nothing)
    assert (eff < draws["terminal_growth"]).any()


def test_items_registry_names_the_vetted_formulas():
    by_name = {item.name: item for item in ITEMS}
    assert set(by_name) == {"revenue", "fcff_margin", "wacc", "terminal_value"}
    assert by_name["revenue"].formula is revenue_paths
    assert by_name["terminal_value"].formula is terminal_value
    assert all(item.doc for item in ITEMS)


# ---------------------------------------------------------------------------
# Engine
# ---------------------------------------------------------------------------

def test_run_returns_result_with_ordered_percentiles():
    result = _standard_engine().run(n_simulations=2000, seed=SEED)
    assert isinstance(result, DriverDCFResult)
    assert result.per_share_p10 < result.per_share_p50 < result.per_share_p90


def test_result_exposes_no_mean_or_std():
    result = _standard_engine().run(n_simulations=500, seed=SEED)
    for banned in ("mean_value", "std_value", "mean_per_share", "std_per_share"):
        assert not hasattr(result, banned)


def test_per_year_array_shapes():
    n, years = 500, 7
    result = _standard_engine(projection_years=years).run(n_simulations=n, seed=SEED)
    assert result.per_share_paths.shape == (n,)
    assert result.fcff.shape == (years, n)
    assert result.pv_fcff.shape == (years, n)
    assert result.pv_terminal.shape == (n,)


def test_per_year_arrays_reconcile_to_per_share():
    result = _standard_engine().run(n_simulations=500, seed=SEED)
    ev = result.pv_fcff.sum(axis=0) + result.pv_terminal
    assert np.allclose((ev - 50e9) / 1e9, result.per_share_paths)


def test_tv_ev_share_at_means_is_a_sane_fraction():
    result = _standard_engine().run(n_simulations=500, seed=SEED)
    assert 0.0 < result.tv_ev_share_at_means < 1.0


def test_seed_makes_run_reproducible():
    r1 = _standard_engine().run(n_simulations=1000, seed=SEED)
    r2 = _standard_engine().run(n_simulations=1000, seed=SEED)
    assert r1.per_share_p50 == r2.per_share_p50
    assert np.array_equal(r1.per_share_paths, r2.per_share_paths)
    assert r1.attribution == r2.attribution


def test_attribution_covers_exactly_the_stochastic_drivers():
    result = _standard_engine().run(n_simulations=500, seed=SEED)
    stochastic = {d.name for d in _standard_model().drivers if d.stochastic}
    assert set(result.attribution) == stochastic
    assert all(isinstance(a, DriverAttribution) for a in result.attribution.values())


def test_attribution_growth_driver_sign_sanity():
    result = _standard_engine().run(n_simulations=2000, seed=SEED)
    g = result.attribution["g_software"]
    assert g.p50_plus_1sd > 0    # higher growth mean -> higher P50
    assert g.p50_minus_1sd < 0


def test_attribution_rf_sign_sanity():
    result = _standard_engine().run(n_simulations=2000, seed=SEED)
    rf = result.attribution["rf"]
    assert rf.p50_plus_1sd < 0   # higher risk-free -> higher WACC -> lower P50
    assert rf.p50_minus_1sd > 0


def test_missing_role_driver_raises_naming_it():
    drivers = [d for d in _standard_drivers() if d.name != "rf"]
    model = DriverModel(drivers=drivers, corr={("g_software", "g_infra"): 0.3})
    with pytest.raises(ValueError, match="'rf'"):
        DriverDCF(
            model,
            [SegmentSpec("software", 30e9, "g_software"),
             SegmentSpec("infra", 20e9, "g_infra")],
            net_debt=50e9, shares=1e9,
        )


def test_missing_growth_driver_raises():
    with pytest.raises(ValueError, match="g_cloud"):
        DriverDCF(
            _standard_model(),
            [SegmentSpec("cloud", 10e9, "g_cloud")],
            net_debt=0.0, shares=1e9,
        )


def test_nonpositive_shares_rejected():
    with pytest.raises(ValueError, match="shares"):
        _standard_engine(shares=0.0)


def test_segment_nonpositive_base_revenue_rejected():
    with pytest.raises(ValueError, match="base_revenue"):
        SegmentSpec("software", base_revenue=0.0, growth_driver="g_software")
