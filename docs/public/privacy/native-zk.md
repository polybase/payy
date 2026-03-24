# Native ZK

Payy natively supports ZK (zero knowledge) proofs, allowing application developers to build privacy preserving applications directly into Payy without sacrificing performance or incurring excessive cost.

Verifying proofs on existing networks is expensive. Proof verification on existing unoptimised chains can take in excess of 5M gas (\~10% of the available blockspace). This makes it impractical for most privacy applications to rollup individual transactions, which means some form of rollup must be used to enable all privacy applications.

Application developers can use the natively supported ZK proofs directly in their EVM smart contract code to validate trustlessly transactions that occur offchain.

{% include "../../../.gitbook/includes/zk-framework.md" %}

### Use cases

Verifying proofs off chain allows developers to build applications like:

* Offchain orderbooks like hyperliquid
* Dark pools
* Private RWA contracts

