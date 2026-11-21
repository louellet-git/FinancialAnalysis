# A-164: ret_w squared in the base model M0

Updated: 2026-11-21. Code: branch econ/premium-test-cmd.

## Choice
M0 = intercept, ret_w, ret_w^2, dzero10. ret_w is the median weekly price return of the list names
(a decimal). The 2-year zero change is not added. Both M0 and M1 hold the square.

## Where used
| Model | File | Parameter name |
|---|---|---|
| premium macro test | `rust/src/macro_test/design.rs`, `m0_row()` (line 25) and `m0_names()` | the M0 columns |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| M0 without the square | VIX is not linear in the return. A rise of VIX in falls would win by that alone. |
| SPY return in place of ret_w | The mechanical price term of S1 is the median return of the list. SPY is a description. |
| Add the 2-year zero change | The Head of Economics left it out at C1 to keep M0 small (a choice, no test). |

## Why this choice
Design rule 4 (frozen at 55f2e92, condition C1: median list return in M0). The Head of Economics added the square at C1: VIX is not linear in the return, so a rise in falls must not win by that alone.

## Effect on the result
Not known. No sensitivity run is recorded. To test: remove `r * r` and its name, rebuild, run again. M0
gets weaker, so the gain of M1 can rise.

## Who decided and when
The Head of Economics, in the design (drafted and frozen 2026-11-21 at 55f2e92, after two review cycles of the Head of Quality and Risk).

## When to examine it again
If the identity check (design rule 7.1) shows a poor R2 of M0, or the SPY check differs much from ret_w.
