---
id: T-20261121-price-update-long-cache
title: Let the price update append new days to a cache file that is longer than the source history
area: importer
status: done
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
branch: claude/price-update-long-cache (not merged)
created: 2026-11-21
merged: not merged; replaced by a data fix (store commit 652dc28)
acceptance: IBM and JNJ have the 2026-11-19 and 2026-11-20 closes on the same basis as the other tickers.
---
The append code worked (JNJ +2 rows; IBM refused, correctly: its overlap differed by 0.691%, the dividend of
mid-November). It showed a larger defect: the IBM and JNJ files were Yahoo files adjusted for splits AND dividends,
while every other store file is adjusted for splits only. An append would build a file with two adjustment kinds,
and a one-week return of IBM across the dividend would be about 0.7% wrong. Decision: no append code. IBM and JNJ
now have the Nasdaq split-only history like every other ticker (10 years); the old Yahoo history is kept as
`prices_yahoo.json` in the store (no code reads it).
