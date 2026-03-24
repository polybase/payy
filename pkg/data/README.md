# data

The `data` crate hosts data models and database access helpers that are shared
across services. Structs, enums, and supporting logic compile without Diesel so
callers that only need type definitions can depend on the crate without pulling
in database infrastructure. Enabling the `diesel` feature activates Diesel-based
queries and derives, including access to the `database::schema` module for table
mappings. Optional `stripe` and `ts-rs` features mirror the previous
functionality that lived in the `database` crate.

## Features

- `diesel`: Enables Diesel derives and async query helpers. Pulls in
  `diesel-async` and forwards the feature to downstream workspace crates that
  require Diesel integration.
- `stripe`: Exposes Stripe conversions for payment types.
- `ts-rs`: Enables TypeScript bindings for exported structs.

Consumers that also need table definitions should continue importing
`database::schema` while using `data` for the associated models and helpers.
