# Hidden assumptions and landmines — the trap registry

Non-obvious assumptions an agent can trip on. Each entry carries an evidence pointer;
every entry below was re-verified against the code/docs on 2026-07-17. If you find a new
one, add it here with a pointer.

## Data layer

**1. Recast-financials landmine (IBM FY2019–20).** The store mixes continuing-ops
(recast) revenue with unrecast consolidated OCF across the Kyndryl divestiture boundary,
so the FY2020 "28.2% FCF margin" is spurious and **margin windows longer than 5 years
are unusable** for IBM until the importer aligns recasts. The backtest surfaces (never
fixes) this via `DATA_FLAGS`; every realized IBM vintage window touches a flagged year.
Evidence: `analysis/IBM/README.md` §Data notes; `financial_analysis/backtest.py:62-69`
(`DATA_FLAGS`); `analysis/IBM/REVIEW.md` F13; commit `8153323` (IBM backtest bias driven
by the recast).

**2. Phantom "10-K" rows.** The importer keys periods off *any* mapped concept's period
end, so stray XBRL facts create annual rows with 1–2 fields and no revenue/OCF (IBM:
2012-06-30, 2018-01-01, 2019-01-01 — the latter two also have `filed_date` equal to
`period_end`, the importer's placeholder). They carry no FCF and never enter the DCF
engines, but any code iterating `company.periods` naively will hit them.
`backtest._clean_annuals` filters them by requiring revenue AND OCF. Evidence:
`financial_analysis/importer.py:149-160` (row creation), `:154` (filed_date
placeholder); `financial_analysis/backtest.py:76-94`; `tests/test_backtest.py:77`;
`analysis/IBM/README.md:91`.

**3. `n_periods=8` is per-concept, not per-company.** `extract_series` returns the last
8 entries *per concept*; the union of period-ends across ~60 concepts yields far more
than 8 periods (IBM: 22 annual rows), with older rows only partially populated (e.g.
IBM buybacks populated FY2012–19 while revenue is None before FY2018). Do not treat
older rows as complete statements. Evidence: `financial_analysis/importer.py:120,146`;
`financial_analysis/xbrl_api.py:74` (`[-n:]`).

**4. Filed-date visibility lag (point-in-time honesty).** For companyfacts-sourced
data, `filed_date` is the date of the filing the figure was *drawn from* — usually a
LATER 10-K's comparative column, because deduplication keeps the latest-filed entry per
end date (IBM FY2019 carries filed 2022-02-22; FY2023 carries 2026-02-24). The backtest
filters vintages on `filed_date <= as_of`, so vintages are **conservative** (see fewer
periods than a real-time observer), never optimistic — the safe leakage direction, but
do not read `filed_date` as "when this became public." Evidence:
`financial_analysis/backtest.py:1-37` (module docstring, caveat #1);
`financial_analysis/xbrl_api.py:25-32` (`_deduplicate`);
`financial_analysis/importer.py:97-117`.

**5. Segment data is hand-entered; companyfacts has no segment axis.** The SEC
companyfacts API strips dimensional facts (verified across every us-gaap concept in
IBM's payload: entries carry exactly `{accn, end, filed, form, fp, frame, fy, start,
val}`), so `PeriodFinancials.segment_revenue` is always None from that source. IBM's
per-segment revenue in `analysis/IBM/driver_model.py:24-29` (`SEGMENT_HISTORY`) is
hand-entered from the Jan 28, 2026 Q4 8-K ("Other" $63M dropped, 0.1% of revenue).
Filing-level instance-document extraction is Ticket 7. Evidence:
`financial_analysis/importer.py:1-14`; `PLANNING/tickets.md` Ticket 7;
`financial_analysis/schema.py:152-156`.

**6. The JSON store holds statements only — no prices, options, or estimates.**
`store.save()` serializes `periods` exclusively; market data attached by
`scripts/analyze.py` lives only in the xlsx and in dated snapshots under
`data/raw/TICKER/` (options/, estimates/, prices.json). IBM's JSON `prices` array is
empty; the report's price/dividend/analyst-target figures were **hand-entered** from
cited sources, not pipeline outputs (disclosed). Evidence:
`financial_analysis/store.py:38-44`; `analysis/IBM/README.md:93`;
`analysis/IBM/report/SOURCE-AUDIT.md` row 43.

**7. Capex sign convention.** The importer stores capex sign-flipped (negative);
`free_cash_flow = OCF − abs(capex)`. A 2026-06-12 bug overstated FCF by 2×capex before
the abs() fix — do not "simplify" it away. Evidence: `financial_analysis/schema.py:161-167`;
`financial_analysis/xbrl_tags.py:106-109` (`sign_flip=True`);
`PLANNING/tickets-closed.md` Ticket 5 results.

**8. `dividends_paid` third fallback is dividends DECLARED, not paid.**
`us-gaap/DividendsCommonStockCash` is an equity charge used as a last-resort proxy when
neither payments concept exists for a period (first-wins merging). Fine for steady
payers; wrong for irregular ones. Evidence: `financial_analysis/xbrl_tags.py:128-138`
(inline precedence comment).

**9. yfinance surfaces are current-only; history accumulates from first pull.** Option
chains and forward consensus estimates have no free historical endpoint — each pull is
persisted as a dated snapshot (one per calendar day; same-day re-run reads disk,
`--refresh` overwrites). If you never ran a ticker, there is no history to load.
Evidence: `financial_analysis/options.py:1-8`, `financial_analysis/estimates.py:1-14`
(module docstrings).

## Model layer (IBM v4 / driver engine)

**10. Buybacks are genuinely absent from the model — not an omission bug.** IBM's
share count is held FLAT (948,675,228 diluted) because SBC is charged as a full cash
cost — also growing the count would double-charge dilution (REVIEW F5). The store's
null `repurchases_common_stock` for FY2020+ mirrors reality (IBM's buybacks stopped;
the mapping itself works — FY2012–19 values are populated) and the source audit
verified the nulls. Capital-return arithmetic in the analysis treats dividends + M&A as
the (debt-funded) return channel. Evidence: `analysis/IBM/driver_model.py:32`
(`SHARES ... held flat (SBC charged in margin)`); `analysis/IBM/REVIEW.md` F5, F13;
`analysis/IBM/report/IBM-valuation-2026-07.md:155`; `SOURCE-AUDIT.md` row 43.

**11. f-conditionality: every headline number is conditional on f = 0.50.** The M&A
recurrence judgment `f` charges f × M&A spend in the margin AND credits f × acquired
growth in `g_software`'s mean — a paired judgment with a single source of truth
(`MA_RECURRENCE`, guarded by a module-level assert so the two legs cannot drift apart).
Changing one leg without the other silently breaks pairing coherence. The f-locus
(0→1, growth credit moving WITH f) is published by `dcf_drivers_run.py`; f is a
judgment, not a fitted parameter. Evidence: `analysis/IBM/driver_model.py:36-43,
161-171, 240-244`; `analysis/IBM/DRIVER-REVIEW.md` ma_recurrence row.

**12. "Tails are decorative."** Each path holds one growth draw for all ten years (no
mean reversion — ticket T-DCF-1, the largest known distortion), so the left tail is
understated even after the v4 correlation matrix fattened it. Standing language: "P50s
are robust; P10s are optimistic. No margin-of-safety claim is made from these tails."
Do not quote P10/P90 as decision-grade. Evidence:
`analysis/IBM/report/IBM-valuation-2026-07.md:99` (tail-caveat callout);
`analysis/IBM/REVIEW.md:54` (T-DCF-1: "Until then, P10/P90 are decorative");
`financial_analysis/driver_dcf.py:43-45` (disclosed limitation).

**13. The scenarios registry still carries IBM v3 values while the engine moved to
v4 — verified still true on 2026-07-17, and deliberate.**
`financial_analysis/scenarios.py:51-78` (`_ibm_v3`, `_SCENARIO_OVERRIDES["IBM"]`,
`CAPITAL_OVERRIDES["IBM"]`) encodes the v3 aggregate framework (base P50 $141 family:
g 2.15%, WACC 7.5%, margin 14.5%). The current v4 headline (P50 $151.84, conditional on
f=0.50) exists ONLY in `analysis/IBM/driver_model.py` + `driver_dcf.py`. The visualizer
runs both, in separate labeled sections ("Aggregate DCF scenarios" vs "Driver-based
model"). Net debt ($54.0B) and shares (948.7M) agree across both. If you read
`scenarios.py` for "the IBM valuation," you get v3. Evidence: files cited;
`scripts/visualize_dcf.py:72-78` (`SECTION_TITLES`), `:837` (`_driver_compare_figure`).

**14. Truncation is clip-after-correlation.** `truncnorm` drivers are clipped to
[lo, hi] AFTER the correlated transform — not a true truncated normal: mass piles on the
bounds and realized correlations distort. Second-order for mild truncations; check no
bound sits within ~2σ of its mean (g_infrastructure's hi=+3% sits at 1.6σ — the bound
mass is disclosed in DRIVERS.md). Evidence: `financial_analysis/driver_dcf.py:20-30`
(module docstring); `analysis/IBM/DRIVERS.md` driver table.

**15. The two engines guard degenerate terminal values differently.** Classic engine:
`np.clip(wacc − tg, 0.001, None)` — near-degenerate paths CAN dominate its tails (F12).
Driver engine: joint guard `tg_eff = min(tg, wacc − 0.02)` with the pinned fraction
reported. Do not assume classic-engine tail output has the driver engine's protections.
Evidence: `financial_analysis/dcf.py:138`; `financial_analysis/driver_dcf.py:405-431`;
`analysis/IBM/REVIEW.md` F12/T-DCF-2.

**16. Classic-engine minimum-history guard.** `StochasticDCF` raises unless ≥ 4 annual
periods have positive FCF (blocks post-IPO/loss-makers); the backtest requires ≥ 3
usable periods per vintage and records skips. Evidence:
`financial_analysis/dcf.py:10,77-88`; `financial_analysis/backtest.py:52,150-155`.

## Pipeline/repo layer

**17. The xlsx always has nine sheets; the console tab count lies by omission.**
`write_xlsx` creates every sheet regardless of flags; `--no-market` etc. just leave
header-only sheets, while the "Done: ... across N tabs" message counts populated groups.
Evidence: `scripts/analyze.py:70-81, 279-284, 370-379`.

**18. (Closed 2026-11-19.) Generated files were tracked.** Two xlsx files and four
.pyc files were in git although `.gitignore` excludes them. They are removed from the
index. `data/` and `__pycache__/` are not in git; do not add them.

**19. RPO rows hide on the Balance Sheet tab.** `remaining_performance_obligation` is a
disclosure fact but is written to the Balance Sheet tab with `statement="disclosure"`
(no dedicated tab). Its current/noncurrent duration split is text-only in filings and
is not extracted. Evidence: `scripts/analyze.py:51-58` (`_TAB_FOR`);
`financial_analysis/xbrl_tags.py:147-155`.

**20. Default scenario presets are generic mature-large-cap.** `DEFAULT_SCENARIOS`
(bear 1%/base 4%/bull 6% growth) applies to ANY ticker without an override — running
`visualize_dcf` on a hypergrowth name silently produces mature-large-cap scenario
labels and assumptions. Net-debt derivation also reads short-term borrowings from
`raw_extras` keys and treats absence as 0. Evidence:
`financial_analysis/scenarios.py:45-49, 92-125`.

## Carried from the parallel registry (PR #11, verified there against code/data 2026-07-17)

Two documentation agents ran the same brief in parallel on 2026-07-17 (see CLAUDE.md
orchestration norms); these entries existed only in the other agent's registry and are
carried over with their original evidence pointers.

**21. `filed_date` can be outright fabricated (placeholder = period_end).** The importer
initializes `filed_date` to the period end and only overwrites it where a *revenue*
concept fact matched that (end, form). Phantom rows and sparse pre-2018 rows keep the
placeholder — e.g. IBM's 2012-06-30 row claims it was "filed" 2012-06-30. Never trust
`filed_date` on a row without revenue.
*Evidence:* `importer.py` (`"filed_date": end,  # placeholder`) and
`_populate_filed_dates` (revenue concepts only); IBM.json phantom rows show
filed_date == period_end.

**22. The correlation matrix is reviewed *judgment*, and it moves tails only.** All 28
entries (including the 3 deliberate zeros) are argued assumptions (rationales in
DRIVERS.md / driver_model.py), PSD-verified (min eigenvalue 0.419). The supply-shock
regime flip (rf·g = −0.15) leaves P50 essentially unchanged ($151.89 vs $151.84) and
widens tails. Do not treat correlation entries as estimated from data — none are.
*Evidence:* `driver_model.py` CORR; dcf_drivers_run.py disclosure runs; SOURCE-AUDIT
claim 32.

**23. RPO was consumed as evidence WITH AN EXPIRY — it lapses at the Q2-2026 10-Q.**
The $71.0B RPO facts corroborate the growth means (they changed no parameter — all four
proposed changes were rejected in the round-3 re-look) and expire when the Q2-2026 10-Q
prints (late July 2026). After that, the rationale footnotes in driver_model.py and the
ledger rows are stale until re-consumed. The +12.7% YoY print consolidates HashiCorp and
is NOT organic evidence.
*Evidence:* `analysis/IBM/research-ledger.md`; DRIVER-REVIEW.md RPO re-look; rule R1 in
PLANNING/RULES.md.

**24. The published v3 numbers no longer reproduce from the current store (store drift).**
`analysis/IBM/dcf_scenarios.py` derives FCFF margins live from `IBM.json`; the
p1-importer re-import shifted the derived margins (5y mean 17.8% → 18.4%), so a fresh run
prints Bear $43 / Base $149 / Bull $243 against the signed-off record of
**$37 / $141 / $232**. The record (README/REVIEW.md) is frozen history; the script output
is live. Do not "correct" either to match the other — cite the record for v3, the engine
for anything current. (The v4 driver run does NOT drift: its inputs are pinned constants.)
*Evidence:* run of `python -m analysis.IBM.dcf_scenarios` on 2026-07-17 vs
analysis/IBM/README.md; `dcf_scenarios.py` `fcff_v3_margins` reads the store.

**25. Default capital-structure derivation ≠ the reviewed IBM bridge.** For tickers
without a `CAPITAL_OVERRIDES` entry, net debt = long-term debt + short-term borrowings −
cash — **no pension, no marketable securities, no stock-vs-flow review**. The IBM
override embeds the F15-reviewed bridge (debt + after-tax pension − cash&securities);
the DUOL override includes HTM securities + LT investments on the cash side. A
cross-ticker comparison (the ORCL/basket runs) that mixes overridden and derived capital
structures compares different net-debt definitions.
*Evidence:* `scenarios.py` `capital_structure_for` vs `CAPITAL_OVERRIDES` entries +
analysis/IBM/README.md bridge table.

**26. officecli mutates a .docx on open.** Reviews and audits must run against a COPY
(`cp doc /tmp/review.docx`); only deliberate edits touch the master.
*Evidence:* the memo-reviewer agent definition (private framework; not in this repo
since 2026-11-19).

**27. (Closed 2026-11-19.) No hooks are in this repo.** The .docx review hook and the
officecli setup hook were part of a private framework and are removed. The review
steps in `CLAUDE.md` are a manual duty.

**28. Timed/background tasks in this environment can be killed mid-run — persist resume
state in files.** A documentation agent died at the reading stage to a session token
limit on 2026-07-17. Write open work-state to the board ticket (`PLANNING/board/`) as
you go. Interrupted fan-outs must resume, never
restart (vision ✦6b).
*Evidence:* git history of `PLANNING/HANDOFF.md` (deleted 2026-11-19); reusable-dcf-vision.md ✦6b.

**29. Branch fragmentation — "done" work may not be on your branch.** Two live examples
from the day this file was written: (a) the backtest harness sat complete-with-tests on
`factory-builds` while `handoff` (then checked out) lacked it — `python -m
scripts.backtest` failed and a doc claiming "backtest ✓" looked wrong until PRs #7/#8
merged both to main; (b) PR #10 (DUOL scenarios) was based on `viz-final-pass` and merged
into that branch *after* #9 had landed, stranding the commit off main until PR #14
cherry-picked it back. Lessons: check `git log --all --oneline -- <path>` before
concluding something doesn't exist; base PRs on `main` unless deliberately stacking (and
retarget after the base merges); never switch branches in a shared tree while agents are
working — use the `wt-*` worktrees for parallel work.
*Evidence:* git history around `8153323`/`9751523` and `c7dfb95`/`a082339` (2026-07-17).

**30. LF/CRLF warnings are normal; PowerShell 5.1 quirks are real.** `core.autocrlf=true`
on this checkout — "LF will be replaced by CRLF" warnings are expected noise, not damage.
The primary shell is Windows PowerShell 5.1: no `&&`/`||` chaining, no ternary, UTF-16
default file encoding (`-Encoding utf8` when writing files other tools read), quirky
native-arg quoting (`--%` stop-parsing token when needed). Git Bash is available for
POSIX scripts.
*Evidence:* `git config core.autocrlf`; environment.

**31. Tests are no-network by construction — keep them that way.** The whole suite runs
on synthetic fixtures (mocked HTTP, synthetic yfinance/consensus/company objects), fixed
seeds (7) everywhere randomness exists: `SCENARIO_SEED = 7`, dcf_drivers_run `SEED = 7`,
backtest default `seed=7`. A test that needs the network or a floating seed is a defect.
Run `pytest -q` for the current count rather than trusting any doc.
*Evidence:* `pytest -q` runs of 2026-07-17; grep "no network" in tests/.

## Rates and inflation (added 2026-11-18)

**32. The `rf` of a driver model is a fixed number from the model date.** IBM uses 4.6%
(July 2026). JNJ uses 5.10% (^TNX, 2026-10-16). The Treasury 10Y was 5.07% on
2026-11-18. To use the rate of a date, run `scripts.verdict` with `--rf-date`. Do not
change `rf` in the IBM model: it is part of the reviewed version 4.
*Evidence:* `analysis/IBM/driver_model.py` (`rf` driver), `analysis/JNJ/driver_model.py`;
`python -m financial_analysis.rates yield --tenor 10Y`.

**33. `--rf-date` changes only `rf`.** The growth drivers and terminal growth are nominal
and do not change. If expected inflation causes a rate change, nominal growth usually
changes too, and a change of `rf` alone gives too low a value. Use `--rf-base-date` to
split the change. From 2026-07-16 to 2026-11-18 the 10Y went up 0.50 pp: the real rate
went up 0.53 pp and breakeven inflation went down 0.03 pp. Thus, for IBM, a change of
`rf` alone was correct.
*Evidence:* `python -m scripts.verdict IBM --price 231.09 --rf-date 2026-11-18 --rf-base-date 2026-07-16`.

**34. Breakeven inflation is not pure expected inflation.** Nominal minus TIPS yield
contains an inflation risk premium and a TIPS liquidity premium. Use it as an
approximation. The 5y5y forward rate (years 6 to 10) is the better reference for
terminal growth, because it excludes near-term changes.
*Evidence:* `financial_analysis/rates.py` module docstring.

**35. CPI has a gap and a publication delay.** BLS has no CPI for October 2025 (2025
lapse in appropriations). Thus no 12-month change exists for October 2026, and
`cpi_inflation` gives September 2026 with a note. The date of a CPI value is the
reference month. BLS publishes it approximately two weeks after the month. For a past
date, `cpi_inflation` can use a value that was not yet public on that date. Do not use
it in a point-in-time backtest without a release-date check.
*Evidence:* BLS series CUUR0000SA0, 2025 M10 value `-`; `tests/test_rates.py`.

## Could not verify

- (none of the named traps failed verification — all located in current code/docs; the
  scenarios-registry item was re-checked and is still true as stated in #13.)
- Entries 21–31 were verified by the parallel PR #11 agent on the same day, not
  independently re-verified by this registry's author; their evidence pointers stand.
