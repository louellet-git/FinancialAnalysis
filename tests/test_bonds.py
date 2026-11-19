"""Offline tests for financial_analysis.bonds. No network: rates._http_get is patched."""
import json
from datetime import date
from unittest.mock import patch

import pytest

from financial_analysis import bonds, rates


@pytest.mark.parametrize("rating,bucket", [
    ("AAA", "AAA"), ("AA+", "AA"), ("A-", "A"), ("BBB+", "BBB"), ("BB-", "BB"), ("B", "B"),
    ("CCC+", "CCC"), ("CC", "CCC"), ("C", "CCC"),
    ("Aaa", "AAA"), ("Aa2", "AA"), ("A3", "A"), ("Baa1", "BBB"), ("Ba3", "BB"), ("B2", "B"),
    ("Caa1", "CCC"), (" A- ", "A"),
])
def test_rating_bucket(rating, bucket):
    assert bonds.rating_bucket(rating) == bucket


@pytest.mark.parametrize("bad", ["", "A++", "aaa", "D", "Baa4", "investment grade"])
def test_rating_bucket_rejects(bad):
    with pytest.raises(ValueError):
        bonds.rating_bucket(bad)


def _fred(series_id: str, rows: dict[str, str]) -> str:
    return f"observation_date,{series_id}\n" + "".join(f"{d},{v}\n" for d, v in rows.items())


_SERIES = {
    "BAMLC0A3CAEY": {"2026-11-13": "5.80", "2026-11-16": "5.79", "2026-11-17": "5.77"},
    "BAMLC0A3CA": {"2026-11-16": "0.77", "2026-11-17": "0.76"},
    "DAAA": {"2026-11-16": "6.20", "2026-11-17": "6.18"},
    "DBAA": {"2026-11-16": "6.65", "2026-11-17": "6.63"},
    "AAA10Y": {"2026-11-16": "1.08", "2026-11-17": "1.08"},
    "BAA10Y": {"2026-11-16": "1.53", "2026-11-17": "1.50"},  # 11-17 disagrees on purpose
}
_UST_2026 = ('Date,"10 Yr"\n'
             "11/18/2026,5.07\n11/17/2026,5.10\n11/16/2026,5.12\n11/13/2026,5.15\n")


def _fake_get(url: str) -> str:
    if "daily-treasury-rates" in url:
        return _UST_2026
    sid = url.split("id=")[1].split("&")[0]
    return _fred(sid, _SERIES[sid])


def test_load_fred_caches_and_validates_header(tmp_path):
    with patch.object(rates, "_http_get", side_effect=_fake_get) as get:
        s = bonds.load_fred("DAAA", tmp_path)
        bonds.load_fred("DAAA", tmp_path)
    assert get.call_count == 1
    assert s == {date(2026, 11, 16): 0.062, date(2026, 11, 17): 0.0618}
    with patch.object(rates, "_http_get", return_value="<html>error</html>\n"):
        with pytest.raises(ValueError, match="not a FRED CSV"):
            bonds.load_fred("DBAA", tmp_path)
    assert not (tmp_path / "bonds" / "fred_DBAA.csv").exists()


def test_latest_refuses_stale_values():
    s = {date(2023, 11, 20): 0.01}
    assert bonds._latest(s, date(2023, 11, 24), 7, "x") == (date(2023, 11, 20), 0.01)
    with pytest.raises(LookupError, match="more than 7 days old"):
        bonds._latest(s, date(2023, 12, 1), 7, "x")
    with pytest.raises(LookupError, match="data starts 2023-11-20"):
        bonds._latest(s, date(2020, 3, 23), 7, "x")


def test_spread_over_10y_uses_same_day_treasury(tmp_path):
    with patch.object(rates, "_http_get", side_effect=_fake_get):
        obs = bonds.spread_over_10y("A-", on=date(2026, 11, 18), cache_dir=tmp_path)
    # ICE latest is 11-17 (5.77%); the 10Y of 11-17 is 5.10%, not the 11-18 value 5.07%.
    assert (obs.date, obs.value) == (date(2026, 11, 17), pytest.approx(0.0067))
    assert obs.note == "A- uses the A index"


def test_cross_check_flags_disagreement(tmp_path):
    with patch.object(rates, "_http_get", side_effect=_fake_get), \
         patch.object(bonds, "date") as d:
        d.today.return_value = date(2026, 11, 18)
        d.fromordinal = date.fromordinal
        out = {c["fred_series"]: c for c in bonds.cross_check_bonds(days=10, cache_dir=tmp_path)}
    assert out["AAA10Y"]["compared"] == 2 and out["AAA10Y"]["mismatches"] == []
    assert out["BAA10Y"]["mismatches"] == [("2026-11-17", pytest.approx(0.0153), 0.015)]


def test_with_kd_spread_replaces_only_kd_spread():
    from financial_analysis.driver_dcf import Driver, DriverModel
    model = DriverModel(drivers=[Driver("kd_spread", stochastic=False, mean=-0.001, rationale="ladder"),
                                 Driver("tax_rate", stochastic=False, mean=0.21)])
    obs = rates.RateObservation("A yield minus UST 10Y", date(2026, 11, 17), 0.0067, "ICE BofA via FRED")
    new = bonds.with_kd_spread(model, obs)
    assert new.driver("kd_spread").mean == 0.0067
    assert "-0.10%" in new.driver("kd_spread").rationale and "ladder" in new.driver("kd_spread").rationale
    assert new.driver("tax_rate") == model.driver("tax_rate")
    assert model.driver("kd_spread").mean == -0.001


def test_cli_over10y_json_and_error(capsys):
    obs = rates.RateObservation("A yield minus UST 10Y", date(2026, 11, 17), 0.0067, "s", "A- uses the A index")
    with patch.object(bonds, "spread_over_10y", return_value=obs) as f:
        assert bonds.main(["over10y", "--rating", "A-", "--json"]) == 0
    f.assert_called_once_with("A-", None)
    assert json.loads(capsys.readouterr().out)["value"] == 0.0067
    assert bonds.main(["spread", "--rating", "ZZZ", "--json"]) == 2
    assert json.loads(capsys.readouterr().err)["type"] == "ValueError"
