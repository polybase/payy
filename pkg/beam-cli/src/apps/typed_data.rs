use std::io::Write;

use contextful::ResultContextExt;
use contracts::Address;
use secp256k1::{Message, SECP256K1, SecretKey};
use serde_json::{Value, json};
use web3::signing::keccak256;

use crate::{
    apps::{
        Error, Result,
        host::{TypedDataSignRequest, parse_hex_data, parse_host_address},
        model::{AppPermissions, ChainOperation},
        permissions::{
            ensure_chain_scope_with_dynamic, normalize_dynamic_contracts,
            validate_dynamic_contracts,
        },
    },
    keystore::{decrypt_private_key, prompt_existing_password},
    runtime::BeamApp,
};

pub async fn sign(
    app: &BeamApp,
    permissions: &AppPermissions,
    request: TypedDataSignRequest,
) -> Result<Value> {
    if !permissions.wallet.sign_typed_data {
        return Err(Error::WalletPermissionDenied {
            permission: "sign-typed-data".to_string(),
        });
    }
    validate_dynamic_contracts(&request.dynamic_contracts, &request.chain)?;
    let (chain, _) = app
        .active_chain_client()
        .await
        .context("connect beam app signing chain client")?;
    if chain.entry.key != request.chain {
        return Err(Error::ChainPermissionDenied {
            chain: request.chain,
            operation: "sign-typed-data".to_string(),
        });
    }
    ensure_chain_scope_with_dynamic(
        permissions,
        &normalize_dynamic_contracts(&request.dynamic_contracts),
        &chain.entry.key,
        ChainOperation::SignTypedData,
        Some(&request.verifying_contract),
        None,
        None,
    )?;

    let wallet = app
        .resolve_wallet(&request.wallet)
        .await
        .context("resolve beam app typed-data signing wallet")?;
    let signer_address =
        wallet
            .address
            .parse::<Address>()
            .map_err(|_| Error::InvalidHostRequest {
                reason: format!("invalid signing wallet address {}", wallet.address),
            })?;
    let verifying_contract = parse_host_address("verifying contract", &request.verifying_contract)?;
    let domain_separator = parse_hash32("domain_separator", &request.domain_separator)?;
    let struct_hash = parse_hash32("struct_hash", &request.struct_hash)?;
    let digest = typed_data_digest(domain_separator, struct_hash);

    prompt_signature(&request, &chain.entry.key, signer_address, digest)?;
    let password = prompt_existing_password().context("read typed-data signing password")?;
    let private_key =
        decrypt_private_key(&wallet, &password).context("decrypt typed-data signing wallet")?;
    let secret_key =
        SecretKey::from_slice(&private_key).map_err(|_| Error::InvalidHostRequest {
            reason: "invalid signing private key".to_string(),
        })?;
    let message = Message::from_digest(digest);
    let signature = SECP256K1.sign_ecdsa_recoverable(&message, &secret_key);
    let (recovery_id, compact) = signature.serialize_compact();
    let recovery_id =
        u8::try_from(recovery_id.to_i32()).map_err(|_| Error::InvalidHostRequest {
            reason: "invalid signature recovery id".to_string(),
        })?;
    let mut out = Vec::with_capacity(65);
    out.extend_from_slice(&compact);
    out.push(27u8.saturating_add(recovery_id));

    Ok(json!({
        "chain": chain.entry.key,
        "digest": format!("0x{}", hex::encode(digest)),
        "primary_type": request.primary_type,
        "signature": format!("0x{}", hex::encode(out)),
        "signer": format!("{signer_address:#x}"),
        "verifying_contract": format!("{verifying_contract:#x}"),
    }))
}

fn prompt_signature(
    request: &TypedDataSignRequest,
    chain: &str,
    signer: Address,
    digest: [u8; 32],
) -> Result<()> {
    let mut stderr = std::io::stderr().lock();
    writeln!(stderr, "Beam app typed-data signature request").context("write signature prompt")?;
    writeln!(stderr, "App chain: {chain}").context("write signature prompt")?;
    writeln!(stderr, "Signing wallet: {signer:#x}").context("write signature prompt")?;
    writeln!(stderr, "Verifying contract: {}", request.verifying_contract)
        .context("write signature prompt")?;
    writeln!(stderr, "Primary type: {}", request.primary_type).context("write signature prompt")?;
    writeln!(stderr, "Typed-data digest: 0x{}", hex::encode(digest))
        .context("write signature prompt")?;
    for field in &request.fields {
        writeln!(stderr, "{} {} = {}", field.kind, field.name, field.value)
            .context("write signature prompt")?;
    }

    Ok(())
}

fn parse_hash32(field: &str, value: &str) -> Result<[u8; 32]> {
    let bytes = parse_hex_data(value)?;
    bytes.try_into().map_err(|_| Error::InvalidHostRequest {
        reason: format!("{field} must be 32 bytes"),
    })
}

fn typed_data_digest(domain_separator: [u8; 32], struct_hash: [u8; 32]) -> [u8; 32] {
    let mut digest_input = Vec::with_capacity(66);
    digest_input.extend_from_slice(&[0x19, 0x01]);
    digest_input.extend_from_slice(&domain_separator);
    digest_input.extend_from_slice(&struct_hash);
    keccak256(&digest_input)
}
