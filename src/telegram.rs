use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use crate::settings::{ProxyKind, TelegramSettings};
use crate::sys;

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn link(t: &TelegramSettings) -> Result<String, String> {
    if t.host.trim().is_empty() || t.port == 0 {
        return Err("Укажите адрес и порт прокси".into());
    }
    let (host, port) = (encode(t.host.trim()), t.port);
    match t.kind {
        ProxyKind::Socks5 => {
            let mut url = format!("tg://socks?server={host}&port={port}");
            if !t.user.is_empty() {
                url.push_str(&format!("&user={}&pass={}", encode(&t.user), encode(&t.pass)));
            }
            Ok(url)
        }
        ProxyKind::MtProto => {
            if t.secret.trim().is_empty() {
                return Err("Укажите secret для MTProto-прокси".into());
            }
            Ok(format!("tg://proxy?server={host}&port={port}&secret={}", encode(t.secret.trim())))
        }
    }
}

pub fn apply(t: &TelegramSettings) -> Result<(), String> {
    let url = link(t)?;
    if sys::open_uri(&url) {
        Ok(())
    } else {
        Err("Не удалось открыть tg:// ссылку (Telegram Desktop установлен?)".into())
    }
}

pub fn probe(t: &TelegramSettings) -> Result<u128, String> {
    let started = Instant::now();
    let addr = (t.host.trim(), t.port)
        .to_socket_addrs()
        .map_err(|e| format!("Не удалось разрешить адрес: {e}"))?
        .next()
        .ok_or("Адрес не найден")?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| format!("Нет соединения: {e}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

    if t.kind == ProxyKind::Socks5 {
        let greeting: &[u8] = if t.user.is_empty() { &[5, 1, 0] } else { &[5, 2, 0, 2] };
        stream.write_all(greeting).map_err(|e| e.to_string())?;
        let mut reply = [0u8; 2];
        stream.read_exact(&mut reply).map_err(|_| "Порт открыт, но это не SOCKS5-прокси".to_string())?;
        if reply[0] != 5 {
            return Err("Порт открыт, но это не SOCKS5-прокси".into());
        }
        if reply[1] == 0xFF {
            return Err("SOCKS5-прокси отклонил методы авторизации".into());
        }
    }
    Ok(started.elapsed().as_millis())
}
