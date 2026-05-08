# Price Cache Interface

Shared read-side interface for token prices.

## Overview

This crate defines the common types used by both server and mobile price-cache
consumers:

- `TokenIdentifier` for symbol-based or network/address-based lookup
- `TokenPrice` with the quoted value, currency, and freshness timestamp
- `PriceCache`, an async trait for reading token prices

Concrete implementations live in sibling crates such as `price-cache-pg` and
`price-cache-http`.
