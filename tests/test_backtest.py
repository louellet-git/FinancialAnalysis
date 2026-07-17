import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from datetime import date, timedelta

import pytest

from financial_analysis import backtest
from financial_analysis.backtest import (
    FILING_LAG_DAYS,
    BacktestResult,
    mechanical_params,
    project,
    run_backtest,
    vintage_periods,
)
from financial_analysis.schema import CompanyFinancials, PeriodFinancials

GROWTH = 0.05
MARGIN = 0.20
BASE_REVENUE = 100e9
FIRST_YEAR = 2018


def _annual(year: int, revenue, ocf, capex=None, filed: date | None = None,
            form: str = "10-K", ticker: str = "SYN") -> PeriodFinancials:
    return PeriodFinancials(
        ticker=ticker,
        period_end=date(year, 12, 31),
        form=form,
        filed_date=filed or date(year + 1, 2, 15),
        revenue=revenue,
        operating_cash_flow=ocf,
        capex=capex,
    )


def _clean_annual(year: int, ticker: str = "SYN") -> PeriodFinancials:
    """Exact 5%/yr revenue growth, exact 20% FCF margin (OCF 22% - capex 2%)."""
    revenue = BASE_REVENUE * (1 + GROWTH) ** (year - FIRST_YEAR)
    return _annual(year, revenue, ocf=revenue * (MARGIN + 0.02),
                   capex=-revenue * 0.02, ticker=ticker)


def _make_company(n_years: int = 8, ticker: str = "SYN") -> CompanyFinancials:
    periods = [_clean_annual(FIRST_YEAR + i, ticker=ticker) for i in range(n_years)]
    return CompanyFinancials(ticker=ticker, cik="0", company_name="Synthetic Co",
                             periods=periods)


def _as_of(fiscal_year: int) -> date:
    return date(fiscal_year, 12, 31) + timedelta(days=FILING_LAG_DAYS)


# ---------------------------------------------------------------------------
# vintage_periods — point-in-time filtering
# ---------------------------------------------------------------------------

def test_vintage_periods_respects_filed_date_not_period_end():
    company = _make_company(4)
    # FY2021 ended before as_of but was FILED after it — must be excluded.
    late = company.periods[-1]
    assert late.period_end == date(2021, 12, 31)
    late.filed_date = date(2022, 9, 1)

    as_of = date(2022, 3, 31)
    visible = vintage_periods(company, as_of)
    assert [p.period_end.year for p in visible] == [2018, 2019, 2020]

    # Once the filing date passes, the period becomes visible.
    assert [p.period_end.year for p in vintage_periods(company, date(2022, 9, 1))] \
        == [2018, 2019, 2020, 2021]


def test_vintage_periods_filters_phantom_rows():
    company = _make_company(4)
    # Phantom "10-K" rows with 1-2 stray fields (IBM-style): missing OCF,
    # missing revenue, or both -> all excluded.
    company.periods.append(_annual(2016, revenue=1e9, ocf=None, filed=date(2016, 6, 30)))
    company.periods.append(_annual(2015, revenue=None, ocf=1e9, filed=date(2015, 6, 30)))
    company.periods.append(_annual(2014, revenue=None, ocf=None, filed=date(2014, 6, 30)))

    visible = vintage_periods(company, date(2030, 1, 1))
    assert [p.period_end.year for p in visible] == [2018, 2019, 2020, 2021]


def test_vintage_periods_ignores_quarterly_forms():
    company = _make_company(4)
    company.periods.append(_annual(2021, revenue=30e9, ocf=8e9, capex=-1e9,
                                   form="10-Q", filed=date(2021, 5, 1)))
    visible = vintage_periods(company, date(2030, 1, 1))
    assert all(p.form == "10-K" for p in visible)
    assert len(visible) == 4


# ---------------------------------------------------------------------------
# mechanical_params
# ---------------------------------------------------------------------------

def test_mechanical_params_recovers_known_growth_and_margin():
    company = _make_company(8)
    params = mechanical_params(vintage_periods(company, date(2030, 1, 1)),
                               as_of=date(2030, 1, 1))
    assert params.growth == pytest.approx(GROWTH, rel=1e-9)
    assert params.fcf_margin_mean == pytest.approx(MARGIN, rel=1e-9)
    # Constant growth/margin history -> stds collapse to the floor.
    assert params.growth_std == pytest.approx(0.01)
    assert params.fcf_margin_std == pytest.approx(0.01)


def test_mechanical_params_base_is_latest_and_window_is_last_five():
    company = _make_company(8)  # FY2018..FY2025
    params = mechanical_params(vintage_periods(company, date(2030, 1, 1)))
    assert params.base_fiscal_year == 2025
    assert params.base_revenue == pytest.approx(BASE_REVENUE * (1 + GROWTH) ** 7)
    assert params.window_years == (2021, 2022, 2023, 2024, 2025)


def test_mechanical_params_requires_three_usable_periods():
    company = _make_company(2)
    with pytest.raises(ValueError, match="usable annual period"):
        mechanical_params(vintage_periods(company, date(2030, 1, 1)))


def test_mechanical_params_no_forward_information():
    # Params at an early as_of must match params from a company that simply
    # ends there — the future periods contribute nothing.
    full = _make_company(8)
    truncated = _make_company(4)
    as_of = _as_of(2021)
    p_full = mechanical_params(vintage_periods(full, as_of), as_of=as_of)
    p_trunc = mechanical_params(vintage_periods(truncated, as_of), as_of=as_of)
    assert p_full == p_trunc


def test_mechanical_params_data_flags_propagate(monkeypatch):
    monkeypatch.setitem(backtest.DATA_FLAGS, "SYN", {2020: "synthetic flag"})
    company = _make_company(8)
    # Window including 2020 -> flagged.
    flagged = mechanical_params(vintage_periods(company, _as_of(2020)))
    assert flagged.data_flags == ("synthetic flag",)
    # Full-history window is FY2021-25 -> clean.
    clean = mechanical_params(vintage_periods(company, date(2030, 1, 1)))
    assert clean.data_flags == ()


# ---------------------------------------------------------------------------
# project
# ---------------------------------------------------------------------------

def test_project_p50_close_to_deterministic_path():
    params = mechanical_params(vintage_periods(_make_company(8), date(2030, 1, 1)))
    proj = project(params, horizon=3, n=20_000, seed=7)
    for t in range(1, 4):
        det_rev = params.base_revenue * (1 + GROWTH) ** t
        assert proj.rev_p50[t - 1] == pytest.approx(det_rev, rel=0.01)
        assert proj.fcf_p50[t - 1] == pytest.approx(det_rev * MARGIN, rel=0.01)


def test_project_percentile_ordering_and_targets():
    params = mechanical_params(vintage_periods(_make_company(8), date(2030, 1, 1)))
    proj = project(params, horizon=3, n=5_000, seed=7)
    assert proj.target_fiscal_years == [2026, 2027, 2028]
    for t in range(3):
        assert proj.rev_p10[t] < proj.rev_p50[t] < proj.rev_p90[t]
        assert proj.fcf_p10[t] < proj.fcf_p50[t] < proj.fcf_p90[t]


def test_project_seed_reproducible():
    params = mechanical_params(vintage_periods(_make_company(8), date(2030, 1, 1)))
    a = project(params, horizon=3, n=2_000, seed=7)
    b = project(params, horizon=3, n=2_000, seed=7)
    assert a.rev_p50 == b.rev_p50
    assert a.fcf_p10 == b.fcf_p10


# ---------------------------------------------------------------------------
# run_backtest — realized join, errors, bands, skips
# ---------------------------------------------------------------------------

def _row(result: BacktestResult, vintage: date, target_fy: int, metric: str) -> dict:
    rows = [r for r in result.table()
            if r["vintage"] == vintage and r["target_fy"] == target_fy
            and r["metric"] == metric]
    assert len(rows) == 1
    return rows[0]


def test_run_backtest_realized_join_and_error_hand_verified():
    result = run_backtest(_make_company(8), horizon=3, n=20_000, seed=7)
    # Vintage at FY2020-end + 90d sees FY2018-20; base FY2020, target FY2021.
    row = _row(result, _as_of(2020), 2021, "revenue")
    assert row["base_fy"] == 2020
    expected_actual = BASE_REVENUE * (1 + GROWTH) ** 3
    assert row["actual"] == pytest.approx(expected_actual)
    # error_pct definition: (actual / p50 - 1) * 100, hand-recomputed.
    assert row["error_pct"] == pytest.approx(
        (row["actual"] / row["p50"] - 1.0) * 100.0)
    # Actual follows the exact deterministic path the params encode.
    assert abs(row["error_pct"]) < 2.0
    assert row["in_band"] is True

    fcf_row = _row(result, _as_of(2020), 2021, "fcf")
    assert fcf_row["actual"] == pytest.approx(expected_actual * MARGIN)
    assert fcf_row["in_band"] is True


def test_run_backtest_unrealized_rows_are_none():
    result = run_backtest(_make_company(8), horizon=3, n=2_000, seed=7)
    # Last vintage: base FY2025, targets FY2026-28 — nothing reported yet.
    for target_fy in (2026, 2027, 2028):
        for metric in ("revenue", "fcf"):
            row = _row(result, _as_of(2025), target_fy, metric)
            assert row["actual"] is None
            assert row["error_pct"] is None
            assert row["in_band"] is None


def test_run_backtest_in_band_false_when_actual_breaks_out():
    company = _make_company(8)
    # FY2024 actual revenue jumps 60% above trend — far outside a band built
    # from an exact-5%-growth history with floor-level stds.
    shock = next(p for p in company.periods if p.period_end.year == 2024)
    shock.revenue = shock.revenue * 1.6
    result = run_backtest(company, horizon=3, n=5_000, seed=7)
    row = _row(result, _as_of(2021), 2024, "revenue")  # clean pre-shock vintage
    assert row["in_band"] is False
    assert row["error_pct"] > 0


def test_run_backtest_skips_thin_history_vintages_with_reason():
    result = run_backtest(_make_company(8), horizon=3, n=1_000, seed=7)
    # FY2018 and FY2019 vintage dates see only 1-2 filed periods.
    assert [as_of for as_of, _ in result.skipped] == [_as_of(2018), _as_of(2019)]
    assert all("usable annual period" in reason for _, reason in result.skipped)
    assert [v.params.as_of for v in result.vintages] \
        == [_as_of(y) for y in range(2020, 2026)]


def test_run_backtest_coverage_and_bias_on_clean_company():
    result = run_backtest(_make_company(8), horizon=3, n=20_000, seed=7)
    inside, total = result.coverage()
    # Deterministic history projected onto its own continuation: everything
    # realized lands in-band.
    assert total > 0
    assert inside == total
    bias = result.bias_by_horizon()
    assert set(bias.keys()) == {1, 2, 3}
    for h in bias:
        for metric in ("revenue", "fcf"):
            mean_err, n = bias[h][metric]
            assert n >= 1
            assert abs(mean_err) < 2.0


def test_run_backtest_table_row_shape():
    result = run_backtest(_make_company(8), horizon=2, n=1_000, seed=7)
    rows = result.table()
    # 6 vintages x 2 horizon years x 2 metrics
    assert len(rows) == 24
    assert set(rows[0].keys()) == {
        "vintage", "base_fy", "target_fy", "metric",
        "p10", "p50", "p90", "actual", "error_pct", "in_band", "data_flags",
    }


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def test_cli_prints_tables_coverage_and_bias(monkeypatch, capsys):
    import scripts.backtest as cli

    monkeypatch.setattr(cli.store, "load", lambda t: _make_company(8, ticker=t))
    monkeypatch.setattr(sys, "argv",
                        ["backtest", "SYN", "--horizon", "3",
                         "--simulations", "2000"])
    cli.main()
    out = capsys.readouterr().out
    assert "REVENUE ($B)" in out
    assert "FREE CASH FLOW ($B)" in out
    assert "Coverage:" in out
    assert "Mean signed error" in out
    assert "Skipped vintages" in out


def test_cli_graceful_on_thin_history(monkeypatch, capsys):
    import scripts.backtest as cli

    monkeypatch.setattr(cli.store, "load", lambda t: _make_company(2, ticker=t))
    monkeypatch.setattr(sys, "argv", ["backtest", "SYN"])
    with pytest.raises(SystemExit) as exc:
        cli.main()
    assert exc.value.code == 1
    assert "no usable vintages" in capsys.readouterr().out


def test_cli_graceful_on_missing_data(monkeypatch, capsys):
    import scripts.backtest as cli

    def _raise(t):
        raise FileNotFoundError(t)

    monkeypatch.setattr(cli.store, "load", _raise)
    monkeypatch.setattr(sys, "argv", ["backtest", "NOPE"])
    with pytest.raises(SystemExit) as exc:
        cli.main()
    assert exc.value.code == 1
    assert "No saved data for NOPE" in capsys.readouterr().out
