---
id: T-20261120-agent-teams-first-run
title: Run the agent teams on one real ticket and correct what does not work
area: planning
status: backlog
owner: louellet-git
branch:
created: 2026-11-20
acceptance: One code ticket and one document ticket went through their flow of `docs/TEAMS.md` with the agents of `.claude/agents/`, and each defect of an agent or a skill found in the runs is corrected.
---
The agents and skills of `.claude/` passed a format check only. No agent ran on real work.

1. Run the flow "Code change" on `T-20261120-option-history-root-defect` (`rust-engineer`, then
   `code-reviewer`).
2. Run the flow "Document or model record" on the first model of `T-20261120-model-docs-backlog`
   (`model-documenter`, then `document-reviewer`).
3. Measure the tokens of each run (`PLANNING/token-ledger.md`). If an agent is too heavy, remove entries
   from its `skills:` list.
4. Keep the teams lean: do not add an agent or a check unless a run shows that it is necessary.

Known limit: a QA agent has no edit tool, but its shell can write files. A hook can prevent this. Add it
only if a run shows the problem.
