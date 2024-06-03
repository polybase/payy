use crate::constants::BLAKE_PERSONALISATION;
use crate::data::ParameterSet;
use crate::params::load_params;
use base64::Engine;
use blake2b_simd::Params as Blake2bParams;
use halo2_base::halo2_proofs::{
    arithmetic::{Field, FieldExt},
    circuit::{AssignedCell, Layouter, Value},
    halo2curves::bn256::{Fr, G1Affine},
    plonk::{
        keygen_pk, keygen_vk, Advice, Assigned, Circuit, Column, Error, ProvingKey, VerifyingKey,
    },
};
use num_bigint::{BigUint, ToBigUint};
use serde::{Deserialize, Deserializer, Serializer};
use zk_primitives::Element;

pub(crate) fn assign_private_input<F: FieldExt, V: Copy, N: Fn() -> NR, NR: Into<String>>(
    name: N,
    mut layouter: impl Layouter<F>,
    column: Column<Advice>,
    value: Value<V>,
) -> Result<AssignedCell<V, F>, Error>
where
    for<'v> Assigned<F>: From<&'v V>,
{
    layouter.assign_region(name, |mut region| {
        region.assign_advice(|| "load advice", column, 0, || value)
    })
}

pub(crate) fn assign_constant<F: FieldExt, V: Copy, N: Fn() -> NR, NR: Into<String>>(
    name: N,
    mut layouter: impl Layouter<F>,
    column: Column<Advice>,
    value: V,
) -> Result<AssignedCell<V, F>, Error>
where
    for<'v> Assigned<F>: From<&'v V>,
{
    layouter.assign_region(name, |mut region| {
        region.assign_advice_from_constant(|| "load constant advice", column, 0, value)
    })
}

pub fn blake_hash<const L: usize>(message: [&[u8]; L]) -> Element {
    let mut h = Blake2bParams::new()
        .hash_length(64)
        .personal(BLAKE_PERSONALISATION)
        .to_state();

    for i in message {
        h.update(i);
    }

    let psi_bytes = *h.finalize().as_array();
    Fr::from_bytes_wide(&psi_bytes).into()
}

// fn random_bytes_32() {
//     let mut rng = rand::thread_rng();
//     let mut bytes = [0u8; 32];
//     rng.fill(&mut bytes[..])
// }

pub(crate) fn random_fr() -> Fr {
    let mut rng = rand::thread_rng();
    Fr::random(&mut rng)
}

pub fn insecure_random_element() -> Element {
    let mut rng: rand::rngs::ThreadRng = rand::thread_rng();
    Element::random(&mut rng).get_insecure()
}
pub(crate) fn keygen_from_params<C: Circuit<Fr>>(
    params: ParameterSet,
    circuit: &C,
) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
    let params = load_params(params);

    let vk = keygen_vk(params, circuit).expect("keygen_vk should not fail");
    let pk = keygen_pk(params, vk.clone(), circuit).expect("keygen_pk should not fail");

    (pk, vk)
}

pub fn decompose_to_limbs(mut num: BigUint, num_limbs: u32, num_bits: u32) -> Vec<BigUint> {
    let mut parts = Vec::new();
    for _ in 0..num_limbs {
        let limb_mask = (BigUint::from(1u128) << num_bits) - 1u32.to_biguint().unwrap();
        let part = &num & &limb_mask;
        parts.push(part);
        num >>= num_bits;
    }

    parts
}

// Custom serializer for Vec<u8> to base64 string
pub fn serialize_base64<S>(value: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let base64_string = base64::engine::general_purpose::STANDARD.encode(value);
    serializer.serialize_str(&base64_string)
}

// Custom deserializer for base64 string to Vec<u8>
pub fn deserialize_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(serde::de::Error::custom)
}

pub fn serialize_hex_0x_prefixed<S>(value: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex_string = format!("0x{}", hex::encode(value));
    serializer.serialize_str(&hex_string)
}

pub fn deserialize_hex_0x_prefixed<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let s = s.trim_start_matches("0x");
    hex::decode(s).map_err(serde::de::Error::custom)
}
