use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, future::Future, marker::PhantomData, pin::Pin, sync::Arc};

pub struct AsyncSpy<
    P: Clone + Send + Sync + 'static,
    R: Send + Sync + 'static,
    F = fn(&P) -> R,
    AF = fn(&P) -> Pin<Box<dyn Future<Output = R> + Send>>,
> {
    calls: Arc<Mutex<Vec<SpyCall<P>>>>,
    return_next: Arc<Mutex<VecDeque<AF>>>,
    default_return: Arc<F>,
    _phantom_data: PhantomData<R>,
}

impl<P: Clone, R, F, AF> AsyncSpy<P, R, F, AF>
where
    P: Clone + Send + Sync + 'static,
    R: Send + Sync + 'static,
    F: Fn(&P) -> R + Send + Sync + 'static,
    AF: Fn(&P) -> Pin<Box<dyn Future<Output = R> + Send>> + Send + Sync + 'static,
{
    /// Creates a new spy
    pub fn new(default_return: F) -> Self {
        Self {
            calls: Arc::new(Mutex::new(vec![])),
            return_next: Arc::new(Mutex::new(VecDeque::new())),
            default_return: Arc::new(default_return),
            _phantom_data: PhantomData,
        }
    }

    pub fn return_next(&self, fn_to_call: AF) {
        self.return_next.lock().push_back(fn_to_call);
    }

    pub async fn register_call(&self, params: P) -> R {
        let cloned_params = params.clone();
        let return_fn = self.return_next.lock().pop_front();

        let return_value = match return_fn {
            Some(v) => v(&cloned_params).await,
            None => {
                let default_fn = Arc::clone(&self.default_return);
                default_fn(&cloned_params)
            }
        };

        self.calls.lock().push(SpyCall::new(params));

        return_value
    }

    /// Resets the spy
    pub fn reset(&self) {
        self.calls.lock().clear();
        self.return_next.lock().clear();
    }

    /// Gets all calls to the spied fn so values can be asserted
    pub fn calls(&self) -> Vec<SpyCall<P>> {
        self.calls.lock().clone()
    }
}

pub struct Spy<P: Clone, R, F = fn(&P) -> R> {
    calls: Arc<Mutex<Vec<SpyCall<P>>>>,
    return_next: Arc<Mutex<VecDeque<R>>>,
    default_return: Arc<Mutex<F>>,
}

impl<P: Clone, R, F> Spy<P, R, F>
where
    F: Fn(&P) -> R + Send + Sync + 'static,
{
    /// Creates a new spy
    pub fn new(default_return: F) -> Self {
        Self {
            calls: Arc::new(Mutex::new(vec![])),
            return_next: Arc::new(Mutex::new(VecDeque::new())),
            default_return: Arc::new(Mutex::new(default_return)),
        }
    }

    /// Registers a call for the spy and returns the relevant return value
    pub fn register_call(&self, params: P) -> R {
        let return_value = match self.return_next.lock().pop_front() {
            Some(v) => v,
            None => {
                let default_fn = &*self.default_return.lock();
                default_fn(&params)
            }
        };

        self.calls.lock().push(SpyCall::new(params));

        return_value
    }

    pub fn register_call_return_next(&self, params: P) -> Option<R> {
        self.calls.lock().push(SpyCall::new(params));
        self.return_next.lock().pop_front()
    }

    /// Updates the default return value for the spy
    pub fn return_default(&self, new_default: F) {
        *self.default_return.lock() = new_default;
    }

    /// Adds a return value to be returned from the mock next
    pub fn return_next(&self, value: R) {
        self.return_next.lock().push_back(value);
    }

    /// Resets the spy
    pub fn reset(&self) {
        self.calls.lock().clear();
        self.return_next.lock().clear();
    }

    /// Gets all calls to the spied fn so values can be asserted
    pub fn calls(&self) -> Vec<SpyCall<P>> {
        self.calls.lock().clone()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpyCall<P: Clone> {
    pub params: P,
}

impl<P: Clone> SpyCall<P> {
    pub fn new(params: P) -> Self {
        Self { params }
    }
}
