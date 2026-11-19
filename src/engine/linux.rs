use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};

use super::conntrack::Iface;
use super::desync::*;
use super::packet::Endpoint;

const NETLINK_NETFILTER: i32 = 12;
const NFNL_SUBSYS_QUEUE: u16 = 3;
const NFQNL_MSG_PACKET: u16 = 0;
const NFQNL_MSG_VERDICT: u16 = 1;
const NFQNL_MSG_CONFIG: u16 = 2;
const NFQA_PACKET_HDR: u16 = 1;
const NFQA_VERDICT_HDR: u16 = 2;
const NFQA_MARK: u16 = 3;
const NFQA_IFINDEX_INDEV: u16 = 5;
const NFQA_IFINDEX_OUTDEV: u16 = 6;
const NFQA_PAYLOAD: u16 = 10;
const NFQA_CFG_CMD: u16 = 1;
const NFQA_CFG_PARAMS: u16 = 2;
const NFQA_CFG_QUEUE_MAXLEN: u16 = 3;
const NFQA_CFG_FLAGS: u16 = 5;
const NFQA_CFG_MASK: u16 = 6;
const NFQNL_CFG_CMD_BIND: u8 = 1;
const NFQNL_CFG_CMD_UNBIND: u8 = 2;
const NFQNL_CFG_CMD_PF_BIND: u8 = 3;
const NFQNL_CFG_CMD_PF_UNBIND: u8 = 4;
const NFQA_CFG_F_FAIL_OPEN: u32 = 1;
const NLM_F_REQUEST: u16 = 1;
const NLM_F_ACK: u16 = 4;
const NF_DROP: u32 = 0;
const NF_ACCEPT: u32 = 1;
const Q_MAXLEN: u32 = 4986;
const Q_RCVBUF: i32 = 1024 * 1024;

fn align4(n: usize) -> usize {
    (n + 3) & !3
}

fn attr(buf: &mut Vec<u8>, typ: u16, data: &[u8]) {
    let len = 4 + data.len();
    buf.extend_from_slice(&(len as u16).to_ne_bytes());
    buf.extend_from_slice(&typ.to_ne_bytes());
    buf.extend_from_slice(data);
    buf.resize(buf.len() + align4(len) - len, 0);
}

fn nl_msg(typ: u16, flags: u16, seq: u32, queue: u16, family: u8, attrs: &[u8]) -> Vec<u8> {
    let len = 16 + 4 + attrs.len();
    let mut m = Vec::with_capacity(len);
    m.extend_from_slice(&(len as u32).to_ne_bytes());
    m.extend_from_slice(&((NFNL_SUBSYS_QUEUE << 8) | typ).to_ne_bytes());
    m.extend_from_slice(&flags.to_ne_bytes());
    m.extend_from_slice(&seq.to_ne_bytes());
    m.extend_from_slice(&0u32.to_ne_bytes());
    m.push(family);
    m.push(0);
    m.extend_from_slice(&queue.to_be_bytes());
    m.extend_from_slice(attrs);
    m
}

struct RawSock {
    s4: i32,
    s6: i32,
}

impl RawSock {
    fn open_one(domain: i32) -> i32 {
        unsafe {
            let fd = libc::socket(domain, libc::SOCK_RAW, libc::IPPROTO_RAW);
            if fd < 0 {
                return -1;
            }
            let yes: i32 = 1;
            let pri: i32 = 6;
            let sndbuf: i32 = 32 * 1024;
            libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_SNDBUF, (&sndbuf as *const i32).cast(), 4);
            libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_PRIORITY, (&pri as *const i32).cast(), 4);
            if domain == libc::AF_INET {
                libc::setsockopt(fd, libc::IPPROTO_IP, 22, (&yes as *const i32).cast(), 4); // IP_NODEFRAG
                libc::setsockopt(fd, libc::IPPROTO_IP, libc::IP_FREEBIND, (&yes as *const i32).cast(), 4);
            } else {
                libc::setsockopt(fd, libc::IPPROTO_IPV6, 78, (&yes as *const i32).cast(), 4); // IPV6_FREEBIND
            }
            fd
        }
    }

    fn new() -> Result<RawSock, String> {
        let s4 = Self::open_one(libc::AF_INET);
        if s4 < 0 {
            return Err(format!("raw socket: {}", std::io::Error::last_os_error()));
        }
        Ok(RawSock { s4, s6: Self::open_one(libc::AF_INET6) })
    }
}

impl Drop for RawSock {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.s4);
            if self.s6 >= 0 {
                libc::close(self.s6);
            }
        }
    }
}

impl Io for RawSock {
    fn send(&mut self, dst: &Endpoint, fwmark: u32, _ifout: &Iface, data: &[u8]) -> bool {
        unsafe {
            let sock = if dst.ip.is_ipv4() { self.s4 } else { self.s6 };
            if sock < 0 {
                return false;
            }
            libc::setsockopt(sock, libc::SOL_SOCKET, libc::SO_MARK, (&fwmark as *const u32).cast(), 4);
            let r = match dst.ip {
                IpAddr::V4(a) => {
                    let mut sa: libc::sockaddr_in = std::mem::zeroed();
                    sa.sin_family = libc::AF_INET as u16;
                    sa.sin_addr.s_addr = u32::from_ne_bytes(a.octets());
                    libc::sendto(sock, data.as_ptr().cast(), data.len(), 0, (&sa as *const libc::sockaddr_in).cast(), size_of::<libc::sockaddr_in>() as u32)
                }
                IpAddr::V6(a) => {
                    let mut sa: libc::sockaddr_in6 = std::mem::zeroed();
                    sa.sin6_family = libc::AF_INET6 as u16;
                    sa.sin6_addr.s6_addr = a.octets();
                    libc::sendto(sock, data.as_ptr().cast(), data.len(), 0, (&sa as *const libc::sockaddr_in6).cast(), size_of::<libc::sockaddr_in6>() as u32)
                }
            };
            r >= 0
        }
    }
}

pub struct Queue {
    fd: i32,
    qnum: u16,
    fwmark: u32,
    raw: RawSock,
    seq: u32,
}

unsafe impl Send for Queue {}

impl Queue {
    fn xact(&mut self, msg: &[u8]) -> Result<(), String> {
        unsafe {
            if libc::send(self.fd, msg.as_ptr().cast(), msg.len(), 0) < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let mut buf = [0u8; 4096];
            let n = libc::recv(self.fd, buf.as_mut_ptr().cast(), buf.len(), 0);
            if n < 20 {
                return Err("short netlink reply".into());
            }
            // NLMSG_ERROR = 2, error code follows header
            let typ = u16::from_ne_bytes([buf[4], buf[5]]);
            if typ == 2 {
                let code = i32::from_ne_bytes([buf[16], buf[17], buf[18], buf[19]]);
                if code != 0 {
                    return Err(std::io::Error::from_raw_os_error(-code).to_string());
                }
            }
        }
        Ok(())
    }

    fn cfg(&mut self, family: u8, attrs: &[u8]) -> Result<(), String> {
        self.seq += 1;
        let m = nl_msg(NFQNL_MSG_CONFIG, NLM_F_REQUEST | NLM_F_ACK, self.seq, self.qnum, family, attrs);
        self.xact(&m)
    }

    fn cmd(&mut self, family: u8, cmd: u8, pf: u16) -> Result<(), String> {
        let mut a = Vec::new();
        attr(&mut a, NFQA_CFG_CMD, &[cmd, 0, (pf >> 8) as u8, pf as u8]);
        self.cfg(family, &a)
    }

    pub fn open(qnum: i32, fwmark: u32) -> Result<Queue, String> {
        if qnum < 0 {
            return Err("Need queue number (--qnum)".into());
        }
        let raw = RawSock::new()?;
        let fd = unsafe { libc::socket(libc::AF_NETLINK, libc::SOCK_RAW | libc::SOCK_CLOEXEC, NETLINK_NETFILTER) };
        if fd < 0 {
            return Err(format!("netlink socket: {}", std::io::Error::last_os_error()));
        }
        unsafe {
            let mut sa: libc::sockaddr_nl = std::mem::zeroed();
            sa.nl_family = libc::AF_NETLINK as u16;
            if libc::bind(fd, (&sa as *const libc::sockaddr_nl).cast(), size_of::<libc::sockaddr_nl>() as u32) < 0 {
                let e = std::io::Error::last_os_error();
                libc::close(fd);
                return Err(format!("netlink bind: {e}"));
            }
            let tv = libc::timeval { tv_sec: 0, tv_usec: 300_000 };
            libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_RCVTIMEO, (&tv as *const libc::timeval).cast(), size_of::<libc::timeval>() as u32);
            let rb: i32 = Q_RCVBUF;
            libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_RCVBUF, (&rb as *const i32).cast(), 4);
            let yes: i32 = 1;
            libc::setsockopt(fd, 270, 5, (&yes as *const i32).cast(), 4); // SOL_NETLINK, NETLINK_NO_ENOBUFS
        }
        let mut q = Queue { fd, qnum: qnum as u16, fwmark, raw, seq: 0 };
        let _ = q.cmd(libc::AF_INET as u8, NFQNL_CFG_CMD_PF_UNBIND, libc::AF_INET as u16);
        let _ = q.cmd(libc::AF_INET as u8, NFQNL_CFG_CMD_PF_BIND, libc::AF_INET as u16);
        q.cmd(0, NFQNL_CFG_CMD_BIND, 0).map_err(|e| format!("не удалось привязать NFQUEUE {qnum} (нужны root и модуль nfnetlink_queue): {e}"))?;
        let mut p = Vec::new();
        let mut params = 0xFFFFu32.to_be_bytes().to_vec();
        params.push(2); // NFQNL_COPY_PACKET
        attr(&mut p, NFQA_CFG_PARAMS, &params);
        q.cfg(0, &p).map_err(|e| format!("copy_packet mode: {e}"))?;
        let mut p = Vec::new();
        attr(&mut p, NFQA_CFG_QUEUE_MAXLEN, &Q_MAXLEN.to_be_bytes());
        let _ = q.cfg(0, &p);
        let mut p = Vec::new();
        attr(&mut p, NFQA_CFG_FLAGS, &NFQA_CFG_F_FAIL_OPEN.to_be_bytes());
        attr(&mut p, NFQA_CFG_MASK, &NFQA_CFG_F_FAIL_OPEN.to_be_bytes());
        let _ = q.cfg(0, &p);
        Ok(q)
    }

    fn verdict(&mut self, id: u32, v: u32, mark: u32, payload: Option<&[u8]>) {
        let mut a = Vec::new();
        let mut vh = v.to_be_bytes().to_vec();
        vh.extend_from_slice(&id.to_be_bytes());
        attr(&mut a, NFQA_VERDICT_HDR, &vh);
        attr(&mut a, NFQA_MARK, &mark.to_be_bytes());
        if let Some(p) = payload {
            attr(&mut a, NFQA_PAYLOAD, p);
        }
        self.seq += 1;
        let m = nl_msg(NFQNL_MSG_VERDICT, NLM_F_REQUEST, self.seq, self.qnum, 0, &a);
        unsafe {
            libc::send(self.fd, m.as_ptr().cast(), m.len(), 0);
        }
    }

    pub fn run(&mut self, engine: &mut Engine, stop: &AtomicBool) {
        let mut buf = vec![0u8; 65536];
        while !stop.load(Ordering::SeqCst) {
            let n = unsafe { libc::recv(self.fd, buf.as_mut_ptr().cast(), buf.len(), 0) };
            if n < 0 {
                continue;
            }
            let n = n as usize;
            let mut off = 0usize;
            while off + 16 <= n {
                let len = u32::from_ne_bytes(buf[off..off + 4].try_into().unwrap()) as usize;
                if len < 16 || off + len > n {
                    break;
                }
                let typ = u16::from_ne_bytes([buf[off + 4], buf[off + 5]]);
                if typ == (NFNL_SUBSYS_QUEUE << 8) | NFQNL_MSG_PACKET && len >= 20 {
                    let msg = buf[off + 20..off + len].to_vec();
                    self.handle_packet(engine, &msg);
                }
                off += align4(len);
            }
        }
        let _ = self.cmd(0, NFQNL_CFG_CMD_UNBIND, 0);
    }

    fn handle_packet(&mut self, engine: &mut Engine, attrs: &[u8]) {
        let (mut id, mut mark, mut indev, mut outdev) = (0u32, 0u32, 0u32, 0u32);
        let mut payload: Option<&[u8]> = None;
        let mut o = 0;
        while o + 4 <= attrs.len() {
            let alen = u16::from_ne_bytes([attrs[o], attrs[o + 1]]) as usize;
            let atyp = u16::from_ne_bytes([attrs[o + 2], attrs[o + 3]]) & 0x7FFF;
            if alen < 4 || o + alen > attrs.len() {
                break;
            }
            let d = &attrs[o + 4..o + alen];
            match atyp {
                NFQA_PACKET_HDR if d.len() >= 4 => id = u32::from_be_bytes([d[0], d[1], d[2], d[3]]),
                NFQA_MARK if d.len() >= 4 => mark = u32::from_be_bytes([d[0], d[1], d[2], d[3]]),
                NFQA_IFINDEX_INDEV if d.len() >= 4 => indev = u32::from_be_bytes([d[0], d[1], d[2], d[3]]),
                NFQA_IFINDEX_OUTDEV if d.len() >= 4 => outdev = u32::from_be_bytes([d[0], d[1], d[2], d[3]]),
                NFQA_PAYLOAD => payload = Some(d),
                _ => {}
            }
            o += align4(alen);
        }
        let Some(pl) = payload else {
            self.verdict(id, NF_ACCEPT, mark, None);
            return;
        };
        if mark & self.fwmark != 0 {
            self.verdict(id, NF_ACCEPT, mark, None);
            return;
        }
        let mut packet = pl.to_vec();
        let v = engine.process(&mut self.raw, mark, Iface { idx: indev, sub: 0 }, Iface { idx: outdev, sub: 0 }, &mut packet);
        match v & VERDICT_MASK {
            VERDICT_MODIFY => self.verdict(id, NF_ACCEPT, mark, Some(&packet)),
            VERDICT_DROP => self.verdict(id, NF_DROP, mark, None),
            _ => self.verdict(id, NF_ACCEPT, mark, None),
        }
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}
