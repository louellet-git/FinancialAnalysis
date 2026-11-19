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

**68. An IPO registration statement (S-1, 424B4) can have no XBRL.** Then the standard
mapping gives no variables; the values come from the prospectus text by hand until the
first 10-Q or 10-K. The company facts of such a registrant have only the fee data
(namespace `ffd`). `mapcheck` gives the status `N/A` with the template `new` and the
finding `no_xbrl_values`. `mapcheck import` stops, thus the Python importer raises
`UnsupportedFilingError`. `mapcheck fetch --prospectus TICKER` puts the latest prospectus
in the cache (424B4; if none, S-1/A; if none, S-1). Example: ANT (Anthropic, PBC), IPO
of 2026-11-18. A foreign registrant with only fee data (form F-1) stays `ERROR`.
*Evidence:* `rust/mapcheck/src/run.rs` (`registration_only`); `rust/mapcheck/src/sec.rs`
(`fetch_prospectus`).

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

**59. FRED refuses some clients in this environment.** FRED closes the connection for a
client with an unknown `User-Agent` text (a `curl` request with a custom text failed;
with `curl/8.5.0` it worked). The Treasury site works with any text, so
`fa forwards check` reads the Treasury file and not FRED. The futures commands are in
traps 70 to 78.
*Evidence:* `rust/src/forwards.rs` (`check`).

**70. A Yahoo `=F` series is a chain of front contracts, not adjusted, and Yahoo chooses the
contract.** `ES=F` before 2026-09-18 differs from `ESZ26` on the same dates by 55 to 68
points (the September contract was the front contract then). From 2026-09-21 the two are
equal. The value of 2026-09-18 (7657.35) is the final settlement of the expiring contract;
it is not a multiple of the 0.25 tick. `ZQ=F` on 2026-11-19 was the January 2027 contract,
not the front month. Do not join old and new values of a `=F` series without a test. For a
curve, use explicit contract symbols (`fa futures curve`).
*Evidence:* `rust/src/futures/yahoo.rs`; `fa futures history ES` and `fa futures history ESZ26`.

**71. A Yahoo daily bar is a close that Yahoo gives. It is not a confirmed settlement.** The
date of a bar is the CME trade date. The time stamp is midnight New York, or 09:30 on a
day with a short session. `meta.gmtoffset` is the offset of today, so a bar of the other
half of the year is one hour away from midnight; the program accepts both. The last bar of
a running session is the last trade; the program gives it as `live` and not as a daily
value. Prices come as 32-bit floats (95.7025 comes as 95.70249938964844); the program
keeps the shortest text of the same 32-bit float. The volume of the newest daily bar was
a copy of the day before for ES, ZN and ZQ on 2026-11-18. The program cannot compare the
close with the exchange settlement, because no exchange file is available (trap 76).
*Evidence:* `rust/src/futures/yahoo.rs` (`parse_chart`, tests).

**72. Yahoo has almost no daily history for SOFR futures, and none for an expired contract.**
For `SR3Z26`, 1646 of 1720 daily rows have no close; the last close before the running
session is 2020-05-08. `SR1X26` has one row. Only the last trade is usable for SR3 and
SR1. `fa futures curve SR3` without `--date` uses the last trades; with `--date` it fails
on purpose. `ESU26` (expired 2026-09-18) gives HTTP 404, so the curve of a past date has
only the contracts that still trade. A quote with no daily bar in the 7 days before the date
gives no daily value and a note.
*Evidence:* `rust/src/futures/yahoo.rs` (`series`, `MAX_STALE_DAYS`); `fa futures quote SR3Z26`.

**73. The Cboe settlement file has no date inside, and it can show the wrong day.** For a
date that is not past, Cboe gives the file of the last trade date (`dt=2026-11-20` gave
the file of 2026-11-18, byte for byte). The program asks only for a date before today (UTC).
A weekend date gives a header with no row. The file of 2025-06-10 had no row and the file
of 2026-10-01 had rows; the age limit is not known. A weekly contract can show the price of
the monthly contract (2026-11-18: `VX47/X6`, `VX48/Z6`, `VX49/Z6` and `VX/Z6` all 21.9372;
2026-11-13: `VX/X6` and `VX47/X6` both 19.1079). Do not read a weekly price as a separate
market price.
*Evidence:* `rust/src/futures/cboe.rs` (`settlement`); the cached files in `data/raw/futures/`.

**74. The Cboe history file of a contract is named with the expiry date, and the date rule has
holiday exceptions.** The standard expiry is the Wednesday 30 days before the third Friday
of the next month. `VX/K7` expires on 2027-05-18, not 2027-05-19: the third Friday of June
2027 is a holiday. The file store answers HTTP 403, not 404, for a file that does not
exist. The program takes the expiry from the settlement file when it has it, and tries the
two days before the standard date when it has not. A file is posted the morning after the
trade date (the row of 2026-11-18 was not there at 04:00 UTC on 2026-11-19). In the history
file, 0 in open, high or low means no trade. The final settlement of an expired contract is
the special opening quotation (SOQ). The program writes a contract with a 2-digit year
(`VXZ26`), as the exchange does in the settlement file (`VX/Z6`).
*Evidence:* `rust/src/futures/cboe.rs` (`standard_expiry`, `history`, tests).

**75. CFTC positions are old, and they are classes of traders.** The data are the positions of
Tuesday. The CFTC says: "The COT Report is generally published each Friday at 3:30 pm
Eastern Time (US)". On 2026-11-19 the newest week was 2026-11-10; the week of 2026-11-17
comes on 2026-11-20. The report is "Traders in Financial Futures", futures only, all
contract months together. A group (dealer, asset manager, leveraged funds, other
reportable) is a class of traders, not one firm. The E-mini S&P 500 has two codes: 13874A
(E-mini, used here) and 13874+ ("S&P 500 Consolidated"). Net = long - short; spread
positions are not in the net.
*Evidence:* `rust/src/futures/cftc.rs`; `fa futures positions ES`.

**76. Only two independent checks exist, and the CME prices have none.** The CME website
refuses automated access (HTTP 403: "This IP address is blocked due to suspected web scraping
activity"). Stooq answers with a page that needs JavaScript ("This site requires JavaScript
to verify your browser"); the program does not run it. `stooq.pl` and `ftp.cmegroup.com`
closed the connection; Nasdaq Data Link and Investing.com answered HTTP 403; Barchart gave an
empty answer. So ES, NQ, the Treasury futures and the SOFR prices have no second source. The
two checks are: the VIX settlement file against the Cboe history files, and the implied rate
of the 30-day fed funds contract of the current month against the average EFFR of the
month (tolerance 5 bp). The second check can fail when the FOMC changes the target range
inside the month. A pass of a check does not prove a price of another contract.
*Evidence:* `rust/src/futures/check.rs`; `fa futures check`.

**77. The contract facts are typed from the exchange specifications.** The tick, the contract
size and the price meaning in `rust/src/futures/facts.rs` were not tested against the
exchange page (the CME page is blocked, trap 76). Only the quarterly months (H, M, U, Z)
of SR3 are listed; the serial months exist and the program does not list them. The Yahoo
short name has 30 characters at most: `Three-Month SOFR Futures,Dec-20` is December 2026.
*Evidence:* `rust/src/futures/facts.rs`.

**78. The Atlanta Fed policy path is a model estimate.** The Market Probability Tracker
estimates a distribution of the average SOFR for each 3-month reference window from the
prices of CME options on 3-month SOFR futures. The file starts on 2023-03-29. Its last row
is one to two days old (2026-11-17 on 2026-11-19). The file says: "Use of this data is permitted for
personal and educational purposes only." The Bank gives rates in basis points and
probabilities in percent; the program gives decimals. `prob_below_range` and
`prob_above_range` are relative to the FOMC target range of the observation date, so they
change when the range changes. Example: on 2026-11-17 the mean for the window that starts
2026-12-16 was 4.2853%; the last trade of `SR3Z26` on 2026-11-19 gave 4.2975%. The two are
two days apart.
*Evidence:* `rust/src/futures/mpt.rs`; `fa futures policy-path`.

**80. The rate series are proxies of the short rate.** The default series is the GSW
1-year zero yield. It is a fitted yield of a 1-year bond, not an overnight rate. It has a
term premium and it moves less than the overnight rate. A model fitted to it is a model of
this series. The instantaneous forward rate at 0 (`f(0,0)`, 4.13% on 2026-11-13) is
different from the 1-year yield (4.63%). A Treasury 3-month history is not built.
*Evidence:* `rust/src/ratemodels/cli.rs` (`load_series`); `fa forwards --date 2026-11-13`.

**81. Mean reversion of a rate is weakly identified.** On the full GSW 1Y history
(1961 to 2026, daily), Vasicek ML gives kappa = 0.046 with standard error 0.037
(t = 1.26): not different from zero. In rolling 5-year windows (weekly data, step 3
months), 64 of 242 windows have an AR(1) coefficient of 1 or more, and the fit gives an
error. Only 12 of the 178 other windows have t above 2.86. Least-squares kappa is also
biased up in a short sample. Do not use theta as a forecast. Read the standard error and
the flag in each fit.
*Evidence:* `fa models calibrate --model vasicek --method ml`; `fa models rolling`.

**82. The parameters depend on the sampling frequency.** Vasicek sigma on the full history
is 0.0096 (daily), 0.0120 (weekly) and 0.0149 (monthly), and kappa is 0.046, 0.071 and
0.109. This is not a code error. The exact map removes the effect of the step for data
that come from the model (tests at two steps). Real changes of rates are autocorrelated,
and the volatility changes with time. So the model is not right at all frequencies.
`dt` is the mean calendar spacing (about 1/249 year for daily data).
*Evidence:* `fa models frequencies --model vasicek --method ml`.

**83. CIR and CKLS need positive rates, and ML can hit a bound.** CIR and CKLS give an
error if one rate is 0 or lower. For the last 10 years (1Y yield near 0 in 2020 and 2021,
about 4.6% now), CIR ML and CKLS quasi-ML push kappa to the lower bound 1e-4 and the code
gives an error. The Feller ratio of the full-history CIR fit is 1.3: near the limit.
The CIR sampling scheme is QE (not the exact chi-square): it is correct near 0 but it is
an approximation of the law of each step (mean and variance are exact).
*Evidence:* `rust/src/ratemodels/calib.rs`; `fa models calibrate --model cir --method ml --start 2016-11-13`.

**84. CKLS gamma is not stable between methods.** On the full history, GMM gives
gamma = 1.00 (standard error 0.07), and quasi-ML gives 0.43. The two methods use different
information (the GMM variance equation uses `E[(e^2 - s r^(2g)) r] = 0`). The quasi-likelihood
has the exact mean and an Euler-type variance, so its AIC is close to the exact ones but
not strictly equal. The CKLS simulation scheme (Milstein, full truncation) is biased near 0;
for gamma = 1/2 use `Cir`.
*Evidence:* `fa models calibrate --model ckls --method gmm` and `--method ml`.

**85. Hull-White and Ho-Lee mix two measures.** `theta(t)` comes from the zero curve of one
date. It is in the pricing measure, and the model reprices that curve. `a` and `sigma` come
from the history, in the real-world measure. A path of this model is not a forecast. The
curve is the Svensson curve of the day, so maturities above 30 years are an extrapolation.
Ho-Lee has no mean reversion: the rate can be negative and the variance grows with time.
*Evidence:* `rust/src/ratemodels/models.rs` (`HullWhite`); the output field `zero_curve`.

**86. The Monte Carlo bond price has two small errors.** The discount integral uses the
trapezoid rule (error of order dt^2). A simulation with few steps for each year has a
bias in the price. The test tolerance is 4 standard errors plus 5e-5. The same seed
gives the same numbers with any number of threads, but not the same numbers if you change
`steps-per-year` or the number of paths.
*Evidence:* `rust/src/ratemodels/sim.rs` (`trapezoid`); `fa models simulate`.

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

**90. Yahoo answers HTTP 429 to the bar downloads from a shared network address.** From the
cloud container, about half of the requests of `fa trade fetch` got HTTP 429 (2026-11-19), also
with 2 seconds between two requests. The program waits 30 seconds and asks one more time. If
the answer is 429 again, it stops, and the cache files stay as they are. Run the command again
later. It does not use a browser identity and it does not go around the limit.
*Evidence:* `rust/src/trading/fetch.rs` (`refresh`, `BACKOFF_SECONDS`).

**91. Yahoo corrects the volume of recent bars, and the cache of an old run can have wrong
volume.** The Python cache of 2026-11-18 had a volume of 59,627,886 for the hourly SPY bar of
10:30 on 2026-11-18; Yahoo gives 5,636,182 on 2026-11-19. The prices were the same. A refresh
replaces the row, so the volume changes. The bot s02 (VWAP) uses the volume. A result of a bot
for the last days can change after a refresh. The older rows were the same text in the
comparison (see the ticket `option-backtest-rust`).
*Evidence:* `fa trade fetch` into a copy of the cache, 2026-11-19.

**92. The live option chain is the Cboe delayed quote file, and it does not list each expiry that
a bot asks for.** The file is the one that the delayed-quotes pages of cboe.com read. It is not a
published API: Cboe can change it. The quotes are about 15 minutes late, and before the open they
can be stale. The bots assume an expiry on each trading day, and they use `busday_offset`
(no holiday calendar). The chain has an expiry on each trading day for about two weeks, then
fewer (SPY on 2026-11-19: each day to 12-04, then 12-11, 12-18, 12-24, 12-31, then one or two
a month). A signal with a missing expiry gets the model price, and `fa trade paper` says so in the signal line. The Python paper
trader said "live chain" in that case. The adjusted options (root `B1SPY`) are not used.
*Evidence:* `rust/src/trading/chain.rs`, `rust/src/trading/paper.rs` (`price_signals`).

**93. Yahoo gives no option chain to the honest client.** The option endpoint needs a cookie and
a crumb. The crumb request answered HTTP 429 to the program with its plain User-Agent
(2026-11-19). The program does not imitate a browser. Thus the chain of Cboe could not be
compared with a chain of Yahoo. The comparison that was possible: the underlying price (Cboe
737.92 and Yahoo `fulldayPrice` 737.919 for SPY before the open) and the last close (both 730.2).
*Evidence:* `curl` of `fc.yahoo.com` and `v1/test/getcrumb` with the User-Agent of `fa`.

**94. The cache has no bar that has not ended.** The earlier Python download wrote the partial bar
of the running session (the daily bar of today, and the hourly bar in progress), and the paper
trader used it as a closed bar. `fa trade fetch` writes a bar only after its end time (the end of
the hour or of the 5 minutes, but not after 16:00; a daily bar after 16:00). A
paper step during the session sees the bars that have ended. The live price row of Yahoo (a row
with seconds, for example 09:35:26) is not a bar and the program drops it. On an early-close day
Yahoo adds a row at 13:00 with no volume; the program drops it with `meta.tradingPeriods`.
*Evidence:* `rust/src/trading/fetch.rs` (`rows_from_chart`).

## Could not verify

- (none of the named traps failed verification — all located in current code/docs; the
  scenarios-registry item was re-checked and is still true as stated in #13.)
- Entries 21–31 were verified by the parallel PR #11 agent on the same day, not
  independently re-verified by this registry's author; their evidence pointers stand.
