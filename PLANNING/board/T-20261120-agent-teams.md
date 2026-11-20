---
id: T-20261120-agent-teams
title: Make the agent teams of the firm, with their skills and quality standards
area: planning
status: in-progress
owner: louellet-git
branch: claude/agent-teams
created: 2026-11-20
acceptance: The repo has one agent definition for each job of the firm, the skills that the agents use, one document of quality standards with a definition of done for each kind of work, and a quality assurance team that is independent of the authors; each file passes a format check.
---
Owner request, 2026-11-20: a small quantitative firm needs different agent teams for
each job type and responsibility, with skills and quality standards, and quality
assurance.

Owner decision, 2026-11-20: the agent teams and the skills are files of this public
repo (`.claude/agents/`, `.claude/skills/`). The rule of `CLAUDE.md` that kept
`.claude/agents/` out of the repo was for a private review framework of a third
party. That framework stays out. The teams of this ticket are our own files.

State, 2026-11-20 (written by the agent, the orchestrator examines it): the files are
written and not committed. They are 16 agents (`.claude/agents/`), 11 skills
(`.claude/skills/`), `docs/TEAMS.md`, `docs/QUALITY-STANDARDS.md`, the templates and one
example of a model document and of an assumption record (`docs/models/`,
`docs/assumptions/`), and the changes of `CLAUDE.md`, `.gitignore` and `README.md`.
Open: the other model documents (ticket `T-20261120-model-docs-backlog`).
