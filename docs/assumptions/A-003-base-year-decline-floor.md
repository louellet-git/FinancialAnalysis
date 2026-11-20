# A-003: Floor on the growth of the base year

Updated: 2026-11-20.

## Choice
When the organic revenue growth of the base year is below 0%, the automatic model uses 0% in its place for two
things only: the company mean of the growth and the first step of the growth process. The older growth values and the
shock size keep the real values. The floor value is 0%. A base year with growth of 0% or more is not changed.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Automatic path model | `rust/src/dcf/auto.rs` | `Rules.last_floor` (default `Some(0.0)`; `--rules last_floor=-1` is off) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Floor on all values of the history | It moves the company mean up for all companies. The error for the classes with growth above 10% becomes worse by 1 to 2 points at T+3. |
| Floor at +3% or +6% | Better for the decline class, but the median error of all company-years becomes -2.9% and -4.7% at T+3 (half 0). |
| A recovery term estimated on the panel | The recovery after a decline is mostly the years 2019 and 2020 (see below). A term that is estimated on this panel fits one episode. It needs a decision of the owner. |
| Phi = 0 for the growth | It gives T+1 right, but T+2 and T+3 stay 6% to 10% too low. |

## Why this choice
The panel (6,030 company-years, cache of 2026-11-19) shows that the growth of the next year after a decline year is
positive at each decline size: median +2.7% (decline 0 to 5%), +3.2%, +5.9%, +7.1% (decline above 20%).
A negative value in the company mean treats the decline as a rate that stays. The floor of 0% has no parameter that
is fitted. Command: `fa dcf pit --diag decline`.

## Effect on the result
Decline class (growth < 0%), median error of the revenue P50 at T+1 / T+2 / T+3, half 1 (the scoring half, split by the
hash of the ticker): +2.0 / +7.8 / +11.8% before, +0.0 / +5.1 / +7.4% after. With `--cut 2015`: +3.1 / +12.7 / +17.9% before,
+1.2 / +8.5 / +13.6% after. The error stays: it is mostly the base years 2019 and 2020 (the real revenue of 2021).
The classes with growth of 0% or more do not change. Side effect: the largest companies (revenue above $100B, 184 company-years
in the `--cut 2015` run) are inside the band in 97.3% of the cases at T+3, before 95.1%.

## Who decided and when
Built by the valuation quant; accepted by the Head of Fundamental Research, 2026-11-20 (ticket
`T-20261120-dcf-engine-accuracy`), on the word of the owner of the same date. The side effect on the largest
companies is accepted: about 4 company-years, in a row that was already above the target. A recovery term is refused:
it would fit one episode (the base years 2019 and 2020). Review: not done yet.

## When to examine it again
When the table is made again with a longer panel, or when an owner decision gives a recovery term.
