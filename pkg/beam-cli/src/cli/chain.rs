use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum ChainAction {
    /// List available chains
    List,
    /// Add a custom chain
    Add(Box<ChainAddArgs>),
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
    #[arg(long)]
    pub privacy_bridge: Option<String>,
    #[arg(long)]
    pub privacy_standard: Option<String>,
    #[arg(long)]
    pub privacy_version: Option<u32>,
    #[arg(long)]
    pub privacy_deployment: Option<String>,
    #[arg(long)]
    pub privacy_prover: Option<String>,
    #[arg(long)]
    pub privacy_token_policy: Option<String>,
    #[arg(long)]
    pub privacy_state_policy: Option<String>,
    #[arg(long, help = "Comma-separated privacy features, or all")]
    pub privacy_features: Option<String>,
}
