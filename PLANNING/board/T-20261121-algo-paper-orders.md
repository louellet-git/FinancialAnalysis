---
id: T-20261121-algo-paper-orders
title: Paper orders for fa algo (Alpaca paper account only, controls 1 to 8)
area: trading
status: backlog
owner: louellet-git
head: quant
story: E4.S1
order: 3.3
branch:
created: 2026-11-21
requested_by: louellet-git
acceptance: `fa algo run --send` sends orders to the Alpaca paper account only, on the laptop or the e2-micro, with each control 1 to 8 of `docs/QUALITY-STANDARDS.md` section 8 covered by a test; the default is a dry run that sends nothing; the gate "Order code (paper only)" (code-reviewer and risk-compliance-officer) passes.
---
Starts after PR 131 (`fa algo`, the simulator) merges. Owner words (2026-11-21, `docs/QUALITY-STANDARDS.md` section 8):
"programs can and will place orders"; "paper orders first". A live order needs a new owner approval.

## Rules (control 0, the Alpaca terms: `docs/DATA-SOURCES.md`, section "Alpaca paper trading", PR 154)
- Host: the constant `https://paper-api.alpaca.markets`; no setting changes it; no redirect.
- Each person has an own paper account and own keys; one account for each sleeve owner; no shared account.
- The order log is private: out of git and out of the store.
- `client_order_id` at most 128 characters; our own guard against a duplicate.
- No documented rate limit: the caps of control 6.

## Cloud and keys (Quality and Risk, 2026-11-21, through the Head of Technology)
a. An order goes out only when `ALPACA_PAPER_KEY_ID` and `ALPACA_PAPER_SECRET_KEY` are set, the account check says
   paper, and the command has `--send`. The default is a dry run.
b. A test with a mock transport: with the variables unset, or without `--send`, no order request is made at all
   (not even one that a proxy could sign). The cloud proxy key is never used for a sleeve.
c. The run log names the host machine and the account check result (paper: yes or no; never the account number).
d. Order runs: the laptop or the e2-micro only; a cloud session never passes `--send`. Controls 3 to 6 (caps,
   stops, kill file) apply on every machine.

## In the same pull request
- `docs/models/paper-trader.md` line 6 changes (QUALITY-STANDARDS section 8).
- The test `no_order_code_in_the_repo` of PR 131 changes: it allows the paper host constant only.
- The model document of `fa algo` and its records (A-155 to A-159, or a new range from the Head of Technology).
