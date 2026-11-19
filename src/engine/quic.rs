use aes::Aes128;
use aes::cipher::{BlockCipherEncrypt, KeyInit};
use aes_gcm::aead::AeadInOut;
use aes_gcm::{Aes128Gcm, Nonce, Tag};
use hkdf::Hkdf;
use sha2::Sha256;

use super::packet::{be16, be32};
use super::proto::is_tls_handshake_client_hello;

pub const QUIC_MAX_CID_LENGTH: usize = 20;

fn varint_size(b: u8) -> usize {
    1 << (b >> 6)
}

fn varint(d: &[u8]) -> (usize, u64) {
    match d[0] >> 6 {
        0 => (1, (d[0] & 0x3F) as u64),
        1 => (2, (be16(d, 0) & 0x3FFF) as u64),
        2 => (4, (be32(d, 0) & 0x3FFF_FFFF) as u64),
        _ => (8, u64::from_be_bytes([d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]]) & 0x3FFF_FFFF_FFFF_FFFF),
    }
}

pub fn quic_draft_version(version: u32) -> u8 {
    if version >> 8 == 0xff0000 {
        return version as u8;
    }
    if version == 0xfaceb001 {
        return 22;
    }
    if version == 0xfaceb002 || version == 0xfaceb00e {
        return 27;
    }
    if version == 0x51303530 || version == 0x54303530 || version == 0x54303531 {
        return 27;
    }
    if version & 0x0F0F0F0F == 0x0a0a0a0a {
        return 29;
    }
    if version == 0x0000_0001 {
        return 34;
    }
    if version == 0x709A50C4 || version == 0x6b3343cf {
        return 100;
    }
    0
}

fn is_draft_max(draft: u8, max: u8) -> bool {
    draft != 0 && draft <= max
}

fn is_v2(version: u32) -> bool {
    version == 0x709A50C4 || version == 0x6b3343cf
}

fn expand_label(secret: &[u8], label: &str, out: &mut [u8]) -> Option<()> {
    let mut info = Vec::with_capacity(4 + label.len());
    info.extend_from_slice(&(out.len() as u16).to_be_bytes());
    info.push(label.len() as u8);
    info.extend_from_slice(label.as_bytes());
    info.push(0);
    Hkdf::<Sha256>::from_prk(secret).ok()?.expand(&info, out).ok()
}

const SALT_DRAFT_22: [u8; 20] = [0x7f, 0xbc, 0xdb, 0x0e, 0x7c, 0x66, 0xbb, 0xe9, 0x19, 0x3a, 0x96, 0xcd, 0x21, 0x51, 0x9e, 0xbd, 0x7a, 0x02, 0x64, 0x4a];
const SALT_DRAFT_23: [u8; 20] = [0xc3, 0xee, 0xf7, 0x12, 0xc7, 0x2e, 0xbb, 0x5a, 0x11, 0xa7, 0xd2, 0x43, 0x2b, 0xb4, 0x63, 0x65, 0xbe, 0xf9, 0xf5, 0x02];
const SALT_DRAFT_29: [u8; 20] = [0xaf, 0xbf, 0xec, 0x28, 0x99, 0x93, 0xd2, 0x4c, 0x9e, 0x97, 0x86, 0xf1, 0x9c, 0x61, 0x11, 0xe0, 0x43, 0x90, 0xa8, 0x99];
const SALT_V1: [u8; 20] = [0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8, 0x0c, 0xad, 0xcc, 0xbb, 0x7f, 0x0a];
const SALT_Q50: [u8; 20] = [0x50, 0x45, 0x74, 0xEF, 0xD0, 0x66, 0xFE, 0x2F, 0x9D, 0x94, 0x5C, 0xFC, 0xDB, 0xD3, 0xA7, 0xF0, 0xD3, 0xB5, 0x6B, 0x45];
const SALT_T50: [u8; 20] = [0x7f, 0xf5, 0x79, 0xe5, 0xac, 0xd0, 0x72, 0x91, 0x55, 0x80, 0x30, 0x4c, 0x43, 0xa2, 0x36, 0x7c, 0x60, 0x48, 0x83, 0x10];
const SALT_T51: [u8; 20] = [0x7a, 0x4e, 0xde, 0xf4, 0xe7, 0xcc, 0xee, 0x5f, 0xa4, 0x50, 0x6c, 0x19, 0x12, 0x4f, 0xc8, 0xcc, 0xda, 0x6e, 0x03, 0x3d];
const SALT_V2: [u8; 20] = [0x0d, 0xed, 0xe3, 0xde, 0xf7, 0x00, 0xa6, 0xdb, 0x81, 0x93, 0x81, 0xbe, 0x6e, 0x26, 0x9d, 0xcb, 0xf9, 0xbd, 0x2e, 0xd9];

fn derive_client_initial_secret(cid: &[u8], version: u32) -> Option<[u8; 32]> {
    let draft = quic_draft_version(version);
    let salt: &[u8; 20] = if version == 0x51303530 {
        &SALT_Q50
    } else if version == 0x54303530 {
        &SALT_T50
    } else if version == 0x54303531 {
        &SALT_T51
    } else if is_draft_max(draft, 22) {
        &SALT_DRAFT_22
    } else if is_draft_max(draft, 28) {
        &SALT_DRAFT_23
    } else if is_draft_max(draft, 32) {
        &SALT_DRAFT_29
    } else if is_draft_max(draft, 34) {
        &SALT_V1
    } else {
        &SALT_V2
    };
    let (prk, _) = Hkdf::<Sha256>::extract(Some(salt), cid);
    let mut out = [0u8; 32];
    expand_label(&prk, "tls13 client in", &mut out)?;
    Some(out)
}

pub fn quic_is_long_header(d: &[u8]) -> bool {
    d.len() >= 9 && d[0] & 0x80 != 0
}

pub fn quic_extract_version(d: &[u8]) -> u32 {
    if quic_is_long_header(d) { be32(d, 1) } else { 0 }
}

pub fn quic_extract_dcid(d: &[u8]) -> Option<&[u8]> {
    if !quic_is_long_header(d) || d[5] == 0 || d[5] as usize > QUIC_MAX_CID_LENGTH || 6 + d[5] as usize > d.len() {
        return None;
    }
    Some(&d[6..6 + d[5] as usize])
}

pub fn quic_decrypt_initial(data: &[u8], max_clean: usize) -> Option<Vec<u8>> {
    let ver = quic_extract_version(data);
    if ver == 0 {
        return None;
    }
    let dcid = quic_extract_dcid(data)?;
    let secret = derive_client_initial_secret(dcid, ver)?;

    let v1 = !is_v2(ver);
    let mut key = [0u8; 16];
    let mut iv = [0u8; 12];
    let mut hp = [0u8; 16];
    expand_label(&secret, if v1 { "tls13 quic key" } else { "tls13 quicv2 key" }, &mut key)?;
    expand_label(&secret, if v1 { "tls13 quic iv" } else { "tls13 quicv2 iv" }, &mut iv)?;
    expand_label(&secret, if v1 { "tls13 quic hp" } else { "tls13 quicv2 hp" }, &mut hp)?;

    let len = data.len();
    let mut pn_offset = 1 + 4 + 1 + data[5] as usize;
    if pn_offset >= len {
        return None;
    }
    pn_offset += 1 + data[pn_offset] as usize;
    if pn_offset >= len || pn_offset + varint_size(data[pn_offset]) >= len {
        return None;
    }
    let (n, token_len) = varint(&data[pn_offset..]);
    pn_offset += n + token_len as usize;
    if pn_offset >= len || pn_offset + varint_size(data[pn_offset]) >= len {
        return None;
    }
    let (n, payload_len) = varint(&data[pn_offset..]);
    pn_offset += n;
    let payload_len = payload_len as usize;
    if payload_len < 20 || pn_offset + payload_len > len {
        return None;
    }

    if pn_offset + 4 + 16 > len {
        return None;
    }
    let cipher = Aes128::new(&hp.into());
    let mut sample = [0u8; 16];
    sample.copy_from_slice(&data[pn_offset + 4..pn_offset + 20]);
    let mut block = sample.into();
    cipher.encrypt_block(&mut block);
    let mask: [u8; 16] = block.into();

    let packet0 = data[0] ^ (mask[0] & 0x0f);
    let pkn_len = (packet0 & 0x03) as usize + 1;
    let mut pkn = 0u32;
    for i in 0..pkn_len {
        pkn |= ((data[pn_offset + i] ^ mask[1 + i]) as u32) << (8 * (pkn_len - 1 - i));
    }
    let tail = u64::from_be_bytes(iv[4..12].try_into().ok()?) ^ pkn as u64;
    iv[4..12].copy_from_slice(&tail.to_be_bytes());

    if payload_len < pkn_len + 16 {
        return None;
    }
    let cryptlen = payload_len - pkn_len - 16;
    if cryptlen > max_clean {
        return None;
    }
    let header_len = pn_offset + pkn_len;
    if header_len > 2048 {
        return None;
    }
    let mut header = data[..header_len].to_vec();
    header[0] = packet0;
    for i in 0..pkn_len {
        header[header_len - 1 - i] = (pkn >> (8 * i)) as u8;
    }

    let mut buf = data[header_len..header_len + cryptlen].to_vec();
    let tag_bytes = &data[header_len + cryptlen..header_len + cryptlen + 16];
    let gcm = Aes128Gcm::new(&key.into());
    let nonce = Nonce::try_from(&iv[..]).ok()?;
    let tag = Tag::try_from(tag_bytes).ok()?;
    gcm.decrypt_inout_detached(&nonce, &header, buf.as_mut_slice().into(), &tag).ok()?;
    Some(buf)
}

const MAX_DEFRAG_PIECES: usize = 128;

// returns (defragmented crypto stream framed as a single CRYPTO frame, full coverage)
pub fn quic_defrag_crypto(clean: &[u8], defrag_cap: usize) -> Option<(Vec<u8>, bool)> {
    if defrag_cap < 10 {
        return None;
    }
    let data_cap = defrag_cap - 10;
    let mut data = vec![0u8; data_cap];
    let mut ranges: Vec<(u64, u64)> = Vec::new();
    let (mut szmax, mut zeropos) = (0u64, 0u64);
    let mut found = false;
    let mut pos = 0usize;

    while pos < clean.len() {
        let ft = clean[pos];
        pos += 1;
        if ft > 1 {
            if ft != 6 {
                return None;
            }
            if pos >= clean.len() || ranges.len() >= MAX_DEFRAG_PIECES {
                return None;
            }
            if pos + varint_size(clean[pos]) >= clean.len() {
                return None;
            }
            let (n, offset) = varint(&clean[pos..]);
            pos += n;
            if pos + varint_size(clean[pos]) > clean.len() {
                return None;
            }
            let (n, sz) = varint(&clean[pos..]);
            pos += n;
            if pos as u64 + sz > clean.len() as u64 {
                return None;
            }
            if offset + sz > data_cap as u64 {
                return None;
            }
            let dup = ranges.iter().any(|&(o, l)| o == offset && l == sz);
            if !dup {
                if zeropos < offset {
                    data[zeropos as usize..offset as usize].fill(0);
                }
                if offset + sz > zeropos {
                    zeropos = offset + sz;
                }
                found = true;
                if offset + sz > szmax {
                    szmax = offset + sz;
                }
                data[offset as usize..(offset + sz) as usize].copy_from_slice(&clean[pos..pos + sz as usize]);
                ranges.push((offset, sz));
            }
            pos += sz as usize;
        }
    }
    if !found {
        return None;
    }
    ranges.sort_by_key(|r| r.0);
    let mut merged: Vec<(u64, u64)> = Vec::new();
    for (o, l) in ranges {
        match merged.last_mut() {
            Some(last) if o <= last.0 + last.1 => {
                let end = (o + l).max(last.0 + last.1);
                last.1 = end - last.0;
            }
            _ => merged.push((o, l)),
        }
    }
    let mut out = Vec::with_capacity(10 + szmax as usize);
    out.push(6);
    out.push(0);
    let mut len8 = szmax.to_be_bytes();
    len8[0] |= 0xC0;
    out.extend_from_slice(&len8);
    out.extend_from_slice(&data[..szmax as usize]);
    let full = merged.len() == 1 && merged[0].0 == 0;
    Some((out, full))
}

pub fn is_quic_crypto_hello(d: &[u8]) -> Option<(usize, usize)> {
    if d.len() < 3 || d[0] != 6 {
        return None;
    }
    let mut offset = 1;
    if offset + varint_size(d[offset]) >= d.len() {
        return None;
    }
    let (n, coff) = varint(&d[offset..]);
    offset += n;
    if coff != 0 || offset + varint_size(d[offset]) >= d.len() {
        return None;
    }
    let (n, clen) = varint(&d[offset..]);
    offset += n;
    let clen = clen as usize;
    if offset + clen > d.len() || !is_tls_handshake_client_hello(&d[offset..offset + clen]) {
        return None;
    }
    Some((offset, clen))
}

pub fn is_quic_initial(d: &[u8]) -> bool {
    if d.len() < 128 {
        return false;
    }
    let ver = quic_extract_version(d);
    if quic_draft_version(ver) < 11 {
        return false;
    }
    if d[0] & 0xF0 != if is_v2(ver) { 0xD0 } else { 0xC0 } {
        return false;
    }
    let len = d.len() as u64;
    let mut offset = 5u64;
    if d[offset as usize] as usize > QUIC_MAX_CID_LENGTH {
        return false;
    }
    offset += 1 + d[offset as usize] as u64;
    if offset >= len {
        return false;
    }
    if d[offset as usize] as usize > QUIC_MAX_CID_LENGTH {
        return false;
    }
    offset += 1 + d[offset as usize] as u64;
    if offset >= len || offset + varint_size(d[offset as usize]) as u64 > len {
        return false;
    }
    let (n, sz) = varint(&d[offset as usize..]);
    let _ = n;
    offset += sz;
    if offset >= len {
        return false;
    }
    let sz2 = varint_size(d[offset as usize]) as u64;
    if offset + sz2 > len {
        return false;
    }
    let (_, sz) = varint(&d[offset as usize..]);
    offset += sz2 + sz;
    offset <= len
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::proto::tls_hello_extract_host_from_handshake;

    #[test]
    fn decrypt_real_initials() {
        let bin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("bin");
        let mut ok = 0;
        for (file, host) in [
            ("quic_initial_www_google_com.bin", "www.google.com"),
            ("quic_initial_4pda_to.bin", "4pda.to"),
            ("quic_initial_rutube_ru.bin", "rutube.ru"),
        ] {
            let Ok(data) = std::fs::read(bin.join(file)) else { continue };
            assert!(is_quic_initial(&data), "{file}: not recognized as initial");
            let clean = quic_decrypt_initial(&data, 16384).unwrap_or_else(|| panic!("{file}: decrypt failed"));
            let (defrag, full) = quic_defrag_crypto(&clean, 16384).unwrap_or_else(|| panic!("{file}: defrag failed"));
            assert!(full, "{file}: not full");
            let (off, len) = is_quic_crypto_hello(&defrag).unwrap_or_else(|| panic!("{file}: no hello"));
            let got = tls_hello_extract_host_from_handshake(&defrag[off..off + len], true);
            assert_eq!(got.as_deref(), Some(host), "{file}");
            ok += 1;
        }
        assert!(ok > 0);
    }
}
