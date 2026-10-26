"""In-repo scrum board: parse, check and create PLANNING/board/<id>.md tickets.

    python scripts/board.py check | branch <name> | status | new --title ... (format: PLANNING/board/README.md)
"""
import argparse
import re
import sys
from datetime import date, datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KEYS = ["id", "title", "area", "status", "owner", "branch", "created", "acceptance"]
STATUSES = ["backlog", "in-progress", "review", "done"]
ACTIVE = ("in-progress", "review")
AREA = re.compile(r"(engine|importer|docs|planning|ticker:[A-Z]{1,5}(\.[A-Z])?)")
ID = re.compile(r"T-\d{8}-[a-z0-9-]+")
LANE = re.compile(r"PLANNING/board/(T-[^/]*\.md|README\.md)")  # the only files a board-only change may touch


def load(root):
    """Parse every ticket under root/PLANNING/board into a dict (plus '_file')."""
    tickets = []
    for f in sorted((Path(root) / "PLANNING" / "board").glob("*.md")):
        if f.name == "README.md":
            continue
        head = f.read_text().split("---\n")[1:2]
        pairs = (ln.partition(":") for ln in (head[0].splitlines() if head else []))
        tickets.append({"_file": f.name, **{k.strip(): v.strip() for k, _, v in pairs}})
    return tickets


def _is_date(s):
    try:
        return bool(re.fullmatch(r"\d{4}-\d\d-\d\d", s)) and bool(datetime.strptime(s, "%Y-%m-%d"))
    except ValueError:
        return False


def _check(tickets):
    out, seen = [], {}
    for t in tickets:
        f = t["_file"]
        out += [f"{f}: missing key {k}" for k in KEYS if k not in t]
        out += [f"{f}: empty {k}" for k in KEYS if t.get(k) == "" and k != "branch"]
        if t.get("id") and not ID.fullmatch(t["id"]):
            out.append(f"{f}: bad id {t['id']}")
        if t.get("id") and f != t["id"] + ".md":
            out.append(f"{f}: id {t['id']} does not match filename")
        if t.get("area") and not AREA.fullmatch(t["area"]):
            out.append(f"{f}: bad area {t['area']}")
        if t.get("status") and t["status"] not in STATUSES:
            out.append(f"{f}: bad status {t['status']}")
        if t.get("created") and not _is_date(t["created"]):
            out.append(f"{f}: bad created {t['created']}")
        if t.get("status") in ACTIVE and not t.get("branch"):
            out.append(f"{f}: {t['status']} ticket needs a branch")
        seen.setdefault(t.get("id"), []).append(f)
    out += [f"duplicate id {i}: {', '.join(fs)}" for i, fs in seen.items() if i and len(fs) > 1]
    active = [t for t in tickets if t.get("status") in ACTIVE]
    for key, label in (("area", "area"), ("branch", "branch")):
        groups = {}
        for t in active:
            groups.setdefault(t.get(key), []).append(t["_file"])
        out += [f"{len(fs)} active tickets on {label} {v}: {', '.join(fs)}"
                for v, fs in groups.items() if v and len(fs) > 1]
    return out


def problems(root):
    """Every rule violation on the board, one string each (any file but a ticket or README is one)."""
    files = (f.relative_to(root).as_posix() for f in (Path(root) / "PLANNING" / "board").rglob("*") if f.is_file())
    return _check(load(root)) + [f"{f}: unexpected file (the board holds only T-*.md tickets and README.md)"
                                 for f in sorted(files) if not LANE.fullmatch(f)]


def ticket_for_branch(root, branch):
    """The single in-progress/review ticket on `branch`, else None."""
    hits = [t for t in load(root) if t.get("status") in ACTIVE and branch and t.get("branch") == branch]
    return hits[0] if len(hits) == 1 else None


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--root", default=ROOT)
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    sub.add_parser("status")
    sub.add_parser("branch").add_argument("name")
    n = sub.add_parser("new")
    for k in ("title", "area", "owner", "branch", "acceptance"):
        n.add_argument(f"--{k}", required=True)
    n.add_argument("--status", default="backlog", choices=STATUSES)
    a = p.parse_args(argv)
    if a.cmd == "check":
        errs = problems(a.root)
        print("\n".join(errs) or "board ok")
        return 1 if errs else 0
    if a.cmd == "status":
        rows = [" ".join(t.get(k, "") for k in ("id", "area", "owner", "branch"))
                for t in load(a.root) if t.get("status") in ACTIVE]
        print("\n".join(rows) or "no active tickets")
        return 0
    if a.cmd == "branch":
        t = ticket_for_branch(a.root, a.name)
        print(t["id"] if t else f"no active ticket on branch {a.name}")
        return 0 if t else 1
    today = date.today()
    slug = re.sub(r"[^a-z0-9]+", "-", a.title.lower()).strip("-")
    t = {"id": f"T-{today:%Y%m%d}-{slug}", "title": a.title, "area": a.area, "status": a.status,
         "owner": a.owner, "branch": a.branch, "created": today.isoformat(), "acceptance": a.acceptance}
    t["_file"] = t["id"] + ".md"
    errs = _check(load(a.root) + [t])
    if errs:
        print("\n".join(errs))
        return 1
    path = Path(a.root) / "PLANNING" / "board" / t["_file"]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("---\n" + "".join(f"{k}: {t[k]}\n" for k in KEYS) + "---\n")
    print(path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
