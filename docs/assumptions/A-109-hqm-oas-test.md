# A-109: HQM against Treasury zero plus mean OAS

Updated: 2026-11-21. Commit: not yet made.

## Choice
The curve-level test of `fa recon bonds` compares the HQM spot rate of each tenor (1, 2, 3, 5, 7,
10, 20, 30 years) with the Treasury zero rate (semi-annual) of our curve plus one spread. The
spread is the equal-weight mean of the ICE BofA OAS of AAA, AA and A on the date. The same
spread is added at all tenors. The HQM value is a monthly average. Our zero rate is for one day.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Recon bonds | `rust/src/recon.rs` lines 285-327 (`hqm_rows`) | `mean_oas`, `res_bp: 0.5` |
| HQM tenors | `rust/src/bonds.rs` (const `HQM_MATURITIES`) | `HQM_MATURITIES` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| OAS weighted by index size | Not in the code. Not known. |
| A spread that depends on tenor | Not in the code. Not known. |

## Why this choice
The HQM curve is a blend of AAA, AA and A bonds (module comment of `bonds.rs`), so the
mean of these three OAS is the matching spread. The equal weight is an author choice, no study.

## Effect on the result
The residual after the OAS is -28.6 bp (1Y) to +50.9 bp (30Y), monthly means 2026-01 to 2026-10
(`rates_recon_2026-11-20.md`, section 3.2). A flat spread does not fit the term structure of
the HQM spread (24 bp at 1Y, 104 bp at 30Y). The note says the ICE OAS is a mean over an index of
mixed maturity. So the result is a convention gap, not an error of one source.

## Who decided and when
Not known. Ticket `T-20261120-arb-2-rates-recon` (2026-11-20).

## When to examine it again
When the index duration of the ICE series is in the store (open point of the step 1 note).
