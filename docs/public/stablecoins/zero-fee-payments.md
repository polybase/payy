# Zero Fee Payments

Payy natively supports zero fee private ERC-20 transfers. This is implemented by zero rating all interactions with the [PrivacyBridge](../protocol/privacybridge.md) contract, which is the [EVM Layer](../protocol/evm-layer.md) interface to the [Privacy Layer](../protocol/privacy-layer/).

Zero fee transfers are only available for private transfers, giving the following benefits:

* Encourages wider use of the Privacy Layer (i.e. privacy pools), increasing the anonymity set for all protocol users
* Payments on the [Privacy Layer](../protocol/privacy-layer/) are highly optimised, so there is significantly less overhead to the network to zero rate these &#x20;
* Built-in spam protection as a result of ZK [Privacy Layer](../protocol/privacy-layer/) architecture

### Compatibility with existing wallets

Zero fee transfers work natively with any wallet that support adding a custom Ethereum compatible RPC - e.g. MetaMask, TrustWallet, etc.

Wallets calculate gas prices through this typical workflow:

1. **Call** `⁠eth_estimateGas` with the transaction parameters to determine the required gas limit
2. Retrieve current network conditions using `eth_gasPrice` or `⁠eth_feeHistory` to understand base fees and priority fee trends
3. **Calculate total fee** using the formula: ⁠(base fee + priority fee) × gas limit

In order to ensure that all existing wallets support free transfers, Payy Network will return `0` gas for ERC-20 transfers, ensuring that existing wallet infrastructure allows submission without gas.

### Spam protection

ZK proofs are required to submit, which must reference the root of Smirk (Payy's sparse merkle tree), this adds a computational cost for submitting proofs. Further proofs cannot be pre-generated, as they automatically expire every \~60 seconds (as proofs must include a reference to a recent root hash of Smirk).

In addition, the transfer function for ERC-20 will only be `0` fee if the gas used in the transfer function is less than 100,000 gas.

### Why it matters

* For payments use cases, users never have to understand or interact with gas
* Consumers expect parity with their tradfi free transfers
* Free global transfers is highly attractive to businesses who pay significant costs to move money globally

###

&#x20;
