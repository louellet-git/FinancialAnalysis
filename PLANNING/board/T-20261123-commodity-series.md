---
id: T-20261123-commodity-series
title: Load the uranium and gold series of the fund pages, each after a terms check
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
story: E3.S5
branch: claude/imf-uranium
created: 2026-11-23
acceptance: Each series below is in the store with a source note and its terms, or the ticket says that its terms forbid a stored copy (then the limit stays on the page); one real run for each loaded series.
---
After the token reset (2026-11-25). Owner word (2026-11-23): "On the limitations section can we fill that
out/begin work on removing limitations." Sources named by the Head of Economics; check each terms page first.

1. Uranium, monthly: IMF Primary Commodity Prices (not the FRED copy). Terms for a stored copy: not read yet.
   The IMF method differs from the Cameco spot (Sep 2026: 72.47 $/lb against about 86 to 92): state it.
2. Uranium, annual spot and term: US EIA Uranium Marketing Annual Report (public domain). The check of item 1.
3. Sprott Physical Uranium Trust (price and pounds held): the terms permit one personal copy only. A store copy
   needs the written consent of Sprott (an outward ask: the owner decides through the Head of Staff).
4. Gold price (for example the LBMA price), to separate the fee and the premium of GLD.
5. World Gold Council data files (central-bank and fund tonnes).
6. The daily holdings file of the GLD fund (its tonnes).
Not permitted: the Cameco page, UxC, TradeTech (their terms forbid a copy).

Start (2026-11-25): owner word "All three layers this week" (URA and GLD decomposition, through the Head of Staff).
First part: items 1 (IMF uranium, monthly) and 4 (a gold price, monthly: the World Bank Pink Sheet, CC BY 4.0 to
verify; LBMA is licensed). Items 2, 3, 5 and 6 stay open.

Part 1 (2026-11-26, pull request 261): item 4 done: `fa commodity get|show gold`, World Bank Pink Sheet monthly file
(CC BY 4.0, datacatalog.worldbank.org/public-licenses, read 2026-11-25): 802 months, 1960-01 to 2026-10, a monthly
average in whole USD per troy ounce; 2026-08 to 2026-10 checked against the Pink Sheet PDF. Series break: spot average
from June 2025, before that the London afternoon fixing (LBMA-linked; the owner accepts or rejects). Item 1 stopped:
the IMF terms pages answer HTTP 403 to our client (Akamai), so the terms are not read; a person reads them in a browser.

Decision (2026-11-26, the Head of Staff for Luc, who asked it to decide): gold: the World Bank monthly series for all
years, in the private store, source named (CC BY 4.0). Uranium: read the IMF series (PCPS, G001.PURAN.USD.M) at each
run through the IMF API (one small request, honest client); keep no copy in the store or the repo; name the source;
log the date and a hash of each read. A risk-compliance read of this choice comes first. The EIA annual series stays
as a public-domain cross-check.

Part 2 (2026-11-26, pull request 265): item 1 done as decided. `fa commodity show uranium` reads the IMF PCPS series
(G001.PURAN.USD.M, SDMX-JSON) at each run, one request, keeps no copy, logs date, address, bytes, SHA-256 and month range
in data/raw/commodity/imf/reads.log; `imf_pcps::read_logged(cache)` is the call for models. Real run 2026-11-26: 418
months, 1992-01 to 2026-10. Gold row: the owner decision recorded. Open: item 2 (EIA annual, the cross-check), 3, 5, 6.
