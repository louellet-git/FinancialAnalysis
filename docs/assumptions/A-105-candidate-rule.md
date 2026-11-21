# A-105: Rule for a rich or cheap candidate

Updated: 2026-11-21. Commit: not yet made.

## Choice
A security is a candidate if all three hold:
1. `|res_bp|` is larger than the larger of its cost and the RMSE of the fit of the day;
2. `|z|` is 2 or more;
3. its life is 0.25 year or more.
Sign: `res_bp` = observed yield minus fitted yield. Plus is cheap. Minus is rich.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Rich and cheap (`docs/models/treasury-fit-rich-cheap.md`) | `rust/src/richcheap.rs` (`survives`) | `z.abs() >= 2.0`, `years >= 0.25` |
| Fit RMSE | `rust/src/richcheap.rs` | `noise_bp` (in-fit securities) |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| A fixed bp limit | Not known. |
| A persistence test | The command does not test it. `fa recon history` does it apart (A-103). |

## Why this choice
The cost and the fit error are the noise that a residual must exceed (comment `richcheap.rs`
). The value 2 for `z` is an author choice, no study. The note
`rates_recon_step2_2026-11-20.md` section 3.4 uses a stricter "lasting" rule (38 dates,
same sign on 90% of days, `|mean|` above cost and 1.9 bp). That rule is in the shell
script of the note, not in the code. The note calls it "mine, set before the run, not tuned".

## Effect on the result
Not known as a sensitivity. Output of 3 dates: 14 of 401, 18 of 400, 14 of 400 securities
are candidates (same note, section 3.2).

## Who decided and when
Not known. Ticket `T-20261120-arb-2-rates-recon` (2026-11-20).

## When to examine it again
When the candidates are used for a decision, and when the repo rate and borrow data exist
(`docs/HIDDEN-ASSUMPTIONS.md` #112).
