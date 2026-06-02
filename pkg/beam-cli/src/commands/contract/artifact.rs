use sourcify_interface::SourcifyClient;

use crate::{output::OutputMode, runtime::BeamApp};

use super::{
    error::{Error, Result, RuntimeUnchecked},
    info::print_sourcify_miss_tip,
    sourcify::{Artifact, artifact_label, lookup_contract},
    target::{InspectionTarget, RpcProbe, optional_rpc_probe},
};

pub(super) async fn runtime_verified_artifact(
    app: &BeamApp,
    sourcify_client: &dyn SourcifyClient,
    target: &InspectionTarget,
    fields: Vec<sourcify_interface::ContractField>,
    cap_bytes: usize,
    artifact: Artifact,
) -> Result<sourcify_interface::ContractResponse> {
    match lookup_contract(sourcify_client, target, fields, cap_bytes).await {
        Ok(response) => match response.contract.runtime_match {
            Some(_) => Ok(response),
            None => {
                let err = Error::SourcifyRuntimeNotVerified {
                    address: target.checksum_address.clone(),
                    artifact: artifact_label(artifact).to_owned(),
                    runtime_unchecked: None,
                };
                print_sourcify_miss_tip(app.output_mode, target, &err, false, None);
                Err(err)
            }
        },
        Err(Error::SourcifyNotVerified { .. }) => {
            let err = Error::SourcifyNotVerified {
                address: target.checksum_address.clone(),
                artifact: artifact_label(artifact).to_owned(),
                runtime_unchecked: None,
            };
            probe_after_sourcify_miss(app.output_mode, app, target, err).await
        }
        Err(err) => Err(err),
    }
}

async fn probe_after_sourcify_miss(
    output_mode: OutputMode,
    app: &BeamApp,
    target: &InspectionTarget,
    miss_error: Error,
) -> Result<sourcify_interface::ContractResponse> {
    match optional_rpc_probe(app, target).await? {
        RpcProbe::NoRuntimeCode => Err(Error::NoRuntimeCode {
            address: target.checksum_address.clone(),
        }),
        RpcProbe::RuntimeCode => {
            print_sourcify_miss_tip(output_mode, target, &miss_error, true, None);
            Err(miss_error)
        }
        RpcProbe::Unchecked { reason } => {
            let miss_error = with_runtime_unchecked(miss_error, reason);
            let reason = runtime_unchecked_reason(&miss_error);
            print_sourcify_miss_tip(output_mode, target, &miss_error, false, reason);
            Err(miss_error)
        }
    }
}

fn with_runtime_unchecked(err: Error, reason: Option<String>) -> Error {
    let runtime_unchecked = Some(RuntimeUnchecked { reason });
    match err {
        Error::SourcifyNotVerified {
            address, artifact, ..
        } => Error::SourcifyNotVerified {
            address,
            artifact,
            runtime_unchecked,
        },
        Error::SourcifyRuntimeNotVerified {
            address, artifact, ..
        } => Error::SourcifyRuntimeNotVerified {
            address,
            artifact,
            runtime_unchecked,
        },
        err => err,
    }
}

fn runtime_unchecked_reason(err: &Error) -> Option<&str> {
    match err {
        Error::SourcifyNotVerified {
            runtime_unchecked, ..
        }
        | Error::SourcifyRuntimeNotVerified {
            runtime_unchecked, ..
        } => runtime_unchecked
            .as_ref()
            .and_then(|unchecked| unchecked.reason.as_deref()),
        _ => None,
    }
}
