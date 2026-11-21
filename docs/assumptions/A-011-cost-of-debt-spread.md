# A-011: cost of debt spread of the automatic model

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
The cost of debt is the risk-free rate plus a spread of 1.50% (`kd_spread` = 1.50%), constant. The tax shield
uses the statutory rate 21% (`statutory_tax`) when the effective rate is not stable. The cost of debt
counts only through the equity weight E / (E + debt). The code limits that weight to 0.2 to 1.0 (a clamp; its source is not known). The cost of equity is CAPM (A-004, A-005, A-007).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model | `rust/src/dcf/auto.rs` | `Rules::kd_spread` = 0.015, `statutory_tax` = 0.21; `[discount] kd_spread` of the model file |
| ANT first pass | `analysis/ANT/driver_model_paths.toml` | `kd_spread` = 0.02 |
| IBM driver model | `analysis/IBM/driver_model.toml` | driver `kd_spread` = -0.10% (a different, reviewed value) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| -0.10% (IBM: fair value of the debt ladder) | A value for one issuer with a short ladder; not a rule for all tickers |
| 2.0% (ANT first pass) | Not known |
| Equity weight 1.0 (no debt in the rate) | `--rules equity_weight_one=1`; measured only |

## Why this choice
The code says "judgment (a spread of an investment grade issuer); it counts only through the equity
weight" (`auto.rs`, explain row "cost of debt"). No source, no date and no name found. The IBM value
and its calibration are in `analysis/IBM/DRIVER-REVIEW.md` (row `kd_after_tax`): the review calibrates the
spread to the fair value of the debt of the issuer.

## Effect on the result
No sensitivity of `kd_spread` alone is known. `equity_weight_one=1` changes the median value / price by
-0.052 (stable set -0.040) (`analysis/priors/dcf_investigation_2026-11-20.md`, section 2).

## Who decided and when
Not known. The value is in commit 176aa8e (2026-11-19, automatic model).

## When to examine it again
When a spread from the debt of the company or from its rating is available. When a sensitivity run exists.
