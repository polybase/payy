use crate::{
    chains::find_chain,
    cli::{ChainAction, Command, RpcAction, WalletAction},
    error::Result,
    runtime::{BeamApp, InvocationOverrides},
};

pub(crate) enum ReplStateMutation {
    WalletRename { name: String },
    Chain,
    RpcRemove { rpc: String },
}

#[derive(Default)]
pub(crate) struct ReplStateSnapshot {
    active_chain_key: Option<String>,
    active_rpc_url: Option<String>,
    affected_chain_key: Option<String>,
    renamed_wallet_address: Option<String>,
    selected_address: Option<String>,
}

pub(crate) fn repl_state_mutation(command: &Command) -> Option<ReplStateMutation> {
    match command {
        Command::Wallet {
            action: WalletAction::Rename { name, .. },
        } => Some(ReplStateMutation::WalletRename { name: name.clone() }),
        Command::Chain {
            action: ChainAction::Remove { .. } | ChainAction::Use { .. },
        } => Some(ReplStateMutation::Chain),
        Command::Rpc {
            action: RpcAction::Remove { rpc },
        } => Some(ReplStateMutation::RpcRemove { rpc: rpc.clone() }),
        _ => None,
    }
}

pub(crate) async fn capture_repl_state(
    app: &BeamApp,
    command_app: &BeamApp,
    overrides: &InvocationOverrides,
    mutation: &ReplStateMutation,
) -> Result<ReplStateSnapshot> {
    let session = repl_session(app, overrides);

    match mutation {
        ReplStateMutation::WalletRename { name } => Ok(ReplStateSnapshot {
            renamed_wallet_address: Some(app.resolve_wallet(name).await?.address),
            selected_address: if overrides.from.is_some() {
                session
                    .active_optional_address()
                    .await?
                    .map(|address| format!("{address:#x}"))
            } else {
                None
            },
            ..ReplStateSnapshot::default()
        }),
        ReplStateMutation::Chain => {
            if overrides.chain.is_none() && overrides.rpc.is_none() {
                return Ok(ReplStateSnapshot::default());
            }

            Ok(ReplStateSnapshot {
                active_chain_key: Some(session.active_chain().await?.entry.key),
                ..ReplStateSnapshot::default()
            })
        }
        ReplStateMutation::RpcRemove { .. } => {
            if overrides.rpc.is_none() {
                return Ok(ReplStateSnapshot::default());
            }

            let session_chain = session.active_chain().await?;
            Ok(ReplStateSnapshot {
                active_chain_key: Some(session_chain.entry.key),
                active_rpc_url: Some(session_chain.rpc_url),
                affected_chain_key: Some(command_app.active_chain().await?.entry.key),
                ..ReplStateSnapshot::default()
            })
        }
    }
}

pub(crate) async fn reconcile_repl_state(
    app: &BeamApp,
    overrides: &mut InvocationOverrides,
    mutation: &ReplStateMutation,
    snapshot: ReplStateSnapshot,
) -> Result<()> {
    match mutation {
        ReplStateMutation::WalletRename { .. } => {
            if overrides.from.is_some()
                && snapshot.selected_address == snapshot.renamed_wallet_address
            {
                overrides.from = app
                    .canonical_wallet_selector(snapshot.selected_address.as_deref())
                    .await?;
            }
        }
        ReplStateMutation::Chain => {
            let Some(previous_chain_key) = snapshot.active_chain_key.as_deref() else {
                return Ok(());
            };

            if overrides.chain.is_some() {
                let available = app.chain_store.get().await;
                if let Ok(chain) = find_chain(previous_chain_key, &available) {
                    overrides.chain = Some(chain.key);
                } else {
                    overrides.chain = None;
                    overrides.rpc = None;
                    return Ok(());
                }
            }

            if overrides.rpc.is_some()
                && repl_session(app, overrides).active_chain().await?.entry.key
                    != previous_chain_key
            {
                overrides.rpc = None;
            }
        }
        ReplStateMutation::RpcRemove { rpc } => {
            if overrides.rpc.is_some()
                && snapshot.active_chain_key == snapshot.affected_chain_key
                && snapshot.active_rpc_url.as_deref() == Some(rpc.as_str())
            {
                overrides.rpc = None;
            }
        }
    }

    Ok(())
}

fn repl_session(app: &BeamApp, overrides: &InvocationOverrides) -> BeamApp {
    BeamApp {
        overrides: overrides.clone(),
        ..app.clone()
    }
}
