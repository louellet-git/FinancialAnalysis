---
name: number-audit
description: Use to trace a number or a claim to its source and calculate it again: in an audit of a report, a review of a model, a data check, or before you write a number in a document. It gives the audit table.
---

# Number audit

Aim: a reader can get the same number from the source. Do not copy the number from the
author. Get it yourself.

## Steps
1. List each claim: each number, date, quote, and statement of fact. Give each an id.
2. Find the source for each claim:
   - Company value: `mapcheck values TICKER`, `mapcheck filing TICKER --statement ...`.
   - Market value: `fa prices show TICKER --from D --to D`, `fa prices last TICKER`.
   - Model value: `fa dcf drivers TICKER --seed S --json`, with the model file of the work.
   - Rate value: `fa rates ...`, `fa bonds ...`.
   - Outside fact: the research ledger (`analysis/<TICKER>/research-ledger.md`) and its
     source address. Open the source if you can reach it.
3. Calculate each derived number again with a script in the scratchpad. Use the same
   rounding as the document.
4. Compare. Write the result: OK, DIFF (give both values), or NO SOURCE.
5. Check the as-of date of each market number and the expiry of each ledger fact.
6. Check that tables add up and each percentage has its base.

## The audit table
| id | Claim | Source (command, or file and line, or address and date) | Your value | Document value | Result |
|---|---|---|---|---|---|

Keep the commit (`git rev-parse HEAD`), the cache date, and the seed with the table.

## Levels
A wrong number that a person uses for a decision: CRIT. A number with no source, or a
secondary source for a load-bearing number: MAJ. A rounding or a date difference with no
effect: MIN.

## Check
- You ran each command yourself.
- Each DIFF has both values and the command.
- The list of claims that you could not check is in the report.
