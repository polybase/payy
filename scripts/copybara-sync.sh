#!/usr/bin/env bash
set -euo pipefail

COPYBARA_VERSION="${COPYBARA_VERSION:-v20260504}"
COPYBARA_JAR_DIR="${COPYBARA_JAR_DIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/copybara}"
COPYBARA_JAR="${COPYBARA_JAR:-${COPYBARA_JAR_DIR}/copybara_deploy-${COPYBARA_VERSION}.jar}"
COPYBARA_JAR_URL="${COPYBARA_JAR_URL:-https://github.com/google/copybara/releases/download/${COPYBARA_VERSION}/copybara_deploy.jar}"
COPYBARA_SOURCE_REF="${COPYBARA_SOURCE_REF:-${GITHUB_SHA:-HEAD}}"
COPYBARA_ORIGIN_VERSION="${COPYBARA_ORIGIN_VERSION:-${GITHUB_SHA:-$COPYBARA_SOURCE_REF}}"
COPYBARA_AUTHOR="${COPYBARA_AUTHOR:-Copybara Bot <copybara@polybase.xyz>}"
COPYBARA_MESSAGE="${COPYBARA_MESSAGE:-Sync ${COPYBARA_ORIGIN_VERSION}}"
COPYBARA_INIT_HISTORY="${COPYBARA_INIT_HISTORY:-0}"
COPYBARA_DESTINATION_LOCK_URL="${COPYBARA_DESTINATION_LOCK_URL:-}"

fail() {
  printf 'copybara sync error: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

need_file() {
  [[ -f "$1" ]] || fail "missing required file: $1"
}

need_dir() {
  [[ -d "$1" ]] || fail "missing required directory: $1"
}

log() {
  printf '%s\n' "$*"
}

workspace_root="$(git rev-parse --show-toplevel)"
snapshot_dir_host="${workspace_root}/.copybara-sync-snapshot"
copybara_config="${workspace_root}/copy.bara.sky"

need_cmd cargo
need_cmd curl
need_cmd git
need_cmd java

: "${COPYBARA_AUTH_DIR:?COPYBARA_AUTH_DIR is required}"

need_dir "$COPYBARA_AUTH_DIR"
need_file "$COPYBARA_AUTH_DIR/.ssh/id_rsa"
need_file "$COPYBARA_AUTH_DIR/.ssh/known_hosts"
need_file "$COPYBARA_AUTH_DIR/.gitconfig"
need_file "$COPYBARA_AUTH_DIR/.git-credentials"

cleanup() {
  rm -rf "$snapshot_dir_host"
}

prepare_lockfile() {
  local seeded_lockfile
  seeded_lockfile=0

  if [[ -n "$COPYBARA_DESTINATION_LOCK_URL" ]]; then
    log "seeding Cargo.lock from ${COPYBARA_DESTINATION_LOCK_URL}"
    if curl -fsSL "$COPYBARA_DESTINATION_LOCK_URL" -o "$snapshot_dir_host/Cargo.lock"; then
      if cargo metadata --format-version 1 --manifest-path "$snapshot_dir_host/Cargo.toml" >/dev/null; then
        seeded_lockfile=1
      else
        rm -f "$snapshot_dir_host/Cargo.lock"
      fi
    fi
  fi

  if [[ "$seeded_lockfile" == "0" ]]; then
    log "generating a fresh Cargo.lock"
    cargo generate-lockfile --manifest-path "$snapshot_dir_host/Cargo.toml"
  fi
}

ensure_copybara_jar() {
  if [[ -f "$COPYBARA_JAR" ]]; then
    return
  fi

  local tmp_jar
  tmp_jar="${COPYBARA_JAR}.tmp"

  mkdir -p "$(dirname "$COPYBARA_JAR")"
  log "downloading Copybara ${COPYBARA_VERSION} from ${COPYBARA_JAR_URL}"
  curl -fsSL "$COPYBARA_JAR_URL" -o "$tmp_jar"
  mv "$tmp_jar" "$COPYBARA_JAR"
}

copybara() {
  ensure_copybara_jar
  # Copybara's JVM and Git subprocesses resolve auth files from different sources.
  HOME="$COPYBARA_AUTH_DIR" \
    GIT_CONFIG_GLOBAL="$COPYBARA_AUTH_DIR/.gitconfig" \
    GIT_SSH_COMMAND="ssh -i $COPYBARA_AUTH_DIR/.ssh/id_rsa -o IdentitiesOnly=yes -o UserKnownHostsFile=$COPYBARA_AUTH_DIR/.ssh/known_hosts" \
    GIT_TERMINAL_PROMPT=0 \
    java -Duser.home="$COPYBARA_AUTH_DIR" -jar "$COPYBARA_JAR" migrate "$copybara_config" "$@"
}

copybara_allow_noop() {
  local status
  status=0

  copybara "$@" || status=$?
  if [[ "$status" == "4" ]]; then
    log "Copybara reported a no-op migration; treating it as successful."
    return 0
  fi

  return "$status"
}

trap cleanup EXIT

rm -rf "$snapshot_dir_host"
mkdir -p "$snapshot_dir_host"

copybara_allow_noop snapshot "$COPYBARA_SOURCE_REF" --ignore-noop --folder-dir "$snapshot_dir_host"

prepare_lockfile

push_args=(
  push_generated
  "$snapshot_dir_host"
  --ignore-noop
  --folder-origin-version "$COPYBARA_ORIGIN_VERSION"
  --force-author "$COPYBARA_AUTHOR"
  --force-message "$COPYBARA_MESSAGE"
  --github-destination-pr-branch "copybara-sync"
)

if [[ "$COPYBARA_INIT_HISTORY" == "1" ]]; then
  push_args+=(--init-history)
fi

copybara_allow_noop "${push_args[@]}"
