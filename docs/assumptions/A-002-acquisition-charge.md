# A-002: acquisition charge of the automatic model
Updated: 2026-11-21.
## Choice
The model charges `w` times the expected spend / revenue, 1.75% + 0.3925 x past (`acq_persist`;
past = the mean of the last 5 fiscal years, capped at 25%; the fit of the panel, see Limits).
Spend = `acquisitions_net_of_cash` plus `acquisitions_net_of_cash_second`, the second line only
when its value differs from the first line or the first line has no value (`acq_second`; a tag
renamed between 10-Ks gives the same value in both lines). `w` is the weight of the history in
the company mean of the growth. A year with no value counts as 0. The growth history stays.
The charge fades with the growth that it pays for (`acq_fade`): if the company mean is
above the fade level L, the credited growth above L falls linearly to zero along the drift ramp,
and the charge falls to zero in the same years. If the mean is at or below L, the charge is
constant. `acq_charge` and `acq_f` are never faded. Defaults: `acq_w` on (`--rules acq_w=1`),
`acq_fade` on (`--rules acq_fade=1`), `acq_second` on (`--rules acq_second=1`), `acq_keep` off (`--rules acq_keep=0`) (on: the charge keeps the share L / mean).
`--rules acq_persist=off` gives the past spend as the base (the rule of 2026-11-20); `acq_persist_a`, `acq_persist_b` set a + b x min(past, 25%).
## Where used
`docs/models/auto-path-model.md`; `rust/src/dcf/auto.rs` (`acq_w`, `acq_fade`, `acq_second`,
`acq_persist`, `acq_keep`); `rust/src/dcf/auto_pit.rs` (`fcff_of_rules`: the same spend).
## Alternatives considered
| Alternative | Command | Why not chosen |
|---|---|---|
| No charge (old model) | `--rules acq_w=0` | Overstates the free cash flow after acquisitions: -1.7% of revenue in year 1 |
| Constant charge | `--rules acq_fade=0` | The charge does not fade with the credited growth |
| First line only | `--rules acq_second=0` | Loses real cash paid (NVDA FY2026: $1.5B of $14.5B) |
| Past spend as the base | `--rules acq_persist=off` | Over-charges heavy buyers: error +1.1 to +1.8% of revenue (half 1) |
## Why this choice
None of the reasons uses the price. Study 2.6: a consistent pair by construction. The model
credits the share `w` of the bought growth, so it charges the share `w` of the spend. It matches
the reviewed IBM `f = 0.5`. The fade keeps the pair true in each year. The second line is real
cash paid for businesses: the charge and the test must count it. The charge is an expected
cash flow, so its base is the expected next spend, not the past spend: a heavy buyer spends
less later (study 2.1: the next 3 years are 0.55 of the past at the mean), and a company with
little past spend spends about 1.3 to 2.2% later. The fit is on half 0 and scored on half 1.
## Limits and options (ticket `T-20261120-dcf-acq-charge-limits`, data store 1804f48)
- **Level (a), adopted.** Fit: `fa dcf pit --half 0 --diag acq` (base revenue 1B and above,
  3,488 company-years, both means capped at 25%). Through the origin (0.5226 x past) is not
  chosen: on half 0 it makes the error of all rows worse (-0.8 to -1.4%). Class "spend above
  5%", free cash flow error T+1 / T+2 / T+3 (% of revenue), past spend to the fit: half 1
  +1.1 / +1.2 / +1.8 to -1.2 / -1.1 / -0.8, coverage +3.9 / +3.3 / +2.5 points; `--cut 2015
  --half 1` +1.2 / +1.7 / +2.5 to -0.7 / -0.2 / +0.2, coverage +5.7 / +4.8 / +2.6 points. No
  other row is worse by 2 points. Limits: the test is a median test and the charge is a mean,
  so the sign of the error flips; real values below P10 go from 19.6% to 23.2% (half 1, T+1).
  Screen: median P50 / price 0.4897 to 0.4966; ILMN -12.21 to -1.10; IBM 93.56 to 116.09.
- **Step (b).** At a mean just above L the charge went to zero; at a mean at or below L it was
  constant. With `acq_keep` the charge is `c0 x g(t) / mean` (g = the credited growth), so it
  ends at `c0 L / mean`: no step. A pit of 3 years cannot test it. Screen: median P50 / price
  0.4897 to 0.4576; ROP 357 to 186; HPE becomes not positive. ILMN and WMB do not change
  (mean below L: the constant charge is the pair). With (a): median P50 / price 0.475.
- **Second line (c).** 8 of 503 tickers of `sp500.txt` change the charge (BIIB, CBRE, J, MRK,
  NVDA 0.05% to 0.17%, TRGP, ZBH 0 to 2.66%, ZTS). MATX, LUMN and WT have the second line only
  before the last 5 years: no change.

## Who decided and when
The Head of Fundamental Research, on the word of the owner of 2026-11-20 (record:
`chiefs/louellet-git/fundamental/RECORD.md`). 2026-11-21, after the measurement: `acq_second`
and `acq_persist` on; `acq_keep` off. Open question (owner: the same head): is the growth at L
paid by organic reinvestment (then the charge goes to 0 for each mean) or in part by
acquisitions (then `acq_keep`)? Review (source rule 3): not done.
## When to examine it again
When sector data exists.
