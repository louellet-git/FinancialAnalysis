---
id: T-20261123-ura-xbrl-scout
title: Scout which URA holdings have SEC facts that mapcheck can read
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
story: E3.S5
branch: claude/e3s5-data-tickets
created: 2026-11-23
acceptance: One short table of the URA holdings by class (us-gaap readable, IFRS SEC filer, not an SEC filer, fund or trust), with the weights.
merged: pull request 222 (branch claude/e3s5-data-tickets)
---
Read-only. URA holdings of the N-PORT of 2026-07-31 (`fa fund holdings URA`), the top 40 holdings
(99.98% of the weight). Taxonomy from the SEC company facts (store, or 8 SEC requests).

| Class | Weight | Holdings (weight %) |
|---|---|---|
| us-gaap, mapcheck can read it now | 25.8% | Oklo 6.09, Uranium Energy 4.81, Centrus 3.46, NuScale 3.04, X-Energy 3.03 (10-Q only, short history), Energy Fuels 3.03, Nano Nuclear 0.73, Ur-Energy 0.48, Uranium Royalty 0.45, Terrestrial Energy 0.35, Lightbridge 0.31 |
| IFRS SEC filer (40-F or 20-F; needs IFRS mapping) | 35.6% | Cameco 22.18, NexGen 6.07, Denison 2.73, Sibanye 2.14, BHP 2.06, IsoEnergy 0.46 |
| Not an SEC filer (needs a different source) | 27.0% | Kazatomprom 4.40, Paladin 3.14, eight Japanese and Korean companies 12.46, Aecon 1.64, CGN Mining 1.09, four Australian miners 2.49 |
| Fund, trust or collateral | 11.6% | Sprott Physical Uranium Trust 6.74, Yellow Cake 1.99, repo and collateral counterparties 2.84 |

Not checked: `mapcheck values` on the IFRS filers (it stops with "no SIC code in the cache" for NXE, DNN,
SBSW, BHP); whether the Sprott trust files a 40-F. Result: the foreign-filer work is IFRS mapping for about
36% of the weight (Cameco alone 22%); about 27% is not in SEC data at all.
