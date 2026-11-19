use super::fakes::{FAKE_HTTP_REQUEST_DEFAULT, FAKE_TLS_CLIENTHELLO_DEFAULT};
use super::lists::Lists;
use super::packet::*;
use super::proto::*;
use super::rng;

pub const FAKE_MAX_TCP: usize = 1460;
pub const FAKE_MAX_UDP: usize = 1472;
pub const MAX_SPLITS: usize = 64;
pub const DPI_DESYNC_MAX_FAKE_LEN: usize = 9216;
pub const FWMARK_DEFAULT: u32 = 0x4000_0000;

pub const BADSEQ_INCREMENT_DEFAULT: u32 = (-10000i32) as u32;
pub const BADSEQ_ACK_INCREMENT_DEFAULT: u32 = (-66000i32) as u32;
pub const TS_INCREMENT_DEFAULT: u32 = (-600000i32) as u32;

pub const TLS_MOD_SAVE_MASK: u32 = 0x0F;
pub const TLS_MOD_SET: u32 = 0x01;
pub const TLS_MOD_CUSTOM_FAKE: u32 = 0x02;
pub const TLS_MOD_RND: u32 = 0x10;
pub const TLS_MOD_DUP_SID: u32 = 0x20;
pub const TLS_MOD_RND_SNI: u32 = 0x40;
pub const TLS_MOD_SNI: u32 = 0x80;
pub const TLS_MOD_PADENCAP: u32 = 0x100;

pub const TLD: [&str; 6] = ["com", "org", "net", "edu", "gov", "biz"];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    #[default]
    None,
    Fake,
    FakeKnown,
    Rst,
    RstAck,
    SynAck,
    SynData,
    FakedSplit,
    FakedDisorder,
    MultiSplit,
    MultiDisorder,
    HostFakeSplit,
    Udplen,
    Tamper,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SynackSplit {
    #[default]
    None,
    Syn,
    SynAck,
    AckSyn,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum IpIdMode {
    #[default]
    Seq,
    SeqGroup,
    Rnd,
    Zero,
    Same,
}

#[derive(Clone, Debug, Default)]
pub struct FakeTlsMod {
    pub sni: String,
    pub mode: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TlsModCache {
    pub extlen_offset: usize,
    pub padlen_offset: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Blob {
    pub data: Vec<u8>,
    pub offset: usize,
    pub size_buf: usize,
    pub tls_mod: Option<FakeTlsMod>,
    pub tls_cache: Option<TlsModCache>,
}

#[derive(Clone, Copy, Debug)]
pub struct PortFilter {
    pub from: u16,
    pub to: u16,
    pub neg: bool,
}

impl PortFilter {
    pub fn in_range(&self, port: u16) -> bool {
        port != 0 && (((self.from == 0 && self.to == 0) || (port >= self.from && port <= self.to)) ^ self.neg)
    }
}

pub fn pf_parse(s: &str) -> Option<PortFilter> {
    if s == "*" {
        return Some(PortFilter { from: 1, to: 0xFFFF, neg: false });
    }
    let (neg, s) = match s.strip_prefix('~') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let (from, to) = if let Some((a, b)) = s.split_once('-') {
        let (a, b): (u32, u32) = (a.parse().ok()?, b.parse().ok()?);
        if a > 65535 || b > 65535 || a > b {
            return None;
        }
        (a as u16, b as u16)
    } else {
        let a: u32 = s.parse().ok()?;
        if a > 65535 {
            return None;
        }
        (a as u16, a as u16)
    };
    let neg = if from == 0 && to == 0 { true } else { neg };
    Some(PortFilter { from, to, neg })
}

pub fn port_filters_in_range(list: &[PortFilter], port: u16) -> bool {
    list.is_empty() || list.iter().any(|p| p.in_range(port))
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub n: usize,

    pub wsize: u16,
    pub wscale: u8,
    pub wssize: u16,
    pub wsscale: u8,
    pub wssize_cutoff_mode: char,
    pub wssize_cutoff: u32,
    pub wssize_no_forced_cutoff: bool,

    pub synack_split: SynackSplit,
    pub ip_id_mode: IpIdMode,

    pub hostcase: bool,
    pub hostnospace: bool,
    pub domcase: bool,
    pub methodeol: bool,
    pub hostspell: [u8; 4],

    pub desync_mode0: Mode,
    pub desync_mode: Mode,
    pub desync_mode2: Mode,
    pub desync_skip_nosni: bool,
    pub desync_any_proto: bool,
    pub desync_repeats: u32,

    pub splits: Vec<ProtoPos>,
    pub seqovl: ProtoPos,
    pub hostfakesplit_midhost: ProtoPos,

    pub dup_start_mode: char,
    pub dup_cutoff_mode: char,
    pub dup_replace: bool,
    pub dup_start: u32,
    pub dup_cutoff: u32,
    pub dup_repeats: u32,
    pub dup_ttl: u8,
    pub dup_ttl6: u8,
    pub dup_fooling_mode: u32,
    pub dup_ts_increment: u32,
    pub dup_badseq_increment: u32,
    pub dup_badseq_ack_increment: u32,
    pub dup_autottl: AutoTtl,
    pub dup_autottl6: AutoTtl,
    pub dup_tcp_flags_set: u16,
    pub dup_tcp_flags_unset: u16,
    pub dup_ip_id_mode: IpIdMode,

    pub orig_mod_start_mode: char,
    pub orig_mod_cutoff_mode: char,
    pub orig_mod_start: u32,
    pub orig_mod_cutoff: u32,
    pub orig_mod_ttl: u8,
    pub orig_mod_ttl6: u8,
    pub orig_autottl: AutoTtl,
    pub orig_autottl6: AutoTtl,
    pub orig_tcp_flags_set: u16,
    pub orig_tcp_flags_unset: u16,

    pub desync_start_mode: char,
    pub desync_cutoff_mode: char,
    pub desync_start: u32,
    pub desync_cutoff: u32,
    pub desync_ttl: u8,
    pub desync_ttl6: u8,
    pub desync_autottl: AutoTtl,
    pub desync_autottl6: AutoTtl,
    pub desync_fooling_mode: u32,
    pub desync_ts_increment: u32,
    pub desync_badseq_increment: u32,
    pub desync_badseq_ack_increment: u32,
    pub desync_tcp_flags_set: u16,
    pub desync_tcp_flags_unset: u16,

    pub fake_http: Vec<Blob>,
    pub fake_tls: Vec<Blob>,
    pub fake_unknown: Vec<Blob>,
    pub fake_unknown_udp: Vec<Blob>,
    pub fake_quic: Vec<Blob>,
    pub fake_wg: Vec<Blob>,
    pub fake_dht: Vec<Blob>,
    pub fake_discord: Vec<Blob>,
    pub fake_stun: Vec<Blob>,
    pub fake_syndata: Vec<u8>,
    pub seqovl_pattern: Vec<u8>,
    pub udplen_pattern: Vec<u8>,
    pub fsplit_pattern: Vec<u8>,
    pub fsplit_pattern_set: bool,

    pub tls_mod_last: FakeTlsMod,
    pub tls_fake_last: Option<usize>,

    pub hfs_host: String,
    pub hfs_ordering: i32,
    pub fs_ordering: i32,
    pub tcp_mod_seq: bool,

    pub udplen_increment: i32,

    pub filter_ipv4: bool,
    pub filter_ipv6: bool,
    pub pf_tcp: Vec<PortFilter>,
    pub pf_udp: Vec<PortFilter>,
    pub filter_l7: u32,

    pub ips_include: Vec<usize>,
    pub ips_exclude: Vec<usize>,
    pub hl_include: Vec<usize>,
    pub hl_exclude: Vec<usize>,
}

impl Profile {
    pub fn new(n: usize) -> Self {
        Profile {
            n,
            wsize: 0,
            wscale: SCALE_NONE,
            wssize: 0,
            wsscale: 0,
            wssize_cutoff_mode: 'n',
            wssize_cutoff: 0,
            wssize_no_forced_cutoff: false,
            synack_split: SynackSplit::None,
            ip_id_mode: IpIdMode::Seq,
            hostcase: false,
            hostnospace: false,
            domcase: false,
            methodeol: false,
            hostspell: *b"host",
            desync_mode0: Mode::None,
            desync_mode: Mode::None,
            desync_mode2: Mode::None,
            desync_skip_nosni: true,
            desync_any_proto: false,
            desync_repeats: 1,
            splits: Vec::new(),
            seqovl: ProtoPos::EMPTY,
            hostfakesplit_midhost: ProtoPos::EMPTY,
            dup_start_mode: 'n',
            dup_cutoff_mode: 'n',
            dup_replace: false,
            dup_start: 0,
            dup_cutoff: 0,
            dup_repeats: 0,
            dup_ttl: 0,
            dup_ttl6: 0xFF,
            dup_fooling_mode: 0,
            dup_ts_increment: TS_INCREMENT_DEFAULT,
            dup_badseq_increment: BADSEQ_INCREMENT_DEFAULT,
            dup_badseq_ack_increment: BADSEQ_ACK_INCREMENT_DEFAULT,
            dup_autottl: AutoTtl::default(),
            dup_autottl6: AutoTtl::default(),
            dup_tcp_flags_set: 0,
            dup_tcp_flags_unset: 0,
            dup_ip_id_mode: IpIdMode::Same,
            orig_mod_start_mode: 'n',
            orig_mod_cutoff_mode: 'n',
            orig_mod_start: 0,
            orig_mod_cutoff: 0,
            orig_mod_ttl: 0,
            orig_mod_ttl6: 0xFF,
            orig_autottl: AutoTtl::default(),
            orig_autottl6: AutoTtl::default(),
            orig_tcp_flags_set: 0,
            orig_tcp_flags_unset: 0,
            desync_start_mode: 'n',
            desync_cutoff_mode: 'n',
            desync_start: 0,
            desync_cutoff: 0,
            desync_ttl: 0,
            desync_ttl6: 0xFF,
            desync_autottl: AutoTtl::default(),
            desync_autottl6: AutoTtl::default(),
            desync_fooling_mode: 0,
            desync_ts_increment: TS_INCREMENT_DEFAULT,
            desync_badseq_increment: BADSEQ_INCREMENT_DEFAULT,
            desync_badseq_ack_increment: BADSEQ_ACK_INCREMENT_DEFAULT,
            desync_tcp_flags_set: 0,
            desync_tcp_flags_unset: 0,
            fake_http: Vec::new(),
            fake_tls: Vec::new(),
            fake_unknown: Vec::new(),
            fake_unknown_udp: Vec::new(),
            fake_quic: Vec::new(),
            fake_wg: Vec::new(),
            fake_dht: Vec::new(),
            fake_discord: Vec::new(),
            fake_stun: Vec::new(),
            fake_syndata: vec![0; 16],
            seqovl_pattern: vec![0; FAKE_MAX_TCP],
            udplen_pattern: vec![0; FAKE_MAX_UDP],
            fsplit_pattern: Vec::new(),
            fsplit_pattern_set: false,
            tls_mod_last: FakeTlsMod::default(),
            tls_fake_last: None,
            hfs_host: String::new(),
            hfs_ordering: 0,
            fs_ordering: 0,
            tcp_mod_seq: false,
            udplen_increment: 2,
            filter_ipv4: true,
            filter_ipv6: true,
            pf_tcp: Vec::new(),
            pf_udp: Vec::new(),
            filter_l7: 0,
            ips_include: Vec::new(),
            ips_exclude: Vec::new(),
            hl_include: Vec::new(),
            hl_exclude: Vec::new(),
        }
    }

    pub fn has_orig_mod(&self) -> bool {
        self.orig_mod_ttl != 0 || self.orig_mod_ttl6 != 0 || self.orig_tcp_flags_set != 0 || self.orig_tcp_flags_unset != 0
    }
}

#[derive(Clone, Debug)]
pub struct WinFilterOpts {
    pub iface: (u32, u32),
    pub ipv4: bool,
    pub ipv6: bool,
    pub tcp: String,
    pub udp: String,
    pub raw: Option<String>,
    pub raw_part: Vec<String>,
    pub filter_lan: bool,
}

pub struct Params {
    pub profiles: Vec<Profile>,
    pub lists: Lists,
    pub qnum: i32,
    pub desync_fwmark: u32,
    pub ctrack_t_syn: u64,
    pub ctrack_t_est: u64,
    pub ctrack_t_fin: u64,
    pub ctrack_t_udp: u64,
    pub ctrack_disable: bool,
    pub ipcache_lifetime: u64,
    pub cache_hostname: bool,
    pub autottl_present: bool,
    pub debug: bool,
    pub wf: WinFilterOpts,
}

impl Params {
    pub fn need_all_out(&self) -> bool {
        self.profiles.iter().any(|p| p.dup_repeats != 0 || p.has_orig_mod())
    }
}

fn atoi(s: &str) -> i64 {
    let s = s.trim_start();
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    let v: i64 = digits.parse().unwrap_or(0);
    if neg { -v } else { v }
}

fn parse_cutoff(s: &str) -> Option<(u32, char)> {
    let (mode, rest) = match s.chars().next() {
        Some(c @ ('n' | 'd' | 's')) => (c, &s[1..]),
        _ => ('n', s),
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    Some((digits.parse().ok()?, mode))
}

fn parse_net32_signed(s: &str) -> Option<u32> {
    let (neg, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    if let Some(h) = body.strip_prefix("0x") {
        let v = u32::from_str_radix(h, 16).ok()?;
        return Some(if neg { v.wrapping_neg() } else { v });
    }
    s.parse::<i32>().ok().map(|v| v as u32)
}

fn parse_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

// 0xaabbcc | filename | @filename | +123@filename
fn load_file_or_hex(name: &str, max: usize) -> Result<(Vec<u8>, usize), String> {
    if let Some(h) = name.strip_prefix("0x") {
        let v = parse_hex(h).filter(|v| !v.is_empty()).ok_or_else(|| format!("invalid hex string: {h}"))?;
        return Ok((v, 0));
    }
    let mut ofs = 0usize;
    let mut f = name;
    if let Some(r) = f.strip_prefix('+') {
        let digits: String = r.chars().take_while(|c| c.is_ascii_digit()).collect();
        ofs = digits.parse().map_err(|_| format!("offset read error: {r}"))?;
        f = r.split_once('@').map(|x| x.1).unwrap_or("");
    } else if let Some(r) = f.strip_prefix('@') {
        f = r;
    }
    let mut data = std::fs::read(f).map_err(|e| format!("could not read {f}: {e}"))?;
    if data.is_empty() {
        return Err(format!("could not read {f}: empty"));
    }
    data.truncate(max);
    if ofs >= data.len() {
        return Err(format!("'{f}' : offset {ofs} is out of data range {}", data.len()));
    }
    Ok((data, ofs))
}

fn parse_fooling(s: &str) -> Option<u32> {
    let mut m = 0;
    for p in s.split(',') {
        m |= match p {
            "md5sig" => FOOL_MD5SIG,
            "ts" => FOOL_TS,
            "badsum" => FOOL_BADSUM,
            "badseq" => FOOL_BADSEQ,
            "datanoack" => FOOL_DATANOACK,
            "hopbyhop" => FOOL_HOPBYHOP,
            "hopbyhop2" => FOOL_HOPBYHOP2,
            "none" => 0,
            _ => return None,
        };
    }
    Some(m)
}

fn parse_tcpflags(s: &str) -> Option<u16> {
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        let v = u32::from_str_radix(h, 16).ok()?;
        return (v & 0xFFF == v).then_some(v as u16);
    }
    if let Ok(v) = s.parse::<u32>() {
        return (v & 0xFFF == v).then_some(v as u16);
    }
    let mut fl = 0u16;
    for p in s.split(',') {
        fl |= match p.to_ascii_uppercase().as_str() {
            "FIN" => TH_FIN,
            "SYN" => TH_SYN,
            "RST" => TH_RST,
            "PSH" | "PUSH" => TH_PUSH,
            "ACK" => TH_ACK,
            "URG" => TH_URG,
            "ECE" => 0x40,
            "CWR" => 0x80,
            "AE" | "AECN" | "ACCECN" => 0x100,
            "R1" => 0x200,
            "R2" => 0x400,
            "R3" => 0x800,
            _ => return None,
        };
    }
    Some(fl)
}

fn parse_autottl(s: Option<&str>, def_delta: i8, def_min: u8, def_max: u8) -> Option<AutoTtl> {
    let mut t = AutoTtl { delta: def_delta, min: def_min, max: def_max };
    let Some(s) = s else { return Some(t) };
    if s == "-" {
        return Some(AutoTtl::default());
    }
    let (neg, s) = if let Some(r) = s.strip_prefix('+') {
        (false, r)
    } else if let Some(r) = s.strip_prefix('-') {
        (true, r)
    } else {
        (true, s)
    };
    let parts: Vec<&str> = s.split([':', '-']).collect();
    let delta: u32 = parts.first()?.parse().ok()?;
    if parts.len() >= 3 {
        let max: u32 = parts[2].parse().ok()?;
        if (delta != 0 && max == 0) || max > 255 {
            return None;
        }
        t.max = max as u8;
    }
    if parts.len() >= 2 {
        let min: u32 = parts[1].parse().ok()?;
        if (delta != 0 && min == 0) || min > 255 || min > t.max as u32 {
            return None;
        }
        t.min = min as u8;
    }
    if delta > 127 {
        return None;
    }
    t.delta = if neg { -(delta as i32) as i8 } else { delta as i8 };
    Some(t)
}

fn parse_l7_list(s: &str) -> Option<u32> {
    let mut m = 0;
    for p in s.split(',') {
        m |= match p {
            "http" => L7_HTTP,
            "tls" => L7_TLS,
            "quic" => L7_QUIC,
            "wireguard" => L7_WIREGUARD,
            "dht" => L7_DHT,
            "discord" => L7_DISCORD,
            "stun" => L7_STUN,
            "unknown" => L7_UNKNOWN,
            _ => return None,
        };
    }
    Some(m)
}

fn parse_marker(s: &str) -> Option<Marker> {
    Some(match s {
        "host" => Marker::Host,
        "endhost" => Marker::HostEnd,
        "sld" => Marker::HostSld,
        "midsld" => Marker::HostMidSld,
        "endsld" => Marker::HostEndSld,
        "method" => Marker::HttpMethod,
        "sniext" => Marker::SniExt,
        _ => return None,
    })
}

fn parse_int16(s: &str) -> Option<i16> {
    let c = s.chars().next()?;
    if c == '+' || c == '-' || c.is_ascii_digit() {
        let v = atoi(s);
        i16::try_from(v).ok()
    } else {
        None
    }
}

fn parse_split_pos(s: &str) -> Option<ProtoPos> {
    if let Some(v) = parse_int16(s) {
        return if v != 0 { Some(ProtoPos { marker: Marker::Abs, pos: v }) } else { None };
    }
    let end = s.find(['+', '-']).unwrap_or(s.len());
    let marker = parse_marker(&s[..end])?;
    let pos = if end < s.len() { parse_int16(&s[end..])? } else { 0 };
    Some(ProtoPos { marker, pos })
}

fn parse_tlsmod_list(s: &str, tm: &mut FakeTlsMod) -> bool {
    tm.mode &= TLS_MOD_SAVE_MASK;
    tm.mode |= TLS_MOD_SET;
    for item in s.split(',') {
        let (name, val) = match item.split_once('=') {
            Some((n, v)) => (n, Some(v)),
            None => (item, None),
        };
        match name {
            "rnd" => tm.mode |= TLS_MOD_RND,
            "rndsni" => tm.mode |= TLS_MOD_RND_SNI,
            "sni" => {
                tm.mode |= TLS_MOD_SNI;
                match val {
                    Some(v) if !v.is_empty() => tm.sni = v.chars().take(126).collect(),
                    _ => return false,
                }
            }
            "padencap" => tm.mode |= TLS_MOD_PADENCAP,
            "dupsid" => tm.mode |= TLS_MOD_DUP_SID,
            "none" => {}
            _ => return false,
        }
    }
    true
}

fn parse_desync_mode(s: Option<&str>) -> Result<Option<Mode>, String> {
    let Some(s) = s else { return Ok(Some(Mode::None)) };
    Ok(Some(match s {
        "fake" => Mode::Fake,
        "fakeknown" => Mode::FakeKnown,
        "rst" => Mode::Rst,
        "rstack" => Mode::RstAck,
        "synack" => Mode::SynAck,
        "syndata" => Mode::SynData,
        "fakeddisorder" | "disorder" => Mode::FakedDisorder,
        "fakedsplit" | "split" => Mode::FakedSplit,
        "multisplit" | "split2" => Mode::MultiSplit,
        "multidisorder" | "disorder2" => Mode::MultiDisorder,
        "hostfakesplit" => Mode::HostFakeSplit,
        "udplen" => Mode::Udplen,
        "tamper" => Mode::Tamper,
        "ipfrag2" | "hopbyhop" | "destopt" | "ipfrag1" => {
            return Err(format!("dpi-desync mode '{s}' is not supported by this engine"));
        }
        _ => return Ok(None),
    }))
}

fn valid_zero_stage(m: Mode) -> bool {
    matches!(m, Mode::SynAck | Mode::SynData)
}
fn valid_first_stage(m: Mode) -> bool {
    matches!(m, Mode::Fake | Mode::FakeKnown | Mode::Rst | Mode::RstAck)
}
fn valid_second_stage(m: Mode) -> bool {
    matches!(
        m,
        Mode::None | Mode::FakedDisorder | Mode::FakedSplit | Mode::MultiSplit | Mode::MultiDisorder | Mode::HostFakeSplit | Mode::Udplen | Mode::Tamper
    )
}

struct OptSpec {
    name: &'static str,
    kind: u8, // 0 none, 1 required, 2 optional
}

const OPTS: &[OptSpec] = &[
    OptSpec { name: "debug", kind: 2 },
    OptSpec { name: "dry-run", kind: 0 },
    OptSpec { name: "version", kind: 0 },
    OptSpec { name: "comment", kind: 2 },
    OptSpec { name: "qnum", kind: 1 },
    OptSpec { name: "daemon", kind: 0 },
    OptSpec { name: "pidfile", kind: 1 },
    OptSpec { name: "user", kind: 1 },
    OptSpec { name: "uid", kind: 1 },
    OptSpec { name: "wsize", kind: 1 },
    OptSpec { name: "wssize", kind: 1 },
    OptSpec { name: "wssize-cutoff", kind: 1 },
    OptSpec { name: "wssize-forced-cutoff", kind: 1 },
    OptSpec { name: "synack-split", kind: 2 },
    OptSpec { name: "ctrack-timeouts", kind: 1 },
    OptSpec { name: "ctrack-disable", kind: 2 },
    OptSpec { name: "ipcache-lifetime", kind: 1 },
    OptSpec { name: "ipcache-hostname", kind: 2 },
    OptSpec { name: "hostcase", kind: 0 },
    OptSpec { name: "hostspell", kind: 1 },
    OptSpec { name: "hostnospace", kind: 0 },
    OptSpec { name: "domcase", kind: 0 },
    OptSpec { name: "methodeol", kind: 0 },
    OptSpec { name: "ip-id", kind: 1 },
    OptSpec { name: "dpi-desync", kind: 1 },
    OptSpec { name: "dpi-desync-fwmark", kind: 1 },
    OptSpec { name: "dup", kind: 1 },
    OptSpec { name: "dup-ttl", kind: 1 },
    OptSpec { name: "dup-ttl6", kind: 1 },
    OptSpec { name: "dup-autottl", kind: 2 },
    OptSpec { name: "dup-autottl6", kind: 2 },
    OptSpec { name: "dup-tcp-flags-set", kind: 2 },
    OptSpec { name: "dup-tcp-flags-unset", kind: 2 },
    OptSpec { name: "dup-fooling", kind: 1 },
    OptSpec { name: "dup-ts-increment", kind: 1 },
    OptSpec { name: "dup-badseq-increment", kind: 1 },
    OptSpec { name: "dup-badack-increment", kind: 1 },
    OptSpec { name: "dup-replace", kind: 2 },
    OptSpec { name: "dup-ip-id", kind: 1 },
    OptSpec { name: "dup-start", kind: 1 },
    OptSpec { name: "dup-cutoff", kind: 1 },
    OptSpec { name: "orig-ttl", kind: 1 },
    OptSpec { name: "orig-ttl6", kind: 1 },
    OptSpec { name: "orig-autottl", kind: 2 },
    OptSpec { name: "orig-autottl6", kind: 2 },
    OptSpec { name: "orig-tcp-flags-set", kind: 2 },
    OptSpec { name: "orig-tcp-flags-unset", kind: 2 },
    OptSpec { name: "orig-mod-start", kind: 1 },
    OptSpec { name: "orig-mod-cutoff", kind: 1 },
    OptSpec { name: "dpi-desync-ttl", kind: 1 },
    OptSpec { name: "dpi-desync-ttl6", kind: 1 },
    OptSpec { name: "dpi-desync-autottl", kind: 2 },
    OptSpec { name: "dpi-desync-autottl6", kind: 2 },
    OptSpec { name: "dpi-desync-tcp-flags-set", kind: 2 },
    OptSpec { name: "dpi-desync-tcp-flags-unset", kind: 2 },
    OptSpec { name: "dpi-desync-fooling", kind: 1 },
    OptSpec { name: "dpi-desync-repeats", kind: 1 },
    OptSpec { name: "dpi-desync-skip-nosni", kind: 2 },
    OptSpec { name: "dpi-desync-split-pos", kind: 1 },
    OptSpec { name: "dpi-desync-split-http-req", kind: 1 },
    OptSpec { name: "dpi-desync-split-tls", kind: 1 },
    OptSpec { name: "dpi-desync-split-seqovl", kind: 1 },
    OptSpec { name: "dpi-desync-split-seqovl-pattern", kind: 1 },
    OptSpec { name: "dpi-desync-fakedsplit-pattern", kind: 1 },
    OptSpec { name: "dpi-desync-fakedsplit-mod", kind: 1 },
    OptSpec { name: "dpi-desync-hostfakesplit-midhost", kind: 1 },
    OptSpec { name: "dpi-desync-hostfakesplit-mod", kind: 1 },
    OptSpec { name: "dpi-desync-ipfrag-pos-tcp", kind: 1 },
    OptSpec { name: "dpi-desync-ipfrag-pos-udp", kind: 1 },
    OptSpec { name: "dpi-desync-ts-increment", kind: 1 },
    OptSpec { name: "dpi-desync-badseq-increment", kind: 1 },
    OptSpec { name: "dpi-desync-badack-increment", kind: 1 },
    OptSpec { name: "dpi-desync-any-protocol", kind: 2 },
    OptSpec { name: "dpi-desync-fake-tcp-mod", kind: 1 },
    OptSpec { name: "dpi-desync-fake-http", kind: 1 },
    OptSpec { name: "dpi-desync-fake-tls", kind: 1 },
    OptSpec { name: "dpi-desync-fake-tls-mod", kind: 1 },
    OptSpec { name: "dpi-desync-fake-unknown", kind: 1 },
    OptSpec { name: "dpi-desync-fake-syndata", kind: 1 },
    OptSpec { name: "dpi-desync-fake-quic", kind: 1 },
    OptSpec { name: "dpi-desync-fake-wireguard", kind: 1 },
    OptSpec { name: "dpi-desync-fake-dht", kind: 1 },
    OptSpec { name: "dpi-desync-fake-discord", kind: 1 },
    OptSpec { name: "dpi-desync-fake-stun", kind: 1 },
    OptSpec { name: "dpi-desync-fake-unknown-udp", kind: 1 },
    OptSpec { name: "dpi-desync-udplen-increment", kind: 1 },
    OptSpec { name: "dpi-desync-udplen-pattern", kind: 1 },
    OptSpec { name: "dpi-desync-cutoff", kind: 1 },
    OptSpec { name: "dpi-desync-start", kind: 1 },
    OptSpec { name: "hostlist", kind: 1 },
    OptSpec { name: "hostlist-domains", kind: 1 },
    OptSpec { name: "hostlist-exclude", kind: 1 },
    OptSpec { name: "hostlist-exclude-domains", kind: 1 },
    OptSpec { name: "hostlist-auto", kind: 1 },
    OptSpec { name: "hostlist-auto-fail-threshold", kind: 1 },
    OptSpec { name: "hostlist-auto-fail-time", kind: 1 },
    OptSpec { name: "hostlist-auto-retrans-threshold", kind: 1 },
    OptSpec { name: "hostlist-auto-debug", kind: 1 },
    OptSpec { name: "new", kind: 0 },
    OptSpec { name: "skip", kind: 0 },
    OptSpec { name: "filter-l3", kind: 1 },
    OptSpec { name: "filter-tcp", kind: 1 },
    OptSpec { name: "filter-udp", kind: 1 },
    OptSpec { name: "filter-l7", kind: 1 },
    OptSpec { name: "ipset", kind: 1 },
    OptSpec { name: "ipset-ip", kind: 1 },
    OptSpec { name: "ipset-exclude", kind: 1 },
    OptSpec { name: "ipset-exclude-ip", kind: 1 },
    OptSpec { name: "bind-fix4", kind: 0 },
    OptSpec { name: "bind-fix6", kind: 0 },
    OptSpec { name: "wf-iface", kind: 1 },
    OptSpec { name: "wf-l3", kind: 1 },
    OptSpec { name: "wf-tcp", kind: 1 },
    OptSpec { name: "wf-udp", kind: 1 },
    OptSpec { name: "wf-raw", kind: 1 },
    OptSpec { name: "wf-raw-part", kind: 1 },
    OptSpec { name: "wf-filter-lan", kind: 1 },
    OptSpec { name: "wf-save", kind: 1 },
    OptSpec { name: "ssid-filter", kind: 1 },
    OptSpec { name: "nlm-filter", kind: 1 },
];

fn parse_l3(s: &str) -> Option<(bool, bool)> {
    let (mut v4, mut v6) = (false, false);
    for p in s.split(',') {
        match p {
            "ipv4" => v4 = true,
            "ipv6" => v6 = true,
            _ => return None,
        }
    }
    Some((v4, v6))
}

fn blob_from(data: Vec<u8>, offset: usize, reserve: usize) -> Blob {
    let size_buf = data.len() + reserve;
    Blob { data, offset, size_buf, tls_mod: None, tls_cache: None }
}

pub fn parse(args: &[String]) -> Result<Params, String> {
    let mut params = Params {
        profiles: Vec::new(),
        lists: Lists::default(),
        qnum: -1,
        desync_fwmark: FWMARK_DEFAULT,
        ctrack_t_syn: 60,
        ctrack_t_est: 300,
        ctrack_t_fin: 60,
        ctrack_t_udp: 60,
        ctrack_disable: false,
        ipcache_lifetime: 7200,
        cache_hostname: false,
        autottl_present: false,
        debug: false,
        wf: WinFilterOpts { iface: (0, 0), ipv4: true, ipv6: true, tcp: String::new(), udp: String::new(), raw: None, raw_part: Vec::new(), filter_lan: true },
    };
    let mut profile_count = 1usize;
    let mut dp = Profile::new(profile_count);
    let mut skip = false;
    let (mut anon_hl, mut anon_hl_ex, mut anon_ips, mut anon_ips_ex): (Option<usize>, Option<usize>, Option<usize>, Option<usize>) = (None, None, None, None);

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        let body = a.strip_prefix("--").or_else(|| a.strip_prefix('-')).ok_or_else(|| format!("unexpected argument: {a}"))?;
        let (name, inline) = match body.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (body, None),
        };
        let spec = OPTS.iter().find(|o| o.name == name).ok_or_else(|| format!("unknown option: --{name}"))?;
        let val: Option<String> = match spec.kind {
            1 => match inline {
                Some(v) => Some(v),
                None => {
                    let v = args.get(i).ok_or_else(|| format!("option --{name} requires an argument"))?.clone();
                    i += 1;
                    Some(v)
                }
            },
            2 => inline,
            _ => None,
        };
        let v = val.as_deref().unwrap_or("");
        let err = |m: &str| -> String { format!("--{name}: {m}") };

        match name {
            "debug" => params.debug = val.as_deref().map(|s| s != "0").unwrap_or(true),
            "dry-run" | "version" | "comment" | "daemon" | "pidfile" | "user" | "uid" | "bind-fix4" | "bind-fix6" | "wf-save" | "ssid-filter" | "nlm-filter" => {}
            "qnum" => {
                params.qnum = atoi(v) as i32;
                if !(0..=65535).contains(&params.qnum) {
                    return Err(err("bad qnum"));
                }
            }
            "wsize" | "wssize" => {
                let (a, b) = v.split_once(':').unwrap_or((v, ""));
                let ws = atoi(a);
                if !(0..=65535).contains(&ws) {
                    return Err(err("bad wsize"));
                }
                let sc = if b.is_empty() {
                    None
                } else {
                    let s = atoi(b);
                    if !(0..=255).contains(&s) {
                        return Err(err("bad wscale"));
                    }
                    Some(s as u8)
                };
                if name == "wsize" {
                    dp.wsize = ws as u16;
                    if let Some(s) = sc {
                        dp.wscale = s;
                    }
                } else {
                    dp.wssize = ws as u16;
                    if let Some(s) = sc {
                        dp.wsscale = s;
                    }
                }
            }
            "wssize-cutoff" => {
                let (c, m) = parse_cutoff(v).ok_or_else(|| err("invalid value"))?;
                dp.wssize_cutoff = c;
                dp.wssize_cutoff_mode = m;
            }
            "wssize-forced-cutoff" => dp.wssize_no_forced_cutoff = atoi(v) == 0,
            "synack-split" => {
                dp.synack_split = match val.as_deref() {
                    None | Some("syn") => SynackSplit::Syn,
                    Some("synack") => SynackSplit::SynAck,
                    Some("acksyn") => SynackSplit::AckSyn,
                    _ => return Err(err("invalid synack-split value")),
                }
            }
            "ctrack-timeouts" => {
                let p: Vec<u64> = v.split(':').filter_map(|x| x.parse().ok()).collect();
                if p.len() < 3 {
                    return Err(err("invalid value"));
                }
                params.ctrack_t_syn = p[0];
                params.ctrack_t_est = p[1];
                params.ctrack_t_fin = p[2];
                if let Some(&u) = p.get(3) {
                    params.ctrack_t_udp = u;
                }
            }
            "ctrack-disable" => params.ctrack_disable = val.is_none() || atoi(v) != 0,
            "ipcache-lifetime" => params.ipcache_lifetime = v.parse().map_err(|_| err("invalid value"))?,
            "ipcache-hostname" => params.cache_hostname = val.is_none() || atoi(v) != 0,
            "hostcase" => dp.hostcase = true,
            "hostspell" => {
                if v.len() != 4 {
                    return Err(err("hostspell must be exactly 4 chars long"));
                }
                dp.hostcase = true;
                dp.hostspell.copy_from_slice(v.as_bytes());
            }
            "hostnospace" => {
                if dp.methodeol {
                    return Err(err("incompatible with --methodeol"));
                }
                dp.hostnospace = true;
            }
            "domcase" => dp.domcase = true,
            "methodeol" => {
                if dp.hostnospace {
                    return Err(err("incompatible with --hostnospace"));
                }
                dp.methodeol = true;
            }
            "ip-id" => {
                dp.ip_id_mode = match v {
                    "zero" => IpIdMode::Zero,
                    "seq" => IpIdMode::Seq,
                    "seqgroup" => IpIdMode::SeqGroup,
                    "rnd" => IpIdMode::Rnd,
                    _ => return Err(err("invalid ip_id mode")),
                }
            }
            "dpi-desync" => {
                let mut parts = v.split(',');
                let (p1, p2, p3) = (parts.next(), parts.next(), parts.next());
                let mode0 = parse_desync_mode(p1)?.ok_or_else(|| err("invalid dpi-desync mode"))?;
                let (m1, m2, m3) = if valid_zero_stage(mode0) {
                    dp.desync_mode0 = mode0;
                    (p2, p3, parts.next())
                } else {
                    dp.desync_mode0 = Mode::None;
                    (p1, p2, p3)
                };
                let mode = parse_desync_mode(m1)?.ok_or_else(|| err("invalid dpi-desync mode"))?;
                let mode2 = parse_desync_mode(m2)?.ok_or_else(|| err("invalid dpi-desync mode"))?;
                if m3.is_some() {
                    return Err(err("invalid desync combo"));
                }
                if mode2 != Mode::None && !(valid_first_stage(mode) && valid_second_stage(mode2)) {
                    return Err(err("invalid desync combo"));
                }
                dp.desync_mode = mode;
                dp.desync_mode2 = mode2;
            }
            "dpi-desync-fwmark" => {
                let n = if let Some(h) = v.strip_prefix("0x") { u32::from_str_radix(h, 16).ok() } else { v.parse().ok() };
                params.desync_fwmark = n.filter(|&n| n != 0).ok_or_else(|| err("fwmark should be decimal or 0xHEX and non zero"))?;
            }
            "dup" => {
                dp.dup_repeats = v.parse().ok().filter(|&n| n <= 1024).ok_or_else(|| err("must be within 0..1024"))?;
            }
            "dup-ttl" => dp.dup_ttl = atoi(v) as u8,
            "dup-ttl6" => dp.dup_ttl6 = atoi(v) as u8,
            "dup-autottl" => {
                dp.dup_autottl = parse_autottl(val.as_deref(), -1, 3, 64).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "dup-autottl6" => {
                dp.dup_autottl6 = parse_autottl(val.as_deref(), -1, 3, 64).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "dup-tcp-flags-set" => dp.dup_tcp_flags_set = parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?,
            "dup-tcp-flags-unset" => dp.dup_tcp_flags_unset = parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?,
            "dup-replace" => dp.dup_replace = val.is_none() || atoi(v) != 0,
            "dup-fooling" => dp.dup_fooling_mode = parse_fooling(v).ok_or_else(|| err("invalid fooling"))?,
            "dup-ts-increment" => dp.dup_ts_increment = parse_net32_signed(v).ok_or_else(|| err("invalid value"))?,
            "dup-badseq-increment" => dp.dup_badseq_increment = parse_net32_signed(v).ok_or_else(|| err("invalid value"))?,
            "dup-badack-increment" => dp.dup_badseq_ack_increment = parse_net32_signed(v).ok_or_else(|| err("invalid value"))?,
            "dup-cutoff" => {
                let (c, m) = parse_cutoff(v).ok_or_else(|| err("invalid value"))?;
                dp.dup_cutoff = c;
                dp.dup_cutoff_mode = m;
            }
            "dup-start" => {
                let (c, m) = parse_cutoff(v).ok_or_else(|| err("invalid value"))?;
                dp.dup_start = c;
                dp.dup_start_mode = m;
            }
            "dup-ip-id" => {
                dp.dup_ip_id_mode = match v {
                    "zero" => IpIdMode::Zero,
                    "same" => IpIdMode::Same,
                    "seq" => IpIdMode::Seq,
                    "rnd" => IpIdMode::Rnd,
                    _ => return Err(err("invalid ip_id mode")),
                }
            }
            "orig-ttl" => dp.orig_mod_ttl = atoi(v) as u8,
            "orig-ttl6" => dp.orig_mod_ttl6 = atoi(v) as u8,
            "orig-autottl" => {
                dp.orig_autottl = parse_autottl(val.as_deref(), 5, 3, 64).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "orig-autottl6" => {
                dp.orig_autottl6 = parse_autottl(val.as_deref(), 5, 3, 64).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "orig-tcp-flags-set" => dp.orig_tcp_flags_set = parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?,
            "orig-tcp-flags-unset" => dp.orig_tcp_flags_unset = parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?,
            "orig-mod-cutoff" => {
                let (c, m) = parse_cutoff(v).ok_or_else(|| err("invalid value"))?;
                dp.orig_mod_cutoff = c;
                dp.orig_mod_cutoff_mode = m;
            }
            "orig-mod-start" => {
                let (c, m) = parse_cutoff(v).ok_or_else(|| err("invalid value"))?;
                dp.orig_mod_start = c;
                dp.orig_mod_start_mode = m;
            }
            "dpi-desync-ttl" => dp.desync_ttl = atoi(v) as u8,
            "dpi-desync-ttl6" => dp.desync_ttl6 = atoi(v) as u8,
            "dpi-desync-autottl" => {
                dp.desync_autottl = parse_autottl(val.as_deref(), -1, 3, 20).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "dpi-desync-autottl6" => {
                dp.desync_autottl6 = parse_autottl(val.as_deref(), -1, 3, 20).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "dpi-desync-tcp-flags-set" => dp.desync_tcp_flags_set = parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?,
            "dpi-desync-tcp-flags-unset" => dp.desync_tcp_flags_unset = parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?,
            "dpi-desync-fooling" => dp.desync_fooling_mode |= parse_fooling(v).ok_or_else(|| err("invalid fooling"))?,
            "dpi-desync-repeats" => {
                dp.desync_repeats = v.parse().ok().filter(|&n| (1..=1024).contains(&n)).ok_or_else(|| err("must be within 1..1024"))?;
            }
            "dpi-desync-skip-nosni" => dp.desync_skip_nosni = val.is_none() || atoi(v) != 0,
            "dpi-desync-split-pos" => {
                for item in v.split(',') {
                    if dp.splits.len() >= MAX_SPLITS {
                        return Err(err("too much split positions"));
                    }
                    dp.splits.push(parse_split_pos(item).ok_or_else(|| err(&format!("could not parse split pos '{item}'")))?);
                }
            }
            "dpi-desync-split-http-req" => {
                dp.splits.push(match v {
                    "method" => ProtoPos { marker: Marker::HttpMethod, pos: 2 },
                    "host" => ProtoPos { marker: Marker::Host, pos: 1 },
                    _ => return Err(err("invalid value")),
                });
            }
            "dpi-desync-split-tls" => {
                dp.splits.push(match v {
                    "sni" => ProtoPos { marker: Marker::Host, pos: 1 },
                    "sniext" => ProtoPos { marker: Marker::SniExt, pos: 1 },
                    "snisld" => ProtoPos { marker: Marker::HostMidSld, pos: 0 },
                    _ => return Err(err("invalid value")),
                });
            }
            "dpi-desync-split-seqovl" => {
                dp.seqovl = if v == "0" { ProtoPos::EMPTY } else { parse_split_pos(v).ok_or_else(|| err("invalid value"))? };
            }
            "dpi-desync-split-seqovl-pattern" => {
                let (data, ofs) = load_file_or_hex(v, FAKE_MAX_TCP).map_err(|e| err(&e))?;
                let mut buf = vec![0u8; FAKE_MAX_TCP];
                fill_pattern(&mut buf, &data[ofs..], 0);
                dp.seqovl_pattern = buf;
            }
            "dpi-desync-fakedsplit-pattern" => {
                let (data, ofs) = load_file_or_hex(v, 32768).map_err(|e| err(&e))?;
                dp.fsplit_pattern = data[ofs..].to_vec();
                dp.fsplit_pattern_set = true;
            }
            "dpi-desync-fakedsplit-mod" => {
                for item in v.split(',') {
                    let (n, val) = item.split_once('=').map(|(a, b)| (a, Some(b))).unwrap_or((item, None));
                    match n {
                        "altorder" => {
                            let o = atoi(val.filter(|s| !s.is_empty()).ok_or_else(|| err("invalid mod"))?) as i32;
                            if (o as u32) & 0xFFFF_FFE4 != 0 || ((o >> 3) & 3) > 2 {
                                return Err(err("invalid mod"));
                            }
                            dp.fs_ordering = o;
                        }
                        "none" => {}
                        _ => return Err(err("invalid mod")),
                    }
                }
            }
            "dpi-desync-hostfakesplit-midhost" => {
                dp.hostfakesplit_midhost = if v == "0" { ProtoPos::EMPTY } else { parse_split_pos(v).ok_or_else(|| err("invalid value"))? };
            }
            "dpi-desync-hostfakesplit-mod" => {
                for item in v.split(',') {
                    let (n, val) = item.split_once('=').map(|(a, b)| (a, Some(b))).unwrap_or((item, None));
                    match n {
                        "host" => {
                            let h = val.filter(|s| !s.is_empty()).ok_or_else(|| err("invalid mod"))?;
                            dp.hfs_host = h.chars().take(126).collect();
                        }
                        "altorder" => {
                            let o = atoi(val.filter(|s| !s.is_empty()).ok_or_else(|| err("invalid mod"))?) as i32;
                            if !(0..=1).contains(&o) {
                                return Err(err("invalid mod"));
                            }
                            dp.hfs_ordering = o;
                        }
                        "none" => {}
                        _ => return Err(err("invalid mod")),
                    }
                }
            }
            "dpi-desync-ipfrag-pos-tcp" | "dpi-desync-ipfrag-pos-udp" => {}
            "dpi-desync-ts-increment" => dp.desync_ts_increment = parse_net32_signed(v).ok_or_else(|| err("invalid value"))?,
            "dpi-desync-badseq-increment" => dp.desync_badseq_increment = parse_net32_signed(v).ok_or_else(|| err("invalid value"))?,
            "dpi-desync-badack-increment" => dp.desync_badseq_ack_increment = parse_net32_signed(v).ok_or_else(|| err("invalid value"))?,
            "dpi-desync-any-protocol" => dp.desync_any_proto = val.is_none() || atoi(v) != 0,
            "dpi-desync-fake-tcp-mod" => {
                for item in v.split(',') {
                    match item {
                        "seq" => dp.tcp_mod_seq = true,
                        "none" => {}
                        _ => return Err(err("invalid tcp mod")),
                    }
                }
            }
            "dpi-desync-fake-http" => {
                let (d, o) = load_file_or_hex(v, FAKE_MAX_TCP).map_err(|e| err(&e))?;
                dp.fake_http.push(blob_from(d, o, 0));
            }
            "dpi-desync-fake-tls" => {
                let mut blob = if v == "!" || v.starts_with("!+") {
                    let ofs = if v.len() > 2 { atoi(&v[1..]) as usize } else { 0 };
                    if ofs >= FAKE_TLS_CLIENTHELLO_DEFAULT.len() {
                        return Err(err("offset out of data range"));
                    }
                    blob_from(FAKE_TLS_CLIENTHELLO_DEFAULT.to_vec(), ofs, 4 + 128)
                } else {
                    let (d, o) = load_file_or_hex(v, FAKE_MAX_TCP).map_err(|e| err(&e))?;
                    blob_from(d, o, 4 + 128)
                };
                let mut tm = dp.tls_mod_last.clone();
                tm.mode |= TLS_MOD_CUSTOM_FAKE;
                blob.tls_mod = Some(tm);
                dp.fake_tls.push(blob);
                dp.tls_fake_last = Some(dp.fake_tls.len() - 1);
            }
            "dpi-desync-fake-tls-mod" => {
                if !parse_tlsmod_list(v, &mut dp.tls_mod_last) {
                    return Err(err("invalid tls mod"));
                }
                if let Some(i) = dp.tls_fake_last {
                    dp.fake_tls[i].tls_mod = Some(dp.tls_mod_last.clone());
                }
            }
            "dpi-desync-fake-unknown" => {
                let (d, o) = load_file_or_hex(v, FAKE_MAX_TCP).map_err(|e| err(&e))?;
                dp.fake_unknown.push(blob_from(d, o, 0));
            }
            "dpi-desync-fake-syndata" => {
                let (d, o) = load_file_or_hex(v, FAKE_MAX_TCP).map_err(|e| err(&e))?;
                dp.fake_syndata = d[o..].to_vec();
            }
            "dpi-desync-fake-quic" | "dpi-desync-fake-wireguard" | "dpi-desync-fake-dht" | "dpi-desync-fake-discord" | "dpi-desync-fake-stun" | "dpi-desync-fake-unknown-udp" => {
                let (d, o) = load_file_or_hex(v, FAKE_MAX_UDP).map_err(|e| err(&e))?;
                let b = blob_from(d, o, 0);
                match name {
                    "dpi-desync-fake-quic" => dp.fake_quic.push(b),
                    "dpi-desync-fake-wireguard" => dp.fake_wg.push(b),
                    "dpi-desync-fake-dht" => dp.fake_dht.push(b),
                    "dpi-desync-fake-discord" => dp.fake_discord.push(b),
                    "dpi-desync-fake-stun" => dp.fake_stun.push(b),
                    _ => dp.fake_unknown_udp.push(b),
                }
            }
            "dpi-desync-udplen-increment" => {
                dp.udplen_increment = v.parse().ok().filter(|n: &i32| (-0x8000..=0x7FFF).contains(n)).ok_or_else(|| err("must be within -32768..32767"))?;
            }
            "dpi-desync-udplen-pattern" => {
                let (data, ofs) = load_file_or_hex(v, FAKE_MAX_UDP).map_err(|e| err(&e))?;
                let mut buf = vec![0u8; FAKE_MAX_UDP];
                fill_pattern(&mut buf, &data[ofs..], 0);
                dp.udplen_pattern = buf;
            }
            "dpi-desync-cutoff" => {
                let (c, m) = parse_cutoff(v).ok_or_else(|| err("invalid value"))?;
                dp.desync_cutoff = c;
                dp.desync_cutoff_mode = m;
            }
            "dpi-desync-start" => {
                let (c, m) = parse_cutoff(v).ok_or_else(|| err("invalid value"))?;
                dp.desync_start = c;
                dp.desync_start_mode = m;
            }
            "hostlist" => {
                if !skip {
                    let id = params.lists.register_hostlist(Some(v));
                    if !dp.hl_include.contains(&id) {
                        dp.hl_include.push(id);
                    }
                }
            }
            "hostlist-domains" => {
                if !skip {
                    let id = *anon_hl.get_or_insert_with(|| {
                        let id = params.lists.register_hostlist(None);
                        dp.hl_include.push(id);
                        id
                    });
                    for d in v.split(',').filter(|d| !d.is_empty()) {
                        params.lists.hosts[id].add_item(d);
                    }
                }
            }
            "hostlist-exclude" => {
                if !skip {
                    let id = params.lists.register_hostlist(Some(v));
                    if !dp.hl_exclude.contains(&id) {
                        dp.hl_exclude.push(id);
                    }
                }
            }
            "hostlist-exclude-domains" => {
                if !skip {
                    let id = *anon_hl_ex.get_or_insert_with(|| {
                        let id = params.lists.register_hostlist(None);
                        dp.hl_exclude.push(id);
                        id
                    });
                    for d in v.split(',').filter(|d| !d.is_empty()) {
                        params.lists.hosts[id].add_item(d);
                    }
                }
            }
            "hostlist-auto" | "hostlist-auto-fail-threshold" | "hostlist-auto-fail-time" | "hostlist-auto-retrans-threshold" | "hostlist-auto-debug" => {
                return Err(err("auto hostlist is not supported by this engine"));
            }
            "new" => {
                if skip {
                    dp = Profile::new(profile_count);
                    skip = false;
                } else {
                    params.profiles.push(dp);
                    profile_count += 1;
                    dp = Profile::new(profile_count);
                }
                anon_hl = None;
                anon_hl_ex = None;
                anon_ips = None;
                anon_ips_ex = None;
            }
            "skip" => skip = true,
            "filter-l3" => {
                let (a, b) = parse_l3(v).ok_or_else(|| err("bad value"))?;
                dp.filter_ipv4 = a;
                dp.filter_ipv6 = b;
            }
            "filter-tcp" | "filter-udp" => {
                let mut list = Vec::new();
                for item in v.split(',') {
                    list.push(pf_parse(item).ok_or_else(|| err("invalid port filter"))?);
                }
                if name == "filter-tcp" {
                    dp.pf_tcp.extend(list);
                    if dp.pf_udp.is_empty() {
                        dp.pf_udp.push(pf_parse("0").unwrap());
                    }
                } else {
                    dp.pf_udp.extend(list);
                    if dp.pf_tcp.is_empty() {
                        dp.pf_tcp.push(pf_parse("0").unwrap());
                    }
                }
            }
            "filter-l7" => dp.filter_l7 = parse_l7_list(v).ok_or_else(|| err("invalid l7 filter"))?,
            "ipset" => {
                if !skip {
                    if std::fs::metadata(v).is_err() {
                        return Err(err(&format!("cannot access ipset file '{v}'")));
                    }
                    let id = params.lists.register_ipset(Some(v));
                    if !dp.ips_include.contains(&id) {
                        dp.ips_include.push(id);
                    }
                }
            }
            "ipset-ip" => {
                if !skip {
                    let id = *anon_ips.get_or_insert_with(|| {
                        let id = params.lists.register_ipset(None);
                        dp.ips_include.push(id);
                        id
                    });
                    for d in v.split(',').filter(|d| !d.is_empty()) {
                        params.lists.ipsets[id].add_item(d);
                    }
                }
            }
            "ipset-exclude" => {
                if !skip {
                    if std::fs::metadata(v).is_err() {
                        return Err(err(&format!("cannot access ipset file '{v}'")));
                    }
                    let id = params.lists.register_ipset(Some(v));
                    if !dp.ips_exclude.contains(&id) {
                        dp.ips_exclude.push(id);
                    }
                }
            }
            "ipset-exclude-ip" => {
                if !skip {
                    let id = *anon_ips_ex.get_or_insert_with(|| {
                        let id = params.lists.register_ipset(None);
                        dp.ips_exclude.push(id);
                        id
                    });
                    for d in v.split(',').filter(|d| !d.is_empty()) {
                        params.lists.ipsets[id].add_item(d);
                    }
                }
            }
            "wf-iface" => {
                let (a, b) = v.split_once('.').unwrap_or((v, "0"));
                params.wf.iface = (a.parse().map_err(|_| err("bad value"))?, b.parse().unwrap_or(0));
            }
            "wf-l3" => {
                let (a, b) = parse_l3(v).ok_or_else(|| err("bad value"))?;
                params.wf.ipv4 = a;
                params.wf.ipv6 = b;
            }
            "wf-tcp" => params.wf.tcp = v.to_string(),
            "wf-udp" => params.wf.udp = v.to_string(),
            "wf-raw" => params.wf.raw = Some(v.to_string()),
            "wf-raw-part" => params.wf.raw_part.push(v.to_string()),
            "wf-filter-lan" => params.wf.filter_lan = atoi(v) != 0,
            _ => return Err(format!("option --{name} is not handled")),
        }
    }
    if !skip {
        params.profiles.push(dp);
    }

    // low-priority default empty profile
    params.profiles.push(Profile::new(0));

    for p in &mut params.profiles {
        finalize_profile(p)?;
    }
    params.lists.load_all()?;
    Ok(params)
}

fn finalize_profile(dp: &mut Profile) -> Result<(), String> {
    if dp.desync_ttl6 == 0xFF {
        dp.desync_ttl6 = dp.desync_ttl;
    }
    if dp.dup_ttl6 == 0xFF {
        dp.dup_ttl6 = dp.dup_ttl;
    }
    if dp.orig_mod_ttl6 == 0xFF {
        dp.orig_mod_ttl6 = dp.orig_mod_ttl;
    }
    if !dp.fsplit_pattern_set {
        dp.fsplit_pattern = vec![0; 64];
    }
    if !dp.desync_autottl6.enabled() {
        dp.desync_autottl6 = dp.desync_autottl;
    }
    if !dp.orig_autottl6.enabled() {
        dp.orig_autottl6 = dp.orig_autottl;
    }
    if !dp.dup_autottl6.enabled() {
        dp.dup_autottl6 = dp.dup_autottl;
    }

    // split_compat
    if dp.splits.is_empty() {
        dp.splits.push(ProtoPos { marker: Marker::Abs, pos: 2 });
    }
    let uses_split_ovl = |m: Mode| matches!(m, Mode::FakedSplit | Mode::MultiSplit);
    if (dp.seqovl.marker != Marker::Abs || dp.seqovl.pos < 0) && (uses_split_ovl(dp.desync_mode) || uses_split_ovl(dp.desync_mode2)) {
        return Err("split seqovl supports only absolute positive positions".into());
    }

    // fake defaults
    if dp.fake_http.is_empty() {
        dp.fake_http.push(blob_from(FAKE_HTTP_REQUEST_DEFAULT.as_bytes().to_vec(), 0, 0));
    }
    if dp.fake_tls.is_empty() {
        let mut b = blob_from(FAKE_TLS_CLIENTHELLO_DEFAULT.to_vec(), 0, 4 + 128);
        b.tls_mod = Some(dp.tls_mod_last.clone());
        dp.fake_tls.push(b);
    }
    if dp.fake_unknown.is_empty() {
        dp.fake_unknown.push(blob_from(vec![0; 256], 0, 0));
    }
    if dp.fake_quic.is_empty() {
        let mut d = vec![0u8; 620];
        d[0] = 0x40;
        dp.fake_quic.push(blob_from(d, 0, 0));
    }
    for list in [&mut dp.fake_wg, &mut dp.fake_dht, &mut dp.fake_discord, &mut dp.fake_stun, &mut dp.fake_unknown_udp] {
        if list.is_empty() {
            list.push(blob_from(vec![0; 64], 0, 0));
        }
    }

    onetime_tls_mod(dp)
}

fn onetime_tls_mod(dp: &mut Profile) -> Result<(), String> {
    let n_profile = dp.n;
    for (idx, blob) in dp.fake_tls.iter_mut().enumerate() {
        let n = idx + 1;
        let Some(tm) = blob.tls_mod.as_mut() else { continue };
        if n_profile != 0 && tm.mode & (TLS_MOD_SET | TLS_MOD_CUSTOM_FAKE) == 0 {
            tm.mode |= TLS_MOD_RND | TLS_MOD_RND_SNI | TLS_MOD_DUP_SID;
        }
        if tm.mode & !TLS_MOD_SAVE_MASK == 0 {
            continue;
        }
        if !is_tls_client_hello(&blob.data, false) || blob.data.len() < 44 + blob.data[43] as usize {
            return Err(format!("profile {n_profile} fake[{n}] tls mod set but tls fake structure invalid"));
        }
        let mut cache = TlsModCache::default();
        let tm = tm.clone();
        onetime_tls_mod_blob(n_profile, n, &tm, blob, &mut cache)?;
        blob.tls_cache = Some(cache);
        if blob.offset >= blob.data.len() {
            return Err(format!("profile {n_profile} fake[{n}] tls mod shrinked data and offset is out of range"));
        }
    }
    Ok(())
}

fn put16v(v: &mut [u8], o: usize, x: u16) {
    v[o..o + 2].copy_from_slice(&x.to_be_bytes());
}
fn get16v(v: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([v[o], v[o + 1]])
}
fn put24v(v: &mut [u8], o: usize, x: u32) {
    v[o] = (x >> 16) as u8;
    v[o + 1] = (x >> 8) as u8;
    v[o + 2] = x as u8;
}
fn get24v(v: &[u8], o: usize) -> u32 {
    ((v[o] as u32) << 16) | ((v[o + 1] as u32) << 8) | v[o + 2] as u32
}

fn onetime_tls_mod_blob(pn: usize, fake_n: usize, tm: &FakeTlsMod, blob: &mut Blob, cache: &mut TlsModCache) -> Result<(), String> {
    cache.extlen_offset = 0;
    cache.padlen_offset = 0;
    if tm.mode & (TLS_MOD_RND_SNI | TLS_MOD_SNI | TLS_MOD_PADENCAP) == 0 {
        return Ok(());
    }
    let d = &mut blob.data;
    cache.extlen_offset = tls_find_ext_len(d).ok_or_else(|| format!("profile {pn} fake[{fake_n}] padencap set but tls fake structure invalid"))?;

    if tm.mode & (TLS_MOD_RND_SNI | TLS_MOD_SNI) != 0 {
        let (sniext, extlen) = tls_find_ext(d, 0, false).ok_or_else(|| format!("profile {pn} fake[{fake_n}] sni mod is set but tls fake does not have SNI"))?;
        let (sni_off, mut slen) = tls_advance_to_host_in_sni(d, sniext, extlen).ok_or_else(|| format!("profile {pn} fake[{fake_n}] sni set but tls fake has invalid SNI structure"))?;
        if tm.mode & TLS_MOD_SNI != 0 {
            let new = tm.sni.as_bytes();
            let delta = new.len() as i64 - slen as i64;
            if delta != 0 {
                if (d.len() as i64 + delta) as usize > blob.size_buf {
                    return Err(format!("profile {pn} fake[{fake_n}] not enough space for new SNI"));
                }
                let tail: Vec<u8> = d[sni_off + slen..].to_vec();
                d.truncate(sni_off);
                d.extend_from_slice(&vec![0; new.len()]);
                d.extend_from_slice(&tail);
                let adj = |v: u16| (v as i64 + delta) as u16;
                let w = get16v(d, 3);
                put16v(d, 3, adj(w));
                let hs = get24v(d, 6);
                put24v(d, 6, (hs as i64 + delta) as u32);
                let eo = cache.extlen_offset;
                let w = get16v(d, eo);
                put16v(d, eo, adj(w));
                let w = get16v(d, sniext - 2);
                put16v(d, sniext - 2, adj(w));
                let w = get16v(d, sniext);
                put16v(d, sniext, adj(w));
                let w = get16v(d, sni_off - 2);
                put16v(d, sni_off - 2, adj(w));
                slen = new.len();
            }
            d[sni_off..sni_off + new.len()].copy_from_slice(new);
        }
        if tm.mode & TLS_MOD_RND_SNI != 0 {
            if slen == 0 {
                return Err(format!("profile {pn} fake[{fake_n}] rndsni set but tls fake has zero sized SNI"));
            }
            rng::fill_az(&mut d[sni_off..sni_off + 1]);
            if slen >= 7 {
                rng::fill_az09(&mut d[sni_off + 1..sni_off + 1 + slen - 5]);
                d[sni_off + slen - 4] = b'.';
                let tld = TLD[rng::next_u32() as usize % TLD.len()];
                d[sni_off + slen - 3..sni_off + slen].copy_from_slice(tld.as_bytes());
            } else {
                rng::fill_az09(&mut d[sni_off + 1..sni_off + slen]);
            }
        }
    }
    if tm.mode & TLS_MOD_PADENCAP != 0 {
        if let Some((po, pl)) = tls_find_ext(d, 21, false) {
            if po + pl != d.len() {
                return Err(format!("profile {pn} fake[{fake_n}] tls padding ext is present but it's not at the end"));
            }
            cache.padlen_offset = po - 2;
        } else {
            if d.len() + 4 > blob.size_buf {
                return Err(format!("profile {pn} fake[{fake_n}] tls padding is absent and there's no space to add it"));
            }
            d.extend_from_slice(&21u16.to_be_bytes());
            cache.padlen_offset = d.len();
            d.extend_from_slice(&0u16.to_be_bytes());
            let eo = cache.extlen_offset;
            let w = get16v(d, eo);
            put16v(d, eo, w + 4);
            let w = get16v(d, 3);
            put16v(d, 3, w + 4);
            let hs = get24v(d, 6);
            put24v(d, 6, hs + 4);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_basic_profiles() {
        let p = parse(&s(&[
            "--wf-tcp=80,443",
            "--wf-udp=443",
            "--filter-udp=443",
            "--hostlist-domains=example.com",
            "--dpi-desync=fake",
            "--dpi-desync-repeats=6",
            "--new",
            "--filter-tcp=80,443",
            "--dpi-desync=fake,multisplit",
            "--dpi-desync-split-pos=1,midsld",
            "--dpi-desync-split-seqovl=681",
            "--dpi-desync-fooling=ts,badseq",
            "--dpi-desync-fake-tls-mod=rnd,dupsid,sni=www.google.com",
        ]))
        .unwrap();
        assert_eq!(p.profiles.len(), 3);
        assert_eq!(p.profiles[0].desync_mode, Mode::Fake);
        assert_eq!(p.profiles[0].desync_repeats, 6);
        assert_eq!(p.profiles[1].desync_mode2, Mode::MultiSplit);
        assert_eq!(p.profiles[1].splits.len(), 2);
        assert_eq!(p.profiles[1].desync_fooling_mode, FOOL_TS | FOOL_BADSEQ);
        assert_eq!(p.profiles[0].pf_tcp.len(), 1);
        assert!(!port_filters_in_range(&p.profiles[0].pf_tcp, 443));
        assert!(port_filters_in_range(&p.profiles[0].pf_udp, 443));
        let tls = &p.profiles[1].fake_tls[0];
        assert!(tls.tls_mod.as_ref().unwrap().mode & TLS_MOD_SNI != 0);
        assert!(crate::engine::proto::tls_hello_extract_host(&tls.data, false).as_deref() == Some("www.google.com"));
        assert_eq!(p.profiles[2].n, 0);
    }

    #[test]
    fn tls_sni_resize_keeps_structure_valid() {
        let p = parse(&s(&["--dpi-desync=fake", "--dpi-desync-fake-tls=!", "--dpi-desync-fake-tls-mod=sni=ya.ru,padencap"])).unwrap();
        let d = &p.profiles[0].fake_tls[0].data;
        assert!(is_tls_client_hello(d, false));
        assert_eq!(tls_hello_extract_host(d, false).as_deref(), Some("ya.ru"));
        let rec = tls_record_len(d);
        assert_eq!(rec, d.len());
        assert!(tls_find_ext(d, 21, false).is_some());
    }

    #[test]
    fn rejects_unsupported() {
        assert!(parse(&s(&["--dpi-desync=ipfrag2"])).is_err());
        assert!(parse(&s(&["--bogus"])).is_err());
    }
}
