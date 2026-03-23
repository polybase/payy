# Nullifiers

As per the [UTXO](utxo.md) model, a note can be used only once. As such, the protocol must keep track of which notes have been used or spent, and which have not. Naively, this could be performed by marking or removing used notes in the merkle tree, however this would reveal to public observers when a note has been spent, reducing privacy by allowing transactions to be linked together.

Instead, to enhance privacy, we use a deterministic nullifier record that for each spend must be inserted into the tree, and during spend of the original input not, must be proven not to exist in the tree via a non-inclusion proof. The presence of the nullifier represents the record being spent. The nullifier is constructed in such a way that it is impossible for an external party to determine which nullifier matches a spent record (or a record to be spent).

The nullifier is calculated as follows:

$$
N = \text{Poseidon}(nk, \psi, cm)
$$

Where:

* $$nk$$ is the Nullifier Key, a unique secret associated with each user (this is a auth commitment to the auth)
* $$\psi$$ (psi) sender controlled randomness, additional entropy provided by Blake2b \[Aumasson et al. 2013] hash. Blake2b provides additional entropy and privacy security.
* $$cm$$ is the note commitment, which is a Poseidon commitment to the note.
