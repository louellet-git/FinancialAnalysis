# A-200: step of the cost of debt for each rating bucket

Updated: 2026-11-21. Commit: not yet made.
## Choice
The cost of debt of a rating is the HQM 10-year spot yield plus a fixed step for the rating
bucket. The step is the default spread of the bucket minus the default spread of the HQM
reference bucket (A). All values are decimals, in percentage points here.

| Bucket | Default spread (Damodaran) | Step over A |
|---|---|---|
| AAA | 0.40 | -0.38 |
| AA | 0.55 | -0.23 |
| A (reference) | 0.78 | 0.00 |
| BBB | 1.11 | +0.33 |
| BB | 1.84 | +1.06 |
| B | 3.21 | +2.43 |
| CCC | 8.85 | +8.07 |

Source: A. Damodaran, "Ratings, Interest Coverage Ratios and Default Spread", data of
January 2026 (page read 2026-11-21, update of 2026-01-09),
`https://pages.stern.nyu.edu/~adamodar/New_Home_Page/datafile/ratings.html`. We use the
middle notch of each letter grade (Aaa/AAA, Aa2/AA, A2/A, Baa2/BBB, Ba2/BB, B2/B, Caa/CCC) from the table for
large non-financial firms. The table says that the spreads come from traded bonds.
Use terms (`.../datahistory.html`, "Usage Rules"): "I want the data to be widely used and to
be a help, rather than a hindrance. Acknowledgements: If you do use my data and wish to
acknowledge that you did get the data off my site, I thank you." The numbers are facts
that we write in `bonds.rs`. We store no copy of a data file.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Cost of debt (`docs/models/cost-of-debt.md`) | `rust/src/bonds.rs` | `DEFAULT_SPREAD`, `rating_step()` |
| Verdict with `--rating` | `rust/src/verdict_rates.rs` | `kd_spread` driver mean |
| Bond curve check | `rust/src/recon.rs` | mean of AAA, AA, A default spreads |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| ICE BofA option-adjusted spreads (FRED) | The terms do not permit a stored copy. FRED keeps 3 years only |
| Moody's Aaa and Baa yields (FRED) | The same terms. Two grades only. Long bonds |
| Own table from a default study | No public study with yields by rating was found with clear terms |

## Why this choice
The page has an open use rule and one number for each notch. A reader can open it. The
reason does not use a stock price. The table changes once a year (January).
Check against ICE (read from a local cache on 2026-11-18, not stored): AAA 0.47, AA 0.67,
A 0.77, BBB 1.16, BB 2.05, B 3.73, CCC 13.53 (percent OAS). The table agrees for AAA to
BBB (within 0.08) and is lower for the high yield grades, most for CCC.

## Effect on the result
For IBM at A-, the step is 0: the cost of debt is the HQM 10Y spot yield. A step of +0.33
(BBB) adds 0.33 points to the pre-tax cost of debt. The debt weight is small in the IBM model,
so the effect on the value is small. Command: `fa verdict IBM --price 228.53 --rf-date today --rating BBB`.

## Who decided and when
The users (Fundamental and Economics heads), 2026-11-21, ticket `T-20261121-fred-local`.

## When to examine it again
Each January, when the Damodaran table is updated. Also if the HQM reference bucket (A-201) changes.
