# A-037: rule for the 8-K release file of a window

Updated: 2026-11-21. Commit: not known (branch `claude/attribution-reality`, not committed).
Model document: `docs/models/baserates-and-reality.md`.

## Choice
`release_path` gives the path of one file of the cache for each ticker. Steps:
1. Take the SEC 8-K filings with item 2.02 filed after D1 and on or before D2.
2. Take the newest by filing date, then by accession number. Skip a filing with no folder
   `<cache>/<TICKER>/filings/8k/<accession>/` and take the next older one.
3. In the folder, take the file with the latest change time, then the name (the larger name wins a tie).
4. No filing or no folder: the path is empty. The code uses no network and no price.

## Where used
| Model | File | Parameter name |
|---|---|---|
| `fa dcf attribute --reality` | `rust/src/dcf/auto_attribute.rs` | `release_path`, `earnings_8k_entries`; CSV column `release_8k_path` |

## Alternatives considered
Not known. A parser of the release text is a data task of Technology (ticket `T-20261121-dcf-engine-integration`).

## Why this choice
The ticket (item R2) asks for the path of the 8-K release of the window when the cache has it. The change time is
the time of the file on the disk, not a date of the filing. So the choice of the file in a folder can change after a
new download. No source for the file rule is known.

## Effect on the result
None on any number. The path is a pointer for a person. Not known: how often a folder has more than one file.

## Who decided and when
Head of Fundamental Research, 2026-11-21.

## When to examine it again
When the cache stores the release text in a fixed file name, or when Technology adds a parser.
