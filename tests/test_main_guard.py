from scripts.main_guard import decide, main


def test_board_only_direct_push_ok():
    assert decide(["PLANNING/board/T-1.md", "PLANNING/board/README.md"], False) == "ok"


def test_code_in_direct_push_reverts():
    assert decide(["PLANNING/board/T-1.md", "financial_analysis/dcf.py"], False) == "revert"


def test_lookalike_prefix_reverts():
    assert decide(["PLANNING/boards/x.md"], False) == "revert"


def test_merged_pr_ok_with_any_paths():
    assert decide(["financial_analysis/dcf.py"], True) == "ok"


def test_empty_range_ok():
    assert decide([], False) == "ok"


def test_main_prints_offending_paths_and_exit_code(capsys):
    assert main(False, ["PLANNING/board/a.md", "x.py"]) == 2
    out = capsys.readouterr().out
    assert "revert" in out and "x.py" in out and "a.md" not in out
    assert main(True, ["x.py"]) == 0
