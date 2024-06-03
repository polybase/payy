use std::{io::Cursor, sync::OnceLock};

use halo2_base::halo2_proofs::{
    halo2curves::bn256::Bn256,
    poly::{commitment::Params, kzg::commitment::ParamsKZG},
};

use crate::data::ParameterSet;

const BYTES_6: &[u8] = include_bytes!("../../../fixtures/params/kzg_bn254_6.srs");
const BYTES_8: &[u8] = include_bytes!("../../../fixtures/params/kzg_bn254_8.srs");
const BYTES_9: &[u8] = include_bytes!("../../../fixtures/params/kzg_bn254_9.srs");
const BYTES_14: &[u8] = include_bytes!("../../../fixtures/params/kzg_bn254_14.srs");
const BYTES_21: &[u8] = include_bytes!("../../../fixtures/params/kzg_bn254_21.srs");

static PARAMS_6: OnceLock<ParamsKZG<Bn256>> = OnceLock::new();
static PARAMS_8: OnceLock<ParamsKZG<Bn256>> = OnceLock::new();
static PARAMS_9: OnceLock<ParamsKZG<Bn256>> = OnceLock::new();
static PARAMS_14: OnceLock<ParamsKZG<Bn256>> = OnceLock::new();
static PARAMS_21: OnceLock<ParamsKZG<Bn256>> = OnceLock::new();

fn load(bytes: &[u8]) -> ParamsKZG<Bn256> {
    ParamsKZG::read(&mut Cursor::new(bytes)).unwrap()
}

pub(crate) fn load_params(params: ParameterSet) -> &'static ParamsKZG<Bn256> {
    match params {
        ParameterSet::Six => PARAMS_6.get_or_init(|| load(BYTES_6)),
        ParameterSet::Eight => PARAMS_8.get_or_init(|| load(BYTES_8)),
        ParameterSet::Nine => PARAMS_9.get_or_init(|| load(BYTES_9)),
        ParameterSet::Fourteen => PARAMS_14.get_or_init(|| load(BYTES_14)),
        ParameterSet::TwentyOne => PARAMS_21.get_or_init(|| load(BYTES_21)),
    }
}
