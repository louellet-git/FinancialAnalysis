# Hidden assumptions — the trap registry

Things that will silently produce wrong analysis if you don't know them. Every entry was
re-verified against code or data on 2026-07-17; each carries its
evidence pointer. If you find a new one, add it here with evidence — this file is the
institutional memory that keeps traps from being re-discovered the expensive way.

## Data / importer traps

**1. FY2019–20 recast mismatch — margin windows longer than 5 years are unusable (IBM).**
The store mixes continuing-ops (recast) revenue with unrecast consolidated OCF across the
Kyndryl boundary: FY2020 revenue $55.2B against OCF $18.2B gives a spurious ~28% FCF
margin. Any margin/growth window reaching past FY2021 is contaminated.
*Evidence:* `data/processed/IBM.json` (FY2020 row), `analysis/IBM/README.md` Data notes,
REVIEW.md F13, `financial_analysis/backtest.py` `DATA_FLAGS`.

**2. Phantom 10-K rows.** IBM's store contains "10-K" rows for 2012-06-30 (1 field),
2018-01-01 (2 fields), 2019-01-01 (1 field) — stray facts the importer keyed into their
own periods. They carry no revenue/OCF so FCF-based code skips them, but anything
iterating "all annual periods" must filter (the backtest's `_clean_annuals` requires
revenue AND OCF for exactly this reason).
*Evidence:* verified in `data/processed/IBM.json`; REVIEW.md F13; ticket T-IMP-1.

**3. `filed_date` is the comparative-column filing date — point-in-time visibility lags
real filings by up to ~2 years.** `xbrl_api._deduplicate` keeps the **latest-filed** entry
per period end, and the importer stamps `filed_date` from the revenue fact that survived.
So IBM FY2019 carries `filed 2022-02-22` (the FY2021 10-K), FY2021 carries `filed
2024-02-26`, FY2023–25 all carry `filed 2026-02-24`. Consequences: (a) `filed_date` is
NOT "when the market first saw this figure"; (b) backtest vintages are conservative
(see fewer periods than a real-time observer), never optimistic — the safe direction, but
it truncates early vintages.
*Evidence:* `financial_analysis/xbrl_api.py` `_deduplicate`; `importer.py`
`_populate_filed_dates`; verified per-row in `IBM.json`; `backtest.py` module docstring.

**4. `filed_date` can be outright fabricated (placeholder = period_end).** The importer
initializes `filed_date` to the period end and only overwrites it where a *revenue*
concept fact matched that (end, form). The phantom rows and sparse pre-2018 rows keep the
placeholder — e.g. the 2012-06-30 row claims it was "filed" 2012-06-30. Never trust
`filed_date` on a row without revenue.
*Evidence:* `importer.py` line ~154 (`"filed_date": end,  # placeholder`) and
`_populate_filed_dates` (revenue concepts only); IBM.json phantom rows show
filed_date == period_end.

**5. Sparse old annual rows are an `n_periods=8`-per-concept artifact, not history.**
`import_company(n_periods=8)` takes the last 8 entries *per concept*. Different concepts
cover different eras, so IBM shows 22 "annual periods" where pre-2018 rows carry 1–5
fields each (e.g. buybacks appear FY2012–19 because those were that concept's last 8
10-K facts; revenue exists only FY2018+). A naive "all history" series is really a
ragged union of per-concept 8-entry windows.
*Evidence:* `importer.py`/`xbrl_api.extract_series` (`[-n:]`); IBM.json field counts per
row (verified: 1–5 fields pre-2018, 45–49 from FY2018).

**6. Buyback absence FY2020+ is GENUINE, not a gap.** `repurchases_common_stock` has real
entries FY2012–FY2019 (last: FY2019, −$1.361B) and none after — IBM actually stopped
repurchasing. Do not "fix" this; conversely, per #5, the FY2012 *start* of the series is
an artifact.
*Evidence:* verified in IBM.json; concept `PaymentsForRepurchaseOfCommonStock` mapped in
`xbrl_tags.py`.

**7. Segment data is hand-entered from 8-Ks — companyfacts has NO dimensional data.**
Every companyfacts fact entry carries exactly `{accn, end, filed, form, fp, frame, fy,
start, val}` — no segment axis/member keys (verified across every us-gaap concept in
IBM's payload). `PeriodFinancials.segment_revenue` stays `None`; the IBM
`SEGMENT_HISTORY` in `driver_model.py` was typed in from Q4 8-Ks (sources in DRIVERS.md).
Filing-level XBRL instance parsing is Ticket 7.
*Evidence:* `importer.py` module docstring; PLANNING/tickets.md Ticket 7.

**8. Prices (and options/estimates/surprises) are never in the JSON store.**
`store.save` persists periods only; `scripts/analyze.py` attaches market data to the
in-memory object *after* loading, and it is never round-tripped. Prices live in
`data/raw/TICKER/prices.json` and the xlsx Prices tab only. Loading `TICKER.json` gives
`prices == []` — this is why v3's price figures are hand-entered and disclosed as such.
*Evidence:* `store.py` `save` payload keys; `IBM.json` top-level keys
(ticker/cik/company_name/periods only); ticketed in T-IMP-1.

## Model / valuation traps

**9. The IBM headline is CONDITIONAL on f = MA_RECURRENCE = 0.50.** f is a *judgment*,
not an estimate: the fraction of M&A spend charged against margin, paired with a
f × acquired-growth credit in g_software's mean. It is a single module-level constant
with an assert tying the driver to it (a two-place hardcode was a caught silent-drift
risk). The published f-locus spans P50 $197.79 (f=0) → $101.44 (f=1). Never quote
P50 $151.84 without the conditionality.
*Evidence:* `analysis/IBM/driver_model.py` `MA_RECURRENCE` + assert; DRIVER-REVIEW.md
ma_recurrence row; report §4 table 6.

**10. The correlation matrix is reviewed *judgment*, and it moves tails only.** All 28
entries (including the 3 deliberate zeros) are argued assumptions (rationales in
DRIVERS.md / driver_model.py), PSD-verified (min eigenvalue 0.419). The supply-shock
regime flip (rf·g = −0.15) leaves P50 essentially unchanged ($151.89 vs $151.84) and
widens tails. Do not treat correlation entries as estimated from data — none are.
*Evidence:* `driver_model.py` CORR; dcf_drivers_run.py disclosure runs; SOURCE-AUDIT
claim 32.

**11. Tail caveat — P10/P90 are decorative.** Growth (and every other driver) is one
draw held constant per path: no mean reversion, so far-year dispersion is overstated in
shape but the left tail is *understated* overall (v3 experiment: AR(1) φ=0.6 cut base
P50 ~10%, P90 ~16%). Binding rule from F6: P50s are robust, P10s are optimistic, and
**no margin-of-safety claim may be made from these tails** until T-DCF-1 lands.
*Evidence:* REVIEW.md F6/T-DCF-1; `driver_dcf.py` module docstring "Known limitation";
tail caveat carried verbatim in the report (audit claim 35).

**12. RPO was consumed as evidence WITH AN EXPIRY — it lapses at the Q2-2026 10-Q.**
The $71.0B RPO facts corroborate the growth means (they changed no parameter — all four
proposed changes were rejected in the round-3 re-look) and expire when the Q2-2026 10-Q
prints (late July 2026). After that, the rationale footnotes in driver_model.py and the
ledger rows are stale until re-consumed. The +12.7% YoY print consolidates HashiCorp and
is NOT organic evidence.
*Evidence:* `analysis/IBM/research-ledger.md`; DRIVER-REVIEW.md RPO re-look; rule R1 in
PLANNING/RULES.md.

**13. `scenarios.py` still carries v3-era IBM values while `driver_model.py` is v4.**
The registry's IBM presets (margins 9.2/14.5/16.1%, WACCs 8.25/7.50/7.25%) are the v3
aggregate-model scenarios; the normative IBM valuation is the v4 driver run (WACC at
means 7.06%, the v3 "7.5% top-of-band" position is explicitly retired). The visualizer's
aggregate panels intentionally show the v3 presets. Flagged, not hidden.
*Evidence:* `financial_analysis/scenarios.py` `_ibm_v3`; DRIVERS.md §WACC attribution.

**14. The published v3 numbers no longer reproduce from the current store (store drift).**
`analysis/IBM/dcf_scenarios.py` derives FCFF margins live from `IBM.json`; the
p1-importer re-import shifted the derived margins (5y mean 17.8% → 18.4%), so a fresh run
prints Bear $43 / Base $149 / Bull $243 against the signed-off record of
**$37 / $141 / $232**. The record (README/REVIEW.md) is frozen history; the script output
is live. Do not "correct" either to match the other — cite the record for v3, the engine
for anything current. (The v4 driver run does NOT drift: its inputs are pinned constants.)
*Evidence:* run of `python -m analysis.IBM.dcf_scenarios` on 2026-07-17 vs
analysis/IBM/README.md; `dcf_scenarios.py` `fcff_v3_margins` reads the store.

**15. Default capital-structure derivation ≠ the reviewed IBM bridge.** For tickers
without a `CAPITAL_OVERRIDES` entry, net debt = long-term debt + short-term borrowings −
cash — **no pension, no marketable securities, no stock-vs-flow review**. The IBM
override embeds the F15-reviewed bridge (debt + after-tax pension − cash&securities).
A cross-ticker comparison (the ORCL/basket runs) that mixes overridden and derived
capital structures compares different net-debt definitions.
*Evidence:* `scenarios.py` `capital_structure_for` vs `CAPITAL_OVERRIDES["IBM"]` +
analysis/IBM/README.md bridge table.

## Environment / process traps

**16. officecli mutates a .docx on open.** Reviews and audits must run against a COPY
(`cp doc /tmp/review.docx`); only deliberate edits touch the master. officecli lives at
`~/.local/bin/officecli` (`export PATH="$HOME/.local/bin:$PATH"`).
*Evidence:* `.claude/agents/memo-reviewer.md` ("Review a COPY, never the original").

**17. A PostToolUse hook fires on any produced .docx.** `scripts/memo-review-reminder.sh`
exits 2 (surfacing a demand for a memo-reviewer pass) whenever a Bash command creates a
.docx by any route (officecli create/merge, pandoc, python-docx save, libreoffice
convert). Expect it; don't fight it.
*Evidence:* `.claude/settings.json` hooks + the script.

**18. Tracked-but-gitignored churn — never commit these.** Tracked files that .gitignore
would exclude: `data/processed/NVDA_financials.xlsx`, `data/processed/PANW_financials.xlsx`,
four `__pycache__/*.pyc` files (`financial_analysis/__init__`, `financial_analysis/edgar`,
`scripts/__init__`, `scripts/import_edgar`), plus the intentional `data/raw/.gitkeep`.
They show as perpetually modified; candidate fix is `git rm --cached`, until then exclude
them from every commit. (HANDOFF.md's "NVDA xlsx + a .pyc" undercounts — it is 2 xlsx + 4 pyc.)
*Evidence:* `git ls-files | git check-ignore --stdin --no-index`, 2026-07-17.

**19. Timed/background tasks in this environment can be killed mid-run — persist resume
state in files.** A prior documentation agent died at the reading stage to a session
token limit; PLANNING/HANDOFF.md is the live pattern: write open work-state to a file as
you go, delete items as completed. Interrupted fan-outs must resume, never restart
(vision ✦6b).
*Evidence:* PLANNING/HANDOFF.md (its own item 1 records the death); reusable-dcf-vision.md ✦6b.

**20. Branch fragmentation — "done" work may not be on your branch.** Live example from
the day this file was written: the backtest harness sat complete-with-tests on
`factory-builds` while `handoff` (then checked out) lacked it — `python -m
scripts.backtest` failed and a doc claiming "backtest ✓" looked wrong; PRs #7/#8 merged
both branches to main hours later, and the main tree's branch was switched mid-flight
under a running agent. Lessons: check `git log --all --oneline -- <path>` before
concluding something doesn't exist; never switch branches in a shared tree while agents
are working (a shared-tree branch switch once misplaced a commit — use the `wt-*` git
worktrees, siblings of the repo directory, for parallel work).
*Evidence:* git history around `8153323`/`9751523` (2026-07-17); HANDOFF.md Known state.

**21. LF/CRLF warnings are normal; PowerShell 5.1 quirks are real.** `core.autocrlf=true`
on this checkout — "LF will be replaced by CRLF" warnings are expected noise, not damage.
The primary shell is Windows PowerShell 5.1: no `&&`/`||` chaining, no ternary, UTF-16
default file encoding (`-Encoding utf8` when writing files other tools read), quirky
native-arg quoting (`--%` stop-parsing token when needed). Git Bash is available for
POSIX scripts.
*Evidence:* `git config core.autocrlf`; environment.

**22. Tests are no-network by construction — keep them that way.** The whole suite runs
on synthetic fixtures (mocked HTTP, synthetic yfinance/consensus/company objects) — 267
tests at the pre-merge handoff snapshot (~16s), 309 after the backtest/visualizer merges
(the backtest tests are slower; ~3 min). Fixed seeds (7) everywhere randomness exists:
`SCENARIO_SEED = 7`, dcf_drivers_run `SEED = 7`, backtest default `seed=7`. A test that
needs the network or a floating seed is a defect. Run `pytest -q` for the current count
rather than trusting any doc.
*Evidence:* `pytest -q` runs of 2026-07-17 (267 passed pre-merge; 309 collected
post-merge, one visualizer test red mid-flight while HANDOFF item 2 lands); grep
"no network" in tests/.
