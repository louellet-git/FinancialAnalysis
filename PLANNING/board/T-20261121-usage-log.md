---
id: T-20261121-usage-log
title: Record the agents, the skills and the tokens of each head session with a hook (no model tokens)
area: planning
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1.6
story: E6.S5
branch: claude/usage-log
created: 2026-11-21
acceptance: After a normal day of work with no action of a head, one command prints for each head session the agents that it started (by type), the skills that it used, and its tokens (input, output, cache), from data that a hook wrote; the hook adds less than 2 seconds to a turn, never stops a turn, and writes no text of the conversation.
---
Owner word, 2026-11-21 (through the Head of Staff): make sure that the heads use their teams and their skills,
and watch the tokens, with a script that "just does what it has to do". No head has to remember it.

What the Head of Staff found (2026-11-21, on its own machine):
- Each cloud session keeps its transcript as a file (`~/.claude/projects/<folder>/<session>.jsonl`). Each
  assistant line has the token counts, and each tool call has its name (`Agent` with the agent type, `Skill`
  with the skill name). A script can count them. No model reads them.
- Hooks run in the cloud sessions (a `Stop` hook is already set on the machine).
- So a hook can write a small summary at the end of a turn. The open part is how the summary leaves the
  machine, because each machine is temporary.

Proposal (the Head of Technology decides the form after a test):
1. A `Stop` hook runs a script. The script counts the transcript and writes one small file: the session title
   or id, the date, the counts by agent type, by skill and by token kind. Numbers and names only.
2. The script pushes that file to its own ref (for example `usage/<session>`) with git plumbing. It does not
   touch the work branch, the index or the working tree. It pushes at most one time in 10 minutes.
3. One read command gets the `usage/*` refs and prints the table. The Head of Staff runs it.
4. When this works, the token ledger row by hand (`CLAUDE.md`, "Token ledger") can stop. That is a rule
   change: ask the person then.

Limits:
- The hook always exits with 0. A fault of the script is silent for the head.
- This repo is public and has a second person. The hook must do nothing in a session that is not a head
  session of `louellet-git`, unless that person agrees. No text of a conversation, no path with a private
  name, no contact detail goes into the file.
- Test first: a new hook in `.claude/settings.json` is possibly read only at the start of a session, and the
  push of a new ref from a cloud session is not tested yet.
- No workflow run can start from these refs (`chiefs/ORIENTATION.md`, section 6).
- Start `risk-compliance-officer` one time on the content of the file before the first push.

Owner rule, 2026-11-21: a hook or a system that constricts a head (it stops, delays or forces a step) needs
a few days of evidence that it is better than each alternative, before it starts. A tool that only reports
and constricts nothing can start now. This ticket is of the second kind; keep it so.

## Tokens by ticket, by head, by agent and by model (owner word, 2026-11-21; added by the Head of Staff)

The person has a fixed number of tokens for each week. The table must show where they go. Still no action of
a head, and no limit that stops work.

What each transcript line already has (checked 2026-11-21): the model, the four token counts (input, cache
write, cache read, output), the time, the git branch and the folder of the work. The lines of a subagent are
in the `subagents/` folder of the session, with the agent type in a small file next to them.

1. **Ticket.** The git branch of a line gives the ticket: the `branch` field of the board. A head already
   sets that field, so no new step. A line on no ticket branch goes to the row "no ticket" of its head
   (coordination, memory, messages). That row is a number to show, not a fault.
2. **Head and agent.** The session gives the head. A line of the main transcript is the head itself; a line
   of a subagent has its agent type.
3. **Model cost.** One unit for all models: weighted tokens. The weights (input, cache write, cache read,
   output, for each model) are one small table in the repo, from the current price list (read it with the
   skill `claude-api`; do not write prices from memory).
4. **Share of the week.** The real figure exists: each cloud session gets a `rate_limit_event` with the
   used share of the 7-day window and its reset time (`list_events`, kind `rate_limit_event`; on 2026-11-21
   18:49 UTC: 71% used, reset 2026-11-25 16:00 UTC). No probe call is necessary. The Head of Staff reads it
   and gives it to the read command (`--week-used 71`), which then shows each row also as a share of the week.
5. **Estimate.** No estimate field for the first days. First collect the actual numbers of some tickets
   (owner rule: evidence first). Then a ticket gets one optional field, for example `tokens: S`, `M` or `L`,
   with the size bands from the measured tickets, and the table shows actual against estimate.

Output of the read command, one table: ticket | head | head itself or agent type | model | weighted tokens |
share of the week. Sums by ticket, by head and by model.

Not from the old `chiefs` repo (it constricted too much): shares for each head, phases of the week, colours,
stop lines, a probe call, a ledger that a head writes.

## The data that the calculator needs (owner word, 2026-11-21; this ticket is its dependency)

| Data | Where it is | Collected now? |
|---|---|---|
| Tokens by ticket, head, agent type, model and time | The transcript files on the machine of each session | **No. It is lost when the machine goes.** This ticket collects it |
| The weeks before the hook | The transcript files of the head sessions that are alive now | **No.** The first run of the script reads the full file of its session (a backfill, with no action of a head). Do it soon |
| The session of each count, to find the head | The session id in the environment; the Head of Staff has the list of titles | With this ticket (write the id; no title is necessary) |
| The real share of the week, some readings in each week | The `rate_limit_event` of a session. It is not in the transcript file | Partly: the Head of Staff writes a reading in its dashboard when it looks. Test if a hook can see it; if not, this is sufficient |
| The ticket: head, area, story, dates, pull request, size of the change | Git and the board | Yes, for free. Git keeps it; read it later |
| The gates and the review cycles of a ticket | The count of QA agent starts on the branch of the ticket | With this ticket (they are agent types in the count) |

Rule for a clean count: one branch for one ticket, and the branch in the ticket (`T-20261121-board-check`
shows the faults). Tokens on a branch with no ticket go to "no ticket".

## GitHub cost

- A push to a `usage/*` ref starts no check: the workflow runs only for a pull request with code and for
  `main` (`.github/workflows/tests.yml`, read 2026-11-21). Do not open a pull request for the usage data.
- One ref for each session, replaced at each push (one commit with no parent). So the repo does not grow
  with each push. The file is small (numbers and names).
- A maximum of one push in 10 minutes for each session, and no push when the counts did not change.
- If a private place is better than this public repo, the private store `louellet-git/fa-data` is the
  alternative. The Head of Technology decides after the test.

Gate: the hook runs only if the environment variable FA_USAGE_RECORD=1. The repo does not set it. The owner sets it in his own cloud environment. The repo holds no account id and no hash.

Build (2026-11-21, branch claude/usage-log): `fa usage record` and `fa usage show` (rust/src/usage.rs);
the hook `.claude/hooks/usage-record.sh` in `.claude/settings.json` (Stop). The hook reads its input and
returns at once (about 5 ms); the count runs in the background (a cold read of a 25 MB transcript took
24 s of disk wait, a warm read 0.06 s). Limits: a head that works in a git worktree gets the branch of
the folder where its session started, so the ticket of each line can be wrong; a session reads a new
hook and a new environment variable only at its start. The push is off until the content review
(`risk-compliance-officer`) and one test push.
