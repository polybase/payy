use super::{
    fs::{load_witness, save_witness},
    rollup::Rollup,
};
use crate::{
    aggregate_utxo::{constants::UTXO_AGGREGATE_3_161_12_VK, AggregateUtxo},
    chips::aggregation::snark::Snark,
    constants::{MERKLE_TREE_DEPTH, UTXO_AGG_LEAVES, UTXO_AGG_NUMBER},
    data::{Batch, ParameterSet, SnarkWitness, Utxo},
    CircuitKind,
};

pub fn create_or_load_agg_utxo_snarks<const N: usize>(params: ParameterSet) -> [Snark; N] {
    if let Some(utxo_aggs) = load_agg_utxo_snarks(params) {
        utxo_aggs
    } else {
        let snarks = create_agg_utxo_snarks::<N, 3>(params);
        for (i, snark) in snarks.iter().enumerate() {
            save_witness(
                &format!("agg_utxo_{i}"),
                &SnarkWitness::V1(snark.to_witness()),
            );
        }
        snarks
    }
}

pub fn load_agg_utxo_snarks<const N: usize>(params: ParameterSet) -> Option<[Snark; N]> {
    let mut snarks = vec![];
    for i in 0..N {
        let sw = load_witness(&format!("agg_utxo_{i}"))?;
        let SnarkWitness::V1(sw) = sw;
        let snark = sw.to_snark(&UTXO_AGGREGATE_3_161_12_VK, params);
        snarks.push(snark);
    }
    Some(snarks.try_into().unwrap())
}

pub fn default_aggregate_utxo() -> AggregateUtxo<UTXO_AGG_NUMBER, MERKLE_TREE_DEPTH, UTXO_AGG_LEAVES>
{
    let mut snarks = vec![];
    for _ in 0..UTXO_AGG_NUMBER {
        snarks.push(
            Utxo::<MERKLE_TREE_DEPTH>::new_padding()
                .snark(CircuitKind::Utxo)
                .unwrap(),
        );
    }
    AggregateUtxo::<UTXO_AGG_NUMBER, MERKLE_TREE_DEPTH, UTXO_AGG_LEAVES>::new(
        snarks.try_into().unwrap(),
        Batch::default(),
    )
}

pub fn create_agg_utxo_snarks<const N: usize, const UTXO_N: usize>(
    params: ParameterSet,
) -> [Snark; N] {
    let mut rollup = Rollup::new();
    let bob = rollup.new_wallet();
    let sally = rollup.new_wallet();

    // Add existing notes to the tree
    let bob_notes = (0..UTXO_N * N)
        .map(|_| rollup.unverified_add_unspent_note(&bob, 100))
        .collect::<Vec<_>>();

    let utxos = bob_notes
        .iter()
        .map(|note| rollup.transfer(note.clone(), sally.new_note(100)))
        .collect::<Vec<_>>();

    utxos
        .chunks(UTXO_N)
        .map(|utxos| {
            let utxo_agg = rollup.aggregate_utxo(utxos.try_into().unwrap());
            utxo_agg.snark(params)
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .try_into()
        .unwrap()
}
