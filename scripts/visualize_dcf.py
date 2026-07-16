"""CLI: interactive Monte Carlo DCF report for a ticker.

Runs the bear/base/bull presets from financial_analysis/scenarios.py against
data/processed/{TICKER}.json and writes a self-contained HTML report (plotly.js
inlined — no CDN or network access needed to view it) with four panels:
FCF projection fan, per-share value distribution, median-case PV build-up
waterfall, and a ±1σ sensitivity tornado.

Usage:
    python -m scripts.visualize_dcf IBM
    python -m scripts.visualize_dcf IBM --price 195.40   # skip yfinance lookup
    python -m scripts.visualize_dcf IBM --out report.html
"""
import argparse
import html
import sys
from dataclasses import replace
from pathlib import Path
from typing import Optional, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import numpy as np
import plotly.graph_objects as go
import plotly.io as pio
from plotly.offline import get_plotlyjs

from financial_analysis import store
from financial_analysis.dcf import DCFAssumptions, DCFResult, StochasticDCF, derive_fcf_margin
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

# Chart chrome (dataviz reference palette, light mode)
_SURFACE = "#fcfcfb"
_PAGE = "#f9f9f7"
_INK = "#0b0b0b"
_INK_SECONDARY = "#52514e"
_INK_MUTED = "#898781"
_GRID = "#e1e0d9"
_AXIS = "#c3c2b7"
_FONT = 'system-ui, -apple-system, "Segoe UI", sans-serif'

# Categorical slots 1-3, fixed order (validated: worst adjacent CVD dE 47.2)
_SCENARIO_COLORS = {"bear": "#2a78d6", "base": "#1baf7a", "bull": "#eda100"}
# Diverging pair for polarity (waterfall contributions, tornado shifts)
_COOL = "#2a78d6"
_WARM = "#e34948"

_TORNADO_INPUTS = (
    ("Revenue growth", "growth_rate_mean", "growth_rate_std"),
    ("WACC", "wacc_mean", "wacc_std"),
    ("Terminal growth", "terminal_growth_mean", "terminal_growth_std"),
    ("FCF margin", "fcf_margin_mean", "fcf_margin_std"),
)


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


def _per_share(result: DCFResult, cap: CapitalStructure) -> np.ndarray:
    return (result.simulations - cap.net_debt) / cap.shares


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


def _tornado_figure(company, base: DCFAssumptions, cap: CapitalStructure) -> go.Figure:
    base_ps, rows = _tornado_data(company, base, cap)
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
    span = max(abs(d) for d in lows + highs)
    fig.update_xaxes(range=[base_ps - span * 1.35, base_ps + span * 1.35])
    fig.update_yaxes(categoryorder="array",
                     categoryarray=list(reversed(labels)), gridcolor=_SURFACE)
    return fig


def _fig_html(fig: go.Figure) -> str:
    return pio.to_html(fig, full_html=False, include_plotlyjs=False,
                       config={"displayModeBar": False, "responsive": True},
                       default_width="100%", default_height=380)


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


def build_report_html(company, scenarios: Tuple[Scenario, ...], results: dict,
                      cap: CapitalStructure, price: Optional[float],
                      price_note: str) -> str:
    base_year = latest_annual(company).period_end.year
    base = next(sc for sc in scenarios if sc.key == "base")
    n_sims = base.assumptions.n_simulations
    horizon = base.assumptions.projection_years

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
    cards = "".join(
        f'<section class="card"><h2>{html.escape(title)}</h2>'
        f'<p class="sub">{html.escape(sub)}</p>{body}</section>'
        for title, sub, body in panels
    )

    ticker = html.escape(company.ticker.upper())
    name = html.escape(company.company_name)
    meta = (f"{n_sims:,} simulations · {horizon}y horizon · FY{base_year} base · "
            f"net debt ${cap.net_debt / 1e9:.1f}B · {cap.shares / 1e9:.3f}B shares · "
            f"{html.escape(price_note)}")

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{ticker} — Monte Carlo DCF</title>
<style>
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; padding: 24px; background: {_PAGE}; color: {_INK};
         font-family: {_FONT}; }}
  header {{ margin: 0 0 20px 4px; }}
  h1 {{ font-size: 22px; margin: 0 0 4px; }}
  h2 {{ font-size: 15px; margin: 0 0 2px; color: {_INK}; }}
  .meta {{ color: {_INK_SECONDARY}; font-size: 13px; margin: 0; }}
  .sub {{ color: {_INK_MUTED}; font-size: 12px; margin: 0 0 8px; }}
  .grid {{ display: grid; grid-template-columns: repeat(2, minmax(0, 1fr));
           gap: 16px; }}
  @media (max-width: 1100px) {{ .grid {{ grid-template-columns: minmax(0, 1fr); }} }}
  .card {{ background: {_SURFACE}; border: 1px solid rgba(11,11,11,0.10);
           border-radius: 8px; padding: 16px; overflow-x: auto; }}
  .card.wide {{ grid-column: 1 / -1; }}
  table {{ border-collapse: collapse; font-size: 13px; width: 100%; }}
  th {{ text-align: left; color: {_INK_MUTED}; font-weight: 600; }}
  th, td {{ padding: 6px 10px; border-bottom: 1px solid {_GRID};
            font-variant-numeric: tabular-nums; white-space: nowrap; }}
  .dot {{ display: inline-block; width: 10px; height: 10px; border-radius: 50%;
          margin-right: 6px; vertical-align: baseline; }}
  footer {{ color: {_INK_MUTED}; font-size: 12px; margin: 16px 4px 0; }}
</style>
<script>{get_plotlyjs()}</script>
</head>
<body>
<header>
  <h1>{ticker} — Monte Carlo DCF</h1>
  <p class="meta">{name}</p>
  <p class="meta">{meta}</p>
</header>
<div class="grid">
{cards}
<section class="card wide"><h2>Scenario summary</h2>
<p class="sub">Per-share equity values are (EV − net debt) / shares at each percentile</p>
{_summary_table(scenarios, results, cap, price)}</section>
</div>
<footer>Assumptions: financial_analysis/scenarios.py · data: data/processed/{ticker}.json (SEC EDGAR 10-K periods) · seeded runs, reproducible</footer>
<script>window.addEventListener("load", () => window.dispatchEvent(new Event("resize")));</script>
</body>
</html>"""


def write_report(company, out_path: Path, price: Optional[float],
                 price_note: str) -> Path:
    scenarios = scenarios_for(company.ticker)
    cap = capital_structure_for(company)
    results = {sc.key: StochasticDCF(company, sc.assumptions).run() for sc in scenarios}
    report = build_report_html(company, scenarios, results, cap, price, price_note)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(report, encoding="utf-8")
    return out_path


def main() -> None:
    parser = argparse.ArgumentParser(description="Interactive Monte Carlo DCF HTML report.")
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
