/// Node client error
pub type Error = client_http::Error<node_interface::RpcError>;

/// Node client result
pub type Result<R> = std::result::Result<R, Error>;
