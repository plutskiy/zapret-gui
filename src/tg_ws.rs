use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use aes::Aes256;
use aes::cipher::{KeyIvInit, StreamCipher};
use ctr::Ctr128BE;
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, SinkExt, StreamExt};
use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::time::{sleep, timeout};
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{WebSocketStream, client_async};

type AesCtr = Ctr128BE<Aes256>;
type Ws = WebSocketStream<Conn>;
type PoolKey = (u32, bool, bool);

const HANDSHAKE_LEN: usize = 64;
const PROTO_POS: usize = 56;
const DC_POS: usize = 60;

const DC_IPS: [(u32, &str); 6] = [
    (1, "149.154.175.50"),
    (2, "149.154.167.51"),
    (3, "149.154.175.100"),
    (4, "149.154.167.91"),
    (5, "149.154.171.5"),
    (203, "91.105.192.100"),
];
const DC_TEST_IPS: [(u32, &str); 3] = [
    (1, "149.154.175.10"),
    (2, "149.154.167.40"),
    (3, "149.154.175.117"),
];
const WS_REDIRECT_IP: &str = "149.154.167.220";
const WS_REDIRECT_DCS: [u32; 2] = [2, 4];
const FRONTING_SNI: &str = "sprinthost.ru";
const CF_DC_IDS: [u32; 6] = [1, 2, 3, 4, 5, 203];

const POOL_SIZE: usize = 4;
const POOL_MAX_AGE: Duration = Duration::from_secs(120);
const POOL_CHECK_INTERVAL: Duration = Duration::from_secs(5);
const REFILL_BACKOFF_MAX: Duration = Duration::from_secs(3600);
const TCP_BACKOFF_INITIAL: Duration = Duration::from_secs(30);
const TCP_BACKOFF_MAX: Duration = Duration::from_secs(3600);
const CF_DOMAINS_MIN_VALID: usize = 3;
const RACE_CF_DOMAINS: usize = 6;
const RACE_CF_STAGGER: Duration = Duration::from_millis(500);
const RACE_TIMEOUT: Duration = Duration::from_secs(20);

const CF_DOMAINS: [&str; 20] = [
    "virkgj.com",
    "vmmzovy.com",
    "mkuosckvso.com",
    "zaewayzmplad.com",
    "twdmbzcm.com",
    "awzwsldi.com",
    "clngqrflngqin.com",
    "tjacxbqtj.com",
    "bxaxtxmrw.com",
    "dmohrsgmohcrwb.com",
    "vwbmtmoi.com",
    "khgrre.com",
    "ulihssf.com",
    "tmhqsdqmfpmk.com",
    "xwuwoqbm.com",
    "orgcnunpj.com",
    "zhkuldz.com",
    "zypoljnslxa.com",
    "efabnxaowuzs.com",
    "zaftuzsftqdq.com",
];
const CF_DOMAINS_HOST: &str = "raw.githubusercontent.com";
const CF_DOMAINS_IP: &str = "185.199.109.133";
const CF_DOMAINS_PATH: &str = "/Flowseal/tg-ws-proxy/main/.github/cfproxy-domains.txt";

struct PacketSplitter {
    proto: u32,
    cipher: Vec<u8>,
    plain: Vec<u8>,
    disabled: bool,
}

impl PacketSplitter {
    fn new(proto: u32) -> Self {
        Self {
            proto,
            cipher: Vec::new(),
            plain: Vec::new(),
            disabled: false,
        }
    }

    fn push(&mut self, cipher: &[u8], plain: &[u8]) -> Vec<Vec<u8>> {
        if self.disabled {
            return vec![cipher.to_vec()];
        }
        self.cipher.extend_from_slice(cipher);
        self.plain.extend_from_slice(plain);
        let mut result = Vec::new();
        let mut offset = 0;
        while offset < self.plain.len() {
            let Some(length) = self.packet_length(offset) else {
                break;
            };
            if length == 0 {
                result.push(self.cipher[offset..].to_vec());
                offset = self.cipher.len();
                self.disabled = true;
                break;
            }
            if self.plain.len() - offset < length {
                break;
            }
            result.push(self.cipher[offset..offset + length].to_vec());
            offset += length;
        }
        if offset != 0 {
            self.cipher.drain(..offset);
            self.plain.drain(..offset);
        }
        result
    }

    fn flush(&mut self) -> Option<Vec<u8>> {
        if self.cipher.is_empty() {
            None
        } else {
            let data = std::mem::take(&mut self.cipher);
            self.plain.clear();
            Some(data)
        }
    }

    fn packet_length(&self, offset: usize) -> Option<usize> {
        let available = self.plain.len().saturating_sub(offset);
        if self.proto == 0xEFEFEFEF {
            if available == 0 {
                return None;
            }
            let first = self.plain[offset];
            let (header, payload) = if first == 0x7f || first == 0xff {
                if available < 4 {
                    return None;
                }
                let words = u32::from_le_bytes([
                    self.plain[offset + 1],
                    self.plain[offset + 2],
                    self.plain[offset + 3],
                    0,
                ]);
                (4, words as usize * 4)
            } else {
                (1, (first & 0x7f) as usize * 4)
            };
            return Some(if payload == 0 { 0 } else { header + payload });
        }
        if self.proto == 0xEEEEEEEE || self.proto == 0xDDDDDDDD {
            if available < 4 {
                return None;
            }
            let payload =
                u32::from_le_bytes(self.plain[offset..offset + 4].try_into().ok()?) & 0x7fff_ffff;
            return Some(if payload == 0 {
                0
            } else {
                4 + payload as usize
            });
        }
        Some(0)
    }
}

fn decode_cf_domain(encoded: &str) -> String {
    let Some(prefix) = encoded.strip_suffix(".com") else {
        return encoded.to_string();
    };
    let shift = prefix.bytes().filter(|b| b.is_ascii_alphabetic()).count() as u8 % 26;
    let decoded: String = prefix
        .bytes()
        .map(|b| match b {
            b'a'..=b'z' => (b'a' + (b - b'a' + 26 - shift) % 26) as char,
            b'A'..=b'Z' => (b'A' + (b - b'A' + 26 - shift) % 26) as char,
            _ => b as char,
        })
        .collect();
    format!("{decoded}.co.uk")
}

fn valid_domain(domain: &str) -> bool {
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        && labels
            .last()
            .is_some_and(|t| t.len() >= 2 && t.bytes().any(|b| b.is_ascii_alphabetic()))
}

fn random_u32() -> u32 {
    let mut bytes = [0u8; 4];
    let _ = getrandom::fill(&mut bytes);
    u32::from_le_bytes(bytes)
}

fn shuffle<T>(items: &mut [T]) {
    for i in (1..items.len()).rev() {
        items.swap(i, random_u32() as usize % (i + 1));
    }
}

#[derive(Default)]
struct Balancer {
    domains: Vec<String>,
    current: HashMap<u32, String>,
    failed: HashMap<String, Instant>,
}

impl Balancer {
    fn update(&mut self, mut list: Vec<String>) {
        let mut a = self.domains.clone();
        a.sort();
        let mut b = list.clone();
        b.sort();
        if a == b || list.is_empty() {
            return;
        }
        self.current = CF_DC_IDS
            .iter()
            .map(|dc| (*dc, list[random_u32() as usize % list.len()].clone()))
            .collect();
        std::mem::swap(&mut self.domains, &mut list);
    }

    fn recently_failed(&self, domain: &str) -> bool {
        self.failed
            .get(domain)
            .is_some_and(|t| t.elapsed() < Duration::from_secs(300))
    }

    fn candidates(&self, dc: u32) -> Vec<String> {
        let current = self.current.get(&dc);
        let mut rest: Vec<String> = self
            .domains
            .iter()
            .filter(|d| Some(*d) != current)
            .cloned()
            .collect();
        shuffle(&mut rest);
        let mut result: Vec<String> = current.into_iter().cloned().collect();
        result.extend(rest);
        result.sort_by_key(|d| self.recently_failed(d));
        result
    }

    fn set(&mut self, dc: u32, domain: String) {
        self.failed.remove(&domain);
        self.current.insert(dc, domain);
    }
}

#[derive(Debug)]
struct IgnoreHostname(Arc<WebPkiServerVerifier>);

impl ServerCertVerifier for IgnoreHostname {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        match self
            .0
            .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
        {
            Err(rustls::Error::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => Ok(ServerCertVerified::assertion()),
            other => other,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.0.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.0.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_verify_schemes()
    }
}

fn tls_configs() -> Result<(Arc<ClientConfig>, Arc<ClientConfig>), String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = Arc::new(RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    });
    let err = |e: rustls::Error| format!("TLS: {e}");
    let strict = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(err)?
        .with_root_certificates(roots.clone())
        .with_no_client_auth();
    let inner = WebPkiServerVerifier::builder_with_provider(roots, provider.clone())
        .build()
        .map_err(|e| format!("TLS: {e}"))?;
    let fronting = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(err)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(IgnoreHostname(inner)))
        .with_no_client_auth();
    Ok((Arc::new(strict), Arc::new(fronting)))
}

#[derive(Default)]
struct Pool {
    idle: Mutex<HashMap<PoolKey, VecDeque<(Ws, Instant)>>>,
    refilling: Mutex<HashSet<PoolKey>>,
    backoff: Mutex<HashMap<PoolKey, (u32, Instant)>>,
    fronting_first: AtomicBool,
}

struct Shared {
    strict: Arc<ClientConfig>,
    fronting: Arc<ClientConfig>,
    pool: Pool,
    balancer: Mutex<Balancer>,
    tcp_backoff: Mutex<HashMap<String, (u32, Instant)>>,
    dns_cache: Mutex<HashMap<String, String>>,
    cf_plain_first: AtomicBool,
}

fn ws_domains(dc: u32, media: bool) -> Vec<String> {
    let dc = if dc == 203 { 2 } else { dc };
    if media {
        vec![
            format!("kws{dc}-1.web.telegram.org"),
            format!("kws{dc}.web.telegram.org"),
        ]
    } else {
        vec![format!("kws{dc}.web.telegram.org")]
    }
}

fn is_alive(ws: &mut Ws) -> bool {
    match ws.next().now_or_never() {
        None => true,
        Some(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => true,
        _ => false,
    }
}

struct Frag {
    inner: TcpStream,
    needle: Vec<u8>,
    split_done: bool,
}

impl Frag {
    fn new(inner: TcpStream, sni: &str) -> Self {
        Self {
            inner,
            needle: sni.as_bytes().to_vec(),
            split_done: false,
        }
    }
}

impl tokio::io::AsyncRead for Frag {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for Frag {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        let cut = if self.split_done || buf.first() != Some(&0x16) || buf.len() < 2 {
            buf.len()
        } else {
            let needle = &self.needle;
            buf.windows(needle.len().max(1))
                .position(|w| w == needle.as_slice())
                .map(|p| p + needle.len() / 2)
                .filter(|c| *c > 0 && *c < buf.len())
                .unwrap_or(1)
        };
        let result = std::pin::Pin::new(&mut self.inner).poll_write(cx, &buf[..cut]);
        if matches!(result, std::task::Poll::Ready(Ok(_))) && cut < buf.len() {
            self.split_done = true;
        }
        result
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

async fn stage<T, E: std::fmt::Display>(
    name: &str,
    secs: u64,
    fut: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, String> {
    match timeout(Duration::from_secs(secs), fut).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => Err(format!("{name}: {e}")),
        Err(_) => Err(format!("{name} timeout")),
    }
}

enum Conn {
    Tls(tokio_rustls::client::TlsStream<Frag>),
    Plain(TcpStream),
}

impl tokio::io::AsyncRead for Conn {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            Conn::Tls(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            Conn::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for Conn {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match self.get_mut() {
            Conn::Tls(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            Conn::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            Conn::Tls(s) => std::pin::Pin::new(s).poll_flush(cx),
            Conn::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            Conn::Tls(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            Conn::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
        }
    }
}

async fn ws_connect(
    shared: &Shared,
    ip: &str,
    host: &str,
    path: &str,
    sni: &str,
    fronted: bool,
    secure: bool,
) -> Result<Ws, String> {
    if !secure {
        let tcp = stage("tcp", 4, TcpStream::connect((ip, 80))).await?;
        let _ = tcp.set_nodelay(true);
        let mut request = format!("ws://{host}{path}")
            .into_client_request()
            .map_err(|e| format!("request: {e}"))?;
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", HeaderValue::from_static("binary"));
        let (ws, _) = stage("ws", 5, client_async(request, Conn::Plain(tcp))).await?;
        return Ok(ws);
    }
    let tcp = stage("tcp", 4, TcpStream::connect((ip, 443))).await?;
    let _ = tcp.set_nodelay(true);
    let name = ServerName::try_from(sni.to_string()).map_err(|e| format!("sni: {e}"))?;
    let config = if fronted {
        shared.fronting.clone()
    } else {
        shared.strict.clone()
    };
    let tls = stage(
        "tls",
        5,
        TlsConnector::from(config).connect(name, Frag::new(tcp, sni)),
    )
    .await?;
    let mut request = format!("wss://{host}{path}")
        .into_client_request()
        .map_err(|e| format!("request: {e}"))?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", HeaderValue::from_static("binary"));
    let (ws, _) = stage("ws", 5, client_async(request, Conn::Tls(tls))).await?;
    Ok(ws)
}

impl Shared {
    fn new() -> Result<Self, String> {
        let (strict, fronting) = tls_configs()?;
        let mut balancer = Balancer::default();
        balancer.update(CF_DOMAINS.iter().map(|d| decode_cf_domain(d)).collect());
        Ok(Self {
            strict,
            fronting,
            pool: Pool::default(),
            balancer: Mutex::new(balancer),
            tcp_backoff: Mutex::new(HashMap::new()),
            dns_cache: Mutex::new(HashMap::new()),
            cf_plain_first: AtomicBool::new(false),
        })
    }

    async fn connect_one(&self, ip: &str, domains: &[String], path: &str) -> Option<Ws> {
        for domain in domains {
            let first = self.pool.fronting_first.load(Ordering::Relaxed);
            for fronted in [first, !first] {
                let sni = if fronted {
                    FRONTING_SNI
                } else {
                    domain.as_str()
                };
                match ws_connect(self, ip, domain, path, sni, fronted, true).await {
                    Ok(ws) => {
                        self.pool.fronting_first.store(fronted, Ordering::Relaxed);
                        log_line(format!(
                            "WS {domain} via {ip} connected (fronting={fronted})"
                        ));
                        return Some(ws);
                    }
                    Err(error) => log_line(format!(
                        "WS {domain} via {ip} failed (fronting={fronted}): {error}"
                    )),
                }
            }
        }
        None
    }

    async fn refill(self: &Arc<Self>, key: PoolKey) {
        let (dc, media, test) = key;
        let needed = {
            let idle = self.pool.idle.lock().unwrap();
            POOL_SIZE.saturating_sub(idle.get(&key).map_or(0, |q| q.len()))
        };
        if needed == 0 {
            return;
        }
        let results =
            futures_util::future::join_all((0..needed).map(|_| self.race_connect(dc, media, test)))
                .await;
        let mut connected = 0;
        {
            let mut idle = self.pool.idle.lock().unwrap();
            let queue = idle.entry(key).or_default();
            for ws in results.into_iter().flatten() {
                queue.push_back((ws, Instant::now()));
                connected += 1;
            }
        }
        let mut backoff = self.pool.backoff.lock().unwrap();
        if connected > 0 {
            backoff.remove(&key);
        } else {
            let failures = backoff.get(&key).map_or(0, |b| b.0) + 1;
            let delay = Duration::from_secs(1u64 << (failures - 1).min(12)).min(REFILL_BACKOFF_MAX);
            backoff.insert(key, (failures, Instant::now() + delay));
        }
    }

    fn spawn_refill(self: &Arc<Self>, key: PoolKey) {
        if self
            .pool
            .backoff
            .lock()
            .unwrap()
            .get(&key)
            .is_some_and(|b| Instant::now() < b.1)
        {
            return;
        }
        if !self.pool.refilling.lock().unwrap().insert(key) {
            return;
        }
        let shared = self.clone();
        tokio::spawn(async move {
            shared.refill(key).await;
            shared.pool.refilling.lock().unwrap().remove(&key);
        });
    }

    fn pool_get(self: &Arc<Self>, key: PoolKey) -> Option<Ws> {
        let mut found = None;
        loop {
            let entry = self
                .pool
                .idle
                .lock()
                .unwrap()
                .get_mut(&key)
                .and_then(|q| q.pop_front());
            let Some((mut ws, created)) = entry else {
                break;
            };
            if created.elapsed() < POOL_MAX_AGE && is_alive(&mut ws) {
                found = Some(ws);
                break;
            }
        }
        self.spawn_refill(key);
        found
    }

    async fn maintain(self: Arc<Self>) {
        loop {
            let mut keys: HashSet<PoolKey> =
                self.pool.idle.lock().unwrap().keys().copied().collect();
            for dc in WS_REDIRECT_DCS {
                for media in [false, true] {
                    keys.insert((dc, media, false));
                }
            }
            for key in keys {
                {
                    let mut idle = self.pool.idle.lock().unwrap();
                    if let Some(queue) = idle.get_mut(&key) {
                        queue.retain_mut(|(ws, created)| {
                            created.elapsed() < POOL_MAX_AGE && is_alive(ws)
                        });
                    }
                }
                self.spawn_refill(key);
            }
            sleep(POOL_CHECK_INTERVAL).await;
        }
    }

    async fn refresh_cf_domains(self: Arc<Self>) {
        loop {
            match self.fetch_cf_domains().await {
                Ok(list) => {
                    let valid: Vec<String> = list
                        .into_iter()
                        .map(|d| d.trim().to_ascii_lowercase())
                        .filter(|d| valid_domain(d))
                        .collect();
                    if valid.len() >= CF_DOMAINS_MIN_VALID {
                        log_line(format!("CF domain list updated ({})", valid.len()));
                        self.balancer.lock().unwrap().update(valid);
                    }
                }
                Err(error) => log_line(format!("CF domain list refresh failed: {error}")),
            }
            sleep(Duration::from_secs(3600)).await;
        }
    }

    async fn fetch_cf_domains(&self) -> Result<Vec<String>, String> {
        let mut last = String::new();
        for ip in [CF_DOMAINS_IP, CF_DOMAINS_HOST] {
            let attempt = async {
                let tcp = TcpStream::connect((ip, 443))
                    .await
                    .map_err(|e| e.to_string())?;
                let name =
                    ServerName::try_from(CF_DOMAINS_HOST.to_string()).map_err(|e| e.to_string())?;
                let mut tls = TlsConnector::from(self.strict.clone())
                    .connect(name, tcp)
                    .await
                    .map_err(|e| e.to_string())?;
                let request = format!(
                    "GET {CF_DOMAINS_PATH} HTTP/1.1\r\nHost: {CF_DOMAINS_HOST}\r\n\
                     User-Agent: tg-ws-proxy\r\nConnection: close\r\n\r\n"
                );
                tls.write_all(request.as_bytes())
                    .await
                    .map_err(|e| e.to_string())?;
                let mut body = Vec::new();
                let _ = (&mut tls).take(65536).read_to_end(&mut body).await;
                let text = String::from_utf8_lossy(&body).into_owned();
                if !text.starts_with("HTTP/1.1 200") {
                    return Err(text.lines().next().unwrap_or("empty response").to_string());
                }
                let content = text.split_once("\r\n\r\n").map_or("", |(_, b)| b);
                Ok(content
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                    .map(decode_cf_domain)
                    .collect::<Vec<_>>())
            };
            match timeout(Duration::from_secs(10), attempt).await {
                Ok(Ok(list)) => return Ok(list),
                Ok(Err(e)) => last = e,
                Err(_) => last = "timeout".into(),
            }
        }
        Err(last)
    }

    async fn resolve_doh(&self, host: &str) -> Option<String> {
        if let Some(ip) = self.dns_cache.lock().unwrap().get(host) {
            return Some(ip.clone());
        }
        let attempt = async {
            let tcp = TcpStream::connect(("1.1.1.1", 443))
                .await
                .map_err(|e| e.to_string())?;
            let name = ServerName::try_from("1.1.1.1".to_string()).map_err(|e| e.to_string())?;
            let mut tls = TlsConnector::from(self.strict.clone())
                .connect(name, tcp)
                .await
                .map_err(|e| e.to_string())?;
            let request = format!(
                "GET /dns-query?name={host}&type=A HTTP/1.1\r\nHost: 1.1.1.1\r\n\
                 Accept: application/dns-json\r\nConnection: close\r\n\r\n"
            );
            tls.write_all(request.as_bytes())
                .await
                .map_err(|e| e.to_string())?;
            let mut body = Vec::new();
            let _ = (&mut tls).take(65536).read_to_end(&mut body).await;
            let text = String::from_utf8_lossy(&body).into_owned();
            let (start, end) = (text.find('{'), text.rfind('}'));
            let json: serde_json::Value = match (start, end) {
                (Some(s), Some(e)) if e > s => {
                    serde_json::from_str(&text[s..=e]).map_err(|e| e.to_string())?
                }
                _ => return Err("пустой ответ DoH".to_string()),
            };
            json["Answer"]
                .as_array()
                .and_then(|a| a.iter().find(|r| r["type"] == 1))
                .and_then(|r| r["data"].as_str())
                .map(str::to_string)
                .ok_or_else(|| "нет A-записи".to_string())
        };
        match timeout(Duration::from_secs(5), attempt).await {
            Ok(Ok(ip)) => {
                self.dns_cache
                    .lock()
                    .unwrap()
                    .insert(host.to_string(), ip.clone());
                Some(ip)
            }
            Ok(Err(e)) => {
                log_line(format!("DoH {host}: {e}"));
                None
            }
            Err(_) => {
                log_line(format!("DoH {host}: timeout"));
                None
            }
        }
    }

    async fn cf_connect(&self, dc: u32, base: String, secure: bool) -> Option<Ws> {
        let host = format!("kws{dc}.{base}");
        let ip = self
            .resolve_doh(&host)
            .await
            .unwrap_or_else(|| host.clone());
        let mode = if secure { "TLS" } else { "без TLS" };
        match ws_connect(self, &ip, &host, "/apiws", &host, false, secure).await {
            Ok(ws) => {
                log_line(format!(
                    "DC{dc}: Cloudflare proxy connected via {host} ({mode})"
                ));
                self.cf_plain_first.store(!secure, Ordering::Relaxed);
                self.balancer.lock().unwrap().set(dc, base);
                Some(ws)
            }
            Err(error) => {
                log_line(format!(
                    "DC{dc}: Cloudflare proxy {host} ({mode}) failed: {error}"
                ));
                None
            }
        }
    }

    async fn race_connect(self: &Arc<Self>, dc: u32, media: bool, test: bool) -> Option<Ws> {
        type Fut = std::pin::Pin<Box<dyn std::future::Future<Output = Option<Ws>> + Send>>;
        let mut futs: FuturesUnordered<Fut> = FuturesUnordered::new();
        if WS_REDIRECT_DCS.contains(&dc) {
            let this = self.clone();
            let domains = ws_domains(dc, media);
            let path = if test { "/apiws_test" } else { "/apiws" };
            futs.push(Box::pin(async move {
                this.connect_one(WS_REDIRECT_IP, &domains, path).await
            }));
        }
        if !test {
            let candidates = self.balancer.lock().unwrap().candidates(dc);
            let plain_first = self.cf_plain_first.load(Ordering::Relaxed);
            let mut i = 0u32;
            for base in candidates.into_iter().take(RACE_CF_DOMAINS) {
                for secure in [!plain_first, plain_first] {
                    let this = self.clone();
                    let base = base.clone();
                    let delay = RACE_CF_STAGGER * i;
                    i += 1;
                    futs.push(Box::pin(async move {
                        sleep(delay).await;
                        this.cf_connect(dc, base, secure).await
                    }));
                }
            }
        }
        timeout(RACE_TIMEOUT, async {
            while let Some(result) = futs.next().await {
                if result.is_some() {
                    return result;
                }
            }
            None
        })
        .await
        .unwrap_or(None)
    }

    async fn tcp_connect(&self, ip: &str, relay: &[u8; 64]) -> Option<TcpStream> {
        {
            let backoff = self.tcp_backoff.lock().unwrap();
            if backoff.get(ip).is_some_and(|b| Instant::now() < b.1) {
                return None;
            }
        }
        let result = async {
            let mut stream = timeout(Duration::from_secs(10), TcpStream::connect((ip, 443)))
                .await
                .map_err(|_| "timeout".to_string())?
                .map_err(|e| e.to_string())?;
            let _ = stream.set_nodelay(true);
            stream.write_all(relay).await.map_err(|e| e.to_string())?;
            Ok::<_, String>(stream)
        }
        .await;
        let mut backoff = self.tcp_backoff.lock().unwrap();
        match result {
            Ok(stream) => {
                backoff.remove(ip);
                Some(stream)
            }
            Err(error) => {
                let failures = backoff.get(ip).map_or(0, |b| b.0) + 1;
                let delay =
                    (TCP_BACKOFF_INITIAL * (1 << (failures - 1).min(7))).min(TCP_BACKOFF_MAX);
                backoff.insert(ip.to_string(), (failures, Instant::now() + delay));
                log_line(format!(
                    "TCP fallback {ip}:443 failed: {error}; retry in {}s",
                    delay.as_secs()
                ));
                None
            }
        }
    }
}

struct Running {
    stop: Option<oneshot::Sender<()>>,
}

static RUNNING: OnceLock<Mutex<Option<Running>>> = OnceLock::new();

fn state() -> &'static Mutex<Option<Running>> {
    RUNNING.get_or_init(|| Mutex::new(None))
}

pub fn log_path() -> PathBuf {
    crate::sys::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tg_ws.log")
}

pub fn read_log(max_lines: usize) -> Vec<String> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(log_path()) else {
        return Vec::new();
    };
    let len = file.metadata().map_or(0, |m| m.len());
    let _ = file.seek(SeekFrom::Start(len.saturating_sub(256 * 1024)));
    let mut bytes = Vec::new();
    let _ = file.read_to_end(&mut bytes);
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    lines[lines.len().saturating_sub(max_lines)..].to_vec()
}

async fn tls_probe(shared: &Shared, ip: &str, sni: &str) -> Result<(), String> {
    let tcp = stage("tcp", 4, TcpStream::connect((ip, 443))).await?;
    let _ = tcp.set_nodelay(true);
    let name = ServerName::try_from(sni.to_string()).map_err(|e| format!("sni: {e}"))?;
    stage(
        "tls",
        5,
        TlsConnector::from(shared.fronting.clone()).connect(name, Frag::new(tcp, sni)),
    )
    .await
    .map(|_| ())
}

pub fn diagnose() {
    std::thread::spawn(|| {
        let Ok(shared) = Shared::new() else { return };
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        runtime.block_on(async {
            let verdict = |r: Result<(), String>, started: Instant| {
                format!(
                    "{} ({} мс)",
                    r.map_or_else(|e| e, |_| "OK".to_string()),
                    started.elapsed().as_millis()
                )
            };
            log_line("=== Диагностика Telegram WS ===");
            log_line(format!(
                "zapret: {}",
                if crate::zapret::is_running() {
                    "запущен"
                } else {
                    "остановлен"
                }
            ));
            for ip in [
                "149.154.167.220",
                "149.154.167.51",
                "149.154.175.50",
                "149.154.171.5",
                "91.105.192.100",
                "1.1.1.1",
            ] {
                let started = Instant::now();
                let r = stage("tcp", 4, TcpStream::connect((ip, 443)))
                    .await
                    .map(|_| ());
                log_line(format!("TCP {ip}:443 -> {}", verdict(r, started)));
            }
            let host = "kws2.web.telegram.org";
            for fronted in [false, true] {
                let sni = if fronted { FRONTING_SNI } else { host };
                let started = Instant::now();
                let r = ws_connect(&shared, WS_REDIRECT_IP, host, "/apiws", sni, fronted, true)
                    .await
                    .map(|_| ());
                log_line(format!(
                    "WS {host} via {WS_REDIRECT_IP} (fronting={fronted}) -> {}",
                    verdict(r, started)
                ));
            }
            let candidates = shared.balancer.lock().unwrap().candidates(2);
            for base in candidates.into_iter().take(5) {
                let cf = format!("kws2.{base}");
                let doh = shared.resolve_doh(&cf).await;
                let ip = doh.unwrap_or_else(|| cf.clone());
                for (label, secure) in [("TLS", true), ("без TLS", false)] {
                    let started = Instant::now();
                    let r = ws_connect(&shared, &ip, &cf, "/apiws", &cf, false, secure)
                        .await
                        .map(|_| ());
                    log_line(format!(
                        "Cloudflare {cf} ({label}) -> {}",
                        verdict(r, started)
                    ));
                }
            }
            let first = shared
                .balancer
                .lock()
                .unwrap()
                .candidates(2)
                .first()
                .cloned();
            if let Some(base) = first {
                let cf = format!("kws2.{base}");
                if let Some(ip) = shared.resolve_doh(&cf).await {
                    for (label, sni) in [
                        ("SNI=cloudflare.com", "cloudflare.com".to_string()),
                        ("SNI=discord.com", "discord.com".to_string()),
                        ("SNI=своё имя", cf.clone()),
                        ("SNI=IP", ip.clone()),
                    ] {
                        let started = Instant::now();
                        let r = tls_probe(&shared, &ip, &sni).await;
                        log_line(format!("TLS {ip} {label} -> {}", verdict(r, started)));
                    }
                }
            }
            log_line("=== Конец диагностики ===");
        });
    });
}

pub fn clear_log() {
    let _ = std::fs::write(log_path(), "");
}

fn log_line(message: impl AsRef<str>) {
    static LOG_LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOG_LOCK.lock();
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(
            file,
            "{} {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            message.as_ref()
        );
    }
}

pub fn start(host: &str, port: u16, secret: &str) -> Result<(), String> {
    let secret = decode_secret(secret)?;
    stop();
    let shared = Arc::new(Shared::new()?);
    let (tx, mut stop_rx) = oneshot::channel();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let host = host.trim().to_string();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Не удалось запустить TG WS runtime: {e}"))?;
    std::thread::Builder::new()
        .name("tg-ws-proxy".into())
        .spawn(move || {
            runtime.block_on(async move {
                let listener = match TcpListener::bind((host.as_str(), port)).await {
                    Ok(listener) => listener,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));
                log_line(format!("TG WS proxy listening on {host}:{port}"));
                tokio::spawn(shared.clone().maintain());
                tokio::spawn(shared.clone().refresh_cf_domains());
                loop {
                    tokio::select! {
                        _ = &mut stop_rx => break,
                        accepted = listener.accept() => match accepted {
                            Ok((stream, _)) => {
                                let shared = shared.clone();
                                tokio::spawn(async move {
                                    if let Err(error) = handle_client(stream, &secret, shared).await {
                                        log_line(format!("client error: {error}"));
                                    }
                                });
                            }
                            Err(_) => break,
                        }
                    }
                }
            });
        })
        .map_err(|e| format!("Не удалось запустить TG WS proxy: {e}"))?;
    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(format!("Не удалось открыть TG WS-порт: {e}")),
        Err(_) => return Err("TG WS proxy не подтвердил запуск".into()),
    }
    *state()
        .lock()
        .map_err(|_| "TG WS proxy state poisoned".to_string())? = Some(Running { stop: Some(tx) });
    Ok(())
}

pub fn stop() {
    if let Ok(mut guard) = state().lock() {
        if let Some(mut running) = guard.take() {
            if let Some(tx) = running.stop.take() {
                let _ = tx.send(());
            }
        }
    }
}

fn decode_secret(secret: &str) -> Result<[u8; 16], String> {
    let secret = secret.trim();
    if secret.len() != 32 || !secret.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("TG WS secret должен содержать 32 hex-символа".into());
    }
    let mut result = [0u8; 16];
    for (i, pair) in secret.as_bytes().chunks_exact(2).enumerate() {
        result[i] = (pair[0] as char).to_digit(16).unwrap() as u8 * 16
            + (pair[1] as char).to_digit(16).unwrap() as u8;
    }
    Ok(result)
}

fn crypt(key: &[u8; 32], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let mut output = data.to_vec();
    let mut cipher = AesCtr::new(key.into(), iv.into());
    cipher.apply_keystream(&mut output);
    output
}

fn client_info(handshake: &[u8; 64], secret: &[u8; 16]) -> Option<(i32, u32, [u8; 48])> {
    let mut prekey_iv = [0u8; 48];
    prekey_iv.copy_from_slice(&handshake[8..56]);
    let mut hash = Sha256::new();
    hash.update(&prekey_iv[..32]);
    hash.update(secret);
    let key: [u8; 32] = hash.finalize().into();
    let plain = crypt(&key, prekey_iv[32..].try_into().unwrap(), handshake);
    let proto = u32::from_le_bytes(plain[PROTO_POS..PROTO_POS + 4].try_into().ok()?);
    if !matches!(proto, 0xEFEFEFEF | 0xEEEEEEEE | 0xDDDDDDDD) {
        return None;
    }
    let dc = i16::from_le_bytes(plain[DC_POS..DC_POS + 2].try_into().ok()?) as i32;
    Some((dc, proto, prekey_iv))
}

fn relay_init(proto: u32, dc: i32) -> [u8; 64] {
    loop {
        let mut result = [0u8; 64];
        if getrandom::fill(&mut result).is_err()
            || result[0] == 0xef
            || matches!(
                &result[..4],
                b"HEAD" | b"POST" | b"GET " | b"OPTI" | b"\xee\xee\xee\xee" | b"\xdd\xdd\xdd\xdd"
            )
            || result[..4] == [0x16, 0x03, 0x01, 0x02]
            || result[4..8] == [0, 0, 0, 0]
        {
            continue;
        }
        let key: [u8; 32] = result[8..40].try_into().unwrap();
        let iv: [u8; 16] = result[40..56].try_into().unwrap();
        let encrypted = crypt(&key, &iv, &result);
        let mut tail_plain = [0u8; 8];
        tail_plain[..4].copy_from_slice(&proto.to_le_bytes());
        tail_plain[4..6].copy_from_slice(&(dc as i16).to_le_bytes());
        getrandom::fill(&mut tail_plain[6..]).expect("system random unavailable");
        for i in 0..8 {
            let keystream = encrypted[56 + i] ^ result[56 + i];
            result[56 + i] = tail_plain[i] ^ keystream;
        }
        return result;
    }
}

fn dc_ip(table: &[(u32, &'static str)], dc: u32) -> Option<&'static str> {
    table.iter().find(|(id, _)| *id == dc).map(|(_, ip)| *ip)
}

async fn handle_client(
    mut client: TcpStream,
    secret: &[u8; 16],
    shared: Arc<Shared>,
) -> io::Result<()> {
    let _ = client.set_nodelay(true);
    let mut handshake = [0u8; HANDSHAKE_LEN];
    timeout(Duration::from_secs(10), client.read_exact(&mut handshake))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "handshake timeout"))??;
    let Some((dc_signed, proto, client_prekey_iv)) = client_info(&handshake, secret) else {
        let mut sink = vec![0u8; 4096];
        let _ = timeout(Duration::from_secs(10), async {
            let mut left = 64 * 1024usize;
            while left > 0 {
                match client.read(&mut sink).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => left = left.saturating_sub(n),
                }
            }
        })
        .await;
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid MTProto handshake or secret",
        ));
    };
    let media = dc_signed < 0;
    let mut dc = dc_signed.unsigned_abs();
    let test = dc >= 10000;
    if test {
        dc -= 10000;
    }
    let dc_idx = if media { -(dc as i32) } else { dc as i32 };
    let tag = format!(
        "DC{dc}{}{}",
        if test { "t" } else { "" },
        if media { "m" } else { "" }
    );
    let relay = relay_init(proto, dc_idx);

    let table: &[(u32, &str)] = if test { &DC_TEST_IPS } else { &DC_IPS };
    let tcp_ip = dc_ip(table, dc);
    if tcp_ip.is_none() && !WS_REDIRECT_DCS.contains(&dc) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported {tag}"),
        ));
    }

    let (client_read, client_write) = client.into_split();
    let client_dec = cipher_for_client(&client_prekey_iv, secret, false);
    let client_enc = cipher_for_client(&client_prekey_iv, secret, true);
    let (relay_enc, relay_dec) = relay_ciphers(&relay);
    let splitter = PacketSplitter::new(proto);

    let mut ws = shared.pool_get((dc, media, test));
    if ws.is_some() {
        log_line(format!("{tag}: WS pool hit"));
    } else {
        ws = shared.race_connect(dc, media, test).await;
    }

    if let Some(mut ws) = ws {
        ws.send(Message::Binary(relay.to_vec().into()))
            .await
            .map_err(io::Error::other)?;
        return bridge_ws(
            client_read,
            client_write,
            ws,
            client_dec,
            client_enc,
            relay_enc,
            relay_dec,
            splitter,
        )
        .await;
    }

    let Some(ip) = tcp_ip else {
        return Err(io::Error::other(format!("{tag}: no upstream available")));
    };
    let Some(upstream) = shared.tcp_connect(ip, &relay).await else {
        return Err(io::Error::other(format!("{tag}: no upstream available")));
    };
    log_line(format!("{tag}: TCP fallback connected to {ip}:443"));
    bridge_tcp(
        client_read,
        client_write,
        upstream,
        client_dec,
        client_enc,
        relay_enc,
        relay_dec,
    )
    .await
}

async fn bridge_ws(
    mut client_read: OwnedReadHalf,
    mut client_write: OwnedWriteHalf,
    mut ws: Ws,
    mut client_dec: AesCtr,
    mut client_enc: AesCtr,
    mut relay_enc: AesCtr,
    mut relay_dec: AesCtr,
    mut splitter: PacketSplitter,
) -> io::Result<()> {
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        tokio::select! {
            read = client_read.read(&mut buf) => {
                let n = read?;
                if n == 0 {
                    if let Some(packet) = splitter.flush() {
                        ws.send(Message::Binary(packet.into())).await.map_err(io::Error::other)?;
                    }
                    let _ = ws.close(None).await;
                    return Ok(());
                }
                let mut plain = buf[..n].to_vec();
                client_dec.apply_keystream(&mut plain);
                let mut cipher = plain.clone();
                relay_enc.apply_keystream(&mut cipher);
                for packet in splitter.push(&cipher, &plain) {
                    ws.feed(Message::Binary(packet.into())).await.map_err(io::Error::other)?;
                }
                ws.flush().await.map_err(io::Error::other)?;
            }
            message = ws.next() => {
                match message {
                    None | Some(Ok(Message::Close(_))) => return Ok(()),
                    Some(Err(e)) => return Err(io::Error::other(e)),
                    Some(Ok(Message::Binary(data))) => {
                        let mut plain = data.to_vec();
                        relay_dec.apply_keystream(&mut plain);
                        client_enc.apply_keystream(&mut plain);
                        client_write.write_all(&plain).await?;
                    }
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}

fn cipher_for_client(prekey_iv: &[u8; 48], secret: &[u8; 16], reverse: bool) -> AesCtr {
    let source = if reverse {
        prekey_iv.iter().rev().copied().collect::<Vec<_>>()
    } else {
        prekey_iv.to_vec()
    };
    let mut hash = Sha256::new();
    hash.update(&source[..32]);
    hash.update(secret);
    let key: [u8; 32] = hash.finalize().into();
    let iv: [u8; 16] = source[32..].try_into().unwrap();
    let mut cipher = AesCtr::new((&key).into(), (&iv).into());
    if !reverse {
        let mut skip = [0u8; 64];
        cipher.apply_keystream(&mut skip);
    }
    cipher
}

async fn bridge_tcp(
    mut client_read: OwnedReadHalf,
    mut client_write: OwnedWriteHalf,
    upstream: TcpStream,
    mut client_dec: AesCtr,
    mut client_enc: AesCtr,
    mut relay_enc: AesCtr,
    mut relay_dec: AesCtr,
) -> io::Result<()> {
    let (mut upstream_read, mut upstream_write) = upstream.into_split();
    let up = async {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = client_read.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            let mut plain = buf[..n].to_vec();
            client_dec.apply_keystream(&mut plain);
            relay_enc.apply_keystream(&mut plain);
            upstream_write.write_all(&plain).await?;
        }
        Ok::<(), io::Error>(())
    };
    let down = async {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = upstream_read.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            let mut plain = buf[..n].to_vec();
            relay_dec.apply_keystream(&mut plain);
            client_enc.apply_keystream(&mut plain);
            client_write.write_all(&plain).await?;
        }
        Ok::<(), io::Error>(())
    };
    tokio::select! { result = up => result, result = down => result }
}

fn relay_ciphers(relay: &[u8; 64]) -> (AesCtr, AesCtr) {
    let enc_key: [u8; 32] = relay[8..40].try_into().unwrap();
    let enc_iv: [u8; 16] = relay[40..56].try_into().unwrap();
    let rev: Vec<u8> = relay[8..56].iter().rev().copied().collect();
    let dec_key: [u8; 32] = rev[..32].try_into().unwrap();
    let dec_iv: [u8; 16] = rev[32..].try_into().unwrap();
    let mut enc = AesCtr::new((&enc_key).into(), (&enc_iv).into());
    let dec = AesCtr::new((&dec_key).into(), (&dec_iv).into());
    let mut skip = [0u8; 64];
    enc.apply_keystream(&mut skip);
    (enc, dec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_domains_decode_to_valid_hosts() {
        for d in CF_DOMAINS {
            let decoded = decode_cf_domain(d);
            assert!(decoded.ends_with(".co.uk"), "{decoded}");
            assert!(valid_domain(&decoded), "{decoded}");
        }
    }

    #[test]
    fn balancer_prefers_current_domain() {
        let mut b = Balancer::default();
        b.update(vec!["a.com".into(), "b.com".into(), "c.com".into()]);
        b.set(2, "b.com".into());
        let list = b.candidates(2);
        assert_eq!(list[0], "b.com");
        assert_eq!(list.len(), 3);
    }
}
