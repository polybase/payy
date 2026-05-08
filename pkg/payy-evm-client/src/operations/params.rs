use payy_evm_client_interface::{Address, B256, EvmAccount, PrivacyAccount, PrivacyAddress};

/// Mint params.
#[derive(Debug, Clone)]
pub struct MintParams {
    /// Privacy account selector.
    pub privacy_account: PrivacyAccount,
    /// EVM funding account selector.
    pub evm_account: EvmAccount,
    /// Token address.
    pub token: Address,
    /// Mint amount.
    pub amount: element::Element,
}

/// Burn params.
#[derive(Debug, Clone)]
pub struct BurnParams {
    /// Privacy account selector.
    pub privacy_account: PrivacyAccount,
    /// Token address.
    pub token: Address,
    /// Burn amount.
    pub amount: element::Element,
    /// Recipient EVM address.
    pub evm_recipient: Address,
}

/// Direct send params.
#[derive(Debug, Clone)]
pub struct DirectSendParams {
    /// Sender privacy account selector.
    pub privacy_account: PrivacyAccount,
    /// Token address.
    pub token: Address,
    /// Send amount.
    pub amount: element::Element,
    /// Recipient privacy address.
    pub recipient: PrivacyAddress,
    /// Optional on-chain bridge memo.
    pub bridge_memo: Option<B256>,
}

/// Ephemeral send params.
#[derive(Debug, Clone)]
pub struct EphemeralSendParams {
    /// Sender privacy account selector.
    pub privacy_account: PrivacyAccount,
    /// Token address.
    pub token: Address,
    /// Send amount.
    pub amount: element::Element,
    /// Optional on-chain bridge memo.
    pub bridge_memo: Option<B256>,
}
