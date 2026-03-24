/// Guild client error
pub type Error = client_http::Error<guild_interface::Error>;

/// Guild client result
pub type Result<R> = std::result::Result<R, Error>;
