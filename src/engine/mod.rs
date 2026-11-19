pub mod args;
pub mod conntrack;
pub mod desync;
pub mod fakes;
pub mod lists;
pub mod packet;
pub mod proto;
pub mod quic;
pub mod rng;
pub mod tcp;
pub mod tcp_modes;
pub mod udp;

#[cfg(test)]
mod tests;

#[cfg(windows)]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

static DEBUG: AtomicBool = AtomicBool::new(false);
static LOGGER: Mutex<Option<Sender<String>>> = Mutex::new(None);

pub fn debug_enabled() -> bool {
    DEBUG.load(Ordering::Relaxed)
}

pub fn log(msg: &str) {
    if let Ok(g) = LOGGER.lock() {
        if let Some(tx) = g.as_ref() {
            let _ = tx.send(msg.to_string());
        }
    }
}

pub struct Running {
    stop: Arc<AtomicBool>,
    shutdown: Box<dyn Fn() + Send + Sync>,
    thread: Option<JoinHandle<()>>,
}

impl Running {
    pub fn is_alive(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        (self.shutdown)();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

pub struct Spec {
    pub args: Vec<String>,
    pub bin_dir: PathBuf,
    pub log: Option<Sender<String>>,
    pub debug: bool,
}

pub fn start(spec: Spec) -> Result<Running, String> {
    *LOGGER.lock().unwrap() = spec.log.clone();
    DEBUG.store(spec.debug, Ordering::Relaxed);
    let params = args::parse(&spec.args)?;
    start_platform(params, &spec)
}

#[cfg(windows)]
fn start_platform(params: args::Params, spec: &Spec) -> Result<Running, String> {
    let filter = windows::make_filter(&params)?;
    crate::dlog!("windivert filter:\n{filter}");
    let div = windows::Divert::open(&spec.bin_dir, &filter)?;
    let stop = Arc::new(AtomicBool::new(false));
    let sd = div.shutdown_handle();
    let engine = desync::Engine::new(params);
    let stop2 = stop.clone();
    let thread = std::thread::Builder::new()
        .name("zapret-engine".into())
        .spawn(move || {
            let mut div = div;
            let mut engine = engine;
            div.run(&mut engine, &stop2);
        })
        .map_err(|e| e.to_string())?;
    Ok(Running { stop, shutdown: Box::new(move || sd()), thread: Some(thread) })
}

#[cfg(target_os = "linux")]
fn start_platform(params: args::Params, _spec: &Spec) -> Result<Running, String> {
    let q = linux::Queue::open(params.qnum, params.desync_fwmark)?;
    let stop = Arc::new(AtomicBool::new(false));
    let engine = desync::Engine::new(params);
    let stop2 = stop.clone();
    let thread = std::thread::Builder::new()
        .name("zapret-engine".into())
        .spawn(move || {
            let mut q = q;
            let mut engine = engine;
            q.run(&mut engine, &stop2);
        })
        .map_err(|e| e.to_string())?;
    Ok(Running { stop, shutdown: Box::new(|| {}), thread: Some(thread) })
}

#[cfg(not(any(windows, target_os = "linux")))]
fn start_platform(_params: args::Params, _spec: &Spec) -> Result<Running, String> {
    Err("Перехват пакетов на этой ОС не поддерживается".into())
}

pub fn supported() -> bool {
    cfg!(any(windows, target_os = "linux"))
}
