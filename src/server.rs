// src/server.rs — RChess relay server
//
// Run with:   cargo run --bin rchess-server -- [--port 9001] [--host 0.0.0.0]
//
// Protocol (line-oriented UTF-8):
//
//  Client → Server          Server → Client
//  ─────────────────────    ──────────────────────────────────────────────
//  CREATE                   ROOM:<6-char code>
//  JOIN:<code>              JOINED:white  |  JOINED:black  |  ERROR:…
//                           START         (sent to both when room is full)
//  MOVE:<uci>               MOVE:<uci>    (forwarded to opponent)
//  DRAW                     DRAW_OFFER    (forwarded)
//  DRAW_ACCEPT              DRAW_ACCEPTED (forwarded)
//  DRAW_DECLINE             DRAW_DECLINED (forwarded)
//  RESIGN                   RESIGNED      (forwarded)
//  PING                     PONG
//                           DISCONNECTED  (sent to surviving player on disconnect)

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

// ── Room bookkeeping ──────────────────────────────────────────────────────────

struct Room {
    /// Sender for first player (always White).
    white_tx: Option<std::sync::mpsc::SyncSender<String>>,
    /// Sender for second player (always Black).
    black_tx: Option<std::sync::mpsc::SyncSender<String>>,
    full: bool,
}

impl Room {
    fn new() -> Self { Room { white_tx: None, black_tx: None, full: false } }
}

type Rooms = Arc<Mutex<HashMap<String, Room>>>;

fn random_code() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static CTR: AtomicUsize = AtomicUsize::new(1);
    let count = CTR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    let pid  = std::process::id() as usize;
    let v    = nanos
        .wrapping_mul(6_364_136_223_846_793_005usize.wrapping_add(count))
        .wrapping_add(pid.wrapping_mul(2_891_336_453));
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    (0..6).map(|i| chars[(v >> (i * 5)) % chars.len()]).collect()
}

// ── Per-client handler ────────────────────────────────────────────────────────

fn handle_client(stream: TcpStream, rooms: Rooms) {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_else(|_| "?".into());
    eprintln!("[server] connect {peer}");

    stream.set_read_timeout(Some(std::time::Duration::from_secs(120))).ok();

    let (client_tx, client_rx) = std::sync::mpsc::sync_channel::<String>(32);

    // ── Write thread for this client ──────────────────────────────────────────
    let mut write_stream = stream.try_clone().expect("clone");
    thread::spawn(move || {
        while let Ok(msg) = client_rx.recv() {
            let line = format!("{msg}\n");
            if write_stream.write_all(line.as_bytes()).is_err() { break; }
        }
    });

    let mut reader      = BufReader::new(stream);
    let mut room_code   = String::new();
    let mut my_color    = String::new();          // "white" or "black"
    let mut opponent_tx: Option<std::sync::mpsc::SyncSender<String>> = None;

    macro_rules! send {
        ($msg:expr) => { let _ = client_tx.send($msg.to_string()); }
    }

    let mut buf = String::new();
    loop {
        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = buf.trim().to_string();
        if line.is_empty() { continue; }

        eprintln!("[server] {peer} → {line}");

        // ── CREATE ────────────────────────────────────────────────────────────
        if line == "CREATE" {
            let code = {
                let mut map = rooms.lock().unwrap();
                let code = loop {
                    let c = random_code();
                    if !map.contains_key(&c) { break c; }
                };
                let mut room = Room::new();
                room.white_tx = Some(client_tx.clone());
                map.insert(code.clone(), room);
                code
            };
            room_code = code.clone();
            my_color  = "white".into();
            send!(format!("ROOM:{code}"));
            continue;
        }

        // ── JOIN:<code> ───────────────────────────────────────────────────────
        if let Some(code) = line.strip_prefix("JOIN:") {
            let code = code.trim().to_uppercase();
            let (err, opp) = {
                let mut map = rooms.lock().unwrap();
                match map.get_mut(&code) {
                    None => (Some("Room not found".to_string()), None),
                    Some(r) if r.full => (Some("Room is full".to_string()), None),
                    Some(r) => {
                        r.black_tx = Some(client_tx.clone());
                        r.full     = true;
                        let opp    = r.white_tx.clone();
                        (None, opp)
                    }
                }
            };
            if let Some(e) = err {
                send!(format!("ERROR:{e}"));
                continue;
            }
            room_code    = code;
            my_color     = "black".into();
            opponent_tx  = opp;
            send!("JOINED:black");
            // Notify White
            if let Some(ref tx) = opponent_tx {
                let _ = tx.send("JOINED:white".into());
                let _ = tx.send("START".into());
            }
            send!("START");
            continue;
        }

        // ── Everything below requires an established room ──────────────────
        if room_code.is_empty() {
            send!("ERROR:Not in a room");
            continue;
        }

        // Lazy opponent resolution for White (who created the room).
        if opponent_tx.is_none() && my_color == "white" {
            let map = rooms.lock().unwrap();
            if let Some(r) = map.get(&room_code) {
                opponent_tx = r.black_tx.clone();
            }
        }

        macro_rules! fwd {
            ($msg:expr) => {
                if let Some(ref tx) = opponent_tx {
                    let _ = tx.send($msg.to_string());
                }
            }
        }

        if let Some(uci) = line.strip_prefix("MOVE:") {
            fwd!(format!("MOVE:{}", uci.trim()));
        } else if line == "DRAW"         { fwd!("DRAW_OFFER"); }
        else if line == "DRAW_ACCEPT"   { fwd!("DRAW_ACCEPTED"); }
        else if line == "DRAW_DECLINE"  { fwd!("DRAW_DECLINED"); }
        else if line == "RESIGN"        { fwd!("RESIGNED"); }
        else if line == "PING"          { send!("PONG"); }
        else {
            send!(format!("ERROR:Unknown command: {line}"));
        }
    }

    // ── Disconnect cleanup ─────────────────────────────────────────────────
    eprintln!("[server] disconnect {peer}");
    if let Some(ref tx) = opponent_tx {
        let _ = tx.send("DISCONNECTED".to_string());
    }
    if !room_code.is_empty() {
        rooms.lock().unwrap().remove(&room_code);
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut port = 9001u16;
    let mut host = "0.0.0.0".to_string();

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if let Some(v) = args.get(i + 1) {
                    port = v.parse().expect("invalid port"); i += 2;
                } else { i += 1; }
            }
            "--host" | "-H" => {
                if let Some(v) = args.get(i + 1) {
                    host = v.clone(); i += 2;
                } else { i += 1; }
            }
            _ => { i += 1; }
        }
    }

    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).expect("bind");
    println!("RChess relay server listening on {addr}");
    println!("Share your IP + this port with your friend.");

    let rooms: Rooms = Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let rooms = Arc::clone(&rooms);
                thread::spawn(move || handle_client(s, rooms));
            }
            Err(e) => eprintln!("[server] accept error: {e}"),
        }
    }
}
