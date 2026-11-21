# The teams of the firm

The agents are files in `.claude/agents/<name>.md`. The shared procedures are skills in
`.claude/skills/<name>/SKILL.md`. The agent files give the Sonnet model. For one run, the
orchestrator can choose a different model that fits the job. This is a permission, not a
duty (owner word, 2026-11-20). The standards are in
`docs/QUALITY-STANDARDS.md`; the rules of the repo are in `CLAUDE.md`.
Status: new, 2026-11-20 (ticket `T-20261120-agent-teams`).

## The agents

| Team | Agent | Job |
|---|---|---|
| Data | `data-engineer` | Sources, loaders, cache rules, `fa sync`, `mapcheck` and its registry |
| Data | `data-quality-analyst` | Cross-source checks, freshness, mapping status, coverage (read only) |
| Research | `valuation-quant` | DCF engines, the automatic model, the calibration |
| Research | `equity-analyst` | Model and report of one ticker; step 1 of the review |
| Research | `rates-credit-quant` | Curves, rate models, bond research |
| Research | `volatility-derivatives-quant` | Option surfaces, volatility models |
| Research | `strategy-researcher` | Option bots, backtest, paper trading |
| Engineering | `rust-engineer` | Core code, speed, CLI, Windows and Linux |
| Model documentation | `model-documenter` | Model documents and assumption records |
| QA | `model-validator` | The skeptic; step 2 |
| QA | `document-reviewer` | Content, format and STE; step 3 |
| QA | `auditor` | Sources and recalculation; step 4 |
| QA | `test-engineer` | Test plan, fixed answers, point-in-time, CI (on request); test audit of an area (each head, from time to time) |
| QA | `code-reviewer` | Correctness of a diff and the repo rules |
| Risk | `risk-compliance-officer` | Model risk, no orders, terms, secrets, conflict of interest, hygiene (on request) |

QA agents and the risk officer have read and run tools only. They have no edit tool and
no write tool, because the documentation of Claude Code cannot limit a write tool to one
file. They give the report in the final message. The orchestrator saves it if needed.

## Main flows

| Flow | Steps |
|---|---|
| Code change | `rust-engineer` → `code-reviewer` |
| New data source | `data-engineer` → `data-quality-analyst` → `code-reviewer` |
| Model change | quant → `model-documenter` (record) → `model-validator` → `code-reviewer` |
| New value model | `valuation-quant` → `data-engineer` (variables) → the model-change flow |
| Strategy or backtest | `strategy-researcher` → `model-validator` |
| Investigation | the researcher → `auditor` |
| Valuation report | `equity-analyst` → `model-validator` → `document-reviewer` → `auditor`; Luc approves a new ticker |

A flow has at most two QA agents, but the valuation report has the four steps of
`CLAUDE.md`. The orchestrator adds `test-engineer` or `risk-compliance-officer` only when
the work needs it: a new engine, a new outside source with terms of use, a change in the
use of keys, a valuation with a conflict of interest (ANT), or a first public release.

## Rules of the orchestrator

The orchestrator is the main Claude Code session. It is not an agent file.
A head session is an orchestrator agent, the orchestrator of its area: `chiefs/ORIENTATION.md` gives the four
heads, their areas, their agents and their message channel.

1. Only the orchestrator commits, pushes, opens and merges pull requests.
2. Ticket first: set it `in-progress` with the login and the branch, and push (skill
   `ticket-workflow`). One branch for one ticket, from `origin/main`.
3. Parallel agents work in git worktrees. Each agent gets its own path.
4. Each brief has the ticket, the goal, the files, the commands and the acceptance. A QA
   agent gets the work and no conclusion, in a fresh context.
5. The orchestrator reads the diff and runs the real command before it commits.
6. It applies the gates of `docs/QUALITY-STANDARDS.md`. After two review cycles it
   decides.
7. It adds a row to the ledger of its own folder for each run (`CLAUDE.md`, "Token ledger"). After the merge it moves
   the ticket file to `PLANNING/board/done/` (never deletes it).
8. A decision for a person (a new ticker, a closed decision, a rule, a paid source) goes
   to the person.

## How a person starts an agent

In Claude Code in this repo, name the agent: "Use the `data-engineer` agent to add the
source X. Ticket: `T-...`." Claude Code also starts an agent when the task fits its
description. `/agents` lists them.
