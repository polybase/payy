use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

#[derive(Debug, Subcommand)]
pub enum ProfilesAction {
    /// Create a delegated signing profile
    Create(ProfileCreateArgs),
    /// List delegated signing profiles
    List,
    /// Show one delegated signing profile
    Show { profile: String },
    /// Add a command or app grant to a profile
    Grant(Box<ProfileGrantArgs>),
    /// Revoke one grant from a profile
    Revoke(ProfileRevokeArgs),
    /// Remove a delegated signing profile
    Remove { profile: String },
    /// Show profile usage ledger entries
    Ledger { profile: String },
    /// Unlock a profile session in the local profile daemon
    Unlock(ProfileUnlockArgs),
    /// Lock an unlocked profile session
    Lock { profile: String },
    /// List unlocked profile sessions
    Sessions,
}

#[derive(Clone, Debug, Args)]
pub struct ProfileCreateArgs {
    pub profile: String,
    #[arg(long)]
    pub from: String,
}

#[derive(Clone, Debug, Args)]
pub struct ProfileGrantArgs {
    pub profile: String,
    #[command(subcommand)]
    pub grant: ProfileGrantKind,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ProfileGrantKind {
    /// Grant a direct public signing command
    Command(ProfileCommandGrantArgs),
    /// Grant a Beam app action plan
    App(ProfileAppGrantArgs),
}

#[derive(Clone, Debug, Args)]
pub struct ProfileCommandGrantArgs {
    #[arg(value_enum)]
    pub command: ProfileCommandKind,
    #[arg(long)]
    pub chain: Option<String>,
    #[arg(long)]
    pub token: Option<String>,
    #[arg(long)]
    pub recipient: Option<String>,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long)]
    pub selector: Option<String>,
    #[arg(long)]
    pub spender: Option<String>,
    #[arg(long)]
    pub max_native: Option<String>,
    #[arg(long)]
    pub max_token: Option<String>,
    #[arg(long)]
    pub max_gas: Option<String>,
    #[arg(long)]
    pub budget: Option<String>,
    #[arg(long)]
    pub ttl: Option<String>,
    #[arg(long, default_value_t = false)]
    pub allow_unlimited_approval: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ProfileCommandKind {
    NativeTransfer,
    Erc20Transfer,
    Erc20Approval,
    ContractTransaction,
    FetchPayment,
}

#[derive(Clone, Debug, Args)]
pub struct ProfileAppGrantArgs {
    pub app: String,
    #[arg(long)]
    pub command: Option<String>,
    #[arg(long)]
    pub approval_id: Option<String>,
    #[arg(long)]
    pub plan_json: Option<PathBuf>,
    #[arg(long)]
    pub registry_url: Option<String>,
    #[arg(long)]
    pub version: Option<String>,
    #[arg(long)]
    pub manifest_digest: Option<String>,
    #[arg(long)]
    pub module_digest: Option<String>,
    #[arg(long)]
    pub chain: Option<String>,
    #[arg(long)]
    pub wallet: Option<String>,
    #[arg(long)]
    pub plan_hash: Option<String>,
    #[arg(long)]
    pub max_gas: Option<String>,
    #[arg(long)]
    pub budget: Option<String>,
    #[arg(long)]
    pub ttl: Option<String>,
    #[arg(long, default_value_t = false)]
    pub allow_unlimited_approval: bool,
}

#[derive(Clone, Debug, Args)]
pub struct ProfileRevokeArgs {
    pub profile: String,
    pub grant_id: String,
}

#[derive(Clone, Debug, Args)]
pub struct ProfileUnlockArgs {
    pub profile: String,
    #[arg(long)]
    pub ttl: Option<String>,
    #[arg(long, default_value_t = false)]
    pub print_env: bool,
}

#[derive(Clone, Debug, Args)]
pub struct ProfileDaemonArgs {
    #[arg(long)]
    pub root: PathBuf,
    #[arg(long)]
    pub profile: String,
    #[arg(long)]
    pub socket: PathBuf,
    #[arg(long)]
    pub session: String,
    #[arg(long)]
    pub expires_at: u64,
}
