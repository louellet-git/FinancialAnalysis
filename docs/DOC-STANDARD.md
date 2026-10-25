---
id: DOC-STANDARD
type: plan
item: 10.22 Information management plan
status: current
date: 2026-10-25
scope: Rules every governed document must meet and how they are checked, for the two owners and their agents.
---
# Documentation standard

## Conformance

Tailored conformance to ISO/IEC/IEEE 15289:2019 (normative references 12207:2017,
15288:2015); process claimed: information management. Generic content (clause 7) is taken
from the 2017 text via GOST R 58609-2019; the 2019 text is unverified. Staleness triggers:
buying the 2019 text, or IEEE P15289 publishing. Declaration (5.2(a)): "the contract" means
the owner rulings plus `docs/COLLABORATION.md`.

## Governed set

Every `*.md` file, tracked or untracked and not ignored, except:

- `.claude/agents/**`, `.claude/commands/**`, `.claude/skills/**` and `.github/**`: tool
  configuration read by models or GitHub.
- `PLANNING/board/T-*.md`: request items, fields checked by `scripts/board.py`. Mapping:
  created=date, area=scope, title=subject, owner=originator, acceptance=detailed
  description, body=rationale.

One item per file. The ledger `docs/doc-reviews.tsv` is a clause 9 record, not governed.

## Header

Front matter between `---` lines, `key: value`, exactly these keys:

| Key | Value |
|---|---|
| id | `DOC-<SLUG>`: A-Z, 0-9, hyphens; unique |
| type | description, plan, policy, procedure, report, request, specification, record |
| item | `10.<n> <2019 title>`, n in 2..74; or `9 record` |
| status | draft, current, frozen, superseded |
| date | YYYY-MM-DD, date of issue |
| scope | one sentence: purpose, audience, scope |

The registry of items is the headers: `python scripts/doc_check.py list`.

## Elements met for every item

- Issuing organization: this repo.
- Change history: `git log -- <path>` plus the PR.
- Approval: merge of the PR carrying the item's PASS review row; either owner may merge.
- References: inline paths or links.
- Glossary: omitted by tailoring (audience: the two owners and their agents). A doc
  defines any non-standard term at first use.

## Required content by type

The header meets date, status and scope for every type. R = by reference (above), T = omitted
by tailoring. The body holds the rest, each element present or tailored out; the critic judges.

| Type | R | T | Body |
|---|---|---|---|
| description | issuer, references, history | glossary | context, notation, body, summary |
| plan | issuer, references, approval, history | glossary | tasks, tools and methods, schedule, budget, resources, responsibilities, interfaces, risks, quality measures, environment and safety, training, review and reporting, other plans, change procedure, completion |
| policy | issuer, references, approval, history | glossary | objectives, enforcers, effective date if not `date` |
| procedure | issuer, references, approval, history | glossary | related plans and procedures, inputs and outputs, ordered steps per participant, error resolution |
| report | issuer, references, history | glossary | participants, summary, context, methods, conclusions and recommendations |
| request | none | none | subject, originator, requested response, description with due date, rationale |
| specification | issuer, references, approval, history | glossary | body, assurance requirements, constraints, one unambiguous identified statement per requirement |
| record | issuer, references | none | subject or category, body; `id` is the record identifier |

## Status

- `frozen`: an audited or binding record (e.g. `analysis/IBM/report/*`, `REVIEW.md`,
  `DRIVER-REVIEW.md`). `superseded`: replaced by a newer item, named in the doc. Both get the
  header check only, are never compacted, and are outside the duplicate-line rule.
- `draft` and `current`: full critic review. Every status needs a PASS row (see Review).

## Compactness (the critic's checklist)

1. State each fact or rule once; link it elsewhere.
2. No preamble; never restate the header.
3. No history narration in current docs; git holds it.
4. Use the shortest form that keeps the meaning; a table or list over prose when shorter.
5. No hedges or filler.
6. Every sentence must change what a reader does or knows.
7. No 60+-character line appears in two docs (code fences, table separators, `frozen` and
   `superseded` docs excepted).

## Review

1. Run the `doc-critic` agent on the doc.
2. On PASS: `python scripts/doc_check.py record <path> PASS`, then commit `docs/doc-reviews.tsv`
   with the doc.
3. On FAIL: fix, re-run.

Every governed doc, whatever its status, needs a PASS row for its current content (CRLF read
as LF); any change voids it. The session writes the row, so CI proves a PASS was recorded,
not that the critic ran.

## Enforcement

- `python scripts/doc_check.py check`: the whole repo, in CI on pull requests and pushes to
  main, after the fact. Fails on a bad header, a duplicate id, a missing PASS row, or a
  breach of Compactness 7.
- Hook (`--hook`): denies a command containing `git` and `commit` while any doc the commit
  could carry fails: differing from HEAD in the index or worktree, or untracked. It reads
  worktree content and ledger, in the repo the command targets (`cd`, `git -C`, else cwd).
- Not seen by the hook: GitHub MCP writes (`push_files`, `create_or_update_file`), commits
  not made through Bash, sessions without python. CI catches them later.

## Plan elements

- Tasks, tools and methods, quality measures, review and reporting: Review, Compactness,
  Enforcement. Responsibilities: the session records, either owner merges (Approval).
- Interfaces: CI and the hook. Other plans: `docs/COLLABORATION.md`. Risks: the unseen paths
  in Enforcement. Change procedure: edit through a PR; Review voids old rows.
- Tailored out: schedule (standing), budget (none), resources (none), environment and
  safety (none), training (self-describing), completion (open-ended).
