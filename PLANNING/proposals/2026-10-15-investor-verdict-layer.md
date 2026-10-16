# Proposal: an investor verdict layer, one tested piece at a time

Status: **proposed, revised 2026-10-16 after Luc's review on PR #16.** Author: Merlan
(merlan1996), drafted with Claude.

## Goal

Turn the driver engine's value range into something an investor can act on: what to
do, how sure we are, and what would prove us wrong. This is the investor-facing form of
the vision's L6 (gap attribution + bet ledger).

## Rules for every piece

- **Additive:** no change to the engines, the registers, the presets or the published reports.
- **Small:** one idea per PR, under about 100 lines plus tests.
- **Value test first:** each piece names a test of whether it delivers value; the next
  piece starts only after that test passes.
- **Python now, Rust-ready:** pure numpy functions, no hidden state. The engine-check
  tests from piece 1 become the fixed-answer tests a future Rust port must pass.

## The pieces, in order

| # | Piece | Value test (agreed before building) |
|---|---|---|
| 1 | **Engine check, no new features.** Grade the IBM report's July 22 watch list against the actual Q2 results; test the engine against an independent hand-built DCF (all randomness off) plus sanity tests (e.g. a higher margin always raises value); run a stock we have not covered yet alongside. | The engine matches the hand calculation; the graded predictions and the new stock show where it is weak |
| 2 | **Verdict band (this PR).** Chance the price is too high + five-band verdict; cutoffs at 25%/75% follow Morningstar's bull/bear calibration. | Luc finds the IBM output clearer than the current report's summary |
| 3 | **Price-implied solver** (reverse DCF per assumption). | Reveals a bet the IBM report did not state (a first run found consulting growth of 11.5% a year would be required) |
| 4 | **Return scenarios + chance of losing money** (price closes 15-30% of the gap per year, Capozza & Israelsen 2010). | Gives a different answer from assuming full convergence, and the difference matters to the verdict |
| 5 | **Scored track record** (each call timestamped in git, graded at its horizon, Brier score). | First IBM entries graded |

Pieces 3 and 4 were built and tested in the first version of this PR (commit
`076a953`); they are parked there and will come back as separate PRs if piece 2
passes its value test.

## Piece 2 output (`python -m scripts.verdict IBM --price 212.40`, seed 7)

P25 $112.57 / P50 $151.84 / P75 $200.39 (P10 $83.84 and P90 $258.03 match the
published v4.1 numbers). Chance the price is too high: 79.2%, so **Strongly
overpriced**. Quartiles, not P10/P90, set the verdict, since P10/P90 are "decorative"
until T-DCF-1.

## Rust

Not now. The full IBM analysis (`python -m analysis.IBM.dcf_drivers_run`) takes
about 0.25 seconds, so speed is not yet a constraint. A port becomes worthwhile when
many tickers, solves and loci run together; piece 1's fixed-answer tests make it safe.

## Sources

- Morningstar Equity Research Methodology (text dated 23 Oct 2020): https://s25.q4cdn.com/634000362/files/doc_downloads/1/Morningstar-Equity-Research-Methodology-2023-06-14.pdf
- Capozza & Israelsen (2010), via CXO Advisory: https://www.cxoadvisory.com/fundamental-valuation/speeddeterminants-of-stock-price-reversion/
