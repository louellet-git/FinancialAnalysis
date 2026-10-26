---
id: DOC-VISUALIZER-DESIGN-BRIEF
type: description
item: 10.15 Design description
status: current
date: 2026-10-25
scope: Contract for a purely cosmetic restyle of the single-page report from scripts/visualize_dcf.py.
---
# Visualizer design brief

## What the page is

Per-ticker page, e.g. `data/processed/IBM_dcf.html`; its form is fixed by hard constraints 1-7 below.
Not served: no app shell or routing; opened from disk or emailed.

Labels: "aggregate" = classic engine, "driver" = driver engine, "mechanical vintages" = backtest. Module map: docs/ARCHITECTURE.md.

Reference render: `python -m scripts.visualize_dcf IBM` (needs the repo's IBM data).
Page length is ≈4,900 px at 1440 px wide.

## Page structure

```
<body>
├── <header>                          h1 title, meta lines, stat-chip row
├── section.rsec  "Price & valuation"
├── section.rsec  "Fundamentals"
├── section.rsec  "Aggregate DCF scenarios"
├── section.rsec  "Driver-based model"       (conditional content)
├── section.rsec  "Backtest — mechanical vintages"
├── section.rsec  "Options market"           (conditional content)
└── <footer>                          provenance/engine lines
```

Each `section.rsec` holds an `h2.section-h` (flex row; hairline `::after` rule to its right)
and a `div.grid`: 2 columns (`repeat(2, minmax(0,1fr))`); breakpoint per constraint 7.
Cards: `section.card`, 1 px hairline border; tables scroll in `div.tblwrap`
(`overflow-x: auto`); `.card.wide` spans both columns. Every card is `h3` title + `p.sub`
caption (secondary ink) + body (a plotly div, an HTML table, a `p.note`, or a table followed by `p.note` blocks).

Missing data: card body is `p.note` (constraint 5). Price-dependent items (Market cap chip, the accent chip, price line and labels, 'vs price' column) are omitted when price is unavailable; the Last price chip stays, value '—' (sub 'unavailable').

## Section inventory (every panel)

### 1. Header band (static HTML)
- `h1` "TICKER — Company report", then two meta lines: company name; as-of date, net
  debt, share count, price source; both in muted ink.
- **Stat-chip row** (`div.chips`, flex, 12 px gap): 6 chips. Each `div.chip` uses the hooks in
  hard constraint 3: muted label, semibold ink value, optional muted sub-label. Chips: Last
  price, Market cap, Bear P50, Base P50, Bull P50, and "Base P50 vs price" — the
  one *accent* chip (`.chip.accent`, the only solid-filled chip); its value renders white
  via CSS, and direction is carried by the sign alone.

### 2. Price & valuation (two wide chart cards)
- **Price history** — plotly line (2 px, categorical slot 1), thinned only when the history has more than 2,600 points (weekly before the trailing
  year and daily within it, otherwise every close daily). One direct end-label (last close).
  Height 380 px.
- **Price vs valuation** — daily close line (2 px, primary
  ink; two years of dailies, default window the trailing 6 months, ~126 points) over: a translucent band (base scenario P10–P90, base
  scenario hue at 10% opacity) and three 2 px horizontal scenario-P50
  lines (bear / base / bull scenario hues), each direct-labeled at the
  right edge ("Bull P50 $V"). Band labeled "Base P10–P90" at top-left.

### 3. Fundamentals (two wide cards)
- **Revenue, free cash flow & margin** — one plotly figure, two stacked
  subplots sharing the x axis (categorical fiscal years, ≤10):
  top (66% height) revenue columns (categorical slot 1, 0.55 category width) + FCF line
  (categorical slot 2, 2 px, 8 px markers with 2 px surface ring); bottom (34%)
  FCF-margin line (same slot 2, % axis). Legend top-left (Revenue, Free cash
  flow). Direct labels on the latest revenue bar, latest FCF point, latest
  margin point only. Height 420 px.
- **Annual fundamentals** — HTML table, 9 columns (FY, Revenue, Net income,
  OCF, Capex, SBC, Acquisitions, FCF, FCF margin) × ≤10 rows. `$x.xB`
  (SBC `$x.xxB`), em-dash for missing values.

### 4. Aggregate DCF scenarios (4 half-width chart cards + 1 wide table card)
- **FCF projection fan** — 3 scenario median lines (2 px; bear / base /
  bull scenario hues) each with a 10%-opacity P10–P90 fill band; 10 x-ticks
  (projection years); scenario names direct-labeled at line ends + legend.
- **Per-share value distribution** — 3 overlaid histograms (bin width = view span/60:
  view clipped to P99, bins run to the data max; opacity 0.55, same scenario hues), a 2 px vertical ink price line labeled
  "Price $P". Legend present.
- **PV build-up — median base case** — plotly waterfall, 13 bars: 10 years +
  terminal value (increases: diverging cool), net debt (decrease: diverging warm),
  equity total (secondary ink); x labels at −45°; two
  annotations ("N% of gross PV", "$V/share").
- **Sensitivity tornado** — 4 horizontal paired bars (−1σ cool / +1σ warm,
  overlay on a shared base), signed delta labels outside bar ends, thin
  vertical base-P50 rule + label.
- **Scenario summary** — wide HTML table: 3 rows × 8–9 columns ("vs price" is the last); first column
  carries a color dot (`span.dot`) keying the scenario hue.

### 5. Driver-based model (conditional on `analysis/<TICKER>/driver_model.py` exposing `build()`; docs/ARCHITECTURE.md §2; when present: 6 wide + 2 half cards + meta line)
- **Segment revenue history** — stacked columns per fiscal year (~3–8 years,
  2–8 segments), categorical slot colors in fixed order, 2 px surface gap
  between stack segments (2 px outline in the surface colour, `_SURFACE`). Legend present.
- **Driver assumptions** — wide HTML table (one row per driver, 16 for IBM, × 7 cols: Driver, Type,
  Dist, Mean, Std, Bounds, Rationale). Type column holds badges
  (`span.badge.stoch` accent-tinted / `span.badge.det` raised-fill), the only badge
  component on the page. Rationale column wraps (`td.wrap`, min 260 px).
- **Driver correlation matrix** — plotly heatmap, N×N for N ≥ 2 stochastic
  drivers (IBM: 8; N < 2 gives a `p.note`), diverging scale warm → neutral midpoint → cool over [−1, 1],
  2 px cell gaps, values annotated on off-diagonal cells only (secondary ink, per
  type scale), no colorbar.
- **Driver model vs aggregate base** — two horizontal dumbbell rows: a 6 px
  35%-opacity range line P10→P90 with 9/13/9 px markers (P10/P50/P90, 2 px
  surface rings); driver row in the driver-model color, aggregate row
  in the base scenario hue. All six values direct-labeled (labels above the bottom row,
  below the top row); 2 px ink price line labeled above the plot.
- **Driver attribution tornado** — same construction as the aggregate
  tornado, one row per stochastic driver.
- **Path explorer (3 wide cards)** — FCFF path spaghetti (~300 sampled paths,
  `_SPAGHETTI_TARGET`: every path beyond P1/P99 plus a seeded middle sample): middle
  paths muted ink at 12% alpha, width 1; bottom 1% bear hue and top 1% bull hue at 60%
  alpha, width 1.5; median path full ink, 2.5 px, labelled "Median path"; parallel
  coordinates (2,000 paths, `_PARCOORDS_TARGET`), coloured by $/sh on the sequential
  ramp, no colorbar; outlier-paths table (bottom/top 10 by
  per-share value, each driver's draw with its z-score, |z| ≥ 1.5 bold).
- **Engine meta line** — a full-width secondary-ink paragraph (sims, seed,
  horizon, net debt, shares, TV share and the fraction of paths with
  pinned terminal growth).
- Absent: card "Driver model".

### 6. Backtest — mechanical vintages (always rendered; chip row + 2 half cards + 1 wide table)
- **Backtest stat-chip row** (same chip component as the
  header) — "P10–P90 coverage" (hits/total, sub-label "% of realized (nominal
  80%)"; "—" with sub-label "no realized outcomes yet" when no outcome is realized), then one "+Ny Revenue bias" and one
  "+Ny FCF bias" chip per horizon (mean error vs P50, sub-label "mean err vs P50 ·
  n="), value colored delta-up (≥ 0) or delta-down.
- **Revenue hedgehog** and **FCF hedgehog** (half cards) — plotly scatter, $B on y,
  fiscal years on x (dtick 1), no legend: realized actuals as a primary-ink line
  with markers, over one thin accent-color spur per vintage (P50 projection fanning
  off that vintage's base-year actual) with P10–P90 whiskers at lower opacity.
  Spur hover shows the vintage as-of date.
- **Vintage scorecard** — wide HTML table, 10 columns (Vintage as-of, Base FY,
  Target FY, Metric, P10, P50, P90, Actual, Err vs P50, In band), one row per
  vintage × target year × metric. A ⚑ after the vintage date marks a vintage whose
  parameter window touches a flagged year. Err is delta-colored; In band is
  ✓ / ✗ / —. Below the table, `p.note` blocks: method (mechanical, point-in-time,
  operating projections only), `filed_date` visibility and as-currently-reported
  actuals, the ⚑ data flags, and skipped vintage dates when any.
- Absent (no vintage with 3 clean annual periods): card "Vintage backtest"; note lists skipped dates.

### 7. Options market (conditional; chip row + 4 half cards + 1 wide table)
- **Chain stat-chip row** — 6 chips (Snapshot, Spot, P/C volume, P/C
  open interest, Total volume, Total open interest), same chip component as
  the header.
- **ATM implied-volatility term structure** — line+markers (categorical slot 1, 2 px, 8 px
  ringed markers), ~10–18 expiries over ~2.5 years; a 1 px vertical rule at
  the earnings date labeled at the plot bottom, and an arrowed annotation
  "Earnings hump — X%" on the first post-earnings expiry.
- **Implied move by expiry** — single-hue (categorical slot 1) columns (0.55 width), ~18
  bars, −35° tick labels. Value labels on bars only when ≤8 bars; otherwise
  tooltips carry the values.
- **Volatility skew by expiry** — diverging columns around a zero hairline:
  positive cool, negative warm. Same labeling rule as implied move. Expiries under 2 days
  (`_SKEW_MIN_DTE`) are dropped: near-expiry wing IVs are solver noise.
- **Open interest by strike** — heatmap, 4 expiry rows × ~40 strike columns,
  sequential ramp (first → last step) with a thin colorbar
  ("OI"), 2 px cell gaps, 2 px ink spot line labeled "Spot $S" above
  the plot.
- **Chain detail — ATM ±10 strikes** — wide HTML table, up to 21 rows × 11 columns
  (5 call cols, bold strike, 5 put cols) with a two-tier header (Calls /
  Strike / Puts group row, then Bid Ask Last IV OI). The ATM row is
  highlighted (`tr.atm`, accent tint).
- Absent: card "Option chain".

### 8. Footer (static HTML)
Three muted lines: data provenance (EDGAR periods, price cache,
options snapshot date, price source), engine parameters (sims × horizon,
seeds), and a generated-by line.

## Current interactions

- **Hover tooltips on every chart except the parallel-coordinates plot** (plotly): unified x-hover on the
  fundamentals subplots and the fan chart (one readout
  listing all series at the hovered x); per-mark hover on bars, histogram
  bins, waterfall steps, tornado bars, dumbbell markers, and heatmap cells.
  Tooltip templates are custom (e.g. "FY20XX: $X.XB", "24 Jul 26 · $K: N
  contracts") with the series name in the tooltip's side tag.
- **Controls**: the Linear/Log updatemenu (price-history card, top-right, above the plot
  area) and the range buttons (both price cards, top-left; price history 1M·6M·1Y·5Y·Max, valuation 1M·3M·6M·1Y·2Y). These are the only controls on
  the page.
- **Zoom/pan**: plotly's default drag-zoom and double-click-reset remain
  active on all other charts; parallel coordinates uses axis brushing instead. The plotly modebar is disabled
  (`displayModeBar: false`) — no floating toolbar appears.
- **Responsive**: charts are `responsive: true`, width 100% of the card; a
  `resize` event is dispatched on load.
- No keyboard-specific affordances; plotly default motion only.

## Current style tokens

Current tokens: `scripts/visualize_dcf.py` (constants and `:root`), applied from
docs/design-specs/2026-07-16-ratings-desk-blueprint.md; a restyle changes those values and the CSS rules in that file.

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
   not a hand-edited HTML file. Class names listed above are the stable hooks, plus: `.chip-label`, `.chip-value`, `.chip-sub`
   (a chip's label, value and sub-label); `header .meta` / `p.meta` (the header's meta lines);
   `div.wide-chips` (a full-width chip row: backtest `div.chips.wide-chips`; options `div.chips` inside `div.wide-chips`); `p.sub.wide-note` (the
   driver section's engine meta line).
4. **Print-readable.** The page is printed/PDF'd. Keep light backgrounds,
   ≥3:1 mark contrast or a labeled relief channel (tables/direct labels
   already provide this), and avoid hover-only information: every hover
   value must remain reachable via a direct label, axis, or table. Exempt: exploratory
   per-path and per-point hovers (spaghetti, histogram counts, hedgehog P50 points, IV term
   points); the outlier table is the spaghetti's print form.
5. **Section degradation stays visible.** "Not available" notes must remain
   distinct, labeled blocks — never hide an empty section.
6. **Color discipline.** Categorical hue order stays fixed; scenario hues
   follow the scenario (bear/base/bull) everywhere they appear; diverging
   midpoint stays neutral; sequential ramp stays single-hue. If the palette
   is re-branded, validate it (adjacent-pair CVD ΔE ≥ 12, 3:1 surface contrast) as in `docs/design-specs/2026-07-16-ratings-desk-blueprint.md` §1 "Chart palette", paragraph "CVD / contrast confirmation", before shipping.
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
