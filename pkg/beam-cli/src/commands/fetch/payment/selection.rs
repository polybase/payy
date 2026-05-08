use crate::{chains::find_chain, error::Result, runtime::BeamApp};

use super::{PaymentChain, resolve::payment_chain_from_entry};

pub(super) async fn selected_payment_chain(app: &BeamApp) -> Result<Option<PaymentChain>> {
    if app.overrides.rpc.is_some() && app.overrides.chain.is_none() {
        return Ok(None);
    }

    let config = app.config_store.get().await;
    let selection = app
        .overrides
        .chain
        .clone()
        .unwrap_or_else(|| config.default_chain.clone());
    let chain_store = app.chain_store.get().await;
    let entry = find_chain(&selection, &chain_store)?;

    Ok(Some(payment_chain_from_entry(&entry, entry.chain_id)))
}
