# Guild Interface

Interface definitions and shared types for the guild service.

## Overview

This package defines the interface contracts and shared data types used by the Guild application server and its clients.

## Features

- API interface definitions
- Shared data structures
- Request/response types
- Error types
- Yield/invest request and response types for Payy swap creation, funding, price reads, and
  aggregate position queries
- Utility functions

## Notes

Bungee request / response types and Bungee-specific error modeling now live in
[`bungee-interface`](../bungee-interface/README.md). `guild-interface` no
longer owns a `bungee` module.
