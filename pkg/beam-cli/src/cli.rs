// lint-long-file-override allow-max-lines=300
mod apps;
mod chain;
mod contract;
mod fetch;
mod gas;
mod normalize;
mod privacy;
mod wallet;

pub mod util;

use clap::{Args, Parser, Subcommand};

use crate::{display::ColorMode, output::OutputMode, runtime::InvocationOverrides};

pub use apps::*;
pub use chain::*;
pub use contract::*;
pub use fetch::FetchArgs;
pub use gas::*;
pub(crate) use normalize::normalize_cli_args;
pub use privacy::*;
use util::UtilAction;
pub use wallet::*;

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

    #[arg(long = "format", global = true, value_enum, default_value_t = OutputMode::Default)]
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
    /// Manage Beam apps
    Apps {
        #[command(subcommand)]
        action: AppsAction,
    },
    /// Run a Beam app
    X(AppRunArgs),
    /// Work with private balances and transfers
    Privacy {
        #[command(subcommand)]
        action: PrivacyAction,
    },
    /// Show balances for tracked tokens or a specific token
    Balance(BalanceArgs),
    /// Send the native token
    Transfer(TransferArgs),
    /// Estimate gas for a native transfer or contract transaction
    #[command(name = "gas", visible_aliases = ["estimate-gas", "estimate"])]
    Gas {
        #[command(subcommand)]
        action: GasAction,
    },
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
    /// Inspect deployed contracts
    Contract {
        #[command(subcommand)]
        action: ContractAction,
    },
    /// Run a read-only contract call
    Call(CallArgs),
    /// Send a contract transaction
    Send(SendArgs),
    /// Fetch an HTTP resource and handle x402 / MPP payment challenges
    Fetch(FetchArgs),
    /// Check for beam updates
    Update,
    #[command(name = "__refresh-update-status", hide = true)]
    RefreshUpdateStatus,
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
        match self {
            Self::Wallet { action } => action.is_sensitive(),
            Self::Privacy { action } => action.is_sensitive(),
            Self::Fetch(args) => args.private_payment,
            _ => false,
        }
    }
}

impl PrivacyAction {
    pub(crate) fn is_sensitive(&self) -> bool {
        match self {
            Self::Send(args) => {
                args.ephemeral || args.claim_link_message.is_some() || args.memo.is_some()
            }
            Self::Claim { .. } => true,
            _ => false,
        }
    }
}
