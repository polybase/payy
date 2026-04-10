use std::collections::BTreeSet;

use crate::{
    chains::{BeamChains, ChainEntry, find_chain},
    commands::fetch::protocol::X402Offer,
};

use super::{PaymentChain, resolve::chain_id_from_network};

pub(super) fn payment_chain_matches_selector(
    chain: &PaymentChain,
    selector: &str,
    chain_store: &BeamChains,
) -> bool {
    selector_matches_chain(
        selector,
        chain_store,
        Some(chain.chain_id),
        Some(chain.key.as_str()),
        Some(chain.display_name.as_str()),
        chain.aliases.iter().map(String::as_str),
    )
}

pub(super) fn x402_offer_matches_selector(
    offer: &X402Offer,
    selector: &str,
    chain_store: &BeamChains,
) -> bool {
    if let Some(chain) = resolved_x402_offer_chain(offer, chain_store) {
        return selector_matches_chain(
            selector,
            chain_store,
            Some(chain.chain_id),
            Some(chain.key.as_str()),
            Some(chain.display_name.as_str()),
            chain.aliases.iter().map(String::as_str),
        );
    }

    selector_matches_chain(
        selector,
        chain_store,
        chain_id_from_network(&offer.network),
        Some(offer.network.as_str()),
        None,
        std::iter::empty(),
    )
}

pub(super) fn x402_offer_matches_payment_chain(
    offer: &X402Offer,
    chain: &PaymentChain,
    chain_store: &BeamChains,
) -> bool {
    if let Some(resolved_chain) = resolved_x402_offer_chain(offer, chain_store) {
        return resolved_chain.chain_id == chain.chain_id;
    }

    let network = offer.network.trim();
    chain_id_from_network(network) == Some(chain.chain_id)
        || network.eq_ignore_ascii_case(&chain.key)
        || network.eq_ignore_ascii_case(&chain.display_name)
        || chain
            .aliases
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(network))
}

pub(super) fn summarize_x402_offer_chains(
    offers: &[X402Offer],
    chain_store: &BeamChains,
) -> String {
    offers
        .iter()
        .filter_map(|offer| x402_offer_chain_summary(offer, chain_store))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ")
}

fn selector_matches_chain<'a>(
    selector: &str,
    chain_store: &BeamChains,
    chain_id: Option<u64>,
    key: Option<&str>,
    display_name: Option<&str>,
    aliases: impl IntoIterator<Item = &'a str>,
) -> bool {
    let selector = selector.trim();
    if selector.is_empty() {
        return false;
    }

    if let Ok(desired_chain) = find_chain(selector, chain_store) {
        return chain_id == Some(desired_chain.chain_id)
            || key.is_some_and(|value| value.eq_ignore_ascii_case(&desired_chain.key));
    }

    chain_id.is_some_and(|value| selector == value.to_string())
        || key.is_some_and(|value| value.eq_ignore_ascii_case(selector))
        || display_name.is_some_and(|value| value.eq_ignore_ascii_case(selector))
        || aliases
            .into_iter()
            .any(|alias| alias.eq_ignore_ascii_case(selector))
}

fn resolved_x402_offer_chain(offer: &X402Offer, chain_store: &BeamChains) -> Option<ChainEntry> {
    chain_id_from_network(&offer.network)
        .map(|chain_id| chain_id.to_string())
        .and_then(|selector| find_chain(&selector, chain_store).ok())
        .or_else(|| find_chain(&offer.network, chain_store).ok())
}

fn x402_offer_chain_summary(offer: &X402Offer, chain_store: &BeamChains) -> Option<String> {
    if let Some(chain_id) = chain_id_from_network(&offer.network) {
        return Some(payment_chain_summary(chain_id, chain_store));
    }

    if let Ok(chain) = find_chain(&offer.network, chain_store) {
        return Some(format!("{} ({})", chain.display_name, chain.chain_id));
    }

    let network = offer.network.trim();
    (!network.is_empty()).then(|| network.to_string())
}

pub(super) fn payment_chain_summary(chain_id: u64, chain_store: &BeamChains) -> String {
    find_chain(&chain_id.to_string(), chain_store)
        .map(|chain| format!("{} ({})", chain.display_name, chain.chain_id))
        .unwrap_or_else(|_| chain_id.to_string())
}
