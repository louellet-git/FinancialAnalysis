# mapcheck

`mapcheck` is the test framework for the mapping from XBRL tags to the standard DCF
input variables. It is a Rust program. It reads the SEC company facts in the disk
cache, makes the standard variables for each company, and does checks on the result.
It uses no network.

Speed on 4 processors: 472 companies in 2.8 seconds (166 companies in one second).
2,000 companies need approximately 12 seconds.

The repo has a second Rust crate, `rust/` (the `fa` command), which calls the Python
pipeline. The two crates are independent.

## One registry, two programs

The registry of standard variables is one file:
`financial_analysis/standard_variables.toml`. The Python importer and this Rust program
read the same file. The two programs are independent, and a parity test compares
their results. A difference shows a defect in one of the two programs.

`mapcheck` contains the registry file at build time. Build the program again after you
change the registry.

## Commands

Run all commands from the repo root.

```bash
# Build (necessary one time, and after each change of the registry)
cargo build --release --manifest-path rust/mapcheck/Cargo.toml

# Get the data for a list of tickers (network: SEC EDGAR)
python -m scripts.fetch_facts --tickers-file rust/mapcheck/universe/batch50.txt
python -m scripts.fetch_facts --top 2000 --write-list rust/mapcheck/universe/top2000.txt

# Run the checks
rust/mapcheck/target/release/mapcheck --tickers-file rust/mapcheck/universe/top500.txt
rust/mapcheck/target/release/mapcheck IBM JNJ --detail        # all findings for each ticker
rust/mapcheck/target/release/mapcheck --list                  # all tickers in the cache
rust/mapcheck/target/release/mapcheck --out report.jsonl      # one JSON line for each ticker

# Compare Rust with Python
python -m scripts.mapping_parity --tickers-file rust/mapcheck/universe/batch50.txt
python -m scripts.mapping_parity --sample 120

# Compare with an independent source (network: Yahoo Finance)
python -m scripts.mapping_crosscheck --tickers-file rust/mapcheck/universe/batch50.txt

# Unit tests
cargo test --manifest-path rust/mapcheck/Cargo.toml
```

`mapcheck --help` shows all options.

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

## Results (2026-11-19)

| Set | Companies | PASS | WARN | FAIL | N/A | ERROR |
|---|---|---|---|---|---|---|
| `universe/panel32.txt` (used to build the registry) | 32 | 7 | 20 | 2 | 3 | 0 |
| `universe/batch50.txt` (50 new tickers) | 50 | 6 | 40 | 3 | 1 | 0 |
| `universe/top500.txt` (the first 500 companies of the SEC list; 472 have data) | 472 | 48 | 240 | 24 | 101 | 59 |

Parity of Rust and Python: 120 random tickers, 2,003 periods, 374,709 values,
0 differences.

The 24 `FAIL` of the 472 companies: 15 have no standard tag for capex (they use a
company-specific tag), 5 have no share count (more than one share class, or a
partnership), 1 has no revenue tag after a change of fiscal year, and 4 have no 10-K
under the ticker (a new registrant).

Of the 312 companies with a judgment: the estimates of total debt disagree for 47,
and 166 have a variable that is a detail inside a different line.

The first run on the 50 new tickers gave 6 `FAIL`. Three were registry gaps and are
corrected (capex and cash aliases, a net-income fallback). The registry got 22
variables and 17 aliases from the 50 and the 472 runs (131 variables now).

## Three tests

| Test | Question | Command |
|---|---|---|
| Consistency | Is the mapped data of a company consistent in itself? | `mapcheck` |
| Parity | Do the Rust mapper and the Python importer give the same values? | `scripts.mapping_parity` |
| Cross-check | Do the values agree with an independent source? | `scripts.mapping_crosscheck` |

Cross-check with Yahoo Finance, the 50 new tickers, latest fiscal year (2026-11-19):

| Input | Agree | Different definition | Differ | No value | Agreement |
|---|---|---|---|---|---|
| income_before_tax, income_tax_expense, operating_cash_flow, cash_and_equivalents, total_assets, total_liabilities, equity | 50 | 0 | 0 | 0 | 100% |
| net_income | 49 | 0 | 0 | 0 | 100% |
| sbc_total | 44 | 0 | 1 | 0 | 98% |
| revenue | 48 | 0 | 2 | 0 | 96% |
| shares_diluted | 46 | 0 | 3 | 1 | 94% |
| total_debt | 39 | 1 | 7 | 2 | 83% |
| capex | 36 | 9 | 3 | 1 | 75% |
| da_total | 35 | 0 | 12 | 1 | 74% |

For 9 companies, capex agrees only when capitalized software and purchased intangibles
are included. The share-count differences are stock splits after the period (Yahoo
adjusts the old numbers) and one company with more than one share class.

## How to expand

1. Get more tickers: `python -m scripts.fetch_facts --top 2000 --write-list ...`.
2. Run `mapcheck` with `--out report.jsonl`.
3. Read the table "Findings" and the table "Material tags with no variable". A tag
   that many companies use is a candidate for the registry.
4. Change `financial_analysis/standard_variables.toml`. Build again. Run the tests
   (`cargo test`, `python -m pytest`) and the parity command.
5. Run `mapcheck` again and compare the numbers with the table above.

## Limits

- The checks use the companyfacts source: no segment data and no company-specific
  tags. These need filing-level XBRL (PLANNING/tickets.md, Ticket 7).
- A `PASS` shows that the mapped data is consistent. It does not show that each value
  is the number that an analyst wants (see `docs/HIDDEN-ASSUMPTIONS.md` #36).
- The income statement has no rollup. Cost lines overlap too much between filers.
