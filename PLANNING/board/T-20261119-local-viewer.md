---
id: T-20261119-local-viewer
title: Local HTML viewer: stock look-up, Monte Carlo DCF, rates, bonds
area: engine
status: review
owner: louellet-git
branch: claude/admiring-allen-f5aaeq
created: 2026-11-19
acceptance: One terminal command starts a local server and opens a page in the browser. The page shows the standard variables and the check findings of a ticker, runs the Monte Carlo DCF for a ticker and a price, and shows the rates and the corporate bond data. The command works on Windows, macOS and Linux.
---
Owner request, 2026-11-19: one place to see what the repo can do, started from the terminal, as an HTML page.

The server is a part of the Rust crate in `rust/` (`fa serve`). It uses the same functions as the other `fa` commands. The page is one HTML file inside the program.

State, 2026-11-19: `fa serve` is built (`rust/src/serve.rs`, `rust/src/viewer.html`). The views Stock, Monte Carlo DCF, Rates and Option bots work with real data. The Bonds view needs FRED; in the cloud container of the work session FRED did not answer the Python client, thus this view was tested only for its error text. Not tested: Windows and macOS (the browser start and the Python default). The Monte Carlo view works for the tickers that have a driver model (IBM, JNJ).
