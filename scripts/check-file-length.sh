#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
TARGET_DIR="$ROOT_DIR/pkg"
DEFAULT_LIMIT=200
OVERRIDE_PATTERN='^[[:space:]]*//[[:space:]]*lint-long-file-override[[:space:]]+allow-max-lines[[:space:]]*=[[:space:]]*([0-9]+)'

if [[ ! -d "$TARGET_DIR" ]]; then
  echo "pkg directory not found at $TARGET_DIR" >&2
  exit 1
fi

violations=()

# Track whether we discovered any Rust files without relying on mapfile (which
# is unavailable on macOS's default bash 3.2).
found_files=0

while IFS= read -r file; do
  found_files=1

  limit=$DEFAULT_LIMIT

  if head -n 1 "$file" | grep -Eq '^//[[:space:]]*@generated'; then
    continue
  fi

  override_line="$(head -n 20 "$file" | grep -E "$OVERRIDE_PATTERN" || true)"
  if [[ -n "$override_line" ]]; then
    if [[ $override_line =~ $OVERRIDE_PATTERN ]]; then
      limit="${BASH_REMATCH[1]}"
    fi
  fi

  line_count=$(wc -l < "$file")
  # Trim whitespace that wc emits
  line_count="${line_count//[[:space:]]/}"

  if [[ "$line_count" =~ ^[0-9]+$ ]] && [[ "$limit" =~ ^[0-9]+$ ]]; then
    if (( line_count > limit )); then
      relative_path="${file#$ROOT_DIR/}"
      violations+=("$relative_path has $line_count lines (limit $limit)")
    fi
  else
    echo "Unable to determine line limit or count for $file" >&2
    exit 1
  fi

done < <(find "$TARGET_DIR" -type f -name '*.rs' -print | sort)

if [[ $found_files -eq 0 ]]; then
  echo "No Rust files found under pkg/."
  exit 0
fi

if [[ ${#violations[@]} -ne 0 ]]; then
  echo "❌ File length check failed. The following files exceed their configured limits:"
  echo ""
  for violation in "${violations[@]}"; do
    echo "- $violation"
  done
  echo ""
  echo "Primary hint: Refactor large files to reduce their length."
  echo "Secondary hint: If the additional length is justified, add an override comment at the top of the file."
  echo "  Example override comment: '// lint-long-file-override allow-max-lines=300' to bump the limit to 300 lines"
  echo "  Bump the limits in increments of 100."
  echo ""
  exit 1
fi

echo "✅ All Rust files in pkg/ comply with the file length limits."
