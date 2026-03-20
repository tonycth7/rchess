// server/persist.rs — Room state persistence to disk

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::{Write, BufRead};

use crate::relay::{Room, RoomOptions};
use crate::config::ServerConfig;

pub fn save_room(data_dir: &Path, code: &str, room: &Room) {
    let path = data_dir.join(format!("{}.room", code));
    let mut f = match std::fs::File::create(&path) { Ok(f) => f, Err(_) => return };
    let _ = writeln!(f, "code={}", code);
    let _ = writeln!(f, "time={}", room.opts.time_secs);
    let _ = writeln!(f, "inc={}", room.opts.increment_secs);
    let _ = writeln!(f, "undo={}", room.opts.undo_allowed);
    let _ = writeln!(f, "engine={}", room.opts.engine_during);
    let _ = writeln!(f, "started={}", room.started);
    if let Some(ref t) = room.white_token { let _ = writeln!(f, "white_token={}", t); }
    if let Some(ref t) = room.black_token { let _ = writeln!(f, "black_token={}", t); }
    let _ = writeln!(f, "white_name={}", room.white_name.as_deref().unwrap_or(""));
    let _ = writeln!(f, "black_name={}", room.black_name.as_deref().unwrap_or(""));
    let _ = writeln!(f, "moves={}", room.moves.join(" "));
    // Save chat history (last 100 messages)
    for msg in room.chat_history.iter().rev().take(100).rev() {
        let _ = writeln!(f, "chat={}:{}", msg.0, msg.1);
    }
}

pub fn delete_room(data_dir: &Path, code: &str) {
    let path = data_dir.join(format!("{}.room", code));
    let _ = std::fs::remove_file(path);
}

pub fn load_all_rooms(cfg: &ServerConfig) -> HashMap<String, Room> {
    let dir = Path::new(&cfg.data_dir);
    let mut map = HashMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return map };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("room") { continue; }
        if let Some(room) = load_room(&path) {
            let code = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("").to_uppercase();
            if !code.is_empty() { map.insert(code, room); }
        }
    }
    map
}

fn load_room(path: &PathBuf) -> Option<Room> {
    let f = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(f);
    let mut opts = RoomOptions::default();
    let mut moves = vec![];
    let mut white_token = None;
    let mut black_token = None;
    let mut started = false;
    let mut white_name = None;
    let mut black_name = None;
    let mut chat_history = vec![];

    for line in reader.lines().flatten() {
        let line = line.trim().to_string();
        let mut kv = line.splitn(2, '=');
        let k = kv.next().unwrap_or("").trim();
        let v = kv.next().unwrap_or("").trim();
        match k {
            "time"        => { if let Ok(n) = v.parse() { opts.time_secs = n; } }
            "inc"         => { if let Ok(n) = v.parse() { opts.increment_secs = n; } }
            "undo"        => opts.undo_allowed  = v == "true",
            "engine"      => opts.engine_during = v == "true",
            "started"     => started            = v == "true",
            "white_token" => white_token        = if v.is_empty() { None } else { Some(v.to_string()) },
            "black_token" => black_token        = if v.is_empty() { None } else { Some(v.to_string()) },
            "white_name"  => white_name         = if v.is_empty() { None } else { Some(v.to_string()) },
            "black_name"  => black_name         = if v.is_empty() { None } else { Some(v.to_string()) },
            "moves"       => {
                if !v.is_empty() {
                    moves = v.split_whitespace().map(|s| s.to_string()).collect();
                }
            }
            "chat" => {
                // format: color:message
                let mut parts = v.splitn(2, ':');
                let color = parts.next().unwrap_or("").to_string();
                let msg   = parts.next().unwrap_or("").to_string();
                if !color.is_empty() { chat_history.push((color, msg)); }
            }
            _ => {}
        }
    }

    let mut room = Room::new(opts);
    room.moves        = moves;
    room.white_token  = white_token;
    room.black_token  = black_token;
    room.white_name   = white_name;
    room.black_name   = black_name;
    room.started      = started;
    room.chat_history = chat_history;
    Some(room)
}
