---
id: T-20261121-algo-host-gcp
title: Set up the always-on host of the quant algo engine on a free Google Cloud e2-micro
area: trading
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
branch: claude/algo-host-guide
created: 2026-11-21
acceptance: A setup guide (at most 2 pages) that the person follows to make the machine; `fa algo run` runs there as a service in market hours; the journal reaches git with a key that can push only the journal; the cost after the first day is known (0 or the IPv4 charge) and written here.
---
Answer of the person (2026-11-21, the question dialog): Yes to a free Google Cloud e2-micro in us-east1 for the
quant algo engine (`T-20261121-algo-1-paper-engine`, Head of Quant Research). Research:
`chiefs/louellet-git/quant/algo-host-2026-11-21.md` (Quant branch `claude/quant-algo-paper`).

- The person makes the Google Cloud account (a card for the identity check). The Head of Technology writes the
  steps: e2-micro, us-east1, the free standard disk size, a small Linux image, the build of `fa`, a systemd
  service for market hours, logs, restart on failure.
- Keys: a separate Alpaca key for this machine (the request limit of 200 a minute is per account: the daily option
  save also uses it; keep the engine at about 60 a minute or less). A GitHub deploy key or a fine-grained token that
  can write only the journal path. No key in the repo.
- Cost: Google's pages do not say if the external IPv4 of a free e2-micro is charged (about $3.65 a month if it is).
  Read the bill after day 1 and write it here. If it is charged and the person does not want it, the next option is a
  DigitalOcean machine at $4 a month (a new question).
- First: one live run on the person's laptop on Monday 2026-11-23 (no question needed).
- The engine sends no order to any broker (fills in its own simulator).

## Details from the Head of Quant Research (2026-11-21), for the guide
- Command: `fa algo run --strategy demo` (`--symbol`, `--poll-secs` default 1.0, `--max-pos`, `--max-loss`, `--until HH:MM`
  default 15:55 New York). Start before 9:30 New York; it waits for the open, ends flat, exits on a weekend or after
  5 read errors in a row. A systemd timer at 9:20 New York on market days; a restart is safe, but a restart on the same
  day resets the max loss.
- Files under `<repo>/data/processed/algo/<date>/`: `demo-inputs.jsonl`, `demo-journal.jsonl`, `demo-summary.json`.
  Kill file: `<repo>/data/processed/algo/STOP` (the engine prints its absolute path at the start).
- Rule: the raw quotes (`demo-inputs.jsonl`) do NOT go into git or the data store (the Alpaca terms for this use are
  not read). Only `demo-journal.jsonl` and `demo-summary.json` go to the journal repo; `inputs.jsonl` stays on the
  machine and is deleted after N days.
- The journal goes to its own small repo (for example `louellet-git/fa-algo-journal`, made by the person): a GitHub deploy
  key is limited to one repo, not to a path.
- Rate: 60 calls a minute by default; with the daily option save (about 50 calls in one minute) the account stays under 200.
- Build here (Linux x86_64, `--release --locked`) and copy the binary; do not build on the 1 GB machine.

Guide written (2026-11-25): `docs/guides/algo-host-gcp.md`. The binary goes into the private store, folder `bin/`, with `bin/SHA256SUMS`. Open: the journal push (after the person makes the journal repo and its deploy key), the holiday check (Head of Quant Research), the cost after day 1.
