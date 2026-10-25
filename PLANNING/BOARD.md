# Project board and workflow

**Board:** https://claude.ai/artifact/82X6JYs9eBHaVrzbLyeJht (private to its owner until
shared from the page's Share menu)

The board is the single source of truth for planned, in-flight and finished work on
this repo. It replaces the OPEN TICKETS section of `PLANNING/tickets.md` and
`PLANNING/HANDOFF.md` (migrated 2026-10-25, see the table at the end). Both people
working here, the owner and Claude, plan, pick up and report work on it.

## The one rule: ticket first, then work

No work starts without a ticket. Code, analysis, docs, reviews and research all count.
A ticket is ready to start when it has:

- a title that says what will be true when it is done;
- a type, a priority and an estimate (or an explicit "not estimated" for research);
- **acceptance criteria**, each one checkable by someone other than the person doing it;
- an assignee, set by whoever starts the work.

Work that turns up mid-task and is outside the ticket gets its own ticket. Note it on the
current ticket, then keep going.

## Lifecycle

| Status | Means | Who moves it there |
|---|---|---|
| To do | Ready, not started | Anyone, at planning |
| In progress | Someone is actively on it; assignee set | The person starting it |
| In review | Work is finished and waiting for the other person to check it | The person who did the work |
| Done | Meets the Definition of Done below | The reviewer, or the owner of the ticket for trivial chores |

**Blocked** is a flag, not a status: set it with a reason ("waiting on the owner's
decision", "SEC API down") and assign the ticket to whoever can unblock it.

**Decisions** are tickets of type `decision`, assigned to the owner. Claude writes the
options and a recommendation into the description; the owner posts the decision as an
update. Implementation gets its own ticket.

## Definition of Done

A ticket is done when all of these hold:

1. Every acceptance criterion is checked.
2. **Smoke test:** real output was printed and confirmed consumable by the next stage
   (the standing policy from `tickets.md`).
3. `python -m pytest` passes; new behaviour has tests.
4. Docs that describe the changed behaviour are updated (`README.md`,
   `docs/ARCHITECTURE.md`, `docs/HIDDEN-ASSUMPTIONS.md`, `CLAUDE.md`, as relevant), and
   `requirements.txt` is current if dependencies changed.
5. Commits name the ticket key; the ticket's Links list the commits or PR.
6. A final update on the ticket says what changed, where, and anything left open
   (as new tickets).
7. For a valuation run: `PLANNING/token-ledger.md` is updated, and a client-grade report
   went through the review pipeline in `CLAUDE.md` before publication.

## Priorities and estimates

| Priority | Use for |
|---|---|
| P0 | Urgent: blocks other work or something time-sensitive is slipping |
| P1 | High: next up, part of the current goal |
| P2 | Normal |
| P3 | Low, or an idea worth keeping |

Estimates are story points on the Fibonacci scale (1, 2, 3, 5, 8, 13), relative effort
and not hours. Anything over 8 should usually be split. Research and ideas may stay
unestimated until scoped.

## Sprints

Two-week sprints, each with a one-line goal.

- **Planning** (start): pick tickets from the backlog into the sprint until the points
  match what the last sprint finished; set the goal.
- **During:** keep the board current. Claude posts an update when starting a ticket, at
  meaningful checkpoints and when handing over to review.
- **Review** (end): demo what is done; **Complete sprint** on the Backlog view returns
  unfinished tickets to the backlog; re-plan.

Epics group tickets by theme; the current set is on the board (Board & process,
IBM v4 upkeep, Importer & data quality, Factory P1, Engine upgrades, New tickers,
Repo hygiene).

## Conventions

- **Keys** are `FA-<n>`, assigned in order by the board.
- **Commits** start with the key: `FA-12: pin v3 inputs as constants`. One ticket per
  commit where practical.
- **Branches:** local work uses `fa-<n>-<slug>` worktrees per `CLAUDE.md`; cloud
  sessions use the branch they are given and still put the key in every commit.
- **Updates** are short and factual: what changed, what is next, what is needed from
  whom. Status changes and field edits are logged automatically.

## How Claude reads and writes the board

The board keeps its data in the artifact's database. In a Claude Code session, use the
`ArtifactData` tool with the board URL above:

- `tickets/FA-<n>`: one document per ticket. Fields: `key`, `title`, `type`
  (feature, bug, chore, research, docs, decision), `status` (todo, in_progress, review,
  done), `priority` (P0–P3), `points` (number or null), `epic`, `sprint` (sprint id or
  null for the backlog), `assignee` (`"claude"`, a person's user id, or null),
  `description`, `acceptance` ([{text, done}]), `links` ([{label, url}]), `blocked`,
  `blockedReason`, `doneAt`, `created` and `updated` ({at, by}), and `log`
  ([{at, by, kind, text}], newest last, capped at 120).
- `board/meta`: `epics` [{id, name}] and `sprints` [{id, name, start, end, goal, state}]
  where state is planned, active or closed.

Claude writes `by: "claude"`. Always pin writes with `if_version` from a fresh read,
append to `log` rather than replacing it, and set `doneAt` when moving to done (null
when moving back). The page's source is `PLANNING/board/board.html`; republish it to the
same URL from that path.

## Migration of 2026-10-25

| Was | Now |
|---|---|
| HANDOFF item 1, v3 reproducibility drift | FA-6 (decision) |
| HANDOFF item 2, filed_date fabrication; tickets.md T-IMP-1 | FA-7 |
| HANDOFF item 3, bridge-per-ticker rule | FA-8 |
| HANDOFF item 4, P1 validators + red-team + coverage | FA-9, FA-10 |
| HANDOFF item 5, ORCL frontier run → basket | FA-13, FA-14 |
| HANDOFF item 6, July 22 IBM Q2 grading | FA-2 |
| HANDOFF housekeeping, tracked-but-ignored churn | FA-4 |
| tickets.md rulebook extraction | FA-11 |
| tickets.md research-ledger schema | FA-12 |
| tickets.md RPO expiry | FA-3 |
| tickets.md DUOL driver build | FA-15 (decision) |
| tickets.md ticket 7, segment revenue | FA-17 |
| tickets.md T-DCF-1, T-DCF-3, marginal and E/V upgrades | FA-18, FA-19, FA-20 |
| tickets.md news-scan idea (THOUGHTS item 2) | FA-21 |
| tickets.md requirements.txt kept current (THOUGHTS item 1) | Definition of Done item 4; FA-5 |
| tickets.md standing per-run duties | Definition of Done item 7 |
| New from the 2026-10-25 review | FA-1, FA-5, FA-16, FA-22 |
