use std::sync::mpsc::{SyncSender, sync_channel};
use std::thread::JoinHandle;

use super::desync::Engine;

const QUEUE_DEPTH: usize = 2048;

pub fn shard_count() -> usize {
    if let Some(n) = std::env::var("ZAPRET_GUI_SHARDS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
    {
        return n.clamp(1, 16);
    }
    std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, 4)
}

/// Worker threads, each owning a private `Engine`. Jobs with the same hash always
/// reach the same worker, so per-connection state never needs locking.
pub struct Pool<J: Send + 'static> {
    senders: Vec<SyncSender<J>>,
    threads: Vec<JoinHandle<()>>,
}

impl<J: Send + 'static> Pool<J> {
    pub fn new<H>(
        engines: Vec<Engine>,
        mut make_handler: impl FnMut(usize) -> H,
    ) -> Result<Self, String>
    where
        H: FnMut(&mut Engine, J) + Send + 'static,
    {
        let mut pool = Pool {
            senders: Vec::new(),
            threads: Vec::new(),
        };
        for (i, mut engine) in engines.into_iter().enumerate() {
            let (tx, rx) = sync_channel::<J>(QUEUE_DEPTH);
            let mut handler = make_handler(i);
            let t = std::thread::Builder::new()
                .name(format!("zapret-shard-{i}"))
                .spawn(move || {
                    while let Ok(job) = rx.recv() {
                        handler(&mut engine, job);
                    }
                })
                .map_err(|e| e.to_string())?;
            pool.senders.push(tx);
            pool.threads.push(t);
        }
        Ok(pool)
    }

    pub fn dispatch(&self, hash: u64, job: J) {
        let _ = self.senders[(hash % self.senders.len() as u64) as usize].send(job);
    }
}

impl<J: Send + 'static> Drop for Pool<J> {
    fn drop(&mut self) {
        self.senders.clear();
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}
