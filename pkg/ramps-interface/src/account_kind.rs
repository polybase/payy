use std::fmt;

use serde::{Deserialize, Serialize};

/// Identifiers describing how an account lookup was performed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum AccountKind {
    AccountId,
    WalletId,
    CardId,
    ExternalId,
    KycExternalId,
}

impl fmt::Display for AccountKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            AccountKind::AccountId => "account_id",
            AccountKind::WalletId => "wallet_id",
            AccountKind::CardId => "card_id",
            AccountKind::ExternalId => "external_id",
            AccountKind::KycExternalId => "kyc_external_id",
        };

        f.write_str(label)
    }
}
