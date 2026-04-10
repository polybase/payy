use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use contextful::ResultContextExt;
use tokio::task_local;

use crate::{
    cli::{Command, Erc20Action},
    error::{Error, Result},
};

use super::interactive::ParsedLine;

task_local! {
    static INTERRUPT_CONTROLLER: InterruptController;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum InterruptOwner {
    Repl,
    Command,
}

impl InterruptOwner {
    fn from_atomic(value: u8) -> Self {
        match value {
            value if value == Self::Repl as u8 => Self::Repl,
            value if value == Self::Command as u8 => Self::Command,
            value => unreachable!("unknown interrupt owner {value}"),
        }
    }
}

#[derive(Clone, Debug)]
struct InterruptController {
    owner: Arc<AtomicU8>,
}

#[derive(Debug)]
#[must_use = "keep the guard alive until command-owned interrupt handling should end"]
pub(crate) struct InterruptOwnerGuard {
    controller: Option<InterruptController>,
    previous_owner: InterruptOwner,
}

impl ParsedLine {
    pub(crate) fn interrupt_owner(&self) -> InterruptOwner {
        match self {
            Self::Cli { cli, .. }
                if matches!(
                    cli.command.as_ref(),
                    Some(Command::Transfer(_))
                        | Some(Command::Send(_))
                        | Some(Command::Erc20 {
                            action: Erc20Action::Transfer { .. },
                        })
                        | Some(Command::Erc20 {
                            action: Erc20Action::Approve { .. },
                        })
                ) =>
            {
                InterruptOwner::Command
            }
            _ => InterruptOwner::Repl,
        }
    }
}

impl InterruptController {
    fn new(owner: InterruptOwner) -> Self {
        Self {
            owner: Arc::new(AtomicU8::new(owner as u8)),
        }
    }

    fn owner(&self) -> InterruptOwner {
        InterruptOwner::from_atomic(self.owner.load(Ordering::SeqCst))
    }

    fn set_owner(&self, owner: InterruptOwner) {
        self.owner.store(owner as u8, Ordering::SeqCst);
    }

    fn delegate_to_command(&self) -> InterruptOwnerGuard {
        let previous_owner = InterruptOwner::from_atomic(
            self.owner
                .swap(InterruptOwner::Command as u8, Ordering::SeqCst),
        );

        InterruptOwnerGuard {
            controller: Some(self.clone()),
            previous_owner,
        }
    }
}

impl InterruptOwnerGuard {
    fn inactive() -> Self {
        Self {
            controller: None,
            previous_owner: InterruptOwner::Repl,
        }
    }
}

impl Drop for InterruptOwnerGuard {
    fn drop(&mut self) {
        if let Some(controller) = self.controller.as_ref() {
            controller.set_owner(self.previous_owner);
        }
    }
}

pub(crate) fn delegate_current_interrupt_to_command() -> InterruptOwnerGuard {
    INTERRUPT_CONTROLLER
        .try_with(InterruptController::delegate_to_command)
        .unwrap_or_else(|_| InterruptOwnerGuard::inactive())
}

pub(crate) async fn run_with_interrupt_owner<T, F, MakeCancel, C>(
    owner: InterruptOwner,
    future: F,
    make_cancel: MakeCancel,
) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
    MakeCancel: FnMut() -> C,
    C: std::future::Future<Output = std::io::Result<()>>,
{
    let controller = InterruptController::new(owner);
    INTERRUPT_CONTROLLER
        .scope(
            controller.clone(),
            run_with_interrupt_controller(controller, future, make_cancel),
        )
        .await
}

async fn run_with_interrupt_controller<T, F, MakeCancel, C>(
    controller: InterruptController,
    future: F,
    mut make_cancel: MakeCancel,
) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
    MakeCancel: FnMut() -> C,
    C: std::future::Future<Output = std::io::Result<()>>,
{
    if controller.owner() == InterruptOwner::Command {
        return future.await;
    }

    let mut future = std::pin::pin!(future);

    loop {
        let cancel = make_cancel();
        tokio::pin!(cancel);

        tokio::select! {
            output = &mut future => return output,
            signal = &mut cancel => {
                signal.context("listen for beam ctrl-c")?;
                if controller.owner() == InterruptOwner::Repl {
                    return Err(Error::Interrupted);
                }
            }
        }
    }
}
