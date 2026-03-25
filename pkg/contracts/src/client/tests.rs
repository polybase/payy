use std::sync::{Arc, atomic::AtomicU16};

use web3::error::{Error, TransportError};

use super::ConfirmationType;

#[tokio::test]
async fn test_retry_on_network_failure() {
    let gen_result = |succeed_at_call_count| async move {
        let call_count = Arc::new(AtomicU16::new(0));

        super::retry_on_network_failure(move || {
            let call_count = Arc::clone(&call_count);
            async move {
                let call_count = call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if call_count == succeed_at_call_count {
                    Ok(())
                } else {
                    Err(Error::Transport(TransportError::Code(call_count)))
                }
            }
        })
        .await
    };

    {
        // Never succeed
        let start = std::time::Instant::now();
        let result = gen_result(u16::MAX).await;
        let elapsed = start.elapsed();

        assert!(
            matches!(&result, Err(Error::Transport(TransportError::Code(4)))),
            "{result:?}"
        );
        assert!(elapsed >= std::time::Duration::from_secs(16), "{elapsed:?}");
    }

    {
        // Succeed first try
        let start = std::time::Instant::now();
        let result = gen_result(1).await;
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "{result:?}");
        assert!(elapsed < std::time::Duration::from_millis(1), "{elapsed:?}");
    }
}

#[tokio::test]
async fn test_retry_on_network_failure_instant() {
    let gen_result = |succeed_at_call_count| async move {
        let call_count = Arc::new(AtomicU16::new(0));

        super::retry_on_network_failure_instant(move || {
            let call_count = Arc::clone(&call_count);
            async move {
                let call_count = call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if call_count == succeed_at_call_count {
                    Ok(())
                } else {
                    Err(Error::Transport(TransportError::Code(call_count)))
                }
            }
        })
        .await
    };

    {
        // Never succeed, but should fail fast
        let start = std::time::Instant::now();
        let result = gen_result(u16::MAX).await;
        let elapsed = start.elapsed();

        assert!(
            matches!(&result, Err(Error::Transport(TransportError::Code(4)))),
            "{result:?}"
        );
        // 4 attempts * 0s sleep ~= 0s. Allow some buffer for scheduling.
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "{elapsed:?}"
        );
    }
}

#[test]
fn test_confirmation_type_eq() {
    assert_eq!(ConfirmationType::Latest, ConfirmationType::Latest);
    assert_eq!(
        ConfirmationType::LatestPlus(5),
        ConfirmationType::LatestPlus(5)
    );
    assert_eq!(ConfirmationType::Finalised, ConfirmationType::Finalised);

    assert_ne!(ConfirmationType::Latest, ConfirmationType::LatestPlus(0));
    assert_ne!(
        ConfirmationType::LatestPlus(5),
        ConfirmationType::LatestPlus(10)
    );
    assert_ne!(ConfirmationType::Latest, ConfirmationType::Finalised);
}

#[test]
fn test_confirmation_type_clone() {
    let latest = ConfirmationType::Latest;
    let latest_cloned = latest.clone();
    assert_eq!(latest, latest_cloned);

    let latest_plus = ConfirmationType::LatestPlus(42);
    let latest_plus_cloned = latest_plus.clone();
    assert_eq!(latest_plus, latest_plus_cloned);

    let finalised = ConfirmationType::Finalised;
    let finalised_cloned = finalised.clone();
    assert_eq!(finalised, finalised_cloned);
}

#[test]
fn test_confirmation_type_debug() {
    let latest = ConfirmationType::Latest;
    let latest_debug = format!("{latest:?}");
    assert!(latest_debug.contains("Latest"));

    let latest_plus = ConfirmationType::LatestPlus(20);
    let latest_plus_debug = format!("{latest_plus:?}");
    assert!(latest_plus_debug.contains("LatestPlus"));
    assert!(latest_plus_debug.contains("20"));

    let finalised = ConfirmationType::Finalised;
    let finalised_debug = format!("{finalised:?}");
    assert!(finalised_debug.contains("Finalised"));
}

#[test]
fn duplicate_submission_messages_are_recognized() {
    for message in [
        "already known",
        "Transaction with the same hash was already imported.",
        "transaction already imported",
    ] {
        assert!(
            super::is_duplicate_submission_rpc_message(message),
            "{message}"
        );
    }
}

#[test]
fn unrelated_submission_messages_are_not_recognized() {
    for message in [
        "unknown transaction",
        "transaction is unknown",
        "known good transaction",
        "nonce too low",
        "replacement transaction underpriced",
        "insufficient funds for gas * price + value",
    ] {
        assert!(
            !super::is_duplicate_submission_rpc_message(message),
            "{message}"
        );
    }
}

#[test]
fn failed_submission_is_recovered_when_local_hash_is_observed() {
    assert!(super::should_recover_failed_submission(
        &non_rpc_error("nonce too low"),
        true,
    ));
}

#[test]
fn failed_submission_is_not_recovered_without_local_hash_or_duplicate_phrase() {
    assert!(!super::should_recover_failed_submission(
        &non_rpc_error("nonce too low"),
        false,
    ));
}

fn non_rpc_error(message: &str) -> Error {
    Error::InvalidResponse(message.to_string())
}
