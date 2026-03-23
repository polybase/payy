# Privacy Rollup

The privacy rollup represents the entire state of the Privacy Layer, where each piece of state is referred to as a [Note](utxo.md#notes), is represented by a commitment hash \[Merkle87]. Hashes result in data loss, so the original messages can provably never be reconstructed, ensuring privacy of the protocol. Each note represents a balance of an ERC-20 token on Payy, which can thus be used to create a privacy pool for every ERC-20 contract.

Each of the hash record states are stored in a merkle tree (our implementation is [Smirk](https://github.com/polybase/payy/tree/main/pkg/smirk)), enabling a single root hash to represent the entire state of the network. No underlying data is stored in the rollup, and therefore any required shared data must be stored in the users [Privacy Vault](../privacy-vault.md). In addition to privacy, an additional advantage of storing only hashes in the rollup is that the rollup on disk data size can be somewhat constrained and deterministic. This is a result of all hashes being a consistent size regardless of the size of the underlying data. This reduction in the disk size requirements for clients, improves the number of clients able to join the network, thereby helping to improve decentralisation.

The merkle tree must enable the following operations:

* Prove inclusion - prove a hash exists in the tree
* Prove non-inclusion - prove a hash does not exist in the tree
* Insert - insert a new hash into the tree if it does not already exist

To satisfy these properties, Payy uses a sparse merkle tree \[DPP16]. A sparse merkle tree has a defined position for every possible value that can be inserted into the tree. Using a 256 bit hash for the tree, the maximum depth is $$2^{256}$$. However, given that all operations require traversal of the entire tree, it is inefficient to use a tree of such depth. Therefore, a tree depth of  $$2^{160}$$ (equivalent entropy as Ethereum addresses) can be used, which provides sufficient entropy whilst minimising the operational overhead.

In addition, further enhancements can be made to Smirk allowing for sharding the tree at different levels.

### Merkle Proofs

As every hash has a unique position in a sparse merkle tree, we can derive its position by decomposing each bit of the hash and traversing the tree based on whether the bit is 0 or 1. For 0, the tree is traversed to the left child, and for 1 the tree is traversed to the right child.

<figure><img src="../../.gitbook/assets/image (12).png" alt=""><figcaption><p><em>Fig 1: 4 bit tree, demonstrating bit decomposition and insertion</em></p></figcaption></figure>

For an inclusion or non-inclusion proof, we can simply check that each of the provided siblings combined with the the computed child results in the root hash. For non-inclusion we are proving that the leaf node is a null value, which does not need to be passed to the proof, as it is a static value.

<figure><img src="../../.gitbook/assets/image (13).png" alt=""><figcaption><p><em>Fig 2: non-inclusion and inclusion proofs</em></p></figcaption></figure>

Insertion proofs are a combination of a non-inclusion proof (existing position must be null) and inclusion proof (proving the new root based on the inserted hash).

### References

\[DPP16] Dahlberg, R., Pulls, T., and Peeters, R. 2016. _Efficient Sparse Merkle Trees_. In: Brumley, B., Röning, J. (eds) Secure IT Systems. NordSec 2016. Lecture Notes in Computer Science (vol 10014). Springer, Cham. [https://doi.org/10.1007/978-3-319-47560-8\_13](https://doi.org/10.1007/978-3-319-47560-8_13)

\[Merkle87] Merkle, R. 1987. _A digital signature based on a conventional encryption function." Advances in Cryptology_ — CRYPTO ’87. Springer.

