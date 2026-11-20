use super::*;

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
        port != 0
            && (((self.from == 0 && self.to == 0) || (port >= self.from && port <= self.to))
                ^ self.neg)
    }
}

pub fn pf_parse(s: &str) -> Option<PortFilter> {
    if s == "*" {
        return Some(PortFilter {
            from: 1,
            to: 0xFFFF,
            neg: false,
        });
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
        self.orig_mod_ttl != 0
            || self.orig_mod_ttl6 != 0
            || self.orig_tcp_flags_set != 0
            || self.orig_tcp_flags_unset != 0
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
        self.profiles
            .iter()
            .any(|p| p.dup_repeats != 0 || p.has_orig_mod())
    }
}
