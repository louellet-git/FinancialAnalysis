"""Defined-risk option structures. All legs share one expiry so max loss is exact.

Quantities are per ONE unit of the structure (e.g. a vertical = +1 / -1). Prices are
per share; multiply by ``MULTIPLIER`` for dollars per contract.
"""
from __future__ import annotations

from dataclasses import dataclass
from datetime import date
from typing import Tuple

from trading.pricing import intrinsic

MULTIPLIER = 100


@dataclass(frozen=True)
class Leg:
    cp: str        # "C" or "P"
    strike: float
    qty: int       # +long / -short, per unit


@dataclass(frozen=True)
class Structure:
    name: str
    symbol: str
    expiry: date
    legs: Tuple[Leg, ...]

    def __post_init__(self):
        if not self.legs:
            raise ValueError("structure needs at least one leg")
        for leg in self.legs:
            if leg.cp not in ("C", "P") or leg.qty == 0 or leg.strike <= 0:
                raise ValueError(f"bad leg {leg}")
        # Defined risk only: no net-short calls (unbounded) or net-short puts (naked).
        for cp in ("C", "P"):
            if sum(l.qty for l in self.legs if l.cp == cp) < 0:
                raise ValueError(f"{self.name}: net short {cp} legs = not defined-risk")

    @property
    def contracts_per_unit(self) -> int:
        return sum(abs(l.qty) for l in self.legs)

    def payoff(self, spot: float) -> float:
        """Expiry value per unit, per share."""
        return sum(l.qty * intrinsic(l.cp, spot, l.strike) for l in self.legs)

    def min_payoff(self) -> float:
        """Payoff is piecewise linear: check 0, every strike, and the far upside."""
        strikes = sorted({l.strike for l in self.legs})
        points = [0.0] + strikes + [strikes[-1] * 10]
        return min(self.payoff(s) for s in points)

    def max_payoff(self) -> float:
        strikes = sorted({l.strike for l in self.legs})
        points = [0.0] + strikes + [strikes[-1] * 10]
        return max(self.payoff(s) for s in points)

    def max_loss(self, entry_net: float) -> float:
        """Worst-case loss per unit per share given net entry price (+debit / -credit)."""
        return entry_net - self.min_payoff()

    def describe(self) -> str:
        legs = " ".join(f"{'+' if l.qty > 0 else ''}{l.qty}{l.cp}{l.strike:g}" for l in self.legs)
        return f"{self.symbol} {self.expiry.isoformat()} {self.name} [{legs}]"


def round_strike(x: float, increment: float = 1.0) -> float:
    return round(round(x / increment) * increment, 2)


def long_call(symbol: str, expiry: date, strike: float) -> Structure:
    return Structure("long_call", symbol, expiry, (Leg("C", strike, 1),))


def long_put(symbol: str, expiry: date, strike: float) -> Structure:
    return Structure("long_put", symbol, expiry, (Leg("P", strike, 1),))


def vertical(symbol: str, expiry: date, cp: str, long_strike: float, short_strike: float) -> Structure:
    """Call debit: long < short. Put debit: long > short. Reverse for credit spreads."""
    if long_strike == short_strike:
        raise ValueError("vertical strikes must differ")
    debit = (long_strike < short_strike) if cp == "C" else (long_strike > short_strike)
    name = f"{'call' if cp == 'C' else 'put'}_{'debit' if debit else 'credit'}_spread"
    return Structure(name, symbol, expiry, (Leg(cp, long_strike, 1), Leg(cp, short_strike, -1)))


def iron_condor(symbol: str, expiry: date, put_long: float, put_short: float,
                call_short: float, call_long: float) -> Structure:
    if not put_long < put_short <= call_short < call_long:
        raise ValueError("iron condor strikes must be ordered put_long < put_short <= call_short < call_long")
    name = "iron_butterfly" if put_short == call_short else "iron_condor"
    return Structure(name, symbol, expiry, (
        Leg("P", put_long, 1), Leg("P", put_short, -1), Leg("C", call_short, -1), Leg("C", call_long, 1),
    ))


def butterfly(symbol: str, expiry: date, cp: str, lower: float, middle: float, upper: float) -> Structure:
    if not lower < middle < upper:
        raise ValueError("butterfly strikes must be ordered")
    return Structure(f"{'call' if cp == 'C' else 'put'}_butterfly", symbol, expiry, (
        Leg(cp, lower, 1), Leg(cp, middle, -2), Leg(cp, upper, 1),
    ))
