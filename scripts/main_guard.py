"""Server-side main guard: a push to main must be a merged PR or touch only board tickets and README.

CLI (used by .github/workflows/main-guard.yml): git diff --no-renames --name-only B A |
python -m scripts.main_guard <1 if A is the merge of a merged PR, else 0>; exit 2 = revert it;
python -m scripts.main_guard revert B undoes the push. tests.yml: python -m scripts.main_guard pr-done BASE HEAD.
"""
import subprocess
import sys

from scripts import board


def decide(paths, from_merged_pr):
    """'ok' or 'revert' for the paths a push to main changed (board-only: tickets and README, see board.LANE)."""
    return "ok" if from_merged_pr or all(board.LANE.fullmatch(p) for p in paths) else "revert"


def main(from_merged_pr, paths):
    if decide(paths, from_merged_pr) == "ok":
        print("ok")
        return 0
    print("revert: direct push to main changed paths outside the board lane (PLANNING/board/ tickets and README; open a PR instead):")
    print("\n".join(p for p in paths if not board.LANE.fullmatch(p)))
    return 2


def newly_done(base_root, head_root):
    """Ids of tickets that are done in head_root's board but were not in base_root's (a PR may not set done:
    done is set on main after the merge)."""
    was = {t["id"] for t in board.load(base_root) if t.get("status") == "done"}
    return sorted(t["id"] for t in board.load(head_root) if t.get("status") == "done" and t["id"] not in was)


def revert(before, cwd=None):
    """Undo every push since `before` with one new commit whose tree is before's. Unlike `git revert before..HEAD`
    it needs no -m for merge commits and undoes changes made inside a merge; history is never rewritten."""
    who = ("-c", "user.name=main-guard", "-c", "user.email=main-guard@users.noreply.github.com")
    for cmd in (("read-tree", "-u", "--reset", before), (*who, "commit", "-qm", f"Revert direct push to main since {before[:8]}")):
        subprocess.run(["git", *cmd], cwd=cwd, check=True)


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "revert":
        revert(sys.argv[2])
    elif mode == "pr-done":
        done = newly_done(*sys.argv[2:4])
        if done:
            print("a pull request may not set a ticket to done (done is set on main after the merge):", *done)
        sys.exit(1 if done else 0)
    else:
        sys.exit(main(mode == "1", sys.stdin.read().splitlines()))
