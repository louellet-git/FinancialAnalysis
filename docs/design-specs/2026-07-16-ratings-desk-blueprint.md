# Redesign spec — "Ratings-desk blueprint"

**Received: 2026-07-16** (from the Claude design pass, responding to
`docs/visualizer-design-brief.md`). Status: ACCEPTED — applied to
`scripts/visualize_dcf.py` (see git history for the application commit).
The alternative "Ledger serif" direction (final paragraph) was NOT applied.

---

One direction. Light technical ground (#f4f4f5) instead of near-white; cards become **line drawings** (transparent, hairline-bordered, square-cornered, with corner registration marks) so density comes from structure, not surface tint. One solid object on the page: the accent chip, filled deep steel. Headings condense (system condensed stack), all figures set tabular. Ink rules (2px #1d1f20) bound the masthead and footer like a rating-agency report. Chart chrome recedes to hairlines; series colors are the only saturated marks.

---

## 1 · TOKEN TABLE

### Color roles

| Role | Hex |
|---|---|
| Page bg | `#f4f4f5` |
| Surface (cards) | `transparent` — cards are line drawings on the page bg; where a literal hex is required use `#f4f4f5` |
| Raised fill (chips, hover rows — only non-transparent surface) | `#eceef0` |
| Ink / primary text | `#1d1f20` |
| Ink secondary (body, chart text) | `#4a5158` |
| Ink faint (meta, ticks, notes) | `#6b7480` |
| Accent | `#5980a6` |
| Accent deep (section headers, links, solid chip) | `#31506e` |
| Accent tint (atm row, stoch badge fill) | `#e9eef4` |
| Hairline (card borders, td rules) | `#c9ced4` |
| Strong rule (masthead, footer, th underline) | `#1d1f20` |
| Gridline (plotly) | `#dde2e8` |
| Axis line / zeroline | `#aab2bb` |
| Neutral fill (bars, bands) | `#d7dce1` |

### Chart palette (validated intent below)

- **Scenario** — bear `#9e3a26` · base `#35618f` · bull `#23755f`
- **Categorical (8, order fixed — this order IS the CVD mechanism)**
  `#35618f`, `#b45309`, `#216e5c`, `#8f4d9f`, `#857f29`, `#882255`, `#667585`, `#0a5e75`
- **Diverging** — neg `#a04a17` ↔ mid `#c6cbd1` (neutral, achromatic) ↔ pos `#35618f`
- **Sequential (8-step, single hue = steel ~212°)**
  `#dfe6ee`, `#c4d1de`, `#a8bccd`, `#8ca7bd`, `#7092ac`, `#557d99`, `#3c6484`, `#274a66`
- **Up / down text** — up `#1c6b4f` · down `#a13326` (both ≥ 4.5:1 on page bg — safe at table sizes)

**CVD / contrast confirmation.** Every categorical, scenario and diverging endpoint sits at WCAG luminance ≤ 0.21 → ≥ 3.7:1 against `#f4f4f5` (weakest: `#857f29` at 3.79:1; strongest: `#882255` at 7.9:1). Adjacent categorical pairs alternate the blue↔yellow opponent axis (which survives protan/deutan/tritan) and carry ≥ 12 L\* or b\* separation: blue→orange, orange→teal, teal→purple, purple→olive, olive→wine (ΔL\*≈22), wine→slate (ΔL\*≈19 + chroma), slate→dark-cyan (ΔL\*≈12 + Δb\*≈15). Simulated ΔE(CIE76) ≥ 12 for all seven adjacencies under all three dichromacies. Scenario trio is orange-red / blue / teal — no red–green pair.

### Type scale (system stacks only — zero requests)

```
--font-display: "Avenir Next Condensed","Arial Narrow","Roboto Condensed","Liberation Sans Narrow","Helvetica Neue",sans-serif;
--font-body:    "Avenir Next","Segoe UI",system-ui,-apple-system,sans-serif;
--font-mono:    ui-monospace,"SF Mono","Cascadia Mono",Consolas,"Liberation Mono",monospace;
```

| Slot | px / weight / tracking / case |
|---|---|
| Report title (header h1) | 30 / 600 / +0.01em / title — display stack |
| Header meta line | 12 / 500 / +0.08em / UPPERCASE — mono, `#6b7480` |
| h2.section-h | 14 / 600 / +0.14em / UPPERCASE — display, `#31506e` |
| h3 card title | 16 / 600 / +0.02em — display, `#1d1f20` |
| Chip value | 26 / 600 / 0 — display, tabular |
| Chip label | 10.5 / 600 / +0.10em / UPPERCASE — body, `#6b7480` |
| Body / p.sub | 13 / 400 / 0 — body, lh 1.55, `#4a5158` |
| p.note | 11.5 / 400 — body, `#6b7480` |
| th | 11 / 600 / +0.08em / UPPERCASE — body, `#6b7480` |
| td | 12.5 / 400 — body, tabular-nums |
| .badge | 10.5 / 600 / +0.06em / UPPERCASE |
| footer | 11 / 400 — mono, `#6b7480` |

### Spacing

`--sp-1:4px; --sp-2:8px; --sp-3:12px; --sp-4:16px; --sp-5:20px; --sp-6:28px; --sp-7:40px; --sp-8:56px`
Card padding 20px · grid gap 20px · gap between sections 56px · header→content 40px · h2→grid 16px · inside-card h3→chart 12px. Radius: **0 everywhere**.

---

## 2 · CSS DELTA (paste-ready)

```css
:root{
  --bg:#f4f4f5; --raised:#eceef0; --ink:#1d1f20; --ink-2:#4a5158; --ink-3:#6b7480;
  --accent:#5980a6; --accent-deep:#31506e; --accent-tint:#e9eef4;
  --hairline:#c9ced4; --rule:#1d1f20; --td-rule:#e3e6ea;
  --font-display:"Avenir Next Condensed","Arial Narrow","Roboto Condensed","Liberation Sans Narrow","Helvetica Neue",sans-serif;
  --font-body:"Avenir Next","Segoe UI",system-ui,-apple-system,sans-serif;
  --font-mono:ui-monospace,"SF Mono","Cascadia Mono",Consolas,"Liberation Mono",monospace;
}
body{margin:0;background:var(--bg);color:var(--ink);
  font:400 13px/1.55 var(--font-body);
  font-variant-numeric:tabular-nums lining-nums;
  -webkit-font-smoothing:antialiased}
a{color:var(--accent-deep)} a:hover{color:var(--accent)}
::selection{background:var(--accent-tint)}

header{padding:28px 0 18px;border-bottom:2px solid var(--rule);margin-bottom:40px}
header h1{margin:0;font:600 30px/1.1 var(--font-display);letter-spacing:.01em}
header p, header .meta{font:500 12px/1.4 var(--font-mono);
  text-transform:uppercase;letter-spacing:.08em;color:var(--ink-3)}

/* Chips — the primary read. Blueprint plates; accent chip is the ONE solid object. */
.chips{display:flex;flex-wrap:wrap;gap:12px;margin:0 0 40px}
.chip{background:transparent;border:1px solid var(--ink);border-radius:0;
  padding:12px 18px 10px;box-shadow:none}
.chip b,.chip strong,.chip .val{font:600 26px/1.1 var(--font-display);
  letter-spacing:0;color:var(--ink);display:block}
.chip small,.chip .lbl{font:600 10.5px/1.2 var(--font-body);
  text-transform:uppercase;letter-spacing:.10em;color:var(--ink-3)}
.chip.accent{background:var(--accent-deep);border-color:var(--accent-deep);color:#fff}
.chip.accent b,.chip.accent strong,.chip.accent .val{color:#fff}
.chip.accent small,.chip.accent .lbl{color:#c4d1de}

section.rsec{margin:0 0 56px}
h2.section-h{display:flex;align-items:center;gap:12px;margin:0 0 16px;
  font:600 14px/1.2 var(--font-display);text-transform:uppercase;
  letter-spacing:.14em;color:var(--accent-deep)}
h2.section-h::after{content:"";flex:1;height:1px;background:var(--hairline)}

.grid{display:grid;grid-template-columns:1fr 1fr;gap:20px}
@media (max-width:1099px){.grid{grid-template-columns:1fr}}

/* Cards — transparent line drawings with corner registration marks */
section.card{position:relative;background:transparent;border:1px solid var(--hairline);
  border-radius:0;box-shadow:none;padding:20px}
.card.wide{grid-column:1/-1}
section.card::before,section.card::after{content:"";position:absolute;
  width:9px;height:9px;pointer-events:none;
  background:linear-gradient(var(--accent),var(--accent)) 50% 0/1px 100% no-repeat,
             linear-gradient(var(--accent),var(--accent)) 0 50%/100% 1px no-repeat}
section.card::before{top:-5px;left:-5px}
section.card::after{bottom:-5px;right:-5px}

h3{margin:0 0 12px;font:600 16px/1.25 var(--font-display);letter-spacing:.02em;color:var(--ink)}
p.sub{margin:-8px 0 12px;font-size:13px;color:var(--ink-2)}
p.note{margin:10px 0 0;font-size:11.5px;line-height:1.5;color:var(--ink-3);
  padding-left:8px;border-left:2px solid var(--hairline)}

table{border-collapse:collapse;width:100%}
th{font:600 11px/1.3 var(--font-body);text-transform:uppercase;letter-spacing:.08em;
  color:var(--ink-3);padding:6px 10px;border-bottom:1px solid var(--rule)}
td{font-size:12.5px;padding:6px 10px;border-bottom:1px solid var(--td-rule);color:var(--ink)}
tr:hover td{background:var(--raised)}
tr.atm td{background:var(--accent-tint);font-weight:600}
tr.atm td:first-child{box-shadow:inset 2px 0 0 var(--accent)}

.badge{border-radius:0;font:600 10.5px/1 var(--font-body);
  text-transform:uppercase;letter-spacing:.06em;padding:3px 7px}
.badge.stoch{background:var(--accent-tint);color:var(--accent-deep);
  border:1px solid var(--accent)}
.badge.det{background:var(--raised);color:var(--ink-2);border:1px solid var(--hairline)}

span.dot{display:inline-block;width:9px;height:9px;border-radius:0;
  vertical-align:baseline;margin-right:6px}   /* square swatch; keep inline bg color */

footer{margin-top:56px;padding-top:14px;border-top:2px solid var(--rule);
  font:400 11px/1.6 var(--font-mono);color:var(--ink-3)}

@media print{
  body{background:#fff}
  section.card{break-inside:avoid}
  tr:hover td{background:transparent}
}
```

Assumptions (adjust selectors if markup differs): chip value/label are the chip's bold + small children; header meta is a `p` or `.meta` inside `header`. Nothing else depends on inner structure.

---

## 3 · PLOTLY CHROME (layout attribute → value)

```
font.family                = '"Avenir Next","Segoe UI",system-ui,sans-serif'
font.size                  = 12
font.color                 = '#4a5158'
paper_bgcolor              = 'rgba(0,0,0,0)'
plot_bgcolor               = 'rgba(0,0,0,0)'
margin                     = {l:46, r:12, t:12, b:38}
colorway                   = ['#35618f','#b45309','#216e5c','#8f4d9f','#857f29','#882255','#667585','#0a5e75']
xaxis.gridcolor            = '#dde2e8'      (same for yaxis)
xaxis.gridwidth            = 1
xaxis.linecolor            = '#aab2bb'
xaxis.zerolinecolor        = '#aab2bb'
xaxis.zerolinewidth        = 1
xaxis.tickfont.size        = 11
xaxis.tickfont.color       = '#6b7480'
xaxis.title.font.size      = 11
xaxis.title.font.color     = '#6b7480'
legend.bgcolor             = 'rgba(0,0,0,0)'
legend.borderwidth         = 0
legend.font.size           = 11
legend.font.color          = '#4a5158'
hoverlabel.bgcolor         = '#1d1f20'
hoverlabel.bordercolor     = '#1d1f20'
hoverlabel.font.color      = '#f4f4f5'
hoverlabel.font.size       = 12
hoverlabel.font.family     = '"Avenir Next","Segoe UI",system-ui,sans-serif'
annotations[].font.color   = '#4a5158'
annotations[].font.size    = 11
```

Transparent chart backgrounds let charts sit inside the line-drawn cards with no white patch; grid hairlines are one step lighter than card borders so data ink dominates.

---

## 4 · HIERARCHY & RHYTHM NOTES

- **Cards vs page indistinguishable** (near-white on near-white) → cards become transparent line drawings on `#f4f4f5` with `#c9ced4` hairlines + registration marks; structure replaces surface tint.
- **Section headers vs card titles too close in weight** → h2 drops to 14px but gains uppercase, +0.14em tracking, accent-deep color and a trailing rule; h3 stays large (16px condensed). Different *kind*, not just size.
- **Chips underpowered as the primary read** → 26px condensed values, ink borders (heavier than card hairlines), and the accent chip is the page's only solid fill — the eye lands there first.
- **Uniform 16px rhythm reads as a dashboard** → two-tier rhythm: 56px between sections, 20px inside; the masthead 2px rule + 40px gap makes the top of page a distinct band.
- **Tables competing with charts** → td rules drop to `#e3e6ea` (lighter than card borders), th gets a single strong `#1d1f20` underline; tables read as one framed figure, not stacked stripes.
- **Numbers misalign / jitter** → `tabular-nums` set once on body; chip values and td inherit it.
- **Chart chrome heavier than data** → gridlines to `#dde2e8`, axis/zeroline to `#aab2bb`, ticks 11px faint-ink, margins tightened; series hues (all ≥3.7:1) are the only saturated marks.
- **Hover values unlabeled at a glance** → hover label inverted to ink-on-`#1d1f20` — unmissable against the light chart field, and tables carry every value in print.
- **"n/a" notes visually lost** → p.note gets a 2px hairline left rule and faint ink — visible, labeled, clearly subordinate.
- **Badges/dots read as UI candy** → square 9px swatch dots and square outlined badges match the blueprint grammar; stoch/det separate by accent-tint vs neutral fill, not hue alone.

---

**Alternative (one paragraph).** *Ledger serif*: keep every structural rule above but swap the display stack to `ui-serif, Georgia, "Times New Roman", serif` for h1/h2/h3/chip values, warm the ground to `#f5f4f1`, hairlines to `#d3cfc7`, and deepen the accent to a bank-note green `#2f5d4e` (scenario/categorical palette unchanged). Reads closer to a Moody's/S&P print report — more gravitas, less technical; choose it if the audience is client-facing rather than internal desk.
