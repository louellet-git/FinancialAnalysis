# A-200: step of the cost of debt for each rating row

Updated: 2026-11-21. Commit: a66a70d (step 3), fix commit to follow.
## Choice
The cost of debt of a rating is the UST 10Y of the date, plus the HQM basis (A-203), plus a
fixed step for the rating row. The step is the default spread of the row minus the default
spread of the HQM reference bucket (A, record A-201). Percentage points:

| Row | Default spread | Step over A | Row | Default spread | Step over A |
|---|---|---|---|---|---|
| AAA | 0.40 | -0.38 | BB+ | 1.38 | +0.60 |
| AA | 0.55 | -0.23 | BB | 1.84 | +1.06 |
| A+ | 0.70 | -0.08 | B+ | 2.75 | +1.97 |
| A | 0.78 | 0.00 | B | 3.21 | +2.43 |
| A- | 0.89 | +0.11 | B- | 5.09 | +4.31 |
| BBB | 1.11 | +0.33 | CCC | 8.85 | +8.07 |
| CC | 12.61 | +11.83 | C | 16.00 | +15.22 |

Source: A. Damodaran, "Ratings, Interest Coverage Ratios and Default Spread" (spreads over
the par T-bond; table for large non-financial firms). Page: `https://pages.stern.nyu.edu/~adamodar/New_Home_Page/datafile/ratings.html`,
read 2026-11-21; it says "Data used is as of January 2026" and "The default spreads are
obtained from traded bonds". The data page (`.../data.html`) says the update was on 2026-01-09.
The table has a row for 14 notches (list in `bonds.rs`). A rating with no row
(AA+, AA-, BBB+, BBB-, BB-, CCC+, Aa1, Aa3, Baa1, Baa3, Ba3, Caa1 to Caa3) takes the row of its
letter grade, and the output has a note. D (default) has no cost of debt.
Use terms (`.../datahistory.html`, "Usage Rules"): "I want the data to be widely used and to
be a help, rather than a hindrance. Acknowledgements: If you do use my data and wish to
acknowledge that you did get the data off my site, I thank you." The numbers are facts that
we write in `rust/src/bonds.rs`. We store no copy of a data file.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Cost of debt (`docs/models/cost-of-debt.md`) | `rust/src/bonds.rs` | `DEFAULT_SPREAD`, `rating_step()` |
| Verdict with `--rating` | `rust/src/verdict_rates.rs` | `kd_spread` driver mean |
| Bond curve check | `rust/src/recon.rs` | default spread of A |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| ICE BofA option-adjusted spreads (FRED) | The terms do not permit a stored copy. FRED keeps 3 years only |
| Moody's Aaa and Baa yields (FRED) | The same terms. Two grades only. Long bonds |
| The letter grade only (no notches) | The table has a row for each notch; A- and A+ differ by 0.19 points |

## Why this choice
The page has an open use rule and one number for each notch. A reader can open it. The reason
does not use a stock price. The table changes once a year (January).
Check against a second source (ICE BofA, read from a local cache on 2026-11-18, not stored,
numbers not copied here): the step table agrees for AAA to BBB within about 0.1 points; it is
lower for the high yield rows, most for CCC.

## Effect on the result
For IBM (A-) the step is +0.11; BBB adds 0.33 points to the pre-tax cost of debt. The debt
weight is small in the IBM model. Command: `fa verdict IBM --price 228.53 --rf-date today --rating BBB`.

## Who decided and when
The users (Fundamental and Economics heads), 2026-11-21, ticket `T-20261121-fred-local`.

## When to examine it again
Each January, when the Damodaran table is updated. Also if the HQM reference bucket (A-201) changes.
