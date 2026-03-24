# Provers

Payy Network provers process L2 sequenced blocks, proving in ZK (zero knowledge) that the EVM Layer and Privacy Layer transactions are valid. The EVM Layer and Privacy Layer proofs are constructed separately and then combined in the final proof.

An entire EVM block is aggregated into a single proof, and further those blocks are aggregated into further blocks.

Provers prove that the L2 blocks are valid. If the block is valid, the proof and root merkle hash of the rollup is updated in Ethereum. The Ethereum smart contract needs only the proof and the new root hash from the prover to the verify and ensure the security of the network.
