use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub fn map<T: Sync, R: Send>(
    items: &[T],
    workers: usize,
    cancel: &AtomicBool,
    on_done: &(dyn Fn(usize) + Sync),
    job: &(dyn Fn(&T) -> R + Sync),
) -> Vec<Option<R>> {
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());

    std::thread::scope(|scope| {
        for _ in 0..workers.clamp(1, items.len().max(1)) {
            scope.spawn(|| {
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let value = job(item);
                    results.lock().unwrap()[i] = Some(value);
                    on_done(done.fetch_add(1, Ordering::Relaxed) + 1);
                }
            });
        }
    });

    results.into_inner().unwrap()
}
