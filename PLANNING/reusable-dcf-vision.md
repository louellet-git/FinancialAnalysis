# The Valuation Factory — reusable, self-improving DCF (standing initiative)

Design approved 2026-07-16 (Luc), with six amendments incorporated (§ marked ✦). Manager
agent: Claude (this vision's completion falls on the manager; work starts when the IBM v4
report pipeline completes). Universe: **non-financial US large-caps first**; archetype
expansion is the frontier.

## The idea

A DCF that is used repeatedly and deterministically: inputs from financial statements and
frozen research artifacts; a mapping from inputs to the DCF that is deterministic +
LLM-assisted with LLM involvement decaying **per archetype** (the honest asymptote is
"LLM only at the frontier," not zero); a deterministic engine; and a review system whose
accepted findings compound into deterministic infrastructure instead of being re-paid on
every run. The primary product of a run is not a P50 — it is a set of **named,
evidence-backed bets against the market price** (✦1).

## Architecture — seven layers

### L1 — Data + Research Ledger
Exists: XBRL importer, market data, option-chain snapshots. New: the **Research Ledger** —
LLM-gathered facts frozen as versioned, source-tagged records `{fact, value, unit, as_of,
source, tier, expiry_policy, confidence}`. ✦5: every record carries an **expiry policy by
fact type** (guidance expires at next earnings; segment history on restatement; market data
by staleness threshold) — expiry trips the L2 validity-condition hard-fail. **Provenance
tiers are schema-enforced**: primary (filings/XBRL) vs secondary (press/transcripts) carry
confidence ceilings that are not reviewer-discretionary; a secondary-tier fact feeding a
load-bearing driver auto-triggers an exception review (researcher agents reading the open
web are an injection surface).

### L2 — Mapping layer
Per-ticker **model register** (formalized `driver_model.py`): drivers with mandatory
stochastic-vs-deterministic rationale, formulas, correlations, capital structure, and
**mandatory validity conditions** — machine-checkable world-assertions (expected XBRL
concepts, segment names, parameter ranges, ledger freshness) whose violation hard-fails
into review. Above the registers: the **archetype template library** (mature large-cap is
template #1, distilled from IBM) and a router — template match → deterministic
parameterization; no match → LLM instantiates → frontier review → the library grows.

### L3 — Engine
`driver_dcf.py`, versioned. Every run pins engine + template + rulebook versions + data
hash + seed: any published number is forever reproducible.

### L4 — Rulebook + validators (the primary ratchet)
`RULES.md`: every accepted review finding that expresses a rule, with provenance. ✦3: each
rule declares **scope — universal / archetype-bound / ticker-bound** ("cyclical drivers get
decade-CAGR dispersion" is a mature-large-cap lesson, not a law); a rule conflicting with a
new mapping auto-triggers a frontier review, never a silent judgment call; **deprecation
requires a review fan-out of the same weight that created the rule**, challenge and
disposition recorded in RULES.md history. `validators.py`: the machine-checkable subset,
run at model build + CI. ✦4: one mandated validator is the **relative-valuation tripwire**
— back out the P50-implied multiples (EV/EBITDA, P/FCF) vs the peer set; violation does
not fail the run, it forces the charter reviewer to explicitly disposition the sentence
"our model prices X at Nx FCF vs peers at Mx" (an acceptable disposition is "yes, and
gap-attribution says why" — the sentence must be confronted, not won by the market).
✦2: **gate-injection testing** — a red-team corpus of known-bad models seeded with past
accepted findings (F2 debt double-count, F16 cost-without-growth, stale parameters,
negative credit spreads) is injected periodically through the FULL pipeline; every gate
must fail them; **each accepted finding generates both a validator and a red-team
specimen** (standing rule). A dead gate is observationally identical to a clean pipeline —
injection catch rate is the only way to see the difference.
✦7: the **statement-coverage validator** — a machine-generated inventory of EVERY item in
the financial statements (all mapped schema fields + every raw_extras XBRL concept, grouped
by statement): each item is classified `used` (naming the driver/formula/bridge line that
consumes it) or `excluded` with a reason code (immaterial-below-threshold, duplicative-of-X,
non-cash-presentation, handled-in-bridge, financing-captive-treatment, disclosure-only).
**Unclassified items fail the run.** The exclusion *reasons* are assumptions like any other:
a dedicated **coverage reviewer** judges them in every frontier fan-out (when a mapping is
created or changed), and the charter review verifies on every run that the coverage table
is current against the latest filing (a new XBRL concept appearing in a fresh 10-K arrives
unclassified and therefore forces attention). This kills the silent-omission failure mode:
nothing in the statements can be ignored by accident, only by argued decision.

### L5 — Review system (three tiers + terminus)
Exists (vendored doc-review framework + the valuation-analyst/skeptic/report-auditor
agents). Tiers: **frontier reviews** (full per-driver/per-formula fan-out + per-item
consolidation — the IBM pattern) for new templates/mappings; **charter review on EVERY
run** — a reviewer examines the applied rules + assumptions + formulas as a package and
must ACCEPT before publication (integral, non-negotiable); **exception reviews** —
validator violations, staleness, secondary-provenance triggers. ✦6a: **the accountability
terminus is Luc, not an agent** — frontier runs and any run accepted-with-reservations
require his explicit sign-off before publication; routine repeat runs on proven templates
publish on charter ACCEPT alone. ✦6b: **resumable fan-out semantics** — every fan-out
records per-review state; interrupted runs resume, never restart; every review is classed
**load-bearing or advisory**; publication is hard-blocked while any load-bearing review is
incomplete (a session limit must never produce a half-reviewed model that looks done).

### L6 — Gap attribution + bet ledger (✦1 — the centerpiece)
The headline artifact of every published run: **why does the market price differ from the
model, specifically?** Solve for the assumption set that reproduces the market price
(market-implied recurrence/growth/WACC vs ours), decompose the gap into named
per-assumption contributions ($/share each). Output: an explicit list of bets we are
taking against the market, each with evidence and rationale — opportunity identification,
not conformity. Calibration survives as lean scorekeeping on those bets: (a) the
**per-bet ledger** — every named disagreement recorded and graded when reality adjudicates
it (a factory that flags mispricings without tracking its hit rate is a story generator);
(b) the **sub-claim ledger** for near-term falsifiable predictions (F19 margins, guidance
growth, implied-move comparisons — July 22 is datapoint #1). Full distribution scoring
(CRPS/PIT/market-adjusted horizons) is a **deferred optional P4 module**, not core;
backtesting mode is out of core scope.

### L7 — Report pipeline
Exists: valuation-analyst ↔ valuation-skeptic → memo-reviewer cleanup loop →
report-auditor claim-traceability gate → .docx. Invoked per run at configurable depth;
the gap-attribution bet list is the report's spine.

## Steady-state run

`value TICKER` → refresh data (ledger expiry checks) → validity conditions → parameterize
→ engine → validators (incl. multiples tripwire) → gap attribution → charter review →
[frontier/reservation ⇒ Luc sign-off] → report + bet-ledger record. Every gate failure
routes to exactly as much LLM as the failure needs.

## Success metrics (five)

1. LLM tokens per repeat run (↓)
2. Frontier fraction of runs (↓)
3. Validator count / rule coverage (↑)
4. **Gate sensitivity: red-team injection catch rate (must stay ~100%)** (✦2)
5. Bet-ledger hit rate, graded as reality adjudicates (✦1)

## Phasing (after IBM v4 report completes)

- **P1** — Rulebook extraction from IBM's three review rounds; first validators; register
  schema with validity conditions; **red-team corpus + injection harness (✦2)**;
  **Research Ledger schema with expiry + provenance tiers (✦5)**; **statement-coverage
  validator built and run retroactively on the IBM register (✦7)** — the backlog of
  unclassified IBM items becomes the first coverage review.
- **P2** — Second ticker (NVDA or PANW; data cached) end-to-end; template extraction;
  **rule scoping/deprecation exercised by the new mapping (✦3)**; **multiples tripwire
  validator (✦4)**.
- **P3** — **Gap-attribution layer + per-bet and sub-claim ledgers (✦1)**; automatic-run
  wiring; **human-terminus gates + resumable fan-out semantics (✦6)**.
- **P4** — Archetype expansion (the frontier); optional distribution-scoring module if
  ever promoted.

## Current-state inventory (what already exists, 2026-07-16)

Engine (driver_dcf.py, 220 tests) · IBM register (driver_model.py, consolidated v4) ·
review framework (vendored doc-review-framework@d9d9075 + renamed agents) · review
records proving the loop (REVIEW.md 19 findings, DRIVER-REVIEW.md 19+4 reviews) ·
options pipeline · visualizer (aggregate; everything-visualizer in flight) ·
first sub-claims for the ledger: the July-22 watch list.
