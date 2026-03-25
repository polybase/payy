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
    error::{Error, Result},
    output::{
        OutputMode, balance_message, confirmed_transaction_message, dropped_transaction_message,
        pending_transaction_message, should_render_loading, with_interrupt, with_loading_interrupt,
    },
};

#[test]
fn loading_indicator_only_renders_for_default_terminal_output() {
    assert!(should_render_loading(OutputMode::Default, true));
    assert!(!should_render_loading(OutputMode::Default, false));
    assert!(!should_render_loading(OutputMode::Json, true));
    assert!(!should_render_loading(OutputMode::Markdown, true));
    assert!(!should_render_loading(OutputMode::Compact, true));
}

#[test]
fn confirmed_transaction_message_includes_transaction_details() {
    let message = confirmed_transaction_message("Confirmed transfer of 1 ETH", "0xabc", Some(42));

    assert_eq!(message, "Confirmed transfer of 1 ETH\nTx: 0xabc\nBlock: 42");
}

#[test]
fn confirmed_transaction_message_handles_unknown_block_numbers() {
    let message = confirmed_transaction_message("Confirmed transfer of 1 ETH", "0xabc", None);

    assert_eq!(
        message,
        "Confirmed transfer of 1 ETH\nTx: 0xabc\nBlock: unknown"
    );
}

#[test]
fn pending_transaction_message_marks_pending_block_state() {
    let message = pending_transaction_message("Submitted transfer of 1 ETH", "0xabc", None);

    assert_eq!(
        message,
        "Submitted transfer of 1 ETH\nTx: 0xabc\nBlock: pending"
    );
}

#[test]
fn dropped_transaction_message_marks_last_seen_block_state() {
    let message = dropped_transaction_message("Dropped transfer of 1 ETH", "0xabc", None);

    assert_eq!(
        message,
        "Dropped transfer of 1 ETH\nTx: 0xabc\nLast seen block: pending"
    );
}

#[test]
fn balance_message_includes_the_resolved_address() {
    let message = balance_message("0 ETH", "0xabc");

    assert_eq!(message, "0 ETH\nAddress: 0xabc");
}

#[tokio::test]
async fn ctrl_c_interrupts_loading_requests() {
    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let dropped = Arc::new(AtomicBool::new(false));
    let err = with_loading_interrupt(
        OutputMode::Quiet,
        "Fetching balance...",
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
    .expect_err("interrupt pending loading request");

    assert!(matches!(err, Error::Interrupted));
    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn ctrl_c_interrupts_non_loading_requests() {
    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let dropped = Arc::new(AtomicBool::new(false));
    let err = with_interrupt(
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
    .expect_err("interrupt pending request");

    assert!(matches!(err, Error::Interrupted));
    assert!(dropped.load(Ordering::SeqCst));
}
