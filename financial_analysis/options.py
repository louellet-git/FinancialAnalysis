"""Option-chain snapshots and derived metrics via yfinance.

yfinance only exposes the CURRENT option chain — there is no free historical
endpoint. Every pull is therefore persisted as a dated snapshot under
``data/raw/TICKER/options/YYYY-MM-DD.json`` (one per calendar day; re-pulling
on the same day overwrites). History accumulates from first use. Paid
providers (FMP, Polygon) are the upgrade path if backfilled history is needed.
"""
from __future__ import annotations
import json
import logging
import math
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import List, Optional

import yfinance as yf

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
# Fetching
# ---------------------------------------------------------------------------

def _clean_float(v) -> Optional[float]:
    if v is None:
        return None
    try:
        f = float(v)
    except (TypeError, ValueError):
        return None
    if math.isnan(f) or math.isinf(f):
        return None
    return round(f, 6)


def _clean_int(v) -> Optional[int]:
    f = _clean_float(v)
    return None if f is None else int(f)


def _parse_chain_df(df, option_type: str, expiry: date) -> List[OptionQuote]:
    if df is None or df.empty:
        return []
    quotes = []
    for _, row in df.iterrows():
        strike = _clean_float(row.get("strike"))
        if strike is None:
            logger.warning("Skipping %s %s contract with no strike", expiry, option_type)
            continue
        itm = row.get("inTheMoney")
        quotes.append(OptionQuote(
            contract_symbol=str(row.get("contractSymbol") or ""),
            option_type=option_type,
            expiry=expiry,
            strike=strike,
            last_price=_clean_float(row.get("lastPrice")),
            bid=_clean_float(row.get("bid")),
            ask=_clean_float(row.get("ask")),
            volume=_clean_int(row.get("volume")),
            open_interest=_clean_int(row.get("openInterest")),
            implied_volatility=_clean_float(row.get("impliedVolatility")),
            in_the_money=None if itm is None else bool(itm),
        ))
    return quotes


def fetch_option_chain(
    ticker: str,
    cache_dir: Path,
    refresh: bool = False,
) -> OptionChainSnapshot:
    """Pull the full option chain via yfinance and persist a dated snapshot.

    One snapshot per calendar day: a same-day re-run loads from disk unless
    refresh=True, in which case the day's snapshot is overwritten.
    """
    key = ticker.upper()
    today = date.today()

    if not refresh:
        path = _options_dir(key, cache_dir) / f"{today.isoformat()}.json"
        if path.exists():
            try:
                return load_snapshot(key, cache_dir, snapshot_date=today)
            except (json.JSONDecodeError, KeyError, ValueError):
                logger.warning("Corrupt options snapshot for %s — re-fetching", key)

    t = yf.Ticker(key)

    spot = None
    hist = t.history(period="5d")
    if hist is None or hist.empty:
        logger.warning("yfinance returned no recent prices for %s — spot unavailable", key)
    else:
        spot = _clean_float(hist["Close"].iloc[-1])

    try:
        expiries = tuple(t.options or ())
    except Exception as e:
        logger.warning("yfinance options listing failed for %s: %s", key, e)
        expiries = ()

    quotes: List[OptionQuote] = []
    for exp_str in expiries:
        try:
            expiry = date.fromisoformat(exp_str)
        except ValueError:
            logger.warning("Skipping unparseable expiry %r for %s", exp_str, key)
            continue
        try:
            chain = t.option_chain(exp_str)
        except Exception as e:
            logger.warning("option_chain(%s) failed for %s: %s", exp_str, key, e)
            continue
        quotes.extend(_parse_chain_df(getattr(chain, "calls", None), "call", expiry))
        quotes.extend(_parse_chain_df(getattr(chain, "puts", None), "put", expiry))

    if not quotes:
        logger.warning("No listed options found for %s — saving empty snapshot", key)

    quotes.sort(key=lambda q: (q.expiry, q.option_type, q.strike))
    snapshot = OptionChainSnapshot(ticker=key, snapshot_date=today, spot_price=spot, quotes=quotes)
    save_snapshot(snapshot, cache_dir)
    return snapshot


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
