"""Documentation-standard checker: header rules, review rows, duplicates, hook on tmp git repos."""
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

from scripts import doc_check

SCRIPT = Path(doc_check.__file__)
HEAD = dict(id="DOC-A", type="plan", item="10.22 Information management plan",
            status="frozen", date="2026-10-25", scope="Test scope.")
COMMIT = {"tool_input": {"command": "git commit -m x"}}
LONG = "A rule that is long enough to count as a stated rule, stated here once only."


def git(root, *args):
    subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True)


def put(root, rel, raw=None, body="Body.\n", **over):
    """Write and stage a doc; a None override drops that key; raw replaces the whole text."""
    f = {k: v for k, v in {**HEAD, **over}.items() if v is not None}
    p = Path(root, rel)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(raw if raw is not None else "---\n" + "".join(f"{k}: {v}\n" for k, v in f.items()) + "---\n" + body)
    git(root, "add", "-A")


def repo(tmp_path, standard=True):
    git(tmp_path, "init", "-q")
    if standard:
        put(tmp_path, "docs/DOC-STANDARD.md", id="DOC-STD")
    return tmp_path


def cli(root, *args):
    return subprocess.run([sys.executable, str(SCRIPT), "--root", str(root), *args], capture_output=True, text=True)


def record(root, rel, verdict="PASS"):
    assert cli(root, "record", rel, verdict).returncode == 0
    git(root, "add", "-A")


def hook(root, payload=COMMIT):
    return subprocess.run([sys.executable, str(SCRIPT), "--hook"], input=json.dumps(payload), capture_output=True,
                          text=True, env={**os.environ, "CLAUDE_PROJECT_DIR": str(root)})


def test_valid_doc_passes(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="current")
    record(tmp_path, "a.md")
    assert doc_check.problems(tmp_path) == []
    assert cli(tmp_path, "check").stdout.strip() == "docs ok"
    assert "DOC-A plan current a.md" in cli(tmp_path, "list").stdout


@pytest.mark.parametrize("over,expect", [
    (dict(raw="No header.\n"), "no header"),
    (dict(scope=None), "missing key scope"),
    (dict(scope=""), "missing key scope"),
    (dict(extra="x"), "unknown key extra"),
    (dict(id="doc-a"), "bad id"),
    (dict(type="memo"), "bad type"),
    (dict(item="10.1 General"), "bad item"),
    (dict(item="10.75 Thing"), "bad item"),
    (dict(item="10.22"), "bad item"),
    (dict(item="9 records"), "bad item"),
    (dict(status="done"), "bad status"),
    (dict(date="2026-13-01"), "bad date"),
    (dict(date="25-10-2026"), "bad date"),
])
def test_header_problem(tmp_path, over, expect):
    repo(tmp_path)
    put(tmp_path, "a.md", **over)
    assert any("a.md" in m and expect in m for m in doc_check.problems(tmp_path))


def test_record_item_accepted(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", type="record", item="9 record")
    assert doc_check.problems(tmp_path) == []


def test_duplicate_id_fails(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md")
    put(tmp_path, "b.md")
    assert any("duplicate id DOC-A" in m for m in doc_check.problems(tmp_path))


def test_review_rows(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="draft")
    assert any("no PASS" in m for m in doc_check.problems(tmp_path))
    record(tmp_path, "a.md", "FAIL")
    assert any("no PASS" in m for m in doc_check.problems(tmp_path))
    record(tmp_path, "a.md")
    assert doc_check.problems(tmp_path) == []
    rows = Path(tmp_path, "docs/doc-reviews.tsv").read_text().splitlines()
    assert rows[0] == "date\tpath\tsha256\tverdict\treviewer"
    assert [r.split("\t")[1:4:2] for r in rows[1:]] == [["a.md", "FAIL"], ["a.md", "PASS"]]
    assert len(rows[2].split("\t")[2]) == 64 and rows[2].endswith("doc-critic/haiku")


def test_content_change_after_record_fails(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="current")
    record(tmp_path, "a.md")
    Path(tmp_path, "a.md").write_text(Path(tmp_path, "a.md").read_text() + "More.\n")
    assert any("no PASS" in m for m in doc_check.problems(tmp_path))


def test_frozen_and_superseded_need_no_row(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="frozen")
    put(tmp_path, "b.md", id="DOC-B", status="superseded")
    assert doc_check.problems(tmp_path) == []


def test_exempt_paths_ignored(tmp_path):
    repo(tmp_path)
    for rel in (".claude/agents/x.md", ".github/y.md", "PLANNING/board/T-20261025-a.md"):
        put(tmp_path, rel, raw="No header.\n")
    put(tmp_path, "PLANNING/board/README.md", raw="No header.\n")
    put(tmp_path, "notes.txt", raw="not markdown\n")
    assert [p.as_posix() for p in doc_check.governed(tmp_path)] == ["PLANNING/board/README.md", "docs/DOC-STANDARD.md"]
    assert doc_check.problems(tmp_path) == ["PLANNING/board/README.md: no header"]


def test_duplicate_line(tmp_path):
    repo(tmp_path)
    long = LONG
    put(tmp_path, "a.md", raw=f"---\nid: DOC-A\n---\n{long}\n")
    put(tmp_path, "b.md", raw=f"{long}\n")
    assert any("duplicate line" in m and "a.md" in m and "b.md" in m for m in doc_check.problems(tmp_path))


def test_duplicate_line_exceptions(tmp_path):
    repo(tmp_path)
    long = LONG
    sep = "|" + "-" * 70 + "|"
    put(tmp_path, "a.md", raw=f"{long}\n{sep}\n```\n{long}\n```\n{long[:50]}\n")
    put(tmp_path, "b.md", raw=f"```\n{long}\n```\n{sep}\n{long[:50]}\n")
    assert not any("duplicate line" in m for m in doc_check.problems(tmp_path))


def test_duplicate_line_skips_frozen_and_superseded(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="frozen", body=LONG + "\n")
    put(tmp_path, "b.md", id="DOC-B", status="superseded", body=LONG + "\n")
    put(tmp_path, "c.md", id="DOC-C", status="current", body=LONG + "\n")
    record(tmp_path, "c.md")
    assert doc_check.problems(tmp_path) == []
    put(tmp_path, "d.md", id="DOC-D", status="current", body=LONG + "\n")
    record(tmp_path, "d.md")
    assert doc_check.problems(tmp_path) == [f"duplicate line in c.md, d.md: {LONG[:50]}"]


def test_crlf_hashes_as_lf(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="current")
    text = Path(tmp_path, "a.md").read_text()
    Path(tmp_path, "a.md").write_bytes(text.replace("\n", "\r\n").encode())
    record(tmp_path, "a.md")
    assert doc_check.problems(tmp_path) == []
    Path(tmp_path, "a.md").write_bytes(text.encode())
    assert doc_check.problems(tmp_path) == []
    assert doc_check.problems(tmp_path, staged=True) == []


def test_staged_reads_index(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md")
    Path(tmp_path, "a.md").write_text("No header.\n")
    assert doc_check.problems(tmp_path) == ["a.md: no header"]
    assert doc_check.problems(tmp_path, staged=True) == []


def test_hook_denies_bad_staged_doc(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", raw="No header.\n")
    out = json.loads(hook(tmp_path).stdout)["hookSpecificOutput"]
    assert out["hookEventName"] == "PreToolUse" and out["permissionDecision"] == "deny"
    assert "a.md: no header" in out["permissionDecisionReason"] and "doc-critic" in out["permissionDecisionReason"]
    assert hook(tmp_path, {"tool_input": {"command": "cd x && git -C y commit -m x"}}).stdout.strip() != ""


def test_hook_allows_good_doc(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="current")
    record(tmp_path, "a.md")
    r = hook(tmp_path)
    assert (r.returncode, r.stdout) == (0, "")


def test_hook_needs_staged_review_row(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="current")
    assert cli(tmp_path, "record", "a.md", "PASS").returncode == 0
    assert "no PASS" in hook(tmp_path).stdout


def test_hook_ignores_other_commands(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", raw="No header.\n")
    for command in ("echo git commit", "git status", "ls"):
        assert hook(tmp_path, {"tool_input": {"command": command}}).stdout == ""


def test_hook_allows_repo_without_standard(tmp_path):
    repo(tmp_path, standard=False)
    put(tmp_path, "a.md", raw="No header.\n")
    assert hook(tmp_path).stdout == ""


def test_hook_denies_on_internal_error(tmp_path):
    repo(tmp_path)
    r = subprocess.run([sys.executable, str(SCRIPT), "--hook"], input="not json", capture_output=True, text=True,
                       env={**os.environ, "CLAUDE_PROJECT_DIR": str(tmp_path)})
    assert json.loads(r.stdout)["hookSpecificOutput"]["permissionDecision"] == "deny"


def broken_worktree(root):
    """a.md staged valid and reviewed; the worktree copy then loses its header."""
    put(root, "a.md", status="current")
    record(root, "a.md")
    Path(root, "a.md").write_text("No header.\n")


@pytest.mark.parametrize("command,denied", [
    ("git commit -m x", False),
    ('git commit -m "use the -a flag"', False),
    ("git commit -am x", True),
    ("git commit -sam x", True),
    ("git commit --all -m x", True),
    ("git commit -m x a.md", True),
    ("git commit -m x -- a.md", True),
    ("cd x && git commit -a -m x && git status", True),
])
def test_hook_worktree_commits(tmp_path, command, denied):
    repo(tmp_path)
    broken_worktree(tmp_path)
    assert (hook(tmp_path, {"tool_input": {"command": command}}).stdout != "") is denied


def test_hook_worktree_commit_reads_worktree_ledger(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="current")
    record(tmp_path, "a.md")
    Path(tmp_path, "a.md").write_text(Path(tmp_path, "a.md").read_text() + "More.\n")
    git(tmp_path, "add", "a.md")
    assert cli(tmp_path, "record", "a.md", "PASS").returncode == 0
    assert "no PASS" in hook(tmp_path).stdout
    assert hook(tmp_path, {"tool_input": {"command": "git commit -am x"}}).stdout == ""
