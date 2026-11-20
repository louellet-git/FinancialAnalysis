---
id: T-20261120-data-store-prices
title: Put the prices in the data store and write the update procedure
area: importer
status: backlog
owner: louellet-git
head: technology
order: 0.7
branch:
created: 2026-11-20
acceptance: The store `louellet-git/fa-data` is private and has the daily prices of the two ticker lists and of SPY, a new clone gives `fa dcf screen` a price ratio for each ticker of the two lists, and the store `README.md` gives the steps of an update.
---
State on 2026-11-20: the store has the SEC company facts (1,249 tickers), the SEC lists of earnings
reports (505 tickers) and the GSW rate curve file. A new clone takes about 17 seconds, and
`fa dcf auto IBM --cache <clone>` runs on it.

Open:
1. The store repo is public. The price data comes from sites and vendors whose terms do not permit a
   public copy. The prices go in the store only after the owner makes the repo private.
2. The prices of `next500.txt` are being downloaded on the machine of the Head of Technology (Nasdaq
   source). The price files of the first list are from Yahoo and Nasdaq (2026-11-19).
3. Write the update steps in the store `README.md`: run `fa sync` on one machine, copy the changed
   files, change `SNAPSHOT.md`, push in parts of less than 40 MB.
4. Git keeps each old version. When the store repo is larger than about 1 GB, start it again from one
   commit (owner decision).
