use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use crate::zapret::{self, ConfigEntry};
use crate::net::{self, Request, Tls};
use crate::parallel;

const PLACEHOLDER: &str = "domain.example.abc";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Proxy,
    Exclude,
}

impl ListKind {
    fn file(self) -> &'static str {
        match self {
            ListKind::Proxy => "list-general-user.txt",
            ListKind::Exclude => "list-exclude-user.txt",
        }
    }

    fn path(self, data: &Path) -> PathBuf {
        data.join("lists").join(self.file())
    }
}

pub fn normalize(input: &str) -> Option<String> {
    let s = input.trim().to_lowercase();
    let s = s.split("://").last().unwrap_or("");
    let s = s.split(['/', '?', '#']).next().unwrap_or("");
    let s = s.rsplit('@').next().unwrap_or("");
    let s = s.split(':').next().unwrap_or("").trim_start_matches("*.").trim_matches('.');
    let valid = !s.is_empty()
        && s.contains('.')
        && s.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_');
    valid.then(|| s.to_string())
}

pub fn parse_input(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for token in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        if let Some(d) = normalize(token) {
            if !out.contains(&d) {
                out.push(d);
            }
        }
    }
    out
}

pub fn read_list(data: &Path, kind: ListKind) -> Vec<String> {
    fs::read_to_string(kind.path(data))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && *l != PLACEHOLDER)
        .map(str::to_string)
        .collect()
}

pub fn write_list(data: &Path, kind: ListKind, domains: &[String]) -> std::io::Result<()> {
    let mut text = String::new();
    if kind == ListKind::Proxy {
        text.push_str("# Never leave this file empty\r\n");
    }
    if domains.is_empty() {
        text.push_str(PLACEHOLDER);
        text.push_str("\r\n");
    }
    for d in domains {
        text.push_str(d);
        text.push_str("\r\n");
    }
    fs::write(kind.path(data), text)
}

pub fn add_domains(data: &Path, kind: ListKind, new: &[String]) -> std::io::Result<usize> {
    let mut list = read_list(data, kind);
    let before = list.len();
    for d in new {
        if !list.contains(d) {
            list.push(d.clone());
        }
    }
    write_list(data, kind, &list)?;
    Ok(list.len() - before)
}

pub fn remove_domain(data: &Path, kind: ListKind, domain: &str) -> std::io::Result<()> {
    let list: Vec<String> = read_list(data, kind).into_iter().filter(|d| d != domain).collect();
    write_list(data, kind, &list)
}

pub struct SiteResult {
    pub domain: String,
    pub ok: bool,
    pub ms: u128,
    pub detail: String,
}

pub enum Event {
    Result(SiteResult),
    Done,
}

pub struct Handle {
    pub rx: Receiver<Event>,
    pub cancel: Arc<AtomicBool>,
}

fn check(domain: &str, timeout: u64) -> SiteResult {
    let url = format!("https://{domain}");
    let resp = net::perform(&Request::head(&url, Tls::Auto, timeout));
    let ms = (resp.time * 1000.0) as u128;
    let (ok, detail) = match (&resp.fail, resp.code) {
        (None, code) => (true, format!("HTTP {}", code.unwrap_or(0))),
        (Some(f), _) => (false, f.describe()),
    };
    SiteResult { domain: domain.to_string(), ok, ms, detail }
}

pub struct Pause {
    pub data: PathBuf,
    pub resume: Option<ConfigEntry>,
}

pub fn spawn_check(domains: Vec<String>, workers: usize, timeout: u64, pause: Option<Pause>, ctx: egui::Context) -> Handle {
    let (tx, rx): (Sender<Event>, Receiver<Event>) = std::sync::mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    std::thread::spawn(move || {
        if pause.is_some() {
            zapret::stop();
        }
        parallel::map(&domains, workers, &flag, &|_| {}, &|d| {
            let _ = tx.send(Event::Result(check(d, timeout)));
            ctx.request_repaint();
        });
        if let Some(Pause { data, resume: Some(cfg) }) = &pause {
            let _ = zapret::start_config(data, cfg);
        }
        let _ = tx.send(Event::Done);
        ctx.request_repaint();
    });
    Handle { rx, cancel }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_domains() {
        let got = parse_input("https://YouTube.com/watch?v=1, discord.com\n*.example.org:8443 foo  localhost user@mail.ru");
        assert_eq!(got, ["youtube.com", "discord.com", "example.org", "mail.ru"]);
        let r = check("example.com", 5);
        println!("{} {} {}ms {}", r.domain, r.ok, r.ms, r.detail);
    }
}
