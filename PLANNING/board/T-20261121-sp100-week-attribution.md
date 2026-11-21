---
id: T-20261121-sp100-week-attribution
title: Valuation attribution of the drop of the S&P 100 in the week to 2026-11-20 (document of 3 pages or less)
area: research
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.005
branch: claude/sp100-week-attribution
created: 2026-11-21
acceptance: (1) One document of 3 pages or less gives the change of the S&P 100 (value-weighted, close of 2026-11-13 to close of 2026-11-20) as a sum of valuation parts (new facts, time, rates, implied premium or implied growth, the part with no model), by company and by sector, from `fa dcf attribute`; (2) the rates part uses one curve source at both dates (`--curve fedinvest`, because the GSW curve lags); (3) the four-step review of a document with numbers (validator, document reviewer, auditor) passes; (4) the commands and the store commit are in the document.
---
Owner words, 2026-11-21 (chat): "Also did you ever get me that 3 page doc on why the market has tanked this week? I want a
valuation, so each piece of the overall drop should be attributable to a piece of valuation. If it's easier to do company
by company then so be it, I'd like it on the S&P100. You can hold off on other work for now and focus on this."

Plan:
1. Code: `fa dcf attribute --curve par` (the NSS fit of `fa recon rich-cheap` to the Treasury par yields of each date; also `--curve fedinvest`), in
   the GSW units. Check on 2026-11-13, 10-year zero: par 5.129%, FedInvest 5.169%, GSW 5.135%; one
   source at both dates).
2. Data (Head of Technology): the 2026-11-20 closes and the S&P 100 member list (FedInvest and GSW have no 11-20 row).
3. Run: `fa dcf attribute --tickers-file <sp100> --from 2026-11-13 --to 2026-11-20 --curve par`, the S&P 10 list, and the S&P 500 as
   a check.
4. Document: `analysis/priors/sp100_week_attribution_2026-11-20.md`. The parts are an accounting split with the model, not
   causes. The value at the means is not the expected value.

Owner words, 2026-11-21 about 15:10 UTC: "Start w S&P10". The document starts with the 10 largest companies of the S&P
100 by market value at the close of 2026-11-13 (one weight for each company), company by company; the S&P 100 follows.

**State (2026-11-21 15:30 UTC):** the code is committed on the branch (`--curve par`, `--curve fedinvest`, the full bridge;
both suites pass: 833 + 17 + 1). The list `rust/mapcheck/universe/sp100.txt` (101 tickers, from the Head of Technology).
A first run to 2026-11-19 (the store ends there): S&P 100 -5.41%; premium-first bridge: rates +2.43, implied premium
-6.10, implied growth -0.76, no model value -1.05 points. Waits for: the 2026-11-20 closes in the store (Head of
Technology, refresh in progress). Next: the run to 2026-11-20, the document, the review.
Owner words, 2026-11-21 about 15:40 UTC: start with the S&P 100, then break down the model valuation of each company of the
S&P 10 so that the sum of the parts is the change of the S&P 10. Done in code: the full bridge has one line for each
ticker for a list of 20 tickers or fewer; the S&P 10 list is `analysis/priors/data/sp10_2026-11-13.txt`.

**Data defects found (2026-11-21 about 15:55 UTC; owner word: "If there's a problem with the data, that needs to be fixed"):**
1. mapcheck, point-in-time mapping (`run.rs`, `check_ticker_filed` with a filed cut): no fill from the filing supplement,
   so V, BRK-B, COP and NEE fail at a past date (`fa dcf implied V --as-of 2026-11-13`: "mapping status FAIL"), and V and
   BRK-B have no share count (no weight). Sent to the Head of Technology (the owner of mapcheck) with a proposed fix: use
   the supplement only when its filing was public on the date. Open until the fix merges.
2. The store file `rates/treasury_nominal_2026.csv` (58e54a2) has the 11/20/2026 row two times. The reader keeps one;
   sent to the Head of Technology.
Not data defects (model limits, stated in the document): AMZN and INTC have a value below 0 at the means (AMZN FY2025:
operating cash flow 139.5B, capex 131.8B, and the model counts all capex and stock pay as a cost); TSLA, AMD and PLTR
need a premium or a growth outside the search range; banks, insurers and REITs have no model template.

**State (2026-11-21 about 16:20 UTC):** correction: defect 2 above (a duplicate par row) was a false report of this head
(a `head -2` printed the row); the store file is correct. The store f634633 has the 2026-11-20 closes of the S&P 100.
Run to 2026-11-20 (not final): S&P 100 -9.40% (SPY 754.37 to 689.81, -8.56%); premium-first bridge: rates +4.93
(10-year zero 5.129% to 4.903%), implied premium -10.71, implied growth -1.68, no model -2.03, time +0.09, facts +0.01.
The prices to 2026-11-19 agree with the Alpaca IEX closes to a few cents. New data defect 3: IBM and JNJ have no close
after 2026-11-18, and `attribute` takes the last close up to 10 days older, so it uses a stale price (sent to the Head
of Technology; a later tool change: flag a price that is older than the date). The final run waits for defects 1 and 3
(owner: "Don't leave out Berkshire ... Have that fixed"). A draft document is in progress (an agent).

**State (2026-11-21 about 16:45 UTC):** code: `attribute` marks a stale price (a close older than the SPY close of the
same date) and leaves the row out of every sum (test; suites pass 834 + 17 + 1). With it, IBM and JNJ leave the S&P
100 sum (-9.48%, 96 companies) until their closes are in the store. The draft document
`analysis/priors/sp100_week_attribution_2026-11-20.md` (an agent; numbers of the run to 2026-11-20 before the fixes;
about 3.5 pages: cut after the final run). Open: data defects 1 and 3 (Head of Technology); the code gate (Quality);
the final run; the review of the document (validator, document reviewer, auditor). A point for the validator: in the
premium reading the rates part is large for names with an implied premium near 0 (AAPL about +10% of its price for
-23 bp); the growth reading and the order range are in the draft.

**State (2026-11-21 about 17:05 UTC). Owner: "Hold off on writing the doc until I give my go ahead on the results"; the
document and its review are ON HOLD (draft kept).** New owner items: (a) the S&P 10 breakdown over 2 months, done:
`analysis/priors/data/sp10_2m/` (nine weekly runs 2026-09-18 to 11-20 and one run over the whole period; store f634633,
`--curve par`). Whole period: -13.18% = facts +4.77 (AAPL FY2026 10-K, filed 2026-11-03: revenue 416.2B to 478.8B,
operating cash flow 111.5B to 145.1B), time +1.10, rates +1.39, premium -10.14, growth -9.79 (NVDA -9.16: no premium
view over the period), no model -0.51 (AMZN, TSLA). (b) Research on the implied premium: an agent writes
`analysis/priors/implied_premium_research_plan_2026-11-21.md` (literature, data inventory, first measurements, 3 to 5
proposals); the Head of Economics is asked whether it owns the market-level part.

**Premium research split (2026-11-21 about 17:25 UTC):** the Head of Economics owns the market-level part (the premium
common to all names against VIX, the variance risk premium, Baa-10Y, the real 10-year yield and NFCI; a design written
and frozen before any regression). This head gives two weekly point-in-time series from 2018-01-05 (S1: the list premium
of `fa dcf implied --price-date`; S2: the premium part of the `attribute` bridge), for the S&P 500 and the S&P 100, after
the point-in-time mapping fix merges. This head keeps the stock-level part (common against stock-specific; growth news
against discount-rate news). The data audit of the store is with the Head of Technology (owner request, 17:15 UTC).

**Premium research plan (2026-11-21 about 17:40 UTC):** `analysis/priors/implied_premium_research_plan_2026-11-21.md`
(an agent; not reviewed). Recommendation: first item 4.1 (the weekly split of the premium change into the part common to
all names and the part of each name, 2 to 5 years of `fa dcf attribute`); it is the base of 4.2 (the Economics
regression on VIX, credit, real yield, NFCI) and 4.3 (forecast revisions: growth news against discount-rate news).
Checked by this head: in the week to 11-20 the implied premium rose for all 65 S&P 100 tickers with a value at both
dates (+0.33 to +2.27 points, median +0.78). Daily collection of the estimates and of the SPX chain asked of the Head
of Technology (no history exists: one snapshot each).

**State (2026-11-21 about 17:55 UTC):** data defect 3 fixed by the Head of Technology (store 652dc28: IBM and JNJ on the
Nasdaq split basis to 2026-11-20). Run on 652dc28: S&P 100 -9.42% (98 companies, no stale price); premium first: facts
+0.01, time +0.09, rates +4.93, premium -10.73, growth -1.68, no model -2.03. The data audit of Technology (PR 143,
`T-20261121-data-audit`) found share counts by class wrong in the SEC cover tag for 10 S&P 100 names; `attribute`
weights by the model count (the diluted weighted average of the last 10-K), and for 8 of them it looks right (label in
the result); V and BRK-B have no count until `T-20261121-shares-by-class` merges (after the pit fix). Not final.

**Premium series built (2026-11-21 about 18:45 UTC):** `fa dcf premium-series` on branch `claude/premium-series`
(5671612, pushed; not reviewed; suites 841 + 17 + 1; the fixed CSV contract with Economics has a test). Smoke run S&P
100, store 0340bc1: S1 1.55% (11-06), 1.18% (11-13), 1.66% (11-20); the 11-20 row gives the same list change as
`attribute` (-9.42%); switch week 2026-11-20. Timing S&P 500: about 6.6 s a week (10.8 s with --bridge). Not done: a
poison test of a fact filed after the anchor (needs a real facts cache), the poison and truncation runs on 3 dates, the
code gate, the run of record (after the Technology fixes). Economics builds `fa macro premium-test` against the CSV.

**Mapping fix (2026-11-21 about 20:45 UTC, from Quality):** the Technology change `pit-supplement-fill` (f4d74ea, not merged,
REVISE for text only, no look-ahead) gives a value at cut 2026-11-13 to all 23 S&P 500 tickers that failed (BRK-B, V,
COP, NEE, CI, LHX, PSX, VLO, ...) and moves some values (value at the means: IBM 171.25 to 164.79, MA 401.39 to 381.49,
CMCSA 92.26 to 91.95, ED 96.70 to 99.31). The final runs use the merged code and say its commit. After the merge,
`docs/models/implied-and-attribute.md` states the rule: with a cut, the 10-K supplement fills an input only when its
filing date is on or before the cut, and the 10-K capex lines replace the company-facts capex (`fa dcf pit` does not use
this path).

**State (2026-11-21 about 21:30 UTC):** the data defects are closed. PR 152 (b01b66a, the point-in-time 10-K fill) gives
V, BRK-B, COP, NEE and 19 more a value; PR 156 (Technology): the share counts by class need no code change (V 1,907M:
"the diluted weighted average of the 10-K, as converted"; BRK-B 2,157.3M class B equivalents). The runs of
`analysis/priors/data/week_2026-11-20_main-b01b66a/` are the runs of the data of record: S&P 100 -9.22% (100
companies), S&P 10 -9.96% (`sp10_2026-11-13_v2.txt`, BRK-B in place of WMT). Open: the code gate of this branch, then
the go-ahead of the owner on the results; the document stays a DRAFT until then.

**Purpose and form of the report (owner, 2026-11-21 about 18:00 UTC).** Purpose: understand a recent price change of a
set of stocks, what contributed, and whether the implied pieces make sense against reality; conclude whether it is a buy
opportunity or a correction. Answers: the conclusion "should just be within the sub 3 page report"; the horizon
"Depends" (the head states the horizon that fits each conclusion). Report structure (3 pages or less): 1 the answer;
2 the bridge (S&P 100, then the S&P 10 by company, common and company parts); 3 the reality check of each implied piece:
(a) the level and the change of the implied premium against its own weekly history from 2019 (`fa dcf premium-series`)
and the index history from 1871 (`fa dcf market`), (b) the implied 10-year growth of each S&P 10 company against the base
rates of the panel, (c) the rates piece against the measured balance-sheet effect, (d) company news in the window (8-K
item 2.02), (e) the economics view when its file exists; 4 the conclusion with its horizon and what would change it;
5 limits and commands. The conclusion is a judgment from fundamentals, not a backtested trade rule; the report says so.
Still on hold: the writing waits for the go-ahead of the owner on the results.
