#!/usr/bin/env bash
# PostToolUse(Bash) hook: when a .docx memo is produced via officecli, require a
# memo-reviewer pass. Fires only on memo CREATION (not on reads/edits), so it nudges
# once at the moment a memo is born. Exit 2 surfaces the message back to the agent.
# Reads the hook payload (tool_input.command) from stdin as JSON.
set -uo pipefail

cmd="$(python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_input',{}).get('command',''))" 2>/dev/null)" || exit 0

# Match a .docx being produced, however it is made: officecli create/merge, pandoc,
# python-docx (Document().save), or libreoffice/soffice --convert-to docx.
case "$cmd" in
  *officecli*create*.docx*|*officecli*merge*.docx*|*pandoc*.docx*|*--convert-to*docx*|*.save\(*.docx*|*python-docx*)
    echo "MEMO PRODUCED (.docx). Repo policy: every memo must be reviewed by the memo-reviewer agent before it is committed or merged. Run memo-reviewer (target=good) on the new .docx, address its findings, and loop until it PASSes. See .claude/agents/memo-reviewer.md and memo-review-rubric.md. (Decks use deck-reviewer; this rule is for .docx memos.)" >&2
    exit 2 ;;
esac
exit 0
