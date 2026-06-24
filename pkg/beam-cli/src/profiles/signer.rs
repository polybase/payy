use std::sync::Arc;

use async_trait::async_trait;
use contextful::ResultContextExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::Mutex,
};
use web3::{
    Web3,
    transports::Http,
    types::{SignedTransaction, TransactionParameters},
};

use crate::{
    error,
    profiles::{
        Error, Result,
        model::PublicSigningIntent,
        session::ProfileSession,
        wire::{DaemonRequest, DaemonResponse, WireTransaction},
    },
    signer::Signer,
};

#[derive(Clone)]
pub struct ProfileSigner {
    address: contracts::Address,
    intent: PublicSigningIntent,
    last_ledger_id: Arc<Mutex<Option<String>>>,
    session: ProfileSession,
}

impl ProfileSigner {
    pub fn new(session: ProfileSession, intent: PublicSigningIntent) -> Result<Self> {
        let address =
            session
                .wallet_address
                .parse()
                .map_err(|_| Error::InvalidSessionWalletAddress {
                    profile: session.profile.clone(),
                    address: session.wallet_address.clone(),
                })?;
        Ok(Self {
            address,
            intent,
            last_ledger_id: Arc::new(Mutex::new(None)),
            session,
        })
    }
}

#[async_trait]
impl Signer for ProfileSigner {
    fn address(&self) -> contracts::Address {
        self.address
    }

    async fn sign_transaction(
        &self,
        _client: &Web3<Http>,
        transaction: TransactionParameters,
    ) -> error::Result<SignedTransaction> {
        let response = request(
            &self.session,
            DaemonRequest::Sign {
                token: self.session.token.clone(),
                intent: Box::new(self.intent.clone()),
                transaction: Box::new(WireTransaction::from_parameters(&transaction)),
            },
        )
        .await?;

        let (ledger_id, transaction) = match response {
            DaemonResponse::Signed {
                ledger_id,
                transaction,
            } => (ledger_id, transaction),
            DaemonResponse::Error { message } => {
                return Err(Error::PolicyDenied { reason: message }.into());
            }
            DaemonResponse::Ok => return Err(Error::InvalidDaemonRequest.into()),
        };
        *self.last_ledger_id.lock().await = Some(ledger_id);
        Ok(transaction.to_signed()?)
    }

    async fn rollback_last_signature(&self) -> error::Result<()> {
        let Some(ledger_id) = self.last_ledger_id.lock().await.take() else {
            return Ok(());
        };
        let response = request(
            &self.session,
            DaemonRequest::Rollback {
                token: self.session.token.clone(),
                ledger_id,
            },
        )
        .await?;
        match response {
            DaemonResponse::Ok => Ok(()),
            DaemonResponse::Error { message } => {
                Err(Error::PolicyDenied { reason: message }.into())
            }
            DaemonResponse::Signed { .. } => Err(Error::InvalidDaemonRequest.into()),
        }
    }
}

pub async fn ping(session: &ProfileSession) -> Result<()> {
    match request(
        session,
        DaemonRequest::Ping {
            token: session.token.clone(),
        },
    )
    .await?
    {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message } => Err(Error::PolicyDenied { reason: message }),
        DaemonResponse::Signed { .. } => Err(Error::InvalidDaemonRequest),
    }
}

pub async fn lock(session: &ProfileSession) -> Result<()> {
    match request(
        session,
        DaemonRequest::Lock {
            token: session.token.clone(),
        },
    )
    .await?
    {
        DaemonResponse::Ok => Ok(()),
        DaemonResponse::Error { message } => Err(Error::PolicyDenied { reason: message }),
        DaemonResponse::Signed { .. } => Err(Error::InvalidDaemonRequest),
    }
}

async fn request(session: &ProfileSession, request: DaemonRequest) -> Result<DaemonResponse> {
    let mut stream =
        UnixStream::connect(&session.socket)
            .await
            .map_err(|_| Error::DaemonConnectionFailed {
                profile: session.profile.clone(),
            })?;
    let bytes = serde_json::to_vec(&request).context("encode profile daemon request")?;
    stream
        .write_all(&bytes)
        .await
        .context("write profile daemon request")?;
    stream
        .shutdown()
        .await
        .context("finish profile daemon request")?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .context("read profile daemon response")?;
    let response = serde_json::from_slice::<DaemonResponse>(&response)
        .context("decode profile daemon response")?;
    match response {
        DaemonResponse::Error { message } if message == "session-token-rejected" => {
            Err(Error::SessionTokenRejected)
        }
        other => Ok(other),
    }
}
