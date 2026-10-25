#!/usr/bin/env bash
# Cloud-environment setup script (docs/COLLABORATION.md#environment); idempotent.
set -euo pipefail
cd "$(dirname "$0")/.."
python -m pip install -q -r requirements.txt -r requirements-dev.txt
