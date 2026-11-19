"""Get SEC company facts for many tickers into the disk cache.

    python -m scripts.fetch_facts --tickers-file rust/mapcheck/universe/batch50.txt
    python -m scripts.fetch_facts --top 500 --write-list rust/mapcheck/universe/top500.txt
    python -m scripts.fetch_facts AMD INTC

The files go to data/raw/<TICKER>/company_facts.json.gz (compressed). A ticker that
is in the cache is not fetched again (use --refresh). ``--top N`` takes the first N
tickers of the SEC ticker list (the SEC sorts it by company size).

SEC fair-access rule: 10 requests in one second, maximum. The default delay between
requests is 0.15 s. The command uses the HTTP function and the User-Agent of
financial_analysis/edgar.py.
"""
from __future__ import annotations

import argparse
import gzip
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from financial_analysis import edgar

FACTS_URL = "https://data.sec.gov/api/xbrl/companyfacts/CIK{cik}.json"


def read_ticker_file(path: Path) -> list[str]:
    tickers = []
    for line in path.read_text(encoding="utf-8").splitlines():
        text = line.split("#", 1)[0].strip().upper()
        if text:
            tickers.append(text)
    return tickers


def sec_ticker_list(delay: float) -> list[tuple[str, str]]:
    """[(ticker, cik)] in the sequence of the SEC ticker list."""
    payload = json.loads(edgar._http_get(edgar.COMPANY_TICKER_URL, request_delay=delay))
    rows = [payload[key] for key in sorted(payload, key=int)]
    return [(str(r["ticker"]).upper(), str(r["cik_str"])) for r in rows if r.get("ticker")]


def is_cached(cache: Path, ticker: str) -> bool:
    folder = cache / ticker
    return (folder / "company_facts.json").exists() or (folder / "company_facts.json.gz").exists()


def fetch_one(cache: Path, ticker: str, cik: str, delay: float) -> int:
    """Fetch and save one ticker. Returns the size of the saved file in bytes."""
    url = FACTS_URL.format(cik=edgar._zero_pad_cik(cik))
    payload = edgar._http_get(url, request_delay=delay)
    facts = json.loads(payload)  # make sure that the payload is JSON
    folder = cache / ticker
    folder.mkdir(parents=True, exist_ok=True)
    path = folder / "company_facts.json.gz"
    with gzip.open(path, "wt", encoding="utf-8", compresslevel=6) as handle:
        json.dump(facts, handle, separators=(",", ":"))
    return path.stat().st_size


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("tickers", nargs="*")
    parser.add_argument("--tickers-file", type=Path)
    parser.add_argument("--top", type=int, help="The first N tickers of the SEC ticker list")
    parser.add_argument("--write-list", type=Path, help="Write the tickers that are in the cache after the run")
    parser.add_argument("--cache", type=Path, default=Path("data/raw"))
    parser.add_argument("--delay", type=float, default=0.15, help="Seconds between requests (minimum 0.1)")
    parser.add_argument("--refresh", action="store_true")
    args = parser.parse_args()
    delay = max(args.delay, 0.1)

    listed = sec_ticker_list(delay)
    cik_of = {}
    for ticker, cik in listed:
        cik_of.setdefault(ticker, cik)

    tickers = [t.upper() for t in args.tickers]
    if args.tickers_file:
        tickers += read_ticker_file(args.tickers_file)
    if args.top:
        seen_cik: set[str] = set()
        for ticker, cik in listed:
            if cik in seen_cik:      # a second share class of the same company
                continue
            seen_cik.add(cik)
            tickers.append(ticker)
            if len(seen_cik) >= args.top:
                break
    tickers = list(dict.fromkeys(tickers))
    if not tickers:
        parser.error("Give tickers, --tickers-file or --top.")

    fetched = cached = 0
    failed: list[tuple[str, str]] = []
    total_bytes = 0
    for number, ticker in enumerate(tickers, 1):
        if not args.refresh and is_cached(args.cache, ticker):
            cached += 1
            continue
        cik = cik_of.get(ticker)
        if cik is None:
            failed.append((ticker, "not in the SEC ticker list"))
            continue
        try:
            total_bytes += fetch_one(args.cache, ticker, cik, delay)
            fetched += 1
        except (edgar.EdgarClientError, json.JSONDecodeError) as exc:
            failed.append((ticker, str(exc)[:80]))
        if number % 50 == 0:
            print(f"  {number} of {len(tickers)} ...", flush=True)

    print(f"Tickers: {len(tickers)} | fetched {fetched} ({total_bytes / 1e6:.0f} MB) | in the cache before {cached} | failed {len(failed)}")
    for ticker, reason in failed:
        print(f"  {ticker:8} {reason}")
    if args.write_list:
        have = [t for t in tickers if is_cached(args.cache, t)]
        args.write_list.parent.mkdir(parents=True, exist_ok=True)
        header = f"# {len(have)} tickers. Made by scripts.fetch_facts"
        if args.top:
            header += f" --top {args.top} (the first companies of the SEC ticker list)"
        args.write_list.write_text(header + ".\n" + "\n".join(have) + "\n", encoding="utf-8")
        print(f"List: {args.write_list}")


if __name__ == "__main__":
    main()
