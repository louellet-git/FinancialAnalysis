---
id: T-20261119-viewer-company-report
title: Join the company report page with the local viewer
area: engine
status: in-progress
owner: louellet-git
branch: claude/viewer-company-report
created: 2026-11-19
acceptance: The page of `fa serve` has a view that shows the company report of a ticker (price and valuation, fundamentals, Monte Carlo DCF panels, driver model, options market), and a person starts only one command to see all views.
---
Owner request, 2026-11-19: the repo has two HTML outputs, the local viewer (`fa serve`, Rust) and the earlier company report (`python -m scripts.visualize_dcf TICKER`, Python with the plotly library, one HTML file for each ticker). Join them.

Step 1 (this ticket): the viewer gets a view "Company report" that makes the report of a ticker with the existing command and shows it in the page. No new Python code.

Step 2 (after the DCF engines are Rust, ticket `T-20261119-stochastic-dcf-drivers` and step 7 of `T-20261119-python-to-rust`): the report sections become Rust code and views of the page, and `scripts/visualize_dcf.py` is deleted.
