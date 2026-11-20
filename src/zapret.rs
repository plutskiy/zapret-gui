use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::firewall;

#[derive(Clone, PartialEq)]
pub struct ConfigEntry {
    pub name: String,
    pub path: PathBuf,
}

#[cfg_attr(windows, allow(dead_code))]
pub struct LaunchSpec {
    pub windows_args: String,
    pub argv: Vec<String>,
    pub wf_tcp: String,
    pub wf_udp: String,
}

fn is_data_dir(p: &Path) -> bool {
    p.join("lists").is_dir() && !list_configs(p).is_empty()
}

pub fn find_data_dir(custom: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if !custom.trim().is_empty() {
        candidates.push(PathBuf::from(custom.trim()));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        candidates.push(dir.join("data"));
        candidates.push(dir);
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("data"));
        candidates.push(cwd);
    }
    #[cfg(debug_assertions)]
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("data"));
    candidates
        .into_iter()
        .find(|p| is_data_dir(p))
        .map(|p| std::path::absolute(&p).unwrap_or(p))
}

fn natural_key(s: &str) -> String {
    let mut out = String::new();
    let mut digits = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            if !digits.is_empty() {
                out.push_str(&format!("{digits:0>8}"));
                digits.clear();
            }
            out.push(c);
        }
    }
    if !digits.is_empty() {
        out.push_str(&format!("{digits:0>8}"));
    }
    out
}

pub fn list_configs(data: &Path) -> Vec<ConfigEntry> {
    let mut list: Vec<ConfigEntry> = fs::read_dir(data.join("configs"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let file = path.file_name()?.to_string_lossy().to_string();
            let lower = file.to_lowercase();
            if !lower.ends_with(".conf") {
                return None;
            }
            Some(ConfigEntry {
                name: file[..file.len() - 5].to_string(),
                path,
            })
        })
        .collect();
    list.sort_by_key(|c| natural_key(&c.name));
    list
}

fn game_filter(data: &Path) -> (String, String) {
    let disabled = ("12".to_string(), "12".to_string());
    let Ok(text) = fs::read_to_string(data.join("main.conf")) else {
        return disabled;
    };
    let text = text.split("\n[").next().unwrap_or_default();
    let valid = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_digit() || c == ',' || c == '-')
    };
    let (mut mode, mut tcp, mut udp) = (
        "disabled".to_string(),
        "1024-65535".to_string(),
        "1024-65535".to_string(),
    );
    for line in text.lines().filter(|l| !l.trim_start().starts_with('#')) {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim().to_lowercase(), value.trim());
        match key.as_str() {
            "game_filter" => mode = value.to_lowercase(),
            "game_filter_tcp" if valid(value) => tcp = value.into(),
            "game_filter_udp" if valid(value) => udp = value.into(),
            _ => {}
        }
    }
    match mode.as_str() {
        "all" => (tcp, udp),
        "tcp" => (tcp, "12".into()),
        "udp" => ("12".into(), udp),
        _ => disabled,
    }
}

fn replace_ci(s: &str, from: &str, to: &str) -> String {
    let (lower, from_lower) = (s.to_ascii_lowercase(), from.to_ascii_lowercase());
    let mut out = String::new();
    let mut pos = 0;
    while let Some(i) = lower[pos..].find(&from_lower) {
        out.push_str(&s[pos..pos + i]);
        out.push_str(to);
        pos += i + from.len();
    }
    out.push_str(&s[pos..]);
    out
}

fn dir_with_slash(p: &Path) -> String {
    let mut s = p.to_string_lossy().to_string();
    if !s.ends_with(std::path::MAIN_SEPARATOR) {
        s.push(std::path::MAIN_SEPARATOR);
    }
    s
}

pub fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (mut cur, mut in_quotes, mut has) = (String::new(), false, false);
    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                has = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if has {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            c => {
                cur.push(c);
                has = true;
            }
        }
    }
    if has {
        out.push(cur);
    }
    out
}

pub fn build_launch(data: &Path, conf: &Path) -> Result<LaunchSpec, String> {
    let bytes =
        fs::read(conf).map_err(|e| format!("Не удалось прочитать {}: {e}", conf.display()))?;
    let mut args = String::from_utf8_lossy(&bytes)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");
    if args.is_empty() {
        return Err("Конфиг пуст".into());
    }

    let (tcp, udp) = game_filter(data);
    for (var, value) in [
        ("%BIN%", dir_with_slash(&data.join("bin"))),
        ("%LISTS%", dir_with_slash(&data.join("lists"))),
        ("%~dp0", dir_with_slash(data)),
        ("%GameFilterTCP%", tcp.clone()),
        ("%GameFilterUDP%", udp),
        ("%GameFilter%", tcp),
    ] {
        args = replace_ci(&args, var, &value);
    }
    let windows_args = args;

    let (mut argv, mut wf_tcp, mut wf_udp) = (Vec::new(), String::new(), String::new());
    for token in tokenize(&windows_args) {
        if let Some(v) = token.strip_prefix("--wf-tcp=") {
            wf_tcp = v.to_string();
        } else if let Some(v) = token.strip_prefix("--wf-udp=") {
            wf_udp = v.to_string();
        } else if !token.starts_with("--wf-") {
            argv.push(token);
        }
    }
    Ok(LaunchSpec {
        windows_args,
        argv,
        wf_tcp,
        wf_udp,
    })
}

pub fn ensure_user_lists(data: &Path) {
    let lists = data.join("lists");
    let _ = fs::create_dir_all(&lists);
    for (file, content) in [
        ("ipset-exclude-user.txt", "203.0.113.113/32\r\n"),
        (
            "list-general-user.txt",
            "# Never leave this file empty\r\ndomain.example.abc\r\n",
        ),
        ("list-exclude-user.txt", "domain.example.abc\r\n"),
    ] {
        let p = lists.join(file);
        if !p.exists() {
            let _ = fs::write(p, content);
        }
    }
}

pub fn engine_path(data: &Path) -> Result<PathBuf, String> {
    if !crate::engine::supported() {
        return Err(
            "На этой системе нет перехвата пакетов (поддерживаются Windows и Linux)".into(),
        );
    }
    #[cfg(windows)]
    {
        let p = data.join("bin").join("WinDivert.dll");
        if !p.is_file() || !data.join("bin").join("WinDivert64.sys").is_file() {
            return Err(format!(
                "Не найдены WinDivert.dll / WinDivert64.sys в {}",
                data.join("bin").display()
            ));
        }
        Ok(p)
    }
    #[cfg(not(windows))]
    {
        Ok(data.join("bin"))
    }
}

static ENGINE: Mutex<Option<crate::engine::Running>> = Mutex::new(None);
static OPERATION: Mutex<()> = Mutex::new(());
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn is_running() -> bool {
    lock(&ENGINE).as_ref().is_some_and(|r| r.is_alive())
}

fn stop_engine() {
    let r = lock(&ENGINE).take();
    if let Some(r) = r {
        r.stop();
    }
    firewall::remove();
}

pub fn stop() {
    let _op = lock(&OPERATION);
    stop_engine();
    firewall::restore_system();
}

pub fn shutdown() {
    SHUTDOWN.store(true, Ordering::SeqCst);
    stop();
}

pub fn start_config(data: &Path, cfg: &ConfigEntry) -> Result<(), String> {
    let _op = lock(&OPERATION);
    if SHUTDOWN.load(Ordering::SeqCst) {
        return Err("Приложение закрывается".into());
    }
    ensure_user_lists(data);
    engine_path(data)?;
    let spec = build_launch(data, &cfg.path)?;
    stop_engine();
    firewall::prepare();

    #[allow(unused_mut)]
    let mut args = tokenize(&spec.windows_args);
    #[cfg(target_os = "linux")]
    {
        args.extend(firewall::engine_args());
    }

    firewall::apply(&spec)?;

    let debug = std::env::var_os("ZAPRET_GUI_DEBUG").is_some();
    let (log_tx, log_rx) = std::sync::mpsc::channel::<String>();
    if debug {
        let path = std::env::temp_dir().join("zapret_gui_engine.log");
        std::thread::spawn(move || {
            use std::io::Write;
            if let Ok(mut f) = File::create(path) {
                while let Ok(l) = log_rx.recv() {
                    let _ = writeln!(f, "{l}");
                }
            }
        });
    }
    let running = crate::engine::start(crate::engine::Spec {
        args,
        bin_dir: data.join("bin"),
        log: debug.then_some(log_tx),
        debug,
    })
    .map_err(|e| {
        firewall::remove();
        e
    })?;

    let deadline = Instant::now() + Duration::from_millis(300);
    while Instant::now() < deadline {
        if !running.is_alive() {
            firewall::remove();
            return Err("Движок завершился сразу после запуска".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    *lock(&ENGINE) = Some(running);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_conf_files_build_launch_spec() {
        let data = find_data_dir("").expect("data dir");
        let configs = list_configs(&data);
        assert!(configs.len() >= 20, "found {}", configs.len());
        for cfg in &configs {
            let spec =
                build_launch(&data, &cfg.path).unwrap_or_else(|e| panic!("{}: {e}", cfg.name));
            assert!(
                !spec.windows_args.contains('%'),
                "{}: unresolved var",
                cfg.name
            );
            assert!(!spec.windows_args.contains('^'), "{}: caret left", cfg.name);
            assert!(spec.windows_args.starts_with("--wf-tcp="), "{}", cfg.name);
            assert!(
                !spec.wf_tcp.is_empty() && !spec.wf_udp.is_empty(),
                "{}",
                cfg.name
            );
            assert!(
                spec.argv.iter().all(|a| !a.starts_with("--wf-")),
                "{}",
                cfg.name
            );
            assert!(spec.argv.iter().all(|a| !a.contains('"')), "{}", cfg.name);
            assert!(spec.argv.contains(&"--new".to_string()));
            let listed: Vec<_> = spec
                .argv
                .iter()
                .filter(|a| a.starts_with("--hostlist="))
                .collect();
            assert!(
                listed
                    .iter()
                    .all(|a| !a.contains(" ") || a.contains(&data.to_string_lossy().to_string()))
            );
        }
        let first = build_launch(&data, &configs[0].path).unwrap();
        println!("wf_tcp={} wf_udp={}", first.wf_tcp, first.wf_udp);
        println!("{:?}", &first.argv[..4]);
    }

    #[test]
    fn tokenizer_keeps_quoted_paths() {
        let t = tokenize(r#"--a="C:\x y\f.txt" --b=1 "--c=2" --d="""#);
        assert_eq!(t, ["--a=C:\\x y\\f.txt", "--b=1", "--c=2", "--d="]);
    }
}
