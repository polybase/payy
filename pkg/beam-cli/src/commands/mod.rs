pub mod balance;
pub mod block;
pub mod call;
pub mod chain;
pub mod erc20;
pub mod interactive;
pub(crate) mod interactive_helper;
pub(crate) mod interactive_history;
pub(crate) mod interactive_interrupt;
pub(crate) mod interactive_parse;
pub(crate) mod interactive_state;
mod interactive_suggestion;
pub mod rpc;
pub(crate) mod signing;
pub(crate) mod token_report;
pub mod tokens;
pub mod transfer;
pub mod txn;
pub mod update;
pub mod util;
pub mod wallet;

use crate::{cli::Command, error::Result, runtime::BeamApp};

pub async fn run(app: &BeamApp, command: Command) -> Result<()> {
    match command {
        Command::Wallet { action } => wallet::run(app, action).await,
        Command::Util { action } => util::run(app.output_mode, action),
        Command::Chain { action } => chain::run(app, action).await,
        Command::Rpc { action } => rpc::run(app, action).await,
        Command::Tokens { action } => tokens::run(app, action).await,
        Command::Balance(args) => balance::run(app, args).await,
        Command::Transfer(args) => transfer::run(app, args).await,
        Command::Txn(args) => txn::run(app, args).await,
        Command::Block(args) => block::run(app, args).await,
        Command::Erc20 { action } => erc20::run(app, action).await,
        Command::Call(args) => call::run_read(app, args).await,
        Command::Send(args) => call::run_write(app, args).await,
        Command::Update => {
            update::run_update(&app.overrides, app.output_mode, app.color_mode).await
        }
        Command::RefreshUpdateStatus => Ok(()),
    }
}
