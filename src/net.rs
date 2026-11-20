use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::{Duration, Instant};

use crate::sys;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tls {
    Auto,
    V12,
    V13,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Fail {
    Cancelled,
    Dns,
    Timeout,
    Reset,
    Cert,
    Refused,
    Other(String),
}

impl Fail {
    pub fn describe(&self) -> String {
        match self {
            Fail::Cancelled => "отменено".into(),
            Fail::Dns => "DNS не резолвится".into(),
            Fail::Timeout => "таймаут".into(),
            Fail::Reset => "соединение сброшено".into(),
            Fail::Cert => "ошибка сертификата".into(),
            Fail::Refused => "отказ в соединении".into(),
            Fail::Other(s) => s.clone(),
        }
    }
}

pub struct Request<'a> {
    pub url: &'a str,
    pub head: bool,
    pub tls: Tls,
    pub body: Option<&'a [u8]>,
    pub range: Option<u64>,
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub max_body: u64,
    pub keep_body: bool,
    pub cancel: Option<Arc<AtomicBool>>,
}

impl<'a> Request<'a> {
    pub fn head(url: &'a str, tls: Tls, timeout: u64) -> Self {
        Self {
            url,
            head: true,
            tls,
            body: None,
            range: None,
            timeout: Duration::from_secs(timeout),
            connect_timeout: Duration::from_secs(timeout.min(2)),
            max_body: 0,
            keep_body: false,
            cancel: None,
        }
    }

    pub fn with_cancel(mut self, cancel: &Arc<AtomicBool>) -> Self {
        self.cancel = Some(cancel.clone());
        self
    }
}

#[derive(Default)]
pub struct Response {
    pub code: Option<u16>,
    pub up: u64,
    pub down: u64,
    pub time: f64,
    pub body: Vec<u8>,
    pub fail: Option<Fail>,
}

fn config(tls: Tls) -> Arc<ClientConfig> {
    static CONFIGS: OnceLock<[Arc<ClientConfig>; 3]> = OnceLock::new();
    let all = CONFIGS.get_or_init(|| {
        let build = |versions: &[&'static rustls::SupportedProtocolVersion]| {
            let roots = RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            Arc::new(
                ClientConfig::builder_with_provider(provider)
                    .with_protocol_versions(versions)
                    .expect("tls versions")
                    .with_root_certificates(roots)
                    .with_no_client_auth(),
            )
        };
        [
            build(rustls::DEFAULT_VERSIONS),
            build(&[&rustls::version::TLS12]),
            build(&[&rustls::version::TLS13]),
        ]
    });
    all[tls as usize].clone()
}

struct Target {
    https: bool,
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Option<Target> {
    let (https, rest) = match url.strip_prefix("https://") {
        Some(r) => (true, r),
        None => match url.strip_prefix("http://") {
            Some(r) => (false, r),
            None => (true, url),
        },
    };
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], rest[i..].to_string()),
        None => (rest, "/".to_string()),
    };
    let default_port = if https { 443 } else { 80 };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => match p.parse::<u16>() {
            Ok(p) => (h, p),
            Err(_) => (hostport, default_port),
        },
        None => (hostport, default_port),
    };
    (!host.is_empty()).then(|| Target {
        https,
        host: host.to_string(),
        port,
        path,
    })
}

fn map_io(e: &io::Error) -> Fail {
    if let Some(re) = e.get_ref().and_then(|i| i.downcast_ref::<rustls::Error>()) {
        return match re {
            rustls::Error::InvalidCertificate(_) => Fail::Cert,
            other => Fail::Other(format!("TLS: {other}")),
        };
    }
    match e.kind() {
        ErrorKind::TimedOut | ErrorKind::WouldBlock => Fail::Timeout,
        ErrorKind::ConnectionReset
        | ErrorKind::ConnectionAborted
        | ErrorKind::BrokenPipe
        | ErrorKind::UnexpectedEof => Fail::Reset,
        ErrorKind::ConnectionRefused => Fail::Refused,
        _ => Fail::Other(e.to_string()),
    }
}

struct Deadline<'a> {
    sock: &'a TcpStream,
    at: Instant,
    cancel: Option<&'a AtomicBool>,
}

impl Deadline<'_> {
    fn arm(&self) -> Result<(), Fail> {
        if self.cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(Fail::Cancelled);
        }
        let left = self.at.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(Fail::Timeout);
        }
        let poll = self
            .cancel
            .map_or(left, |_| left.min(Duration::from_millis(100)));
        let _ = self.sock.set_read_timeout(Some(poll));
        let _ = self.sock.set_write_timeout(Some(poll));
        Ok(())
    }
}

pub fn perform(req: &Request) -> Response {
    let started = Instant::now();
    let mut resp = Response::default();
    if let Err(f) = run(req, started + req.timeout, &mut resp) {
        resp.fail = Some(f);
    }
    resp.time = started.elapsed().as_secs_f64();
    resp
}

fn resolve(target: &Target, req: &Request, deadline: Instant) -> Result<Vec<SocketAddr>, Fail> {
    let Some(cancel) = req.cancel.as_deref() else {
        return (target.host.as_str(), target.port)
            .to_socket_addrs()
            .map(|a| a.collect())
            .map_err(|_| Fail::Dns);
    };
    let (tx, rx) = mpsc::channel();
    let (host, port) = (target.host.clone(), target.port);
    std::thread::spawn(move || {
        let _ = tx.send(
            (host.as_str(), port)
                .to_socket_addrs()
                .map(|a| a.collect::<Vec<_>>()),
        );
    });
    loop {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(Ok(addrs)) => return Ok(addrs),
            Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Disconnected) => return Err(Fail::Dns),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if cancel.load(Ordering::Relaxed) {
                    return Err(Fail::Cancelled);
                }
                if Instant::now() >= deadline {
                    return Err(Fail::Timeout);
                }
            }
        }
    }
}

fn connect(target: &Target, req: &Request, deadline: Instant) -> Result<TcpStream, Fail> {
    let mut addrs = resolve(target, req, deadline)?;
    if addrs.is_empty() {
        return Err(Fail::Dns);
    }
    addrs.sort_by_key(|a| a.is_ipv6());
    let mut last = Fail::Timeout;
    for addr in addrs {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(Fail::Timeout);
        }
        let connect_limit = req.connect_timeout.min(left);
        let result = match req.cancel.as_deref() {
            None => TcpStream::connect_timeout(&addr, connect_limit),
            Some(cancel) => {
                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let _ = tx.send(TcpStream::connect_timeout(&addr, connect_limit));
                });
                loop {
                    match rx.recv_timeout(Duration::from_millis(20)) {
                        Ok(r) => break r,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if cancel.load(Ordering::Relaxed) {
                                return Err(Fail::Cancelled);
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return Err(Fail::Other("connect".into()));
                        }
                    }
                }
            }
        };
        match result {
            Ok(s) => return Ok(s),
            Err(e) => last = map_io(&e),
        }
    }
    Err(last)
}

fn run(req: &Request, deadline: Instant, resp: &mut Response) -> Result<(), Fail> {
    let target = parse_url(req.url).ok_or_else(|| Fail::Other("некорректный URL".into()))?;
    let tcp = connect(&target, req, deadline)?;
    let _ = tcp.set_nodelay(true);
    let ctl = tcp.try_clone().map_err(|e| map_io(&e))?;
    let dl = Deadline {
        sock: &ctl,
        at: deadline,
        cancel: req.cancel.as_deref(),
    };

    if target.https {
        let name = rustls::pki_types::ServerName::try_from(target.host.clone())
            .map_err(|_| Fail::Other("некорректное имя хоста".into()))?;
        let conn =
            ClientConnection::new(config(req.tls), name).map_err(|e| Fail::Other(e.to_string()))?;
        exchange(StreamOwned::new(conn, tcp), &target, req, &dl, resp)
    } else {
        exchange(tcp, &target, req, &dl, resp)
    }
}

fn exchange<S: Read + Write>(
    mut s: S,
    target: &Target,
    req: &Request,
    dl: &Deadline,
    resp: &mut Response,
) -> Result<(), Fail> {
    let method = if req.head {
        "HEAD"
    } else if req.body.is_some() {
        "POST"
    } else {
        "GET"
    };
    let mut head = format!(
        "{method} {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: zapret_gui\r\nAccept: */*\r\nAccept-Encoding: identity\r\nConnection: close\r\n",
        target.path, target.host
    );
    if let Some(n) = req.range {
        head.push_str(&format!("Range: bytes=0-{}\r\n", n.saturating_sub(1)));
    }
    if let Some(b) = req.body {
        head.push_str(&format!(
            "Content-Type: application/octet-stream\r\nContent-Length: {}\r\n",
            b.len()
        ));
    }
    head.push_str("\r\n");

    let write_result = (|| -> Result<(), Fail> {
        write_all(&mut s, head.as_bytes(), dl, None)?;
        if let Some(body) = req.body {
            write_all(&mut s, body, dl, Some(&mut resp.up))?;
        }
        retry(dl, || s.flush())
    })();

    let read_result = read_response(&mut s, req, dl, resp);
    match (write_result, read_result) {
        (_, Ok(())) if resp.code.is_some() => Ok(()),
        (Err(w), _) => Err(w),
        (Ok(()), r) => r,
    }
}

fn is_poll_timeout(e: &io::Error) -> bool {
    matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
}

fn retry<T>(dl: &Deadline, mut f: impl FnMut() -> io::Result<T>) -> Result<T, Fail> {
    loop {
        dl.arm()?;
        match f() {
            Ok(v) => return Ok(v),
            Err(e) if is_poll_timeout(&e) => {}
            Err(e) => return Err(map_io(&e)),
        }
    }
}

fn write_all<S: Write>(
    s: &mut S,
    mut data: &[u8],
    dl: &Deadline,
    mut sent: Option<&mut u64>,
) -> Result<(), Fail> {
    while !data.is_empty() {
        let n = retry(dl, || s.write(&data[..data.len().min(4096)]))?;
        if n == 0 {
            return Err(Fail::Reset);
        }
        data = &data[n..];
        if let Some(c) = sent.as_deref_mut() {
            *c += n as u64;
        }
    }
    Ok(())
}

fn read_response<S: Read>(
    s: &mut S,
    req: &Request,
    dl: &Deadline,
    resp: &mut Response,
) -> Result<(), Fail> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut header_end = None;
    let mut chunked = false;

    loop {
        if header_end.is_some() && (req.head || resp.down >= req.max_body) {
            break;
        }
        dl.arm()?;
        let n = match s.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if is_poll_timeout(&e) => continue,
            Err(e) if e.kind() == ErrorKind::UnexpectedEof && header_end.is_some() => break,
            Err(e) => return Err(map_io(&e)),
        };
        match header_end {
            None => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&buf[..pos]).to_lowercase();
                    resp.code = head.split_whitespace().nth(1).and_then(|c| c.parse().ok());
                    chunked = head.contains("transfer-encoding: chunked");
                    let rest = buf.split_off(pos + 4);
                    resp.down = rest.len() as u64;
                    buf.clear();
                    if req.keep_body {
                        resp.body = rest;
                    }
                    header_end = Some(pos);
                } else if buf.len() > 64 * 1024 {
                    return Err(Fail::Other("слишком длинные заголовки".into()));
                }
            }
            Some(_) => {
                resp.down += n as u64;
                if req.keep_body && resp.body.len() < 4 * 1024 * 1024 {
                    resp.body.extend_from_slice(&chunk[..n]);
                }
            }
        }
    }

    if header_end.is_none() {
        return Err(Fail::Reset);
    }
    if chunked && req.keep_body {
        resp.body = dechunk(&resp.body);
    }
    Ok(())
}

fn dechunk(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(rel) = data[pos..].windows(2).position(|w| w == b"\r\n") {
        let size = std::str::from_utf8(&data[pos..pos + rel])
            .ok()
            .and_then(|s| usize::from_str_radix(s.split(';').next().unwrap_or("").trim(), 16).ok());
        let Some(size) = size else { break };
        pos += rel + 2;
        if size == 0 || pos + size > data.len() {
            if size != 0 {
                out.extend_from_slice(&data[pos..]);
            }
            break;
        }
        out.extend_from_slice(&data[pos..pos + size]);
        pos += size + 2;
    }
    out
}

pub fn ping_cancellable(host: &str, cancel: &AtomicBool) -> Option<u32> {
    let mut cmd = sys::hidden("ping");
    if cfg!(windows) {
        cmd.args(["-n", "1", "-w", "1000", host]);
    } else if cfg!(target_os = "macos") {
        cmd.args(["-c", "1", "-W", "1000", host]);
    } else {
        cmd.args(["-c", "1", "-W", "1", host]);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = cmd.spawn().ok()?;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if cancel.load(Ordering::Relaxed) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
    let mut stdout = Vec::new();
    child.stdout.take()?.read_to_end(&mut stdout).ok()?;
    let text = String::from_utf8_lossy(&stdout);
    for line in text.lines() {
        let upper = line.to_ascii_uppercase();
        let Some(ttl) = upper.find("TTL=") else {
            continue;
        };
        if !cfg!(windows) {
            let pos = upper.find("TIME=")? + 5;
            let num: String = line[pos..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            return Some(
                num.parse::<f64>()
                    .map(|v| v.round() as u32)
                    .unwrap_or(1)
                    .max(1),
            );
        }
        let digits: String = line[..ttl]
            .chars()
            .rev()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let digits: String = digits.chars().rev().collect();
        return Some(digits.parse().unwrap_or(1).max(1));
    }
    None
}
