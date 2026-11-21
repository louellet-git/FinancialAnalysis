---
id: T-20261121-weekly-trade-proposals
title: Propose 3 fundamental trades each Friday for the live trader, from 2026-11-27
area: engine
status: in-progress
owner: louellet-git
requested_by: louellet-git
head: fundamental
order: 0.03
branch: claude/dcf-engine-backtest
created: 2026-11-21
acceptance: Each Friday from 2026-11-27, three trade write-ups (`trading/journal/<date>-fundamental-<ticker>.md`, form of `trading/journal/README.md`, under 3 pages each) are ready for Luc, each from the valuation engine with a dated, reviewed rule of a side and size, and each passed the gate of the Head of Quality and Risk; Luc decides each trade and places it himself (the repo sends no order).
---
Owner word, 2026-11-21 03:47 UTC: "And every week you will need to propose 3 trades to begin on the live trader,
beginning next Friday. A valuation engine will be required. Potentially a hedging engine when and if that's needed."

Plan of the Head of Fundamental Research (first Friday: 2026-11-27):
1. The basis is a strategy, not three single-name reports. A full four-step report for each name each week is
   not possible, and a first pass alone is not permitted for a trade (`CLAUDE.md`). So the weekly names come from
   the backtested rule (`analysis/priors/dcf_engine_backtest_2026-11-21.md`: the point-in-time DCF value rank,
   sector-neutral), and the rule itself goes through the gate once. Each weekly write-up then checks the name for
   the data faults that the screen cannot see (mapping status, a stale fiscal year, a share-count guard, an event).
2. The rule of a side and of the size (`T-20261121-trade-side-rule`) must be dated and reviewed before
   2026-11-27. It is written from the backtest, before the first weekly run, and is not tuned on the names.
3. Sleeve limits (largest position, largest loss, stop of the sleeve): the Head of Quality and Risk proposes them
   (`T-20261120-paper-desk`, decision 2); Luc sets the capital.
4. Hedging engine: only if the rule needs it. The backtest result is relative to the equal weight of the list, not
   absolute, so a beta hedge with SPY (a stock instrument) is the first candidate. No option work in this area.
5. Honest limit for Luc: the backtest evidence is weak (+2.9 to +3.5 points a year over the equal weight,
   t 1.6 to 2.1, 9 years, today's lists). Small sizes until the live record or the long history adds evidence.

Owner answers, 2026-11-21 about 03:55 UTC (relayed by the Head of Quality and Risk): "Paper 4 weeks, then live".
So the first 4 Fridays (2026-11-27 to 2026-12-18) are paper; live from about 2026-12-25, only after Luc approves
the sleeve limits ("Not sure" for now; the limits of the Head of Quality and Risk apply to paper as an interim rule).

Disclosure in each weekly write-up (advice of the Head of Economics, 2026-11-21; from the blind projection test),
word for word: "When the VIX closes at 25 or more (the trigger of analysis/macro/2026-11-21-stress-windows.md), the
stated P10 and P5 are not reliable; in the three stress windows of 2024-2026 the tail below P10 was 20-33%." Also the
backtested figures of the rule: hit rate 50.8% (+3.4 points over the base rate), 13-week P10 -15.2% and P5 -20.7%,
the worst year 2025. No size rule for stress now (a rule fitted to three seen windows has no test).

**Status 2026-11-21 (strategy gate cycle 2: PASS-WITH-NOTES on the negative result):** on prices with the split basis,
rule A-009 fails its decision rule (R1-hold -1.10 points a year against the equal weight, t -0.58, 3 of 9 years;
`analysis/priors/dcf_engine_backtest_rerun_2026-11-21.md`). No weekly proposal uses A-009, and no weekly proposal has
a backed rule now. A new rule needs its own test, written before its run. Luc chooses the next path.
