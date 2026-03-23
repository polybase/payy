use client_http::HttpBody;
use guild_interface::eip7702 as iface;

use crate::GuildClientHttp;

/// EIP-7702 error
pub type Error = client_http::Error<client_http::NoRpcError>;

impl GuildClientHttp {
    /// Relay a SetCode (type 0x04) transaction using a signed authorization
    pub async fn eip7702_relay_upgrade(
        &self,
        request: &iface::RelayUpgradeInput,
    ) -> Result<iface::RelayTxOutput, Error> {
        self.http_client
            .post(
                "/crypto/eip7702/relay-upgrade",
                Some(HttpBody::json(request.clone())),
            )
            .exec::<client_http::NoRpcError>()
            .await?
            .to_value::<iface::RelayTxOutput, client_http::NoRpcError>()
            .await
    }

    /// Relay a meta-transaction (executeMeta)
    pub async fn eip7702_relay_meta(
        &self,
        request: &iface::RelayMetaInput,
    ) -> Result<iface::RelayTxOutput, Error> {
        self.http_client
            .post(
                "/crypto/eip7702/relay-meta",
                Some(HttpBody::json(request.clone())),
            )
            .exec::<client_http::NoRpcError>()
            .await?
            .to_value::<iface::RelayTxOutput, client_http::NoRpcError>()
            .await
    }
}
