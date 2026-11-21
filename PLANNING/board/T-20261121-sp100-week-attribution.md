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
1. Code: `fa dcf attribute --curve fedinvest` (the NSS fit of `fa recon rich-cheap` to the FedInvest prices of each date, in
   the GSW units). Check: the fit against GSW on 2026-11-13 (10-year zero 5.169% against 5.135%; the level differs, so one
   source at both dates).
2. Data (Head of Technology): the 2026-11-20 closes, `fedinvest_2026-11-20.csv`, the S&P 100 member list.
3. Run: `fa dcf attribute --tickers-file <sp100> --from 2026-11-13 --to 2026-11-20 --curve fedinvest`, and the S&P 500 as
   a check.
4. Document: `analysis/priors/sp100_week_attribution_2026-11-20.md`. The parts are an accounting split with the model, not
   causes. The value at the means is not the expected value.
