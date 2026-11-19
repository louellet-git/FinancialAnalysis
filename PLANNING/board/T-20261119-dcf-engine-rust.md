---
id: T-20261119-dcf-engine-rust
title: Move the DCF engines, the scenarios, the verdict and the driver models to Rust
area: engine
status: in-progress
owner: louellet-git
branch: claude/dcf-engine-rust
created: 2026-11-19
acceptance: The Rust commands give the same values as the Python engines for IBM, JNJ and ANT (same seed, same percentiles), the IBM version 4 headline stays $83.84 / $151.84 / $258.03, and the Python engine files are deleted.
---
Owner request, 2026-11-19: refactor the code base to Rust; the subsequent work is the
improvement of the DCF. This ticket is steps 1 and 2 of `T-20261119-python-to-rust`.
The branch starts from `claude/admiring-allen-f5aaeq` (it has the ANT first pass).

Stages:

1. Record the outputs of the Python engines as fixed answers. Write the Rust engines
   (classic and driver), the scenarios and the verdict. The random numbers must be the
   same as those of numpy (`default_rng(seed)`, `standard_normal`), so that each
   published number stays the same.
2. Driver models (IBM, JNJ, ANT) as data files, and their runs as Rust commands.
3. Backtest.
4. Report and `analyze`; then delete the Python engines, the store and the schema.

The Python files stay until each Python user of an engine is moved (stage 4). The
branch is not merged before that, because one part must have one implementation.

## State (2026-11-19): stages 1 and 2 are done; stages 3 and 4 are open

The status of the ticket stays `in-progress`: the acceptance text needs the deletion of the
Python engine files, which is stage 4.

**Stage 1 (proved).**
- The random numbers are the numpy numbers: `rust/src/dcf/nprandom.rs`. Source: numpy
  v2.5.3 (tables, algorithm, `SeedSequence`, `PCG64`). The tests compare bit for bit:
  the raw stream and 1,000 normals for the seeds 0, 7, 42 and 123456789, a (3, 500)
  matrix, `normal(mean, std)`, and a stream of 200,000 values that reaches the tail
  branch. `np.percentile` is also equal bit for bit.
- The fixtures are in `rust/tests/fixtures/dcf/`. They were recorded from the Python
  engines before the Rust code was written.
- Rust engines: classic (`classic.rs`), driver (`driver.rs`), scenarios, verdict.
  Parity with Python: percentiles 2e-16 relative at most; attribution 1e-14; the tests
  accept 1e-9. The IBM headline from `fa dcf drivers IBM` is $83.84 / $151.84 / $258.03.
- Fixed-answer tests: closed-form calculations in exact fractions for the driver engine
  (two models with all drivers deterministic: with and without year-1 pins and a
  schedule) and for the classic engine (all std = 0). They cover a part of ticket
  `engine-fixed-answer-tests`.

**Stage 2 (done).**
- Driver models are data: `analysis/{IBM,JNJ,ANT}/driver_model.toml`. A test shows that
  each file gives the same model as `build()` (numbers bit for bit, schedules, every
  rationale text) and the recorded results.
- Commands: `fa dcf drivers|classic|scenarios|models`; `fa verdict` runs in Rust when the
  ticker has a model file and the run has no market rates. The viewer `/api/verdict`
  uses the same function and has the same JSON fields.
- One feature set for the runner analyses: overrides by driver name (`--set`,
  `--shift-bounds`, `--flat`, `--ramp`, `--corr`, `--net-debt`, `--shares`, `--years`,
  `--year1-revenue`), `--sensitivity`, `--implied-price`. The tests
  `runner_tests.rs` make each printed number of the three runners again and find
  the line in the recorded output of the Python runner: IBM (all sections), JNJ (all),
  ANT (all sections, including the fade and the commitment check).

**Not done.**
- `fa verdict --rf-date` and `--rating` still call `scripts.verdict` (Python): the
  Treasury and ICE data come from Python.
- `scripts/visualize_dcf.py`, `scripts/analyze.py`, `financial_analysis/backtest.py`
  and the tests still use the Python engines.
- The Rust `fa dcf classic` reads `data/processed/<TICKER>.json`. The importer is still
  Python or `mapcheck`.

**Python files with a proven Rust equal, and what blocks their deletion.**

| Python file | Rust equal | Blocked by |
|---|---|---|
| `financial_analysis/driver_dcf.py` | `dcf/driver.rs` | `analysis/*/driver_model.py` (3), runners (3), `financial_analysis/verdict.py`, `scripts/verdict.py` (rates path), `scripts/visualize_dcf.py`, `financial_analysis/rates.py` and `bonds.py` (`with_risk_free`, `with_kd_spread` change a Python `DriverModel`), tests `test_driver_dcf`, `test_verdict_rates`, `test_visualize_dcf`, `test_bonds`, `test_rates` |
| `financial_analysis/dcf.py` | `dcf/classic.rs` | `financial_analysis/scenarios.py`, `scripts/run_dcf.py`, `scripts/visualize_dcf.py`, `analysis/IBM/dcf_scenarios.py`, `tests/test_dcf.py`; `schema.py` and `store.py` (company data) |
| `financial_analysis/scenarios.py` | `dcf/scenarios.rs` | `scripts/visualize_dcf.py`, `analysis/IBM/dcf_scenarios.py`, `tests/test_scenarios.py` |
| `financial_analysis/verdict.py` | `dcf/verdict.rs` | `scripts/verdict.py`, `tests/test_verdict.py` |
| `scripts/run_dcf.py` | `fa dcf classic` | nothing in the code; README line and tests |
| `scripts/verdict.py` | `fa verdict` (no rates) | the `--rf-date` and `--rating` paths |
| `analysis/IBM/dcf_drivers_run.py`, `analysis/JNJ/first_pass_run.py`, `analysis/ANT/first_pass_run.py` | `dcf/runner_tests.rs` and `fa dcf drivers` | `CLAUDE.md` rule 6 and the README name `python -m analysis.IBM.dcf_drivers_run` as the source of the version 4 headline: the owner must decide the new source before the file is deleted |
| `analysis/{IBM,JNJ,ANT}/driver_model.py` | the `.toml` files | `scripts/visualize_dcf.py`, `scripts/verdict.py`, the runners, `tests/test_driver_dcf.py`, `tests/test_visualize_dcf.py`; two copies until then (`HIDDEN-ASSUMPTIONS` #123) |

Next: stage 3 (backtest), then stage 4 (report, `analyze`, deletion). Open question for
the owner: the numbers of the Python runners become tests with a recorded text; keep the
fixture files as the record when the runners are deleted.
