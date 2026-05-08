# Wallet Compatibility

Payy is compatible with existing wallets such as Metamask and Phantom, but private transfers require one extra piece of metadata beyond a normal EVM transfer: the recipient's Payy private address.

## Viewing Private Balances

When `eth_getBalance` is called, the RPC can combine public balance data with the user's private note data to present a full wallet balance view.

## Sending a Private Transfer

Wallets and RPCs can still wrap the privacy flow behind familiar send UX, but the standard direct-send protocol now works like this:

1. The user signs a transfer request and supplies the recipient's Payy private address off-chain.
2. RPC / wallet service constructs a `transfer_send` proof and submits it to the native [`PrivacyBridge`](../protocol/privacybridge.md).
3. `PrivacyBridge` verifies the proof, updates the Merkle tree, stores sender / recipient encrypted note data, and emits `ExternalTransfer(prefix6, txHash)` for recipient discovery.
4. The recipient wallet watches matching prefix logs, decrypts the incoming note, and later claims it with `transfer_claim`.

The recipient private address is:

- a hex-encoded compressed 32-byte Grumpkin public key
- shared off-chain, similarly to how applications share destination payment details
- distinct from the recipient's EVM address
- defined canonically in the [privacy-layer private-address spec](../protocol/privacy-layer/private-address.md)

If you only have an EVM address and no Payy private address, the RPC cannot construct the standard direct private send flow by itself.
