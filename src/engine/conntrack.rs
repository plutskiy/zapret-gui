use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use super::packet::*;
use super::proto::L7;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ConnKey {
    pub src: IpAddr,
    pub dst: IpAddr,
    pub sport: u16,
    pub dport: u16,
    pub tcp: bool,
}

impl ConnKey {
    pub fn from_pkt(p: &Pkt, reverse: bool) -> ConnKey {
        let (s, d) = (p.src_ip(), p.dst_ip());
        let (sp, dp) = (p.sport(), p.dport());
        if reverse {
            ConnKey {
                src: d,
                dst: s,
                sport: dp,
                dport: sp,
                tcp: p.tcp,
            }
        } else {
            ConnKey {
                src: s,
                dst: d,
                sport: sp,
                dport: dp,
                tcp: p.tcp,
            }
        }
    }
    pub fn swapped(&self) -> ConnKey {
        ConnKey {
            src: self.dst,
            dst: self.src,
            sport: self.dport,
            dport: self.sport,
            tcp: self.tcp,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnState {
    Syn,
    Established,
    Fin,
}

#[derive(Default, Debug)]
pub struct Reasm {
    pub packet: Vec<u8>,
    pub seq: u32,
    pub size: usize,
    pub size_present: usize,
}

impl Reasm {
    pub fn clear(&mut self) {
        self.packet = Vec::new();
        self.size = 0;
        self.size_present = 0;
    }
    pub fn init(&mut self, size: usize, seq: u32) {
        self.packet = vec![0; size];
        self.size = size;
        self.size_present = 0;
        self.seq = seq;
    }
    pub fn feed(&mut self, seq: u32, payload: &[u8]) -> bool {
        if self.seq != seq {
            return false;
        }
        let room = self.size - self.size_present;
        let n = payload.len().min(room);
        self.packet[self.size_present..self.size_present + n].copy_from_slice(&payload[..n]);
        self.size_present += n;
        self.seq = self.seq.wrapping_add(n as u32);
        true
    }
    pub fn has_space(&self, len: usize) -> bool {
        self.size_present + len <= self.size
    }
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
    pub fn is_full(&self) -> bool {
        !self.is_empty() && self.size == self.size_present
    }
    pub fn data(&self) -> &[u8] {
        &self.packet[..self.size_present]
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Iface {
    pub idx: u32,
    pub sub: u32,
}

#[derive(Debug)]
pub struct Delayed {
    pub dst: Endpoint,
    pub fwmark: u32,
    pub ifin: Iface,
    pub ifout: Iface,
    pub packet: Vec<u8>,
    pub len_payload: usize,
}

#[derive(Debug)]
pub struct Ctrack {
    pub b_check_done: bool,
    pub b_check_result: bool,
    pub b_check_excluded: bool,

    pub dp: Option<usize>,
    pub dp_search_complete: bool,

    pub t_last: Instant,
    pub pcounter_orig: u64,
    pub pcounter_reply: u64,
    pub pdcounter_orig: u64,
    pub pdcounter_reply: u64,
    pub pos_orig: u32,
    pub pos_reply: u32,
    pub seq_last: u32,
    pub ack_last: u32,

    pub state: ConnState,
    pub seq0: u32,
    pub ack0: u32,
    pub winsize_orig: u16,
    pub winsize_reply: u16,
    pub scale_orig: u8,
    pub scale_reply: u8,

    pub req_seq_present: bool,
    pub req_seq_finalized: bool,
    pub req_seq_abandoned: bool,
    pub req_seq_start: u32,
    pub req_seq_end: u32,

    pub incoming_ttl: u8,
    pub desync_autottl: u8,
    pub orig_autottl: u8,
    pub dup_autottl: u8,
    pub b_autottl_discovered: bool,

    pub b_cutoff: bool,
    pub b_wssize_cutoff: bool,
    pub b_desync_cutoff: bool,
    pub b_dup_cutoff: bool,
    pub b_orig_mod_cutoff: bool,

    pub ip_id: u16,

    pub l7proto: L7,
    pub l7proto_discovered: bool,
    pub hostname: Option<String>,
    pub hostname_is_ip: bool,
    pub hostname_discovered: bool,

    pub reasm_orig: Reasm,
    pub delayed: Vec<Delayed>,
}

impl Ctrack {
    pub fn new() -> Ctrack {
        let now = Instant::now();
        Ctrack {
            b_check_done: false,
            b_check_result: false,
            b_check_excluded: false,
            dp: None,
            dp_search_complete: false,
            t_last: now,
            pcounter_orig: 0,
            pcounter_reply: 0,
            pdcounter_orig: 0,
            pdcounter_reply: 0,
            pos_orig: 0,
            pos_reply: 0,
            seq_last: 0,
            ack_last: 0,
            state: ConnState::Syn,
            seq0: 0,
            ack0: 0,
            winsize_orig: 0,
            winsize_reply: 0,
            scale_orig: SCALE_NONE,
            scale_reply: SCALE_NONE,
            req_seq_present: false,
            req_seq_finalized: false,
            req_seq_abandoned: false,
            req_seq_start: 0,
            req_seq_end: 0,
            incoming_ttl: 0,
            desync_autottl: 0,
            orig_autottl: 0,
            dup_autottl: 0,
            b_autottl_discovered: false,
            b_cutoff: false,
            b_wssize_cutoff: false,
            b_desync_cutoff: false,
            b_dup_cutoff: false,
            b_orig_mod_cutoff: false,
            ip_id: 0,
            l7proto: L7::Unknown,
            l7proto_discovered: false,
            hostname: None,
            hostname_is_ip: false,
            hostname_discovered: false,
            reasm_orig: Reasm::default(),
            delayed: Vec::new(),
        }
    }

    // non-tcp packets are passed with tcp=None but len_payload filled
    fn feed_packet(&mut self, reverse: bool, tcp: Option<&Pkt>, len_payload: usize) {
        if reverse {
            self.pcounter_reply += 1;
            self.pdcounter_reply += u64::from(len_payload != 0);
        } else {
            self.pcounter_orig += 1;
            self.pdcounter_orig += u64::from(len_payload != 0);
        }
        if let Some(p) = tcp {
            let fl = p.tcp_flags();
            if tcp_syn_segment(fl) {
                if self.state != ConnState::Syn {
                    self.reinit();
                }
                self.seq0 = p.seq();
            } else if tcp_synack_segment(fl) {
                let seq0 = p.ack().wrapping_sub(1);
                if self.state != ConnState::Syn && self.seq0 != seq0 {
                    self.reinit();
                }
                if self.seq0 == 0 {
                    self.seq0 = seq0;
                }
                self.ack0 = p.seq();
            } else if fl & (TH_FIN | TH_RST) != 0 {
                self.state = ConnState::Fin;
            } else if self.state == ConnState::Syn {
                self.state = ConnState::Established;
                if !reverse && self.ack0 == 0 {
                    self.ack0 = p.ack().wrapping_sub(1);
                }
            }
            let scale = p.tcp_scale_factor();
            if reverse {
                self.seq_last = p.ack();
                self.pos_orig = self.seq_last;
                self.ack_last = p.seq();
                self.pos_reply = self.ack_last.wrapping_add(len_payload as u32);
                self.winsize_reply = p.win();
                if scale != SCALE_NONE {
                    self.scale_reply = scale;
                }
            } else {
                self.seq_last = p.seq();
                self.pos_orig = self.seq_last.wrapping_add(len_payload as u32);
                self.ack_last = p.ack();
                self.pos_reply = self.ack_last;
                self.winsize_orig = p.win();
                if scale != SCALE_NONE {
                    self.scale_orig = scale;
                }
            }
        } else if reverse {
            self.ack_last = self.pos_reply;
            self.pos_reply = self.pos_reply.wrapping_add(len_payload as u32);
        } else {
            self.seq_last = self.pos_orig;
            self.pos_orig = self.pos_orig.wrapping_add(len_payload as u32);
        }
        self.t_last = Instant::now();
    }

    fn reinit(&mut self) {
        *self = Ctrack::new();
    }
}

pub struct Conntrack {
    pub map: HashMap<ConnKey, Ctrack>,
    pub timeout_syn: u64,
    pub timeout_established: u64,
    pub timeout_fin: u64,
    pub timeout_udp: u64,
    last_purge: Instant,
}

impl Conntrack {
    pub fn new(syn: u64, est: u64, fin: u64, udp: u64) -> Self {
        Conntrack {
            map: HashMap::new(),
            timeout_syn: syn,
            timeout_established: est,
            timeout_fin: fin,
            timeout_udp: udp,
            last_purge: Instant::now(),
        }
    }

    pub fn purge(&mut self) {
        if self.last_purge.elapsed() < Duration::from_secs(10) {
            return;
        }
        let (ts, te, tf, tu) = (
            self.timeout_syn,
            self.timeout_established,
            self.timeout_fin,
            self.timeout_udp,
        );
        self.map.retain(|k, t| {
            let idle = t.t_last.elapsed().as_secs();
            !(t.b_cutoff
                || (k.tcp
                    && ((t.state == ConnState::Syn && idle >= ts)
                        || (t.state == ConnState::Established && idle >= te)
                        || (t.state == ConnState::Fin && idle >= tf)))
                || (!k.tcp && idle >= tu))
        });
        self.last_purge = Instant::now();
    }

    // find or create + feed. returns (key, reverse). the entry is left in the map
    pub fn feed(&mut self, p: &Pkt, len_payload: usize) -> Option<(ConnKey, bool)> {
        let conn = ConnKey::from_pkt(p, false);
        let tcp = if p.tcp { Some(p) } else { None };
        if let Some(t) = self.map.get_mut(&conn) {
            t.feed_packet(false, tcp, len_payload);
            return Some((conn, false));
        }
        let swp = conn.swapped();
        if let Some(t) = self.map.get_mut(&swp) {
            t.feed_packet(true, tcp, len_payload);
            return Some((swp, true));
        }
        let rev = p.tcp && tcp_synack_segment(p.tcp_flags());
        if (p.tcp && tcp_syn_segment(p.tcp_flags())) || rev || p.udp {
            let key = if rev { swp } else { conn };
            let mut t = Ctrack::new();
            t.feed_packet(rev, tcp, len_payload);
            self.map.insert(key, t);
            return Some((key, rev));
        }
        None
    }
}

pub fn seq_within(s: u32, s1: u32, s2: u32) -> bool {
    (s2 >= s1 && s >= s1 && s <= s2) || (s2 < s1 && (s <= s2 || s >= s1))
}
