#!/usr/bin/env bash
# OfficeCLI setup helper.
#
# Two modes:
#   (default)  CHECK ONLY — used by the SessionStart hook. Reports whether the
#              `officecli` binary is available so the officecli MCP server
#              (.mcp.json) can start. It NEVER downloads or executes remote code.
#   --install  Performs the actual install (downloads + runs the official
#              installer). Run this ONCE, deliberately, by hand — it is not wired
#              into any automatic hook.
set -uo pipefail

INSTALL_CMD='curl -fsSL https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.sh | bash'

if command -v officecli >/dev/null 2>&1; then
  echo "officecli present: $(officecli --version 2>/dev/null | head -1)"
  exit 0
fi

if [ "${1:-}" = "--install" ]; then
  echo "Installing officecli (iOfficeAI/OfficeCLI)…"
  if command -v curl >/dev/null 2>&1; then
    eval "$INSTALL_CMD"
    echo "Done. This repo's .mcp.json already registers the officecli MCP server,"
    echo "so no extra 'officecli mcp claude' step is needed here."
  else
    echo "curl not available; download a binary from https://github.com/iOfficeAI/OfficeCLI/releases" >&2
    exit 1
  fi
  exit 0
fi

# CHECK-ONLY path (hook): inform, do not act.
echo "officecli not installed — the officecli MCP server will be unavailable until you install it."
echo "Install (run by hand, one time):  bash scripts/setup-officecli.sh --install"
echo "  which runs: $INSTALL_CMD"
exit 0
