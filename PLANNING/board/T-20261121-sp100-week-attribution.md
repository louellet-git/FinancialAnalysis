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
