---
id: T-20261123-non-sec-accounts-scout
title: Scout the sources of the accounts of the URA holdings that do not file with the SEC
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
story: E3.S5
branch: claude/non-sec-scout
created: 2026-11-23
acceptance: One short table for the non-SEC holdings of URA (27.0% of its weight): where each company publishes its accounts, the format (structured data or PDF), the terms for automatic reading and for a stored copy, and whether a key or an account is necessary; what a program cannot read is named plainly.
merged: pull request 242 (branch claude/non-sec-scout)
---
After the token reset and after the Cameco test of the IFRS mapping. Owner word (2026-11-23, through the Head
of Staff): "So let's grab the other 27%". Companies (the scout T-20261123-ura-xbrl-scout): Kazatomprom 4.40,
Paladin 3.14, eight Japanese and Korean companies 12.46, Aecon 1.64, CGN Mining 1.09, four Australian miners
2.49. Look first at the national filing systems with structured data for many companies (Japan, Korea),
because one source there covers about 12.5% of the fund. Use the skill data-source-onboarding. Then one source
first (the most weight for the least work), with tests; the real size of the rest goes to the Head of Staff
before more is built. A key, an account or terms that need the owner: one question to the Head of Staff.

Result (2026-11-25, read only, about 22 page reads; some official pages gave 503 or 404).

| Source | Holdings (weight) | Format | Automatic reading | Stored copy | Key |
|---|---|---|---|---|---|
| EDINET API v2 (Japan, FSA) | ITOCHU, Mitsubishi Heavy and the other Japanese holdings (Japan + Korea 12.5%) | XBRL through an official API | Yes, "moderate" use (FAQ Q14), poll about once a minute (Q6) | Public Data License 1.0: free reuse, the source must be named | Free key, mandatory (FAQ Q17); a non-resident's right to a key: not found |
| OpenDART (Korea, FSS) | Samsung C&T, Doosan, Hyundai E&C, Daewoo E&C, KEPCO E&C, GS E&C | JSON or XML API (from third-party documents) | Not verified: the official site gave 503 | Not found | Free key (third-party documents); a non-Korean's right: not found |
| SEDAR+ (Canada) | Aecon 1.6% | PDF | No: the terms forbid scraping and automated means | Limited extracts only | none |
| ASX (Australia) | Paladin, Deep Yellow, Silex, Bannerman, Boss (5.6%) | PDF | No: the terms forbid robots and screen scrapers | Private and personal use only | Paid feeds exist |
| HKEXnews (Hong Kong) | CGN Mining 1.1% | PDF | No | No (written permission needed) | none |
| London RNS, FCA NSM, KASE | Kazatomprom 4.4% | PDF (IFRS statements) | Not found | NSM: "any lawful purpose" (from a snippet) | none |

A program cannot read the PDF-only sources, and for SEDAR+, ASX and HKEX the terms forbid it. That part
(about 12.7% of the fund) stays a limit on the page. First source: EDINET (structured, official API, free,
reuse permitted). Second: OpenDART, after the person reads its terms in a browser. Both keys go in a query
parameter, not a header, so the cloud proxy cannot add them: a run with a key happens on the laptop or the
e2-micro, with the key in an environment variable there, never in the repo.
