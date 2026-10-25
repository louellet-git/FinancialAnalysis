"""Documentation-standard checker (rules: docs/DOC-STANDARD.md).

    python scripts/doc_check.py check | list | record <path> <PASS|FAIL> | --hook
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from datetime import date, datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KEYS = ["id", "type", "item", "status", "date", "scope"]
TYPES = ["description", "plan", "policy", "procedure", "report", "request", "specification", "record"]
STATUSES = ["draft", "current", "frozen", "superseded"]
EXEMPT = re.compile(r"(\.claude/(agents|commands|skills)|\.github)/.*|PLANNING/board/T-[^/]*\.md")
SEPARATOR = re.compile(r"[|:\s-]+")
GIT = re.compile(r"\bgit\b")
VERB = re.compile(r"(?<![\w-])(commit|merge|cherry-pick|revert|pull|am|rebase|push)(?![\w-])")
TARGET = re.compile(r"""(?:\b(?:cd|pushd|Set-Location)\s+(?:-\w+\s+)?|\bgit\s+-C\s+|--(?:git-dir|work-tree)[=\s]+"""
                    r"""|\bGIT_(?:DIR|WORK_TREE)=)("[^"]*"|'[^']*'|[^\s;&|]+)""")
REVIEWS = "docs/doc-reviews.tsv"
FIX = "Fix: run the doc-critic agent on each doc, then python scripts/doc_check.py record <path> PASS and git add " + REVIEWS


def _git(root, *args, check=True):
    return subprocess.run(["git", "-C", str(root), *args], capture_output=True, check=check).stdout


def _sha(raw):
    """sha256 hex of the content with CRLF read as LF."""
    return hashlib.sha256(raw.replace(b"\r\n", b"\n")).hexdigest()


def _is_date(s):
    try:
        return len(s) == 10 and bool(datetime.strptime(s, "%Y-%m-%d"))
    except ValueError:
        return False


RULES = {"id": lambda s: re.fullmatch(r"DOC-[A-Z0-9-]+", s), "type": TYPES.__contains__, "item": lambda s: s == "9 record" or re.fullmatch(r"10\.([2-9]|[1-6]\d|7[0-4]) \S.*", s),
         "status": STATUSES.__contains__, "date": _is_date}


def governed(root):
    """Tracked or untracked-not-ignored *.md files (relative to root) minus the exempt ones."""
    names = _git(root, "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "*.md")
    return [Path(n) for n in sorted(set(names.decode().split("\0"))) if n and not EXEMPT.fullmatch(n)
            and Path(root, n).is_file()]


def header(text):
    """Front-matter key/value dict, or None when there is no header."""
    m = re.match(r"---\r?\n(.*?\n)---\r?\n", text, re.S)
    return None if not m else {k.strip(): v.strip() for k, _, v in (ln.partition(":") for ln in m[1].splitlines())}


def _reviews(root):
    """{(path, sha256): last verdict} from the review ledger."""
    f = Path(root, REVIEWS)
    rows = [ln.split("\t") for ln in (f.read_text().splitlines()[1:] if f.exists() else [])]
    return {(r[1], r[2]): r[3] for r in rows if len(r) >= 4}


def changed(root):
    """*.md files this branch carries: untracked, or differing (worktree) from the merge-base with origin/main
    (else the root commit). None when there is no HEAD."""
    if not _git(root, "rev-parse", "--verify", "-q", "HEAD", check=False):
        return None
    base = (_git(root, "merge-base", "HEAD", "origin/main", check=False)
            or _git(root, "rev-list", "--max-parents=0", "HEAD")).decode().split()[0]
    cmds = (("diff", "--name-only", "-z", base), ("ls-files", "-z", "--others", "--exclude-standard"))
    return {n for c in cmds for n in _git(root, *c, "--", "*.md").decode().split("\0") if n}


def problems(root, scope=None):
    """Rule violations across the governed docs, one string each; only those touching `scope` (a set of
    names) when given. Duplicates are found over all docs and reported when any doc involved is in scope."""
    out, ids, lines, reviews = [], {}, {}, _reviews(root)
    hit = lambda names: scope is None or any(n in scope for n in names)
    for p in governed(root):
        name, mine = p.as_posix(), []
        raw = Path(root, p).read_bytes()
        h = header(raw.decode())
        if h is None:
            mine.append(f"{name}: no header")
        else:
            mine += [f"{name}: missing key {k}" for k in KEYS if not h.get(k)]
            mine += [f"{name}: unknown key {k}" for k in h if k not in KEYS]
            mine += [f"{name}: bad {k} {h[k]}" for k, ok in RULES.items() if h.get(k) and not ok(h[k])]
            ids.setdefault(h.get("id"), []).append(name)
            if reviews.get((name, _sha(raw))) != "PASS":
                mine.append(f"{name}: no PASS review row for current content")
        out += mine if hit([name]) else []
        frozen = h is not None and h.get("status") in ("frozen", "superseded")
        fenced = False
        for ln in ([] if frozen else raw.decode().splitlines()):
            s = ln.strip()
            fenced ^= s.startswith("```")
            if not fenced and not s.startswith("```") and len(s) >= 60 and not SEPARATOR.fullmatch(s):
                lines.setdefault(s, set()).add(name)
    out += [f"duplicate id {i}: {', '.join(ns)}" for i, ns in ids.items() if i and len(ns) > 1 and hit(ns)]
    out += [f"duplicate line in {', '.join(sorted(ns))}: {s[:50]}" for s, ns in lines.items()
            if len(ns) > 1 and hit(ns)]
    return out


def _roots(command, cwd):
    """(git toplevels, unresolved targets) for the base dir (payload cwd, else CLAUDE_PROJECT_DIR) and every
    directory the command names."""
    base = cwd or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd()
    roots, bad = set(), []
    for t in [""] + [t.strip("\"'") for t in TARGET.findall(command)]:
        d = Path(base, os.path.expandvars(os.path.expanduser(t)))
        try:
            roots.add(_git(d, "rev-parse", "--show-toplevel").decode().strip())
        except (subprocess.CalledProcessError, OSError):
            bad.append(t or str(base))
    return roots, bad


def hook():
    """PreToolUse: deny a history-changing git command while a doc this branch carries has problems."""
    try:
        payload = json.load(sys.stdin)
        command = payload.get("tool_input", {}).get("command") or ""
        if not (GIT.search(command) and VERB.search(command)):
            return 0
        roots, bad = _roots(command, payload.get("cwd"))
        found = [m for r in sorted(roots) if Path(r, "docs/DOC-STANDARD.md").exists()
                 for m in problems(r, scope=changed(r))]
        if not (found or bad):
            return 0
        reason = "\n".join([f"cannot resolve a git repo for {t}" for t in bad]
                          + (["A governed doc changed on this branch fails:"] + found if found else [])
                          + [FIX])
    except Exception as e:
        reason = f"doc_check internal error: {e}"
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                                             "permissionDecisionReason": reason}}))
    return 0


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--root", default=ROOT)
    p.add_argument("--hook", action="store_true")
    sub = p.add_subparsers(dest="cmd")
    sub.add_parser("check")
    sub.add_parser("list")
    r = sub.add_parser("record")
    r.add_argument("path")
    r.add_argument("verdict", choices=["PASS", "FAIL"])
    r.add_argument("--reviewer", default="doc-critic/haiku")
    a = p.parse_args(argv)
    if a.hook:
        return hook()
    if a.cmd == "check":
        found = problems(a.root)
        print("\n".join(found) or "docs ok")
        return 1 if found else 0
    if a.cmd == "list":
        hs = [(header(Path(a.root, f).read_text()) or {}, f.as_posix()) for f in governed(a.root)]
        print("\n".join(" ".join([h.get(k, "-") for k in ("id", "type", "status")] + [f]) for h, f in hs))
        return 0
    if a.cmd == "record":
        f = Path(a.root, a.path)
        if not f.is_file():
            return print(f"no such file: {a.path}", file=sys.stderr) or 1
        row = [date.today().isoformat(), f.resolve().relative_to(Path(a.root).resolve()).as_posix(),
               _sha(f.read_bytes()), a.verdict, a.reviewer]
        with Path(a.root, REVIEWS).open("a") as fh:
            fh.write(("" if fh.tell() else "date\tpath\tsha256\tverdict\treviewer\n") + "\t".join(row) + "\n")
        return 0
    p.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
