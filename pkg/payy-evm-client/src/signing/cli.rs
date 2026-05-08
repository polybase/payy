use std::{
    io::Write,
    process::{Command, Stdio},
};

use contextful::ResultContextExt;
use payy_evm_client_interface::{B256, Error, Result, SignerResponseField};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SchnorrConstructSignatureCommand<'a> {
    #[serde(with = "serde_bytes")]
    message: &'a [u8],
    #[serde(with = "serde_bytes")]
    private_key: &'a [u8],
}

#[derive(Deserialize)]
struct SchnorrConstructSignatureResponse {
    #[serde(with = "serde_bytes")]
    s: Vec<u8>,
    #[serde(with = "serde_bytes")]
    e: Vec<u8>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    message: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SchnorrConstructSignaturePayload {
    Signature(SchnorrConstructSignatureResponse),
    Error(ErrorResponse),
}

pub(crate) fn schnorr_construct_signature(
    message: B256,
    private_key: B256,
) -> Result<(B256, B256)> {
    let request = [(
        "SchnorrConstructSignature",
        SchnorrConstructSignatureCommand {
            message: &message,
            private_key: &private_key,
        },
    )];
    let request = rmp_serde::to_vec_named(&request).context("encode bb schnorr request")?;
    let mut framed_request = Vec::with_capacity(4 + request.len());
    framed_request.extend_from_slice(
        &u32::try_from(request.len())
            .context("encode bb schnorr request length")?
            .to_le_bytes(),
    );
    framed_request.extend_from_slice(&request);

    let mut child = Command::new("bb")
        .arg("msgpack")
        .arg("run")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn bb msgpack run")?;

    {
        let mut stdin = child.stdin.take().ok_or(Error::SignerCommandFailed {
            message: "bb msgpack stdin unavailable".to_owned(),
        })?;
        stdin
            .write_all(&framed_request)
            .context("write bb schnorr request")?;
    }
    let output = child
        .wait_with_output()
        .context("wait for bb schnorr response")?;

    if !output.status.success() {
        return Err(Error::SignerCommandFailed {
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let response = read_framed_response(&output.stdout)?;
    let (variant, payload): (String, SchnorrConstructSignaturePayload) =
        rmp_serde::from_slice(response).context("decode bb schnorr response")?;

    match (variant.as_str(), payload) {
        (
            "SchnorrConstructSignatureResponse",
            SchnorrConstructSignaturePayload::Signature(signature),
        ) => Ok((
            signature_field_to_b256(SignerResponseField::S, signature.s)?,
            signature_field_to_b256(SignerResponseField::E, signature.e)?,
        )),
        ("ErrorResponse", SchnorrConstructSignaturePayload::Error(error)) => {
            Err(Error::SignerCommandFailed {
                message: error.message,
            })
        }
        (variant, _) => Err(Error::UnexpectedSignerResponse {
            variant: variant.to_owned(),
        }),
    }
}

fn signature_field_to_b256(field: SignerResponseField, value: Vec<u8>) -> Result<B256> {
    let length = value.len();
    value
        .try_into()
        .map_err(|_| Error::InvalidSignerResponseLength { field, length })
}

fn read_framed_response(stdout: &[u8]) -> Result<&[u8]> {
    let length = stdout
        .get(..4)
        .ok_or(Error::SignerCommandFailed {
            message: "bb msgpack response missing length prefix".to_owned(),
        })
        .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))?;
    let length = usize::try_from(length).context("decode bb schnorr response length")?;
    let response = stdout
        .get(4..4 + length)
        .ok_or(Error::SignerCommandFailed {
            message: "bb msgpack response body incomplete".to_owned(),
        })?;
    Ok(response)
}
