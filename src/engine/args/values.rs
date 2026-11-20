use super::*;

pub(super) fn atoi(s: &str) -> i64 {
    let s = s.trim_start();
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    let v: i64 = digits.parse().unwrap_or(0);
    if neg { -v } else { v }
}

pub(super) fn parse_cutoff(s: &str) -> Option<(u32, char)> {
    let (mode, rest) = match s.chars().next() {
        Some(c @ ('n' | 'd' | 's')) => (c, &s[1..]),
        _ => ('n', s),
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    Some((digits.parse().ok()?, mode))
}

pub(super) fn parse_net32_signed(s: &str) -> Option<u32> {
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

pub(super) fn parse_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

// 0xaabbcc | filename | @filename | +123@filename
pub(super) fn load_file_or_hex(name: &str, max: usize) -> Result<(Vec<u8>, usize), String> {
    if let Some(h) = name.strip_prefix("0x") {
        let v = parse_hex(h)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| format!("invalid hex string: {h}"))?;
        return Ok((v, 0));
    }
    let mut ofs = 0usize;
    let mut f = name;
    if let Some(r) = f.strip_prefix('+') {
        let digits: String = r.chars().take_while(|c| c.is_ascii_digit()).collect();
        ofs = digits
            .parse()
            .map_err(|_| format!("offset read error: {r}"))?;
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
        return Err(format!(
            "'{f}' : offset {ofs} is out of data range {}",
            data.len()
        ));
    }
    Ok((data, ofs))
}

pub(super) fn parse_fooling(s: &str) -> Option<u32> {
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

pub(super) fn parse_tcpflags(s: &str) -> Option<u16> {
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

pub(super) fn parse_autottl(
    s: Option<&str>,
    def_delta: i8,
    def_min: u8,
    def_max: u8,
) -> Option<AutoTtl> {
    let mut t = AutoTtl {
        delta: def_delta,
        min: def_min,
        max: def_max,
    };
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
    t.delta = if neg {
        -(delta as i32) as i8
    } else {
        delta as i8
    };
    Some(t)
}

pub(super) fn parse_l7_list(s: &str) -> Option<u32> {
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

pub(super) fn parse_marker(s: &str) -> Option<Marker> {
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

pub(super) fn parse_int16(s: &str) -> Option<i16> {
    let c = s.chars().next()?;
    if c == '+' || c == '-' || c.is_ascii_digit() {
        let v = atoi(s);
        i16::try_from(v).ok()
    } else {
        None
    }
}

pub(super) fn parse_split_pos(s: &str) -> Option<ProtoPos> {
    if let Some(v) = parse_int16(s) {
        return if v != 0 {
            Some(ProtoPos {
                marker: Marker::Abs,
                pos: v,
            })
        } else {
            None
        };
    }
    let end = s.find(['+', '-']).unwrap_or(s.len());
    let marker = parse_marker(&s[..end])?;
    let pos = if end < s.len() {
        parse_int16(&s[end..])?
    } else {
        0
    };
    Some(ProtoPos { marker, pos })
}

pub(super) fn parse_tlsmod_list(s: &str, tm: &mut FakeTlsMod) -> bool {
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

pub(super) fn parse_desync_mode(s: Option<&str>) -> Result<Option<Mode>, String> {
    let Some(s) = s else {
        return Ok(Some(Mode::None));
    };
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
            return Err(format!(
                "dpi-desync mode '{s}' is not supported by this engine"
            ));
        }
        _ => return Ok(None),
    }))
}

pub(super) fn valid_zero_stage(m: Mode) -> bool {
    matches!(m, Mode::SynAck | Mode::SynData)
}
pub(super) fn valid_first_stage(m: Mode) -> bool {
    matches!(m, Mode::Fake | Mode::FakeKnown | Mode::Rst | Mode::RstAck)
}
pub(super) fn valid_second_stage(m: Mode) -> bool {
    matches!(
        m,
        Mode::None
            | Mode::FakedDisorder
            | Mode::FakedSplit
            | Mode::MultiSplit
            | Mode::MultiDisorder
            | Mode::HostFakeSplit
            | Mode::Udplen
            | Mode::Tamper
    )
}
