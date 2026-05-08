# Bungee Interface

Shared interface crate for the Bungee stack.

## Overview

This crate is a leaf dependency that owns both of the public contracts used by
the Bungee integration:

- `bungee_interface::client` contains the domain-facing `BungeeClient` trait,
  public request / response types, and the wire-stable client error enum used by
  Guild and wallet clients.
- `bungee_interface::api` contains the upstream REST API calque, including the
  `BungeeApi` trait, transport error surface, response wrapper, and per-endpoint
  DTOs.

## Stack Layout

The shipped Bungee integration is split across three crates:

- `bungee-interface` is the leaf crate that owns the shared domain contract and
  upstream API calque.
- `bungee-client-http` depends only on `bungee-interface` and implements
  `bungee_interface::api::BungeeApi` with reqwest transport.
- `bungee` depends only on `bungee-interface` and implements
  `bungee_interface::client::BungeeClient` with quote-selection and response
  normalization logic.
- `guild` is the composition site: it constructs `BungeeHttpClient`, wraps it
  in `BungeeQuoter`, and stores the resulting `Arc<dyn BungeeClient>` in server
  state.
- Wallet-facing crates depend only on `bungee-interface` for request/response
  types and error decoding; they do not instantiate the HTTP or domain-service
  layers directly.

## Wire Stability

The `client` module is the durable Guild and wallet-facing contract. Snapshot
tests in this crate cover request / response encoding and the public error
payloads so schema drift is caught before it reaches downstream clients that
roll out independently.

## Testing

The crate keeps wire-compat snapshot coverage for the public domain types and
error payloads so clients can detect accidental schema drift.
