use benchy::{benchmark, BenchmarkRun};
use halo2_base::halo2_proofs::{dev::MockProver, halo2curves::bn256::Fr};
use rand::thread_rng;
use smirk::{hash_merge, Element, Tree};
use zk_circuits::{
    aggregate_utxo::AggregateUtxo,
    chips::aggregation::snark::Snark,
    data::{Batch, InputNote, Insert, MerklePath, Note, SnarkWitnessV1, Utxo, UtxoKind},
    test::util::{get_params, get_snark},
    util::insecure_random_element,
};

const MERKLE_TREE_DEPTH: usize = 161;

fn gen_utxo() -> (Snark, Fr, Fr) {
    let k = 12;

    let pk = Element::secure_random(thread_rng());
    let from_address = hash_merge([pk, Element::ZERO]);
    let to_address = insecure_random_element();

    // Input notes
    let note = Note::new(from_address, Element::from(100u64));
    let path = MerklePath::<MERKLE_TREE_DEPTH>::default();
    let input_note = InputNote::new(note.clone(), pk, path.clone());
    let nullifier = input_note.nullifer();
    let input_notes = [input_note, InputNote::padding_note()];
    let recent_root = path.compute_root(note.commitment());

    // Output notes
    let output_note = Note::new(to_address, Element::from(100u64));
    let output_notes = [output_note.clone(), Note::padding_note()];

    let circuit = Utxo::new(input_notes, output_notes, recent_root, UtxoKind::Transfer);
    let instances = circuit.public_inputs();

    // Assert this proof is valid before trying to aggregate it
    let prover = MockProver::<Fr>::run(k, &circuit, vec![circuit.public_inputs()]).unwrap();
    prover.assert_satisfied();

    (
        get_snark(k, circuit, instances).unwrap(),
        nullifier.into(),
        output_note.commitment().into(),
    )
}

fn utxo_to_inserts<const N_INSERTS: usize>(
    utxos: &[SnarkWitnessV1],
) -> [Insert<MERKLE_TREE_DEPTH>; N_INSERTS] {
    let mut tree = Tree::<MERKLE_TREE_DEPTH, ()>::new();

    let (mut inserts, _old_tree, _new_tree) = {
        let old_tree = tree.root_hash();

        let mut leaves = vec![];

        // Extract leaves to be inserted from proof
        for proof in utxos {
            let instances = &proof.instances[0];
            let elements = instances
                .iter()
                .skip(1)
                .map(|f| Element::from_base(f.to_base()))
                .collect::<Vec<Element>>();

            // Skip the first instance, as that is the root
            leaves.extend(elements);
        }

        let paths = tree
            .insert_with_paths_default(leaves.iter().copied())
            .unwrap();

        let mut inserts = vec![];

        // Convert paths to merkle paths
        for (path, leaf) in paths.iter().zip(&leaves) {
            let fpath = path
                .siblings
                .iter()
                .cloned()
                .take(MERKLE_TREE_DEPTH)
                .collect::<Vec<Element>>();
            let mp: MerklePath<MERKLE_TREE_DEPTH> = MerklePath::new(fpath);
            let insert = Insert::new(*leaf, mp);
            inserts.push(insert)
        }

        let new_tree = tree.root_hash();

        (inserts, old_tree, new_tree)
    };

    while inserts.len() < N_INSERTS {
        inserts.push(Insert::padding_insert());
    }

    inserts
        .try_into()
        .unwrap_or_else(|_| panic!("Expected slice of length {N_INSERTS}"))
}

fn gen_aggregate_input<const UTXOS: usize, const BATCH_N: usize>(
) -> ([Snark; UTXOS], Batch<BATCH_N, MERKLE_TREE_DEPTH>) {
    assert_eq!(UTXOS * 4, BATCH_N);

    let mut utxos = Vec::new();
    for _ in 0..UTXOS {
        utxos.push(gen_utxo().0);
    }

    let inserts = utxo_to_inserts::<BATCH_N>(
        &utxos
            .iter()
            .map(|utxo| utxo.to_witness())
            .collect::<Vec<_>>(),
    );

    // let params = halo2_base::utils::fs::gen_srs(if BATCH_N <= 8 { 15 } else { 16 });
    (utxos.try_into().unwrap(), Batch::new(inserts))
}

fn aggregate<const UTXOS: usize, const BATCH_N: usize>(b: &mut BenchmarkRun) {
    let k = 21;

    let (utxos, batch) = gen_aggregate_input::<UTXOS, BATCH_N>();
    let circuit = AggregateUtxo::new(utxos, batch);

    let (params, vk, pk) = get_params(k, &circuit);
    let public_inputs = circuit.public_inputs();

    let proof = b
        .run(|| {
            zk_circuits::proof::Proof::create(
                &params,
                &pk,
                circuit,
                &[&public_inputs],
                rand::thread_rng(),
            )
        })
        .unwrap();
    proof.verify(&vk, &params, &[&public_inputs]).unwrap();
}

#[benchmark]
fn aggregate_1_utxo(b: &mut BenchmarkRun) {
    aggregate::<1, 4>(b);
}

#[benchmark]
fn aggregate_2_utxo(b: &mut BenchmarkRun) {
    aggregate::<2, 8>(b);
}

// This size requires num_lookup_advice: 2, bumped from 1
#[benchmark]
fn aggregate_3_utxo(b: &mut BenchmarkRun) {
    aggregate::<3, 12>(b);
}

benchy::main!(aggregate_1_utxo, aggregate_2_utxo, aggregate_3_utxo);
