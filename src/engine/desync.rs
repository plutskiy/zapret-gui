use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::args::*;
use super::conntrack::*;
use super::lists::Lists;
use super::packet::*;
use super::proto::*;
use super::rng;

pub const VERDICT_PASS: u8 = 0;
pub const VERDICT_MODIFY: u8 = 1;
pub const VERDICT_DROP: u8 = 2;
pub const VERDICT_MASK: u8 = 3;
pub const VERDICT_NOCSUM: u8 = 4;

pub const TCP_MAX_REASM: usize = 16384;
pub const UDP_MAX_REASM: usize = 16384;
pub const MAX_PKT: usize = DPI_DESYNC_MAX_FAKE_LEN + 100;

pub trait Io {
    fn send(&mut self, dst: &Endpoint, fwmark: u32, ifout: &Iface, data: &[u8]) -> bool;
}


#[macro_export]
macro_rules! dlog {
    ($($arg:tt)*) => {
        if $crate::engine::debug_enabled() {
            $crate::engine::log(&format!($($arg)*));
        }
    };
}

#[derive(Default)]
pub struct IpCacheItem {
    pub hops: u8,
    pub hostname: Option<String>,
    pub hostname_is_ip: bool,
    pub touched: Option<Instant>,
}

pub struct Engine {
    pub params: Arc<Params>,
    pub lists: Lists,
    pub ct: Conntrack,
    pub ipcache: HashMap<(IpAddr, u32), IpCacheItem>,
    ipcache_last_purge: Instant,
}

pub enum Next {
    SendOrig,
    Unsplit,
    Done(u8),
}

pub struct Tx<'a> {
    pub io: &'a mut dyn Io,
    pub dst: Endpoint,
    pub fwmark: u32,
    pub ifout: Iface,
}

impl Tx<'_> {
    pub fn send(&mut self, data: &[u8]) -> bool {
        self.io.send(&self.dst, self.fwmark, &self.ifout, data)
    }
    pub fn send_rep(&mut self, repeats: u32, data: &[u8]) -> bool {
        for _ in 0..repeats {
            if !self.send(data) {
                return false;
            }
        }
        true
    }
}

pub fn ip_id_fix(pkt: &Pkt, mode: IpIdMode) -> u16 {
    if pkt.v6 {
        return 0;
    }
    let id = pkt.ip_id();
    match mode {
        IpIdMode::Seq | IpIdMode::SeqGroup => {
            if id != 0 { id } else { rng::next_u16() }
        }
        IpIdMode::Same => id,
        IpIdMode::Rnd => (rng::next_u32() % 0xFFFF) as u16 + 1,
        IpIdMode::Zero => 0,
    }
}

pub fn ip_id_add(id: u16, inc: i16, mode: IpIdMode) -> u16 {
    match mode {
        IpIdMode::Seq | IpIdMode::SeqGroup => {
            let mut id = id;
            if id != 0 {
                id = id.wrapping_add(inc as u16);
                if id == 0 {
                    id = id.wrapping_add(if inc < 0 { 0xFFFF } else { 1 });
                }
            }
            id
        }
        IpIdMode::Same => id,
        IpIdMode::Rnd => (rng::next_u32() % 0xFFFF) as u16 + 1,
        IpIdMode::Zero => 0,
    }
}
pub fn ip_id_next(id: u16, mode: IpIdMode) -> u16 {
    ip_id_add(id, 1, mode)
}
pub fn ip_id_prev(id: u16, mode: IpIdMode) -> u16 {
    ip_id_add(id, -1, mode)
}

pub fn cutoff_get_limit(ct: &Ctrack, mode: char) -> u64 {
    match mode {
        'n' => ct.pcounter_orig,
        'd' => ct.pdcounter_orig,
        's' => ct.seq_last.wrapping_sub(ct.seq0) as u64,
        _ => 0,
    }
}

pub fn cutoff_test(ct: &Ctrack, cutoff: u32, mode: char) -> bool {
    cutoff != 0 && cutoff_get_limit(ct, mode) >= cutoff as u64
}

pub fn maybe_cutoff(ct: &mut Ctrack, dp: &Profile, tcp: bool) {
    if tcp {
        let b = cutoff_test(ct, dp.wssize_cutoff, dp.wssize_cutoff_mode);
        ct.b_wssize_cutoff |= b;
    }
    let b = cutoff_test(ct, dp.desync_cutoff, dp.desync_cutoff_mode);
    ct.b_desync_cutoff |= b;
    let b = cutoff_test(ct, dp.dup_cutoff, dp.dup_cutoff_mode);
    ct.b_dup_cutoff |= b;
    let b = cutoff_test(ct, dp.orig_mod_cutoff, dp.orig_mod_cutoff_mode);
    ct.b_orig_mod_cutoff |= b;
}

pub fn forced_wssize_cutoff(ct: &mut Ctrack, dp: &Profile) {
    if !ct.b_wssize_cutoff {
        ct.b_wssize_cutoff = true;
        maybe_cutoff(ct, dp, true);
    }
}

pub fn check_desync_interval(dp: &Profile, ct: Option<&Ctrack>) -> bool {
    if dp.desync_start != 0 {
        match ct {
            Some(c) => {
                if !cutoff_test(c, dp.desync_start, dp.desync_start_mode) {
                    return false;
                }
            }
            None => return false,
        }
    }
    if dp.desync_cutoff != 0 {
        match ct {
            Some(c) => {
                if c.b_desync_cutoff {
                    return false;
                }
            }
            None => return false,
        }
    }
    true
}

pub fn check_dup_interval(dp: &Profile, ct: Option<&Ctrack>) -> bool {
    if dp.dup_start != 0 {
        match ct {
            Some(c) => {
                if !cutoff_test(c, dp.dup_start, dp.dup_start_mode) {
                    return false;
                }
            }
            None => return false,
        }
    }
    if dp.dup_cutoff != 0 {
        match ct {
            Some(c) => {
                if c.b_dup_cutoff {
                    return false;
                }
            }
            None => return false,
        }
    }
    true
}

pub fn check_orig_mod_interval(dp: &Profile, ct: Option<&Ctrack>) -> bool {
    if dp.orig_mod_start != 0 {
        match ct {
            Some(c) => {
                if !cutoff_test(c, dp.orig_mod_start, dp.orig_mod_start_mode) {
                    return false;
                }
            }
            None => return false,
        }
    }
    if dp.orig_mod_cutoff != 0 {
        match ct {
            Some(c) => {
                if c.b_orig_mod_cutoff {
                    return false;
                }
            }
            None => return false,
        }
    }
    true
}

pub fn rewrite_tcp_flags(flags: &mut u16, unset: u16, set: u16) -> bool {
    if set != 0 || unset != 0 {
        let n = (*flags & !unset) | set;
        if n != *flags {
            *flags = n;
            return true;
        }
    }
    false
}

pub fn orig_mod(dp: &Profile, ct: Option<&Ctrack>, pkt: &mut Pkt) -> bool {
    let mut modded = false;
    if check_orig_mod_interval(dp, ct) {
        let ttl = match ct {
            Some(c) if c.orig_autottl != 0 => c.orig_autottl,
            _ => {
                if pkt.v6 { dp.orig_mod_ttl6 } else { dp.orig_mod_ttl }
            }
        };
        if ttl != 0 && pkt.set_ttl(ttl) {
            modded = true;
        }
        if pkt.tcp {
            let mut fl = pkt.tcp_flags();
            if rewrite_tcp_flags(&mut fl, dp.orig_tcp_flags_unset, dp.orig_tcp_flags_set) {
                pkt.set_tcp_flags(fl);
                modded = true;
            }
        }
    }
    modded
}

pub fn pos_normalize(split_pos: usize, reasm_offset: usize, len_payload: usize) -> usize {
    if split_pos > reasm_offset && split_pos - reasm_offset < len_payload { split_pos - reasm_offset } else { 0 }
}

pub fn verdict_csum_fix(verdict: u8, pkt: &mut Pkt) {
    if verdict & VERDICT_NOCSUM == 0 {
        let fix = if cfg!(windows) { verdict & VERDICT_MASK != VERDICT_DROP } else { verdict & VERDICT_MASK == VERDICT_MODIFY };
        if fix {
            pkt.fix_l4_checksum();
        }
    }
}

pub fn ct_new_postnat_fix(ct: Option<&Ctrack>, pkt: &mut Pkt) -> u8 {
    if cfg!(target_os = "linux") {
        let syn = pkt.tcp && {
            let f = pkt.tcp_flags();
            tcp_syn_segment(f) || tcp_synack_segment(f)
        };
        if ct.is_some_and(|c| c.pcounter_orig == 1) || syn {
            if pkt.v6 {
                pkt.buf[6] = 255;
                pkt.buf[7] = 1;
            } else {
                pkt.buf[9] = 255;
                pkt.buf[8] = 1;
            }
            return VERDICT_MODIFY | VERDICT_NOCSUM;
        }
    }
    VERDICT_DROP
}

pub struct Match {
    pub dp: Option<usize>,
    pub check_done: bool,
    pub check_result: bool,
    pub excluded: bool,
}

impl Engine {
    pub fn new(mut params: Params) -> Engine {
        let lists = std::mem::take(&mut params.lists);
        let ct = Conntrack::new(params.ctrack_t_syn, params.ctrack_t_est, params.ctrack_t_fin, params.ctrack_t_udp);
        Engine { params: Arc::new(params), lists, ct, ipcache: HashMap::new(), ipcache_last_purge: Instant::now() }
    }

    fn dp_match(&self, dp: &Profile, tcp: bool, dest: &Endpoint, hostname: Option<&str>, no_subdom: bool, l7: L7) -> (bool, bool, bool, bool) {
        // returns (match, check_done, check_result, excluded)
        let none = (false, false, false, false);
        if (dest.ip.is_ipv4() && !dp.filter_ipv4) || (dest.ip.is_ipv6() && !dp.filter_ipv6) {
            return none;
        }
        if tcp && !port_filters_in_range(&dp.pf_tcp, dest.port) {
            return none;
        }
        if !tcp && !port_filters_in_range(&dp.pf_udp, dest.port) {
            return none;
        }
        if dp.filter_l7 != 0 && !l7_match(l7, dp.filter_l7) {
            return none;
        }
        let lists_empty = self.lists.hostlists_empty(&dp.hl_include) && self.lists.hostlists_empty(&dp.hl_exclude);
        if hostname.is_none() && !lists_empty {
            return none;
        }
        if !self.lists.ipset_check(&dp.ips_include, &dp.ips_exclude, &dest.ip) {
            return none;
        }
        if lists_empty {
            return (true, false, false, false);
        }
        if let Some(h) = hostname {
            let (ok, ex) = self.lists.host_check(&dp.hl_include, &dp.hl_exclude, h, no_subdom);
            return (ok, true, ok, ex);
        }
        none
    }

    pub fn dp_find(&self, tcp: bool, dest: &Endpoint, hostname: Option<&str>, no_subdom: bool, l7: L7) -> Match {
        for (i, dp) in self.params.profiles.iter().enumerate() {
            let (m, done, res, ex) = self.dp_match(dp, tcp, dest, hostname, no_subdom, l7);
            if m {
                crate::dlog!("desync profile {} matches", dp.n);
                return Match { dp: Some(i), check_done: done, check_result: res, excluded: ex };
            }
        }
        crate::dlog!("desync profile not found");
        Match { dp: None, check_done: false, check_result: false, excluded: false }
    }

    pub fn ipcache_purge(&mut self) {
        if self.ipcache_last_purge.elapsed() < Duration::from_secs(60) {
            return;
        }
        self.ipcache_last_purge = Instant::now();
        let life = self.params.ipcache_lifetime;
        if life == 0 {
            return;
        }
        self.ipcache.retain(|_, v| v.touched.is_none_or(|t| t.elapsed().as_secs() < life));
    }

    pub fn ipcache_touch(&mut self, ip: IpAddr, iface: u32) -> &mut IpCacheItem {
        let e = self.ipcache.entry((ip, iface)).or_default();
        e.touched = Some(Instant::now());
        e
    }

    pub fn ipcache_put_hostname(&mut self, ip: IpAddr, host: &str, is_ip: bool) {
        if !self.params.cache_hostname {
            return;
        }
        let e = self.ipcache_touch(ip, 0);
        if e.hostname.as_deref() != Some(host) {
            e.hostname = Some(host.to_string());
            e.hostname_is_ip = is_ip;
        }
    }

    pub fn ipcache_get_hostname(&mut self, ip: IpAddr) -> Option<(String, bool)> {
        if !self.params.cache_hostname {
            return None;
        }
        let e = self.ipcache_touch(ip, 0);
        e.hostname.clone().map(|h| (h, e.hostname_is_ip))
    }

    pub fn autottl_discover(&mut self, ct: &mut Ctrack, dp: &Profile, dst_ip: IpAddr, iface: u32) {
        if !self.params.autottl_present || ct.b_autottl_discovered {
            return;
        }
        let incoming = ct.incoming_ttl;
        let hops = {
            let e = self.ipcache_touch(dst_ip, iface);
            if incoming != 0 {
                e.hops = hop_count_guess(incoming);
            }
            e.hops
        };
        if hops != 0 {
            let v6 = dst_ip.is_ipv6();
            ct.desync_autottl = guess(if v6 { &dp.desync_autottl6 } else { &dp.desync_autottl }, hops);
            ct.orig_autottl = guess(if v6 { &dp.orig_autottl6 } else { &dp.orig_autottl }, hops);
            ct.dup_autottl = guess(if v6 { &dp.dup_autottl6 } else { &dp.dup_autottl }, hops);
        }
        ct.b_autottl_discovered = true;
    }

    pub fn autottl_rediscover(&mut self, ct: &mut Ctrack, dp: &Profile, dst_ip: IpAddr, iface: u32) {
        ct.b_autottl_discovered = false;
        self.autottl_discover(ct, dp, dst_ip, iface);
    }

    pub fn tick(&mut self) {
        self.lists.maybe_reload();
        self.ct.purge();
        self.ipcache_purge();
    }

    // main entry. packet is replaced by the (possibly modified) packet
    pub fn process(&mut self, io: &mut dyn Io, fwmark: u32, ifin: Iface, ifout: Iface, packet: &mut Vec<u8>) -> u8 {
        self.tick();
        self.play(io, false, 0, fwmark, ifin, ifout, packet, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn play(&mut self, io: &mut dyn Io, replay: bool, reasm_offset: usize, fwmark: u32, ifin: Iface, ifout: Iface, packet: &mut Vec<u8>, ct_replay: Option<&mut Ctrack>) -> u8 {
        let Some(mut pkt) = Pkt::dissect(std::mem::take(packet)) else {
            return VERDICT_PASS;
        };
        let verdict;
        if pkt.tcp {
            verdict = self.tcp_play(io, replay, reasm_offset, fwmark, ifin, ifout, &mut pkt, ct_replay);
            verdict_csum_fix(verdict, &mut pkt);
        } else if pkt.udp {
            verdict = self.udp_play(io, replay, reasm_offset, fwmark, ifin, ifout, &mut pkt, ct_replay);
            verdict_csum_fix(verdict, &mut pkt);
        } else {
            verdict = VERDICT_PASS;
        }
        *packet = pkt.buf;
        verdict
    }

    pub fn replay_queue(&mut self, io: &mut dyn Io, q: Vec<Delayed>, ct: &mut Ctrack) -> bool {
        let mut ok = true;
        let mut offset = 0usize;
        for (i, mut d) in q.into_iter().enumerate() {
            crate::dlog!("REPLAYING delayed packet #{} offset {}", i + 1, offset);
            let v = self.play(io, true, offset, d.fwmark, d.ifin, d.ifout, &mut d.packet, Some(&mut *ct));
            match v & VERDICT_MASK {
                VERDICT_MODIFY | VERDICT_PASS => {
                    ok &= io.send(&d.dst, d.fwmark, &d.ifout, &d.packet);
                }
                _ => {}
            }
            offset += d.len_payload;
        }
        ok
    }
}

fn guess(a: &AutoTtl, hops: u8) -> u8 {
    if a.enabled() { a.eval(hops) } else { 0 }
}

// runtime part of fake tls modification. returns modified copy if altered
pub fn runtime_tls_mod(cache: Option<&TlsModCache>, tm: &FakeTlsMod, fake: &[u8], payload: &[u8]) -> Option<Vec<u8>> {
    let cache = cache?;
    let mut out: Option<Vec<u8>> = None;
    let g16 = |d: &[u8], o: usize| u16::from_be_bytes([d[o], d[o + 1]]) as usize;
    if tm.mode & TLS_MOD_PADENCAP != 0 {
        let pl = payload.len();
        let sz_rec = g16(fake, 3) + pl;
        let sz_hs = (((fake[6] as usize) << 16) | ((fake[7] as usize) << 8) | fake[8] as usize) + pl;
        let sz_ext = g16(fake, cache.extlen_offset) + pl;
        let sz_pad = g16(fake, cache.padlen_offset) + pl;
        if sz_rec & !0xFFFF == 0 && sz_hs & !0xFF_FFFF == 0 && sz_ext & !0xFFFF == 0 && sz_pad & !0xFFFF == 0 {
            let mut v = fake.to_vec();
            v[3..5].copy_from_slice(&(sz_rec as u16).to_be_bytes());
            v[6] = (sz_hs >> 16) as u8;
            v[7] = (sz_hs >> 8) as u8;
            v[8] = sz_hs as u8;
            v[cache.extlen_offset..cache.extlen_offset + 2].copy_from_slice(&(sz_ext as u16).to_be_bytes());
            v[cache.padlen_offset..cache.padlen_offset + 2].copy_from_slice(&(sz_pad as u16).to_be_bytes());
            out = Some(v);
        }
    }
    if tm.mode & TLS_MOD_RND != 0 {
        let v = out.get_or_insert_with(|| fake.to_vec());
        rng::fill_bytes(&mut v[11..43]);
        let sl = v[43] as usize;
        rng::fill_bytes(&mut v[44..44 + sl]);
    }
    if tm.mode & TLS_MOD_DUP_SID != 0 && payload.len() >= 44 && fake[43] == payload[43] && payload.len() >= 44 + payload[43] as usize {
        let v = out.get_or_insert_with(|| fake.to_vec());
        let sl = v[43] as usize;
        v[44..44 + sl].copy_from_slice(&payload[44..44 + sl]);
    }
    out
}
