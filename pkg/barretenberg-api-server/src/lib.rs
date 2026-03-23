mod error;
mod extractors;
mod handlers;
pub mod server;

pub use barretenberg_api_interface::{ProveRequest, ProveResponse, VerifyRequest, VerifyResponse};
pub use server::build_app;
