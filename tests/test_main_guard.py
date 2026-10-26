from scripts.main_guard import decide, main


def test_board_only_direct_push_ok():
    assert decide(["PLANNING/board/T-1.md", "PLANNING/board/README.md"], False) == "ok"


def test_code_in_direct_push_reverts():
    assert decide(["PLANNING/board/T-1.md", "financial_analysis/dcf.py"], False) == "revert"


def test_lookalike_prefix_reverts():
    assert decide(["PLANNING/boards/x.md"], False) == "revert"


def test_merged_pr_ok_with_any_paths():
    assert decide(["financial_analysis/dcf.py"], True) == "ok"


def test_empty_range_ok():
    assert decide([], False) == "ok"


def test_main_prints_offending_paths_and_exit_code(capsys):
    assert main(False, ["PLANNING/board/a.md", "x.py"]) == 2
    out = capsys.readouterr().out
    assert "revert" in out and "x.py" in out and "a.md" not in out
    assert main(True, ["x.py"]) == 0


# ---- round 3 (B5, B7) ----
import subprocess  # noqa: E402
import sys  # noqa: E402

import pytest  # noqa: E402

from scripts import main_guard  # noqa: E402
from tests.test_board import put  # noqa: E402


def test_b5_newly_done_names_tickets_a_pr_sets_to_done(tmp_path):
    base, head = tmp_path / "base", tmp_path / "head"
    put(base, "a", status="review", branch="a", area="docs")
    put(base, "old", status="done")
    put(head, "a", status="done", branch="a", area="docs")  # review -> done
    put(head, "old", status="done")  # was already done
    put(head, "fresh", status="done")  # new file, born done
    put(head, "b", status="backlog")
    assert main_guard.newly_done(base, head) == ["T-20261025-a", "T-20261025-fresh"]
    assert main_guard.newly_done(base, base) == []


def test_b5_pr_done_cli_exit_code_and_listing(tmp_path):
    base, head = tmp_path / "base", tmp_path / "head"
    put(base, "a", status="review", branch="a")
    put(head, "a", status="review", branch="a")
    run = lambda: subprocess.run([sys.executable, "-m", "scripts.main_guard", "pr-done", str(base), str(head)],
                                 capture_output=True, text=True, cwd=main_guard.__file__.rsplit("/scripts/", 1)[0])
    assert run().returncode == 0
    put(head, "a", status="done", branch="a")
    r = run()
    assert r.returncode == 1 and "T-20261025-a" in r.stdout and "done is set on main" in r.stdout


def g(repo, *args):
    return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-C", str(repo), *args],
                          check=True, capture_output=True, text=True).stdout.strip()


@pytest.fixture
def pushed_range(tmp_path):
    """BEFORE, then a direct push of: a main commit, and a --no-ff merge of a side branch with an evil change."""
    r = tmp_path / "r"
    g(tmp_path, "init", "-q", "-b", "main", "r")
    (r / "a.txt").write_text("a\n")
    g(r, "add", "-A"); g(r, "commit", "-qm", "before")
    before = g(r, "rev-parse", "HEAD")
    g(r, "switch", "-qc", "side")
    (r / "code.py").write_text("x = 1\n")
    g(r, "add", "-A"); g(r, "commit", "-qm", "side")
    g(r, "switch", "-q", "main")
    (r / "m.txt").write_text("m\n")
    g(r, "add", "-A"); g(r, "commit", "-qm", "main")
    g(r, "merge", "-q", "--no-ff", "--no-commit", "side")
    (r / "a.txt").write_text("evil\n")
    g(r, "add", "-A"); g(r, "commit", "-qm", "merge")
    return r, before


def test_b7_revert_undoes_a_range_containing_a_merge_without_rewriting_history(pushed_range):
    r, before = pushed_range
    after = g(r, "rev-parse", "HEAD")
    assert subprocess.run(["git", "-C", str(r), "revert", "--no-edit", f"{before}..{after}"], capture_output=True).returncode
    g(r, "revert", "--abort")
    main_guard.revert(before, cwd=r)
    assert g(r, "rev-parse", "HEAD^{tree}") == g(r, "rev-parse", f"{before}^{{tree}}")
    assert g(r, "rev-parse", "HEAD^") == after  # a new commit on top; AFTER stays an ancestor
    assert g(r, "status", "--porcelain") == ""
