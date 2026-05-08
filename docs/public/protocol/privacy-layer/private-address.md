# Private Address

A Payy private address is the public wire format used to share a recipient's privacy key material off-chain for direct private transfers.

It is not:

- an EVM address
- a private key
- the note `owner` field stored inside notes

Instead, it is the transport encoding of a Grumpkin Schnorr public key. Wallets and RPCs decode it to an affine Grumpkin point `(x, y)`, derive `owner = Poseidon(x, y)`, and use that owner hash inside the privacy protocol.

## Canonical Encoding

The canonical binary encoding is exactly 32 bytes:

1. Start from a non-infinity affine Grumpkin public key point `(x, y)`.
2. Serialize `x` as a 32-byte big-endian field element.
3. Set the most significant bit of byte `0` to `y mod 2`.
4. Require the second-most significant bit of byte `0` to remain `0`.
5. Use the remaining 254 bits for `x`.

Bit layout of byte `0`:

- bit `7`: `y` parity (`0` for even, `1` for odd)
- bit `6`: reserved, must be `0`
- bits `5..0`: high bits of `x`

The canonical string form is:

- `0x` followed by 64 hex characters
- wallets should emit lowercase hex
- decoders may accept either hex case, but must reject wrong length or non-hex input

## Decoding Rules

To decode a private address:

1. Parse the 32 bytes.
2. Read `y_parity` from bit `7` of byte `0`.
3. Reject the input if bit `6` of byte `0` is set.
4. Clear bit `7` and interpret the remaining 32 bytes as big-endian `x`.
5. Reject the input if `x` is not a canonical Grumpkin base-field element.
6. Recover `y` from the Grumpkin curve equation and choose the root whose parity matches `y_parity`.
7. Reject the input if no valid affine point exists or if the decoded point is the point at infinity.

Wallets and RPCs must reject non-canonical encodings rather than silently normalizing them.

## Protocol Use

After decoding, wallets and RPCs must:

- derive note ownership as `owner = Poseidon(x, y)`
- derive `receive_prefix` from that owner hash
- use the same decoded point for recipient-side encryption and Schnorr verification

In the standard direct-send flow:

- the sender obtains the recipient private address off-chain
- `transfer_send` derives the recipient-owned incoming note from the decoded point
- the bridge emits `ExternalTransfer(prefix6, txHash)`, where `prefix6` comes from the derived owner hash rather than from the raw 32-byte private-address encoding

## Library Compatibility

Payy's current proving helpers use barretenberg / `bb.js` Schnorr APIs, and those APIs expose Grumpkin public keys as affine `x` / `y` coordinates rather than this 32-byte transport format. Third-party wallets and RPCs should therefore treat the private address as the stable wire format and convert it to affine coordinates before calling those APIs.
