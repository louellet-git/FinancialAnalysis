---
id: T-20261119-share-count-and-crosscheck
title: Add the share-count check and the external cross-check to the mapping tests
area: importer
status: done
owner: louellet-git
branch: claude/admiring-allen-f5aaeq
created: 2026-11-19
acceptance: `mapcheck` reports a share count that does not agree with net income divided by EPS, the importer corrects a share count that is wrong by a factor of 1,000 or 1,000,000, and one command compares the core inputs with Yahoo Finance for a list of tickers.
---
Owner approval, 2026-11-19 (items 1 and 3 of the mapping assessment).

Source: a comparison with Yahoo Finance for 70 companies. McDonald's reports 717 where the share count is 717 million. The comparison was a script that ran one time; it must be a command of the framework.
