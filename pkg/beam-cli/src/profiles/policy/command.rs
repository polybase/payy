use contracts::U256;

use crate::profiles::{
    ledger::LedgerState,
    model::{CommandGrant, PublicCommandKind, PublicSigningIntent},
    policy::{amount_allows, budget_allows, optional_addr_eq, optional_eq, optional_selector_eq},
    wire::WireTransaction,
};

pub(super) fn command_grant_matches(
    grant: &CommandGrant,
    intent: &PublicSigningIntent,
    transaction: &WireTransaction,
    ledger: &LedgerState,
) -> bool {
    let Some(summary) = command_summary(intent) else {
        return false;
    };
    grant.command == summary.command
        && optional_eq(grant.chain.as_deref(), Some(summary.chain))
        && optional_eq(grant.token.as_deref(), summary.token)
        && optional_addr_eq(grant.recipient.as_deref(), summary.recipient)
        && optional_addr_eq(grant.target.as_deref(), summary.target)
        && optional_selector_eq(grant.selector.as_deref(), summary.selector)
        && optional_addr_eq(grant.spender.as_deref(), summary.spender)
        && amount_allows(grant.max_native_value.as_deref(), summary.native_value)
        && amount_allows(grant.max_token_amount.as_deref(), summary.token_amount)
        && amount_allows(grant.max_gas.as_deref(), Some(&transaction.gas))
        && approval_allows(grant, summary.token_amount)
        && budget_allows(
            grant.cumulative_budget.as_deref(),
            ledger,
            &grant.id,
            summary.asset,
            summary.spend,
        )
}

struct CommandSummary<'a> {
    command: PublicCommandKind,
    chain: &'a str,
    token: Option<&'a str>,
    recipient: Option<&'a str>,
    target: Option<&'a str>,
    selector: Option<&'a str>,
    spender: Option<&'a str>,
    native_value: Option<&'a str>,
    token_amount: Option<&'a str>,
    asset: &'a str,
    spend: Option<&'a str>,
}

fn command_summary(intent: &PublicSigningIntent) -> Option<CommandSummary<'_>> {
    match intent {
        PublicSigningIntent::NativeTransfer(intent) => Some(CommandSummary {
            command: PublicCommandKind::NativeTransfer,
            chain: &intent.chain,
            token: None,
            recipient: Some(&intent.recipient),
            target: None,
            selector: None,
            spender: None,
            native_value: Some(&intent.amount),
            token_amount: None,
            asset: &intent.asset,
            spend: Some(&intent.amount),
        }),
        PublicSigningIntent::Erc20Transfer(intent) => Some(CommandSummary {
            command: PublicCommandKind::Erc20Transfer,
            chain: &intent.chain,
            token: Some(&intent.token),
            recipient: Some(&intent.recipient),
            target: Some(&intent.token),
            selector: Some("0xa9059cbb"),
            spender: None,
            native_value: None,
            token_amount: Some(&intent.amount),
            asset: &intent.token,
            spend: Some(&intent.amount),
        }),
        PublicSigningIntent::Erc20Approval(intent) => Some(CommandSummary {
            command: PublicCommandKind::Erc20Approval,
            chain: &intent.chain,
            token: Some(&intent.token),
            recipient: None,
            target: Some(&intent.token),
            selector: Some("0x095ea7b3"),
            spender: Some(&intent.spender),
            native_value: None,
            token_amount: Some(&intent.amount),
            asset: &intent.token,
            spend: Some(&intent.amount),
        }),
        PublicSigningIntent::ContractTransaction(intent) => Some(CommandSummary {
            command: PublicCommandKind::ContractTransaction,
            chain: &intent.chain,
            token: None,
            recipient: None,
            target: Some(&intent.target),
            selector: Some(&intent.selector),
            spender: None,
            native_value: Some(&intent.native_value),
            token_amount: None,
            asset: "native",
            spend: Some(&intent.native_value),
        }),
        PublicSigningIntent::FetchPayment(intent) => Some(CommandSummary {
            command: PublicCommandKind::FetchPayment,
            chain: &intent.chain,
            token: Some(&intent.asset),
            recipient: Some(&intent.recipient),
            target: None,
            selector: None,
            spender: None,
            native_value: None,
            token_amount: Some(&intent.amount),
            asset: &intent.asset,
            spend: Some(&intent.amount),
        }),
        PublicSigningIntent::AppActionPlan(_) => None,
    }
}

fn approval_allows(grant: &CommandGrant, amount: Option<&str>) -> bool {
    if grant.command != PublicCommandKind::Erc20Approval {
        return true;
    }
    grant.allow_unlimited_approval
        || amount.is_none_or(|amount| parse_u256(amount).is_ok_and(|value| value != U256::MAX))
}

fn parse_u256(value: &str) -> Result<U256, ()> {
    if let Some(value) = value.strip_prefix("0x") {
        return U256::from_str_radix(value, 16).map_err(|_| ());
    }
    U256::from_dec_str(value).map_err(|_| ())
}
