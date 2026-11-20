---
id: T-20261120-bank-utility-study
title: Study the data and the model form for banks and for utilities
area: engine
status: done
owner: louellet-git
branch: claude/bank-utility-study
created: 2026-11-20
acceptance: A findings file gives, for banks and for utilities, the variables that the mapping has for each company, the model form with its formulas, a first calculation for five companies of each type, and the measured persistence of the main driver from the panel.
---
Owner request, 2026-11-20: investigate the recommendations. This ticket prepares
`T-20261119-evaluator-bank` (first in the board sequence) and a model for utilities
(27 utilities get a value at or below zero from the free-cash-flow model).

It is a study. It adds no model to `fa dcf auto`. The design goes into the tickets of
the models.

**Result (2026-11-20).** The findings file is `analysis/priors/bank_utility_study_2026-11-20.md`. The design summary and the
list of tags are in `T-20261119-evaluator-bank` (status stays `backlog`). The new ticket for the utility model is
`T-20261120-evaluator-utility`. The study adds no code. The orchestrator examines the work, commits it, and opens the pull request.
