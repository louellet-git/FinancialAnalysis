---
id: T-20261121-algo-host-gcp
title: Set up the always-on host of the quant algo engine on a free Google Cloud e2-micro
area: trading
status: backlog
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
branch:
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
