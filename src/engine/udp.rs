use super::args::*;
use super::conntrack::*;
use super::desync::*;
use super::packet::*;
use super::proto::*;
use super::quic::*;
use super::tcp::reasm_orig_cancel;

fn send_delayed(io: &mut dyn Io, ct: &mut Ctrack) {
    for d in std::mem::take(&mut ct.delayed) {
        io.send(&d.dst, d.fwmark, &d.ifout, &d.packet);
    }
}

fn quic_reasm_cancel(io: &mut dyn Io, ct: &mut Option<&mut Ctrack>, dp_any: bool) -> bool {
    reasm_orig_cancel(io, ct.as_deref_mut());
    dp_any
}

fn udp_orig_send(
    verdict: u8,
    tx: &mut Tx,
    dp: &Profile,
    ct: Option<&Ctrack>,
    pkt: &mut Pkt,
    force: bool,
) -> bool {
    if dp.dup_repeats == 0 && !force {
        return false;
    }
    let ttl_orig = pkt.ttl();
    verdict_csum_fix(verdict, pkt);
    if dp.dup_repeats != 0 && check_dup_interval(dp, ct) {
        let ttl_fake = match ct {
            Some(c) if c.dup_autottl != 0 => c.dup_autottl,
            _ => {
                if pkt.v6 {
                    if dp.dup_ttl6 != 0 {
                        dp.dup_ttl6
                    } else {
                        ttl_orig
                    }
                } else if dp.dup_ttl != 0 {
                    dp.dup_ttl
                } else {
                    ttl_orig
                }
            }
        };
        if dp.dup_fooling_mode != 0 || (!pkt.v6 && dp.dup_ip_id_mode != IpIdMode::Same) {
            let mut ip_id = ip_id_fix(pkt, dp.ip_id_mode);
            let Some(mut seg) = prepare_udp_segment(
                &pkt.src(),
                &pkt.dst(),
                &UdpBuild {
                    df: pkt.df(),
                    ttl: ttl_fake,
                    tos: pkt.tos(),
                    ip_id,
                    flow: pkt.flow(),
                    fooling: dp.dup_fooling_mode,
                    padding: None,
                    padlen: 0,
                    data: pkt.payload(),
                },
                MAX_PKT,
            ) else {
                return false;
            };
            for _ in 0..dp.dup_repeats {
                if !tx.send(&seg) {
                    return false;
                }
                ip_id = ip_id_next(ip_id, dp.dup_ip_id_mode);
                if !pkt.v6 {
                    seg[4..6].copy_from_slice(&ip_id.to_be_bytes());
                    super::tcp::ip4_header_checksum_zeroed(&mut seg);
                }
            }
        } else {
            let o = pkt.ttl();
            pkt.set_ttl(ttl_fake);
            for _ in 0..dp.dup_repeats {
                if !tx.send(&pkt.buf) {
                    pkt.set_ttl(o);
                    return false;
                }
            }
            pkt.set_ttl(o);
        }
        if dp.dup_replace {
            return true;
        }
        return tx.send(&pkt.buf);
    }
    if force {
        return tx.send(&pkt.buf);
    }
    false
}

impl Engine {
    #[allow(clippy::too_many_arguments)]
    pub fn udp_play(
        &mut self,
        io: &mut dyn Io,
        replay: bool,
        reasm_offset: usize,
        fwmark: u32,
        ifin: Iface,
        ifout: Iface,
        pkt: &mut Pkt,
        ct_replay: Option<&mut Ctrack>,
    ) -> u8 {
        let params = self.params.clone();
        let mut owned: Option<Ctrack> = None;
        let mut key = None;
        let mut reverse = false;
        if !replay && !params.ctrack_disable {
            let plen = pkt.payload().len();
            if let Some((k, rev)) = self.ct.feed(pkt, plen) {
                owned = self.ct.map.remove(&k);
                key = Some(k);
                reverse = rev;
            }
        }
        let mut ct: Option<&mut Ctrack> = if replay { ct_replay } else { owned.as_mut() };
        let mut verdict = VERDICT_PASS;
        let mut b_fake = false;
        let mut dp_sel: Option<usize> = None;
        let desync_fwmark = fwmark | params.desync_fwmark;
        let mut tx = Tx {
            io,
            dst: pkt.dst(),
            fwmark: desync_fwmark,
            ifout,
        };

        let next = self.udp_core(
            &params,
            &mut tx,
            replay,
            reverse,
            reasm_offset,
            ifin,
            pkt,
            &mut ct,
            &mut verdict,
            &mut b_fake,
            &mut dp_sel,
        );
        let v = match next {
            Next::Done(v) => v,
            _ => {
                let ct_nr = if replay { None } else { ct.as_deref() };
                if verdict & VERDICT_MASK == VERDICT_DROP {
                    ct_new_postnat_fix(ct_nr, pkt)
                } else if let Some(di) = dp_sel {
                    if udp_orig_send(
                        verdict,
                        &mut tx,
                        &params.profiles[di],
                        ct.as_deref(),
                        pkt,
                        b_fake,
                    ) {
                        ct_new_postnat_fix(ct_nr, pkt)
                    } else {
                        verdict
                    }
                } else {
                    verdict
                }
            }
        };
        drop(ct);
        if let (Some(k), Some(c)) = (key, owned) {
            self.ct.map.insert(k, c);
        }
        v
    }

    #[allow(clippy::too_many_arguments)]
    fn udp_core(
        &mut self,
        params: &Params,
        tx: &mut Tx,
        replay: bool,
        reverse: bool,
        reasm_offset: usize,
        ifin: Iface,
        pkt: &mut Pkt,
        ct: &mut Option<&mut Ctrack>,
        verdict: &mut u8,
        b_fake: &mut bool,
        dp_sel: &mut Option<usize>,
    ) -> Next {
        let src = pkt.src();
        let dst = pkt.dst();

        let mut dp_idx: Option<usize>;
        if replay {
            let Some(c) = ct.as_deref_mut() else {
                return Next::Done(VERDICT_PASS);
            };
            dp_idx = c.dp;
            if dp_idx.is_none() && !c.dp_search_complete {
                let m = self.dp_find(
                    false,
                    &dst,
                    c.hostname.as_deref(),
                    c.hostname_is_ip,
                    c.l7proto,
                );
                c.dp = m.dp;
                c.dp_search_complete = true;
                dp_idx = m.dp;
            }
            if dp_idx.is_none() {
                return Next::Done(VERDICT_PASS);
            }
        } else {
            dp_idx = ct.as_deref().and_then(|c| c.dp);
            if dp_idx.is_none() && !ct.as_deref().is_some_and(|c| c.dp_search_complete) {
                let (mut hostname, mut hip, l7) = match ct.as_deref() {
                    Some(c) => (c.hostname.clone(), c.hostname_is_ip, c.l7proto),
                    None => (None, false, L7::Unknown),
                };
                if ct.is_some() && hostname.is_none() && !reverse {
                    if let Some((h, is_ip)) = self.ipcache_get_hostname(dst.ip) {
                        if let Some(c) = ct.as_deref_mut() {
                            c.hostname = Some(h.clone());
                        }
                        hostname = Some(h);
                        hip = is_ip;
                    }
                }
                let m = self.dp_find(false, &dst, hostname.as_deref(), hip, l7);
                if let Some(c) = ct.as_deref_mut() {
                    c.dp = m.dp;
                    c.dp_search_complete = true;
                }
                dp_idx = m.dp;
            }
            let Some(di) = dp_idx else {
                return Next::Done(VERDICT_PASS);
            };
            *dp_sel = Some(di);
            let dp = &params.profiles[di];
            if let Some(c) = ct.as_deref_mut() {
                maybe_cutoff(c, dp, false);
            }
            if reverse {
                if let Some(c) = ct.as_deref_mut() {
                    if c.incoming_ttl == 0 {
                        c.incoming_ttl = pkt.ttl();
                        let mut cc = std::mem::replace(c, Ctrack::new());
                        self.autottl_rediscover(&mut cc, dp, src.ip, ifin.idx);
                        *c = cc;
                    }
                }
                return Next::Done(*verdict);
            }
            if let Some(c) = ct.as_deref_mut() {
                let mut cc = std::mem::replace(c, Ctrack::new());
                self.autottl_discover(&mut cc, dp, dst.ip, tx.ifout.idx);
                *c = cc;
            }
            if orig_mod(dp, ct.as_deref(), pkt) {
                *verdict = VERDICT_MODIFY;
            }
            if !check_desync_interval(dp, ct.as_deref()) {
                return Next::SendOrig;
            }
        }

        let mut dp_idx = dp_idx.unwrap();
        *dp_sel = Some(dp_idx);
        let mut dp = &params.profiles[dp_idx];
        let df = pkt.df();

        if pkt.payload().is_empty() {
            return Next::SendOrig;
        }

        let mut ip_id = 0u16;
        if replay {
            if let Some(c) = ct.as_deref() {
                ip_id = c.ip_id;
            }
        }
        if ip_id == 0 {
            ip_id = ip_id_fix(pkt, dp.ip_id_mode);
        }

        let mut l7 = L7::Unknown;
        let mut host: Option<String> = None;
        let mut have_host = false;
        let payload: Vec<u8> = pkt.payload().to_vec();

        if is_quic_initial(&payload) {
            l7 = L7::Quic;
            if let Some(c) = ct.as_deref_mut() {
                if c.l7proto == L7::Unknown {
                    c.l7proto = l7;
                }
            }
            let mut clean: Option<Vec<u8>> = if replay {
                ct.as_deref().map(|c| c.reasm_orig.data().to_vec())
            } else {
                quic_decrypt_initial(&payload, 16384)
            };
            let any = dp.desync_any_proto;
            if let Some(mut cl) = clean.take() {
                if !replay {
                    if let Some(c) = ct.as_deref_mut() {
                        if !c.reasm_orig.is_empty() {
                            if c.reasm_orig.has_space(cl.len()) {
                                let seq = c.reasm_orig.size_present as u32;
                                c.reasm_orig.feed(seq, &cl);
                                cl = c.reasm_orig.data().to_vec();
                            } else {
                                reasm_orig_cancel(tx.io, Some(c));
                                return Next::SendOrig;
                            }
                        }
                    }
                }
                if let Some((defrag, full)) = quic_defrag_crypto(&cl, UDP_MAX_REASM) {
                    if full {
                        let hello = is_quic_crypto_hello(&defrag);
                        let req_full =
                            hello.is_some_and(|(o, l)| is_tls_handshake_full(&defrag[o..o + l]));
                        if let Some(c) = ct.as_deref_mut() {
                            if !replay {
                                if hello.is_some() && !req_full && c.reasm_orig.is_empty() {
                                    c.reasm_orig.init(UDP_MAX_REASM, 0);
                                    c.reasm_orig.feed(0, &cl);
                                }
                                if !c.reasm_orig.is_empty() {
                                    verdict_csum_fix(*verdict, pkt);
                                    c.delayed.push(Delayed {
                                        dst,
                                        fwmark: tx.fwmark,
                                        ifin,
                                        ifout: tx.ifout,
                                        packet: pkt.buf.clone(),
                                        len_payload: payload.len(),
                                    });
                                    if req_full {
                                        let q = std::mem::take(&mut c.delayed);
                                        self.replay_queue(tx.io, q, c);
                                        c.reasm_orig.clear();
                                        send_delayed(tx.io, c);
                                    }
                                    return Next::Done(ct_new_postnat_fix(Some(&*c), pkt));
                                }
                            }
                        }
                        if let Some((o, l)) = hello {
                            host = tls_hello_extract_host_from_handshake(
                                &defrag[o..o + l],
                                TLS_PARTIALS_ENABLE,
                            );
                            have_host = host.is_some();
                            if !have_host && dp.desync_skip_nosni {
                                reasm_orig_cancel(tx.io, ct.as_deref_mut());
                                return Next::SendOrig;
                            }
                        } else if !quic_reasm_cancel(tx.io, ct, any) {
                            return Next::SendOrig;
                        }
                    } else if let Some(c) = ct.as_deref_mut() {
                        if !replay {
                            if c.reasm_orig.is_empty() {
                                c.reasm_orig.init(UDP_MAX_REASM, 0);
                                c.reasm_orig.feed(0, &cl);
                            }
                            verdict_csum_fix(*verdict, pkt);
                            c.delayed.push(Delayed {
                                dst,
                                fwmark: tx.fwmark,
                                ifin,
                                ifout: tx.ifout,
                                packet: pkt.buf.clone(),
                                len_payload: payload.len(),
                            });
                            return Next::Done(ct_new_postnat_fix(Some(&*c), pkt));
                        }
                    } else if !quic_reasm_cancel(tx.io, ct, any) {
                        return Next::SendOrig;
                    }
                } else if !quic_reasm_cancel(tx.io, ct, any) {
                    return Next::SendOrig;
                }
            } else if !quic_reasm_cancel(tx.io, ct, any) {
                return Next::SendOrig;
            }
        } else {
            reasm_orig_cancel(tx.io, ct.as_deref_mut());
            if is_wireguard_handshake_initiation(&payload) {
                l7 = L7::Wireguard;
            } else if is_dht_d1(&payload) {
                l7 = L7::Dht;
            } else if is_discord_ip_discovery_request(&payload) {
                l7 = L7::Discord;
            } else if is_stun_message(&payload) {
                l7 = L7::Stun;
            } else if !dp.desync_any_proto {
                return Next::SendOrig;
            }
            if l7 != L7::Unknown {
                if let Some(c) = ct.as_deref_mut() {
                    if c.l7proto == L7::Unknown {
                        c.l7proto = l7;
                    }
                }
            }
        }

        let mut host_is_ip = false;
        if let Some(h) = host.as_mut() {
            host_is_ip = strip_host_to_ip(h);
        }

        let discovered_l7 = match ct.as_deref_mut() {
            Some(c) => {
                let d = !c.l7proto_discovered && c.l7proto != L7::Unknown;
                if d {
                    c.l7proto_discovered = true;
                }
                d
            }
            None => l7 != L7::Unknown,
        };
        let discovered_host = have_host && !ct.as_deref().is_some_and(|c| c.hostname_discovered);
        if discovered_host {
            if let Some(c) = ct.as_deref_mut() {
                c.hostname_discovered = true;
                c.hostname = host.clone();
                c.hostname_is_ip = host_is_ip;
                self.ipcache_put_hostname(dst.ip, host.as_deref().unwrap_or(""), host_is_ip);
            }
        }

        let (mut check_done, mut check_result) = (false, false);
        if discovered_l7 || discovered_host {
            let prev = dp_idx;
            let (hn, hip, l7x) = match ct.as_deref() {
                Some(c) => (c.hostname.clone(), c.hostname_is_ip, c.l7proto),
                None => (host.clone(), host_is_ip, l7),
            };
            let m = self.dp_find(false, &dst, hn.as_deref(), hip, l7x);
            check_done = m.check_done;
            check_result = m.check_result;
            if let Some(c) = ct.as_deref_mut() {
                c.dp = m.dp;
                c.dp_search_complete = true;
                c.b_check_done = m.check_done;
                c.b_check_result = m.check_result;
                c.b_check_excluded = m.excluded;
            }
            let Some(ndp) = m.dp else {
                reasm_orig_cancel(tx.io, ct.as_deref_mut());
                return Next::SendOrig;
            };
            dp_idx = ndp;
            dp = &params.profiles[ndp];
            *dp_sel = Some(ndp);
            if ndp != prev {
                if let Some(c) = ct.as_deref_mut() {
                    let mut cc = std::mem::replace(c, Ctrack::new());
                    self.autottl_rediscover(&mut cc, dp, dst.ip, tx.ifout.idx);
                    *c = cc;
                }
                ip_id = ip_id_fix(pkt, dp.ip_id_mode);
                if replay {
                    if orig_mod(dp, ct.as_deref(), pkt) {
                        *verdict = VERDICT_MODIFY;
                    }
                } else {
                    if let Some(c) = ct.as_deref_mut() {
                        maybe_cutoff(c, dp, false);
                    }
                    if orig_mod(dp, ct.as_deref(), pkt) {
                        *verdict = VERDICT_MODIFY;
                    }
                    if !check_desync_interval(dp, ct.as_deref()) {
                        return Next::SendOrig;
                    }
                }
            }
        } else if let Some(c) = ct.as_deref() {
            check_done = c.b_check_done;
            check_result = c.b_check_result;
        }
        let _ = dp_idx;

        let lists_empty = self.lists.hostlists_empty(&dp.hl_include)
            && self.lists.hostlists_empty(&dp.hl_exclude);
        if have_host && !lists_empty {
            if !check_done {
                check_result = self
                    .lists
                    .host_check(
                        &dp.hl_include,
                        &dp.hl_exclude,
                        host.as_deref().unwrap_or(""),
                        host_is_ip,
                    )
                    .0;
            }
            if !check_result {
                return Next::SendOrig;
            }
        }

        let fake: &[Blob] = match l7 {
            L7::Quic => &dp.fake_quic,
            L7::Wireguard => &dp.fake_wg,
            L7::Dht => &dp.fake_dht,
            L7::Discord => &dp.fake_discord,
            L7::Stun => &dp.fake_stun,
            _ => &dp.fake_unknown_udp,
        };

        let ttl_orig = pkt.ttl();
        let ttl_fake = match ct.as_deref() {
            Some(c) if c.desync_autottl != 0 => c.desync_autottl,
            _ => {
                if pkt.v6 {
                    if dp.desync_ttl6 != 0 {
                        dp.desync_ttl6
                    } else {
                        ttl_orig
                    }
                } else if dp.desync_ttl != 0 {
                    dp.desync_ttl
                } else {
                    ttl_orig
                }
            }
        };

        match dp.desync_mode {
            Mode::FakeKnown | Mode::Fake => {
                if dp.desync_mode == Mode::FakeKnown && l7 == L7::Unknown {
                } else if reasm_offset == 0 {
                    for item in fake {
                        let data = &item.data[item.offset.min(item.data.len())..];
                        let Some(s) = prepare_udp_segment(
                            &src,
                            &dst,
                            &UdpBuild {
                                df,
                                ttl: ttl_fake,
                                tos: pkt.tos(),
                                ip_id,
                                flow: pkt.flow(),
                                fooling: dp.desync_fooling_mode,
                                padding: None,
                                padlen: 0,
                                data,
                            },
                            MAX_PKT,
                        ) else {
                            return Next::SendOrig;
                        };
                        if !tx.send_rep(dp.desync_repeats, &s) {
                            return Next::SendOrig;
                        }
                        ip_id = ip_id_next(ip_id, dp.ip_id_mode);
                    }
                    *b_fake = true;
                }
            }
            _ => {}
        }

        let mode = if dp.desync_mode2 == Mode::None {
            dp.desync_mode
        } else {
            dp.desync_mode2
        };
        match mode {
            Mode::Udplen => {
                let Some(s) = prepare_udp_segment(
                    &src,
                    &dst,
                    &UdpBuild {
                        df,
                        ttl: ttl_orig,
                        tos: pkt.tos(),
                        ip_id,
                        flow: pkt.flow(),
                        fooling: 0,
                        padding: Some(&dp.udplen_pattern),
                        padlen: dp.udplen_increment,
                        data: &payload,
                    },
                    MAX_PKT,
                ) else {
                    return Next::SendOrig;
                };
                if !tx.send(&s) {
                    return Next::SendOrig;
                }
                Next::Done(ct_new_postnat_fix(ct.as_deref(), pkt))
            }
            Mode::Tamper => {
                if is_dht_d1(&payload) {
                    let mut p2 = b"d2:001:x".to_vec();
                    p2.extend_from_slice(&payload[1..]);
                    let Some(s) = prepare_udp_segment(
                        &src,
                        &dst,
                        &UdpBuild {
                            df,
                            ttl: ttl_orig,
                            tos: pkt.tos(),
                            ip_id,
                            flow: pkt.flow(),
                            fooling: 0,
                            padding: None,
                            padlen: 0,
                            data: &p2,
                        },
                        MAX_PKT,
                    ) else {
                        return Next::SendOrig;
                    };
                    if !tx.send(&s) {
                        return Next::SendOrig;
                    }
                    return Next::Done(ct_new_postnat_fix(ct.as_deref(), pkt));
                }
                Next::SendOrig
            }
            _ => Next::SendOrig,
        }
    }
}
