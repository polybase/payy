# Parse Link

Link parsing and validation utilities.

## Overview

This package provides utilities for parsing and validating various types of links used throughout the zk-rollup system. It now hosts the canonical implementations for:

- Activity link payload encoding/decoding (moved from `zk-primitives`)
- Payment link parsing logic (migrated from the TypeScript `parseLink.ts` helper)

## Features

- Decode and encode `NoteURLPayload` structures used by `s#<payload>` links
- Default omitted note kinds to Polygon bridged USDC while preserving non-USDC kinds in v2 links
- Parse request, send, and invite URLs into strongly typed Rust structs
- Extract common metadata (invite codes, memos) from parsed links
- Shareable parsing API for the React Native bridge and backend services

## Key APIs

- `parse_link::parse_url(&str) -> Result<Link, ParseError>` parses any supported Payy link with detailed errors.
- `parse_link::parse_send_url(&str) -> Option<SendLink>` focuses on send links.
- `parse_link::NoteURLPayload` exposes helpers to `encode_activity_url_payload`, `decode_activity_url_payload`, `address`, `psi`, and `commitment`.

`NoteURLPayload` keeps version `2` for current send links. Old links continue to decode as Polygon
bridged USDC when `note_kind` is omitted. New non-USDC links append a note-kind trailer under the
same version, while new USDC links still encode in the legacy-compatible format.

See `src/parse.rs` and `src/note_url.rs` for more details and unit tests covering the supported formats.
