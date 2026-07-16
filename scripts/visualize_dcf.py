"""CLI: single-page interactive company report for a ticker.

Everything the repo knows about a ticker, one self-contained HTML file
(plotly.js inlined — no CDN or network access needed to view it):

1. Header band     — company, price, market cap, scenario P50 stat chips.
2. Price & valuation — full price history + a 6-month zoom with the DCF
                     scenario P50s and the base P10–P90 band overlaid.
3. Fundamentals    — annual revenue/FCF/margin chart + per-FY table.
4. Aggregate DCF   — the four Monte Carlo panels (fan, distribution,
                     waterfall, tornado) from financial_analysis/scenarios.py.
5. Driver model    — conditional on analysis/<TICKER>/driver_model.py
                     exposing build(): segment history, driver table,
                     correlation heatmap, DriverDCF percentiles vs the
                     aggregate base, driver attribution tornado.
6. Options market  — conditional on a snapshot under data/raw/<TICKER>/options/:
                     ATM IV term structure, implied move, skew, open-interest
                     heatmap, chain stat chips, post-earnings chain table.
7. Footer          — data provenance and engine parameters.

Sections degrade gracefully: missing data yields a labeled "not available"
note, never a crash.

Usage:
    python -m scripts.visualize_dcf IBM
    python -m scripts.visualize_dcf IBM --price 195.40   # skip yfinance lookup
    python -m scripts.visualize_dcf IBM --out report.html
"""
import argparse
import html
import importlib
import json
import sys
from dataclasses import replace
from datetime import date, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import numpy as np
import plotly.graph_objects as go
import plotly.io as pio
from plotly.subplots import make_subplots
from plotly.offline import get_plotlyjs

from financial_analysis import store
from financial_analysis.dcf import (
    _ANNUAL_FORMS,
    DCFAssumptions,
    DCFResult,
    StochasticDCF,
    derive_fcf_margin,
)
from financial_analysis.driver_dcf import DriverDCF, DriverDCFResult, DriverModel, SegmentSpec
from financial_analysis.schema import CompanyFinancials, PricePoint
from financial_analysis.scenarios import (
    CapitalStructure,
    Scenario,
    capital_structure_for,
    latest_annual,
    scenarios_for,
)

PANEL_TITLES = (
    "FCF projection fan",
    "Per-share value distribution",
    "PV build-up — median base case",
    "Sensitivity tornado",
)

SECTION_TITLES = (
    "Price & valuation",
    "Fundamentals",
    "Aggregate DCF scenarios",
    "Driver-based model",
    "Options market",
)

# Driver-model engine parameters (fixed for reproducibility).
DRIVER_SIMS = 10_000
DRIVER_SEED = 7

_DEFAULT_CACHE_DIR = Path("data/raw")

# Chart chrome (dataviz reference palette, light mode)
_SURFACE = "#fcfcfb"
_PAGE = "#f9f9f7"
_INK = "#0b0b0b"
_INK_SECONDARY = "#52514e"
_INK_MUTED = "#898781"
_GRID = "#e1e0d9"
_AXIS = "#c3c2b7"
_NEUTRAL = "#f0efec"
_FONT = 'system-ui, -apple-system, "Segoe UI", sans-serif'

# Categorical slots 1-3, fixed order (validated: worst adjacent CVD dE 47.2)
_SCENARIO_COLORS = {"bear": "#2a78d6", "base": "#1baf7a", "bull": "#eda100"}
# Full 8-slot categorical order (validated: worst adjacent CVD dE 24.2) —
# used for revenue segments and any other multi-series identity encoding.
_CATEGORICAL = ("#2a78d6", "#1baf7a", "#eda100", "#008300",
                "#4a3aa7", "#e34948", "#e87ba4", "#eb6834")
# Diverging pair for polarity (waterfall contributions, tornado shifts, skew,
# correlation heatmap) with a neutral gray midpoint.
_COOL = "#2a78d6"
_WARM = "#e34948"
_DIVERGING_SCALE = [[0.0, _WARM], [0.5, _NEUTRAL], [1.0, _COOL]]
# Sequential blue ramp (magnitude — OI heatmap); near zero recedes to surface.
_SEQ_BLUES = [[0.0, "#f4f8fd"], [0.14, "#cde2fb"], [0.3, "#9ec5f4"],
              [0.45, "#6da7ec"], [0.6, "#3987e5"], [0.75, "#256abf"],
              [0.9, "#184f95"], [1.0, "#0d366b"]]
# Delta text colors (direction × whether up is good) — text tokens, not marks.
_DELTA_UP = "#006300"
_DELTA_DOWN = "#d03b3b"
# Driver-model series color (categorical slot 5 — distinct from the three
# scenario slots so the two engines never share a hue).
_DRIVER_COLOR = "#4a3aa7"

_TORNADO_INPUTS = (
    ("Revenue growth", "growth_rate_mean", "growth_rate_std"),
    ("WACC", "wacc_mean", "wacc_std"),
    ("Terminal growth", "terminal_growth_mean", "terminal_growth_std"),
    ("FCF margin", "fcf_margin_mean", "fcf_margin_std"),
)


# ---------------------------------------------------------------------------
# Small shared helpers
# ---------------------------------------------------------------------------

def _rgba(hex_color: str, alpha: float) -> str:
    r, g, b = (int(hex_color[i:i + 2], 16) for i in (1, 3, 5))
    return f"rgba({r},{g},{b},{alpha})"


def _fetch_price(ticker: str) -> Optional[float]:
    try:
        import yfinance as yf
        price = getattr(yf.Ticker(ticker).fast_info, "last_price", None)
        return float(price) if price else None
    except Exception:
        return None


def _fetch_price_history(ticker: str) -> List[PricePoint]:
    """Full daily price history via yfinance; [] when offline/unavailable."""
    try:
        import yfinance as yf
        df = yf.Ticker(ticker.upper()).history(period="max")
        if df is None or df.empty:
            return []
        return [
            PricePoint(date=ts.date(), open=float(r["Open"]), high=float(r["High"]),
                       low=float(r["Low"]), close=float(r["Close"]),
                       volume=int(r["Volume"]))
            for ts, r in df.iterrows()
        ]
    except Exception:
        return []


def _load_price_history(ticker: str, cache_dir: Path) -> List[PricePoint]:
    """Cached prices under data/raw/<T>/prices.json, else a guarded yfinance pull."""
    path = cache_dir / ticker.upper() / "prices.json"
    if path.exists():
        try:
            from financial_analysis import market_data
            return market_data.load_prices(ticker, cache_dir)
        except Exception:
            return []
    return _fetch_price_history(ticker)


def _next_earnings_date(ticker: str, cache_dir: Path, after: date) -> Optional[date]:
    """Next earnings date on/after `after` from the cached yfinance earnings file."""
    path = cache_dir / ticker.upper() / "yf_earnings.json"
    if not path.exists():
        return None
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None
    upcoming = []
    for r in raw if isinstance(raw, list) else []:
        try:
            d = date.fromisoformat(r["date"])
        except (KeyError, TypeError, ValueError):
            continue
        if d >= after:
            upcoming.append(d)
    return min(upcoming) if upcoming else None


def _weekly(points: List[PricePoint]) -> List[PricePoint]:
    """Last point of each ISO week — keeps long histories light."""
    out = []
    for i, p in enumerate(points):
        nxt = points[i + 1] if i + 1 < len(points) else None
        if nxt is None or nxt.date.isocalendar()[:2] != p.date.isocalendar()[:2]:
            out.append(p)
    return out


def _per_share(result: DCFResult, cap: CapitalStructure) -> np.ndarray:
    return (result.simulations - cap.net_debt) / cap.shares


def _scenario_percentiles(scenarios: Tuple[Scenario, ...], results: dict,
                          cap: CapitalStructure) -> Dict[str, Tuple[float, float, float]]:
    out = {}
    for sc in scenarios:
        ps = _per_share(results[sc.key], cap)
        out[sc.key] = tuple(float(np.percentile(ps, q)) for q in (10, 50, 90))
    return out


def _money_b(v: Optional[float], decimals: int = 1) -> str:
    return "—" if v is None else f"${v / 1e9:,.{decimals}f}B"


def _fmt(v: Optional[float]) -> str:
    return "—" if v is None else f"{v:,.4g}"


def _apply_chrome(fig: go.Figure, *, x_title: str = "", y_title: str = "",
                  show_legend: bool = True) -> None:
    fig.update_layout(
        paper_bgcolor=_SURFACE, plot_bgcolor=_SURFACE,
        font=dict(family=_FONT, size=12, color=_INK_SECONDARY),
        margin=dict(l=56, r=24, t=8, b=44),
        showlegend=show_legend,
        legend=dict(orientation="h", yanchor="bottom", y=1.0, x=0,
                    font=dict(color=_INK_SECONDARY), bgcolor="rgba(0,0,0,0)"),
        hoverlabel=dict(font=dict(family=_FONT, size=12)),
    )
    fig.update_xaxes(title=dict(text=x_title, font=dict(color=_INK_MUTED)),
                     gridcolor=_GRID, gridwidth=1, linecolor=_AXIS,
                     zeroline=False, tickfont=dict(color=_INK_MUTED))
    fig.update_yaxes(title=dict(text=y_title, font=dict(color=_INK_MUTED)),
                     gridcolor=_GRID, gridwidth=1, linecolor=_AXIS,
                     zeroline=False, tickfont=dict(color=_INK_MUTED))


def _fig_html(fig: go.Figure, height: int = 380) -> str:
    return pio.to_html(fig, full_html=False, include_plotlyjs=False,
                       config={"displayModeBar": False, "responsive": True},
                       default_width="100%", default_height=height)


def _card(title: str, sub: str, body: str, wide: bool = False) -> str:
    cls = "card wide" if wide else "card"
    return (f'<section class="{cls}"><h3>{html.escape(title)}</h3>'
            f'<p class="sub">{html.escape(sub)}</p>{body}</section>')


def _note(text: str) -> str:
    return f'<p class="note">{html.escape(text)}</p>'


def _section(title: str, body: str) -> str:
    return (f'<section class="rsec"><h2 class="section-h">{html.escape(title)}</h2>'
            f'<div class="grid">{body}</div></section>')


def _chip(label: str, value: str, sub: str = "", value_color: Optional[str] = None,
          accent: bool = False) -> str:
    style = f' style="color:{value_color}"' if value_color else ""
    cls = "chip accent" if accent else "chip"
    sub_html = f'<div class="chip-sub">{html.escape(sub)}</div>' if sub else ""
    return (f'<div class="{cls}"><div class="chip-label">{html.escape(label)}</div>'
            f'<div class="chip-value"{style}>{html.escape(value)}</div>{sub_html}</div>')


# ---------------------------------------------------------------------------
# Section 1 — header band
# ---------------------------------------------------------------------------

def _header_html(company: CompanyFinancials, cap: CapitalStructure,
                 scenarios: Tuple[Scenario, ...], stats: Dict[str, Tuple[float, float, float]],
                 price: Optional[float], price_note: str, as_of: date) -> str:
    ticker = html.escape(company.ticker.upper())
    name = html.escape(company.company_name)

    chips = []
    if price is not None:
        chips.append(_chip("Last price", f"${price:,.2f}"))
        chips.append(_chip("Market cap", _money_b(price * cap.shares, 0)))
    else:
        chips.append(_chip("Last price", "—", sub="unavailable"))
    for sc in scenarios:
        p50 = stats[sc.key][1]
        chips.append(_chip(f"{sc.key.title()} P50", f"${p50:,.0f}", sub="per share"))
    if price is not None and "base" in stats:
        delta = (stats["base"][1] - price) / price
        chips.append(_chip("Base P50 vs price", f"{delta:+.0%}",
                           sub="implied up/downside",
                           value_color=_DELTA_UP if delta >= 0 else _DELTA_DOWN,
                           accent=True))

    meta = (f"As of {as_of.isoformat()} · net debt {_money_b(cap.net_debt)} · "
            f"{cap.shares / 1e9:.3f}B shares · {html.escape(price_note)}")
    return (f"<header><h1>{ticker} — Company report</h1>"
            f'<p class="meta">{name}</p><p class="meta">{meta}</p>'
            f'<div class="chips">{"".join(chips)}</div></header>')


# ---------------------------------------------------------------------------
# Section 2 — price & valuation
# ---------------------------------------------------------------------------

def _price_history_figure(points: List[PricePoint]) -> go.Figure:
    pts = _weekly(points) if len(points) > 2600 else points
    x = [p.date for p in pts]
    y = [p.close for p in pts]
    fig = go.Figure(go.Scatter(
        x=x, y=y, mode="lines", line=dict(color=_CATEGORICAL[0], width=2),
        name="Close", hovertemplate="%{x|%Y-%m-%d}: %{y:$,.2f}<extra></extra>",
    ))
    fig.add_annotation(x=x[-1], y=y[-1], text=f"${y[-1]:,.0f}", showarrow=False,
                       xanchor="left", xshift=6,
                       font=dict(size=11, color=_INK_SECONDARY))
    _apply_chrome(fig, y_title="Close ($)", show_legend=False)
    fig.update_layout(
        margin=dict(l=56, r=56, t=30, b=44),
        updatemenus=[dict(
            type="buttons", direction="right", x=1.0, xanchor="right",
            y=1.18, yanchor="top", showactive=True,
            bgcolor=_SURFACE, bordercolor=_GRID, borderwidth=1,
            font=dict(size=11, color=_INK_SECONDARY), pad=dict(l=4, r=4, t=2, b=2),
            buttons=[
                dict(label="Linear", method="relayout", args=[{"yaxis.type": "linear"}]),
                dict(label="Log", method="relayout", args=[{"yaxis.type": "log"}]),
            ],
        )],
    )
    return fig


def _valuation_zoom_figure(points: List[PricePoint],
                           scenarios: Tuple[Scenario, ...],
                           stats: Dict[str, Tuple[float, float, float]]) -> go.Figure:
    cutoff = points[-1].date - timedelta(days=183)
    recent = [p for p in points if p.date >= cutoff] or points
    x = [p.date for p in recent]
    y = [p.close for p in recent]

    fig = go.Figure()
    if "base" in stats:
        b10, _, b90 = stats["base"]
        fig.add_hrect(y0=b10, y1=b90, fillcolor=_rgba(_SCENARIO_COLORS["base"], 0.10),
                      line_width=0)
        fig.add_annotation(x=x[0], y=b90, text="Base P10–P90", showarrow=False,
                           xanchor="left", yshift=10,
                           font=dict(size=11, color=_INK_MUTED))
    for sc in scenarios:
        p50 = stats[sc.key][1]
        color = _SCENARIO_COLORS[sc.key]
        fig.add_hline(y=p50, line_color=color, line_width=2)
        fig.add_annotation(x=x[-1], y=p50, text=f"{sc.key.title()} P50 ${p50:,.0f}",
                           showarrow=False, xanchor="left", xshift=6,
                           font=dict(size=11, color=_INK_SECONDARY))
    fig.add_trace(go.Scatter(
        x=x, y=y, mode="lines", line=dict(color=_INK, width=2), name="Price",
        hovertemplate="%{x|%Y-%m-%d}: %{y:$,.2f}<extra>Price</extra>",
    ))

    lo = min(min(y), *(stats[sc.key][1] for sc in scenarios))
    hi = max(max(y), *(stats[sc.key][1] for sc in scenarios))
    if "base" in stats:
        lo, hi = min(lo, stats["base"][0]), max(hi, stats["base"][2])
    pad = (hi - lo) * 0.06 or 1.0
    _apply_chrome(fig, y_title="Price / value per share ($)", show_legend=False)
    fig.update_yaxes(range=[lo - pad, hi + pad])
    fig.update_layout(margin=dict(l=56, r=130, t=8, b=44))
    return fig


def _price_section(company: CompanyFinancials, cache_dir: Path,
                   scenarios: Tuple[Scenario, ...],
                   stats: Dict[str, Tuple[float, float, float]]) -> str:
    points = _load_price_history(company.ticker, cache_dir)
    if not points:
        return _section(SECTION_TITLES[0], _card(
            "Price history", "Daily closes",
            _note("Price history not available — no cached prices under "
                  f"data/raw/{company.ticker.upper()}/prices.json and yfinance "
                  "was unreachable."), wide=True))
    cards = _card(
        "Price history",
        f"Closes {points[0].date.isoformat()} → {points[-1].date.isoformat()}"
        + (" (weekly resolution to keep the file light)" if len(points) > 2600
           else "") + " — linear/log toggle top right",
        _fig_html(_price_history_figure(points)), wide=True)
    cards += _card(
        "Price vs valuation — 6-month view",
        "Recent closes with each scenario's P50 per-share value and the base P10–P90 band",
        _fig_html(_valuation_zoom_figure(points, scenarios, stats)), wide=True)
    return _section(SECTION_TITLES[0], cards)


# ---------------------------------------------------------------------------
# Section 3 — fundamentals
# ---------------------------------------------------------------------------

def _fundamental_annuals(company: CompanyFinancials, max_years: int = 10):
    annuals = [p for p in company.periods
               if p.form in _ANNUAL_FORMS and p.revenue is not None]
    annuals.sort(key=lambda p: p.period_end)
    return annuals[-max_years:]


def _fundamentals_figure(annuals) -> go.Figure:
    years = [str(p.period_end.year) for p in annuals]
    revenue = [p.revenue / 1e9 for p in annuals]
    fcf = [p.free_cash_flow / 1e9 if p.free_cash_flow is not None else None
           for p in annuals]
    margin = [p.fcf_margin for p in annuals]

    fig = make_subplots(rows=2, cols=1, shared_xaxes=True,
                        row_heights=[0.66, 0.34], vertical_spacing=0.10)
    fig.add_trace(go.Bar(
        x=years, y=revenue, name="Revenue", width=0.55,
        marker=dict(color=_CATEGORICAL[0]),
        hovertemplate="FY%{x}: %{y:$,.1f}B<extra>Revenue</extra>",
    ), row=1, col=1)
    fig.add_trace(go.Scatter(
        x=years, y=fcf, name="Free cash flow", mode="lines+markers",
        line=dict(color=_CATEGORICAL[1], width=2),
        marker=dict(size=8, color=_CATEGORICAL[1],
                    line=dict(color=_SURFACE, width=2)),
        hovertemplate="FY%{x}: %{y:$,.1f}B<extra>FCF</extra>",
    ), row=1, col=1)
    fig.add_trace(go.Scatter(
        x=years, y=margin, name="FCF margin", mode="lines+markers",
        showlegend=False, line=dict(color=_CATEGORICAL[1], width=2),
        marker=dict(size=8, color=_CATEGORICAL[1],
                    line=dict(color=_SURFACE, width=2)),
        hovertemplate="FY%{x}: %{y:.1%}<extra>FCF margin</extra>",
    ), row=2, col=1)

    # Selective direct labels: latest revenue bar cap, latest FCF/margin points.
    # Category axes take numeric annotation x (the category index) — a string
    # like "2025" would be date-parsed by plotly and collapse the axis.
    last_x = len(years) - 1
    fig.add_annotation(x=last_x, y=revenue[-1], text=f"${revenue[-1]:,.0f}B",
                       yshift=10, showarrow=False, row=1, col=1,
                       font=dict(size=11, color=_INK_SECONDARY))
    last_fcf = next((v for v in reversed(fcf) if v is not None), None)
    if last_fcf is not None:
        fig.add_annotation(x=last_x, y=last_fcf, text=f"FCF ${last_fcf:,.1f}B",
                           xanchor="left", xshift=8, showarrow=False, row=1, col=1,
                           font=dict(size=11, color=_INK_SECONDARY))
    last_margin = next((v for v in reversed(margin) if v is not None), None)
    if last_margin is not None:
        fig.add_annotation(x=last_x, y=last_margin, text=f"{last_margin:.1%}",
                           xanchor="left", xshift=8, showarrow=False, row=2, col=1,
                           font=dict(size=11, color=_INK_SECONDARY))

    _apply_chrome(fig)
    fig.update_layout(bargap=0.4, margin=dict(l=56, r=72, t=8, b=36),
                      hovermode="x unified")
    # Years are labels, not timestamps — stop plotly date-parsing "2018".
    fig.update_xaxes(type="category")
    fig.update_yaxes(title_text="$B", row=1, col=1)
    fig.update_yaxes(title_text="FCF margin", tickformat=".0%", row=2, col=1)
    return fig


def _fundamentals_table(annuals) -> str:
    head = ("<tr><th>FY</th><th>Revenue</th><th>Net income</th><th>OCF</th>"
            "<th>Capex</th><th>SBC</th><th>Acquisitions</th><th>FCF</th>"
            "<th>FCF margin</th></tr>")
    rows = []
    for p in annuals:
        margin = "—" if p.fcf_margin is None else f"{p.fcf_margin:.1%}"
        rows.append(
            f"<tr><td>{p.period_end.year}</td><td>{_money_b(p.revenue)}</td>"
            f"<td>{_money_b(p.net_income)}</td><td>{_money_b(p.operating_cash_flow)}</td>"
            f"<td>{_money_b(p.capex)}</td><td>{_money_b(p.sbc_total, 2)}</td>"
            f"<td>{_money_b(p.acquisitions_net_of_cash)}</td>"
            f"<td>{_money_b(p.free_cash_flow)}</td><td>{margin}</td></tr>")
    return f"<table>{head}{''.join(rows)}</table>"


def _fundamentals_section(company: CompanyFinancials) -> str:
    annuals = _fundamental_annuals(company)
    if not annuals:
        return _section(SECTION_TITLES[1], _card(
            "Annual fundamentals", "From SEC EDGAR 10-K periods",
            _note("Fundamentals not available — no annual periods with revenue "
                  "in the store."), wide=True))
    span = f"FY{annuals[0].period_end.year}–FY{annuals[-1].period_end.year}"
    cards = _card("Revenue, free cash flow & margin",
                  f"Annual (10-K) periods, {span} — margin on its own scale below",
                  _fig_html(_fundamentals_figure(annuals), height=420), wide=True)
    cards += _card("Annual fundamentals",
                   "Values as reported (capex and acquisitions are cash outflows)",
                   _fundamentals_table(annuals), wide=True)
    return _section(SECTION_TITLES[1], cards)


# ---------------------------------------------------------------------------
# Section 4 — aggregate DCF scenarios (the original four panels)
# ---------------------------------------------------------------------------

def _fan_figure(scenarios: Tuple[Scenario, ...], results: dict, base_year: int) -> go.Figure:
    fig = go.Figure()
    years = None
    for sc in scenarios:
        r = results[sc.key]
        n_years = r.fcfs.shape[0]
        years = [base_year + 1 + i for i in range(n_years)]
        p10 = np.percentile(r.fcfs, 10, axis=1) / 1e9
        p50 = np.percentile(r.fcfs, 50, axis=1) / 1e9
        p90 = np.percentile(r.fcfs, 90, axis=1) / 1e9
        color = _SCENARIO_COLORS[sc.key]
        fig.add_trace(go.Scatter(
            x=years + years[::-1], y=list(p90) + list(p10[::-1]),
            fill="toself", fillcolor=_rgba(color, 0.10),
            line=dict(width=0), hoverinfo="skip",
            legendgroup=sc.key, showlegend=False, name=f"{sc.key} P10–P90",
        ))
        fig.add_trace(go.Scatter(
            x=years, y=p50, mode="lines",
            line=dict(color=color, width=2),
            legendgroup=sc.key, name=sc.key.title(),
            hovertemplate="%{y:$.1f}B<extra>" + sc.key.title() + "</extra>",
        ))
        fig.add_annotation(x=years[-1], y=p50[-1], text=sc.key.title(),
                           xanchor="left", showarrow=False, xshift=6,
                           font=dict(size=11, color=_INK_SECONDARY))
    fig.update_layout(hovermode="x unified")
    _apply_chrome(fig, x_title="Projection year", y_title="Free cash flow ($B)")
    fig.update_layout(margin=dict(l=56, r=56, t=8, b=44))
    if years:
        fig.update_xaxes(tickmode="array", tickvals=years)
    return fig


def _distribution_figure(scenarios: Tuple[Scenario, ...], results: dict,
                         cap: CapitalStructure, price: Optional[float]) -> go.Figure:
    per_share = {sc.key: _per_share(results[sc.key], cap) for sc in scenarios}
    lo = min(float(v.min()) for v in per_share.values())
    # Clip the view (not the bins) to P99 so the extreme terminal-value tail
    # doesn't squeeze the mass of the distribution into a sliver.
    view_hi = max(float(np.percentile(v, 99)) for v in per_share.values())
    if price is not None:
        lo, view_hi = min(lo, price), max(view_hi, price * 1.1)
    size = (view_hi - lo) / 60 or 1.0
    data_hi = max(float(v.max()) for v in per_share.values())

    fig = go.Figure()
    for sc in scenarios:
        fig.add_trace(go.Histogram(
            x=per_share[sc.key], name=sc.key.title(),
            xbins=dict(start=lo, end=data_hi + size, size=size),
            marker_color=_SCENARIO_COLORS[sc.key], opacity=0.55,
            hovertemplate="%{y:,} sims at %{x:$,.0f}<extra>" + sc.key.title() + "</extra>",
        ))
    fig.update_layout(barmode="overlay")
    if price is not None:
        fig.add_vline(x=price, line_color=_INK, line_width=2)
        fig.add_annotation(x=price, yref="paper", y=0.94, yanchor="top",
                           xanchor="left", xshift=6,
                           text=f"Price ${price:,.2f}", showarrow=False,
                           font=dict(size=11, color=_INK))
    _apply_chrome(fig, x_title="Equity value per share ($)", y_title="Simulations")
    fig.update_xaxes(range=[lo, view_hi])
    return fig


def _waterfall_figure(base_result: DCFResult, cap: CapitalStructure,
                      base_year: int) -> go.Figure:
    med_pv = np.median(base_result.pv_fcfs, axis=1) / 1e9
    med_terminal = float(np.median(base_result.pv_terminal)) / 1e9
    net_debt = cap.net_debt / 1e9
    equity = float(med_pv.sum()) + med_terminal - net_debt
    per_share = equity * 1e9 / cap.shares
    terminal_pct = med_terminal / (float(med_pv.sum()) + med_terminal)

    labels = ([str(base_year + 1 + i) for i in range(len(med_pv))]
              + ["Terminal value", "Net debt", "Equity value"])
    values = list(med_pv) + [med_terminal, -net_debt, 0.0]
    measure = ["relative"] * (len(med_pv) + 2) + ["total"]

    fig = go.Figure(go.Waterfall(
        x=labels, y=values, measure=measure,
        increasing=dict(marker=dict(color=_COOL)),
        decreasing=dict(marker=dict(color=_WARM)),
        totals=dict(marker=dict(color=_INK_SECONDARY)),
        connector=dict(line=dict(color=_GRID, width=1)),
        hovertemplate="%{x}: %{delta:$,.1f}B<extra></extra>",
    ))
    fig.add_annotation(x="Terminal value",
                       y=float(med_pv.sum()) + med_terminal,
                       text=f"{terminal_pct:.0%} of gross PV",
                       showarrow=False, yshift=14,
                       font=dict(size=11, color=_INK_SECONDARY))
    fig.add_annotation(x="Equity value", y=float(med_pv.sum()) + med_terminal,
                       text=f"${per_share:,.0f}/share",
                       showarrow=False, yshift=36,
                       font=dict(size=11, color=_INK))
    _apply_chrome(fig, y_title="Present value ($B)", show_legend=False)
    fig.update_xaxes(tickangle=-45)
    fig.update_yaxes(rangemode="tozero")
    return fig


def _tornado_data(company, base: DCFAssumptions,
                  cap: CapitalStructure) -> tuple[float, list]:
    if base.fcf_margin_mean is None or base.fcf_margin_std is None:
        margin_mean, margin_std = derive_fcf_margin(company)
        base = replace(base, fcf_margin_mean=margin_mean, fcf_margin_std=margin_std)

    def p50_per_share(assumptions: DCFAssumptions) -> float:
        r = StochasticDCF(company, assumptions).run()
        return (float(np.median(r.simulations)) - cap.net_debt) / cap.shares

    base_ps = p50_per_share(base)
    rows = []
    for label, mean_field, std_field in _TORNADO_INPUTS:
        mean, std = getattr(base, mean_field), getattr(base, std_field)
        low = p50_per_share(replace(base, **{mean_field: mean - std})) - base_ps
        high = p50_per_share(replace(base, **{mean_field: mean + std})) - base_ps
        rows.append((label, low, high))
    rows.sort(key=lambda r: abs(r[1]) + abs(r[2]), reverse=True)
    return base_ps, rows


def _tornado_bars(base_ps: float, rows: list) -> go.Figure:
    """Shared tornado renderer: rows = [(label, low_delta, high_delta), ...]."""
    labels = [r[0] for r in rows]
    lows = [r[1] for r in rows]
    highs = [r[2] for r in rows]

    fig = go.Figure()
    for name, deltas, color in (("Input −1σ", lows, _COOL), ("Input +1σ", highs, _WARM)):
        fig.add_trace(go.Bar(
            y=labels, x=deltas, base=base_ps, orientation="h",
            name=name, marker_color=color, width=0.5,
            text=[f"{d:+,.0f}" for d in deltas],
            textposition="outside", cliponaxis=False,
            textfont=dict(size=11, color=_INK_SECONDARY),
            hovertemplate="%{y}: %{text} vs base P50<extra>" + name + "</extra>",
        ))
    fig.update_layout(barmode="overlay")
    fig.add_vline(x=base_ps, line_color=_AXIS, line_width=1)
    fig.add_annotation(x=base_ps, yref="paper", y=1.0, yanchor="bottom",
                       text=f"Base P50 ${base_ps:,.0f}", showarrow=False,
                       font=dict(size=11, color=_INK_MUTED))
    _apply_chrome(fig, x_title="P50 equity value per share ($)")
    span = max(abs(d) for d in lows + highs) or 1.0
    fig.update_xaxes(range=[base_ps - span * 1.35, base_ps + span * 1.35])
    fig.update_yaxes(categoryorder="array",
                     categoryarray=list(reversed(labels)), gridcolor=_SURFACE)
    return fig


def _tornado_figure(company, base: DCFAssumptions, cap: CapitalStructure) -> go.Figure:
    base_ps, rows = _tornado_data(company, base, cap)
    return _tornado_bars(base_ps, rows)


def _summary_table(scenarios: Tuple[Scenario, ...], results: dict,
                   cap: CapitalStructure, price: Optional[float]) -> str:
    head = ("<tr><th>Scenario</th><th>Growth</th><th>WACC</th><th>Terminal</th>"
            "<th>EV P50</th><th>$/sh P10</th><th>$/sh P50</th><th>$/sh P90</th>"
            + ("<th>vs price</th>" if price is not None else "") + "</tr>")
    rows = []
    for sc in scenarios:
        r = results[sc.key]
        a = sc.assumptions
        ps = _per_share(r, cap)
        p10, p50, p90 = (float(np.percentile(ps, q)) for q in (10, 50, 90))
        cells = (
            f'<td><span class="dot" style="background:{_SCENARIO_COLORS[sc.key]}"></span>'
            f"{html.escape(sc.label)}</td>"
            f"<td>{a.growth_rate_mean:.1%}</td><td>{a.wacc_mean:.1%}</td>"
            f"<td>{a.terminal_growth_mean:.1%}</td>"
            f"<td>${r.median_value / 1e9:,.0f}B</td>"
            f"<td>${p10:,.0f}</td><td>${p50:,.0f}</td><td>${p90:,.0f}</td>"
        )
        if price is not None:
            cells += f"<td>{(p50 - price) / price:+.0%}</td>"
        rows.append(f"<tr>{cells}</tr>")
    return f"<table>{head}{''.join(rows)}</table>"


def _aggregate_section(company, scenarios: Tuple[Scenario, ...], results: dict,
                       cap: CapitalStructure, price: Optional[float],
                       base_year: int) -> str:
    base = next(sc for sc in scenarios if sc.key == "base")
    panels = (
        (PANEL_TITLES[0],
         "Simulated free cash flow — P10–P90 band, median line, per scenario",
         _fig_html(_fan_figure(scenarios, results, base_year))),
        (PANEL_TITLES[1],
         "Equity value per share across simulations (EV − net debt, per share)",
         _fig_html(_distribution_figure(scenarios, results, cap, price))),
        (PANEL_TITLES[2],
         "Median PV of each projection year + terminal value − net debt (base case)",
         _fig_html(_waterfall_figure(results["base"], cap, base_year))),
        (PANEL_TITLES[3],
         "P50 per-share impact of shifting one input ±1σ, others held at their means (base case)",
         _fig_html(_tornado_figure(company, base.assumptions, cap))),
    )
    cards = "".join(_card(title, sub, body) for title, sub, body in panels)
    cards += _card(
        "Scenario summary",
        "Per-share equity values are (EV − net debt) / shares at each percentile",
        _summary_table(scenarios, results, cap, price), wide=True)
    return _section(SECTION_TITLES[2], cards)


# ---------------------------------------------------------------------------
# Section 5 — driver-based model (conditional)
# ---------------------------------------------------------------------------

def _load_driver_build(ticker: str):
    """The build() callable from analysis/<TICKER>/driver_model.py, or None."""
    try:
        module = importlib.import_module(f"analysis.{ticker.upper()}.driver_model")
    except Exception:
        return None
    build = getattr(module, "build", None)
    return build if callable(build) else None


def _segment_history_figure(segments: Sequence[SegmentSpec],
                            segment_history: dict) -> go.Figure:
    # Fixed slot order follows the segments declaration; history-only names append.
    names = [s.name for s in segments if s.name in segment_history]
    names += [n for n in segment_history if n not in names]
    years = sorted({str(y) for hist in segment_history.values() for y in hist})

    fig = go.Figure()
    for i, name in enumerate(names):
        hist = {str(y): v for y, v in segment_history[name].items()}
        fig.add_trace(go.Bar(
            x=years, y=[hist.get(y, 0) / 1e9 for y in years],
            name=name, width=0.55,
            marker=dict(color=_CATEGORICAL[i % len(_CATEGORICAL)],
                        line=dict(color=_SURFACE, width=2)),
            hovertemplate="FY%{x}: %{y:$,.1f}B<extra>" + name + "</extra>",
        ))
    fig.update_layout(barmode="stack", bargap=0.4)
    _apply_chrome(fig, y_title="Segment revenue ($B)")
    # Years are labels, not timestamps — stop plotly date-parsing "2023".
    fig.update_xaxes(type="category")
    return fig


def _driver_table(model: DriverModel) -> str:
    head = ("<tr><th>Driver</th><th>Type</th><th>Dist</th><th>Mean</th>"
            "<th>Std</th><th>Bounds</th><th>Rationale</th></tr>")
    rows = []
    for d in model.drivers:
        badge = ('<span class="badge stoch">stochastic</span>' if d.stochastic
                 else '<span class="badge det">deterministic</span>')
        bounds = "—"
        if d.lo is not None or d.hi is not None:
            bounds = f"[{_fmt(d.lo)}, {_fmt(d.hi)}]"
        rows.append(
            f"<tr><td>{html.escape(d.name)}</td><td>{badge}</td>"
            f"<td>{html.escape(d.dist)}</td><td>{_fmt(d.mean)}</td>"
            f"<td>{_fmt(d.std) if d.stochastic else '—'}</td><td>{bounds}</td>"
            f'<td class="wrap">{html.escape(d.rationale or "—")}</td></tr>')
    return f"<table>{head}{''.join(rows)}</table>"


def _corr_heatmap_figure(model: DriverModel) -> go.Figure:
    names = [d.name for d in model.stochastic_drivers]
    mat = model.correlation_matrix()
    # Annotate off-diagonal cells only — the trivial 1.00 diagonal is the
    # darkest fill, where ink-on-dark text would be unreadable.
    labels = [["" if i == j else f"{mat[i, j]:.2f}" for j in range(len(names))]
              for i in range(len(names))]
    # Reverse rows so the diagonal runs top-left → bottom-right.
    fig = go.Figure(go.Heatmap(
        z=mat[::-1], x=names, y=names[::-1],
        zmin=-1, zmax=1, colorscale=_DIVERGING_SCALE,
        text=labels[::-1], texttemplate="%{text}",
        textfont=dict(size=10, color=_INK_SECONDARY),
        showscale=False, xgap=2, ygap=2,
        hovertemplate="%{y} × %{x}: %{z:.2f}<extra></extra>",
    ))
    _apply_chrome(fig, show_legend=False)
    fig.update_xaxes(tickangle=-35, showgrid=False)
    fig.update_yaxes(showgrid=False)
    return fig


def _driver_compare_figure(driver_stats: Tuple[float, float, float],
                           base_stats: Tuple[float, float, float],
                           price: Optional[float]) -> go.Figure:
    rows = (("Aggregate base", base_stats, _SCENARIO_COLORS["base"], 18),
            # Top row labels sit below the marks so they never collide with
            # the price annotation above the plot.
            ("Driver model", driver_stats, _DRIVER_COLOR, -24))
    fig = go.Figure()
    for label, (p10, p50, p90), color, label_shift in rows:
        fig.add_trace(go.Scatter(
            x=[p10, p90], y=[label, label], mode="lines",
            line=dict(color=_rgba(color, 0.35), width=6),
            hoverinfo="skip", showlegend=False,
        ))
        fig.add_trace(go.Scatter(
            x=[p10, p50, p90], y=[label] * 3, mode="markers", name=label,
            marker=dict(size=[9, 13, 9], color=color,
                        line=dict(color=_SURFACE, width=2)),
            hovertemplate="%{x:$,.0f}<extra>" + label + "</extra>",
        ))
        for v, tag in ((p10, "P10"), (p50, "P50"), (p90, "P90")):
            fig.add_annotation(x=v, y=label, text=f"{tag} ${v:,.0f}",
                               showarrow=False, yshift=label_shift,
                               font=dict(size=11, color=_INK_SECONDARY))
    if price is not None:
        fig.add_vline(x=price, line_color=_INK, line_width=2)
        fig.add_annotation(x=price, yref="paper", y=1.0, yanchor="bottom",
                           text=f"Price ${price:,.2f}", showarrow=False,
                           font=dict(size=11, color=_INK))
    _apply_chrome(fig, x_title="Equity value per share ($)", show_legend=False)
    # Room for the price annotation that sits above the plot area.
    fig.update_layout(margin=dict(l=56, r=24, t=28, b=44))
    fig.update_yaxes(showgrid=False)
    return fig


def _driver_tornado_figure(result: DriverDCFResult) -> go.Figure:
    rows = [(name, attr.p50_minus_1sd, attr.p50_plus_1sd)
            for name, attr in result.attribution.items()]
    rows.sort(key=lambda r: abs(r[1]) + abs(r[2]), reverse=True)
    return _tornado_bars(result.per_share_p50, rows)


def _driver_section(ticker: str, base_stats: Optional[Tuple[float, float, float]],
                    price: Optional[float]) -> str:
    build = _load_driver_build(ticker)
    if build is None:
        return _section(SECTION_TITLES[3], _card(
            "Driver model", f"analysis/{ticker.upper()}/driver_model.py",
            _note("Driver model not available — no "
                  f"analysis/{ticker.upper()}/driver_model.py exposing build()."),
            wide=True))
    try:
        spec = build()
        model: DriverModel = spec["model"]
        segments: List[SegmentSpec] = list(spec["segments"])
        net_debt = float(spec["net_debt"])
        shares = float(spec["shares"])
    except Exception as e:  # malformed contract → note, never a crash
        return _section(SECTION_TITLES[3], _card(
            "Driver model", f"analysis/{ticker.upper()}/driver_model.py",
            _note(f"Driver model could not be loaded: {e}"), wide=True))

    segment_history = spec.get("segment_history")

    try:
        engine = DriverDCF(model, segments, net_debt=net_debt, shares=shares)
        result = engine.run(DRIVER_SIMS, seed=DRIVER_SEED)
    except Exception as e:
        return _section(SECTION_TITLES[3], _card(
            "Driver model", f"analysis/{ticker.upper()}/driver_model.py",
            _note(f"Driver model run failed: {e}"), wide=True))

    cards = ""
    if segment_history:
        cards += _card(
            "Segment revenue history",
            "Reported revenue by segment, stacked per fiscal year",
            _fig_html(_segment_history_figure(segments, segment_history)), wide=True)
    else:
        cards += _card("Segment revenue history", "Reported revenue by segment",
                       _note("Segment history not available — the driver model "
                             "declares no segment_history."), wide=True)

    cards += _card(
        "Driver assumptions",
        "Every model input, its distribution, and the rationale reviewers signed off on",
        _driver_table(model), wide=True)

    stoch = model.stochastic_drivers
    if len(stoch) >= 2:
        cards += _card(
            "Driver correlation matrix",
            "Pairwise correlations of the stochastic drivers (sampled jointly via Cholesky)",
            _fig_html(_corr_heatmap_figure(model)))
    else:
        cards += _card("Driver correlation matrix", "Stochastic drivers",
                       _note("Fewer than two stochastic drivers — no correlation "
                             "structure to show."))

    driver_stats = (result.per_share_p10, result.per_share_p50, result.per_share_p90)
    if base_stats is not None:
        cards += _card(
            "Driver model vs aggregate base",
            "P10–P90 per-share range with the P50 marked, both engines, same capital structure basis",
            _fig_html(_driver_compare_figure(driver_stats, base_stats, price)))
    else:
        cards += _card("Driver model vs aggregate base", "Per-share percentiles",
                       _note("Aggregate base scenario unavailable for comparison."))

    cards += _card(
        "Driver attribution tornado",
        "P50 per-share delta when one driver's mean shifts ±1σ — same seed, correlations intact",
        _fig_html(_driver_tornado_figure(result)), wide=True)

    engine_meta = (f"DriverDCF: {DRIVER_SIMS:,} simulations · seed {DRIVER_SEED} · "
                   f"{engine.projection_years}y horizon · net debt {_money_b(net_debt)} · "
                   f"{shares / 1e9:.3f}B shares · terminal value "
                   f"{result.tv_ev_share_at_means:.0%} of EV at driver means")
    if hasattr(result, "tg_pinned_fraction"):
        engine_meta += (f" · terminal growth pinned at WACC−2% on "
                        f"{result.tg_pinned_fraction:.1%} of paths")
    cards += f'<p class="sub wide-note">{html.escape(engine_meta)}</p>'
    return _section(SECTION_TITLES[3], cards)


# ---------------------------------------------------------------------------
# Section 6 — options market (conditional)
# ---------------------------------------------------------------------------

def _expiry_label(d: date) -> str:
    return d.strftime("%d %b %y")


def _iv_term_figure(metrics, next_earnings: Optional[date]) -> Optional[go.Figure]:
    ems = [em for em in metrics.expiries if em.atm_iv is not None]
    if not ems:
        return None
    x = [em.expiry for em in ems]
    y = [em.atm_iv * 100 for em in ems]
    fig = go.Figure(go.Scatter(
        x=x, y=y, mode="lines+markers", name="ATM IV",
        line=dict(color=_CATEGORICAL[0], width=2),
        marker=dict(size=8, color=_CATEGORICAL[0], line=dict(color=_SURFACE, width=2)),
        hovertemplate="%{x|%Y-%m-%d}: %{y:.1f}%<extra>ATM IV</extra>",
    ))
    if next_earnings is not None:
        fig.add_vline(x=next_earnings, line_color=_AXIS, line_width=1)
        # Anchor at the plot bottom so it never collides with the hump label.
        fig.add_annotation(x=next_earnings, yref="paper", y=0.02, yanchor="bottom",
                           xanchor="left", xshift=6,
                           text=f"Earnings {next_earnings.isoformat()}",
                           showarrow=False, font=dict(size=11, color=_INK_MUTED))
        hump = next((em for em in ems if em.expiry >= next_earnings), None)
        if hump is not None:
            fig.add_annotation(
                x=hump.expiry, y=hump.atm_iv * 100,
                text=f"Earnings hump — {hump.atm_iv * 100:.1f}%",
                showarrow=True, arrowhead=2, arrowcolor=_INK_SECONDARY,
                ax=40, ay=-32, font=dict(size=11, color=_INK_SECONDARY))
    _apply_chrome(fig, x_title="Expiry", y_title="ATM implied vol (%)",
                  show_legend=False)
    return fig


def _implied_move_figure(metrics) -> Optional[go.Figure]:
    ems = [em for em in metrics.expiries if em.implied_move is not None]
    if not ems:
        return None
    x = [_expiry_label(em.expiry) for em in ems]
    y = [em.implied_move * 100 for em in ems]
    # Direct labels only while they stay sparse; tooltips carry dense chains.
    labels = [f"{v:.1f}%" for v in y] if len(y) <= 8 else None
    fig = go.Figure(go.Bar(
        x=x, y=y, width=0.55, marker=dict(color=_CATEGORICAL[0]),
        text=labels, textposition="outside", cliponaxis=False,
        textfont=dict(size=10, color=_INK_SECONDARY),
        hovertemplate="%{x}: ±%{y:.1f}%<extra>Implied move</extra>",
    ))
    _apply_chrome(fig, x_title="Expiry", y_title="Implied move (±%)",
                  show_legend=False)
    fig.update_layout(bargap=0.4)
    fig.update_xaxes(tickangle=-35)
    return fig


def _skew_figure(metrics) -> Optional[go.Figure]:
    ems = [em for em in metrics.expiries if em.skew is not None]
    if not ems:
        return None
    x = [_expiry_label(em.expiry) for em in ems]
    y = [em.skew * 100 for em in ems]
    # Direct labels only while they stay sparse; tooltips carry dense chains.
    labels = [f"{v:+.1f}" for v in y] if len(y) <= 8 else None
    fig = go.Figure(go.Bar(
        x=x, y=y, width=0.55,
        marker=dict(color=[_COOL if v >= 0 else _WARM for v in y]),
        text=labels, textposition="outside", cliponaxis=False,
        textfont=dict(size=10, color=_INK_SECONDARY),
        hovertemplate="%{x}: %{y:+.1f} IV pts<extra>Skew</extra>",
    ))
    fig.add_hline(y=0, line_color=_AXIS, line_width=1)
    _apply_chrome(fig, x_title="Expiry",
                  y_title="Skew (90% put IV − 110% call IV, pts)",
                  show_legend=False)
    fig.update_layout(bargap=0.4)
    fig.update_xaxes(tickangle=-35)
    return fig


def _oi_heatmap_figure(snapshot, spot: Optional[float]) -> Optional[go.Figure]:
    expiries = sorted({q.expiry for q in snapshot.quotes})[:4]
    if not expiries:
        return None
    lo = spot * 0.75 if spot else None
    hi = spot * 1.25 if spot else None
    oi: Dict[date, Dict[float, int]] = {e: {} for e in expiries}
    for q in snapshot.quotes:
        if q.expiry not in oi or not q.open_interest:
            continue
        if lo is not None and not (lo <= q.strike <= hi):
            continue
        oi[q.expiry][q.strike] = oi[q.expiry].get(q.strike, 0) + q.open_interest
    strikes = set()
    for row in oi.values():
        strikes.update(row)
    if not strikes:
        return None
    xs = sorted(strikes)
    z = [[oi[e].get(s) for s in xs] for e in expiries]
    fig = go.Figure(go.Heatmap(
        z=z, x=xs, y=[_expiry_label(e) for e in expiries],
        colorscale=_SEQ_BLUES, xgap=2, ygap=2, zmin=0,
        colorbar=dict(title=dict(text="OI", font=dict(color=_INK_MUTED)),
                      thickness=12, outlinewidth=0,
                      tickfont=dict(color=_INK_MUTED)),
        hovertemplate="%{y} · $%{x}: %{z:,} contracts<extra></extra>",
    ))
    if spot is not None:
        fig.add_vline(x=spot, line_color=_INK, line_width=2)
        fig.add_annotation(x=spot, yref="paper", y=1.0, yanchor="bottom",
                           text=f"Spot ${spot:,.2f}", showarrow=False,
                           font=dict(size=11, color=_INK))
    _apply_chrome(fig, x_title="Strike ($)", show_legend=False)
    # Room for the spot annotation that sits above the plot area.
    fig.update_layout(margin=dict(l=56, r=24, t=28, b=44))
    fig.update_xaxes(showgrid=False)
    fig.update_yaxes(showgrid=False, autorange="reversed")
    return fig


def _options_chips(metrics) -> str:
    total_vol = sum(em.call_volume + em.put_volume for em in metrics.expiries)
    total_oi = sum(em.call_open_interest + em.put_open_interest
                   for em in metrics.expiries)
    ratio = (lambda v: "—" if v is None else f"{v:.2f}")
    chips = (
        _chip("Snapshot", metrics.snapshot_date.isoformat())
        + _chip("Spot", "—" if metrics.spot_price is None
                else f"${metrics.spot_price:,.2f}")
        + _chip("P/C volume", ratio(metrics.put_call_volume_ratio), sub="chain-level")
        + _chip("P/C open interest", ratio(metrics.put_call_oi_ratio), sub="chain-level")
        + _chip("Total volume", f"{total_vol:,}", sub="contracts")
        + _chip("Total open interest", f"{total_oi:,}", sub="contracts")
    )
    return f'<div class="chips">{chips}</div>'


def _chain_table(snapshot, expiry: date, atm_strike: Optional[float],
                 width: int = 10) -> str:
    quotes = [q for q in snapshot.quotes if q.expiry == expiry]
    strikes = sorted({q.strike for q in quotes})
    if not strikes:
        return _note(f"No quotes for expiry {expiry.isoformat()}.")
    if atm_strike is None:
        atm_strike = strikes[len(strikes) // 2]
    idx = strikes.index(min(strikes, key=lambda s: abs(s - atm_strike)))
    window = strikes[max(0, idx - width):idx + width + 1]
    calls = {q.strike: q for q in quotes if q.option_type == "call"}
    puts = {q.strike: q for q in quotes if q.option_type == "put"}

    def px(v: Optional[float]) -> str:
        return "—" if v is None else f"{v:,.2f}"

    def iv(v: Optional[float]) -> str:
        return "—" if v is None else f"{v:.0%}"

    def oi(v: Optional[int]) -> str:
        return "—" if v is None else f"{v:,}"

    head = ('<tr><th colspan="5">Calls</th><th>Strike</th><th colspan="5">Puts</th></tr>'
            "<tr><th>Bid</th><th>Ask</th><th>Last</th><th>IV</th><th>OI</th><th></th>"
            "<th>Bid</th><th>Ask</th><th>Last</th><th>IV</th><th>OI</th></tr>")
    rows = []
    for s in window:
        c, p = calls.get(s), puts.get(s)
        cls = ' class="atm"' if s == strikes[idx] else ""
        rows.append(
            f"<tr{cls}>"
            f"<td>{px(c.bid if c else None)}</td><td>{px(c.ask if c else None)}</td>"
            f"<td>{px(c.last_price if c else None)}</td>"
            f"<td>{iv(c.implied_volatility if c else None)}</td>"
            f"<td>{oi(c.open_interest if c else None)}</td>"
            f"<td><strong>{s:g}</strong></td>"
            f"<td>{px(p.bid if p else None)}</td><td>{px(p.ask if p else None)}</td>"
            f"<td>{px(p.last_price if p else None)}</td>"
            f"<td>{iv(p.implied_volatility if p else None)}</td>"
            f"<td>{oi(p.open_interest if p else None)}</td></tr>")
    return f"<table>{head}{''.join(rows)}</table>"


def _options_section(ticker: str, cache_dir: Path, price: Optional[float]) -> str:
    from financial_analysis.options import compute_chain_metrics, load_snapshot
    try:
        snapshot = load_snapshot(ticker, cache_dir)
    except FileNotFoundError:
        return _section(SECTION_TITLES[4], _card(
            "Option chain", f"data/raw/{ticker.upper()}/options/",
            _note("No option-chain snapshot available — run "
                  f"financial_analysis.options.fetch_option_chain('{ticker.upper()}', ...) "
                  "to capture one."), wide=True))
    if not snapshot.quotes:
        return _section(SECTION_TITLES[4], _card(
            "Option chain", f"Snapshot {snapshot.snapshot_date.isoformat()}",
            _note("The snapshot contains no quotes — no listed options found."),
            wide=True))

    metrics = compute_chain_metrics(snapshot)
    next_earnings = _next_earnings_date(ticker, cache_dir, snapshot.snapshot_date)
    spot = snapshot.spot_price if snapshot.spot_price is not None else price

    cards = f'<div class="wide-chips">{_options_chips(metrics)}</div>'

    def chart_card(title: str, sub: str, fig: Optional[go.Figure],
                   missing: str, wide: bool = False) -> str:
        body = _fig_html(fig) if fig is not None else _note(missing)
        return _card(title, sub, body, wide=wide)

    cards += chart_card(
        "ATM implied-volatility term structure",
        "Average of ATM call/put IV per expiry — the near-dated bump is the earnings premium",
        _iv_term_figure(metrics, next_earnings),
        "ATM IV unavailable — no expiry has a solvable ATM implied vol.")
    cards += chart_card(
        "Implied move by expiry",
        "ATM straddle mid ÷ spot — what the market prices for a move by each expiry",
        _implied_move_figure(metrics),
        "Implied move unavailable — no expiry has both ATM legs quoted.")
    cards += chart_card(
        "Volatility skew by expiry",
        "IV of the ~90%-moneyness put minus the ~110%-moneyness call (positive = downside bid)",
        _skew_figure(metrics),
        "Skew unavailable — wings not quoted with solvable IVs.")
    cards += chart_card(
        "Open interest by strike",
        "Calls + puts open interest, four nearest expiries, strikes within ±25% of spot",
        _oi_heatmap_figure(snapshot, spot),
        "Open interest unavailable — no contracts carry open interest near spot.")

    expiries = sorted({q.expiry for q in snapshot.quotes})
    target = None
    target_note = ""
    if next_earnings is not None:
        target = next((e for e in expiries if e >= next_earnings), None)
        target_note = f"first expiry after earnings on {next_earnings.isoformat()}"
    if target is None:
        target = expiries[0]
        target_note = "nearest expiry (no upcoming earnings date on file)"
    em = next((m for m in metrics.expiries if m.expiry == target), None)
    atm = em.atm_strike if em is not None else None
    cards += _card(
        "Chain detail — ATM ±10 strikes",
        f"Expiry {target.isoformat()} ({target_note}); ATM row highlighted",
        _chain_table(snapshot, target, atm), wide=True)

    return _section(SECTION_TITLES[4], cards)


# ---------------------------------------------------------------------------
# Footer
# ---------------------------------------------------------------------------

def _footer_html(company: CompanyFinancials, base_year: int, n_sims: int,
                 horizon: int, seed, cache_dir: Path, price_note: str) -> str:
    ticker = company.ticker.upper()
    n_annual = len([p for p in company.periods if p.form in _ANNUAL_FORMS])
    prices_path = cache_dir / ticker / "prices.json"
    prices_src = (f"cached data/raw/{ticker}/prices.json" if prices_path.exists()
                  else "yfinance at run time")
    from financial_analysis.options import list_snapshots
    snaps = list_snapshots(ticker, cache_dir)
    options_src = (f"snapshot {snaps[-1].isoformat()}" if snaps
                   else "no snapshot on disk")
    lines = (
        f"Data: SEC EDGAR XBRL 10-K/10-Q → data/processed/{ticker}.json "
        f"({n_annual} annual periods, latest FY{base_year}) · prices: {prices_src} · "
        f"options: {options_src} · {html.escape(price_note)}",
        f"Engines: aggregate StochasticDCF {n_sims:,} sims × {horizon}y (seed {seed}, "
        f"scenarios from financial_analysis/scenarios.py) · driver model "
        f"{DRIVER_SIMS:,} sims (seed {DRIVER_SEED}) when analysis/{ticker}/driver_model.py exists",
        "Generated by scripts/visualize_dcf.py — seeded runs, reproducible",
    )
    return "<footer>" + "".join(f"<p>{line}</p>" for line in lines) + "</footer>"


# ---------------------------------------------------------------------------
# Page assembly
# ---------------------------------------------------------------------------

def build_report_html(company, scenarios: Tuple[Scenario, ...], results: dict,
                      cap: CapitalStructure, price: Optional[float],
                      price_note: str,
                      cache_dir: Path = _DEFAULT_CACHE_DIR) -> str:
    base_year = latest_annual(company).period_end.year
    base = next(sc for sc in scenarios if sc.key == "base")
    n_sims = base.assumptions.n_simulations
    horizon = base.assumptions.projection_years
    stats = _scenario_percentiles(scenarios, results, cap)
    as_of = date.today()

    header = _header_html(company, cap, scenarios, stats, price, price_note, as_of)
    sections = (
        _price_section(company, cache_dir, scenarios, stats)
        + _fundamentals_section(company)
        + _aggregate_section(company, scenarios, results, cap, price, base_year)
        + _driver_section(company.ticker, stats.get("base"), price)
        + _options_section(company.ticker, cache_dir, price)
    )
    footer = _footer_html(company, base_year, n_sims, horizon,
                          base.assumptions.seed, cache_dir, price_note)

    ticker = html.escape(company.ticker.upper())
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{ticker} — Company report</title>
<style>
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; padding: 24px; background: {_PAGE}; color: {_INK};
         font-family: {_FONT}; }}
  header {{ margin: 0 0 12px 4px; }}
  h1 {{ font-size: 22px; margin: 0 0 4px; }}
  .section-h {{ font-size: 17px; margin: 28px 4px 12px; color: {_INK};
                border-bottom: 1px solid {_GRID}; padding-bottom: 6px; }}
  h3 {{ font-size: 15px; margin: 0 0 2px; color: {_INK}; }}
  .meta {{ color: {_INK_SECONDARY}; font-size: 13px; margin: 0; }}
  .sub {{ color: {_INK_MUTED}; font-size: 12px; margin: 0 0 8px; }}
  .grid {{ display: grid; grid-template-columns: repeat(2, minmax(0, 1fr));
           gap: 16px; }}
  @media (max-width: 1100px) {{ .grid {{ grid-template-columns: minmax(0, 1fr); }} }}
  .card {{ background: {_SURFACE}; border: 1px solid rgba(11,11,11,0.10);
           border-radius: 8px; padding: 16px; overflow-x: auto; }}
  .card.wide, .wide-chips, .wide-note {{ grid-column: 1 / -1; }}
  .chips {{ display: flex; flex-wrap: wrap; gap: 12px; margin: 12px 0 4px; }}
  .chip {{ background: {_SURFACE}; border: 1px solid rgba(11,11,11,0.10);
           border-radius: 8px; padding: 10px 14px; min-width: 116px; }}
  .chip.accent {{ border-color: {_AXIS}; }}
  .chip-label {{ font-size: 11px; color: {_INK_MUTED}; }}
  .chip-value {{ font-size: 18px; font-weight: 600; color: {_INK}; }}
  .chip-sub {{ font-size: 11px; color: {_INK_MUTED}; }}
  .badge {{ font-size: 11px; border-radius: 999px; padding: 1px 8px;
            border: 1px solid; white-space: nowrap; }}
  .badge.stoch {{ color: #1c5cab; border-color: #86b6ef; background: #cde2fb; }}
  .badge.det {{ color: {_INK_SECONDARY}; border-color: {_AXIS}; background: {_NEUTRAL}; }}
  .note {{ color: {_INK_SECONDARY}; font-size: 13px; background: {_NEUTRAL};
           padding: 10px 12px; border-radius: 6px; margin: 4px 0; }}
  table {{ border-collapse: collapse; font-size: 13px; width: 100%; }}
  th {{ text-align: left; color: {_INK_MUTED}; font-weight: 600; }}
  th, td {{ padding: 6px 10px; border-bottom: 1px solid {_GRID};
            font-variant-numeric: tabular-nums; white-space: nowrap; }}
  td.wrap {{ white-space: normal; min-width: 260px; font-variant-numeric: normal; }}
  tr.atm td {{ background: rgba(42,120,214,0.10); }}
  .dot {{ display: inline-block; width: 10px; height: 10px; border-radius: 50%;
          margin-right: 6px; vertical-align: baseline; }}
  footer {{ color: {_INK_MUTED}; font-size: 12px; margin: 24px 4px 0; }}
  footer p {{ margin: 2px 0; }}
</style>
<script>{get_plotlyjs()}</script>
</head>
<body>
{header}
{sections}
{footer}
<script>window.addEventListener("load", () => window.dispatchEvent(new Event("resize")));</script>
</body>
</html>"""


def write_report(company, out_path: Path, price: Optional[float],
                 price_note: str, cache_dir: Path = _DEFAULT_CACHE_DIR) -> Path:
    scenarios = scenarios_for(company.ticker)
    cap = capital_structure_for(company)
    results = {sc.key: StochasticDCF(company, sc.assumptions).run() for sc in scenarios}
    report = build_report_html(company, scenarios, results, cap, price, price_note,
                               cache_dir=cache_dir)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(report, encoding="utf-8")
    return out_path


def main() -> None:
    parser = argparse.ArgumentParser(description="Interactive single-page company report (HTML).")
    parser.add_argument("ticker", help="Stock ticker symbol (e.g. IBM)")
    parser.add_argument("--price", type=float, default=None,
                        help="Current share price (skips the yfinance lookup)")
    parser.add_argument("--out", type=Path, default=None,
                        help="Output path (default: data/processed/{TICKER}_dcf.html)")
    args = parser.parse_args()

    ticker = args.ticker.upper()
    try:
        company = store.load(ticker)
    except FileNotFoundError:
        print(f"No saved data for {ticker}. Run: python -m scripts.import_financials {ticker}")
        sys.exit(1)

    if args.price is not None:
        price, price_note = args.price, f"price ${args.price:,.2f} (--price override)"
    else:
        price = _fetch_price(ticker)
        price_note = (f"last close ${price:,.2f} (yfinance)" if price is not None
                      else "current price unavailable (offline?) — price marker omitted")

    out_path = args.out or Path("data/processed") / f"{ticker}_dcf.html"
    try:
        write_report(company, out_path, price, price_note)
    except ValueError as e:
        print(f"Cannot visualize {ticker}: {e}")
        sys.exit(1)
    print(f"Report written: {out_path}")


if __name__ == "__main__":
    main()
