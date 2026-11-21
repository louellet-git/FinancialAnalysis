# Model: premium macro test (`fa macro premium-test`)

Updated: 2026-11-21. Code: branch econ/premium-test-cmd. Design: `analysis/macro/2026-11-21-premium-macro-design.md`,
frozen at 55f2e92; records R1 (e77f089, bc7da33) and R2 (2026-11-21, dNFCI excluded).

## Purpose
The market-level part of the question on the implied premium: does the weekly change of the premium that
the S&P list implies (S1) have a link with macro risk measures that the mechanical terms do not explain?
This is a measurement. It is not a cause, not a trade signal, and not an input of any valuation model.
The answer is "yes" or "no share shown".

## Inputs
| Input | Source | Note |
|---|---|---|
| Weekly series (S1, zero10, ret_w, the S2 parts) | CSV of `fa dcf premium-series`, option `--series` | The header is fixed (`HEADER` in `data.rs`). Decimals. An empty field is missing. |
| VIX close | `data/raw/vix/VIX_History.csv` (store, option `--cache`) | Cboe file. Private store. |
| SPY close | `data/raw/SPY/prices.json` (store) | Description only (SPY check). Not a regressor. |
| DFII10, T10YIE | FRED CSV, options `--dfii10`, `--t10yie` (optional) | Description only. Without them the block prints "not run". |

The command reads files only. It starts no download. The repo commits results only, never the weekly
T10YIE series (design rule 4).

## Method (short)
1. Build the weekly rows from 2019-01-04. y = change of S1 in percentage points.
2. Apply the data rules (design rule 4): daily series take the last value on or before the anchor, at most
   3 days old. A missing level drops the changes of week w and w+1. The
   switch week of the curve drops from all models.
3. Sanity stop (rule 7.2): the correlation of `s2_rates` with the change of zero10 must be below -0.5.
   If not, the bridge has a defect and the command stops with no result.
4. M0 = intercept, ret_w, ret_w^2, dzero10. M1 = M0 + dVIX (R2). M1 has no credit spread (R1) and no financial-conditions index (R2).
5. Fit by expanding-window OLS on the kept weeks in time order. The first fit uses 104 kept weeks.
6. The one test: Clark-West, M1 against M0, one-sided, Newey-West lag 8. Gain = 1 - SSE(M1) / SSE(M0).
7. "Yes" rule: p < 0.05 and gain >= 0.05.
8. Placebo: shift the block dVIX circularly by 13 to 52 weeks, both ways (80 runs). Apply the
   same "yes" rule to each run. The answer is "yes" only if the real result is "yes" and at most 4 of 80
   runs are "yes". Otherwise: "no share shown". This does not mean "zero share".
9. Descriptions (no p-value, no change of the answer): in-sample R2 and coefficients with Newey-West t;
   S2 in the same form; R2 in the stress windows; the variance split of S2; M0 with dREAL10 and dBEI10;
   the share of SSE of the 5 largest weeks; the SPY return next to ret_w.
10. Optional sensitivity (`--exclude-from`, `--exclude-to`): the same test without the seen weeks.
    The output states if the answer depends on them.
The 12 reading choices of the code where the design is unclear are in the module comment of
`rust/src/macro_test/mod.rs`. The coordinator accepted them on 2026-11-21. They set no value that a
person tunes, so they have no record.

## Outputs
Text on stdout: sanity check, counts (kept, dropped, drop reasons, carried values), the primary result
(OOS R2 of M0 "mechanical", gain, Clark-West t and p, placebo count and gain range, answer), and the
descriptions. With `--out FILE`: JSON of the results only, no weekly series. Run once for each list
(`--list` is a label only).

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-160 | Training window | 104 kept weeks | First fit; the first OOS week is the 105th kept week |
| A-161 | Newey-West lag | 8 | Rounded up from T^(1/3), about 7 for about 310 weeks |
| A-162 | The "yes" thresholds | gain >= 0.05, p < 0.05 | Both must hold |
| A-163 | Placebo shifts and limit | 13 to 52 weeks both ways; 4 of 80 | At most 5% of the runs |
| A-164 | ret_w squared in M0 | included | VIX is not linear in the return |
| A-165 | Staleness limit | 3 days | Daily series only |
| A-166 | BAA10Y and NFCI excluded | no credit spread, no NFCI | Design records R1 and R2 |
| A-167 | Sanity limit | correlation below -0.5 | The rates part of S2 must move against zero10 |

## Limits
- No credit spread after R1 and no financial-conditions index after R2 (the Chicago Fed terms do not permit a stored copy of NFCI). "No share shown" says nothing about those two risks.
- Survivorship: the lists are today's lists.
- S1 is a residual. A growth or cash-flow error of the model enters it. The residual absorbs model error.
- VIX mixes risk and risk aversion. A link with dVIX does not say which one moved.
- The window 2026-10-06 to 2026-11-20 is "seen" (design rule 9). Use the sensitivity run.
- A statistical link only. Most premium predictors fail out of sample (design rule 8).

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/macro_test/` (`mod.rs`, `data.rs`, `design.rs`, `stats.rs`, `extras.rs`) |
| Design | `analysis/macro/2026-11-21-premium-macro-design.md` (the design branch `econ/premium-macro-design`) |
| Series command | `fa dcf premium-series` (makes the CSV) |
| Command | `fa macro premium-test --series FILE [--dfii10 FILE] [--t10yie FILE] [--cache DIR] [--out FILE] [--exclude-from D --exclude-to D] [--list sp500\|sp100]` |
| Tests | `cargo test --manifest-path rust/Cargo.toml --locked macro_test`: loader, OLS, Newey-West, Clark-West, placebo, sanity and extras tests with fixed answers |

The constants are in `design.rs` (lines 8 to 13), `stats.rs` (line 4), `data.rs` (line 13) and
`extras.rs` (line 9). The values were chosen by the Head of Economics in the design, after two review
cycles of the Head of Quality and Risk. The sample start 2019-01-04 is in A-160.

## Backtest

Backtest: none applies, because the command is itself a point-in-time historical test (an expanding window, scored only out of sample, on data dated on or before each anchor), and it sets no value of a model. Its own pre-registration is the design file, frozen at 55f2e92.

## Review status
Not reviewed. The design is frozen (55f2e92 with R1 and R2). No review record of the code is known to this
document. No run of record is known to this document.
