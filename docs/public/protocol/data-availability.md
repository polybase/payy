# Data Availability

The data availability on Ethereum serves two purposes:

1. Availability of transaction data (UTXO proofs and EVM transactions), so all network participants can determine for themselves the full state of Smirk (our sparse merkle tree). Without this, colluding provers could submit a rollup to change the root hash on Ethereum without providing the txn data to other nodes, making it impossible for other nodes to know the current state of the merkle tree is (only that it is valid). Without the current state of the merkle tree, no other nodes would be able to create a proof, as the merkle tree is used in proof generation.
2. A commitment to a set of transactions and their order (so final state can be known before it is proved). This allows for optimistic protocols to pre-fill bridging activity.

The batched transactions are ordered and stored on Ethereum, providing a definitive state of the network, even if the rollup root state hash has not yet been updated to reflect these transactions. Once the transactions have been finalised on Ethereum, the order of the transactions cannot change, and therefore the future state is predictable and fixed.
