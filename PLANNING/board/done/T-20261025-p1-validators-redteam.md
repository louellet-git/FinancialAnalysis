---
id: T-20261025-p1-validators-redteam
title: P1 remainder: validators, red-team corpus, coverage validator
area: engine
status: done
owner: louellet-git
head: quality
order: 3
branch:
created: 2026-10-25
acceptance: Validators, a red-team specimen corpus and a statement-coverage validator exist and pass in pytest.
merged: none (superseded)
---
RULES.md R1 plus extraction backlog; audit flags C1-C5 seed the coverage audit.

Closed 2026-11-21 as superseded. The repo has no Python, so the Pytest acceptance is obsolete. The Rust replacements are rows 25 (`fa selftest golden`) and 26 (mutation tests on the changed files) of `T-20261121-quant-engine-integration`, and the QA gates of `docs/QUALITY-STANDARDS.md` section 5.
