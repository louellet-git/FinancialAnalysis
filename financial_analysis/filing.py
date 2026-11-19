"""Filing-level XBRL: the statements of one 10-K as the filer structures them.

The SEC companyfacts source (the importer) gives values for standard tags only, and
does not say which line is a part of which line. Each filing has this information:

- ``MetaLinks.json``: the list of statements, each tag with its label, and the
  calculation tree (the parent of each line, with a weight of +1 or -1);
- the instance document (``*_htm.xml``): the value of each tag, company-specific tags
  included, for each context (period, and segment if the fact has one).

This module gets the two files for the latest 10-K of a ticker and builds the three
statements as trees. It is the first part of filing-level XBRL (PLANNING/tickets.md,
Ticket 7): it reads the consolidated values only. Facts with a segment are counted
but not used.
"""
from __future__ import annotations
import json
import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from financial_analysis.xbrl_tags import ALL_TAGS

ARCHIVE_URL = "https://www.sec.gov/Archives/edgar/data/{cik}/{accession}/{name}"
ANNUAL_FORMS = ("10-K", "10-K/A")
_ANNUAL_DAYS = (350, 380)
SUM_TOLERANCE = 0.005

_XBRLI = "http://www.xbrl.org/2003/instance"
_STANDARD_PREFIXES = ("us-gaap", "srt", "dei", "ecd", "cyd", "country", "currency", "exch", "naics", "sic", "stpr")

# The standard variable of each tag in the registry: "us-gaap_Assets" -> "total_assets".
VARIABLE_OF_TAG: Dict[str, str] = {
    concept.replace("/", "_", 1): name for name, spec in ALL_TAGS.items() for concept in spec.concepts
}

class FilingError(Exception):
    pass


@dataclass
class Line:
    tag: str                      # "us-gaap_Assets", or a company tag such as "vz_..."
    label: str
    weight: float                 # +1 or -1 toward the parent line
    value: Optional[float] = None
    children: List["Line"] = field(default_factory=list)

    @property
    def variable(self) -> Optional[str]:
        """The standard variable of the tag (None: the registry does not have the tag)."""
        return VARIABLE_OF_TAG.get(self.tag)

    @property
    def company_tag(self) -> bool:
        return self.tag.split("_", 1)[0] not in _STANDARD_PREFIXES

    def walk(self):
        yield self
        for child in self.children:
            yield from child.walk()

    def parts_sum(self) -> Optional[float]:
        """Sum of the parts with their weights (None if a part has no value)."""
        if not self.children or any(c.value is None for c in self.children):
            return None
        return sum(c.weight * c.value for c in self.children)

    def sum_agrees(self) -> Optional[bool]:
        total = self.parts_sum()
        if total is None or self.value is None:
            return None
        return abs(total - self.value) <= SUM_TOLERANCE * max(abs(self.value), 1.0)


@dataclass
class Statement:
    kind: str                     # "income", "balance" or "cash"
    name: str
    role: str
    roots: List[Line]

    def lines(self):
        for root in self.roots:
            yield from root.walk()

    def find(self, tag: str) -> Optional[Line]:
        return next((line for line in self.lines() if line.tag == tag), None)


@dataclass
class Filing:
    ticker: str
    accession: str
    fiscal_year_end: Optional[date]
    statements: List[Statement]
    facts_total: int = 0          # numeric facts in the instance document
    facts_with_segment: int = 0   # of these, facts with a segment (not used yet)

    def statement(self, kind: str) -> Optional[Statement]:
        return next((s for s in self.statements if s.kind == kind), None)


# --------------------------------------------------------------------------- fetch

def latest_annual_filing(facts: dict) -> Tuple[str, str]:
    """(cik, accession number) of the latest 10-K in the company facts."""
    best: Optional[dict] = None
    for namespace in facts.get("facts", {}).values():
        for concept in namespace.values():
            for entries in concept.get("units", {}).values():
                for e in entries:
                    if e.get("form") in ANNUAL_FORMS and e.get("accn"):
                        if best is None or e.get("filed", "") > best.get("filed", ""):
                            best = e
    if best is None:
        raise FilingError("the company facts have no 10-K")
    return str(facts.get("cik", "")), best["accn"]


def filing_folder(cache: Path, ticker: str, accession: str) -> Path:
    return cache / ticker.upper() / "filings" / accession


def fetch_filing(ticker: str, facts: dict, cache: Path = Path("data/raw"), refresh: bool = False) -> Path:
    """Get MetaLinks.json and the instance document of the latest 10-K into the cache.

    Returns the folder. Network: www.sec.gov (3 requests for a filing that is not in
    the cache).
    """
    from financial_analysis import edgar

    cik, accession = latest_annual_filing(facts)
    folder = filing_folder(cache, ticker, accession)
    meta_path = folder / "MetaLinks.json"
    if meta_path.exists() and any(folder.glob("*_htm.xml")) and not refresh:
        return folder
    folder.mkdir(parents=True, exist_ok=True)

    def get(name: str) -> bytes:
        url = ARCHIVE_URL.format(cik=cik, accession=accession.replace("-", ""), name=name)
        try:
            return edgar._http_get(url, request_delay=0.15)
        except edgar.EdgarClientError as exc:
            raise FilingError(f"{ticker} {accession}: {exc}") from exc

    index = json.loads(get("index.json"))
    names = [item["name"] for item in index.get("directory", {}).get("item", [])]
    instance = next((n for n in names if n.endswith("_htm.xml")), None)
    if "MetaLinks.json" not in names or instance is None:
        raise FilingError(f"{ticker} {accession}: the filing has no MetaLinks.json or no instance document")
    meta_path.write_bytes(get("MetaLinks.json"))
    (folder / instance).write_bytes(get(instance))
    return folder


# --------------------------------------------------------------------------- parse

def _statement_kind(long_name: str) -> Optional[str]:
    text = long_name.lower()
    if "parenthetical" in text or "comprehensive" in text:
        return None
    if "cash flow" in text:
        return "cash"
    if "balance sheet" in text or "financial position" in text or "financial condition" in text:
        return "balance"
    if "equity" in text or "capital" in text:
        return None
    if "income" in text or "operations" in text or "earnings" in text:
        return "income"
    return None


def parse_instance(xml_bytes: bytes, tag_of: Dict[Tuple[str, str], str]) -> Tuple[Dict[str, float], Optional[date], int, int]:
    """Consolidated values of the fiscal year of the filing.

    Returns ({tag: value}, fiscal year end, numeric facts, numeric facts with a
    segment). ``tag_of`` maps (namespace URI, local name) to the tag key of
    MetaLinks.json. For a flow, the value is the fact that covers the full fiscal
    year; for a balance, the fact on the fiscal year end.
    """
    root = ET.fromstring(xml_bytes)
    contexts: Dict[str, Tuple[Optional[str], str, bool]] = {}   # id -> (start, end, has segment)
    for ctx in root.iter(f"{{{_XBRLI}}}context"):
        period = ctx.find(f"{{{_XBRLI}}}period")
        if period is None:
            continue
        instant = period.findtext(f"{{{_XBRLI}}}instant")
        start = period.findtext(f"{{{_XBRLI}}}startDate")
        end = instant or period.findtext(f"{{{_XBRLI}}}endDate")
        if not end:
            continue
        has_segment = ctx.find(f"{{{_XBRLI}}}entity/{{{_XBRLI}}}segment") is not None
        contexts[ctx.get("id", "")] = (None if instant else start, end.strip(), has_segment)

    fiscal_year_end: Optional[date] = None
    raw: List[Tuple[str, str, float]] = []      # (tag, context id, value)
    total = with_segment = 0
    for element in root:
        ref = element.get("contextRef")
        if ref is None or ref not in contexts or not element.tag.startswith("{"):
            continue
        uri, local = element.tag[1:].split("}", 1)
        text = (element.text or "").strip()
        if local == "DocumentPeriodEndDate" and text:
            try:
                fiscal_year_end = date.fromisoformat(text)
            except ValueError:
                pass
            continue
        if element.get("unitRef") is None or not text:
            continue
        try:
            value = float(text)
        except ValueError:
            continue
        total += 1
        if contexts[ref][2]:
            with_segment += 1
            continue
        tag = tag_of.get((uri, local))
        if tag is not None:
            raw.append((tag, ref, value))

    if fiscal_year_end is None:
        ends = [date.fromisoformat(c[1]) for c in contexts.values() if not c[2]]
        fiscal_year_end = max(ends) if ends else None
    values: Dict[str, float] = {}
    if fiscal_year_end is not None:
        end_text = fiscal_year_end.isoformat()
        for tag, ref, value in raw:
            start, end, _ = contexts[ref]
            if end != end_text:
                continue
            if start is not None:
                days = (fiscal_year_end - date.fromisoformat(start.strip())).days
                if not _ANNUAL_DAYS[0] <= days <= _ANNUAL_DAYS[1]:
                    continue
            values.setdefault(tag, value)
    return values, fiscal_year_end, total, with_segment


def build_statements(meta: dict, values: Dict[str, float]) -> List[Statement]:
    """The income statement, the balance sheet and the cash-flow statement as trees."""
    instance = next(iter(meta["instance"].values()))
    tags: Dict[str, Any] = instance["tag"]

    def label(tag: str) -> str:
        roles = tags.get(tag, {}).get("lang", {}).get("en-us", {}).get("role", {})
        return roles.get("terseLabel") or roles.get("label") or tag

    statements: List[Statement] = []
    seen_kinds: set = set()
    for report in instance["report"].values():
        if report.get("groupType") != "statement" or report.get("subGroupType"):
            continue
        kind = _statement_kind(report.get("longName", ""))
        if kind is None or kind in seen_kinds:
            continue
        role = report["role"]
        children: Dict[Optional[str], List[Tuple[float, str, float]]] = {}
        for tag, info in tags.items():
            arc = (info.get("calculation") or {}).get(role)
            if arc is None:
                continue
            parent = arc.get("parentTag")
            children.setdefault(parent, []).append((arc.get("order") or 0.0, tag, arc.get("weight") or 1.0))

        def node(tag: str, weight: float) -> Line:
            line = Line(tag=tag, label=label(tag), weight=weight, value=values.get(tag))
            for _, child, child_weight in sorted(children.get(tag, [])):
                line.children.append(node(child, child_weight))
            return line

        roots = [node(tag, 1.0) for _, tag, _ in sorted(children.get(None, []))]
        if not roots:
            continue
        name = report.get("longName", "").split(" - ", 2)[-1]
        statements.append(Statement(kind=kind, name=name, role=role, roots=roots))
        seen_kinds.add(kind)
    return statements


def load_filing(ticker: str, folder: Path) -> Filing:
    meta_path = folder / "MetaLinks.json"
    instance_path = next(iter(sorted(folder.glob("*_htm.xml"))), None)
    if not meta_path.exists() or instance_path is None:
        raise FilingError(f"{folder}: no MetaLinks.json or no instance document")
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    instance = next(iter(meta["instance"].values()))
    tag_of = {(info["nsuri"], info["localname"]): tag for tag, info in instance["tag"].items()
              if "nsuri" in info and "localname" in info}
    values, fiscal_year_end, total, with_segment = parse_instance(instance_path.read_bytes(), tag_of)
    return Filing(
        ticker=ticker.upper(),
        accession=folder.name,
        fiscal_year_end=fiscal_year_end,
        statements=build_statements(meta, values),
        facts_total=total,
        facts_with_segment=with_segment,
    )


# --------------------------------------------------------------------------- questions

INVESTING_TOTALS = (
    "us-gaap_NetCashProvidedByUsedInInvestingActivities",
    "us-gaap_NetCashProvidedByUsedInInvestingActivitiesContinuingOperations",
)
OPERATING_TOTALS = (
    "us-gaap_NetCashProvidedByUsedInOperatingActivities",
    "us-gaap_NetCashProvidedByUsedInOperatingActivitiesContinuingOperations",
)
LIABILITY_TOTALS = ("us-gaap_Liabilities", "us-gaap_LiabilitiesAndStockholdersEquity")

# Payments for long-life assets of the business. Purchased intangibles are a separate
# pattern: they are not always a regular cost (see README.md, "Standard input variables").
CAPEX_PATTERN = re.compile(
    r"PropertyPlantAndEquipment|ProductiveAssets|CapitalExpenditure|CapitalImprovements|"
    r"OilAndGasProperty|Software|Construction|PropertyAndEquipment", re.I)
INTANGIBLES_PATTERN = re.compile(r"IntangibleAssets|Spectrum|Licenses", re.I)
_NOT_A_PAYMENT = re.compile(r"^Proceeds|Businesses|Acquisitions?Net", re.I)
DA_PATTERN = re.compile(r"Depreciation|Amortization|Depletion", re.I)
_NOT_DA = re.compile(r"DebtDiscount|DebtIssuance|FinancingCosts|DeferredCharges|Premium|ShareBased|Contract|Content", re.I)
DEBT_PATTERN = re.compile(r"Debt|Borrowings|NotesPayable|CommercialPaper|LineOfCredit|LongTermNotes|SeniorNotes", re.I)
_NOT_DEBT = re.compile(r"DebtSecurities|RightOfUse|PropertyPlant|Receivable|Investments?$", re.I)


def _local(tag: str) -> str:
    return tag.split("_", 1)[-1]


def lines_matching(
    statement: Optional[Statement],
    pattern: re.Pattern,
    under: Tuple[str, ...] = (),
    exclude: Optional[re.Pattern] = None,
) -> List[Line]:
    """The top lines with a value whose tag matches the pattern.

    ``under``: only lines below one of these tags (for example the investing total).
    A line below a line that matches is not in the result: the parent already contains
    it, and a sum of the two counts the amount two times.
    """
    if statement is None:
        return []
    out: List[Line] = []
    seen: set = set()

    def visit(line: Line) -> None:
        local = _local(line.tag)
        if line.value is not None and pattern.search(local) and not (exclude and exclude.search(local)):
            if line.tag not in seen:
                seen.add(line.tag)
                out.append(line)
            return
        for child in line.children:
            visit(child)

    if under:
        for top in (line for line in statement.lines() if line.tag in under):
            for child in top.children:
                visit(child)
    else:
        for root in statement.roots:
            visit(root)
    return out


def capex_lines(filing: Filing) -> List[Line]:
    return lines_matching(filing.statement("cash"), CAPEX_PATTERN, INVESTING_TOTALS, _NOT_A_PAYMENT)


def intangible_purchase_lines(filing: Filing) -> List[Line]:
    lines = lines_matching(filing.statement("cash"), INTANGIBLES_PATTERN, INVESTING_TOTALS, _NOT_A_PAYMENT)
    capex = {line.tag for line in capex_lines(filing)}
    return [line for line in lines if line.tag not in capex]


def depreciation_lines(filing: Filing) -> List[Line]:
    return lines_matching(filing.statement("cash"), DA_PATTERN, OPERATING_TOTALS, _NOT_DA)


def debt_lines(filing: Filing) -> List[Line]:
    return lines_matching(filing.statement("balance"), DEBT_PATTERN, LIABILITY_TOTALS, _NOT_DEBT)


@dataclass
class FilingEstimates:
    """Three inputs from the statements of the filing (None: no line found).

    The values use the sign of the standard variables: capex is negative (cash out).
    """
    capex: Optional[float]
    capex_with_intangibles: Optional[float]
    da_total: Optional[float]
    total_debt: Optional[float]


def _cash_effect(lines: List[Line]) -> Optional[float]:
    # weight * value is the effect of the line on its section total (negative = cash out)
    return sum(line.weight * line.value for line in lines) if lines else None


def estimates(filing: Filing) -> FilingEstimates:
    """Capex, depreciation and amortization, and total debt from the filing lines."""
    capex = _cash_effect(capex_lines(filing))
    intangibles = _cash_effect(intangible_purchase_lines(filing))
    with_intangibles = None if capex is None else capex + (intangibles or 0.0)
    da = depreciation_lines(filing)
    debt = debt_lines(filing)
    return FilingEstimates(
        capex=capex,
        capex_with_intangibles=with_intangibles,
        da_total=sum(line.value for line in da) if da else None,
        total_debt=sum(line.value for line in debt) if debt else None,
    )
