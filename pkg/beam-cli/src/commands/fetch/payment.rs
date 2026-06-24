// lint-long-file-override allow-max-lines=300
mod approval;
mod chain_match;
mod execute;
mod prepare;
mod private;
mod resolve;
mod selection;

use contracts::{Address, Client, U256};
use serde_json::Value;

use crate::{
    chains::BeamChains,
    error::{Error, Result},
    evm::{TransactionGasPolicy, format_units, parse_units},
    human_output::sanitize_control_chars,
    privacy_config::PrivacyProfile,
};

#[cfg(test)]
pub(crate) use self::approval::approve_payment_with;
pub(crate) use self::{
    approval::approve_payment,
    execute::execute_payment,
    prepare::{prepare_mpp_payment, prepare_x402_payment},
};

#[derive(Clone, Debug)]
pub(crate) struct PreparedPayment {
    pub accepted: Value,
    pub amount: U256,
    pub amount_display: String,
    pub asset: PaymentAsset,
    pub asset_id: String,
    pub chain: PaymentChain,
    pub client: Client,
    pub description: Option<String>,
    pub gas: GasEstimate,
    pub network: String,
    pub payer: Address,
    pub private_recipient: Option<String>,
    pub recipient: Address,
    pub selected_chain: Option<PaymentChain>,
    pub scheme: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ExecutedPayment {
    pub accepted: Value,
    pub network: String,
    pub proof: Value,
    pub scheme: String,
    pub source: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PaymentChain {
    pub aliases: Vec<String>,
    pub chain_id: u64,
    pub display_name: String,
    pub key: String,
    pub native_symbol: String,
    pub privacy: Option<PrivacyProfile>,
}

#[derive(Clone, Debug)]
pub(crate) struct GasEstimate {
    pub fee: U256,
    pub gas_limit: U256,
    pub gas_price: U256,
}

#[derive(Clone, Debug)]
pub(crate) struct PaymentAsset {
    pub decimals: u8,
    pub kind: PaymentAssetKind,
    pub label: String,
}

#[derive(Clone, Debug)]
pub(crate) enum PaymentAssetKind {
    Erc20(Address),
    Native,
}

impl PaymentChain {
    pub(crate) fn matches_selector(&self, selector: &str, chain_store: &BeamChains) -> bool {
        chain_match::payment_chain_matches_selector(self, selector, chain_store)
    }

    pub(crate) fn summary(&self) -> String {
        format!("{} ({})", self.display_name, self.chain_id)
    }
}

impl PreparedPayment {
    pub(crate) fn ensure_max_fee_allows(&self, max_fee: &str) -> Result<()> {
        let gas_threshold = parse_units(max_fee, 18)?;
        if self.gas.fee > gas_threshold {
            return Err(Error::FetchPaymentExceedsMaxFee);
        }

        match &self.asset.kind {
            PaymentAssetKind::Erc20(_) if self.private_recipient.is_some() => {
                let asset_threshold = parse_units(max_fee, usize::from(self.asset.decimals))?;
                if self.amount > asset_threshold {
                    return Err(Error::FetchPaymentExceedsMaxFee);
                }
            }
            PaymentAssetKind::Native => {
                if self.amount.saturating_add(self.gas.fee) > gas_threshold {
                    return Err(Error::FetchPaymentExceedsMaxFee);
                }
            }
            PaymentAssetKind::Erc20(_) => {
                let asset_threshold = parse_units(max_fee, usize::from(self.asset.decimals))?;
                if self.amount > asset_threshold {
                    return Err(Error::FetchPaymentExceedsMaxFee);
                }
            }
        }

        Ok(())
    }

    pub(super) fn transaction_gas(&self) -> TransactionGasPolicy {
        TransactionGasPolicy {
            gas_limit: Some(self.gas.gas_limit),
            max_network_fee: Some(self.gas.fee),
        }
    }

    pub(crate) fn confirmation_message(&self, protocol: &str) -> String {
        let mut lines = vec![
            format!("Payment required via {protocol}"),
            format!(
                "Amount: {} {}",
                self.amount_display,
                sanitize_control_chars(&self.asset.label)
            ),
            format!("Recipient: {:#x}", self.recipient),
            format!(
                "Network: {} ({})",
                sanitize_control_chars(&self.chain.display_name),
                self.chain.chain_id
            ),
            format!(
                "Estimated gas: {} {} (limit {}, price {})",
                format_units(self.gas.fee, 18),
                sanitize_control_chars(&self.chain.native_symbol),
                self.gas.gas_limit,
                self.gas.gas_price,
            ),
        ];

        if let Some(private_recipient) = self.private_recipient.as_ref() {
            lines[2] = format!(
                "Private recipient: {}",
                sanitize_control_chars(private_recipient)
            );
        }

        if let Some(description) = self.description.as_ref() {
            lines.push(format!(
                "Description: {}",
                sanitize_control_chars(description)
            ));
        }

        lines.join("\n")
    }
}
