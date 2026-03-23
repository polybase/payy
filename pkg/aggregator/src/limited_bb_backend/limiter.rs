use std::{collections::BTreeMap, sync::Mutex};
use tokio::sync::Notify;

pub(super) struct ConcurrencyLimiter {
    state: Mutex<LimiterState>,
    notify: Notify,
    max: usize,
}

struct LimiterState {
    in_flight: usize,
    waiters: BTreeMap<u64, usize>,
}

impl ConcurrencyLimiter {
    pub(super) fn new(max: usize) -> Self {
        Self {
            state: Mutex::new(LimiterState {
                in_flight: 0,
                waiters: BTreeMap::new(),
            }),
            notify: Notify::new(),
            max,
        }
    }

    pub(super) async fn acquire(&self, priority: u64) -> Permit<'_> {
        let mut waiter_guard = WaiterGuard::new(self, priority);
        loop {
            let notification = self.notify.notified();

            {
                let mut state = self.state.lock().expect("limiter mutex poisoned");

                let can_proceed = |state: &LimiterState, priority: u64| {
                    state.in_flight < self.max
                        && (state.waiters.is_empty()
                            || priority <= *state.waiters.keys().next().expect("waiters empty"))
                };

                if can_proceed(&state, priority) {
                    waiter_guard.disarm(&mut state);
                    state.in_flight += 1;
                    return Permit { limiter: self };
                }

                waiter_guard.arm(&mut state);
            }

            notification.await;
        }
    }

    fn release(&self) {
        let mut state = self.state.lock().expect("limiter mutex poisoned");
        debug_assert!(state.in_flight > 0, "release called without acquire");
        state.in_flight -= 1;
        self.notify.notify_waiters();
    }
}

pub(super) struct Permit<'a> {
    limiter: &'a ConcurrencyLimiter,
}

struct WaiterGuard<'a> {
    limiter: &'a ConcurrencyLimiter,
    priority: u64,
    active: bool,
}

impl<'a> WaiterGuard<'a> {
    fn new(limiter: &'a ConcurrencyLimiter, priority: u64) -> Self {
        Self {
            limiter,
            priority,
            active: false,
        }
    }

    fn arm(&mut self, state: &mut LimiterState) {
        if !self.active {
            *state.waiters.entry(self.priority).or_insert(0) += 1;
            self.active = true;
        }
    }

    fn disarm(&mut self, state: &mut LimiterState) {
        if !self.active {
            return;
        }

        if let Some(count) = state.waiters.get_mut(&self.priority) {
            if *count > 1 {
                *count -= 1;
            } else {
                state.waiters.remove(&self.priority);
            }
        }

        self.active = false;
    }
}

impl Drop for WaiterGuard<'_> {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        let mut notify = false;
        {
            let mut state = self.limiter.state.lock().expect("limiter mutex poisoned");
            if let Some(count) = state.waiters.get_mut(&self.priority) {
                if *count > 1 {
                    *count -= 1;
                } else {
                    state.waiters.remove(&self.priority);
                    notify = true;
                }
            }
            self.active = false;
        }

        if notify {
            self.limiter.notify.notify_waiters();
        }
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.limiter.release();
    }
}
