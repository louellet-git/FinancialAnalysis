# A-004: equity risk premium of the automatic model

Updated: 2026-11-21. Commit: 90ed737 (base of the branch).

## Choice
The equity risk premium (ERP) is drawn for each path. Mean: 4.41%. Standard deviation: 0.75%.
Lower bound of the draw: 2% (`lo = min(mean, 0.02)`). The cost of equity is the risk-free rate (A-005)
plus the beta (A-007) times the premium. Unit: a yearly rate.
The reviewed IBM model has its own value: mean 4.5%, std 0.75%, lower bound 2%
(`analysis/IBM/driver_model.toml`, driver `erp`). That value does not change.
The JNJ first pass has 4.20% (Damodaran, 1 October 2026; `analysis/JNJ/driver_model.toml`).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model (`docs/models/auto-path-model.md`) | `rust/src/dcf/auto.rs` | `Rules::erp` = (0.0441, 0.0075); `--rules erp=V` |
| `fa dcf implied` (the search moves this mean) | `rust/src/dcf/auto_implied.rs` | `ERP_RANGE` (-4% to +20%) |
| ANT first pass | `analysis/ANT/driver_model_paths.toml` | `[discount] erp` |
| IBM driver model | `analysis/IBM/driver_model.toml` | driver `erp` (4.5%) |

## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| 3.5% | `--rules erp=0.035` | Not known. The study lists it as a test of the effect only |
| 4.23% (Damodaran, 1 January 2026) | `--rules erp=0.0423` | Not known. A person did not choose it |
| 4.5% (the IBM review value) | `--rules erp=0.045` | Not known |

## Why this choice
The code gives the source as "repo convention: Damodaran implied ERP, 4.41% in
`analysis/ANT/driver_model_paths.toml` (2026-11-19)" (`auto.rs`, explain row "equity risk premium").
The study `analysis/priors/dcf_conventions_study_2026-11-20.md` could not match the value
4.41% to a Damodaran publication. It finds 4.23% on 1 January 2026 (his post of 23 January 2026).
**So the value has no confirmed source.** The standard deviation 0.75% is the IBM review value:
"spans the post-2008 implied-ERP range" (`analysis/IBM/DRIVER-REVIEW.md`, row `erp`). The study also
says that the Damodaran premium assumes a growth after year 5 that equals the risk-free rate.
The automatic model grows below that rate (A-001). The model then values an average company
below the index by construction.

## Effect on the result
`fa dcf screen`, 438 tickers, 2026-11-19, `erp=0.035` against 4.41%: the median value / price
changes by +0.060 (stable set +0.053); the median change of the P50 is +12%
(`analysis/priors/dcf_investigation_2026-11-20.md`, section 2). The 4.41% value is a
"convention" in that study. `fa dcf implied` gives the premium that the price implies.

## Who decided and when
Not known. The value first appears in the repo on 2026-11-19 (commits 5b9d746, 7980f50 and the ANT first
pass cc32980). No file names the person.
The IBM value 4.5% and its range: IBM driver review (closed decision).

## When to examine it again
When a person finds a published source for the premium, or at a new Damodaran January update.
Before any use of the P50 / price ratio as a signal (`docs/HIDDEN-ASSUMPTIONS.md`, #260).
