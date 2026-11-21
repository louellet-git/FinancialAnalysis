# A-160: training window of the premium test

Updated: 2026-11-21. Code: branch econ/premium-test-cmd. Model document: `docs/models/premium-macro-test.md`.

## Choice
The first fit uses the first 104 kept weeks from 2019-01-04. The first out-of-sample week is the 105th kept
week (about 2021-01). Later weeks use all earlier kept weeks (expanding window).

## Where used
| Model | File | Parameter name |
|---|---|---|
| premium macro test | `rust/src/macro_test/design.rs`, line 8 | `TRAIN` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Rolling window | Not known. The design names only the expanding window. |
| Shorter training window | Not known. The design gives no other length. |
| Start of the sample in 2018 | The design excludes 2018: the beta is 1 for every name until about 2018-11. |

## Why this choice
The sample starts on 2019-01-04 (design rule 3). The reason: the beta is 1 for each name until about 2018-11.
See design rule 3 and rule 5.1 (frozen at 55f2e92). The design states 104 weeks (two years).
It also states that March 2020 is in training only. So the OOS window has about 310 weeks.

## Effect on the result
Not known. No sensitivity run is recorded. To test: change `TRAIN`, rebuild, run the same command on the same
files. Rule 10 of the design needs a new dated record for a change after the first result.

## Who decided and when
The Head of Economics, in the design (drafted and frozen 2026-11-21 at 55f2e92, after two review cycles of the Head of Quality and Risk).

## When to examine it again
After the first run of record, if the answer depends on the end of the training window, or if the sample
gets a new start date.
