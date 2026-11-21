---
id: T-20261121-shares-by-class
title: Give multi-class companies their full share count (the cover count by class)
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch: claude/shares-by-class
created: 2026-11-21
acceptance: V, MA, BRK-B, GOOG, GOOGL, META, PLTR, DELL, CMCSA and UPS get a share count from the latest 10-K or 10-Q that is the sum of their classes (class B of BRK in class B equivalents), point in time by the filing date; the market value check of the data audit is plausible for each; a fixed-answer test.
merged: pull request 156 (branch claude/shares-by-class)
---
From the data audit (T-20261121-data-audit). The SEC company facts drop `dei:EntityCommonStockSharesOutstanding`
when a filer gives it for each share class (dimension members). So the latest count is from 2010-2016, or one class
only (V $189B, MA $77B, BRK-B $0.5B as market values). The 10-K supplement already sums the class members for V
(`rust/mapcheck/src/supplement.rs`): fetch the latest 10-K and 10-Q filing files for these tickers (SEC, a few
requests each) and use the supplement, with the point-in-time rule of T-20261121-pit-supplement-fill. Next500:
HEI, UHAL, LEVI, AUR. Priority: the S&P 100 week attribution of Fundamental Research.

Scope after PR 152 (2026-11-21). The audit found the counts of MA, CMCSA, UPS, GOOG, META, DELL,
PLTR and MCD right (the latest quarter, within 2% of the class totals; PLTR +7% is dilution). PR 152
gives V (class A only, 1,907M) and BRK-B (2,157M class B equivalents) a count at a cut. Left: V
as converted (classes B and C convert into class A), a check of all ten and of HEI, UHAL, LEVI, AUR.

Result (2026-11-21, `fa dcf auto <T> --as-of 2026-11-13 --explain`, store e06070c). No code change.
All counts are within 3% of the class total, except PLTR (+7%, the dilution of options and RSUs).
V: 1,907M is the XBRL diluted weighted average of the class A member, which the 10-K (Note 16)
gives as converted (classes B-1, B-2, B-3 and C included). The period-end as-converted count is
1,867M (+2.1%, buybacks during the year). BRK-B: 2,157.3M class B equivalents (A x 1500 + B), exact.
MA -1.8%, GOOG and GOOGL +0.9% (one Alphabet total), META +0.8%, DELL +1.1%, CMCSA -1.0%, UPS +0.3%,
HEI +1.1%, UHAL equal, LEVI +1.2%. AUR has no value for another reason (history too short).
The store has filing files for V, MA, BRK-B, PLTR, CMCSA and UPS only.
