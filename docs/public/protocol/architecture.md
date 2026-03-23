# Architecture

Payy Network is an L2 ZK rollup on Ethereum with the following architecture:

<figure><img src="../.gitbook/assets/L2 Rollup-L2 Rollup.drawio.png" alt=""><figcaption></figcaption></figure>

### Roles

Each node on the Payy Network protocol can perform one or more of the following distinct roles:

* **Sequencer** - add pending unproven blocks to the L1, guaranteeing the order of transactions
* **Rollup Prover** - prove that an existing sequenced block is valid or invalid
* **Client** - proves valid UTXO transactions
* **Encrypted Registries (Optional)** - component to store transactions so that transactions can be made with offline users
* **Ethereum** - provides security and data availability for the network
