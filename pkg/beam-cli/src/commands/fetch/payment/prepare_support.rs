use contracts::U256;

use crate::{
    commands::fetch::protocol::AmountValue,
    error::{Error, Result},
    evm::parse_units,
    runtime::BeamApp,
};

use super::super::{chain_match::payment_chain_summary, selection::selected_payment_chain};

pub(super) async fn ensure_mpp_chain_matches_override(
    app: &BeamApp,
    challenge_chain_id: Option<u64>,
) -> Result<()> {
    if app.overrides.chain.is_none() {
        return Ok(());
    }

    let Some(challenge_chain_id) = challenge_chain_id else {
        return Ok(());
    };
    let Some(selected_chain) = selected_payment_chain(app).await? else {
        return Ok(());
    };
    if selected_chain.chain_id == challenge_chain_id {
        return Ok(());
    }

    let chain_store = app.chain_store.get().await;
    let challenge = payment_chain_summary(challenge_chain_id, &chain_store);

    Err(Error::FetchPaymentChainMismatch {
        challenge,
        selected: selected_chain.summary(),
    })
}

pub(super) fn parse_payment_amount(amount: &AmountValue, decimals: u8) -> Result<U256> {
    match amount {
        AmountValue::Atomic(value) => {
            U256::from_dec_str(value).map_err(|_| Error::FetchInvalidPaymentResponse)
        }
        AmountValue::Human(value) => parse_units(value, usize::from(decimals))
            .map_err(|_| Error::FetchInvalidPaymentResponse),
    }
}
