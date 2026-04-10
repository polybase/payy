use std::{
    future::pending,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::time::sleep;

use crate::{
    commands::{
        interactive::parse_line,
        interactive_interrupt::{
            InterruptOwner, delegate_current_interrupt_to_command, run_with_interrupt_owner,
        },
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
        "fetch https://api.example.com/paid",
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
        || async { Ok(()) },
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
        || async {
            sleep(Duration::from_millis(10)).await;
            Ok(())
        },
    )
    .await
    .expect_err("interrupt pending non-write command");

    assert!(matches!(err, Error::Interrupted));
    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn fetch_payment_flow_can_delegate_interrupts_to_command_handler() {
    let parsed = parse_line("fetch https://api.example.com/paid").expect("parse fetch command");
    let ran = Arc::new(AtomicBool::new(false));

    run_with_interrupt_owner(
        parsed.interrupt_owner(),
        {
            let ran = Arc::clone(&ran);
            async move {
                let _interrupt_guard = delegate_current_interrupt_to_command();
                sleep(Duration::from_millis(20)).await;
                ran.store(true, Ordering::SeqCst);
                Ok(())
            }
        },
        || async {
            sleep(Duration::from_millis(10)).await;
            Ok(())
        },
    )
    .await
    .expect("delegated fetch payment flow should own ctrl-c");

    assert!(ran.load(Ordering::SeqCst));
}

#[tokio::test]
async fn fetch_payment_flow_restores_repl_interrupts_after_payment_execution() {
    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let parsed = parse_line("fetch https://api.example.com/paid").expect("parse fetch command");
    let dropped = Arc::new(AtomicBool::new(false));
    let entered_retry = Arc::new(AtomicBool::new(false));
    let cancel_calls = Arc::new(AtomicUsize::new(0));

    let err = run_with_interrupt_owner(
        parsed.interrupt_owner(),
        {
            let dropped = Arc::clone(&dropped);
            let entered_retry = Arc::clone(&entered_retry);
            async move {
                let _guard = DropFlag(dropped);

                {
                    let _interrupt_guard = delegate_current_interrupt_to_command();
                    sleep(Duration::from_millis(20)).await;
                }

                entered_retry.store(true, Ordering::SeqCst);
                pending::<Result<()>>().await
            }
        },
        {
            let cancel_calls = Arc::clone(&cancel_calls);
            move || {
                let delay_ms = match cancel_calls.fetch_add(1, Ordering::SeqCst) {
                    0 => 10,
                    _ => 20,
                };

                async move {
                    sleep(Duration::from_millis(delay_ms)).await;
                    Ok(())
                }
            }
        },
    )
    .await
    .expect_err("interrupt restored fetch retry/download path");

    assert!(matches!(err, Error::Interrupted));
    assert!(entered_retry.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(cancel_calls.load(Ordering::SeqCst), 2);
}
