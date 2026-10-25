---
id: DOC-TOKEN-LEDGER
type: record
item: 9 record
status: current
date: 2026-10-25
scope: Subagent token cost per valuation run (success metric 1); append-only, one row per run.
---
# Token ledger — cost per run (success metric #1)

Instrumented from run one, per the approved industrialization plan. Subagent output
tokens summed from orchestration records; orchestrator overhead excluded (roughly
constant per run).

| Run | Date | Scope | Subagent tokens (approx) | Notes |
|---|---|---|---|---|
| IBM v1→v4.0 (frontier, everything) | 2026-07-16 | full stack: engine build, 19+4 reviews, report pipeline, visualizer, options, research | ~2.6M | includes one-time infrastructure (engine, framework vendoring, visualizer) not chargeable to future tickers |
| IBM v4.0 reviews only | 2026-07-16 | 19 driver/formula/matrix reviews + 4 consolidations | ~0.9M | the fan-out the P1 validators must compress to 3–5 targeted reviews |
| IBM report pipeline | 2026-07-16 | analyst (3 passes) + skeptic (2) + memo + auditor | ~1.1M | the deterministic report generator must cut ~80% of the analyst share (~0.77M) |
| IBM v4.1 RPO update | 2026-07-17 | targeted: 2 driver re-looks + report delta | ~160k | first exception-scope run: 2 re-looks (~79k) + analyst delta (~82k). vs the 2.6M frontier run — the compression the validators must make routine |
| DUOL first pass | 2026-07-17 | import + consensus-anchored presets + capital override + html | ~0 (inline) | orchestrator-inline, no subagents: analyze, two scenarios.py registry entries, report. NOT a frontier run — no driver model, no reviews; mechanical backtest coverage 0/6 says growth-phase tickers need the driver engine |

Targets: frontier ticker ≤500k · repeat run ≤50k · ORCL (ticker #2) is the first measured
frontier run under the cheapened pipeline.
