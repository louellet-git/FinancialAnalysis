"""Audit the mapping from XBRL tags to the standard DCF input variables.

    python -m scripts.mapping_audit IBM JNJ          # full report for each ticker
    python -m scripts.mapping_audit --panel          # summary for the 32-ticker panel
    python -m scripts.mapping_audit --panel --field total_debt   # one variable, all tickers

The report is for the latest fiscal year of each ticker. It shows the source of each
value (an XBRL tag, or a formula), the accounting identities that do not agree, and
the material tags that have no standard variable. Network: SEC EDGAR (disk cache in
data/raw/). The command writes no files in data/processed/.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from financial_analysis.cache import load_company_facts
from financial_analysis.importer import UnsupportedFilingError, import_company
from financial_analysis.mapping_audit import (
    audit_period,
    core_fill_rates,
    filer_template,
    latest_annual_period,
)
from financial_analysis.xbrl_tags import ALL_TAGS, CORE_FIELDS, STATEMENT_MAP, UNIT_MAP

PANEL = (
    "AAPL MSFT NVDA IBM ORCL GOOGL AMZN META TSLA JNJ PFE UNH WMT COST HD KO PG XOM CVX "
    "CAT BA GE DUOL PANW CSCO ACN T DIS NKE NEE JPM O"
).split()


def _fmt(name: str, value: float) -> str:
    unit = UNIT_MAP.get(name, "USD")
    if unit == "USD":
        return f"{value / 1e9:>12,.3f} B"
    if unit == "shares":
        return f"{value / 1e6:>12,.1f} M"
    return f"{value:>12,.2f}  "


def _load(ticker: str, refresh: bool):
    company = import_company(ticker, save=False, refresh=refresh)
    facts = load_company_facts(ticker, cache_dir=Path("data/raw"))
    period = latest_annual_period(company)
    end = period.period_end.isoformat() if period else None
    return company, filer_template(facts, end)


def report_ticker(ticker: str, refresh: bool) -> None:
    company, template = _load(ticker, refresh)
    period = latest_annual_period(company)
    if period is None:
        print(f"\n{ticker}: no annual period with total assets.")
        return
    audit = audit_period(period)
    annual = sum(1 for p in company.periods if p.form == "10-K")
    quarterly = sum(1 for p in company.periods if p.form == "10-Q")
    print(f"\n{'=' * 100}\n{ticker}  {company.company_name}  |  fiscal year end {period.period_end}"
          f"  |  template: {template}  |  {annual} annual rows, {quarterly} quarterly rows")
    if template != "standard":
        print(f"  NOTE: a {template} has a different statement structure. The standard variables do not fit fully.")
    last_statement = None
    for name in ALL_TAGS:
        statement = STATEMENT_MAP[name]
        if statement != last_statement:
            print(f"\n  --- {statement} ---")
            last_statement = statement
        value = getattr(period, name)
        core = "*" if name in CORE_FIELDS else " "
        if value is None:
            if name in CORE_FIELDS:
                print(f"  {core} {name:38} {'NO VALUE':>14}")
            continue
        print(f"  {core} {name:38} {_fmt(name, value)}  {period.sources.get(name, '')}")
    print(f"\n  Variables with a value: {audit.filled} of {len(ALL_TAGS)} ({audit.from_formula} from a formula)."
          f"  Core inputs (*): {audit.core_filled} of {len(CORE_FIELDS)}.")
    if audit.core_missing:
        print(f"  Core inputs with no value: {', '.join(audit.core_missing)}")
    print("  Identity failures: " + ("none" if not audit.identity_failures else ""))
    for failure in audit.identity_failures:
        print(f"    - {failure}")
    print(f"  Material tags with no standard variable: {len(audit.unmapped)}")
    for concept, value in audit.unmapped[:15]:
        print(f"    - {concept:80} {value / 1e9:>10,.3f} B")


def report_panel(tickers: list[str], refresh: bool, field: str | None) -> None:
    rows = []
    for ticker in tickers:
        try:
            company, template = _load(ticker, refresh)
        except (UnsupportedFilingError, ValueError, KeyError) as exc:
            print(f"{ticker}: not imported ({type(exc).__name__}: {str(exc)[:70]})")
            continue
        period = latest_annual_period(company)
        if period is None:
            print(f"{ticker}: no annual period with total assets.")
            continue
        rows.append((ticker, template, company, period, audit_period(period)))

    if field:
        print(f"\n{field} in the latest fiscal year")
        for ticker, template, _, period, _ in rows:
            value = getattr(period, field)
            shown = _fmt(field, value) if value is not None else f"{'NO VALUE':>14}"
            print(f"  {ticker:6} {str(period.period_end)}  {shown}  {period.sources.get(field, '')}")
        return

    print(f"\n{'ticker':7}{'template':9}{'FY end':11}{'annual':>7}{'filled':>8}{'core':>7}{'ident.':>8}{'unmapped':>9}  core inputs with no value")
    for ticker, template, company, period, audit in rows:
        annual = sum(1 for p in company.periods if p.form == "10-K")
        print(f"{ticker:7}{template:9}{str(period.period_end):11}{annual:>7}{audit.filled:>8}"
              f"{audit.core_filled:>4}/{len(CORE_FIELDS):<2}{len(audit.identity_failures):>8}{len(audit.unmapped):>9}"
              f"  {', '.join(audit.core_missing)}")

    standard = [a for _, template, _, _, a in rows if template == "standard"]
    print(f"\nFill rate of the core DCF inputs, {len(standard)} companies with the standard template:")
    for name, rate in core_fill_rates(standard).items():
        print(f"  {name:24} {rate:>6.0%}")
    failures = [(t, f) for t, template, _, _, a in rows if template == "standard" for f in a.identity_failures]
    print(f"\nIdentity failures (standard template): {len(failures)}")
    for ticker, failure in failures:
        print(f"  {ticker:6} {failure}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("tickers", nargs="*", help="Tickers for a full report")
    parser.add_argument("--panel", action="store_true", help="Summary for the 32-ticker panel (or the given tickers)")
    parser.add_argument("--field", help="With --panel: show one standard variable for each ticker")
    parser.add_argument("--refresh", action="store_true", help="Get new data from SEC EDGAR")
    args = parser.parse_args()

    if args.field and args.field not in ALL_TAGS:
        parser.error(f"'{args.field}' is not a standard variable (financial_analysis/xbrl_tags.py).")
    tickers = [t.upper() for t in args.tickers]
    if args.panel:
        report_panel(tickers or PANEL, args.refresh, args.field)
    elif tickers:
        for ticker in tickers:
            report_ticker(ticker, args.refresh)
    else:
        parser.error("Give one or more tickers, or --panel.")


if __name__ == "__main__":
    main()
