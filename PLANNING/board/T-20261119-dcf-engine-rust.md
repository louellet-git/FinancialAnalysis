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
