use std::{
    future::pending,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::time::sleep;

use crate::{
    commands::{
        interactive::parse_line,
        interactive_interrupt::{InterruptOwner, run_with_interrupt_owner},
    },
    error::{Error, Result},
};

#[test]
fn interactive_write_commands_delegate_interrupts_to_command_handlers() {
    for line in [
        "transfer 0xabc 1",
        "send 0xabc transfer()",
        "erc20 transfer USDC 0xabc 1",
        "erc20 approve USDC 0xabc 1",
    ] {
        let parsed = parse_line(line).expect("parse interactive write command");
        assert_eq!(parsed.interrupt_owner(), InterruptOwner::Command, "{line}",);
    }
}

#[test]
fn interactive_non_write_commands_keep_repl_interrupts() {
    for line in [
        "balance",
        "call 0xabc totalSupply():(uint256)",
        "erc20 balance USDC",
        "wallets list",
    ] {
        let parsed = parse_line(line).expect("parse interactive non-write command");
        assert_eq!(parsed.interrupt_owner(), InterruptOwner::Repl, "{line}");
    }
}

#[tokio::test]
async fn write_commands_ignore_repl_interrupt_wrapper() {
    let parsed = parse_line("transfer 0xabc 1").expect("parse interactive write command");
    let ran = Arc::new(AtomicBool::new(false));

    run_with_interrupt_owner(
        parsed.interrupt_owner(),
        {
            let ran = Arc::clone(&ran);
            async move {
                ran.store(true, Ordering::SeqCst);
                Ok(())
            }
        },
        async { Ok(()) },
    )
    .await
    .expect("write command should own ctrl-c");

    assert!(ran.load(Ordering::SeqCst));
}

#[tokio::test]
async fn read_commands_still_use_repl_interrupt_wrapper() {
    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let parsed = parse_line("balance").expect("parse interactive non-write command");
    let dropped = Arc::new(AtomicBool::new(false));
    let err = run_with_interrupt_owner(
        parsed.interrupt_owner(),
        {
            let dropped = Arc::clone(&dropped);
            async move {
                let _guard = DropFlag(dropped);
                pending::<Result<()>>().await
            }
        },
        async {
            sleep(Duration::from_millis(10)).await;
            Ok(())
        },
    )
    .await
    .expect_err("interrupt pending non-write command");

    assert!(matches!(err, Error::Interrupted));
    assert!(dropped.load(Ordering::SeqCst));
}
