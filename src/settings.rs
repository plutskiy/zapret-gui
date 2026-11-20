use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProxyKind {
    #[default]
    Socks5,
    MtProto,
    TgWs,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelegramSettings {
    pub kind: ProxyKind,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub secret: String,
    pub on_zapret_start: bool,
    pub on_app_start: bool,
}

impl Default for TelegramSettings {
    fn default() -> Self {
        let mut secret = [0u8; 16];
        let _ = getrandom::fill(&mut secret);
        Self {
            kind: ProxyKind::Socks5,
            host: "127.0.0.1".into(),
            port: 1080,
            user: String::new(),
            pass: String::new(),
            secret: secret.iter().map(|b| format!("{b:02x}")).collect(),
            on_zapret_start: false,
            on_app_start: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub data_dir: String,
    pub last_config: String,
    pub autostart_last_config: bool,
    pub test_parallel: usize,
    pub test_timeout: u64,
    pub site_parallel: usize,
    pub site_timeout: u64,
    pub dpi_range: u64,
    pub telegram: TelegramSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            data_dir: String::new(),
            last_config: String::new(),
            autostart_last_config: false,
            test_parallel: 12,
            test_timeout: 4,
            site_parallel: 16,
            site_timeout: 6,
            dpi_range: 65536,
            telegram: TelegramSettings::default(),
        }
    }
}

fn path() -> Option<PathBuf> {
    Some(crate::sys::config_dir()?.join("settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        let Some(p) = path() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(&p) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_else(|_| {
            let _ = std::fs::rename(&p, p.with_extension("json.bad"));
            Self::default()
        })
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let Ok(json) = serde_json::to_string_pretty(self) else {
            return;
        };
        let tmp = p.with_extension("json.tmp");
        if write_private(&tmp, json.as_bytes()).is_ok() && std::fs::rename(&tmp, &p).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_private_replaces_content() {
        let p = std::env::temp_dir().join("zapret_gui_settings_test.json");
        write_private(&p, b"first-long-content").unwrap();
        write_private(&p, b"x").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"x");
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn settings_roundtrip_through_json() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.test_parallel, s.test_parallel);
        assert!(serde_json::from_str::<Settings>("{}").is_ok());
    }
}

fn write_private(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(data)?;
    f.sync_all()
}
