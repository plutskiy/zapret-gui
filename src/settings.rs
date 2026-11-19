use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProxyKind {
    #[default]
    Socks5,
    MtProto,
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
        Self {
            kind: ProxyKind::Socks5,
            host: "127.0.0.1".into(),
            port: 1080,
            user: String::new(),
            pass: String::new(),
            secret: String::new(),
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
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, json);
        }
    }
}
