"""Runs the Rust program ``rust/mapcheck`` with the SEC contact.

Rust is the only implementation of the import (``fa import``, ``fa analyze``, ``mapcheck import``).
This module has only ``_run``: it starts the Rust program and gives it the environment variable
``SEC_USER_AGENT`` (the SEC contact) when the variable is not set. It stays until the legacy
SEC client is deleted; after that the contact comes from ``SEC_USER_AGENT`` only (board ticket
``report-rust``, part 4).

Build the program one time (and after each change of the registry file)::

    cargo build --release --manifest-path rust/mapcheck/Cargo.toml
"""
from __future__ import annotations
import os
import subprocess
import sys
from pathlib import Path

SEC_USER_AGENT_ENV = "SEC_USER_AGENT"
BINARY_ENV = "FA_MAPCHECK"

_ROOT = Path(__file__).resolve().parents[1]


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
