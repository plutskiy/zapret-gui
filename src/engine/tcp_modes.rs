use super::args::*;
use super::conntrack::*;
use super::desync::*;
use super::packet::*;
use super::proto::*;
use super::rng;
use super::tcp::*;

fn is_any(dp: &Profile, m: Mode) -> bool {
    dp.desync_mode == m || dp.desync_mode2 == m
}

impl Engine {
    #[allow(clippy::too_many_arguments)]
    pub fn tcp_desync_modes(
        &mut self,
        _params: &Params,
        tx: &mut Tx,
        replay: bool,
        reasm_offset: usize,
        pkt: &mut Pkt,
        ct: &mut Option<&mut Ctrack>,
        st: &mut St,
        dp: &Profile,
        base: &Base,
        rdata: &[u8],
        l7: L7,
        ttl_orig: u8,
        ttl_fake: u8,
        _sack: bool,
        _mss: u16,
        mut ip_id: u16,
    ) -> Next {
        let fake_list: &[Blob] = match l7 {
            L7::Http => &dp.fake_http,
            L7::Tls => &dp.fake_tls,
            _ => &dp.fake_unknown,
        };
        let plen = pkt.payload().len();
        let incs = desync_incs(dp);
        let flags_orig = pkt.tcp_flags();
        let mut flags_fake = flags_orig;
        rewrite_tcp_flags(&mut flags_fake, dp.desync_tcp_flags_unset, dp.desync_tcp_flags_set);
        let seq = pkt.seq();
        let mode_id = dp.ip_id_mode;

        let mut split_pos = 0usize;
        let mut multi: Vec<usize> = Vec::new();

        if is_any(dp, Mode::MultiSplit) || is_any(dp, Mode::MultiDisorder) {
            multi = resolve_multi_pos(rdata, l7, &dp.splits);
            multi = multi.iter().map(|&p| pos_normalize(p, reasm_offset, plen)).filter(|&p| p != 0).collect();
        } else if is_any(dp, Mode::FakedSplit) || is_any(dp, Mode::FakedDisorder) {
            let mut sp = 0;
            for s in dp.splits.iter().filter(|s| s.marker != Marker::Abs) {
                if sp == 0 {
                    sp = resolve_pos(rdata, l7, s);
                }
            }
            if sp == 0 {
                for s in dp.splits.iter().filter(|s| s.marker == Marker::Abs) {
                    if sp == 0 {
                        sp = resolve_pos(rdata, l7, s);
                    }
                }
            }
            if sp == 0 {
                sp = 1;
            }
            if sp == 0 || sp > rdata.len() {
                sp = 1;
            }
            split_pos = pos_normalize(sp, reasm_offset, plen);
        } else if is_any(dp, Mode::HostFakeSplit) {
            let splits = [ProtoPos { marker: Marker::Host, pos: 0 }, ProtoPos { marker: Marker::HostEnd, pos: 0 }];
            let mut v = resolve_multi_pos(rdata, l7, &splits);
            if v.len() != 2 {
                v.clear();
            } else {
                v = v.iter().map(|&p| pos_normalize(p, reasm_offset, plen)).filter(|&p| p != 0).collect();
                if v.len() != 2 {
                    v.clear();
                } else {
                    let mid = resolve_pos(rdata, l7, &dp.hostfakesplit_midhost);
                    if mid != 0 {
                        let mid = pos_normalize(mid, reasm_offset, plen);
                        if mid > v[0] && mid < v[1] {
                            v.push(mid);
                        }
                    }
                }
            }
            multi = v;
        }

        let seqovl_pos = if is_any(dp, Mode::FakedSplit) || is_any(dp, Mode::MultiSplit) {
            if dp.seqovl.marker == Marker::Abs && dp.seqovl.pos > 0 { dp.seqovl.pos as usize } else { 0 }
        } else if is_any(dp, Mode::FakedDisorder) || is_any(dp, Mode::MultiDisorder) {
            pos_normalize(resolve_pos(rdata, l7, &dp.seqovl), reasm_offset, plen)
        } else {
            0
        };

        let mut b_fake = false;
        match dp.desync_mode {
            Mode::FakeKnown | Mode::Fake => {
                if dp.desync_mode == Mode::FakeKnown && l7 == L7::Unknown {
                    // nothing
                } else if reasm_offset == 0 {
                    let mut sequence = seq;
                    for (i, item) in fake_list.iter().enumerate() {
                        let _ = i;
                        let mut data_buf: Option<Vec<u8>> = None;
                        if l7 == L7::Tls && item.data.len() <= FAKE_MAX_TCP {
                            if let Some(tm) = &item.tls_mod {
                                data_buf = runtime_tls_mod(item.tls_cache.as_ref(), tm, &item.data, rdata);
                            }
                        }
                        let full: &[u8] = data_buf.as_deref().unwrap_or(&item.data);
                        let fake_data = &full[item.offset.min(full.len())..];
                        let Some(s) = base.seg(flags_fake, sequence, ttl_fake, ip_id, dp.desync_fooling_mode, incs, fake_data) else {
                            reasm_orig_cancel(tx.io, ct.as_deref_mut());
                            return Next::SendOrig;
                        };
                        if !tx.send_rep(dp.desync_repeats, &s) {
                            reasm_orig_cancel(tx.io, ct.as_deref_mut());
                            return Next::SendOrig;
                        }
                        ip_id = ip_id_next(ip_id, mode_id);
                        if dp.tcp_mod_seq {
                            sequence = sequence.wrapping_add(fake_data.len() as u32);
                        }
                    }
                    b_fake = true;
                }
            }
            Mode::Rst | Mode::RstAck => {
                if reasm_offset == 0 {
                    let fl = TH_RST | if dp.desync_mode == Mode::RstAck { TH_ACK } else { 0 };
                    let Some(s) = base.seg(fl, seq, ttl_fake, ip_id, dp.desync_fooling_mode, incs, &[]) else {
                        reasm_orig_cancel(tx.io, ct.as_deref_mut());
                        return Next::SendOrig;
                    };
                    if !tx.send_rep(dp.desync_repeats, &s) {
                        reasm_orig_cancel(tx.io, ct.as_deref_mut());
                        return Next::SendOrig;
                    }
                    ip_id = ip_id_next(ip_id, mode_id);
                    b_fake = true;
                }
            }
            _ => {}
        }
        if b_fake {
            st.b_fake = true;
        }

        reasm_orig_cancel(tx.io, ct.as_deref_mut());

        let mode = if dp.desync_mode2 == Mode::None { dp.desync_mode } else { dp.desync_mode2 };
        let payload: Vec<u8> = pkt.payload().to_vec();
        let ok_ip_id = |ct: &mut Option<&mut Ctrack>, v: u16| {
            if replay {
                if let Some(c) = ct.as_deref_mut() {
                    c.ip_id = v;
                }
            }
        };
        macro_rules! send_or_orig {
            ($e:expr) => {
                if !$e {
                    return Next::SendOrig;
                }
            };
        }
        macro_rules! seg_or_orig {
            ($e:expr) => {
                match $e {
                    Some(v) => v,
                    None => return Next::SendOrig,
                }
            };
        }

        match mode {
            Mode::HostFakeSplit => {
                if multi.len() < 2 {
                    return Next::Unsplit;
                }
                if replay {
                    if let Some(c) = ct.as_deref() {
                        if c.ip_id != 0 {
                            ip_id = c.ip_id;
                        }
                    }
                }
                let (pos_host, pos_endhost) = (multi[0], multi[1]);
                let pos_split_host = if multi.len() >= 3 { multi[2] } else { 0 };
                let host_size = pos_endhost - pos_host;

                let s1 = seg_or_orig!(base.seg(flags_orig, seq, ttl_orig, ip_id, 0, incs, &payload[..pos_host]));
                ip_id = ip_id_next(ip_id, mode_id);
                send_or_orig!(tx.send(&s1));

                let mut fakehost = vec![0u8; host_size];
                if !dp.hfs_host.is_empty() {
                    let h = dp.hfs_host.as_bytes();
                    if host_size <= h.len() {
                        fakehost.copy_from_slice(&h[h.len() - host_size..]);
                    } else {
                        let mut sz = host_size - h.len();
                        fakehost[sz..].copy_from_slice(h);
                        sz -= 1;
                        fakehost[sz] = b'.';
                        if sz > 0 {
                            rng::fill_az(&mut fakehost[0..1]);
                            sz -= 1;
                        }
                        rng::fill_az09(&mut fakehost[1..1 + sz]);
                    }
                } else {
                    rng::fill_az(&mut fakehost[0..1]);
                    rng::fill_az09(&mut fakehost[1..host_size]);
                    if host_size >= 7 {
                        fakehost[host_size - 4] = b'.';
                        let tld = TLD[rng::next_u32() as usize % TLD.len()];
                        fakehost[host_size - 3..].copy_from_slice(tld.as_bytes());
                    }
                }
                crate::dlog!("generated fake host: {}", String::from_utf8_lossy(&fakehost));

                let mut pkt2 = seg_or_orig!(base.seg(flags_fake, seq.wrapping_add(pos_host as u32), ttl_fake, ip_id, dp.desync_fooling_mode, incs, &fakehost));
                if mode_id != IpIdMode::SeqGroup {
                    ip_id = ip_id_next(ip_id, mode_id);
                }
                send_or_orig!(tx.send_rep(dp.desync_repeats, &pkt2));

                let mut ip_id_after_host = ip_id_next(ip_id, mode_id);
                if pos_split_host != 0 {
                    ip_id_after_host = ip_id_next(ip_id_after_host, mode_id);
                }
                let mut pkt3 = seg_or_orig!(base.seg(flags_orig, seq.wrapping_add(pos_endhost as u32), ttl_orig, ip_id_after_host, 0, incs, &payload[pos_endhost..]));
                if dp.hfs_ordering == 1 {
                    send_or_orig!(tx.send(&pkt3));
                }
                let sz = if pos_split_host != 0 { pos_split_host - pos_host } else { host_size };
                let s = seg_or_orig!(base.seg(flags_orig, seq.wrapping_add(pos_host as u32), ttl_orig, ip_id, 0, incs, &payload[pos_host..pos_host + sz]));
                ip_id = ip_id_next(ip_id, mode_id);
                send_or_orig!(tx.send(&s));
                if pos_split_host != 0 {
                    let sz = pos_endhost - pos_split_host;
                    let s = seg_or_orig!(base.seg(flags_orig, seq.wrapping_add(pos_split_host as u32), ttl_orig, ip_id, 0, incs, &payload[pos_split_host..pos_split_host + sz]));
                    ip_id = ip_id_next(ip_id, mode_id);
                    send_or_orig!(tx.send(&s));
                }
                if dp.hfs_ordering == 0 {
                    if mode_id != IpIdMode::SeqGroup {
                        if !pkt.v6 {
                            pkt2[4..6].copy_from_slice(&ip_id.to_be_bytes());
                            ip4_header_checksum_zeroed(&mut pkt2);
                        }
                        ip_id = ip_id_next(ip_id, mode_id);
                    }
                    send_or_orig!(tx.send_rep(dp.desync_repeats, &pkt2));
                    if mode_id != IpIdMode::SeqGroup {
                        ip_id_after_host = ip_id;
                        if !pkt.v6 {
                            pkt3[4..6].copy_from_slice(&ip_id.to_be_bytes());
                            ip4_header_checksum_zeroed(&mut pkt3);
                        }
                    }
                    send_or_orig!(tx.send(&pkt3));
                }
                ok_ip_id(ct, ip_id_next(ip_id_after_host, mode_id));
                Next::Done(VERDICT_DROP)
            }

            Mode::MultiSplit => {
                if multi.is_empty() {
                    return Next::Unsplit;
                }
                if replay {
                    if let Some(c) = ct.as_deref() {
                        if c.ip_id != 0 {
                            ip_id = c.ip_id;
                        }
                    }
                }
                let mut from = 0usize;
                for i in 0..=multi.len() {
                    let to = if i == multi.len() { plen } else { multi[i] };
                    let mut seqovl = if i == 0 && reasm_offset == 0 { seqovl_pos } else { 0 };
                    loop {
                        let seg_data: Vec<u8> = if seqovl != 0 {
                            let seg_len = to - from + seqovl;
                            if seg_len > MAX_PKT {
                                return Next::SendOrig;
                            }
                            let mut v = vec![0u8; seqovl];
                            fill_pattern(&mut v, &dp.seqovl_pattern, 0);
                            v.extend_from_slice(&payload[from..to]);
                            v
                        } else {
                            payload[from..to].to_vec()
                        };
                        let s = seg_or_orig!(base.seg(flags_orig, seq.wrapping_add(from as u32).wrapping_sub(seqovl as u32), ttl_orig, ip_id, 0, incs, &seg_data));
                        ip_id = ip_id_next(ip_id, mode_id);
                        if !tx.send(&s) {
                            if seqovl != 0 && cfg!(target_os = "linux") {
                                seqovl = 0;
                                continue;
                            }
                            return Next::SendOrig;
                        }
                        break;
                    }
                    from = to;
                }
                ok_ip_id(ct, ip_id);
                Next::Done(VERDICT_DROP)
            }

            Mode::MultiDisorder => {
                if multi.is_empty() {
                    return Next::Unsplit;
                }
                if replay {
                    if let Some(c) = ct.as_deref() {
                        if c.ip_id != 0 {
                            ip_id = c.ip_id;
                        }
                    }
                }
                ip_id = ip_id_add(ip_id, multi.len() as i16, mode_id);
                let ip_id_end = ip_id;
                let mut to = plen;
                let mut i = multi.len() as i32 - 1;
                while i >= -1 {
                    let from = if i >= 0 { multi[i as usize] } else { 0 };
                    let mut seqovl = 0usize;
                    let mut seg_data: Vec<u8> = payload[from..to].to_vec();
                    if i == 0 {
                        if seqovl_pos >= from {
                            // cancel seqovl
                        } else {
                            seqovl = seqovl_pos;
                            if to - from + seqovl > MAX_PKT {
                                return Next::SendOrig;
                            }
                            let mut v = vec![0u8; seqovl];
                            fill_pattern(&mut v, &dp.seqovl_pattern, 0);
                            v.extend_from_slice(&payload[from..to]);
                            seg_data = v;
                        }
                    }
                    let s = seg_or_orig!(base.seg(flags_orig, seq.wrapping_add(from as u32).wrapping_sub(seqovl as u32), ttl_orig, ip_id, 0, incs, &seg_data));
                    ip_id = ip_id_prev(ip_id, mode_id);
                    send_or_orig!(tx.send(&s));
                    to = from;
                    i -= 1;
                }
                ok_ip_id(ct, ip_id_next(ip_id_end, mode_id));
                Next::Done(VERDICT_DROP)
            }

            Mode::FakedDisorder => {
                if plen > DPI_DESYNC_MAX_FAKE_LEN {
                    return Next::SendOrig;
                }
                if replay {
                    if let Some(c) = ct.as_deref() {
                        if c.ip_id != 0 {
                            ip_id = c.ip_id;
                        }
                    }
                }
                let mut seqovl = 0usize;
                let order;
                if split_pos != 0 {
                    order = dp.fs_ordering & 3;
                    if seqovl_pos < split_pos {
                        seqovl = seqovl_pos;
                    }
                } else {
                    order = (dp.fs_ordering >> 3) & 3;
                }
                let mut pat = vec![0u8; plen];
                fill_pattern(&mut pat, &dp.fsplit_pattern, reasm_offset);

                let seg2: Vec<u8> = if seqovl != 0 {
                    let mut v = vec![0u8; seqovl];
                    fill_pattern(&mut v, &dp.seqovl_pattern, 0);
                    v.extend_from_slice(&payload[split_pos..]);
                    v
                } else {
                    payload[split_pos..].to_vec()
                };
                let mut fakeseg2 = seg_or_orig!(base.seg(flags_fake, seq.wrapping_add(split_pos as u32), ttl_fake, ip_id, dp.desync_fooling_mode, incs, &pat[split_pos..]));
                if order == 0 {
                    if mode_id != IpIdMode::SeqGroup {
                        ip_id = ip_id_next(ip_id, mode_id);
                    }
                    send_or_orig!(tx.send_rep(dp.desync_repeats, &fakeseg2));
                }
                let s = seg_or_orig!(base.seg(flags_orig, seq.wrapping_add(split_pos as u32).wrapping_sub(seqovl as u32), ttl_orig, ip_id, 0, incs, &seg2));
                ip_id = ip_id_next(ip_id, mode_id);
                send_or_orig!(tx.send(&s));
                if order <= 1 {
                    if mode_id != IpIdMode::SeqGroup {
                        if !pkt.v6 {
                            fakeseg2[4..6].copy_from_slice(&ip_id.to_be_bytes());
                            ip4_header_checksum_zeroed(&mut fakeseg2);
                        }
                        ip_id = ip_id_next(ip_id, mode_id);
                    }
                    send_or_orig!(tx.send_rep(dp.desync_repeats, &fakeseg2));
                }
                if split_pos != 0 {
                    let mut fakeseg = seg_or_orig!(base.seg(flags_fake, seq, ttl_fake, ip_id, dp.desync_fooling_mode, incs, &pat[..split_pos]));
                    if mode_id != IpIdMode::SeqGroup {
                        ip_id = ip_id_next(ip_id, mode_id);
                    }
                    send_or_orig!(tx.send_rep(dp.desync_repeats, &fakeseg));
                    let s = seg_or_orig!(base.seg(flags_orig, seq, ttl_orig, ip_id, 0, incs, &payload[..split_pos]));
                    ip_id = ip_id_next(ip_id, mode_id);
                    send_or_orig!(tx.send(&s));
                    if order <= 2 {
                        if mode_id != IpIdMode::SeqGroup {
                            if !pkt.v6 {
                                fakeseg[4..6].copy_from_slice(&ip_id.to_be_bytes());
                                ip4_header_checksum_zeroed(&mut fakeseg);
                            }
                            ip_id = ip_id_next(ip_id, mode_id);
                        }
                        send_or_orig!(tx.send_rep(dp.desync_repeats, &fakeseg));
                    }
                }
                ok_ip_id(ct, ip_id);
                Next::Done(VERDICT_DROP)
            }

            Mode::FakedSplit => {
                if replay {
                    if let Some(c) = ct.as_deref() {
                        if c.ip_id != 0 {
                            ip_id = c.ip_id;
                        }
                    }
                }
                if plen > DPI_DESYNC_MAX_FAKE_LEN {
                    return Next::SendOrig;
                }
                let mut pat = vec![0u8; plen];
                fill_pattern(&mut pat, &dp.fsplit_pattern, reasm_offset);
                let order;
                if split_pos != 0 {
                    order = dp.fs_ordering & 3;
                } else {
                    order = (dp.fs_ordering >> 3) & 3;
                    split_pos = plen;
                }
                let mut fakeseg = seg_or_orig!(base.seg(flags_fake, seq, ttl_fake, ip_id, dp.desync_fooling_mode, incs, &pat[..split_pos]));
                if order == 0 {
                    if mode_id != IpIdMode::SeqGroup {
                        ip_id = ip_id_next(ip_id, mode_id);
                    }
                    send_or_orig!(tx.send_rep(dp.desync_repeats, &fakeseg));
                }
                let mut seqovl = if reasm_offset != 0 || split_pos >= plen { 0 } else { seqovl_pos };
                loop {
                    let seg_data: Vec<u8> = if seqovl != 0 {
                        if split_pos + seqovl > MAX_PKT {
                            return Next::SendOrig;
                        }
                        let mut v = vec![0u8; seqovl];
                        fill_pattern(&mut v, &dp.seqovl_pattern, 0);
                        v.extend_from_slice(&payload[..split_pos]);
                        v
                    } else {
                        payload[..split_pos].to_vec()
                    };
                    let s = seg_or_orig!(base.seg(flags_orig, seq.wrapping_sub(seqovl as u32), ttl_orig, ip_id, 0, incs, &seg_data));
                    ip_id = ip_id_next(ip_id, mode_id);
                    if !tx.send(&s) {
                        if seqovl != 0 && cfg!(target_os = "linux") {
                            seqovl = 0;
                            continue;
                        }
                        return Next::SendOrig;
                    }
                    break;
                }
                if order <= 1 {
                    if mode_id != IpIdMode::SeqGroup {
                        if !pkt.v6 {
                            fakeseg[4..6].copy_from_slice(&ip_id.to_be_bytes());
                            ip4_header_checksum_zeroed(&mut fakeseg);
                        }
                        ip_id = ip_id_next(ip_id, mode_id);
                    }
                    send_or_orig!(tx.send_rep(dp.desync_repeats, &fakeseg));
                }
                if split_pos < plen {
                    let mut fs2 = seg_or_orig!(base.seg(flags_fake, seq.wrapping_add(split_pos as u32), ttl_fake, ip_id, dp.desync_fooling_mode, incs, &pat[split_pos..]));
                    if mode_id != IpIdMode::SeqGroup {
                        ip_id = ip_id_next(ip_id, mode_id);
                    }
                    send_or_orig!(tx.send_rep(dp.desync_repeats, &fs2));
                    let s = seg_or_orig!(base.seg(flags_orig, seq.wrapping_add(split_pos as u32), ttl_orig, ip_id, 0, incs, &payload[split_pos..]));
                    ip_id = ip_id_next(ip_id, mode_id);
                    send_or_orig!(tx.send(&s));
                    if order <= 2 {
                        if mode_id != IpIdMode::SeqGroup {
                            if !pkt.v6 {
                                fs2[4..6].copy_from_slice(&ip_id.to_be_bytes());
                                ip4_header_checksum_zeroed(&mut fs2);
                            }
                            ip_id = ip_id_next(ip_id, mode_id);
                        }
                        send_or_orig!(tx.send_rep(dp.desync_repeats, &fs2));
                    }
                }
                ok_ip_id(ct, ip_id);
                Next::Done(VERDICT_DROP)
            }
            _ => Next::SendOrig,
        }
    }
}
