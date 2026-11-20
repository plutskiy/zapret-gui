use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

use super::args;
use super::conntrack::Iface;
use super::desync::*;
use super::packet::*;

struct Mock {
    sent: Vec<Vec<u8>>,
}

impl Io for Mock {
    fn send(&mut self, _dst: &Endpoint, _fwmark: u32, _ifout: &Iface, data: &[u8]) -> bool {
        self.sent.push(data.to_vec());
        true
    }
}

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

fn config_args(name: &str) -> Vec<String> {
    let data = data_dir();
    let spec =
        crate::zapret::build_launch(&data, &data.join("configs").join(format!("{name}.conf")))
            .unwrap();
    crate::zapret::tokenize(&spec.windows_args)
}

const CLIENT: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 10);

fn tcp_pkt(
    server: Ipv4Addr,
    sport: u16,
    dport: u16,
    flags: u16,
    seq: u32,
    ack: u32,
    data: &[u8],
) -> Vec<u8> {
    prepare_tcp_segment(
        &Endpoint {
            ip: IpAddr::V4(CLIENT),
            port: sport,
        },
        &Endpoint {
            ip: IpAddr::V4(server),
            port: dport,
        },
        &TcpBuild {
            flags,
            sack: true,
            mss: 1460,
            seq,
            ack,
            win: 64240,
            scale: 8,
            ts: Some([0, 0, 1, 0, 0, 0, 0, 0]),
            df: true,
            ttl: 128,
            tos: 0,
            ip_id: 1000,
            flow: 0,
            fooling: 0,
            ts_inc: 0,
            badseq_inc: 0,
            badseq_ack_inc: 0,
            data,
        },
        9400,
    )
    .unwrap()
}

fn udp_pkt(server: Ipv4Addr, sport: u16, dport: u16, data: &[u8]) -> Vec<u8> {
    prepare_udp_segment(
        &Endpoint {
            ip: IpAddr::V4(CLIENT),
            port: sport,
        },
        &Endpoint {
            ip: IpAddr::V4(server),
            port: dport,
        },
        &UdpBuild {
            df: false,
            ttl: 128,
            tos: 0,
            ip_id: 2000,
            flow: 0,
            fooling: 0,
            padding: None,
            padlen: 0,
            data,
        },
        9400,
    )
    .unwrap()
}

fn checksum_ok(p: &Pkt) -> bool {
    let off = p.l3_len;
    let c = l4_checksum(&p.src_ip(), &p.dst_ip(), p.proto, &p.buf[off..]);
    c == 0 || (p.udp && c == 0xFFFF)
}

struct Outcome {
    verdict: u8,
    sent: Vec<Vec<u8>>,
}

fn run_tls(cfg: &str, server: Ipv4Addr, hello: &[u8]) -> (Outcome, u32) {
    let params = args::parse(&config_args(cfg)).unwrap_or_else(|e| panic!("{cfg}: {e}"));
    let mut eng = Engine::new(params);
    let mut io = Mock { sent: Vec::new() };
    let isn = 1_000_000u32;
    let mut syn = tcp_pkt(server, 50000, 443, TH_SYN, isn, 0, &[]);
    let iface = Iface { idx: 1, sub: 0 };
    eng.process(&mut io, 0, iface, iface, &mut syn);
    let mut ack = tcp_pkt(server, 50000, 443, TH_ACK, isn + 1, 5000, &[]);
    eng.process(&mut io, 0, iface, iface, &mut ack);
    io.sent.clear();
    let mut data = tcp_pkt(server, 50000, 443, TH_ACK | TH_PUSH, isn + 1, 5000, hello);
    let verdict = eng.process(&mut io, 0, iface, iface, &mut data);
    if verdict & VERDICT_MASK != VERDICT_DROP {
        io.sent.push(data);
    }
    (
        Outcome {
            verdict,
            sent: io.sent,
        },
        isn + 1,
    )
}

fn all_configs() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(data_dir().join("configs"))
        .unwrap()
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            (n.starts_with("general") && n.ends_with(".conf")).then(|| n[..n.len() - 5].to_string())
        })
        .collect();
    v.sort();
    v
}

#[test]
fn every_real_config_parses() {
    let cfgs = all_configs();
    assert!(cfgs.len() >= 20);
    for c in &cfgs {
        let p = args::parse(&config_args(c)).unwrap_or_else(|e| panic!("{c}: {e}"));
        assert!(p.profiles.len() >= 5, "{c}: {} profiles", p.profiles.len());
        assert!(!p.wf.tcp.is_empty() && !p.wf.udp.is_empty(), "{c}");
        #[cfg(windows)]
        {
            let f = super::windows::make_filter(&p).unwrap_or_else(|e| panic!("{c}: {e}"));
            assert!(f.contains("outbound") && f.contains("inbound"), "{c}");
        }
    }
}

#[test]
fn windivert_filter_shape() {
    let p = args::parse(&config_args("general")).unwrap();
    let f = super::windows::make_filter(&p).unwrap();
    println!("{f}");
    assert!(f.starts_with("!impostor and !loopback and"));
    assert!(f.contains("tcp.DstPort == 443"));
    assert!(
        f.contains("udp.DstPort >= 19294 and udp.DstPort <= 19344")
            || f.contains("(udp.DstPort >= 19294 and udp.DstPort <= 19344)")
    );
}

#[test]
fn tls_hello_every_config_keeps_data_and_checksums() {
    let hello = std::fs::read(
        data_dir()
            .join("bin")
            .join("tls_clienthello_www_google_com.bin"),
    )
    .unwrap();
    assert!(crate::engine::proto::is_tls_client_hello(&hello, false));
    let server = Ipv4Addr::new(142, 250, 74, 46);
    for cfg in all_configs() {
        let (out, isn) = run_tls(&cfg, server, &hello);
        assert!(!out.sent.is_empty(), "{cfg}: nothing sent");
        let mut covered = vec![false; hello.len()];
        let mut fakes = 0;
        for raw in &out.sent {
            let p = Pkt::dissect(raw.clone()).unwrap_or_else(|| panic!("{cfg}: bad packet"));
            assert!(p.tcp, "{cfg}");
            let badsum = !checksum_ok(&p);
            let ip_ok = ip4_header_checksum(&p.buf[..p.l3_len]) == 0;
            assert!(ip_ok, "{cfg}: bad ip checksum");
            let pl = p.payload();
            let rel = p.seq().wrapping_sub(isn) as i64;
            let is_real = !badsum
                && p.ttl() == 128
                && rel >= 0
                && (rel as usize) < hello.len()
                && pl.len() <= hello.len() - rel as usize
                && pl == &hello[rel as usize..rel as usize + pl.len()]
                && p.tcp_timestamps() == Some([0, 0, 1, 0, 0, 0, 0, 0]);
            if is_real {
                for i in 0..pl.len() {
                    covered[rel as usize + i] = true;
                }
            } else {
                fakes += 1;
            }
        }
        let missing = covered.iter().filter(|c| !**c).count();
        println!(
            "{cfg}: verdict={} sent={} fakes_or_seqovl={} uncovered={}",
            out.verdict,
            out.sent.len(),
            fakes,
            missing
        );
        assert_eq!(missing, 0, "{cfg}: original data not fully delivered");
    }
}

#[test]
fn quic_initial_gets_fake_and_original() {
    let quic = std::fs::read(
        data_dir()
            .join("bin")
            .join("quic_initial_www_google_com.bin"),
    )
    .unwrap();
    let server = Ipv4Addr::new(142, 250, 74, 46);
    for cfg in ["general", "general (ALT)", "general (SIMPLE FAKE)"] {
        let params = args::parse(&config_args(cfg)).unwrap();
        let mut eng = Engine::new(params);
        let mut io = Mock { sent: Vec::new() };
        let iface = Iface { idx: 1, sub: 0 };
        let mut pkt = udp_pkt(server, 50001, 443, &quic);
        let v = eng.process(&mut io, 0, iface, iface, &mut pkt);
        println!("{cfg}: quic verdict={v} sent={}", io.sent.len());
        for s in &io.sent {
            let p = Pkt::dissect(s.clone()).unwrap();
            assert!(p.udp && checksum_ok(&p));
        }
        let orig_sent = io
            .sent
            .iter()
            .any(|s| Pkt::dissect(s.clone()).unwrap().payload() == &quic[..])
            || v & VERDICT_MASK != VERDICT_DROP;
        assert!(orig_sent, "{cfg}: original quic initial lost");
    }
}

#[test]
fn unlisted_host_is_untouched() {
    let hello = std::fs::read(data_dir().join("bin").join("tls_clienthello_4pda_to.bin")).unwrap();
    let host = crate::engine::proto::tls_hello_extract_host(&hello, true).unwrap();
    println!("host in file: {host}");
    let (out, _) = run_tls("general", Ipv4Addr::new(203, 0, 113, 77), &hello);
    println!("verdict={} sent={}", out.verdict, out.sent.len());
}

fn hello_for(sni: &str) -> Vec<u8> {
    let a = vec![
        "--dpi-desync=fake".to_string(),
        "--dpi-desync-fake-tls=!".to_string(),
        format!("--dpi-desync-fake-tls-mod=sni={sni}"),
    ];
    let p = args::parse(&a).unwrap();
    p.profiles[0].fake_tls[0].data.clone()
}

#[test]
fn listed_hosts_are_desynced_in_every_config() {
    let server = Ipv4Addr::new(142, 250, 74, 46);
    for sni in ["youtube.com", "discord.com", "www.youtube.com"] {
        let hello = hello_for(sni);
        assert_eq!(
            crate::engine::proto::tls_hello_extract_host(&hello, false).as_deref(),
            Some(sni)
        );
        for cfg in all_configs() {
            let (out, isn) = run_tls(&cfg, server, &hello);
            let mut covered = vec![false; hello.len()];
            let (mut fakes, mut real) = (0, 0);
            for raw in &out.sent {
                let p = Pkt::dissect(raw.clone()).unwrap_or_else(|| panic!("{cfg}: bad packet"));
                assert!(p.tcp, "{cfg}");
                assert_eq!(
                    ip4_header_checksum(&p.buf[..p.l3_len]),
                    0,
                    "{cfg}: bad ip checksum"
                );
                assert_eq!(p.win(), 64240, "{cfg}: tcp window lost");
                assert!(
                    p.ack() == 5000
                        || p.ack() == 5000u32.wrapping_sub(66000)
                        || p.tcp_flags() & TH_ACK == 0,
                    "{cfg}: bad ack {}",
                    p.ack()
                );
                let badsum = !checksum_ok(&p);
                let pl = p.payload();
                let rel0 = p.seq().wrapping_sub(isn) as i32 as i64;
                let skip = if rel0 < 0 { (-rel0) as usize } else { 0 };
                let rel = rel0 + skip as i64;
                let pl = if skip <= pl.len() {
                    &pl[skip..]
                } else {
                    &pl[..0]
                };
                let is_real = !badsum
                    && p.ttl() == 128
                    && !pl.is_empty()
                    && (rel as usize) < hello.len()
                    && pl.len() <= hello.len() - rel as usize
                    && pl == &hello[rel as usize..rel as usize + pl.len()]
                    && p.tcp_timestamps() == Some([0, 0, 1, 0, 0, 0, 0, 0]);
                if is_real {
                    real += 1;
                    for i in 0..pl.len() {
                        covered[rel as usize + i] = true;
                    }
                } else {
                    fakes += 1;
                }
            }
            let missing = covered.iter().filter(|c| !**c).count();
            println!(
                "{sni} {cfg}: verdict={} packets={} real={} fake/ovl={} uncovered={}",
                out.verdict,
                out.sent.len(),
                real,
                fakes,
                missing
            );
            assert_eq!(missing, 0, "{cfg}/{sni}: original data not fully delivered");
            assert!(
                out.sent.len() >= 2,
                "{cfg}/{sni}: expected desync (>=2 packets), got {}",
                out.sent.len()
            );
        }
    }
}

#[cfg(windows)]
#[test]
fn windivert_dll_loads_and_open_reports_error_or_succeeds() {
    let p = args::parse(&config_args("general")).unwrap();
    let f = super::windows::make_filter(&p).unwrap();
    match super::windows::Divert::open(&data_dir().join("bin"), &f) {
        Ok(_) => println!("windivert opened (running elevated)"),
        Err(e) => {
            println!("windivert open error: {e}");
            assert!(e.contains("код"), "unexpected failure kind: {e}");
        }
    }
}
