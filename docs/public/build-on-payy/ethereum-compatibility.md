# Ethereum Compatibility

Payy is fully compatible with Ethereum and the EVM. All existing Ethereum tooling (such as Hardhat, Foundry, etc) will work seamlessly with Payy.

Sequencers use the Reth execution environment, and Provers use a ZK implementation of the EVM execution environment - the Payy zkEVM.&#x20;

{% hint style="success" %}
Payy supports the Fusaka hard fork.
{% endhint %}

### What's the same

* Transaction construction is identical to Ethereum
* Address generation and signing work the same as Ethereum
* All EVM opcodes are supported
* All Ethereum precompiles are supported
* All Ethereum RPC endpoints are supported
* Tracing is supported
* Solidity and EVM bytecode will function identically

### What's been changed/added

Payy offers a superset of features to the Ethereum specification, designed to support privacy and stablecoin use cases.

#### Precompiles

Payy adds protocol-specific precompiles for proof verification, Poseidon hashing, sparse merkle tree maintenance, and millisecond timestamps.

See [Precompiles](../protocol/precompiles.md) for the canonical address list, gas costs, calldata / return formats, and the distinction between public precompiles and internal-only protocol precompiles.

#### Zero gas fees on private transfers

Payy operates zero gas fees for `transfer` fn calls on the native [`PrivacyBridge`](../protocol/privacybridge.md) contract. This ensures that private ERC-20 transfers can be made for free.

#### Consistent gas fee

Gas fees remain consistent, a priority tip can be provided to prioritise transactions - see [Stable Gas](../stablecoins/stable-gas.md).

#### Block times

Payy produces blocks every 300ms. Ethereum `block.timestamp` only supports seconds granularity. To ensure compatibility `block.timestamp` remains unchanged. If sub-second granularity is required, then the `blockTimestampMs` pre-compile can be used.

#### CodeSize <a href="#codesize" id="codesize"></a>

`EXTCODESIZE` is expensive to calculate and verify in ZK, so we store the size on contract creation and use a storage proof instead.


