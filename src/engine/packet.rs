use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::rng;

pub const FOOL_MD5SIG: u32 = 0x01;
pub const FOOL_BADSUM: u32 = 0x02;
pub const FOOL_TS: u32 = 0x04;
pub const FOOL_BADSEQ: u32 = 0x08;
pub const FOOL_HOPBYHOP: u32 = 0x10;
pub const FOOL_HOPBYHOP2: u32 = 0x20;
pub const FOOL_DESTOPT: u32 = 0x40;
pub const FOOL_IPFRAG1: u32 = 0x80;
pub const FOOL_DATANOACK: u32 = 0x100;

pub const SCALE_NONE: u8 = 255;

pub const TH_FIN: u16 = 0x01;
pub const TH_SYN: u16 = 0x02;
pub const TH_RST: u16 = 0x04;
pub const TH_PUSH: u16 = 0x08;
pub const TH_ACK: u16 = 0x10;
pub const TH_URG: u16 = 0x20;

pub const IPPROTO_TCP: u8 = 6;
pub const IPPROTO_UDP: u8 = 17;
pub const IPPROTO_HOPOPTS: u8 = 0;
pub const IPPROTO_DSTOPTS: u8 = 60;
pub const IPPROTO_FRAGMENT: u8 = 44;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Endpoint {
    pub ip: IpAddr,
    pub port: u16,
}

pub fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}
pub fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
pub fn put16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_be_bytes());
}
pub fn put32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_be_bytes());
}

fn sum_words(data: &[u8], mut acc: u64) -> u64 {
    let mut chunks = data.chunks_exact(2);
    for c in &mut chunks {
        acc += u16::from_be_bytes([c[0], c[1]]) as u64;
    }
    if let [last] = chunks.remainder() {
        acc += (*last as u64) << 8;
    }
    acc
}

fn fold(mut acc: u64) -> u16 {
    while acc >> 16 != 0 {
        acc = (acc & 0xFFFF) + (acc >> 16);
    }
    !(acc as u16)
}

pub fn ip4_header_checksum(hdr: &[u8]) -> u16 {
    fold(sum_words(hdr, 0))
}

fn pseudo_sum(src: &IpAddr, dst: &IpAddr, proto: u8, len: usize) -> u64 {
    let mut acc = 0u64;
    match (src, dst) {
        (IpAddr::V4(s), IpAddr::V4(d)) => {
            acc = sum_words(&s.octets(), acc);
            acc = sum_words(&d.octets(), acc);
            acc += proto as u64;
            acc += len as u64;
        }
        (IpAddr::V6(s), IpAddr::V6(d)) => {
            acc = sum_words(&s.octets(), acc);
            acc = sum_words(&d.octets(), acc);
            acc += (len as u64 >> 16) + (len as u64 & 0xFFFF);
            acc += proto as u64;
        }
        _ => {}
    }
    acc
}

pub fn l4_checksum(src: &IpAddr, dst: &IpAddr, proto: u8, seg: &[u8]) -> u16 {
    fold(sum_words(seg, pseudo_sum(src, dst, proto, seg.len())))
}

#[derive(Clone, Debug)]
pub struct Pkt {
    pub buf: Vec<u8>,
    pub v6: bool,
    pub l3_len: usize,
    #[allow(dead_code)]
    pub proto: u8,
    pub tcp: bool,
    pub udp: bool,
    pub payload_off: usize,
}

fn skip_ipv6(data: &[u8]) -> Option<(usize, u8, usize)> {
    let mut off = 40usize;
    let mut last = 6usize;
    let mut ht = data[6];
    while off < data.len() {
        let hdrlen = match ht {
            0 | 43 | 60 | 135 | 139 | 140 => {
                if data.len() - off < 2 {
                    return Some((off, ht, last));
                }
                8 + ((data[off + 1] as usize) << 3)
            }
            44 => 8,
            51 => {
                if data.len() - off < 2 {
                    return Some((off, ht, last));
                }
                8 + ((data[off + 1] as usize) << 2)
            }
            59 => return Some((off, ht, last)),
            _ => return Some((off, ht, last)),
        };
        if data.len() - off < hdrlen {
            return Some((off, ht, last));
        }
        ht = data[off];
        last = off;
        off += hdrlen;
    }
    Some((off, ht, last))
}

impl Pkt {
    pub fn dissect(buf: Vec<u8>) -> Option<Pkt> {
        let len = buf.len();
        let (v6, l3_len, proto) = if len >= 20 && buf[0] & 0xF0 == 0x40 && len >= ((buf[0] & 0x0F) as usize) << 2 {
            let hl = ((buf[0] & 0x0F) as usize) << 2;
            (false, hl, buf[9])
        } else if len >= 40 && buf[0] & 0xF0 == 0x60 && (len - 40) >= be16(&buf, 4) as usize {
            let (off, proto, _) = skip_ipv6(&buf)?;
            (true, off, proto)
        } else {
            return None;
        };
        let rest = len - l3_len;
        let mut p = Pkt { buf, v6, l3_len, proto, tcp: false, udp: false, payload_off: len };
        if proto == IPPROTO_TCP && rest >= 20 && rest >= ((p.buf[l3_len + 12] & 0xF0) as usize) >> 2 {
            p.tcp = true;
            p.payload_off = l3_len + (((p.buf[l3_len + 12] & 0xF0) as usize) >> 2);
        } else if proto == IPPROTO_UDP && rest >= 8 && rest >= be16(&p.buf, l3_len + 4) as usize {
            p.udp = true;
            p.payload_off = l3_len + 8;
        }
        Some(p)
    }

    pub fn payload(&self) -> &[u8] {
        &self.buf[self.payload_off..]
    }
    pub fn payload_mut(&mut self) -> &mut [u8] {
        let o = self.payload_off;
        &mut self.buf[o..]
    }
    pub fn src_ip(&self) -> IpAddr {
        if self.v6 {
            let mut o = [0u8; 16];
            o.copy_from_slice(&self.buf[8..24]);
            IpAddr::V6(Ipv6Addr::from(o))
        } else {
            IpAddr::V4(Ipv4Addr::new(self.buf[12], self.buf[13], self.buf[14], self.buf[15]))
        }
    }
    pub fn dst_ip(&self) -> IpAddr {
        if self.v6 {
            let mut o = [0u8; 16];
            o.copy_from_slice(&self.buf[24..40]);
            IpAddr::V6(Ipv6Addr::from(o))
        } else {
            IpAddr::V4(Ipv4Addr::new(self.buf[16], self.buf[17], self.buf[18], self.buf[19]))
        }
    }
    pub fn sport(&self) -> u16 {
        if self.tcp || self.udp { be16(&self.buf, self.l3_len) } else { 0 }
    }
    pub fn dport(&self) -> u16 {
        if self.tcp || self.udp { be16(&self.buf, self.l3_len + 2) } else { 0 }
    }
    pub fn src(&self) -> Endpoint {
        Endpoint { ip: self.src_ip(), port: self.sport() }
    }
    pub fn dst(&self) -> Endpoint {
        Endpoint { ip: self.dst_ip(), port: self.dport() }
    }
    pub fn ttl(&self) -> u8 {
        if self.v6 { self.buf[7] } else { self.buf[8] }
    }
    pub fn ip_id(&self) -> u16 {
        if self.v6 { 0 } else { be16(&self.buf, 4) }
    }
    pub fn set_ip_id(&mut self, id: u16) {
        if !self.v6 {
            put16(&mut self.buf, 4, id);
            self.fix_ip4_checksum();
        }
    }
    pub fn df(&self) -> bool {
        !self.v6 && be16(&self.buf, 6) & 0x4000 != 0
    }
    pub fn tos(&self) -> u8 {
        if self.v6 { 0 } else { self.buf[1] }
    }
    pub fn flow(&self) -> u32 {
        if self.v6 { be32(&self.buf, 0) & 0x0FFF_FFFF } else { 0 }
    }
    pub fn set_ttl(&mut self, ttl: u8) -> bool {
        if ttl == 0 || self.ttl() == ttl {
            return false;
        }
        if self.v6 {
            self.buf[7] = ttl;
        } else {
            self.buf[8] = ttl;
            self.fix_ip4_checksum();
        }
        true
    }
    pub fn fix_ip4_checksum(&mut self) {
        if !self.v6 {
            let hl = self.l3_len;
            self.buf[10] = 0;
            self.buf[11] = 0;
            let c = ip4_header_checksum(&self.buf[..hl]);
            put16(&mut self.buf, 10, c);
        }
    }
    pub fn fix_l4_checksum(&mut self) {
        let (src, dst, off) = (self.src_ip(), self.dst_ip(), self.l3_len);
        if self.tcp {
            self.buf[off + 16] = 0;
            self.buf[off + 17] = 0;
            let c = l4_checksum(&src, &dst, IPPROTO_TCP, &self.buf[off..]);
            put16(&mut self.buf, off + 16, c);
        } else if self.udp {
            self.buf[off + 6] = 0;
            self.buf[off + 7] = 0;
            let mut c = l4_checksum(&src, &dst, IPPROTO_UDP, &self.buf[off..]);
            if c == 0 {
                c = 0xFFFF;
            }
            put16(&mut self.buf, off + 6, c);
        }
    }

    pub fn tcp_flags(&self) -> u16 {
        self.buf[self.l3_len + 13] as u16 | (((self.buf[self.l3_len + 12] & 0x0F) as u16) << 8)
    }
    pub fn set_tcp_flags(&mut self, fl: u16) {
        let o = self.l3_len;
        self.buf[o + 13] = fl as u8;
        self.buf[o + 12] = (self.buf[o + 12] & 0xF0) | ((fl >> 8) as u8 & 0x0F);
    }
    pub fn seq(&self) -> u32 {
        be32(&self.buf, self.l3_len + 4)
    }
    pub fn ack(&self) -> u32 {
        be32(&self.buf, self.l3_len + 8)
    }
    pub fn win(&self) -> u16 {
        be16(&self.buf, self.l3_len + 14)
    }
    pub fn set_win(&mut self, w: u16) {
        let o = self.l3_len + 14;
        put16(&mut self.buf, o, w);
    }

    pub fn tcp_find_option(&self, kind: u8) -> Option<usize> {
        let base = self.l3_len + 20;
        let end = self.payload_off;
        let b = &self.buf;
        let mut t = base;
        while t < end {
            match b[t] {
                0 => return None,
                1 => t += 1,
                k => {
                    if t + 1 >= end || b[t + 1] < 2 || t + b[t + 1] as usize > end {
                        return None;
                    }
                    if k == kind {
                        return Some(t);
                    }
                    t += b[t + 1] as usize;
                }
            }
        }
        None
    }
    pub fn tcp_timestamps(&self) -> Option<[u8; 8]> {
        let t = self.tcp_find_option(8)?;
        if self.buf[t + 1] != 10 {
            return None;
        }
        let mut o = [0u8; 8];
        o.copy_from_slice(&self.buf[t + 2..t + 10]);
        Some(o)
    }
    pub fn tcp_scale_factor(&self) -> u8 {
        match self.tcp_find_option(3) {
            Some(t) if self.buf[t + 1] == 3 => self.buf[t + 2],
            _ => SCALE_NONE,
        }
    }
    pub fn tcp_has_fastopen(&self) -> bool {
        if self.tcp_find_option(34).is_some() {
            return true;
        }
        match self.tcp_find_option(254) {
            Some(t) => self.buf[t + 1] >= 4 && self.buf[t + 2] == 0xF9 && self.buf[t + 3] == 0x89,
            None => false,
        }
    }
    pub fn tcp_mss(&self) -> u16 {
        match self.tcp_find_option(2) {
            Some(t) if self.buf[t + 1] == 4 => be16(&self.buf, t + 2),
            _ => 0,
        }
    }
    pub fn tcp_has_sack(&self) -> bool {
        self.tcp_find_option(4).is_some()
    }
    pub fn rewrite_winsize(&mut self, winsize: u16, scale: u8) {
        self.set_win(winsize);
        if scale != SCALE_NONE {
            if let Some(t) = self.tcp_find_option(3) {
                if self.buf[t + 1] == 3 && scale < self.buf[t + 2] {
                    self.buf[t + 2] = scale;
                }
            }
        }
    }
}

const TCP_FLAGS_MASK: u16 = TH_URG | TH_ACK | TH_PUSH | TH_RST | TH_SYN | TH_FIN;
pub fn tcp_syn_segment(fl: u16) -> bool {
    fl & TCP_FLAGS_MASK == TH_SYN
}
pub fn tcp_synack_segment(fl: u16) -> bool {
    fl & TCP_FLAGS_MASK == (TH_ACK | TH_SYN)
}

pub struct TcpBuild<'a> {
    pub flags: u16,
    pub sack: bool,
    pub mss: u16,
    pub seq: u32,
    pub ack: u32,
    pub win: u16,
    pub scale: u8,
    pub ts: Option<[u8; 8]>,
    pub df: bool,
    pub ttl: u8,
    pub tos: u8,
    pub ip_id: u16,
    pub flow: u32,
    pub fooling: u32,
    pub ts_inc: u32,
    pub badseq_inc: u32,
    pub badseq_ack_inc: u32,
    pub data: &'a [u8],
}

fn tcpopt_len(sack: bool, mss: bool, fooling: u32, ts: bool, scale: u8) -> usize {
    let mut t = 0;
    if sack {
        t += 2;
    }
    if mss {
        t += 4;
    }
    if fooling & FOOL_MD5SIG != 0 {
        t += 18;
    }
    if ts {
        t += 10;
    }
    if scale != SCALE_NONE {
        t += 3;
    }
    (t + 3) & !3
}

fn fill_tcphdr(h: &mut [u8], src_port: u16, dst_port: u16, p: &TcpBuild) {
    for b in h.iter_mut().take(20) {
        *b = 0;
    }
    put16(h, 0, src_port);
    put16(h, 2, dst_port);
    let (seq, ack) = if p.fooling & FOOL_BADSEQ != 0 {
        (p.seq.wrapping_add(p.badseq_inc), p.ack.wrapping_add(p.badseq_ack_inc))
    } else {
        (p.seq, p.ack)
    };
    put32(h, 4, seq);
    put32(h, 8, ack);
    let mut flags = p.flags;
    if p.fooling & FOOL_DATANOACK != 0 && flags & (TH_SYN | TH_RST) == 0 && !p.data.is_empty() {
        flags &= !TH_ACK;
    }
    h[13] = flags as u8;
    put16(h, 14, p.win);
    let mut off = 5u8;
    let mut t = 20usize;
    if p.mss != 0 {
        h[t] = 2;
        h[t + 1] = 4;
        put16(h, t + 2, p.mss);
        t += 4;
    }
    if p.sack {
        h[t] = 4;
        h[t + 1] = 2;
        t += 2;
    }
    if p.fooling & FOOL_MD5SIG != 0 {
        h[t] = 19;
        h[t + 1] = 18;
        for i in 0..4 {
            put32(h, t + 2 + i * 4, rng::next_u32());
        }
        t += 18;
    }
    if let Some(ts) = p.ts {
        h[t] = 8;
        h[t + 1] = 10;
        h[t + 2..t + 10].copy_from_slice(&ts);
        if p.fooling & FOOL_TS != 0 {
            let v = u32::from_be_bytes([ts[0], ts[1], ts[2], ts[3]]).wrapping_add(p.ts_inc);
            put32(h, t + 2, v);
        }
        t += 10;
    }
    if p.scale != SCALE_NONE {
        h[t] = 3;
        h[t + 1] = 3;
        h[t + 2] = p.scale;
        t += 3;
    }
    while t & 3 != 0 {
        h[t] = 1;
        t += 1;
    }
    off += ((t - 20) >> 2) as u8;
    h[12] = (off << 4) | ((flags >> 8) as u8 & 0x0F);
}

fn ip6_ext_len(fooling: u32) -> usize {
    8 * usize::from(fooling & (FOOL_HOPBYHOP | FOOL_HOPBYHOP2) == FOOL_HOPBYHOP)
        + 16 * usize::from(fooling & FOOL_HOPBYHOP2 != 0)
        + 8 * usize::from(fooling & FOOL_DESTOPT != 0)
        + 8 * usize::from(fooling & FOOL_IPFRAG1 != 0)
}

fn write_ip6_header(buf: &mut [u8], src: &Ipv6Addr, dst: &Ipv6Addr, payload_len: usize, proto: u8, ttl: u8, flow: u32) {
    put32(buf, 0, (flow & 0x0FFF_FFFF) | 0x6000_0000);
    put16(buf, 4, payload_len as u16);
    buf[6] = proto;
    buf[7] = ttl;
    buf[8..24].copy_from_slice(&src.octets());
    buf[24..40].copy_from_slice(&dst.octets());
}

fn write_ip4_header(buf: &mut [u8], src: &Ipv4Addr, dst: &Ipv4Addr, pktlen: usize, proto: u8, df: bool, ttl: u8, tos: u8, id: u16) {
    buf[0] = 0x45;
    buf[1] = tos;
    put16(buf, 2, pktlen as u16);
    put16(buf, 4, id);
    put16(buf, 6, if df { 0x4000 } else { 0 });
    buf[8] = ttl;
    buf[9] = proto;
    buf[10] = 0;
    buf[11] = 0;
    buf[12..16].copy_from_slice(&src.octets());
    buf[16..20].copy_from_slice(&dst.octets());
    let c = ip4_header_checksum(&buf[..20]);
    put16(buf, 10, c);
}

// builds ipv6 extension chain, returns (first proto, offset of transport)
fn build_ip6_ext(buf: &mut [u8], base: usize, fooling: u32, transport_proto: u8) -> (u8, usize) {
    let mut off = base;
    let mut proto = transport_proto;
    let mut nexttype: Option<usize> = None;
    if fooling & (FOOL_HOPBYHOP | FOOL_HOPBYHOP2) != 0 {
        let hbh = off;
        for b in &mut buf[hbh..hbh + 8] {
            *b = 0;
        }
        off += 8;
        let mut last = hbh;
        if fooling & FOOL_HOPBYHOP2 != 0 {
            last = off;
            for b in &mut buf[last..last + 8] {
                *b = 0;
            }
            off += 8;
        }
        buf[last] = transport_proto;
        nexttype = Some(last);
        proto = IPPROTO_HOPOPTS;
    }
    if fooling & FOOL_DESTOPT != 0 {
        let d = off;
        for b in &mut buf[d..d + 8] {
            *b = 0;
        }
        buf[d] = transport_proto;
        off += 8;
        match nexttype {
            Some(n) => buf[n] = IPPROTO_DSTOPTS,
            None => proto = IPPROTO_DSTOPTS,
        }
        nexttype = Some(d);
    }
    if fooling & FOOL_IPFRAG1 != 0 {
        let f = off;
        buf[f] = transport_proto;
        buf[f + 1] = 0;
        put16(buf, f + 2, 0);
        put32(buf, f + 4, 1 + rng::next_u32() % 0xFFFF_FFFF);
        off += 8;
        match nexttype {
            Some(n) => buf[n] = IPPROTO_FRAGMENT,
            None => proto = IPPROTO_FRAGMENT,
        }
    }
    (proto, off)
}

pub fn prepare_tcp_segment(src: &Endpoint, dst: &Endpoint, p: &TcpBuild, max_len: usize) -> Option<Vec<u8>> {
    let optlen = tcpopt_len(p.sack, p.mss != 0, p.fooling, p.ts.is_some(), p.scale);
    let tcp_len = 20 + optlen + p.data.len();
    match (src.ip, dst.ip) {
        (IpAddr::V4(s), IpAddr::V4(d)) => {
            let pktlen = 20 + tcp_len;
            if pktlen > max_len || pktlen > 0xFFFF {
                return None;
            }
            let mut buf = vec![0u8; pktlen];
            write_ip4_header(&mut buf, &s, &d, pktlen, IPPROTO_TCP, p.df, p.ttl, p.tos, p.ip_id);
            fill_tcphdr(&mut buf[20..], src.port, dst.port, p);
            buf[20 + 20 + optlen..].copy_from_slice(p.data);
            let c = l4_checksum(&src.ip, &dst.ip, IPPROTO_TCP, &buf[20..]);
            put16(&mut buf, 20 + 16, c);
            if p.fooling & FOOL_BADSUM != 0 {
                let c = be16(&buf, 36) ^ (1 + (rng::next_u32() % 0xFFFF) as u16);
                put16(&mut buf, 36, c);
            }
            Some(buf)
        }
        (IpAddr::V6(s), IpAddr::V6(d)) => {
            let ext = ip6_ext_len(p.fooling);
            let ip_payload = tcp_len + ext;
            let pktlen = 40 + ip_payload;
            if pktlen > max_len || pktlen > 0xFFFF + 40 {
                return None;
            }
            let mut buf = vec![0u8; pktlen];
            let (proto, toff) = build_ip6_ext(&mut buf, 40, p.fooling, IPPROTO_TCP);
            write_ip6_header(&mut buf, &s, &d, ip_payload, proto, p.ttl, p.flow);
            fill_tcphdr(&mut buf[toff..], src.port, dst.port, p);
            buf[toff + 20 + optlen..].copy_from_slice(p.data);
            let c = l4_checksum(&src.ip, &dst.ip, IPPROTO_TCP, &buf[toff..]);
            put16(&mut buf, toff + 16, c);
            if p.fooling & FOOL_BADSUM != 0 {
                let c = be16(&buf, toff + 16) ^ (1 + (rng::next_u32() % 0xFFFF) as u16);
                put16(&mut buf, toff + 16, c);
            }
            Some(buf)
        }
        _ => None,
    }
}

pub struct UdpBuild<'a> {
    pub df: bool,
    pub ttl: u8,
    pub tos: u8,
    pub ip_id: u16,
    pub flow: u32,
    pub fooling: u32,
    pub padding: Option<&'a [u8]>,
    pub padlen: i32,
    pub data: &'a [u8],
}

pub fn fill_pattern(buf: &mut [u8], pattern: &[u8], offset: usize) {
    if pattern.is_empty() || buf.is_empty() {
        return;
    }
    let mut pos = 0;
    let off = offset % pattern.len();
    if off != 0 {
        let n = (pattern.len() - off).min(buf.len());
        buf[..n].copy_from_slice(&pattern[off..off + n]);
        pos = n;
    }
    while pos < buf.len() {
        let n = (buf.len() - pos).min(pattern.len());
        buf[pos..pos + n].copy_from_slice(&pattern[..n]);
        pos += n;
    }
}

pub fn prepare_udp_segment(src: &Endpoint, dst: &Endpoint, p: &UdpBuild, max_len: usize) -> Option<Vec<u8>> {
    let mut len = p.data.len() as i32;
    let mut padlen = p.padlen;
    if len + padlen <= 0 {
        padlen = -len + 1;
    }
    if len + padlen > 0xFFFF {
        padlen = 0xFFFF - len;
    }
    if padlen < 0 {
        len += padlen;
        padlen = 0;
    }
    let datalen = (len + padlen) as usize;
    let udp_len = 8 + datalen;

    let fill = |payload: &mut [u8]| {
        payload[..len as usize].copy_from_slice(&p.data[..len as usize]);
        match p.padding {
            Some(pat) => fill_pattern(&mut payload[len as usize..], pat, 0),
            None => payload[len as usize..].fill(0),
        }
    };

    match (src.ip, dst.ip) {
        (IpAddr::V4(s), IpAddr::V4(d)) => {
            let pktlen = 20 + udp_len;
            if pktlen > max_len || pktlen > 0xFFFF {
                return None;
            }
            let mut buf = vec![0u8; pktlen];
            write_ip4_header(&mut buf, &s, &d, pktlen, IPPROTO_UDP, p.df, p.ttl, p.tos, p.ip_id);
            put16(&mut buf, 20, src.port);
            put16(&mut buf, 22, dst.port);
            put16(&mut buf, 24, udp_len as u16);
            fill(&mut buf[28..]);
            let mut c = l4_checksum(&src.ip, &dst.ip, IPPROTO_UDP, &buf[20..]);
            if c == 0 {
                c = 0xFFFF;
            }
            put16(&mut buf, 26, c);
            if p.fooling & FOOL_BADSUM != 0 {
                let c = be16(&buf, 26) ^ (1 + (rng::next_u32() % 0xFFFF) as u16);
                put16(&mut buf, 26, c);
            }
            Some(buf)
        }
        (IpAddr::V6(s), IpAddr::V6(d)) => {
            let ext = ip6_ext_len(p.fooling);
            let ip_payload = udp_len + ext;
            let pktlen = 40 + ip_payload;
            if pktlen > max_len || pktlen > 0xFFFF + 40 {
                return None;
            }
            let mut buf = vec![0u8; pktlen];
            let (proto, toff) = build_ip6_ext(&mut buf, 40, p.fooling, IPPROTO_UDP);
            write_ip6_header(&mut buf, &s, &d, ip_payload, proto, p.ttl, p.flow);
            put16(&mut buf, toff, src.port);
            put16(&mut buf, toff + 2, dst.port);
            put16(&mut buf, toff + 4, udp_len as u16);
            fill(&mut buf[toff + 8..]);
            let mut c = l4_checksum(&src.ip, &dst.ip, IPPROTO_UDP, &buf[toff..]);
            if c == 0 {
                c = 0xFFFF;
            }
            put16(&mut buf, toff + 6, c);
            if p.fooling & FOOL_BADSUM != 0 {
                let c = be16(&buf, toff + 6) ^ (1 + (rng::next_u32() % 0xFFFF) as u16);
                put16(&mut buf, toff + 6, c);
            }
            Some(buf)
        }
        _ => None,
    }
}

pub fn hop_count_guess(ttl: u8) -> u8 {
    let orig = if ttl > 223 {
        255
    } else if ttl < 128 && ttl > 96 {
        128
    } else if ttl < 64 && ttl > 32 {
        64
    } else {
        return 0;
    };
    orig - ttl
}

#[derive(Clone, Copy, Default, Debug)]
pub struct AutoTtl {
    pub delta: i8,
    pub min: u8,
    pub max: u8,
}

impl AutoTtl {
    pub fn enabled(&self) -> bool {
        self.delta != 0 || self.min != 0 || self.max != 0
    }
    pub fn eval(&self, hop_count: u8) -> u8 {
        let d = hop_count as i32 + self.delta as i32;
        let fake = if d < self.min as i32 {
            self.min
        } else if d > self.max as i32 {
            self.max
        } else {
            d as u8
        };
        if (self.delta < 0 && fake >= hop_count) || (self.delta >= 0 && fake < hop_count) {
            return 0;
        }
        fake
    }
}
