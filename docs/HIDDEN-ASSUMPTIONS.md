---
id: DOC-HIDDEN-ASSUMPTIONS
type: description
item: 10.15 Design description
status: current
date: 2026-10-25
scope: Registry of non-obvious assumptions and traps in the pipeline and IBM model; entry numbers are cited elsewhere and stay stable.
---
# Trap registry

Each entry has an evidence pointer (symbols, not line numbers). Add new traps with a
pointer. Claims about IBM.json contents (entries 2, 3, 4, 6, 10, 21) need
`python -m scripts.import_financials IBM` to check.

## Data layer

**1. Recast-financials landmine (IBM FY2019–20).** The store mixes continuing-ops
(recast) revenue with unrecast consolidated OCF across the Kyndryl divestiture boundary,
so the FY2020 "28.2% margin" is spurious and **margin windows longer than 5 years
are unusable** for IBM until the importer aligns recasts. The backtest surfaces (never
fixes) this via `DATA_FLAGS`; every realized IBM vintage's data window touches a flagged year.
Evidence: `analysis/IBM/README.md` §Data notes; `backtest.DATA_FLAGS`;
`analysis/IBM/REVIEW.md` F13; `git log -- financial_analysis/backtest.py`.

**2. Phantom "10-K" rows.** The importer keys periods off *any* mapped concept's period
end, so stray XBRL facts create annual rows with 1–2 fields and no revenue/OCF (IBM:
2012-06-30, 2018-01-01, 2019-01-01; placeholder `filed_date`, see #21). They carry no FCF and are skipped by the `fcf_margin is not None` filter, but
`derive_fcf_margin` slices the last five annual rows before that filter (dcf.py:49-52), so a
phantom row inside that window shrinks it. Any code iterating `company.periods` naively
will hit them.
`backtest._clean_annuals` filters them by requiring revenue AND OCF. Evidence:
`financial_analysis/importer.py` (`all_periods[key]` row creation);
`backtest._clean_annuals`; `tests/test_backtest.py`;
`analysis/IBM/README.md` §Data notes.

**3. `n_periods=8` is per-concept, not per-company.** `extract_series` returns the last
8 entries *per concept*; the union of period-ends across ~60 concepts yields far more
than 8 periods (IBM: 22 annual rows), with older rows only partially populated (e.g.
IBM buybacks populated FY2012–19 while revenue is None before FY2018). Do not treat
older rows as complete statements. Evidence: `financial_analysis/importer.py` (`_resolve_field`, `n_periods`);
`financial_analysis/xbrl_api.py` (`_deduplicate(filtered)[-n:]`).

**4. Filed-date visibility lag.** `filed_date` is the filing the figure was drawn from,
usually a later 10-K's comparative column (dedup keeps the latest-filed entry per end
date; IBM FY2019 carries 2022-02-22, FY2023 2026-02-24). The backtest filters
`filed_date <= as_of`, so vintages are conservative, never optimistic. Do not read it as
when a figure became public. Evidence:
`financial_analysis/backtest.py` (module docstring, caveat #1);
`financial_analysis/xbrl_api.py` (`_deduplicate`); `financial_analysis/importer.py`
(`_populate_filed_dates`).

**5. Segment data is hand-entered; companyfacts has no segment axis.** The SEC
companyfacts API strips dimensional facts (verified across every us-gaap concept in
IBM's payload: entries carry exactly `{accn, end, filed, form, fp, frame, fy, start,
val}`), so `PeriodFinancials.segment_revenue` is always None from that source. IBM's
per-segment revenue in `analysis/IBM/driver_model.py` (`SEGMENT_HISTORY`) is
hand-entered from the Jan 28, 2026 Q4 8-K ("Other" $63M dropped, 0.1% of revenue).
Filing-level instance-document extraction is Ticket 7. Evidence:
`financial_analysis/importer.py` (module docstring); `PLANNING/tickets.md` Ticket 7;
`financial_analysis/schema.py` (`segment_revenue`).

**6. The JSON store holds statements only — no prices, options, or estimates.**
`store.save()` writes `ticker`, `cik`, `company_name` and `periods` only; market data lives
in the xlsx and in `data/raw/TICKER/` (dated `options/` and `estimates/` snapshots;
`prices.json` cache). The report's price/dividend/analyst-target figures were **hand-entered** from
cited sources, not pipeline outputs (disclosed). Evidence:
`financial_analysis/store.py` (`save`); `analysis/IBM/README.md` §Data notes;
`analysis/IBM/report/SOURCE-AUDIT.md` row 43.

**7. Capex sign convention.** The importer stores capex sign-flipped (negative);
`free_cash_flow = OCF − abs(capex)`. Dropping abs() overstates FCF by 2×capex. Evidence: `financial_analysis/schema.py` (`free_cash_flow`);
`financial_analysis/xbrl_tags.py` (`capex`, `sign_flip=True`); `context.json`
`fcf_sign_note`; `PLANNING/tickets-closed.md` Ticket 5 results.

**8. `dividends_paid` third fallback is dividends DECLARED, not paid.**
`us-gaap/DividendsCommonStockCash` is an equity charge used as a last-resort proxy when
neither payments concept exists for a period (first-wins merging). Fine for steady
payers; wrong for irregular ones. Evidence: `financial_analysis/xbrl_tags.py` (`dividends_paid`
precedence comment).

**9. yfinance surfaces are current-only; history accumulates from first pull.** Option
chains and forward consensus estimates have no free historical endpoint — each pull is
persisted as a dated snapshot (one per calendar day; same-day re-run reads disk,
`--refresh` overwrites); empty snapshots are saved so absence is dated evidence.
Evidence: `financial_analysis/options.py`, `financial_analysis/estimates.py` (module
docstrings).

**21. `filed_date` can be outright fabricated (placeholder = period_end).** The importer
initializes `filed_date` to the period end and only overwrites it where a *revenue*
concept fact matched that (end, form). Phantom rows (#2) and sparse pre-2018 rows keep the
placeholder, so their `filed_date` equals `period_end`. Never trust
`filed_date` on a row without revenue.
Evidence: `importer.py` (`"filed_date": end,  # placeholder`) and
`_populate_filed_dates` (revenue concepts only); IBM.json phantom rows show
filed_date == period_end.

## Model layer (IBM v4 / driver engine)

**10. Buybacks are genuinely absent from the model — not an omission bug.** IBM's
share count is held FLAT (948,675,228 diluted) because SBC is charged as a full cash
cost — also growing the count would double-charge dilution (REVIEW F5). The store's
null `repurchases_common_stock` for FY2020+ mirrors reality (IBM's buybacks stopped;
the mapping itself works, see #3) and the source audit
verified the nulls. Capital-return arithmetic in the analysis treats dividends + M&A as
the (debt-funded) return channel. Evidence: `analysis/IBM/driver_model.py` (`SHARES ... held flat (SBC charged in margin)`);
`analysis/IBM/REVIEW.md` F5, F13, F14; `analysis/IBM/report/IBM-valuation-2026-07.md` §3.1;
`analysis/IBM/report/SOURCE-AUDIT.md` row 43.

**11. f-conditionality: every headline number is conditional on f = 0.50.** The M&A
recurrence judgment `f` charges f × M&A spend in the margin AND credits f × acquired
growth in `g_software`'s mean — a paired judgment with a single source of truth
(`MA_RECURRENCE`, guarded by a module-level assert so the two legs cannot drift apart).
Changing one leg without the other silently breaks pairing coherence. The f-locus
(0→1, growth credit moving WITH f) is published by `dcf_drivers_run.py`; f is a
judgment, not a fitted parameter. Evidence: `analysis/IBM/driver_model.py`
(`MA_RECURRENCE`, `G_SOFTWARE_MEAN`, the `ma_recurrence` driver, the module-level assert);
`analysis/IBM/DRIVER-REVIEW.md` ma_recurrence row.

**12. "Tails are decorative."** Each path holds one growth draw for all ten years (no
mean reversion — ticket T-DCF-1, PLANNING/tickets.md, the largest known distortion; far-year dispersion widens), so the left tail is
understated even after the v4 correlation matrix fattened it. Standing wording: the
tail caveat in `analysis/IBM/report/IBM-valuation-2026-07.md` §4 (verbatim there, per REVIEW F6).
Do not quote P10/P90 as decision-grade. Evidence:
`analysis/IBM/report/IBM-valuation-2026-07.md` (tail-caveat callout);
`analysis/IBM/REVIEW.md` (T-DCF-1: "Until then, P10/P90 are decorative");
`financial_analysis/driver_dcf.py` (module docstring, disclosed limitation).

**13. The scenarios registry carries IBM v3 values; the engine is v4 — deliberate.**
`financial_analysis/scenarios.py` (`_ibm_v3`, `_SCENARIO_OVERRIDES["IBM"]`,
`CAPITAL_OVERRIDES["IBM"]`) encodes the v3 aggregate framework (g 2.15%,
WACC 7.5%, margin 14.5%; the v3 record does not reproduce: #24). The v4 headline is recorded in analysis/IBM/report/IBM-valuation-2026-07.md §4 and computed only by
`analysis/IBM/driver_model.py` + `financial_analysis/driver_dcf.py`. The visualizer runs both, in separate labeled sections ("Aggregate DCF scenarios" vs "Driver-based
model"). Net debt ($54.0B) agrees across both; shares agree (#10). If you read
`scenarios.py` for "the IBM valuation," you get v3. Evidence: files cited;
`scripts/visualize_dcf.py` (`SECTION_TITLES`, `_driver_compare_figure`).

**14. Truncation is clip-after-correlation.** `truncnorm` drivers are clipped to
[lo, hi] AFTER the correlated transform — not a true truncated normal: mass piles on the
bounds and realized correlations distort. Second-order for mild truncations; check no
bound sits within ~2σ of its mean (bound masses: analysis/IBM/DRIVERS.md driver table).
Evidence: `financial_analysis/driver_dcf.py` (module docstring).

**15. The two engines guard degenerate terminal values differently.** Classic engine
(docs/ARCHITECTURE.md §1):
`np.clip(wacc − tg, 0.001, None)` — near-degenerate paths CAN dominate its tails (F12).
Driver engine: joint guard, pinned fraction reported (docs/ARCHITECTURE.md, Driver-based
engine). Do not assume classic-engine tail output has the driver engine's protections.
Evidence: `financial_analysis/dcf.py` (terminal-value clip);
`driver_dcf.effective_terminal_growth`; `analysis/IBM/REVIEW.md` F12/T-DCF-2.

**16. Classic-engine minimum-history guard.** `StochasticDCF` raises unless ≥ 4 annual
periods have positive FCF (blocks post-IPO/loss-makers); the backtest requires ≥ 3
usable periods per vintage and records skips. Evidence:
`financial_analysis/dcf.py` (`_MIN_POSITIVE_FCF_PERIODS`);
`financial_analysis/backtest.py` (`_MIN_USABLE_PERIODS`).

**20. Default scenario presets are generic mature-large-cap.** `DEFAULT_SCENARIOS`
(bear 1%/base 4%/bull 6% growth) applies to ANY ticker without an override — running
`visualize_dcf` on a hypergrowth name silently produces mature-large-cap scenario
labels and assumptions. Net-debt derivation (docs/ARCHITECTURE.md, `scenarios.py` row)
treats absent short-term borrowings as 0. Evidence:
`financial_analysis/scenarios.py` (`DEFAULT_SCENARIOS`, `capital_structure_for`).

**22. The correlation matrix is reviewed *judgment*, and it moves tails only.** All 28
entries (including the 3 deliberate zeros) are argued assumptions (rationales in
DRIVERS.md / driver_model.py), PSD-verified (see analysis/IBM/DRIVERS.md, Correlation matrix). The supply-shock
regime flip (rf·g = −0.15) moves P50 by $0.05 ($151.89 vs $151.84) and
widens tails. Do not treat correlation entries as estimated from data — none are.
Evidence: `driver_model.py` CORR; dcf_drivers_run.py disclosure runs;
`analysis/IBM/report/SOURCE-AUDIT.md` row 32.

**23. RPO evidence lapsed at the Q2-2026 10-Q (late July 2026).** Rationale footnotes in
`driver_model.py` and the RPO rows of `research-ledger.md` are stale until re-consumed.
The +12.7% YoY print consolidates HashiCorp and
is NOT organic evidence.
Evidence: `analysis/IBM/research-ledger.md`; DRIVER-REVIEW.md RPO re-look; rule R1 in
PLANNING/RULES.md.

**24. The published v3 numbers do not reproduce from the current store.**
`analysis/IBM/dcf_scenarios.py` derives FCFF margins live from `IBM.json`, so its output
drifts with store re-imports and does not match the signed-off v3 record ($37 / $141 / $232,
`analysis/IBM/README.md`). Cite the record for v3 and the engine for current values; do
not "correct" either to match the other. The v4 driver run does not drift: its inputs are
pinned constants.
Evidence: `dcf_scenarios.py` `fcff_v3_margins` reads the store.

**25. Default capital-structure derivation ≠ the reviewed IBM bridge.** For tickers
without a `CAPITAL_OVERRIDES` entry, the derived net debt (docs/ARCHITECTURE.md, `scenarios.py`
row) has **no pension, no marketable securities, no stock-vs-flow review**. The IBM
override embeds the F15-reviewed bridge (debt + after-tax pension − cash&securities);
the DUOL override includes HTM securities + LT investments on the cash side. A
cross-ticker comparison (the ORCL/basket runs; basket amendment in PLANNING/reusable-dcf-vision.md) that mixes overridden and derived capital
structures compares different net-debt definitions.
Evidence: `scenarios.py` `capital_structure_for` vs `CAPITAL_OVERRIDES` entries +
analysis/IBM/README.md bridge table.

**26-27. Retired.** Numbers are not reused.

## Pipeline/repo layer

**17. The xlsx always has nine sheets; the console tab count lies by omission.**
`write_xlsx` creates every sheet regardless of flags; `--no-market` etc. just leave
header-only sheets, while the "Done: ... across N tabs" message counts populated groups.
Evidence: `scripts/analyze.py` (`_flatten` tab dict, `write_xlsx`).

**18. `data/` outputs are gitignored and untracked.** Commit rule: CLAUDE.md §Orchestration
norms. Evidence: `git ls-files data/`; `.gitignore`.

**19. RPO rows hide on the Balance Sheet tab.** `remaining_performance_obligation` is a
disclosure fact but is written to the Balance Sheet tab with `statement="disclosure"`
(no dedicated tab). Its current/noncurrent duration split is text-only in filings and
is not extracted. Evidence: `scripts/analyze.py` (`_TAB_FOR`);
`financial_analysis/xbrl_tags.py` (`DISCLOSURE_TAGS`).

**28. Background tasks can be killed mid-run.** Persist resume state in
`PLANNING/handoff/` as you go; delete items as completed. Interrupted fan-outs resume,
never restart (vision ✦6b).
Evidence: `PLANNING/handoff/`; reusable-dcf-vision.md ✦6b.

**29. Done work may sit on another branch.** Check `git log --all --oneline -- <path>`
before concluding something is missing. Branch rules: CLAUDE.md §Orchestration norms.
Evidence: `git log --all --oneline -- <path>`.

**30. LF/CRLF warnings are normal; PowerShell 5.1 quirks apply.** `core.autocrlf` may
be true. The primary shell (the owner's local Windows setup) is Windows PowerShell 5.1:
no `&&`/`||` chaining, no ternary, UTF-16 default file encoding (`-Encoding utf8` when writing files other tools read), quirky
native-arg quoting (`--%` stop-parsing token when needed). Git Bash is available for
POSIX scripts.
Evidence: `git config core.autocrlf`; `$PSVersionTable.PSVersion`.

**31. Tests are no-network by construction — keep them that way.** The whole suite runs
on synthetic fixtures (mocked HTTP, synthetic yfinance/consensus/company objects), seeds
are fixed wherever runs are compared or published (`SCENARIO_SEED`, dcf_drivers_run `SEED`,
visualize `DRIVER_SEED`, backtest `seed=7`); tests fix their own (42, 7, 8). Engines default
to floating seeds (`DCFAssumptions.seed`, `DriverDCF.run`), so pass a seed to reproduce. A test that needs the network or a floating seed is a defect.
Run `pytest -q` for the current count rather than trusting any doc.
Evidence: `pytest -q`; grep "no network" in tests/.
