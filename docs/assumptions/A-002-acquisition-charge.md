# A-002: acquisition charge of the automatic model

Updated: 2026-11-20.

## Choice
The model charges `w` times the mean of `acquisitions_net_of_cash / revenue` of the last
5 fiscal years. `w` is the weight of the history in the company mean of the growth (the
shrink weight, printed in the row "growth: company mean"). A year with no reported value
counts as 0. The charge is a fixed cost for each year of the forecast. The growth history
stays as it is.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model (`docs/models/auto-path-model.md`) | `rust/src/dcf/auto.rs` | `Rules::acq_w` = `true` |
| Point-in-time test | `rust/src/dcf/auto_pit.rs` (`fcff_of_rules`) | the real free cash flow is after acquisitions when `acq_w` is on |
| Reviewed IBM model | `analysis/IBM/driver_model.toml` | not used; the IBM review F2 and F16 have `f = 0.5` |

## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| No charge (the old model) | `--rules acq_w=0` | The model overstates the free cash flow after acquisitions: median error 1.7% of revenue in the next year |
| Full charge | `--rules acq_charge=1` | Too much: the shrink of the growth already takes out much of the bought growth |
| Charge a share `f` and cut the bought growth | `--rules acq_f=F,acq_mult=M` | Needs an outside number (`acq_mult`) |

## Why this choice
None of the reasons uses the price.
The study (section 2.6) shows that this is a consistent pair by construction: the model
credits the share `w` of the growth that the spend bought, so it charges the share `w` of
the spend. It needs no outside number. It matches the reviewed IBM `f = 0.5`. In the
point-in-time test, the error of the free cash flow after acquisitions goes from
-1.7 / -1.4 / -0.6 to -0.4 / 0.0 / +0.8 % of revenue (median, 1 / 2 / 3 years after the base year).
Source: `analysis/priors/dcf_conventions_study_2026-11-20.md`.

Known limits:
- Heavy buyers (spend above 5% of revenue) are over-charged in the point-in-time test:
  +1.8 / +2.2 / +3.1 % of revenue (1 / 2 / 3 years after the base year).
- The spend is lumpy. The 5-year mean can hold one large deal.
- 33% to 48% of company-years have no reported spend and count as 0.

## Effect on the result
Study, 430 tickers, median P50 / price: no charge 0.489; `acq_w=1` 0.382; full charge
0.309. The run with both new defaults: not yet run. Example on 2026-11-20, AAPL, seed 7:
the charge is 0.01% of revenue. The effects of several changes do not add.

## Who decided and when
The Head of Fundamental Research, on the word of the owner of 2026-11-20 (record:
`chiefs/louellet-git/fundamental/RECORD.md`).
Review of the same weight as the IBM review (`CLAUDE.md`, source rule 3): review: not done yet.

## When to examine it again
When a study of the persistence of the spend exists (panel: next 3 years =
1.39% + 0.39 x past), or when sector data exists.
