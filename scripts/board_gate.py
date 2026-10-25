"""PreToolUse hook: no edit, commit, push or PR without an active board ticket on the branch.

Reads the hook JSON on stdin; prints a deny decision or nothing (allow). Rules: PLANNING/board/README.md.
Every action is checked against the repo it targets, and tickets are read from that repo's origin/main.
"""
import json
import os
import re
import shlex
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import board  # noqa: E402

EDIT_TOOLS = ("Edit", "Write", "MultiEdit", "NotebookEdit")
HEAD_TOOLS = tuple("mcp__github__" + t for t in
                   ("create_pull_request", "create_or_update_file", "push_files", "delete_file", "create_branch"))
PUNCT = ";&|<>()\n"
WRAPPERS = {"env", "command", "exec", "nohup", "time", "xargs", "export", "{", "then", "else", "do", "if",
            "while", "until", "!"}
SHELLS = {"bash", "sh", "zsh", "dash"}
GIT_PATHS = {"--git-dir": "GIT_DIR", "--work-tree": "GIT_WORK_TREE"}
BOARD = "PLANNING/board/"
FIX = ("Create the ticket first and fill it in: python scripts/board.py new --title ... --area ... "
       "--owner <github login> --branch <branch> --acceptance ... --status in-progress ; commit only that "
       "file and push it to main; then git switch -c <branch>.")
DIRECT = ("Commits and pushes to main take only PLANNING/board/ files and need a passing "
          "`python scripts/board.py check`; code reaches main through a pull request (mark a ticket done "
          "with a board-only push to main right after the merge).")
STALE = ("This push is not a fast-forward of origin/main: git fetch origin; git switch --detach origin/main, "
         "redo the board-only commit there, push HEAD:main.")


def run(t, *args):
    """git in target t = (cwd, GIT_* env overrides)."""
    return subprocess.run(["git", *args], cwd=t[0], env={**os.environ, **t[1]}, capture_output=True, text=True)


def git(t, *args):
    r = run(t, *args)
    if r.returncode:
        raise RuntimeError(f"git {' '.join(args)}: {r.stderr.strip()}")
    return r.stdout.strip()


def gated(cmd, cwd, env=None):
    """(verb, target, args) for each `git commit`, `git push` and `gh pr create` at command position.

    Follows cd, -C, --git-dir, --work-tree, GIT_DIR and wrappers (env, xargs, bash -c, eval ...) to the repo hit.
    """
    lex = shlex.shlex(cmd, posix=True, punctuation_chars=PUNCT)
    lex.whitespace, lex.whitespace_split, lex.commenters = " \t\r", True, ""
    try:
        toks = list(lex)
    except ValueError:  # unbalanced quote (e.g. an apostrophe in a heredoc): not a runnable git line
        return []
    segs, cur = [], []
    for t in toks:
        if set(t) <= set(PUNCT):
            segs, cur = segs + [cur], []
        else:
            cur.append(t)
    env, out = dict(env or {}), []
    for seg in segs + [cur]:
        while seg:
            if seg[0] in WRAPPERS:
                seg = seg[1:]
                while seg and seg[0].startswith("-"):
                    seg = seg[1:]
            elif "=" in seg[0] and not seg[0].startswith("-"):  # leading VAR=value
                k, _, v = seg[0].partition("=")
                if k in GIT_PATHS.values():
                    env[k] = str(Path(cwd) / v)
                seg = seg[1:]
            else:
                break
        head, rest = (seg or [""])[0], seg[1:]
        flag = next((i for i, a in enumerate(rest) if re.fullmatch(r"-\w*c", a)), None)
        if head in ("cd", "pushd") and rest:
            cwd = Path(cwd) / rest[0]
        elif head in SHELLS and flag is not None and flag + 1 < len(rest):
            out += gated(rest[flag + 1], cwd, env)
        elif head == "eval":
            out += gated(" ".join(rest), cwd, env)
        elif head == "gh" and rest[:2] == ["pr", "create"]:
            out.append(("pr", (cwd, env), rest[2:]))
        elif head == "git":
            c, e, i = cwd, dict(env), 0
            while i < len(rest) and rest[i].startswith("-"):
                o, _, v = rest[i].partition("=")
                if o == "-C":
                    c, i = Path(c) / rest[i + 1], i + 1
                elif o in GIT_PATHS:
                    e[GIT_PATHS[o]], i = str(Path(c) / (v or rest[i + 1])), i + (not v)
                elif o == "-c":
                    i += 1
                i += 1
            if rest[i:i + 1] in (["commit"], ["push"]) and "--dry-run" not in rest:
                out.append((rest[i], (c, e), rest[i + 1:]))
    return out


def advise(base, branch):
    """None if `branch` has exactly one active ticket on origin/main; else why not."""
    act = [x for x in board.load(base) if x.get("status") in board.ACTIVE]
    hits = [x for x in act if x.get("branch") == branch]
    if branch != "main" and len(hits) == 1:
        return None
    if branch == "main":
        msg = "Work happens on a branch named in your ticket, never on main (or a detached HEAD)."
    elif hits:
        msg = f"Branch {branch} has {len(hits)} active tickets on origin/main (one allowed): fix the board."
    else:
        msg = (f"Branch {branch or '(none)'} has no active (in-progress or review) ticket on origin/main; "
               "the push of the ticket to main is the claim, so git fetch if you pushed it.")
    resume = "".join(f" {x['id']} -> git fetch && git switch {x['branch']};" for x in act)
    return msg + (" To resume an existing ticket:" + resume if resume else "") + " " + FIX


def main_push(t, top, src):
    """Why pushing `src` to main is denied: it must fast-forward origin/main and change only board files."""
    if run(t, "merge-base", "--is-ancestor", "origin/main", src).returncode:
        return STALE
    paths = git(t, "diff", "--no-renames", "--name-only", "origin/main", src).splitlines()
    return None if all(p.startswith(BOARD) for p in paths) and not board.problems(top) else DIRECT


def push_reason(t, top, branch, args):
    """(main-push denial or None, whether a non-main ref is pushed)."""
    pos = [a for a in args if not a.startswith("-")]
    if "--mirror" in args:
        return "git push --mirror would overwrite main; push named branches.", True
    specs = pos[1:] or ([] if "--all" in args else ["HEAD"])
    if "--all" in args:
        specs += git(t, "for-each-ref", "--format=%(refname:short)", "refs/heads").split()
    others = False
    for spec in specs:
        spec = spec.lstrip("+")
        src, _, dst = spec.partition(":") if ":" in spec else (spec, "", spec)
        dst = branch if dst == "HEAD" else dst.removeprefix("refs/heads/")
        if dst != "main" and "*" not in dst:
            others = True
        elif not src or "--delete" in args or "-d" in args:
            return "Deleting main is not allowed.", others
        elif (why := main_push(t, top, src)):
            return why, others
    return None, others


def verdict(verb, t, args=(), path=None, head=None):
    """Deny reason for one action on target t, or None."""
    t = (next(d for d in (Path(t[0]), *Path(t[0]).parents) if d.is_dir()), t[1])  # a `cd` into a dir not yet made
    try:
        top = Path(git(t, "rev-parse", "--show-toplevel")).resolve()
    except RuntimeError:
        top = Path(t[0]).resolve()  # outside any repo, or GIT_DIR without a work tree
    t = (top, t[1])
    with tempfile.TemporaryDirectory() as base:
        tar = Path(base) / "b.tar"
        if run(t, "archive", "--format=tar", "-o", str(tar), "origin/main", BOARD).returncode:
            if not (top / BOARD).is_dir():
                return None  # another repo
        else:
            with tarfile.open(tar) as f:
                f.extractall(base, filter="data")
        branch = run(t, "branch", "--show-current").stdout.strip() or "main"  # detached HEAD counts as main
        if verb == "edit":
            rel = Path(path).resolve()
            if not rel.is_relative_to(top) or rel.relative_to(top).parts[:2] == ("PLANNING", "board") \
                    or rel.relative_to(top).parts[:1] == (".delegation",):
                return None
            return advise(base, branch)
        if verb == "head":
            return advise(base, head or branch)
        if verb == "commit" and branch == "main":
            all_ = any(re.fullmatch(r"--all|-[a-zA-Z]*a[a-zA-Z]*", a) for a in args)  # -a also commits unstaged files
            paths = git(t, "diff", "--no-renames", "--name-only", "HEAD" if all_ else "--cached").splitlines()
            ok = all(p.startswith(BOARD) for p in paths) and not board.problems(top)
            return None if ok else DIRECT + (" " + FIX if advise(base, branch) else "")
        why = advise(base, branch)
        if verb == "push":
            denial, others = push_reason(t, top, branch, args)
            return denial or (why if others else None)
        return why


def reason(p, cwd):
    tool, ti = p.get("tool_name", ""), p.get("tool_input") or {}
    if tool in EDIT_TOOLS:
        path = Path(cwd) / (ti.get("file_path") or ti.get("notebook_path") or "")
        return verdict("edit", (path.parent, {}), path=path)
    if tool in ("Bash", "PowerShell"):
        for verb, t, args in gated(ti.get("command", ""), cwd):
            head = None
            if verb == "pr":
                head = next((a.split("=", 1)[-1] for a in args if a.startswith("--head=")), None) or \
                    next((args[i + 1] for i, a in enumerate(args[:-1]) if a in ("--head", "-H")), None)
                verb, head = "head", head and head.rpartition(":")[2]
            if (why := verdict(verb, t, args, head=head)):
                return why
    if tool in HEAD_TOOLS:
        return verdict("head", (cwd, {}), head=ti.get("head") or ti.get("branch") or "(none)")


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
