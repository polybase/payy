use crate::{
    cli::{Command, Erc20Action},
    error::Result,
    output::with_interrupt,
};

use super::interactive::ParsedLine;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InterruptOwner {
    Repl,
    Command,
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

pub(crate) async fn run_with_interrupt_owner<T, F, C>(
    owner: InterruptOwner,
    future: F,
    cancel: C,
) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
    C: std::future::Future<Output = std::io::Result<()>>,
{
    match owner {
        InterruptOwner::Repl => with_interrupt(future, cancel).await,
        InterruptOwner::Command => future.await,
    }
}
