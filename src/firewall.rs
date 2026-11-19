#![cfg_attr(windows, allow(dead_code))]

use crate::zapret::LaunchSpec;

pub const QNUM: u16 = 200;
pub const FWMARK: &str = "0x40000000";
pub const TABLE: &str = "zapret_gui";
const PACKET_LIMIT: u32 = 12;

pub fn engine_args() -> Vec<String> {
    vec![format!("--qnum={QNUM}"), format!("--dpi-desync-fwmark={FWMARK}")]
}

fn clean_ports(list: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in list.split(',').map(str::trim) {
        let valid = !p.is_empty() && p != "12" && p.chars().all(|c| c.is_ascii_digit() || c == '-');
        if valid && !out.iter().any(|x| x == p) {
            out.push(p.to_string());
        }
    }
    out
}

pub fn ruleset(tcp: &str, udp: &str, packet_limit: bool) -> String {
    let limit = if packet_limit { format!("ct original packets 1-{PACKET_LIMIT} ") } else { String::new() };
    let mut rules = String::new();
    for (proto, ports) in [("tcp", clean_ports(tcp)), ("udp", clean_ports(udp))] {
        if !ports.is_empty() {
            rules.push_str(&format!(
                "    {proto} dport {{ {} }} {limit}meta mark and {FWMARK} == 0 queue num {QNUM} bypass\n",
                ports.join(", ")
            ));
        }
    }
    format!(
        "add table inet {TABLE}\ndelete table inet {TABLE}\ntable inet {TABLE} {{\n  chain post {{\n    type filter hook postrouting priority 101; policy accept;\n{rules}  }}\n}}\n"
    )
}

#[cfg(target_os = "linux")]
mod imp {
    use std::io::Write;
    use std::process::Stdio;

    use super::*;
    use crate::sys;

    fn write_proc(path: &str, value: &str) -> bool {
        std::fs::write(path, value).is_ok()
    }

    pub fn prepare() {
        write_proc("/proc/sys/net/ipv4/tcp_timestamps", "1");
        let _ = sys::hidden("modprobe").arg("nfnetlink_queue").output();
    }

    fn conntrack_accounting() -> bool {
        const ACCT: &str = "/proc/sys/net/netfilter/nf_conntrack_acct";
        if !std::path::Path::new(ACCT).exists() {
            let _ = sys::hidden("modprobe").arg("nf_conntrack").output();
        }
        write_proc(ACCT, "1");
        std::fs::read_to_string(ACCT).map(|s| s.trim() == "1").unwrap_or(false)
    }

    fn nft(script: &str) -> Result<(), String> {
        let mut child = sys::hidden("nft")
            .args(["-f", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Не удалось запустить nft (установлен nftables?): {e}"))?;
        child.stdin.take().ok_or("нет stdin у nft")?.write_all(script.as_bytes()).map_err(|e| e.to_string())?;
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!("nft: {}", String::from_utf8_lossy(&out.stderr).trim()))
        }
    }

    pub fn apply(spec: &LaunchSpec) -> Result<(), String> {
        nft(&ruleset(&spec.wf_tcp, &spec.wf_udp, conntrack_accounting()))
    }

    pub fn remove() {
        let _ = nft(&format!("add table inet {TABLE}\ndelete table inet {TABLE}\n"));
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::sys;

    pub fn prepare() {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let _ = sys::hidden("netsh").args(["interface", "tcp", "set", "global", "timestamps=enabled"]).output();
        });
    }

    pub fn apply(_: &LaunchSpec) -> Result<(), String> {
        Ok(())
    }

    pub fn remove() {}
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    use super::*;

    pub fn prepare() {}

    pub fn apply(_: &LaunchSpec) -> Result<(), String> {
        Ok(())
    }

    pub fn remove() {}
}

pub use imp::{apply, prepare, remove};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nft_ruleset_is_well_formed() {
        let s = ruleset("80,443,2053,8443,12,1024-65535,443", "443,19294-19344,12", true);
        println!("{s}");
        assert!(s.contains("tcp dport { 80, 443, 2053, 8443, 1024-65535 } ct original packets 1-12 meta mark and 0x40000000 == 0 queue num 200 bypass"));
        assert!(s.contains("udp dport { 443, 19294-19344 }"));
        let none = ruleset("12", "", false);
        assert!(!none.contains("dport"));
    }
}
