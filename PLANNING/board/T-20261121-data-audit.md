---
id: T-20261121-data-audit
title: Full data audit of the store (defects of the kind found on 2026-11-21), with an owner for each defect
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 1
branch: claude/data-audit
created: 2026-11-21
acceptance: Each check below has a count on the store, each defect found has an owner and a ticket or a fix, and the S&P 100 defects are listed first.
---
Word of the person (2026-11-21, relayed by the Head of Fundamental Research): "Can the head of tech check if there are
other problems like those" (the split basis, the point-in-time mapping FAIL of V, BRK-B, COP and NEE, the IBM and JNJ
price update).
Checks: (1) point-in-time mapping FAIL where the normal mapping gives WARN (23 sp500 tickers on 2026-11-13; after
`T-20261121-pit-supplement-fill`); (2) price files: missing, no meta, stale; (3a) each universe ticker has a price file,
company facts, a share count at a past date and a CIK; (3b) price gaps against SPY trading days, 2 years; (3c) a one-day
jump above 40% with no split row; (3d) the model share count x price against a plausible market value (class or scale
errors); (3e) two tickers with one CIK beyond the known share classes; (3f) rates and macro files: last date, missing
days, duplicate dates; (3g) template counts ("template new", "history too short").

## Results (2026-11-21, store f634633 to 652dc28; read-only data-engineer run)
S&P 100 first. Owner of each defect: Technology unless named.
| Check | Defect | Tickers | Ticket |
|---|---|---|---|
| 3a, 3d | Share count by class: the SEC company facts drop the cover count filed per share class, so the latest count is from 2010-2016 or covers one class | **V, MA, BRK-B, GOOG, GOOGL, META, PLTR, DELL, CMCSA, UPS**; next500: HEI, UHAL, LEVI, AUR | `T-20261121-shares-by-class` |
| 3d | Share-count scale error (thousands against units or millions) | **MCD** (weighted 194K), ERIE, FOX, FOXA, WMG, SEI, NTNX, CYTK, LOAR; also WAT (10-Q) | `T-20261121-share-count-scale-guard` |
| 1 | Point-in-time mapping FAIL where the normal mapping gives WARN | 23 sp500 tickers incl. **V, BRK-B, COP, NEE** | `T-20261121-pit-supplement-fill` (count after the fix) |
| 2 | Price file and history: IBM and JNJ were Yahoo files (split+dividend) and stopped at 11-18 | **IBM, JNJ** | done: store 652dc28 (Nasdaq split-only like all others; Yahoo kept as prices_yahoo.json) |
| 2 | No price file | **BF-B**; next500: MOG-A, ET, SUN, CFR, DDS | `T-20261121-missing-price-files` |
| 3c | A one-day move above 40% with no split row: possible missing split | QXO 2024-07-30, CORT 2025-12-31, CRDO 2023-02-15 | `T-20261121-missing-price-files` |
| 3c | Price history of a predecessor (reverse merger, ticker reuse) | ORKA, SYRE, PRAX, SMMT 2018, KNTK, DKNG 2019 | Fundamental Research: do not use the pre-merger history |
| 3b | Gap against SPY days in 2 years | SUPN 2026-01-23 | `T-20261121-missing-price-files` |
| 3e | Two tickers with one CIK beyond GOOG/GOOGL, FOX/FOXA, NWS/NWSA | none | - |
| 3f | Rates, macro, FedInvest, futures, VIX files | no defect (GSW lags a week; FedInvest and VIX need 11-20) | - |
| 3g | Template counts | sp500: template new HONA, VYLR; short history 9 (incl. **NEE**); next500: FAIL 7 (FWONA, NLY, BWXT, AGNC, PRAX, CYTK, FPS) | Fundamental Research (template choice of new listings) |
Also found: IBM and JNJ were the only files with split+dividend adjustment; a cross-section mixed price returns with total returns for them before 652dc28.
