# A-002: acquisition charge of the automatic model

Updated: 2026-11-20.
## Choice
The model charges `w` times the mean of `acquisitions_net_of_cash / revenue` of the last
5 fiscal years. `w` is the weight of the history in the company mean of the growth (row
"growth: company mean"). A year with no reported value counts as 0. The growth history
stays. The charge fades with the growth that it pays for (`acq_fade`): if the company mean
is above the fade level L, the credited growth above L falls linearly to zero along the
drift ramp, and the charge falls from its year 1 value to zero in the same years (same
timing). If the mean is at or below L, the charge is constant. `acq_charge` and `acq_f`
are never faded. Defaults of the code: `acq_w=on`, `acq_fade=on`.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model (`docs/models/auto-path-model.md`) | `rust/src/dcf/auto.rs` | `acq_w` = true, `acq_fade` = true |
| Point-in-time test | `rust/src/dcf/auto_pit.rs` (`fcff_of_rules`) | real free cash flow is after acquisitions when `acq_w` is on |

## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| No charge (old model) | `--rules acq_w=0` | Overstates the free cash flow after acquisitions: -1.7% of revenue in year 1 |
| Constant charge | `--rules acq_fade=0` | The charge does not fade with the credited growth: the pair holds in year 1 only |

## Why this choice
None of the reasons uses the price. Study 2.6: a consistent pair by construction. The
model credits the share `w` of the bought growth, so it charges the share `w` of the spend;
no outside number; it matches the reviewed IBM `f = 0.5`. The fade keeps the pair true in
each year (decision of the Head of Fundamental Research, review cycle 1). Direction
from study 2.1: high spenders spend 0.55x (mean) and 0.24x (median) as much in the next 3
years. The point-in-time test of the free cash flow is in sample and a median test, while
the charge uses the mean spend: it supports the rule and does not select it.

## Limits
- Heavy buyers (spend above 5% of revenue, 1,650 company-years) are over-charged: free
  cash flow error +1.8 / +1.8 / +2.4% of revenue at T+1 / T+2 / T+3 (constant charge:
  +1.8 / +2.2 / +3.1%; old model -3.3 / -2.9 / -2.4%). A P50 at or below 0 of such a
  company is an effect of the charge, not a value. Their band: 63.4% inside P10 to P90 (19.5%
  below P10, 17.1% above). The spend is lumpy. Ticket `T-20261120-dcf-acq-charge-limits`.
- 33% to 48% of company-years have no reported spend (0). In the screen 147 of 431
  tickers (34.1%) have no charge: the tag `PaymentsToAcquireBusinessesNetOfCashAcquired` is missing.

## Effect on the result
Data of 2026-11-19 (data store commit 41116d0). `fa dcf screen`, `sp500.txt`, 4,000 paths,
seed 7, 430 tickers, median P50 / price: old 0.4895; fading charge alone (`long_run=off`)
0.4245; constant charge alone 0.382; both defaults 0.4745 (constant: 0.427). P50 at or below
0: 38 (old), 39 (new), 50 (constant). ROP (price $470.87) P50: $438.34 old, $356.60 new,
-$90.95 constant. ILMN and WMB stay not positive (no drift: constant charge). `fa dcf pit`
(`analysis/priors/auto_pit_*_2026-11-20.txt`), free cash flow after acquisitions, error
T+1 / T+2 / T+3: -1.7 / -1.4 / -0.6% (old, `acq_f=0,acq_mult=0`), -0.4 / -0.1 / +0.6%
(new). Worse: coverage P10 to P90 77.3 / 78.0 / 78.4% (old), 76.3 / 77.4 / 77.8% (new).

## Who decided and when
The Head of Fundamental Research, on the word of the owner of 2026-11-20 (record:
`chiefs/louellet-git/fundamental/RECORD.md`). Review (`CLAUDE.md`, source rule 3): not done yet.

## When to examine it again
When a study of the persistence of the spend exists (panel: next 3 years =
1.39% + 0.39 x past), or when sector data exists.
