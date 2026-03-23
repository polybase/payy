# diesel-util

Shared Diesel utilities for Polybase services.

## Overview

This crate hosts helper macros and traits that smooth over Diesel integration
across multiple crates. It currently provides the `derive_pg_text_enum!` macro,
which implements `ToSql`/`FromSql` for enums stored as `TEXT` columns in
PostgreSQL. The macro is gated behind the `diesel` feature so crates can depend
on these helpers without pulling in Diesel unless they need it.

## Features

- `diesel`: Enables the Diesel dependency and exports the
  `derive_pg_text_enum!` macro.

## Usage

```rust
use diesel_util::derive_pg_text_enum;

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "diesel",
    derive(diesel::AsExpression, diesel::FromSqlRow)
)]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
pub enum Status {
    Pending,
    Completed,
}

#[cfg(feature = "diesel")]
derive_pg_text_enum!(Status);
```
