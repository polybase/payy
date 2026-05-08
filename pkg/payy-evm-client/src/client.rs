// lint-long-file-override allow-max-lines=300
mod namespace;
mod prover;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use payy_evm_client_interface::{
    B256, Error, PayyEvmReadClient, PayyEvmSubmitter, PayyNetworkConfig,
    PayyRawTransactionSubmitter, PrivacyAddress, PrivacySigner, Result,
};
use payy_evm_client_prover_interface::PrivacyProver;

pub use namespace::PrivacyNamespace;
use prover::default_prover;

use crate::bridge::BridgeClient;
use crate::links::LinksClient;
use crate::local_signer::LocalPrivacySigner;
use crate::raw_submitter::LocalRawEvmSubmitter;

pub(crate) type CacheKey = (PrivacyAddress, payy_evm_client_interface::Address);

/// Shared client internals.
pub(crate) struct ClientInner {
    pub(crate) network: PayyNetworkConfig,
    pub(crate) read_client: Arc<dyn PayyEvmReadClient>,
    pub(crate) privacy_signer: Option<Arc<dyn PrivacySigner>>,
    pub(crate) evm_submitter: Option<Arc<dyn PayyEvmSubmitter>>,
    pub(crate) raw_submitter: Option<Arc<dyn PayyRawTransactionSubmitter>>,
    pub(crate) prover: Arc<dyn PrivacyProver>,
    pub(crate) checkpoints:
        Arc<Mutex<BTreeMap<CacheKey, payy_evm_client_interface::OwnedNoteState>>>,
}

impl ClientInner {
    pub(crate) async fn validate_read_chain(&self) -> Result<()> {
        let chain_id = self.read_client.get_chain_id().await?;
        if chain_id != self.network.chain_id {
            return Err(Error::ChainIdMismatch {
                expected: self.network.chain_id,
                actual: chain_id,
            });
        }
        Ok(())
    }

    pub(crate) fn privacy_signer(&self) -> Result<Arc<dyn PrivacySigner>> {
        self.privacy_signer.clone().ok_or(Error::MissingCapability {
            capability: "privacy_signer",
        })
    }

    pub(crate) fn evm_submitter(&self) -> Result<Arc<dyn PayyEvmSubmitter>> {
        self.evm_submitter.clone().ok_or(Error::MissingCapability {
            capability: "evm_submitter",
        })
    }
}

/// Builder for base Payy client instances.
pub struct PayyClientBuilder {
    network: PayyNetworkConfig,
    read_client: Arc<dyn PayyEvmReadClient>,
    raw_submitter: Option<Arc<dyn PayyRawTransactionSubmitter>>,
    prover: Option<Arc<dyn PrivacyProver>>,
}

impl PayyClientBuilder {
    /// Create a new builder.
    #[must_use]
    pub fn new(network: PayyNetworkConfig, read_client: Arc<dyn PayyEvmReadClient>) -> Self {
        Self {
            network,
            read_client,
            raw_submitter: None,
            prover: None,
        }
    }

    /// Configure a raw transaction submitter for local EVM signing.
    #[must_use]
    pub fn raw_transaction_submitter(
        mut self,
        raw_submitter: Arc<dyn PayyRawTransactionSubmitter>,
    ) -> Self {
        self.raw_submitter = Some(raw_submitter);
        self
    }

    /// Configure the privacy prover implementation.
    #[must_use]
    pub fn prover(mut self, prover: Arc<dyn PrivacyProver>) -> Self {
        self.prover = Some(prover);
        self
    }

    /// Build a base client.
    #[must_use]
    pub fn build(self) -> BaseClient {
        let prover = self.prover.unwrap_or_else(default_prover);
        BaseClient {
            inner: Arc::new(ClientInner {
                network: self.network,
                read_client: self.read_client,
                privacy_signer: None,
                evm_submitter: None,
                raw_submitter: self.raw_submitter,
                prover,
                checkpoints: Arc::new(Mutex::new(BTreeMap::new())),
            }),
        }
    }
}

/// Base read-only client.
#[derive(Clone)]
pub struct BaseClient {
    pub(crate) inner: Arc<ClientInner>,
}

impl BaseClient {
    /// Start building a client.
    #[must_use]
    pub fn builder(
        network: PayyNetworkConfig,
        read_client: Arc<dyn PayyEvmReadClient>,
    ) -> PayyClientBuilder {
        PayyClientBuilder::new(network, read_client)
    }

    /// Raw bridge reads.
    #[must_use]
    pub fn bridge(&self) -> BridgeClient {
        BridgeClient::new(self.inner.clone())
    }

    /// Claim-link parsing helpers.
    #[must_use]
    pub fn links(&self) -> LinksClient {
        LinksClient
    }

    /// Reserved transactions namespace.
    #[must_use]
    pub const fn transactions(&self) -> TransactionsClient {
        TransactionsClient
    }

    /// Attach delegated privacy signer capability.
    #[must_use]
    pub fn privacy_signer(self, signer: Arc<dyn PrivacySigner>) -> PrivacyClient {
        PrivacyClient::from_parts(&self.inner, Some(signer), None)
    }

    /// Attach delegated EVM submitter capability.
    #[must_use]
    pub fn evm_signer(self, submitter: Arc<dyn PayyEvmSubmitter>) -> EvmClient {
        EvmClient::from_parts(&self.inner, None, Some(submitter))
    }

    /// Local convenience path from one EVM private key.
    pub fn with_evm_private_key(self, evm_private_key: B256) -> Result<PrivacyClient> {
        let signer = Arc::new(LocalPrivacySigner::from_evm_private_key(evm_private_key)?);
        let evm_submitter = self.inner.raw_submitter.clone().map(|raw_submitter| {
            Arc::new(LocalRawEvmSubmitter::new(evm_private_key, raw_submitter))
                as Arc<dyn PayyEvmSubmitter>
        });
        Ok(PrivacyClient::from_parts(
            &self.inner,
            Some(signer),
            evm_submitter,
        ))
    }

    /// Explicit alias for callers that want the key type in the method name.
    pub fn with_secp256k1_private_key(self, evm_private_key: B256) -> Result<PrivacyClient> {
        self.with_evm_private_key(evm_private_key)
    }

    /// Configure a local privacy signer from an already-derived Grumpkin private key.
    pub fn with_grumpkin_private_key(self, grumpkin_private_key: B256) -> Result<PrivacyClient> {
        let signer = Arc::new(LocalPrivacySigner::from_grumpkin_private_key(
            grumpkin_private_key,
        )?);
        Ok(PrivacyClient::from_parts(&self.inner, Some(signer), None))
    }

    /// Validate the read adapter is connected to the configured network.
    pub async fn validate_read_chain(&self) -> Result<()> {
        self.inner.validate_read_chain().await
    }
}

/// Privacy-capable client.
#[derive(Clone)]
pub struct PrivacyClient {
    pub(crate) inner: Arc<ClientInner>,
}

impl PrivacyClient {
    pub(crate) fn from_parts(
        inner: &Arc<ClientInner>,
        privacy_signer: Option<Arc<dyn PrivacySigner>>,
        evm_submitter: Option<Arc<dyn PayyEvmSubmitter>>,
    ) -> Self {
        let inner = Arc::new(ClientInner {
            network: inner.network,
            read_client: inner.read_client.clone(),
            privacy_signer: privacy_signer.or_else(|| inner.privacy_signer.clone()),
            evm_submitter: evm_submitter.or_else(|| inner.evm_submitter.clone()),
            raw_submitter: inner.raw_submitter.clone(),
            prover: inner.prover.clone(),
            checkpoints: inner.checkpoints.clone(),
        });
        Self { inner }
    }

    /// Attach delegated EVM submitter capability.
    #[must_use]
    pub fn evm_signer(self, submitter: Arc<dyn PayyEvmSubmitter>) -> Self {
        Self::from_parts(&self.inner, None, Some(submitter))
    }

    /// Privacy namespace.
    #[must_use]
    pub fn privacy(&self) -> PrivacyNamespace {
        PrivacyNamespace::from_inner(&self.inner)
    }
}

/// EVM-submit-capable client.
#[derive(Clone)]
pub struct EvmClient {
    pub(crate) inner: Arc<ClientInner>,
}

impl EvmClient {
    pub(crate) fn from_parts(
        inner: &Arc<ClientInner>,
        privacy_signer: Option<Arc<dyn PrivacySigner>>,
        evm_submitter: Option<Arc<dyn PayyEvmSubmitter>>,
    ) -> Self {
        let inner = Arc::new(ClientInner {
            network: inner.network,
            read_client: inner.read_client.clone(),
            privacy_signer: privacy_signer.or_else(|| inner.privacy_signer.clone()),
            evm_submitter: evm_submitter.or_else(|| inner.evm_submitter.clone()),
            raw_submitter: inner.raw_submitter.clone(),
            prover: inner.prover.clone(),
            checkpoints: inner.checkpoints.clone(),
        });
        Self { inner }
    }

    /// Attach delegated privacy signer capability.
    #[must_use]
    pub fn privacy_signer(self, signer: Arc<dyn PrivacySigner>) -> PrivacyClient {
        PrivacyClient::from_parts(&self.inner, Some(signer), None)
    }
}

/// Reserved future transactions namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionsClient;
