#!/usr/bin/env bash
set -euo pipefail

readonly BEAM_MANIFEST="pkg/beam-cli/Cargo.toml"
readonly PAYY_REPO_URL="${PAYY_REPO_URL:-https://github.com/polybase/payy.git}"

current_version() {
  awk -F' = ' '/^version = / { gsub(/"/, "", $2); print $2; exit }' "$BEAM_MANIFEST"
}

validate_version() {
  local version="$1"
  if [[ ! "$version" =~ ^[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
    echo "Beam version must be semver-like: ${version}" >&2
    exit 1
  fi
}

release_type_from_commits() {
  local before="$1"
  local sha="$2"
  local messages

  messages="$(git log --format=%B "${before}..${sha}")"

  if grep -Eq '(^|[[:space:]])BREAKING CHANGE:|(^|[[:space:]])[*-]?[[:space:]]*[a-zA-Z]+(\([^)]+\))?!:' <<<"$messages"; then
    printf '%s\n' "major"
  elif grep -Eq '(^|[[:space:]])[*-]?[[:space:]]*feat(\([^)]+\))?!?:' <<<"$messages"; then
    printf '%s\n' "minor"
  else
    printf '%s\n' "patch"
  fi
}

relevant_beam_change_exists() {
  local before="$1"
  local sha="$2"

  git diff --name-only "$before" "$sha" |
    grep -Eq '^(pkg/beam-cli/(Cargo[.]toml|src/)|scripts/install-beam[.]sh$|Cargo[.]toml$|Cargo[.]lock$|rust-toolchain[.]toml$)'
}

push_is_release_bump() {
  local before="$1"
  local sha="$2"

  git log --format=%s "${before}..${sha}" | grep -Eq '^chore\(beam\): release [0-9]+[.][0-9]+[.][0-9]+'
}

beam_version_changed() {
  local before="$1"
  local sha="$2"

  git diff "$before" "$sha" -- "$BEAM_MANIFEST" |
    grep -Eq '^[+-]version = "[0-9]+[.][0-9]+[.][0-9]+'
}

bump_version() {
  local version="$1"
  local release_type="$2"
  local major minor patch

  IFS='.' read -r major minor patch <<<"${version%%[-+]*}"

  case "$release_type" in
    major)
      major=$((major + 1))
      minor=0
      patch=0
      ;;
    minor)
      minor=$((minor + 1))
      patch=0
      ;;
    patch)
      patch=$((patch + 1))
      ;;
    *)
      echo "Unsupported release type: ${release_type}" >&2
      exit 1
      ;;
  esac

  printf '%s.%s.%s\n' "$major" "$minor" "$patch"
}

version_gt() {
  local candidate="$1"
  local current="$2"
  local candidate_major candidate_minor candidate_patch
  local current_major current_minor current_patch

  IFS='.' read -r candidate_major candidate_minor candidate_patch <<<"${candidate%%[-+]*}"
  IFS='.' read -r current_major current_minor current_patch <<<"${current%%[-+]*}"

  if ((candidate_major != current_major)); then
    ((candidate_major > current_major))
    return
  fi

  if ((candidate_minor != current_minor)); then
    ((candidate_minor > current_minor))
    return
  fi

  ((candidate_patch > current_patch))
}

ensure_current_version_is_public() {
  local version="$1"
  local tag="beam-v${version}"

  if ! git ls-remote --exit-code --tags "$PAYY_REPO_URL" "refs/tags/${tag}" >/dev/null 2>&1; then
    echo "Payy does not have ${tag}; skipping another Beam bump until the current version publishes."
    exit 0
  fi
}

public_release_source_rev() {
  local version="$1"
  local tag="beam-v${version}"
  local message

  git fetch --depth=1 "$PAYY_REPO_URL" "refs/tags/${tag}" >/dev/null 2>&1 || return 0
  message="$(git show -s --format=%B 'FETCH_HEAD^{commit}' 2>/dev/null || true)"

  awk '/^FolderOrigin-RevId: / { print $2; exit }' <<<"$message"
}

update_manifest_version() {
  local version="$1"

  perl -0pi -e 's/^version = "[^"]+"/version = "$ENV{BEAM_NEXT_VERSION}"/m' "$BEAM_MANIFEST"
}

create_or_update_pr() {
  local version="$1"
  local branch="beam/release-v${version}"
  local title="chore(beam): release ${version}"
  local body

  if [[ "${BEAM_RELEASE_DRY_RUN:-}" == "1" ]]; then
    echo "Would create or update ${branch} with ${title}."
    return
  fi

  body="$(cat <<EOF
Prepares Beam ${version} in the source repository.

After this PR merges and Copybara syncs to polybase/payy, the Payy Beam release workflow publishes the public beam-v${version} assets.
EOF
)"

  git checkout -B "$branch"

  BEAM_NEXT_VERSION="$version" update_manifest_version "$version"
  cargo update -p beam-cli

  if git diff --quiet -- "$BEAM_MANIFEST" Cargo.lock; then
    echo "Beam release ${version} produced no manifest or lockfile changes."
    exit 0
  fi

  git add "$BEAM_MANIFEST" Cargo.lock
  git commit -m "$title" -m "$body"
  git fetch origin "refs/heads/${branch}:refs/remotes/origin/${branch}" || true
  git push --force-with-lease origin "$branch"

  local existing_pr
  existing_pr="$(gh pr list --head "$branch" --base main --json number --jq '.[0].number // empty')"

  if [[ -n "$existing_pr" ]]; then
    gh pr edit "$existing_pr" --title "$title" --body "$body"
    echo "Updated Beam release PR #${existing_pr} for ${version}."
  else
    gh pr create --base main --head "$branch" --title "$title" --body "$body"
  fi
}

main() {
  git config user.name "${GIT_AUTHOR_NAME:-github-actions}"
  git config user.email "${GIT_AUTHOR_EMAIL:-github-actions@github.com}"

  local current
  current="$(current_version)"
  validate_version "$current"

  local exact_version="${BEAM_RELEASE_VERSION:-}"
  local release_type="${BEAM_RELEASE_TYPE:-}"

  if [[ -z "$exact_version" && "${BEAM_RELEASE_EVENT_NAME:-}" == "push" ]]; then
    local before="${BEAM_RELEASE_BEFORE:-}"
    local sha="${BEAM_RELEASE_SHA:-HEAD}"

    if [[ -z "$before" || "$before" == "0000000000000000000000000000000000000000" ]]; then
      before="$(git rev-parse "${sha}^")"
    fi

    if push_is_release_bump "$before" "$sha"; then
      echo "Push is a Beam release bump; no follow-up release PR needed."
      exit 0
    fi

    if beam_version_changed "$before" "$sha"; then
      echo "Beam version changed in this push; no follow-up release PR needed."
      exit 0
    fi

    local release_base
    release_base="$(public_release_source_rev "$current")"
    if [[ -z "$release_base" ]] || ! git cat-file -e "${release_base}^{commit}" 2>/dev/null; then
      release_base="$before"
    fi

    if ! relevant_beam_change_exists "$release_base" "$sha"; then
      echo "No releasable Beam runtime change detected; skipping release PR."
      exit 0
    fi

    release_type="$(release_type_from_commits "$release_base" "$sha")"
  fi

  if [[ -n "$exact_version" ]]; then
    validate_version "$exact_version"
  else
    release_type="${release_type:-patch}"
    exact_version="$(bump_version "$current" "$release_type")"
  fi

  if ! version_gt "$exact_version" "$current"; then
    echo "Beam release ${exact_version} is not newer than current version ${current}."
    exit 0
  fi

  ensure_current_version_is_public "$current"
  create_or_update_pr "$exact_version"
}

main "$@"
