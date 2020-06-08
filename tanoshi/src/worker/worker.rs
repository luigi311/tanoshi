use std::time::Duration;
use tokio::runtime::{self, Runtime};
use tokio::time::{self, delay_for};

pub struct Worker {
    pub rt: Runtime,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            rt: runtime::Builder::new()
                .threaded_scheduler()
                .build()
                .unwrap(),
        }
    }

    pub fn start_interval<F>(&self, interval: u64, f: F)
    where
        F: FnOnce() + Send + Copy + 'static,
    {
        self.rt.spawn(async move {
            loop {
                f();
                delay_for(Duration::from_secs(interval));
            }
        });
    }

    pub fn start_once<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.rt.spawn(async move { f() });
    }
}
