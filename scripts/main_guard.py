"""Server-side main guard: a push to main must be a merged PR or touch only PLANNING/board/.

CLI (used by .github/workflows/main-guard.yml): git diff --no-renames --name-only B A |
python -m scripts.main_guard <1 if A is the merge of a merged PR, else 0>; exit 2 = revert it.
"""
import sys

BOARD = "PLANNING/board/"


def decide(paths, from_merged_pr):
    """'ok' or 'revert' for the paths a push to main changed."""
    return "ok" if from_merged_pr or all(p.startswith(BOARD) for p in paths) else "revert"


def main(from_merged_pr, paths):
    if decide(paths, from_merged_pr) == "ok":
        print("ok")
        return 0
    print("revert: direct push to main changed paths outside PLANNING/board/ (open a PR instead):")
    print("\n".join(p for p in paths if not p.startswith(BOARD)))
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] == "1", sys.stdin.read().splitlines()))
