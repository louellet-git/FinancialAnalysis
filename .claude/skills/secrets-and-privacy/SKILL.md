---
name: secrets-and-privacy
description: Use when you handle a key, a token, a password, an e-mail address, the SEC contact text, or a private file, and when a secret appears in a chat, a file, a commit or a log. It says where a secret may be and what to do when one leaks.
---

# Secrets and privacy

This repo is public. A secret in a commit stays in the history.

## Where a secret may be
- In `.env` (not in git) or in the environment. `.env.example` has names only.
- In a secure form of the tool (`request_credential`) for a cloud session. The value
  never enters the chat.
- The SEC contact text (a name and a contact) is in the SEC contact file
  (`sec_contact.toml`, a project address, public on purpose) or in `SEC_USER_AGENT`. Do not write it in a
  document, a test, a fixture, or the client name of other sources.

## Where a secret may not be
Source code, tests, fixtures, documents, the board, commit messages, pull request texts,
logs, the output of a command, a chat message, a report.

## Rules
1. Read a key from the environment. Never print it. A debug line prints "key set" or "key
   missing".
2. Do not ask a person to paste a key into the chat. Ask for the secure form or `.env`.
3. Before you give work to the orchestrator, search the diff: `git diff origin/main...HEAD |
   rg -i "api[_-]?key|secret|token|password|@[a-z0-9.-]+\.[a-z]{2,}"`. Check each hit.
4. `data/`, `.env`, and private framework files are not committed.

## When a secret appears
1. Stop the work that touches it. Do not copy it into your report.
2. Tell the orchestrator the file and line (or the place in the chat), not the value.
3. The owner of the key revokes it and makes a new one. This is the first step. Removing
   the text from git does not make the old key safe.
4. If it is in a commit that is not pushed, the orchestrator removes the commit. If it is
   pushed, the owner decides (a history rewrite and a revoked key).
5. Add a check or a `.gitignore` line if the leak can happen again.

## Check
- No secret in the diff, the history of the branch, or the report.
- Keys come from the environment only.
