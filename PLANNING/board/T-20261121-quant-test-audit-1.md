---
id: T-20261121-quant-test-audit-1
title: Add parser tests for the option, volatility and futures commands, and a bond age test
area: engine
status: review
owner: louellet-git
requested_by: louellet-git
head: quant
order: 1.4
branch: claude/quant-test-audit-1
created: 2026-11-21
acceptance: The argument parsers of `fa options`, `fa vol` and `fa futures` have tests for an unknown flag, a missing positional value and a value that is not a number, `bonds::latest` has a test at the age limit, one day over and with an empty series, and the command outputs do not change.
---
From the test audit of the quant area (2026-11-21, `test-engineer`, audit mode). Proposals 3 and 4 are taken.
Proposals 1 and 2 (faster rate-model tests) are not taken: the whole suite takes about 10 s and the build more than
2 minutes, and a shorter simulated path can make a statistical recovery test weaker. Proposal 5: no new `option_arb`
tests; two possible duplicate pairs are checked when that file next changes.

1. `rust/src/market/options_cli.rs`, `rust/src/volmodels/cli.rs`, `rust/src/futures/cli.rs`: tests that call the
   parsers with fixed string slices; the expected value is the error, from the help text, not from the code.
2. `rust/src/bonds.rs` `latest` (`max_age_days`): an entry on the limit, one day over, and an empty series.

Agent: `volatility-derivatives-quant` (1), `rates-credit-quant` (2). Gate: `code-reviewer`.

Review 2026-11-21: PASS-WITH-NOTES (`code-reviewer`). MIN 1 and 2 (`--max-spread` below 0, and the raw text in its
error) are done in this pull request. Open for a later change of `option_arb`: the reviewer reports that
`fa options check --min-size` and `--iv-mark` still accept `nan` and `inf`, or check them only after the snapshot
loads; a test cannot reach that path without a file.
