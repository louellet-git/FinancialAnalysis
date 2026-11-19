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

## DCF in Rust (added 2026-11-19)

**120. The Rust random numbers are the numpy stream for an integer seed only.**
`rust/src/dcf/nprandom.rs` makes the same numbers as `np.random.default_rng(seed)`:
`SeedSequence`, `PCG64` and the ziggurat `standard_normal`. The tables and the
algorithm are from numpy v2.5.3. The tests compare 1,000 values for each of the seeds
0, 7, 42 and 123456789, bit for bit, and a long stream that reaches the tail branch.
A seed must be an integer from 0 to 2^64 - 1 (a list or a `SeedSequence` is not
supported). Numpy can change a `Generator` method in a new version. If the fixture
tests fail after a numpy update, the Python numbers changed, not the Rust numbers.
*Evidence:* `rust/src/dcf/tests.rs` (`standard_normal_matches_numpy_bit_for_bit`).

**121. Rust and Python differ in the last digit for some values.** The driver engine in
Rust makes `(1 + g)^t` and the discount factor by repeated multiplication. Python used
`**` on arrays. The values differ by about 1e-16 relative. The percentiles of the IBM,
JNJ and ANT runs differ by 2e-16 at most. The attribution values differ by 1e-14 at
most. The Rust mean and std of the classic engine use a plain sum, numpy uses a
pairwise sum (1e-15). The tests accept 1e-9. A printed number with 2 decimals does not
change.
*Evidence:* `cargo test parity_report -- --ignored --nocapture`.

**122. A seed of None gives a new number at each run in both languages.** In Python
`seed=None` uses the operating system. In Rust the seed comes from the clock. Do not
use a run without a seed for a published number. The Rust drivers command has the
default seed 7.

**123. A driver model has two copies until the Python code is deleted.**
`analysis/<TICKER>/driver_model.toml` is the Rust copy. `driver_model.py` is the Python
copy. A test shows that the TOML file gives the numbers that the Python code gave on
2026-11-19, including each rationale text. A change in one copy does not change the
other. Until the Python files are deleted (ticket `T-20261119-dcf-engine-rust`, stage 4),
make a model change in both files. Then record the new fixtures. The IBM version 4
model is a closed record: its test must not change without a review of the same weight.
*Evidence:* `rust/src/dcf/tests.rs` (`toml_models_give_the_python_models`).

**124. The Rust engine allows a schedule on the margin drivers only.** The Python engine
accepted a schedule on any deterministic driver. For a CAPM driver or a growth driver
the Python result had one WACC for each year and path, and no model used it. The Rust
engine refuses this input. A schedule can be a list of values or a ramp (`start`,
`end`, `ramp_years`); only a ramp follows a change of the horizon (`--years`).

**125. The runners did not use the same rule for a changed mean.** The IBM and JNJ
runners changed the mean and kept the bounds `lo` and `hi`. The ANT runner moved the
bounds with the mean. `fa dcf drivers --set` keeps the bounds. Add `--shift-bounds` to
move them. The wrong choice changes the result: for ANT a growth mean of 30% with fixed
bounds gives a different P50.
*Evidence:* `rust/src/dcf/runner_tests.rs` (the ANT lines use `shift_bounds`).

**126. `fa verdict` with market rates still runs Python.** A run without `--rf-date`
and without `--rating` uses the Rust engine when the ticker has `driver_model.toml`.
A run with market rates needs the Treasury and ICE data, which only Python gets, and
runs `scripts.verdict` (same for a ticker that has only `driver_model.py`). Both paths
have the same JSON fields. The last digit of a percentile can differ (see #121).
`fa dcf classic` and `fa dcf scenarios` read `data/processed/<TICKER>.json`; a new
worktree has no `data/` folder (it is not in git): use `--data-dir`.

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
without this check. Update 2026-11-19: see #100 and #101 for the tests and the cause.
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
automated request. Ask the owner to confirm before a heavy use. Update 2026-11-19: the cause of the Rust defect is in #69.
*Evidence:* <https://www.treasurydirect.gov/legal-information/terms/>.

## Bond research (added 2026-11-19, branch `claude/bond-arb`)

**69. Cause of the price-download defect (replaces the open question in #67).** The Rust client
`mapcheck::http::Client` uses `ureq` 2.12.1. With an HTTP proxy, `ureq` puts the full URL in the request
line, also for HTTPS inside the CONNECT tunnel: `GET https://www.treasurydirect.gov/GA-FI/... HTTP/1.1`.
TreasuryDirect (Windows-Azure, HTTP.sys) refuses this form with "400 InvalidUri". `curl` sends
`GET /GA-FI/... HTTP/1.1` and works. We saw the bytes with a local TLS server. Other sites (Yahoo, the
Treasury, the Federal Reserve) accept the full URL. The code of `ureq` 2.12.1 uses the full URL for each HTTP proxy,
whatever the scheme of the target; the settings of `http.rs` were right. Fixed on 2026-11-19: `mapcheck` uses `ureq` 3.5
(features `rustls`, `platform-verifier`, `gzip`). Version 3 sends the short form inside the tunnel. A new date
(2026-09-04) came from `fa recon bonds --date 2026-09-04` (464 lines). The other users of the client were
tested again: GSW file, Cboe file, Yahoo chart (one `429` on the first call, a normal answer on the second;
`curl` got `200` in between), Treasury par curve. The SEC path was not tested.
*Evidence:* the request seen by a local TLS server; `ureq-2.12.1/src/unit.rs` (function `send_prelude`).

**100. The bill difference is not a convention error. It is the bill nodes of the par curve.** On
2026-10-30, 2026-11-10 and 2026-11-17, the Treasury par curve has bill nodes 1M, 1.5M, 2M and 3M that are
8 to 11 bp above the coupon-equivalent rates of the Treasury's own "Daily Treasury Bill Rates" of the
same day. The nodes 4M and 6M agree (0 to -4 bp). The FedInvest bill prices agree with the bill-rate
table. If the bill nodes come from that table, the mean bill difference falls from -5.6, -7.1 and -6.8 bp to -1.0,
-2.3 and -1.9 bp (`fa recon bills`). We do not know why the two Treasury series differ. Bills of 6 to 12
months stay 3 to 6 bp below the curve (6 bills; the 6M to 1Y interpolation, and the 1Y par node is 3 bp
above the 52-week bill). Do not call the 1M to 3M bill difference a market effect: the same source says
that bills are where our bills are.
*Evidence:* `fa recon bills --date 2026-11-17`; the bill-rate file of the Treasury.

**101. Bill conventions that were tested and are right.** (1) Settlement is T+1 business day: for
2375 of 2375 bills in 50 files, the END OF DAY price is a bank discount rate that is a multiple of
0.01 % at T+1 settlement. With T+0 this is true for 3.8 %, with T+2 for 3.1 %. (2) The yield is
the bond-equivalent yield, actual/365: a discount rate of 3.92 % for 28 days is 3.99 %, the number
in the Treasury table. A test of this is in `billdiag.rs`. (3) Other bases make the difference worse
(actual/360: 5.6 bp more; bank discount: 10 bp more) or are wrong (semi-annual gives 2 bp less and is
not the Treasury convention). A change to T+2 gives about 0 bp by chance: it contradicts (1).
(4) The price field changes the mean by less than 1 bp.

**102. SELL is the bid side, BUY is above it.** In the FedInvest file the SELL column is equal to END OF DAY for
most notes and bonds, and BUY is 1/64 to 1/32 above it. So END OF DAY is a bid-side price and BUY - SELL is the
bid-ask of the file. The BUY column is 0 for 25 notes and 1 bond of 463 rows on 2026-11-17. For bills, BUY and SELL are
discount rates of the price date (T+0) that differ by less than 1 bp, and END OF DAY is the T+1 value. We do not know if
BUY and SELL are quotes that a person can trade. The spread is the step of the 64ths in many rows, so it may be
a minimum and not a measured spread.
*Evidence:* the files of 2026-10-30, 2026-11-10, 2026-11-17.

**103. The fitted curve has a model error and some group effects look like outliers.** The 6-parameter curve
has a fit error (RMSE) of 1.5 to 2.5 bp a day (about 1 bp at 3 to 7 years, 2 to 3 bp at 3 to 6 months and
10 to 20 years). A residual below this has no information; the candidate rule uses it. The Treasury has no
note between 10 and about 13 years, so the sector between 2036 and 2039 holds only the old 30-year
bonds. They are 5 to 14 bp rich as a group, so the fit leaves most of them out as outliers and the curve
interpolates around them. The z-score against the 20 nearest securities cannot find a group effect,
because the neighbours are in the group. Read the group table and `fa recon history` before you read one security.
Our curve agrees with the GSW zero curve within 1 bp on average from 1 to 20 years (47 dates); at 25 and 30 years the
mean is -2 and -8 bp and one date is 33 bp away (the end of the curve has few securities).

**104. GSW has a delay.** On 2026-11-19 the last GSW row was 2026-11-13. The comparison with GSW uses
the GSW date and our fit of the same day, and prints the GSW date.

**105. A residual that stays is not a convergence.** Over 50 business days (2026-09-08 to 2026-11-18, 16289
security-days), the raw autocorrelation of the residual is 0.95 (lag 1) and 0.92 (lag 10): a security keeps
its place. After the mean of each security is removed, it is 0.37 (lag 1), 0.15 (lag 5), 0.01 (lag 10): the
deviation from its own mean has a half-life of under 2 days. The 10 percent most cheap and most rich of a date keep 84 to 96 % of their
residual after 10 days; after the mean is removed, 4 to 7 %. So the large residuals are mostly permanent
(group, coupon, issue-type effects), and the temporary part is small and can be noise of the price source (price steps).
There is no evidence in these 50 days that a rich or cheap security goes back to the curve. A trade on the
permanent part needs a financing view that the data does not have.

**106. The cost is one leg and a lower bound.** The cost of a candidate is its bid-ask (file, else the median
of the file in the same maturity bucket; at least one 1/64 step). It is not a trade cost: a pair needs a
hedge leg and an exit, and the repo rate, the short-sale cost, the balance-sheet cost, the tax and the size of a trade
are not in the data. Of 400 securities on 2026-11-18, 14 pass (3 cheap, 11 rich); they are
mostly old 30-year bonds and low-coupon notes. Call them candidates.

**107. The HTTP client is shared; the change to `ureq` 3 touches each Rust data command.** `mapcheck/src/http.rs`
is the only file that uses `ureq`. The error rule is the same as before: an answer with a code of 400 or more
is an error with the code, and a 3xx answer is not an error (the form session needs it). The session client follows no redirect. The normal client follows up to 5 redirects, as before.
Version 3 decodes gzip itself. The SEC commands (`mapcheck`) use the same
client and were not run in this work. Run one SEC request before you trust that path.

**108. The New York Fed documentation page cannot be read without JavaScript.** The endpoints for the
secured rates and for the SOMA holdings were tested by their answers (HTTP 200, the field names in
`refdata.rs`). They were not checked against the text of the documentation. A NY Fed FAQ (seen only in a search result, not read in full)
says that the Desk holds at most 70 % of the amount outstanding of one issue: the SOMA share of 70.0 % for many
bonds is that limit, not a measurement of the free float of the market. The SOMA file is weekly (as of
2026-11-10 for the price date 2026-11-18); a CUSIP that is not in the file has a share of 0. The rates are general
collateral rates: SOFR, BGCR and TGCR. A special repo rate for one CUSIP is not free data. On 2026-11-18, TGCR was
3.88 % and SOFR 3.90 %.

**109. In MSPD table 5 the field `cusip` is the CUSIP of the principal strip.** The CUSIP of the note or bond
is in `security_class2_desc`. Amounts are in thousands of dollars. The table is for a month end (the last one
was 2026-10-31), so a security issued after it has no row: 6 of the 327 securities (new notes and one new bond). The stripped share is the amount in stripped form divided by the amount
outstanding.

**110. The original term is from the auction record, rounded to the standard term.** The first auction of a
May bond has the term "29-Year 9-Month" (a 30-year bond with a short first period). A test on the exact text
"30-Year" misses 2 of the 9 bonds (912810PU6, 912810PX0) and makes one of them the on-the-run bond. The code
rounds a term to 2, 3, 5, 7, 10, 20 or 30 years if it is within 3 months below or 4 months above. The first
issue date is the earliest issue date of the CUSIP. The on-the-run security of a term is the CUSIP of its latest auction that
settled on or before the date, so a reopening is on-the-run. Auction records start in 1979; an older security
has no record (none in the 327).

**111. The regression is an association, not a cause.** The features are not independent. The SOMA share is
60 to 70 % for most of the old 30-year bonds, so it does not separate them from each other. With the two
original 30-year groups in the model, the SOMA coefficient is 0.0 bp for each 1 % (t = -0.2) and R2 is 0.55. Without them, the SOMA
coefficient is -0.11 bp for each 1 % (t = -4.7) and R2 is 0.18. Bonds with a SOMA share of 60 to 70 % are not
rich at every maturity: the bonds of 2027 to 2028 are 10 to 15 bp cheap, the bonds of 2043 to 2046 are 1 to 2 bp
cheap, and the bonds of 2036 to 2039 are 6 to 19 bp rich. So the SOMA share alone does not explain the
richness. The t values of the regression on the mean residual are too large (the 50 dates are not independent).
The dummy "original 30-year, 5 to 15 years left" takes -9.7 bp: that is the group effect, and the regression does
not say why it exists.

**112. The nine old 30-year bonds (2036 to 2039) are rich by 6 to 19 bp, and the curve does not make that.**
Against the note that has the same maturity (912810FT0 against 91282CPZ8, 2036-02-15) the bond has a yield 17.5 bp
lower; the curve gives 0.2 bp for the coupon difference of 0.375 points. The price difference is 1.25 points more
than the curve gives. The curve fitted without all original 30-year bonds and without all bonds with a SOMA
share of 60 % or more gives the same sign and a size that is 1.4 to 1.6 bp larger for each of the nine. But the notes end in 2036-11 and no note or
non-SOMA security is in the 2036 to 2042 range, so for the bonds of 2037 to 2039 the curve is an interpolation
across a gap. Only the bond of 2036-02 has a note of the same maturity. The richness falls with maturity
(-18.6 bp for 2036, -6.4 bp for 2039-11, -5 bp for 2040, 0 bp for 2042) as if a sector effect fades. None of the
nine is deliverable into a Treasury future on 2026-12-01 (the bonds have less than 15 years left; ZN and TN take notes only).
The carry of a long position that is financed at the general collateral rate is +100 to +132 bp a year, so
financing does not explain the richness. The data cannot show a special repo rate or a shortage of bonds to borrow. A person must ask a
dealer for the special repo rate and the availability for these nine CUSIPs before the word "arbitrage" is used. A short position in a
rich bond pays that rate, and a long position in the note in its place is not a hedge of the same duration.

**113. The CME rules were not read in the original page.** `www.cmegroup.com` refused the requests of this work
(HTTP 503 from the fetch tool, 403 with `curl`), and the work did not go around the refusal. The rules in
`explain.rs` come from the text that web search returned about CME documents (the pages were not read by this work), on 2026-11-19: the delivery
guide <https://www.cmegroup.com/trading/interest-rates/files/us-treasury-futures-delivery-process.pdf> (8th edition,
May 2023) for ZT, ZF, ZN and the 3-month rounding; the Special Executive Report SER 9520 of 2025-02-21
<https://www.cmegroup.com/content/dam/cmegroup/notices/ser/2025/02/ser-9520.pdf> and the delivery guide for ZN
(6 years 6 months to less than 8 years); the contract page
<https://www.cmegroup.com/markets/interest-rates/us-treasury/ultra-10-year-us-treasury-note.contractSpecs.html>
for TN (original 10-year notes, 9 years 5 months to 10 years); the rulebook chapter 18
<https://www.cmegroup.com/rulebook/CBOT/II/18.pdf> (rule 18101.A) for ZB (15 years to less than 25 years); and
<https://www.cmegroup.com/trading/interest-rates/ultra-tbond-futures.html> for UB (25 years or more). A CME
education page still says "not more than 10 years" for ZN; we took the newer 8-year limit. The
10-year limit would not change the result for the nine bonds (they are bonds). Read the current rulebook
before you use a flag for a trade.

**114. The flag "deliverable" is for the front month only and for the rules above.** The front month on
2026-11-18 is December 2026 (first delivery day 2026-12-01). The flag does not say which security is the cheapest
to deliver. The conversion factors and the futures prices are not in the data. Of the 327 securities: ZT 10,
ZF 8, ZN 10, TN 2, ZB 59, UB 19 are deliverable (108 flags; a note can be in more than one basket).

**115. Coverage and units of the joins on 2026-11-18.** The regression has 321 securities (6 of 327 have no row in
MSPD table 5 or no auction record), and 317 have a mean residual over 40 or more dates. The residual in the
regression is in bp of yield. The coupon gap is in percentage points. The curve for the par rate uses the first
coupon period as a full period (an error of a few tenths of a bp). The securities that the fit left out as outliers (13)
are in the regression. The two bonds of 2027 (6.375 % and 6.125 %) have a cost of 26 to 28 bp: they do not matter for
the conclusion, and they move the intercept little.

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
