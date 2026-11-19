---
id: T-20261119-ant-ipo-valuation
title: Value Anthropic (ANT) at its IPO
area: ticker:ANT
status: in-progress
owner: louellet-git
branch: claude/admiring-allen-f5aaeq
created: 2026-11-19
acceptance: analysis/ANT contains a report with a value range from the engine, compared with the IPO price of $83.00, and the report went through the four review steps.
---
Anthropic, PBC (CIK 2130357) filed Form S-1 on 2026-10-26 (file 333-299633). The IPO
price is $83.00 for each Class A share (2026-11-18). Trading on Nasdaq starts on
2026-11-19 with the symbol ANT.

Steps:

1. Get the S-1 values through `mapcheck`. The company has no 10-K. Record each input
   that the standard mapping cannot give.
2. Write the driver model and run the engine.
3. Do the four review steps (analyst, skeptic, reviewer, auditor).

Conflict of interest: the agents that do this work are Claude models. Anthropic makes
Claude. The report must show this fact on its first page.
