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
| Rates and inflation source | 2026-11-18 | `financial_analysis/rates.py`, `scripts.verdict --rf-date`, `rust/` interface; IBM and JNJ verdicts at the Treasury rate | ~0 (inline) | orchestrator only, no subagents; not a valuation run. Cross-check: 27,520 Treasury points since 2020 agree with FRED |
| Repo consolidation | 2026-11-19 | merge of PRs #16 and #17 and the trading branch; removal of generated and private files; status documents rewritten | ~0 (inline) | orchestrator only, no subagents; not a valuation run |
| Standard input mapping | 2026-11-19 | registry of 109 standard variables, period selection, source record, mapping audit on a 32-ticker panel | ~0 (inline) | orchestrator only, no subagents; not a valuation run |
| Mapping test framework (Rust) | 2026-11-19 | registry as one TOML file; `rust/mapcheck` (mapper, 22 checks, parallel run); bulk fetch; Rust-Python parity; runs on 32, 50 and 472 tickers | ~0 (inline) | orchestrator only, no subagents; not a valuation run |
| Share-count rule and external cross-check | 2026-11-19 | share-count scale rule and check (Python and Rust); `scripts.mapping_crosscheck` against Yahoo Finance on 50 tickers | ~0 (inline) | orchestrator only, no subagents; not a valuation run |

Targets: frontier ticker ≤500k · repeat run ≤50k · ORCL (ticker #2) is the first measured
frontier run under the cheapened pipeline.
