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

## State (2026-11-19)

Built: the command `fa recon bonds [--date D] [--out FILE]` (crate `rust/`; modules `curve.rs`, `prices.rs`, `recon.rs`). Method and conventions: `docs/ARCHITECTURE.md`. Traps and open questions: `docs/HIDDEN-ASSUMPTIONS.md` #48 to #55.

First run (2026-11-17, 400 instruments, price source TreasuryDirect FedInvest): mean absolute yield difference 2.98 bp for all bills, notes and bonds; bills -6.8 bp on average (observed below theoretical); the middle of the curve (2 to 7 years) is within 1 to 2 bp; the largest differences are short notes (price step) and some off-the-run bonds. The causes are open questions. Nobody has explained them.

Open:
- The Rust download of the price file gets HTTP 400 in the container (curl works). Fill the cache `data/raw/prices/` with curl.
- Cause of the bill difference (#52).
- No corporate bond prices (#53). No STRIPS prices (#54).
- Owner to confirm the use of the FedInvest data (#55).
- This branch is not merged to `main` (owner request).
