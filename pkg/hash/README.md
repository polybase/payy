# Hash

Noir-compatible Poseidon hash implementation with optimized operations for zero-knowledge circuits.

## Overview

This package provides the primary hashing functionality using a Noir-compatible Poseidon hash implementation via `bn254_blackbox_solver`. This is the current hash implementation that replaces the deprecated `hash-poseidon` package.

## Hash Algorithm

- **Algorithm**: Poseidon hash function
- **Compatibility**: Noir-compatible via `bn254_blackbox_solver`
- **Curve**: BN254/BN256 
- **Operations**: Non-symmetric hash merging for Merkle tree operations

## Features

- **Noir-compatible Poseidon hashing** for zero-knowledge circuit integration
- **Element-based operations** with support for arbitrary-length arrays
- **Byte hashing** with automatic chunking and padding
- **Merkle tree operations** with `hash_merge` for parent node calculation
- **Path-based hashing** for tree traversal operations
- **Test utilities** with operation counting and snapshot testing
- **Non-symmetric hashing** ensuring `hash(a,b) ≠ hash(b,a)`

## Migration Note

This package replaces the deprecated `hash-poseidon` package. Use this implementation for all new development requiring Poseidon hashing functionality.

