"""Scrum-board checker: format rules, branch lookup and CLI on tmp_path boards."""
import subprocess
import sys
from pathlib import Path

from scripts import board

SCRIPT = Path(board.__file__)
BASE = dict(title="T", area="docs", status="backlog", owner="luc", branch="",
            created="2026-10-25", acceptance="It works.")


def put(root, slug, **over):
    """Write PLANNING/board/T-20261025-<slug>.md; id defaults to the filename."""
    f = {"id": f"T-20261025-{slug}", **BASE, **over}
    d = root / "PLANNING" / "board"
    d.mkdir(parents=True, exist_ok=True)
    (d / f"T-20261025-{slug}.md").write_text(
        "---\n" + "".join(f"{k}: {v}\n" for k, v in f.items()) + "---\nbody\n")


def cli(root, *args):
    return subprocess.run([sys.executable, str(SCRIPT), "--root", str(root), *args],
                          capture_output=True, text=True)


def test_valid_board_passes(tmp_path):
    put(tmp_path, "a", status="in-progress", branch="b1")
    put(tmp_path, "b", area="ticker:IBM")
    assert board.problems(tmp_path) == []
    assert [t["id"] for t in board.load(tmp_path)] == ["T-20261025-a", "T-20261025-b"]
    assert cli(tmp_path, "check").stdout.strip() == "board ok"


def test_missing_key_fails(tmp_path):
    put(tmp_path, "a")
    p = tmp_path / "PLANNING/board/T-20261025-a.md"
    p.write_text(p.read_text().replace("owner: luc\n", ""))
    assert any("missing key owner" in m for m in board.problems(tmp_path))


def test_empty_key_fails(tmp_path):
    put(tmp_path, "a", acceptance="")
    assert any("empty acceptance" in m for m in board.problems(tmp_path))


def test_id_filename_mismatch_fails(tmp_path):
    put(tmp_path, "a", id="T-20261025-other")
    assert any("filename" in m for m in board.problems(tmp_path))


def test_bad_id_fails(tmp_path):
    put(tmp_path, "A_b", id="T-20261025-A_b")
    assert any("bad id" in m for m in board.problems(tmp_path))


def test_trailing_dot_ticker_area_fails(tmp_path):
    put(tmp_path, "a", area="ticker:IBM.")
    assert any("bad area" in m for m in board.problems(tmp_path))


def test_class_share_ticker_area_passes(tmp_path):
    put(tmp_path, "a", area="ticker:BRK.B")
    assert board.problems(tmp_path) == []


def test_bad_area_fails(tmp_path):
    put(tmp_path, "a", area="ticker:ibm")
    assert any("bad area" in m for m in board.problems(tmp_path))


def test_bad_status_fails(tmp_path):
    put(tmp_path, "a", status="wip")
    assert any("bad status" in m for m in board.problems(tmp_path))


def test_bad_created_fails(tmp_path):
    put(tmp_path, "a", created="2026-13-40")
    assert any("bad created" in m for m in board.problems(tmp_path))


def test_active_without_branch_fails(tmp_path):
    put(tmp_path, "a", status="review", branch="")
    assert any("needs a branch" in m for m in board.problems(tmp_path))


def test_active_without_owner_fails(tmp_path):
    put(tmp_path, "a", status="in-progress", branch="b1", owner="")
    assert any("empty owner" in m for m in board.problems(tmp_path))


def test_two_active_in_one_area_fails(tmp_path):
    put(tmp_path, "a", status="in-progress", branch="b1", area="engine")
    put(tmp_path, "b", status="review", branch="b2", area="engine")
    assert any("area engine" in m for m in board.problems(tmp_path))


def test_active_in_different_areas_passes(tmp_path):
    put(tmp_path, "a", status="in-progress", branch="b1", area="engine")
    put(tmp_path, "b", status="in-progress", branch="b2", area="docs")
    assert board.problems(tmp_path) == []


def test_two_active_on_one_branch_fails(tmp_path):
    put(tmp_path, "a", status="in-progress", branch="b1", area="engine")
    put(tmp_path, "b", status="in-progress", branch="b1", area="docs")
    assert any("branch b1" in m for m in board.problems(tmp_path))


def test_duplicate_id_fails(tmp_path):
    put(tmp_path, "a")
    put(tmp_path, "b", id="T-20261025-a")
    assert any("duplicate id" in m for m in board.problems(tmp_path))


def test_check_cli_exits_1_with_one_line_per_problem(tmp_path):
    put(tmp_path, "a", status="wip", area="nope")
    r = cli(tmp_path, "check")
    assert r.returncode == 1
    assert len(r.stdout.strip().splitlines()) == 2


def test_branch_finds_active_ticket_only(tmp_path):
    put(tmp_path, "a", status="in-progress", branch="b1", area="engine")
    put(tmp_path, "b", status="backlog", branch="b2", area="docs")
    put(tmp_path, "c", status="done", branch="b3", area="importer")
    assert board.ticket_for_branch(tmp_path, "b1")["id"] == "T-20261025-a"
    assert [board.ticket_for_branch(tmp_path, b) for b in ("b2", "b3", "zzz", "")] == [None] * 4
    ok, none = cli(tmp_path, "branch", "b1"), cli(tmp_path, "branch", "zzz")
    assert (ok.returncode, ok.stdout.strip()) == (0, "T-20261025-a")
    assert none.returncode == 1


def test_new_creates_ticket_check_accepts(tmp_path):
    r = cli(tmp_path, "new", "--title", "Fix the Thing!", "--area", "ticker:IBM",
            "--owner", "luc", "--branch", "b1", "--acceptance", "Done.",
            "--status", "in-progress")
    path = Path(r.stdout.strip())
    assert r.returncode == 0 and path.parent == tmp_path / "PLANNING/board"
    assert path.stem.startswith("T-") and path.stem.endswith("-fix-the-thing")
    assert cli(tmp_path, "check").stdout.strip() == "board ok"
    assert board.ticket_for_branch(tmp_path, "b1")["title"] == "Fix the Thing!"


def test_new_refuses_rule_break(tmp_path):
    put(tmp_path, "a", status="in-progress", branch="b1", area="docs")
    r = cli(tmp_path, "new", "--title", "Second", "--area", "docs", "--owner", "luc",
            "--branch", "b2", "--acceptance", "Done.", "--status", "in-progress")
    assert r.returncode == 1 and "area docs" in r.stdout
    assert len(list((tmp_path / "PLANNING/board").glob("*.md"))) == 1


def test_status_lists_active_tickets_only(tmp_path):
    put(tmp_path, "a", status="in-progress", branch="b1", area="engine", owner="luc")
    put(tmp_path, "b", status="backlog", area="docs")
    assert cli(tmp_path, "status").stdout == "T-20261025-a engine luc b1\n"
