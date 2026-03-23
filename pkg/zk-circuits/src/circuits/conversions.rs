macro_rules! impl_circuit_proof_conversions {
    (
        $primitive_public_inputs:ty,
        $circuit_public_inputs:ty,
        $primitive_proof:ty,
        $circuit_proof:ty,
        [$($field:ident),+ $(,)?]
    ) => {
        impl_circuit_proof_conversions!(
            $primitive_public_inputs,
            $circuit_public_inputs,
            $primitive_proof,
            $circuit_proof,
            [$($field),+],
            []
        );
    };

    (
        $primitive_public_inputs:ty,
        $circuit_public_inputs:ty,
        $primitive_proof:ty,
        $circuit_proof:ty,
        [$($field:ident),+ $(,)?],
        [$($proof_extra_field:ident : $proof_extra_value:expr),* $(,)?]
    ) => {
        impl From<$circuit_public_inputs> for $primitive_public_inputs {
            fn from(value: $circuit_public_inputs) -> Self {
                Self {
                    $(
                        $field: value.$field,
                    )+
                }
            }
        }

        impl From<$primitive_public_inputs> for $circuit_public_inputs {
            fn from(value: $primitive_public_inputs) -> Self {
                Self {
                    $(
                        $field: value.$field,
                    )+
                }
            }
        }

        impl From<$circuit_proof> for $primitive_proof {
            fn from(value: $circuit_proof) -> Self {
                Self {
                    proof: zk_primitives::ProofBytes(value.proof),
                    public_inputs: value.public_inputs.into(),
                    $(
                        $proof_extra_field: $proof_extra_value,
                    )*
                }
            }
        }

        impl From<$primitive_proof> for $circuit_proof {
            fn from(value: $primitive_proof) -> Self {
                Self {
                    proof: value.proof.0,
                    public_inputs: value.public_inputs.into(),
                }
            }
        }
    };
}

pub(crate) use impl_circuit_proof_conversions;
