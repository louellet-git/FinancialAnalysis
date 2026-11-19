"""Registry of standard variables: the one mapping from XBRL tags to model inputs.

The registry data is in ``financial_analysis/standard_variables.toml``. Read the
header of that file for the rules. This module loads the file and gives the Python
names that the importer, the audit and the xlsx writer use. The Rust test framework
(``rust/mapcheck``) reads the same file, thus Python and Rust use one definition.

The importer records where each value came from in ``PeriodFinancials.sources``
(a tag name, or the formula). Tags that are not in the registry stay available in
``PeriodFinancials.raw_extras``.

Limits: the SEC companyfacts API has no dimensional facts (segments, products,
regions) and no company-specific tags. Those need filing-level XBRL
(PLANNING/tickets.md, Ticket 7).
"""
from __future__ import annotations
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

REGISTRY_PATH = Path(__file__).with_name("standard_variables.toml")

_PERIOD_FOR_STATEMENT = {
    "income_statement": "duration",
    "balance_sheet": "instant",
    "cash_flow": "duration",
    "disclosure": "instant",
}


@dataclass
class TagSpec:
    concepts: list[str]
    unit: str = "USD"
    sign_flip: bool = False   # store as a negative number (a cash outflow)
    negate: bool = False      # change the sign of the tag value
    formulas: list[str] = field(default_factory=list)
    period: Optional[str] = None  # "duration" or "instant"; set from the statement
    core: bool = False
    memo: bool = False
    note: str = ""


@dataclass(frozen=True)
class Rollup:
    name: str
    total: str
    parts: tuple[str, ...]      # lines that are usually separate lines on the statement
    details: tuple[str, ...]    # lines that are frequently a detail inside one of the parts
    combined: dict  # {one-line variable: (the parts it replaces, ...)}
    signed: bool = False   # parts have signs (cash flows): the sum can be negative
    exact: bool = False    # the parts are all the lines: the sum must equal the total


def _load() -> dict:
    with REGISTRY_PATH.open("rb") as handle:
        return tomllib.load(handle)


_RAW = _load()

INCOME_STATEMENT_TAGS: dict[str, TagSpec] = {}
BALANCE_SHEET_TAGS: dict[str, TagSpec] = {}
CASH_FLOW_TAGS: dict[str, TagSpec] = {}
DISCLOSURE_TAGS: dict[str, TagSpec] = {}
_BY_STATEMENT = {
    "income_statement": INCOME_STATEMENT_TAGS,
    "balance_sheet": BALANCE_SHEET_TAGS,
    "cash_flow": CASH_FLOW_TAGS,
    "disclosure": DISCLOSURE_TAGS,
}

# All variables, in the sequence of the registry file.
ALL_TAGS: dict[str, TagSpec] = {}
STATEMENT_MAP: dict[str, str] = {}

for _entry in _RAW["variable"]:
    _spec = TagSpec(
        concepts=list(_entry["tags"]),
        unit=_entry.get("unit", "USD"),
        sign_flip=_entry.get("sign_flip", False),
        negate=_entry.get("negate", False),
        formulas=list(_entry.get("formulas", [])),
        period=_PERIOD_FOR_STATEMENT[_entry["statement"]],
        core=_entry.get("core", False),
        memo=_entry.get("memo", False),
        note=_entry.get("note", ""),
    )
    if _entry["name"] in ALL_TAGS:
        raise ValueError(f"standard_variables.toml: variable '{_entry['name']}' is there two times")
    ALL_TAGS[_entry["name"]] = _spec
    _BY_STATEMENT[_entry["statement"]][_entry["name"]] = _spec
    STATEMENT_MAP[_entry["name"]] = _entry["statement"]

STATEMENT_MAP["free_cash_flow"] = "cash_flow"
STATEMENT_MAP["fcf_margin"] = "cash_flow"

CORE_FIELDS: tuple[str, ...] = tuple(k for k, s in ALL_TAGS.items() if s.core)

# Tags that show that a period exists. A fiscal year (or a quarter) is in the
# output only if one of these tags has a fact of the correct length for it.
ANCHOR_CONCEPTS: tuple[str, ...] = tuple(_RAW["anchors"])

# Material tags that have no standard variable on purpose: (regex, reason).
EXCLUDED_CONCEPTS: tuple[tuple[str, str], ...] = tuple(
    (e["pattern"], e["reason"]) for e in _RAW["excluded"]
)

ROLLUPS: tuple[Rollup, ...] = tuple(
    Rollup(
        name=r["name"],
        total=r["total"],
        parts=tuple(r["parts"]),
        details=tuple(r.get("details", [])),
        combined={k: tuple(v) for k, v in r.get("combined", {}).items()},
        signed=r.get("signed", False),
        exact=r.get("exact", False),
    )
    for r in _RAW.get("rollup", [])
)

# (part, total): a variable that must not be larger than a total.
WITHIN: tuple[tuple[str, str], ...] = tuple((w["part"], w["total"]) for w in _RAW.get("within", []))

UNIT_MAP: dict[str, str] = {
    "eps_basic": "USD/share",
    "eps_diluted": "USD/share",
    "shares_basic": "shares",
    "shares_diluted": "shares",
    "shares_outstanding": "shares",
    "fcf_margin": "ratio",
}
