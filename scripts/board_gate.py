"""PreToolUse hook: no edit, commit, push or PR without an active board ticket on the branch (PLANNING/board/README.md).

Reads the hook JSON on stdin; prints a deny decision or nothing. Each action is checked against the repo it targets,
with tickets read from that repo's origin/main (fetched first)."""
import contextlib
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import board  # noqa: E402
import gitcmd  # noqa: E402

EDIT_TOOLS = ("Edit", "Write", "MultiEdit", "NotebookEdit")
GH_TOOLS = tuple("mcp__github__" + t for t in ("create_pull_request", "create_or_update_file", "push_files",
                                                "delete_file", "create_branch"))
BOARD = "PLANNING/board/"
FIX = ("Create the ticket first and fill it in: python scripts/board.py new --title ... --area ... "
       "--owner <github login> --branch <branch> --acceptance ... --status in-progress ; commit only that "
       "file and push it to main; then git switch -c <branch>.")
DIRECT = ("Commits and pushes to main take only PLANNING/board/ files and need a passing "
          "`python scripts/board.py check`; code reaches main through a pull request (mark a ticket done "
          "with a board-only push to main right after the merge).")
STALE = ("This push is not a fast-forward of origin/main: git fetch origin; git switch --detach origin/main, "
         "redo the board-only commit there, push HEAD:main.")


def run(t, *args, timeout=None):
    """git in target t = (cwd, GIT_* env overrides)."""
    return subprocess.run(["git", *args], cwd=t[0], env={**os.environ, **t[1]}, capture_output=True, text=True,
                          timeout=timeout)


def git(t, *args):
    r = run(t, *args)
    if r.returncode:
        raise RuntimeError(f"git {' '.join(args)}: {r.stderr.strip()}")
    return r.stdout.strip()


def extract(t, rev, dest):
    """Unpack rev's PLANNING/board into dest; False when rev has none."""
    tar = Path(dest) / "b.tar"
    ok = not run(t, "archive", "--format=tar", "-o", str(tar), rev, BOARD).returncode
    if ok:
        with tarfile.open(tar) as f:
            f.extractall(dest, filter="data")
    return ok


def advise(base, branch):
    """None if `branch` has exactly one active ticket on origin/main; else why not."""
    act = [x for x in board.load(base) if x.get("status") in board.ACTIVE]
    hits = [x for x in act if x.get("branch") == branch]
    if branch != "main" and len(hits) == 1:
        return None
    msg = ("Work happens on a branch named in your ticket, never on main (or a detached HEAD)." if branch == "main"
           else f"Branch {branch} has {len(hits)} active tickets on origin/main (one allowed): fix the board." if hits
           else f"Branch {branch or '(none)'} has no active (in-progress or review) ticket on origin/main; "
                "the push of the ticket to main is the claim, so git fetch if you pushed it.")
    resume = "".join(f" {x['id']} -> git fetch && git switch {x['branch']};" for x in act)
    return msg + (" To resume an existing ticket:" + resume if resume else "") + " " + FIX


def finished(t, base, pushed, f, x):
    """Is f's only change status -> done, on a ticket whose branch no longer exists on origin (merged and deleted)?"""
    new, was = (Path(d, BOARD, f) for d in (pushed, base))
    gone = run(t, "ls-remote", "--heads", "origin", "refs/heads/" + x["branch"])
    return new.is_file() and new.read_text() == was.read_text().replace(f"status: {x['status']}\n", "status: done\n", 1) \
        and gone.returncode == 0 and not gone.stdout.strip()


def main_push(t, base, branch, src):
    """Why pushing `src` to main is denied: stale, not board-only, invalid pushed board, or another branch's ticket."""
    if run(t, "merge-base", "--is-ancestor", "origin/main", src).returncode:
        return STALE
    changes = git(t, "diff", "--no-renames", "--name-only", "origin/main", src).splitlines()
    with tempfile.TemporaryDirectory() as pushed:
        if not all(p.startswith(BOARD) for p in changes) or not extract(t, src, pushed) or board.problems(pushed):
            return DIRECT
        old = {x["_file"]: x for x in board.load(base)}
        for f in (Path(p).name for p in changes):
            if (x := old.get(f)) and x.get("branch") not in ("", branch) and not finished(t, base, pushed, f, x):
                return (f"A board push to main may change only tickets of the current branch ({branch}), or mark done "
                        f"one whose branch is deleted from origin; {f} belongs to {x['branch']}: ask its owner first.")


def push_reason(t, base, branch, args, why):
    """Denial of a push, by destination: main (board-only) or the current branch (`why`: its ticket's denial)."""
    pos = [a for a in args if not a.startswith("-")]
    if "--mirror" in args:
        return "git push --mirror would overwrite main; push named branches."
    specs = pos[1:] or ([] if "--all" in args else ["HEAD"])
    if "--all" in args:
        specs += git(t, "for-each-ref", "--format=%(refname:short)", "refs/heads").split()
    for spec in sorted(specs, key=lambda s: "main" not in s):  # main's verdict wins
        src, _, dst = spec.lstrip("+").partition(":") if ":" in spec else (spec.lstrip("+"), "", spec.lstrip("+"))
        dst = branch if dst == "HEAD" else dst.removeprefix("refs/heads/")
        if dst == "main" or "*" in dst:
            if not src or "--delete" in args or "-d" in args:
                return "Deleting main is not allowed."
            if (denial := main_push(t, base, branch, src)):
                return denial
        elif why or (dst != branch and not dst.startswith("refs/")):
            return why or f"Push only the current branch ({branch}), which holds your ticket; not {dst}."


def verdict(verb, t, args=(), path=None, head=None):
    """Deny reason for one action on target t = (cwd, GIT_* env), or None."""
    top = Path(run(t, "rev-parse", "--show-toplevel").stdout.strip() or t[0]).resolve()  # t[0]: no repo here
    r = (top, t[1])
    with tempfile.TemporaryDirectory() as base:
        with contextlib.suppress(subprocess.TimeoutExpired):  # a ticket pushed since the last fetch counts
            run(r, "fetch", "origin", "main", "--quiet", timeout=10)
        if not extract(r, "origin/main", base) and not (top / BOARD).is_dir():
            return None  # another repo
        branch = run(r, "branch", "--show-current").stdout.strip() or "main"  # detached HEAD counts as main
        if verb == "edit":
            rel = Path(path).resolve()
            free = not rel.is_relative_to(top) or rel.relative_to(top).parts[:2] == ("PLANNING", "board") \
                or rel.relative_to(top).parts[:1] == (".delegation",)
            return None if free else advise(base, branch)
        if verb in ("head", "write"):  # a PR's head needs a ticket; a GitHub MCP write also the push rule: head is current
            return advise(base, head or branch) or (f"Write only the current branch ({branch}); not {head}."
                                                    if verb == "write" and head != branch else None)
        if verb == "commit" and branch == "main":
            specs = gitcmd.pathspecs(args)  # what the commit carries: its pathspecs, -a (everything), or the index
            diff = ("HEAD", "--", *[":/" if s == "." else s for s in specs]) if specs else ("--cached",)
            paths = git(t, "diff", "--no-renames", "--name-only", *diff).splitlines()
            ok = all(p.startswith(BOARD) for p in paths) and not board.problems(top)
            return None if ok else DIRECT + " " + FIX
        why = advise(base, branch)
        return push_reason(r, base, branch, args, why) if verb == "push" else why


def reason(p, cwd):
    tool, ti = p.get("tool_name", ""), p.get("tool_input") or {}
    if tool in EDIT_TOOLS:
        path = Path(cwd) / (ti.get("file_path") or ti.get("notebook_path") or "")
        near = next(d for d in (path.parent, *path.parent.parents) if d.is_dir())  # a file in a dir not yet made
        return verdict("edit", (near, {}), path=path)
    if tool in ("Bash", "PowerShell"):
        for verb, args, dirs in gitcmd.calls(ti.get("command", "")):
            head = re.search(r"(?:--head[= ]|-H[= ]?)(?:[^ :]*:)?(\S+)", " ".join(args)) if verb == "pr" else None
            if verb in ("commit", "push", "pr") and "--dry-run" not in args and (
                    why := verdict("head" if verb == "pr" else verb, gitcmd.target(dirs, cwd), args, head=head and head[1])):
                return why
    if tool in GH_TOOLS:
        return verdict("head" if tool.endswith("pull_request") else "write", (cwd, {}),
                       head=ti.get("head") or ti.get("branch") or "(none)")


def main():
    try:
        p = json.load(sys.stdin)
        why = reason(p, p.get("cwd") or os.getcwd())
    except Exception as e:
        why = f"board gate error: {e}"
    if why:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                                                 "permissionDecisionReason": why}}))


main()
