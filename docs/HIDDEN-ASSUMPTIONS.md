# Hidden assumptions and landmines — the trap registry

Non-obvious assumptions an agent can trip on. Each entry carries an evidence pointer;
every entry below was re-verified against the code/docs on 2026-07-17. If you find a new
one, add it here with a pointer.

**Deleted evidence files (2026-11-19).** The entries 1 to 135 name Python files as evidence. These
files are deleted: `financial_analysis/{backtest,dcf,driver_dcf,scenarios,schema,store,market_data,options}.py`,
`analysis/{IBM,JNJ,ANT}/driver_model.py` and `scripts/visualize_dcf.py`. A trap is still a trap; its
Rust form is in the files below. The line numbers refer to the old files: read the old text with
`git show 4e6d0ba:<path>` (the last commit that has the files).

| Deleted file | Rust form |
|---|---|
| `driver_dcf.py`, `analysis/<T>/driver_model.py` | `rust/src/dcf/driver.rs`, `rust/src/dcf/model.rs`, `analysis/<T>/driver_model.toml` |
| `dcf.py`, `scenarios.py` | `rust/src/dcf/classic.rs`, `rust/src/dcf/scenarios.rs` |
| `backtest.py` | `rust/src/dcf/backtest.rs` |
| `schema.py`, `store.py` | `rust/src/company.rs` (`Company`, `series`), `rust/mapcheck/src/store.rs` |
| `market_data.py`, `options.py` | `rust/src/market/prices.rs`, `rust/src/market/options.rs` |
| `scripts/visualize_dcf.py` | `rust/src/report/` (`fa report`) |

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
`fa analyze` (earlier `scripts/analyze.py`) lives only in the xlsx and in dated snapshots under
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
logic. (Update 2026-11-19: `scripts.analyze` and `scripts.import_financials` are deleted; `fa import` and
`fa analyze` call the Rust import in the same process.) Each Python command that imports a
ticker runs `rust/mapcheck/target/release/mapcheck`. If the program is not built, they
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
(Closed 2026-11-19: `fa analyze` prints the number of sheets, 9, and the number of data rows.)
Evidence: `rust/src/analyze.rs` (`command`).

**18. (Closed 2026-11-19.) Generated files were tracked.** Two xlsx files and four
.pyc files were in git although `.gitignore` excludes them. They are removed from the
index. `data/` and `__pycache__/` are not in git; do not add them.

**19. RPO rows hide on the Balance Sheet tab.** `remaining_performance_obligation` is a
disclosure fact but is written to the Balance Sheet tab with `statement="disclosure"`
(no dedicated tab). Its current/noncurrent duration split is text-only in filings and
is not extracted. Evidence: `rust/src/analyze.rs` (`sheet_for`);
`financial_analysis/standard_variables.toml` (`remaining_performance_obligation`).

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

**123. A driver model has one copy: the TOML file (changed 2026-11-19).**
`analysis/<TICKER>/driver_model.toml` is the only copy. The Python copies `driver_model.py`
and the runners (`dcf_drivers_run.py`, `first_pass_run.py`) are deleted. A test shows that the TOML
file gives the numbers that the Python code gave on 2026-11-19, including each rationale text
(the fixtures are the record). After a model change, record new fixtures. The IBM version 4
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

**126. `fa verdict` with market rates runs in Rust (changed 2026-11-19).** All options
of the old `scripts.verdict` (`--rf-date`, `--rf-base-date`, `--rating`) run in Rust. A
ticker needs `driver_model.toml`.
The JSON fields are the same. The last digit of a percentile can differ (see #121).
`fa dcf classic`, `fa dcf scenarios` and `fa backtest` read
`data/processed/<TICKER>.json`; a new worktree has no `data/` folder (it is not in
git): use `--data-dir`.

**127. The backtest has one copy (changed 2026-11-19).** `fa backtest` and the report use
`rust/src/dcf/backtest.rs`. The Python copy `financial_analysis/backtest.py` and the Python command
`scripts.backtest` are deleted. For IBM on the real store the text of the Rust command was the same as the
text of the Python command, character for character; the fixtures have synthetic companies with the IBM
data flag, a 10-K/A, late filings, phantom rows and thin history.
*Evidence:* `rust/src/dcf/tests.rs` (`backtest_parity`); `rust/tests/fixtures/backtest/`.

**128. FRED does not answer the cloud container.** On 2026-11-19 the connection to
`fred.stlouisfed.org` was closed by the other side (Rust: "peer closed connection without
sending TLS close_notify"; curl: HTTP/2 stream INTERNAL_ERROR). The Python client had the
same refusal. The Treasury, the New York Fed and the BLS sites answered the client
`FinancialAnalysis fa (personal research)`. The code does not retry, does not change the
client name and does not imitate a browser. These commands need FRED: `fa bonds` (all
subcommands), `fa verdict` with `--rating`, `fa rates fred` and `fa rates check`. They
could not run against the real site. The other `fa rates` commands do not need FRED. Their parsers and logic are tested on fixtures: the
FRED series in `rust/tests/fixtures/rates/fred.json` are made up (ICE data may not be
copied into a public repo). On a network that FRED answers, run `fa bonds` and
`fa bonds check` once before you trust a `--rating` result.
*Evidence:* `fa rates fred --series DGS10` on 2026-11-19.

**129. The BLS API has a daily request limit for each address.** On 2026-11-19 the CPI
request got "the daily threshold for total number of requests ... has been reached". API
version 1 gives 25 requests a day for each address, and an address can be shared. The code
saves the answer for 24 hours in `data/raw/rates/bls_cpi.json`, does not save an answer
that failed, and uses an old file with a warning when the request fails. The file that
the old Python code wrote is valid for the Rust code. If there is no file, `fa rates
inflation` and `fa verdict --rf-date` stop with the BLS text.

**130. The rates, the bonds and the verdict with rates are equal to the Python output.**
Checked 2026-11-19 on the real Treasury, New York Fed and BLS answers (not FRED): `fa rates
yield|curve|history|nyfed|inflation|snapshot` and `fa verdict IBM --price 228.53 --rf-date
today --rf-base-date 2026-07-16` equal the Python JSON to 1e-9 relative, and the error text
for a missing tenor is the same. Two differences: the snapshot has the field `real` in the
two curves (the Rust type always had it; the real curve is now `true`), and the Rust
programs send one request less (the EFFR rate and the FOMC range come from one answer).
The ICE, Moody's and HQM paths equal the Python output on the fixtures only (#128).
*Evidence:* `rust/src/rates_tests.rs` (`parity_with_python`).

**131. The network commands wait and do not retry.** Each `fa` data command waits 2
seconds between requests and makes one request for each file; the Python code tried 3
times. A first `fa bonds` makes 24 FRED requests (about 50 seconds); later runs read
`data/raw/bonds/` (6 hours). A refusal is an error. If a download fails and a cache file
exists, the old file is used and a warning goes to stderr: read the dates of the output.

**132. `fa dcf scenarios IBM` keeps the version 3 inputs.** The Python script
`dcf_scenarios.py` (deleted) took the FCFF margins from the store. On the store of
2026-11-19 it gave a mean margin of 18.4% and P50 $43 / $149 / $243 (bear / base / bull);
the pinned version 3 inputs of the README have 17.8% and $37 / $141 / $232. The Rust
scenario entries are the pinned inputs, so the Rust command gives the README numbers. The
loci and the WACC grid of the README: `fa dcf classic ...` (see `analysis/IBM/README.md`).
A classic row reads the base revenue from `data/processed/IBM.json`; the margin, the net
debt and the share count come from the options.

**133. Two numbers of the JNJ first pass differ in the second decimal.** The text of
`analysis/JNJ/FIRST-PASS.md` used a solver that is not in the repo (commit `076a953`):
core margin 32.4%, terminal growth 4.78%, rf 3.13% and 2.26%. `--implied-price` gives
32.3%, 4.76%, 3.15% and 2.27%. The classic-engine row ($142.55 / $200.59) cannot be made
again: `data/processed/JNJ.json` changed after the first pass.

**134. A change of the mature margin needs a rule for the margin of year 1.** The ANT
runner kept the margin of year 1 when it changed `core_margin` (the drift ramp moved).
The option `--keep-year1-margin` does this. Without the option, a change of
`core_margin` changes year 1 too. A model without a ramp on `margin_mix_drift` gives an
error with the option.
*Evidence:* `rust/src/dcf/runner_tests.rs` (`keep_year1_margin_is_the_core_mean_of_the_ant_runner`).

**135. Python files that stay, and why (changed 2026-11-19).** The report moved to Rust, and the Python engine
files, the store, the schema, the market-data readers, the report script, the three `driver_model.py`
files and all Python tests are deleted (#147 to #153). Five Python files remain: the legacy SEC
client in `financial_analysis/` (the SEC contact text is in it), `financial_analysis/importer.py` (only `_run`),
`financial_analysis/__init__.py`, `scripts/import_edgar.py` and `scripts/__init__.py`. They stay until the owner
decides the new place of the SEC contact text (ticket `T-20261119-python-to-rust`, the one open step).

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

## Report, market data and company data in Rust (added 2026-11-19, branch `claude/report-rust`)

**140. Prices are adjusted for splits and dividends.** The Python code used `yfinance` with
`auto_adjust=True`: open, high and low are multiplied by `adjclose / close`, and the close is the adjusted
close. The Rust code does the same with the `adjclose` array of the Yahoo chart reply (`includeAdjustedClose=true`).
Thus an old close in `data/raw/<T>/prices.json` is not the price of that day. Each new dividend changes
all older rows, and a cache file that is old has rows that differ from a new download. The last row is the
live row of the running session, as in the Python code. A row with no open, high, low or close is dropped
(the Python code wrote `NaN`, which is not JSON); a row with all prices and the volume empty or zero is dropped.
The date of a row is the New York date (`America/New_York` is the time zone of all U.S. stocks; for another
zone the program uses the offset of `meta.gmtoffset`).
*Evidence:* `rust/src/market/prices.rs`; the fixture `rust/tests/fixtures/market/yahoo_prices.json` is made with the
functions of yfinance (`parse_quotes`, `fix_Yahoo_returning_live_separate`, `auto_adjust`).

**141. Yahoo answers HTTP 429 to the prices request from the container.** Tested 2026-11-19: two requests,
some minutes apart, both HTTP 429. The program does not get around this and does not imitate a browser. It
says "Yahoo answered HTTP 429 (too many requests)". A cache file stays as it is: with `--refresh` and an
answer of 429, the old file is used and the message says so. With no cache file, the prices are empty and
the other sheets are still written. The price loader was tested with a fixture reply and with the cache of the
Python code, not with a live reply.
*Evidence:* `rust/src/market/prices.rs` (`load_prices`), test `a_cache_file_is_used_and_a_refresh_that_fails_keeps_it`.

**142. The forward consensus and the earnings dates have no source in Rust.** `estimates.py` and
`load_earnings_surprises` used `yfinance`. Its endpoints need a cookie and a crumb, and Yahoo refuses them for a
program that gives its real name (#93). The program does not imitate a browser. So `fa analyze` reads the files that
exist (`data/raw/<T>/estimates/<date>.json` and `yf_earnings.json`, made by the earlier Python code) and says
"no source" when they do not exist. For the snapshot, today's file is used; else the latest older file is used and the
console says its date (each row of the sheet has the `snapshot_date`). Nothing makes new files of these two kinds.
**Open decision of the owner:** which free source gives estimates and earnings dates (Financial Modeling Prep
with a key, a data source with a published API, or another).
*Evidence:* `rust/src/market/consensus.rs`.

**143. The option chain is from Cboe, not Yahoo.** Four differences from the Python snapshots: (a) the implied
volatility is the value of Cboe (a fraction, 0.31); Yahoo solved its own values, so a history that has both
kinds of snapshot is not one series; (b) the spot price is `current_price` of the Cboe file (about 15 minutes
late), not the last daily close; (c) `in_the_money` is computed from that spot price (call: strike below spot, put:
strike above); (d) only the options of the root of the ticker are in the snapshot (the adjusted options, for
example `B1IBM`, are not). Cboe gives IV 0.0 to an option with no solved value; the metrics treat an IV
below 0.0001 as missing, as before. The metrics (`compute_chain_metrics`) are the same calculation as in Python
(tested on 7 chains with the expected values from the Python functions, 533 quotes). A ticker with no listed
options gets an HTTP 404 from Cboe: no snapshot file is written.
*Evidence:* `rust/src/market/options.rs`; `curl` of the IBM file 2026-11-19 (2,208 options, 20 expiries).

**144. FMP: key handling, legacy endpoints, error objects.** The key is read from `FMP_API_KEY` (environment, else
the `.env` file in the repository folder). The key is in the URL, so the program takes it out of every
error text. The endpoints are the v3 endpoints of the Python code (`api/v3/analyst-estimates`,
`api/v3/earnings-surprises`); FMP closed the v3 endpoints for new keys in August 2025, and the new program
was not tested with a live key (there is none in the container). An answer with HTTP 200 and an error object
(`{"Error Message": ...}`) is an error now. The Python code wrote that object into the cache and failed later. A
surprise record with a missing number is skipped; the Python code stopped with `TypeError`.
*Evidence:* `rust/src/market/fmp.rs`; tests with fixture text only.

**145. The xlsx file of Rust differs from the file of Python in three small ways.** Compared on 2026-11-19 for IBM and
JNJ (203,004 and 179,351 cells): no difference in a value, a sheet name, a sheet size or a number format
(all "General"). The differences: (a) a source text that starts with `=` (a registry formula such as
`= revenue - cogs_total`, 199 and 203 cells) is a text cell; openpyxl wrote a formula, and Excel shows `#NAME?`;
(b) the source text of the derived rows `free_cash_flow` and `fcf_margin` does not name the deleted file `schema.py`; (c) openpyxl
writes 16 significant digits, and Rust writes the exact value, so 16 and 13 cells differ in the last digit
(relative difference 3e-16 or less). A NaN is an empty cell in both files (the future dates of `yf_earnings.json`).
*Evidence:* `rust/src/analyze.rs`; the proof is the comparison script of the ticket `report-rust`.

**146. The company data has one list of variable names: the registry.** The Python dataclass
`PeriodFinancials` repeated the names, and `store.load` dropped each field that the dataclass did not have. The Rust
type (`rust/src/company.rs`) reads each value by the name from the registry, so a new variable cannot be lost
and the test "the field order of `schema.py` equals the registry" is not necessary in Rust. `schema.py` and
`tests/test_schema.py` are deleted (2026-11-19).
The unit text of the xlsx (`USD/share`, `shares`, `ratio`) is a fixed rule in `analyze.rs` (`unit_of`); the registry says
`USD/shares` for the EPS lines, and the xlsx keeps the old text.
*Evidence:* `rust/src/company.rs`, `rust/src/analyze.rs`.

## The HTML report and the end of the Python engines (added 2026-11-19, branch `claude/dcf-engine-rust`)

**147. The report page needs the network to draw its charts.** `fa report` writes a page that loads the
plotly.js library from `https://cdn.plot.ly/plotly-4.1.1.min.js` (a fixed version: the version that the Python
library plotly 7.1.0 inlined in the old page). The old page had the library inside the file (about 3.5 MB) and
worked offline. Now the tables and the texts show without the network, and each chart shows a note. If the CDN
removes the file, or a firewall blocks it, no chart shows. The program does not carry a copy of the library (it
is about 4.8 MB, and the repo is public). **Open decision of the owner:** an option that inlines a local copy
of the library into the page.
*Evidence:* `rust/src/report/mod.rs` (`PLOTLY_JS_URL`); the test `the_page_loads_plotly_from_the_cdn_with_a_fixed_version_...`.

**148. A plotly figure from Rust has no plotly Python template.** The Python figures carried the template
`plotly` (about 5 KB of defaults in each figure). The Rust figures carry only the values that change the picture:
`autotypenumbers: "strict"`, `automargin`, `ticks`, and the title standoff. Without `autotypenumbers` a year as
text ("2026") on a category axis is read as a number, and plotly.js writes the error `rotate(0,NaN,..)` in the
console (the waterfall chart). The bars have no thin template outline. A screenshot of the IBM page differs from the
screenshot of the Python page in 0.16% of the pixels. If you add a figure, set `type: "category"` for a text axis.
*Evidence:* `rust/src/report/figs.rs` (`chrome`); a browser run of the page with the same library gives 0 console
errors for IBM, JNJ and KO.

**149. Without `--price` the report uses the last close of the price history.** The Python script asked
`yfinance` for the live price. The Rust report reads `data/raw/<TICKER>/prices.json` (the cache file of
`fa analyze`), and asks the Yahoo chart endpoint only when there is no cache file (HTTP 429 is common: #141). An
old cache file gives an old price. The header says where the price comes from (`last close $X (data/raw/<T>/prices.json)`).
For a decision, give the price: `fa report IBM --price 228.53`.
*Evidence:* `rust/src/report/mod.rs` (`make_report`); the test `without_a_price_the_report_uses_the_last_close_and_says_so`.

**150. The path sample uses the numpy `Generator.choice` without replacement for one case only.** The path
explorer draws a seeded sample of the middle band (`default_rng(7).choice(middle, n, replace=False)`). The Rust
code (`Pcg64::choice_set`) does Floyd's algorithm with the 32-bit Lemire bounded integer; it equals numpy for 8
recorded cases. Numpy uses a tail shuffle when the population is above 10,000 and the sample is more than 1/50 of it.
The Rust code returns an error for this case and does not guess. It cannot happen now: the driver run has
10,000 paths, so the middle band is smaller than 10,000. A change of `DRIVER_SIMS` above 10,000 needs the tail shuffle.
*Evidence:* `rust/src/dcf/nprandom.rs` (`choice_set`); the test `the_path_sample_equals_numpy`.

**151. The report fixtures are the only record of the Python page.** The tests of `fa report` compare the page with
the page that the Python script made (IBM, JNJ, KO without prices and driver model, KO with a short price history and an
empty option chain). The Python script is deleted. A change of a text or a figure of the report needs new expected
files by hand: the old values are in `rust/tests/fixtures/report/`. The words that name a Python command are other words
in the Rust page; the list is `TEXT_CHANGES` in `rust/src/report/tests.rs`. The inputs of the fixtures are small
copies of the cache files (prices as `date,close`, the store with the fields that the report reads), so the tests need no
`data/` folder. ANT has no company data (its prospectus has no XBRL values, see #68; `fa import ANT` gives no data), so no ANT report exists.
*Evidence:* `rust/tests/fixtures/report/`; `rust/src/report/tests.rs`.

**152. Segment names that are only in `segment_history` come in alphabetical order.** The model file is read into a
sorted map. The Python code kept the order of the file. It matters only for a name in `segment_history` that is not a
segment of the model (none of the three models has one): the order of the legend of the segment chart.
*Evidence:* `rust/src/dcf/model.rs` (`segment_history`), `rust/src/report/figs.rs` (`segment_history_figure`).

**153. No `fa` command starts Python (changed 2026-11-19).** `fa run`, `Fa::run_captured`, `default_python`, the
variable `FA_PYTHON` and the Python error type are deleted. The viewer calls `fa report` in the same process. `FaError`
has two cases: `Data` and `Json`. The SEC contact text is in the legacy SEC client only. `fa import` and `mapcheck` need the
environment variable `SEC_USER_AGENT`; the Python function `importer._run` gives it the text of the legacy client when the
variable is not set. This is the one open step of the move to Rust.
*Evidence:* `rust/src/lib.rs`, `rust/src/serve.rs`, `financial_analysis/importer.py`.

**154. The Python tests had cases without a recorded fixture; each has a Rust test now.** Before the deletion, each Python
test case was mapped to a Rust test or ported (the lists are in the ticket `T-20261119-dcf-engine-rust`). New Rust
tests: `python_tests` in `rust/src/dcf/tests.rs` (classic engine properties, net debt and shares rules, vetted
formulas, sampling, driver run properties), `series_are_sorted_and_have_the_derived_metrics` and
`a_written_store_file_is_pretty_json_and_loads_again_in_any_case` in `rust/src/company.rs`, and the ports in
`rust/src/report/tests.rs`. Two Python functions have no Rust form: `store.compare` (a loop over `get_series`, no
caller) and the in-memory cache of `store.load` (Rust reads the file each time).

## Path engine (added 2026-11-19, branch `claude/stochastic-dcf`)

**155. The path engine is a second engine. The old engine does not change.** A model file with
`engine = "paths"` runs in `rust/src/dcf/paths.rs`. A file without this key runs in `driver.rs`, as
before. The reviewed IBM numbers ($83.84 / $151.84 / $258.03), JNJ P50 $112.64 and ANT P50 $30.97 come
from the old engine only. The command is `fa dcf drivers TICKER --model FILE`. The three old model
files are not changed. A path model of a ticker is a new file, and it is not a reviewed record.
*Evidence:* `rust/src/dcf/paths_tests.rs` (`limit_copy_of_the_old_model_with_the_same_draws_gives_the_old_percentiles`).

**156. A path model has two kinds of randomness. Do not count one dispersion two times.** A drawn
parameter (`{ mean, std }`) is one number for each path: the uncertainty about a parameter. A yearly
shock (`sigma`) is new each year. The std of a driver in the old model is the dispersion of a
10-year constant. When a model gets yearly shocks, the std of the drawn long-run number must become
smaller. A yearly shock of size `s` with low persistence adds about `s / 3` to the dispersion of the
10-year average. The IBM comparison file shows the rule.
*Evidence:* `analysis/IBM/driver_model_paths.toml` (rationale texts).

**157. Growth has a slow part and a fast part.** The slow part is the center of the process: it goes
from `start` to `target` with `half_life`. The fast part is the shock: AR(1) with `persistence` and
size `sigma`. With `persistence` not set, the shock has the persistence of the center (the one-part
form). The calibration of 996 companies (`analysis/priors/process_priors_2026-11-19.toml`) gives a
persistence of 0.14 to 0.19 and a size of 13% above $1B revenue. The half-life of the slow part is
not in the data for a company like ANT: it is a judgment. `sigma_level` makes the shock size at least
`sigma_level` times the level of the center.
*Evidence:* `rust/src/dcf/process.rs` (`ProcessSpec`), `paths_tests.rs` (`moments_of_growth_and_margin_follow_the_ar1_formulas`).

**158. `start` is the value of year 1 of the process, not the pin.** The pin of a segment
(`year1_growth`) sets the revenue of year 1 only. The first step of the process is year 2. For ANT
the pin is +198% and `start` is 60%: the growth of year 2 at the means is 0.07 + (0.60 - 0.07) x
0.707 = 44.5%. A reader who takes `start` for the growth of year 2 reads a wrong number.
*Evidence:* `analysis/ANT/driver_model_paths.toml`.

**159. The risk-free rate of the CAPM is a yield, not the short rate. The mode decides the drift.**
With `mode = "long"` the rate of year `t` is the 10-year zero yield that the model gives at the start of
year `t`. On an upward curve its expected value rises with `t` (ANT: 5.1% in year 1, about 6.1% in
year 10), so the DCF discounts later years at higher rates. The product of the yearly factors is
then not the curve discount factor. With `mode = "long_flat"` the expected yield stays at the level of
year 1 and only the deviations of the model remain: this is the convention of the old engine (one
flat rf). The ANT and IBM files use `long_flat`. For ANT, `long` gives P50 $23.23 against $25.01.
*Evidence:* `pathrate.rs`, `paths_tests.rs` (`long_flat_mode_keeps_the_expected_risk_free_rate_at_the_level_of_time_zero`).

**160. Rates follow the pricing measure of the model. There is no term-premium drift.** Hull-White
reprices the curve of the date. The ERP is added to this rate. The parameters `a = 0.14` and
`sigma = 0.0075` come from `fa models calibrate --model vasicek --method ml` on the GSW 1-year zero
yield, monthly, 2000 to 2026-11. The mean reversion is not significant (t = 1.78 against 2.86): `a`
is weakly known. The 1-year zero yield is a proxy of the short rate.
*Evidence:* `fa models calibrate` on 2026-11-19.

**161. Terminal growth has three caps, in this order.** (1) The `target` of the process in
`terminal.cap_by_process` (growth after the horizon does not pass the long-run growth of the revenue).
(2) The risk-free rate of the path at the end plus `rf_margin`. (3) The guard of the old engine:
terminal cost of capital minus 2% (T-DCF-2). The output shows how often each cap binds. After the
horizon the growth falls from the last yearly value to the terminal growth in one step. The
terminal value uses the cost of capital of the end of the horizon on each path.
*Evidence:* `paths.rs` (`eval`), `paths_tests.rs` (`terminal_growth_caps_are_counted`).

**162. Option B (sales-to-capital) has four simplifications.** The reinvestment is
`max(revenue change, 0) / s2c`: a revenue fall releases no capital. The loss pool is used in the
years of the horizon only; a rest at the end gives no value. In the terminal value the tax is
the full rate. If the last EBIT is negative the terminal value is negative (as in the old engine).
Option A (the margin is the FCFF margin) is the option of both application files.
*Evidence:* `paths_tests.rs` (`fixed_answer_option_b_with_loss_carry_forward_literal`).

**163. The failure hazard is off by default.** With `[failure]` each path fails with the yearly
hazard. The company is sold at `recovery_revenue_multiple` times the revenue of the year before, and
the cash flow of the failure year is lost. Net cash stays in the equity value (the bridge does not
change). `mode = "expected"` uses survival weights and has no noise. Neither ANT file uses it. A
failure of ANT with 230 billion of cash is a judgment that no part of the repo supports.
*Evidence:* `paths_tests.rs` (`failure_expected_mode_equals_the_closed_form`).

**164. The path engine has its own random numbers.** Path `p` uses the stream
`Rng::for_path(seed, p)` of `ratemodels/rng.rs`. The result is the same with 1, 3 or 4 threads. It
is not the numpy stream: a path model does not give the same numbers as the old engine for the same
seed. A test can give the correlated normals of the old engine (`simulate(.., Some(z), ..)`); then
the values of all 10,000 IBM paths agree with the old engine to 3e-15 (ANT: 6e-14). Sensitivities,
attribution and implied values use the same seed, so they use the same normals.
*Evidence:* `paths_tests.rs` (`a_run_gives_the_same_numbers_with_one_and_four_threads_and_for_each_seed`).

**165. Draws are cut, not truncated.** A drawn parameter is `mean + std z`, then cut at `lo` and `hi`
(as in the old engine). The value of a process in a year is cut at its `lo` and `hi`; the cut does not
feed back into the process. Edges get probability mass. `--pset NAME=V` changes the mean only; add
`--shift-bounds` to move `lo` and `hi` with it (needed for implied values, as in #125). A drawn
half-life needs `lo > 0`.

**166. Attribution and variance shares have a stated meaning.** Attribution moves the mean of one
drawn number by one std, with the same normals; the bounds do not move. The variance table has two
columns for each group: the variance when only this group is random, and the share that goes when
this group is fixed. The columns do not add to 100% (interaction).

**167. The calibrated margin shock is large.** The table gives a shock of 11% (persistence 0.44) for
the free-cash-flow margin of all companies. It is the size that the ANT file uses. It widens the P10 to
P90 range of ANT (P10 $3.60 against $12.82). For IBM the file uses 2%, a judgment, because the
pooled value is too wide for a company with a margin of 19% that moves slowly. The pooled value is a
mean over many company types, and ANT is outside the data (37 company-years with growth above 100% at
revenue above $10B).

**168. The curve is in the model file.** The six GSW numbers of one date are in `[rates.curve]`, so a
run does not need `data/`. A new date needs a new file. The Board can revise the file. The curve of
2026-07-16 (IBM) has large `beta2` and `beta3` (a fit with near-collinear terms): the yields are
right, the parameters are not readable. The horizon plus the tenor must be 30 years or less.

**169. Test tolerances.** Fixed answers: 1e-12 relative (1e-6 for the Hull-White curve case with
volatility 1e-9, where the curve yield is the reference). Same draws as the old engine: 1e-10 per
path. Moments: 4 standard errors (variance: 4 x var x sqrt(2/n)). Discount factors: 3 standard
errors of the mean plus 2e-5 for the monthly trapezoid (the 400,000-path run has |z| below 1). Failure
share and mean: 3 standard errors.
*Evidence:* `rust/src/dcf/paths_tests.rs`.

## Calibration of the growth and margin processes (added 2026-11-19, branch `claude/dcf-calibration`)

**170. The panel has no sector.** The company facts and the cache files have no SIC code. A group is a size bucket and a
template (`standard`, `bank`, `insurer`, `reit`, `fund`). The `standard` template mixes all industries: an oil company, a
software company and a retailer have one set of parameters. The argument `sector` of `priors_for` is not used.
*Evidence:* `rust/src/dcf/calibrate.rs` (`priors_for`), `rust/src/dcf/calib_report.rs` (`render_panel`).

**171. A restated year has the value of the latest filing, and two years can come from two filings.** The mapping takes, for each
period, the fact of the latest filing. The revenue of year t and the revenue of year t-1 can come from two different filings. A
recast (a sold business moved to discontinued operations) then shows as growth. The panel does not count recast years. The jump
rule catches the large ones only.
*Evidence:* `rust/src/dcf/panel.rs` (rule 1), `rust/mapcheck/src/facts.rs`.

**172. A 53-week year stays in the panel.** A fiscal year of 371 days has about 1.9% more revenue than a year of 364 days. The
panel counts these years (`long_years`) and does not change the revenue. The effect is small against a shock of 12% to 18%.
*Evidence:* `rust/src/dcf/panel.rs` (rule 3).

**173. The jump rule removes real growth and keeps some mergers.** A growth above 60% for a company with $1B or more of revenue
is a jump, and a jump is not in the AR(1) sample. NVIDIA (126% in the year to January 2024) is a jump. A merger paid in shares
(Cigna and Express Scripts, 2019) is a jump with no cash for businesses, so it is not `acq_likely` and stays in the fast-grower
tables. The rule `glitch` (two jumps that undo each other) removes a wrong value of one year (an example is PG in 2015) but not
a wrong value that stays.
*Evidence:* `rust/src/dcf/panel.rs` (rules 4 and 5), the output of `fa dcf calibrate` (list of very large fast growers).

**174. The size bucket is the revenue of the year before, and this selects.** A company is in a high bucket partly because the
year before was a peak (a commodity price spike, a one-time sale). The growth after the peak is low. So the growth mean of the
large buckets is low (3% for `30-100B`, 9% for all) and the growth mean of the small buckets is high. The result is a mean
reversion of the level that is not in the process.
*Evidence:* the table, cells `30-100B` and `<1B` (`mean`).

**175. The within estimate of `phi` is biased on a short panel, and the correction is approximate.** With about 12 years for each
company, the within estimate is too low by about 0.1 to 0.2 (Nickell bias). The correction solves the first order formula with the
number of observations of each company. It assumes shocks that are independent, a constant company mean and a start in the steady
state. The Anderson-Hsiao value is a check. The two agree to about 0.1 for growth (0.11 and 0.16) and differ more for margins in
the largest bucket (standard errors of 0.13 to 0.3). Use the standard errors.
*Evidence:* `rust/src/dcf/calibrate.rs` (`nickell_bias`, `fit_ar1`), the tests with known values.

**176. `phi` of growth is the speed of the return to the company mean, not the fade of a growth rate.** The within model has one
constant mean for each company. For growth, `phi` = 0.11 to 0.26 (half-life 0.3 to 0.5 years): a growth that is above the company
mean falls back in one year. The company mean itself changes slowly, and a new company has no known mean. The pooled `phi`
(0.27) mixes the two effects. The data show, for companies with growth above 50%, that the median growth falls to 13% to 30%
(by size) in one year and to 3% to 16% in three years, and that after five to ten years it is usually above the mean of the bucket. One AR(1) with one
mean does not give all of this. A process with a fast part (`phi` about 0.2) and a slow company mean is a better form.
*Evidence:* the fade tables of `fa dcf calibrate` (column `imp.phi`).

**177. The size of the shock depends on the level.** The table has one `sigma` for each bucket. After a year with growth above 50%,
the growth of the next year has a P10 to P90 range of -18% to +89%. An AR(1) with `sigma` = 18% gives +2% to +51%. The bands of
fast growers are too narrow.
*Evidence:* the fade tables of `fa dcf calibrate` (columns P10, P90, AR1 P10, AR1 P90).

**178. The growth less the year mean is not growth less GDP.** The repo cache has no GDP series. The column `growth_excess` subtracts
the mean growth of all companies of the same fiscal year. This mean has the company itself in it and depends on the companies
in the cache. The fiscal year is the calendar year of the middle of the fiscal year.
*Evidence:* `rust/src/dcf/calibrate.rs` (`year_means`).

**179. Margins outside -100% to +100% are not used, and the margin of a bank or an insurer is a different thing.** 309 of 11,151
operating margins and 353 of 13,405 free cash flow margins are outside. The free cash flow margin of a bank (27%) is not
comparable with the margin of an industrial company. The persistence of margins includes slow changes of the company mean, so the
half-life is too long for a pure shock.
*Evidence:* `rust/src/dcf/calibrate.rs` (`limited_margin`), the table.

**180. The shock correlation is between the shocks of the same year.** It uses the residuals of two within models. It does not
measure a lag between growth and margin. The standard error comes from a bootstrap over companies (200 resamples, fixed seed).
The correlation of growth with the free cash flow margin is about 0 and the correlation with the operating margin is +0.25. The
operating margin has fixed costs: a growth shock moves the margin.
*Evidence:* `rust/src/dcf/calibrate.rs` (`shock_correlation`).

**181. Survivorship bias.** The cache has companies that exist now (the S&P 500 and the next 500 of the SEC ticker list). Failed and
acquired companies are missing, and the index adds a company after its rise. Direction: the growth mean is too high, the shock
size and the lower tail are too low, the persistence of margins is too high, and the fade of fast growers is too slow (a company
that fell hard left the sample). The bias is not measured.
*Evidence:* the text `SURVIVORSHIP` in the output of `fa dcf calibrate`.

**182. A company like ANT (revenue about $35B, growth above 100%) is outside the data.** Of 141 company-years with a growth
above 50% and a revenue of $10B or more (standard template), many are a commodity price move, a merger, a change of the
business (an asset manager, for example), or a data error (the list in the output shows them; the sector is not known, so the share is
not counted). The cases of lasting organic growth are very few: NVIDIA (61%, 126%, 114% and 65% in the years to
January 2022, 2024, 2025 and 2026), Apple (fiscal 2010 and 2011), Tesla (2017, 2018, 2021 and 2022). For these companies the one-year `phi` is far above the pooled value (NVIDIA: about
0.9). An AR(1) with the table values predicts 20% to 28% growth after a year of 100%. The table does not support a value for ANT.
*Evidence:* the section `VERY LARGE FAST GROWERS` of the output.

**183. The band test has three limits.** (1) The legacy band is calculated exactly (growth is a normal draw, revenue is a monotone
function), not by simulation. (2) The AR(1) bands use the same 2,000 normal draws for all companies. (3) The test in sample uses
parameters that include the target years; the test out of sample uses the years up to 2015 for the parameters and the base years
from 2015. The actual value is the value as now reported, so a recast is in it. A base year needs 5 consecutive fiscal years
before it and 3 after it, so the companies with a short history, and the companies that left, are not in the test.
*Evidence:* `rust/src/dcf/calib_bands.rs`.

**184. `priors_for` falls back to a wider cell.** A cell needs 150 growth observations and 25 companies. A cell of a bank in the size
`>100B` has none, so the call returns the cell of the size for all templates. The result says which cell it is (`size`,
`template`). A cell with few companies has a large standard error (`phi_se`): a person must read it.
*Evidence:* `rust/src/dcf/calibrate.rs` (`priors_for`, `MIN_OBS`, `MIN_UNITS`).

## The automatic model (added 2026-11-19, branch `claude/dcf-auto`)

Ticket `T-20261119-dcf-auto-model`. Code: `rust/src/dcf/auto.rs` (the builder), `auto_cli.rs` (`fa dcf auto`, `fa dcf screen`),
`auto_pit.rs` (`fa dcf pit`). An automatic model is a screen. It is not a reviewed valuation (`CLAUDE.md`, "Review of a valuation
report"). Do not use it alone for a trade.

**185. Each number of an automatic model is a rule, not a judgment of a person.** The rules are in the module text of `auto.rs`
and in `docs/ARCHITECTURE.md`. `fa dcf auto TICKER --explain` prints each number with its source and rule. The hand models of IBM and
JNJ have judgments that no rule gives: segments, a pension bridge, a charge for acquisitions, guidance. The automatic model gives the
hand number for none of them. A large difference to a hand model is not an error of the automatic model by itself. The largest one is the cost of acquisitions: the
hand models have a charge for them (IBM 3.55% of revenue, JNJ about 5.8%), and the automatic model has none (the free cash flow of the mapping has no acquisitions, rule R2 of `PLANNING/RULES.md`), but the
growth history keeps the bought growth of the years below the jump limit of 60%. Run of 2026-11-19, value for each share, P10 / P50 / P90: IBM automatic 21.53 / 140.67 / 389.94, IBM version 4 83.84 / 151.84 / 258.03,
IBM path comparison file 79.17 / 154.03 / 277.20; JNJ automatic 56.44 / 151.35 / 342.42, JNJ first pass 69.99 / 112.64 / 175.38. The bands are 2 to 3 times wider than the hand bands because the shocks are those of the
table (and of the company history), not the judgments of a review.
*Evidence:* `fa dcf auto IBM --explain`, `fa dcf auto JNJ --explain`.

**186. The growth history keeps a jump that has no cash for businesses behind it.** The calibration table was estimated with every
jump above 60% (revenue of the year before $1B or more) removed. If the model history removed them too, NVIDIA (growth of 126%, 114% and
65% in three years) would have an empty history and a start growth of 2%. The model history removes only a jump with cash paid for
businesses of at least 10% of the revenue change, and a jump that reverses (`growth_organic` of `panel.rs`). The table `phi` was not
estimated on such jumps. For a company like NVIDIA the table does not describe the persistence of the growth (#182). The rule `history_organic`
turns this off; the test with the jumps removed has 69% to 74% coverage for the companies with growth above 50% against 84% with the rule.
*Evidence:* `fa dcf pit --rules history_organic=0`.

**187. The company mean is a shrunk mean, with the weight of the calibration (`calib_bands`).** The weight of the history is
`tau^2 / (tau^2 + sigma^2 mvf(phi, n))` with `tau`, `sigma` and `phi` of the table cell of the size bucket. The variance after the shrink
becomes the standard deviation of the drawn long-run growth, and the year-1 value has the correlation with it that the AR(1) gives. The
history has the last 4 growth values (5 margin values); a longer history is not used. A cell with `tau = 0` gives the cell mean.
*Evidence:* `ar1_fit` and its test (hand calculation).

**188. The shock size of a company is not the table value.** (1) The table value is the average of all companies in the cell. A company with a
stable margin has a smaller shock, a cyclical company a larger one: with the table value the free cash flow bands had 97% coverage for
the companies with a margin range below 5 points and 75% for those above 10 points (target 80%). The shock size is now
`sqrt((4 sigma_table^2 + sum of residual^2) / (4 + number of residuals))` (weight of the table: 4 years; `own_sigma_nu`). (2) A company in a
high-growth state has a larger spread of the next growth than the average company: the fade table has P10 to P90 of the growth one year after
a growth above 50% of -19% to +89% (all sizes). The shock of the growth is at least `kappa` times the level of the growth, with `kappa`
the median over the thresholds of the spread over the start growth in the fade table (`level_spread_ratio`: 0.5 now). Before the two rules the
revenue coverage for companies with growth above 50% was 48%, 44% and 51% (T+1 to T+3); after them 84%, 82% and 84%. The rules widen the
bands for all companies: the overall coverage is 86%, 84% and 87% against the target of 80%.
*Evidence:* `fa dcf pit --rules own_sigma_nu=0,level_kappa=0` (before) and without `--rules` (after).

**189. The slow fade to the long-run cap is a linear ramp, not the exponential fade of the fade table.** The engine has no second
process, so the fade is a drift that is added to the growth: linear from 0 in year 1 to `(cap - company mean) * 9 / (N - 1)` in year 10, with `N`
chosen so that the value in year 10 equals that of an exponential fade with the half-life of the fade table (about 3.7 years: the median over the
thresholds of the fade from year 3 to year 10, `slow_half_life`). The cap is the 10-year zero yield of the GSW curve (5.13% on 2026-11-13); the rate is
a proxy for the long-run nominal growth of the economy. The fade applies only if the company mean is above the cap. The fade table was
made with the companies that survive: the real fade is faster (#181). The ramp changes the revenue coverage by less than 1 point at T+3 in the
test; it matters for the value, which the test does not measure.
*Evidence:* `fa dcf pit --rules fade_to_cap=0`.

**190. The margin of the model is the free cash flow before SBC and before interest.** `(operating cash flow - capex_total + (1 - t) interest paid) / revenue`.
Operating cash flow already adds back the SBC: a formula with "plus SBC" would count it twice. The SBC is a cost of the model (the ratio is a separate process), as in the IBM
and ANT models. The table margin series is the cash flow after interest. So the history is shrunk in the space of the table and the company mean of the after-tax interest
ratio (last 3 years) is added to the start and to the target. The interest is `interest_paid`, else `interest_expense`; a company with neither has a ratio of 0.
*Evidence:* `rust/src/company.rs` (`free_cash_flow`), `YearData::fcf_margin`.

**191. The phi of a margin cell with a large standard error is replaced.** The cells of the margin for the size buckets 30-100B and >100B have 127 and 38
companies and a standard error of `phi` of 0.139 and 0.134 (the cell of all sizes: 0.027). The model takes the `phi` of the cell of all sizes when the standard error is above
0.08 (`margin_pool_se`); the mean, `tau` and `sigma` of the size cell stay. The effect on the coverage in the test is below 1 point.
*Evidence:* `fa dcf pit --rules margin_pool_se=0`.

**192. The SBC ratio is a constant.** It is the mean of the last 3 years, with no shock. The table has no SBC series. A company with no `sbc_total` in the 3 years has a ratio of 0 and a warning.

**193. The net debt bridge is simple.** `total_debt` less `cash_and_short_term_investments`, from the latest balance sheet (a 10-Q if it is newer than the 10-K). It
leaves out pensions, operating leases, minority interests, litigation reserves and investments. The output has a memo line that shows the
size of the three items that are in the mapping (per share). The variable `total_debt` has two estimates for many companies (check `total_debt_estimates`); the model takes the value of
the mapping and prints the warning. For IBM the debt includes the debt of the financing business.
*Evidence:* `fa dcf auto IBM --explain` (the line `net debt` and the memo line).

**194. The balance sheet and the flows can have different dates.** Revenue, margin and SBC come from the last fiscal year. Net debt and the share count come from
the latest 10-Q if it is newer. The trailing four quarters are not used (the quarters of the mapping are single quarters; the fourth quarter is not in the 10-Q files). For a fast
grower the revenue base is 3 to 12 months old. The point-in-time test uses fiscal years only.

**195. The share count is the weighted average of the diluted shares of the latest period,** not the count at the end of the period. A company with large buybacks or an issue of
shares in the period has a different count. The price per share of the output follows this count.

**196. Beta and the equity weight need prices that the cache mostly does not have.** The cache has price files for two tickers (IBM and JNJ) and the SPY file for the index. Other tickers have
beta 1.0 and an equity weight of 1.0 (the cost of equity for the cash flow of the whole company): for a company with much debt the value is too low. The output has a warning. The regression
beta uses 5 years of weekly returns of the stock (adjusted close) against SPY (the close of the trading cache) and the weight 2/3 (the rest is 1.0). The standard error of such a beta is about 0.1.

**197. Only the template `standard` has a model.** A bank, an insurer, a REIT, a fund, a foreign filer, a company with no revenue and a new registrant get the text "no model for this template". A free-cash-flow DCF is the wrong
tool for them. The next step is a model for each template (excess return for a bank, FFO for a REIT). A mapping status of FAIL, or a necessary variable with no value
(operating cash flow, capex, cash, diluted shares), also gives no value. The status WARN runs and prints the warnings that touch a model input.

**198. The price-implied values are one-number searches.** The long-run growth (the level that the growth fades to), the target margin and the half-life of the fade each move one number of the model and
keep the others (bisection on the P50 with the same random numbers). The half-life is found from the whole number of years of the ramp (a step of the P50), so it has a resolution of one ramp year.
The implied values depend on the other numbers of the model: they are not a statement about the company.

**199. The point-in-time test has five limits.** (1) The table was estimated on all years: the result "in sample" is not out of sample. With the table from the years up to 2015 (base years 2015 and later) the revenue
coverage is 85%, 83% and 87% (T+1 to T+3), and the median real revenue is 0.9%, 3.5% and 6.2% above the P50: the growth after 2015 was above the growth before 2016. (2) The mapping takes the latest filing of each year, so a
restatement enters the data of the base year. (3) The companies exist now (survivorship, #181). (4) The model of the test has no price, a fixed rate and no failure; revenue and cash flow do not depend on them. (5) The value (the P50) is not tested: a test against
later stock returns needs prices, and the cache has two price files. A model with a systematic error that the test shows and that has no rule yet: companies with a decline in the base year (the real revenue is 8% to 11% above the P50 at T+2 and T+3, the real value is above the P90 in 15% to 19% of the cases);
and the free cash flow at T+3 (the real value is 1.6% of revenue above the P50, 60% of the real values above the P50).
*Evidence:* `fa dcf pit` and `fa dcf pit --cut 2015`; the result files in `analysis/priors/`.


## Could not verify

- (none of the named traps failed verification — all located in current code/docs; the
  scenarios-registry item was re-checked and is still true as stated in #13.)
- Entries 21–31 were verified by the parallel PR #11 agent on the same day, not
  independently re-verified by this registry's author; their evidence pointers stand.
