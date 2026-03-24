# Burn Substitutor

Tool for handling burn operations and substitutions in the zk-rollup system.

## Overview

This package provides functionality for processing burn operations, allowing users to convert rollup assets to external blockchain assets.

## Features

- Burn operation processing
- Asset substitution logic
- Command-line interface
- Integration with rollup contracts

## Configuration

- `CHAIN_ID`: EVM chain id for the rollup contract (defaults to `137`).
- `CHAIN_ID` is validated against the RPC network on startup and the process exits on mismatch.
- `EXCLUDED_BURN_ADDRESSES`: Comma-separated list of burn recipient addresses that should never be substituted.
