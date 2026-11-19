"""Compare the core inputs with Yahoo Finance (an independent source).

    python -m scripts.mapping_crosscheck IBM JNJ
    python -m scripts.mapping_crosscheck --tickers-file rust/mapcheck/universe/batch50.txt
    python -m scripts.mapping_crosscheck --tickers-file ... --out crosscheck.jsonl

For each ticker the command compares the latest fiscal year that the two sources have.
It prints, for each core input, the number of companies where the values agree, where
they agree only with a different definition, and where they differ.

Network: Yahoo Finance (through yfinance), one ticker each second approximately. The
SEC data must be in the cache (``scripts.fetch_facts``) or the command gets it.
Yahoo is not the truth: read a difference as a question, not as an error.
"""
from __future__ import annotations

import argparse
import json
import sys
import warnings
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from financial_analysis.crosscheck import (
    AGREE, DEFINITION, DIFFER, NO_VALUE_OURS, Summary, compare_period, match_fiscal_year,
)
from financial_analysis.importer import UnsupportedFilingError, import_company


def read_ticker_file(path: Path) -> list[str]:
    out = []
    for line in path.read_text(encoding="utf-8").splitlines():
        text = line.split("#", 1)[0].strip().upper()
        if text:
            out.append(text)
    return out


def yahoo_statements(ticker: str) -> dict:
    """{fiscal year end (date): {row label: value}} from the three annual statements."""
    import yfinance as yf

    handle = yf.Ticker(ticker)
    out: dict = {}
    for frame in (handle.financials, handle.balance_sheet, handle.cashflow):
        if frame is None or frame.empty:
            continue
        for column in frame.columns:
            day = column.date()
            for label, value in frame[column].items():
                if value == value:  # not NaN
                    out.setdefault(day, {})[str(label)] = float(value)
    return out


def _b(value) -> str:
    return "-" if value is None else f"{value / 1e9:,.2f}"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("tickers", nargs="*")
    parser.add_argument("--tickers-file", type=Path)
    parser.add_argument("--out", type=Path, help="Write one JSON line for each ticker and variable")
    parser.add_argument("--examples", type=int, default=6, help="Differences to print for each variable")
    args = parser.parse_args()

    tickers = [t.upper() for t in args.tickers]
    if args.tickers_file:
        tickers += read_ticker_file(args.tickers_file)
    tickers = list(dict.fromkeys(tickers))
    if not tickers:
        parser.error("Give tickers or --tickers-file.")

    warnings.filterwarnings("ignore")
    summary = Summary()
    skipped: list[tuple[str, str]] = []
    lines: list[dict] = []
    for ticker in tickers:
        try:
            company = import_company(ticker, save=False)
            yahoo = yahoo_statements(ticker)
        except UnsupportedFilingError:
            skipped.append((ticker, "not a us-gaap filer"))
            continue
        except Exception as exc:  # network and data errors of one ticker do not stop the run
            skipped.append((ticker, f"{type(exc).__name__}: {str(exc)[:60]}"))
            continue
        annual = {p.period_end: p for p in company.periods if p.form == "10-K"}
        match = match_fiscal_year(list(annual), list(yahoo))
        if match is None:
            skipped.append((ticker, "no fiscal year in both sources"))
            continue
        ours_end, theirs_end = match
        results = compare_period(annual[ours_end], yahoo[theirs_end])
        summary.add(ticker, results)
        for r in results:
            lines.append({"ticker": ticker, "fiscal_year_end": ours_end.isoformat(), "variable": r.variable,
                          "outcome": r.outcome, "ours": r.ours, "yahoo": r.theirs, "note": r.note})

    print(f"Cross-check with Yahoo Finance: {summary.companies} companies compared, {len(skipped)} skipped\n")
    print(f"{'variable':22}{'agree':>7}{'definition':>12}{'differ':>8}{'no value':>10}{'agreement':>11}")
    for variable, counts in summary.counts.items():
        share = summary.agreement(variable)
        shown = "-" if share is None else f"{share:.0%}"
        print(f"{variable:22}{counts[AGREE]:>7}{counts[DEFINITION]:>12}{counts[DIFFER]:>8}{counts[NO_VALUE_OURS]:>10}{shown:>11}")
    print("\n'definition': the values agree with a different definition (see the note). "
          "'no value': we have no value and Yahoo has one.")
    for variable, examples in summary.examples.items():
        shown = [e for e in examples if e[1].outcome in (DIFFER, NO_VALUE_OURS)][: args.examples]
        if shown:
            print(f"\n{variable}: differences (ticker: ours / Yahoo, $B or billions of shares)")
            for ticker, r in shown:
                print(f"  {ticker:6} {_b(r.ours):>10} / {_b(r.theirs):>10}")
        notes = sorted({e[1].note for e in examples if e[1].outcome == DEFINITION})
        for note in notes:
            names = [t for t, r in examples if r.outcome == DEFINITION and r.note == note]
            print(f"\n{variable}: {len(names)} agree with the definition '{note}': {' '.join(names[:12])}")
    if skipped:
        print(f"\nSkipped ({len(skipped)}):")
        for ticker, reason in skipped[:20]:
            print(f"  {ticker:6} {reason}")
    if args.out:
        args.out.write_text("".join(json.dumps(line) + "\n" for line in lines), encoding="utf-8")
        print(f"\nReport: {args.out}")


if __name__ == "__main__":
    main()
