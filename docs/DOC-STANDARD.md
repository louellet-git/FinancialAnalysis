---
id: DOC-STANDARD
type: plan
item: 10.22 Information management plan
status: current
date: 2026-10-25
scope: Rules every governed document must meet, and how they are checked.
---
# Documentation standard

## Conformance

Tailored conformance to ISO/IEC/IEEE 15289:2019 (normative references 12207:2017,
15288:2015); process claimed: information management. Generic content (clause 7) is taken
from the 2017 text via GOST R 58609-2019; the 2019 text is unverified. Staleness triggers:
buying the 2019 text, or IEEE P15289 publishing. "The contract" means the owner rulings
plus `docs/COLLABORATION.md`.

## Governed set

Every tracked `*.md` except:

- `.claude/**` and `.github/**`: tool configuration read by models or GitHub.
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
- References: inline links.
- Glossary: omitted by tailoring (audience: the two owners and their agents). A doc
  defines any non-standard term at first use.

## Required content by type

H = header, R = by reference (above), T = omitted by tailoring. The body holds the rest,
each element when it applies; the critic judges.

| Type | H | R | T | Body |
|---|---|---|---|---|
| description | date, status, scope | issuer, references, history | glossary | context, notation, body, summary |
| plan | date, status, scope | issuer, references, approval, history | glossary | tasks, tools and methods, schedule, budget, resources, responsibilities, interfaces, risks, quality measures, environment and safety, training, review and reporting, other plans, change procedure, completion |
| policy | date, status, scope | issuer, references, approval, history | glossary | objectives, enforcers, effective date if not `date` |
| procedure | date, status, scope | issuer, references, approval, history | glossary | related plans and procedures, inputs and outputs, ordered steps per participant, error resolution |
| report | date, status, scope | issuer, references, history | glossary | participants, summary, context, methods, conclusions and recommendations |
| request | date, scope | none | none | subject, originator, requested response, description with due date, rationale |
| specification | date, status, scope | issuer, references, approval, history | glossary | body, assurance requirements, constraints, one unambiguous identified statement per requirement |
| record | date, status, scope | issuer, references | none | subject or category, body; `id` is the record identifier |

## Status

- `frozen` (audited or binding records, e.g. `analysis/IBM/report/*`, `REVIEW.md`,
  `DRIVER-REVIEW.md`) and `superseded`: header only, never compacted, no review row.
- `draft` and `current`: need a PASS row for the current content.

## Compactness (the critic's checklist)

1. State each fact or rule once; link it elsewhere.
2. No preamble; never restate the header.
3. No history narration in current docs; git holds it.
4. Use the shortest form that keeps the meaning; a table or list over prose when shorter.
5. No hedges or filler.
6. Every sentence must change what a reader does or knows.

## Review

1. Run the `doc-critic` agent on the doc.
2. On PASS: `python scripts/doc_check.py record <path> PASS`, then stage `docs/doc-reviews.tsv`.
3. On FAIL: fix, re-run.

Any content change voids the row. The session writes the row, so CI proves a PASS was
recorded for the current content, not that the critic ran.

## Enforcement

`python scripts/doc_check.py check` fails on a bad header, a duplicate id, a `draft` or
`current` doc without a PASS row, or a line of 60+ characters in two docs (code fences,
table separators and `frozen` or `superseded` docs excepted). A PreToolUse hook denies
`git commit` while the docs it would commit fail (the index; the worktree for `-a` or a
pathspec); CI runs the same check. The hook does not see GitHub MCP writes (`push_files`,
`create_or_update_file`) or commits not made through Bash; CI still catches them.
