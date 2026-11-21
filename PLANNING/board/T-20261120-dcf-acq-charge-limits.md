---
id: T-20261120-dcf-acq-charge-limits
title: Correct the two limits of the acquisition charge of the automatic model
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 1.4
branch: claude/dcf-acq-charge-limits
created: 2026-11-20
acceptance: The charge of a heavy buyer follows the persistence of the spend that the panel shows, the charge has no step at a company growth mean equal to L, and the point-in-time error of the class with a spend above 5% of revenue is near zero out of sample.
---
Condition of the gate review of `T-20261120-dcf-conventions-decision` (Head of Quality and Risk, cycle 2,
2026-11-20, PASS-WITH-NOTES). Record: `docs/assumptions/A-002-acquisition-charge.md`, section "Limits".

(a) **The level does not follow the persistence of the spend.** With the fade, years 1 to 3 keep about
100 / 92 / 83% of `w` x the past mean spend (about 0.52 x the past for ROP). The panel of the conventions
study (`analysis/priors/dcf_conventions_study_2026-11-20.md`, section 2.1) gives a spend in the next 3 years of
0.55 (mean) and 0.24 (median) of the past for a spend above 5% of revenue. Point-in-time test of that class
(1,650 company-years): free cash flow error +1.8 / +1.8 / +2.4% of revenue; 63.4% inside P10 to P90 (19.5%
below P10, 17.1% above P90); old model after acquisitions 68.1% inside.

(b) **A step at a company mean equal to L.** Above L the charge fades to 0 and is 0 in the terminal value;
at or below L it is constant and stays in the terminal value. ILMN (mean 3.19%, charge 7.03%): P50 -$12.08
against +$11.33 in the old model. WMB (mean 3.46%): -$2.18 against +$3.74. A company at 4.01% gets no charge
in the terminal value.

Rules: a correction has a reason that does not use the price; a parameter from the panel is estimated with
a fixed rule and scored out of sample (`fa dcf pit --half`, `--cut 2015`).

(c) **The second acquisition line** (`acquisitions_net_of_cash_second`, pull request 103 of the Head of Technology): the charge reads only the first line now (NVDA FY2026: $1.5B of $14.5B; ZBH, MATX, LUMN, WT have no charge). Add it only when its value differs from the first line or the first has none (a tag rename between 10-Ks gives the same value: EQT FY2021, QSR FY2023, MGY FY2022).
