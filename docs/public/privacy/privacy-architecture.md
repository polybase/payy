# Privacy Architecture

Payy uses ZK (zero knowledge) cryptography to enable privacy whilst maintaining EVM compatibility.&#x20;

There are three core privacy enabling flows on Payy:

1. [**Private Transfers**](../build-on-payy/private-transfers.md) - private ERC-20 and native token transfers, transparently upgraded
2. [**Stealth Transactions**](stealth-transactions.md) - private transactions on the EVM Layer using newly minted addresses for each transaction
3. [**Native ZK**](native-zk.md) - a ZK verification precompile reducing cost for custom privacy applications

### Architecture

The Payy Privacy architecture is composed of the following components:

* [**Privacy Layer**](../protocol/privacy-layer/) - a high performance privacy rollup, creating a native privacy pool for every ERC-20 token.
* [**PrivacyBridge**](../protocol/privacybridge.md) - a predeployed bridge to the Privacy Layer accessible from the [EVM Layer](../protocol/evm-layer.md).
* [**Privacy Vault**](../protocol/privacy-vault.md) - stores private data, converts signed Ethereum transactions into private transactions using ZK proofs.

<figure><img src="../.gitbook/assets/privacy arch.png" alt=""><figcaption></figcaption></figure>

### Levels of Privacy in Payy

* [**Private Transfers**](editor.md) - occur entirely within the [Privacy Layer](../protocol/privacy-layer/). This ensures zero overhead on the [EVM Layer](../protocol/evm-layer.md), maximal privacy and massive throughput. Payments are fully private. That means the sender, receiver, amount and asset are all hidden. Total privacy.
* [**Stealth Transactions**](stealth-transactions.md) - transaction occurs on the EVM Layer, but with funds originating from the Privacy Layer, it's impossible to determine the true sender or receiver.
* [**Native ZK**](native-zk.md) - dependent on the privacy application

<figure><img src="../.gitbook/assets/Levels of privacy (1).png" alt=""><figcaption></figcaption></figure>

### Encrypted Lineage

Payy uses nullifiers to ensure that lineage of transactions cannot be tracked by external observers. This is critical to ensure that transactions moving in and out of the EVM Layer cannot be trivially exposed. To prevent actors using the privacy pools maliciously to move in toxic assets, the encrypted lineage can be decrypted in exceptional cases using an on-chain proposal.

