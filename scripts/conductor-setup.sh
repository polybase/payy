#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required to run workspace setup" >&2
    exit 1
fi

echo "Running cargo xtask setup..."
eval "$(cargo xtask setup)"
