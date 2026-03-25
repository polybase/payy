#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=install-beam.sh
source "${SCRIPT_DIR}/install-beam.sh"

assert_eq() {
  local expected="$1"
  local actual="$2"

  if [[ "$expected" != "$actual" ]]; then
    printf 'expected %s, got %s\n' "$expected" "$actual" >&2
    exit 1
  fi
}

assert_contains() {
  local expected="$1"
  local actual="$2"

  if [[ "$actual" != *"$expected"* ]]; then
    printf 'expected %s to contain %s\n' "$actual" "$expected" >&2
    exit 1
  fi
}

test_default_release_repo_points_to_public_mirror() {
  assert_eq "polybase/payy" "$REPO"
}

test_stable_tags_order_newest_stable_beam_releases_first() {
  local actual
  actual="$(
    cat <<'JSON' | stable_tags_from_releases_json
[
  {
    "tag_name": "1.1.12-1",
    "draft": false,
    "prerelease": false
  },
  {
    "tag_name": "beam-v0.1.0",
    "draft": false,
    "prerelease": false
  },
  {
    "tag_name": "beam-v0.3.0",
    "draft": false,
    "prerelease": false
  }
]
JSON
  )"

  assert_eq $'beam-v0.3.0\nbeam-v0.1.0' "$actual"
}

test_stable_tags_ignore_drafts_and_prereleases() {
  local actual
  actual="$(
    cat <<'JSON' | stable_tags_from_releases_json
[
  {
    "tag_name": "beam-v0.4.0",
    "draft": true,
    "prerelease": false
  },
  {
    "tag_name": "beam-v0.5.0",
    "draft": false,
    "prerelease": true
  },
  {
    "tag_name": "beam-v0.3.0",
    "draft": false,
    "prerelease": false
  }
]
JSON
  )"

  assert_eq "beam-v0.3.0" "$actual"
}

test_stable_tags_handle_nested_release_objects() {
  local actual
  actual="$(
    cat <<'JSON' | stable_tags_from_releases_json
[
  {
    "url": "https://api.github.com/repos/polybase/payy/releases/1",
    "tag_name": "beam-v0.6.0",
    "draft": false,
    "prerelease": false,
    "author": {
      "login": "polybase"
    },
    "assets": [
      {
        "name": "beam-x86_64-unknown-linux-gnu"
      }
    ]
  }
]
JSON
  )"

  assert_eq "beam-v0.6.0" "$actual"
}

test_stable_tags_prefer_highest_numeric_version() {
  local actual
  actual="$(
    cat <<'JSON' | stable_tags_from_releases_json
[
  {
    "tag_name": "beam-v0.9.9",
    "draft": false,
    "prerelease": false
  },
  {
    "tag_name": "beam-v0.10.0",
    "draft": false,
    "prerelease": false
  }
]
JSON
  )"

  assert_eq $'beam-v0.10.0\nbeam-v0.9.9' "$actual"
}

test_stable_tags_ignore_semver_prerelease_tags_even_when_public() {
  local actual
  actual="$(
    cat <<'JSON' | stable_tags_from_releases_json
[
  {
    "tag_name": "beam-v0.10.0-rc.1",
    "draft": false,
    "prerelease": false
  },
  {
    "tag_name": "beam-v0.9.9",
    "draft": false,
    "prerelease": false
  }
]
JSON
  )"

  assert_eq "beam-v0.9.9" "$actual"
}

test_stable_tags_use_releases_endpoint() {
  local actual
  local args_file
  args_file="$(mktemp)"

  curl() {
    printf '%s\n' "$*" >> "${args_file}"
    if [[ "$*" == "-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=1" ]]; then
      cat <<'JSON'
[
  {
    "tag_name": "beam-v0.7.0",
    "draft": false,
    "prerelease": false
  }
]
JSON
      return
    fi

    if [[ "$*" == "-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=2" ]]; then
      printf '[]\n'
      return
    fi

    printf 'unexpected curl args: %s\n' "$*" >&2
    exit 1
  }

  actual="$(stable_tags)"

  assert_eq "beam-v0.7.0" "$actual"
  local expected_calls
  expected_calls="$(cat <<EOF
-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=1
-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=2
EOF
)"
  assert_eq \
    "${expected_calls}" \
    "$(cat "${args_file}")"
  rm -f "${args_file}"
}

test_stable_tags_scan_multiple_pages_for_beam_releases() {
  local actual
  local args_file
  args_file="$(mktemp)"

  curl() {
    printf '%s\n' "$*" >> "${args_file}"

    if [[ "$*" == "-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=1" ]]; then
      cat <<'JSON'
[
  {
    "tag_name": "wallet-v1.0.0",
    "draft": false,
    "prerelease": false
  }
]
JSON
      return
    fi

    if [[ "$*" == "-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=2" ]]; then
      cat <<'JSON'
[
  {
    "tag_name": "beam-v0.7.0",
    "draft": false,
    "prerelease": false
  }
]
JSON
      return
    fi

    if [[ "$*" == "-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=3" ]]; then
      cat <<'JSON'
[
  {
    "tag_name": "beam-v0.8.0",
    "draft": false,
    "prerelease": false
  }
]
JSON
      return
    fi

    if [[ "$*" == "-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=4" ]]; then
      printf '[]\n'
      return
    fi

    printf 'unexpected curl args: %s\n' "$*" >&2
    exit 1
  }

  actual="$(stable_tags)"

  assert_eq $'beam-v0.8.0\nbeam-v0.7.0' "$actual"
  local expected_calls
  expected_calls="$(cat <<EOF
-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=1
-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=2
-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=3
-fsSL https://api.github.com/repos/${REPO}/releases?per_page=100&page=4
EOF
)"
  assert_eq \
    "${expected_calls}" \
    "$(cat "${args_file}")"
  rm -f "${args_file}"
}

test_release_asset_metadata_extracts_matching_asset() {
  local actual
  actual="$(
    cat <<'JSON' | release_asset_metadata_from_json "beam-x86_64-unknown-linux-gnu"
{
  "draft": false,
  "prerelease": false,
  "assets": [
    {
      "name": "beam-aarch64-apple-darwin",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "browser_download_url": "https://example.invalid/beam-aarch64-apple-darwin"
    },
    {
      "name": "beam-x86_64-unknown-linux-gnu",
      "uploader": {
        "login": "polybase"
      },
      "digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "browser_download_url": "https://example.invalid/beam-x86_64-unknown-linux-gnu"
    }
  ]
}
JSON
  )"

  assert_eq \
    $'https://example.invalid/beam-x86_64-unknown-linux-gnu\tsha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' \
    "$actual"
}

test_release_asset_metadata_uses_tag_endpoint() {
  local actual
  local args_file
  args_file="$(mktemp)"

  curl() {
    printf '%s\n' "$*" > "${args_file}"
    cat <<'JSON'
{
  "tag_name": "beam-v0.7.0",
  "draft": false,
  "prerelease": false,
  "assets": [
    {
      "name": "beam-x86_64-unknown-linux-gnu",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "browser_download_url": "https://example.invalid/beam-x86_64-unknown-linux-gnu"
    }
  ]
}
JSON
  }

  actual="$(release_asset_metadata "beam-v0.7.0" "beam-x86_64-unknown-linux-gnu")"

  assert_eq \
    $'https://example.invalid/beam-x86_64-unknown-linux-gnu\tsha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc' \
    "$actual"
  assert_eq "-fsSL https://api.github.com/repos/${REPO}/releases/tags/beam-v0.7.0" "$(cat "${args_file}")"
  rm -f "${args_file}"
}

test_release_asset_metadata_ignores_prerelease_tags() {
  local actual

  curl() {
    cat <<'JSON'
{
  "tag_name": "beam-v0.8.0",
  "draft": false,
  "prerelease": true,
  "assets": [
    {
      "name": "beam-x86_64-unknown-linux-gnu",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "browser_download_url": "https://example.invalid/beam-x86_64-unknown-linux-gnu"
    }
  ]
}
JSON
  }

  actual="$(release_asset_metadata "beam-v0.8.0" "beam-x86_64-unknown-linux-gnu")"

  assert_eq "" "$actual"
}

test_release_asset_metadata_ignores_semver_prerelease_tags_even_when_public() {
  local actual

  curl() {
    cat <<'JSON'
{
  "tag_name": "beam-v0.8.0-rc.1",
  "draft": false,
  "prerelease": false,
  "assets": [
    {
      "name": "beam-x86_64-unknown-linux-gnu",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "browser_download_url": "https://example.invalid/beam-x86_64-unknown-linux-gnu"
    }
  ]
}
JSON
  }

  actual="$(release_asset_metadata "beam-v0.8.0-rc.1" "beam-x86_64-unknown-linux-gnu")"

  assert_eq "" "$actual"
}

test_latest_release_asset_metadata_falls_back_to_newest_complete_release() {
  local actual
  local calls_file
  calls_file="$(mktemp)"

  stable_tags() {
    printf '%s\n' \
      "beam-v1002.0.0" \
      "beam-v1001.0.0" \
      "beam-v1000.0.0"
  }

  release_asset_metadata() {
    printf '%s\n' "$1" >> "${calls_file}"

    case "$1" in
      beam-v1002.0.0)
        printf '%s\n' $'https://example.invalid/beam-v1002.0.0\tsha1:not-valid'
        ;;
      beam-v1001.0.0)
        ;;
      beam-v1000.0.0)
        printf '%s\n' \
          $'https://example.invalid/beam-v1000.0.0\tsha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd'
        ;;
      *)
        printf 'unexpected tag lookup: %s\n' "$1" >&2
        exit 1
        ;;
    esac
  }

  actual="$(latest_release_asset_metadata "beam-x86_64-unknown-linux-gnu")"

  assert_eq \
    $'beam-v1000.0.0\thttps://example.invalid/beam-v1000.0.0\tsha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd' \
    "$actual"
  assert_eq \
    $'beam-v1002.0.0\nbeam-v1001.0.0\nbeam-v1000.0.0' \
    "$(cat "${calls_file}")"
  rm -f "${calls_file}"
}

test_release_sha256_from_digest_normalizes_uppercase_hex() {
  local actual

  actual="$(
    release_sha256_from_digest \
      "beam-x86_64-unknown-linux-gnu" \
      "sha256:ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789"
  )"

  assert_eq "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789" "$actual"
}

test_verify_asset_sha256_accepts_matching_digest() {
  local temp_file
  temp_file="$(mktemp)"

  printf 'beam' > "${temp_file}"

  verify_asset_sha256 \
    "${temp_file}" \
    "beam-x86_64-unknown-linux-gnu" \
    "sha256:ae4b867cf2eeb128ceab8c7df148df2eacfe2be35dbd40856a77bfc74f882236"

  rm -f "${temp_file}"
}

test_verify_asset_sha256_normalizes_uppercase_actual_checksum() {
  local temp_file
  temp_file="$(mktemp)"

  printf 'beam' > "${temp_file}"

  (
    sha256_file() {
      printf '%s\n' "AE4B867CF2EEB128CEAB8C7DF148DF2EACFE2BE35DBD40856A77BFC74F882236"
    }

    verify_asset_sha256 \
      "${temp_file}" \
      "beam-x86_64-unknown-linux-gnu" \
      "sha256:ae4b867cf2eeb128ceab8c7df148df2eacfe2be35dbd40856a77bfc74f882236"
  )

  rm -f "${temp_file}"
}

test_verify_asset_sha256_rejects_mismatch() {
  local stderr_file temp_file
  stderr_file="$(mktemp)"
  temp_file="$(mktemp)"

  printf 'beam' > "${temp_file}"

  if (
    verify_asset_sha256 \
      "${temp_file}" \
      "beam-x86_64-unknown-linux-gnu" \
      "sha256:0000000000000000000000000000000000000000000000000000000000000000"
  ) > /dev/null 2>"${stderr_file}"; then
    printf 'expected checksum verification to fail\n' >&2
    exit 1
  fi

  assert_contains \
    "checksum mismatch for beam-x86_64-unknown-linux-gnu" \
    "$(cat "${stderr_file}")"
  rm -f "${stderr_file}" "${temp_file}"
}

test_default_release_repo_points_to_public_mirror
test_stable_tags_order_newest_stable_beam_releases_first
test_stable_tags_ignore_drafts_and_prereleases
test_stable_tags_handle_nested_release_objects
test_stable_tags_prefer_highest_numeric_version
test_stable_tags_ignore_semver_prerelease_tags_even_when_public
test_stable_tags_use_releases_endpoint
test_stable_tags_scan_multiple_pages_for_beam_releases
test_release_asset_metadata_extracts_matching_asset
test_release_asset_metadata_ignores_prerelease_tags
test_release_asset_metadata_ignores_semver_prerelease_tags_even_when_public
test_release_asset_metadata_uses_tag_endpoint
test_latest_release_asset_metadata_falls_back_to_newest_complete_release
test_release_sha256_from_digest_normalizes_uppercase_hex
test_verify_asset_sha256_accepts_matching_digest
test_verify_asset_sha256_normalizes_uppercase_actual_checksum
test_verify_asset_sha256_rejects_mismatch
