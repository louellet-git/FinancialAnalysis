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

**2. (Closed 2026-11-19.) Phantom "10-K" rows.** Before this date, half of the annual
rows were not fiscal years (on a 29-ticker panel: 338 of 672). A 10-K also contains
quarter values, and the old importer made a row for each end date. Now a row exists
only if an anchor tag (revenue, net income, operating cash flow, tax expense) has a
fact of the correct length for that end date. Evidence: `rust/mapcheck/src/facts.rs`
(`period_ends` and its tests).

**3. (Closed 2026-11-19.) `n_periods` counts fiscal years.** `import_company(n_periods=12)`
gives the latest 12 fiscal years and the latest 12 quarters. Each row has all the
variables that the filer reports for that period. Evidence:
`financial_analysis/importer.py` (`DEFAULT_N_PERIODS`).

**4. Filed-date visibility lag (point-in-time honesty).** For companyfacts-sourced
data, `filed_date` is the date of the filing the figure was *drawn from* — usually a
LATER 10-K's comparative column, because deduplication keeps the latest-filed entry per
end date (IBM FY2019 carries filed 2022-02-22; FY2023 carries 2026-02-24). The backtest
filters vintages on `filed_date <= as_of`, so vintages are **conservative** (see fewer
periods than a real-time observer), never optimistic — the safe leakage direction, but
do not read `filed_date` as "when this became public." Evidence:
`financial_analysis/backtest.py:1-37` (module docstring, caveat #1);
`rust/mapcheck/src/facts.rs` (`latest`: the latest filing wins);
`rust/mapcheck/src/mapper.rs` (`resolve_period`).

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

**36. One tag name can have a different scope for a different filer.** The registry
(`financial_analysis/xbrl_tags.py`) puts tags in one alias list only if their scope is
the same, but filers do not all obey the tag definitions. Known cases: the
`...AndCapitalLeaseObligations` debt tags include finance leases for some filers (HD)
and not for others (IBM); `CostOfGoodsAndServicesSold` for CVX is only purchased crude
and products, thus the `gross_profit` formula result for CVX is not a real gross profit;
`Revenues` for CVX includes other income; the tag
`StockholdersEquityIncludingPortionAttributableToNoncontrollingInterest` of Agilent (A)
has a value that is not total equity. Read the `sources` of a value before you use
it in a model. A value whose source starts with `=` came from a formula.

**37. A new registrant has no history under its ticker.** The ticker XOM now points to
ExxonMobil Holdings Corporation (CIK 2115436), which has only 10-Q facts; the history is
under the old CIK. `import_company("XOM")` gives no annual rows. No tool joins the two.

**38. Company-specific tags and segment data are not in companyfacts.** A filer that
uses its own tag for a line (CAT and NEE for interest expense, NEE for capital
expenditure) has no value for that variable, or a fallback (`interest_expense` uses
`= interest_paid`). Filing-level XBRL is necessary (PLANNING/tickets.md, Ticket 7).

**39. Many balance-sheet variables are a detail inside a different variable.** The
source does not say which lines are parts of which line. On 312 US companies
(2026-11-19), 166 have one or more variables that are inside a different line of the
same filer: `other_accrued_liabilities` (76 companies), `accrued_compensation` (62),
`operating_lease_liabilities_current` (54), `income_taxes_payable` (51). If you add
such a variable to the line that contains it, you count the amount two times. Do not
build a sum of balance-sheet variables for a ticker before you read
`mapcheck TICKER --detail` (findings `nested` and `overlap`). Known overlap of separate
lines: TGT (short-term investments are inside cash), CVX (`debt_current` is before a
reclassification to long-term debt).
*Evidence:* `rust/mapcheck/README.md`; `rust/mapcheck/src/checks.rs`.

**40. `total_debt` has no value for a company with no debt tags.** The mapping does not
change a missing tag to zero. On 343 companies, 10% have no `total_debt`; most have no
debt (PLTR, ISRG, ANET), but some have debt under tags that are not in the registry.
`capital_structure_for` then raises an error and a `CAPITAL_OVERRIDES` entry is
necessary.

**41. Foreign filers and IFRS filers are out of scope.** A foreign filer with annual
reports on form 20-F or 40-F has no 10-K (51 of the 500 largest registrants), and an
IFRS filer has no us-gaap facts (59 of 500). `mapcheck` gives them `N/A` and `ERROR`.
A filer that reports in a currency that is not USD (IMO, Canadian dollars) is `N/A`.

**42. Three core inputs are weak in the standard variables.** A comparison with Yahoo
Finance for 50 companies (2026-11-19) gives agreement of 98% or more for eight core
inputs, but 75% for `capex`, 74% for `da_total` and 83% for `total_debt`. Causes: the
standard capex is equipment only (capitalized software and company tags are not in it);
the depreciation tags have a different scope for each filer; debt tags overlap. Before
you use these three inputs for a ticker, run `mapcheck filing TICKER --focus` and
compare. Free cash flow is too high when capex is too low. Since rule R2
(`PLANNING/RULES.md`), free cash flow uses `capex_total` (equipment and capitalized
software). This corrects only the companies that use a standard tag for software (14 of
324); a company tag for software (IBM) is not in the standard source.
*Evidence:* `rust/mapcheck/README.md` (the record of the Yahoo comparison, and
`mapcheck crosscheck`).

**43. The filing-level values come from tag-name patterns.** `rust/mapcheck/src/filing.rs`
finds the capex, depreciation and debt lines by words in the tag name. A line can be
missing (a capex line with an unusual name) or incorrect
(FDX and DUK: debt is counted two times; V and MA: a line that is not depreciation).
The printed lines show what the value contains.

**44. A share count can have no scale.** McDonald's tags 716.4 for 716.4 million shares.
The importer and the Rust mapper correct a factor of 1,000 or 1,000,000 when net income
divided by EPS shows it, and the source records the correction. A stock split after the
period is not corrected: the value is the count that the filer reported.

**48. The Python commands need the Rust program.** The Python importer has no mapping
logic. `scripts.analyze`, `scripts.import_financials` and each command that imports a
ticker run `rust/mapcheck/target/release/mapcheck`. If the program is not built, they
stop with `MapcheckError` and the build command. Build it again after a change of
`financial_analysis/standard_variables.toml` (the program contains the file).

**49. A request to the SEC needs `SEC_USER_AGENT`.** The Rust program reads the contact
text from this environment variable. `www.sec.gov` (ticker list, filing files) refuses
a request with no contact. The Python importer gives the Rust program the contact text
of the legacy Python SEC client when the variable is not set.

**50. A critical input can come from the 10-K, and it is an estimate.** When revenue,
capex or the share count has no value in the company facts, and the 10-K is in the
cache, `mapcheck` reads the value from the filing (`rust/mapcheck/src/supplement.rs`;
rules in `rust/mapcheck/README.md`, "Values from the 10-K"). 24 of the S&P 500 members
and 37 of the next 500 companies get a value in this way. The source text starts with
`filing`, and the finding `filled_from_filing` is a warning. Read the source before you
use the value: a capex value is a sum of lines that are found by words in the tag name
or the label (NEE: four lines, 25B; URI: rental equipment and other equipment).
If the 10-K is not in the cache, the input has no value and the status is `FAIL`; run
`mapcheck fetch --filings --gaps`.

**51. Capex is 0 when the 10-K has no capex line.** Some companies do not show capital
expenditure in the cash-flow statement (ABNB, APP, COIN, TPL, royalty companies, asset
managers). The amount is small for them and it is inside a different investing line.
`mapcheck` then gives `capex` = 0 with a source text that says so (5 of the S&P 500,
9 of the next 500). Free cash flow is then equal to operating cash flow, which is some
percent too high. The purchase of royalty interests or of mineral rights (RGLD, VNOM,
TPL) is not capex under rule R2; for these companies it is the main investment, and a
valuation must treat it separately.

**52. The share count of a company with share classes is a selection.** The 10-K gives
a count for each class. `mapcheck` uses the largest class (V, BRK-B, ERIE: the count of
this class includes the other classes after conversion), or the sum of the classes when
the sum agrees with net income divided by EPS. For a company with an operating
partnership (HESM, PAGP) or a tracking stock (FWONA: no value), examine the count.

**53. A 10-K gives three fiscal years.** A company that gets capex from the 10-K has
capex for the three years of the latest 10-K only. Earlier years have no value, and
the finding `short_history` says that the classic engine does not have its four years.
An oil company: `capex` is the payments to explore and develop; the purchase of
properties from other companies is not in it (FANG: 5.9B in 2025).

**54. Revenue has four sources, in a fixed sequence.** (1) A total tag (`Revenues`, the
utility tag, the tag for revenue net of interest expense). (2) A bank: net interest
income and noninterest income. (3) Contract revenue. (4) Lease income (a landlord),
then total investment income (an investment company). A bank uses the contract-revenue
tag only for a part of its fee income; before this rule, MTB had 1.5B of revenue, not
9.3B. A mortgage REIT (NLY, AGNC) has no revenue total and stays `FAIL`. The filing
date of a period comes from the revenue fact or the contract-revenue fact.

**55. A formula value that is larger than its total is not used.** If a formula gives
a value that is more than 2% larger than a total that must contain it (the `[[within]]`
rules of the registry), two terms contain the same amount. The variable then has no
value, the source text says why, and the finding is `formula_not_used` (ASTS: the
short-term investments are inside the cash line; INCY 2018 and VMRK 2022: total debt).
A value from a tag is not removed: it stays, with the finding `exceeds_total` (`FAIL`).

**56. `mapcheck` judges a bank, an insurer and a REIT, but the DCF engines do not fit
them.** The status of these company types says that revenue, net income, total assets,
equity and the share count are there. It does not say that a free-cash-flow DCF gives
a correct value. A company with no revenue in the latest three fiscal years is `N/A`.

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
labels and assumptions. Evidence:
`financial_analysis/scenarios.py` (`DEFAULT_SCENARIOS`).

## Carried from the parallel registry (PR #11, verified there against code/data 2026-07-17)

Two documentation agents ran the same brief in parallel on 2026-07-17 (see CLAUDE.md
orchestration norms); these entries existed only in the other agent's registry and are
carried over with their original evidence pointers.

**21. (Closed 2026-11-19.) `filed_date` was the period end on rows with no revenue.**
Now `filed_date` is the filing date of the revenue fact; if the period has no revenue
fact, it is the latest filing date of the facts in use. Rows with no anchor fact do not
exist (#2). Trap #4 (the date is that of a later filing) stays.
*Evidence:* `financial_analysis/importer.py` (`resolve_period`);
`tests/test_importer.py::test_filed_date_is_a_filing_date_when_the_period_has_no_revenue`.

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
without a `CAPITAL_OVERRIDES` entry, net debt = the standard variable `total_debt` −
cash — **no pension, no marketable securities, no stock-vs-flow review**. `total_debt`
is a first estimate: filers tag debt in different ways and the tags can overlap (on the
2026-11-19 panel the two formulas disagree for IBM, JNJ, HD, CVX and CSCO; see
`mapcheck --detail`). The IBM
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

**57. The GSW curve is a fitted model, not a market quote.** The Federal Reserve Board
fits a Svensson curve to Treasury prices each business day. The zero-coupon yields and the
forward rates are values of this curve. The Board says the file is a staff research
product that can change or be revised without notice. Do not use a GSW value as a price
that a person can trade. The file has a delay: on 2026-11-19 the last row was 2026-11-13.
`fa forwards` gives the last row on or before the date that you ask for, and its `note`
says so.
*Evidence:* `rust/src/forwards.rs` (`forwards_on`); the first line of
`https://www.federalreserve.gov/data/yield-curve-tables/feds200628.csv`.

**58. GSW rates have different compounding conventions.** The zero-coupon yield (SVENY)
and the instantaneous forward rate (SVENF) are continuously compounded. The par yield
(SVENPY) and the one-year forward rate (SVEN1F) are coupon-equivalent. The one-year
forward rate is not `exp((s+1) z(s+1) - s z(s)) - 1` of the published zero-coupon yields:
the two differ by 3 bp to 7 bp. Use the convention of each series. Do not mix them. Only
three one-year forward rates exist (start at 1, 4 and 9 years). The par yield of the GSW
curve differs from the Treasury par yield by 0.18 pp at most (last year, 1Y); the check
`fa forwards check` uses 0.25 pp.
*Evidence:* `rust/src/forwards.rs` (`PAR_TOLERANCE`); the header of the GSW file.

**59. FRED refuses some clients in this environment, and no free futures source is in the
repo.** FRED closes the connection for a client with an unknown `User-Agent` text (a
`curl` request with a custom text failed; with `curl/8.5.0` it worked). The Treasury
site works with any text, so `fa forwards check` reads the Treasury file and not FRED. No
free source of interest-rate futures or index futures prices exists whose terms permit
automated use (the CME terms forbid data mining with scripts, software, spiders and robots).
Thus `fa` has no futures command. Ticket `T-20261119-futures-forwards-data` has the options.
*Evidence:* `rust/src/forwards.rs` (`check`); ticket text.

## Corporate bonds (added 2026-11-18)

**45. For IBM, `--rating` replaces a reviewed calibration with a proxy.** The IBM
`kd_spread` (-0.10%) comes from the fair value of the IBM debt ladder (DRIVER-REVIEW,
adjudicated conflict #2). The A index spread over the 10Y was +0.67% on 2026-11-17. With
`--rating A-` the IBM P50 goes from $133.90 to $127.71. The index is a general proxy for
the rating. It is not a correction of the review. To change the IBM value, do a review of
the same weight. For a ticker without a calibrated spread (for example JNJ), the index
gives a sourced value.
*Evidence:* `python -m scripts.verdict IBM --price 231.09 --rf-date 2026-11-18 --rating A-`.

**46. The ICE data on FRED has only 3 years, and ICE does not allow redistribution.**
The series start on 2023-11-20. A date before that gives an error, not an old value. Do
not commit the cache (`data/raw/bonds/`) or put ICE values in a public file. For a
longer history, use Moody's Aaa and Baa (from 1983 and 1986). Those are long bonds (20
years or more), thus their spread over the 10Y is larger than the spread of an index.
*Evidence:* FRED BAMLC0A3CA first observation 2023-11-20.

**47. The rating index has no notches and its own duration.** `A-`, `A` and `A+` all use
the `A` index. The index duration is approximately 6 to 7 years, not 10 years. Thus the
OAS and the yield minus the 10Y are different numbers. The engine uses the yield minus
the 10Y, because the engine defines `kd = rf_10Y + kd_spread`. The HQM curve is monthly
and comes through FRED. A check on 2026-11-18 found that FRED and the Treasury .xls file
agree (306 values, no difference). The repo has no `.xls` reader, thus this check is
not automatic.
*Evidence:* `financial_analysis/bonds.py` module docstring.

## Bond reconciliation (added 2026-11-19, branch `claude/bond-recon`)

**60. The price source is not a market quote feed.** The prices are the end-of-day prices
of the Federal Investments Program (TreasuryDirect FedInvest). The page says that they
apply to "market-based Treasury special securities". The CUSIPs are the CUSIPs of the
public bills, notes and bonds. The time of day, and whether the price is bid, ask or mid,
are not stated on the page. Our curve is built from bid-side prices at about 3:30 PM New
York time. Thus a difference can come from the price type or the time. The BUY and SELL
columns are in the CSV file, but the BUY column is often 0.
*Evidence:* `fa recon bonds` output; <https://www.treasurydirect.gov/GA-FI/FedInvest/selectSecurityPriceDate>.

**61. The latest day has no end-of-day price.** On 2026-11-18 the file had 0 in the END OF DAY column for each row. The
column is filled later. The command steps back to the last day that has prices.
*Evidence:* the file of 2026-11-18, downloaded on 2026-11-19.

**62. The Treasury curve is made from on-the-run securities only.** The inputs are the
most recently auctioned bills, notes and bonds. An older (off-the-run) security can trade
above or below the curve. This is a cause of differences that is not a data error. Our
curve is also not the exact curve of the Treasury (the program is not published).
*Evidence:* <https://home.treasury.gov/policy-issues/financing-the-government/interest-rate-statistics/treasury-yield-curve-methodology>.

**63. Notes and bonds are priced in 64ths.** A price step of 1/64 is 0.0156 per 100. For an
instrument with 13 days to maturity, this step is 47 bp of yield. The summary by maturity
shows large yield differences for short instruments because of this. Read the price
difference and the column `resolution_bp` first.
*Evidence:* the column `step bp` of the output; `recon.rs`.

**64. The first run shows bills above the curve.** On 2026-10-30, 2026-11-10 and
2026-11-17 the mean bill yield difference was about -6 bp (observed yield below theoretical
yield). The sign is the same on each date. We do not know the cause. Possible causes (open
questions): the price type (bid for the curve against another price type in the file);
the time of day; the way the Treasury converts a bill discount rate; our 365-day basis;
the settlement day (T+1 here); real bill demand. Do not use the curve to price a bill
without this check.
*Evidence:* `fa recon bonds --date 2026-11-17` (47 bills).

**65. No corporate bond prices.** We know no free official source of prices of single
corporate bonds. FINRA TRACE has terms of use (the owner request says so); we did not read
them in this work and we did not use TRACE. The Treasury and Fiscal Service files have no
corporate bond prices. Thus the corporate comparison is at
curve level only. The HQM curve is a monthly average (the date is the first day of the
month; on 2026-11-17 the latest value was for 2026-10-01). Our Treasury zero curve is for
one day. The ICE OAS is one number for the whole index (duration of about 6 to 7 years),
and not a number for each maturity. The HQM blend of AAA, AA and A has unknown weights, so
the mean of the three OAS is an arbitrary choice. We assume that the HQM spot rates are
semi-annual (the Treasury slides say so; we did not read the full paper).
*Evidence:* `fa recon bonds` output, section CORPORATE CURVE.

**66. STRIPS have no free official price source in the repo.** The FedInvest file has no STRIPS. The
only zero-coupon instruments in the set are bills (up to 52 weeks). The Federal Reserve
zero-coupon curve of Gurkaynak, Sack and Wright is a useful second reference for long
zero rates. The branch `claude/futures-forwards-data` loads it. This branch does not.

**67. Download of the price file.** The download needs a session (cookie, a CSRF token and a POST).
In the container of 2026-11-19, `curl` got the file, and the Rust client `mapcheck::http::Client` got
"HTTP 400 InvalidUri" from `www.treasurydirect.gov` for the same request bytes. We did not find the
cause. The Rust download code is thus not tested against the real site. Download the
file with `curl` into `data/raw/prices/fedinvest_<date>.csv`; the command uses this cache file.
The Fiscal Service terms say "Government works generally cannot be copyrighted" and ask for
credit; they also say that content may be used for non-commercial personal use. They do not forbid an
automated request. Ask the owner to confirm before a heavy use.
*Evidence:* <https://www.treasurydirect.gov/legal-information/terms/>.

## Could not verify

- (none of the named traps failed verification — all located in current code/docs; the
  scenarios-registry item was re-checked and is still true as stated in #13.)
- Entries 21–31 were verified by the parallel PR #11 agent on the same day, not
  independently re-verified by this registry's author; their evidence pointers stand.
