use clap::{Args, Subcommand};

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
    /// Export a stored wallet's raw private key
    ExportPrivateKey { wallet: Option<String> },
    /// Export a Payy-compatible wallet recovery phrase
    ExportRecoveryPhrase { wallet: Option<String> },
    /// Import a wallet from a Payy-compatible recovery phrase
    ImportRecoveryPhrase {
        #[command(flatten)]
        phrase_source: RecoveryPhraseSourceArgs,
        #[arg(long)]
        expected_address: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    /// List stored wallets
    List,
    /// Change a stored wallet's encryption password
    ChangePassword { wallet: Option<String> },
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

#[derive(Clone, Debug, Default, Args, PartialEq, Eq)]
pub struct RecoveryPhraseSourceArgs {
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "phrase_fd",
        help = "Read the recovery phrase from stdin instead of prompting"
    )]
    pub phrase_stdin: bool,

    #[arg(
        long,
        value_name = "FD",
        conflicts_with = "phrase_stdin",
        help = "Read the recovery phrase from an already-open file descriptor"
    )]
    pub phrase_fd: Option<u32>,
}

impl WalletAction {
    pub(crate) fn is_sensitive(&self) -> bool {
        matches!(
            self,
            Self::Import { .. }
                | Self::ExportPrivateKey { .. }
                | Self::ImportRecoveryPhrase { .. }
                | Self::Address { .. }
        )
    }
}
