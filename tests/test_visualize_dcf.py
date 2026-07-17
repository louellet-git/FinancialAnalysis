import html as html_mod
import json
import re
from datetime import date, timedelta
from unittest import mock

import numpy as np
import pytest

from financial_analysis.driver_dcf import Driver, DriverModel, SegmentSpec
from financial_analysis.options import save_snapshot
from financial_analysis.schema import (
    CompanyFinancials,
    OptionChainSnapshot,
    OptionQuote,
    PeriodFinancials,
)
from scripts.visualize_dcf import (
    PANEL_TITLES,
    SECTION_TITLES,
    _load_driver_build,
    _stratified_path_sample,
    write_report,
)

_SNAPSHOT_DATE = date(2026, 7, 10)
_EARNINGS_DATE = date(2026, 7, 22)
_SPOT = 150.0


# ---------------------------------------------------------------------------
# Synthetic company (no network anywhere in this module)
# ---------------------------------------------------------------------------

def _annual_period(year: int, revenue: float, ocf: float, capex: float,
                   **kwargs) -> PeriodFinancials:
    return PeriodFinancials(
        ticker="ACME", period_end=date(year, 12, 31),
        form="10-K", filed_date=date(year + 1, 2, 15),
        revenue=revenue, operating_cash_flow=ocf, capex=capex, **kwargs,
    )


def _make_company() -> CompanyFinancials:
    periods = [
        _annual_period(2021 + i, revenue=50e9 * (1.05 ** i),
                       ocf=12e9 * (1.05 ** i), capex=-2e9,
                       net_income=8e9, sbc_total=0.5e9,
                       acquisitions_net_of_cash=-1e9)
        for i in range(4)
    ]
    periods.append(_annual_period(
        2025, revenue=50e9 * (1.05 ** 4), ocf=12e9 * (1.05 ** 4), capex=-2e9,
        net_income=9e9, sbc_total=0.6e9, acquisitions_net_of_cash=-2e9,
        long_term_debt=20e9, cash_and_equivalents=5e9,
        raw_extras={"us-gaap/ShortTermBorrowings": 1e9},
        shares_outstanding=1e9,
    ))
    return CompanyFinancials(ticker="ACME", cik="0", company_name="Acme Corp",
                             periods=periods)


# ---------------------------------------------------------------------------
# Synthetic cache: prices, earnings dates, option-chain snapshot
# ---------------------------------------------------------------------------

def _write_prices(cache_dir, n_days: int = 300) -> None:
    start = _SNAPSHOT_DATE - timedelta(days=n_days)
    records = []
    for i in range(n_days):
        d = start + timedelta(days=i)
        close = 140.0 + 0.05 * i
        records.append({"date": d.isoformat(), "open": close - 1, "high": close + 1,
                        "low": close - 2, "close": close, "volume": 1_000_000})
    path = cache_dir / "ACME" / "prices.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(records), encoding="utf-8")


def _write_earnings(cache_dir) -> None:
    records = [
        {"date": _EARNINGS_DATE.isoformat(), "actual_eps": None,
         "estimated_eps": 2.5, "surprise": None, "surprise_pct": None},
        {"date": "2026-04-22", "actual_eps": 2.4, "estimated_eps": 2.3,
         "surprise": 0.1, "surprise_pct": 0.043},
    ]
    path = cache_dir / "ACME" / "yf_earnings.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(records), encoding="utf-8")


def _quote(expiry: date, strike: float, opt: str, iv: float,
           mid: float) -> OptionQuote:
    return OptionQuote(
        contract_symbol=f"ACME{expiry:%y%m%d}{'C' if opt == 'call' else 'P'}{int(strike):08d}",
        option_type=opt, expiry=expiry, strike=strike,
        last_price=mid, bid=mid - 0.1, ask=mid + 0.1,
        volume=100, open_interest=250, implied_volatility=iv,
        in_the_money=(strike < _SPOT) if opt == "call" else (strike > _SPOT),
    )


def _write_options(cache_dir) -> None:
    # Four expiries; the first post-earnings expiry (07-24) carries an IV hump.
    expiries = {
        date(2026, 7, 17): 0.24,
        date(2026, 7, 24): 0.38,   # earnings hump
        date(2026, 8, 21): 0.28,
        date(2026, 9, 18): 0.26,
    }
    quotes = []
    for expiry, atm_iv in expiries.items():
        for strike in range(110, 195, 5):
            s = float(strike)
            # Smile: higher IV away from spot, put wing steeper than call wing.
            dist = abs(s - _SPOT) / _SPOT
            for opt in ("call", "put"):
                wing = 0.30 if (opt == "put" and s < _SPOT) else 0.10
                iv = atm_iv + wing * dist
                intrinsic = max(0.0, (_SPOT - s) if opt == "call" else (s - _SPOT))
                mid = round(intrinsic + 40 * atm_iv * max(0.05, 1 - dist), 2)
                quotes.append(_quote(expiry, s, opt, round(iv, 4), mid))
    snapshot = OptionChainSnapshot(ticker="ACME", snapshot_date=_SNAPSHOT_DATE,
                                   spot_price=_SPOT, quotes=quotes)
    save_snapshot(snapshot, cache_dir)


# ---------------------------------------------------------------------------
# Synthetic driver model (matches the analysis/<T>/driver_model.py contract)
# ---------------------------------------------------------------------------

def _driver_build() -> dict:
    drivers = [
        Driver("g_soft", True, 0.06, 0.02, rationale="Software CAGR FY21-25"),
        Driver("g_infra", True, 0.01, 0.03, rationale="Infrastructure cycle"),
        Driver("core_margin", True, 0.20, 0.02, dist="truncnorm",
               lo=0.05, hi=0.35, rationale="Core FCFF margin build"),
        Driver("terminal_growth", True, 0.02, 0.005, rationale="Nominal GDP share"),
        Driver("sbc_ratio", False, 0.02, rationale="Held at FY25 level"),
        Driver("ma_ratio", False, 0.05, rationale="5y average outlay"),
        Driver("ma_recurrence", False, 0.5, rationale="Half of M&A recurring"),
        Driver("rf", False, 0.042, rationale="10y treasury"),
        Driver("beta", False, 0.9, rationale="5y weekly beta"),
        Driver("erp", False, 0.05, rationale="Consensus ERP"),
        Driver("kd_spread", False, -0.001, rationale="Fair-value spread over rf"),
        Driver("tax_rate", False, 0.21, rationale="Statutory shield"),
        Driver("margin_mix_drift", False, 0.0, rationale="No mix drift in fixture"),
        Driver("equity_weight", False, 0.85, rationale="Market-value weights"),
    ]
    corr = {("g_soft", "g_infra"): 0.3, ("g_soft", "core_margin"): 0.2}
    model = DriverModel(drivers, corr)
    segments = [SegmentSpec("Software", 30e9, "g_soft"),
                SegmentSpec("Infrastructure", 20e9, "g_infra")]
    return {
        "model": model,
        "segments": segments,
        "net_debt": 16e9,
        "shares": 1e9,
        "segment_history": {
            "Software": {2023: 26e9, 2024: 28e9, 2025: 30e9},
            "Infrastructure": {2023: 21e9, 2024: 20.5e9, 2025: 20e9},
        },
    }


# ---------------------------------------------------------------------------
# Fixtures: one report with every section populated, one with all
# conditional/data-backed sections absent.
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def full_cache(tmp_path_factory):
    cache = tmp_path_factory.mktemp("cache")
    _write_prices(cache)
    _write_earnings(cache)
    _write_options(cache)
    return cache


@pytest.fixture(scope="module")
def report_html(tmp_path_factory, full_cache) -> str:
    out = tmp_path_factory.mktemp("reports") / "ACME_dcf.html"
    with mock.patch("scripts.visualize_dcf._load_driver_build",
                    return_value=_driver_build):
        path = write_report(_make_company(), out, price=150.0,
                            price_note="price $150.00 (test)",
                            cache_dir=full_cache)
    assert path.exists()
    return path.read_text(encoding="utf-8")


@pytest.fixture(scope="module")
def bare_report_html(tmp_path_factory) -> str:
    """No price cache, no options snapshot, no driver model, no price."""
    out = tmp_path_factory.mktemp("reports_bare") / "ACME_dcf.html"
    empty_cache = tmp_path_factory.mktemp("cache_bare")
    note = "current price unavailable (offline?) — price marker omitted"
    with mock.patch("scripts.visualize_dcf._fetch_price_history", return_value=[]):
        path = write_report(_make_company(), out, price=None, price_note=note,
                            cache_dir=empty_cache)
    return path.read_text(encoding="utf-8")


# ---------------------------------------------------------------------------
# Report-wide invariants
# ---------------------------------------------------------------------------

def test_report_contains_plotly(report_html):
    assert "Plotly.newPlot" in report_html
    assert 'class="plotly-graph-div"' in report_html


def test_report_contains_all_panel_titles(report_html):
    for title in PANEL_TITLES:
        assert title in report_html


def test_report_contains_all_section_titles(report_html):
    for title in SECTION_TITLES:
        assert html_mod.escape(title) in report_html


def test_bare_report_still_has_all_section_titles(bare_report_html):
    # Sections degrade to labeled notes — they never disappear or crash.
    for title in SECTION_TITLES:
        assert html_mod.escape(title) in bare_report_html


def test_report_is_self_contained(report_html):
    # plotly.js is inlined; the markup loads nothing over the network
    # (the inlined bundle itself contains URL string literals — those never load
    # for the trace types used here, so only actual load-bearing tags are checked)
    assert "<script src=" not in report_html
    assert "<link " not in report_html
    assert '<img src="http' not in report_html
    assert len(report_html) > 1_000_000  # the ~4MB bundle is really embedded


def test_page_bg_token_is_blueprint_ground(report_html):
    # Ratings-desk blueprint: light technical ground, not near-white.
    assert "--bg:#f4f4f5" in report_html
    assert "background:var(--bg)" in report_html


def test_cards_have_zero_border_radius(report_html):
    # Cards are square-cornered line drawings (radius 0 everywhere).
    card_rule = re.search(r"section\.card\{[^}]*\}", report_html)
    assert card_rule is not None
    assert "border-radius:0" in card_rule.group(0)
    assert "border-radius: 8px" not in report_html
    assert "border-radius:8px" not in report_html


def test_report_shows_price_marker(report_html):
    assert "Price $150.00" in report_html


def test_report_without_price_omits_marker(bare_report_html):
    assert "Price $" not in bare_report_html
    assert "price marker omitted" in bare_report_html


# ---------------------------------------------------------------------------
# Header band
# ---------------------------------------------------------------------------

def test_header_stat_chips(report_html):
    assert "Company report" in report_html
    assert "Last price" in report_html
    assert "Market cap" in report_html
    for key in ("Bear P50", "Base P50", "Bull P50"):
        assert key in report_html
    assert "Base P50 vs price" in report_html


def test_header_without_price_omits_delta_chip(bare_report_html):
    assert "Base P50 vs price" not in bare_report_html
    assert "Market cap" not in bare_report_html


# ---------------------------------------------------------------------------
# Price & valuation
# ---------------------------------------------------------------------------

def test_price_section_present(report_html):
    assert "Price history" in report_html
    assert "Price vs valuation" in report_html
    # Valuation band annotation lives inside plotly JSON, where the en dash is
    # serialized as – — assert on the ASCII prefix.
    assert "Base P10" in report_html
    assert "Log" in report_html           # scale toggle button


def test_price_section_absent(bare_report_html):
    assert "Price history not available" in bare_report_html
    assert "Price vs valuation" not in bare_report_html


def test_price_charts_have_range_buttons(report_html):
    # Both price figures carry a rangeselector; button labels are serialized
    # into the figures' plotly JSON. (The plotly.js bundle itself also
    # mentions "rangeselector", so assert on the labels, not the key.)
    for label in ("1M", "3M", "6M", "1Y", "5Y", "2Y", "Max"):
        assert f'"{label}"' in report_html


# ---------------------------------------------------------------------------
# Fundamentals
# ---------------------------------------------------------------------------

def test_fundamentals_present(report_html):
    assert "Revenue, free cash flow &amp; margin" in report_html
    assert "Annual fundamentals" in report_html
    for col in ("Net income", "OCF", "Capex", "SBC", "Acquisitions", "FCF margin"):
        assert col in report_html


def test_fundamentals_absent_without_revenue():
    company = _make_company()
    for p in company.periods:
        p.revenue = None
    # No revenue also breaks the DCF engines themselves — fundamentals degrade
    # before that, so exercise the section builder directly.
    from scripts.visualize_dcf import _fundamentals_section
    section = _fundamentals_section(company)
    assert "Fundamentals not available" in section


# ---------------------------------------------------------------------------
# Driver-based model (conditional)
# ---------------------------------------------------------------------------

def test_driver_section_present(report_html):
    assert "Segment revenue history" in report_html
    assert "Driver assumptions" in report_html
    assert "Driver correlation matrix" in report_html
    assert "Driver model vs aggregate base" in report_html
    assert "Driver attribution tornado" in report_html
    # Table content: badges, a driver name, a rationale.
    assert "stochastic" in report_html
    assert "deterministic" in report_html
    assert "g_soft" in report_html
    assert "Software CAGR FY21-25" in report_html
    # Engine footer line.
    assert "DriverDCF: 10,000 simulations · seed 7" in report_html


def test_driver_section_absent(bare_report_html):
    assert "Driver model not available" in bare_report_html
    assert "Driver attribution tornado" not in bare_report_html


def test_driver_section_bad_contract_degrades(tmp_path, full_cache):
    out = tmp_path / "ACME_bad_driver.html"
    with mock.patch("scripts.visualize_dcf._load_driver_build",
                    return_value=lambda: {"model": None}):  # missing keys
        html = write_report(_make_company(), out, price=150.0,
                            price_note="price $150.00 (test)",
                            cache_dir=full_cache).read_text(encoding="utf-8")
    assert "Driver model could not be loaded" in html
    assert "Driver attribution tornado" not in html


def test_load_driver_build_returns_none_for_unknown_ticker():
    assert _load_driver_build("NOSUCHTICKER") is None


# ---------------------------------------------------------------------------
# Path explorer (inside the driver section)
# ---------------------------------------------------------------------------

def test_path_explorer_panels_present(report_html):
    assert "Path explorer — FCFF path spaghetti" in report_html
    assert "Path explorer — parallel coordinates" in report_html
    assert "Path explorer — outlier paths" in report_html


def test_path_explorer_spaghetti_content(report_html):
    # Sample-size disclosure in the card caption, and the direct-labeled
    # median path.
    assert re.search(r"of 10,000 paths shown \(all beyond P1/P99 \+ stratified",
                     report_html)
    assert "Median path" in report_html
    # Grouped traces carry per-path hover data in customdata.
    assert "customdata" in report_html
    assert "Bottom 1%" in report_html
    assert "Top 1%" in report_html


def test_path_explorer_parcoords_content(report_html):
    # The parcoords trace ends on the per-share value dimension.
    assert "parcoords" in report_html.lower()
    assert "$/sh" in report_html


def test_path_explorer_outlier_table_content(report_html):
    assert "Bottom 10 by per-share value" in report_html
    assert "Top 10 by per-share value" in report_html
    # z-scores are printed in parentheses; extremes are bolded.
    assert re.search(r"<strong>[^<]*\([+-]\d+\.\d\)</strong>", report_html)


def test_path_explorer_absent_without_driver_model(bare_report_html):
    assert "Path explorer" not in bare_report_html


# ---------------------------------------------------------------------------
# Stratified path sampler
# ---------------------------------------------------------------------------

def _sample_paths(n: int = 5000) -> np.ndarray:
    return np.random.default_rng(0).normal(100.0, 20.0, n)


def test_sampler_includes_every_tail_path():
    ps = _sample_paths()
    bottom, middle, top = _stratified_path_sample(ps, 300)
    p1, p99 = np.percentile(ps, [1, 99])
    assert set(bottom.tolist()) == set(np.flatnonzero(ps < p1).tolist())
    assert set(top.tolist()) == set(np.flatnonzero(ps > p99).tolist())


def test_sampler_reaches_target_count():
    ps = _sample_paths()
    bottom, middle, top = _stratified_path_sample(ps, 300)
    assert bottom.size + middle.size + top.size == 300


def test_sampler_middle_stays_inside_the_band():
    ps = _sample_paths()
    _, middle, _ = _stratified_path_sample(ps, 300)
    p1, p99 = np.percentile(ps, [1, 99])
    assert (ps[middle] >= p1).all()
    assert (ps[middle] <= p99).all()


def test_sampler_is_deterministic_under_seed():
    ps = _sample_paths()
    a = _stratified_path_sample(ps, 300, seed=7)
    b = _stratified_path_sample(ps, 300, seed=7)
    for x, y in zip(a, b):
        assert np.array_equal(x, y)
    c = _stratified_path_sample(ps, 300, seed=8)
    assert not np.array_equal(a[1], c[1])


def test_sampler_target_below_tail_count_keeps_all_tails():
    ps = _sample_paths()
    bottom, middle, top = _stratified_path_sample(ps, 10)
    p1, p99 = np.percentile(ps, [1, 99])
    assert middle.size == 0
    assert bottom.size == np.count_nonzero(ps < p1)
    assert top.size == np.count_nonzero(ps > p99)


# ---------------------------------------------------------------------------
# Backtest — mechanical vintages
# ---------------------------------------------------------------------------

def test_backtest_section_present(report_html):
    assert "Backtest — mechanical vintages" in report_html
    assert "Revenue hedgehog" in report_html
    assert "FCF hedgehog" in report_html
    assert "Vintage scorecard" in report_html
    assert "P10–P90 coverage" in report_html


def test_backtest_skipped_vintages_disclosed(report_html):
    # ACME's first two fiscal-year-end vintage dates lack the 3-period
    # minimum — they must be listed, not silently dropped.
    assert "Skipped vintage dates (insufficient history)" in report_html


def test_backtest_scorecard_scores_realized_years(report_html):
    # The FY2023 vintage is scored against realized FY2024/FY2025 (in-band
    # symbols) while unrealized target years carry the dash.
    assert "✓" in report_html or "✗" in report_html
    assert "filed_date visibility" in report_html   # honesty note


def test_backtest_no_flags_for_unflagged_ticker(report_html):
    # ACME has no DATA_FLAGS entry — the flag note must not render.
    assert "data flags surfaced" not in report_html


def test_backtest_section_degrades_without_history():
    from scripts.visualize_dcf import _backtest_section
    company = _make_company()
    company.periods = company.periods[:2]   # < 3 clean annuals at every vintage
    section = _backtest_section(company)
    assert "No vintage date has the 3 clean annual periods" in section


# ---------------------------------------------------------------------------
# Options market (conditional)
# ---------------------------------------------------------------------------

def test_options_section_present(report_html):
    assert "ATM implied-volatility term structure" in report_html
    assert "Implied move by expiry" in report_html
    assert "Volatility skew by expiry" in report_html
    assert "Open interest by strike" in report_html
    assert "Chain detail — ATM ±10 strikes" in report_html
    assert "P/C volume" in report_html
    assert "P/C open interest" in report_html


def test_options_earnings_annotations(report_html):
    assert f"Earnings {_EARNINGS_DATE.isoformat()}" in report_html
    assert "Earnings hump" in report_html
    # Chain detail targets the first post-earnings expiry.
    assert "Expiry 2026-07-24" in report_html
    assert "first expiry after earnings" in report_html


def test_options_section_absent(bare_report_html):
    assert "No option-chain snapshot available" in bare_report_html
    assert "ATM implied-volatility term structure" not in bare_report_html


# ---------------------------------------------------------------------------
# Footer
# ---------------------------------------------------------------------------

def test_footer_provenance(report_html):
    assert "SEC EDGAR" in report_html
    assert "Generated by scripts/visualize_dcf.py" in report_html
    assert f"snapshot {_SNAPSHOT_DATE.isoformat()}" in report_html


def test_footer_reports_missing_options(bare_report_html):
    assert "no snapshot on disk" in bare_report_html


# ---------------------------------------------------------------------------
# Skew near-expiry exclusion
# ---------------------------------------------------------------------------

def _skew_metrics(*expiries):
    from financial_analysis.options import ChainMetrics, ExpiryMetrics
    return ChainMetrics(
        ticker="ACME", snapshot_date=_SNAPSHOT_DATE, spot_price=_SPOT,
        expiries=[ExpiryMetrics(expiry=e, skew=s) for e, s in expiries])


def test_skew_figure_drops_near_expiry_wings():
    from scripts.visualize_dcf import _skew_figure
    # 0-1 DTE wing IVs are solver noise (observed -106 pts on a dying expiry);
    # only the 7-days-out expiry may survive.
    fig = _skew_figure(_skew_metrics(
        (_SNAPSHOT_DATE, -1.06),
        (_SNAPSHOT_DATE + timedelta(days=1), 0.55),
        (_SNAPSHOT_DATE + timedelta(days=7), 0.07),
    ))
    assert list(fig.data[0].x) == ["17 Jul 26"]
    assert fig.data[0].y[0] == pytest.approx(7.0)


def test_skew_figure_none_when_only_near_expiry():
    from scripts.visualize_dcf import _skew_figure
    assert _skew_figure(_skew_metrics((_SNAPSHOT_DATE, -1.06))) is None
