# A-006: horizon of the automatic model

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
The automatic model projects 10 years (`years` = 10 years), in annual steps. After year 10 the terminal value
uses the Gordon form (A-001). The classic engine has the same default: `projection_years` = 10.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model | `rust/src/dcf/auto.rs` | `Rules::years`; `--rules years=N` |
| Classic engine | `rust/src/dcf/classic.rs` | `DcfAssumptions::projection_years` |
| IBM driver model | `analysis/IBM/driver_model.toml` | `projection_years` = 10 |
| Fade of the growth | `rust/src/dcf/auto.rs` | the company mean fades to L by year 10 |

## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| 20 years, growth stays at the cap | `--rules years=20` | Not known. Measured only |
| 20 years, linear fade to 2.25% | `--rules years=20,long_cap=0.0225,fade_years=20` | Not known. Measured only |

## Why this choice
The code gives "owner rule" as the source (`auto.rs`, explain row "horizon": "owner rule, annual
steps, Gordon terminal value"). No file with the wording, the date or the name of the owner found.
The study treats it as a "convention" (`analysis/priors/dcf_investigation_2026-11-20.md`).
The IBM model also has 10 years (`analysis/IBM/REVIEW.md` and `DRIVER-REVIEW.md` do not change it).

## Effect on the result
Median value / price, 438 tickers, 2026-11-19 (`dcf_investigation_2026-11-20.md`, section 2):
`years=20` with the growth at the cap after the fade: +0.079 (stable set +0.046), median P50 +14%.
`years=20` with a linear fade to 2.25%: +0.009 (+0.015), +2%. `docs/HIDDEN-ASSUMPTIONS.md`, #260.
The effect depends on what happens to the growth after year 10, not on the length alone.

## Who decided and when
The code says "owner rule". The date and the wording are not known.

## When to examine it again
When the fade rule changes (A-001). When a sector study exists for a company with a long growth period.
