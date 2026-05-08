#!/usr/bin/env bash

# SPEC(docs/specs/privacy-protocol#fixture-generation-steps)
set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

cargo run -p xtask -- noir-fixtures "$@"
