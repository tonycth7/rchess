// server/main.rs — RChess relay server v2.0 (Tokio async)
//
// Usage:
//   rchess-server [OPTIONS]
//
//   --host   <addr>     Bind address           (default: 0.0.0.0)
//   --port   <n>        Game TCP port          (default: 9001)
//   --admin  <n>        Admin dashboard port   (default: 9002)
//   --data   <dir>      Data directory         (default: ./rchess_data)
//   --pw     <pass>     Server password        (required by clients)
//   --apw    <pass>     Admin panel password
//   --name   <str>      Server display name
//   --motd   <str>      Message of the day
//   --ttl    <mins>     Room TTL after disconnect (default: 120)
//   --public <host>     Public hostname/IP for invite links
//   --fresh             Delete all saved rooms on startup

#[path = "config.rs"]  mod config;
#[path = "auth.rs"]    mod auth;
#[path = "persist.rs"] mod persist;
#[path = "relay.rs"]   mod relay;
#[path = "admin.rs"]   mod admin;
#[path = "chat.rs"]    mod chat;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

use config::ServerConfig;
use relay::Rooms;
use auth::RateLimiter;
use admin::{AdminState, SharedAdmin};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut overrides: Vec<(&str, String)> = vec![];
    #[allow(unused_assignments)]
    let mut fresh_start = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--host"   => { if let Some(v) = args.get(i+1) { overrides.push(("host",        v.clone())); i += 2; } else { i += 1; } }
            "--port"   => { if let Some(v) = args.get(i+1) { overrides.push(("port",        v.clone())); i += 2; } else { i += 1; } }
            "--admin"  => { if let Some(v) = args.get(i+1) { overrides.push(("admin_port",  v.clone())); i += 2; } else { i += 1; } }
            "--data"   => { if let Some(v) = args.get(i+1) { overrides.push(("data_dir",    v.clone())); i += 2; } else { i += 1; } }
            "--pw"     => { if let Some(v) = args.get(i+1) { overrides.push(("server_password", v.clone())); i += 2; } else { i += 1; } }
            "--apw"    => { if let Some(v) = args.get(i+1) { overrides.push(("admin_password",  v.clone())); i += 2; } else { i += 1; } }
            "--name"   => { if let Some(v) = args.get(i+1) { overrides.push(("server_name", v.clone())); i += 2; } else { i += 1; } }
            "--motd"   => { if let Some(v) = args.get(i+1) { overrides.push(("motd",        v.clone())); i += 2; } else { i += 1; } }
            "--ttl"    => { if let Some(v) = args.get(i+1) { overrides.push(("room_ttl_mins", v.clone())); i += 2; } else { i += 1; } }
            "--public" => { if let Some(v) = args.get(i+1) { overrides.push(("public_host", v.clone())); i += 2; } else { i += 1; } }
            "--fresh" | "-f" => { fresh_start = true; i += 1; }
            "--help"  | "-h" => { print_help(); return; }
            _ => { i += 1; }
        }
    }

    let owned: Vec<(&str, &str)> = overrides.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let cfg = ServerConfig::load(&owned);
    ServerConfig::write_default_if_missing();

    // Data directory
    std::fs::create_dir_all(&cfg.data_dir).expect("create data dir");

    if fresh_start {
        let mut n = 0usize;
        if let Ok(entries) = std::fs::read_dir(&cfg.data_dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("room") {
                    if std::fs::remove_file(&p).is_ok() { n += 1; }
                }
            }
        }
        eprintln!("[server] --fresh: deleted {} room files", n);
    }

    // Load persisted rooms
    let initial: HashMap<String, relay::Room> = persist::load_all_rooms(&cfg);
    let loaded = initial.len();
    let rooms: Rooms = Arc::new(Mutex::new(initial));

    let admin: SharedAdmin = Arc::new(Mutex::new(AdminState::new()));
    let limiter = RateLimiter::new(cfg.max_conns_per_ip, 60);
    let cfg     = Arc::new(cfg);
    let data_dir = Arc::new(std::path::PathBuf::from(&cfg.data_dir));

    // ── Banner ────────────────────────────────────────────────────────────────
    let link_host = cfg.link_host();
    println!();
    println!("  ♟  RChess Server v2.0");
    println!("  ─────────────────────────────────────────────");
    println!("  Game port    : {}:{}", cfg.host, cfg.port);
    println!("  Admin panel  : http://{}:{}", cfg.host, cfg.admin_port);
    println!("  Data dir     : {}", cfg.data_dir);
    println!("  Rooms loaded : {}", loaded);
    println!("  Link host    : {}  (used in invite links)", link_host);
    if cfg.server_password.is_some() {
        println!("  Auth         : 🔒 Password required");
    } else {
        println!("  Auth         : 🔓 Open (no password)");
    }
    if !cfg.motd.is_empty() {
        println!("  MOTD         : {}", cfg.motd);
    }
    println!("  ─────────────────────────────────────────────");
    println!();

    // ── Background: admin HTTP server (std thread — blocking is fine) ─────────
    {
        let cfg_a    = Arc::clone(&cfg);
        let admin_a  = Arc::clone(&admin);
        let data_a   = Arc::clone(&data_dir);
        std::thread::spawn(move || admin::run_admin_server(cfg_a, admin_a, data_a));
    }

    // ── Background: rate-limiter purge every 2 minutes ────────────────────────
    {
        let lim2 = limiter.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                lim2.purge_expired();
            }
        });
    }

    // ── Background: admin room snapshot every 5s ──────────────────────────────
    {
        let rooms2 = Arc::clone(&rooms);
        let admin2 = Arc::clone(&admin);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let snap: Vec<admin::RoomInfo> = rooms2.lock().unwrap().iter().map(|(code, r)| admin::RoomInfo {
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
                }).collect();
                admin2.lock().unwrap().rooms = snap;
            }
        });
    }

    // ── Main accept loop ──────────────────────────────────────────────────────
    let addr     = format!("{}:{}", cfg.host, cfg.port);
    let listener = TcpListener::bind(&addr).await.expect("bind game port");
    println!("[server] ready on {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let rooms    = Arc::clone(&rooms);
                let data_dir = Arc::clone(&data_dir);
                let cfg      = Arc::clone(&cfg);
                let limiter  = limiter.clone();
                let admin    = Arc::clone(&admin);
                tokio::spawn(relay::handle_client(stream, rooms, data_dir, cfg, limiter, admin));
            }
            Err(e) => eprintln!("[server] accept error: {}", e),
        }
    }
}

fn print_help() {
    println!("RChess Relay Server v2.0 (Tokio async)");
    println!();
    println!("USAGE:  rchess-server [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --host   <addr>   Bind address              (default: 0.0.0.0)");
    println!("  --port   <n>      Game TCP port             (default: 9001)");
    println!("  --admin  <n>      Admin dashboard port      (default: 9002)");
    println!("  --data   <dir>    Data directory            (default: ./rchess_data)");
    println!("  --pw     <pass>   Require server password");
    println!("  --apw    <pass>   Admin panel password");
    println!("  --name   <str>    Server display name");
    println!("  --motd   <str>    Message of the day");
    println!("  --ttl    <mins>   Room TTL after disconnect (default: 120)");
    println!("  --public <host>   Public hostname for invite links");
    println!("  --fresh           Delete all saved room files on startup");
    println!("  --help            Show this help");
    println!();
    println!("ADMIN:   http://<host>:<admin_port>   (auto-refreshes every 5s)");
    println!("CONFIG:  ~/.config/rchess/server.toml");
    println!();
    println!("INVITE LINKS:");
    println!("  When a room is created, the server sends an invite link:");
    println!("  rc1:base64(host:port:ROOMCODE)");
    println!("  Paste this in the rchess TUI (Online → Join → paste link)");
    println!();
    println!("DEPLOYMENT:");
    println!("  Docker:   docker run -p 9001:9001 -p 9002:9002 rchess-server");
    println!("  Systemd:  see DEPLOYMENT.md");
}
