#!/bin/bash
set -euo pipefail

# Run the export script
workspace_root="$(git rev-parse --show-toplevel)"
script_dir="$(cd "$(dirname "$0")" && pwd)"
bash "${script_dir}/export-ts-types.sh"

# Check for any changes in generated TypeScript files, including untracked.
# We intentionally validate the entire bindings directory, not just tracked
# files, so newly generated files cannot slip through unnoticed.
bindings_status="$(git -C "${workspace_root}" status --porcelain --untracked-files=all -- app/packages/payy/src/ts-rs-bindings)"
non_delete_status="$(printf '%s\n' "$bindings_status" | grep -Ev '^( D|D |DD) ' || true)"
if [ -n "$non_delete_status" ]; then
  echo "❌ Rust and TypeScript types are out of sync!"
  echo "$non_delete_status"
  echo "Please run: scripts/export-ts-types.sh and commit all changes under app/packages/payy/src/ts-rs-bindings."
  exit 1
else
  echo "✅ Rust and TypeScript types are in sync."
fi 
