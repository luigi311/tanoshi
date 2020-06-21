use std::time::Duration;
use tokio::runtime::{self, Runtime};
use tokio::time::delay_for;

pub struct Worker {
    pub rt: Runtime,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            rt: runtime::Builder::new()
                .threaded_scheduler()
                .enable_all()
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
                delay_for(Duration::from_secs(interval)).await;
            }
        });
    }

    pub fn remove_cache(&self, interval: u64) {
        if interval == 0 {
            return;
        }

        self.start_interval(interval * 84600, move || {
            if let Some(cache_dir) = dirs::home_dir() {
                let cache_dir = cache_dir.join(".tanoshi/cache");
                match std::fs::remove_dir_all(cache_dir) {
                    Ok(_) => {}
                    Err(e) => error!("error remove cache: {}", e),
                }
            }
        });
    }
}
