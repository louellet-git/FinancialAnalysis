"""US interest rates and inflation from official, free, keyless sources.

Primary sources: the publishers of record, not aggregators.

- U.S. Treasury Daily Par Yield Curve (nominal, 1M-30Y, from 1990). This is the
  canonical risk-free curve. FRED's DGSxx series republish these same numbers.
- U.S. Treasury Daily Par Real Yield Curve (TIPS, 5Y-30Y, from 2003).
- Federal Reserve Bank of New York reference rates: SOFR, EFFR (with the FOMC
  target range), OBFR, TGCR, BGCR.
- Bureau of Labor Statistics CPI-U (headline and core, U.S. city average, not
  seasonally adjusted), public API v1.

Derived inflation measures (same formulas as FRED, so they cross-check exactly):

- Breakeven inflation = nominal par yield - real (TIPS) par yield, same date
  (FRED T5YIE, T10YIE). It contains an inflation risk premium and a TIPS
  liquidity premium, so it is an approximation of expected CPI inflation.
- 5y5y forward inflation = ((1 + be10)^10 / (1 + be5)^5)^(1/5) - 1 (FRED T5YIFR).
  This is the market's expected inflation for years 6-10. It is the better anchor
  for terminal growth because it excludes near-term noise.

Cross-check and long history: FRED (St. Louis Fed) public fredgraph CSV, no API key.

UNITS: every rate returned here is a DECIMAL (0.0507 == 5.07%), matching the
engine convention (``Driver("rf", mean=0.046)``). The sources publish percent.

CLI (``--json`` on any subcommand gives machine output for the Rust crate in ``rust/``)::

    python -m financial_analysis.rates                     # snapshot + FRED cross-check
    python -m financial_analysis.rates yield --tenor 10Y --date 2026-07-16 --json
    python -m financial_analysis.rates inflation --json
"""
from __future__ import annotations

import argparse
import csv
import io
import json
import logging
import re
import sys
import time
from dataclasses import asdict, dataclass
from datetime import date, datetime, timedelta
from pathlib import Path

import requests

logger = logging.getLogger(__name__)

DEFAULT_CACHE_DIR = Path(__file__).resolve().parents[1] / "data" / "raw"
_UA = {"User-Agent": "financial-analysis-rates/1.0"}
_TIMEOUT = 30

TREASURY_CSV = (
    "https://home.treasury.gov/resource-center/data-chart-center/interest-rates/"
    "daily-treasury-rates.csv/{year}/all?type={kind}&field_tdr_date_value={year}&page&_format=csv"
)
_TREASURY_KIND = {False: "daily_treasury_yield_curve", True: "daily_treasury_real_yield_curve"}
TREASURY_FIRST_YEAR = {False: 1990, True: 2003}

NYFED_URL = "https://markets.newyorkfed.org/api/rates/{group}/{rate}/search.json?startDate={start}&endDate={end}"
NYFED_GROUP = {"SOFR": "secured", "TGCR": "secured", "BGCR": "secured",
               "EFFR": "unsecured", "OBFR": "unsecured"}

BLS_URL = "https://api.bls.gov/publicAPI/v1/timeseries/data/"
CPI_SERIES = {"headline": "CUUR0000SA0", "core": "CUUR0000SA0L1E"}

FRED_CSV = "https://fred.stlouisfed.org/graph/fredgraph.csv?id={series}&cosd={start}"

# Canonical tenor keys in curve order.
TENORS = ["1M", "1.5M", "2M", "3M", "4M", "6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "20Y", "30Y"]
BREAKEVEN_TENORS = ("5Y", "7Y", "10Y", "20Y", "30Y")

# FRED republication of the Treasury and BLS data. Used only for cross-checking.
FRED_NOMINAL = {"1M": "DGS1MO", "3M": "DGS3MO", "6M": "DGS6MO", "1Y": "DGS1", "2Y": "DGS2",
                "3Y": "DGS3", "5Y": "DGS5", "7Y": "DGS7", "10Y": "DGS10", "20Y": "DGS20",
                "30Y": "DGS30"}
FRED_REAL = {"5Y": "DFII5", "7Y": "DFII7", "10Y": "DFII10", "20Y": "DFII20", "30Y": "DFII30"}
FRED_CPI = {"headline": "CPIAUCNS", "core": "CPILFENS"}


@dataclass(frozen=True)
class RateObservation:
    series: str      # e.g. "UST 10Y", "UST real 10Y", "SOFR", "Breakeven inflation 10Y"
    date: date       # the observation date (NOT the date you asked for)
    value: float     # decimal: 0.0507 == 5.07%
    source: str      # publisher of record
    note: str = ""   # caveat the caller must see, e.g. a data gap that moved the date

    def __str__(self) -> str:
        s = f"{self.series} {self.value:.2%} on {self.date.isoformat()} ({self.source})"
        return f"{s} [{self.note}]" if self.note else s

    def to_dict(self) -> dict:
        d = asdict(self)
        d["date"] = self.date.isoformat()
        return d


# ---------------------------------------------------------------------------
# HTTP
# ---------------------------------------------------------------------------

def _http(url: str, payload: dict | None = None) -> str:
    last: Exception | None = None
    for attempt in range(3):
        try:
            if payload is None:
                r = requests.get(url, headers=_UA, timeout=_TIMEOUT)
            else:
                r = requests.post(url, json=payload, headers=_UA, timeout=_TIMEOUT)
            r.raise_for_status()
            return r.text
        except requests.RequestException as e:
            last = e
            time.sleep(1.5 * (attempt + 1))
    raise RuntimeError(f"fetch failed after 3 attempts: {url}") from last


def _http_get(url: str) -> str:
    return _http(url)


def _http_post(url: str, payload: dict) -> str:
    return _http(url, payload)


def _pct_to_decimal(raw: str) -> float | None:
    raw = raw.strip()
    if raw in ("", ".", "N/A", "NA", "ND"):
        return None
    return round(float(raw) / 100.0, 6)


def _is_fresh(path: Path, max_age_hours: float) -> bool:
    return path.exists() and (time.time() - path.stat().st_mtime) < max_age_hours * 3600


# ---------------------------------------------------------------------------
# U.S. Treasury par yield curves
# ---------------------------------------------------------------------------

def normalize_tenor(header: str) -> str:
    """'1 Mo' / '1.5 Month' / '10 Yr' / '10 YR' -> '1M' / '1.5M' / '10Y' / '10Y'."""
    m = re.fullmatch(r"\s*([\d.]+)\s*(mo|month|months|yr|year|years)\s*", header, re.IGNORECASE)
    if not m:
        raise ValueError(f"unrecognized Treasury tenor header: {header!r}")
    num, unit = m.groups()
    return f"{num}{'M' if unit.lower().startswith('mo') else 'Y'}"


def parse_treasury_csv(text: str) -> dict[date, dict[str, float]]:
    """Parse a Treasury daily-rates CSV into {date: {tenor: decimal}}.
    Blank cells are omitted. Example: 30Y was not issued from 2002 to 2006."""
    reader = csv.reader(io.StringIO(text.lstrip("﻿")))
    header = next(reader, [])
    if len(header) < 2 or header[0].strip() != "Date":
        raise ValueError(f"not a Treasury rates CSV (header: {header[:3]!r})")
    tenors = [normalize_tenor(h) for h in header[1:]]
    out: dict[date, dict[str, float]] = {}
    for row in reader:
        if not row or not row[0].strip():
            continue
        d = datetime.strptime(row[0].strip(), "%m/%d/%Y").date()
        curve = {}
        for tenor, cell in zip(tenors, row[1:]):
            v = _pct_to_decimal(cell)
            if v is not None:
                curve[tenor] = v
        out[d] = curve
    return out


def load_treasury_curve(
    year: int,
    cache_dir: Path = DEFAULT_CACHE_DIR,
    real: bool = False,
    refresh: bool = False,
    max_age_hours: float = 6.0,
) -> dict[date, dict[str, float]]:
    """All daily curves for one calendar year: {date: {tenor: decimal}}.

    Past years are cached indefinitely. The current year is refetched once the cache
    is older than ``max_age_hours``. If a refetch fails and a cache exists, the stale
    cache is used with a warning. The observation dates show the staleness."""
    kind = _TREASURY_KIND[real]
    path = cache_dir / "rates" / f"treasury_{'real' if real else 'nominal'}_{year}.csv"
    current = year >= date.today().year
    use_cache = path.exists() and not refresh and (not current or _is_fresh(path, max_age_hours))
    if use_cache:
        return parse_treasury_csv(path.read_text(encoding="utf-8"))
    try:
        text = _http_get(TREASURY_CSV.format(year=year, kind=kind))
        parsed = parse_treasury_csv(text)  # validate before caching
    except Exception:
        if path.exists():
            logger.warning("Treasury fetch failed for %s %s; using cached copy", kind, year)
            return parse_treasury_csv(path.read_text(encoding="utf-8"))
        raise
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return parsed


def treasury_curve(
    on: date | None = None,
    real: bool = False,
    cache_dir: Path = DEFAULT_CACHE_DIR,
    refresh: bool = False,
) -> tuple[date, dict[str, float]]:
    """The most recent full curve on or before ``on`` (default: today).
    Weekends and holidays fall back to the prior business day, across year-end if needed."""
    on = on or date.today()
    for year in (on.year, on.year - 1):
        if year < TREASURY_FIRST_YEAR[real]:
            break
        curves = load_treasury_curve(year, cache_dir, real=real, refresh=refresh)
        eligible = [d for d in curves if d <= on]
        if eligible:
            d = max(eligible)
            return d, curves[d]
    raise LookupError(f"no Treasury {'real' if real else 'nominal'} curve on or before {on}")


def treasury_yield(
    tenor: str = "10Y",
    on: date | None = None,
    real: bool = False,
    cache_dir: Path = DEFAULT_CACHE_DIR,
    refresh: bool = False,
) -> RateObservation:
    """One tenor's par yield on or before ``on``. Example: treasury_yield("10Y").value -> 0.0507."""
    d, curve = treasury_curve(on, real=real, cache_dir=cache_dir, refresh=refresh)
    if tenor not in curve:
        raise KeyError(f"tenor {tenor} not published on {d}; available: {sorted(curve, key=TENORS.index)}")
    label = f"UST {'real ' if real else ''}{tenor}"
    return RateObservation(label, d, curve[tenor], "U.S. Treasury")


def treasury_history(
    tenor: str = "10Y",
    start: date | None = None,
    end: date | None = None,
    real: bool = False,
    cache_dir: Path = DEFAULT_CACHE_DIR,
) -> dict[date, float]:
    """Daily series for one tenor, {date: decimal}, from ``start`` to ``end`` inclusive."""
    end = end or date.today()
    start = start or date(end.year, 1, 1)
    out: dict[date, float] = {}
    for year in range(max(start.year, TREASURY_FIRST_YEAR[real]), end.year + 1):
        for d, curve in load_treasury_curve(year, cache_dir, real=real).items():
            if start <= d <= end and tenor in curve:
                out[d] = curve[tenor]
    return dict(sorted(out.items()))


# ---------------------------------------------------------------------------
# Market-implied inflation (from the two Treasury curves)
# ---------------------------------------------------------------------------

def _same_day_curves(
    on: date | None, cache_dir: Path, refresh: bool = False
) -> tuple[date, dict[str, float], dict[str, float]]:
    """(date, nominal curve, real curve) for the latest date <= ``on`` that has both curves."""
    d, _ = treasury_curve(on, real=True, cache_dir=cache_dir, refresh=refresh)
    for _ in range(10):
        nd, nom = treasury_curve(d, real=False, cache_dir=cache_dir, refresh=refresh)
        rd, real = treasury_curve(nd, real=True, cache_dir=cache_dir, refresh=refresh)
        if nd == rd:
            return nd, nom, real
        d = min(nd, rd)
    raise LookupError(f"no date on or before {on} has both Treasury curves")


def _breakeven(nom: dict[str, float], real: dict[str, float], tenor: str) -> float:
    if tenor not in nom or tenor not in real:
        raise KeyError(f"breakeven needs {tenor} on both curves; tenors: {BREAKEVEN_TENORS}")
    return round(nom[tenor] - real[tenor], 6)


def _forward_5y5y(be5: float, be10: float) -> float:
    return round(((1 + be10) ** 10 / (1 + be5) ** 5) ** 0.2 - 1, 6)


def breakeven_inflation(
    tenor: str = "10Y", on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR,
) -> RateObservation:
    """Market-implied average CPI inflation over ``tenor``: nominal minus TIPS par yield."""
    d, nom, real = _same_day_curves(on, cache_dir)
    return RateObservation(f"Breakeven inflation {tenor}", d, _breakeven(nom, real, tenor),
                           "U.S. Treasury (nominal minus TIPS par yield)")


def forward_inflation_5y5y(on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR) -> RateObservation:
    """Market-implied average inflation for years 6-10 (FRED T5YIFR formula)."""
    d, nom, real = _same_day_curves(on, cache_dir)
    v = _forward_5y5y(_breakeven(nom, real, "5Y"), _breakeven(nom, real, "10Y"))
    return RateObservation("5y5y forward inflation", d, v, "U.S. Treasury (from 5Y and 10Y breakevens)")


# ---------------------------------------------------------------------------
# Realized inflation: BLS CPI-U
# ---------------------------------------------------------------------------

def parse_bls_json(text: str) -> dict[str, dict[date, float]]:
    """{series_id: {month_start: index}}. Skips M13 (annual average) and missing values
    ('-'), for example October 2025, which BLS did not collect (2025 lapse in appropriations)."""
    d = json.loads(text)
    if d.get("status") != "REQUEST_SUCCEEDED":
        raise ValueError(f"BLS request failed: {d.get('status')} {d.get('message')}")
    out: dict[str, dict[date, float]] = {}
    for s in d["Results"]["series"]:
        rows = {}
        for x in s["data"]:
            p = x["period"]
            if not re.fullmatch(r"M(0[1-9]|1[0-2])", p):
                continue
            v = x["value"].strip()
            if v in ("", "-"):
                continue
            rows[date(int(x["year"]), int(p[1:]), 1)] = float(v)
        out[s["seriesID"]] = dict(sorted(rows.items()))
    return out


def load_cpi(
    cache_dir: Path = DEFAULT_CACHE_DIR, refresh: bool = False, max_age_hours: float = 24.0,
) -> dict[str, dict[date, float]]:
    """CPI-U index levels for the last 10 years, headline and core, in one BLS request.
    The cache lasts ``max_age_hours`` because API v1 allows 25 requests per day."""
    path = cache_dir / "rates" / "bls_cpi.json"
    if path.exists() and not refresh and _is_fresh(path, max_age_hours):
        return parse_bls_json(path.read_text(encoding="utf-8"))
    end = date.today().year
    payload = {"seriesid": list(CPI_SERIES.values()), "startyear": str(end - 9), "endyear": str(end)}
    try:
        text = _http_post(BLS_URL, payload)
        parsed = parse_bls_json(text)  # validate before caching
    except Exception:
        if path.exists():
            logger.warning("BLS fetch failed; using cached CPI")
            return parse_bls_json(path.read_text(encoding="utf-8"))
        raise
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return parsed


def _year_before(m: date) -> date:
    return date(m.year - 1, m.month, 1)


def cpi_inflation(
    kind: str = "headline", on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR,
) -> RateObservation:
    """12-month CPI-U change for the latest month <= ``on`` that has a value 12 months
    before. If a gap moves the month back, ``note`` says so.

    The date is the reference month. BLS publishes it about two weeks after the month
    ends, so for a past ``on`` this can use a value that was not yet public on that date."""
    if kind not in CPI_SERIES:
        raise ValueError(f"kind must be one of {sorted(CPI_SERIES)}")
    on = on or date.today()
    idx = load_cpi(cache_dir)[CPI_SERIES[kind]]
    months = [m for m in idx if m <= on]
    if not months:
        raise LookupError(f"no CPI data on or before {on}")
    latest = max(months)
    usable = [m for m in months if _year_before(m) in idx]
    if not usable:
        raise LookupError(f"no 12-month CPI change on or before {on}")
    m = max(usable)
    note = ""
    if m != latest:
        note = (f"latest month {latest:%b %Y} has no 12-month change: "
                f"BLS has no index for {_year_before(latest):%b %Y}")
    label = "CPI-U 12-month" if kind == "headline" else "Core CPI-U 12-month"
    return RateObservation(label, m, round(idx[m] / idx[_year_before(m)] - 1, 6),
                           "Bureau of Labor Statistics", note)


def inflation_snapshot(on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR) -> dict:
    """Market-implied and realized inflation on or before ``on``, as RateObservations."""
    d, nom, real = _same_day_curves(on, cache_dir)
    src = "U.S. Treasury (nominal minus TIPS par yield)"
    be = {t: _breakeven(nom, real, t) for t in BREAKEVEN_TENORS if t in nom and t in real}
    out = {f"breakeven_{t.lower()}": RateObservation(f"Breakeven inflation {t}", d, v, src)
           for t, v in be.items()}
    out["forward_5y5y"] = RateObservation("5y5y forward inflation", d, _forward_5y5y(be["5Y"], be["10Y"]),
                                          "U.S. Treasury (from 5Y and 10Y breakevens)")
    out["real_10y"] = RateObservation("UST real 10Y", d, real["10Y"], "U.S. Treasury")
    out["cpi_12m"] = cpi_inflation("headline", on, cache_dir)
    out["core_cpi_12m"] = cpi_inflation("core", on, cache_dir)
    return out


# ---------------------------------------------------------------------------
# New York Fed reference rates
# ---------------------------------------------------------------------------

def parse_nyfed_json(text: str) -> list[dict]:
    rows = []
    for r in json.loads(text).get("refRates", []):
        rows.append({
            "date": date.fromisoformat(r["effectiveDate"]),
            "type": r["type"],
            "value": round(float(r["percentRate"]) / 100.0, 6),
            "target_from": round(float(r["targetRateFrom"]) / 100.0, 6) if "targetRateFrom" in r else None,
            "target_to": round(float(r["targetRateTo"]) / 100.0, 6) if "targetRateTo" in r else None,
        })
    return sorted(rows, key=lambda x: x["date"])


def nyfed_history(rate: str = "SOFR", start: date | None = None, end: date | None = None) -> list[dict]:
    """Daily NY Fed reference rate rows (decimals). SOFR starts 2018-04-02. EFFR rows also
    carry the FOMC target range as ``target_from`` and ``target_to``."""
    rate = rate.upper()
    if rate not in NYFED_GROUP:
        raise ValueError(f"unknown NY Fed rate {rate!r}; choose from {sorted(NYFED_GROUP)}")
    end = end or date.today()
    start = start or end - timedelta(days=14)
    url = NYFED_URL.format(group=NYFED_GROUP[rate], rate=rate.lower(), start=start, end=end)
    return parse_nyfed_json(_http_get(url))


def nyfed_rate(rate: str = "SOFR", on: date | None = None) -> RateObservation:
    """Latest published NY Fed reference rate on or before ``on``. Note that SOFR for day T
    is published on T+1 morning, so 'today' resolves to the prior business day."""
    on = on or date.today()
    rows = [r for r in nyfed_history(rate, on - timedelta(days=14), on) if r["date"] <= on]
    if not rows:
        raise LookupError(f"no {rate} observation in the 14 days up to {on}")
    r = rows[-1]
    return RateObservation(rate.upper(), r["date"], r["value"], "Federal Reserve Bank of New York")


# ---------------------------------------------------------------------------
# FRED (cross-check and long history; no key needed for the CSV endpoint)
# ---------------------------------------------------------------------------

def parse_fred_csv(text: str, percent: bool = True) -> dict[date, float]:
    """{date: value}. ``percent=True`` converts percent to decimal; False keeps raw levels
    (for index series such as CPIAUCNS)."""
    reader = csv.reader(io.StringIO(text.lstrip("﻿")))
    next(reader)  # header: observation_date,<SERIES>
    out = {}
    for row in reader:
        if len(row) < 2 or row[1].strip() in ("", "."):
            continue
        out[date.fromisoformat(row[0])] = _pct_to_decimal(row[1]) if percent else float(row[1])
    return out


def fred_series(series_id: str, start: date | None = None, percent: bool = True) -> dict[date, float]:
    """Any FRED series as {date: value}. Examples: DGS10, DFII10, T10YIE (percent), CPIAUCNS (raw)."""
    start = start or date.today() - timedelta(days=30)
    text = _http_get(FRED_CSV.format(series=series_id, start=start.isoformat()))
    return parse_fred_csv(text, percent=percent)


def _compare(label: str, ours: dict[date, float], fred: dict[date, float], fred_id: str,
             tol: float) -> dict:
    common = sorted(set(ours) & set(fred))
    bad = [(d.isoformat(), ours[d], fred[d]) for d in common if abs(ours[d] - fred[d]) > tol]
    return {"check": label, "fred_series": fred_id, "compared": len(common), "mismatches": bad}


def cross_check(
    tenor: str = "10Y",
    real: bool = False,
    days: int = 30,
    tol_bp: float = 0.5,
    cache_dir: Path = DEFAULT_CACHE_DIR,
) -> dict:
    """Compare Treasury's own curve to FRED's republication over the last ``days``.
    FRED can lag Treasury by one day, so only dates that both sources have are compared."""
    fred_id = (FRED_REAL if real else FRED_NOMINAL)[tenor]
    start = date.today() - timedelta(days=days)
    ust = treasury_history(tenor, start=start, real=real, cache_dir=cache_dir)
    label = f"UST {'real ' if real else ''}{tenor}"
    return _compare(label, ust, fred_series(fred_id, start=start), fred_id, tol_bp / 10_000)


def cross_check_inflation(days: int = 30, cache_dir: Path = DEFAULT_CACHE_DIR) -> list[dict]:
    """Our breakevens and 5y5y vs FRED T10YIE / T5YIFR (FRED rounds to 0.01%), and BLS CPI
    index levels vs FRED CPIAUCNS / CPILFENS (exact)."""
    start = date.today() - timedelta(days=days)
    nom10 = treasury_history("10Y", start=start, cache_dir=cache_dir)
    real10 = treasury_history("10Y", start=start, real=True, cache_dir=cache_dir)
    nom5 = treasury_history("5Y", start=start, cache_dir=cache_dir)
    real5 = treasury_history("5Y", start=start, real=True, cache_dir=cache_dir)
    days_all = sorted(set(nom10) & set(real10) & set(nom5) & set(real5))
    be10 = {d: round(nom10[d] - real10[d], 6) for d in days_all}
    fwd = {d: round(_forward_5y5y(nom5[d] - real5[d], be10[d]), 4) for d in days_all}
    out = [
        _compare("Breakeven 10Y", be10, fred_series("T10YIE", start), "T10YIE", 0.5e-4),
        _compare("5y5y forward", fwd, fred_series("T5YIFR", start), "T5YIFR", 0.5e-4 + 1e-9),
    ]
    cpi_start = date(date.today().year - 2, 1, 1)
    cpi = load_cpi(cache_dir)
    for kind, fred_id in FRED_CPI.items():
        ours = {m: v for m, v in cpi[CPI_SERIES[kind]].items() if m >= cpi_start}
        out.append(_compare(f"CPI-U {kind} index", ours, fred_series(fred_id, cpi_start, percent=False),
                            fred_id, 1e-9))
    return out


# ---------------------------------------------------------------------------
# DCF hook
# ---------------------------------------------------------------------------

def with_risk_free(model, obs: RateObservation):
    """Copy of a driver-engine ``DriverModel`` whose ``rf`` driver mean is ``obs.value``.
    Std, bounds, distribution and correlations do not change. The rationale records
    the source, the date and the replaced mean."""
    from dataclasses import replace
    rf = model.driver("rf")
    new = replace(rf, mean=obs.value,
                  rationale=f"{obs}; replaces the model mean {rf.mean:.2%}. Model rationale: {rf.rationale}")
    return replace(model, drivers=[new if d.name == "rf" else d for d in model.drivers])


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _jsonable(x):
    if isinstance(x, RateObservation):
        return x.to_dict()
    if isinstance(x, date):
        return x.isoformat()
    if isinstance(x, dict):
        return {(k.isoformat() if isinstance(k, date) else k): _jsonable(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [_jsonable(v) for v in x]
    return x


def parse_date_arg(s: str | None) -> date | None:
    if s is None or s.lower() == "today":
        return None if s is None else date.today()
    return date.fromisoformat(s)


def _fmt_curve(curve: dict[str, float]) -> str:
    return "  ".join(f"{t}={curve[t]:.2%}" for t in TENORS if t in curve)


def snapshot(on: date | None = None, cache_dir: Path = DEFAULT_CACHE_DIR) -> dict:
    d, nom = treasury_curve(on, cache_dir=cache_dir)
    dr, real = treasury_curve(on, real=True, cache_dir=cache_dir)
    effr_rows = [r for r in nyfed_history("EFFR", end=on) if r["date"] <= (on or date.today())]
    return {
        "nominal_curve": {"date": d, "source": "U.S. Treasury", "tenors": nom},
        "real_curve": {"date": dr, "source": "U.S. Treasury", "tenors": real},
        "sofr": nyfed_rate("SOFR", on),
        "effr": nyfed_rate("EFFR", on),
        "fomc_target": {"from": effr_rows[-1]["target_from"], "to": effr_rows[-1]["target_to"]},
        "inflation": inflation_snapshot(on, cache_dir),
    }


def _print_snapshot(s: dict) -> None:
    n, r = s["nominal_curve"], s["real_curve"]
    print(f"U.S. Treasury nominal par curve, {n['date']}:\n  {_fmt_curve(n['tenors'])}")
    print(f"U.S. Treasury real (TIPS) par curve, {r['date']}:\n  {_fmt_curve(r['tenors'])}\n")
    print(s["sofr"])
    print(s["effr"])
    print(f"FOMC target range: {s['fomc_target']['from']:.2%}-{s['fomc_target']['to']:.2%}\n")
    print("Inflation:")
    for obs in s["inflation"].values():
        print(f"  {obs}")


def _build_parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--json", action="store_true", help="machine output (decimals, ISO dates)")
    p = argparse.ArgumentParser(prog="python -m financial_analysis.rates", parents=[common],
                                description="US rates and inflation from Treasury, NY Fed, BLS; FRED cross-check.")
    sub = p.add_subparsers(dest="cmd")
    s = sub.add_parser("snapshot", parents=[common], help="curves, SOFR, EFFR, FOMC range, inflation")
    s.add_argument("--date", help="YYYY-MM-DD or 'today' (default: latest)")
    s = sub.add_parser("yield", parents=[common], help="one Treasury par yield")
    s.add_argument("--tenor", default="10Y")
    s.add_argument("--real", action="store_true")
    s.add_argument("--date")
    s = sub.add_parser("curve", parents=[common], help="one full Treasury curve")
    s.add_argument("--real", action="store_true")
    s.add_argument("--date")
    s = sub.add_parser("history", parents=[common], help="daily Treasury series for one tenor")
    s.add_argument("--tenor", default="10Y")
    s.add_argument("--real", action="store_true")
    s.add_argument("--start", required=True)
    s.add_argument("--end")
    s = sub.add_parser("nyfed", parents=[common], help="SOFR, EFFR, OBFR, TGCR or BGCR")
    s.add_argument("--rate", default="SOFR")
    s.add_argument("--date")
    s = sub.add_parser("inflation", parents=[common], help="breakevens, 5y5y forward, CPI")
    s.add_argument("--date")
    s = sub.add_parser("fred", parents=[common], help="any FRED series")
    s.add_argument("--series", required=True)
    s.add_argument("--start")
    s.add_argument("--raw", action="store_true", help="keep levels (index series), no percent conversion")
    s = sub.add_parser("check", parents=[common], help="cross-check every source against FRED")
    s.add_argument("--days", type=int, default=30)
    return p


def _run(args) -> object:
    cmd = args.cmd or "snapshot"
    on = parse_date_arg(getattr(args, "date", None))
    if cmd == "snapshot":
        return snapshot(on)
    if cmd == "yield":
        return treasury_yield(args.tenor, on, real=args.real)
    if cmd == "curve":
        d, c = treasury_curve(on, real=args.real)
        return {"date": d, "source": "U.S. Treasury", "real": args.real, "tenors": c}
    if cmd == "history":
        return treasury_history(args.tenor, parse_date_arg(args.start), parse_date_arg(args.end), real=args.real)
    if cmd == "nyfed":
        return nyfed_rate(args.rate, on)
    if cmd == "inflation":
        return inflation_snapshot(on)
    if cmd == "fred":
        return fred_series(args.series, parse_date_arg(args.start), percent=not args.raw)
    if cmd == "check":
        out = [cross_check(t, real=r, days=args.days) for t, r in (("10Y", False), ("2Y", False), ("10Y", True))]
        return out + cross_check_inflation(args.days)
    raise ValueError(f"unknown command {cmd}")


def _print_human(cmd: str, result) -> None:
    if cmd == "snapshot":
        _print_snapshot(result)
    elif cmd == "curve":
        print(f"{result['date']}: {_fmt_curve(result['tenors'])}")
    elif cmd == "inflation":
        for obs in result.values():
            print(obs)
    elif cmd == "check":
        for c in result:
            status = "OK" if not c["mismatches"] else f"{len(c['mismatches'])} MISMATCHES {c['mismatches']}"
            print(f"{c['check']} vs FRED {c['fred_series']}: {c['compared']} points compared, {status}")
    elif isinstance(result, dict):
        for k, v in result.items():
            print(f"{k.isoformat()},{v}")
    else:
        print(result)


def main(argv: list[str] | None = None) -> int:
    logging.basicConfig(level=logging.WARNING)
    args = _build_parser().parse_args(argv)
    cmd = args.cmd or "snapshot"
    try:
        result = _run(args)
        if args.json:
            print(json.dumps(_jsonable(result)))
            return 0
        _print_human(cmd, result)
        if args.cmd is None:  # bare invocation: snapshot plus the FRED cross-check
            print()
            _print_human("check", _run(_build_parser().parse_args(["check"])))
        return 0
    except Exception as e:  # noqa: BLE001 - CLI boundary: report, do not traceback
        if args.json:
            print(json.dumps({"error": str(e), "type": type(e).__name__}), file=sys.stderr)
        else:
            print(f"error: {type(e).__name__}: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
