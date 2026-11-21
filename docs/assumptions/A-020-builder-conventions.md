# A-020: smaller conventions of the automatic model builder

Updated: 2026-11-21. Commit: 90ed737 (base).
Model document: `docs/models/auto-path-model.md`.

## Choice
One record for the defaults of `Rules::default` that have no record of their own. Each is a convention that
a person did not study. The value is read from `rust/src/dcf/auto.rs` (`Rules::default`). The register is
`fa dcf assumptions`.
| Name (`--rules` key) | Default | Meaning |
|---|---|---|
| `min_years` | 3 years | least fiscal years in the chain (two growth values) |
| `HIST_MARGIN`, `HIST_RATIO` | 5 and 3 years | years of the margin history and of the SBC and ratio history |
| `default_slow_half_life` | 4 years | fade half-life when the table has no fade cells |
| `own_sigma_nu` | 4 years | weight of the table shock size against the company residuals |
| `level_kappa` | -1 | shock at least the ratio of the fade table times the growth level |
| `margin_pool_se` | 0.08 | margin phi of all sizes when the standard error of a cell is above this |
| `fade_to_cap` | on | the company growth fades to L |
| `terminal_growth` | 2.25% (std 0.75%) | terminal mean when `long_run` is off; the std stays 0.75% |
| `stub`, `mid_year` | on, off | discount from the model date; flows at the year end |
| `bridge_lti`, `bridge_leases` | on, off | long-term investments are cash; leases are not debt |
| `no_interest_income` | on | interest income is out of the free cash flow (IBM F1a, F15) |
| `shares_latest` | on | latest count plus dilution |
| `history_organic` | on | an organic jump (no cash for businesses) stays in the growth history |

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model | `rust/src/dcf/auto.rs` | the names above |
| Driver engine guard | `rust/src/dcf/driver.rs` | `TG_WACC_GAP` = 2% (terminal growth below the cost of capital by 2%) |

## Alternatives considered
Each row has a `--rules` key. The study `analysis/priors/dcf_investigation_2026-11-20.md` (section 2) tested
`fade_to_cap=0`, `bridge_leases=1`, `bridge_lti=1`, `no_interest_income=1`, `terminal_growth=0.03`.

## Why this choice
Rows with a reason in a file: `own_sigma_nu`, `level_kappa`, `margin_pool_se` (`docs/HIDDEN-ASSUMPTIONS.md`
#188, #191 and #286: a measured effect on the coverage), `no_interest_income` and `bridge_lti`
(the investigation, section 4.2: a defect that was corrected), `stub` (#261). Other rows: convention, no source.

## Effect on the result
Median value / price, 438 tickers, 2026-11-19 (investigation, section 2): `fade_to_cap=0` +0.028;
`terminal_growth=0.03` +0.029; `bridge_leases=1` -0.018; `bridge_lti=1` +0.011;
`no_interest_income=1` -0.024; both of the last two: -0.007. Others: no sensitivity known.

## Who decided and when
Not known by name. The values are in commits 176aa8e (2026-11-19) and fc2afc2 (2026-11-20).

## When to examine it again
When one row binds in a review. A row that changes the median by 0.05 or more deserves its own record.
