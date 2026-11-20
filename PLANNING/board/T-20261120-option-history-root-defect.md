---
id: T-20261120-option-history-root-defect
title: Keep the SPX and SPXW quotes apart in the option history
area: trading
status: in-progress
owner: louellet-git
head: quant
order: 1
branch: claude/upbeat-cori-a4ykf5
created: 2026-11-20
acceptance: `fill_from_mids` makes one group for each root and expiry date, a fixture test with an SPX quote and an SPXW quote of the same date gives two groups, and `fa options history` runs on one real day.
---
Defect from the volatility study (`analysis/priors/volatility_study_2026-11-20.md`): `fill_from_mids` in
`rust/src/market/option_history.rs` groups the quotes by the expiry date only. On the third Friday of a
month the chain has the root SPX (settles on the morning) and the root SPXW (settles on the afternoon). The
function mixes them, so the forward and the implied volatility of that date are wrong.

Correction: group by root and expiry date, and write the root in the output.
Do this before `T-20261120-vol-1-heston-bates` and `T-20261120-vol-7-chain-copy` (step 1 of that ticket).
