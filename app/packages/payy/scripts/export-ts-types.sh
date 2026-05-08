#!/bin/bash
set -euo pipefail

workspace_root="$(git rev-parse --show-toplevel)"
script_dir="$(dirname "$0")"
bindings_dir="${workspace_root}/app/packages/payy/src/ts-rs-bindings"

echo "🔄 Exporting TypeScript bindings from Rust crates..."

echo "🧪 Running collision check + per-package export..."
TS_RS_MERGE_EXPORT_DIR="${bindings_dir}" bash "${script_dir}/check-ts-collisions.sh"

echo "✅ TypeScript bindings exported from all crates."
echo "📋 Checking generated types ..."

# Type-check only the generated TypeScript types using the custom tsconfig
cd "${workspace_root}/app/packages/payy"

# Keep this aligned with the app TypeScript toolchain (see app/yarn.lock).
TS_RS_TS_VERSION="5.9.3"
echo "Using TypeScript ${TS_RS_TS_VERSION} for generated-bindings type checking."

# Note: this expects at least one top-level `src/ts-rs-bindings/*.ts` file.
# If we adopt namespaced exports via `#[ts(export_to = ".../")]`, revisit this
# check and `src/ts-rs-bindings/tsconfig.tsrs.json` include patterns.
if ls src/ts-rs-bindings/*.ts && npx --yes --package typescript@${TS_RS_TS_VERSION} tsc --project src/ts-rs-bindings/tsconfig.tsrs.json; then
  echo "✅ TypeScript type check passed for generated bindings."
else
  echo "❌ TypeScript type check failed for generated bindings!"
  exit 1
fi
