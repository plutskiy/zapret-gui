use super::fakes::{FAKE_HTTP_REQUEST_DEFAULT, FAKE_TLS_CLIENTHELLO_DEFAULT};
use super::lists::Lists;
use super::packet::*;
use super::proto::*;
use super::rng;

mod table;
mod tls_mod;
mod types;
mod values;

use table::*;
use tls_mod::*;
pub use types::*;
use values::*;

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
    Blob {
        data,
        offset,
        size_buf,
        tls_mod: None,
        tls_cache: None,
    }
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
        wf: WinFilterOpts {
            iface: (0, 0),
            ipv4: true,
            ipv6: true,
            tcp: String::new(),
            udp: String::new(),
            raw: None,
            raw_part: Vec::new(),
            filter_lan: true,
        },
    };
    let mut profile_count = 1usize;
    let mut dp = Profile::new(profile_count);
    let mut skip = false;
    let (mut anon_hl, mut anon_hl_ex, mut anon_ips, mut anon_ips_ex): (
        Option<usize>,
        Option<usize>,
        Option<usize>,
        Option<usize>,
    ) = (None, None, None, None);

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        let body = a
            .strip_prefix("--")
            .or_else(|| a.strip_prefix('-'))
            .ok_or_else(|| format!("unexpected argument: {a}"))?;
        let (name, inline) = match body.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (body, None),
        };
        let spec = OPTS
            .iter()
            .find(|o| o.name == name)
            .ok_or_else(|| format!("unknown option: --{name}"))?;
        let val: Option<String> = match spec.kind {
            1 => match inline {
                Some(v) => Some(v),
                None => {
                    let v = args
                        .get(i)
                        .ok_or_else(|| format!("option --{name} requires an argument"))?
                        .clone();
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
            "dry-run" | "version" | "comment" | "daemon" | "pidfile" | "user" | "uid"
            | "bind-fix4" | "bind-fix6" | "wf-save" | "ssid-filter" | "nlm-filter" => {}
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
            "ipcache-lifetime" => {
                params.ipcache_lifetime = v.parse().map_err(|_| err("invalid value"))?
            }
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
                let n = if let Some(h) = v.strip_prefix("0x") {
                    u32::from_str_radix(h, 16).ok()
                } else {
                    v.parse().ok()
                };
                params.desync_fwmark = n
                    .filter(|&n| n != 0)
                    .ok_or_else(|| err("fwmark should be decimal or 0xHEX and non zero"))?;
            }
            "dup" => {
                dp.dup_repeats = v
                    .parse()
                    .ok()
                    .filter(|&n| n <= 1024)
                    .ok_or_else(|| err("must be within 0..1024"))?;
            }
            "dup-ttl" => dp.dup_ttl = atoi(v) as u8,
            "dup-ttl6" => dp.dup_ttl6 = atoi(v) as u8,
            "dup-autottl" => {
                dp.dup_autottl =
                    parse_autottl(val.as_deref(), -1, 3, 64).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "dup-autottl6" => {
                dp.dup_autottl6 =
                    parse_autottl(val.as_deref(), -1, 3, 64).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "dup-tcp-flags-set" => {
                dp.dup_tcp_flags_set = parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?
            }
            "dup-tcp-flags-unset" => {
                dp.dup_tcp_flags_unset =
                    parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?
            }
            "dup-replace" => dp.dup_replace = val.is_none() || atoi(v) != 0,
            "dup-fooling" => {
                dp.dup_fooling_mode = parse_fooling(v).ok_or_else(|| err("invalid fooling"))?
            }
            "dup-ts-increment" => {
                dp.dup_ts_increment = parse_net32_signed(v).ok_or_else(|| err("invalid value"))?
            }
            "dup-badseq-increment" => {
                dp.dup_badseq_increment =
                    parse_net32_signed(v).ok_or_else(|| err("invalid value"))?
            }
            "dup-badack-increment" => {
                dp.dup_badseq_ack_increment =
                    parse_net32_signed(v).ok_or_else(|| err("invalid value"))?
            }
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
                dp.orig_autottl =
                    parse_autottl(val.as_deref(), 5, 3, 64).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "orig-autottl6" => {
                dp.orig_autottl6 =
                    parse_autottl(val.as_deref(), 5, 3, 64).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "orig-tcp-flags-set" => {
                dp.orig_tcp_flags_set = parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?
            }
            "orig-tcp-flags-unset" => {
                dp.orig_tcp_flags_unset =
                    parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?
            }
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
                dp.desync_autottl =
                    parse_autottl(val.as_deref(), -1, 3, 20).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "dpi-desync-autottl6" => {
                dp.desync_autottl6 =
                    parse_autottl(val.as_deref(), -1, 3, 20).ok_or_else(|| err("value error"))?;
                params.autottl_present = true;
            }
            "dpi-desync-tcp-flags-set" => {
                dp.desync_tcp_flags_set =
                    parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?
            }
            "dpi-desync-tcp-flags-unset" => {
                dp.desync_tcp_flags_unset =
                    parse_tcpflags(v).ok_or_else(|| err("invalid tcp flags"))?
            }
            "dpi-desync-fooling" => {
                dp.desync_fooling_mode |= parse_fooling(v).ok_or_else(|| err("invalid fooling"))?
            }
            "dpi-desync-repeats" => {
                dp.desync_repeats = v
                    .parse()
                    .ok()
                    .filter(|&n| (1..=1024).contains(&n))
                    .ok_or_else(|| err("must be within 1..1024"))?;
            }
            "dpi-desync-skip-nosni" => dp.desync_skip_nosni = val.is_none() || atoi(v) != 0,
            "dpi-desync-split-pos" => {
                for item in v.split(',') {
                    if dp.splits.len() >= MAX_SPLITS {
                        return Err(err("too much split positions"));
                    }
                    dp.splits.push(
                        parse_split_pos(item)
                            .ok_or_else(|| err(&format!("could not parse split pos '{item}'")))?,
                    );
                }
            }
            "dpi-desync-split-http-req" => {
                dp.splits.push(match v {
                    "method" => ProtoPos {
                        marker: Marker::HttpMethod,
                        pos: 2,
                    },
                    "host" => ProtoPos {
                        marker: Marker::Host,
                        pos: 1,
                    },
                    _ => return Err(err("invalid value")),
                });
            }
            "dpi-desync-split-tls" => {
                dp.splits.push(match v {
                    "sni" => ProtoPos {
                        marker: Marker::Host,
                        pos: 1,
                    },
                    "sniext" => ProtoPos {
                        marker: Marker::SniExt,
                        pos: 1,
                    },
                    "snisld" => ProtoPos {
                        marker: Marker::HostMidSld,
                        pos: 0,
                    },
                    _ => return Err(err("invalid value")),
                });
            }
            "dpi-desync-split-seqovl" => {
                dp.seqovl = if v == "0" {
                    ProtoPos::EMPTY
                } else {
                    parse_split_pos(v).ok_or_else(|| err("invalid value"))?
                };
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
                    let (n, val) = item
                        .split_once('=')
                        .map(|(a, b)| (a, Some(b)))
                        .unwrap_or((item, None));
                    match n {
                        "altorder" => {
                            let o = atoi(
                                val.filter(|s| !s.is_empty())
                                    .ok_or_else(|| err("invalid mod"))?,
                            ) as i32;
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
                dp.hostfakesplit_midhost = if v == "0" {
                    ProtoPos::EMPTY
                } else {
                    parse_split_pos(v).ok_or_else(|| err("invalid value"))?
                };
            }
            "dpi-desync-hostfakesplit-mod" => {
                for item in v.split(',') {
                    let (n, val) = item
                        .split_once('=')
                        .map(|(a, b)| (a, Some(b)))
                        .unwrap_or((item, None));
                    match n {
                        "host" => {
                            let h = val
                                .filter(|s| !s.is_empty())
                                .ok_or_else(|| err("invalid mod"))?;
                            dp.hfs_host = h.chars().take(126).collect();
                        }
                        "altorder" => {
                            let o = atoi(
                                val.filter(|s| !s.is_empty())
                                    .ok_or_else(|| err("invalid mod"))?,
                            ) as i32;
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
            "dpi-desync-ts-increment" => {
                dp.desync_ts_increment =
                    parse_net32_signed(v).ok_or_else(|| err("invalid value"))?
            }
            "dpi-desync-badseq-increment" => {
                dp.desync_badseq_increment =
                    parse_net32_signed(v).ok_or_else(|| err("invalid value"))?
            }
            "dpi-desync-badack-increment" => {
                dp.desync_badseq_ack_increment =
                    parse_net32_signed(v).ok_or_else(|| err("invalid value"))?
            }
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
                    let ofs = if v.len() > 2 {
                        atoi(&v[1..]) as usize
                    } else {
                        0
                    };
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
            "dpi-desync-fake-quic"
            | "dpi-desync-fake-wireguard"
            | "dpi-desync-fake-dht"
            | "dpi-desync-fake-discord"
            | "dpi-desync-fake-stun"
            | "dpi-desync-fake-unknown-udp" => {
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
                dp.udplen_increment = v
                    .parse()
                    .ok()
                    .filter(|n: &i32| (-0x8000..=0x7FFF).contains(n))
                    .ok_or_else(|| err("must be within -32768..32767"))?;
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
            "hostlist-auto"
            | "hostlist-auto-fail-threshold"
            | "hostlist-auto-fail-time"
            | "hostlist-auto-retrans-threshold"
            | "hostlist-auto-debug" => {
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
            "filter-l7" => {
                dp.filter_l7 = parse_l7_list(v).ok_or_else(|| err("invalid l7 filter"))?
            }
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
                params.wf.iface = (
                    a.parse().map_err(|_| err("bad value"))?,
                    b.parse().unwrap_or(0),
                );
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
        dp.splits.push(ProtoPos {
            marker: Marker::Abs,
            pos: 2,
        });
    }
    let uses_split_ovl = |m: Mode| matches!(m, Mode::FakedSplit | Mode::MultiSplit);
    if (dp.seqovl.marker != Marker::Abs || dp.seqovl.pos < 0)
        && (uses_split_ovl(dp.desync_mode) || uses_split_ovl(dp.desync_mode2))
    {
        return Err("split seqovl supports only absolute positive positions".into());
    }

    // fake defaults
    if dp.fake_http.is_empty() {
        dp.fake_http.push(blob_from(
            FAKE_HTTP_REQUEST_DEFAULT.as_bytes().to_vec(),
            0,
            0,
        ));
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
    for list in [
        &mut dp.fake_wg,
        &mut dp.fake_dht,
        &mut dp.fake_discord,
        &mut dp.fake_stun,
        &mut dp.fake_unknown_udp,
    ] {
        if list.is_empty() {
            list.push(blob_from(vec![0; 64], 0, 0));
        }
    }

    onetime_tls_mod(dp)
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
        assert!(
            crate::engine::proto::tls_hello_extract_host(&tls.data, false).as_deref()
                == Some("www.google.com")
        );
        assert_eq!(p.profiles[2].n, 0);
    }

    #[test]
    fn tls_sni_resize_keeps_structure_valid() {
        let p = parse(&s(&[
            "--dpi-desync=fake",
            "--dpi-desync-fake-tls=!",
            "--dpi-desync-fake-tls-mod=sni=ya.ru,padencap",
        ]))
        .unwrap();
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
