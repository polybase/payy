// lint-long-file-override allow-max-lines=270
mod command;

use contracts::U256;

use crate::profiles::{
    Error, Result,
    ledger::{LedgerState, spent_for_grant},
    model::{AppActionIntent, AppGrant, ProfileRecord, PublicSigningIntent},
    store::now,
    wire::WireTransaction,
};

use command::command_grant_matches;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDecision {
    pub grant_id: String,
}

pub fn authorize(
    profile: &ProfileRecord,
    ledger: &LedgerState,
    intent: &PublicSigningIntent,
    transaction: &WireTransaction,
) -> Result<PolicyDecision> {
    ensure_wallet(profile, intent)?;
    ensure_final_transaction(intent, transaction)?;
    if !profile.policy.privacy.is_empty() {
        let capability = profile.policy.privacy[0].capability.clone();
        return Err(Error::PrivacyCapabilityUnsupported { capability });
    }

    match intent {
        PublicSigningIntent::AppActionPlan(intent) => profile
            .policy
            .app_grants
            .iter()
            .find(|grant| app_grant_matches(grant, intent, transaction, ledger))
            .map(|grant| PolicyDecision {
                grant_id: grant.id.clone(),
            })
            .ok_or_else(|| Error::PolicyDenied {
                reason: "no app grant matched final action plan".to_string(),
            }),
        _ => profile
            .policy
            .command_grants
            .iter()
            .find(|grant| command_grant_matches(grant, intent, transaction, ledger))
            .map(|grant| PolicyDecision {
                grant_id: grant.id.clone(),
            })
            .ok_or_else(|| Error::PolicyDenied {
                reason: "no command grant matched final transaction".to_string(),
            }),
    }
}

fn ensure_wallet(profile: &ProfileRecord, intent: &PublicSigningIntent) -> Result<()> {
    let wallet = match intent {
        PublicSigningIntent::NativeTransfer(intent) => &intent.wallet,
        PublicSigningIntent::Erc20Transfer(intent) => &intent.wallet,
        PublicSigningIntent::Erc20Approval(intent) => &intent.wallet,
        PublicSigningIntent::ContractTransaction(intent) => &intent.wallet,
        PublicSigningIntent::FetchPayment(intent) => &intent.wallet,
        PublicSigningIntent::AppActionPlan(intent) => &intent.wallet,
    };
    if wallet.eq_ignore_ascii_case(&profile.wallet_address) {
        Ok(())
    } else {
        Err(Error::PolicyDenied {
            reason: "wallet mismatch".to_string(),
        })
    }
}

fn ensure_final_transaction(
    intent: &PublicSigningIntent,
    transaction: &WireTransaction,
) -> Result<()> {
    let to = transaction.to.as_deref().unwrap_or_default();
    let value = parse_u256(&transaction.value)?;
    match intent {
        PublicSigningIntent::NativeTransfer(intent) => {
            ensure_eq_addr(to, &intent.recipient, "recipient")?;
            ensure_eq_amount(value, &intent.amount, "native value")?;
            ensure_empty_data(transaction)?;
        }
        PublicSigningIntent::Erc20Transfer(intent) => {
            ensure_eq_addr(to, &intent.token, "token")?;
            ensure_selector(transaction, "0xa9059cbb")?;
            ensure_eq_amount(value, "0", "native value")?;
        }
        PublicSigningIntent::Erc20Approval(intent) => {
            ensure_eq_addr(to, &intent.token, "token")?;
            ensure_selector(transaction, "0x095ea7b3")?;
            ensure_eq_amount(value, "0", "native value")?;
        }
        PublicSigningIntent::ContractTransaction(intent) => {
            ensure_eq_addr(to, &intent.target, "target")?;
            ensure_selector(transaction, &intent.selector)?;
            ensure_eq_amount(value, &intent.native_value, "native value")?;
        }
        PublicSigningIntent::FetchPayment(intent) => {
            if intent.private_payment {
                return Err(Error::PrivacyCapabilityUnsupported {
                    capability: "private-payment".to_string(),
                });
            }
        }
        PublicSigningIntent::AppActionPlan(intent) => {
            if intent.expires_at < now() {
                return Err(Error::PolicyDenied {
                    reason: "app action plan expired".to_string(),
                });
            }
        }
    }
    Ok(())
}

fn app_grant_matches(
    grant: &AppGrant,
    intent: &AppActionIntent,
    transaction: &WireTransaction,
    ledger: &LedgerState,
) -> bool {
    grant.app_id == intent.app_id
        && optional_eq(
            grant.registry_url.as_deref(),
            intent.registry_url.as_deref(),
        )
        && optional_eq(grant.app_version.as_deref(), Some(&intent.app_version))
        && optional_eq(
            grant.manifest_digest.as_deref(),
            Some(&intent.manifest_digest),
        )
        && optional_eq(grant.module_digest.as_deref(), Some(&intent.module_digest))
        && optional_eq(grant.command.as_deref(), Some(&intent.command))
        && optional_addr_eq(grant.wallet.as_deref(), Some(&intent.wallet))
        && optional_eq(grant.chain.as_deref(), Some(&intent.chain))
        && optional_eq(grant.plan_hash.as_deref(), Some(&intent.plan_hash))
        && grant.expires_at.is_none_or(|expiry| expiry >= now())
        && grant
            .bindings
            .iter()
            .all(|binding| intent.bindings.iter().any(|candidate| candidate == binding))
        && grant
            .constraints
            .iter()
            .all(|constraint| intent.constraints.contains(constraint))
        && (grant.steps.is_empty() || grant.steps == intent.steps)
        && amount_allows(grant.max_gas.as_deref(), Some(&transaction.gas))
        && budget_allows(
            grant.cumulative_budget.as_deref(),
            ledger,
            &grant.id,
            "app",
            Some(&transaction.value),
        )
}

pub(super) fn optional_eq(expected: Option<&str>, actual: Option<&str>) -> bool {
    expected.is_none_or(|expected| actual.is_some_and(|actual| actual == expected))
}

pub(super) fn optional_addr_eq(expected: Option<&str>, actual: Option<&str>) -> bool {
    expected
        .is_none_or(|expected| actual.is_some_and(|actual| expected.eq_ignore_ascii_case(actual)))
}

pub(super) fn optional_selector_eq(expected: Option<&str>, actual: Option<&str>) -> bool {
    expected.is_none_or(|expected| {
        actual.is_some_and(|actual| normalize_selector(expected) == normalize_selector(actual))
    })
}

pub(super) fn amount_allows(limit: Option<&str>, value: Option<&str>) -> bool {
    let Some(limit) = limit else {
        return true;
    };
    let Some(value) = value else {
        return true;
    };
    parse_u256(value).is_ok_and(|value| parse_u256(limit).is_ok_and(|limit| value <= limit))
}

pub(super) fn budget_allows(
    budget: Option<&str>,
    ledger: &LedgerState,
    grant_id: &str,
    asset: &str,
    value: Option<&str>,
) -> bool {
    let Some(budget) = budget else {
        return true;
    };
    let Some(value) = value else {
        return true;
    };
    let Ok(budget) = parse_u256(budget) else {
        return false;
    };
    let Ok(value) = parse_u256(value) else {
        return false;
    };
    spent_for_grant(ledger, grant_id, asset).saturating_add(value) <= budget
}

fn ensure_eq_addr(actual: &str, expected: &str, field: &str) -> Result<()> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(Error::PolicyDenied {
            reason: format!("{field} mismatch"),
        })
    }
}

fn ensure_eq_amount(actual: U256, expected: &str, field: &str) -> Result<()> {
    if actual == parse_u256(expected)? {
        Ok(())
    } else {
        Err(Error::PolicyDenied {
            reason: format!("{field} mismatch"),
        })
    }
}

fn ensure_empty_data(transaction: &WireTransaction) -> Result<()> {
    if transaction.data == "0x" || transaction.data == "0x00" || transaction.data.is_empty() {
        Ok(())
    } else {
        Err(Error::PolicyDenied {
            reason: "native transfer calldata mismatch".to_string(),
        })
    }
}

fn ensure_selector(transaction: &WireTransaction, selector: &str) -> Result<()> {
    if normalize_selector(&transaction.data).starts_with(&normalize_selector(selector)) {
        Ok(())
    } else {
        Err(Error::PolicyDenied {
            reason: "function selector mismatch".to_string(),
        })
    }
}

fn normalize_selector(value: &str) -> String {
    let value = value.strip_prefix("0x").unwrap_or(value);
    format!("0x{}", value.to_ascii_lowercase())
}

fn parse_u256(value: &str) -> Result<U256> {
    if let Some(value) = value.strip_prefix("0x") {
        return U256::from_str_radix(value, 16).map_err(|_| Error::InvalidAmount {
            value: format!("0x{value}"),
        });
    }
    U256::from_dec_str(value).map_err(|_| Error::InvalidAmount {
        value: value.to_string(),
    })
}
