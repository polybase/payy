/// Responsible for mapping all errors to the appropriate RPC/Http error codes,
/// so that the error responses provided by the server RPC are useful to a client.
///
/// Any errors you expect to send to the user via RPC should be defined here, all other
/// errors will be internal errors.
mod http_error;
pub mod routes;
pub mod server;
pub mod stats;
pub mod stats_today;
