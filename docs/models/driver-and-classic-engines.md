# Model: driver engine and classic engine (reviewed IBM model, scenarios, verdict)

Updated: 2026-11-21. Commit: 90ed737 (base).

Backtest: `fa backtest TICKER` (point-in-time vintages of the revenue and free cash flow projections). None applies to the value of one reviewed model, because one ticker gives no sample; the four-step review applies. The classic IBM version 3 is frozen.

## Purpose
Two engines for a hand-built valuation of one company. The driver engine (`fa dcf drivers`) draws named
drivers together and gives a value range for each share. The classic engine (`fa dcf classic`, `fa dcf scenarios`)
draws four numbers directly. A person uses them for the reviewed IBM report and for the first passes (JNJ, ANT).
The verdict names the chance that the price is too high.

## Inputs
| Input | Source | Note |
|---|---|---|
| Driver model | `analysis/<TICKER>/driver_model.toml` | Drivers, correlations, segments, net debt, shares. IBM, JNJ, ANT exist |
| Rationale of each driver | `analysis/IBM/DRIVERS.md` | The TOML file is the correct source (`CLAUDE.md`, rule 2) |
| Company periods (classic) | `data/processed/<TICKER>.json` (`fa import`) | Net debt and shares from the latest 10-K, or a curated override |
| Price | `--price`, or the last close | For the chance and the verdict |

## Method (short)
1. Driver engine (`rust/src/dcf/driver.rs`). Stochastic drivers are drawn from correlated normals (Cholesky
   factor of the correlation matrix), then `mean + std x z`, then clipped to `[lo, hi]` (clip after correlation).
2. Revenue by segment, each with its growth driver. FCFF margin = core + drift - SBC - recurrence x M&A ratio.
3. WACC = equity weight x (rf + beta x ERP) + (1 - equity weight) x (rf + kd spread) x (1 - tax). The cost of
   debt floats with the risk-free draw.
4. Terminal value: Gordon form. The terminal growth is at most WACC less 2% on each path (`TG_WACC_GAP`).
5. Value per share = (enterprise value - net debt) / shares. The run also gives the P50 change for each driver
   mean moved by one standard deviation (attribution).
6. Classic engine (`classic.rs`): one draw of growth, margin, WACC and terminal growth for each path, 10 years.
7. Scenarios (`scenarios.rs`): bear, base and bull presets of the classic engine. IBM entries are version 3
   (A-019).
8. Verdict (`verdict.rs`): the share of path values below the price, in five bands (A-018).

## Outputs
`fa dcf drivers IBM` (seed 7, 10,000 paths): P10 / P50 / P90 of $83.84 / $151.84 / $258.03 for each share,
the engine check against a hand DCF, and the attribution. `fa dcf scenarios IBM`: bear, base, bull (version 3).
`fa verdict IBM --price P`: the chance and the band. Same seed gives the same numbers.

## Assumptions
The choices of the IBM driver model are the closed decisions of the IBM reviews, not records. Records:

| Record | Name | Value | One line |
|---|---|---|---|
| A-018 | Verdict bands | 25 / 40 / 60 / 75% | Cutoffs 25% and 75% follow Morningstar (proposal); 40% and 60%: no source |
| A-019 | Classic presets, IBM version 3 | bear / base / bull: growth 1 / 4 / 6%, WACC 9.5 / 8.5 / 8%, terminal 1.5 / 2.5 / 3% | IBM v3 entries (base 2.15% / 7.5% / 2.25%) are on purpose (`HIDDEN-ASSUMPTIONS.md` #13) |
| A-022 | Equity risk premium | 4.41% (auto); 4.5% (IBM) | IBM value from its review; the auto value has no confirmed source |
| A-025 | Beta | 2/3 raw + 1/3 (auto); 0.85 fixed (IBM) | IBM 0.85: DRIVER-REVIEW, row `beta` |
| A-026 | SBC is a cost | on | IBM review F5 |
| A-011 | Cost of debt spread | 1.50% (auto); -0.10% (IBM) | IBM: fair value of the debt ladder |

## Limits
- P10 and P90 are too narrow (`HIDDEN-ASSUMPTIONS.md` #12, ticket T-DCF-1): read the P50. The chance is too sure.
- The IBM P50 is conditional on the M&A recurrence f = 0.50 (a judgment, not fitted).
- `scenarios.rs` has IBM version 3. Version 4 exists only in `fa dcf drivers IBM`.
- A first pass (JNJ, ANT, DUOL presets) did not pass the four review steps. Do not use it alone for a trade.
- The classic engine takes the shares of a curated file: no split factor (A-017).
- Published reports in `analysis/IBM/report/` are frozen.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/dcf/driver.rs`, `model.rs`, `classic.rs`, `scenarios.rs`, `verdict.rs`, `cli.rs` |
| Model or parameter file | `analysis/IBM/driver_model.toml`, `analysis/JNJ/driver_model.toml`, `analysis/ANT/driver_model.toml` |
| Command | `fa dcf drivers IBM`; `fa dcf classic JNJ`; `fa dcf scenarios IBM`; `fa verdict IBM --price 228.53` |
| Tests | `rust/src/dcf/tests.rs`, `runner_tests.rs`; `cargo test --manifest-path rust/Cargo.toml --locked` |

## Review status
IBM: reviewed. Version 3: `analysis/IBM/REVIEW.md` (F1 to F19). Version 4: `analysis/IBM/DRIVER-REVIEW.md`
(19 driver reviews). Both are closed decisions. JNJ and ANT: first passes, not reviewed. The default scenario
presets and the verdict cutoffs 40% and 60%: not reviewed.
