# Register of no-arbitrage relations

One file for each relation: `<ID>.md`. The file name is the id. A test (`market/arb_run.rs`, `register_and_code_agree`)
reads this folder: each `active` file has a check id in `option_arb::CHECK_IDS`, and each check id has an `active` file.
`fa arb run --symbol T --date D` runs the active relations on one stored snapshot and prints one table.

WARNING: a break is a measurement of one stored file. On indicative Alpaca quotes (not OPRA) it is not a tradable price.

## Front matter (between two lines `---`)
| Field | Meaning |
|---|---|
| id | Same as the file name. Prefix OPT, VIX, BOX. |
| title | One line. |
| class | `hard`: inside one chain copy. `soft`: across sources or times (a time-gap field is needed). |
| instruments | The symbols the relation applies to. |
| statement | One line with the formula. |
| tolerance | At bid and at ask. The value in the code. |
| for | How many consecutive snapshots a break must last before it counts. Now 2. Not used yet. |
| severity | `report` (goes in the dated report) or `alert` (a symptom: a stale or empty copy). |
| status | `active`: a check exists in code. `planned`: no check yet. |
| check | The command and the Rust function. |
| source | The paper or document of the relation. |
| limits | What the result is not. |

## Body
At most 25 lines: why the relation holds, what a break most likely means, what the check does not do.

## Rules
- A leg with a null quote, bid above ask, or no bid and no ask above 0 is dropped and counted by reason.
- A bid of 0 with an ask above 0 is a buy leg only. A relation that needs a bid there is "not tested", not "pass".
- A quote with bid = ask is flagged and still used.
- Do not set a tolerance from a price. A new relation gets a fixed-answer test before it is `active`.
- Industry note: `analysis/priors/arb_invariants_industry_2026-11-21.md`. Model: `docs/models/options-static-arbitrage-check.md`.
