"""s20: walk-forward L2-logistic model of the next-5-day direction, high-confidence only.

numpy only. Every ~21 sessions the model is refit on samples whose 5-day outcome was
already known at that bar (sample t is admitted only when t + HORIZON <= now), with
standardisation stats taken from the training window only. Trades a 5-DTE $1-wide
debit spread in the predicted direction when P(up) is >= P_HI (call spread) or
<= P_LO (put spread). Out-of-sample model quality is exposed via ``self.pred_log``.
"""
from __future__ import annotations

import numpy as np

from trading.strategy import ExitRules, Strategy
from trading.structures import vertical

FEATURES = ["r1", "r3", "r5", "r10", "r21", "d10", "d50", "d200", "rv10", "vix", "dvix5", "vrp", "mon", "fri"]


def make_features(close: np.ndarray, vix: np.ndarray, dow: int):
    """Feature row from history ending at the LAST element (no future data). None if short."""
    if len(close) < 201 or len(vix) < 6:
        return None
    c = close[-1]
    lr = np.diff(np.log(close[-22:]))
    rv10 = lr[-10:].std(ddof=1) * np.sqrt(252)
    rv20 = lr[-20:].std(ddof=1) * np.sqrt(252)
    ret = lambda k: np.log(c / close[-1 - k])
    sma = lambda k: close[-k:].mean()
    v = vix[-1]
    return np.array([
        ret(1), ret(3), ret(5), ret(10), ret(21),
        c / sma(10) - 1, c / sma(50) - 1, c / sma(200) - 1,
        rv10, np.log(v), np.log(v / vix[-6]), v - rv20,
        float(dow == 0), float(dow == 4),
    ])


def fit_logistic(X: np.ndarray, y: np.ndarray, lam: float, iters: int = 12):
    """L2 logistic regression via Newton; intercept unpenalised. X already standardised."""
    n, d = X.shape
    A = np.c_[np.ones(n), X]
    w = np.zeros(d + 1)
    pen = np.r_[0.0, np.full(d, lam)]
    for _ in range(iters):
        z = np.clip(A @ w, -30, 30)
        p = 1 / (1 + np.exp(-z))
        g = A.T @ (p - y) + pen * w
        H = (A * (p * (1 - p))[:, None]).T @ A + np.diag(pen) + 1e-9 * np.eye(d + 1)
        w -= np.linalg.solve(H, g)
    return w


class WalkForwardModel:
    """Expanding-window walk-forward fitter. Leak-free by construction."""

    def __init__(self, horizon=5, lam=300.0, retrain=21, min_train=750):
        self.h, self.lam, self.retrain, self.min_train = horizon, lam, retrain, min_train
        self.feats: dict = {}     # bar index -> feature row
        self.closes: dict = {}    # bar index -> close
        self.w = self.mu = self.sd = None
        self.last_fit = -10**9

    def observe(self, i: int, x, close: float):
        self.closes[i] = close
        if x is not None:
            self.feats[i] = x

    def maybe_fit(self, i: int):
        if i - self.last_fit < self.retrain:
            return
        # only samples whose h-bar outcome is already observed at bar i
        idx = sorted(t for t in self.feats if t + self.h <= i)
        if len(idx) < self.min_train:
            return
        X = np.array([self.feats[t] for t in idx])
        y = np.array([float(self.closes[t + self.h] > self.closes[t]) for t in idx])
        self.mu, self.sd = X.mean(0), X.std(0) + 1e-12
        self.w = fit_logistic((X - self.mu) / self.sd, y, self.lam)
        self.last_fit = i

    def predict(self, x):
        if self.w is None or x is None:
            return None
        z = self.w[0] + ((x - self.mu) / self.sd) @ self.w[1:]
        return float(1 / (1 + np.exp(-z)))


class WalkForwardDirection(Strategy):
    name = "s20_walkforward_model"
    description = ("Walk-forward L2 logistic model on returns/MA distance/realised vol/VIX/day-of-week "
                   "predicting 5-day direction; trades only high-confidence calls with 5-DTE debit spreads.")
    symbols = ("SPY",)
    interval = "1d"
    warmup_bars = 0
    start = "2005-01-03"

    HORIZON = 5
    LAMBDA = 300.0
    RETRAIN = 21
    MIN_TRAIN = 750
    P_HI = 0.62
    P_LO = 0.38
    DTE = 5
    DELTA = 0.50
    WIDTH = 1.0
    TAKE_PROFIT = None
    STOP_LOSS = None
    EXIT_DTE = 0

    def on_start(self, ctx):
        self.model = WalkForwardModel(self.HORIZON, self.LAMBDA, self.RETRAIN, self.MIN_TRAIN)
        self.pred_log = []  # (bar index, timestamp, P(up)) for every prediction made

    def on_bar(self, ctx):
        sym = self.symbols[0]
        close = ctx.bars(sym)["close"].to_numpy()[-260:]
        vix = ctx.vol_history(sym)["atm_close"].to_numpy()[-10:]
        x = make_features(close, vix, ctx.today.weekday())
        self.model.observe(ctx.i, x, float(close[-1]))
        self.model.maybe_fit(ctx.i)
        p = self.model.predict(x)
        if p is None:
            return
        self.pred_log.append((ctx.i, ctx.ts, p))
        if ctx.positions or ctx.pending_entries:
            return
        if p >= self.P_HI:
            cp = "C"
        elif p <= self.P_LO:
            cp = "P"
        else:
            return
        exp = ctx.expiry(self.DTE + 1)  # fills next session
        k = ctx.strike_for_delta(sym, cp, self.DELTA, exp)
        other = k + self.WIDTH if cp == "C" else k - self.WIDTH
        ctx.open(vertical(sym, exp, cp, k, other),
                 ExitRules(take_profit=self.TAKE_PROFIT, stop_loss=self.STOP_LOSS, exit_dte=self.EXIT_DTE),
                 tag=f"{cp}@{p:.2f}")


STRATEGY = WalkForwardDirection
