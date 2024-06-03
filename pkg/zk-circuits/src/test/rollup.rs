use crate::constants::{MERKLE_TREE_DEPTH, UTXO_AGG_LEAVES, UTXO_AGG_NUMBER};
use crate::data::{Batch, InputNote, Insert, MerklePath, Note, Utxo, UtxoKind};
use crate::CircuitKind;
use crate::{
    aggregate_utxo::AggregateUtxo, chips::poseidon::poseidon_hash, util::insecure_random_element,
};
use halo2_base::halo2_proofs::halo2curves::bn256::Fr;
use smirk::{Path, Tree};
use zk_primitives::Element;

#[derive(Debug, Clone, Default)]
pub struct Rollup {
    pub tree: Tree<MERKLE_TREE_DEPTH, ()>,
}

impl Rollup {
    pub fn new() -> Self {
        let tree = smirk::Tree::new();
        Self { tree }
    }

    pub fn new_wallet(&self) -> Wallet {
        Wallet::new()
    }

    /// Adds an unspent note to the tree without verifying any proofs (test only to simulate
    /// existing notes in a tree)
    pub fn unverified_add_unspent_note(&mut self, wallet: &Wallet, amount: u64) -> WalletNote {
        let note = wallet.new_wallet_note(amount);
        self.tree.insert(note.commitment(), ()).unwrap();
        note
    }

    /// Convert WalletNote into an InputNote to be used in a UTXO txn
    pub fn to_input_note(&self, note: &WalletNote) -> InputNote<MERKLE_TREE_DEPTH> {
        note.to_input_note(self.tree.path_for(note.commitment()))
    }

    pub fn merkle_path(&self, el: Element) -> MerklePath<MERKLE_TREE_DEPTH> {
        merkle_path(&self.tree, el)
    }

    pub fn root_hash(&self) -> Element {
        self.tree.root_hash()
    }

    pub fn transfer(&self, input_note: WalletNote, output_note: Note) -> Utxo<MERKLE_TREE_DEPTH> {
        let input_notes = [self.to_input_note(&input_note), InputNote::padding_note()];
        let output_notes = [output_note, Note::padding_note()];

        Utxo::new(
            input_notes,
            output_notes,
            self.root_hash(),
            UtxoKind::Transfer,
        )
    }

    // pub fn mint(&self, output_note: Note) -> Utxo {
    //     Utxo::new_mint(output_note, self.root_hash())
    // }

    // pub fn burn(&self, input_note: WalletNote) -> Utxo {
    //     Utxo::new_burn(self.to_input_note(input_note), self.root_hash())
    // }

    pub fn batch_inserts_for_utxos(
        &mut self,
        utxos: &[Utxo<MERKLE_TREE_DEPTH>; UTXO_AGG_NUMBER],
    ) -> Batch<UTXO_AGG_LEAVES, MERKLE_TREE_DEPTH> {
        let mut inserts = vec![];
        for utxo in utxos.iter() {
            for leaf in utxo.leafs() {
                inserts.push(Insert::new(
                    leaf.into(),
                    merkle_path(&self.tree, leaf.into()),
                ));
                if Element::from(leaf) != Note::padding_note().commitment() {
                    self.tree.insert(leaf.into(), ()).unwrap();
                }
            }
        }

        Batch::new(inserts.try_into().unwrap())
    }

    pub fn aggregate_utxo(
        &mut self,
        utxos: &[Utxo<MERKLE_TREE_DEPTH>; UTXO_AGG_NUMBER],
    ) -> AggregateUtxo<UTXO_AGG_NUMBER, MERKLE_TREE_DEPTH, UTXO_AGG_LEAVES> {
        // Convert UTXO to Snarks
        let snarks = utxos
            .iter()
            .map(|utxo| utxo.snark(CircuitKind::Utxo))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let batch_inserts = self.batch_inserts_for_utxos(utxos);

        AggregateUtxo::new(snarks.try_into().unwrap(), batch_inserts)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Wallet {
    pub pk: Element,
}

impl Wallet {
    pub fn new() -> Self {
        Self {
            pk: insecure_random_element(),
        }
    }

    pub fn address(&self) -> Fr {
        poseidon_hash([self.pk.to_base(), Fr::zero()])
    }

    pub fn new_note(&self, amount: u64) -> Note {
        Note::new(self.address().into(), Element::from(amount))
    }

    pub fn new_wallet_note(&self, amount: u64) -> WalletNote {
        WalletNote::new(
            *self,
            Note::new(self.address().into(), Element::from(amount)),
        )
    }
}

#[derive(Clone, Debug)]
pub struct WalletNote {
    note: Note,
    wallet: Wallet,
}

impl WalletNote {
    pub fn new(wallet: Wallet, note: Note) -> Self {
        Self { note, wallet }
    }

    pub fn commitment(&self) -> Element {
        self.note.commitment()
    }

    pub fn nullifier(&self) -> Element {
        self.note.nullifier(self.wallet.pk)
    }

    pub fn note(&self) -> Note {
        self.note.clone()
    }

    pub fn to_input_note(&self, path: Path<MERKLE_TREE_DEPTH>) -> InputNote<MERKLE_TREE_DEPTH> {
        let merkle_path = MerklePath::new(path.siblings_deepest_first().to_vec());
        InputNote::new(self.note.clone(), self.wallet.pk, merkle_path)
    }
}

pub fn merkle_path(
    tree: &Tree<MERKLE_TREE_DEPTH, ()>,
    el: Element,
) -> MerklePath<MERKLE_TREE_DEPTH> {
    MerklePath::new(tree.path_for(el).siblings_deepest_first().to_vec())
}
