# Plan: algo trading on the Alpaca paper account (2026-11-25)

Owner goal (2026-11-25): "begin algo trading on a paper account (continuous data w fees and spreads and all, not
simulated by us ideally)". Story E4.S1. Facts: `chiefs/louellet-git/quant/alpaca-paper-facts-2026-11-25.md` (20 Alpaca pages).
Limits that stay: paper account only; QUALITY-STANDARDS section 8 (controls 0 to 8); gate "Order code (paper only)".

## (a) The shortest safe path
1. Merge PR 131 (`fa algo`, our simulator) now. Its live data loop ran today from this session, in market hours:
   36 live SPY quotes (IEX) in 3.1 minutes, 11.8 requests a minute, no order, the kill file printed, exit 0. The
   person's laptop run is not necessary for the data path. It is necessary only to run it on Windows (not tested).
2. `T-20261121-algo-paper-orders`: a paper broker in `fa algo` in place of the simulator fill. Default dry run;
   `--send` sends; controls 1 to 8 with a test each. Order in three steps, each with its own gate pass:
   A. SPY shares, 1 share, limit orders at the quote (the simplest order; it proves the pipe and the controls).
   B. One SPY option, single leg, 1 contract.  C. A SPY condor as one multi-leg order (at most 4 legs), 1 lot.
   Conflict: control 6 of section 8 says "no option". The person permitted option orders on paper (1 contract,
   defined risk only; 2026-11-25, through the Head of Staff). No code for B or C before the new text of control 6
   passes a review of Quality and Risk.
3. Each order writes our simulator's fill next to the broker's fill: the quote at the decision, the fill price,
   the delay, partial fills. A daily report compares them. This is "spreads by the broker, not by us".

## (b) What Alpaca paper gives, and what we still add
- It gives: a fill at the ask or bid when the order is marketable (no mid, no price improvement); random partial
  fills (10%); multi-leg option orders; SPY and index options (Level 3); 200 requests a minute for each account.
- It does not give: regulatory fees (ORF $0.015, OCC $0.025 a contract, CAT, SEC on sells: we add them from the fee
  schedule of 2026-11-17); market impact, depth, queue; dividends; documented assignment or early exercise.
- The option fill rule on paper is not documented: we measure it with step B before step C.
- The options data of the free plan are indicative. The report must say which quote each paper fill used (the
  real one or the indicative one) before step C is read as a cost measurement.
- Data: the free plan gives IEX for stocks and the indicative feed for options (not OPRA). OPRA needs Algo Trader
  Plus ($99 a month); not needed for steps A to C. The person decides later, from what the reports show.

## (c) Continuous runs and what only the person can do
- A cloud session is not always on. Host: the Google e2-micro (`T-20261121-algo-host-gcp`, Technology; the person
  said yes). Order runs: the e2-micro or the laptop only; a cloud session runs dry runs and tests, never `--send`.
- Keys: this session reaches both Alpaca hosts through the proxy (HTTP 200 today); the Head of Staff's session gets
  401. The order code reads only `ALPACA_PAPER_KEY_ID` and `ALPACA_PAPER_SECRET_KEY` and never a proxy key.
- Only the person can: make the Google VM (free tier); make his own paper keys in the Alpaca dashboard and set them
  on the VM (never in a chat); merlan1996 uses an own account. A live account has conditions of residence; check them before any live plan.

## (d) The first strategy
No strategy passed its gate (alpha-1 stopped at gate 1). The first paper orders are a test of the process at the
smallest size, judged on fills, spreads and fees against our simulator, not on profit. Steps A and B use the
`demo` rule. Step C uses the s05 condor at 1 lot, as a cost measurement (B8 of `T-20261121-quant-doc-findings`): the
spread is its main sensitivity (spread x2 removes 74% of its mean P&L), so real paper fills answer the open question.

## Order and size of the work (about 8% of the week, with Technology)
1. Today: merge PR 131 (main merged into it; one check run). 2. Order adapter step A (one agent, one gate).
This week: steps 1 and 2 only. Steps B and C only if room stays, and after the review of control 6.
Then the daily comparison report, and the host when Technology and the person are ready.
