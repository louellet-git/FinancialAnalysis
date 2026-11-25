---
id: T-20261123-ifrs-mapping-cameco
title: Add the IFRS taxonomy to the SEC mapping, with Cameco as the first test
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
story: E3.S5
branch: claude/ifrs-mapping-cameco
created: 2026-11-23
acceptance: `mapcheck values CCJ` gives the standard variables of Cameco from its ifrs-full facts (40-F), with fixed-answer tests and a check of the main values against the filing; then one message to the Head of Staff with the real size of the work for the other IFRS filers, before more is built.
merged: pull request 237 (branch claude/ifrs-mapping-cameco)
---
After the token reset (2026-11-25, 11:00 Toronto time). Owner answer (2026-11-23, dialog of the Head of Staff):
"Yes, Cameco first (Recommended)". The scout (T-20261123-ura-xbrl-scout): IFRS SEC filers are 35.6% of URA
(Cameco 22.2%). Rule of CLAUDE.md: the tags go into financial_analysis/standard_variables.toml with the
scope of their variable; rust/mapcheck is the only code that makes the variables; build mapcheck and run
both test suites. Known first fault: mapcheck stops with "no SIC code in the cache" for NXE, DNN, SBSW, BHP.
The facts alone give no value for a miner: the Head of Fundamental Research designs that model.

Result (2026-11-25, QA PASS-WITH-NOTES cycle 2). mapcheck reads ifrs-full only for a filer with no us-gaap
statement facts; 40-F and 20-F count as annual forms; the filer's currency is kept. CCJ FY2025 (CAD, checked
against Exhibit 99.2 of the 40-F 0001193125-26-116229 by QA): revenue 3,481.9M, net income to owners 589.6M,
OCF 1,408.4M, capex 333.0M, debt 996.3M, cash 1,114.9M; 13 of 14 core inputs (no sbc_total). fa refuses a
non-USD filer (NO VALUE: reporting currency CAD, no conversion) on the auto path and on the import path.
CRH byte-identical; NXPI gains its 2014-16 20-F years, and FY2017 equity 13.527bn (StockholdersEquity; the
base 13.716bn included the non-controlling interest). Size of the rest: BHP and SBSW 12 of 14 inputs with the
same tags; DNN 9, NXE and ISOU 8 (pre-revenue). The open work is a currency rule and a model for a pre-revenue
miner (Head of Fundamental Research), not more mapping.
