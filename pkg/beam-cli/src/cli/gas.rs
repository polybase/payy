use clap::Subcommand;

use super::{SendArgs, TransferArgs};

#[derive(Debug, Subcommand)]
pub enum GasAction {
    /// Estimate gas for a native token transfer
    Transfer(TransferArgs),
    /// Estimate gas for ERC20 token operations
    Erc20 {
        #[command(subcommand)]
        action: Erc20GasAction,
    },
    /// Estimate gas for a contract transaction
    Send(SendArgs),
}

#[derive(Debug, Subcommand)]
pub enum Erc20GasAction {
    /// Estimate gas for an ERC20 token transfer
    Transfer {
        token: String,
        to: String,
        amount: String,
    },
    /// Estimate gas for an ERC20 approval
    Approve {
        token: String,
        spender: String,
        amount: String,
    },
}
