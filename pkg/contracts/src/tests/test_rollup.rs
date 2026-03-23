#[cfg(test)]
pub mod rollup {
    use element::Element;
    use rand::{RngCore, rngs::OsRng};
    use smirk::Tree;
    use zk_primitives::{InputNote, MerklePath, Note, get_address_for_private_key};

    /// Depth expected by all current circuits & tests (see
    /// `verify_transfers`, `burn_to_*`, etc.).
    pub const MERKLE_TREE_DEPTH: usize = 161;

    /// A throw‑away roll‑up model backed by a `smirk::Tree`.
    #[derive(Debug, Clone, Default)]
    pub struct Rollup {
        tree: Tree<MERKLE_TREE_DEPTH, ()>,
    }

    impl Rollup {
        /// Create an empty tree whose root equals the
        /// `empty_merkle_tree_root_hash.txt` reference.
        pub fn new() -> Self {
            Self::default()
        }

        /// Convenience for the tests – builds a brand‑new wallet with a
        /// random private key.
        pub fn new_wallet(&self) -> Wallet {
            Wallet::random()
        }

        /// Insert an **_unspent_** note directly into the tree (used by
        /// the tests to create pre‑existing UTXOs without generating a
        /// proof first).
        #[expect(unused)]
        pub fn unverified_add_unspent_note(
            &mut self,
            wallet: &Wallet,
            amount: u64,
            kind: Element,
        ) -> WalletNote {
            let note = wallet.new_note(amount, kind);
            self.tree.insert(note.commitment(), ()).unwrap();
            WalletNote::new(*wallet, note)
        }

        /// Return the current root as a `element::Element`.
        #[expect(unused)]
        pub fn root_hash(&self) -> Element {
            self.tree.root_hash()
        }

        // ----------------------------------------------------------------
        // Helpers – currently *not* used by the compiled tests but kept
        // around because they were relied upon before, and they’re handy
        // for local experimentation.
        // ----------------------------------------------------------------

        /// Convert a `WalletNote` into an `InputNote` by attaching its
        /// Merkle path.
        #[expect(unused)]
        pub fn to_input_note(&self, wn: &WalletNote) -> InputNote {
            InputNote {
                note: wn.note(),
                secret_key: wn.wallet.pk,
            }
        }

        /// Return the Merkle‑proof for any element currently in the tree.
        #[expect(unused)]
        pub fn merkle_path(&self, el: Element) -> MerklePath<MERKLE_TREE_DEPTH> {
            let path = self.tree.path_for(el);
            // `smirk` returns siblings deepest‑first, which matches the
            // ordering expected by the circuits.
            MerklePath::new(path.siblings_deepest_first().to_vec())
        }
    }

    // =====================================================================
    // Wallet & helpers
    // =====================================================================

    #[derive(Clone, Copy, Debug)]
    pub struct Wallet {
        /// *Private* key in the zk‑Primitive sense – **NOT** an ECDSA key!
        pub pk: Element,
    }

    impl Wallet {
        /// Create a wallet from an explicit private key.
        #[expect(unused)]
        pub fn new(pk: Element) -> Self {
            Self { pk }
        }

        /// Create a wallet with a random 256‑bit private key.
        pub fn random() -> Self {
            let mut bytes = [0u8; 32];
            OsRng.fill_bytes(&mut bytes);
            Self {
                pk: Element::from_be_bytes(bytes),
            }
        }

        /// Derive the *address* (Poseidon‑hashed) that the circuits use.
        pub fn address(&self) -> Element {
            get_address_for_private_key(self.pk)
        }

        pub fn new_note(&self, amount: u64, contract: Element) -> Note {
            Note {
                kind: Element::new(2),
                value: Element::new(amount),
                address: self.address(),
                contract,
                psi: Element::new(0),
            }
        }
    }

    // =====================================================================
    // A note bundled with the wallet that created / owns it
    // =====================================================================

    #[derive(Clone, Debug)]
    pub struct WalletNote {
        note: Note,
        pub wallet: Wallet,
    }

    impl WalletNote {
        fn new(wallet: Wallet, note: Note) -> Self {
            Self { note, wallet }
        }

        /// Raw commitment (32‑byte field element).
        #[expect(unused)]
        pub fn commitment(&self) -> Element {
            self.note.commitment()
        }

        /// Borrow the underlying `Note`.
        pub fn note(&self) -> Note {
            self.note.clone()
        }
    }
}
