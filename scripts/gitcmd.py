"""Shell-line parser shared by the git hooks (doc_check, board_gate): which git commands a line runs, and where."""
import itertools
import os
import re
import shlex
import subprocess
from pathlib import Path

SHELLS = {"bash", "sh", "zsh", "pwsh", "powershell"}
GIT = re.compile(r"(^|[\\/])git(\.exe)?$")
CD = {"cd", "pushd", "Set-Location"}
ENV = re.compile(r"(GIT_DIR|GIT_WORK_TREE)=(.*)", re.S)
COMMIT_VALUE_OPTS = {"--message", "--file", "--author", "--date", "--template", "--cleanup", "--fixup", "--squash",
                     "--reuse-message", "--reedit-message", "--trailer", "--pathspec-from-file"}
TARGET = re.compile(r"""(?:\b(?:cd|pushd|Set-Location)\s+(?:-\w+\s+)?|\bGIT_(?:DIR|WORK_TREE)=)"""
                    r"""("[^"]*"|'[^']*'|[^\s;&|]+)""")


def git_out(root, *args, check=True):
    return subprocess.run(["git", "-C", str(root), *args], capture_output=True, check=check).stdout


def calls(command, dirs=()):
    """[(subcommand, args, dirs)] per `git` (and `gh pr create` as "pr"); dirs = [(kind, value)] in order: earlier
    "cd"s and GIT_DIR/GIT_WORK_TREE assignments, then its own -C, --git-dir, --work-tree."""
    command = command.replace("\\\n", " ").replace("\n", ";")
    lex = shlex.shlex(command, posix=True, punctuation_chars=True)
    lex.commenters, lex.whitespace_split = "", True
    try:
        toks = list(lex)
    except ValueError:
        toks = command.split()
    out, cds = [], list(dirs)
    for seg in (list(g) for punct, g in itertools.groupby(toks, lambda t: bool(t) and set(t) <= set(";&|()<>")) if not punct):
        g = next((i for i, w in enumerate(seg) if GIT.search(w) or w == "gh"), None)
        if g is None and (seg[0].rsplit("/", 1)[-1] in SHELLS or seg[0] == "eval"):
            out += calls(" ".join(seg[1:]), cds)
        if seg[0] in CD and (a := [w for w in seg[1:] if w == "-" or not w.startswith("-")]):
            cds.append(("pushd" if seg[0] == "pushd" else "cd", a[0]))  # `cd -` is the previous directory
        cds += [("popd", "")] * (seg[0] == "popd")
        env = [(m[1], m[2]) for w in seg[:g] if (m := ENV.fullmatch(w))]  # binds to this command only...
        if g is None:
            cds += env * (seg[0] == "export" or len(env) == len(seg))  # ...unless it is `export X=` or a bare `X=`
            continue
        if seg[g] == "gh":
            out += [("pr", seg[g + 3:], cds + env)] * (seg[g + 1:g + 3] == ["pr", "create"])
            continue
        it, own = iter(seg[g + 1:]), env
        for w in it:
            if w in ("-C", "--git-dir", "--work-tree"):
                own.append((w, next(it, "")))
            elif w in ("-c", "--namespace", "--config-env"):
                next(it, None)
            elif w.startswith(("--git-dir=", "--work-tree=")):
                own.append(tuple(w.split("=", 1)))
            elif not w.startswith("-"):
                out.append((w, list(it), cds + own))
    return out


def target(dirs, cwd):
    """(cwd, GIT_* env) a call with these dirs runs in, ~ and $VARS expanded; ValueError if cwd is no directory."""
    env, old, stack = {}, cwd, []
    for kind, v in dirs:  # $PWD and $OLDPWD are the session's, not the hook process's
        v = re.sub(r"\$\{?(PWD|OLDPWD)\}?", lambda m: str(cwd if m[1] == "PWD" else old), v)
        p = Path(old if v == "-" else Path(cwd, os.path.expandvars(os.path.expanduser(v))))
        if kind == "popd":
            old, cwd = cwd, (stack.pop() if stack else cwd)
        elif kind in ("cd", "-C", "pushd"):
            stack += [cwd] * (kind == "pushd")
            old, cwd = cwd, p
        else:
            env["GIT_DIR" if "dir" in kind.lower() else "GIT_WORK_TREE"] = str(p)
    if not Path(cwd).is_dir():
        raise ValueError(f"cannot resolve a git repo for {cwd}")
    return Path(cwd), env


def pathspecs(args):
    """None for a plain `git commit`; else its pathspecs (["."] for -a/--all). Over-approximates short flags."""
    paths, it = [], iter(args)
    for t in it:
        if t == "--":
            paths += it
        elif t == "--all":
            return ["."]
        elif t in COMMIT_VALUE_OPTS:
            next(it, None)
        elif re.fullmatch(r"-[a-zA-Z]+", t):  # a short cluster: the first value letter takes the rest as its value
            for i, c in enumerate(t[1:], 2):
                if c == "a":
                    return ["."]
                if c in "mFcCt":
                    next(it, None) if i == len(t) else None
                    break
        elif not t.startswith("-"):
            paths.append(t)
    return paths or None


def roots(command, cwd, dirs):
    """(git toplevels, unresolved targets) of the base dir (cwd, else CLAUDE_PROJECT_DIR) and every dir named."""
    base = cwd or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd()
    found, bad = set(), []
    for t in [""] + [t.strip("\"'") for t in TARGET.findall(command)] + [d for d in dirs if d not in ("", "-")]:
        d = Path(base, os.path.expandvars(os.path.expanduser(t)))
        try:
            found.add(git_out(d, "rev-parse", "--show-toplevel").decode().strip())
        except (subprocess.CalledProcessError, OSError):
            bad.append(t or str(base))
    return found, bad
