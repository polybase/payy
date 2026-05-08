use std::sync::Arc;

#[cfg(not(any(feature = "bb-bindings", feature = "bb-cli")))]
use async_trait::async_trait;
#[cfg(not(any(feature = "bb-bindings", feature = "bb-cli")))]
use contextful::ErrorContextExt;
#[cfg(any(feature = "bb-bindings", feature = "bb-cli"))]
use payy_evm_client_prover_bb::BarretenbergPrivacyProver;
use payy_evm_client_prover_interface::PrivacyProver;
#[cfg(not(any(feature = "bb-bindings", feature = "bb-cli")))]
use payy_evm_client_prover_interface::{ProveOutput, ProveRequest};

#[cfg(feature = "bb-bindings")]
pub(super) fn default_prover() -> Arc<dyn PrivacyProver> {
    Arc::new(BarretenbergPrivacyProver::new(Arc::new(
        barretenberg_rs::BindingBackend,
    )))
}

#[cfg(all(not(feature = "bb-bindings"), feature = "bb-cli"))]
pub(super) fn default_prover() -> Arc<dyn PrivacyProver> {
    Arc::new(BarretenbergPrivacyProver::new(Arc::new(
        barretenberg_cli::CliBackend,
    )))
}

#[cfg(not(any(feature = "bb-bindings", feature = "bb-cli")))]
pub(super) fn default_prover() -> Arc<dyn PrivacyProver> {
    Arc::new(UnavailablePrivacyProver)
}

#[cfg(not(any(feature = "bb-bindings", feature = "bb-cli")))]
struct UnavailablePrivacyProver;

#[cfg(not(any(feature = "bb-bindings", feature = "bb-cli")))]
#[async_trait]
impl PrivacyProver for UnavailablePrivacyProver {
    async fn prove(
        &self,
        _request: ProveRequest,
    ) -> payy_evm_client_prover_interface::Result<ProveOutput> {
        Err(payy_evm_client_prover_interface::Error::Internal(
            std::io::Error::other("enable payy-evm-client feature `bb-bindings` or `bb-cli`")
                .context("select privacy prover backend")
                .into(),
        ))
    }
}
