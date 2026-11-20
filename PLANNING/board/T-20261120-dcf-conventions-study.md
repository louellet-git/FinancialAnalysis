---
id: T-20261120-dcf-conventions-study
title: Study the long-run growth convention and the acquisition charge of the automatic DCF
area: engine
status: in-progress
owner: louellet-git
branch: claude/dcf-conventions-study
created: 2026-11-20
acceptance: A findings file gives, for each of the two conventions, the evidence from the company panel and from named sources, the effect of each alternative on the point-in-time test and on the return test, and one recommendation with its reason.
---
Owner request, 2026-11-20: investigate the recommendations of
`analysis/priors/dcf_investigation_2026-11-20.md`. This ticket covers two of them:

1. The model uses 5.13% (the risk-free rate) as the level that growth fades to, and
   2.25% as the terminal growth. Choose one long-run number, with evidence.
2. Acquisitions have no charge in the automatic model. A full charge moves the median
   of value divided by price by -0.16. Find the correct paired rule (a charge for the
   acquisition spend with a credit for the growth that it buys).

No default changes in this ticket. The owner decides after the findings.

Result, 2026-11-20 (not yet committed): the findings file is
`analysis/priors/dcf_conventions_study_2026-11-20.md` (evidence in
`analysis/priors/dcf_conventions_study_2026-11-20_panel.txt`).

1. One long-run growth. Recommendation: one number `L = lower of the 10-year zero yield and the
   long-run nominal growth of the economy` (now 3.8%) for the fade level and the terminal growth.
   Option `--rules long_run=0.038` (off by default). Ratio +0.057. The value 3.8% comes from a
   secondary report of CBO: the owner must check it.
2. Acquisition charge. Recommendation: charge `w` times the mean spend, with `w` the weight of the
   growth history in the shrink; history unchanged. Option `--rules acq_w=1` (off by default).
   Ratio -0.107. The IBM `f = 0.5` is near `w = 0.55`.

The owner decides both defaults. A default change of the charge needs a review of the same weight as
the IBM review. When the pull request is open, set the status to `review`.
