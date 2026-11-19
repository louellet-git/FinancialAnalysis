"""Option-chain snapshots (read side) and derived metrics.

Legacy reader of the dated snapshots under ``data/raw/TICKER/options/YYYY-MM-DD.json``.
The snapshots are made by the Rust program (``fa analyze``, from the Cboe delayed
quotes); the Rust form of this module is ``rust/src/market/options.rs``. This file stays
only because ``scripts/visualize_dcf.py`` (the HTML report, not yet in Rust) reads
snapshots and the metrics through it. Delete it when that report moves to Rust.
"""
from __future__ import annotations
import json
import logging
import math
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import List, Optional

from financial_analysis.schema import OptionChainSnapshot, OptionQuote

logger = logging.getLogger(__name__)

# Moneyness targets for the skew metric (downside put wing vs upside call wing).
SKEW_PUT_MONEYNESS = 0.90
SKEW_CALL_MONEYNESS = 1.10

# yfinance reports ~1e-5 implied vol for quotes it could not solve — treat as missing.
_MIN_VALID_IV = 1e-4


# ---------------------------------------------------------------------------
# Persistence: one dated snapshot per calendar day
# ---------------------------------------------------------------------------

def _options_dir(ticker: str, cache_dir: Path) -> Path:
    return cache_dir / ticker.upper() / "options"


def _snapshot_to_dict(snapshot: OptionChainSnapshot) -> dict:
    return {
        "ticker": snapshot.ticker,
        "snapshot_date": snapshot.snapshot_date.isoformat(),
        "spot_price": snapshot.spot_price,
        "quotes": [
            {
                "contract_symbol": q.contract_symbol,
                "option_type": q.option_type,
                "expiry": q.expiry.isoformat(),
                "strike": q.strike,
                "last_price": q.last_price,
                "bid": q.bid,
                "ask": q.ask,
                "volume": q.volume,
                "open_interest": q.open_interest,
                "implied_volatility": q.implied_volatility,
                "in_the_money": q.in_the_money,
            }
            for q in snapshot.quotes
        ],
    }


def _snapshot_from_dict(raw: dict) -> OptionChainSnapshot:
    quotes = [
        OptionQuote(
            contract_symbol=r["contract_symbol"],
            option_type=r["option_type"],
            expiry=date.fromisoformat(r["expiry"]),
            strike=r["strike"],
            last_price=r.get("last_price"),
            bid=r.get("bid"),
            ask=r.get("ask"),
            volume=r.get("volume"),
            open_interest=r.get("open_interest"),
            implied_volatility=r.get("implied_volatility"),
            in_the_money=r.get("in_the_money"),
        )
        for r in raw["quotes"]
    ]
    return OptionChainSnapshot(
        ticker=raw["ticker"],
        snapshot_date=date.fromisoformat(raw["snapshot_date"]),
        spot_price=raw.get("spot_price"),
        quotes=quotes,
    )


def save_snapshot(snapshot: OptionChainSnapshot, cache_dir: Path) -> Path:
    path = _options_dir(snapshot.ticker, cache_dir) / f"{snapshot.snapshot_date.isoformat()}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(_snapshot_to_dict(snapshot), indent=2), encoding="utf-8")
    return path


def list_snapshots(ticker: str, cache_dir: Path) -> List[date]:
    """Return the dates of all persisted snapshots for a ticker, oldest first."""
    directory = _options_dir(ticker, cache_dir)
    if not directory.exists():
        return []
    dates = []
    for p in directory.glob("*.json"):
        try:
            dates.append(date.fromisoformat(p.stem))
        except ValueError:
            logger.warning("Ignoring non-snapshot file in %s: %s", directory, p.name)
    return sorted(dates)


def load_snapshot(
    ticker: str,
    cache_dir: Path,
    snapshot_date: Optional[date] = None,
) -> OptionChainSnapshot:
    """Load a persisted option-chain snapshot. None → most recent snapshot."""
    key = ticker.upper()
    if snapshot_date is None:
        available = list_snapshots(key, cache_dir)
        if not available:
            raise FileNotFoundError(
                f"No option-chain snapshots for {key} under {_options_dir(key, cache_dir)}"
            )
        snapshot_date = available[-1]

    path = _options_dir(key, cache_dir) / f"{snapshot_date.isoformat()}.json"
    if not path.exists():
        raise FileNotFoundError(f"No option-chain snapshot for {key} on {snapshot_date} at {path}")
    return _snapshot_from_dict(json.loads(path.read_text(encoding="utf-8")))


# ---------------------------------------------------------------------------
# Derived metrics
# ---------------------------------------------------------------------------

@dataclass
class ExpiryMetrics:
    expiry: date
    atm_strike: Optional[float] = None
    atm_iv: Optional[float] = None
    straddle_mid: Optional[float] = None
    implied_move: Optional[float] = None
    skew: Optional[float] = None
    call_volume: int = 0
    put_volume: int = 0
    call_open_interest: int = 0
    put_open_interest: int = 0

    @property
    def put_call_volume_ratio(self) -> Optional[float]:
        if self.call_volume == 0:
            return None
        return round(self.put_volume / self.call_volume, 6)

    @property
    def put_call_oi_ratio(self) -> Optional[float]:
        if self.call_open_interest == 0:
            return None
        return round(self.put_open_interest / self.call_open_interest, 6)


@dataclass
class ChainMetrics:
    ticker: str
    snapshot_date: date
    spot_price: Optional[float]
    expiries: List[ExpiryMetrics] = field(default_factory=list)
    put_call_volume_ratio: Optional[float] = None
    put_call_oi_ratio: Optional[float] = None


def _valid_iv(quote: Optional[OptionQuote]) -> Optional[float]:
    if quote is None or quote.implied_volatility is None:
        return None
    iv = quote.implied_volatility
    if math.isnan(iv) or iv < _MIN_VALID_IV:
        return None
    return iv


def _quote_price(quote: Optional[OptionQuote]) -> Optional[float]:
    """Bid/ask mid where both sides are quoted, falling back to last trade."""
    if quote is None:
        return None
    if quote.bid and quote.ask and quote.bid > 0 and quote.ask > 0:
        return (quote.bid + quote.ask) / 2
    if quote.last_price and quote.last_price > 0:
        return quote.last_price
    return None


def _nearest_by_strike(quotes: List[OptionQuote], target: float) -> Optional[OptionQuote]:
    if not quotes:
        return None
    return min(quotes, key=lambda q: abs(q.strike - target))


def compute_chain_metrics(snapshot: OptionChainSnapshot) -> ChainMetrics:
    """Derive per-expiry ATM IV, implied move, skew, and put/call ratios.

    Sparse chains are handled defensively: any metric whose inputs are
    missing (no bid/ask/last, unsolved IV, single-sided chain) is None
    rather than a fabricated number.
    """
    metrics = ChainMetrics(
        ticker=snapshot.ticker,
        snapshot_date=snapshot.snapshot_date,
        spot_price=snapshot.spot_price,
    )
    spot = snapshot.spot_price

    by_expiry: dict[date, List[OptionQuote]] = {}
    for q in snapshot.quotes:
        by_expiry.setdefault(q.expiry, []).append(q)

    for expiry in sorted(by_expiry):
        group = by_expiry[expiry]
        calls = [q for q in group if q.option_type == "call"]
        puts = [q for q in group if q.option_type == "put"]

        em = ExpiryMetrics(
            expiry=expiry,
            call_volume=sum(q.volume or 0 for q in calls),
            put_volume=sum(q.volume or 0 for q in puts),
            call_open_interest=sum(q.open_interest or 0 for q in calls),
            put_open_interest=sum(q.open_interest or 0 for q in puts),
        )

        if spot is not None and spot > 0 and group:
            strikes = sorted({q.strike for q in group})
            atm_strike = min(strikes, key=lambda s: abs(s - spot))
            em.atm_strike = atm_strike
            atm_call = next((q for q in calls if q.strike == atm_strike), None)
            atm_put = next((q for q in puts if q.strike == atm_strike), None)

            # ATM implied vol: average call/put IV where both exist.
            ivs = [iv for iv in (_valid_iv(atm_call), _valid_iv(atm_put)) if iv is not None]
            if ivs:
                em.atm_iv = round(sum(ivs) / len(ivs), 6)

            # Implied move: ATM straddle mid ÷ spot (needs both legs priced).
            call_px = _quote_price(atm_call)
            put_px = _quote_price(atm_put)
            if call_px is not None and put_px is not None:
                em.straddle_mid = round(call_px + put_px, 6)
                em.implied_move = round(em.straddle_mid / spot, 6)

            # Skew: IV of ~90%-moneyness put minus IV of ~110%-moneyness call.
            skew_put = _nearest_by_strike(
                [q for q in puts if _valid_iv(q) is not None], spot * SKEW_PUT_MONEYNESS
            )
            skew_call = _nearest_by_strike(
                [q for q in calls if _valid_iv(q) is not None], spot * SKEW_CALL_MONEYNESS
            )
            if skew_put is not None and skew_call is not None:
                em.skew = round(_valid_iv(skew_put) - _valid_iv(skew_call), 6)

        metrics.expiries.append(em)

    total_call_vol = sum(em.call_volume for em in metrics.expiries)
    total_put_vol = sum(em.put_volume for em in metrics.expiries)
    total_call_oi = sum(em.call_open_interest for em in metrics.expiries)
    total_put_oi = sum(em.put_open_interest for em in metrics.expiries)
    if total_call_vol > 0:
        metrics.put_call_volume_ratio = round(total_put_vol / total_call_vol, 6)
    if total_call_oi > 0:
        metrics.put_call_oi_ratio = round(total_put_oi / total_call_oi, 6)

    return metrics
