# mapcheck

`mapcheck` is the Rust program that makes the standard DCF input variables from SEC
data and checks them. It is the only implementation of this work: fetch, period
selection, tags, formulas, sources, checks, filing statements. No Python code does
these steps. The valuation code (`fa`: engines, backtest, report) reads the store file
that `mapcheck import` writes.

Speed on 4 processors: 472 companies in 2.8 seconds (166 companies in one second).
2,000 companies need approximately 12 seconds.

The registry of standard variables is one data file:
`financial_analysis/standard_variables.toml`. `mapcheck` contains the file at build
time. Build the program again after you change the registry.

The repo has a second Rust crate, `rust/` (the `fa` command). It uses `mapcheck` as a
library; it is not part of `mapcheck`.

## Commands

Run all commands from the repo root.

```bash
# Build (one time, and after each change of the registry)
cargo build --release --manifest-path rust/mapcheck/Cargo.toml

# Get the data (network: SEC EDGAR; the contact comes from sec_contact.toml, see below)
rust/mapcheck/target/release/mapcheck fetch --tickers-file rust/mapcheck/universe/batch50.txt
rust/mapcheck/target/release/mapcheck fetch --top 2000 --write-list rust/mapcheck/universe/top2000.txt
# The 10-K files, only for the companies that have a critical input with no value
rust/mapcheck/target/release/mapcheck fetch --filings --gaps --tickers-file rust/mapcheck/universe/sp500.txt
# The latest prospectus of a new registrant (424B4; if none, S-1/A; if none, S-1). The company facts must be in the cache.
rust/mapcheck/target/release/mapcheck fetch --prospectus ANT

# Map and check (no network)
rust/mapcheck/target/release/mapcheck --tickers-file rust/mapcheck/universe/sp500.txt
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

### SEC contact

The SEC asks each program to give its name and a contact in each request. The contact is in
the file `sec_contact.toml` in the repo root (key `user_agent`; a project address, public on
purpose). The program looks for the file in the folder of `FA_REPO`, in the current folder and
each parent folder, and in the repo root known at build time. A person who wants their own
contact sets the environment variable `SEC_USER_AGENT`. It replaces the file:

```bash
export SEC_USER_AGENT="FinancialAnalysis your-name your-contact"
```

PowerShell: `$env:SEC_USER_AGENT = "FinancialAnalysis your-name your-contact"`.
`www.sec.gov` (the ticker list and the filing files) refuses a request that has no
contact. A command that reads only the cache does not need a contact.

## Status of a company

| Status | Meaning |
|---|---|
| `PASS` | No finding that a person must examine. |
| `WARN` | A person must examine one or more findings before the data is used in a model. |
| `FAIL` | The mapping is incorrect, or the DCF cannot use the data. |
| `N/A` | Not judged: a foreign filer (form 20-F or 40-F), a filer that does not report in USD, a new company with no 10-K (also a new registrant with a form S-1 that has no XBRL values: finding `no_xbrl_values`), or a company with no revenue in the latest three fiscal years. |
| `ERROR` | The file cannot be read, or the filer does not use us-gaap (IFRS). |

The status is for the latest fiscal year. A `FAIL` in an earlier fiscal year gives the
finding `earlier_years_fail` (a warning).

### Company types

A bank, an insurer, an investment company and a REIT have a different statement
structure. The program decides the type from the sizes of the special lines, and each
type has its own list of critical inputs.

| Type | Rule | Critical inputs |
|---|---|---|
| `bank` | Deposits are 20% or more of total assets. | revenue, net income, total assets, equity, share count |
| `insurer` | Premiums are 20% or more of revenue, or policyholder liabilities are 30% or more of total assets; and the balance sheet has no current-assets total. | the same as a bank |
| `fund` | Revenue is the total investment income, and the balance sheet has no current-assets total. | the same as a bank |
| `reit` | Investment property is 30% or more of total assets. | revenue, net income, operating cash flow, total assets, share count |
| `standard` | All other companies. | revenue, operating cash flow, capex, cash, total assets, share count |

A free-cash-flow DCF does not fit a bank, an insurer or an investment company. The
status of these types says that the inputs are there and consistent. It does not say
that the DCF engines can value the company.

## Values from the 10-K

The company facts have no fact with a segment and no company tag. Thus some companies
have no value for a critical input: a company that gives capex with its own tag, a
company with two share classes, a company that gives revenue only for each product.
The 10-K has these values.

If a fiscal year has no value for revenue, capex or the share count, and the cache has
the 10-K (`mapcheck fetch --filings --gaps`), the program reads the filing
(`src/supplement.rs`). The rules:

1. **A standard tag that has values only with a segment member.** The value is the sum
   of the members of one axis. If one member is equal to the sum of the other members,
   it is their total, and it is the value. A member of the legal-entity axis is a
   subsidiary and is not used.
2. **Share count of a company with share classes.** The value is the largest class.
   If the sum of the classes agrees with net income divided by EPS, and the largest
   class does not, the value is the sum (classes with the same rights).
3. **Capex.** The sum of the capex lines in the investing section of the cash-flow
   statement, company tags included. A line is a capex line because of its tag name or
   its label. A line that is a different standard variable (capitalized software) is
   not included.
4. **No capex line.** If the investing section has lines but no capex line, capex is 0.
   The filer does not show capex because the amount is small for this filer.

One 10-K gives the values for its three fiscal years. The source text of each value
starts with `filing <accession number>`, and the check `filled_from_filing` gives a
warning: a person must read the lines that the value contains. Only revenue, capex and
the share count can get a value from the filing.

## The checks

| Question | Finding | Level | Meaning |
|---|---|---|---|
| Can the DCF use the inputs? | `core_missing_critical` | FAIL | A critical input of the company type has no value (standard: revenue, operating cash flow, capex, cash, total assets, share count). |
| | `filled_from_filing` | WARN | A critical input comes from the 10-K, not from the company facts (see "Values from the 10-K"). |
| | `revenue_small` | WARN | Revenue comes from the lease-income tag or the investment-income tag and is smaller than net income: it can be a part of the revenue. |
| | `core_missing` | WARN | A different core input has no value (for example total debt for a company with no debt tags). |
| | `core_approximate` | INFO | A core input comes from a fallback (for example interest expense from interest paid). |
| | `sign` | FAIL | A value has an impossible sign (negative revenue, negative cash). |
| | `no_revenue` | WARN | Revenue is zero. |
| | `short_history` | WARN | Fewer than 4 fiscal years have revenue, operating cash flow and capex. |
| | `share_count_identity` | WARN | The share count differs more than 10% from net income divided by EPS. |
| | `share_count_scaled` | INFO | The filer reports the share count with no scale; the mapper corrected a factor of 1,000 or 1,000,000. |
| Was a value counted two times? | `exceeds_total` | FAIL | One variable is larger than a total that contains it. |
| | `formula_not_used` | WARN | A formula gave a value that is larger than its total (two terms contain the same amount). The variable has no value. |
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
or incorrect; read the printed lines. Facts with two or more segment members are not
used. The standard variables use the filing values only for a critical input that has
no value (see "Values from the 10-K").

## Results (2026-11-20)

| Set | Companies | PASS | WARN | FAIL | N/A | ERROR |
|---|---|---|---|---|---|---|
| `universe/sp500.txt` (the S&P 500 members) | 503 | 68 | 433 | 0 | 2 | 0 |
| `universe/next500.txt` (the 500 companies after them in the SEC list) | 500 | 20 | 459 | 5 | 16 | 0 |
| `universe/next1000.txt` (the 1,000 companies after the first two lists) | 1000 | 42 | 840 | 30 | 88 | 0 |
| `universe/panel32.txt` (used to build the registry) | 32 | 7 | 25 | 0 | 0 | 0 |
| `universe/batch50.txt` (the first 50 new tickers) | 50 | 6 | 44 | 0 | 0 | 0 |
| `universe/top500.txt` (the first 500 companies of the SEC list; 472 have data) | 472 | 49 | 310 | 0 | 54 | 59 |

S&P 500: each company with a 10-K has a value for each critical input. The 2 `N/A` are
new companies with no 10-K yet (HONA, VYLR). Company types: 23 banks, 19 insurers,
19 REITs, 440 with the standard template. 24 companies get a critical input from the
10-K (16 capex, 7 share count, 2 revenue); for 5 of them capex is 0 because the filing
has no capex line.

Next 500: 37 companies get a critical input from the 10-K. The 16 `N/A`: 10 new
companies, 5 companies with no revenue, 1 company that reports in Canadian dollars.
The 7 `FAIL` of 2026-11-19, each with its cause (5 stay `FAIL`):

| Ticker | Finding | Cause |
|---|---|---|
| BWXT | `exceeds_total` | Error of the filer: the lease liability is tagged 1,000 times too large in three 10-Ks. |
| PRAX, CYTK | `exceeds_total` | The filer uses the tag for cash and short-term investments for a total that includes long-term securities. |
| NLY, AGNC | (closed on 2026-11-20) | Mortgage REITs: the top line is the interest income. The registry has it as `investment_income_operating` (tag `InterestIncomeOperating`). Both are now `WARN` with the type `fund`. |
| FWONA | `core_missing_critical` | A tracking stock: the share count has two segment members (group and share class). |
| FPS | `core_missing_critical` | First 10-K after the public offering: the share count covers only a part of the fiscal year. |

Next 1,000 (the SEC list of 2026-11-20, `mapcheck fetch --top 2600`: 2,467 tickers have
company facts; the list leaves out 238 with no us-gaap data and 183 foreign filers):
30 `FAIL` (28 with the standard template of 736, 3.8%), after the 10-K files of the 95
companies with a gap (`fetch --filings --gaps`). Before these files: 78 `FAIL`. The causes:

| Finding | Tickers | Cause |
|---|---|---|
| `core_missing_critical`: share count | LGN, VSNT, WBI, IMKTA | A new listing or spin-off (the first annual period is a stub), or only share-class members (IMKTA). |
| `core_missing_critical`: capex | HASI, CRCT, KRSA, DMLP | The 10-K has no capex tag in the last year (HASI buys real estate and finance receivables). |
| `core_missing_critical`: revenue | ARR, ENHA, FRNM, FRMI | A mortgage REIT with the tag for interest income on debt securities only; blank-check companies; a company with no revenue. |
| `core_missing_critical`: other | KN, CNK, AB, PVLA | KN: no cash-flow value for the last year in the company facts. CNK: a combined filing of two registrants (all values have a legal-entity member). AB: a holding LP with income from an equity-method investment only. |
| `exceeds_total` | HTFL, ZBIO, AGIO, KMTS, ORA, RUN, OPK, NBR | A filer tag that is larger than its total (the same cause as PRAX and CYTK, or an error of the filer). |
| `no_annual_period` | SKT, XXI, AGLT | No fiscal year with an anchor fact in a 10-K. |
| `sign`, `quarters_exceed_year` | AAMI, CLMT, FBNC | A negative share-based compensation value; a quarter sum larger than the year. |

The tag for interest income on debt securities (`InterestIncomeDebtSecuritiesOperating`)
is not in the registry as a revenue tag: a biotech with no revenue (BHVN) has it, and it
would make the revenue of that company interest income.

Standard template, FAIL share on 2026-11-20: S&P 500 0 of 432, next 500 5 of 406, next 1,000
28 of 736, together 33 of 1,574 (2.1%; the base before this ticket: 37 of 1,578). The acceptance
value (below 3%) is met for the 2,003 tickers together, not for the next 1,000 alone (3.8%).

The two interest-income tags of `investment_income_operating` have a scope (registry keys
`unclassified_only` and `scoped_min_assets_share`): they give a value only for a filer with no
current-assets total and with a value of 1% or more of total assets (a quarter: 0.25%). Before
this scope, 21 revenue rows of 9 companies with another business had interest income as revenue
(APTV, AGCO, ARE, UMH, NGVT, DNTH, IMNM, PLSE, AVLN). The FAIL counts did not change.

A `WARN` is not an error of the mapping. The most frequent warnings: more than 10% of a
rollup total has no variable (`unmapped_lines`), an earlier fiscal year has a `FAIL`
(`earlier_years_fail`), a core input that is not critical has no value (`core_missing`,
most frequently total debt for a company with no debt tags).

Standard variable against the filing lines, the 50 new tickers (`mapcheck crosscheck`,
run before the changes of the S&P 500 work):

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
  amortization 74%. Yahoo gets its numbers from the same filings; it was a second
  mapping, not a second source. The Rust program does not call Yahoo.

## How to expand

1. Get more tickers: `mapcheck fetch --top 2000 --write-list ...`.
2. Run `mapcheck` with `--out report.jsonl`.
3. Read the table "Findings" and the table "Material tags with no variable". A tag
   that many companies use is a candidate for the registry.
4. Change `financial_analysis/standard_variables.toml`. Build again. Run `cargo test`.
5. Run `mapcheck` again and compare the numbers with the tables above.

## Limits

- The standard variables use the companyfacts source: no segment data and no
  company-specific tags. The 10-K gives a value only for revenue, capex and the share
  count, and only when the company facts have no value.
- One 10-K has three fiscal years. A company that gets capex from the 10-K has capex
  for three years only; the classic engine needs four (`short_history`).
- A capex value from the 10-K is a sum of lines that are found by words. It can be
  too small (a line with an unusual name) or too large. Read the source text.
- A `PASS` shows that the mapped data is consistent. It does not show that each value
  is the number that an analyst wants (see `docs/HIDDEN-ASSUMPTIONS.md` #36).
- The income statement has no rollup. Cost lines overlap too much between filers.
- A company that changed its SEC registrant has its history under the earlier CIK.
  `universe/earlier_registrants.txt` has these companies (now: XOM); `mapcheck fetch`
  gets the facts of each CIK and makes one file.
