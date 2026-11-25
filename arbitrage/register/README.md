# Register of no-arbitrage relations

One file for each relation: `<ID>.md`. The file name is the id. Prefixes: OPT (option chain), VIX (the index).
A test (`rust/src/market/arb_run.rs`, `register_and_code_agree`) reads this folder.
Each `active` file has an id in `option_arb::CHECK_IDS`. Each id in the code has an `active` file.
`fa arb run --symbol T --date D` runs the active relations on one stored snapshot and prints one table.

WARNING: a break is a measurement of one stored file. On indicative Alpaca quotes (not OPRA) it is not a tradable price.

## Symbols
S: the spot of the file (a last price, not a bid or ask). K: a strike. F: the forward of one expiry.
df: the discount factor to the expiry, exp(-r T). D: a dividend. T: time to the expiry in years (calendar days / 365).
C, P: a call and a put price. A price is in USD for one share (an index: index points).

## Front matter (between two lines `---`)
| Field | Meaning |
|---|---|
| id, title | The id (same as the file name) and one line. |
| class | `hard`: inside one chain copy. `soft`: across sources or times. A soft break needs a time-gap field. |
| instruments | The symbols. |
| statement | One line with the formula. |
| tolerance | At bid and at ask. The value in the code and its unit. `EPS 1e-9` is in USD for one share. |
| for | Consecutive snapshots that a break must last before it counts. Now 2. Not used yet. |
| severity | `report` (goes in the dated report) or `alert` (a symptom: a stale or empty copy). |
| status | `active`: a check exists in code. `planned`: no check yet. |
| check | The command and the Rust function. |
| source | The paper or document. |
| limits | What the result is not. |

Body: at most 25 lines. Why the relation holds, what a break most likely means, what the check does not do.

## Rules
- A quote with no ask, bid above ask, or no bid and no ask above 0 is dropped. It is counted by reason.
- A bid of 0 with an ask above 0 is a buy leg only. A relation that needs its bid is "not tested", not "pass".
- A quote with bid = ask is flagged and still used.
- Do not set a tolerance from a price. A new relation gets a fixed-answer test before it is `active`.

## Zero bids: what we measured
SPX 2026-11-20 (Alpaca indicative, Quality and Risk, 2026-11-21): 3062 quotes have a null bid.
1857 of them have an ask. 1205 have none. 1071 are used as buy-only legs.
They enter 101112 relations on the ask side: monotony 6555, slope 94249, butterfly 308. That is about 3.5% of 2.92M tests.
None of the 143 violations depends on a zero-bid leg.
Industry practice is to drop zero-bid quotes. That is a filter for stale quotes. It is not a rule of arbitrage:
a zero bid with an ask is a valid buy price. We keep the leg as a buy leg and count each relation that needs its bid as "not tested".

Industry note: `analysis/priors/arb_invariants_industry_2026-11-21.md`. Model: `docs/models/options-static-arbitrage-check.md`.
