use serde::{Deserialize, Serialize};
use zk_primitives::SignatureProof;

/// Request auth with signature proof
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRequest {
    /// Proof to verify the address of the user
    pub proof: SignatureProof,
}

/// Response for auth request
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Guild JWT
    pub guild: String,
}
