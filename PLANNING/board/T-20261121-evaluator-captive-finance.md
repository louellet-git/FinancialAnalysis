---
id: T-20261121-evaluator-captive-finance
title: Sum-of-parts value model for a company with a captive finance arm (industrial DCF plus a finance-arm equity value)
area: engine
status: backlog
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.009
branch:
created: 2026-11-21
acceptance: (1) `fa dcf auto` finds a company with a captive finance arm from SEC facts (segment or tag rules written before any run, with the list it finds), and the screen row says that the one-model DCF does not fit until (2) exists; (2) the value = an FCFF DCF of the industrial business (without the revenue, assets and debt of the finance arm) + an equity value of the finance arm with the excess-return model of `T-20261119-evaluator-bank`; (3) a model document, the assumption records, and a point-in-time backtest written before its run (the Backtest row of QUALITY-STANDARDS section 4); gate: model-validator + code-reviewer.
---
Owner words, 2026-11-21 about 17:20 UTC (in the session of the Head of Quality and Risk, relayed): after the explanation that a
company with a captive finance arm (Deere, CNH; CAT, F, GM and PCAR possible, not checked) does not fit one consolidated FCFF
DCF, and that the standard (McKinsey Valuation; Damodaran) is a sum of parts, the answer was: "Enhance the DCF engine."

Interim rule (stays until (2) exists): the lease lines stay out, and a screen row of such a company says that the model
does not fit it.

Order: after the bank evaluator (`T-20261119-evaluator-bank`), which gives the finance-arm part; the detection rule (1) can
come first. Dependencies: the SEC segment facts of the finance arm (revenue, assets, debt) in the variable registry
(`financial_analysis/standard_variables.toml`, Head of Technology).
