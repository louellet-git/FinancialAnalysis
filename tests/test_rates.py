"""Offline tests for financial_analysis.rates. No network: _http_get is patched."""
import json
from datetime import date
from unittest.mock import patch

import pytest

from financial_analysis import rates

_NOMINAL_2026 = (
    'Date,"1 Mo","1.5 Month","2 Mo","3 Mo","4 Mo","6 Mo","1 Yr","2 Yr","3 Yr","5 Yr",'
    '"7 Yr","10 Yr","20 Yr","30 Yr"\n'
    "01/05/2026,3.70,3.70,3.66,3.65,3.62,3.58,3.47,3.47,3.55,3.74,3.95,4.17,4.80,4.85\n"
    "01/02/2026,3.72,3.71,3.66,3.65,3.62,3.58,3.47,3.47,3.55,3.74,3.95,4.19,4.81,4.86\n"
)
_NOMINAL_2025 = (
    'Date,"1 Mo","1.5 Month","2 Mo","3 Mo","4 Mo","6 Mo","1 Yr","2 Yr","3 Yr","5 Yr",'
    '"7 Yr","10 Yr","20 Yr","30 Yr"\n'
    "12/31/2025,3.74,3.72,3.68,3.67,3.63,3.59,3.48,3.48,3.56,3.73,3.94,4.18,4.79,4.84\n"
)
# 2004 era: no 1.5M/2M/4M columns and no 30Y (not issued 2002-2006).
_NOMINAL_2004 = (
    'Date,"1 Mo","3 Mo","6 Mo","1 Yr","2 Yr","3 Yr","5 Yr","7 Yr","10 Yr","20 Yr"\n'
    "12/31/2004,1.89,2.22,2.59,2.75,3.08,3.25,3.63,3.94,4.24,4.85\n"
)
_REAL = 'Date,"5 YR","7 YR","10 YR","20 YR","30 YR"\n11/18/2026,2.66,2.76,2.88,3.12,3.25\n'
_NYFED_EFFR = (
    '{ "refRates": [ { "effectiveDate": "2026-11-17", "type": "EFFR", "percentRate": 3.88,'
    ' "targetRateFrom": 3.75, "targetRateTo": 4.00 },'
    ' { "effectiveDate": "2026-11-16", "type": "EFFR", "percentRate": 3.87,'
    ' "targetRateFrom": 3.75, "targetRateTo": 4.00 } ] }'
)
_FRED = "observation_date,DGS10\n2026-11-10,4.98\n2026-11-11,\n2026-11-12,5.10\n2026-11-13,.\n"


@pytest.mark.parametrize("header,expected", [
    ("1 Mo", "1M"), ("1.5 Month", "1.5M"), ("4 Mo", "4M"), ("1 Yr", "1Y"),
    ("10 Yr", "10Y"), ("10 YR", "10Y"), (" 30 Yr ", "30Y"),
])
def test_normalize_tenor(header, expected):
    assert rates.normalize_tenor(header) == expected


def test_normalize_tenor_rejects_unknown():
    with pytest.raises(ValueError):
        rates.normalize_tenor("LT COMPOSITE")


def test_parse_treasury_csv_converts_percent_to_decimal():
    curves = rates.parse_treasury_csv(_NOMINAL_2026)
    c = curves[date(2026, 1, 2)]
    assert c["10Y"] == pytest.approx(0.0419)
    assert c["1.5M"] == pytest.approx(0.0371)
    assert list(c) == [t for t in rates.TENORS if t in c]  # curve order preserved


def test_parse_treasury_csv_handles_missing_tenors():
    c = rates.parse_treasury_csv(_NOMINAL_2004)[date(2004, 12, 31)]
    assert "30Y" not in c and "2M" not in c
    assert c["20Y"] == pytest.approx(0.0485)


def test_parse_treasury_csv_real_curve_and_bom():
    c = rates.parse_treasury_csv("﻿" + _REAL)[date(2026, 11, 18)]
    assert c == {"5Y": 0.0266, "7Y": 0.0276, "10Y": 0.0288, "20Y": 0.0312, "30Y": 0.0325}


def _fake_treasury(url: str) -> str:
    if "/2026/" in url:
        return _NOMINAL_2026
    if "/2025/" in url:
        return _NOMINAL_2025
    raise AssertionError(f"unexpected url {url}")


def test_treasury_yield_on_or_before_and_year_rollover(tmp_path):
    with patch.object(rates, "_http_get", side_effect=_fake_treasury):
        # Saturday 2026-01-03 resolves to Friday 2026-01-02.
        obs = rates.treasury_yield("10Y", on=date(2026, 1, 3), cache_dir=tmp_path)
        assert (obs.date, obs.value) == (date(2026, 1, 2), 0.0419)
        # New Year's Day has no 2026 row on or before it, so it falls back to 2025-12-31.
        obs = rates.treasury_yield("10Y", on=date(2026, 1, 1), cache_dir=tmp_path)
        assert (obs.date, obs.value) == (date(2025, 12, 31), 0.0418)
    assert (tmp_path / "rates" / "treasury_nominal_2025.csv").exists()


def test_past_year_served_from_cache(tmp_path):
    with patch.object(rates, "_http_get", side_effect=_fake_treasury) as get:
        rates.load_treasury_curve(2025, cache_dir=tmp_path)
        rates.load_treasury_curve(2025, cache_dir=tmp_path)
    assert get.call_count == 1


def test_fetch_failure_falls_back_to_cache(tmp_path):
    with patch.object(rates, "_http_get", side_effect=_fake_treasury):
        rates.load_treasury_curve(2026, cache_dir=tmp_path)
    with patch.object(rates, "_http_get", side_effect=RuntimeError("down")):
        curves = rates.load_treasury_curve(2026, cache_dir=tmp_path, refresh=True)
    assert date(2026, 1, 5) in curves


def test_bad_payload_is_not_cached(tmp_path):
    with patch.object(rates, "_http_get", return_value="<html>maintenance</html>"):
        with pytest.raises(ValueError):
            rates.load_treasury_curve(2025, cache_dir=tmp_path)
    assert not (tmp_path / "rates" / "treasury_nominal_2025.csv").exists()


def test_missing_tenor_raises_with_available_list(tmp_path):
    with patch.object(rates, "_http_get", return_value=_NOMINAL_2004):
        with pytest.raises(KeyError, match="30Y"):
            rates.treasury_yield("30Y", on=date(2004, 12, 31), cache_dir=tmp_path)


def test_parse_nyfed_effr_with_target_range():
    rows = rates.parse_nyfed_json(_NYFED_EFFR)
    assert [r["date"] for r in rows] == [date(2026, 11, 16), date(2026, 11, 17)]
    assert rows[-1]["value"] == 0.0388
    assert (rows[-1]["target_from"], rows[-1]["target_to"]) == (0.0375, 0.04)


def test_nyfed_rate_latest(tmp_path):
    with patch.object(rates, "_http_get", return_value=_NYFED_EFFR):
        obs = rates.nyfed_rate("effr", on=date(2026, 11, 18))
    assert (obs.series, obs.date, obs.value) == ("EFFR", date(2026, 11, 17), 0.0388)


def test_nyfed_rejects_unknown_rate():
    with pytest.raises(ValueError):
        rates.nyfed_history("LIBOR")


def test_parse_fred_skips_blanks_and_dots():
    s = rates.parse_fred_csv(_FRED)
    assert s == {date(2026, 11, 10): 0.0498, date(2026, 11, 12): 0.051}


# --- inflation ---------------------------------------------------------------

_NOM_DAY = ('Date,"5 Yr","7 Yr","10 Yr","20 Yr","30 Yr"\n'
            "11/18/2026,4.89,4.96,5.07,5.44,5.41\n")
_REAL_DAY = 'Date,"5 YR","7 YR","10 YR","20 YR","30 YR"\n11/18/2026,2.66,2.76,2.88,3.12,3.25\n'


def _fake_curves(url: str) -> str:
    return _REAL_DAY if "real_yield" in url else _NOM_DAY


def test_breakeven_and_5y5y_match_fred_formula(tmp_path):
    # FRED on 2026-11-18: T10YIE 2.19, T5YIFR 2.15.
    with patch.object(rates, "_http_get", side_effect=_fake_curves):
        be = rates.breakeven_inflation("10Y", on=date(2026, 11, 18), cache_dir=tmp_path)
        fwd = rates.forward_inflation_5y5y(on=date(2026, 11, 18), cache_dir=tmp_path)
    assert (be.date, be.value) == (date(2026, 11, 18), pytest.approx(0.0219))
    assert round(fwd.value * 100, 2) == 2.15


def _bls(rows: list[tuple[str, str, str]], status: str = "REQUEST_SUCCEEDED") -> str:
    data = [{"year": y, "period": p, "value": v} for y, p, v in rows]
    return json.dumps({"status": status, "message": [],
                       "Results": {"series": [{"seriesID": "CUUR0000SA0", "data": data},
                                              {"seriesID": "CUUR0000SA0L1E", "data": data}]}})


_BLS_GAP = _bls([("2026", "M10", "337.511"), ("2026", "M09", "336.940"), ("2025", "M13", "321.943"),
                 ("2025", "M10", "-"), ("2025", "M09", "324.800")])


def test_parse_bls_skips_annual_average_and_missing():
    s = rates.parse_bls_json(_BLS_GAP)["CUUR0000SA0"]
    assert list(s) == [date(2025, 9, 1), date(2026, 9, 1), date(2026, 10, 1)]


def test_parse_bls_rejects_failed_request():
    with pytest.raises(ValueError, match="BLS request failed"):
        rates.parse_bls_json(_bls([], status="REQUEST_NOT_PROCESSED"))


def test_cpi_inflation_steps_back_over_gap_and_says_so(tmp_path):
    with patch.object(rates, "_http_post", return_value=_BLS_GAP):
        obs = rates.cpi_inflation("headline", on=date(2026, 11, 18), cache_dir=tmp_path)
    assert obs.date == date(2026, 9, 1)
    assert obs.value == pytest.approx(336.940 / 324.800 - 1, abs=1e-6)
    assert "Oct 2025" in obs.note


def test_parse_fred_raw_levels():
    s = rates.parse_fred_csv("observation_date,CPIAUCNS\n2026-10-01,337.511\n", percent=False)
    assert s == {date(2026, 10, 1): 337.511}


# --- DCF hook and CLI --------------------------------------------------------

def test_with_risk_free_replaces_only_rf_mean():
    from financial_analysis.driver_dcf import Driver, DriverModel
    model = DriverModel(drivers=[
        Driver("rf", stochastic=True, mean=0.046, std=0.005, dist="truncnorm", lo=0.01, rationale="old"),
        Driver("erp", stochastic=True, mean=0.045, std=0.0075),
    ], corr={("rf", "erp"): -0.3})
    obs = rates.RateObservation("UST 10Y", date(2026, 11, 18), 0.0507, "U.S. Treasury")
    new = rates.with_risk_free(model, obs)
    rf = new.driver("rf")
    assert (rf.mean, rf.std, rf.lo, rf.dist) == (0.0507, 0.005, 0.01, "truncnorm")
    assert "2026-11-18" in rf.rationale and "4.60%" in rf.rationale and "old" in rf.rationale
    assert new.driver("erp") == model.driver("erp") and new.corr == model.corr
    assert model.driver("rf").mean == 0.046  # the original model does not change


def test_cli_yield_json(capsys):
    obs = rates.RateObservation("UST 10Y", date(2026, 1, 2), 0.0419, "U.S. Treasury")
    with patch.object(rates, "treasury_yield", return_value=obs) as ty:
        assert rates.main(["yield", "--tenor", "10Y", "--date", "2026-01-03", "--json"]) == 0
    ty.assert_called_once_with("10Y", date(2026, 1, 3), real=False)
    assert json.loads(capsys.readouterr().out) == {
        "series": "UST 10Y", "date": "2026-01-02", "value": 0.0419, "source": "U.S. Treasury", "note": ""}


def test_cli_error_is_json_on_stderr(capsys):
    with patch.object(rates, "nyfed_rate", side_effect=LookupError("no SOFR")):
        assert rates.main(["nyfed", "--json"]) == 2
    err = json.loads(capsys.readouterr().err)
    assert err == {"error": "no SOFR", "type": "LookupError"}
