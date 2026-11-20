use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn hidden(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

pub fn timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
}

pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|b| PathBuf::from(b).join("zapret_gui"))
    }
    #[cfg(unix)]
    {
        let home = std::env::var_os("ZAPRET_GUI_HOME")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)?;
        if cfg!(target_os = "macos") {
            Some(
                home.join("Library")
                    .join("Application Support")
                    .join("zapret_gui"),
            )
        } else {
            Some(home.join(".config").join("zapret_gui"))
        }
    }
}

#[cfg(windows)]
pub use win::*;

#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod win {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    use super::hidden;

    fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
        s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
    }

    fn quote_arg(arg: &str) -> String {
        let mut out = String::from("\"");
        let mut backslashes = 0;
        for c in arg.chars() {
            match c {
                '\\' => backslashes += 1,
                '"' => {
                    out.push_str(&"\\".repeat(backslashes * 2 + 1));
                    out.push('"');
                    backslashes = 0;
                }
                c => {
                    out.push_str(&"\\".repeat(backslashes));
                    out.push(c);
                    backslashes = 0;
                }
            }
        }
        out.push_str(&"\\".repeat(backslashes * 2));
        out.push('"');
        out
    }

    pub fn needs_root() -> bool {
        true
    }

    pub fn is_elevated() -> bool {
        unsafe {
            let mut token = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return false;
            }
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut returned = 0u32;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut returned,
            );
            CloseHandle(token);
            ok != 0 && elevation.TokenIsElevated != 0
        }
    }

    fn shell_execute(verb: &str, file: impl AsRef<OsStr>, params: &str) -> bool {
        let (verb, file, params) = (wide(verb), wide(file), wide(params));
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                params.as_ptr(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        result as isize > 32
    }

    pub fn relaunch_elevated() -> bool {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        let params = std::env::args_os()
            .skip(1)
            .map(|a| quote_arg(&a.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ");
        shell_execute("runas", exe, &params)
    }

    pub fn open_uri(uri: &str) -> bool {
        shell_execute("open", uri, "")
    }

    pub fn service_running(name: &str) -> bool {
        hidden("sc")
            .args(["query", name])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("RUNNING"))
            .unwrap_or(false)
    }
}

#[cfg(unix)]
mod unix {
    use super::hidden;

    const FORWARD_ENV: [&str; 6] = [
        "DISPLAY",
        "XAUTHORITY",
        "WAYLAND_DISPLAY",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
        "XDG_SESSION_TYPE",
    ];

    pub fn needs_root() -> bool {
        cfg!(target_os = "linux")
    }

    pub fn is_elevated() -> bool {
        unsafe { libc::geteuid() == 0 }
    }

    pub fn relaunch_elevated() -> bool {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        let mut env_args = vec!["env".to_string()];
        for key in FORWARD_ENV {
            if let Ok(v) = std::env::var(key) {
                env_args.push(format!("{key}={v}"));
            }
        }
        for (key, var) in [("ZAPRET_GUI_HOME", "HOME"), ("ZAPRET_GUI_USER", "USER")] {
            if let Ok(v) = std::env::var(var) {
                env_args.push(format!("{key}={v}"));
            }
        }
        hidden("pkexec")
            .args(env_args)
            .arg(exe)
            .args(std::env::args_os().skip(1))
            .spawn()
            .is_ok()
    }

    pub fn open_uri(uri: &str) -> bool {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let user = std::env::var("ZAPRET_GUI_USER")
            .ok()
            .filter(|_| is_elevated());
        let mut cmd = match user {
            Some(user) => {
                let mut c = hidden("runuser");
                c.args(["-u", &user, "--", "env"]);
                if let Ok(home) = std::env::var("ZAPRET_GUI_HOME") {
                    c.arg(format!("HOME={home}"));
                }
                c.arg(opener);
                c
            }
            None => hidden(opener),
        };
        cmd.arg(uri).spawn().is_ok()
    }

    pub fn service_running(_name: &str) -> bool {
        false
    }
}

pub fn has_required_privileges() -> bool {
    !needs_root() || is_elevated()
}
