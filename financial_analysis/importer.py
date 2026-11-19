"""Standard variables for a ticker, from the Rust program ``rust/mapcheck``.

Rust is the only implementation of the mapping from SEC data to the standard
variables (fetch, period selection, tags, formulas, sources, checks). This module has
no mapping logic: it runs ``mapcheck import`` and reads the store file that the Rust
program writes. The Python valuation code is legacy code that will move to Rust; it
gets its data through this module until then.

Build the program one time (and after each change of the registry file)::

    cargo build --release --manifest-path rust/mapcheck/Cargo.toml

Segment revenue is not available from the SEC companyfacts source
(``PeriodFinancials.segment_revenue`` is None); see PLANNING/tickets.md, Ticket 7.
"""
from __future__ import annotations
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from financial_analysis import store
from financial_analysis.schema import CompanyFinancials

DEFAULT_N_PERIODS = 12
SEC_USER_AGENT_ENV = "SEC_USER_AGENT"
BINARY_ENV = "FA_MAPCHECK"

_ROOT = Path(__file__).resolve().parents[1]


class UnsupportedFilingError(Exception):
    pass


class MapcheckError(RuntimeError):
    """The Rust program is not built, or it gave an error."""


def mapcheck_binary() -> Path:
    """The Rust program. The environment variable FA_MAPCHECK selects a different file."""
    override = os.environ.get(BINARY_ENV)
    if override:
        return Path(override)
    name = "mapcheck.exe" if sys.platform.startswith("win") else "mapcheck"
    return _ROOT / "rust" / "mapcheck" / "target" / "release" / name


def _run(arguments: list[str]) -> subprocess.CompletedProcess:
    binary = mapcheck_binary()
    if not binary.exists():
        raise MapcheckError(
            f"The Rust program is not built ({binary}). Build it: "
            "cargo build --release --manifest-path rust/mapcheck/Cargo.toml"
        )
    env = dict(os.environ)
    if SEC_USER_AGENT_ENV not in env:
        # The legacy SEC client has the contact text; give it to the Rust program.
        from financial_analysis import edgar
        env[SEC_USER_AGENT_ENV] = edgar.USER_AGENT
    return subprocess.run([str(binary), *arguments], capture_output=True, text=True, env=env)


def import_company(
    ticker: str,
    n_periods: int = DEFAULT_N_PERIODS,
    save: bool = True,
    refresh: bool = False,
    cache_dir: Path = Path("data/raw"),
) -> CompanyFinancials:
    """The latest ``n_periods`` fiscal years and ``n_periods`` quarters of a ticker.

    ``save``: the store file stays in ``data/processed``. ``refresh``: get the company
    facts from the SEC again. The values, their sources and the period rules are those
    of the Rust program (rust/mapcheck/README.md).
    """
    ticker = ticker.upper()
    if refresh:
        fetched = _run(["fetch", "--refresh", "--cache", str(cache_dir), ticker])
        if fetched.returncode != 0:
            raise MapcheckError(f"{ticker}: {fetched.stderr.strip() or fetched.stdout.strip()}")

    def run_import(directory: Path) -> CompanyFinancials:
        result = _run(["import", ticker, "--cache", str(cache_dir), "--store", str(directory),
                       "--periods", str(n_periods)])
        if result.returncode != 0:
            message = result.stderr.strip() or result.stdout.strip()
            if "does not report under us-gaap" in message:
                raise UnsupportedFilingError(message)
            raise MapcheckError(message)
        store._CACHE.pop(ticker, None)
        return store.load(ticker, data_dir=directory)

    if save:
        return run_import(store._DEFAULT_DATA_DIR)
    with tempfile.TemporaryDirectory() as tmp:
        company = run_import(Path(tmp))
    store._CACHE.pop(ticker, None)
    return company
