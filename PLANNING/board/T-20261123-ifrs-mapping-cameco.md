---
id: T-20261123-ifrs-mapping-cameco
title: Add the IFRS taxonomy to the SEC mapping, with Cameco as the first test
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
story: E3.S5
branch: claude/ifrs-mapping-cameco
created: 2026-11-23
acceptance: `mapcheck values CCJ` gives the standard variables of Cameco from its ifrs-full facts (40-F), with fixed-answer tests and a check of the main values against the filing; then one message to the Head of Staff with the real size of the work for the other IFRS filers, before more is built.
---
After the token reset (2026-11-25, 11:00 Toronto time). Owner answer (2026-11-23, dialog of the Head of Staff):
"Yes, Cameco first (Recommended)". The scout (T-20261123-ura-xbrl-scout): IFRS SEC filers are 35.6% of URA
(Cameco 22.2%). Rule of CLAUDE.md: the tags go into financial_analysis/standard_variables.toml with the
scope of their variable; rust/mapcheck is the only code that makes the variables; build mapcheck and run
both test suites. Known first fault: mapcheck stops with "no SIC code in the cache" for NXE, DNN, SBSW, BHP.
The facts alone give no value for a miner: the Head of Fundamental Research designs that model.
