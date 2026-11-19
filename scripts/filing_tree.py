"""Print the statements of the latest 10-K as the filer structures them.

    python -m scripts.filing_tree VZ                     # the three statements
    python -m scripts.filing_tree VZ --statement cash    # one statement: income, balance, cash
    python -m scripts.filing_tree VZ NEE COP --focus     # only the capex, depreciation and debt lines

Each line shows its sign toward the parent line, its value, and one of:
  [variable]      the tag is in the registry; this is its standard variable
  [no variable]   a standard tag that the registry does not have
  [company tag]   a tag that the company made (the companyfacts source does not have it)
A parent line with "parts differ" has parts that do not sum to its value.

Network: www.sec.gov, 3 requests for a filing that is not in the cache
(data/raw/<TICKER>/filings/).
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from financial_analysis.cache import load_company_facts
from financial_analysis.filing import (
    Filing, FilingError, Line, capex_lines, debt_lines, depreciation_lines, estimates, fetch_filing,
    intangible_purchase_lines, load_filing,
)


def _value(line: Line) -> str:
    return f"{'-':>12}" if line.value is None else f"{line.value / 1e9:>12,.3f}"


def _kind(line: Line) -> str:
    if line.variable:
        return f"[{line.variable}]"
    return "[company tag]" if line.company_tag else "[no variable]"


def print_line(line: Line, depth: int = 0) -> None:
    sign = "+" if line.weight >= 0 else "-"
    note = ""
    if line.sum_agrees() is False:
        note = f"  parts differ: {line.parts_sum() / 1e9:,.3f}"
    name = f"{'  ' * depth}{sign} {line.label}"[:62]
    print(f"  {name:62} {_value(line)}  {_kind(line)} {line.tag}{note}")
    for child in line.children:
        print_line(child, depth + 1)


def print_focus(filing: Filing) -> None:
    for title, lines in (
        ("Capital expenditure lines (in the investing section)", capex_lines(filing)),
        ("Purchases of intangible assets (in the investing section)", intangible_purchase_lines(filing)),
        ("Depreciation and amortization lines (in the operating section)", depreciation_lines(filing)),
        ("Debt lines (balance sheet)", debt_lines(filing)),
    ):
        print(f"\n  {title}:")
        if not lines:
            print("    none")
        for line in lines:
            print(f"    {line.label[:58]:58} {_value(line)}  {_kind(line)} {line.tag}")
    e = estimates(filing)

    def b(value) -> str:
        return "no line found" if value is None else f"{value / 1e9:,.3f}"

    print(f"\n  From these lines: capex {b(e.capex)} | capex with purchased intangibles {b(e.capex_with_intangibles)}"
          f" | depreciation and amortization {b(e.da_total)} | total debt {b(e.total_debt)}")


def report(ticker: str, statement: str | None, focus: bool, refresh: bool) -> None:
    facts = load_company_facts(ticker, cache_dir=Path("data/raw"))
    filing = load_filing(ticker, fetch_filing(ticker, facts, refresh=refresh))
    share = 100.0 * filing.facts_with_segment / filing.facts_total if filing.facts_total else 0.0
    print(f"\n{'=' * 110}\n{filing.ticker}  10-K {filing.accession}  fiscal year end {filing.fiscal_year_end}"
          f"  |  {filing.facts_total:,} numeric facts, {share:.0f}% with a segment (not used yet)  |  values in $B")
    if not focus:
        for st in filing.statements:
            if statement and st.kind != statement:
                continue
            lines = list(st.lines())
            company = sum(1 for line in lines if line.company_tag)
            unknown = sum(1 for line in lines if not line.company_tag and not line.variable)
            print(f"\n  --- {st.kind}: {st.name} ({len(lines)} lines; {company} company tags; "
                  f"{unknown} standard tags with no variable) ---")
            for root in st.roots:
                print_line(root)
    if focus or statement is None:
        print_focus(filing)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("tickers", nargs="+")
    parser.add_argument("--statement", choices=("income", "balance", "cash"))
    parser.add_argument("--focus", action="store_true", help="Only the capex, depreciation and debt lines")
    parser.add_argument("--refresh", action="store_true", help="Get the filing again")
    args = parser.parse_args()
    for ticker in args.tickers:
        try:
            report(ticker.upper(), args.statement, args.focus, args.refresh)
        except FilingError as exc:
            print(f"\n{ticker.upper()}: {exc}")


if __name__ == "__main__":
    main()
