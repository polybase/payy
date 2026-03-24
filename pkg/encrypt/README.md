# Encrypt

Cryptographic operations using XSalsa20-Poly1305 and X25519 for secure data encryption.

## Overview

This package provides authenticated encryption functionality using modern cryptographic primitives. It implements both symmetric and asymmetric encryption schemes with a versioned data format for storage and transmission.

## Encryption Algorithms

### Symmetric Encryption
- **Algorithm**: XSalsa20-Poly1305
- **Authentication**: Built-in AEAD (Authenticated Encryption with Associated Data)
- **Nonce size**: 24 bytes (automatically generated)
- **Key size**: 32 bytes

### Asymmetric Encryption  
- **Key Exchange**: X25519 (Curve25519 Diffie-Hellman)
- **Encryption**: XSalsa20-Poly1305
- **Ephemeral keys**: Generated per encryption for forward secrecy
- **Authentication**: Built-in AEAD

## Features

- **Symmetric encryption/decryption** with XSalsa20-Poly1305
- **Asymmetric encryption/decryption** with X25519-XSalsa20-Poly1305
- **Automatic nonce generation** for each encryption operation
- **Versioned data format** for backward compatibility
- **Key generation utilities** for both symmetric and asymmetric keys
- **Compact serialization** for encrypted data storage
- **Error handling** for all cryptographic operations

