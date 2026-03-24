# Smirk

Sparse Merkle tree implementation with batch operations and storage management.

## Overview

This package provides a high-performance sparse Merkle tree implementation for the zk-rollup system.

## Features

- Sparse Merkle tree operations
- Batch processing
- Storage management
- Hash caching
- Tree iteration
- Property testing

## Benchmarks

- `cargo bench -p smirk --bench insert_batch_mainnet` downloads a mainnet snapshot/diff from
  `validators.mainnet.payy.network` and benchmarks tree construction plus a single `insert_batch`
  call (download time excluded). Update `TARGET_HEIGHT` in
  `pkg/smirk/benches/insert_batch_mainnet.rs` if needed.
