use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use crate::zapret::{self, ConfigEntry};
use crate::net::{self, Fail, Request, Tls};
use crate::{parallel, sys};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Standard,
    Dpi,
}

#[derive(Clone)]
pub struct Params {
    pub kind: Kind,
    pub parallel: usize,
    pub timeout: u64,
    pub dpi_range: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Check {
    Ok,
    Error,
    Ssl,
}

impl Check {
    pub fn label(self) -> &'static str {
        match self {
            Check::Ok => "OK",
            Check::Error => "ERR",
            Check::Ssl => "SSL",
        }
    }
}

pub struct StdResult {
    pub name: String,
    pub checks: Vec<(&'static str, Check)>,
    pub ping: Option<u32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DpiStatus {
    Ok,
    Fail,
    LikelyBlocked,
}

pub struct DpiLine {
    pub label: &'static str,
    pub code: String,
    pub up_kb: f64,
    pub down_kb: f64,
    pub time: f64,
    pub status: DpiStatus,
}

pub struct DpiResult {
    pub id: String,
    pub provider: String,
    pub country: String,
    pub lines: Vec<DpiLine>,
}

pub enum Outcome {
    Std(StdResult),
    Dpi(DpiResult),
}

pub struct Report {
    pub config: String,
    pub error: Option<String>,
    pub outcomes: Vec<Outcome>,
}

#[derive(Default, Clone, Copy)]
pub struct Summary {
    pub ok: i64,
    pub fail: i64,
    pub blocked: i64,
    pub ping_ok: i64,
    pub ping_fail: i64,
}

impl Report {
    pub fn summary(&self) -> Summary {
        let mut s = Summary::default();
        for outcome in &self.outcomes {
            match outcome {
                Outcome::Std(r) => {
                    for (_, c) in &r.checks {
                        match c {
                            Check::Ok => s.ok += 1,
                            Check::Error | Check::Ssl => s.fail += 1,
                        }
                    }
                    if r.ping.is_some() {
                        s.ping_ok += 1;
                    } else {
                        s.ping_fail += 1;
                    }
                }
                Outcome::Dpi(r) => {
                    for l in &r.lines {
                        match l.status {
                            DpiStatus::Ok => s.ok += 1,
                            DpiStatus::Fail => s.fail += 1,
                            DpiStatus::LikelyBlocked => s.blocked += 1,
                        }
                    }
                }
            }
        }
        s
    }

    pub fn score(&self) -> (i64, i64) {
        let s = self.summary();
        (s.ok, s.ping_ok - s.blocked)
    }
}

pub enum Event {
    Log(String),
    ConfigStart { index: usize, total: usize, name: String },
    Progress { done: usize, total: usize },
    Report(Report),
    Finished { cancelled: bool },
}

pub struct Handle {
    pub rx: std::sync::mpsc::Receiver<Event>,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Clone)]
struct Tx {
    tx: Sender<Event>,
    ctx: egui::Context,
}

impl Tx {
    fn send(&self, e: Event) {
        let _ = self.tx.send(e);
        self.ctx.request_repaint();
    }
    fn log(&self, s: impl Into<String>) {
        self.send(Event::Log(s.into()));
    }
}

#[derive(Clone)]
pub struct Target {
    pub name: String,
    pub url: Option<String>,
    pub ping_host: String,
}

struct DpiTarget {
    id: String,
    provider: String,
    country: String,
    host: String,
}

pub fn load_targets(data: &Path) -> Vec<Target> {
    let text = fs::read_to_string(data.join("utils").join("targets.txt")).unwrap_or_default();
    let mut targets = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let name = key.trim().to_string();
        let value = value.trim().trim_matches('"').trim();
        if name.is_empty() || value.is_empty() {
            continue;
        }
        if let Some(ip) = value.strip_prefix("PING:") {
            targets.push(Target { name, url: None, ping_host: ip.trim().to_string() });
        } else {
            let host = value
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .split('/')
                .next()
                .unwrap_or("")
                .to_string();
            targets.push(Target { name, url: Some(value.to_string()), ping_host: host });
        }
    }
    if targets.is_empty() {
        for (name, url) in [
            ("DiscordMain", "https://discord.com"),
            ("YouTubeWeb", "https://www.youtube.com"),
            ("GoogleMain", "https://www.google.com"),
            ("CloudflareWeb", "https://www.cloudflare.com"),
        ] {
            targets.push(Target {
                name: name.into(),
                url: Some(url.into()),
                ping_host: url.trim_start_matches("https://").into(),
            });
        }
        for ip in ["1.1.1.1", "8.8.8.8", "9.9.9.9"] {
            targets.push(Target { name: format!("PING_{ip}"), url: None, ping_host: ip.into() });
        }
    }
    targets
}

const STD_TESTS: [(&str, Tls); 3] = [("HTTP", Tls::Auto), ("TLS1.2", Tls::V12), ("TLS1.3", Tls::V13)];

fn http_check(url: &str, tls: Tls, timeout: u64) -> Check {
    match net::perform(&Request::head(url, tls, timeout)).fail {
        None => Check::Ok,
        Some(Fail::Dns | Fail::Cert) => Check::Ssl,
        Some(_) => Check::Error,
    }
}

fn check_standard(t: &Target, timeout: u64) -> StdResult {
    let mut checks = Vec::new();
    if let Some(url) = &t.url {
        for (label, tls) in STD_TESTS {
            checks.push((label, http_check(url, tls, timeout)));
        }
    }
    StdResult { name: t.name.clone(), checks, ping: net::ping(&t.ping_host) }
}

fn fetch_dpi_suite(timeout: u64) -> Vec<DpiTarget> {
    let url = "https://hyperion-cs.github.io/dpi-checkers/ru/tcp-16-20/suite.v2.json";
    let resp = net::perform(&Request {
        url,
        head: false,
        tls: Tls::Auto,
        body: None,
        range: None,
        timeout: Duration::from_secs(timeout.max(8)),
        connect_timeout: Duration::from_secs(5),
        max_body: 4 * 1024 * 1024,
        keep_body: true,
    });
    let Ok(serde_json::Value::Array(items)) = serde_json::from_slice(&resp.body) else {
        return Vec::new();
    };
    let field = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    items
        .iter()
        .map(|v| DpiTarget { id: field(v, "id"), provider: field(v, "provider"), country: field(v, "country"), host: field(v, "host") })
        .filter(|t| !t.host.is_empty())
        .collect()
}

fn check_dpi(t: &DpiTarget, payload: &[u8], timeout: u64, range: u64) -> DpiResult {
    let url = format!("https://{}", t.host);
    let mut lines = Vec::new();
    for (label, tls) in STD_TESTS {
        let resp = net::perform(&Request {
            url: &url,
            head: false,
            tls,
            body: Some(payload),
            range: Some(range),
            timeout: Duration::from_secs(timeout),
            connect_timeout: Duration::from_secs(timeout.min(3)),
            max_body: range,
            keep_body: false,
        });
        let status = match &resp.fail {
            None => DpiStatus::Ok,
            Some(Fail::Timeout) if resp.up > 0 && resp.down == 0 && resp.time >= timeout as f64 => DpiStatus::LikelyBlocked,
            Some(_) => DpiStatus::Fail,
        };
        lines.push(DpiLine {
            label,
            code: resp.code.map(|c| c.to_string()).unwrap_or_else(|| "NA".into()),
            up_kb: resp.up as f64 / 1024.0,
            down_kb: resp.down as f64 / 1024.0,
            time: resp.time,
            status,
        });
    }
    DpiResult { id: t.id.clone(), provider: t.provider.clone(), country: t.country.clone(), lines }
}

struct IpsetGuard {
    file: PathBuf,
    backup: PathBuf,
    active: bool,
}

impl IpsetGuard {
    fn new(data: &Path) -> Self {
        let lists = data.join("lists");
        Self { file: lists.join("ipset-all.txt"), backup: lists.join("ipset-all.test-backup.txt"), active: false }
    }

    fn set_any(&mut self) {
        let current = fs::read_to_string(&self.file).unwrap_or_default();
        if current.trim().is_empty() {
            return;
        }
        if fs::copy(&self.file, &self.backup).is_ok() && fs::write(&self.file, "").is_ok() {
            self.active = true;
        }
    }

    fn restore(&mut self) {
        if self.backup.exists() && fs::copy(&self.backup, &self.file).is_ok() {
            let _ = fs::remove_file(&self.backup);
        }
        self.active = false;
    }
}

impl Drop for IpsetGuard {
    fn drop(&mut self) {
        if self.active {
            self.restore();
        }
    }
}

pub fn restore_leftover_ipset(data: &Path) {
    IpsetGuard::new(data).restore();
}

fn random_payload(len: usize) -> Vec<u8> {
    let mut state = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        | 1;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

pub fn spawn(
    data: PathBuf,
    configs: Vec<ConfigEntry>,
    params: Params,
    resume: Option<ConfigEntry>,
    ctx: egui::Context,
) -> Handle {
    let (tx, rx) = std::sync::mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let tx = Tx { tx, ctx };
    let flag = cancel.clone();
    std::thread::spawn(move || {
        run(&data, &configs, &params, resume, &tx, &flag);
        tx.send(Event::Finished { cancelled: flag.load(Ordering::Relaxed) });
    });
    Handle { rx, cancel }
}

fn run(data: &Path, configs: &[ConfigEntry], params: &Params, resume: Option<ConfigEntry>, tx: &Tx, cancel: &AtomicBool) {
    let mut ipset = IpsetGuard::new(data);
    let (targets, dpi_targets, payload) = match params.kind {
        Kind::Standard => (load_targets(data), Vec::new(), None),
        Kind::Dpi => {
            ipset.set_any();
            tx.log("Загрузка списка DPI-целей...");
            let suite = fetch_dpi_suite(params.timeout);
            if suite.is_empty() {
                tx.log("Не удалось загрузить список DPI-целей (нужен доступ в интернет без zapret).");
                return;
            }
            tx.log(format!("DPI-целей: {}", suite.len()));
            (Vec::new(), suite, Some(random_payload(params.dpi_range as usize)))
        }
    };

    let total_targets = if params.kind == Kind::Standard { targets.len() } else { dpi_targets.len() };

    for (index, cfg) in configs.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        tx.send(Event::ConfigStart { index, total: configs.len(), name: cfg.name.clone() });
        tx.send(Event::Progress { done: 0, total: total_targets });

        if let Err(e) = zapret::start_config(data, cfg) {
            tx.log(format!("[{}] не запустилась: {e}", cfg.name));
            tx.send(Event::Report(Report { config: cfg.name.clone(), error: Some(e), outcomes: Vec::new() }));
            continue;
        }
        std::thread::sleep(Duration::from_millis(400));

        let progress = |done: usize| tx.send(Event::Progress { done, total: total_targets });
        let outcomes: Vec<Outcome> = match params.kind {
            Kind::Standard => parallel::map(&targets, params.parallel, cancel, &progress, &|t| {
                Outcome::Std(check_standard(t, params.timeout))
            })
            .into_iter()
            .flatten()
            .collect(),
            Kind::Dpi => {
                let payload = payload.as_deref().unwrap_or_default();
                parallel::map(&dpi_targets, params.parallel, cancel, &progress, &|t| {
                    Outcome::Dpi(check_dpi(t, payload, params.timeout, params.dpi_range))
                })
                .into_iter()
                .flatten()
                .collect()
            }
        };
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        tx.send(Event::Report(Report { config: cfg.name.clone(), error: None, outcomes }));
    }

    zapret::stop();
    ipset.restore();
    if let Some(cfg) = resume {
        match zapret::start_config(data, &cfg) {
            Ok(()) => tx.log(format!("Восстановлена конфигурация {}", cfg.name)),
            Err(e) => tx.log(format!("Не удалось восстановить {}: {e}", cfg.name)),
        }
    }
}

pub fn render_report(kind: Kind, reports: &[Report]) -> String {
    let mut out = String::new();
    for r in reports {
        out.push_str(&format!("Config: {} (Type: {})\n", r.config, if kind == Kind::Standard { "standard" } else { "dpi" }));
        if let Some(e) = &r.error {
            out.push_str(&format!("  FAILED TO START: {e}\n\n"));
            continue;
        }
        for o in &r.outcomes {
            match o {
                Outcome::Std(s) => {
                    let http: Vec<String> = s.checks.iter().map(|(l, c)| format!("{l}:{}", c.label())).collect();
                    let ping = s.ping.map(|p| format!("{p} ms")).unwrap_or_else(|| "Timeout".into());
                    out.push_str(&format!("  {} : {} | Ping: {ping}\n", s.name, http.join(" ")));
                }
                Outcome::Dpi(d) => {
                    out.push_str(&format!("  Target: [{}] {} ({})\n", d.country, d.id, d.provider));
                    for l in &d.lines {
                        out.push_str(&format!(
                            "    {}: code={} up={:.1} KB down={:.1} KB time={:.2}s status={}\n",
                            l.label,
                            l.code,
                            l.up_kb,
                            l.down_kb,
                            l.time,
                            match l.status {
                                DpiStatus::Ok => "OK",
                                DpiStatus::Fail => "FAIL",
                                DpiStatus::LikelyBlocked => "LIKELY_BLOCKED",
                            }
                        ));
                    }
                }
            }
        }
        out.push('\n');
    }
    out.push_str("=== ANALYTICS ===\n");
    let mut best: Option<(&str, (i64, i64))> = None;
    for r in reports.iter().filter(|r| r.error.is_none()) {
        let s = r.summary();
        out.push_str(&format!(
            "{} : OK: {}, FAIL: {}, BLOCKED: {}, Ping OK: {}, Ping fail: {}\n",
            r.config, s.ok, s.fail, s.blocked, s.ping_ok, s.ping_fail
        ));
        if best.is_none_or(|(_, sc)| r.score() > sc) {
            best = Some((&r.config, r.score()));
        }
    }
    if let Some((name, _)) = best {
        out.push_str(&format!("Best strategy: {name}\n"));
    }
    out
}

pub fn save_report(data: &Path, kind: Kind, reports: &[Report]) -> std::io::Result<PathBuf> {
    let dir = data.join("utils").join("test results");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("test_results_{}.txt", sys::timestamp()));
    fs::write(&path, render_report(kind, reports))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_primitives() {
        println!("ping 1.1.1.1 = {:?}", net::ping("1.1.1.1"));
        for (label, tls) in STD_TESTS {
            println!("{label} example.com = {:?}", http_check("https://example.com", tls, 5));
        }
        println!("bad host = {:?}", http_check("https://nonexistent.invalid", Tls::Auto, 3));
        let r = net::perform(&Request::head("https://example.com", Tls::V13, 5));
        println!("head: code={:?} fail={:?} t={:.2}", r.code, r.fail, r.time);
        let payload = random_payload(65536);
        let d = check_dpi(&DpiTarget { id: "x".into(), provider: "p".into(), country: "c".into(), host: "example.com".into() }, &payload, 5, 65536);
        for l in &d.lines {
            println!("dpi {} code={} up={:.1} down={:.1} t={:.2} ok={}", l.label, l.code, l.up_kb, l.down_kb, l.time, l.status == DpiStatus::Ok);
        }
        let suite = fetch_dpi_suite(8);
        println!("dpi suite: {}", suite.len());
        let data = zapret::find_data_dir("").unwrap();
        let t = load_targets(&data);
        println!("targets: {}", t.len());
        assert_eq!(t.len(), 16);
        let std = check_standard(&t[0], 4);
        println!("{} {:?} {:?}", std.name, std.checks, std.ping);
    }
}
