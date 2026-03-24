#!/usr/bin/env bash

set -euo pipefail

PARAMS_URL="https://storage.googleapis.com/payy-public-fixtures/g1.max.dat"
PARAMS_SHA256="3ef417367184adaf0dcdebfccc440b5bb72f9a228b278f17d1411b1340f2daa5"
PARAMS_CACHE_DIR="${POLYBASE_PARAMS_DIR:-$HOME/.polybase/fixtures/params}"
CACHE_TARGET="$PARAMS_CACHE_DIR/g1.max.dat"
BB_CRS_DIR="${BB_CRS_DIR:-$HOME/.bb-crs}"
BB_CRS_TARGET="$BB_CRS_DIR/bn254_g1.dat"

mkdir -p "$PARAMS_CACHE_DIR"
mkdir -p "$BB_CRS_DIR"

sha256_file() {
  local target="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$target" | cut -d' ' -f1
    return
  fi

  shasum -a 256 "$target" | cut -d' ' -f1
}

verify_sha256() {
  local target="$1"
  local actual

  actual=$(sha256_file "$target")

  if [[ "$actual" != "$PARAMS_SHA256" ]]; then
    echo "Checksum mismatch for $target (expected $PARAMS_SHA256, got $actual)" >&2
    return 1
  fi
}

if [[ -f "$CACHE_TARGET" ]] && ! verify_sha256 "$CACHE_TARGET"; then
  rm -f "$CACHE_TARGET"
  rm -f "$BB_CRS_TARGET"
fi

if [[ ! -f "$CACHE_TARGET" ]]; then
  TMP_FILE=$(mktemp)
  trap 'rm -f "$TMP_FILE"' EXIT

  echo "Downloading params from $PARAMS_URL" >&2
  curl -fsSL "$PARAMS_URL" -o "$TMP_FILE"

  if ! verify_sha256 "$TMP_FILE"; then
    exit 1
  fi

  mv "$TMP_FILE" "$CACHE_TARGET"
fi

echo "Params available at $CACHE_TARGET"

if [[ -L "$BB_CRS_TARGET" ]]; then
  rm -f "$BB_CRS_TARGET"
fi

if [[ ! -e "$BB_CRS_TARGET" ]]; then
  ln -s "$CACHE_TARGET" "$BB_CRS_TARGET"
  echo "Linked $CACHE_TARGET to $BB_CRS_TARGET"
fi
