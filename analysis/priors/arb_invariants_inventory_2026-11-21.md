# Inventory of no-arbitrage relations (step 1)

Ticket: `PLANNING/board/T-20261121-arb-invariants` (step 1). Date: 2026-11-21. Cache: `data/raw`, listed 2026-11-21.
This file lists each relation that our data can test, what we check today, and what is missing. It runs no command and changes no code.
A break on indicative or closing data is a measurement of a file. It is not a tradable price.
Sources: `docs/models/options-static-arbitrage-check.md` (OSA), `futures-check.md` (FUT), `treasury-fit-rich-cheap.md` (TFR),
`heston-bates.md` (HB); tickets arb-3 (`T-20261120-arb-3-index-parity-box`), arb-4 (`T-20261120-arb-4-futures-basis`),
`T-20261121-quant-doc-findings` (QDF, items O1 to O9).

## What the cache holds (2026-11-21)
- `SPX/options/2026-11-20.json`: one day, Alpaca indicative, roots SPX (10412 quotes) and SPXW (21634), 53 expiries.
- `IBM/options/2026-11-19.json`, `JNJ/options/...`: one day each, Cboe delayed (American).
- Not in cache: XSP, SPY, QQQ chains; VIX, VIXW chains; SR3; SOFR. SPY and QQQ have prices only (`SPY/prices.json`, `trading/SPY_1d.csv`).
- `futures/cboe_settle_2026-11-19.csv`, `futures/cboe_VX_<expiry>.csv` (history of each monthly), `futures/yahoo_ZQX26.CBT.json`, `futures/nyfed_effr_2026-10-25_2026-11-19.json`.
- `rates/treasury_nominal_2026.csv` (par), `treasury_bill_rates_2026.csv`, `treasury_real_2026.csv`, `gsw_feds200628.csv`; `prices/fedinvest_<date>.csv` (58 days to 2026-11-19).
- `vix/VIX_History.csv` (daily, to 2026-11-19); `trading/_VIX_1d.csv`, `_VIX1D_*`, `_VXN_*`, `SPY_1d/1h/5m.csv`, `_IRX_1d.csv`.

## Table
Columns: id, relation | instruments | cache | checked today | tolerance, time match | a break means | effort, value.

| id, relation | instruments | cache | checked today | tolerance, time match | a break most likely means | effort, value |
|---|---|---|---|---|---|---|
| OPT-MONO: call price falls and put price rises with K: `C(K1) >= C(K2)`, `P(K1) <= P(K2)`, K1<K2 | SPX, SPXW, IBM (SPY, QQQ, XSP later) | `<T>/options/<D>.json` | yes: `fa options check`, `option_arb.rs` `check_chain` (monotony) | executable (buy ask, sell bid); EPS 1e-9, `--min-size`; one file, quotes not at one time | stale or indicative quote; one bad quote gives many pair breaks | done; high (cheap, daily) |
| OPT-SLOPE: `0 <= -dC/dK <= df`, `0 <= dP/dK <= df` | same | same | yes: `check_chain` (slope; no discount, A-137) | as above | stale leg; the bound uses no discount, so it is loose | S to tighten with df; low |
| OPT-BFLY: convexity, `C(K1)-2C(K2)+C(K3) >= 0` (equal spacing; weights if not) | same | same | yes: `check_chain` (butterfly, consecutive triples) | as above | a wide-spread or stale middle quote | done; high |
| OPT-CAL: calendar, total variance `w(T) = sigma^2 T` rises with T at fixed log-moneyness; price form: `C(K,T2) >= C(K,T1)` for American | SPY, QQQ, IBM; SPX | same | partly: price form for American only; no total-variance form for index (OSA step 6) | as above; time is days/365, no clock | time gap between expiries; model input (forward, rate); stale far leg | M; high (the index case is untested) |
| OPT-INTR: `C >= max(S-K df, 0)` (European, on F); American `C >= S-K` | all chains | same | yes: `check_chain` (intrinsic, bound; American only; needs spot) | EPS 1e-9; spot is a last price (A-124) | spot time gap; dividend not in S (O8: silent without spot) | S to extend to F; medium |
| OPT-PARITY: European `C - P = df (F - K)` per strike; across strikes `F` is constant | SPX, SPXW, XSP | same | partly: parity only gives the forward (median of 3 strikes, `option_iv.rs`); no per-strike spread test; American upper bound `C - P <= S - K df` only | none for European; 3 against 5 strikes (O2) | stale leg, one-sided quote, rate input | S; high (spread of F across strikes is a quote-quality score) |
| OPT-AMER: `S-K <= C-P`-type bounds `S - K <= C - P <= S - K df` (no dividend) with dividends: `S - D - K <= C - P` | SPY, QQQ, IBM | same | partly: upper bound only; no dividend input (A-124) | none; spot a last price | dividend missing; early exercise; borrow | M; medium (needs a dividend source) |
| BOX: `K2-K1 = df (C1-C2+P2-P1)`; implied rate at ask (borrow) and bid (lend) | SPX (European) | SPX chain | no (arb-3 step 1) | none; same file; method of NY Fed: regress put-minus-call mid on K, R2 >= 0.99999 | indicative quote; wide spread; stale; the rate is not a Treasury rate (SPX box about 35 bp above) | M; high (tests the rate input of all checks) |
| BOX-RATE: box rate against Treasury bill curve and SOFR of the day | SPX, par and bills | `rates/treasury_*_2026.csv`; SOFR not in cache | no | none; bill curve is daily, chain is one file | a spread (not a break); model input | S after BOX; medium (SOFR is missing) |
| IDX-SPY: `F_SPX(T) = 10 F_SPY(T)` up to the dividend and ratio drift; scale 10 | SPX/SPXW, SPY, XSP | SPY, XSP chains not in cache | no (arb-3 step 2) | none; need the same minute; AM or PM root as SPY | time gap; dividend difference; SPY ratio drift (#280) | M; high (first test of two independent feeds) |
| IDX-FEED: one day of SPX from Cboe and Alpaca: bid, ask, F agree | SPX | only Alpaca copy | no (arb-3 step 3) | none | indicative modification | S; medium (once a Cboe copy exists) |
| VIX-SPX: the VIX from the SPX chain (Cboe method, `2/T sum dK/K^2 e^(rT) Q - (F/K0-1)^2/T`, 30-day interpolation) equals the published VIX | SPX, SPXW; VIX | SPX chain; `vix/VIX_History.csv` (close only) | no | none; chain one stamp, VIX close is 16:15 ET | indicative quote; chain time not close; rate; wrong 30-day weights | M; high (owner question; checks the whole chain) |
| VIXOPT-FWD: the parity forward of a VIX option expiry equals the VX settle of that expiry | VIX options, VX | VIX chain not in cache; VX in cache | no (arb-4 step 2) | none; settle at close against a chain stamp | stale leg; indicative; wrong future (monthly against weekly) | M; high (needs VIX chain copy) |
| VX-TERM: VX settle of the first contract is near the VIX; the term structure has no jump; the spot-month contract converges to the VIX at expiry | VX, VIX | `futures/cboe_*`, `vix/VIX_History.csv` | no | none; daily settle against daily close | not a strict relation (a basis); a weekly contract mis-labelled | S; medium (a sanity check, not a bound) |
| VX-FILES: VX settle file against the history file of the same date | VX | `futures/cboe_settle_*`, `cboe_VX_*` | yes: `fa futures check`, `futures/check.rs` `compare_vx` | 0.005 index points (A-128; about 100 times the rounding, O4); same date, daily | a copy fault; both files are from Cboe; a 403 stops the run (O9) | S to fix; medium |
| FUT-CARRY: `F = S e^((r-q)T)` (ES, VX excluded) | ES, NQ, ZN, with SPX or SPY | futures not in cache (CFTC files only) | no (arb-4 step 4) | none | dividend and rate input; roll date | L; low now (no spot and no dividend source) |
| ZQ-EFFR: `100 - ZQ price` = mean EFFR over the month | ZQ, EFFR | `yahoo_ZQX26.CBT.json`, `nyfed_effr_*.json` | yes: `fa futures check`, `check_zq` (one date, one contract) | 5 bp (A-129); a Yahoo bar, not a CME settle | a target change in the month (O6); not the settle | S to extend to all dates and months; medium |
| SR3-SOFR: 3-month SOFR future implied rate against the SOFR path and the ZQ strip | SR3, SOFR, ZQ | not in cache | no (arb-4 step 3) | none | needs data | L; low until data exists |
| UST-SMOOTH: the par curve and the forward curve have no jump; forward rates are positive; df falls with T | Treasury par, `fa.curve` | `rates/treasury_nominal_2026.csv` | partly: the bootstrap in `fa recon bonds` (`curve.rs`) reports a monotone-convex fit; no break count | none | a data fault in one tenor; a convention (BEY bills) | S; medium |
| UST-FIT: each note and bond is within cost and fit error of the fitted NSS curve (residual bp, `z`) | FedInvest prices | `prices/fedinvest_<D>.csv` | yes: `fa recon rich-cheap`, `richcheap.rs` `analyse`; `fa recon bonds` | cost = max(tick, bid-ask); candidate: `\|z\|` >= 2 (A-105); end-of-day bid, one day | a model residual (RMSE 1.46 to 2.50 bp), off-the-run premium; not a trade (TFR) | done; medium (the runner needs a data return) |
| UST-BILL: bill yield against the note curve and the par curve | bills, par | `treasury_bill_rates_2026.csv`, fedinvest | partly: `fa recon bills` (`billdiag.rs`) | none; bills 4 to 12 bp rich in 3 dates (hypothesis, TFR) | a convention (BEY, T+1) | S; low |
| BILL-OIS: 1M, 3M bill against SOFR and EFFR compounding | bills, SOFR, EFFR | EFFR window only; SOFR not in cache | no | none | a spread (bills are tax-free and scarce); not a bound | M; low (a spread, not an arbitrage) |
| REAL-NOM: breakeven `nominal - real` against CPI swaps and carry | nominal, real par, CPI | `treasury_real_2026.csv`, `bls_cpi.json` | no | none | a measure (no bound) | M; low |
| VOL-HB: the fitted Heston or Bates surface has no butterfly or calendar break; put-call parity of the model price is exact | SPX/SPY fits | the chain file | partly: parity holds by construction (HB limits); no fit surface arbitrage count | 1e-9 on the model | a fit artifact; not a market break | M; medium |
| BAR-CLOSE: SPY close against SPX close / 10 (scale); VIX close inside the high-low band | SPY, SPX, VIX daily | `trading/SPY_1d.csv`, `vix/VIX_History.csv` | partly: `price_check.rs` compares price sources (not this relation) | none | a data fault or a corporate action | S; low |

## (a) The 3 relations to add first
1. **OPT-PARITY (per strike spread of F) with BOX.** The data is in the cache (SPX, IBM). The code has the forward and the discount factor. The F-spread across strikes and the box rate tell us how good the quotes and the rate input are. All other options checks depend on the rate and the forward (O2, A-126). Effort S and M.
2. **VIX-SPX (Cboe method).** Owner question. The cache has the SPX chain and the VIX close. It is a full test of the chain, the rate and the time (A-125). Read the VIX close with a stamp gap (chain stamp against 16:15 ET) and say so.
3. **OPT-CAL for the European index (total variance), with the other chain checks on every daily copy.** The index case has no calendar test today, and the chain copies start 2026-11-23 (B8). It uses the same file and only needs `option_iv.rs`.

Not first: VIXOPT-FWD and IDX-SPY are high value, but they need a chain that is not in the cache (VIX, SPY or XSP).

## (b) What a daily runner needs from the existing code
- Call as they are (pure, no network): `option_arb::check_chain` (takes the snapshot, a flag, a rate closure and `CheckOpts`; gives a `Report` with counts, `Dropped`, breaks), `Report::bands`, `option_iv::{black76, implied_vol, forward_from_parity_n}`, `option_arb::compare`, `futures::check::{parse_effr, expected_average}`, `richcheap::analyse` (gives an `Analysis`), `recon::build_rows`, `richcheap::cached_quotes`.
- The rate closure `options_cli::rate_fn` uses `Rates::live`, which can download the Treasury curve. Pass a closure that reads `rates/treasury_nominal_2026.csv`, or a flat rate.
- Print only, need a data return: `options_cli::run` (`check` prints through `option_arb::render`), `recon::run` and `richcheap::run_rich_cheap` (print and write a CSV), `futures::check::run` (returns `CheckResult`, so it is usable, but it prints in the command).
- Download: `futures::check::check_vx`, `check_zq` and `run` (Cboe history, Yahoo, NY Fed; a 403 stops the run, O9); `fa recon bonds` (FedInvest, QDF defect D5); `richcheap::run_rich_cheap` (`forwards::load` can fetch the GSW file when older than 12 h); `fa options get/snapshot`.
- A runner must read the dated file from the store, never fetch. The `compare_vx` function takes a closure for the history, so it can read the cache.

## (c) Data gaps
- No past bid and ask: Alpaca history has closes only. A break on a past day cannot be tested at executable prices. Fix: save the daily chain copies (start 2026-11-23); the store has one day of SPX now.
- Indicative Alpaca quotes are not NBBO, and Cboe delayed files have no quote time. Fix: a same-minute NBBO source (paid, owner decides) or Cboe files with a stamp.
- No chain for VIX, XSP, SPY, QQQ. Fix: add them to the daily copy (the first two to be able to test VIXOPT-FWD and IDX-SPY).
- No SOFR, no SR3, no ZQ other than the 2026-11 Yahoo bar. Fix: NY Fed SOFR file in the store; a CME settlement source (paid).
- No dividend forecast for SPY, QQQ, IBM and SPX, so S-based and carry relations (OPT-AMER, FUT-CARRY) are open. Fix: a dividend source or a forward from parity as the reference.
- No ES, NQ or Treasury futures in the store (CFTC positions only). FUT-CARRY stays open.
- VIX close is a daily value (`vix/VIX_History.csv`). A one-stamp chain compares to it with a time gap. Fix: `trading/_VIX_5m.csv` gives a nearer stamp (Yahoo bars, not the official print).
- One day of each chain and one year of Treasury prices (58 days): a break rate is not a rate until many days exist.
