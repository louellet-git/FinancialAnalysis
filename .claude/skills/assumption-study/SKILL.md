---
name: assumption-study
description: Use to study one assumption of the automatic DCF model (a value that a person could choose, for example the equity risk premium, the horizon, mid-year flows). It gives the steps from the question to the record, with evidence that does not use the price, and one command that measures each alternative again. Do not use it for a defect that has a test, or for one ticker.
---

# Assumption study

The register is `fa dcf assumptions` (`rust/src/dcf/assumptions.rs`). A row has the default, the
`--rules` key, the record and the state. The price measures and ranks. It never sets a value.
Do not use this skill for a defect that has a test (a defect is corrected, not studied), or for
one ticker. For a study with many rows, write the file with the skill `investigation-report`.

## Steps
1. **Question and present value.** Write one question. Find the row in the register. Read its
   record in `docs/assumptions/`. The register says "record not written" or "-" when no record
   exists: write that. Write the present value and who set it.
2. **Alternatives as `--rules`.** `--rules` takes numbers: a percent is a fraction (4% is
   `0.04`), a flag is 0 or 1. Write each alternative as `--rules KEY=VALUE`. A value with no
   key in the register is a new row: add it first (default from `Rules::default()`, and its test).
3. **Evidence that does not use the price.**
   - The panel of the cache (`fa dcf calibrate`): what the data say about the value.
   - Named sources, each with an address and a read date. A source that you cannot read is a
     secondary source: say so.
   - The point-in-time test: `fa dcf pit --rules KEY=VALUE`, with `--cut YEAR` (table estimated
     before the test years) and with `--half 0|1` (develop on one half, score on the other).
     It tests revenue and free cash flow, not value.
   - The return test (`fa dcf returns`) is secondary. It never decides.
4. **The effect of each alternative.** For each one, run
   `fa dcf assumptions --effect KEY=VALUE --tickers-file rust/mapcheck/universe/sp500.txt --cache <data store> --as-of <date>`.
   It runs the screen and the point-in-time test, two times, and gives the block for the record
   (command, seed, data date, cache commit). `fa dcf pit` alone gives the full table.
   One change at a time. The effects of several changes do not add.
5. **The decision.** The Head of Fundamental Research decides; the reason does not use the price
   (record of the owner word: `chiefs/louellet-git/fundamental/RECORD.md`, 2026-11-20 19:27 UTC).
   A closed decision (`REVIEW.md`, `DRIVER-REVIEW.md`) or a rule of `PLANNING/RULES.md` stays: stop and ask.
6. **The record.** Write or change the record (`docs/assumptions/TEMPLATE.md`). The section
   "Choice" has the default as `fa dcf assumptions` prints it. Add the effect block.
7. **The register and the model document.** Change the row of the register (id, record, state)
   and `docs/models/auto-path-model.md` in the same change. The test of the register fails if
   a record and a default disagree.
8. **The gate.** A change of the model goes to `model-validator` and `code-reviewer`. Run the skill
   `release-checks`. The IBM numbers (`fa dcf drivers IBM`, seed 7) stay $83.84 / $151.84 / $258.03.

## Check
- The record has the question, the alternatives with their commands, the reason, and the effect block.
- The state of the row is `decided <date>`, or the row says `convention, not studied`.
- No value was set or chosen from the price.
