use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum AppsAction {
    /// Install an app from the Beam registry
    Install(AppInstallArgs),
    /// List installed apps
    List,
    /// Show app metadata
    Info { app: String },
    /// Update one app or every installed app
    Update { app: Option<String> },
    /// Remove an installed app
    Remove(AppRemoveArgs),
    /// Show app permissions
    Permissions { app: String },
    /// Run an installed app
    Run(AppRunArgs),
    /// Manage app approval continuations
    Approvals {
        #[command(subcommand)]
        action: AppApprovalAction,
    },
}

#[derive(Clone, Debug, Args)]
pub struct AppInstallArgs {
    pub app: String,
    #[arg(long)]
    pub version: Option<String>,
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Clone, Debug, Args)]
pub struct AppRemoveArgs {
    pub app: String,
    #[arg(long, default_value_t = false)]
    pub purge_data: bool,
}

#[derive(Clone, Debug, Args)]
pub struct AppRunArgs {
    pub app: String,
    #[arg(long, default_value_t = false)]
    pub prepare: bool,
    #[arg(long, default_value_t = false)]
    pub no_prompt: bool,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum AppApprovalAction {
    /// List app approval continuations
    List,
    /// Show an app approval continuation
    Show { approval_id: String },
    /// Approve an app approval continuation
    Approve {
        approval_id: String,
        #[arg(long, default_value_t = false)]
        execute: bool,
    },
    /// Reject an app approval continuation
    Reject { approval_id: String },
}
