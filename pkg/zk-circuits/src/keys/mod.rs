use std::sync::OnceLock;

use halo2_base::halo2_proofs::{
    halo2curves::bn256::G1Affine,
    plonk::{ProvingKey, VerifyingKey},
};

use crate::{
    aggregate_utxo::AggregateUtxo,
    data::{AggregateAgg, Burn, Mint, ParameterSet, Points, Signature, Utxo},
};

type VK = VerifyingKey<G1Affine>;
type PK = ProvingKey<G1Affine>;

// macro because we don't have a trait for this, just a convention
macro_rules! create {
    ($self:expr, $circuit:ty) => {{
        let circ = <$circuit>::default();
        circ.keygen($self.params())
    }};
}

macro_rules! vk_function {
    ($name:ident, $t:ty) => {
        fn $name() -> &'static VerifyingKey<G1Affine> {
            static CACHE: OnceLock<VerifyingKey<G1Affine>> = OnceLock::new();
            const VK_HEX: &str = include_str!(concat!("vk/", stringify!($name)));

            CACHE.get_or_init(|| {
                let vk_bytes = hex::decode(VK_HEX.replace(['\n', '"', ' '], "")).unwrap();
                VerifyingKey::<G1Affine>::from_bytes::<$t>(
                    &vk_bytes,
                    halo2_base::halo2_proofs::SerdeFormat::Processed,
                )
                .unwrap()
            })
        }
    };
}

vk_function!(agg_agg_2, AggregateAgg::<2>);
vk_function!(points, Points);
vk_function!(utxo, Utxo::<161>);
vk_function!(utxo_agg_3_161_12, AggregateUtxo::<3, 161, 12>);


pub enum CircuitKind {
    Signature,
    Points,
    Utxo,
    AggUtxo,
    AggAgg,
    Burn,
    Mint,
}

impl CircuitKind {
    #[inline]
    pub fn params(&self) -> ParameterSet {
        match self {
            Self::Points => ParameterSet::Fourteen,
            Self::Utxo => ParameterSet::Fourteen,
            Self::AggUtxo => ParameterSet::TwentyOne,
            Self::AggAgg => ParameterSet::TwentyOne,
            Self::Signature => ParameterSet::Six,
            Self::Burn => ParameterSet::Nine,
            Self::Mint => ParameterSet::Eight,
        }
    }

    pub(crate) fn vk(&self) -> &'static VK {
        match self {
            Self::Points => points(),
            Self::AggUtxo => utxo_agg_3_161_12(),
            Self::AggAgg => agg_agg_2(),
            Self::Utxo => utxo(),

            _ => {
                let (_, vk) = self.keys();
                vk
            }
        }
    }

    pub(crate) fn pk(&self) -> &'static PK {
        let (pk, _) = self.keys();
        pk
    }

    fn keys(&self) -> &'static (PK, VK) {
        static SIGNATURE: OnceLock<(PK, VK)> = OnceLock::new();
        static POINTS: OnceLock<(PK, VK)> = OnceLock::new();
        static UTXO_KEYS: OnceLock<(PK, VK)> = OnceLock::new();
        static AGG_UTXO: OnceLock<(PK, VK)> = OnceLock::new();
        static AGG_AGG: OnceLock<(PK, VK)> = OnceLock::new();
        static BURN_KEYS: OnceLock<(PK, VK)> = OnceLock::new();
        static MINT: OnceLock<(PK, VK)> = OnceLock::new();

        match self {
            Self::Signature => SIGNATURE.get_or_init(|| create!(self, Signature)),
            Self::Points => POINTS.get_or_init(|| create!(self, Points)),
            Self::Utxo => UTXO_KEYS.get_or_init(|| create!(self, Utxo::<161>)),
            Self::AggUtxo => AGG_UTXO.get_or_init(|| create!(self, AggregateUtxo::<3, 161, 12>)),
            Self::AggAgg => AGG_AGG.get_or_init(|| create!(self, AggregateAgg::<2>)),
            Self::Burn => BURN_KEYS.get_or_init(|| create!(self, Burn::<1>)),
            Self::Mint => MINT.get_or_init(|| create!(self, Mint::<1>)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        let kinds = [
            CircuitKind::Signature,
            CircuitKind::Points,
            CircuitKind::Utxo,
            CircuitKind::AggUtxo,
            CircuitKind::Burn,
            CircuitKind::Mint,
        ];

        for kind in kinds {
            let _ = kind.vk();
            let _ = kind.keys();
        }
    }
}
