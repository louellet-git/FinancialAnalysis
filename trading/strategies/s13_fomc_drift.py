"""Pre-FOMC announcement drift (Lucca & Moench 2015) via short-dated call debit spreads.

Equities have historically earned a large share of their annual equity premium in the
~24h before scheduled FOMC statements (2:00pm ET). With daily bars we cannot isolate
the exact 24h window, so we hold from the OPEN of the session before the announcement
day through the CLOSE of the announcement day (the announcement itself is in that
window; a little post-announcement noise is accepted).

Dates are the SCHEDULED statement days (published a year ahead, so not lookahead);
unscheduled/emergency actions (2008-01-22, 2008-10-08, 2020-03-03, 2020-03-15) and the
cancelled 2020-03-18 meeting are deliberately excluded.
"""
from __future__ import annotations

from datetime import date

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical

_FOMC = """2005 0202 0322 0503 0630 0809 0920 1101 1213
2006 0131 0328 0510 0629 0808 0920 1025 1212
2007 0131 0321 0509 0628 0807 0918 1031 1211
2008 0130 0318 0430 0625 0805 0916 1029 1216
2009 0128 0318 0429 0624 0812 0923 1104 1216
2010 0127 0316 0428 0623 0810 0921 1103 1214
2011 0126 0315 0427 0622 0809 0921 1102 1213
2012 0125 0313 0425 0620 0801 0913 1024 1212
2013 0130 0320 0501 0619 0731 0918 1030 1218
2014 0129 0319 0430 0618 0730 0917 1029 1217
2015 0128 0318 0429 0617 0729 0917 1028 1216
2016 0127 0316 0427 0615 0727 0921 1102 1214
2017 0201 0315 0503 0614 0726 0920 1101 1213
2018 0131 0321 0502 0613 0801 0926 1108 1219
2019 0130 0320 0501 0619 0731 0918 1030 1211
2020 0129 0429 0610 0729 0916 1105 1216
2021 0127 0317 0428 0616 0728 0922 1103 1215
2022 0126 0316 0504 0615 0727 0921 1102 1214
2023 0201 0322 0503 0614 0726 0920 1101 1213
2024 0131 0320 0501 0612 0731 0918 1107 1218
2025 0129 0319 0507 0618 0730 0917 1029 1210
2026 0128 0318 0429 0617 0729 0916 1028 1209"""

FOMC_DATES = frozenset(
    date(int(l.split()[0]), int(d[:2]), int(d[2:])) for l in _FOMC.splitlines() for d in l.split()[1:]
)


class PreFomcDrift(Strategy):
    name = "s13_fomc_drift"
    description = ("Buy a ~0.50-delta $1 SPY call debit spread at the open of the session before a "
                   "scheduled FOMC announcement; exit at the close of announcement day")
    symbols = ("SPY",)
    interval = "1d"
    warmup_bars = 0
    start = "2005-01-01"

    DELTA = 0.50          # long-strike delta
    WIDTH = 1.0           # spread width in $
    EXTRA_DTE = 1         # sessions of expiry beyond announcement day (avoid 0DTE at exit)
    MIN_YEAR = 2005       # optional: set to 2015 to test only the post-decay era
    SKIP_VIX_ABOVE = 1.0  # decimals; VIX above this -> skip (1.0 = off)

    def on_bar(self, ctx):
        # Exit first: announcement day close.
        for p in ctx.positions:
            if ctx.today >= ctx.state.get("exit_on", ctx.today):
                ctx.close(p, "fomc_day_close")
        if ctx.positions or ctx.pending_entries:
            return
        # Decision at close of D-2 -> fills at D-1 open. D = this day + 2 sessions.
        d = ctx.expiry(2)
        if d not in FOMC_DATES or d.year < self.MIN_YEAR:
            return
        if ctx.vol().atm >= self.SKIP_VIX_ABOVE:
            return
        exp = ctx.expiry(2 + self.EXTRA_DTE)
        k = ctx.strike_for_delta("SPY", "C", self.DELTA, exp)
        ctx.state["exit_on"] = d
        ctx.open(vertical("SPY", exp, "C", k, k + self.WIDTH),
                 ExitRules(), tag="fomc")


STRATEGY = PreFomcDrift
