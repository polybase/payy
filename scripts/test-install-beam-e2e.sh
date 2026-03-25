#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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

assert_file_exists() {
  local path="$1"

  if [[ ! -f "$path" ]]; then
    printf 'expected file to exist: %s\n' "$path" >&2
    exit 1
  fi
}

assert_file_executable() {
  local path="$1"

  if [[ ! -x "$path" ]]; then
    printf 'expected file to be executable: %s\n' "$path" >&2
    exit 1
  fi
}

compute_sha256() {
  local target="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$target" | cut -d' ' -f1
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$target" | cut -d' ' -f1
    return
  fi

  openssl dgst -sha256 -r "$target" | cut -d' ' -f1
}

host_release_target() {
  local arch os
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
      printf 'unsupported smoke test host: %s %s\n' "$os" "$arch" >&2
      exit 1
      ;;
  esac
}

test_installer_main_installs_latest_release_for_host() {
  (
    set -euo pipefail

    local asset_digest asset_name asset_path asset_url curl_calls expected_target install_dir
    local mock_bin release_tag release_tag_response releases_page_one releases_page_two repo
    local stdout_file stderr_file temp_root tmp_dir

    temp_root="$(mktemp -d)"
    trap 'rm -rf "${temp_root}"' EXIT

    expected_target="$(host_release_target)"
    asset_name="beam-${expected_target}"
    asset_url="https://downloads.example.invalid/${asset_name}"
    release_tag="beam-v9.9.9"
    repo="polybase/payy"

    mock_bin="${temp_root}/mock-bin"
    install_dir="${temp_root}/install"
    tmp_dir="${temp_root}/tmp"
    curl_calls="${temp_root}/curl-calls.txt"
    releases_page_one="${temp_root}/releases-page-1.json"
    releases_page_two="${temp_root}/releases-page-2.json"
    release_tag_response="${temp_root}/release-tag.json"
    asset_path="${temp_root}/${asset_name}"
    stdout_file="${temp_root}/stdout.txt"
    stderr_file="${temp_root}/stderr.txt"

    mkdir -p "${mock_bin}" "${install_dir}" "${tmp_dir}"

    cat <<'EOF' > "${asset_path}"
#!/usr/bin/env bash
printf 'beam smoke install ok\n'
EOF
    chmod 644 "${asset_path}"
    asset_digest="sha256:$(compute_sha256 "${asset_path}")"

    cat <<EOF > "${releases_page_one}"
[
  {
    "tag_name": "${release_tag}",
    "draft": false,
    "prerelease": false
  }
]
EOF
    printf '[]\n' > "${releases_page_two}"

    cat <<EOF > "${release_tag_response}"
{
  "tag_name": "${release_tag}",
  "draft": false,
  "prerelease": false,
  "assets": [
    {
      "name": "beam-unused-target",
      "digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "browser_download_url": "https://downloads.example.invalid/beam-unused-target"
    },
    {
      "name": "${asset_name}",
      "digest": "${asset_digest}",
      "browser_download_url": "${asset_url}"
    }
  ]
}
EOF

    cat <<EOF > "${mock_bin}/curl"
#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "\$*" >> "${curl_calls}"

if [[ "\$#" -eq 2 && "\$1" == "-fsSL" ]]; then
  case "\$2" in
    "https://api.github.com/repos/${repo}/releases?per_page=100&page=1")
      cat "${releases_page_one}"
      exit 0
      ;;
    "https://api.github.com/repos/${repo}/releases?per_page=100&page=2")
      cat "${releases_page_two}"
      exit 0
      ;;
    "https://api.github.com/repos/${repo}/releases/tags/${release_tag}")
      cat "${release_tag_response}"
      exit 0
      ;;
  esac
fi

if [[ "\$#" -eq 4 && "\$1" == "-fsSL" && "\$2" == "${asset_url}" && "\$3" == "-o" ]]; then
  cp "${asset_path}" "\$4"
  exit 0
fi

printf 'unexpected curl args: %s\n' "\$*" >&2
exit 1
EOF
    chmod 755 "${mock_bin}/curl"

    PATH="${mock_bin}:${PATH}" \
      BEAM_INSTALL_DIR="${install_dir}" \
      BEAM_REPO="${repo}" \
      TMPDIR="${tmp_dir}" \
      /bin/bash "${SCRIPT_DIR}/install-beam.sh" > "${stdout_file}" 2> "${stderr_file}"

    assert_eq "" "$(cat "${stderr_file}")"
    assert_file_exists "${install_dir}/beam"
    assert_file_executable "${install_dir}/beam"
    assert_eq "beam smoke install ok" "$("${install_dir}/beam")"

    assert_contains "Downloading beam 9.9.9 for ${expected_target}..." "$(cat "${stdout_file}")"
    assert_contains "Installed beam to ${install_dir}/beam" "$(cat "${stdout_file}")"
    assert_contains "Add ${install_dir} to your PATH to run \`beam\` directly." "$(cat "${stdout_file}")"
    assert_contains "Run \`beam --help\` to get started." "$(cat "${stdout_file}")"

    assert_eq "4" "$(awk 'END { print NR }' "${curl_calls}")"
    assert_eq \
      "-fsSL https://api.github.com/repos/${repo}/releases?per_page=100&page=1" \
      "$(sed -n '1p' "${curl_calls}")"
    assert_eq \
      "-fsSL https://api.github.com/repos/${repo}/releases?per_page=100&page=2" \
      "$(sed -n '2p' "${curl_calls}")"
    assert_eq \
      "-fsSL https://api.github.com/repos/${repo}/releases/tags/${release_tag}" \
      "$(sed -n '3p' "${curl_calls}")"
    assert_contains \
      "-fsSL ${asset_url} -o " \
      "$(sed -n '4p' "${curl_calls}")"
  )
}

test_installer_main_installs_latest_release_for_host
