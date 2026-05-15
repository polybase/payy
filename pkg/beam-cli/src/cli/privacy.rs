use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum PrivacyAction {
    /// Show the selected wallet's private address
    Address,
    /// Show private balances
    Balance(PrivacyBalanceArgs),
    /// Deposit ERC20 value into the privacy pool
    Mint(PrivacyTokenAmountArgs),
    /// Withdraw private value to a public EVM recipient
    Burn(PrivacyBurnArgs),
    /// Send private value
    Send(PrivacySendArgs),
    /// Discover incoming private transfers
    Incoming {
        #[command(subcommand)]
        action: PrivacyIncomingAction,
    },
    /// Claim a stored incoming note, claim link, or ephemeral artifact
    Claim { source: String },
    /// Manage persisted privacy state
    State {
        #[command(subcommand)]
        action: PrivacyStateAction,
    },
}

#[derive(Clone, Debug, Args)]
pub struct PrivacyBalanceArgs {
    pub token: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct PrivacyTokenAmountArgs {
    pub token: String,
    pub amount: String,
}

#[derive(Clone, Debug, Args)]
pub struct PrivacyBurnArgs {
    pub token: String,
    pub amount: String,
    pub recipient: String,
}

#[derive(Clone, Debug, Args)]
pub struct PrivacySendArgs {
    #[arg(value_name = "ARGS", num_args = 2..=3)]
    pub args: Vec<String>,
    #[arg(long)]
    pub memo: Option<String>,
    #[arg(long)]
    pub claim_link_message: Option<String>,
    #[arg(long, default_value_t = false)]
    pub ephemeral: bool,
}

#[derive(Debug, Subcommand)]
pub enum PrivacyIncomingAction {
    /// List incoming private transfers
    List(PrivacyIncomingListArgs),
    /// Watch for incoming private transfers
    Watch(PrivacyIncomingWatchArgs),
}

#[derive(Clone, Debug, Args)]
pub struct PrivacyIncomingListArgs {
    #[arg(long)]
    pub from_block: Option<u64>,
    #[arg(long)]
    pub to_block: Option<u64>,
    #[arg(long, default_value_t = false)]
    pub include_spent: bool,
}

#[derive(Clone, Debug, Args)]
pub struct PrivacyIncomingWatchArgs {
    #[arg(long)]
    pub from_block: Option<u64>,
    #[arg(long, default_value_t = false)]
    pub include_spent: bool,
    #[arg(long)]
    pub poll_interval_ms: Option<u64>,
}

#[derive(Debug, Subcommand)]
pub enum PrivacyStateAction {
    /// Reset persisted privacy state for the active wallet and chain
    Reset,
    /// Move aside a corrupted privacy state file
    Repair,
}
