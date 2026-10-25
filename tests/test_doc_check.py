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
            status="current", date="2026-10-25", scope="Test scope.")
LONG = "A rule that is long enough to count as a stated rule, stated here once only."
BAD = "No header.\n"


def git(root, *args):
    subprocess.run(["git", "-C", str(root), "-c", "user.name=t", "-c", "user.email=t@t", *args], check=True,
                   capture_output=True)


def record(root, rel, verdict="PASS"):
    assert doc_check.main(["--root", str(root), "record", rel, verdict]) == 0
    git(root, "add", "-A")


def put(root, rel, raw=None, body="Body.\n", review=True, **over):
    """Write and stage a doc (with a PASS row unless review=False); a None override drops that key."""
    f = {k: v for k, v in {**HEAD, **over}.items() if v is not None}
    p = Path(root, rel)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(raw if raw is not None else "---\n" + "".join(f"{k}: {v}\n" for k, v in f.items()) + "---\n" + body)
    if review:
        record(root, rel)
    git(root, "add", "-A")


def commit(root, origin=False):
    """Commit everything; origin=True also makes it origin/main."""
    git(root, "add", "-A")
    git(root, "commit", "-qm", "c")
    if origin:
        git(root, "update-ref", "refs/remotes/origin/main", "HEAD")


def repo(path, standard=True, base=False):
    path.mkdir(exist_ok=True)
    git(path, "init", "-q")
    if standard:
        put(path, "docs/DOC-STANDARD.md", id="DOC-STD")
    if base:
        commit(path, origin=True)
    return path


def cli(root, *args):
    return subprocess.run([sys.executable, str(SCRIPT), "--root", str(root), *args], capture_output=True, text=True)


def hook(root, command="git commit -m x", cwd=None, home=None, project=True):
    payload = {"tool_input": {"command": command}, **({"cwd": str(cwd)} if cwd else {})}
    env = {k: v for k, v in os.environ.items() if k != "CLAUDE_PROJECT_DIR"}
    env.update(({"CLAUDE_PROJECT_DIR": str(root)} if project else {}), **({"HOME": str(home)} if home else {}))
    return subprocess.run([sys.executable, str(SCRIPT), "--hook"], input=json.dumps(payload), capture_output=True,
                          text=True, env=env)


def denied(result):
    out = result.stdout and json.loads(result.stdout)["hookSpecificOutput"]
    return out and out["permissionDecision"] == "deny" and out["permissionDecisionReason"]


def test_valid_doc_passes(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md")
    assert doc_check.problems(tmp_path) == []
    assert cli(tmp_path, "check").stdout.strip() == "docs ok"
    assert "DOC-A plan current a.md" in cli(tmp_path, "list").stdout


@pytest.mark.parametrize("over,expect", [
    (dict(raw=BAD), "no header"),
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
    put(tmp_path, "a.md", status="draft", review=False)
    assert any("no PASS" in m for m in doc_check.problems(tmp_path))
    record(tmp_path, "a.md", "FAIL")
    assert any("no PASS" in m for m in doc_check.problems(tmp_path))
    record(tmp_path, "a.md")
    assert doc_check.problems(tmp_path) == []
    rows = [r for r in Path(tmp_path, "docs/doc-reviews.tsv").read_text().splitlines() if "\ta.md\t" in r]
    assert [r.split("\t")[1:4:2] for r in rows] == [["a.md", "FAIL"], ["a.md", "PASS"]]
    assert len(rows[1].split("\t")[2]) == 64 and rows[1].endswith("doc-critic/haiku")
    assert Path(tmp_path, "docs/doc-reviews.tsv").read_text().startswith("date\tpath\tsha256\tverdict\treviewer\n")


def test_content_change_after_record_fails(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md")
    Path(tmp_path, "a.md").write_text(Path(tmp_path, "a.md").read_text() + "More.\n")
    assert any("no PASS" in m for m in doc_check.problems(tmp_path))


@pytest.mark.parametrize("status", ["draft", "current", "frozen", "superseded"])
def test_every_status_needs_a_row(tmp_path, status):
    repo(tmp_path)
    put(tmp_path, "a.md", status=status, review=False)
    assert doc_check.problems(tmp_path) == ["a.md: no PASS review row for current content"]
    record(tmp_path, "a.md")
    assert doc_check.problems(tmp_path) == []


def test_exempt_paths_ignored(tmp_path):
    repo(tmp_path)
    for rel in (".claude/agents/x.md", ".claude/commands/x.md", ".claude/skills/s/SKILL.md", ".github/y.md",
                "PLANNING/board/T-20261025-a.md"):
        put(tmp_path, rel, raw=BAD)
    for rel in ("PLANNING/board/README.md", ".claude/notes.md"):
        put(tmp_path, rel, raw=BAD)
    put(tmp_path, "notes.txt", raw="not markdown\n")
    assert [p.as_posix() for p in doc_check.governed(tmp_path)] == [
        ".claude/notes.md", "PLANNING/board/README.md", "docs/DOC-STANDARD.md"]
    assert doc_check.problems(tmp_path) == [".claude/notes.md: no header", "PLANNING/board/README.md: no header"]


def test_untracked_docs_are_governed_unless_ignored(tmp_path):
    repo(tmp_path)
    Path(tmp_path, ".gitignore").write_text("i.md\n")
    Path(tmp_path, "u.md").write_text(BAD)
    Path(tmp_path, "i.md").write_text(BAD)
    assert doc_check.problems(tmp_path) == ["u.md: no header"]


def test_duplicate_line(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", raw=f"---\nid: DOC-A\n---\n{LONG}\n")
    put(tmp_path, "b.md", raw=f"{LONG}\n")
    assert any("duplicate line" in m and "a.md" in m and "b.md" in m for m in doc_check.problems(tmp_path))


def test_duplicate_line_exceptions(tmp_path):
    repo(tmp_path)
    sep = "|" + "-" * 70 + "|"
    put(tmp_path, "a.md", raw=f"{LONG}\n{sep}\n```\n{LONG}\n```\n{LONG[:50]}\n")
    put(tmp_path, "b.md", raw=f"```\n{LONG}\n```\n{sep}\n{LONG[:50]}\n")
    assert not any("duplicate line" in m for m in doc_check.problems(tmp_path))


def test_duplicate_line_skips_frozen_and_superseded(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", status="frozen", body=LONG + "\n")
    put(tmp_path, "b.md", id="DOC-B", status="superseded", body=LONG + "\n")
    put(tmp_path, "c.md", id="DOC-C", body=LONG + "\n")
    assert doc_check.problems(tmp_path) == []
    put(tmp_path, "d.md", id="DOC-D", body=LONG + "\n")
    assert doc_check.problems(tmp_path) == [f"duplicate line in c.md, d.md: {LONG[:50]}"]


def test_crlf_hashes_as_lf(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", review=False)
    text = Path(tmp_path, "a.md").read_text()
    Path(tmp_path, "a.md").write_bytes(text.replace("\n", "\r\n").encode())
    record(tmp_path, "a.md")
    assert doc_check.problems(tmp_path) == []
    Path(tmp_path, "a.md").write_bytes(text.encode())
    assert doc_check.problems(tmp_path) == []


def test_scope_limits_reported_problems(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", body=LONG + "\n")
    put(tmp_path, "b.md", id="DOC-B", body=LONG + "\n")
    put(tmp_path, "c.md", raw=BAD)
    dup = f"duplicate line in a.md, b.md: {LONG[:50]}"
    assert doc_check.problems(tmp_path, scope={"c.md"}) == ["c.md: no header"]
    assert doc_check.problems(tmp_path, scope={"b.md"}) == [dup]
    assert doc_check.problems(tmp_path, scope=set()) == []
    assert sorted(doc_check.problems(tmp_path)) == sorted([dup, "c.md: no header"])


def test_hook_denies_bad_doc_staged_or_chained(tmp_path):
    repo(tmp_path, base=True)
    put(tmp_path, "a.md", raw=BAD)
    out = json.loads(hook(tmp_path).stdout)["hookSpecificOutput"]
    assert out["hookEventName"] == "PreToolUse" and out["permissionDecision"] == "deny"
    assert "a.md: no header" in out["permissionDecisionReason"] and "doc-critic" in out["permissionDecisionReason"]
    git(tmp_path, "reset", "-q")
    assert "a.md: no header" in denied(hook(tmp_path, "git add a.md && git commit -m x"))


@pytest.mark.parametrize("origin", [True, False])
def test_hook_code_only_commit_ignores_legacy_docs(tmp_path, origin):
    repo(tmp_path)
    put(tmp_path, "legacy.md", raw=LONG + "\n", review=False)
    put(tmp_path, "legacy2.md", raw=LONG + "\n", review=False)
    commit(tmp_path, origin=origin)
    put(tmp_path, "code.py", raw="x = 1\n")
    r = hook(tmp_path)
    assert (r.returncode, r.stdout) == (0, "")
    put(tmp_path, "legacy.md", raw=LONG + "\nmore\n", review=False)
    assert "legacy.md: no header" in denied(hook(tmp_path))


def test_hook_duplicate_with_unchanged_doc_denies(tmp_path):
    repo(tmp_path)
    put(tmp_path, "a.md", body=LONG + "\n")
    commit(tmp_path)
    put(tmp_path, "b.md", id="DOC-B", body=LONG + "\n")
    assert "duplicate line in a.md, b.md" in denied(hook(tmp_path))


def test_hook_allows_good_doc_and_reads_worktree_ledger(tmp_path):
    repo(tmp_path, base=True)
    put(tmp_path, "a.md", review=False)
    assert "no PASS" in denied(hook(tmp_path))
    assert doc_check.main(["--root", str(tmp_path), "record", "a.md", "PASS"]) == 0
    r = hook(tmp_path)
    assert (r.returncode, r.stdout) == (0, "")


def test_hook_finds_the_repo_a_commit_targets(tmp_path):
    main = repo(tmp_path / "main", base=True)
    git(main, "worktree", "add", "-q", str(tmp_path / "wt"), "-b", "b")
    put(tmp_path / "wt", "a.md", raw=BAD)
    assert hook(main, "git commit -m x").stdout == ""
    for command, kw in [("cd ../wt && git commit -m x", dict(cwd=main)),
                        (f"cd {tmp_path}/wt && git commit -m x", {}),
                        (f"git -C {tmp_path}/wt commit -m x", {}),
                        ("cd ~/wt && git commit -m x", dict(home=tmp_path)),
                        ("cd $HOME/wt && git commit -m x", dict(home=tmp_path)),
                        ("git commit -m x", dict(cwd=tmp_path / "wt"))]:
        assert "a.md: no header" in denied(hook(main, command, **kw)), command
    assert "a.md: no header" in denied(hook(main, "git commit -m x", cwd=tmp_path / "wt", project=False))


def test_hook_denies_unresolvable_root(tmp_path):
    repo(tmp_path, base=True)
    assert "/nonexistent/dir" in denied(hook(tmp_path, "cd /nonexistent/dir && git commit -m x"))


@pytest.mark.parametrize("command", [
    "git commit -m x && cd {wt}",
    "git commit -m x; git -C {wt} status",
    "cd {wt} && git commit -m x && cd {main}",
])
def test_hook_checks_every_repo_the_command_names(tmp_path, command):
    main = repo(tmp_path / "main", base=True)
    git(main, "worktree", "add", "-q", str(tmp_path / "wt"), "-b", "b")
    put(main if "{main}" not in command else tmp_path / "wt", "a.md", raw=BAD)
    assert "a.md: no header" in denied(hook(main, command.format(wt=tmp_path / "wt", main=main), cwd=main))


@pytest.mark.parametrize("command", [
    "pushd {wt} >/dev/null && git commit -m x",
    "Set-Location {wt}; git commit -m x",
    "cd -P {wt} && git commit -m x",
    "git --git-dir={main}/.git/worktrees/wt --work-tree={wt} commit -m x",
    "GIT_WORK_TREE={wt} git commit -m x",
])
def test_hook_directory_forms(tmp_path, command):
    main = repo(tmp_path / "main", base=True)
    git(main, "worktree", "add", "-q", str(tmp_path / "wt"), "-b", "b")
    put(tmp_path / "wt", "a.md", raw=BAD)
    assert "a.md: no header" in denied(hook(main, command.format(wt=tmp_path / "wt", main=main), cwd=main))


@pytest.mark.parametrize("command", [
    "git merge --no-edit feat", "git cherry-pick feat", "git revert --no-edit HEAD", "git pull --no-rebase . feat",
    "git push origin HEAD", "git am x.patch", "git rebase origin/main",
])
def test_hook_gates_every_history_verb(tmp_path, command):
    repo(tmp_path, base=True)
    put(tmp_path, "a.md", raw=BAD)
    commit(tmp_path)
    assert "a.md: no header" in denied(hook(tmp_path, command))


def test_hook_merge_does_not_launder_a_bad_doc(tmp_path):
    repo(tmp_path, base=True)
    git(tmp_path, "checkout", "-qb", "feat")
    put(tmp_path, "a.md", raw=BAD)
    commit(tmp_path)
    git(tmp_path, "checkout", "-q", "-")
    git(tmp_path, "merge", "-q", "--no-edit", "feat")
    put(tmp_path, "code.py", raw="x = 1\n")
    reason = denied(hook(tmp_path))
    assert "changed on this branch" in reason and "a.md: no header" in reason


def test_hook_legacy_bad_doc_on_origin_does_not_block(tmp_path):
    repo(tmp_path, base=True)
    put(tmp_path, "legacy.md", raw=BAD, review=False)
    commit(tmp_path, origin=True)
    put(tmp_path, "code.py", raw="x = 1\n")
    assert hook(tmp_path).stdout == ""


def test_hook_fires_on_any_git_and_commit(tmp_path):
    repo(tmp_path, base=True)
    put(tmp_path, "a.md", raw=BAD)
    for command in ("echo git commit", "git add a.md; git  commit -m x", 'git commit -m "$(cat <<\'EOF\'\nmsg\nEOF\n)"'):
        assert denied(hook(tmp_path, command)), command
    for command in ("git status", "ls", "echo commit", "git log"):
        assert hook(tmp_path, command).stdout == "", command


def test_hook_allows_repo_without_standard(tmp_path):
    repo(tmp_path, standard=False)
    put(tmp_path, "a.md", raw=BAD, review=False)
    assert hook(tmp_path).stdout == ""


def test_hook_denies_on_internal_error(tmp_path):
    repo(tmp_path)
    r = subprocess.run([sys.executable, str(SCRIPT), "--hook"], input="not json", capture_output=True, text=True,
                       env={**os.environ, "CLAUDE_PROJECT_DIR": str(tmp_path)})
    assert denied(r)
