# Wallet Compatibility

Payy is compatible with all existing wallets, like Metamask and Phantom.

### Viewing private balances

When `eth_getBalance` is called, the RPC node requests  the Privacy for the user (as well as the public balance) to determine the complete balance for the user.

### Sending a private transfer

Many wallets have built-in capabilities to send ERC-20 directly from within the wallet.

Sending an ERC-20 transfer `eth_sendRawTransaction()` can also transparently be upgraded.

1. User sends the signed `transfer()` transaction to the RPC
2. RPC constructs ZK proof and submits the transaction to the native `PrivacyBridge` - privacy bridge will verify the zk proof and update the merkle tree
3. RPC will forward the private transaction data to the private storage of the receiving user. If the [`PrivacyVaultRegistry`](../protocol/privacyvaultregistry.md) contract contains a storage destination, then data will be sent there, otherwise it will be sent to the default Payy Network storage provider.



