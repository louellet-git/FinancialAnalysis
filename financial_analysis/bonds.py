"""US corporate bond yields and credit spreads from free, keyless sources.

The driver engine prices debt as ``kd = rf + kd_spread`` with rf the Treasury 10Y.
This module gives market values for ``kd_spread`` and the context around them.

Sources, all through the FRED public CSV (no API key):

- ICE BofA US corporate indexes by rating (AAA, AA, A, BBB, BB, B, CCC and lower):
  option-adjusted spread (OAS) and effective yield, daily. FRED keeps only the last
  three years (ICE licence), and ICE does not allow redistribution. The cache is
  local only (``data/`` is not in git).
- Moody's seasoned Aaa and Baa corporate bond yields, daily from 1983 and 1986.
  These bonds have 20 years or more to maturity, so they are long-duration yields.
- U.S. Treasury HQM (High Quality Market) corporate bond yield curve: monthly average
  spot rates, 1 to 100 years, from 1984, a blend of AAA, AA and A bonds. Treasury
  publishes it as .xls. FRED HQMCB<n>YR carries the same numbers (checked 2026-11-18:
  306 values from 2024-01 to 2026-10, no differences).

Two spread measures:

- ``spread_over_10y`` = index effective yield - UST 10Y on the same date. This is the
  engine's definition of ``kd_spread`` (kd = rf_10Y + kd_spread), so the verdict uses it.
- OAS = spread over the Treasury curve at each bond's own maturity. It differs from
  ``spread_over_10y`` by the curve slope between the index duration and 10 years.

UNITS: decimals (0.0067 == 0.67%). Run ``python -m financial_analysis.bonds``.
"""
from __future__ import annotations

import argparse
import json
import logging
import re
import sys
from datetime import date
from pathlib import Path

from financial_analysis import rates
from financial_analysis.rates import DEFAULT_CACHE_DIR, RateObservation

logger = logging.getLogger(__name__)

RATINGS = ("AAA", "AA", "A", "BBB", "BB", "B", "CCC")
ICE_OAS = {"AAA": "BAMLC0A1CAAA", "AA": "BAMLC0A2CAA", "A": "BAMLC0A3CA", "BBB": "BAMLC0A4CBBB",
           "BB": "BAMLH0A1HYBB", "B": "BAMLH0A2HYB", "CCC": "BAMLH0A3HYC"}
ICE_YIELD = {k: f"{v}EY" for k, v in ICE_OAS.items()}
MOODYS = {"Aaa": "DAAA", "Baa": "DBAA"}
MOODYS_SPREAD_10Y = {"Aaa": "AAA10Y", "Baa": "BAA10Y"}  # FRED: Moody's yield - DGS10
HQM_MATURITIES = (1, 2, 3, 5, 7, 10, 20, 30)

ICE_SOURCE = "ICE BofA via FRED"
MOODYS_SOURCE = "Moody's via FRED"
HQM_SOURCE = "U.S. Treasury HQM via FRED"

_DAILY_MAX_AGE = 7     # days: weekends, holidays and the one-day FRED lag
_MONTHLY_MAX_AGE = 70  # days: a monthly average is dated the 1st and published after month end

_MOODYS_LETTERS = {"Aaa": "AAA", "Aa": "AA", "A": "A", "Baa": "BBB", "Ba": "BB", "B": "B",
                   "Caa": "CCC", "Ca": "CCC", "C": "CCC"}


def rating_bucket(rating: str) -> str:
    """S&P/Fitch ('A-', 'BBB+', 'AAA') or Moody's ('A3', 'Baa2', 'Aaa') -> ICE bucket.
    Notches (+/-, 1/2/3) are dropped: ICE indexes are by letter grade. CC and C go to CCC."""
    s = rating.strip()
    m = re.fullmatch(r"(AAA|AA|A|BBB|BB|B|CCC|CC|C)[+-]?", s)
    if m:
        g = m.group(1)
        return "CCC" if g in ("CC", "C") else g
    m = re.fullmatch(r"(Aaa|Aa|A|Baa|Ba|B|Caa|Ca|C)[123]?", s)
    if m:
        return _MOODYS_LETTERS[m.group(1)]
    raise ValueError(f"unrecognized rating {rating!r}; use S&P (A-, BBB+) or Moody's (A3, Baa2)")


# ---------------------------------------------------------------------------
# FRED with a local cache
# ---------------------------------------------------------------------------

def load_fred(
    series_id: str,
    cache_dir: Path = DEFAULT_CACHE_DIR,
    refresh: bool = False,
    max_age_hours: float = 6.0,
) -> dict[date, float]:
    """Full history of a FRED percent series as {date: decimal}, cached in
    ``data/raw/bonds/``. A failed fetch falls back to the cache with a warning."""
    path = cache_dir / "bonds" / f"fred_{series_id}.csv"
    if path.exists() and not refresh and rates._is_fresh(path, max_age_hours):
        return rates.parse_fred_csv(path.read_text(encoding="utf-8"))
    try:
        text = rates._http_get(rates.FRED_CSV.format(series=series_id, start="1900-01-01"))
        header = text.lstrip("﻿").split("\n", 1)[0].strip()
        if header.split(",")[-1] != series_id:
            raise ValueError(f"not a FRED CSV for {series_id} (header: {header[:60]!r})")
        parsed = rates.parse_fred_csv(text)
    except Exception:
        if path.exists():
            logger.warning("FRED fetch failed for %s; using cached copy", series_id)
            return rates.parse_fred_csv(path.read_text(encoding="utf-8"))
        raise
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return parsed


def _latest(series: dict[date, float], on: date | None, max_age_days: int, label: str) -> tuple[date, float]:
    """Latest (date, value) on or before ``on``. Raises if that value is older than
    ``max_age_days``, so a gap never returns a stale number silently."""
    on = on or date.today()
    eligible = [d for d in series if d <= on]
    if not eligible:
        first = min(series) if series else None
        raise LookupError(f"{label}: no value on or before {on} (data starts {first})")
    d = max(eligible)
    if (on - d).days > max_age_days:
        raise LookupError(f"{label}: latest value on or before {on} is from {d}, "
                          f"more than {max_age_days} days old")
    return d, series[d]


# ---------------------------------------------------------------------------
# Rating indexes (ICE BofA)
# ---------------------------------------------------------------------------

def _bucket_note(rating: str, bucket: str) -> str:
    return "" if rating.strip() == bucket else f"{rating.strip()} uses the {bucket} index"


def credit_spread(rating: str = "A", on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR) -> RateObservation:
    """ICE BofA option-adjusted spread for the rating's index."""
    b = rating_bucket(rating)
    d, v = _latest(load_fred(ICE_OAS[b], cache_dir), on, _DAILY_MAX_AGE, f"ICE {b} OAS")
    return RateObservation(f"ICE BofA {b} OAS", d, v, ICE_SOURCE, _bucket_note(rating, b))


def corporate_yield(rating: str = "A", on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR) -> RateObservation:
    """ICE BofA effective yield for the rating's index."""
    b = rating_bucket(rating)
    d, v = _latest(load_fred(ICE_YIELD[b], cache_dir), on, _DAILY_MAX_AGE, f"ICE {b} yield")
    return RateObservation(f"ICE BofA {b} effective yield", d, v, ICE_SOURCE, _bucket_note(rating, b))


def spread_over_10y(rating: str = "A", on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR) -> RateObservation:
    """Index effective yield minus the UST 10Y on the same date: the engine's kd_spread."""
    y = corporate_yield(rating, on, cache_dir)
    t = rates.treasury_yield("10Y", on=y.date, cache_dir=cache_dir)
    notes = [n for n in (y.note, "" if t.date == y.date else f"UST 10Y from {t.date}") if n]
    b = rating_bucket(rating)
    return RateObservation(f"{b} yield minus UST 10Y", y.date, round(y.value - t.value, 6),
                           f"{ICE_SOURCE}; U.S. Treasury", "; ".join(notes))


# ---------------------------------------------------------------------------
# Moody's seasoned Aaa / Baa and the Treasury HQM corporate curve
# ---------------------------------------------------------------------------

def moodys_yield(grade: str = "Baa", on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR) -> RateObservation:
    """Moody's seasoned corporate bond yield, 'Aaa' or 'Baa' (20+ years to maturity)."""
    if grade not in MOODYS:
        raise ValueError(f"grade must be one of {sorted(MOODYS)}")
    d, v = _latest(load_fred(MOODYS[grade], cache_dir), on, _DAILY_MAX_AGE, f"Moody's {grade}")
    return RateObservation(f"Moody's seasoned {grade}", d, v, MOODYS_SOURCE)


def hqm_curve(on: date | None = None, maturities=HQM_MATURITIES, cache_dir: Path = DEFAULT_CACHE_DIR) -> dict:
    """Treasury HQM corporate spot curve for the latest month on or before ``on``:
    {"date": month start, "source", "maturities": {"10Y": decimal, ...}}."""
    series = {n: load_fred(f"HQMCB{n}YR", cache_dir) for n in maturities}
    month = min(_latest(s, on, _MONTHLY_MAX_AGE, f"HQM {n}Y")[0] for n, s in series.items())
    return {"date": month, "source": HQM_SOURCE,
            "maturities": {f"{n}Y": series[n][month] for n in maturities if month in series[n]}}


def credit_snapshot(on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR) -> dict:
    return {
        "ratings": {b: {"oas": credit_spread(b, on, cache_dir),
                        "yield": corporate_yield(b, on, cache_dir),
                        "spread_over_10y": spread_over_10y(b, on, cache_dir)} for b in RATINGS},
        "moodys": {g: moodys_yield(g, on, cache_dir) for g in MOODYS},
        "hqm_curve": hqm_curve(on, cache_dir=cache_dir),
    }


def cross_check_bonds(days: int = 90, cache_dir: Path = DEFAULT_CACHE_DIR) -> list[dict]:
    """Moody's yield minus the Treasury 10Y (our join) vs FRED AAA10Y / BAA10Y, which
    FRED calculates from DGS10. Both inputs have 2 decimals, so they must agree exactly."""
    start = date.fromordinal(date.today().toordinal() - days)
    ust = rates.treasury_history("10Y", start=start, cache_dir=cache_dir)
    out = []
    for grade, fid in MOODYS_SPREAD_10Y.items():
        y = load_fred(MOODYS[grade], cache_dir)
        ours = {d: round(y[d] - ust[d], 6) for d in ust if d in y}
        fred = {d: v for d, v in load_fred(fid, cache_dir).items() if d >= start}
        out.append(rates._compare(f"Moody's {grade} - UST 10Y", ours, fred, fid, 0.5e-4 + 1e-9))
    return out


# ---------------------------------------------------------------------------
# DCF hook
# ---------------------------------------------------------------------------

def with_kd_spread(model, obs: RateObservation):
    """Copy of a ``DriverModel`` whose ``kd_spread`` mean is ``obs.value``. The rationale
    records the source, the date and the replaced mean."""
    from dataclasses import replace
    kd = model.driver("kd_spread")
    new = replace(kd, mean=obs.value, rationale=f"{obs}; replaces the model mean {kd.mean:+.2%}. "
                                                f"Model rationale: {kd.rationale}")
    return replace(model, drivers=[new if d.name == "kd_spread" else d for d in model.drivers])


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--json", action="store_true", help="machine output (decimals, ISO dates)")
    common.add_argument("--date", help="YYYY-MM-DD or 'today' (default: latest)")
    p = argparse.ArgumentParser(prog="python -m financial_analysis.bonds", parents=[common],
                                description="Corporate bond yields and credit spreads (ICE BofA, Moody's, Treasury HQM).")
    sub = p.add_subparsers(dest="cmd")
    sub.add_parser("snapshot", parents=[common], help="every rating, Moody's, HQM curve")
    for name, help_ in (("spread", "ICE OAS for a rating"), ("yield", "ICE effective yield for a rating"),
                        ("over10y", "index yield minus UST 10Y: the engine's kd_spread")):
        s = sub.add_parser(name, parents=[common], help=help_)
        s.add_argument("--rating", default="A", help="S&P (A-) or Moody's (A3)")
    s = sub.add_parser("moodys", parents=[common], help="Moody's seasoned Aaa or Baa yield")
    s.add_argument("--grade", default="Baa", choices=sorted(MOODYS))
    sub.add_parser("hqm", parents=[common], help="Treasury HQM corporate spot curve")
    s = sub.add_parser("check", parents=[common], help="Moody's - UST 10Y vs FRED AAA10Y/BAA10Y")
    s.add_argument("--days", type=int, default=90)
    return p


def _run(args):
    cmd = args.cmd or "snapshot"
    on = rates.parse_date_arg(args.date)
    if cmd == "snapshot":
        return credit_snapshot(on)
    if cmd == "spread":
        return credit_spread(args.rating, on)
    if cmd == "yield":
        return corporate_yield(args.rating, on)
    if cmd == "over10y":
        return spread_over_10y(args.rating, on)
    if cmd == "moodys":
        return moodys_yield(args.grade, on)
    if cmd == "hqm":
        return hqm_curve(on)
    if cmd == "check":
        return cross_check_bonds(args.days)
    raise ValueError(f"unknown command {cmd}")


def _print_human(cmd: str, result) -> None:
    if cmd == "snapshot":
        print(f"{'':4}{'OAS':>8}{'yield':>8}{'- 10Y':>8}  date")
        for b, q in result["ratings"].items():
            print(f"{b:<4}{q['oas'].value:>8.2%}{q['yield'].value:>8.2%}{q['spread_over_10y'].value:>+8.2%}"
                  f"  {q['yield'].date}")
        print("(ICE BofA via FRED; '- 10Y' = index yield minus UST 10Y, the engine's kd_spread)\n")
        for obs in result["moodys"].values():
            print(obs)
        _print_human("hqm", result["hqm_curve"])
    elif cmd == "hqm":
        c = result
        print(f"HQM corporate spot curve, {c['date']:%b %Y} ({c['source']}):\n  "
              + "  ".join(f"{m}={v:.2%}" for m, v in c["maturities"].items()))
    elif cmd == "check":
        for c in result:
            status = "OK" if not c["mismatches"] else f"{len(c['mismatches'])} MISMATCHES {c['mismatches']}"
            print(f"{c['check']} vs FRED {c['fred_series']}: {c['compared']} points compared, {status}")
    else:
        print(result)


def main(argv: list[str] | None = None) -> int:
    logging.basicConfig(level=logging.WARNING)
    args = _build_parser().parse_args(argv)
    cmd = args.cmd or "snapshot"
    try:
        result = _run(args)
        if args.json:
            print(json.dumps(rates._jsonable(result)))
            return 0
        _print_human(cmd, result)
        if args.cmd is None:
            print()
            _print_human("check", cross_check_bonds())
        return 0
    except Exception as e:  # noqa: BLE001 - CLI boundary: report, do not traceback
        msg = {"error": str(e), "type": type(e).__name__}
        print(json.dumps(msg) if args.json else f"error: {msg['type']}: {msg['error']}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
