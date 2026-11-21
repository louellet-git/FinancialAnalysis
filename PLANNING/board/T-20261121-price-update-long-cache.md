---
id: T-20261121-price-update-long-cache
title: Let the price update append new days to a cache file that is longer than the source history
area: importer
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: technology
order: 2
branch: claude/price-update-long-cache
created: 2026-11-21
acceptance: `fa prices get IBM JNJ --source nasdaq` appends the days after the cache's last date (with the overlap check) and keeps the older history; a fixed-answer test with a long cache file and a shorter source history.
---
IBM and JNJ have Yahoo price files with the full history (16,330 rows). The Nasdaq update refuses them: "the full
history has 2514 rows; the cache has 16330: the cache stays", so they stop at 2026-11-18 and drop out of the week
attribution of the S&P 100 (Head of Fundamental Research, 2026-11-21). The update must append the new days after
the overlap check, not compare the length of the full history.
