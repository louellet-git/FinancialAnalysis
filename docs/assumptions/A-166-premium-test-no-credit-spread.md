# A-166: BAA10Y and NFCI excluded, no credit spread and no financial-conditions index in the premium test

Updated: 2026-11-21. Code: branch econ/premium-test-cmd.

## Choice
M1 = M0 + dVIX. The credit spread (BAA10Y) and the index NFCI are not regressors. The placebo block is dVIX only.
This follows design record R1 (2026-11-21, about 16:20 UTC, before any run; BAA10Y) and R2 (2026-11-21, about
17:15 UTC, before any run; dNFCI). Before R2, M1 = M0 + dVIX + dNFCI.

## Where used
| Model | File | Parameter name |
|---|---|---|
| premium macro test | `rust/src/macro_test/design.rs`, `test_rows()` | the M1 block |
| premium macro test | `rust/src/macro_test/mod.rs`, module comment, choice 9 | the note of the decision |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| Keep BAA10Y | No lawful daily source. The FRED terms prohibit storage. Moody's has no free daily series. |
| Monthly HQM spread | It does not fit a weekly test. |
| Excess bond premium (Gilchrist and Zakrajsek) | Monthly and revised (design rule 8). |
| Keep NFCI (R2) | The Chicago Fed legal notice does not permit a stored copy, except with written permission. The FRED copy has the same source. |
| Ask the Chicago Fed for written permission (R2) | An outward action of the owner. It delays the test. |

## Why this choice
Design section 6, records R1 (commits e77f089 and bc7da33) and R2. The Head of Technology gave the data facts on
2026-11-21 (the FRED terms; the Chicago Fed legal notice). No value of S1, y or a regressor was seen for this choice.

## Effect on the result
Not known. The limit that follows: "no share shown" says nothing about credit risk or about financial conditions.
To test: add a credit series or an index from a lawful source, as a new dated design record before the run.

## Who decided and when
The Head of Economics, in design records R1 and R2 (both 2026-11-21), on the data facts of the Head of Technology.

## When to examine it again
When a lawful credit-spread or financial-conditions source exists in the store, or the Chicago Fed gives written permission.
