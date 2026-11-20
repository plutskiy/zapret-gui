use std::ffi::{CString, c_void};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use libloading::Library;
use windows_sys::Win32::Foundation::GetLastError;

use super::args::*;
use super::conntrack::Iface;
use super::desync::*;
use super::packet::{Endpoint, flow_hash};
use super::pool::Pool;

const DIVERT_NO_LOCALNETS_V4_DST: &str = "((ip.DstAddr < 127.0.0.1 or ip.DstAddr > 127.255.255.255) and (ip.DstAddr < 10.0.0.0 or ip.DstAddr > 10.255.255.255) and (ip.DstAddr < 192.168.0.0 or ip.DstAddr > 192.168.255.255) and (ip.DstAddr < 172.16.0.0 or ip.DstAddr > 172.31.255.255) and (ip.DstAddr < 169.254.0.0 or ip.DstAddr > 169.254.255.255))";
const DIVERT_NO_LOCALNETS_V4_SRC: &str = "((ip.SrcAddr < 127.0.0.1 or ip.SrcAddr > 127.255.255.255) and (ip.SrcAddr < 10.0.0.0 or ip.SrcAddr > 10.255.255.255) and (ip.SrcAddr < 192.168.0.0 or ip.SrcAddr > 192.168.255.255) and (ip.SrcAddr < 172.16.0.0 or ip.SrcAddr > 172.31.255.255) and (ip.SrcAddr < 169.254.0.0 or ip.SrcAddr > 169.254.255.255))";
const DIVERT_NO_LOCALNETS_V6_DST: &str = "((ipv6.DstAddr > ::1) and (ipv6.DstAddr < 2001::0 or ipv6.DstAddr >= 2001:1::0) and (ipv6.DstAddr < fc00::0 or ipv6.DstAddr >= fe00::0) and (ipv6.DstAddr < fe80::0 or ipv6.DstAddr >= fec0::0) and (ipv6.DstAddr < ff00::0 or ipv6.DstAddr >= ffff::0))";
const DIVERT_NO_LOCALNETS_V6_SRC: &str = "((ipv6.SrcAddr > ::1) and (ipv6.SrcAddr < 2001::0 or ipv6.SrcAddr >= 2001:1::0) and (ipv6.SrcAddr < fc00::0 or ipv6.SrcAddr >= fe00::0) and (ipv6.SrcAddr < fe80::0 or ipv6.SrcAddr >= fec0::0) and (ipv6.SrcAddr < ff00::0 or ipv6.SrcAddr >= ffff::0))";
const DIVERT_TCP_NOT_EMPTY: &str = "(!tcp or tcp.Syn or tcp.Rst or tcp.Fin or tcp.PayloadLength>0)";
const DIVERT_TCP_INBOUNDS: &str = "(tcp.Ack and tcp.Syn or tcp.Rst or tcp.Fin)";
const DIVERT_PROLOG: &str = "!impostor and !loopback";

fn wf_make_pf(opt: &str, l4: &str, portname: &str) -> Result<String, String> {
    let mut parts = Vec::new();
    for p in opt.split(',') {
        let pf = pf_parse(p).ok_or_else(|| format!("bad value for wf port filter: {p}"))?;
        let s = if pf.from == pf.to {
            format!(
                "({l4}.{portname} {} {})",
                if pf.neg { "!=" } else { "==" },
                pf.from
            )
        } else {
            format!(
                "({l4}.{portname} {} {} {} {l4}.{portname} {} {})",
                if pf.neg { "<" } else { ">=" },
                pf.from,
                if pf.neg { "or" } else { "and" },
                if pf.neg { ">" } else { "<=" },
                pf.to
            )
        };
        parts.push(s);
    }
    Ok(format!("({})", parts.join(" or ")))
}

pub fn make_filter(params: &Params) -> Result<String, String> {
    let wf = &params.wf;
    if let Some(raw) = &wf.raw {
        return Ok(raw.clone());
    }
    let (tcp_src, tcp_dst, udp_src, udp_dst);
    if wf.tcp.is_empty() {
        tcp_src = String::new();
        tcp_dst = String::new();
    } else {
        tcp_src = wf_make_pf(&wf.tcp, "tcp", "SrcPort")?;
        tcp_dst = wf_make_pf(&wf.tcp, "tcp", "DstPort")?;
    }
    if wf.udp.is_empty() {
        udp_src = String::new();
        udp_dst = String::new();
    } else {
        udp_src = wf_make_pf(&wf.udp, "udp", "SrcPort")?;
        udp_dst = wf_make_pf(&wf.udp, "udp", "DstPort")?;
    }
    if tcp_src.is_empty() && udp_src.is_empty() && wf.raw_part.is_empty() {
        return Err("windivert filter: must specify port or/and partial raw filter".into());
    }
    let (ipv4, ipv6) = (wf.ipv4, wf.ipv6);
    let iface = if wf.iface.0 != 0 {
        format!(" ifIdx={} and subIfIdx={} and", wf.iface.0, wf.iface.1)
    } else {
        String::new()
    };
    let f_tcp_not_empty = if !tcp_src.is_empty() && !params.need_all_out() {
        format!("{DIVERT_TCP_NOT_EMPTY} and ")
    } else {
        String::new()
    };
    let f_tcpin = if !tcp_src.is_empty() {
        DIVERT_TCP_INBOUNDS
    } else {
        ""
    };

    let mut f = format!(
        "{DIVERT_PROLOG} and{iface}{}\n(",
        if ipv4 {
            if ipv6 { "" } else { " ip and" }
        } else {
            " ipv6 and"
        }
    );
    let mut n = 0;
    for part in &wf.raw_part {
        f.push_str(&format!(
            "{}(\n{part}\n )",
            if n > 0 { "\n or\n " } else { "\n " }
        ));
        n += 1;
    }
    if !tcp_src.is_empty() || !udp_src.is_empty() {
        let pf_dst = if !tcp_src.is_empty() && !udp_src.is_empty() {
            format!("({tcp_dst} or {udp_dst})")
        } else if !tcp_dst.is_empty() {
            tcp_dst.clone()
        } else {
            udp_dst.clone()
        };
        f.push_str(if n > 0 { "\n or\n " } else { "\n " });
        f.push_str(&format!(
            "(\n  (outbound and {f_tcp_not_empty}{pf_dst})\n  or\n  (inbound and tcp{}{}{}{}{})\n )",
            if tcp_src.is_empty() { " and false" } else { "" },
            if f_tcpin.is_empty() { "" } else { " and " },
            f_tcpin,
            if tcp_src.is_empty() { "" } else { " and " },
            tcp_src
        ));
    }
    f.push_str("\n)");
    if wf.filter_lan {
        let (d, s) = if ipv4 {
            if ipv6 {
                (
                    format!("({DIVERT_NO_LOCALNETS_V4_DST} or {DIVERT_NO_LOCALNETS_V6_DST})"),
                    format!("({DIVERT_NO_LOCALNETS_V4_SRC} or {DIVERT_NO_LOCALNETS_V6_SRC})"),
                )
            } else {
                (
                    DIVERT_NO_LOCALNETS_V4_DST.to_string(),
                    DIVERT_NO_LOCALNETS_V4_SRC.to_string(),
                )
            }
        } else {
            (
                DIVERT_NO_LOCALNETS_V6_DST.to_string(),
                DIVERT_NO_LOCALNETS_V6_SRC.to_string(),
            )
        };
        f.push_str(&format!(
            "\nand\n(\n outbound and {d}\n or\n inbound and {s}\n)\n"
        ));
    }
    Ok(f)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Addr {
    timestamp: i64,
    bits: u32,
    reserved2: u32,
    union: [u8; 64],
}

impl Addr {
    const OUTBOUND: u32 = 1 << 17;
    const LOOPBACK: u32 = 1 << 18;
    const IMPOSTOR: u32 = 1 << 19;
    const IPV6: u32 = 1 << 20;
    const IP_CSUM: u32 = 1 << 21;
    const TCP_CSUM: u32 = 1 << 22;
    const UDP_CSUM: u32 = 1 << 23;

    fn zero() -> Addr {
        Addr {
            timestamp: 0,
            bits: 0,
            reserved2: 0,
            union: [0; 64],
        }
    }
    fn if_idx(&self) -> (u32, u32) {
        (
            u32::from_le_bytes(self.union[0..4].try_into().unwrap()),
            u32::from_le_bytes(self.union[4..8].try_into().unwrap()),
        )
    }
    fn set_if_idx(&mut self, i: u32, s: u32) {
        self.union[0..4].copy_from_slice(&i.to_le_bytes());
        self.union[4..8].copy_from_slice(&s.to_le_bytes());
    }
}

type Handle = *mut c_void;
type OpenFn = unsafe extern "C" fn(*const i8, i32, i16, u64) -> Handle;
type RecvFn = unsafe extern "C" fn(Handle, *mut c_void, u32, *mut u32, *mut Addr) -> i32;
type SendFn = unsafe extern "C" fn(Handle, *const c_void, u32, *mut u32, *const Addr) -> i32;
type ShutdownFn = unsafe extern "C" fn(Handle, i32) -> i32;
type CloseFn = unsafe extern "C" fn(Handle) -> i32;

struct Api {
    _lib: Library,
    open: OpenFn,
    recv: RecvFn,
    send: SendFn,
    shutdown: ShutdownFn,
    close: CloseFn,
}

#[derive(Clone, Copy)]
struct SendHandle(Handle);
impl SendHandle {
    fn get(self) -> Handle {
        self.0
    }
}
// WinDivert handles may be used concurrently from several threads (recv/send/shutdown are thread-safe).
unsafe impl Send for SendHandle {}
unsafe impl Sync for SendHandle {}

pub struct Divert {
    api: Arc<Api>,
    h: SendHandle,
}

unsafe impl Send for Divert {}

impl Divert {
    pub fn open(bin_dir: &Path, filter: &str) -> Result<Divert, String> {
        let dll = bin_dir.join("WinDivert.dll");
        let lib = unsafe { Library::new(&dll) }
            .map_err(|e| format!("Не удалось загрузить {}: {e}", dll.display()))?;
        let api = unsafe {
            let open: OpenFn = *lib
                .get::<OpenFn>(b"WinDivertOpen\0")
                .map_err(|e| e.to_string())?;
            let recv: RecvFn = *lib
                .get::<RecvFn>(b"WinDivertRecv\0")
                .map_err(|e| e.to_string())?;
            let send: SendFn = *lib
                .get::<SendFn>(b"WinDivertSend\0")
                .map_err(|e| e.to_string())?;
            let shutdown: ShutdownFn = *lib
                .get::<ShutdownFn>(b"WinDivertShutdown\0")
                .map_err(|e| e.to_string())?;
            let close: CloseFn = *lib
                .get::<CloseFn>(b"WinDivertClose\0")
                .map_err(|e| e.to_string())?;
            Api {
                _lib: lib,
                open,
                recv,
                send,
                shutdown,
                close,
            }
        };
        let cf = CString::new(filter).map_err(|e| e.to_string())?;
        let h = unsafe { (api.open)(cf.as_ptr(), 0, 0, 0) };
        if h as isize == -1 || h.is_null() {
            let err = unsafe { GetLastError() };
            let hint = match err {
                2 | 3 => " (не найден WinDivert64.sys рядом с WinDivert.dll)",
                5 => " (нужны права администратора)",
                87 => " (неверный фильтр)",
                577 => {
                    " (драйвер не прошёл проверку подписи: отключите Secure Boot или установите обновления Windows)"
                }
                1275 => " (драйвер заблокирован системой)",
                1058 | 1060 => " (драйвер отключён)",
                _ => "",
            };
            return Err(format!(
                "windivert: ошибка открытия фильтра, код {err}{hint}"
            ));
        }
        Ok(Divert {
            api: Arc::new(api),
            h: SendHandle(h),
        })
    }

    pub fn shutdown_handle(&self) -> impl Fn() + Send + Sync + 'static {
        let api = self.api.clone();
        let h = self.h;
        move || unsafe {
            (api.shutdown)(h.get(), 3);
        }
    }

    pub fn run(&mut self, engines: Vec<Engine>, stop: &AtomicBool) -> Result<(), String> {
        let (api, h) = (self.api.clone(), self.h);
        let pool: Pool<(Addr, Vec<u8>)> = Pool::new(engines, |_| {
            let api = api.clone();
            move |engine: &mut Engine, (addr, mut packet): (Addr, Vec<u8>)| {
                let (idx, sub) = addr.if_idx();
                let iface = Iface { idx, sub };
                let verdict = {
                    let mut io = WinIo { api: &api, h };
                    engine.process(&mut io, 0, iface, iface, &mut packet)
                };
                if matches!(verdict & VERDICT_MASK, VERDICT_PASS | VERDICT_MODIFY) {
                    let mut sl = 0u32;
                    let r = unsafe {
                        (api.send)(
                            h.0,
                            packet.as_ptr().cast(),
                            packet.len() as u32,
                            &mut sl,
                            &addr,
                        )
                    };
                    if r == 0 {
                        crate::dlog!("windivert: reinject failed, error {}", unsafe {
                            GetLastError()
                        });
                    }
                }
            }
        })?;
        let mut buf = vec![0u8; 16384];
        let mut failures = 0u32;
        loop {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let mut addr = Addr::zero();
            let mut rlen = 0u32;
            let ok = unsafe {
                (self.api.recv)(
                    self.h.0,
                    buf.as_mut_ptr().cast(),
                    buf.len() as u32,
                    &mut rlen,
                    &mut addr,
                )
            };
            if ok == 0 {
                let err = unsafe { GetLastError() };
                match err {
                    122 => continue, // insufficient buffer
                    232 => break,    // no data: shut down
                    _ => {
                        crate::dlog!("windivert: recv failed, error {err}");
                        failures += 1;
                        if stop.load(Ordering::SeqCst) || failures >= 100 {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        continue;
                    }
                }
            }
            failures = 0;
            let data = &buf[..rlen as usize];
            if addr.bits & (Addr::IMPOSTOR | Addr::LOOPBACK) != 0 {
                let mut sl = 0u32;
                unsafe {
                    (self.api.send)(self.h.0, data.as_ptr().cast(), rlen, &mut sl, &addr);
                }
            } else {
                pool.dispatch(flow_hash(data), (addr, data.to_vec()));
            }
        }
        drop(pool);
        Ok(())
    }
}

impl Drop for Divert {
    fn drop(&mut self) {
        unsafe {
            (self.api.close)(self.h.0);
        }
    }
}

struct WinIo<'a> {
    api: &'a Api,
    h: SendHandle,
}

impl Io for WinIo<'_> {
    fn send(&mut self, dst: &Endpoint, _fwmark: u32, ifout: &Iface, data: &[u8]) -> bool {
        let mut wa = Addr::zero();
        wa.set_if_idx(ifout.idx, ifout.sub);
        wa.bits = Addr::OUTBOUND
            | Addr::IP_CSUM
            | Addr::TCP_CSUM
            | Addr::UDP_CSUM
            | if dst.ip.is_ipv6() { Addr::IPV6 } else { 0 };
        let mut sl = 0u32;
        unsafe {
            (self.api.send)(
                self.h.0,
                data.as_ptr().cast(),
                data.len() as u32,
                &mut sl,
                &wa,
            ) != 0
        }
    }
}
