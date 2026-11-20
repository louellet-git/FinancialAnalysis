---
id: T-20261120-owner-first-run
title: Do the first run on the owner's computer with the keys
area: planning
status: backlog
owner: louellet-git
head: owner
order: 1
branch:
created: 2026-11-20
acceptance: On the owner's Windows computer, `fa sync --status IBM` shows each key as found, one `fa sync IBM` run completes, and `fa dcf drivers IBM` gives $83.84 / $151.84 / $258.03.
---
Owner task. An agent cannot do it: the keys stay on the owner's computer.

1. Get the repo and build the two programs (`README.md`, "First run on your computer").
2. Copy `.env.example` to `.env` and put the keys in it (FRED, Tiingo, Alpaca). `.env` is not in git. Do
   not put a key in a chat, a document or a commit.
3. Run `fa sync --status IBM`, then `fa sync IBM`.
4. Record what fails. Some sources refuse cloud addresses but can work from a home address (the Cboe
   delayed quotes, the Yahoo chart, the FRED address with no key): `docs/DATA-SOURCES.md`.

No SEC setup is necessary: the contact is in `sec_contact.toml`.
