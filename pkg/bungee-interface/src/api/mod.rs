use async_trait::async_trait;

/// Endpoint-specific request and response DTOs for the upstream API.
pub mod endpoints;
mod error;
mod response;

pub use error::TransportError;
pub use response::{Response, ResponseHeaders};

/// Typed calque trait for the upstream Bungee v1 REST API.
#[unimock::unimock(api = BungeeV1ApiMock)]
#[async_trait]
pub trait BungeeV1Api: Send + Sync + 'static {
    /// Call `GET /api/v1/bungee/quote`.
    async fn get_api_v1_bungee_quote(
        &self,
        headers: Option<endpoints::quote::Headers>,
        query: Option<endpoints::quote::Query>,
    ) -> Result<Response<endpoints::quote::ResponseEnum>, TransportError>;

    /// Call `GET /api/v1/bungee/build-tx`.
    async fn get_api_v1_bungee_build_tx(
        &self,
        headers: Option<endpoints::build_tx::Headers>,
        query: Option<endpoints::build_tx::Query>,
    ) -> Result<Response<endpoints::build_tx::ResponseEnum>, TransportError>;

    /// Call `GET /api/v1/tokens/list`.
    async fn get_api_v1_tokens_list(
        &self,
        headers: Option<endpoints::tokens_list::Headers>,
        query: Option<endpoints::tokens_list::Query>,
    ) -> Result<Response<endpoints::tokens_list::ResponseEnum>, TransportError>;

    /// Call `GET /api/v1/bungee/status`.
    async fn get_api_v1_bungee_status(
        &self,
        headers: Option<endpoints::status::Headers>,
        query: Option<endpoints::status::Query>,
    ) -> Result<Response<endpoints::status::ResponseEnum>, TransportError>;
}

/// Typed calque trait for the Socket Swap v3 REST API.
#[unimock::unimock(api = SocketSwapV3ApiMock)]
#[async_trait]
pub trait SocketSwapV3Api: Send + Sync + 'static {
    /// Call `GET /v3/swap/quote`.
    async fn get_v3_swap_quote(
        &self,
        headers: Option<endpoints::socket_swap_quote::Headers>,
        query: Option<endpoints::socket_swap_quote::Query>,
    ) -> Result<Response<endpoints::socket_swap_quote::ResponseEnum>, TransportError>;

    /// Call `GET /v3/swap/tokens/list`.
    async fn get_v3_swap_tokens_list(
        &self,
        headers: Option<endpoints::socket_swap_tokens_list::Headers>,
        query: Option<endpoints::socket_swap_tokens_list::Query>,
    ) -> Result<Response<endpoints::socket_swap_tokens_list::ResponseEnum>, TransportError>;

    /// Call `GET /v3/swap/status`.
    async fn get_v3_swap_status(
        &self,
        headers: Option<endpoints::socket_swap_status::Headers>,
        query: Option<endpoints::socket_swap_status::Query>,
    ) -> Result<Response<endpoints::socket_swap_status::ResponseEnum>, TransportError>;
}
