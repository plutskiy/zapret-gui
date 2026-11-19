use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

type Sig = Option<(SystemTime, u64)>;

fn file_sig(p: &std::path::Path) -> Sig {
    let m = std::fs::metadata(p).ok()?;
    Some((m.modified().ok()?, m.len()))
}

#[derive(Default)]
pub struct HostFile {
    path: Option<PathBuf>,
    set: HashMap<String, bool>,
    sig: Sig,
}

impl HostFile {
    fn add_line(&mut self, line: &str) {
        let l = line.trim_end_matches(['\r', '\n']);
        let first = l.chars().next();
        if matches!(first, None | Some('#') | Some(';') | Some('/')) {
            return;
        }
        let (strict, name) = match l.strip_prefix('^') {
            Some(r) => (true, r),
            None => (false, l),
        };
        if name.is_empty() {
            return;
        }
        self.set.insert(name.to_ascii_lowercase(), strict);
    }

    pub fn add_item(&mut self, s: &str) {
        self.add_line(s);
    }

    fn load(&mut self) -> std::io::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        let text = std::fs::read(path)?;
        let text = String::from_utf8_lossy(&text);
        self.set.clear();
        let mut tmp = HostFile::default();
        for line in text.lines() {
            tmp.add_line(line);
        }
        self.set = tmp.set;
        self.sig = file_sig(path);
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}

#[derive(Default)]
pub struct IpsetFile {
    path: Option<PathBuf>,
    v4: Vec<(u32, u32)>,
    v6: Vec<(u128, u128)>,
    sig: Sig,
}

fn parse_cidr(s: &str) -> Option<(IpAddr, u8)> {
    let s = s.trim();
    let (ip, len) = match s.split_once('/') {
        Some((a, b)) => (a, Some(b.trim().parse::<u8>().ok()?)),
        None => (s, None),
    };
    let ip: IpAddr = ip.trim().parse().ok()?;
    let max = if ip.is_ipv4() { 32 } else { 128 };
    let len = len.unwrap_or(max);
    if len > max {
        return None;
    }
    Some((ip, len))
}

fn merge<T: Ord + Copy + std::ops::Add<Output = T> + From<u8>>(v: &mut Vec<(T, T)>) {
    v.sort_unstable();
    let mut out: Vec<(T, T)> = Vec::with_capacity(v.len());
    for &(s, e) in v.iter() {
        match out.last_mut() {
            Some(last) if s <= last.1 + T::from(1) || last.1 == e => {
                if e > last.1 {
                    last.1 = e;
                }
            }
            _ => out.push((s, e)),
        }
    }
    *v = out;
}

impl IpsetFile {
    fn add_line(&mut self, line: &str) {
        let l = line.trim();
        let first = l.chars().next();
        if matches!(first, None | Some('#') | Some(';') | Some('/')) {
            return;
        }
        match parse_cidr(l) {
            Some((IpAddr::V4(a), len)) => {
                let a = u32::from(a);
                let mask = if len == 0 { 0 } else { u32::MAX << (32 - len) };
                self.v4.push((a & mask, a | !mask));
            }
            Some((IpAddr::V6(a), len)) => {
                let a = u128::from(a);
                let mask = if len == 0 { 0 } else { u128::MAX << (128 - len) };
                self.v6.push((a & mask, a | !mask));
            }
            None => {}
        }
    }

    pub fn add_item(&mut self, s: &str) {
        self.add_line(s);
        self.finish();
    }

    fn finish(&mut self) {
        merge(&mut self.v4);
        merge(&mut self.v6);
    }

    fn load(&mut self) -> std::io::Result<()> {
        let Some(path) = self.path.clone() else { return Ok(()) };
        let text = std::fs::read(&path)?;
        let text = String::from_utf8_lossy(&text);
        self.v4.clear();
        self.v6.clear();
        for line in text.lines() {
            self.add_line(line);
        }
        self.finish();
        self.sig = file_sig(&path);
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.v4.is_empty() && self.v6.is_empty()
    }

    fn contains(&self, ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(a) => {
                let a = u32::from(*a);
                let i = self.v4.partition_point(|r| r.0 <= a);
                i > 0 && self.v4[i - 1].1 >= a
            }
            IpAddr::V6(a) => {
                let a = u128::from(*a);
                let i = self.v6.partition_point(|r| r.0 <= a);
                i > 0 && self.v6[i - 1].1 >= a
            }
        }
    }
}

pub struct Lists {
    pub hosts: Vec<HostFile>,
    pub ipsets: Vec<IpsetFile>,
    last_check: Instant,
}

impl Default for Lists {
    fn default() -> Self {
        Self { hosts: Vec::new(), ipsets: Vec::new(), last_check: Instant::now() }
    }
}

impl Lists {
    pub fn register_hostlist(&mut self, path: Option<&str>) -> usize {
        if let Some(p) = path {
            let pb = PathBuf::from(p);
            if let Some(i) = self.hosts.iter().position(|h| h.path.as_ref() == Some(&pb)) {
                return i;
            }
            self.hosts.push(HostFile { path: Some(pb), ..Default::default() });
        } else {
            self.hosts.push(HostFile::default());
        }
        self.hosts.len() - 1
    }

    pub fn register_ipset(&mut self, path: Option<&str>) -> usize {
        if let Some(p) = path {
            let pb = PathBuf::from(p);
            if let Some(i) = self.ipsets.iter().position(|h| h.path.as_ref() == Some(&pb)) {
                return i;
            }
            self.ipsets.push(IpsetFile { path: Some(pb), ..Default::default() });
        } else {
            self.ipsets.push(IpsetFile::default());
        }
        self.ipsets.len() - 1
    }

    pub fn load_all(&mut self) -> Result<(), String> {
        for h in &mut self.hosts {
            h.load().map_err(|e| format!("hostlist {}: {e}", h.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default()))?;
        }
        for i in &mut self.ipsets {
            i.load().map_err(|e| format!("ipset {}: {e}", i.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default()))?;
        }
        Ok(())
    }

    pub fn maybe_reload(&mut self) {
        if self.last_check.elapsed().as_secs() < 3 {
            return;
        }
        self.last_check = Instant::now();
        for h in &mut self.hosts {
            if let Some(p) = &h.path {
                let sig = file_sig(p);
                if sig.is_some() && sig != h.sig {
                    let _ = h.load();
                }
            }
        }
        for i in &mut self.ipsets {
            if let Some(p) = &i.path {
                let sig = file_sig(p);
                if sig.is_some() && sig != i.sig {
                    let _ = i.load();
                }
            }
        }
    }

    pub fn hostlists_empty(&self, ids: &[usize]) -> bool {
        ids.iter().all(|&i| self.hosts[i].is_empty())
    }

    pub fn ipsets_empty(&self, ids: &[usize]) -> bool {
        ids.iter().all(|&i| self.ipsets[i].is_empty())
    }

    fn search_host(&self, id: usize, host: &str, no_match_subdomains: bool) -> bool {
        let set = &self.hosts[id].set;
        if set.is_empty() {
            return false;
        }
        let mut p = host;
        let mut full = true;
        loop {
            if let Some(&strict) = set.get(p) {
                if !(strict && !full) {
                    return true;
                }
            }
            if no_match_subdomains {
                break;
            }
            match p.find('.') {
                Some(i) => p = &p[i + 1..],
                None => break,
            }
            full = false;
        }
        false
    }

    // returns (apply, excluded)
    pub fn host_check(&self, include: &[usize], exclude: &[usize], host: &str, no_match_subdomains: bool) -> (bool, bool) {
        for &id in exclude {
            if self.search_host(id, host, no_match_subdomains) {
                return (false, true);
            }
        }
        if !self.hostlists_empty(include) {
            for &id in include {
                if self.search_host(id, host, no_match_subdomains) {
                    return (true, false);
                }
            }
            return (false, false);
        }
        (true, false)
    }

    pub fn ipset_check(&self, include: &[usize], exclude: &[usize], ip: &IpAddr) -> bool {
        if include.is_empty() && exclude.is_empty() {
            return true;
        }
        for &id in exclude {
            if self.ipsets[id].contains(ip) {
                return false;
            }
        }
        if !self.ipsets_empty(include) {
            return include.iter().any(|&id| self.ipsets[id].contains(ip));
        }
        true
    }
}

#[allow(dead_code)]
fn _types(_: Ipv4Addr, _: Ipv6Addr) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostlist_matching() {
        let mut l = Lists::default();
        let a = l.register_hostlist(None);
        l.hosts[a].add_item("example.com");
        l.hosts[a].add_item("^strict.org");
        let ex = l.register_hostlist(None);
        l.hosts[ex].add_item("bad.example.com");
        assert_eq!(l.host_check(&[a], &[ex], "www.example.com", false), (true, false));
        assert_eq!(l.host_check(&[a], &[ex], "bad.example.com", false), (false, true));
        assert_eq!(l.host_check(&[a], &[ex], "x.bad.example.com", false), (false, true));
        assert_eq!(l.host_check(&[a], &[], "sub.strict.org", false), (false, false));
        assert_eq!(l.host_check(&[a], &[], "strict.org", false), (true, false));
        assert_eq!(l.host_check(&[a], &[], "www.example.com", true), (false, false));
    }

    #[test]
    fn ipset_matching() {
        let mut l = Lists::default();
        let a = l.register_ipset(None);
        l.ipsets[a].add_item("10.0.0.0/8");
        l.ipsets[a].add_item("2001:db8::/32");
        let ex = l.register_ipset(None);
        l.ipsets[ex].add_item("10.1.0.0/16");
        assert!(l.ipset_check(&[a], &[ex], &"10.2.3.4".parse().unwrap()));
        assert!(!l.ipset_check(&[a], &[ex], &"10.1.3.4".parse().unwrap()));
        assert!(!l.ipset_check(&[a], &[ex], &"11.0.0.1".parse().unwrap()));
        assert!(l.ipset_check(&[a], &[], &"2001:db8::1".parse().unwrap()));
        assert!(l.ipset_check(&[], &[], &"1.1.1.1".parse().unwrap()));
    }
}
