from datetime import date

import pytest

from financial_analysis.options import (
    compute_chain_metrics,
    list_snapshots,
    load_snapshot,
    save_snapshot,
)
from financial_analysis.schema import OptionChainSnapshot, OptionQuote


# ---------------------------------------------------------------------------
# Persistence: round-trip, listing, latest/dated loading
# ---------------------------------------------------------------------------

def _snapshot(day: date) -> OptionChainSnapshot:
    return OptionChainSnapshot(
        ticker="TEST", snapshot_date=day, spot_price=100.0,
        quotes=[
            OptionQuote("TEST260724C00100000", "call", date(2026, 7, 24), 100.0,
                        last_price=5.0, bid=4.9, ask=5.1, volume=120, open_interest=800,
                        implied_volatility=0.30, in_the_money=False),
            OptionQuote("TEST260724P00100000", "put", date(2026, 7, 24), 100.0,
                        last_price=4.8, bid=None, ask=None, volume=None, open_interest=0,
                        implied_volatility=None, in_the_money=None),
        ],
    )


def test_snapshot_round_trip(tmp_path):
    original = _snapshot(date(2026, 7, 16))
    save_snapshot(original, cache_dir=tmp_path)
    loaded = load_snapshot("TEST", tmp_path, snapshot_date=date(2026, 7, 16))
    assert loaded == original


def test_list_snapshots_sorted_oldest_first(tmp_path):
    for d in (date(2026, 7, 16), date(2026, 7, 14), date(2026, 7, 15)):
        save_snapshot(_snapshot(d), cache_dir=tmp_path)
    assert list_snapshots("TEST", tmp_path) == [
        date(2026, 7, 14), date(2026, 7, 15), date(2026, 7, 16),
    ]


def test_list_snapshots_empty_when_none(tmp_path):
    assert list_snapshots("TEST", tmp_path) == []


def test_load_snapshot_default_is_latest(tmp_path):
    for d in (date(2026, 7, 14), date(2026, 7, 16), date(2026, 7, 15)):
        snap = _snapshot(d)
        snap.spot_price = 100.0 + d.day
        save_snapshot(snap, cache_dir=tmp_path)
    loaded = load_snapshot("TEST", tmp_path)
    assert loaded.snapshot_date == date(2026, 7, 16)
    assert loaded.spot_price == 116.0


def test_load_snapshot_specific_date(tmp_path):
    for d in (date(2026, 7, 14), date(2026, 7, 16)):
        save_snapshot(_snapshot(d), cache_dir=tmp_path)
    loaded = load_snapshot("TEST", tmp_path, snapshot_date=date(2026, 7, 14))
    assert loaded.snapshot_date == date(2026, 7, 14)


def test_load_snapshot_missing_ticker_raises(tmp_path):
    with pytest.raises(FileNotFoundError, match="No option-chain snapshots for TEST"):
        load_snapshot("TEST", tmp_path)


def test_load_snapshot_missing_date_raises(tmp_path):
    save_snapshot(_snapshot(date(2026, 7, 16)), cache_dir=tmp_path)
    with pytest.raises(FileNotFoundError, match="on 2026-07-01"):
        load_snapshot("TEST", tmp_path, snapshot_date=date(2026, 7, 1))


# ---------------------------------------------------------------------------
# Derived metrics
# ---------------------------------------------------------------------------

def _q(option_type, expiry, strike, last=None, bid=None, ask=None,
       volume=None, oi=None, iv=None):
    return OptionQuote(
        contract_symbol=f"TEST-{option_type}-{strike}",
        option_type=option_type, expiry=expiry, strike=strike,
        last_price=last, bid=bid, ask=ask, volume=volume,
        open_interest=oi, implied_volatility=iv,
    )


NEAR = date(2026, 7, 24)
FAR = date(2026, 8, 21)


def _rich_snapshot() -> OptionChainSnapshot:
    return OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0,
        quotes=[
            _q("call", NEAR, 90.0, last=11.0, bid=10.8, ask=11.2, volume=5, oi=100, iv=0.36),
            _q("call", NEAR, 100.0, last=5.0, bid=4.9, ask=5.1, volume=120, oi=800, iv=0.30),
            _q("call", NEAR, 110.0, last=1.2, bid=1.1, ask=1.3, volume=30, oi=300, iv=0.28),
            _q("put", NEAR, 90.0, last=1.1, bid=1.0, ask=1.2, volume=40, oi=500, iv=0.40),
            _q("put", NEAR, 100.0, last=4.8, bid=4.7, ask=4.9, volume=60, oi=400, iv=0.32),
        ],
    )


def test_atm_iv_averages_call_and_put():
    m = compute_chain_metrics(_rich_snapshot())
    em = m.expiries[0]
    assert em.atm_strike == 100.0
    assert em.atm_iv == pytest.approx((0.30 + 0.32) / 2)


def test_atm_iv_single_sided_uses_available_leg():
    snap = OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0,
        quotes=[_q("call", NEAR, 100.0, last=5.0, iv=0.30)],
    )
    em = compute_chain_metrics(snap).expiries[0]
    assert em.atm_iv == pytest.approx(0.30)


def test_atm_iv_none_when_iv_unsolved():
    # yfinance's 1e-5 placeholder IV must not be treated as a real vol
    snap = OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0,
        quotes=[_q("call", NEAR, 100.0, last=5.0, iv=1e-5)],
    )
    em = compute_chain_metrics(snap).expiries[0]
    assert em.atm_iv is None


def test_implied_move_from_bid_ask_mids():
    em = compute_chain_metrics(_rich_snapshot()).expiries[0]
    # call mid 5.0 + put mid 4.8 = 9.8 straddle on spot 100
    assert em.straddle_mid == pytest.approx(9.8)
    assert em.implied_move == pytest.approx(0.098)


def test_implied_move_falls_back_to_last_when_no_bid():
    snap = OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0,
        quotes=[
            _q("call", NEAR, 100.0, last=5.0),          # no bid/ask
            _q("put", NEAR, 100.0, bid=4.7, ask=4.9),
        ],
    )
    em = compute_chain_metrics(snap).expiries[0]
    assert em.straddle_mid == pytest.approx(5.0 + 4.8)


def test_implied_move_none_for_single_sided_chain():
    snap = OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0,
        quotes=[_q("call", NEAR, 100.0, last=5.0, bid=4.9, ask=5.1, iv=0.30)],
    )
    em = compute_chain_metrics(snap).expiries[0]
    assert em.implied_move is None
    assert em.straddle_mid is None


def test_implied_move_none_when_leg_has_no_price_at_all():
    snap = OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0,
        quotes=[
            _q("call", NEAR, 100.0, last=5.0, bid=4.9, ask=5.1),
            _q("put", NEAR, 100.0),  # zero bid, no last trade
        ],
    )
    em = compute_chain_metrics(snap).expiries[0]
    assert em.implied_move is None


def test_skew_put_wing_minus_call_wing():
    em = compute_chain_metrics(_rich_snapshot()).expiries[0]
    # 90-strike put IV 0.40 minus 110-strike call IV 0.28
    assert em.skew == pytest.approx(0.12)


def test_skew_none_without_valid_put_iv():
    snap = OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0,
        quotes=[
            _q("call", NEAR, 110.0, last=1.2, iv=0.28),
            _q("put", NEAR, 90.0, last=1.1, iv=None),
        ],
    )
    em = compute_chain_metrics(snap).expiries[0]
    assert em.skew is None


def test_put_call_ratios_per_expiry_and_chain():
    m = compute_chain_metrics(_rich_snapshot())
    em = m.expiries[0]
    assert em.call_volume == 155
    assert em.put_volume == 100
    assert em.put_call_volume_ratio == pytest.approx(100 / 155)
    assert em.put_call_oi_ratio == pytest.approx(900 / 1200)
    assert m.put_call_volume_ratio == pytest.approx(100 / 155)
    assert m.put_call_oi_ratio == pytest.approx(900 / 1200)


def test_put_call_ratio_none_when_no_call_activity():
    snap = OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0,
        quotes=[_q("put", NEAR, 100.0, volume=60, oi=400, iv=0.32)],
    )
    m = compute_chain_metrics(snap)
    assert m.expiries[0].put_call_volume_ratio is None
    assert m.put_call_volume_ratio is None
    assert m.put_call_oi_ratio is None


def test_metrics_empty_snapshot():
    snap = OptionChainSnapshot(ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0)
    m = compute_chain_metrics(snap)
    assert m.expiries == []
    assert m.put_call_volume_ratio is None
    assert m.put_call_oi_ratio is None


def test_metrics_without_spot_still_counts_volume():
    snap = OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=None,
        quotes=[
            _q("call", NEAR, 100.0, last=5.0, volume=120, oi=800, iv=0.30),
            _q("put", NEAR, 100.0, last=4.8, volume=60, oi=400, iv=0.32),
        ],
    )
    m = compute_chain_metrics(snap)
    em = m.expiries[0]
    assert em.atm_iv is None
    assert em.implied_move is None
    assert em.skew is None
    assert em.call_volume == 120
    assert m.put_call_volume_ratio == pytest.approx(0.5)


def test_metrics_expiries_sorted():
    snap = OptionChainSnapshot(
        ticker="TEST", snapshot_date=date(2026, 7, 16), spot_price=100.0,
        quotes=[
            _q("call", FAR, 100.0, last=8.0, iv=0.25),
            _q("call", NEAR, 100.0, last=5.0, iv=0.30),
        ],
    )
    m = compute_chain_metrics(snap)
    assert [em.expiry for em in m.expiries] == [NEAR, FAR]
