# A-150: definition and horizon of the variance premium

Updated: 2026-11-21. Commit: not yet made.

## Choice
30-day premium on day t: `(VIX_t/100)^2` minus the realised variance of SPY over the next
21 trading days. Realised variance is the sum of the squared close-to-close log returns times
252/21, with no demeaning. Unit: annual variance (0.01 is about 10 vol points squared).
1-day premium: `(VIX1D open/100)^2/252` minus the squared return of the same day. Two returns are
shown: close to close (overnight and session) and open to close (session only).

## Where used
| Model | File | Parameter name |
|---|---|---|
| Variance premium test (`docs/models/variance-premium-test.md`) | `rust/src/trading/premium.rs` | `premium_30d`, `report` |
| Variance premium test | `rust/src/trading/premium.rs` | `--horizon` (default 21 in `command`) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Calendar 30 days (about 21 sessions, but the count changes) | Not known |
| Demeaned returns | Not known. The code comment says "no demeaning" |
| One of the two 1-day conventions only | The note shows both: they bracket the answer (`trading/research/alpha-1-stage1.md`, stage 1a) |

## Why this choice
Author choice, no study. The VIX is a 30-calendar-day measure. 21 sessions is the usual count
for that span: this is a convention, and no file states it. The VIX close of day t is known at
the close. The code uses only returns of later days, so no signal uses a later fact than its date.
VIX1D is taken at the open because it is known at the open (`premium.rs` header).
The VIX is a risk-neutral measure, so the premium is not a P&L (`premium.rs` header).

## Effect on the result
The 1-day sign depends on the convention: close to close, outside the windows, t_NW -4.88;
open to close, t_NW +3.13 (`trading/research/alpha-1-stage1.md`). No other horizon was run
in the note. The command option `--horizon N` runs another one.

## Who decided and when
Not known. The code and the research note of 2026-11-21 give the choice. No file names a person.

## When to examine it again
When a horizon other than 21 is run, when real option quotes exist (stage 2), or when VIX1D
has a better source than Yahoo (the note says its open print may be stale).
