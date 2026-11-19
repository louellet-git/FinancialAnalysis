---
id: T-20261119-dcf-engine-rust
title: Move the DCF engines, the scenarios, the verdict and the driver models to Rust
area: engine
status: review
owner: louellet-git
branch: claude/dcf-engine-rust
created: 2026-11-19
acceptance: The Rust commands give the same values as the Python engines for IBM, JNJ and ANT (same seed, same percentiles), the IBM version 4 headline stays $83.84 / $151.84 / $258.03, and the Python engine files are deleted.
---
Owner request, 2026-11-19: refactor the code base to Rust; the subsequent work is the
improvement of the DCF. This ticket is steps 1 and 2 of `T-20261119-python-to-rust`.
The branch starts from `claude/admiring-allen-f5aaeq` (it has the ANT first pass).

Stages:

1. Record the outputs of the Python engines as fixed answers. Write the Rust engines
   (classic and driver), the scenarios and the verdict. The random numbers must be the
   same as those of numpy (`default_rng(seed)`, `standard_normal`), so that each
   published number stays the same.
2. Driver models (IBM, JNJ, ANT) as data files, and their runs as Rust commands.
3. Backtest.
4. Report and `analyze`; then delete the Python engines, the store and the schema.

The Python files stay until each Python user of an engine is moved (stage 4). The
branch is not merged before that, because one part must have one implementation.

## State (2026-11-19): stages 1 and 2 are done; stages 3 and 4 are open

The status of the ticket stays `in-progress`: the acceptance text needs the deletion of the
Python engine files, which is stage 4.

**Stage 1 (proved).**
- The random numbers are the numpy numbers: `rust/src/dcf/nprandom.rs`. Source: numpy
  v2.5.3 (tables, algorithm, `SeedSequence`, `PCG64`). The tests compare bit for bit:
  the raw stream and 1,000 normals for the seeds 0, 7, 42 and 123456789, a (3, 500)
  matrix, `normal(mean, std)`, and a stream of 200,000 values that reaches the tail
  branch. `np.percentile` is also equal bit for bit.
- The fixtures are in `rust/tests/fixtures/dcf/`. They were recorded from the Python
  engines before the Rust code was written.
- Rust engines: classic (`classic.rs`), driver (`driver.rs`), scenarios, verdict.
  Parity with Python: percentiles 2e-16 relative at most; attribution 1e-14; the tests
  accept 1e-9. The IBM headline from `fa dcf drivers IBM` is $83.84 / $151.84 / $258.03.
- Fixed-answer tests: closed-form calculations in exact fractions for the driver engine
  (two models with all drivers deterministic: with and without year-1 pins and a
  schedule) and for the classic engine (all std = 0). They cover a part of ticket
  `engine-fixed-answer-tests`.

**Stage 2 (done).**
- Driver models are data: `analysis/{IBM,JNJ,ANT}/driver_model.toml`. A test shows that
  each file gives the same model as `build()` (numbers bit for bit, schedules, every
  rationale text) and the recorded results.
- Commands: `fa dcf drivers|classic|scenarios|models`; `fa verdict` runs in Rust when the
  ticker has a model file and the run has no market rates. The viewer `/api/verdict`
  uses the same function and has the same JSON fields.
- One feature set for the runner analyses: overrides by driver name (`--set`,
  `--shift-bounds`, `--flat`, `--ramp`, `--corr`, `--net-debt`, `--shares`, `--years`,
  `--year1-revenue`), `--sensitivity`, `--implied-price`. The tests
  `runner_tests.rs` make each printed number of the three runners again and find
  the line in the recorded output of the Python runner: IBM (all sections), JNJ (all),
  ANT (all sections, including the fade and the commitment check).

**Not done.**
- `fa verdict --rf-date` and `--rating` still call `scripts.verdict` (Python): the
  Treasury and ICE data come from Python.
- `scripts/visualize_dcf.py`, `scripts/analyze.py`, `financial_analysis/backtest.py`
  and the tests still use the Python engines.
- The Rust `fa dcf classic` reads `data/processed/<TICKER>.json`. The importer is still
  Python or `mapcheck`.

**Python files with a proven Rust equal, and what blocks their deletion.**

| Python file | Rust equal | Blocked by |
|---|---|---|
| `financial_analysis/driver_dcf.py` | `dcf/driver.rs` | `analysis/*/driver_model.py` (3), runners (3), `financial_analysis/verdict.py`, `scripts/verdict.py` (rates path), `scripts/visualize_dcf.py`, `financial_analysis/rates.py` and `bonds.py` (`with_risk_free`, `with_kd_spread` change a Python `DriverModel`), tests `test_driver_dcf`, `test_verdict_rates`, `test_visualize_dcf`, `test_bonds`, `test_rates` |
| `financial_analysis/dcf.py` | `dcf/classic.rs` | `financial_analysis/scenarios.py`, `scripts/run_dcf.py`, `scripts/visualize_dcf.py`, `analysis/IBM/dcf_scenarios.py`, `tests/test_dcf.py`; `schema.py` and `store.py` (company data) |
| `financial_analysis/scenarios.py` | `dcf/scenarios.rs` | `scripts/visualize_dcf.py`, `analysis/IBM/dcf_scenarios.py`, `tests/test_scenarios.py` |
| `financial_analysis/verdict.py` | `dcf/verdict.rs` | `scripts/verdict.py`, `tests/test_verdict.py` |
| `scripts/run_dcf.py` | `fa dcf classic` | nothing in the code; README line and tests |
| `scripts/verdict.py` | `fa verdict` (no rates) | the `--rf-date` and `--rating` paths |
| `analysis/IBM/dcf_drivers_run.py`, `analysis/JNJ/first_pass_run.py`, `analysis/ANT/first_pass_run.py` | `dcf/runner_tests.rs` and `fa dcf drivers` | `CLAUDE.md` rule 6 and the README name `python -m analysis.IBM.dcf_drivers_run` as the source of the version 4 headline: the owner must decide the new source before the file is deleted |
| `analysis/{IBM,JNJ,ANT}/driver_model.py` | the `.toml` files | `scripts/visualize_dcf.py`, `scripts/verdict.py`, the runners, `tests/test_driver_dcf.py`, `tests/test_visualize_dcf.py`; two copies until then (`HIDDEN-ASSUMPTIONS` #123) |

Next: stage 3 (backtest), then stage 4 (report, `analyze`, deletion). Open question for
the owner: the numbers of the Python runners become tests with a recorded text; keep the
fixture files as the record when the runners are deleted.

## State (2026-11-19, later): stage 3 and the rates part of stage 4 are done

**Done in this branch.**
- Backtest: `fa backtest TICKER` (`rust/src/dcf/backtest.rs`). The text is the same as the
  Python text, character for character, on the real IBM store and on synthetic companies
  (`rust/tests/fixtures/backtest/`). `scripts/backtest.py` and its tests are deleted.
- Rates and bonds: `fa rates`, `fa bonds` (`rust/src/rates.rs`, `bonds.rs`) with the same
  JSON fields; `fa verdict` with `--rf-date`, `--rf-base-date` and `--rating` in Rust
  (`verdict_rates.rs`). Deleted: `rates.py`, `bonds.py`, `verdict.py`, `scripts/verdict.py`,
  `scripts/run_dcf.py` and their tests. FRED does not answer the cloud container
  (HIDDEN-ASSUMPTIONS #128).
- Runners: the four Python runners are deleted. `fa dcf drivers` has the options for each of
  their numbers (`--keep-year1-margin`, `--path`, `--fade`, `--commitments`,
  `--ev-fixed-point`, `--reinvestment-check`); `fa dcf classic` has `--margin-mean`,
  `--margin-std`, `--net-debt`, `--shares`. The commands: `analysis/ANT/COMMANDS.md`,
  `analysis/JNJ/COMMANDS.md`, `analysis/IBM/README.md`.

**Still open (stage 4).** `scripts/visualize_dcf.py` imports `driver_dcf.py`, `dcf.py`,
`scenarios.py` and `backtest.py`, and reads `analysis/<TICKER>/driver_model.py`. These
files, `analysis/*/driver_model.py` and their tests stay until the report is Rust
(HIDDEN-ASSUMPTIONS #135).

## State (2026-11-19, final): stage 4 is done; the acceptance is met

**The HTML report is Rust.** `fa report TICKER [--price P] [--out FILE]` (`rust/src/report/`) writes the same page as
`scripts/visualize_dcf.py`. The Python script is deleted. The viewer routes `/api/report/make`, `/api/report/info` and
`/report/file` call `report::make_report` in the same process (the JSON fields are the same).

- **Charts.** The Rust code writes each plotly figure as JSON (`serde_json`). The page draws them with plotly.js 4.1.1
  (the version that the installed Python library plotly 7.1.0 inlined) from `https://cdn.plot.ly/plotly-4.1.1.min.js`.
  **The page needs the network when a person opens it.** The Python page had the library inside the file and worked
  offline. Open point for the owner: an option that inlines a local copy (the repo does not carry the 4.8 MB file).
- **Parity** (fixtures recorded from the Python script before the Rust code; `rust/tests/fixtures/report/`, 0.7 MB):

  | Case | Sections | Figures | Numbers compared (all arrays) | Largest relative difference |
  |---|---|---|---|---|
  | IBM (price, options, driver model) | 6 of 6 | 19 | 94,772 | 0 (equal bit for bit) |
  | JNJ (price, options, driver model) | 6 of 6 | 19 | 89,271 | 0 |
  | KO (no price history, no driver model) | 6 of 6, two notes | 11 | 31,096 | 0 |
  | KO with a short price history and an empty option chain | 6 of 6, three notes | 9 | 32,856 | 0 |

  The text of the body (each heading, text, table cell and chip) is equal, except the words that name a Python command
  or file (`TEXT_CHANGES` in `rust/src/report/tests.rs`: `driver_model.py` is `driver_model.toml`, `scenarios.py` is
  `scenarios.rs`, `scripts/visualize_dcf.py` is `fa report`, `yfinance` is `Yahoo`). The tests keep the 1e-9 limit and a
  digest for each array of more than 200 numbers. The page in a browser (same library) shows 0 console errors; a
  screenshot of the IBM page has the same height as the Python page and 0.16% different pixels (the bars have no
  template outline).
- **Same random numbers.** The path sample (`np.random.default_rng(7).choice(..., replace=False)`) is
  `Pcg64::choice_set` (Floyd's algorithm), equal to numpy for 8 recorded cases and for the pages above.
- **Speed.** IBM: Python 1.1 s, Rust 0.15 s for the full page (both read the caches, no network).

**Deleted** (git history has them): `scripts/visualize_dcf.py`; `financial_analysis/{backtest,dcf,driver_dcf,scenarios,schema,store,market_data,options}.py`;
`analysis/{IBM,JNJ,ANT}/driver_model.py`; `tests/test_{backtest,dcf,driver_dcf,scenarios,schema,schema_market,store,market_data,options,visualize_dcf}.py`;
`tests/__init__.py`; `requirements-dev.txt`. The bridge of the `fa` crate is removed (`fa run`, `Fa::run_captured`,
`default_python`, `FA_PYTHON`, `FaError::Python` and `FaError::Spawn`).

**Each Python test has a Rust test or a port.** The 211 Python tests were mapped to Rust tests before the deletion.

| Python file (tests) | Rust equal |
|---|---|
| `test_backtest.py` (17) | `rust/src/dcf/tests.rs`, module `backtest`: filed date, phantom rows, quarterly forms, 10-K/A, parameters, three periods, no forward information, IBM flag, projection, realized join and scoring, unrealized rows, shock outside the band, skipped vintages, coverage and bias; the flat `table()` rows are the scorecard of the report (`report::tests`). |
| `test_dcf.py` (16) | The recorded classic runs (`classic_engine_and_scenarios_match_python`, `error_messages_name_the_cause`, `classic_engine_matches_exact_closed_form_with_zero_std`) and the ported properties `python_tests::classic_engine_properties` and `classic_engine_needs_four_annual_periods_with_positive_fcf_and_ignores_quarters` (shapes, per-year parts add up, PV below FCF, seed, explicit margin, per-share, 3 periods, negative FCF, quarterly rows). |
| `test_scenarios.py` (14) | `scenario_registry_matches_python` (defaults, keys, order, IBM override in any case, IBM capital) and the port `python_tests::net_debt_and_shares_follow_the_rules_of_the_python_tests` (total debt, current debt, latest annual period, missing debt, cash, shares, diluted fallback, no annual period). |
| `test_driver_dcf.py` (61) | The recorded runs for IBM, JNJ and ANT (`driver_engine_matches_python_for_ibm_jnj_ant`), `driver_validation_rejects_the_same_inputs_as_python`, `model_rejects_duplicates_bad_correlations_and_non_pd_matrices`, `engine_rejects_missing_drivers_and_bad_structure`, the exact hand calculations (`driver_engine_matches_exact_hand_calculation`, `terminal_growth_guard_binds_when_tg_is_close_to_wacc`), the override tests, and the ports `python_tests::the_vetted_formulas_give_the_hand_values_of_the_python_tests`, `sampling_realizes_the_correlation_the_bounds_and_the_marginals` and `a_driver_run_has_ordered_percentiles_reproducible_paths_and_arrays_that_add_up`. Not ported, no meaning in Rust: `test_items_registry_names_the_vetted_formulas` (the `ITEMS` registry of Python functions does not exist) and `test_result_exposes_no_mean_or_std` (the Rust result struct has no such field; the compiler checks it). |
| `test_schema.py` (19), `test_schema_market.py` (2) | `company::tests` (`free_cash_flow_and_margin_follow_the_python_property`, `every_variable_of_the_registry_is_read_from_a_store_file`, `mapped_data_gives_typed_periods_...`); defaults and empty lists are the type system of Rust. The dataclass `PricePoint` is `market::prices::PricePoint`. The market fields of `CompanyFinancials` (`prices`, `option_chain`) have no Rust form: the report reads the cache files. |
| `test_store.py` (17) | `company::tests::a_written_store_file_is_pretty_json_and_loads_again_in_any_case` (folder made, indented JSON, ISO dates, load in any case, all fields and `raw_extras`), `series_are_sorted_and_have_the_derived_metrics` (`get_series`: sorted, `free_cash_flow`, `fcf_margin`, invalid metric), `a_missing_store_file_gives_the_import_hint`. No Rust form, no caller: `store.compare` (a loop over `get_series`) and the in-memory cache of `load`. |
| `test_market_data.py` (2) | `market::prices::tests` (the cache file is read; no cache and no answer is an error with a clear text). |
| `test_options.py` (21) | `market::options::tests` (`metrics_match_the_python_functions`: 7 recorded chains with the expected values of the Python functions, among them synthetic, single-sided, no spot, zero spot and empty snapshot; `persistence_round_trip_listing_and_the_same_day_rule`, now with a missing date). |
| `test_visualize_dcf.py` (42) | `report::tests`: the body and the figures of 4 recorded pages (the sections, titles, chips, tables, the price marker, range buttons, the driver section and the path explorer, the backtest section, the options section, the footer); the ports `the_skew_figure_drops_expiries_that_are_less_than_two_days_out`, `fundamentals_without_revenue_and_a_backtest_without_history_give_notes`, `the_path_sampler_keeps_every_tail_path_reaches_the_target_and_is_deterministic`, `without_any_price_the_report_has_no_price_marker_and_no_crash`, `the_page_uses_the_blueprint_tokens`; `serve::tests` (a bad model file gives a note and the report goes on). Not kept: `test_report_is_self_contained` (the page now loads plotly.js from the CDN; test `the_page_loads_plotly_from_the_cdn_...` records this). |

**CI and requirements.** `requirements.txt` keeps `requests` and `python-dotenv` (the legacy SEC client may need them; its
file was not read). `requirements-dev.txt` had only `pytest` and is deleted. The workflow `tests.yml` has no `pytest` job
any more (no Python test remains; a new Python test would be new Python code). The job `mapcheck` runs the mapcheck tests and
the release build, and the job `cargo` runs the `fa` crate tests. Test counts now: `rust/` 325 + 17 + 1 (before: 299 + 17 + 1),
`rust/mapcheck` 89, pytest 0 (before: 211).

**Open for the owner.** (1) An offline report: an option that inlines a local copy of plotly.js. (2) The new place of the SEC
contact text, so that the legacy client can go (ticket `python-to-rust`, step 9). (3) The source of the version 4 headline
text in `CLAUDE.md` rule 6 is now `fa dcf drivers IBM`.
