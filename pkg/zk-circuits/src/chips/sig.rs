use std::marker::PhantomData;

use ::secp256k1::{ecdsa::RecoverableSignature, PublicKey};
use eth_types::{
    sign_types::{pk_bytes_le, pk_bytes_swap_endianness, SignData},
    Field,
};
use halo2_base::halo2_proofs::{
    circuit::Layouter,
    halo2curves::secp256k1,
    plonk::{self, ConstraintSystem},
};
use zkevm_circuits::{
    sig_circuit::{utils::AssignedSignatureVerify, SigCircuitConfig, SigCircuitConfigArgs},
    table::{KeccakTable, SigTable},
    util::{Challenges, SubCircuitConfig},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to convert bytes to secp256k1::Fp")]
    FailedToConvertBytesToSecp256k1Fp,
    #[error("Failed to convert bytes to secp256k1::Fq")]
    FailedToConvertBytesToSecp256k1Fq,
}

pub fn convert_sig_to_sign_data(
    signature: RecoverableSignature,
    message: &str,
    public_key: PublicKey,
    msg_hash: [u8; 32],
) -> Result<SignData, Error> {
    let message_bytes = message.as_bytes().to_vec();

    let (recovery_id, compact_signature) = signature.serialize_compact();

    let r = &compact_signature[..32];
    let s = &compact_signature[32..64];

    let mut r_arr = [0u8; 32];
    let mut s_arr = [0u8; 32];
    r_arr.copy_from_slice(r);
    s_arr.copy_from_slice(s);

    let v = recovery_id.to_i32() as u8;

    sig_to_sign_data(public_key, message_bytes, msg_hash, r_arr, s_arr, v)
}

fn sig_to_sign_data(
    pk: PublicKey,
    msg: Vec<u8>,
    mut msg_hash: [u8; 32],
    mut r: [u8; 32],
    mut s: [u8; 32],
    v: u8,
) -> Result<SignData, Error> {
    let mut pk_x_bytes: [u8; 32] = pk.serialize_uncompressed()[1..33].try_into().unwrap();
    let mut pk_y_bytes: [u8; 32] = pk.serialize_uncompressed()[33..65].try_into().unwrap();

    // Change endianness
    pk_x_bytes.reverse();
    pk_y_bytes.reverse();
    r.reverse();
    s.reverse();
    msg_hash.reverse();

    let pk = secp256k1::Secp256k1Affine {
        x: match secp256k1::Fp::from_bytes(&pk_x_bytes) {
            opt if bool::from(opt.is_some()) => opt.unwrap(),
            _ => return Err(Error::FailedToConvertBytesToSecp256k1Fp),
        },
        y: match secp256k1::Fp::from_bytes(&pk_y_bytes) {
            opt if bool::from(opt.is_some()) => opt.unwrap(),
            _ => return Err(Error::FailedToConvertBytesToSecp256k1Fp),
        },
    };

    let r = match secp256k1::Fq::from_bytes(&r) {
        opt if bool::from(opt.is_some()) => opt.unwrap(),
        _ => return Err(Error::FailedToConvertBytesToSecp256k1Fq),
    };
    let s = match secp256k1::Fq::from_bytes(&s) {
        opt if bool::from(opt.is_some()) => opt.unwrap(),
        _ => return Err(Error::FailedToConvertBytesToSecp256k1Fq),
    };

    let msg_hash = match secp256k1::Fq::from_bytes(&msg_hash) {
        opt if bool::from(opt.is_some()) => opt.unwrap(),
        _ => return Err(Error::FailedToConvertBytesToSecp256k1Fq),
    };

    Ok(SignData {
        signature: (r, s, v),
        pk,
        msg: msg.into(),
        msg_hash,
    })
}

#[derive(Clone)]
pub struct SignatureChipConfig<F: Field> {
    challenges: Challenges<plonk::Challenge>,
    sig_circuit_config: SigCircuitConfig<F>,
}

impl<F: Field> SignatureChipConfig<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let keccak_table = KeccakTable::construct(meta);
        let sig_table = SigTable::construct(meta);
        let challenges = Challenges::construct(meta);
        let challenges_exprs = challenges.exprs(meta);

        let sig_circuit_config = SigCircuitConfig::new(
            meta,
            SigCircuitConfigArgs {
                keccak_table,
                sig_table,
                challenges: challenges_exprs,
            },
        );

        Self {
            challenges,
            sig_circuit_config,
        }
    }
}

pub struct SignatureChip<F: Field> {
    config: SignatureChipConfig<F>,
}

impl<F: Field> SignatureChip<F> {
    pub fn construct(config: SignatureChipConfig<F>) -> Self {
        Self { config }
    }

    fn keccak_inputs_sign_verify(&self, sigs: &[SignData]) -> Vec<Vec<u8>> {
        let mut inputs = Vec::new();
        let dummy_sign_data = SignData::default();

        for sig in sigs.iter().chain(std::iter::once(&dummy_sign_data)) {
            let pk_le = pk_bytes_le(&sig.pk);
            let pk_be = pk_bytes_swap_endianness(&pk_le);
            inputs.push(pk_be.to_vec());
            inputs.push(sig.msg.to_vec());
        }

        inputs
    }

    pub fn verify(
        &self,
        layouter: &mut impl Layouter<F>,
        signatures: &[SignData],
    ) -> Result<Vec<AssignedSignatureVerify<F>>, plonk::Error> {
        let sig_circuit = zkevm_circuits::sig_circuit::SigCircuit::<F> {
            max_verif: signatures.len(),
            signatures: signatures.to_vec(),
            _marker: PhantomData,
        };

        let challenges_values = self.config.challenges.values(layouter);

        self.config
            .sig_circuit_config
            .ecdsa_config
            .load_lookup_table(layouter)?;

        let assigned = sig_circuit.assign(
            &self.config.sig_circuit_config,
            layouter,
            &sig_circuit.signatures,
            &challenges_values,
        )?;

        self.config.sig_circuit_config.keccak_table.dev_load(
            layouter,
            &self.keccak_inputs_sign_verify(signatures),
            &challenges_values,
        )?;

        Ok(assigned)
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use ::secp256k1::{
        ecdsa::RecoverableSignature, rand::SeedableRng, Message, PublicKey, Secp256k1, SecretKey,
    };
    use eth_types::{
        sign_types::{biguint_to_32bytes_le, SignData},
        Field,
    };
    use halo2_base::halo2_proofs::{
        circuit::SimpleFloorPlanner,
        dev::MockProver,
        halo2curves::bn256,
        plonk::{Circuit, Column, Instance},
    };
    use halo2_ecc::fields::PrimeField;
    use num_bigint::BigUint;
    use sha3::{Digest, Keccak256};

    use crate::util::decompose_to_limbs;

    use super::*;

    #[derive(Default)]
    struct SignatureCircuit<F: Field> {
        signatures: Vec<SignData>,
        _marker: PhantomData<F>,
    }

    impl<F: Field> SignatureCircuit<F> {
        pub fn new(signatures: Vec<SignData>) -> Self {
            Self {
                signatures,
                _marker: PhantomData,
            }
        }
    }

    impl SignatureCircuit<bn256::Fr> {
        pub fn public_inputs(&self) -> Vec<bn256::Fr> {
            let mut public_inputs = Vec::new();

            for sign_data in &self.signatures {
                let msg_hash_big = BigUint::from_bytes_le(&sign_data.msg_hash.to_bytes());
                for limb in decompose_to_limbs(msg_hash_big, 3, 88)
                    .into_iter()
                    .map(|b| bn256::Fr::from_bytes(&biguint_to_32bytes_le(b)))
                {
                    public_inputs.push(limb.unwrap());
                }
            }

            public_inputs
        }
    }

    impl<F: Field> Circuit<F> for SignatureCircuit<F> {
        type Config = (Column<Instance>, SignatureChipConfig<F>);

        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut plonk::ConstraintSystem<F>) -> Self::Config {
            let fixed_column = meta.fixed_column();
            meta.enable_constant(fixed_column);

            let msg_hash_instance = meta.instance_column();
            meta.enable_equality(msg_hash_instance);

            (msg_hash_instance, SignatureChipConfig::configure(meta))
        }

        fn synthesize(
            &self,
            (msg_hash_instance, config): Self::Config,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), plonk::Error> {
            let sig_chip = SignatureChip::construct(config);

            let verified_sigs = sig_chip.verify(&mut layouter, &self.signatures)?;

            let msg_hash = verified_sigs[0].assigned_ecdsa.msg_hash.limbs();
            for (i, limb) in msg_hash.iter().enumerate() {
                layouter.constrain_instance(limb.cell, msg_hash_instance, i)?;
            }

            layouter.assign_region(
                || "check sig_is_valid = 1",
                |mut region| {
                    region.constrain_constant(verified_sigs[0].sig_is_valid.cell(), F::one())?;

                    Ok(())
                },
            )?;

            Ok(())
        }
    }

    fn generate_signature(
        message_str: &str,
    ) -> (PublicKey, SecretKey, RecoverableSignature, [u8; 32]) {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(
            &mut ::secp256k1::rand::prelude::SmallRng::from_seed([0u8; 32]),
        );

        let message_bytes = message_str.as_bytes();

        let mut hasher = Keccak256::new();
        hasher.update(message_bytes);
        let hash = hasher.finalize();

        let message = Message::from_digest_slice(&hash).expect("32 bytes");
        let signature = secp.sign_ecdsa_recoverable(&message, &secret_key);

        (public_key, secret_key, signature, hash.into())
    }

    #[test]
    fn verify_sig_in_circuit() {
        let msg = "hello world";
        let (pk, _sk, sig, msg_hash) = generate_signature(msg);
        let sign_data = convert_sig_to_sign_data(sig, msg, pk, msg_hash).unwrap();
        let signatures = vec![sign_data];

        let circuit = SignatureCircuit::<bn256::Fr>::new(signatures);
        let prover =
            MockProver::<bn256::Fr>::run(20, &circuit, vec![circuit.public_inputs()]).unwrap();
        prover.assert_satisfied();
    }
}
