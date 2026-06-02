use serde::{Deserialize, Serialize};

use crate::apps::{Error, Result, model::PrivacyCapability};

#[expect(
    dead_code,
    reason = "privacy host API shapes are reserved before wiring"
)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "operation", rename_all = "kebab-case")]
pub enum PrivacyHostRequest {
    PrivateAddress,
    PrivateBalance {
        token: String,
    },
    PrivateTransfer {
        recipient: String,
        token: String,
        amount: String,
    },
    MintProof {
        token: String,
        amount: String,
    },
    BurnProof {
        token: String,
        amount: String,
        recipient: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum PrivacyHostResponse {
    Unsupported { capability: PrivacyCapability },
}

#[allow(
    dead_code,
    reason = "privacy host API shapes are reserved before wiring"
)]
pub fn reject_unsupported(capability: PrivacyCapability) -> Result<PrivacyHostResponse> {
    Err(Error::UnsupportedPrivacyCapability { capability })
}
