# Predeployed Contracts

### System Contracts <a href="#system-contracts" id="system-contracts"></a>

Native core protocol contracts that provide enhanced capabilities to Payy:

| Contract                                                        | Address                                      | Description                                              |
| --------------------------------------------------------------- | -------------------------------------------- | -------------------------------------------------------- |
| **PUSD**                                                        | `0x0200000000000000000000000000000000000000` | Native ERC-20 view over Payy balances                    |
| [**PrivacyBridge**](../protocol/privacybridge.md)               | `0x3100000000000000000000000000000000000000` | Bridge funds to and from the native ERC-20 privacy pools |
| [**PrivacyVaultRegistry**](../protocol/privacyvaultregistry.md) | `TBC`                                        | Stores the registry of a given address                   |
| **Poseidon**                                                    | `0x3300000000000000000000000000000000000000` | Solidity wrapper around the public Poseidon precompile   |
| [**Rollup**](../protocol/rollup.md)                             | `0x3200000000000000000000000000000000000000` | Sparse merkle rollup tree interface                      |
| **BlockTimestampMs**                                            | `0x3400000000000000000000000000000000000000` | Solidity wrapper around the millisecond timestamp precompile |
| [**TransactionBridge**](../protocol/transactionbridge.md)       | `0x3000000000000000000000000000000000000000` | Handle fee payments and conversions                      |

Custom Payy precompiles are documented separately in [Precompiles](../protocol/precompiles.md).

### Standard Utilities <a href="#standard-utilities" id="standard-utilities"></a>

Popular Ethereum utility contracts:

| Contract                                                                 | Address | Description                             |
| ------------------------------------------------------------------------ | ------- | --------------------------------------- |
| [**Multicall3**](https://www.multicall3.com/)                            | `TBC`   | Batch multiple calls in one transaction |
| [**Permit2**](https://docs.uniswap.org/contracts/permit2/overview)       | `TBC`   | Token approvals and transfers           |
| [**CreateX**](https://github.com/pcaversaccio/createx)                   | `TBC`   | Deterministic contract deployment       |
| [**Safe Deployer**](https://github.com/safe-fndn/safe-singleton-factory) | `TBC`   | Safe deployer contract                  |
| [**CREATE2 Factory (Arachnid)**](https://github.com/arachnid)            | `TBC`   | Deterministic deployment proxy contract |
