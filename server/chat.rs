// server/chat.rs — Chat rate-limiting and sanitisation

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Bucket { count: u32, window_end: Instant }

pub struct ChatRateLimit {
    inner:  Arc<Mutex<HashMap<String, Bucket>>>,
    max:    u32,
    window: Duration,
}

impl ChatRateLimit {
    pub fn new(max_per_minute: u32) -> Self {
        ChatRateLimit {
            inner:  Arc::new(Mutex::new(HashMap::new())),
            max:    max_per_minute,
            window: Duration::from_secs(60),
        }
    }
    pub fn allow(&self, key: &str) -> bool {
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
}

pub fn sanitise(payload: &str) -> String {
    payload.chars()
        .filter(|c| c.is_ascii() && *c >= ' ' && *c != ':' && *c != '\n' && *c != '\r')
        .take(400)
        .collect()
}

pub struct RoomChat {
    pub message_count: u64,
    pub rate_limiter:  ChatRateLimit,
}

impl RoomChat {
    pub fn new() -> Self {
        RoomChat { message_count: 0, rate_limiter: ChatRateLimit::new(30) }
    }
    /// Returns `Some(sanitised)` to forward, `None` if blocked.
    pub fn process(&mut self, sender_color: &str, payload: &str) -> Option<String> {
        if !self.rate_limiter.allow(sender_color) { return None; }
        if payload.is_empty() || payload.len() > 800 { return None; }
        if payload.contains('\n') || payload.contains('\r') { return None; }
        let safe = sanitise(payload);
        if safe.is_empty() { return None; }
        self.message_count += 1;
        Some(safe)
    }
}
