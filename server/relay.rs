// server/relay.rs — Async Tokio relay server
//
// Each connected client gets a single tokio task. Rooms are shared via
// Arc<Mutex<HashMap<String, Room>>>. Channels are tokio::sync::mpsc.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self as tmpsc, Sender as TSender};

use crate::admin::{RoomInfo, SharedAdmin};
use crate::auth::{check_password, encode_invite, generate_room_code, generate_token, RateLimiter};
use crate::chat::RoomChat;
use crate::config::ServerConfig;
use crate::persist;

// ── Room options ──────────────────────────────────────────────────────────────

#[derive(Clone, Default, Debug)]
pub struct RoomOptions {
    pub time_secs:      u32,
    pub increment_secs: u32,
    pub undo_allowed:   bool,
    pub engine_during:  bool,
}

impl RoomOptions {
    pub fn from_wire(s: &str) -> Self {
        let mut o = Self::default();
        for part in s.split(',') {
            let mut kv = part.splitn(2, '=');
            let k = kv.next().unwrap_or("").trim();
            let v = kv.next().unwrap_or("").trim();
            match k {
                "time"   => { if let Ok(n) = v.parse() { o.time_secs = n; } }
                "inc"    => { if let Ok(n) = v.parse() { o.increment_secs = n; } }
                "undo"   => { o.undo_allowed  = v == "true"; }
                "engine" => { o.engine_during = v == "true"; }
                _ => {}
            }
        }
        o
    }
    pub fn to_wire(&self) -> String {
        format!("time={},inc={},undo={},engine={}",
            self.time_secs, self.increment_secs, self.undo_allowed, self.engine_during)
    }
}

// ── Room ──────────────────────────────────────────────────────────────────────

pub struct Room {
    pub opts:         RoomOptions,
    pub moves:        Vec<String>,
    pub white_tx:     Option<TSender<String>>,
    pub black_tx:     Option<TSender<String>>,
    pub white_conn:   bool,
    pub black_conn:   bool,
    pub white_token:  Option<String>,
    pub black_token:  Option<String>,
    pub white_name:   Option<String>,
    pub black_name:   Option<String>,
    pub started:      bool,
    pub created_at:   Instant,
    pub chat:         RoomChat,
    /// Persistent chat log (color, sanitised message)
    pub chat_history: Vec<(String, String)>,
}

impl Room {
    pub fn new(opts: RoomOptions) -> Self {
        Room {
            opts, moves: vec![],
            white_tx: None, black_tx: None,
            white_conn: false, black_conn: false,
            white_token: None, black_token: None,
            white_name: None, black_name: None,
            started: false,
            created_at: Instant::now(),
            chat: RoomChat::new(),
            chat_history: vec![],
        }
    }
    pub fn is_full(&self) -> bool { self.white_tx.is_some() && self.black_tx.is_some() }
    pub fn opp_tx(&self, my_color: &str) -> Option<&TSender<String>> {
        if my_color == "white" { self.black_tx.as_ref() } else { self.white_tx.as_ref() }
    }
}

pub type Rooms = Arc<Mutex<HashMap<String, Room>>>;

// ── Snapshot helpers ─────────────────────────────────────────────────────────

fn snapshot(rooms: &Rooms) -> Vec<RoomInfo> {
    rooms.lock().unwrap().iter().map(|(code, r)| RoomInfo {
        code:       code.clone(),
        moves:      r.moves.len(),
        white_conn: r.white_conn,
        black_conn: r.black_conn,
        white_name: r.white_name.clone().unwrap_or_default(),
        black_name: r.black_name.clone().unwrap_or_default(),
        time_secs:  r.opts.time_secs,
        inc_secs:   r.opts.increment_secs,
        undo:       r.opts.undo_allowed,
        engine:     r.opts.engine_during,
        started:    r.started,
        chat_msgs:  r.chat_history.len(),
    }).collect()
}

fn cleanup(code: &str, rooms: &Rooms, data_dir: &Arc<PathBuf>, admin: &SharedAdmin) {
    rooms.lock().unwrap().remove(code);
    persist::delete_room(data_dir, code);
    let mut a = admin.lock().unwrap();
    a.push_log(format!("[room] {} finished and cleaned up", code));
    a.rooms = snapshot(rooms);
    a.room_txs.remove(code);
}

// ── Per-client async handler ─────────────────────────────────────────────────

pub async fn handle_client(
    stream:   TcpStream,
    rooms:    Rooms,
    data_dir: Arc<PathBuf>,
    cfg:      Arc<ServerConfig>,
    limiter:  RateLimiter,
    admin:    SharedAdmin,
) {
    let peer = stream.peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "?".into());
    let peer_ip = peer.split(':').next().unwrap_or(&peer).to_string();

    // Rate limit
    if !limiter.check(&peer_ip) {
        eprintln!("[relay] rate-limited {}", peer_ip);
        // try to write the error then drop
        let (_r, mut w) = stream.into_split();
        let _ = w.write_all(b"ERROR:Rate limited. Please wait.\n").await;
        return;
    }

    { let mut a = admin.lock().unwrap(); a.total_conns += 1; a.push_log(format!("[+] connect {}", peer)); }

    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Per-client send channel
    let (tx, mut rx) = tmpsc::channel::<String>(128);

    // Spawn writer task
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let line = format!("{}\n", msg);
            if write_half.write_all(line.as_bytes()).await.is_err() { break; }
        }
    });

    macro_rules! send {
        // Use try_send (non-blocking) so we never hold a MutexGuard across an await point.
        // The channel buffer is 128 — more than enough for any burst of messages.
        ($m:expr) => { let _ = tx.try_send($m.to_string()); }
    }

    // Send welcome
    if !cfg.motd.is_empty() { send!(format!("MOTD:{}", cfg.motd)); }
    send!(format!("SERVER_INFO:name={},version=2.0,rooms={}",
        cfg.server_name, rooms.lock().unwrap().len()));

    let mut room_code = String::new();
    let mut my_color  = String::new();
    let mut authed    = cfg.server_password.is_none();
    let mut my_name   = String::new();

    let mut buf = String::new();
    'main: loop {
        buf.clear();
        match tokio::time::timeout(
            std::time::Duration::from_secs(300),
            reader.read_line(&mut buf),
        ).await {
            Err(_) => {
                // 5-minute idle timeout — send ping challenge
                send!("PING_REQ");
                // Give 10 more seconds
                let mut pong = String::new();
                match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    reader.read_line(&mut pong),
                ).await {
                    Ok(Ok(n)) if n > 0 => {
                        // got a response — continue
                        buf = pong;
                    }
                    _ => break 'main,
                }
            }
            Ok(Ok(0)) | Ok(Err(_)) => break 'main,
            Ok(Ok(_)) => {}
        }

        let line = buf.trim().to_string();
        if line.is_empty() { continue; }

        // ── KICK check (extract data before any .await) ─────────────────────
        let was_kicked = {
            let mut a = admin.lock().unwrap();
            if !room_code.is_empty() && a.kick_queue.contains(&room_code) {
                a.kick_queue.retain(|c| c != &room_code);
                // Grab opp tx now while we hold locks, before any await
                let opp_kick = rooms.lock().unwrap()
                    .get(&room_code)
                    .and_then(|r| r.opp_tx(&my_color).cloned());
                (true, opp_kick)
            } else {
                (false, None)
            }
        }; // ← MutexGuard dropped here, before any await
        if was_kicked.0 {
            let _ = tx.try_send("KICKED:Admin disconnected this room".to_string());
            if let Some(ref opp) = was_kicked.1 {
                let _ = opp.try_send("KICKED:Admin disconnected this room".to_string());
            }
            cleanup(&room_code, &rooms, &data_dir, &admin);
            break 'main;
        }

        // ── AUTH ──────────────────────────────────────────────────────────────
        if let Some(pw) = line.strip_prefix("AUTH:") {
            if let Some(expected) = &cfg.server_password {
                authed = check_password(pw.trim(), expected);
                if authed { send!("AUTH_OK"); } else { send!("ERROR:Wrong server password"); }
            } else { authed = true; send!("AUTH_OK"); }
            continue;
        }
        if !authed { send!("ERROR:Authentication required. Send AUTH:<password>"); continue; }

        // ── PING ──────────────────────────────────────────────────────────────
        if line == "PING" || line == "PONG" { send!("PONG"); continue; }

        // ── NAME:<name> ────────────────────────────────────────────────────────
        if let Some(name) = line.strip_prefix("NAME:") {
            let safe: String = name.trim().chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == ' ')
                .take(20).collect();
            my_name = safe.clone();
            if !room_code.is_empty() {
                let mut map = rooms.lock().unwrap();
                if let Some(r) = map.get_mut(&room_code) {
                    if my_color == "white" { r.white_name = Some(safe.clone()); }
                    else                   { r.black_name  = Some(safe.clone()); }
                    persist::save_room(&data_dir, &room_code, r);
                    // Forward to opponent
                    if let Some(opp) = r.opp_tx(&my_color) {
                        let _ = opp.try_send(format!("OPP_NAME:{}", safe));
                    }
                }
            }
            send!(format!("NAME_ACK:{}", safe));
            continue;
        }

        // ── CREATE:<opts> ─────────────────────────────────────────────────────
        if let Some(opts_str) = line.strip_prefix("CREATE:") {
            if rooms.lock().unwrap().len() >= cfg.max_rooms {
                send!("ERROR:Server full. Try again later."); continue;
            }
            let opts  = RoomOptions::from_wire(opts_str.trim());
            let token = generate_token();
            let code  = {
                let mut map = rooms.lock().unwrap();
                let c = loop {
                    let c = generate_room_code();
                    if !map.contains_key(&c) { break c; }
                };
                let mut room     = Room::new(opts);
                room.white_tx    = Some(tx.clone());
                room.white_conn  = true;
                room.white_token = Some(token.clone());
                if !my_name.is_empty() { room.white_name = Some(my_name.clone()); }
                persist::save_room(&data_dir, &c, &room);
                map.insert(c.clone(), room);
                c
            };
            room_code = code.clone();
            my_color  = "white".into();
            send!(format!("ROOM:{}", code));
            send!(format!("TOKEN:{}", token));
            // Send invite link
            let link = encode_invite(&cfg.link_host(), cfg.port, &code);
            send!(format!("INVITE:{}", link));
            {
                let mut a = admin.lock().unwrap();
                a.total_games += 1;
                a.room_txs.insert(code.clone(), (Some(tx.clone()), None));
                a.push_log(format!("[room] created {} by {} ({})", code, peer, my_name));
                a.rooms = snapshot(&rooms);
            }
            continue;
        }

        // ── JOIN:<code> ───────────────────────────────────────────────────────
        if let Some(code_raw) = line.strip_prefix("JOIN:") {
            let code  = code_raw.trim().to_uppercase();
            let token = generate_token();
            let (err, opp_tx_clone, opts_wire, history) = {
                let mut map = rooms.lock().unwrap();
                match map.get_mut(&code) {
                    None => (Some("Room not found".to_string()), None, String::new(), vec![]),
                    Some(r) if r.is_full() && r.started =>
                        (Some("Room is full — use REJOIN if you disconnected".to_string()), None, String::new(), vec![]),
                    Some(r) if r.is_full() =>
                        (Some("Room is full".to_string()), None, String::new(), vec![]),
                    Some(r) => {
                        r.black_tx    = Some(tx.clone());
                        r.black_conn  = true;
                        r.black_token = Some(token.clone());
                        if !my_name.is_empty() { r.black_name = Some(my_name.clone()); }
                        let opp = r.white_tx.clone();
                        let wire = r.opts.to_wire();
                        let hist = r.chat_history.clone();
                        persist::save_room(&data_dir, &code, r);
                        (None, opp, wire, hist)
                    }
                }
            };
            if let Some(e) = err { send!(format!("ERROR:{}", e)); continue; }
            room_code = code.clone();
            my_color  = "black".into();
            send!("JOINED:black");
            send!(format!("TOKEN:{}", token));
            send!(format!("LOBBY:{}", opts_wire));
            // Replay chat history for new joiner
            for (color, msg) in &history {
                send!(format!("CHAT_HISTORY:{}:{}", color, msg));
            }
            // Notify white
            if let Some(ref opp) = opp_tx_clone {
                let _ = opp.try_send("JOINED:white".to_string());
                if !my_name.is_empty() {
                    let _ = opp.try_send(format!("OPP_NAME:{}", my_name));
                }
                let _ = opp.try_send("START".to_string());
            }
            send!("START");
            {
                let mut map = rooms.lock().unwrap();
                if let Some(r) = map.get_mut(&code) { r.started = true; }
            }
            {
                let mut a = admin.lock().unwrap();
                if let Some(entry) = a.room_txs.get_mut(&code) { entry.1 = Some(tx.clone()); }
                a.push_log(format!("[room] {} started — {} vs {} ({})", code,
                    rooms.lock().unwrap().get(&code).and_then(|r| r.white_name.as_deref()).unwrap_or("?"),
                    my_name, peer));
                a.rooms = snapshot(&rooms);
            }
            continue;
        }

        // ── REJOIN:<code>:<color>:<token> ─────────────────────────────────────
        if let Some(rest) = line.strip_prefix("REJOIN:") {
            let parts: Vec<&str> = rest.trim().splitn(3, ':').collect();
            let code  = parts.get(0).map(|s| s.trim().to_uppercase()).unwrap_or_default();
            let color = parts.get(1).map(|s| s.trim().to_lowercase()).unwrap_or_default();
            let token = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default();

            if code.is_empty() || (color != "white" && color != "black") {
                send!("ERROR:REJOIN format: REJOIN:<code>:<white|black>:<token>"); continue;
            }

            let (err, opp, opts_wire, moves_str, history) = {
                let mut map = rooms.lock().unwrap();
                match map.get_mut(&code) {
                    None => (Some("Room not found or expired".to_string()), None, String::new(), String::new(), vec![]),
                    Some(r) => {
                        let stored = if color == "white" { &r.white_token } else { &r.black_token };
                        if let Some(ref stored_tok) = stored {
                            if !token.is_empty() && !check_password(&token, stored_tok) {
                                // silently close — token mismatch (security)
                                return;
                            }
                        }
                        let new_tok = generate_token();
                        let opp = if color == "white" {
                            r.white_tx    = Some(tx.clone());
                            r.white_conn  = true;
                            r.white_token = Some(new_tok);
                            if !my_name.is_empty() { r.white_name = Some(my_name.clone()); }
                            r.black_tx.clone()
                        } else {
                            r.black_tx    = Some(tx.clone());
                            r.black_conn  = true;
                            r.black_token = Some(new_tok);
                            if !my_name.is_empty() { r.black_name = Some(my_name.clone()); }
                            r.white_tx.clone()
                        };
                        persist::save_room(&data_dir, &code, r);
                        let wire  = r.opts.to_wire();
                        let moves = r.moves.join(" ");
                        let hist  = r.chat_history.clone();
                        (None, opp, wire, moves, hist)
                    }
                }
            };
            if let Some(e) = err { send!(format!("ERROR:{}", e)); continue; }
            room_code = code.clone();
            my_color  = color.clone();
            send!(format!("RECONNECTED:{}:{}", color, moves_str));
            send!(format!("LOBBY:{}", opts_wire));
            // Replay chat history
            for (c, msg) in &history {
                send!(format!("CHAT_HISTORY:{}:{}", c, msg));
            }
            if let Some(ref opp) = opp {
                let _ = opp.try_send("OPPONENT_RECONNECTED".to_string());
                if !my_name.is_empty() { let _ = opp.try_send(format!("OPP_NAME:{}", my_name)); }
            }
            {
                let mut a = admin.lock().unwrap();
                a.push_log(format!("[room] {} rejoined {} as {}", peer, code, color));
                a.rooms = snapshot(&rooms);
            }
            continue;
        }

        // ── From here: require room membership ────────────────────────────────
        if room_code.is_empty() { send!("ERROR:Not in a room"); continue; }

        // ── CHAT:<message> ────────────────────────────────────────────────────
        if let Some(payload) = line.strip_prefix("CHAT:") {
            let (forward, opp) = {
                let mut map = rooms.lock().unwrap();
                if let Some(r) = map.get_mut(&room_code) {
                    let fwd = r.chat.process(&my_color, payload.trim());
                    if let Some(ref safe) = fwd {
                        // Keep chat history (last 200)
                        if r.chat_history.len() >= 200 { r.chat_history.remove(0); }
                        r.chat_history.push((my_color.clone(), safe.clone()));
                        persist::save_room(&data_dir, &room_code, r);
                    }
                    let opp = r.opp_tx(&my_color).cloned();
                    (fwd, opp)
                } else { (None, None) }
            };
            if let Some(safe) = forward {
                let wire = format!("CHAT:{}:{}", my_color, safe);
                // Echo back to sender too (so they see their own message confirmed)
                send!(wire.clone());
                if let Some(ref opp) = opp { let _ = opp.try_send(wire); }
            }
            continue;
        }

        // Get opponent TX once (lazy)
        let opp_tx = {
            let map = rooms.lock().unwrap();
            map.get(&room_code).and_then(|r| r.opp_tx(&my_color).cloned())
        };
        macro_rules! fwd { ($m:expr) => { if let Some(ref t) = opp_tx { let _ = t.try_send($m.to_string()); } }; }

        if let Some(uci) = line.strip_prefix("MOVE:") {
            let uci = uci.trim().to_string();
            {
                let mut map = rooms.lock().unwrap();
                if let Some(r) = map.get_mut(&room_code) {
                    r.moves.push(uci.clone());
                    persist::save_room(&data_dir, &room_code, r);
                }
            }
            fwd!(format!("MOVE:{}", uci));
            let mut a = admin.lock().unwrap();
            a.total_moves += 1;
            a.rooms = snapshot(&rooms);
        }
        else if line == "DRAW"         { fwd!("DRAW_OFFER"); }
        else if line == "DRAW_ACCEPT"  {
            fwd!("DRAW_ACCEPTED");
            cleanup(&room_code, &rooms, &data_dir, &admin);
            break 'main;
        }
        else if line == "DRAW_DECLINE" { fwd!("DRAW_DECLINED"); }
        else if line == "UNDO_REQUEST" { fwd!("UNDO_REQUEST"); }
        else if line == "UNDO_ACCEPT"  {
            {
                let mut map = rooms.lock().unwrap();
                if let Some(r) = map.get_mut(&room_code) {
                    r.moves.truncate(r.moves.len().saturating_sub(2));
                    persist::save_room(&data_dir, &room_code, r);
                }
            }
            fwd!("UNDO_ACCEPTED");
        }
        else if line == "UNDO_DECLINE" { fwd!("UNDO_DECLINED"); }
        else if line == "RESIGN" {
            fwd!("RESIGNED");
            cleanup(&room_code, &rooms, &data_dir, &admin);
            break 'main;
        }
        else if let Some(r) = line.strip_prefix("CLOCK:") {
            // Forward clock sync: CLOCK:<white_ms>:<black_ms>
            fwd!(format!("CLOCK:{}", r));
        }
        else {
            send!(format!("ERROR:Unknown command: {}", &line[..line.len().min(32)]));
        }
    }

    // ── Disconnect cleanup ─────────────────────────────────────────────────────
    eprintln!("[relay] disconnect {} (room={})", peer, room_code);
    {
        let mut a = admin.lock().unwrap();
        a.push_log(format!("[-] disconnect {} room={}", peer, room_code));
    }
    if room_code.is_empty() { return; }

    // Notify opponent, mark disconnected
    {
        let mut map = rooms.lock().unwrap();
        if let Some(r) = map.get_mut(&room_code) {
            if my_color == "white" { r.white_conn = false; r.white_tx = None; }
            else                   { r.black_conn = false; r.black_tx = None; }
            if let Some(opp) = r.opp_tx(&my_color) {
                let _ = opp.try_send("OPPONENT_DISCONNECTED".to_string());
            }
        }
    }
    {
        let mut a = admin.lock().unwrap();
        if let Some(entry) = a.room_txs.get_mut(&room_code) {
            if my_color == "white" { entry.0 = None; } else { entry.1 = None; }
        }
        a.rooms = snapshot(&rooms);
    }

    // TTL-based room expiry
    let code2     = room_code.clone();
    let rooms2    = Arc::clone(&rooms);
    let data_dir2 = Arc::clone(&data_dir);
    let admin2    = Arc::clone(&admin);
    let ttl       = cfg.room_ttl_mins;
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(ttl * 60)).await;
        let both_gone = {
            let map = rooms2.lock().unwrap();
            map.get(&code2).map(|r| !r.white_conn && !r.black_conn).unwrap_or(true)
        };
        if both_gone {
            cleanup(&code2, &rooms2, &data_dir2, &admin2);
            eprintln!("[relay] room {} expired (TTL {}m)", code2, ttl);
        }
    });
}
