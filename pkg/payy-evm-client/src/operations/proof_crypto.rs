use bn254_blackbox_solver::multi_scalar_mul;
use contextful::ResultContextExt;
use element::Element;
use hash::hash_merge;
use payy_evm_client_interface::{B256, PrivacyAccount, Result, ValidationErrorKind};
use zk_primitives::EvmNote;

use crate::client::PrivacyNamespace;
use crate::util::{b256_to_element, element_to_b256, random_nonzero_field, random_u240_field};

pub(super) fn encrypted_note(note: EvmNote, symmetric_key: Element) -> [Element; 5] {
    encrypt_payload(
        [note.token, note.nonce, note.psi, note.owner, note.value],
        symmetric_key,
    )
}

pub(super) fn encrypt_payload<const N: usize>(
    payload: [Element; N],
    symmetric_key: Element,
) -> [Element; N] {
    core::array::from_fn(|index| {
        Element::from_base(
            payload[index].to_base()
                + hash_merge([symmetric_key, Element::new(index as u64)]).to_base(),
        )
    })
}

pub(super) fn encrypt_key_for_public_key(
    symmetric_key: Element,
    public_key_x: Element,
    public_key_y: Element,
) -> Result<[B256; 4]> {
    let ephemeral_private_key = random_nonzero_field();
    let (ephemeral_x, ephemeral_y) =
        payy_evm_client_interface::grumpkin_public_key_from_private_key(
            ephemeral_private_key.to_be_bytes(),
        )?;
    let (shared_x, shared_y) = grumpkin_mul(ephemeral_private_key, public_key_x, public_key_y)?;
    Ok([
        element_to_b256(ephemeral_x),
        element_to_b256(ephemeral_y),
        element_to_b256(Element::from_base(
            symmetric_key.to_base() + hash_merge([shared_x, shared_y]).to_base(),
        )),
        [0; 32],
    ])
}

pub(super) fn encrypt_chain_key(
    symmetric_key: Element,
    chain_public_key_x: Element,
    chain_public_key_y: Element,
) -> Result<[Element; 3]> {
    let (shared_x, shared_y) = grumpkin_mul(symmetric_key, chain_public_key_x, chain_public_key_y)?;
    let (ephemeral_x, ephemeral_y) =
        payy_evm_client_interface::grumpkin_public_key_from_private_key(
            symmetric_key.to_be_bytes(),
        )?;
    Ok([
        ephemeral_x,
        ephemeral_y,
        Element::from_base(symmetric_key.to_base() + hash_merge([shared_x, shared_y]).to_base()),
    ])
}

pub(super) async fn chain_public_key(client: &PrivacyNamespace) -> Result<[Element; 2]> {
    let (x, y) = client.bridge().get_chain_public_key().await?;
    Ok([b256_to_element(x), b256_to_element(y)])
}

pub(super) async fn encryption_for_address(
    client: &PrivacyNamespace,
    note: EvmNote,
    privacy_account: &PrivacyAccount,
) -> Result<(Element, [Element; 5], [Element; 3], [B256; 4])> {
    let symmetric_key = random_u240_field();
    let (public_key_x, public_key_y) = privacy_account.privacy_address().public_key()?;
    let chain_key = chain_public_key(client).await?;
    Ok((
        symmetric_key,
        encrypted_note(note, symmetric_key),
        encrypt_chain_key(symmetric_key, chain_key[0], chain_key[1])?,
        encrypt_key_for_public_key(symmetric_key, public_key_x, public_key_y)?,
    ))
}

fn grumpkin_mul(
    scalar: Element,
    public_key_x: Element,
    public_key_y: Element,
) -> Result<(Element, Element)> {
    let bytes = scalar.to_be_bytes();
    let scalar_hi = u128::from_be_bytes(
        bytes[..16]
            .try_into()
            .context("read grumpkin scalar high limb")?,
    );
    let scalar_lo = u128::from_be_bytes(
        bytes[16..]
            .try_into()
            .context("read grumpkin scalar low limb")?,
    );
    let (x, y, infinite) = multi_scalar_mul(
        &[
            public_key_x.to_base(),
            public_key_y.to_base(),
            Element::ZERO.to_base(),
        ],
        &[Element::from(scalar_lo).to_base()],
        &[Element::from(scalar_hi).to_base()],
        true,
    )
    .context("compute grumpkin shared secret")?;
    if infinite != Element::ZERO.to_base() {
        return Err(payy_evm_client_interface::Error::Validation {
            kind: ValidationErrorKind::InvalidPrivacyAddress,
        });
    }
    Ok((Element::from_base(x), Element::from_base(y)))
}
