use super::args::*;
use super::conntrack::*;
use super::desync::*;
use super::packet::*;
use super::proto::*;
use super::rng;

pub struct Base {
    pub src: Endpoint,
    pub dst: Endpoint,
    pub win: u16,
    pub ack: u32,
    pub scale: u8,
    pub ts: Option<[u8; 8]>,
    pub df: bool,
    pub tos: u8,
    pub flow: u32,
}

impl Base {
    pub fn from_pkt(pkt: &Pkt) -> Base {
        Base { src: pkt.src(), dst: pkt.dst(), win: pkt.win(), ack: pkt.ack(), scale: pkt.tcp_scale_factor(), ts: pkt.tcp_timestamps(), df: pkt.df(), tos: pkt.tos(), flow: pkt.flow() }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn seg_full(&self, flags: u16, sack: bool, mss: u16, seq: u32, ack: u32, ttl: u8, ip_id: u16, fooling: u32, incs: (u32, u32, u32), data: &[u8]) -> Option<Vec<u8>> {
        prepare_tcp_segment(
            &self.src,
            &self.dst,
            &TcpBuild {
                flags,
                sack,
                mss,
                seq,
                ack,
                win: self.win,
                scale: self.scale,
                ts: self.ts,
                df: self.df,
                ttl,
                tos: self.tos,
                ip_id,
                flow: self.flow,
                fooling,
                ts_inc: incs.0,
                badseq_inc: incs.1,
                badseq_ack_inc: incs.2,
                data,
            },
            MAX_PKT,
        )
    }

    pub fn seg(&self, flags: u16, seq: u32, ttl: u8, ip_id: u16, fooling: u32, incs: (u32, u32, u32), data: &[u8]) -> Option<Vec<u8>> {
        self.seg_full(flags, false, 0, seq, self.ack, ttl, ip_id, fooling, incs, data)
    }
}

pub struct St {
    pub verdict: u8,
    pub b_fake: bool,
    pub dp: Option<usize>,
}

pub fn desync_incs(dp: &Profile) -> (u32, u32, u32) {
    (dp.desync_ts_increment, dp.desync_badseq_increment, dp.desync_badseq_ack_increment)
}

fn send_delayed(io: &mut dyn Io, ct: &mut Ctrack) {
    for d in std::mem::take(&mut ct.delayed) {
        io.send(&d.dst, d.fwmark, &d.ifout, &d.packet);
    }
}

pub fn reasm_orig_cancel(io: &mut dyn Io, ct: Option<&mut Ctrack>) {
    if let Some(ct) = ct {
        ct.reasm_orig.clear();
        send_delayed(io, ct);
    }
}

impl Engine {
    #[allow(clippy::too_many_arguments)]
    pub fn tcp_play(&mut self, io: &mut dyn Io, replay: bool, reasm_offset: usize, fwmark: u32, ifin: Iface, ifout: Iface, pkt: &mut Pkt, ct_replay: Option<&mut Ctrack>) -> u8 {
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
        let ct: Option<&mut Ctrack> = if replay { ct_replay } else { owned.as_mut() };
        let mut ct = ct;
        let mut st = St { verdict: VERDICT_PASS, b_fake: false, dp: None };
        let desync_fwmark = fwmark | params.desync_fwmark;
        let mut tx = Tx { io, dst: pkt.dst(), fwmark: desync_fwmark, ifout };

        let next = self.tcp_core(&params, &mut tx, replay, reverse, reasm_offset, ifin, pkt, &mut ct, &mut st);
        let v = match next {
            Next::Done(v) => v,
            Next::Unsplit => {
                let mut r = None;
                if replay && !pkt.v6 {
                    if let (Some(c), Some(dpi)) = (ct.as_deref_mut(), st.dp) {
                        if c.ip_id != 0 {
                            pkt.set_ip_id(c.ip_id);
                            c.ip_id = ip_id_next(c.ip_id, params.profiles[dpi].ip_id_mode);
                            r = Some(VERDICT_MODIFY);
                        }
                    }
                }
                match r {
                    Some(v) => v,
                    None => self.tcp_send_orig(&params, &mut tx, replay, pkt, ct.as_deref(), &st),
                }
            }
            Next::SendOrig => self.tcp_send_orig(&params, &mut tx, replay, pkt, ct.as_deref(), &st),
        };
        drop(ct);
        if let (Some(k), Some(c)) = (key, owned) {
            self.ct.map.insert(k, c);
        }
        v
    }

    fn tcp_send_orig(&mut self, params: &Params, tx: &mut Tx, replay: bool, pkt: &mut Pkt, ct: Option<&Ctrack>, st: &St) -> u8 {
        let ct_nr = if replay { None } else { ct };
        if st.verdict & VERDICT_MASK == VERDICT_DROP {
            return ct_new_postnat_fix(ct_nr, pkt);
        }
        let Some(dpi) = st.dp else { return st.verdict };
        let dp = &params.profiles[dpi];
        if tcp_orig_send(st.verdict, tx, dp, ct, pkt, st.b_fake) {
            return ct_new_postnat_fix(ct_nr, pkt);
        }
        st.verdict
    }

    #[allow(clippy::too_many_arguments)]
    fn tcp_core(&mut self, params: &Params, tx: &mut Tx, replay: bool, reverse: bool, reasm_offset: usize, ifin: Iface, pkt: &mut Pkt, ct: &mut Option<&mut Ctrack>, st: &mut St) -> Next {
        let src = pkt.src();
        let dst = pkt.dst();
        let ttl_in = pkt.ttl();
        let flags = pkt.tcp_flags();

        // --- profile selection
        let mut dp_idx: Option<usize>;
        if replay {
            let Some(c) = ct.as_deref_mut() else { return Next::Done(VERDICT_PASS) };
            dp_idx = c.dp;
            if dp_idx.is_none() && !c.dp_search_complete {
                let m = self.dp_find(true, &dst, c.hostname.as_deref(), c.hostname_is_ip, c.l7proto);
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
                let (mut hostname, mut host_is_ip, l7) = match ct.as_deref() {
                    Some(c) => (c.hostname.clone(), c.hostname_is_ip, c.l7proto),
                    None => (None, false, L7::Unknown),
                };
                if ct.is_some() && hostname.is_none() && !reverse {
                    if let Some((h, is_ip)) = self.ipcache_get_hostname(dst.ip) {
                        if let Some(c) = ct.as_deref_mut() {
                            c.hostname = Some(h.clone());
                        }
                        hostname = Some(h);
                        host_is_ip = is_ip;
                    }
                }
                let m = self.dp_find(true, &dst, hostname.as_deref(), host_is_ip, l7);
                if let Some(c) = ct.as_deref_mut() {
                    c.dp = m.dp;
                    c.dp_search_complete = true;
                }
                dp_idx = m.dp;
            }
            let Some(di) = dp_idx else { return Next::Done(VERDICT_PASS) };
            st.dp = Some(di);
            let dp = &params.profiles[di];
            if let Some(c) = ct.as_deref_mut() {
                maybe_cutoff(c, dp, true);
            }

            if tcp_synack_segment(flags) {
                if dp.wsize != 0 {
                    pkt.rewrite_winsize(dp.wsize, dp.wscale);
                    st.verdict = VERDICT_MODIFY;
                }
                if dp.synack_split == SynackSplit::Syn {
                    let f = pkt.tcp_flags() & !TH_ACK;
                    pkt.set_tcp_flags(f);
                    st.verdict = VERDICT_MODIFY;
                }
            }

            if reverse {
                if let Some(c) = ct.as_deref_mut() {
                    if c.incoming_ttl == 0 {
                        c.incoming_ttl = ttl_in;
                        let mut cc = std::mem::replace(c, Ctrack::new());
                        self.autottl_rediscover(&mut cc, dp, src.ip, ifin.idx);
                        *c = cc;
                    }
                }
                return Next::Done(st.verdict);
            }

            if let Some(c) = ct.as_deref_mut() {
                let mut cc = std::mem::replace(c, Ctrack::new());
                self.autottl_discover(&mut cc, dp, dst.ip, tx.ifout.idx);
                *c = cc;
            }

            if orig_mod(dp, ct.as_deref(), pkt) {
                st.verdict = VERDICT_MODIFY;
            }

            if dp.wssize != 0 {
                if let Some(c) = ct.as_deref() {
                    if !c.b_wssize_cutoff {
                        pkt.rewrite_winsize(dp.wssize, dp.wsscale);
                        st.verdict = VERDICT_MODIFY;
                    }
                }
            }

            if matches!(dp.synack_split, SynackSplit::SynAck | SynackSplit::AckSyn) && tcp_synack_segment(flags) {
                let f = pkt.tcp_flags() & !TH_ACK;
                pkt.set_tcp_flags(f);
                pkt.fix_l4_checksum();
                let base = Base::from_pkt(pkt);
                let ack_seg = base.seg_full(TH_ACK, false, 0, pkt.seq(), pkt.ack(), ttl_in, ip_id_fix(pkt, dp.ip_id_mode), 0, (0, 0, 0), &[]);
                let Some(ack_seg) = ack_seg else { return Next::SendOrig };
                let order = if dp.synack_split == SynackSplit::SynAck { ['S', 'A'] } else { ['A', 'S'] };
                for o in order {
                    let ok = if o == 'S' { tx.send_rep(dp.desync_repeats, &pkt.buf) } else { tx.send_rep(dp.desync_repeats, &ack_seg) };
                    if !ok {
                        return Next::SendOrig;
                    }
                }
                return Next::Done(VERDICT_DROP);
            }

            if !check_desync_interval(dp, ct.as_deref()) {
                reasm_orig_cancel(tx.io, ct.as_deref_mut());
                return Next::SendOrig;
            }
        }

        // (replay path: st.dp)
        let di = dp_idx.unwrap();
        st.dp = Some(di);
        let mut dp = &params.profiles[di];
        let dp_idx = di;

        let mut ttl_orig = pkt.ttl();
        let ttl_fake_of = |dp: &Profile, ct: &Option<&mut Ctrack>, ttl_orig: u8, v6: bool| -> u8 {
            match ct.as_deref() {
                Some(c) if c.desync_autottl != 0 => c.desync_autottl,
                _ => {
                    if v6 {
                        if dp.desync_ttl6 != 0 { dp.desync_ttl6 } else { ttl_orig }
                    } else if dp.desync_ttl != 0 {
                        dp.desync_ttl
                    } else {
                        ttl_orig
                    }
                }
            }
        };
        let mut ttl_fake = ttl_fake_of(dp, ct, ttl_orig, pkt.v6);
        let flags_orig = pkt.tcp_flags();
        let sack = pkt.tcp_has_sack();
        let mss = pkt.tcp_mss();
        let base = Base::from_pkt(pkt);

        let mut ip_id = 0u16;
        if replay {
            if let Some(c) = ct.as_deref() {
                ip_id = c.ip_id;
            }
        }
        if ip_id == 0 {
            ip_id = ip_id_fix(pkt, dp.ip_id_mode);
        }

        if !replay && tcp_syn_segment(flags_orig) {
            match dp.desync_mode0 {
                Mode::SynAck => {
                    let Some(s) = base.seg_full(TH_SYN | TH_ACK, false, 0, pkt.seq(), pkt.ack(), ttl_fake, ip_id, dp.desync_fooling_mode, desync_incs(dp), &[]) else {
                        return Next::SendOrig;
                    };
                    if !tx.send_rep(dp.desync_repeats, &s) {
                        return Next::SendOrig;
                    }
                }
                Mode::SynData => {
                    if pkt.tcp_has_fastopen() || !pkt.payload().is_empty() {
                        return Next::SendOrig;
                    }
                    let Some(s) = base.seg_full(flags_orig, sack, mss, pkt.seq(), pkt.ack(), ttl_orig, ip_id, 0, (0, 0, 0), &dp.fake_syndata) else {
                        return Next::SendOrig;
                    };
                    if !tx.send_rep(dp.desync_repeats, &s) {
                        return Next::SendOrig;
                    }
                    st.verdict = VERDICT_DROP;
                }
                _ => {}
            }
            return Next::SendOrig;
        }

        if flags_orig & TH_SYN != 0 || pkt.payload().is_empty() {
            return Next::SendOrig;
        }

        // ---------------- data segment
        let mut rdata: Vec<u8>;
        if replay {
            rdata = ct.as_deref().map(|c| c.reasm_orig.data().to_vec()).unwrap_or_default();
        } else {
            rdata = pkt.payload().to_vec();
            if let Some(c) = ct.as_deref_mut() {
                if !c.reasm_orig.is_empty() {
                    let seq = c.seq_last;
                    if c.reasm_orig.feed(seq, pkt.payload()) {
                        rdata = c.reasm_orig.data().to_vec();
                    } else {
                        reasm_orig_cancel(tx.io, Some(c));
                    }
                }
            }
        }

        let mut l7 = L7::Unknown;
        let mut host: Option<String> = None;
        let mut have_host = false;

        if is_http(&rdata) {
            l7 = L7::Http;
            if let Some(c) = ct.as_deref_mut() {
                if c.l7proto == L7::Unknown {
                    c.l7proto = l7;
                }
            }
            if !replay {
                reasm_orig_cancel(tx.io, ct.as_deref_mut());
            }
            if !dp.wssize_no_forced_cutoff && !replay {
                if let Some(c) = ct.as_deref_mut() {
                    forced_wssize_cutoff(c, dp);
                }
            }
            host = http_extract_host(&rdata);
            have_host = host.is_some();
            if !have_host {
                return Next::SendOrig;
            }
            if let Some(c) = ct.as_deref_mut() {
                if !replay && !c.req_seq_present {
                    c.req_seq_start = c.seq_last;
                    c.req_seq_end = c.pos_orig.wrapping_sub(1);
                    c.req_seq_present = true;
                    c.req_seq_finalized = true;
                }
            }
        } else if is_tls_client_hello(&rdata, TLS_PARTIALS_ENABLE) {
            let req_full = is_tls_record_full(&rdata);
            l7 = L7::Tls;
            host = tls_hello_extract_host(&rdata, TLS_PARTIALS_ENABLE);
            have_host = host.is_some();

            if let Some(c) = ct.as_deref_mut() {
                if !replay {
                    if c.l7proto == L7::Unknown {
                        c.l7proto = l7;
                    }
                    if !req_full
                        && c.reasm_orig.is_empty()
                        && !c.req_seq_abandoned
                        && !(c.req_seq_finalized && seq_within(c.seq_last, c.req_seq_start, c.req_seq_end))
                    {
                        let sz = tls_record_len(pkt.payload());
                        if sz <= TCP_MAX_REASM {
                            c.reasm_orig.init(sz, c.seq_last);
                            c.reasm_orig.feed(c.seq_last, pkt.payload());
                        } else {
                            reasm_orig_cancel(tx.io, Some(c));
                            return Next::SendOrig;
                        }
                    }
                    if !c.req_seq_finalized {
                        if !c.req_seq_present {
                            c.req_seq_start = c.seq_last;
                            c.req_seq_present = true;
                        }
                        c.req_seq_end = c.pos_orig.wrapping_sub(1);
                        c.req_seq_finalized |= req_full;
                    }
                    if !dp.wssize_no_forced_cutoff && (req_full || c.reasm_orig.is_empty()) {
                        forced_wssize_cutoff(c, dp);
                    }
                    if !c.reasm_orig.is_empty() {
                        verdict_csum_fix(st.verdict, pkt);
                        c.delayed.push(Delayed { dst, fwmark: tx.fwmark, ifin, ifout: tx.ifout, packet: pkt.buf.clone(), len_payload: pkt.payload().len() });
                        if c.reasm_orig.is_full() {
                            let q = std::mem::take(&mut c.delayed);
                            self.replay_queue(tx.io, q, c);
                            c.reasm_orig.clear();
                            send_delayed(tx.io, c);
                        }
                        return Next::Done(VERDICT_DROP);
                    }
                }
            }
            if dp.desync_skip_nosni && !have_host {
                reasm_orig_cancel(tx.io, ct.as_deref_mut());
                return Next::SendOrig;
            }
        }

        if let Some(c) = ct.as_deref_mut() {
            if !replay && c.req_seq_finalized {
                let dseq = c.seq_last.wrapping_sub(c.req_seq_end);
                if dseq >= 0x100_0000 && dseq & 0x8000_0000 == 0 {
                    c.req_seq_abandoned = true;
                }
            }
        }

        let mut host_is_ip = false;
        if let Some(h) = host.as_mut() {
            host_is_ip = strip_host_to_ip(h);
            crate::dlog!("hostname: {h}");
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
                c.hostname = host.clone();
                c.hostname_is_ip = host_is_ip;
                c.hostname_discovered = true;
                self.ipcache_put_hostname(dst.ip, host.as_deref().unwrap_or(""), host_is_ip);
            }
        }

        let (mut check_done, mut check_result, mut check_excluded) = (false, false, false);
        if discovered_l7 || discovered_host {
            let prev = dp_idx;
            let (hn, hip, l7x) = match ct.as_deref() {
                Some(c) => (c.hostname.clone(), c.hostname_is_ip, c.l7proto),
                None => (host.clone(), host_is_ip, l7),
            };
            let m = self.dp_find(true, &dst, hn.as_deref(), hip, l7x);
            check_done = m.check_done;
            check_result = m.check_result;
            check_excluded = m.excluded;
            if let Some(c) = ct.as_deref_mut() {
                c.dp = m.dp;
                c.dp_search_complete = true;
                c.b_check_done = check_done;
                c.b_check_result = check_result;
                c.b_check_excluded = check_excluded;
            }
            let Some(ndp) = m.dp else {
                reasm_orig_cancel(tx.io, ct.as_deref_mut());
                return Next::SendOrig;
            };
            dp = &params.profiles[ndp];
            st.dp = Some(ndp);
            if ndp != prev {
                if let Some(c) = ct.as_deref_mut() {
                    let mut cc = std::mem::replace(c, Ctrack::new());
                    self.autottl_rediscover(&mut cc, dp, dst.ip, tx.ifout.idx);
                    *c = cc;
                }
                ip_id = ip_id_fix(pkt, dp.ip_id_mode);
                if replay {
                    if orig_mod(dp, ct.as_deref(), pkt) {
                        st.verdict = VERDICT_MODIFY;
                    }
                } else {
                    if let Some(c) = ct.as_deref_mut() {
                        maybe_cutoff(c, dp, true);
                    }
                    if orig_mod(dp, ct.as_deref(), pkt) {
                        st.verdict = VERDICT_MODIFY;
                    }
                    if !check_desync_interval(dp, ct.as_deref()) {
                        reasm_orig_cancel(tx.io, ct.as_deref_mut());
                        return Next::SendOrig;
                    }
                }
                ttl_orig = pkt.ttl();
                ttl_fake = ttl_fake_of(dp, ct, ttl_orig, pkt.v6);
            }
        } else if let Some(c) = ct.as_deref() {
            check_done = c.b_check_done;
            check_result = c.b_check_result;
            check_excluded = c.b_check_excluded;
        }
        let _ = check_excluded;

        let lists_empty = self.lists.hostlists_empty(&dp.hl_include) && self.lists.hostlists_empty(&dp.hl_exclude);
        if have_host && !lists_empty {
            if !check_done {
                let (ok, _ex) = self.lists.host_check(&dp.hl_include, &dp.hl_exclude, host.as_deref().unwrap_or(""), host_is_ip);
                check_result = ok;
            }
            if !check_result {
                reasm_orig_cancel(tx.io, ct.as_deref_mut());
                return Next::SendOrig;
            }
        }

        if l7 == L7::Unknown && !dp.desync_any_proto {
            reasm_orig_cancel(tx.io, ct.as_deref_mut());
            return Next::SendOrig;
        }

        // http tampering
        if l7 == L7::Http && (dp.hostcase || dp.hostnospace || dp.domcase || dp.methodeol) {
            if let Some(ph) = http_find_host(pkt.payload()) {
                let plen = pkt.payload().len();
                if dp.hostcase {
                    pkt.payload_mut()[ph..ph + 4].copy_from_slice(&dp.hostspell);
                    st.verdict = VERDICT_MODIFY;
                }
                if dp.domcase {
                    let pl = pkt.payload_mut();
                    let mut p = ph + 5;
                    while p < plen && pl[p] != b'\r' && pl[p] != b'\n' {
                        pl[p] = if p & 1 != 0 { pl[p].to_ascii_lowercase() } else { pl[p].to_ascii_uppercase() };
                        p += 1;
                    }
                    st.verdict = VERDICT_MODIFY;
                }
                if dp.hostnospace {
                    let pl = pkt.payload_mut();
                    let ua = find_sub(pl, b"\r\nUser-Agent: ", 0);
                    let pua = ua.and_then(|u| find_sub(pl, b"\r\n", u + 1));
                    if let Some(pua) = pua {
                        if pua > ph {
                            pl.copy_within(ph + 6..pua, ph + 5);
                            pl[pua - 1] = b' ';
                        } else {
                            let u = ua.unwrap();
                            pl.copy_within(u..ph + 5, u + 1);
                            pl[u] = b' ';
                        }
                        st.verdict = VERDICT_MODIFY;
                    }
                } else if dp.methodeol {
                    let pl = pkt.payload_mut();
                    if pl[ph + 5] == b' ' || pl[ph + 5] == b'\t' {
                        pl.copy_within(0..ph + 5, 1);
                        pl[0] = b'\n';
                        st.verdict = VERDICT_MODIFY;
                    }
                }
            }
        }

        if dp.desync_mode == Mode::None {
            reasm_orig_cancel(tx.io, ct.as_deref_mut());
            return Next::SendOrig;
        }

        self.tcp_desync_modes(params, tx, replay, reasm_offset, pkt, ct, st, dp, &base, &rdata, l7, ttl_orig, ttl_fake, sack, mss, ip_id)
    }
}

pub fn find_sub(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= hay.len() || hay.len() - from < needle.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

// returns true if original was sent completely
pub fn tcp_orig_send(verdict: u8, tx: &mut Tx, dp: &Profile, ct: Option<&Ctrack>, pkt: &mut Pkt, force: bool) -> bool {
    if dp.dup_repeats == 0 && !force {
        return false;
    }
    let ttl_orig = pkt.ttl();
    verdict_csum_fix(verdict, pkt);
    if dp.dup_repeats != 0 && check_dup_interval(dp, ct) {
        let ttl_dup = match ct {
            Some(c) if c.dup_autottl != 0 => c.dup_autottl,
            _ => {
                if pkt.v6 {
                    if dp.dup_ttl6 != 0 { dp.dup_ttl6 } else { ttl_orig }
                } else if dp.dup_ttl != 0 {
                    dp.dup_ttl
                } else {
                    ttl_orig
                }
            }
        };
        let mut flags_dup = pkt.tcp_flags();
        let tf = rewrite_tcp_flags(&mut flags_dup, dp.dup_tcp_flags_unset, dp.dup_tcp_flags_set);
        if tf || dp.dup_fooling_mode != 0 || (!pkt.v6 && dp.dup_ip_id_mode != IpIdMode::Same) {
            let base = Base::from_pkt(pkt);
            let mut ip_id = ip_id_fix(pkt, dp.ip_id_mode);
            let Some(mut seg) = base.seg_full(
                flags_dup,
                pkt.tcp_has_sack(),
                pkt.tcp_mss(),
                pkt.seq(),
                pkt.ack(),
                ttl_dup,
                ip_id,
                dp.dup_fooling_mode,
                (dp.dup_ts_increment, dp.dup_badseq_increment, dp.dup_badseq_ack_increment),
                pkt.payload(),
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
                    let c = ip4_header_checksum_zeroed(&mut seg);
                    let _ = c;
                }
            }
        } else {
            let orig_ttl = pkt.ttl();
            pkt.set_ttl(ttl_dup);
            for _ in 0..dp.dup_repeats {
                if !tx.send(&pkt.buf) {
                    pkt.set_ttl(orig_ttl);
                    return false;
                }
            }
            pkt.set_ttl(orig_ttl);
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

pub fn ip4_header_checksum_zeroed(buf: &mut [u8]) -> u16 {
    let hl = ((buf[0] & 0x0F) as usize) << 2;
    buf[10] = 0;
    buf[11] = 0;
    let c = ip4_header_checksum(&buf[..hl]);
    buf[10..12].copy_from_slice(&c.to_be_bytes());
    c
}

#[allow(dead_code)]
fn _unused() {
    let _ = rng::next_u32();
}
