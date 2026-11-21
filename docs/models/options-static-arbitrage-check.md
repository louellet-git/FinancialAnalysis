# Model: Static arbitrage check of an option chain

Updated: 2026-11-21. Commit: not yet made.

## Purpose
WARNING: a break is a measurement of one stored file. It is not a tradable price and not a profit.
On Alpaca indicative quotes (`quote_kind: indicative`) the quotes are modified, so a break is not a finding.

The check tests one stored option chain against the static bounds of no-arbitrage, at executable prices
(buy at the ask, sell at the bid). It also gives the Black-76 implied volatility and the parity forward that
the check, the option history and the volatility fit use. It answers: which quote pairs break a bound, and how large is each break?

## Inputs
| Input | Source | Note |
|---|---|---|
| Chain | `data/raw/<T>/options/<D>.json` or `<D>T<hhmm>.json` (`OptionChainSnapshot`) | The last file of the date. `--time HHMM` (UTC) takes one file. |
| Spot | `spot_price` of the file | A last price, not a bid or ask. Cboe file: no time of the quotes. |
| Rate | `--rate R` (flat), else the Treasury par curve of the date | Linear in time, flat outside the tenors (`options_cli.rs`). Without `--rate` one file can download. |
| Time | Calendar days to the expiry, divided by 365 | No clock time (`option_arb.rs`). |

## Method (short)
1. Group the quotes in series: one root and one expiry (`occ_root`, `option_history.rs`). Count each dropped quote by reason (`Dropped`).
2. Use a quote when ask >= bid > 0. A quote with bid 0 and ask above 0 is a buy leg only (`option_arb.rs`).
3. With `--otm` (default for a European index) keep only calls with K >= 0.99 F and puts with K <= 1.01 F.
4. American options only: remove a quote with an ask below the intrinsic value, or a bid above S (call) or K (put). They leave the other checks.
5. For each series and type, test monotony and slope (all strike pairs) and butterfly (consecutive triples).
6. American options only: test the calendar rule (same strike, all expiry pairs) and the parity upper bound `bid C - ask P > S - K df`.
7. Mark a quote when its Black-76 implied volatility of the mid is far from the median of up to 2 neighbours on each side. A mark is a flag; the quote stays in the checks.
8. Drop breaks below `--min-size`, sort by size, count in four size bands, and show breaks with and without a marked leg.
9. `--compare D2` lists the breaks of day D that stay on day D2 (same check, same legs).
10. Implied volatility: bisection on the Black-76 price between 1e-4 and 5.0, to 1e-12 (`option_iv.rs`).
11. Parity forward: median of `K + (C - P) / df` over the 3 strikes nearest to the spot (`option_iv.rs`). Without a spot, the strike with the least `|C - P|` is the guess (`option_iv.rs`).

## Outputs
A table for each series (root, expiry): cells breaks/tests for each check, distinct quotes in breaks, marked quotes,
clean breaks, largest break in USD for one share, the rate, and the implied dividend `q = r - ln(F/S)/t`.
Then the breaks in size bands, the largest breaks with no marked leg, with a marked leg, and the intrinsic and bound breaks.
Example: `fa options check IBM --date 2026-11-19 --rate 0.04` (Cboe file, 2026-11-19): 470 breaks, 61 with no marked leg, none above 10 USD among those.

## Assumptions
Each choice has one record in `docs/assumptions/`.

| Record | Name | Value | One line |
|---|---|---|---|
| A-120 | European index list | 12 symbols | They get no calendar, parity, intrinsic or bound test. |
| A-121 | Strike filter | 0.99 F and 1.01 F | Default only for a European index. |
| A-122 | Marker threshold | 10 vol points, 2 + 2 neighbours | Flags a quote; does not remove it. |
| A-123 | Tick and size bands | 0.01 below 3 USD, else 0.05; 0.10, 1, 10 | Display only. |
| A-124 | Spot and parity bound | Spot is a last price; no dividend | S drives the intrinsic, bound and parity checks. |
| A-125 | Time and rate | Days / 365; par curve | Sets the discount factor. |
| A-126 | Parity forward | 3 strikes, median, 0.5 S to 2 S | Differs from the 5 strikes of the Heston fit (A-134). |
| A-127 | Black-76 volatility | European model on American options | Used for the marker and the history. |
| A-137 | Bounds of the checks | Slope with no discount; EPS 1e-9 | Sets which pairs break. |

## Register and runner
The relations of this check are in the register `arbitrage/register/` (one file for each relation, form in its README). Seven are `active`
(OPT-MONOTONY, OPT-SLOPE, OPT-BUTTERFLY, OPT-CALENDAR, OPT-PARITY-UPPER, OPT-INTRINSIC, OPT-BOUND: one for each check of `CHECK_NAMES`);
three are `planned` (OPT-PARITY-STRIKE, OPT-BOX-RATE, VIX-REPLICATION). A test reads the folder: each active file names a check id
of `option_arb::CHECK_IDS`, and each id has an active file.

Command: `fa arb run --symbol T --date D [--time HHMM] [--rate R] [--out PATH]` (`market/arb_run.rs`). It reads one stored snapshot,
calls `option_arb::check_chain` with the defaults of `fa options check` (no size floor, strike filter for a European index, no marker), and prints
one table: id, class, status, tested, pass, break, not tested, and the dropped legs by reason. It writes the same as JSON (default
`data/processed/arb/<D>/<T>-<HHMM or close>.json`). Exit 0 also with breaks; a usage error exits 64.
- Cache only. The rate is `--rate R` or the Treasury par curve file `data/raw/rates/treasury_nominal_<year>.csv` (read only). If the file is missing
  the command says so and stops. It never downloads (a test reads the code for the live rate source).
- Quote gate: a quote with no ask, bid above ask, or no bid and no ask above 0 is dropped and counted by reason. A bid of 0 with an ask above 0 is kept as a
  buy leg only. A relation whose sold leg is buy-only is "not tested", not "pass" (`SeriesRow::not_tested`). A quote with bid = ask is counted
  (`Report::locked`) and still used. A dropped quote has no relation, so it is counted as a dropped leg, not as a relation.
- Status: `break` (one break or more), `pass` (tested, no break), `not tested` (no test could run), `n/a European` (the check is for American options).
- Not used yet: the field `for` (2 snapshots). The checks run on one file.

## Limits
- Limits of use: a break is not a profit. No costs, no borrow, no time of the quotes, no early exercise (`option_arb.rs`).
- Limits of use: on Alpaca indicative quotes a break is a measurement, not a tradable price (the report prints this; Alpaca documentation, https://docs.alpaca.markets/docs/historical-option-data, read 2026-11-21).
- A Cboe delayed file has no time of the quotes. Quotes of one file are not at one time.
- Monotony and slope use all strike pairs, so one bad quote makes many breaks.
- The VIX checks are untested: VIX options settle on a VIX future, not on the index (`option_arb.rs`).
- Black-76 is a European model. The volatility of a deep in-the-money American quote is not right.
- The implied dividend is an approximation: early exercise and borrow are inside it. For a 1-day series it is not usable (IBM 2026-11-20 series: -55%).
- Backtest: none applies, because the command is a check of one chain. It is not a forecast.
- A fixed-answer test plants each break. No test shows that a break on real quotes is tradable.

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/market/option_arb.rs`, `options_cli.rs` (`check`), `option_iv.rs`, `option_history.rs` |
| Command | `fa options check SYMBOL --date D [--time HHMM] [--rate R] [--top N] [--min-size USD] [--iv-mark POINTS] [--compare D2] [--otm \| --all-strikes]` |
| Tests | `cargo test --manifest-path rust/Cargo.toml --lib option_` |
| Findings | `trading/research/arb-01-chain-check-2026-11-19.md` |

## Review status
Review of 2026-11-20: PASS-WITH-NOTES. See the tickets `T-20261120-arb-1-chain-check` and `T-20261120-arb-1b-chain-check-depth`.
The records of this document are not reviewed.
