#!/bin/sh
# Stop hook: count the tokens of this session into one small JSON file (numbers and names only).
# Reads the hook JSON on stdin. Never stops or slows a turn: it exits 0 always, and it is silent.
# Rules: ticket T-20261121-usage-log. No text of the conversation, no model name, no path in the file.
#   - GATE: runs only if the environment variable FA_USAGE_RECORD=1. The repo never sets it. The owner
#     sets it in the settings of his own cloud environment, so it is on in his sessions only.
#     The repo holds no account id and no hash of one.
#   - runs only if rust/target/release/fa exists
#   - finds the repo root from "cwd" of the hook input (walk up to the folder with .git);
#     if that fails, FA_REPO, then the folder of this script
#   - runs at most one time in 10 minutes for each session (stamp file in /tmp)
#   - USAGE_PUSH=1 pushes the file to refs/usage/<short session id> (plumbing only: no index, no work
#     tree, no branch). By default it does not push.
trap 'exit 0' EXIT INT TERM
[ "${FA_USAGE_RECORD:-0}" = "1" ] || exit 0
input=$(cat)
# The work runs in the background, so the turn never waits (a cold read of a large transcript can take
# many seconds of disk wait; it then ends later, or the timeout stops it).
(
  field() { printf '%s' "$input" | sed -n "s/.*\"$1\" *: *\"\([^\"]*\)\".*/\1/p" | head -n 1; }
  sid=$(field session_id)
  tp=$(field transcript_path)
  cwd=$(field cwd)

  root=""
  d=$cwd
  while [ -n "$d" ] && [ "$d" != "/" ]; do
    if [ -e "$d/.git" ]; then root=$d; break; fi
    d=$(dirname "$d")
  done
  [ -n "$root" ] || root=${FA_REPO:-}
  [ -n "$root" ] || root=$(cd "$(dirname "$0")/../.." 2>/dev/null && pwd) || exit 0
  fa="$root/rust/target/release/fa"
  [ -x "$fa" ] || exit 0

  [ -n "$sid" ] && [ -f "$tp" ] || exit 0
  short=$(printf '%s' "$sid" | tr -cd 'a-zA-Z0-9' | cut -c1-8)
  [ -n "$short" ] || exit 0

  stamp="/tmp/fa-usage-stamp-$short"
  if [ -f "$stamp" ] && [ -z "$(find "$stamp" -mmin +10 2>/dev/null)" ]; then exit 0; fi
  : > "$stamp"

  dir=${FA_USAGE_DIR:-/tmp/fa-usage}
  mkdir -p "$dir" || exit 0
  out="$dir/$short.json"
  sub="${tp%.jsonl}/subagents"
  if [ -d "$sub" ]; then
    FA_REPO="$root" timeout 60 "$fa" usage record --transcript "$tp" --subagents "$sub" --out "$out" >/dev/null 2>&1
  else
    FA_REPO="$root" timeout 60 "$fa" usage record --transcript "$tp" --out "$out" >/dev/null 2>&1
  fi
  [ -s "$out" ] || exit 0

  [ "${USAGE_PUSH:-0}" = "1" ] || exit 0
  cd "$root" || exit 0
  blob=$(git hash-object -w "$out") || exit 0
  tree=$(printf '100644 blob %s\tusage.json\n' "$blob" | git mktree) || exit 0
  commit=$(GIT_AUTHOR_NAME=usage GIT_AUTHOR_EMAIL=usage@localhost GIT_COMMITTER_NAME=usage GIT_COMMITTER_EMAIL=usage@localhost \
    git commit-tree "$tree" -m "usage $short") || exit 0
  timeout 20 git push --quiet --force origin "$commit:refs/usage/$short" >/dev/null 2>&1
) </dev/null >/dev/null 2>&1 &
exit 0
