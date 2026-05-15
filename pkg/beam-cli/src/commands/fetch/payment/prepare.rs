// lint-long-file-override allow-max-lines=300
#[path = "prepare_support.rs"]
mod prepare_support;

use contracts::{Address, U256};
use serde_json::Value;

use crate::{
    cli::FetchArgs,
    error::{Error, Result},
    evm::{erc20_balance, format_units, native_balance},
    privacy::parse_privacy_address,
    runtime::{BeamApp, parse_address},
};

use super::{
    PaymentAssetKind, PreparedPayment,
    chain_match::{
        summarize_x402_offer_chains, x402_offer_matches_payment_chain, x402_offer_matches_selector,
    },
    resolve::{
        chain_id_from_network, estimate_payment_gas, resolve_payment_asset, resolve_payment_chain,
    },
    selection::selected_payment_chain,
};
use crate::commands::fetch::protocol::{AmountValue, MppChallenge, X402Challenge, X402Offer};

use self::prepare_support::{ensure_mpp_chain_matches_override, parse_payment_amount};

pub(crate) async fn prepare_x402_payment(
    app: &BeamApp,
    args: &FetchArgs,
    challenge: &X402Challenge,
) -> Result<PreparedPayment> {
    let selected_chain = selected_payment_chain(app).await?;
    let offers = prioritized_x402_offers(app, args, challenge, selected_chain.as_ref()).await?;
    let mut last_error = None;
    let mut had_max_fee_exceeded = false;
    let mut had_insufficient_balance = false;

    for offer in offers {
        match prepare_offer_payment(app, args, offer).await {
            Ok(payment) => {
                if let Some(max_fee) = args.max_fee.as_ref() {
                    match payment.ensure_max_fee_allows(max_fee) {
                        Ok(()) => {}
                        Err(Error::FetchPaymentExceedsMaxFee) => {
                            had_max_fee_exceeded = true;
                            continue;
                        }
                        Err(err) => return Err(err),
                    }
                }

                if payment_has_sufficient_balance(&payment).await? {
                    return Ok(payment);
                }

                had_insufficient_balance = true;
            }
            Err(err) => last_error = Some(err),
        }
    }

    if had_insufficient_balance {
        return Err(Error::FetchPaymentInsufficientBalance);
    }

    if had_max_fee_exceeded {
        return Err(Error::FetchPaymentExceedsMaxFee);
    }

    Err(last_error.unwrap_or(Error::FetchInvalidPaymentResponse))
}

pub(crate) async fn prepare_mpp_payment(
    app: &BeamApp,
    challenge: &MppChallenge,
) -> Result<PreparedPayment> {
    let auth = challenge
        .auth
        .as_ref()
        .ok_or(Error::FetchInvalidPaymentResponse)?;

    if challenge.request.chain_id.is_none()
        && app.overrides.chain.is_none()
        && app.overrides.rpc.is_none()
    {
        return Err(Error::FetchPaymentChainRequired);
    }

    ensure_mpp_chain_matches_override(app, challenge.request.chain_id).await?;

    let network = challenge
        .request
        .chain_id
        .map(|chain_id| format!("eip155:{chain_id}"));
    let description = challenge
        .request
        .description
        .clone()
        .or_else(|| challenge.problem.detail.clone());
    let scheme = auth.method.clone();

    prepare_payment(
        app,
        PaymentRequest {
            accepted: Value::Null,
            amount: &challenge.request.amount,
            asset_id: &challenge.request.currency,
            chain_hint: None,
            chain_id_hint: challenge.request.chain_id,
            description,
            network,
            private_recipient: challenge.request.private_address.as_deref(),
            private_required: false,
            recipient: &challenge.request.recipient,
            scheme,
        },
    )
    .await
}

async fn prepare_payment(app: &BeamApp, request: PaymentRequest<'_>) -> Result<PreparedPayment> {
    let wallet = app.active_wallet().await?;
    let payer = parse_address(&wallet.address).map_err(|_| Error::FetchInvalidPaymentResponse)?;
    let (chain, client) =
        resolve_payment_chain(app, request.chain_hint, request.chain_id_hint).await?;
    let selected_chain = selected_payment_chain(app).await?;
    let asset = resolve_payment_asset(app, &client, &chain, request.asset_id).await?;
    let amount = parse_payment_amount(request.amount, asset.decimals)?;
    let private_recipient = request.private_recipient.map(ToString::to_string);
    if request.private_required && private_recipient.is_none() {
        return Err(Error::FetchInvalidPaymentResponse);
    }
    if let Some(private_recipient) = private_recipient.as_ref() {
        let _ = parse_privacy_address(private_recipient)?;
        if matches!(asset.kind, PaymentAssetKind::Native) {
            return Err(Error::FetchInvalidPaymentResponse);
        }
    }
    let recipient = if private_recipient.is_some() {
        Address::zero()
    } else {
        parse_address(request.recipient).map_err(|_| Error::FetchInvalidPaymentResponse)?
    };
    let gas = if private_recipient.is_some() {
        super::GasEstimate {
            fee: U256::zero(),
            gas_limit: U256::zero(),
            gas_price: U256::zero(),
        }
    } else {
        estimate_payment_gas(&client, payer, recipient, amount, &asset).await?
    };

    Ok(PreparedPayment {
        accepted: request.accepted,
        amount,
        amount_display: format_units(amount, asset.decimals),
        asset,
        asset_id: request.asset_id.to_string(),
        network: request
            .network
            .unwrap_or_else(|| format!("eip155:{}", chain.chain_id)),
        chain,
        client,
        description: request.description,
        gas,
        payer,
        private_recipient,
        recipient,
        selected_chain,
        scheme: request.scheme,
    })
}

async fn prepare_offer_payment(
    app: &BeamApp,
    args: &FetchArgs,
    offer: &X402Offer,
) -> Result<PreparedPayment> {
    prepare_payment(
        app,
        PaymentRequest {
            accepted: offer.raw.clone(),
            amount: &offer.amount,
            asset_id: &offer.asset,
            chain_hint: Some(&offer.network),
            chain_id_hint: chain_id_from_network(&offer.network),
            description: None,
            network: Some(offer.network.clone()),
            private_recipient: offer.private_address.as_deref(),
            private_required: args.private_payment,
            recipient: &offer.pay_to,
            scheme: offer.scheme.clone(),
        },
    )
    .await
}

async fn prioritized_x402_offers<'a>(
    app: &BeamApp,
    args: &FetchArgs,
    challenge: &'a X402Challenge,
    selected_chain: Option<&super::PaymentChain>,
) -> Result<Vec<&'a X402Offer>> {
    let chain_store = app.chain_store.get().await;
    let mut offers = challenge.offers.iter().collect::<Vec<_>>();
    let selector = app.overrides.chain.as_deref();
    let allowlist_active = selector.is_none() && !args.allowed_chains.is_empty();

    if let Some(selector) = selector {
        offers.retain(|offer| x402_offer_matches_selector(offer, selector, &chain_store));
    } else if allowlist_active {
        offers.retain(|offer| {
            args.allowed_chains
                .iter()
                .any(|allowed| x402_offer_matches_selector(offer, allowed, &chain_store))
        });
    }

    if offers.is_empty() {
        let challenge = (!challenge.offers.is_empty())
            .then(|| summarize_x402_offer_chains(&challenge.offers, &chain_store))
            .ok_or(Error::FetchInvalidPaymentResponse)?;
        return if let Some(selector) = selector {
            Err(Error::FetchPaymentChainMismatch {
                challenge,
                selected: selected_chain.map_or_else(|| selector.into(), |chain| chain.summary()),
            })
        } else if allowlist_active {
            Err(Error::FetchPaymentChainNotAllowed { chain: challenge })
        } else {
            Err(Error::FetchInvalidPaymentResponse)
        };
    }

    if selector.is_none()
        && let Some(selected_chain) = selected_chain
    {
        let (mut preferred, mut remaining): (Vec<_>, Vec<_>) =
            offers.into_iter().partition(|offer| {
                x402_offer_matches_payment_chain(offer, selected_chain, &chain_store)
            });
        preferred.append(&mut remaining);
        offers = preferred;
    }

    Ok(offers)
}

async fn payment_has_sufficient_balance(payment: &PreparedPayment) -> Result<bool> {
    if payment.private_recipient.is_some() {
        return Ok(true);
    }

    match payment.asset.kind {
        PaymentAssetKind::Native => {
            let balance = native_balance(&payment.client, payment.payer).await?;
            Ok(balance >= payment.amount.saturating_add(payment.gas.fee))
        }
        PaymentAssetKind::Erc20(token) => {
            let native = native_balance(&payment.client, payment.payer).await?;
            if native < payment.gas.fee {
                return Ok(false);
            }

            let token_balance = erc20_balance(&payment.client, token, payment.payer).await?;
            Ok(token_balance >= payment.amount)
        }
    }
}

struct PaymentRequest<'a> {
    accepted: Value,
    amount: &'a AmountValue,
    asset_id: &'a str,
    chain_hint: Option<&'a str>,
    chain_id_hint: Option<u64>,
    description: Option<String>,
    network: Option<String>,
    private_recipient: Option<&'a str>,
    private_required: bool,
    recipient: &'a str,
    scheme: String,
}
