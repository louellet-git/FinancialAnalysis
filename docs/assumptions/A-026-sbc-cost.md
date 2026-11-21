# A-026: stock-based compensation is a cost of the automatic model

Updated: 2026-11-21. Commit: 90ed737 (base).
Model document: `docs/models/auto-path-model.md`.

## Choice
SBC is a cash cost (`sbc_cost` on, `--rules sbc_cost=1`). The free cash flow is after SBC. The charge is the
mean of `abs(sbc_total) / revenue` of the last 3 fiscal years (`HIST_RATIO`). It is constant over the horizon.
The start and the target of the ramp of 2 years are the same number. With no value in 3 years, the ratio is 0
and the model prints a warning. The share count is the latest count plus the dilution (`shares_latest`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model | `rust/src/dcf/auto.rs` | `Rules::sbc_cost`, `HIST_RATIO`; `[process.sbc]` of the model file |
| Point-in-time test (the free cash flow of the test) | `rust/src/dcf/auto_pit.rs` | `fcff_of`, `fcff_of_rules` |
| IBM driver model | `analysis/IBM/driver_model.toml` | driver `sbc_ratio` (a ramp) |
| Classic engine, IBM v3 | `rust/src/dcf/scenarios.rs` | the v3 margin is after SBC |

## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| SBC is not a cost (add it back) | `--rules sbc_cost=0` | IBM review F5: with static shares it gives a double benefit |

## Why this choice
IBM review F5 (`analysis/IBM/REVIEW.md`): "Static shares while adding back SBC - dilution double-benefit".
Disposition: FIXED, SBC charged as a cash cost, diluted count, flat; effect -$17 per share for IBM.
The review is a closed decision for IBM. For the automatic model the choice is the same rule applied to
all tickers. The study lists it as a "convention (IBM review F5)". The window of 3 years: not known.

## Effect on the result
438 tickers, 2026-11-19: `sbc_cost=0` gives +0.079 in the median value / price (stable set +0.035),
median P50 +9% (`analysis/priors/dcf_investigation_2026-11-20.md`, section 2). `docs/HIDDEN-ASSUMPTIONS.md`, #260.

## Who decided and when
IBM review F5 (2026-07, IBM model). For the automatic model: not known. The SBC cost is in the first model (commit 176aa8e, 2026-11-19). The switch `sbc_cost` came in commit fc2afc2 (2026-11-20).

## When to examine it again
When the share count of the model changes (the pairing of SBC and the count). For a company with SBC larger
than the free cash flow (OKTA, RDDT, TWLO, TTWO, AXON, HOOD in the investigation).
