---
id: T-20261121-premium-company-part
title: What explains the company part of the change of the implied premium (the change of a company less the common change)
area: research
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.006
branch: claude/premium-series
created: 2026-11-21
acceptance: (1) The design below is committed before any analysis of it; (2) `fa dcf attribute` writes, for each ticker, the beta of each date and the sensitivity of the value to the premium (d ln V / d premium at the premium of D1); (3) a description of the week 2026-11-13 to 11-20 for the S&P 100 (no test, about 65 names); (4) the test on the weekly history waits for the premium series (`T-20261121-sp100-week-attribution`, item 4.1 of the research plan) and has its own design, written before its run.
---
Owner words, 2026-11-21 about 19:00 UTC: "For the difference i.e attributable to the indiviudal company, that is somethign to be
investigated/understood." Context: in the week to 2026-11-20 the implied premium rose for all 65 S&P 100 tickers with a
value at both dates (median +0.78 points, the common part used now). The company part is the change of a company less
that median: NVDA +1.49, META +0.29, GOOG -0.30, MSFT -0.32, AAPL -0.44, WMT -0.45.

Design (Head of Fundamental Research, written before any analysis):
1. **Identity.** For a small change, the price change r_i of a company is about -D_i x dx_i plus the rates, time and facts
   parts, where D_i = -d ln V / d premium (the sensitivity of the value to the premium at D1) and dx_i the change of its
   implied premium. So dx_i is about -(r_i - other parts) / D_i. Two companies with the same price fall get different dx
   when their D differs. The company part therefore has three sources, measured in this order:
   - (a) **Sensitivity (mechanical).** D_i differs: a long-duration value (a low premium, a high growth) moves more for
     the same dx. Measure: D_i from the engine (two values at x1 +- 0.1 point).
   - (b) **Market exposure.** The price change has a market part, beta_i x r_m. Measure: beta_i of the model (D1 and
     D2), r_m = the value-weighted change of the list.
   - (c) **Company news.** The rest of the price change, r_i - beta_i x r_m. Candidates, fixed now: an earnings release in
     the week (SEC 8-K item 2.02 filed between D1 and D2; the model facts change only at the 10-Q or 10-K; in this week:
     NVDA 2026-11-17, WMT 2026-11-19); the sector (FF12); the level x1 (a premium near 0 gives a steep reading).
2. **Week of 2026-11-20 (description only).** One table for the S&P 100: dx_i, D_i, beta_i, r_i, the market part, the
   company return, the 8-K flag, the sector. The share of the cross-section variance of dx that (a), (b) and (c) give,
   in this order (the order is fixed; the parts do not add when they are not orthogonal: both orders are shown). No
   p-value and no claim with about 65 names in one week.
3. **History (a test, later).** With the weekly premium series, the same split each week from 2019; the design of that
   test (a pooled cross-section with week effects, standard errors clustered by week, an out-of-sample check) is written
   and frozen before its run.
4. **Rules.** A market-implied value is a measurement, not an input; the value at the means is not the expected value; no
   input of the model is set from the price; the model cannot tell a premium change from a growth change (both readings
   are shown); a residual is not a cause.

**Design change (owner, 2026-11-21 about 20:10 UTC: "I think that the implied premium/implied growth should also be part of
the attribution engine"). Written before any code or run of it.** The split goes into `fa dcf attribute` itself:
1. The common change c of a view: the median of (x2 - x1) over the tickers of the run with that view (a single-ticker run
   takes c from `--common-list FILE`, else c = 0 and the line says so). Option `--common vw` takes the value-weighted mean
   in place of the median; the median is the default, fixed now.
2. For each ticker, the plug of the view (ln N4 / N2) splits into two engine steps in sequence: common = ln V(x1 + c) -
   ln V(x1), company = ln V(x2) - ln V(x1 + c), both at the facts, curve, beta and weight of D2 (the inputs of node N2).
   The two add exactly to the plug (no linear step). A value at x1 + c that the engine cannot give puts the whole plug in
   the company part, with a flag.
3. Beta is inside the valuation (cost of equity = rf + beta x premium), so a common premium change already acts more on a
   high-beta name; there is no beta line.
4. The full bridge gets the columns premium-common, premium-company, growth-common, growth-company (they add to the old
   premium and growth columns), and one group line: the tickers with an earnings 8-K (item 2.02) in the window.
5. The CSV gets the columns c_premium, c_growth, p_premium_common_pct, p_premium_company_pct, g_growth_common_pct,
   g_growth_company_pct at the end; the CSV of `fa dcf premium-series` does not change.
Tests: the two parts add to the plug; c = 0 gives company = plug; a hand-made list with a known median; the group line of
the 8-K names adds to the sum of their rows.

**Design change 2 (owner, 2026-11-21 about 20:30 UTC: "For attribution, there are numbers in the valuation that are a
direct result of the math, but there are some that are implied. The implied components require the view of the
economists." and "The economists require their own engine in that regard.").** The math parts (facts, time, rates) stay in
`fa dcf attribute`. The implied parts (premium, growth) take their common change from the view of an Economics engine
(owner: the Head of Economics), read as a file: `--common file --common-file F` (header `date,erp_change,growth_change,
source`, decimals, the change over the window, matched on D2). The plug then splits into "common (economics view)" and
"company (residual)". The list median and the value-weighted mean stay as measured comparators. Condition: the inputs of
the economics view never come from the prices that the attribution explains (else the split is circular). Dependency: the
Economics engine and its first file (asked 2026-11-21, answer open).

**Design change 3 (2026-11-21 about 21:00 UTC; agreed with the Head of Economics, review REVISE of its view design):**
for `--common file` (the economics view), the common step applies BOTH changes together, in BOTH views: the premium of
the start node + erp_change, and G of A-001 + growth_change (`Rules.long_run`; L = min(10-year zero, G), so L moves only
where G binds). The common part is that one value change; the company part is the rest of the plug. growth_change does not
go to the growth-shift free variable (a shift of the growth path of years 1 to 10). An EMPTY field (no release in the
window) means "no information": the plug of that view is labelled "not covered by the economics view", and the text gives
the count of covered rows. The list median and the value-weighted mean keep the rule of design change 1 (c on the free
variable of the view).

**Gate verdict on claude/premium-series (a993e68), cycle 1 (20:15 UTC, 2026-11-21): PASS WITH CONDITIONS.** The code and the
method passed (code-reviewer, model-validator, document-reviewer of the Head of Quality and Risk). The three document
conditions D1 to D3 are fixed in ee82144 on claude/premium-series (pushed); Quality checks that commit itself.
TO DO in the next code commit on claude/attribution-reality, before its gate request (the MIN points, no block):
1. The test of the step h: assert 0.001 with a literal, and assert each bound alone.
2. The usage text "the last four columns" is now ten columns (ATTRIBUTE_USAGE); the test name at auto_attribute_tests.rs:584.
3. `--common-file` without `--common file`, and `--common-list` with `--common file`: make each an error (now ignored).
4. csv_line: `idx(name)` with unwrap_or(0) can write over the ticker column; use expect.
5. mark_stale gives 0 with no message when SPY is missing: print a warning.
6. An empty beta means 1.0: write 1.0 and a flag.
7. Documents: the common step uses the beta and the weight of D1 (the design text says D2); the company part holds the
   change of beta and weight; the default c is the median of the same prices that it explains, so the company part is
   not news; the long sentences of premium-series.md (Purpose, Method 3 and 5).
8. Merge ee82144 into claude/attribution-reality (the same documents are on both branches).
Before the series run of record: the poison and truncation tests on 3 real dates must also cover a 10-K/A after the
anchor, a split after the anchor, the GSW and par rows and the switch week, a holiday Friday, every column, and the cut
table. The first three dates (2020-03-20, 2022-06-17, 2025-04-04) are in `analysis/priors/data/premium_series_pit/`.

**Pull request 190 (20:32 UTC, 2026-11-21):** claude/premium-series (9f8863c, main c251fa3 merged; 966 + 17 + 1 and 126 tests
pass; the IBM headline is unchanged; the S&P 10 smoke test gives -9.96%). After the green check: merge, then the run of
record of the week on main (the commands of pages 2 and 3 with `--curve par`), then merge main into
claude/attribution-reality. The second branch waits for its gate verdict (a0e2480).

**Golden case (20:49 UTC, 2026-11-21).** Pull request 190 is merged (main 7e88cfe). It changed the output of
`fa dcf attribute`, and the stored-output test of the Head of Quant Research (`fa selftest golden`, case
`dcf_attribute_panel32`) failed on main. Pull request 195 (claude/golden-attribute-pr190) has the new expected text;
it waits for the look of the Head of Quant Research and the green check; then merge it.
TO DO in the pull request of claude/attribution-reality: run `fa selftest golden` (it needs the data store of the case
as data/raw; FA_REPO names the repo root) and update each case that the new default curve and the beta flag change,
with a reason. Rule from now on: `fa selftest golden` is a release check of each change of a command output.

**Merged (21:06 UTC, 2026-11-21).** Pull requests 190, 195 and 199 are merged (main 3fc1acc); the code list of gate cycle 1
is done. STILL OPEN on this ticket: (1) the series run of record (S&P 500 from 2019-01-04): first a review of the pit
results of 61fe8de against the eight points of Quality, and a parameter table cut before 2019 (`--table`), or a header
that states the look-ahead; then send the commit and the switch week (2026-11-20) to the Head of Economics; (2) a
truncation test of the base rates in the suite; (3) the long sentences of `docs/models/premium-series.md`; (4) the
register rows of `assumptions.rs` for A-027 to A-037.

**Point-in-time tests of the series on main (22:27 UTC, 2026-11-21).** The Head of Quality and Risk reviewed the first
results (61fe8de): REVISE (P1 the split basis, P2 the par path, P3 the code of main, P4 the parameter table). New tests:
`analysis/priors/data/premium_series_pit_main/` (result.md, cmp.txt, the scripts, the CSV files): 5 dates truncation
and poison, the par path with a switch week, the share basis of D, six controls; all pass on main 99c5af6. Sent to
Quality for its check. STILL OPEN before the series run of record: a parameter table cut before 2019 (none exists), or
the label "parameters in-sample" in the header and the document.

**Series run of record, S&P 100 (22:46 UTC, 2026-11-21).** Pull request 208 (the point-in-time tests on main) is merged.
The series of the S&P 100 list from 2019-01-04 ran on main e2557b6 (412 weeks, 12 minutes): median 3.56%, lowest 1.20%
(2026-11-13), 2026-11-20 1.67% (98.1% of the weeks before are higher), highest 6.67% (2020-03-20). Folder:
`analysis/priors/data/premium_series_sp100_2019_record_main-e2557b6/`, label "parameters in-sample". Page 2 of the week
study: the row is no more "preliminary"; its numbers did not change. STILL OPEN: the series of the S&P 500 list (it
runs; then send the commit and the switch week 2026-11-20 to the Head of Economics); a cut parameter table; a
truncation test of the base rates in the suite; the long sentences of premium-series.md; the register rows of
`assumptions.rs`.

**Series run of record, S&P 500 (23:31 UTC, 2026-11-21).** 412 weeks on main e2557b6 (46 minutes): median 4.19%, lowest
2.50% (2026-11-13), 2026-11-20 3.14% (94.9% of the weeks before are higher), highest 7.77% (2020-03-20); switch week
2026-11-20. Folder: `analysis/priors/data/premium_series_sp500_2019_record_main-e2557b6/` (S1 only, no bridge; label
"parameters in-sample"). Sent to the Head of Economics with the commit and the switch week, for its frozen test.
