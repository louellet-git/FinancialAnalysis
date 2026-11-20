---
id: T-20261120-dcf-conventions-decision
title: Decide the long-run growth and the acquisition charge of the automatic model
area: engine
status: in-progress
owner: louellet-git
head: fundamental
order: 1
branch: claude/magical-planck-41ip3i
created: 2026-11-20
acceptance: Each of the two conventions has a decision (the owner, or the Head of Fundamental Research on the word of the owner of 2026-11-20), a review of the weight of the IBM review passed for each default that changed, and the assumption records give the decision and its date.
---
Source: `analysis/priors/dcf_conventions_study_2026-11-20.md`. The study changed no default. Its options
are off: `--rules long_run=`, `acq_w=1`, `acq_f=` with `acq_mult=`.

**Decision 1: long-run growth.** The model now uses two numbers that do not agree: the growth fades to the
10-year zero yield (5.13%), and the mean terminal growth is 2.25%.

| Choice | Option | Median P50 / price (430 tickers) |
|---|---|---|
| Keep the two numbers | none | 0.489 |
| One number: the lower of the yield and the growth of the economy (3.8%) | `long_run=0.038` | 0.546 |
| One number: the 10-year zero yield | `terminal_rf=1` | 0.603 |

The study recommends the second choice. Before the decision, read the two inputs of the 3.8% (real growth
1.8%, inflation 2.0%) in the CBO data file: the study got them from secondary summaries only.

**Decision 2: acquisition charge.** The model has no charge now. It overstates the free cash flow after
acquisitions (median error 1.7% of revenue in the next year).

| Choice | Option | Median P50 / price |
|---|---|---|
| No charge | none | 0.489 |
| Charge the share of the spend that the growth mean credits | `acq_w=1` | 0.382 |
| Full charge | `acq_charge=1` | 0.309 |

The study recommends the second choice.

**Rules.** Do not use the price to choose (`docs/QUALITY-STANDARDS.md`, section 6). A change of a default
needs a review of the same weight as the IBM review (`CLAUDE.md`, source rule 3): flow "Model change" of
`docs/TEAMS.md`. Update `docs/assumptions/A-001-terminal-growth.md` and write the record of the acquisition
charge in the same pull request.

**Owner word, 2026-11-20.** The owner does not choose each convention. The Head of Fundamental Research
decides what is most appropriate, and writes the study and the decision in the assumption records
(`chiefs/louellet-git/fundamental/RECORD.md`, entry of 2026-11-20 19:27 UTC).

**State, parked on the word of the owner, 2026-11-20.** Decided and built on the branch: `long_run = 0.040`
(one number L, the lower of the 10-year zero yield and the FOMC longer-run growth of 2.0% + 2.0%) and
`acq_w = 1`. Records: `docs/assumptions/A-001-terminal-growth.md`, `A-002-acquisition-charge.md`. Checks of
the orchestrator: 554 + 17 + 1 tests pass; `fa dcf drivers IBM` gives $83.84 / $151.84 / $258.03; AAPL P50
$128.84 (old) and $140.71 (new), seed 7. Open, in this order:
1. The full runs with the new defaults (`fa dcf screen` of `sp500.txt`, `fa dcf pit`, `fa dcf returns`), and
   their numbers in the two records and in `docs/models/auto-path-model.md` (now "not yet run"). They wait for
   the data store of the Head of Technology: this machine has 215 tickers (A to CME).
2. The gate: a request to the Head of Quality and Risk (model-validator, code-reviewer). That machine needs
   the same data.
3. The pull request, after a pass.
