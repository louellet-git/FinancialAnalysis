import json
import re
import time
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

USER_AGENT = "Luc Ouellet ouellet.luc1@gmail.com"
COMPANY_TICKER_URL = "https://www.sec.gov/files/company_tickers.json"
SUBMISSIONS_URL = "https://data.sec.gov/submissions/CIK{cik}.json"
INDEX_BASE_URL = "https://www.sec.gov/Archives/edgar/full-index"
MASTER_INDEX_FILENAME = "master.idx"
DEFAULT_REQUEST_DELAY = 0.15
_last_request_time: Optional[float] = None


class EdgarClientError(Exception):
    pass


def _http_get(url: str, request_delay: float = DEFAULT_REQUEST_DELAY) -> bytes:
    global _last_request_time
    if _last_request_time is not None:
        elapsed = time.monotonic() - _last_request_time
        if elapsed < request_delay:
            time.sleep(request_delay - elapsed)

    headers = {
        "User-Agent": USER_AGENT,
        "Accept": "application/json, text/plain, */*",
        "Accept-Language": "en-US,en;q=0.9",
        "Connection": "close",
    }
    request = Request(url, headers=headers)
    try:
        with urlopen(request, timeout=30) as response:
            data = response.read()
    except HTTPError as exc:
        raise EdgarClientError(f"HTTP error {exc.code} for {url}") from exc
    except URLError as exc:
        raise EdgarClientError(f"URL error for {url}: {exc.reason}") from exc
    finally:
        _last_request_time = time.monotonic()

    return data


def _parse_date(value: Optional[str]) -> Optional[date]:
    if not value:
        return None
    for fmt in ("%Y-%m-%d", "%Y%m%d", "%m/%d/%Y"):
        try:
            return datetime.strptime(value, fmt).date()
        except ValueError:
            continue
    return None


def _zero_pad_cik(cik: str) -> str:
    return cik.zfill(10)


def _safe_filename(text: str) -> str:
    return re.sub(r"[^0-9A-Za-z._-]", "_", text)


def get_cik_for_ticker(ticker: str, request_delay: float = DEFAULT_REQUEST_DELAY) -> str:
    ticker_upper = ticker.strip().upper()

    payload = _http_get(COMPANY_TICKER_URL, request_delay=request_delay)
    raw = json.loads(payload)
    # SEC company_tickers.json is a dict with numeric keys; iterate over values
    for record in raw.values():
        if record.get("ticker", "").upper() == ticker_upper:
            cik_str = str(record.get("cik_str", "")).strip()
            if cik_str:
                return cik_str
    raise EdgarClientError(f"CIK not found for ticker {ticker_upper}")


def get_company_submissions(ticker: str, request_delay: float = DEFAULT_REQUEST_DELAY) -> Dict[str, Any]:
    cik = get_cik_for_ticker(ticker, request_delay=request_delay)
    padded = _zero_pad_cik(cik)
    url = SUBMISSIONS_URL.format(cik=padded)
    payload = _http_get(url, request_delay=request_delay)
    return json.loads(payload)


def _quarter_for_date(target_date: date) -> int:
    return (target_date.month - 1) // 3 + 1


def _quarter_range(start_date: date, end_date: date) -> Iterable[tuple[int, int]]:
    current_year = start_date.year
    current_quarter = _quarter_for_date(start_date)
    end_year = end_date.year
    end_quarter = _quarter_for_date(end_date)

    while current_year < end_year or (current_year == end_year and current_quarter <= end_quarter):
        yield current_year, current_quarter
        current_quarter += 1
        if current_quarter > 4:
            current_quarter = 1
            current_year += 1


def _master_index_url(year: int, quarter: int) -> str:
    return f"{INDEX_BASE_URL}/{year}/QTR{quarter}/{MASTER_INDEX_FILENAME}"


def download_master_index(year: int, quarter: int, output_dir: Path, request_delay: float = DEFAULT_REQUEST_DELAY) -> Path:
    output_dir.mkdir(parents=True, exist_ok=True)
    local_path = output_dir / f"{year}_QTR{quarter}_master.idx"
    if local_path.exists():
        return local_path

    url = _master_index_url(year, quarter)
    content = _http_get(url, request_delay=request_delay)
    local_path.write_bytes(content)
    return local_path


def parse_master_index(index_path: Path) -> List[Dict[str, Any]]:
    content = index_path.read_text(encoding="latin1")
    lines = content.splitlines()
    entries: List[Dict[str, Any]] = []
    header_found = False

    for line in lines:
        if not header_found:
            if line.startswith("CIK|"):
                header_found = True
            continue
        if not line.strip():
            continue

        columns = line.split("|")
        if len(columns) < 5:
            continue

        cik = columns[0].strip().lstrip("0")
        entries.append(
            {
                "cik": cik,
                "company_name": columns[1].strip(),
                "form": columns[2].strip(),
                "date_filed": _parse_date(columns[3].strip()),
                "filename": columns[4].strip(),
            }
        )

    return entries


def _select_index_entries(
    entries: Iterable[Dict[str, Any]],
    cik: str,
    valuation_date: date,
    lookback_months: int = 18,
    lookahead_months: int = 6,
    forms: Optional[List[str]] = None,
) -> List[Dict[str, Any]]:
    window_start = valuation_date - timedelta(days=lookback_months * 30)
    window_end = valuation_date + timedelta(days=lookahead_months * 30)
    results: List[Dict[str, Any]] = []

    for entry in entries:
        if entry["cik"] != cik:
            continue
        if not entry["date_filed"]:
            continue
        if not (window_start <= entry["date_filed"] <= window_end):
            continue
        if forms and entry["form"] not in forms:
            continue
        results.append(entry)

    return results


def _download_index_file(entry: Dict[str, Any], output_dir: Path, request_delay: float = DEFAULT_REQUEST_DELAY) -> Optional[Path]:
    filename = entry["filename"]
    target_name = _safe_filename(Path(filename).name)
    target_path = output_dir / target_name
    if target_path.exists():
        return target_path

    url = f"https://www.sec.gov/Archives/{filename}"
    try:
        data = _http_get(url, request_delay=request_delay)
    except EdgarClientError:
        return None

    target_path.parent.mkdir(parents=True, exist_ok=True)
    target_path.write_bytes(data)
    return target_path


def download_filings_for_ticker(
    ticker: str,
    valuation_date: date,
    output_root: Path,
    lookback_months: int = 18,
    lookahead_months: int = 6,
    allowed_extensions: Optional[List[str]] = None,
    request_delay: float = DEFAULT_REQUEST_DELAY,
) -> Dict[str, Any]:
    cik = get_cik_for_ticker(ticker, request_delay=request_delay)
    window_start = valuation_date - timedelta(days=lookback_months * 30)
    window_end = valuation_date + timedelta(days=lookahead_months * 30)

    quarter_start = _quarter_for_date(window_start)
    quarter_end = _quarter_for_date(window_end)
    output_dir = output_root / ticker.upper()
    output_dir.mkdir(parents=True, exist_ok=True)

    index_dir = output_dir / "indexes"
    selected: List[Dict[str, Any]] = []
    downloaded_files: List[str] = []

    for year, quarter in _quarter_range(window_start, window_end):
        try:
            index_path = download_master_index(year, quarter, index_dir, request_delay=request_delay)
        except EdgarClientError:
            continue

        entries = parse_master_index(index_path)
        matching = _select_index_entries(entries, cik, valuation_date, lookback_months, lookahead_months)
        selected.extend(matching)

    for entry in selected:
        if allowed_extensions and not any(entry["filename"].lower().endswith(ext) for ext in allowed_extensions):
            continue
        downloaded = _download_index_file(entry, output_dir, request_delay=request_delay)
        if downloaded:
            downloaded_files.append(str(downloaded))

    return {
        "ticker": ticker.upper(),
        "cik": cik,
        "valuation_date": valuation_date.isoformat(),
        "selected_filings": selected,
        "downloaded_files": downloaded_files,
    }

class EdgarClient:
    def __init__(self, user_agent: Optional[str] = None, request_delay: float = DEFAULT_REQUEST_DELAY) -> None:
        if user_agent:
            global USER_AGENT
            USER_AGENT = user_agent
        self.request_delay = request_delay

    def download_for_ticker(
        self,
        ticker: str,
        valuation_date: date,
        output_root: Path,
        lookback_months: int = 18,
        lookahead_months: int = 6,
        allowed_extensions: Optional[List[str]] = None,
    ) -> Dict[str, Any]:
        return download_filings_for_ticker(
            ticker,
            valuation_date,
            output_root,
            lookback_months=lookback_months,
            lookahead_months=lookahead_months,
            allowed_extensions=allowed_extensions,
            request_delay=self.request_delay,
        )
