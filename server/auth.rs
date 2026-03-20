// server/auth.rs — Token generation, password hashing, rate limiting

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ── Password check ─────────────────────────────────────────────────────────────

pub fn check_password(input: &str, expected: &str) -> bool {
    // Constant-time comparison to resist timing attacks
    let a = input.as_bytes();
    let b = expected.as_bytes();
    if a.len() != b.len() { return false; }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

// ── Token / code generation ────────────────────────────────────────────────────

/// Generate a pseudo-random 32-char hex token (no external deps).
pub fn generate_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Mix wall-clock nanos with a static counter for uniqueness
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(1);
    let t   = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
    let ctr = CTR.fetch_add(1, Ordering::Relaxed);
    let a   = t.wrapping_mul(0x9e3779b97f4a7c15).wrapping_add(ctr);
    let b   = a.wrapping_mul(0x6c62272e07bb0142).wrapping_add(0xdeadbeefcafe1234);
    format!("{:016x}{:016x}", a, b)
}

/// Generate a 6-char uppercase room code (no ambiguous chars: 0/O 1/I/L).
pub fn generate_room_code() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(1);
    const CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    let t    = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
    let ctr  = CTR.fetch_add(1, Ordering::Relaxed);
    let mut seed = t.wrapping_mul(0x9e3779b97f4a7c15).wrapping_add(ctr.wrapping_mul(0x6c62272e07bb0142));
    (0..6).map(|_| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        CHARS[((seed >> 33) as usize) % CHARS.len()] as char
    }).collect()
}

// ── Invite link encoding ───────────────────────────────────────────────────────
//
// An invite link encodes: host:port + room_code into a compact base64url string.
// Format: rc1:<base64url(host:port:ROOMCODE)>
//
// Example: rc1:bG9jYWxob3N0OjkwMDE6QUJDREVG
// In the TUI, pasting this auto-fills host+port+code.

pub fn encode_invite(host: &str, port: u16, room_code: &str) -> String {
    let raw = format!("{}:{}", host, port);
    // Simple base64url without external deps
    let payload = format!("{}:{}", raw, room_code);
    let b = base64url_encode(payload.as_bytes());
    format!("rc1:{}", b)
}

pub fn decode_invite(link: &str) -> Option<(String, u16, String)> {
    let inner = link.trim().strip_prefix("rc1:")?;
    let decoded = base64url_decode(inner)?;
    let s = String::from_utf8(decoded).ok()?;
    // Expected: host:port:ROOMCODE  (host may contain colons for IPv6)
    // Split from the right: last segment is room code, second-last is port
    let parts: Vec<&str> = s.rsplitn(3, ':').collect();
    if parts.len() < 3 { return None; }
    let room_code = parts[0].to_uppercase();
    let port: u16 = parts[1].parse().ok()?;
    let host = parts[2].to_string();
    Some((host, port, room_code))
}

fn base64url_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i+1 < data.len() { data[i+1] as u32 } else { 0 };
        let b2 = if i+2 < data.len() { data[i+2] as u32 } else { 0 };
        out.push(TABLE[((b0 >> 2) & 0x3f) as usize] as char);
        out.push(TABLE[(((b0 << 4) | (b1 >> 4)) & 0x3f) as usize] as char);
        if i+1 < data.len() { out.push(TABLE[(((b1 << 2) | (b2 >> 6)) & 0x3f) as usize] as char); }
        if i+2 < data.len() { out.push(TABLE[(b2 & 0x3f) as usize] as char); }
        i += 3;
    }
    out
}

fn base64url_decode(s: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let decode_char = |c: u8| -> Option<u32> {
        TABLE.iter().position(|&x| x == c).map(|p| p as u32)
    };
    let bytes: Vec<u8> = s.bytes().collect();
    let mut out = vec![];
    let mut i = 0;
    while i + 1 < bytes.len() {
        let b0 = decode_char(bytes[i])?;
        let b1 = decode_char(bytes[i+1])?;
        out.push(((b0 << 2) | (b1 >> 4)) as u8);
        if i+2 < bytes.len() {
            let b2 = decode_char(bytes[i+2])?;
            out.push(((b1 << 4) | (b2 >> 2)) as u8);
            if i+3 < bytes.len() {
                let b3 = decode_char(bytes[i+3])?;
                out.push(((b2 << 6) | b3) as u8);
            }
        }
        i += 4;
    }
    Some(out)
}

// ── Rate limiter ───────────────────────────────────────────────────────────────

struct Bucket { count: u32, window_end: Instant }

#[derive(Clone)]
pub struct RateLimiter {
    inner:  Arc<Mutex<HashMap<String, Bucket>>>,
    max:    u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_per_window: u32, window_secs: u64) -> Self {
        RateLimiter {
            inner:  Arc::new(Mutex::new(HashMap::new())),
            max:    max_per_window,
            window: Duration::from_secs(window_secs),
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut m = self.inner.lock().unwrap();
        let entry = m.entry(key.to_string()).or_insert(Bucket {
            count: 0, window_end: now + self.window,
        });
        if now >= entry.window_end {
            entry.count      = 1;
            entry.window_end = now + self.window;
            return true;
        }
        entry.count += 1;
        entry.count <= self.max
    }

    pub fn purge_expired(&self) {
        let now = Instant::now();
        self.inner.lock().unwrap().retain(|_, v| now < v.window_end);
    }
}
