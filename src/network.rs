// src/network.rs — RChess online multiplayer client v2.0

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ── Room options ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct RoomOptions {
    pub time_secs:          u32,
    pub increment_secs:     u32,
    pub undo_allowed:       bool,
    pub engine_during_game: bool,
}

impl RoomOptions {
    pub fn to_wire(&self) -> String {
        format!("time={},inc={},undo={},engine={}",
            self.time_secs, self.increment_secs,
            self.undo_allowed, self.engine_during_game)
    }
    pub fn from_wire(s: &str) -> Self {
        let mut o = Self::default();
        for part in s.split(',') {
            let mut kv = part.splitn(2, '=');
            let k = kv.next().unwrap_or("").trim();
            let v = kv.next().unwrap_or("").trim();
            match k {
                "time"   => { if let Ok(n) = v.parse() { o.time_secs = n; } }
                "inc"    => { if let Ok(n) = v.parse() { o.increment_secs = n; } }
                "undo"   => { o.undo_allowed       = v == "true"; }
                "engine" => { o.engine_during_game = v == "true"; }
                _ => {}
            }
        }
        o
    }
    pub fn time_label(&self) -> &'static str {
        match self.time_secs {
            0    => "Unlimited",
            60   => "1 min",
            180  => "3 min",
            300  => "5 min",
            600  => "10 min",
            900  => "15 min",
            1800 => "30 min",
            _    => "Custom",
        }
    }
    pub fn inc_label(&self) -> String {
        if self.increment_secs == 0 { "None".to_string() }
        else { format!("{}s/move", self.increment_secs) }
    }
}

// ── Server info ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct ServerInfo {
    pub name:         String,
    pub version:      String,
    pub active_rooms: u32,
    pub motd:         String,
    pub ping_ms:      Option<u64>,
}

// ── Messages ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum NetMsg {
    AuthOk,
    AuthFailed(String),
    Motd(String),
    ServerInfo(ServerInfo),
    RoomCreated(String),
    InviteLink(String),          // rc1:… sharable invite link
    Token(String),
    Joined { color: String },
    Lobby(RoomOptions),
    Start,
    OpponentMove(String),
    Reconnected { color: String, moves: Vec<String> },
    OpponentDisconnected,
    OpponentReconnected,
    DrawOffer,
    DrawAccepted,
    DrawDeclined,
    UndoRequest,
    UndoAccepted,
    UndoDeclined,
    OpponentResigned,
    Chat { color: String, msg: String },
    ChatHistory { color: String, msg: String }, // replayed on rejoin
    OppName(String),             // opponent's display name
    NameAck(String),             // server confirmed our name
    ServerMsg(String),           // admin message pushed to room
    Kicked(String),              // admin kicked room — clear everything
    Clock { white_ms: u64, black_ms: u64 },
    PingReq,                     // server asking us to prove we're alive
    Pong { rtt_ms: u64 },
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum NetCmd {
    Disconnect,
    Raw(String),
}

// ── Client ────────────────────────────────────────────────────────────────────

pub struct NetClient {
    pub rx:        Receiver<NetMsg>,
    tx:            Sender<NetCmd>,
    pub connected: bool,
    ping_sent:     Arc<Mutex<Option<Instant>>>,
}

impl NetClient {
    pub fn connect(addr: &str) -> Result<Self, String> {
        let mut stream = if let Ok(sock_addr) = addr.parse::<std::net::SocketAddr>() {
            TcpStream::connect_timeout(&sock_addr, Duration::from_secs(8))
                .map_err(|e| format!("Cannot connect to {}: {}", addr, e))?
        } else {
            TcpStream::connect(addr)
                .map_err(|e| format!("Cannot connect to {}: {}", addr, e))?
        };
        stream.set_read_timeout(None).ok();
        stream.set_nodelay(true).ok();

        let (in_tx,  in_rx)  = mpsc::channel::<NetMsg>();
        let (out_tx, out_rx) = mpsc::channel::<NetCmd>();
        let ping_sent        = Arc::new(Mutex::new(None::<Instant>));

        // Write thread
        let mut ws = stream.try_clone().map_err(|e| format!("clone: {}", e))?;
        thread::spawn(move || {
            loop {
                match out_rx.recv() {
                    Ok(NetCmd::Raw(line)) => {
                        if ws.write_all(format!("{}\n", line).as_bytes()).is_err() { break; }
                        let _ = ws.flush();
                    }
                    Ok(NetCmd::Disconnect) | Err(_) => {
                        let _ = ws.shutdown(std::net::Shutdown::Both);
                        break;
                    }
                }
            }
        });

        // Read thread — byte-by-byte to avoid BufReader timeout bugs
        let in_tx2 = in_tx;
        let ps2    = Arc::clone(&ping_sent);
        thread::spawn(move || {
            let mut line_buf: Vec<u8> = Vec::with_capacity(512);
            let mut byte = [0u8; 1];
            loop {
                match stream.read(&mut byte) {
                    Ok(0) => { let _ = in_tx2.send(NetMsg::Disconnected); break; }
                    Ok(_) => {
                        let b = byte[0];
                        if b == b'\n' {
                            let line = String::from_utf8_lossy(&line_buf).trim().to_string();
                            line_buf.clear();
                            if line.is_empty() { continue; }
                            let msg  = parse_line(&line, &ps2);
                            let done = matches!(msg, NetMsg::Disconnected | NetMsg::Kicked(_));
                            if in_tx2.send(msg).is_err() { break; }
                            if done { break; }
                        } else if b != b'\r' {
                            line_buf.push(b);
                            if line_buf.len() > 8192 { line_buf.clear(); }
                        }
                    }
                    Err(e) => {
                        use std::io::ErrorKind::*;
                        match e.kind() {
                            WouldBlock | TimedOut | Interrupted => continue,
                            _ => { let _ = in_tx2.send(NetMsg::Disconnected); break; }
                        }
                    }
                }
            }
        });

        Ok(NetClient { rx: in_rx, tx: out_tx, connected: true, ping_sent })
    }

    // ── Outbound helpers ──────────────────────────────────────────────────────

    pub fn send_auth(&self, pw: &str)              { self.raw(&format!("AUTH:{}", pw)); }
    pub fn send_name(&self, name: &str)            { self.raw(&format!("NAME:{}", name)); }
    pub fn send_create(&self, opts: &RoomOptions)  { self.raw(&format!("CREATE:{}", opts.to_wire())); }
    pub fn send_join(&self, room: &str)            { self.raw(&format!("JOIN:{}", room)); }
    pub fn send_rejoin(&self, room: &str, color: &str, token: &str) {
        self.raw(&format!("REJOIN:{}:{}:{}", room, color, token));
    }
    pub fn send_move(&self, uci: &str)             { self.raw(&format!("MOVE:{}", uci)); }
    pub fn send_draw_offer(&self)                  { self.raw("DRAW"); }
    pub fn send_draw_accept(&self)                 { self.raw("DRAW_ACCEPT"); }
    pub fn send_draw_decline(&self)                { self.raw("DRAW_DECLINE"); }
    pub fn send_undo_request(&self)                { self.raw("UNDO_REQUEST"); }
    pub fn send_undo_accept(&self)                 { self.raw("UNDO_ACCEPT"); }
    pub fn send_undo_decline(&self)                { self.raw("UNDO_DECLINE"); }
    pub fn send_resign(&self)                      { self.raw("RESIGN"); }
    pub fn send_clock(&self, white_ms: u64, black_ms: u64) {
        self.raw(&format!("CLOCK:{}:{}", white_ms, black_ms));
    }
    pub fn send_chat(&self, msg: &str) {
        let safe: String = msg.chars().take(400).collect();
        self.raw(&format!("CHAT:{}", safe));
    }
    pub fn send_ping(&self) {
        *self.ping_sent.lock().unwrap() = Some(Instant::now());
        self.raw("PING");
    }
    pub fn send_pong(&self) { self.raw("PONG"); }

    pub fn disconnect(&mut self) {
        let _ = self.tx.send(NetCmd::Disconnect);
        self.connected = false;
    }
    fn raw(&self, s: &str) { let _ = self.tx.send(NetCmd::Raw(s.to_string())); }

    pub fn drain(&self) -> Vec<NetMsg> {
        let mut out = vec![];
        loop {
            match self.rx.try_recv() {
                Ok(msg)                         => out.push(msg),
                Err(TryRecvError::Empty)         => break,
                Err(TryRecvError::Disconnected)  => { out.push(NetMsg::Disconnected); break; }
            }
        }
        out
    }
}

// ── Invite link helpers (mirrors server/auth.rs) ──────────────────────────────

/// Decode an rc1:… invite link into (host, port, room_code).
pub fn decode_invite(link: &str) -> Option<(String, u16, String)> {
    let inner = link.trim().strip_prefix("rc1:")?;
    let decoded = base64url_decode(inner)?;
    let s = String::from_utf8(decoded).ok()?;
    // format: host:port:ROOMCODE  (rsplitn so IPv6 addresses work)
    let parts: Vec<&str> = s.rsplitn(3, ':').collect();
    if parts.len() < 3 { return None; }
    let room_code = parts[0].to_uppercase();
    let port: u16 = parts[1].parse().ok()?;
    let host = parts[2].to_string();
    Some((host, port, room_code))
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

// ── Line parser ───────────────────────────────────────────────────────────────

fn parse_line(line: &str, ping_sent: &Arc<Mutex<Option<Instant>>>) -> NetMsg {
    if line == "AUTH_OK"  { return NetMsg::AuthOk; }
    if line == "PONG"     {
        let rtt = ping_sent.lock().unwrap().take()
            .map(|t| t.elapsed().as_millis() as u64).unwrap_or(0);
        return NetMsg::Pong { rtt_ms: rtt };
    }
    if line == "PING_REQ" { return NetMsg::PingReq; }

    if let Some(r) = line.strip_prefix("MOTD:")        { return NetMsg::Motd(r.trim().to_string()); }
    if let Some(r) = line.strip_prefix("ROOM:")        { return NetMsg::RoomCreated(r.trim().to_string()); }
    if let Some(r) = line.strip_prefix("INVITE:")      { return NetMsg::InviteLink(r.trim().to_string()); }
    if let Some(r) = line.strip_prefix("TOKEN:")       { return NetMsg::Token(r.trim().to_string()); }
    if let Some(r) = line.strip_prefix("LOBBY:")       { return NetMsg::Lobby(RoomOptions::from_wire(r.trim())); }
    if let Some(r) = line.strip_prefix("OPP_NAME:")    { return NetMsg::OppName(r.trim().to_string()); }
    if let Some(r) = line.strip_prefix("NAME_ACK:")    { return NetMsg::NameAck(r.trim().to_string()); }
    if let Some(r) = line.strip_prefix("SERVER_MSG:")  { return NetMsg::ServerMsg(r.trim().to_string()); }
    if let Some(r) = line.strip_prefix("KICKED:")      { return NetMsg::Kicked(r.trim().to_string()); }
    if let Some(r) = line.strip_prefix("MOVE:")        { return NetMsg::OpponentMove(r.trim().to_string()); }

    if let Some(r) = line.strip_prefix("JOINED:") { return NetMsg::Joined { color: r.trim().to_lowercase() }; }
    if line == "START"                              { return NetMsg::Start; }

    if let Some(r) = line.strip_prefix("SERVER_INFO:") {
        let mut info = ServerInfo::default();
        for part in r.split(',') {
            let mut kv = part.splitn(2, '=');
            let k = kv.next().unwrap_or("").trim();
            let v = kv.next().unwrap_or("").trim();
            match k {
                "name"    => info.name         = v.to_string(),
                "version" => info.version      = v.to_string(),
                "rooms"   => { if let Ok(n) = v.parse() { info.active_rooms = n; } }
                _ => {}
            }
        }
        return NetMsg::ServerInfo(info);
    }

    if let Some(r) = line.strip_prefix("RECONNECTED:") {
        let mut parts = r.trim().splitn(2, ':');
        let color     = parts.next().unwrap_or("white").trim().to_lowercase();
        let moves_str = parts.next().unwrap_or("").trim();
        let moves: Vec<String> = if moves_str.is_empty() { vec![] }
            else { moves_str.split_whitespace().map(|s| s.to_string()).collect() };
        return NetMsg::Reconnected { color, moves };
    }

    if let Some(r) = line.strip_prefix("CHAT:") {
        let mut parts = r.trim().splitn(2, ':');
        let color = parts.next().unwrap_or("?").trim().to_string();
        let msg   = parts.next().unwrap_or("").trim().to_string();
        return NetMsg::Chat { color, msg };
    }

    if let Some(r) = line.strip_prefix("CHAT_HISTORY:") {
        let mut parts = r.trim().splitn(2, ':');
        let color = parts.next().unwrap_or("?").trim().to_string();
        let msg   = parts.next().unwrap_or("").trim().to_string();
        return NetMsg::ChatHistory { color, msg };
    }

    if let Some(r) = line.strip_prefix("CLOCK:") {
        let mut parts = r.trim().splitn(2, ':');
        let white_ms: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        let black_ms: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        return NetMsg::Clock { white_ms, black_ms };
    }

    if line == "OPPONENT_DISCONNECTED" { return NetMsg::OpponentDisconnected; }
    if line == "OPPONENT_RECONNECTED"  { return NetMsg::OpponentReconnected; }
    if line == "DRAW_OFFER"            { return NetMsg::DrawOffer; }
    if line == "DRAW_ACCEPTED"         { return NetMsg::DrawAccepted; }
    if line == "DRAW_DECLINED"         { return NetMsg::DrawDeclined; }
    if line == "UNDO_REQUEST"          { return NetMsg::UndoRequest; }
    if line == "UNDO_ACCEPTED"         { return NetMsg::UndoAccepted; }
    if line == "UNDO_DECLINED"         { return NetMsg::UndoDeclined; }
    if line == "RESIGNED"              { return NetMsg::OpponentResigned; }
    if line == "DISCONNECTED"          { return NetMsg::Disconnected; }

    if let Some(r) = line.strip_prefix("ERROR:") {
        let msg = r.trim().to_string();
        if msg.to_lowercase().contains("password") || msg.to_lowercase().contains("auth") {
            return NetMsg::AuthFailed(msg);
        }
        return NetMsg::Error(msg);
    }

    // Unknown line — silently ignore (forward compatibility)
    NetMsg::Pong { rtt_ms: 0 }
}
