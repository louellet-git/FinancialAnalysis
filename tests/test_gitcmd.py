"""gitcmd: the shell-line parser shared by the doc_check and board_gate hooks."""
import pytest

from scripts import gitcmd


def test_calls_carry_cd_env_and_git_options_in_order():
    got = gitcmd.calls("cd a && GIT_DIR=b git -c x=y -C c commit -m 'a b'; git push origin HEAD")
    assert got == [("commit", ["-m", "a b"], [("cd", "a"), ("GIT_DIR", "b"), ("-C", "c")]),
                   ("push", ["origin", "HEAD"], [("cd", "a")])]


@pytest.mark.parametrize("cmd", ["bash -c 'git commit -m x'", "eval git commit -m x", "env A=1 git commit -m x",
                                 "{ git commit -m x; }", "xargs git commit -m x", "time git commit -m x"])
def test_calls_see_through_wrappers(cmd):
    assert [c[0] for c in gitcmd.calls(cmd)] == ["commit"]


def test_calls_see_gh_pr_create_and_ignore_prose():
    assert gitcmd.calls("gh pr create --head b") == [("pr", ["--head", "b"], [])]
    assert gitcmd.calls('echo "git push; git commit"; gh pr list') == []


def test_target_follows_dirs_and_expands_home_and_vars(tmp_path, monkeypatch):
    (tmp_path / "wt" / "sub").mkdir(parents=True)
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("SUB", "sub")
    assert gitcmd.target([("cd", "~/wt"), ("-C", "$SUB")], "/") == (tmp_path / "wt" / "sub", {})
    cwd, env = gitcmd.target([("--git-dir", "$HOME/g"), ("GIT_WORK_TREE", "w")], tmp_path)
    assert (cwd, env) == (tmp_path, {"GIT_DIR": str(tmp_path / "g"), "GIT_WORK_TREE": str(tmp_path / "w")})


@pytest.mark.parametrize("dirs", [[("cd", "~/nope")], [("cd", "$NOPE_UNSET/x")], [("-C", "/nonexistent")]])
def test_target_raises_on_an_unresolvable_directory(tmp_path, monkeypatch, dirs):
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.delenv("NOPE_UNSET", raising=False)
    with pytest.raises(ValueError, match="cannot resolve"):
        gitcmd.target(dirs, tmp_path)


def test_pathspecs():
    assert gitcmd.pathspecs(["-m", "x"]) is None
    assert gitcmd.pathspecs(["-m", "x", "a.py", "--", "b.py"]) == ["a.py", "b.py"]
    assert gitcmd.pathspecs(["-am", "x"]) == ["."]


def test_an_empty_argument_does_not_split_the_call():
    assert gitcmd.calls('git commit -m "" code.py') == [("commit", ["-m", "", "code.py"], [])]


def test_env_prefix_binds_to_its_command_but_export_and_bare_assignments_persist():
    d = lambda c: [x[2] for x in gitcmd.calls(c)]
    assert d("GIT_DIR=b git commit") == [[("GIT_DIR", "b")]]
    assert d("GIT_DIR=b true; git commit") == [[]]
    assert d("GIT_DIR=b git status; git commit") == [[("GIT_DIR", "b")], []]
    assert d("export GIT_DIR=b; git commit") == [[("GIT_DIR", "b")]]
    assert d("GIT_DIR=b; git commit") == [[("GIT_DIR", "b")]]
    assert d("cd a; git status && cd b && git commit") == [[("cd", "a")], [("cd", "a"), ("cd", "b")]]


def test_pathspecs_take_attached_option_values():
    assert gitcmd.pathspecs(["-mInit", "code.py"]) == ["code.py"]
    assert gitcmd.pathspecs(["-qmInit", "--", "a.py"]) == ["a.py"]
    assert gitcmd.pathspecs(["-qm", "x", "a.py"]) == ["a.py"]
    assert gitcmd.pathspecs(["-ma", "b.py"]) == ["b.py"]  # `a` is part of the message
    assert gitcmd.pathspecs(["-am", "x"]) == ["."]


def test_target_expands_pwd_and_oldpwd_from_the_given_cwd(tmp_path, monkeypatch):
    monkeypatch.setenv("PWD", "/elsewhere")
    (tmp_path / "a" / "b").mkdir(parents=True)
    assert gitcmd.target([("cd", "$PWD/a"), ("cd", "${PWD}/b")], tmp_path)[0] == tmp_path / "a" / "b"
    assert gitcmd.target([("cd", "a"), ("cd", "$OLDPWD/a/b")], tmp_path)[0] == tmp_path / "a" / "b"
