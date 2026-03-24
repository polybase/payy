# database

Holds Diesel schema definitions and migration utilities for the primary
PostgreSQL database. Application data models and query helpers now live in the
[`data`](../data/README.md) crate; depend on that crate for structs and business
logic and keep this crate focused on schema management.

## Diesel

We use diesel as our postgres client. You can install the cargo diesel CLI using:

```bash
brew install postgresql
cargo install diesel_cli --no-default-features --features postgres
```

### Update database schema

Update `src/schema.rs` and then run:

```
diesel migration generate --diff-schema <change_name>
```

You can then run the following to apply the changes:

```
diesel migration run
```

### Tests

The integration test at `tests/replit_permissions.rs` provisions a disposable PostgreSQL instance via Docker to
validate column-level privileges. The test automatically skips when the `docker` binary is not available on the
`PATH`, so ensure Docker is installed locally if you want to exercise it end-to-end.
