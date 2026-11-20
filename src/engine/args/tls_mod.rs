use super::*;

pub(super) fn onetime_tls_mod(dp: &mut Profile) -> Result<(), String> {
    let n_profile = dp.n;
    for (idx, blob) in dp.fake_tls.iter_mut().enumerate() {
        let n = idx + 1;
        let Some(tm) = blob.tls_mod.as_mut() else {
            continue;
        };
        if n_profile != 0 && tm.mode & (TLS_MOD_SET | TLS_MOD_CUSTOM_FAKE) == 0 {
            tm.mode |= TLS_MOD_RND | TLS_MOD_RND_SNI | TLS_MOD_DUP_SID;
        }
        if tm.mode & !TLS_MOD_SAVE_MASK == 0 {
            continue;
        }
        if !is_tls_client_hello(&blob.data, false) || blob.data.len() < 44 + blob.data[43] as usize
        {
            return Err(format!(
                "profile {n_profile} fake[{n}] tls mod set but tls fake structure invalid"
            ));
        }
        let mut cache = TlsModCache::default();
        let tm = tm.clone();
        onetime_tls_mod_blob(n_profile, n, &tm, blob, &mut cache)?;
        blob.tls_cache = Some(cache);
        if blob.offset >= blob.data.len() {
            return Err(format!(
                "profile {n_profile} fake[{n}] tls mod shrinked data and offset is out of range"
            ));
        }
    }
    Ok(())
}

pub(super) fn put16v(v: &mut [u8], o: usize, x: u16) {
    v[o..o + 2].copy_from_slice(&x.to_be_bytes());
}
pub(super) fn get16v(v: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([v[o], v[o + 1]])
}
pub(super) fn put24v(v: &mut [u8], o: usize, x: u32) {
    v[o] = (x >> 16) as u8;
    v[o + 1] = (x >> 8) as u8;
    v[o + 2] = x as u8;
}
pub(super) fn get24v(v: &[u8], o: usize) -> u32 {
    ((v[o] as u32) << 16) | ((v[o + 1] as u32) << 8) | v[o + 2] as u32
}

pub(super) fn onetime_tls_mod_blob(
    pn: usize,
    fake_n: usize,
    tm: &FakeTlsMod,
    blob: &mut Blob,
    cache: &mut TlsModCache,
) -> Result<(), String> {
    cache.extlen_offset = 0;
    cache.padlen_offset = 0;
    if tm.mode & (TLS_MOD_RND_SNI | TLS_MOD_SNI | TLS_MOD_PADENCAP) == 0 {
        return Ok(());
    }
    let d = &mut blob.data;
    cache.extlen_offset = tls_find_ext_len(d).ok_or_else(|| {
        format!("profile {pn} fake[{fake_n}] padencap set but tls fake structure invalid")
    })?;

    if tm.mode & (TLS_MOD_RND_SNI | TLS_MOD_SNI) != 0 {
        let (sniext, extlen) = tls_find_ext(d, 0, false).ok_or_else(|| {
            format!("profile {pn} fake[{fake_n}] sni mod is set but tls fake does not have SNI")
        })?;
        let (sni_off, mut slen) =
            tls_advance_to_host_in_sni(d, sniext, extlen).ok_or_else(|| {
                format!(
                    "profile {pn} fake[{fake_n}] sni set but tls fake has invalid SNI structure"
                )
            })?;
        if tm.mode & TLS_MOD_SNI != 0 {
            let new = tm.sni.as_bytes();
            let delta = new.len() as i64 - slen as i64;
            if delta != 0 {
                if (d.len() as i64 + delta) as usize > blob.size_buf {
                    return Err(format!(
                        "profile {pn} fake[{fake_n}] not enough space for new SNI"
                    ));
                }
                let tail: Vec<u8> = d[sni_off + slen..].to_vec();
                d.truncate(sni_off);
                d.extend_from_slice(&vec![0; new.len()]);
                d.extend_from_slice(&tail);
                let adj = |v: u16| (v as i64 + delta) as u16;
                let w = get16v(d, 3);
                put16v(d, 3, adj(w));
                let hs = get24v(d, 6);
                put24v(d, 6, (hs as i64 + delta) as u32);
                let eo = cache.extlen_offset;
                let w = get16v(d, eo);
                put16v(d, eo, adj(w));
                let w = get16v(d, sniext - 2);
                put16v(d, sniext - 2, adj(w));
                let w = get16v(d, sniext);
                put16v(d, sniext, adj(w));
                let w = get16v(d, sni_off - 2);
                put16v(d, sni_off - 2, adj(w));
                slen = new.len();
            }
            d[sni_off..sni_off + new.len()].copy_from_slice(new);
        }
        if tm.mode & TLS_MOD_RND_SNI != 0 {
            if slen == 0 {
                return Err(format!(
                    "profile {pn} fake[{fake_n}] rndsni set but tls fake has zero sized SNI"
                ));
            }
            rng::fill_az(&mut d[sni_off..sni_off + 1]);
            if slen >= 7 {
                rng::fill_az09(&mut d[sni_off + 1..sni_off + 1 + slen - 5]);
                d[sni_off + slen - 4] = b'.';
                let tld = TLD[rng::next_u32() as usize % TLD.len()];
                d[sni_off + slen - 3..sni_off + slen].copy_from_slice(tld.as_bytes());
            } else {
                rng::fill_az09(&mut d[sni_off + 1..sni_off + slen]);
            }
        }
    }
    if tm.mode & TLS_MOD_PADENCAP != 0 {
        if let Some((po, pl)) = tls_find_ext(d, 21, false) {
            if po + pl != d.len() {
                return Err(format!(
                    "profile {pn} fake[{fake_n}] tls padding ext is present but it's not at the end"
                ));
            }
            cache.padlen_offset = po - 2;
        } else {
            if d.len() + 4 > blob.size_buf {
                return Err(format!(
                    "profile {pn} fake[{fake_n}] tls padding is absent and there's no space to add it"
                ));
            }
            d.extend_from_slice(&21u16.to_be_bytes());
            cache.padlen_offset = d.len();
            d.extend_from_slice(&0u16.to_be_bytes());
            let eo = cache.extlen_offset;
            let w = get16v(d, eo);
            put16v(d, eo, w + 4);
            let w = get16v(d, 3);
            put16v(d, 3, w + 4);
            let hs = get24v(d, 6);
            put24v(d, 6, hs + 4);
        }
    }
    Ok(())
}
