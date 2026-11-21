# Model: economics view engine (`fa macro view`)

Updated: 2026-11-21. Code: branch econ/view-engine-cmd. Design: `analysis/macro/2026-11-21-view-engine-design.md`,
frozen at 3982324; records R1, R2 and R3 (section 6; R2 and R3 were uncommitted text when this document was written).

## Purpose
The economics view for the attribution of Fundamental Research (`fa dcf attribute`). It gives, for each window, the change
of the equity risk premium and of the long-run growth that the economists expect. The source is the Survey of Professional
Forecasters (SPF), not the stock prices that the attribution explains. This is layer A only (the SPF level).
It is a reading of the market, not a cause, not a trade signal and not an input of a valuation model.
A person decides with the output how much of the plug of a window to read as a market-wide view and how much as a company effect.

## Inputs
| Input | Source | Note |
|---|---|---|
| RGDP10, CPI10, STOCK10 | SPF medians of the first-quarter surveys, `macro/spf/medianLevel_<date>.xlsx` (newest file) in the store | Points per year. Private copy. No series in the repo (R2). |
| Release dates | `macro/spf/spf-release-dates_<date>.txt` (newest file) in the store | One public release date for each Q1 survey. |
| GSW10 | column SVENY10 of `rates/gsw_feds200628.csv` in the store | Continuously compounded zero yield. Last row on or before the release date. |

The command reads files only. It starts no download. `--cache` sets the store (default `data/raw`).

## Method (short)
1. Build one level for each Q1 survey: ERP_A = STOCK10 - GSW10 at the release date; Growth_A = RGDP10 + CPI10. Points per year.
2. The survey of a date t is the latest Q1 survey with a release date on or before t. The level is a step function of the release date.
3. A window (D1, D2) comes from the file of `--windows`. It is covered if a release date falls after D1 and on or before D2, and a survey is out at D1.
4. For a covered window: change = level of D2 minus level of D1, divided by 100 (0.40 points = 0.0040).
5. A window that is not covered has EMPTY changes and the source "no release". The command never writes 0.
6. A survey with a bound release date or a missing value is left out and counted.

## Outputs
CSV, one row for each window, on stdout or in `--out FILE`. The header is `date,d1,erp_change,growth_change,source,growth_parts`.
| Column | Meaning | Unit |
|---|---|---|
| date | D2, end of the window | YYYY-MM-DD |
| d1 | D1, start of the window | YYYY-MM-DD |
| erp_change | change of ERP_A; EMPTY if no release | decimals per year |
| growth_change | change of Growth_A; EMPTY if no release | decimals per year |
| source | "spf" or "no release" | text |
| growth_parts | for example "RGDP10 2.0 + CPI10 2.3"; empty if no release | SPF points, as published |

On stderr: windows, covered windows, release windows, surveys used, surveys left out.

## Assumptions
| Record | Name | Value | One line |
|---|---|---|---|
| A-168 | ERP form | STOCK10 minus GSW10 | Not minus BOND10 or BILL10 |
| A-169 | Growth form | RGDP10 + CPI10 | Nominal; maps to G of A-001 only |
| A-170 | Release date sets the step | release date | Not the survey quarter |
| A-171 | EMPTY rows | EMPTY, "no release" | Not 0 |
| A-172 | Surveys left out | bound date or missing value | Counted and printed (R3.2) |
| A-173 | No compounding conversion | none | GSW10 continuous; STOCK10 form not known (R3.3) |

R3.1 (the caller gives the windows in `--windows FILE`) sets no value, so it has no record.

## Limits
- Q1 surveys only. The design estimates about 2% of the weekly windows are covered (about 8 releases from 2019 in about 400 windows). A covered window takes a full year of change in one week. This is mainly a yearly level check, not a weekly view.
- Compounding: no conversion (R3.3). ERP_A can differ from a like-for-like premium by about 0.23 points at a 7% return. A change is less affected than the level.
- The return form of STOCK10 (arithmetic or geometric, total or price) is not known.
- The panel changes (about 40 forecasters). A median can be revised. The medians in the store are not a vintage file.
- The surveys lag the market. A forecaster can follow the market, so the view is not fully independent of the price.
- Layer B (a measurement part fitted to dS1) is not built.
- The split of the plug is not in this command. The reader of Fundamental Research does it (`--view-file`, planned).

## Code and command
| What | Path or command |
|---|---|
| Code | `rust/src/macro_test/view.rs` (also `mod.rs`; `spf_dir` and `newest` in `rust/src/market/spf.rs`) |
| Command | `fa macro view --windows FILE [--cache DIR] [--out FILE]` |
| Windows file | lines `d1,date` (YYYY-MM-DD); a header line is allowed |
| Tests | `cargo test --manifest-path rust/Cargo.toml --locked macro_test::view`: 12 tests with fixed answers. Names are in `view.rs`. |

The 12 tests cover: level formula, unit (0.40 points = 0.0040), step by release date, release-file line format, EMPTY row,
no survey at D1, header and sources, poison, truncation, left-out surveys, GSW row choice, windows file.

## Backtest
Backtest: none applies, because the command splits an attribution and moves no value or trade rule; its point-in-time check is the poison and truncation tests.

## Review status
Code gate: not yet run. No review record of the code is known to this document. No run on the real SPF file exists.
The first real run waits for the code gate (R1). Other conditions of R1 are in the design.
