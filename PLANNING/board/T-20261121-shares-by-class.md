---
id: T-20261121-shares-by-class
title: Give multi-class companies their full share count (the cover count by class)
area: importer
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch:
created: 2026-11-21
acceptance: V, MA, BRK-B, GOOG, GOOGL, META, PLTR, DELL, CMCSA and UPS get a share count from the latest 10-K or 10-Q that is the sum of their classes (class B of BRK in class B equivalents), point in time by the filing date; the market value check of the data audit is plausible for each; a fixed-answer test.
---
From the data audit (T-20261121-data-audit). The SEC company facts drop `dei:EntityCommonStockSharesOutstanding`
when a filer gives it for each share class (dimension members). So the latest count is from 2010-2016, or one class
only (V $189B, MA $77B, BRK-B $0.5B as market values). The 10-K supplement already sums the class members for V
(`rust/mapcheck/src/supplement.rs`): fetch the latest 10-K and 10-Q filing files for these tickers (SEC, a few
requests each) and use the supplement, with the point-in-time rule of T-20261121-pit-supplement-fill. Next500:
HEI, UHAL, LEVI, AUR. Priority: the S&P 100 week attribution of Fundamental Research.
