use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use parking_lot::Mutex;

pub struct Pool<T> {
    state: Arc<Mutex<PoolState<T>>>,
}

impl<T> Clone for Pool<T> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

struct PoolState<T> {
    values: VecDeque<T>,
    /// This uses an Unbounded channel so that `.send` in [PoolGuard::drop] is
    /// not async because it's not blocking.
    waiters: VecDeque<tokio::sync::mpsc::UnboundedSender<PoolGuard<T>>>,
}

pub struct PoolGuard<T> {
    // The pool this value should be returned to
    return_state: Arc<Mutex<PoolState<T>>>,
    // This is only an option so that we can take it out at Drop
    value: Option<T>,
}

impl<T> Pool<T> {
    pub fn new(values: VecDeque<T>) -> Self {
        Self {
            state: Arc::new(Mutex::new(PoolState {
                values,
                waiters: VecDeque::new(),
            })),
        }
    }

    pub async fn get(&self) -> PoolGuard<T> {
        let mut receiver = {
            let mut state = self.state.lock();
            if let Some(value) = state.values.pop_front() {
                return PoolGuard {
                    return_state: Arc::clone(&self.state),
                    value: Some(value),
                };
            }

            let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
            state.waiters.push_back(sender);

            receiver
        };

        #[allow(clippy::unwrap_used)]
        receiver.recv().await.unwrap()
    }
}

impl<T> Deref for PoolGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        #[allow(clippy::expect_used)]
        self.value.as_ref().expect("this will never be None")
    }
}

impl<T> DerefMut for PoolGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        #[allow(clippy::expect_used)]
        self.value.as_mut().expect("this will never be None")
    }
}

impl<T> Drop for PoolGuard<T> {
    fn drop(&mut self) {
        let mut state = self.return_state.lock();

        #[allow(clippy::expect_used)]
        let mut value = self.value.take().expect("this will never be None");

        while let Some(sender) = state.waiters.pop_front() {
            if sender.is_closed() {
                continue;
            }

            match sender.send(Self {
                return_state: Arc::clone(&self.return_state),
                value: Some(value),
            }) {
                Ok(_) => {
                    return;
                }
                Err(mut v) => {
                    value = v.0.value.take().expect("this will never be None");
                    continue;
                }
            }
        }

        // No one is waiting, so we can just return the value to the pool
        state.values.push_back(value);
    }
}

impl<T> std::fmt::Debug for PoolGuard<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PoolGuard")
            .field("value", &self.value)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool() {
        let pool = Arc::new(Pool::new(VecDeque::from(vec![1, 2, 3])));
        let guard = pool.get().await;
        assert_eq!(*guard, 1);
        drop(guard);
        let guard = pool.get().await;
        assert_eq!(*guard, 2);
        drop(guard);
        let guard = pool.get().await;
        assert_eq!(*guard, 3);
        drop(guard);

        let guard_1 = pool.get().await;
        let guard_2 = pool.get().await;
        let guard_3 = pool.get().await;

        assert_eq!(*guard_1, 1);
        assert_eq!(*guard_2, 2);
        assert_eq!(*guard_3, 3);

        let task = tokio::spawn(async move {
            // This should block until guard_2 and guard_3 is dropped.
            // It should then get the value 2 when guard_2's
            // drop calls sender.send to the waiting receiver
            // and the same for guard_3.
            let (guard_4, guard_5) = tokio::join!(pool.get(), pool.get());
            assert_eq!(*guard_4, 2);
            assert_eq!(*guard_5, 3);
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        drop(guard_2);
        drop(guard_3);
        task.await.unwrap();
    }
}
