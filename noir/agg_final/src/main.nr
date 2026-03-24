use dep::std;
use poseidon::poseidon2;

global AGG_UTXO_VERIFICATION_KEY_HASH: Field = 5897206462939179313090278977855702552208277022861650698363349603526525271812;
global AGG_AGG_VERIFICATION_KEY_HASH: Field = 11623735493169461569078977608660595680947806981338331162316688672907570294243;

fn main(
    verification_key: [Field; 115],
    verification_key_hash: Field,
    proof: [Field; 508],
    old_root: pub Field,
    new_root: pub Field,
    commit_hash: pub Field,
    messages: pub [Field; 1000],
) {
    // Must be an agg_agg proof
    assert(verification_key_hash == AGG_AGG_VERIFICATION_KEY_HASH, "only agg_agg proof allowed");

    // Build public inputs for the agg_agg proof
    let mut public_inputs: [Field; 1005] = [0; 1005];
    public_inputs[0] = AGG_UTXO_VERIFICATION_KEY_HASH;
    public_inputs[1] = AGG_AGG_VERIFICATION_KEY_HASH;
    public_inputs[2] = old_root;
    public_inputs[3] = new_root;
    public_inputs[4] = commit_hash;

    for i in 0..1000 {
        public_inputs[i + 5] = messages[i];
    }

    // Verify the agg_agg proof
    std::verify_proof_with_type(
        verification_key,
        proof,
        public_inputs,
        verification_key_hash,
        7,
    );
}
