# A-019: presets of the classic engine and the IBM version 3 entries

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
`fa dcf classic` and `fa dcf scenarios` draw growth, margin, WACC and terminal growth directly, 10,000 paths,
10 years. Default presets for a ticker with no entry (`default_scenarios`, `scenarios.rs`), growth / WACC /
terminal growth: bear 1% / 9.5% / 1.5%; base 4% / 8.5% / 2.5%; bull 6% / 8% / 3%. Standard deviations:
growth 2%, WACC 1%, terminal growth 0.5%. Seed 7 (`SCENARIO_SEED`).
IBM entries are version 3 on purpose: growth / WACC / terminal growth / margin: bear 1% / 8.25% / 1.5% / 9.2%;
base 2.15% / 7.5% / 2.25% / 14.5%; bull 4.5% / 7.25% / 2.5% / 16.1%; WACC std 0.75%, margin std 2%.
Capital structure of IBM: net debt $54.0B, 948,675,228 shares. DUOL has first-pass presets (not a vetted build).
The version 4 numbers come from `fa dcf drivers IBM` only (seed 7: $83.84 / $151.84 / $258.03).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Classic engine | `rust/src/dcf/classic.rs` | `DcfAssumptions::default` (growth 15%, WACC 10%, terminal 3%) |
| Presets | `rust/src/dcf/scenarios.rs` | `default_scenarios`, `ibm_v3`, `duol`, `capital_override` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| IBM version 4 in `scenarios.rs` | The v4 model is a driver model; `docs/HIDDEN-ASSUMPTIONS.md` #13 keeps v3 here on purpose, in separate labeled report sections |

## Why this choice
The IBM v3 numbers are the closed decisions of `analysis/IBM/REVIEW.md`: F15 (net debt $47.5B to $54.0B),
F16 (growth and M&A pairing: g = 1.65% + f x 1.0 pp), F5 (SBC), F4. The default presets (1/4/6%, 9.5/8.5/8%,
1.5/2.5/3%) and the standard deviations have no source in any file read. They are generic placeholders.

## Effect on the result
IBM v3 base P50 is in the $141 family (`HIDDEN-ASSUMPTIONS.md` #13). The default presets: no sensitivity known.

## Who decided and when
IBM v3: the IBM review (REVIEW.md, closed, 2026-07). The default presets: not known.

## When to examine it again
Never for IBM v3 without a review of the same weight (`CLAUDE.md`, source rule 3 and 6). For the default
presets: before any use of them for a decision (a first pass is not a valuation).
