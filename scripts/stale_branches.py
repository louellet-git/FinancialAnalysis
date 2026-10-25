"""Branches safe to delete: merged into main, unprotected, not an open PR's head,
not named by any not-done board ticket (a fresh branch with no commits yet counts as merged).

CLI: python -m scripts.stale_branches BRANCHES MERGED OPEN_PR_HEADS
(newline-separated files; run from the repo root, whose board it reads; fails if the board
is missing); prints one branch per line.
"""
import sys

from pathlib import Path

from scripts.board import load


def protected_branches(root):
    """Branches named by any ticket not yet done; a missing board is an error, never 'none'."""
    if not (Path(root) / "PLANNING" / "board").is_dir():
        raise FileNotFoundError(f"no PLANNING/board under {root}")
    return [t["branch"] for t in load(root) if t.get("status") != "done" and t.get("branch")]


def to_delete(branches, merged_into_main, open_pr_heads, active_ticket_branches=(), protected=("main",)):
    keep = set(protected) | set(open_pr_heads) | set(active_ticket_branches)
    return sorted((set(branches) & set(merged_into_main)) - keep)


if __name__ == "__main__":
    lines = [open(p).read().split() for p in sys.argv[1:4]]
    for branch in to_delete(*lines, protected_branches(".")):
        print(branch)
