# mapcheck

`mapcheck` is the Rust program that makes the standard DCF input variables from SEC
data and checks them. It is the only implementation of this work: fetch, period
selection, tags, formulas, sources, checks, filing statements. No Python code does
these steps. The Python valuation code (engines, backtest, report) is legacy code; it
reads the store file that `mapcheck import` writes, until it moves to Rust.

Speed on 4 processors: 472 companies in 2.8 seconds (166 companies in one second).
2,000 companies need approximately 12 seconds.

The registry of standard variables is one data file:
`financial_analysis/standard_variables.toml`. `mapcheck` contains the file at build
time. Build the program again after you change the registry.

The repo has a second Rust crate, `rust/` (the `fa` command). It calls the Python code;
it is not part of `mapcheck`.

## Commands

Run all commands from the repo root.

```bash
# Build (one time, and after each change of the registry)
cargo build --release --manifest-path rust/mapcheck/Cargo.toml

# Get the data (network: SEC EDGAR; set SEC_USER_AGENT first, see below)
rust/mapcheck/target/release/mapcheck fetch --tickers-file rust/mapcheck/universe/batch50.txt
rust/mapcheck/target/release/mapcheck fetch --top 2000 --write-list rust/mapcheck/universe/top2000.txt

# Map and check (no network)
rust/mapcheck/target/release/mapcheck --tickers-file rust/mapcheck/universe/top500.txt
rust/mapcheck/target/release/mapcheck IBM JNJ --detail        # all findings for each ticker
rust/mapcheck/target/release/mapcheck --list                  # all tickers in the cache
rust/mapcheck/target/release/mapcheck --out report.jsonl      # one JSON line for each ticker

# Each standard variable of a ticker with its value and its source
rust/mapcheck/target/release/mapcheck values IBM

# Write data/processed/IBM.json for the valuation code
rust/mapcheck/target/release/mapcheck import IBM

# The statements of the latest 10-K, as the company structures them
rust/mapcheck/target/release/mapcheck filing VZ --statement cash
rust/mapcheck/target/release/mapcheck filing IBM COP --focus  # capex, depreciation and debt lines

# Capex, depreciation and debt: the standard variable against the filing lines
rust/mapcheck/target/release/mapcheck crosscheck --tickers-file rust/mapcheck/universe/batch50.txt

# Unit tests
cargo test --manifest-path rust/mapcheck/Cargo.toml
```

`mapcheck --help` shows all options.

### SEC_USER_AGENT

The SEC asks each program to give its name and a contact in each request. Set the
environment variable `SEC_USER_AGENT` before a command that gets data from the SEC:

```bash
export SEC_USER_AGENT="FinancialAnalysis your-name your-contact"
```

PowerShell: `$env:SEC_USER_AGENT = "FinancialAnalysis your-name your-contact"`.
`www.sec.gov` (the ticker list and the filing files) refuses a request that has no
contact. A command that reads only the cache does not need the variable.

## Status of a company

| Status | Meaning |
|---|---|
| `PASS` | No finding that a person must examine. |
| `WARN` | A person must examine one or more findings before the data is used in a model. |
| `FAIL` | The mapping is incorrect, or the DCF cannot use the data. |
| `N/A` | A bank, an insurer or a REIT (a different statement structure), or a foreign filer (form 20-F or 40-F). Not judged. |
| `ERROR` | The file cannot be read, or the filer does not use us-gaap (IFRS). |

The status is for the latest fiscal year. A `FAIL` in an earlier fiscal year gives the
finding `earlier_years_fail` (a warning).

## The checks

| Question | Finding | Level | Meaning |
|---|---|---|---|
| Can the DCF use the inputs? | `core_missing_critical` | FAIL | Revenue, operating cash flow, capex, cash, total assets or share count has no value. |
| | `core_missing` | WARN | A different core input has no value (for example total debt for a company with no debt tags). |
| | `core_approximate` | INFO | A core input comes from a fallback (for example interest expense from interest paid). |
| | `sign` | FAIL | A value has an impossible sign (negative revenue, negative cash). |
| | `no_revenue` | WARN | Revenue is zero. |
| | `short_history` | WARN | Fewer than 4 fiscal years have revenue, operating cash flow and capex. |
| | `share_count_identity` | WARN | The share count differs more than 10% from net income divided by EPS. |
| | `share_count_scaled` | INFO | The filer reports the share count with no scale; the mapper corrected a factor of 1,000 or 1,000,000. |
| Was a value counted two times? | `exceeds_total` | FAIL | One variable is larger than a total that contains it. |
| | `overlap:<rollup>` | WARN | Lines that are usually separate are together larger than their total: two of them overlap for this filer. |
| | `nested:<rollup>` | INFO | A variable is a detail inside a different line of this filer. Do not add it to the other lines. |
| | `equal_parts:<rollup>` | WARN | Two variables have the same value: possibly one item with two tags. |
| | `total_debt_estimates` | WARN | The ways to calculate total debt disagree by more than 2%. |
| | `capex_software_overlap` | WARN | The capex tag is a "productive assets" tag and the filer also has capitalized software: `capex_total` (the sum, rule R2) can count the software two times. |
| Was a value lost? | `unmapped_lines:<rollup>` | WARN | More than 10% of a total has no variable (25% for cash-flow totals). |
| | `material_unmapped` | INFO | Large tags of the filer that have no variable and no exclusion. |
| Do the statements agree? | `balance_sheet_identity` | FAIL | Total assets are not equal to total liabilities and equity. |
| | `rollup_mismatch:liabilities_and_equity` | WARN | Liabilities + temporary equity + equity are not equal to the total. |
| | `change_in_cash` | WARN | The three cash-flow totals and the exchange-rate effect do not give the change in cash. |
| | `net_income_identity` | WARN | Pre-tax income less tax is not equal to net income. |
| Are the periods correct? | `no_annual_period` | FAIL | No fiscal year in a 10-K. |
| | `quarters_exceed_year` | FAIL | The revenue of three quarters is larger than the revenue of the fiscal year. |
| | `fiscal_year_spacing` | WARN | Two fiscal year ends are not one year apart (a missing year, or a change of fiscal year). |
| | `stale_annual` | WARN | The latest fiscal year ended more than 500 days before the as-of date. |
| | `source_switch` | WARN | The tag of revenue changed and the value moved more than 30%. |

### Rollups: lost values and double counts

A rollup is a total and the lines that are its parts (for example current assets). The
registry file contains 8 rollups. The SEC companyfacts source does not say which lines
are parts of which line. One tag can be a line on the statement for one filer and a
detail inside a different line for a second filer. Thus each rollup has two lists:

- `parts`: lines that are usually separate lines on the statement.
- `details`: lines that are frequently a detail inside one of the parts.

The check adds the parts. Then it adds each detail only if the sum stays in the total.
A detail that does not fit is `nested` for that filer. If the parts alone are larger
than the total, the finding is `overlap`. If the final sum is much smaller than the
total, the finding is `unmapped_lines`.

The registry loader also refuses a registry that can cause a double count: one tag in
two variables, a formula that mixes memo ("of which") variables with line variables,
and a memo variable in a rollup.

## Filing statements

`mapcheck filing` reads two files of the latest 10-K: `MetaLinks.json` (the statements,
the label of each tag, the parent of each line) and the instance document (the value of
each tag, company-specific tags included). It prints the income statement, the balance
sheet and the cash-flow statement as trees. Each line shows its sign toward the parent
line, its value, and one of: `[variable]` (the standard variable of the tag),
`[no variable]` (a standard tag that the registry does not have), `[company tag]`.

`--focus` prints the capex, depreciation and debt lines and a value from these lines.
Capex is equipment and software (rule R2 in `PLANNING/RULES.md`); purchased intangibles
are shown on a separate line.

`mapcheck crosscheck` compares the standard variable with the value from the filing
lines for capex, depreciation and amortization, and total debt. The two values come
from different methods (tag names, and the structure of the statements of the filer),
and the filing is the source of truth. It uses no outside data source.

Limits: the filing lines are found by words in the tag name, thus a line can be missing
or incorrect; read the printed lines. Facts with a segment (60% to 70% of the facts in
a filing) are counted but not used. The standard variables do not use the filing values
yet (board ticket `filing-values-in-variables`).

## Results (2026-11-19)

| Set | Companies | PASS | WARN | FAIL | N/A | ERROR |
|---|---|---|---|---|---|---|
| `universe/panel32.txt` (used to build the registry) | 32 | 7 | 20 | 2 | 3 | 0 |
| `universe/batch50.txt` (50 new tickers) | 50 | 6 | 40 | 3 | 1 | 0 |
| `universe/top500.txt` (the first 500 companies of the SEC list; 472 have data) | 472 | 48 | 240 | 24 | 101 | 59 |

The 24 `FAIL` of the 472 companies: 15 have no standard tag for capex (they use a
company-specific tag), 5 have no share count (more than one share class, or a
partnership), 1 has no revenue tag after a change of fiscal year, and 4 have no 10-K
under the ticker (a new registrant). Of the 312 companies with a judgment: the
estimates of total debt disagree for 47, and 166 have a variable that is a detail inside
a different line.

Standard variable against the filing lines, the 50 new tickers (`mapcheck crosscheck`):

| Input | Agree | Differ | No value (standard variable) | No line (filing) | Agreement |
|---|---|---|---|---|---|
| capex (`capex_total`, rule R2) | 45 | 3 | 1 | 1 | 94% |
| total_debt | 38 | 8 | 2 | 2 | 83% |
| da_total | 33 | 15 | 1 | 1 | 69% |

Records of two comparisons that no longer run (the Python code for them is deleted):

- Rust mapper against the old Python importer, before the Python importer was removed:
  120 random tickers, 374,709 values, 0 differences.
- Standard variables against Yahoo Finance, 50 new tickers: eight core inputs agreed for
  98% or more, revenue 96%, share count 94%, total debt 83%, capex 79%, depreciation and
  amortization 74%. Against the same source, the filing lines agreed for capex 92%, total
  debt 90%, depreciation and amortization 81%. Yahoo gets its numbers from the same
  filings; it was a second mapping, not a second source. The Rust program does not call
  Yahoo.

## How to expand

1. Get more tickers: `mapcheck fetch --top 2000 --write-list ...`.
2. Run `mapcheck` with `--out report.jsonl`.
3. Read the table "Findings" and the table "Material tags with no variable". A tag
   that many companies use is a candidate for the registry.
4. Change `financial_analysis/standard_variables.toml`. Build again. Run `cargo test`.
5. Run `mapcheck` again and compare the numbers with the tables above.

## Limits

- The standard variables use the companyfacts source: no segment data and no
  company-specific tags. The filing statements have them, but the standard variables
  do not use the filing values yet.
- A `PASS` shows that the mapped data is consistent. It does not show that each value
  is the number that an analyst wants (see `docs/HIDDEN-ASSUMPTIONS.md` #36).
- The income statement has no rollup. Cost lines overlap too much between filers.
- `mapcheck fetch` and the fetch of a new filing were not run from this program
  against `www.sec.gov` on 2026-11-19 (no contact text was available in that session).
  The data API `data.sec.gov` and the cached files were used.
