use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use crate::settings::{ProxyKind, TelegramSettings};
use crate::{sys, tg_ws};

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn validate_secret(secret: &str) -> Result<String, String> {
    let secret = secret.trim();
    if secret.is_empty() {
        return Err("Укажите secret для MTProto-прокси".into());
    }

    let valid = secret.len() == 32 && secret.bytes().all(|b| b.is_ascii_hexdigit())
        || secret.len() == 34
            && secret.starts_with("dd")
            && secret[2..].bytes().all(|b| b.is_ascii_hexdigit());
    if !valid {
        return Err("Secret должен быть 32 hex-символа или 34 символа с префиксом dd".into());
    }
    Ok(secret.to_ascii_lowercase())
}

pub fn link(t: &TelegramSettings) -> Result<String, String> {
    if t.host.trim().is_empty() || t.port == 0 {
        return Err("Укажите адрес и порт прокси".into());
    }
    let (host, port) = (encode(t.host.trim()), t.port);
    match t.kind {
        ProxyKind::Socks5 => {
            let mut url = format!("tg://socks?server={host}&port={port}");
            if !t.user.is_empty() || !t.pass.is_empty() {
                if t.user.is_empty() {
                    return Err("Укажите логин для SOCKS5-прокси".into());
                }
                url.push_str(&format!(
                    "&user={}&pass={}",
                    encode(t.user.trim()),
                    encode(t.pass.trim())
                ));
            }
            Ok(url)
        }
        ProxyKind::MtProto => {
            let secret = validate_secret(&t.secret)?;
            Ok(format!(
                "tg://proxy?server={host}&port={port}&secret={secret}"
            ))
        }
        ProxyKind::TgWs => {
            let secret = validate_ws_secret(&t.secret)?;
            Ok(format!(
                "tg://proxy?server={host}&port={port}&secret=dd{secret}"
            ))
        }
    }
}

pub fn apply(t: &TelegramSettings) -> Result<(), String> {
    if t.kind == ProxyKind::TgWs {
        let secret = validate_ws_secret(&t.secret)?;
        tg_ws::start(t.host.trim(), t.port, &secret)?;
    } else {
        tg_ws::stop();
    }
    let url = link(t)?;
    if sys::open_uri(&url) {
        Ok(())
    } else {
        Err("Не удалось открыть tg:// ссылку (Telegram Desktop установлен?)".into())
    }
}

pub fn probe(t: &TelegramSettings) -> Result<u128, String> {
    if t.kind == ProxyKind::TgWs {
        let secret = validate_ws_secret(&t.secret)?;
        let started = Instant::now();
        tg_ws::start(t.host.trim(), t.port, &secret)?;
        return Ok(started.elapsed().as_millis());
    }
    let started = Instant::now();
    let addr = (t.host.trim(), t.port)
        .to_socket_addrs()
        .map_err(|e| format!("Не удалось разрешить адрес: {e}"))?
        .next()
        .ok_or("Адрес не найден")?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))
        .map_err(|e| format!("Нет соединения: {e}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

    if t.kind == ProxyKind::Socks5 {
        let greeting: &[u8] = if t.user.is_empty() {
            &[5, 1, 0]
        } else {
            &[5, 2, 0, 2]
        };
        stream.write_all(greeting).map_err(|e| e.to_string())?;
        let mut reply = [0u8; 2];
        stream
            .read_exact(&mut reply)
            .map_err(|_| "Порт открыт, но это не SOCKS5-прокси".to_string())?;
        if reply[0] != 5 {
            return Err("Порт открыт, но это не SOCKS5-прокси".into());
        }
        if reply[1] == 0xFF {
            return Err("SOCKS5-прокси отклонил методы авторизации".into());
        }
        if reply[1] == 0x02 {
            if t.user.is_empty() {
                return Err("SOCKS5-прокси требует логин и пароль".into());
            }
            let user = t.user.as_bytes();
            let pass = t.pass.as_bytes();
            if user.len() > 255 || pass.len() > 255 {
                return Err("Логин или пароль SOCKS5 длиннее 255 байт".into());
            }
            let mut auth = Vec::with_capacity(3 + user.len() + pass.len());
            auth.push(1);
            auth.push(user.len() as u8);
            auth.extend_from_slice(user);
            auth.push(pass.len() as u8);
            auth.extend_from_slice(pass);
            stream.write_all(&auth).map_err(|e| e.to_string())?;
            let mut auth_reply = [0u8; 2];
            stream
                .read_exact(&mut auth_reply)
                .map_err(|_| "SOCKS5-прокси не ответил на авторизацию".to_string())?;
            if auth_reply[0] != 1 || auth_reply[1] != 0 {
                return Err("Логин или пароль SOCKS5 отклонены".into());
            }
        }

        let mut request = vec![5, 1, 0, 1, 127, 0, 0, 1, 0, 80];
        stream.write_all(&mut request).map_err(|e| e.to_string())?;
        let mut response = [0u8; 4];
        stream
            .read_exact(&mut response)
            .map_err(|_| "SOCKS5-прокси не ответил на проверку".to_string())?;
        if response[0] != 5 {
            return Err("Порт открыт, но это не SOCKS5-прокси".into());
        }
        if response[1] != 0 {
            let description = match response[1] {
                1 => "общая ошибка SOCKS-сервера",
                2 => "соединение запрещено правилами",
                3 => "сеть недоступна",
                4 => "хост недоступен",
                5 => "соединение отклонено",
                6 => "истекло время ожидания",
                7 => "тип команды не поддерживается",
                8 => "тип адреса не поддерживается",
                _ => "SOCKS5 отклонил проверочное соединение",
            };
            return Err(description.into());
        }
        match response[3] {
            1 => {
                let mut tail = [0u8; 6];
                stream
                    .read_exact(&mut tail)
                    .map_err(|_| "Некорректный ответ SOCKS5".to_string())?;
            }
            3 => {
                let mut len = [0u8; 1];
                stream
                    .read_exact(&mut len)
                    .map_err(|_| "Некорректный ответ SOCKS5".to_string())?;
                let mut tail = vec![0u8; len[0] as usize + 2];
                stream
                    .read_exact(&mut tail)
                    .map_err(|_| "Некорректный ответ SOCKS5".to_string())?;
            }
            4 => {
                let mut tail = [0u8; 18];
                stream
                    .read_exact(&mut tail)
                    .map_err(|_| "Некорректный ответ SOCKS5".to_string())?;
            }
            _ => return Err("Некорректный тип адреса в ответе SOCKS5".into()),
        }
    } else if t.kind == ProxyKind::MtProto {
        validate_secret(&t.secret)?;
    }
    Ok(started.elapsed().as_millis())
}

fn validate_ws_secret(secret: &str) -> Result<String, String> {
    let secret = secret.trim();
    if secret.len() == 32 && secret.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(secret.to_ascii_lowercase())
    } else {
        Err("Secret TG WS должен быть 32 hex-символа".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtproto_link_requires_valid_secret() {
        let mut settings = TelegramSettings {
            kind: ProxyKind::MtProto,
            host: "proxy.example.com".into(),
            port: 443,
            secret: String::new(),
            ..TelegramSettings::default()
        };
        assert!(link(&settings).is_err());

        settings.secret = "0".repeat(32);
        assert_eq!(
            link(&settings).unwrap(),
            "tg://proxy?server=proxy.example.com&port=443&secret=00000000000000000000000000000000"
        );
    }

    #[test]
    fn socks_link_keeps_credentials() {
        let settings = TelegramSettings {
            kind: ProxyKind::Socks5,
            host: "127.0.0.1".into(),
            port: 1080,
            user: "user".into(),
            pass: "p@ss".into(),
            ..TelegramSettings::default()
        };
        assert_eq!(
            link(&settings).unwrap(),
            "tg://socks?server=127.0.0.1&port=1080&user=user&pass=p%40ss"
        );
    }

    #[test]
    fn tg_ws_link_uses_dd_secret() {
        let settings = TelegramSettings {
            kind: ProxyKind::TgWs,
            host: "127.0.0.1".into(),
            port: 1443,
            secret: "0".repeat(32),
            ..TelegramSettings::default()
        };
        assert_eq!(
            link(&settings).unwrap(),
            "tg://proxy?server=127.0.0.1&port=1443&secret=dd00000000000000000000000000000000"
        );
    }
}
