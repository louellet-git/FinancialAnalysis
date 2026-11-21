# A-034: size buckets of the base rates (edges 1B, 10B, 50B of base-year revenue)

Updated: 2026-11-21. Commit: not known (branch `claude/attribution-reality`, not committed).
Model document: `docs/models/baserates-and-reality.md`.

## Choice
The base rates have four size buckets. The size is the revenue of the base year of a window, in nominal US dollars
(not adjusted for inflation). The edges are 1B, 10B and 50B (`BUCKET_EDGES` = 1e9, 10e9, 50e9). An edge value goes to the
higher bucket (revenue of exactly 10B is in "10B to 50B"). The names are in `BUCKETS`: "below 1B", "1B to 10B",
"10B to 50B", "50B or more". The edges are fixed in the code. No option changes them.

## Where used
| Model | File | Parameter name |
|---|---|---|
| `fa dcf baserates` | `rust/src/dcf/auto_baserate.rs` | `BUCKET_EDGES`, `BUCKETS`, `bucket_of` |
| `fa dcf attribute --reality` | `rust/src/dcf/auto_reality.rs` | `bucket_of` of the last fiscal year revenue at D2 |

## Alternatives considered
Not known. The ticket names no other edges and no other measure of size (market value, real dollars).

## Why this choice
The ticket (`PLANNING/board/T-20261121-dcf-engine-integration.md`, item R1) says: "nominal dollars; fixed now". It gives
no reason for the three edge values. No source for the values is known.

## Effect on the result
Not known. No sensitivity number exists. A company moves to a lower bucket in the base rates only by the revenue of
its base year. The percentile of an implied growth depends on the bucket. A large company with a high base revenue
is compared with large companies only.

## Who decided and when
Head of Fundamental Research, 2026-11-21 (ticket item R1).

## When to examine it again
When a bucket has fewer cases than a review accepts, or when the first review of `--reality` asks for other edges or
for real dollars.
