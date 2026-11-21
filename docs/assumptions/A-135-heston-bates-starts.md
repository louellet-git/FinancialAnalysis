# A-135: starts and iterations of the Heston and Bates fit

Updated: 2026-11-21. Commit: not yet made.

## Choice
- 20 starts (the first is the plain start from the at-the-money variance and the skew; the others are the plain start times a log-normal disturbance with scale 0.5, and rho plus a normal with scale 0.3).
- The 3 starts with the lowest loss get 3 LM iterations each (stage 1, `STAGE1_ITER`). The best one goes on to at most 60 iterations (stage 2).
- Seed 7. `--starts N` and `--seed S` replace the first and the last.

## Where used
| Model | File | Parameter name |
|---|---|---|
| Heston, Bates | `rust/src/volmodels/calib.rs:12`, `26` | `STAGE1_ITER`, `FitOptions::default` |
| Heston, Bates | `rust/src/volmodels/calib.rs:188-198` | `random_start` |
| Heston, Bates | `rust/src/volmodels/cli.rs:77-81` | `--starts`, `--seed` |

## Alternatives considered
| Alternative | Why not chosen |
|---|---|
| 5 iterations in stage 1 | The older text of the model document says 5. It was set to 3 for speed (SPX Heston: 3.3 s against 3.9 s wall, same loss 145232.630). |
| Refine all 20 starts | Cost. A start costs a warm-up. |

## Why this choice
The ticket says "20 random starts around a start from the ATM variance and the skew". The number of refined starts, the iterations and the disturbance scales: the ticket does not say; author choice for speed, no study.

## Effect on the result
Only the best start goes to the end, so one run does not show if the optimum is unique. IBM, 40 quotes, Bates, `--rate 0.04`, 2026-11-21: final loss 2.238, 2.141, 2.141, 2.141, 2.141 for seeds 1 to 5, and 1.996 for seed 7 (all "NOT converged: iteration cap"). See `docs/models/heston-bates.md`.

## Who decided and when
Number of starts (20): owner ticket of 2026-11-20. Seed 7, refined starts, iterations and scales: not known (author choice).

## When to examine it again
When a fit is not "converged"; when the run time target of 5 s (Heston) is not met; before the parameters are used for a decision.
