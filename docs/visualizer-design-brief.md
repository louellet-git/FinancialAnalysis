# Visualizer design brief — cosmetic redesign handoff

Audience: a design agent doing a **purely cosmetic** restyle of the single-page
company report produced by `scripts/visualize_dcf.py` (e.g.
`data/processed/IBM_dcf.html`). This document is the contract: everything a
designer needs to restyle the page without reading the code, plus the hard
constraints that must survive the redesign.

## What the page is

One self-contained HTML file per ticker. All content is generated server-side
by Python and baked into the file: static HTML (header, stat chips, tables,
notes, footer) interleaved with interactive plotly.js charts. There is no
app shell, no routing, no data fetching — the page is an artifact meant to be
opened from disk, attached to an email, or printed.

Reference renders: `python -m scripts.visualize_dcf IBM` (requires the repo's
IBM data). Page length is roughly 4,900 px at 1440 px wide.

## Page structure

```
<body>
├── <header>                          h1 title, 2 meta lines, stat-chip row
├── section.rsec  "Price & valuation"        (h2.section-h + div.grid)
├── section.rsec  "Fundamentals"
├── section.rsec  "Aggregate DCF scenarios"
├── section.rsec  "Driver-based model"       (conditional content)
├── section.rsec  "Options market"           (conditional content)
└── <footer>                          3 provenance/engine lines
```

Each `section.rsec` contains an `h2.section-h` (17 px, hairline bottom border)
and a `div.grid` — a 2-column CSS grid (`repeat(2, minmax(0,1fr))`, 16 px gap)
that collapses to 1 column below 1100 px. Cards are `section.card` blocks
(surface background, 1 px hairline border `rgba(11,11,11,0.10)`, 8 px radius,
16 px padding, `overflow-x: auto`); `.card.wide` spans both columns. Every
card is `h3` title (15 px) + `p.sub` caption (12 px, muted) + body (a plotly
div, an HTML table, or a `p.note`).

When a section's data is missing the card body is a `p.note` — a quiet
rounded box (13 px text on `#f0efec`) with a labeled "not available" message.
Sections never disappear; they degrade to these notes.

## Section inventory (every panel)

### 1. Header band (static HTML)
- `h1` "TICKER — Company report" (22 px), company name and a meta line
  (as-of date, net debt, share count, price source) at 13 px secondary ink.
- **Stat-chip row** (`div.chips`, flex, 12 px gap): 5–6 chips. Each `div.chip`
  is label (11 px muted) / value (18 px semibold ink) / optional sub-label
  (11 px muted). Chips: Last price, Market cap, Bear P50, Base P50, Bull P50,
  and "Base P50 vs price" — the one *accent* chip (`.chip.accent`, darker
  border) whose value is colored `#006300` (upside) or `#d03b3b` (downside).

### 2. Price & valuation (two wide chart cards)
- **Price history** — plotly line (2 px, blue `#2a78d6`), up to ~3,300 weekly
  points spanning decades. One direct end-label (last close). Two-button
  linear/log scale toggle, top-right (plotly updatemenus). Height 380 px.
- **Price vs valuation — 6-month view** — daily close line (2 px, ink
  `#0b0b0b`, ~126 points) over: a translucent green band (base scenario
  P10–P90, `rgba(27,175,122,0.10)`) and three 2 px horizontal scenario-P50
  lines (bear blue / base green / bull yellow), each direct-labeled at the
  right edge ("Bull P50 $232"). Band labeled "Base P10–P90" at top-left.

### 3. Fundamentals (two wide cards)
- **Revenue, free cash flow & margin** — one plotly figure, two stacked
  subplots sharing the x axis (categorical fiscal years, ≤10):
  top (66% height) revenue columns (blue, 0.55 category width) + FCF line
  (aqua `#1baf7a`, 2 px, 8 px markers with 2 px surface ring); bottom (34%)
  FCF-margin line (same aqua, % axis). Legend top-left (Revenue, Free cash
  flow). Direct labels on the latest revenue bar, latest FCF point, latest
  margin point only. Height 420 px.
- **Annual fundamentals** — HTML table, 9 columns (FY, Revenue, Net income,
  OCF, Capex, SBC, Acquisitions, FCF, FCF margin) × ≤10 rows. `$x.xB`
  formatting, em-dash for missing values.

### 4. Aggregate DCF scenarios (4 half-width chart cards + 1 wide table card)
- **FCF projection fan** — 3 scenario median lines (2 px; bear blue, base
  green, bull yellow) each with a 10%-opacity P10–P90 fill band; 10 x-ticks
  (projection years); scenario names direct-labeled at line ends + legend.
- **Per-share value distribution** — 3 overlaid histograms (~60 bins each,
  opacity 0.55, same scenario hues), a 2 px vertical ink price line labeled
  "Price $218.00". Legend present.
- **PV build-up — median base case** — plotly waterfall, 13 bars: 10 years +
  terminal value (increases blue `#2a78d6`), net debt (decrease red
  `#e34948`), equity total (gray `#52514e`); x labels at −45°; two
  annotations ("60% of gross PV", "$141/share").
- **Sensitivity tornado** — 4 horizontal paired bars (−1σ blue / +1σ red,
  overlay on a shared base), signed delta labels outside bar ends, thin
  vertical base-P50 rule + label.
- **Scenario summary** — wide HTML table: 3 rows × 8–9 columns; first column
  carries a 10 px color dot (`span.dot`) keying the scenario hue.

### 5. Driver-based model (conditional; when present: 3 wide + 2 half cards + meta line)
- **Segment revenue history** — stacked columns per fiscal year (~3–8 years,
  2–8 segments), categorical slot colors in fixed order, 2 px surface gap
  between stack segments (white marker outline). Legend present.
- **Driver assumptions** — wide HTML table (~12 rows × 7 cols: Driver, Type,
  Dist, Mean, Std, Bounds, Rationale). Type column holds pill badges
  (`span.badge.stoch` blue-tinted / `span.badge.det` gray) — the only pill
  component on the page. Rationale column wraps (`td.wrap`, min 260 px).
- **Driver correlation matrix** — plotly heatmap, N×N for N stochastic
  drivers (typically 4–8), diverging scale red `#e34948` → neutral `#f0efec`
  → blue `#2a78d6` over [−1, 1], 2 px cell gaps, values annotated on
  off-diagonal cells only (10 px secondary ink), no colorbar.
- **Driver model vs aggregate base** — two horizontal dumbbell rows: a 6 px
  35%-opacity range line P10→P90 with 9/13/9 px markers (P10/P50/P90, 2 px
  surface rings); driver row violet `#4a3aa7`, aggregate row green
  `#1baf7a`. All six values direct-labeled (labels above the bottom row,
  below the top row); 2 px ink price line labeled above the plot.
- **Driver attribution tornado** — same construction as the aggregate
  tornado, one row per stochastic driver.
- **Engine meta line** — a full-width 12 px muted paragraph (sims, seed,
  horizon, net debt, shares, TV share; optionally the fraction of paths with
  pinned terminal growth).
- Absent state: a single wide card titled "Driver model" with a note.

### 6. Options market (conditional; chip row + 4 half cards + 1 wide table)
- **Chain stat-chip row** — 6 chips (Snapshot date, Spot, P/C volume, P/C
  open interest, Total volume, Total open interest), same chip component as
  the header.
- **ATM implied-volatility term structure** — line+markers (blue, 2 px, 8 px
  ringed markers), ~10–18 expiries over ~2.5 years; a 1 px vertical rule at
  the earnings date labeled at the plot bottom, and an arrowed annotation
  "Earnings hump — 60.4%" on the first post-earnings expiry.
- **Implied move by expiry** — single-hue blue columns (0.55 width), ~18
  bars, −35° tick labels. Value labels on bars only when ≤8 bars; otherwise
  tooltips carry the values.
- **Volatility skew by expiry** — diverging columns around a zero hairline:
  positive blue, negative red. Same labeling rule as implied move.
- **Open interest by strike** — heatmap, 4 expiry rows × ~40 strike columns,
  sequential blue ramp (`#f4f8fd` → `#0d366b`) with a thin colorbar
  ("OI"), 2 px cell gaps, 2 px ink spot line labeled "Spot $214.16" above
  the plot.
- **Chain detail — ATM ±10 strikes** — wide HTML table, 21 rows × 11 columns
  (5 call cols, bold strike, 5 put cols) with a two-tier header (Calls /
  Strike / Puts group row, then Bid Ask Last IV OI). The ATM row is
  highlighted (`tr.atm`, `rgba(42,120,214,0.10)` wash).
- Absent state: one wide card with a note.

### 7. Footer (static HTML)
Three 12 px muted lines: data provenance (EDGAR periods, price cache,
options snapshot date, price source), engine parameters (sims × horizon,
seeds), and a generated-by line.

## Current interactions

- **Hover tooltips on every chart** (plotly): unified x-hover on the price
  history, 6-month view, fan chart, and fundamentals subplots (one readout
  listing all series at the hovered x); per-mark hover on bars, histogram
  bins, waterfall steps, tornado bars, dumbbell markers, and heatmap cells.
  Tooltip templates are custom (e.g. "FY2025: $67.5B", "24 Jul 26 · $215:
  2,714 contracts") with the series name in the tooltip's side tag.
- **Scale toggle**: the price-history card has a two-button Linear/Log
  updatemenu (top-right inside the plot area). This is the only control on
  the page — there are no global filters, date pickers, or cross-chart
  interactions.
- **Zoom/pan**: plotly's default drag-zoom and double-click-reset remain
  active on all charts. The plotly modebar is disabled
  (`displayModeBar: false`) — no floating toolbar appears.
- **Responsive**: charts are `responsive: true`, width 100% of the card; a
  `resize` event is dispatched on load. The grid drops to one column below
  1100 px viewport width.
- No keyboard-specific affordances, no animations or transitions beyond
  plotly defaults, no dark mode.

## Current style tokens

All values below are hard-coded hex in `scripts/visualize_dcf.py` (module
constants) and in the single `<style>` block of the emitted page.

**Typeface** — `system-ui, -apple-system, "Segoe UI", sans-serif` everywhere
(page and charts). No display/serif face. `font-variant-numeric:
tabular-nums` on all table cells; chip values use default proportional
figures.

**Type scale** — h1 22 px · h2 section header 17 px · h3 card title 15 px ·
chip value 18 px semibold · meta/note 13 px · chart base font 12 px · sub
captions 12 px · chip/annotation labels 11 px · in-chart bar labels and
heatmap cell values 10 px.

**Chrome & ink (light mode only)**

| Role | Hex |
|---|---|
| Page background | `#f9f9f7` |
| Card / chart surface | `#fcfcfb` |
| Primary ink | `#0b0b0b` |
| Secondary ink | `#52514e` |
| Muted ink (axis, labels, subs) | `#898781` |
| Gridlines (1 px solid) | `#e1e0d9` |
| Axis line / baseline | `#c3c2b7` |
| Neutral fill (notes, badges, heatmap midpoint) | `#f0efec` |
| Hairline border | `rgba(11,11,11,0.10)` |

**Chart palette** (dataviz reference palette; categorical order is the
CVD-safety mechanism — do not reorder or introduce hues):

- Scenario slots (fixed): bear `#2a78d6` blue · base `#1baf7a` aqua-green ·
  bull `#eda100` yellow.
- Full categorical order (segments etc.): `#2a78d6`, `#1baf7a`, `#eda100`,
  `#008300`, `#4a3aa7`, `#e34948`, `#e87ba4`, `#eb6834`.
- Driver-model series: `#4a3aa7` (slot 5), deliberately distinct from the
  scenario hues.
- Diverging pair (tornados, waterfall, skew, correlation): cool `#2a78d6` ↔
  warm `#e34948`, neutral midpoint `#f0efec`.
- Sequential ramp (OI heatmap): `#f4f8fd`, `#cde2fb`, `#9ec5f4`, `#6da7ec`,
  `#3987e5`, `#256abf`, `#184f95`, `#0d366b`.
- Delta text colors (text only, never marks): up `#006300`, down `#d03b3b`.
- Band/area fills are the series hue at 10% opacity; dumbbell range lines at
  35% opacity.

**Mark specs** — 2 px lines; ≥8 px markers with a 2 px surface-colored ring;
bars at 0.55 category width with `bargap` 0.4; 2 px surface gaps between
stacked segments and heatmap cells; hairline solid gridlines (never dashed);
direct labels are selective (endpoints/extremes), 10–11 px, always in ink
tones — text never wears a series color.

**Spacing** — body padding 24 px; grid gap 16 px; card padding 16 px; section
header margin 28 px top / 12 px bottom; chip padding 10×14 px, min-width
116 px; table cells 6×10 px with 1 px bottom rules; chart heights 380 px
(420 px for the fundamentals stack); chart margins l 56 / r 24–130 / t 8–30 /
b 36–44 px.

## Hard constraints (must survive any redesign)

1. **Single self-contained file.** Everything inline: one `<style>` block,
   plotly.js (~4.5 MB) inlined in a `<script>` tag. **Zero external
   requests** — no webfonts, CDN links, external images/icons. Fonts must
   come from the system stack; icons, if any, must be inline SVG/unicode.
2. **Charts are plotly.js** rendered from Python. Restyle via layout/trace
   attributes and CSS around the chart divs; do not swap the chart library
   or hand-edit the generated trace JSON.
3. **Generated page.** All markup comes from Python f-strings — the deliverable
   of a redesign is new tokens/CSS/layout specs (and plotly layout values),
   not a hand-edited HTML file. Class names listed above are the stable hooks.
4. **Print-readable.** The page is printed/PDF'd. Keep light backgrounds,
   ≥3:1 mark contrast or a labeled relief channel (tables/direct labels
   already provide this), and avoid hover-only information: every hover
   value must remain reachable via a direct label, axis, or table.
5. **Section degradation stays visible.** "Not available" notes must remain
   distinct, labeled blocks — never hide an empty section.
6. **Color discipline.** Categorical hue order stays fixed; scenario hues
   follow the scenario (bear/base/bull) everywhere they appear; diverging
   midpoint stays neutral; sequential ramp stays single-hue. If the palette
   is re-branded, validate it (lightness band, chroma floor, adjacent-pair
   CVD ΔE ≥ 12, 3:1 surface contrast) before shipping.
7. **Responsive floor.** Two-column grid ≥1100 px, single column below;
   tables scroll horizontally inside their cards; the page body must never
   scroll horizontally.

## Non-goals (out of scope for the redesign)

- No content or data changes: numbers, labels, captions, tooltip text,
  table columns, and chart types stay as they are.
- No new sections, panels, chips, or controls; no removal of existing ones.
- No new interactions (filters, tabs, dark mode, animations) — the redesign
  is visual: tokens, spacing, typography, card/chip/table/badge styling,
  and plotly chrome (fonts, margins, grid/axis colors).
- No changes to the CLI, file naming, or the report generation pipeline.
