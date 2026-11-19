"""SPY/QQQ options trading research harness.

Separate from the valuation factory (``financial_analysis``). Layers:

- ``data``       — bar/vol/rate loader (yfinance, CSV cache that accumulates history)
- ``pricing``    — pluggable option pricer; default is SYNTHETIC (Black-Scholes on
                   VIX/VXN/VIX1D + skew + spread). No free historical option quotes exist.
- ``structures`` — defined-risk option structures (max loss known at entry)
- ``strategy``   — the Strategy / Context API every bot implements
- ``engine``     — event-driven backtester with account-level risk rules (sizing, PDT)
- ``metrics``    — performance stats, in-sample / out-of-sample split
- ``registry``   — discovers bots in ``trading/strategies/``
- ``paper``      — forward paper trading (no broker orders, ever)
"""
