// lint-long-file-override allow-max-lines=300
mod approval;
mod chain_match;
mod prepare;
mod private;
mod resolve;
mod selection;

use contracts::{Address, Client, U256};
use serde_json::{Value, json};
use web3::ethabi::StateMutability;

use crate::{
    abi::parse_function,
    chains::BeamChains,
    commands::signing::prompt_active_signer,
    error::{Error, Result},
    evm::{
        FunctionCall, TransactionGasPolicy, format_units, parse_units, send_function_with_gas,
        send_native_with_gas,
    },
    human_output::sanitize_control_chars,
    output::with_loading_handle,
    privacy_config::PrivacyProfile,
    runtime::BeamApp,
    transaction::{TransactionExecution, loading_message},
};

#[cfg(test)]
pub(crate) use self::approval::approve_payment_with;
pub(crate) use self::{
    approval::approve_payment,
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

pub(crate) async fn execute_payment(
    app: &BeamApp,
    payment: &PreparedPayment,
) -> Result<ExecutedPayment> {
    if payment.private_recipient.is_some() {
        return private::execute_private_payment(app, payment).await;
    }

    let signer = prompt_active_signer(app).await?;
    let action = format!(
        "payment of {} {} to {:#x}",
        payment.amount_display, payment.asset.label, payment.recipient
    );
    let client = payment.client.clone();
    let recipient = payment.recipient;
    let amount = payment.amount;
    let gas = payment.transaction_gas();

    let execution = with_loading_handle(
        app.output_mode,
        format!("Sending {action} and waiting for confirmation..."),
        |loading| async move {
            match payment.asset.kind.clone() {
                PaymentAssetKind::Native => {
                    send_native_with_gas(
                        &client,
                        &signer,
                        recipient,
                        amount,
                        Some(gas),
                        move |update| loading.set_message(loading_message(&action, &update)),
                        tokio::signal::ctrl_c(),
                    )
                    .await
                }
                PaymentAssetKind::Erc20(token) => {
                    let function =
                        parse_function("transfer(address,uint256)", StateMutability::NonPayable)?;
                    let args = vec![format!("{recipient:#x}"), amount.to_string()];

                    send_function_with_gas(
                        &client,
                        &signer,
                        FunctionCall {
                            args: &args,
                            contract: token,
                            function: &function,
                            value: U256::zero(),
                        },
                        Some(gas),
                        move |update| loading.set_message(loading_message(&action, &update)),
                        tokio::signal::ctrl_c(),
                    )
                    .await
                }
            }
        },
    )
    .await?;

    let tx_hash = match execution {
        TransactionExecution::Confirmed(outcome) => outcome.tx_hash,
        TransactionExecution::Pending(pending) => {
            return Err(Error::FetchPaymentUnconfirmed {
                tx_hash: pending.tx_hash,
            });
        }
        TransactionExecution::Dropped(dropped) => {
            return Err(Error::FetchPaymentUnconfirmed {
                tx_hash: dropped.tx_hash,
            });
        }
    };

    Ok(ExecutedPayment {
        accepted: payment.accepted.clone(),
        network: payment.network.clone(),
        proof: json!({
            "amount": payment.amount.to_string(),
            "asset": payment.asset_id,
            "chainId": payment.chain.chain_id,
            "from": format!("{:#x}", payment.payer),
            "kind": "beam-evm-transfer",
            "network": payment.network,
            "to": format!("{:#x}", payment.recipient),
            "txHash": tx_hash,
        }),
        scheme: payment.scheme.clone(),
        source: Some(format!(
            "did:pkh:eip155:{}:{:#x}",
            payment.chain.chain_id, payment.payer
        )),
    })
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

    fn transaction_gas(&self) -> TransactionGasPolicy {
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
