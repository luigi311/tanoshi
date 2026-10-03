use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use tokio::sync::oneshot;

/// Priority applies to waiting requests; running extension calls finish normally.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RequestPriority {
    #[default]
    High,
    Low,
}

pub(crate) struct RequestQueue {
    state: Mutex<QueueState>,
}

struct QueueState {
    available: usize,
    next_id: u64,
    closed: bool,
    high: BTreeMap<u64, Waiter>,
    low: BTreeMap<u64, Waiter>,
}

struct Waiter {
    ready: oneshot::Sender<()>,
    granted: Arc<AtomicBool>,
}

pub(crate) struct RequestPermit {
    queue: Arc<RequestQueue>,
    id: u64,
    priority: RequestPriority,
    granted: Arc<AtomicBool>,
}

#[derive(Debug)]
pub(crate) struct QueueClosed;

impl RequestQueue {
    pub(crate) fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(QueueState {
                available: capacity.max(1),
                next_id: 0,
                closed: false,
                high: BTreeMap::new(),
                low: BTreeMap::new(),
            }),
        })
    }

    pub(crate) async fn acquire(
        self: &Arc<Self>,
        priority: RequestPriority,
    ) -> Result<RequestPermit, QueueClosed> {
        let (ready, receiver) = oneshot::channel();
        let granted = Arc::new(AtomicBool::new(false));
        let permit = {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            if state.closed {
                return Err(QueueClosed);
            }
            let id = state.next_id;
            state.next_id += 1;
            state.waiters(priority).insert(
                id,
                Waiter {
                    ready,
                    granted: granted.clone(),
                },
            );
            state.dispatch();
            RequestPermit {
                queue: self.clone(),
                id,
                priority,
                granted,
            }
        };
        // This guard also removes a cancelled waiter or returns a slot granted
        // just before cancellation. No extension task has started at this point.
        receiver.await.map_err(|_| QueueClosed)?;
        Ok(permit)
    }

    pub(crate) fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.closed = true;
        state.high.clear();
        state.low.clear();
    }
}

impl QueueState {
    fn waiters(&mut self, priority: RequestPriority) -> &mut BTreeMap<u64, Waiter> {
        match priority {
            RequestPriority::High => &mut self.high,
            RequestPriority::Low => &mut self.low,
        }
    }

    fn dispatch(&mut self) {
        while !self.closed && self.available > 0 {
            let Some((_, waiter)) = self.high.pop_first().or_else(|| self.low.pop_first()) else {
                break;
            };
            self.available -= 1;
            waiter.granted.store(true, Ordering::Release);
            // Send only a wakeup, so a cancelled receiver cannot drop a permit
            // and try to acquire this same mutex while dispatch holds it.
            if waiter.ready.send(()).is_err() {
                waiter.granted.store(false, Ordering::Release);
                self.available += 1;
            }
        }
    }
}

impl Drop for RequestPermit {
    fn drop(&mut self) {
        let mut state = self
            .queue
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.granted.load(Ordering::Acquire) {
            state.available += 1;
        } else {
            state.waiters(self.priority).remove(&self.id);
        }
        state.dispatch();
    }
}

#[cfg(test)]
mod tests;
