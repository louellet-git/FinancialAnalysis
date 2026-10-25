"""board_gate PreToolUse hook: run as a subprocess against tmp git repos."""
import json
import subprocess
import sys
from pathlib import Path

import pytest

from tests.test_board import put

GATE = Path(__file__).resolve().parent.parent / "scripts" / "board_gate.py"


def git(repo, *args):
    return subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-C", str(repo), *args],
                          check=True, capture_output=True, text=True).stdout


def commit(repo, name, text="changed\n"):
    f = repo / name
    f.parent.mkdir(parents=True, exist_ok=True)
    f.write_text(text)
    git(repo, "add", name)
    git(repo, "commit", "-m", "c")


def hook(repo, tool, **tool_input):
    """The hook's decision dict, or None when it prints nothing (allow)."""
    r = subprocess.run([sys.executable, str(GATE)], capture_output=True, text=True,
                       input=json.dumps({"tool_name": tool, "tool_input": tool_input, "cwd": str(repo)}))
    assert (r.returncode, r.stderr) == (0, "")
    return json.loads(r.stdout)["hookSpecificOutput"] if r.stdout else None


def bash(repo, command):
    return hook(repo, "Bash", command=command)


def edit(repo, rel="financial_analysis/dcf.py"):
    return hook(repo, "Edit", file_path=str(repo / rel))


def denied(out, *fragments):
    assert out["hookEventName"] == "PreToolUse" and out["permissionDecision"] == "deny"
    for f in fragments:
        assert f in out["permissionDecisionReason"]


@pytest.fixture
def repo(tmp_path):
    """Clone of a bare origin: main (origin/main = HEAD) has code.py and a board; branch feat has an in-progress ticket, bare has none."""
    git(tmp_path, "init", "--bare", "-b", "main", "origin.git")
    git(tmp_path, "init", "-b", "main", "repo")
    r = tmp_path / "repo"
    put(r, "feat", status="in-progress", branch="feat")
    (r / "code.py").write_text("x = 1\n")
    git(r, "add", "-A")
    git(r, "commit", "-m", "init")
    git(r, "remote", "add", "origin", str(tmp_path / "origin.git"))
    git(r, "push", "origin", "main")
    git(r, "branch", "feat")
    git(r, "branch", "bare")
    return r


def test_edit_denied_without_ticket_and_names_the_fix(repo):
    git(repo, "switch", "bare")
    denied(edit(repo), "bare", "python scripts/board.py new", "git switch -c")


def test_edit_allowed_with_ticket(repo):
    git(repo, "switch", "feat")
    assert edit(repo) is None


def test_every_edit_tool_is_gated(repo):
    git(repo, "switch", "bare")
    f = str(repo / "code.py")
    for out in (hook(repo, "Write", file_path=f), hook(repo, "MultiEdit", file_path=f),
                hook(repo, "NotebookEdit", notebook_path=f)):
        denied(out, "bare")


def test_edit_denied_on_main(repo):
    denied(edit(repo), "branch named in your ticket")


def test_edit_denied_on_detached_head(repo):
    git(repo, "switch", "--detach")
    denied(edit(repo), "branch named in your ticket")


def test_board_and_delegation_edits_allowed_on_main(repo):
    assert edit(repo, "PLANNING/board/T-20261025-new.md") is None
    assert edit(repo, ".delegation/briefs/x.md") is None


def test_edit_outside_repo_allowed(repo, tmp_path_factory):
    assert hook(repo, "Write", file_path=str(tmp_path_factory.mktemp("o") / "scratch.txt")) is None


def new_ticket(repo):
    put(repo, "new", area="engine")
    git(repo, "add", "PLANNING/board/T-20261025-new.md")


def test_commit_on_main_of_board_only_allowed(repo):
    new_ticket(repo)
    assert bash(repo, "git commit -m ticket") is None


def test_commit_on_detached_head_of_board_only_allowed(repo):
    git(repo, "switch", "--detach")
    new_ticket(repo)
    assert bash(repo, "git commit -m ticket") is None


def test_commit_on_main_of_code_denied(repo):
    (repo / "code.py").write_text("x = 2\n")
    git(repo, "add", "code.py")
    denied(bash(repo, "git commit -m code"), "PLANNING/board/", "python scripts/board.py new")


def test_commit_all_counts_unstaged_code(repo):
    (repo / "code.py").write_text("x = 2\n")
    new_ticket(repo)
    assert bash(repo, "git commit -m ticket") is None
    denied(bash(repo, "git commit -am ticket"), "PLANNING/board/")


def test_commit_on_main_denied_while_board_has_problems(repo):
    put(repo, "broken", status="wip")
    git(repo, "add", "-A")
    denied(bash(repo, "git commit -m ticket"), "board.py check")


def test_commit_and_pr_create_need_ticket_on_a_branch(repo):
    git(repo, "switch", "feat")
    assert bash(repo, "git commit -m x") is None
    assert bash(repo, "gh pr create --fill") is None
    git(repo, "switch", "bare")
    denied(bash(repo, "git commit -m x"), "bare")
    denied(bash(repo, "gh pr create --fill"), "bare")


def test_push_of_non_board_commit_to_main_denied(repo):
    commit(repo, "code.py")
    denied(bash(repo, "git push origin main"), "PLANNING/board/")


def test_push_of_board_only_commit_to_main_allowed(repo):
    new_ticket(repo)
    git(repo, "commit", "-m", "ticket")
    assert bash(repo, "git push origin main") is None


def test_ticketed_branch_cannot_push_code_to_main_but_can_push_itself(repo):
    git(repo, "switch", "feat")
    commit(repo, "code.py")
    denied(bash(repo, "git push origin HEAD:main"), "PLANNING/board/")
    denied(bash(repo, "git push origin feat:refs/heads/main"), "PLANNING/board/")
    assert bash(repo, "git push -u origin feat") is None


def test_detached_head_board_only_push_to_main_allowed(repo):
    git(repo, "switch", "--detach")
    new_ticket(repo)
    git(repo, "commit", "-m", "ticket")
    assert bash(repo, "git push origin HEAD:main") is None


def test_push_on_unticketed_branch_denied(repo):
    git(repo, "switch", "bare")
    denied(bash(repo, "git push origin bare"), "bare")


def test_prose_quotes_and_dry_run_are_not_commands(repo):
    git(repo, "switch", "bare")
    assert bash(repo, 'echo "git push; git commit"') is None
    assert bash(repo, "git push --dry-run origin bare") is None
    assert bash(repo, "git status && git log") is None


def test_gated_command_after_separator_is_caught(repo):
    git(repo, "switch", "bare")
    denied(bash(repo, "cd . && FOO=1 git -C . commit -m x"), "bare")


def test_pr_create_tool_checks_head_ticket(repo):
    denied(hook(repo, "mcp__github__create_pull_request", head="bare"), "bare")
    assert hook(repo, "mcp__github__create_pull_request", head="feat") is None


def test_repo_without_board_allowed(tmp_path):
    git(tmp_path, "init", "-b", "main")
    assert edit(tmp_path) is None


def test_unreadable_board_on_origin_denies_code_edit_but_not_board_edit(repo):
    (repo / "PLANNING/board/T-20261025-bad.md").write_bytes(b"\xff\xfe")
    git(repo, "add", "-A")
    git(repo, "commit", "-m", "bad")
    git(repo, "push", "origin", "main")
    git(repo, "switch", "feat")
    denied(edit(repo), "can't decode")
    assert edit(repo, "PLANNING/board/T-20261025-bad.md") is None


def side_ticket(repo, slug="s"):
    """Branch `side` off main with one board-only commit; leaves main checked out."""
    git(repo, "switch", "-c", "side")
    put(repo, slug)
    git(repo, "add", "-A")
    git(repo, "commit", "-m", "t")
    git(repo, "switch", "main")


def test_push_denied_when_a_merge_commit_carries_code(repo):
    side_ticket(repo)
    git(repo, "merge", "--no-ff", "--no-commit", "side")
    (repo / "code.py").write_text("x = 2\n")
    git(repo, "add", "code.py")
    git(repo, "commit", "-m", "merge")
    denied(bash(repo, "git push origin main"), "PLANNING/board/")


def test_push_denied_when_a_rename_moves_code_into_the_board(repo):
    git(repo, "mv", "code.py", "PLANNING/board/code.txt")
    git(repo, "commit", "-m", "rename")
    denied(bash(repo, "git push origin main"), "PLANNING/board/")


def test_push_all_denied_when_local_main_has_code(repo):
    commit(repo, "code.py")
    git(repo, "switch", "feat")
    denied(bash(repo, "git push --all origin"), "PLANNING/board/")


@pytest.mark.parametrize("cmd", ["git push --mirror origin", "git push origin :main",
                                 "git push origin --delete main", "git push -d origin refs/heads/main"])
def test_push_that_deletes_or_mirrors_main_denied(repo, cmd):
    git(repo, "switch", "feat")
    denied(bash(repo, cmd), "main")


def test_push_that_rewinds_main_denied(repo):
    side_ticket(repo)
    git(repo, "merge", "side")
    git(repo, "push", "origin", "main")
    git(repo, "switch", "feat")
    denied(bash(repo, "git push origin +feat:main"), "git fetch origin; git switch --detach origin/main")


@pytest.fixture
def other(repo, tmp_path):
    """A second clone on branch bare (no ticket); the session repo sits on feat."""
    git(tmp_path, "clone", str(tmp_path / "origin.git"), "other")
    git(tmp_path / "other", "switch", "-c", "bare", "origin/main")
    git(repo, "switch", "feat")
    return tmp_path / "other"


@pytest.mark.parametrize("cmd", ["cd {o} && git commit -m x", "git -C {o} commit -m x",
                                 "git --git-dir={o}/.git --work-tree={o} commit -m x",
                                 "GIT_DIR={o}/.git git commit -m x", "git push origin bare; cd {o}; git push origin bare",
                                 "gh pr create --head bare"])
def test_command_is_checked_against_the_repo_it_targets(repo, other, cmd):
    denied(bash(repo, cmd.format(o=other)), "bare")


def test_command_in_a_ticketed_repo_allowed_from_an_unticketed_session(repo):
    git(repo, "worktree", "add", "../wt-feat", "feat")
    git(repo, "switch", "bare")
    assert bash(repo, f"git -C {repo.parent / 'wt-feat'} commit -m x") is None


def test_edit_in_a_sibling_worktree_checked_against_its_branch(repo):
    git(repo, "worktree", "add", "../wt-bare", "bare")
    denied(hook(repo, "Edit", file_path=str(repo.parent / "wt-bare" / "code.py")), "bare")


@pytest.mark.parametrize("cmd", ["env FOO=1 git commit -m x", "command git commit -m x", "exec git commit -m x",
                                 "nohup git commit -m x", "time git commit -m x", "xargs git commit -m x",
                                 "{ git commit -m x; }", "true; then git commit -m x", "! git commit -m x",
                                 "bash -c 'git commit -m x'", 'sh -c "git commit -m x"', "eval 'git commit -m x'",
                                 "eval git commit -m x"])
def test_wrapped_command_is_still_gated(repo, cmd):
    git(repo, "switch", "bare")
    denied(bash(repo, cmd), "bare")


def test_powershell_commands_are_gated(repo):
    git(repo, "switch", "bare")
    denied(hook(repo, "PowerShell", command="git commit -m x"), "bare")


@pytest.mark.parametrize("tool", ["create_or_update_file", "push_files", "delete_file", "create_branch"])
def test_github_write_tools_need_a_ticketed_branch_never_main(repo, tool):
    tool = "mcp__github__" + tool
    denied(hook(repo, tool, branch="main"), "never on main")
    denied(hook(repo, tool, branch="bare"), "bare")
    assert hook(repo, tool, branch="feat") is None


def test_ticket_only_in_working_tree_does_not_authorize(repo):
    git(repo, "switch", "bare")
    put(repo, "mine", status="in-progress", branch="bare", area="docs")
    denied(edit(repo), "git fetch")
    git(repo, "switch", "main")
    git(repo, "add", "-A")
    git(repo, "commit", "-m", "ticket")
    git(repo, "push", "origin", "main")
    git(repo, "switch", "bare")
    assert edit(repo) is None


def test_denial_names_the_existing_ticket_to_resume(repo):
    git(repo, "switch", "bare")
    denied(edit(repo), "T-20261025-feat", "git fetch && git switch feat")


def test_denial_says_when_a_branch_has_two_tickets(repo):
    put(repo, "twin", status="in-progress", branch="feat", area="docs")
    git(repo, "add", "-A")
    git(repo, "commit", "-m", "twin")
    git(repo, "push", "origin", "main")
    git(repo, "switch", "feat")
    denied(edit(repo), "2 active tickets")
