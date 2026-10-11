"""The API every bot implements.

A bot subclasses ``Strategy``, sets ``name`` / ``symbols`` / ``interval``, and implements
``on_bar(ctx)``. On each bar close it may call ``ctx.open(...)`` (filled at the NEXT
bar's open, sized by the engine's risk rules) and ``ctx.close(...)`` (filled at this
bar's close). Exits declared in ``ExitRules`` are checked by the engine on every bar
close before ``on_bar`` runs. Positions still open at expiry settle at intrinsic.

``ctx`` only exposes data up to the current bar close — there is no way to see the
future through it. Do not load data yourself inside a strategy.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional, Tuple


@dataclass(frozen=True)
class ExitRules:
    """All thresholds are fractions of the entry PREMIUM (|net debit| or |net credit|).

    Debit trade: take_profit=1.0 → exit when up 100% of the debit; stop_loss=0.5 → exit
    when half the debit is lost. Credit trade: take_profit=0.5 → exit after capturing
    half the credit; stop_loss=2.0 → exit when the loss equals 2x the credit.
    Evaluated on liquidation value (long legs at bid, short legs at ask).
    """

    take_profit: Optional[float] = None
    stop_loss: Optional[float] = None
    minutes_before_close: Optional[int] = None  # on expiry day, exit this long before 16:00
    exit_dte: Optional[int] = None              # exit when <= this many sessions to expiry
    max_hold_bars: Optional[int] = None


class Strategy:
    name: str = ""
    description: str = ""
    symbols: Tuple[str, ...] = ("SPY",)
    interval: str = "1d"          # "1d", "1h" or "5m"
    warmup_bars: int = 0          # bars before this never trade (indicator warm-up)
    start: Optional[str] = None   # default backtest start date (ISO), None = all data

    def on_start(self, ctx) -> None:
        """Called once before the first bar."""

    def on_bar(self, ctx) -> None:
        raise NotImplementedError

    def params(self) -> dict:
        """Tunable parameters (public UPPERCASE class attributes) for reporting."""
        return {k: getattr(self, k) for k in dir(self) if k.isupper() and not k.startswith("_")}
