use super::packet::{be16, be32};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum L7 {
    #[default]
    Unknown,
    Http,
    Tls,
    Quic,
    Wireguard,
    Dht,
    Discord,
    Stun,
}

pub const L7_HTTP: u32 = 0x1;
pub const L7_TLS: u32 = 0x2;
pub const L7_QUIC: u32 = 0x4;
pub const L7_WIREGUARD: u32 = 0x8;
pub const L7_DHT: u32 = 0x10;
pub const L7_DISCORD: u32 = 0x20;
pub const L7_STUN: u32 = 0x40;
pub const L7_UNKNOWN: u32 = 0x8000_0000;

pub fn l7_match(l7: L7, filter: u32) -> bool {
    let bit = match l7 {
        L7::Unknown => L7_UNKNOWN,
        L7::Http => L7_HTTP,
        L7::Tls => L7_TLS,
        L7::Quic => L7_QUIC,
        L7::Wireguard => L7_WIREGUARD,
        L7::Dht => L7_DHT,
        L7::Discord => L7_DISCORD,
        L7::Stun => L7_STUN,
    };
    filter & bit != 0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Marker {
    Abs,
    Host,
    HostEnd,
    HostSld,
    HostMidSld,
    HostEndSld,
    HttpMethod,
    SniExt,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProtoPos {
    pub marker: Marker,
    pub pos: i16,
}

impl ProtoPos {
    pub const EMPTY: ProtoPos = ProtoPos { marker: Marker::Abs, pos: 0 };
}

fn check_pos(sz: usize, offset: i64) -> usize {
    if offset >= 0 && (offset as usize) < sz { offset as usize } else { 0 }
}

fn any_proto_pos(marker: Marker, pos: i16, sz: usize) -> usize {
    match marker {
        Marker::Abs => {
            let offset = if pos < 0 { sz as i64 + pos as i64 } else { pos as i64 };
            check_pos(sz, offset)
        }
        _ => 0,
    }
}

// find N level domain, returns (start, len)
fn find_nld(dom: &[u8], level: usize) -> Option<(usize, usize)> {
    let mut p2 = dom.len();
    for _ in 1..level {
        if p2 == 0 {
            return None;
        }
        p2 -= 1;
        while p2 > 0 && dom[p2] != b'.' {
            p2 -= 1;
        }
        if p2 == 0 {
            return None;
        }
    }
    let mut p1 = p2.saturating_sub(1);
    while p1 > 0 && dom[p1] != b'.' {
        p1 -= 1;
    }
    if dom.get(p1) == Some(&b'.') {
        p1 += 1;
    }
    Some((p1, p2 - p1))
}

fn host_pos(marker: Marker, pos: i16, data: &[u8], offset_host: usize, len_host: usize) -> usize {
    let sz = data.len();
    let offset: i64 = match marker {
        Marker::Host => offset_host as i64 + pos as i64,
        Marker::HostEnd => (offset_host + len_host) as i64 + pos as i64,
        Marker::HostSld | Marker::HostMidSld | Marker::HostEndSld => {
            if offset_host + len_host <= sz {
                match find_nld(&data[offset_host..offset_host + len_host], 2) {
                    Some((p, slen)) => {
                        let base = offset_host + p;
                        (match marker {
                            Marker::HostSld => base,
                            Marker::HostEndSld => base + slen,
                            _ => {
                                if slen == 1 { base + 1 } else { base + slen / 2 }
                            }
                        }) as i64
                            + pos as i64
                    }
                    None => 0,
                }
            } else {
                0
            }
        }
        _ => 0,
    };
    check_pos(sz, offset)
}

pub fn resolve_pos(data: &[u8], l7: L7, sp: &ProtoPos) -> usize {
    match l7 {
        L7::Http => http_pos(sp.marker, sp.pos, data),
        L7::Tls => tls_pos(sp.marker, sp.pos, data),
        _ => any_proto_pos(sp.marker, sp.pos, data.len()),
    }
}

pub fn resolve_multi_pos(data: &[u8], l7: L7, splits: &[ProtoPos]) -> Vec<usize> {
    let mut v: Vec<usize> = splits.iter().map(|s| resolve_pos(data, l7, s)).filter(|&p| p != 0).collect();
    v.sort_unstable();
    v.dedup();
    v
}

const HTTP_METHODS: [&[u8]; 8] = [b"GET /", b"POST /", b"HEAD /", b"OPTIONS ", b"PUT /", b"DELETE /", b"CONNECT ", b"TRACE /"];

pub fn is_http(data: &[u8]) -> bool {
    HTTP_METHODS.iter().any(|m| data.len() >= m.len() && data.starts_with(m))
}

fn is_host_at(p: &[u8]) -> bool {
    p[0] == b'\n' && p[1].eq_ignore_ascii_case(&b'h') && p[2].eq_ignore_ascii_case(&b'o') && p[3].eq_ignore_ascii_case(&b's') && p[4].eq_ignore_ascii_case(&b't') && p[5] == b':'
}

// returns offset of "Host:" (after the leading '\n')
pub fn http_find_host(buf: &[u8]) -> Option<usize> {
    if buf.len() < 6 {
        return None;
    }
    (0..=buf.len() - 6).find(|&p| is_host_at(&buf[p..])).map(|p| p + 1)
}


fn find_ci(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()].eq_ignore_ascii_case(needle))
}

pub fn http_extract_header(data: &[u8], header: &str) -> Option<String> {
    let p = find_ci(data, header.as_bytes())? + header.len();
    let mut s = p;
    while s < data.len() && (data[s] == b' ' || data[s] == b'\t') {
        s += 1;
    }
    let st = s;
    while s < data.len() && !matches!(data[s], b'\r' | b'\n' | b' ' | b'\t') {
        s += 1;
    }
    if s > st {
        Some(String::from_utf8_lossy(&data[st..s.min(st + 255)]).to_lowercase())
    } else {
        None
    }
}

pub fn http_extract_host(data: &[u8]) -> Option<String> {
    http_extract_header(data, "\nHost:")
}


fn http_pos(marker: Marker, pos: i16, data: &[u8]) -> usize {
    let sz = data.len();
    match marker {
        Marker::HttpMethod => {
            if sz < 10 {
                return 0;
            }
            let mut m = 0;
            if data[m] == b'\n' || data[m] == b'\r' {
                m += 1;
            }
            if data[m] == b'\n' || data[m] == b'\r' {
                m += 1;
            }
            let mut i = 0;
            let mut p = m;
            while i < 9 && p < sz && data[p].is_ascii_uppercase() {
                i += 1;
                p += 1;
            }
            if i < 3 || p >= sz || data[p] != b' ' {
                return 0;
            }
            check_pos(sz, m as i64 + pos as i64)
        }
        Marker::Host | Marker::HostEnd | Marker::HostSld | Marker::HostMidSld | Marker::HostEndSld => {
            if let Some(h) = http_find_host(data) {
                if h + 7 < sz {
                    let mut host = h + 5;
                    if data[host] == b' ' || data[host] == b'\t' {
                        host += 1;
                    }
                    let offset_host = host;
                    let len_host = if marker != Marker::Host {
                        let mut l = 0;
                        while offset_host + l < sz && data[offset_host + l] != b'\r' && data[offset_host + l] != b'\n' {
                            l += 1;
                        }
                        l
                    } else {
                        0
                    };
                    return host_pos(marker, pos, data, offset_host, len_host);
                }
            }
            0
        }
        _ => any_proto_pos(marker, pos, sz),
    }
}

pub fn tls_record_data_len(d: &[u8]) -> usize {
    be16(d, 3) as usize
}
pub fn tls_record_len(d: &[u8]) -> usize {
    tls_record_data_len(d) + 5
}
pub fn is_tls_record_full(d: &[u8]) -> bool {
    tls_record_len(d) <= d.len()
}
pub fn is_tls_client_hello(d: &[u8], partial_ok: bool) -> bool {
    d.len() >= 6 && d[0] == 0x16 && d[1] == 0x03 && d[2] <= 0x03 && d[5] == 0x01 && (partial_ok || tls_record_len(d) <= d.len())
}
pub fn tls_handshake_len(d: &[u8]) -> usize {
    ((d[1] as usize) << 16) | ((d[2] as usize) << 8) | d[3] as usize
}
pub fn is_tls_handshake_client_hello(d: &[u8]) -> bool {
    d.len() >= 4 && d[0] == 0x01 && tls_handshake_len(d) > 0
}
pub fn is_tls_handshake_full(d: &[u8]) -> bool {
    4 + tls_handshake_len(d) <= d.len()
}

pub fn tls_find_ext_len_offset_in_handshake(d: &[u8]) -> Option<usize> {
    let len = d.len();
    let mut l = 1 + 3 + 2 + 32;
    if len < l + 1 {
        return None;
    }
    l += d[l] as usize + 1;
    if len < l + 2 {
        return None;
    }
    l += be16(d, l) as usize + 2;
    if len < l + 1 {
        return None;
    }
    l += d[l] as usize + 1;
    if len < l + 2 {
        return None;
    }
    Some(l)
}

pub fn tls_find_ext_len(d: &[u8]) -> Option<usize> {
    if d.len() < 5 {
        return None;
    }
    tls_find_ext_len_offset_in_handshake(&d[5..]).map(|o| o + 5)
}

// returns (offset, len) of the extension body inside `d`
pub fn tls_find_ext_in_handshake(d: &[u8], etype: u16, partial_ok: bool) -> Option<(usize, usize)> {
    if !partial_ok && !is_tls_handshake_full(d) {
        return None;
    }
    let mut off = tls_find_ext_len_offset_in_handshake(d)?;
    let mut l = be16(d, off) as usize;
    off += 2;
    let remain = d.len() - off;
    if partial_ok {
        if remain < l {
            l = remain;
        }
    } else if remain < l {
        return None;
    }
    while l >= 4 {
        let t = be16(d, off);
        let elen = be16(d, off + 2) as usize;
        off += 4;
        l -= 4;
        if l < elen {
            break;
        }
        if t == etype {
            return Some((off, elen));
        }
        off += elen;
        l -= elen;
    }
    None
}

pub const TLS_PARTIALS_ENABLE: bool = true;

pub fn tls_find_ext(d: &[u8], etype: u16, partial_ok: bool) -> Option<(usize, usize)> {
    if !is_tls_client_hello(d, partial_ok) {
        return None;
    }
    let mut len = d.len();
    let reclen = tls_record_len(d);
    if reclen < len {
        len = reclen;
    }
    tls_find_ext_in_handshake(&d[5..len], etype, partial_ok).map(|(o, l)| (o + 5, l))
}

// advance SNI ext body to the host name. returns (offset_of_host, host_len)
pub fn tls_advance_to_host_in_sni(d: &[u8], ext_off: usize, elen: usize) -> Option<(usize, usize)> {
    if elen < 5 || d[ext_off + 2] != 0 {
        return None;
    }
    let slen = be16(d, ext_off + 3) as usize;
    if slen > elen - 5 {
        return None;
    }
    Some((ext_off + 5, slen))
}

fn extract_host_from_sni(d: &[u8], ext_off: usize, elen: usize) -> Option<String> {
    let (o, slen) = tls_advance_to_host_in_sni(d, ext_off, elen)?;
    let slen = slen.min(255);
    Some(String::from_utf8_lossy(&d[o..o + slen]).to_lowercase())
}

pub fn tls_hello_extract_host(d: &[u8], partial_ok: bool) -> Option<String> {
    let (o, l) = tls_find_ext(d, 0, partial_ok)?;
    extract_host_from_sni(d, o, l)
}

pub fn tls_hello_extract_host_from_handshake(d: &[u8], partial_ok: bool) -> Option<String> {
    let (o, l) = tls_find_ext_in_handshake(d, 0, partial_ok)?;
    extract_host_from_sni(d, o, l)
}

fn tls_pos(marker: Marker, pos: i16, data: &[u8]) -> usize {
    let sz = data.len();
    match marker {
        Marker::Host | Marker::HostEnd | Marker::HostSld | Marker::HostMidSld | Marker::HostEndSld | Marker::SniExt => {
            if let Some((eo, el)) = tls_find_ext(data, 0, TLS_PARTIALS_ENABLE) {
                if marker == Marker::SniExt {
                    return check_pos(sz, eo as i64 + pos as i64);
                }
                match tls_advance_to_host_in_sni(data, eo, el) {
                    Some((o, l)) => host_pos(marker, pos, data, o, l),
                    None => 0,
                }
            } else {
                0
            }
        }
        _ => any_proto_pos(marker, pos, sz),
    }
}

pub fn is_wireguard_handshake_initiation(d: &[u8]) -> bool {
    d.len() == 148 && d[0] == 1
}
pub fn is_dht_d1(d: &[u8]) -> bool {
    d.len() >= 7 && d[0] == b'd' && d[1] == b'1' && d[d.len() - 1] == b'e'
}
pub fn is_discord_ip_discovery_request(d: &[u8]) -> bool {
    d.len() == 74 && d[0] == 0 && d[1] == 1 && d[2] == 0 && d[3] == 70 && d[8..72].iter().all(|&b| b == 0)
}
pub fn is_stun_message(d: &[u8]) -> bool {
    d.len() >= 20 && d[0] & 0xC0 == 0 && d[3] & 3 == 0 && be32(d, 4) == 0x2112A442 && be16(d, 2) as usize <= d.len() - 20
}

pub fn strip_host_to_ip(host: &mut String) -> bool {
    // "[1.2.3.4]:443" style and bare ip literals
    let mut s = host.as_str();
    if let Some(r) = s.strip_prefix('[') {
        if let Some(e) = r.find(']') {
            s = &r[..e];
        }
    }
    let is_ip = s.parse::<std::net::IpAddr>().is_ok();
    if is_ip {
        *host = s.to_string();
    }
    is_ip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_positions() {
        let req = b"GET / HTTP/1.1\r\nHost: www.example.com\r\nUser-Agent: x\r\n\r\n";
        assert!(is_http(req));
        assert_eq!(http_extract_host(req).as_deref(), Some("www.example.com"));
        let p = resolve_pos(req, L7::Http, &ProtoPos { marker: Marker::Host, pos: 0 });
        assert_eq!(&req[p..p + 3], b"www");
        let e = resolve_pos(req, L7::Http, &ProtoPos { marker: Marker::HostEnd, pos: 0 });
        assert_eq!(&req[e..e + 2], b"\r\n");
        let m = resolve_pos(req, L7::Http, &ProtoPos { marker: Marker::HostMidSld, pos: 0 });
        assert!(req[m..].starts_with(b"mple.com") || req[m..].starts_with(b"ample.com"), "{:?}", String::from_utf8_lossy(&req[m..]));
    }

    #[test]
    fn nld() {
        assert_eq!(find_nld(b"www.google.com", 2), Some((4, 6)));
        assert_eq!(find_nld(b"google.com", 2), Some((0, 6)));
        assert_eq!(find_nld(b"com", 2), None);
    }
}
