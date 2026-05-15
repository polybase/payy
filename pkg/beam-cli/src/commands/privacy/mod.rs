mod common;
mod ephemeral;
mod read;
mod render;
mod write;
mod write_support;

use crate::{
    cli::{PrivacyAction, PrivacyIncomingAction, PrivacyStateAction},
    error::Result,
    runtime::BeamApp,
};

pub async fn run(app: &BeamApp, action: PrivacyAction) -> Result<()> {
    match action {
        PrivacyAction::Address => read::address(app).await,
        PrivacyAction::Balance(args) => read::balance(app, args).await,
        PrivacyAction::Mint(args) => write::mint(app, args).await,
        PrivacyAction::Burn(args) => write::burn(app, args).await,
        PrivacyAction::Send(args) => write::send(app, args).await,
        PrivacyAction::Incoming { action } => match action {
            PrivacyIncomingAction::List(args) => read::incoming_list(app, args).await,
            PrivacyIncomingAction::Watch(args) => read::incoming_watch(app, args).await,
        },
        PrivacyAction::Claim { source } => write::claim(app, &source).await,
        PrivacyAction::State { action } => match action {
            PrivacyStateAction::Reset => read::state_reset(app).await,
            PrivacyStateAction::Repair => read::state_repair(app).await,
        },
    }
}
