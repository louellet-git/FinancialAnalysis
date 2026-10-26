import pytest

from scripts.stale_branches import protected_branches, to_delete


def test_merged_branch_deleted():
    assert to_delete(["main", "done"], ["main", "done"], []) == ["done"]


def test_unmerged_branch_kept():
    assert to_delete(["main", "wip"], ["main"], []) == []


def test_main_kept_even_if_merged():
    assert to_delete(["main"], ["main"], []) == []


def test_open_pr_head_kept_even_if_merged():
    assert to_delete(["a", "b"], ["a", "b"], ["a"]) == ["b"]


def test_result_sorted():
    assert to_delete(["z", "a"], ["z", "a"], []) == ["a", "z"]


def test_empty_inputs():
    assert to_delete([], [], []) == []


def test_active_ticket_branch_kept_even_if_merged():
    assert to_delete(["a", "b"], ["a", "b"], [], active_ticket_branches=["a"]) == ["b"]


def ticket(root, slug, status, branch):
    d = root / "PLANNING" / "board"
    d.mkdir(parents=True, exist_ok=True)
    (d / f"T-20261025-{slug}.md").write_text(f"---\nid: T-20261025-{slug}\nstatus: {status}\nbranch: {branch}\n---\n")


def test_protected_branches_are_every_non_done_ticket_branch(tmp_path):
    for slug, status, branch in [("a", "backlog", "bk"), ("b", "in-progress", "ip"), ("c", "review", "rv"), ("d", "done", "dn")]:
        ticket(tmp_path, slug, status, branch)
    assert sorted(protected_branches(tmp_path)) == ["bk", "ip", "rv"]


def test_missing_board_is_an_error(tmp_path):
    with pytest.raises(FileNotFoundError):
        protected_branches(tmp_path)
