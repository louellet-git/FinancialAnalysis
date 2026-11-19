"""Compare the Rust mapper (rust/mapcheck) with the Python importer.

    python -m scripts.mapping_parity --tickers-file rust/mapcheck/universe/batch50.txt
    python -m scripts.mapping_parity IBM JNJ
    python -m scripts.mapping_parity --sample 40          # 40 random tickers of the cache

The two programs read one registry (financial_analysis/standard_variables.toml) and
the same cached SEC files. For each ticker and each period, the command compares the
period list, the filing date, each standard variable, its source, and the tags with
no variable. A difference is a defect in one of the two programs. Exit code 1 if a
difference exists. No network (the tickers must be in data/raw/).
"""
from __future__ import annotations

import argparse
import json
import random
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from financial_analysis.importer import DEFAULT_N_PERIODS, UnsupportedFilingError, import_company
from financial_analysis.xbrl_tags import ALL_TAGS

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "rust" / "mapcheck" / "target" / "release" / "mapcheck"
REL_TOLERANCE = 1e-9


def read_ticker_file(path: Path) -> list[str]:
    out = []
    for line in path.read_text(encoding="utf-8").splitlines():
        text = line.split("#", 1)[0].strip().upper()
        if text:
            out.append(text)
    return out


def cached_tickers(cache: Path) -> list[str]:
    return sorted(
        p.name for p in cache.iterdir()
        if (p / "company_facts.json").exists() or (p / "company_facts.json.gz").exists()
    )


def rust_values(tickers: list[str], cache: Path, binary: Path = BINARY) -> dict:
    """{(ticker, form, period_end): record} from `mapcheck --emit`."""
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "values.jsonl"
        listing = Path(tmp) / "tickers.txt"
        listing.write_text("\n".join(tickers) + "\n", encoding="utf-8")
        subprocess.run(
            [str(binary), "--cache", str(cache), "--tickers-file", str(listing),
             "--periods", str(DEFAULT_N_PERIODS), "--emit", str(out)],
            check=True, capture_output=True, text=True,
        )
        records = {}
        with out.open(encoding="utf-8") as handle:
            for line in handle:
                rec = json.loads(line)
                records[(rec["ticker"], rec["form"], rec["period_end"])] = rec
    return records


def python_values(ticker: str, cache: Path) -> dict:
    """The same records from the Python importer."""
    try:
        company = import_company(ticker, n_periods=DEFAULT_N_PERIODS, save=False, cache_dir=cache)
    except UnsupportedFilingError:
        return {}
    records = {}
    for p in company.periods:
        values = {name: getattr(p, name) for name in ALL_TAGS if getattr(p, name) is not None}
        records[(ticker, p.form, p.period_end.isoformat())] = {
            "filed_date": p.filed_date.isoformat(),
            "values": values,
            "sources": dict(p.sources),
            "raw_extras": dict(p.raw_extras),
        }
    return records


def _same(a: float, b: float) -> bool:
    return abs(a - b) <= REL_TOLERANCE * max(abs(a), abs(b), 1.0)


def compare(python: dict, rust: dict) -> tuple[list[str], int]:
    """(differences, number of values compared)."""
    diffs: list[str] = []
    compared = 0
    for key in sorted(set(python) | set(rust)):
        label = f"{key[0]} {key[1]} {key[2]}"
        if key not in rust:
            diffs.append(f"{label}: period only in Python")
            continue
        if key not in python:
            diffs.append(f"{label}: period only in Rust")
            continue
        p, r = python[key], rust[key]
        if p["filed_date"] != r["filed_date"]:
            diffs.append(f"{label}: filed_date Python {p['filed_date']} / Rust {r['filed_date']}")
        for part in ("values", "raw_extras"):
            for name in sorted(set(p[part]) | set(r[part])):
                compared += 1
                pv, rv = p[part].get(name), r[part].get(name)
                if pv is None or rv is None or not _same(float(pv), float(rv)):
                    diffs.append(f"{label}: {part} {name} Python {pv} / Rust {rv}")
        for name in sorted(set(p["sources"]) | set(r["sources"])):
            if p["sources"].get(name) != r["sources"].get(name):
                diffs.append(f"{label}: source {name} Python {p['sources'].get(name)!r} / Rust {r['sources'].get(name)!r}")
    return diffs, compared


def run(tickers: list[str], cache: Path, binary: Path = BINARY) -> tuple[list[str], int, int]:
    """(differences, periods compared, values compared)."""
    rust = rust_values(tickers, cache, binary)
    python: dict = {}
    for ticker in tickers:
        python.update(python_values(ticker, cache))
    diffs, compared = compare(python, rust)
    return diffs, len(set(python) & set(rust)), compared


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("tickers", nargs="*")
    parser.add_argument("--tickers-file", type=Path)
    parser.add_argument("--sample", type=int, help="This number of random tickers of the cache")
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--cache", type=Path, default=Path("data/raw"))
    args = parser.parse_args()

    if not BINARY.exists():
        sys.exit(f"No Rust program at {BINARY}. Build it: cargo build --release --manifest-path rust/mapcheck/Cargo.toml")
    tickers = [t.upper() for t in args.tickers]
    if args.tickers_file:
        tickers += read_ticker_file(args.tickers_file)
    if args.sample:
        tickers += random.Random(args.seed).sample(cached_tickers(args.cache), args.sample)
    if not tickers:
        tickers = cached_tickers(args.cache)
    tickers = list(dict.fromkeys(tickers))

    diffs, periods, compared = run(tickers, args.cache)
    print(f"Parity: {len(tickers)} tickers, {periods} periods, {compared:,} values compared, {len(diffs)} differences")
    for line in diffs[:40]:
        print(f"  {line}")
    if len(diffs) > 40:
        print(f"  ... and {len(diffs) - 40} more")
    sys.exit(1 if diffs else 0)


if __name__ == "__main__":
    main()
