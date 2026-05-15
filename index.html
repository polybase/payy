#!/usr/bin/env bash
set -euo pipefail

REPO="${BEAM_REPO:-polybase/payy}"
INSTALL_DIR="${BEAM_INSTALL_DIR:-$HOME/.local/bin}"
VERSION_INPUT="${1:-${BEAM_VERSION:-}}"

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'beam installer error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

to_lowercase() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

need_sha256_tool() {
  has_cmd sha256sum || has_cmd shasum || has_cmd openssl ||
    fail "missing required command: sha256sum, shasum, or openssl"
}

resolve_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os/$arch" in
    Linux/x86_64)
      printf 'x86_64-unknown-linux-gnu'
      ;;
    Darwin/x86_64)
      printf 'x86_64-apple-darwin'
      ;;
    Darwin/arm64|Darwin/aarch64)
      printf 'aarch64-apple-darwin'
      ;;
    *)
      fail "unsupported platform: ${os} ${arch}"
      ;;
  esac
}

normalize_tag() {
  local version="$1"
  if [[ -z "$version" ]]; then
    return 1
  fi
  if [[ "$version" == beam-v* ]]; then
    printf '%s' "$version"
  else
    printf 'beam-v%s' "$version"
  fi
}

stable_tags_from_releases_json() {
  awk '
    function stable_tag_name(candidate, version) {
      version = candidate
      sub(/^beam-v/, "", version)
      return version ~ /^[0-9]+(\.[0-9]+)*(\+[0-9A-Za-z.-]+)?$/
    }

    function version_is_newer(candidate, current, candidate_parts, current_parts, candidate_count, current_count, part_index, max_count, candidate_part, current_part) {
      sub(/^beam-v/, "", candidate)
      sub(/^beam-v/, "", current)
      candidate_count = split(candidate, candidate_parts, ".")
      current_count = split(current, current_parts, ".")
      max_count = candidate_count > current_count ? candidate_count : current_count

      for (part_index = 1; part_index <= max_count; part_index++) {
        candidate_part = (part_index in candidate_parts) ? candidate_parts[part_index] : 0
        current_part = (part_index in current_parts) ? current_parts[part_index] : 0
        sub(/[^0-9].*$/, "", candidate_part)
        sub(/[^0-9].*$/, "", current_part)
        candidate_part += 0
        current_part += 0

        if (candidate_part > current_part) {
          return 1
        }
        if (candidate_part < current_part) {
          return 0
        }
      }

      return 0
    }

    function insert_candidate(candidate, insert_index, previous) {
      if (candidate in seen) {
        return
      }

      seen[candidate] = 1
      tag_count++
      tags[tag_count] = candidate

      for (insert_index = tag_count; insert_index > 1 && version_is_newer(tags[insert_index], tags[insert_index - 1]); insert_index--) {
        previous = tags[insert_index - 1]
        tags[insert_index - 1] = tags[insert_index]
        tags[insert_index] = previous
      }
    }

    function consider_release(record, candidate) {
      if (record !~ /"draft"[[:space:]]*:[[:space:]]*false/) {
        return
      }
      if (record !~ /"prerelease"[[:space:]]*:[[:space:]]*false/) {
        return
      }
      if (!match(record, /"tag_name"[[:space:]]*:[[:space:]]*"beam-v[^"]*"/)) {
        return
      }

      candidate = substr(record, RSTART, RLENGTH)
      sub(/^.*"tag_name"[[:space:]]*:[[:space:]]*"/, "", candidate)
      sub(/"$/, "", candidate)
      if (!stable_tag_name(candidate)) {
        return
      }

      insert_candidate(candidate)
    }

    {
      json = json $0
    }

    END {
      capture = 0
      depth = 0
      escape = 0
      in_string = 0
      record = ""

      for (char_index = 1; char_index <= length(json); char_index++) {
        ch = substr(json, char_index, 1)

        if (capture) {
          record = record ch
        }

        if (escape) {
          escape = 0
          continue
        }
        if (ch == "\\") {
          if (in_string) {
            escape = 1
          }
          continue
        }
        if (ch == "\"") {
          in_string = !in_string
          continue
        }
        if (in_string) {
          continue
        }

        if (ch == "{") {
          depth++
          if (depth == 1) {
            capture = 1
            record = "{"
          }
          continue
        }
        if (ch == "}") {
          if (depth == 1) {
            consider_release(record)
            capture = 0
            record = ""
          }
          depth--
        }
      }

      for (tag_index = 1; tag_index <= tag_count; tag_index++) {
        print tags[tag_index]
      }
    }
  '
}

stable_tags() {
  local compact_page page page_json

  page=1
  while :; do
    page_json="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases?per_page=100&page=${page}")"
    compact_page="${page_json//[[:space:]]/}"
    if [[ "$compact_page" == "[]" ]]; then
      break
    fi

    printf '%s\n' "$page_json"
    page=$((page + 1))
  done | stable_tags_from_releases_json
}

release_asset_metadata_from_json() {
  local asset_name="$1"

  awk -v asset_name="$asset_name" '
    function consider_object(record, name, url, digest) {
      if (record !~ /"browser_download_url"[[:space:]]*:[[:space:]]*"/) {
        return
      }
      if (record !~ /"digest"[[:space:]]*:[[:space:]]*"/) {
        return
      }
      if (!match(record, /"name"[[:space:]]*:[[:space:]]*"[^"]*"/)) {
        return
      }

      name = substr(record, RSTART, RLENGTH)
      sub(/^.*"name"[[:space:]]*:[[:space:]]*"/, "", name)
      sub(/"$/, "", name)

      if (name != asset_name) {
        return
      }

      if (!match(record, /"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*"/)) {
        return
      }
      url = substr(record, RSTART, RLENGTH)
      sub(/^.*"browser_download_url"[[:space:]]*:[[:space:]]*"/, "", url)
      sub(/"$/, "", url)

      match(record, /"digest"[[:space:]]*:[[:space:]]*"[^"]*"/)
      digest = substr(record, RSTART, RLENGTH)
      sub(/^.*"digest"[[:space:]]*:[[:space:]]*"/, "", digest)
      sub(/"$/, "", digest)

      print url "\t" digest
      exit
    }

    {
      json = json $0
    }

    END {
      depth = 0
      escape = 0
      in_string = 0

      for (char_index = 1; char_index <= length(json); char_index++) {
        ch = substr(json, char_index, 1)

        if (depth > 0) {
          for (record_index = 1; record_index <= depth; record_index++) {
            record[record_index] = record[record_index] ch
          }
        }

        if (escape) {
          escape = 0
          continue
        }
        if (ch == "\\") {
          if (in_string) {
            escape = 1
          }
          continue
        }
        if (ch == "\"") {
          in_string = !in_string
          continue
        }
        if (in_string) {
          continue
        }

        if (ch == "{") {
          depth++
          record[depth] = "{"
          continue
        }
        if (ch == "}") {
          consider_object(record[depth])
          delete record[depth]
          depth--
        }
      }
    }
  '
}

release_is_stable_from_json() {
  awk '
    function stable_tag_name(candidate, version) {
      version = candidate
      sub(/^beam-v/, "", version)
      return version ~ /^[0-9]+(\.[0-9]+)*(\+[0-9A-Za-z.-]+)?$/
    }

    {
      json = json $0
    }

    END {
      if (json ~ /"draft"[[:space:]]*:[[:space:]]*false/ &&
          json ~ /"prerelease"[[:space:]]*:[[:space:]]*false/) {
        if (!match(json, /"tag_name"[[:space:]]*:[[:space:]]*"beam-v[^"]*"/)) {
          exit 1
        }

        candidate = substr(json, RSTART, RLENGTH)
        sub(/^.*"tag_name"[[:space:]]*:[[:space:]]*"/, "", candidate)
        sub(/"$/, "", candidate)

        if (stable_tag_name(candidate)) {
          exit 0
        }
      }

      exit 1
    }
  '
}

release_asset_metadata() {
  local tag="$1"
  local asset_name="$2"
  local release_json

  release_json="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/tags/${tag}")"

  if ! printf '%s\n' "$release_json" | release_is_stable_from_json; then
    return 0
  fi

  printf '%s\n' "$release_json" | release_asset_metadata_from_json "$asset_name"
}

digest_has_valid_sha256() {
  local digest="$1"

  [[ "$digest" =~ ^sha256:[[:xdigit:]]{64}$ ]]
}

latest_release_asset_metadata() {
  local asset_digest asset_metadata asset_name asset_url tag tags
  asset_name="$1"
  tags="$(stable_tags)"

  while IFS= read -r tag; do
    [[ -n "$tag" ]] || continue

    asset_metadata="$(release_asset_metadata "$tag" "$asset_name")"
    [[ -n "$asset_metadata" ]] || continue

    IFS=$'\t' read -r asset_url asset_digest <<< "$asset_metadata"
    [[ -n "$asset_url" && -n "$asset_digest" ]] || continue

    if ! digest_has_valid_sha256 "$asset_digest"; then
      continue
    fi

    printf '%s\t%s\t%s\n' "$tag" "$asset_url" "$asset_digest"
    return 0
  done <<< "$tags"
}

sha256_file() {
  local target="$1"

  if has_cmd sha256sum; then
    sha256sum "$target" | cut -d' ' -f1
    return
  fi

  if has_cmd shasum; then
    shasum -a 256 "$target" | cut -d' ' -f1
    return
  fi

  openssl dgst -sha256 -r "$target" | cut -d' ' -f1
}

release_sha256_from_digest() {
  local asset_name="$1"
  local digest="$2"
  local sha256

  if ! digest_has_valid_sha256 "$digest"; then
    if [[ "$digest" != sha256:* ]]; then
      fail "unsupported release digest for ${asset_name}: ${digest}"
    fi
    fail "invalid SHA-256 digest for ${asset_name}: ${digest}"
  fi

  sha256="${digest#sha256:}"
  to_lowercase "$sha256"
}

verify_asset_sha256() {
  local target="$1"
  local asset_name="$2"
  local digest="$3"
  local actual expected

  expected="$(release_sha256_from_digest "$asset_name" "$digest")"
  actual="$(to_lowercase "$(sha256_file "$target")")"

  if [[ "$actual" != "$expected" ]]; then
    fail "checksum mismatch for ${asset_name} (expected ${expected}, got ${actual})"
  fi
}

ensure_install_dir() {
  mkdir -p "$INSTALL_DIR"
}

main() {
  need_cmd awk
  need_cmd curl
  need_cmd mktemp
  need_cmd chmod
  need_cmd mv
  need_cmd tr
  need_sha256_tool

  local asset_digest asset_metadata asset_name asset_url target tag temp_file version
  target="$(resolve_target)"
  asset_name="beam-${target}"

  if [[ -n "$VERSION_INPUT" ]]; then
    tag="$(normalize_tag "$VERSION_INPUT")"
    version="${tag#beam-v}"
    asset_metadata="$(release_asset_metadata "$tag" "$asset_name")"
    [[ -n "$asset_metadata" ]] ||
      fail "could not resolve stable release metadata for ${asset_name} in ${tag}"
    IFS=$'\t' read -r asset_url asset_digest <<< "$asset_metadata"
    [[ -n "$asset_url" && -n "$asset_digest" ]] ||
      fail "invalid stable release metadata for ${asset_name} in ${tag}"
    digest_has_valid_sha256 "$asset_digest" ||
      fail "invalid stable release metadata for ${asset_name} in ${tag}"
  else
    asset_metadata="$(latest_release_asset_metadata "$asset_name")"
    [[ -n "$asset_metadata" ]] ||
      fail "could not determine the latest complete beam release for ${asset_name}"
    IFS=$'\t' read -r tag asset_url asset_digest <<< "$asset_metadata"
    [[ -n "$tag" && -n "$asset_url" && -n "$asset_digest" ]] ||
      fail "invalid release metadata for ${asset_name}"
    version="${tag#beam-v}"
  fi

  ensure_install_dir
  temp_file="$(mktemp "${TMPDIR:-/tmp}/beam.XXXXXX")"
  trap 'rm -f "$temp_file"' EXIT

  log "Downloading beam ${version} for ${target}..."
  curl -fsSL "$asset_url" -o "$temp_file"
  verify_asset_sha256 "$temp_file" "$asset_name" "$asset_digest"
  chmod 755 "$temp_file"
  mv "$temp_file" "${INSTALL_DIR}/beam"
  trap - EXIT

  log "Installed beam to ${INSTALL_DIR}/beam"
  if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    log "Add ${INSTALL_DIR} to your PATH to run \`beam\` directly."
  fi
  log "Run \`beam --help\` to get started."
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
