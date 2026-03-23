#!/bin/bash
set -euo pipefail

# This script detects ts-rs output collisions before we export into the shared
# app `ts-rs-bindings` directory. A collision means two packages generate the
# same relative output path (for example `Status.ts`) with different content.
#
# Why this approach:
# - We export each package into an isolated temporary directory.
# - We compare generated files by relative path + content hash.
# - We fail only when the same relative path has different hashes.
#   (Same file content from multiple packages is allowed.)
#
# Optional output:
# - Set `TS_RS_MERGE_EXPORT_DIR` to materialize the collision-checked output
#   into a directory (for example, the app ts-rs bindings folder).

workspace_root="$(git rev-parse --show-toplevel)"

if ! command -v jq >/dev/null 2>&1; then
  echo "❌ jq is required to discover ts-rs workspace packages."
  echo "   Install with: brew install jq  (macOS) or sudo apt-get install -y jq  (Debian/Ubuntu)"
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  hash_file() {
    sha256sum "$1" | awk '{print $1}'
  }
elif command -v shasum >/dev/null 2>&1; then
  hash_file() {
    shasum -a 256 "$1" | awk '{print $1}'
  }
else
  echo "❌ A SHA-256 tool is required (sha256sum or shasum)."
  exit 1
fi

TS_RS_PACKAGES=()
while IFS= read -r pkg; do
  TS_RS_PACKAGES+=("$pkg")
done < <(
  cargo metadata \
    --manifest-path "${workspace_root}/Cargo.toml" \
    --no-deps \
    --format-version 1 \
    | jq -r '
      .packages[]
      | select(
          (.features | has("ts-rs"))
          and (
            [.targets[].kind[]]
            | map(select(. == "lib" or . == "staticlib" or . == "cdylib"))
            | length > 0
          )
        )
      | .name
    ' \
    | sort -u
)

if [ "${#TS_RS_PACKAGES[@]}" -eq 0 ]; then
  echo "❌ No workspace lib packages with a ts-rs feature were found."
  exit 1
fi

echo "🔎 Checking ts-rs output collisions across packages..."
echo "📦 Packages: ${TS_RS_PACKAGES[*]}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

per_pkg_root="${tmp_dir}/per-package"
mkdir -p "${per_pkg_root}"

exports_manifest="${tmp_dir}/exports.tsv"
touch "${exports_manifest}"

for pkg in "${TS_RS_PACKAGES[@]}"; do
  pkg_export_dir="${per_pkg_root}/${pkg}"
  mkdir -p "${pkg_export_dir}"

  echo "  • Exporting ${pkg}"
  TS_RS_EXPORT_DIR="${pkg_export_dir}" \
    cargo test \
      --manifest-path "${workspace_root}/Cargo.toml" \
      --features ts-rs \
      -p "${pkg}" \
      --lib \
      -- \
      export_bindings_ >/dev/null

  # Record: <relative output path>\t<sha256>\t<package>\t<full path>
  while IFS= read -r -d '' file_path; do
    relative_path="${file_path#${pkg_export_dir}/}"
    file_hash="$(hash_file "${file_path}")"
    printf '%s\t%s\t%s\t%s\n' "${relative_path}" "${file_hash}" "${pkg}" "${file_path}" >>"${exports_manifest}"
  done < <(find "${pkg_export_dir}" -type f -name '*.ts' -print0)
done

if [ ! -s "${exports_manifest}" ]; then
  echo "❌ No TypeScript files were produced during isolated exports."
  exit 1
fi

key_hash_manifest="${tmp_dir}/key-hash.tsv"
cut -f1,2 "${exports_manifest}" | sort -u >"${key_hash_manifest}"

collision_paths="${tmp_dir}/collision-paths.txt"
cut -f1 "${key_hash_manifest}" | sort | uniq -d >"${collision_paths}"

if [ -s "${collision_paths}" ]; then
  echo "❌ ts-rs name collisions detected."
  echo "   The same output path is generated with different type content:"
  while IFS= read -r relative_path; do
    echo "  - ${relative_path}"
    awk -F '\t' -v rel="${relative_path}" '$1 == rel { printf "    - %s (%s)\n", $3, substr($2, 1, 12) }' "${exports_manifest}" | sort -u
  done <"${collision_paths}"

  echo
  echo "Resolve by renaming one type (#[ts(rename = \"...\")])"
  echo "or by namespacing output paths (#[ts(export_to = \"crate_name/\")])."
  exit 1
fi

echo "✅ No ts-rs output collisions found."

if [ -n "${TS_RS_MERGE_EXPORT_DIR:-}" ]; then
  merge_dir="${TS_RS_MERGE_EXPORT_DIR}"
  echo "📁 Writing merged, collision-checked output to: ${merge_dir}"

  mkdir -p "${merge_dir}"
  # Keep non-generated files (for example tsconfig.tsrs.json), replace only
  # generated TypeScript binding files.
  find "${merge_dir}" -type f -name '*.ts' -delete

  for pkg in "${TS_RS_PACKAGES[@]}"; do
    pkg_export_dir="${per_pkg_root}/${pkg}"
    while IFS= read -r -d '' file_path; do
      relative_path="${file_path#${pkg_export_dir}/}"
      destination_path="${merge_dir}/${relative_path}"
      mkdir -p "$(dirname "${destination_path}")"
      cp "${file_path}" "${destination_path}"
    done < <(find "${pkg_export_dir}" -type f -name '*.ts' -print0)
  done
fi
