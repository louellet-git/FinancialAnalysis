---
id: T-20261121-selftest-golden
title: fa selftest golden, stored commands with the hash of their expected output
area: engine
status: in-progress
owner: louellet-git
head: quant
story: E4.S4
order: 0.84
branch: claude/quant-selftest-golden
created: 2026-11-21
requested_by: louellet-git
acceptance: `fa selftest golden` runs the cases of one committed case file, compares the SHA-256 of each stdout (after its mask) with the stored hash, skips and names a case whose store commit is not present, exits 1 on a difference with the first 20 differing lines, never downloads; `--update REASON` writes the new hash with its reason; the first five cases pass on the current main; gate code-reviewer.
---
Row 25 of `T-20261121-quant-engine-integration` (owner request, 2026-11-21: repeatable steps into the engine). Rules
of the Head of Quality and Risk (2026-11-21): (a) one case file, for example `rust/tests/golden/cases.toml`; each case
has a name, the fa arguments, the store commit it needs, and the SHA-256 of the expected stdout; (b) a mask list for
each case, only for lines that must vary (the timing lines; anything else needs a written reason); (c) a case whose
store commit is not present is SKIPPED and named, not passed; (d) exit 1 on a difference, with the first 20 differing
lines; (e) `--update` needs a reason, written next to the new hash, so a changed golden shows in the PR diff; (f) it
never downloads; (g) not in CI (the store is private; the owner pays for CI): the author and the reviewer run it.
First cases: `fa dcf drivers IBM` (seed 7); `fa options check SPX --date 2026-11-20 --rate 0.04`; `fa trade backtest
--end 2026-10-09`; `fa vol fit heston --symbol SPX --date 2026-11-20 --rate 0.04 --seed 7`; `fa dcf attribute` on
sp100 with `--curve par` for 2026-11-13 to 2026-11-20.
