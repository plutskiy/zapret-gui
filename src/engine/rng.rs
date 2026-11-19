use std::cell::Cell;

thread_local! {
    static STATE: Cell<u64> = Cell::new({
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let addr = &t as *const u64 as u64;
        (t ^ addr.rotate_left(32)) | 1
    });
}

pub fn next_u64() -> u64 {
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    })
}

pub fn next_u32() -> u32 {
    (next_u64() >> 32) as u32
}

pub fn next_u16() -> u16 {
    (next_u64() >> 40) as u16
}

pub fn fill_bytes(buf: &mut [u8]) {
    for b in buf {
        *b = next_u64() as u8;
    }
}

pub fn fill_az(buf: &mut [u8]) {
    for b in buf {
        *b = b'a' + (next_u32() % (b'z' - b'a' + 1) as u32) as u8;
    }
}

pub fn fill_az09(buf: &mut [u8]) {
    for b in buf {
        let r = (next_u32() % (10 + (b'z' - b'a') as u32 + 1)) as u8;
        *b = if r < 10 { b'0' + r } else { b'a' + r - 10 };
    }
}
