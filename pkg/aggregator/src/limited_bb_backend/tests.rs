use super::*;
use async_trait::async_trait;
use barretenberg_interface::BbBackend;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

struct TestBackend {
    concurrent: AtomicUsize,
    max_allowed: usize,
}

impl TestBackend {
    fn new(max_allowed: usize) -> Self {
        Self {
            concurrent: AtomicUsize::new(0),
            max_allowed,
        }
    }
}

#[async_trait]
impl BbBackend for TestBackend {
    async fn prove(
        &self,
        _program: &[u8],
        _bytecode: &[u8],
        _key: &[u8],
        _witness: &[u8],
        _oracle: bool,
    ) -> barretenberg_interface::error::Result<Vec<u8>> {
        self.enter();
        tokio::time::sleep(Duration::from_millis(50)).await;
        self.exit();
        Ok(Vec::new())
    }

    async fn verify(
        &self,
        _proof: &[u8],
        _public_inputs: &[u8],
        _key: &[u8],
        _oracle: bool,
    ) -> barretenberg_interface::error::Result<()> {
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
}

impl TestBackend {
    fn enter(&self) {
        let current = self.concurrent.fetch_add(1, Ordering::SeqCst);
        assert!(
            current < self.max_allowed,
            "backend concurrency exceeded limit"
        );
    }

    fn exit(&self) {
        self.concurrent.fetch_sub(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn limits_parallel_prove_calls() {
    let inner: Arc<dyn BbBackend> = Arc::new(TestBackend::new(1));
    let backend = Arc::new(LimitedBbBackend::new(inner, 1));

    let mut handles = Vec::new();
    for _ in 0..2 {
        let backend = Arc::clone(&backend);
        handles.push(tokio::spawn(async move {
            backend
                .prove(&[], &[], &[], &[], false)
                .await
                .expect("prove succeeds");
        }));
    }

    for handle in handles {
        handle.await.expect("task join succeeds");
    }
}

#[tokio::test]
async fn verify_not_limited() {
    let inner: Arc<dyn BbBackend> = Arc::new(TestBackend::new(1));
    let backend = Arc::new(LimitedBbBackend::new(inner, 1));

    let b = Arc::clone(&backend);
    let prove_handle = tokio::spawn(async move {
        b.prove(&[], &[], &[], &[], false)
            .await
            .expect("prove succeeds");
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let mut handles = Vec::new();
    for _ in 0..5 {
        let b = Arc::clone(&backend);
        handles.push(tokio::spawn(async move {
            b.verify(&[], &[], &[], false)
                .await
                .expect("verify succeeds");
        }));
    }

    for handle in handles {
        handle.await.expect("verify task join succeeds");
    }

    prove_handle.await.expect("prove task join succeeds");
}

#[tokio::test]
async fn respects_priority() {
    let inner: Arc<dyn BbBackend> = Arc::new(TestBackend::new(1));
    let base_backend = Arc::new(LimitedBbBackend::new(inner, 1));
    let results = Arc::new(Mutex::new(Vec::new()));

    let b = base_backend.with_priority(10);
    let r = Arc::clone(&results);
    tokio::spawn(async move {
        b.prove(&[], &[], &[], &[], false).await.unwrap();
        r.lock().unwrap().push(10);
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    let b = base_backend.with_priority(1);
    let r = Arc::clone(&results);
    tokio::spawn(async move {
        b.prove(&[], &[], &[], &[], false).await.unwrap();
        r.lock().unwrap().push(1);
    });

    let b = base_backend.with_priority(5);
    let r = Arc::clone(&results);
    tokio::spawn(async move {
        b.prove(&[], &[], &[], &[], false).await.unwrap();
        r.lock().unwrap().push(5);
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(*results.lock().unwrap(), vec![10, 1, 5]);
}
