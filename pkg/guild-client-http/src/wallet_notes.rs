use client_http::{HttpBody, NoRpcError};
use guild_interface::wallet_notes::{WalletNote, WalletNoteRequest};

use crate::GuildClientHttp;

/// Error for wallet note rpc
pub type Error = client_http::Error<NoRpcError>;

impl GuildClientHttp {
    /// Send wallet notes upsert request to Guild
    pub async fn upsert_wallet_notes(
        &self,
        wallet_notes: Vec<WalletNote>,
    ) -> Result<Vec<WalletNote>, Error> {
        self.http_client
            .post(
                "/wallets/me/notes",
                Some(HttpBody::json(WalletNoteRequest {
                    notes: wallet_notes,
                })),
            )
            .auth()
            .exec()
            .await?
            .to_value()
            .await
    }

    /// Lists ALL unspent wallet notes for the authenticated user
    pub async fn list_wallet_notes(&self) -> Result<Vec<WalletNote>, Error> {
        self.http_client
            .get("/wallets/me/notes")
            .auth()
            .exec()
            .await?
            .to_value()
            .await
    }
}
