use std::time::Duration;
use tokio::runtime::{self, Runtime};
use tokio::time::{self, Interval};

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

    pub fn start(&mut self, interval: u64) {
        let mut interval = time::interval(Duration::from_secs(interval));
        self.rt.spawn(async move {
            loop {
                interval.tick().await;
                println!("now running on a worker thread");
            }
        });
    }
}
