from datetime import date

import numpy as np
import pytest

from financial_analysis.schema import CompanyFinancials, PeriodFinancials
from financial_analysis.dcf import DCFAssumptions, DCFResult, StochasticDCF


def _annual_period(year: int, revenue: float, ocf: float, capex: float,
                   shares: float | None = None) -> PeriodFinancials:
    return PeriodFinancials(
        ticker="PANW", period_end=date(year, 7, 31),
        form="10-K", filed_date=date(year, 9, 12),
        revenue=revenue, operating_cash_flow=ocf, capex=capex,
        shares_outstanding=shares,
    )


def _make_company(n_periods: int = 5, shares: float | None = None) -> CompanyFinancials:
    base_revenue = 5_000_000_000.0
    base_ocf = 1_500_000_000.0
    periods = [
        _annual_period(
            2019 + i,
            revenue=base_revenue * (1.2 ** i),
            ocf=base_ocf * (1.2 ** i),
            capex=-200_000_000.0,  # importer stores capex sign-flipped (negative)
            shares=shares,
        )
        for i in range(n_periods)
    ]
    return CompanyFinancials(
        ticker="PANW", cik="1327567",
        company_name="Palo Alto Networks Inc",
        periods=periods,
    )


def test_dcf_run_returns_dcf_result():
    result = StochasticDCF(_make_company(), DCFAssumptions(n_simulations=1000)).run()
    assert isinstance(result, DCFResult)


def test_dcf_percentile_ordering():
    result = StochasticDCF(_make_company(), DCFAssumptions(n_simulations=1000)).run()
    assert result.p10_value < result.median_value < result.p90_value


def test_dcf_all_values_positive():
    result = StochasticDCF(_make_company(), DCFAssumptions(n_simulations=1000)).run()
    assert result.p10_value > 0
    assert result.median_value > 0


def test_dcf_simulations_array_shape():
    n = 500
    result = StochasticDCF(_make_company(), DCFAssumptions(n_simulations=n)).run()
    assert result.simulations.shape == (n,)


def test_dcf_uses_explicit_fcf_margin_when_provided():
    assumptions = DCFAssumptions(
        n_simulations=1000, fcf_margin_mean=0.30, fcf_margin_std=0.02,
    )
    result = StochasticDCF(_make_company(), assumptions).run()
    assert result.median_value > 0


def test_dcf_derives_fcf_margin_from_history():
    result = StochasticDCF(_make_company(n_periods=5), DCFAssumptions(n_simulations=1000)).run()
    assert result.median_value > 0


def test_dcf_seed_makes_run_reproducible():
    a = DCFAssumptions(n_simulations=1000, seed=42)
    r1 = StochasticDCF(_make_company(), a).run()
    r2 = StochasticDCF(_make_company(), a).run()
    assert r1.median_value == r2.median_value
    assert np.array_equal(r1.simulations, r2.simulations)


def test_dcf_per_share_value_when_shares_available():
    result = StochasticDCF(
        _make_company(shares=650_000_000.0), DCFAssumptions(n_simulations=1000)
    ).run()
    assert result.median_per_share == pytest.approx(result.median_value / 650_000_000.0)


def test_dcf_per_share_none_when_shares_missing():
    result = StochasticDCF(_make_company(), DCFAssumptions(n_simulations=1000)).run()
    assert result.median_per_share is None


def test_dcf_fewer_than_4_positive_fcf_periods_raises():
    company = _make_company(n_periods=3)
    with pytest.raises(ValueError, match="positive FCF"):
        StochasticDCF(company, DCFAssumptions(n_simulations=100)).run()


def test_dcf_negative_fcf_history_raises():
    # 5 annual periods but FCF is negative in all of them (post-IPO style)
    periods = [
        _annual_period(2021 + i, revenue=1e9 * (1.3 ** i), ocf=-2e8, capex=-3e8)
        for i in range(5)
    ]
    company = CompanyFinancials(ticker="X", cik="0", company_name="X", periods=periods)
    with pytest.raises(ValueError, match="positive FCF"):
        StochasticDCF(company, DCFAssumptions(n_simulations=100)).run()


def test_dcf_ignores_quarterly_periods():
    # 5 good annual periods + quarterly rows must not change eligibility
    company = _make_company(n_periods=5)
    company.periods.append(
        PeriodFinancials(
            ticker="PANW", period_end=date(2024, 4, 30),
            form="10-Q", filed_date=date(2024, 6, 7),
            revenue=2_000_000_000.0, operating_cash_flow=500_000_000.0, capex=-50_000_000.0,
        )
    )
    result = StochasticDCF(company, DCFAssumptions(n_simulations=500)).run()
    assert result.median_value > 0


def test_dcf_quarterly_only_history_raises():
    periods = [
        PeriodFinancials(
            ticker="X", period_end=date(2024, m, 28), form="10-Q", filed_date=date(2024, m, 28),
            revenue=1e9, operating_cash_flow=3e8, capex=-5e7,
        )
        for m in (2, 5, 8, 11)
    ]
    company = CompanyFinancials(ticker="X", cik="0", company_name="X", periods=periods)
    with pytest.raises(ValueError, match="positive FCF"):
        StochasticDCF(company, DCFAssumptions(n_simulations=100)).run()
