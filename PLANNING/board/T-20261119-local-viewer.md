---
id: T-20261119-local-viewer
title: Local HTML viewer: stock look-up, Monte Carlo DCF, rates, bonds
area: tooling
status: in-progress
owner: louellet-git
branch: claude/admiring-allen-f5aaeq
created: 2026-11-19
acceptance: One terminal command starts a local server and opens a page in the browser. The page shows the standard variables and the check findings of a ticker, runs the Monte Carlo DCF for a ticker and a price, and shows the rates and the corporate bond data. The command works on Windows, macOS and Linux.
---
Owner request, 2026-11-19: one place to see what the repo can do, started from the terminal, as an HTML page.

The server is a part of the Rust crate in `rust/` (`fa serve`). It uses the same functions as the other `fa` commands. The page is one HTML file inside the program.
