---
id: T-20261119-bond-recon
title: Compare theoretical bond values with the values from the rates that the repo gets
area: engine
status: in-progress
owner: louellet-git
branch: claude/bond-recon
created: 2026-11-19
acceptance: One command prints, for a date, the difference between the theoretical value and the value from our rates for each zero-coupon bond and each plain coupon bond in the comparison set, and writes the differences to a file.
---
Owner request, 2026-11-19: a script that finds the differences between the theoretical values of zero-coupon bonds and plain bonds, and the values that our rates give. The owners examine the differences. Do not merge to `main` before the owner says so.
