// lint-long-file-override allow-max-lines=280
pub mod util;

use clap::{Args, Parser, Subcommand};

use crate::{display::ColorMode, output::OutputMode, runtime::InvocationOverrides};
use util::UtilAction;

#[derive(Debug, Parser)]
#[command(name = "beam", version, about = "Ethereum wallet CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(long, global = true)]
    pub rpc: Option<String>,

    #[arg(long, global = true)]
    pub from: Option<String>,

    #[arg(long, global = true)]
    pub chain: Option<String>,

    #[arg(long, global = true, value_enum, default_value_t = OutputMode::Default)]
    pub output: OutputMode,

    #[arg(
        long,
        global = true,
        value_enum,
        default_value_t = ColorMode::Auto,
        help = "The color of the log messages"
    )]
    pub color: ColorMode,

    #[arg(long, global = true, hide = true, default_value_t = false)]
    pub no_update_check: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage stored wallets
    #[command(name = "wallets")]
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },
    /// Run standalone utility commands
    Util {
        #[command(subcommand)]
        action: UtilAction,
    },
    /// Manage chain presets
    #[command(name = "chains")]
    Chain {
        #[command(subcommand)]
        action: ChainAction,
    },
    /// Manage RPC endpoints for the active chain
    Rpc {
        #[command(subcommand)]
        action: RpcAction,
    },
    /// Manage tracked tokens for the active chain
    Tokens {
        #[command(subcommand)]
        action: Option<TokenAction>,
    },
    /// Show balances for tracked tokens or a specific token
    Balance(BalanceArgs),
    /// Send the native token
    Transfer(TransferArgs),
    /// Inspect a transaction
    #[command(name = "txn", visible_alias = "tx")]
    Txn(TxnArgs),
    /// Inspect a block
    Block(BlockArgs),
    /// Work with ERC20 tokens
    Erc20 {
        #[command(subcommand)]
        action: Erc20Action,
    },
    /// Run a read-only contract call
    Call(CallArgs),
    /// Send a contract transaction
    Send(SendArgs),
    /// Check for beam updates
    Update,
    #[command(name = "__refresh-update-status", hide = true)]
    RefreshUpdateStatus,
}

#[derive(Debug, Subcommand)]
pub enum WalletAction {
    /// Create a new wallet
    Create { name: Option<String> },
    /// Import a wallet from a private key
    Import {
        #[command(flatten)]
        private_key_source: PrivateKeySourceArgs,
        #[arg(long)]
        name: Option<String>,
    },
    /// List stored wallets
    List,
    /// Rename a stored wallet
    Rename { name: String, new_name: String },
    /// Derive an address from a private key
    Address {
        #[command(flatten)]
        private_key_source: PrivateKeySourceArgs,
    },
    /// Set the default wallet
    Use { name: String },
}

#[derive(Debug, Subcommand)]
pub enum ChainAction {
    /// List available chains
    List,
    /// Add a custom chain
    Add(ChainAddArgs),
    /// Remove a custom chain
    Remove { chain: String },
    /// Set the default chain
    Use { chain: String },
}

#[derive(Clone, Debug, Args)]
pub struct ChainAddArgs {
    pub name: Option<String>,
    pub rpc: Option<String>,
    #[arg(long)]
    pub chain_id: Option<u64>,
    #[arg(long)]
    pub native_symbol: Option<String>,
}

#[derive(Clone, Debug, Default, Args, PartialEq, Eq)]
pub struct PrivateKeySourceArgs {
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "private_key_fd",
        help = "Read the private key from stdin instead of prompting"
    )]
    pub private_key_stdin: bool,

    #[arg(
        long,
        value_name = "FD",
        conflicts_with = "private_key_stdin",
        help = "Read the private key from an already-open file descriptor"
    )]
    pub private_key_fd: Option<u32>,
}

#[derive(Debug, Subcommand)]
pub enum RpcAction {
    /// List RPC endpoints for the active chain
    List,
    /// Add an RPC endpoint to the active chain
    Add(RpcAddArgs),
    /// Remove an RPC endpoint from the active chain
    Remove { rpc: String },
    /// Set the default RPC endpoint for the active chain
    Use { rpc: String },
}

#[derive(Clone, Debug, Args)]
pub struct RpcAddArgs {
    pub rpc: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Erc20Action {
    /// Show an ERC20 token balance
    Balance {
        token: String,
        address: Option<String>,
    },
    /// Send ERC20 tokens
    Transfer {
        token: String,
        to: String,
        amount: String,
    },
    /// Approve an ERC20 spender
    Approve {
        token: String,
        spender: String,
        amount: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum TokenAction {
    /// List tracked tokens and their balances
    List,
    /// Add a token to the tracked list
    Add(TokenAddArgs),
    /// Remove a token from the tracked list
    Remove { token: String },
}

#[derive(Clone, Debug, Args)]
pub struct TokenAddArgs {
    pub token: Option<String>,
    pub label: Option<String>,
    #[arg(long)]
    pub decimals: Option<u8>,
}

#[derive(Clone, Debug, Args)]
pub struct BalanceArgs {
    pub token: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct TransferArgs {
    pub to: String,
    pub amount: String,
}

#[derive(Clone, Debug, Args)]
pub struct TxnArgs {
    pub tx_hash: String,
}

#[derive(Clone, Debug, Args)]
pub struct BlockArgs {
    pub block: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct CallArgs {
    pub contract: String,
    pub function_sig: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Args)]
pub struct SendArgs {
    #[command(flatten)]
    pub call: CallArgs,

    #[arg(long, help = "Amount of native token to attach to the contract call")]
    pub value: Option<String>,
}

impl Cli {
    pub fn overrides(&self) -> InvocationOverrides {
        InvocationOverrides {
            chain: self.chain.clone(),
            from: self.from.clone(),
            rpc: self.rpc.clone(),
        }
    }

    pub fn is_interactive(&self) -> bool {
        self.command.is_none()
    }
}

impl Command {
    pub(crate) fn is_sensitive(&self) -> bool {
        matches!(self, Self::Wallet { action } if action.is_sensitive())
    }
}

impl WalletAction {
    pub(crate) fn is_sensitive(&self) -> bool {
        matches!(self, Self::Import { .. } | Self::Address { .. })
    }
}
