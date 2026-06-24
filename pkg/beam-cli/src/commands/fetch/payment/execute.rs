use contracts::U256;
use serde_json::json;
use web3::ethabi::StateMutability;

use crate::{
    abi::parse_function,
    commands::{
        fetch::payment::{ExecutedPayment, PaymentAssetKind, PreparedPayment, private},
        signing::active_signer_for_intent,
    },
    error::{Error, Result},
    evm::{FunctionCall, send_function_with_gas, send_native_with_gas},
    output::with_loading_handle,
    profiles::{
        Error as ProfileError,
        model::{FetchPaymentIntent, PublicSigningIntent},
    },
    runtime::BeamApp,
    transaction::{TransactionExecution, loading_message},
};

pub(crate) async fn execute_payment(
    app: &BeamApp,
    payment: &PreparedPayment,
) -> Result<ExecutedPayment> {
    if payment.private_recipient.is_some() {
        if app.overrides.profile.is_some() {
            return Err(ProfileError::PrivacyCapabilityUnsupported {
                capability: "private-payment".to_string(),
            }
            .into());
        }
        return private::execute_private_payment(app, payment).await;
    }

    let asset = match payment.asset.kind.clone() {
        PaymentAssetKind::Native => "native".to_string(),
        PaymentAssetKind::Erc20(token) => format!("{token:#x}"),
    };
    let signer = active_signer_for_intent(
        app,
        PublicSigningIntent::FetchPayment(FetchPaymentIntent {
            wallet: format!("{:#x}", payment.payer),
            scheme: payment.scheme.clone(),
            origin: payment.network.clone(),
            chain: payment.chain.key.clone(),
            asset,
            recipient: format!("{:#x}", payment.recipient),
            amount: payment.amount.to_string(),
            private_payment: false,
        }),
    )
    .await?;
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
                        signer.as_ref(),
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
                        signer.as_ref(),
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
