// server/admin.rs — Powerful HTTP admin dashboard
//
// GET  /                        HTML live dashboard (auto-refresh)
// GET  /api/status              JSON server stats
// GET  /api/rooms               JSON room list
// GET  /api/log                 JSON event log
// POST /api/rooms/<CODE>/kick   Kick room immediately + clear state
// POST /api/rooms/<CODE>/message  Send server message to room
// POST /api/note                Set server note
// POST /api/motd                Update MOTD
// POST /api/clear-rooms         Delete all room files
// POST /api/broadcast           Broadcast server message to all rooms

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::config::ServerConfig;

pub type WriteTx = tokio::sync::mpsc::Sender<String>;

#[derive(Clone, Default)]
pub struct RoomInfo {
    pub code:       String,
    pub moves:      usize,
    pub white_conn: bool,
    pub black_conn: bool,
    pub white_name: String,
    pub black_name: String,
    pub time_secs:  u32,
    pub inc_secs:   u32,
    pub undo:       bool,
    pub engine:     bool,
    pub started:    bool,
    pub chat_msgs:  usize,
}

pub struct AdminState {
    pub rooms:        Vec<RoomInfo>,
    pub total_games:  u64,
    pub total_moves:  u64,
    pub total_conns:  u64,
    pub uptime_start: Instant,
    pub log:          VecDeque<String>,
    pub kick_queue:   Vec<String>,
    pub room_txs:     std::collections::HashMap<String, (Option<WriteTx>, Option<WriteTx>)>,
    pub server_note:  String,
    pub live_motd:    String,
}

impl AdminState {
    pub fn new() -> Self {
        AdminState {
            rooms: vec![], total_games: 0, total_moves: 0, total_conns: 0,
            uptime_start: Instant::now(),
            log: VecDeque::new(),
            kick_queue: vec![],
            room_txs: std::collections::HashMap::new(),
            server_note: String::new(),
            live_motd: String::new(),
        }
    }
    pub fn push_log(&mut self, msg: String) {
        if self.log.len() >= 500 { self.log.pop_front(); }
        let secs = self.uptime_start.elapsed().as_secs();
        let h = secs / 3600; let m = (secs % 3600) / 60; let s = secs % 60;
        self.log.push_back(format!("[{h:02}:{m:02}:{s:02}] {msg}"));
    }
    /// Directly push KICKED to both players. Returns true if live connections were found.
    pub fn kick_room_direct(&mut self, code: &str) -> bool {
        let found = if let Some((w, b)) = self.room_txs.get(code) {
            let msg = "KICKED:Disconnected by server admin".to_string();
            if let Some(ref tx) = w { let _ = tx.try_send(msg.clone()); }
            if let Some(ref tx) = b { let _ = tx.try_send(msg); }
            w.is_some() || b.is_some()
        } else { false };
        self.kick_queue.push(code.to_string());
        self.room_txs.remove(code);
        found
    }
    pub fn send_to_room(&self, code: &str, msg: &str) {
        if let Some((w, b)) = self.room_txs.get(code) {
            let m = format!("SERVER_MSG:{}", msg);
            if let Some(ref tx) = w { let _ = tx.try_send(m.clone()); }
            if let Some(ref tx) = b { let _ = tx.try_send(m); }
        }
    }
    pub fn broadcast(&self, msg: &str) {
        for (_, (w, b)) in &self.room_txs {
            let m = format!("SERVER_MSG:{}", msg);
            if let Some(ref tx) = w { let _ = tx.try_send(m.clone()); }
            if let Some(ref tx) = b { let _ = tx.try_send(m); }
        }
    }
}

pub type SharedAdmin = Arc<Mutex<AdminState>>;

// ── HTTP admin server (std threads — intentionally not async) ────────────────

pub fn run_admin_server(cfg: Arc<ServerConfig>, state: SharedAdmin, data_dir: Arc<PathBuf>) {
    let addr = format!("{}:{}", cfg.host, cfg.admin_port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l)  => { eprintln!("[admin] dashboard → http://{}", addr); l }
        Err(e) => { eprintln!("[admin] cannot bind {}: {}", addr, e); return; }
    };
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();
        stream.set_write_timeout(Some(std::time::Duration::from_secs(5))).ok();
        let state    = Arc::clone(&state);
        let cfg      = Arc::clone(&cfg);
        let data_dir = Arc::clone(&data_dir);
        std::thread::spawn(move || handle_admin_request(&mut stream, state, cfg, data_dir));
    }
}

fn handle_admin_request(
    stream:   &mut std::net::TcpStream,
    state:    SharedAdmin,
    cfg:      Arc<ServerConfig>,
    data_dir: Arc<PathBuf>,
) {
    let mut buf = vec![0u8; 16384];
    let n = match stream.read(&mut buf) { Ok(n) => n, Err(_) => return };
    let req  = String::from_utf8_lossy(&buf[..n]).to_string();
    let fl   = req.lines().next().unwrap_or("");
    let meth = fl.split_whitespace().next().unwrap_or("GET");
    let path = fl.split_whitespace().nth(1).unwrap_or("/");
    let path_base = path.split('?').next().unwrap_or("/");

    let body = req.find("\r\n\r\n").map(|p| req[p+4..].trim().to_string()).unwrap_or_default();

    // Auth check
    let authed = match &cfg.admin_password {
        None     => true,
        Some(pw) => req.contains(&format!("pw={}", pw))
                 || req.contains(&format!("Authorization: Bearer {}", pw))
                 || body.starts_with(&format!("pw={}", pw)),
    };
    if !authed {
        respond(stream, "401 Unauthorized", "text/plain",
            "Unauthorized — add ?pw=<admin_password> to the URL");
        return;
    }

    let pw_param = cfg.admin_password.as_deref()
        .map(|p| format!("?pw={}", p)).unwrap_or_default();

    match (meth, path_base) {
        ("GET",  "/")              => respond(stream, "200 OK", "text/html",    &build_dashboard(&state, &cfg, &pw_param)),
        ("GET",  "/api/status")    => respond(stream, "200 OK", "application/json", &json_status(&state, &cfg)),
        ("GET",  "/api/rooms")     => respond(stream, "200 OK", "application/json", &json_rooms(&state)),
        ("GET",  "/api/log")       => respond(stream, "200 OK", "application/json", &json_log(&state)),

        ("POST", p) if p.starts_with("/api/rooms/") && p.ends_with("/kick") => {
            let code = &p["/api/rooms/".len()..p.len()-"/kick".len()];
            let code = code.to_uppercase();
            let found = state.lock().unwrap().kick_room_direct(&code);
            state.lock().unwrap().push_log(format!("[admin] KICKED room {} (live={})", code, found));
            let json = format!(r#"{{"ok":true,"kicked":"{}","had_players":{}}}"#, code, found);
            respond(stream, "200 OK", "application/json", &json);
        }
        ("POST", p) if p.starts_with("/api/rooms/") && p.ends_with("/message") => {
            let code = &p["/api/rooms/".len()..p.len()-"/message".len()];
            let code = code.to_uppercase();
            let msg  = body.chars().take(200).collect::<String>();
            state.lock().unwrap().send_to_room(&code, &msg);
            state.lock().unwrap().push_log(format!("[admin] sent message to room {}: {}", code, &msg[..msg.len().min(40)]));
            respond(stream, "200 OK", "application/json", r#"{"ok":true}"#);
        }
        ("POST", "/api/broadcast") => {
            let msg = body.chars().take(200).collect::<String>();
            state.lock().unwrap().broadcast(&msg);
            state.lock().unwrap().push_log(format!("[admin] broadcast: {}", &msg[..msg.len().min(60)]));
            respond(stream, "200 OK", "application/json", r#"{"ok":true}"#);
        }
        ("POST", "/api/note") => {
            let note = body.chars().take(500).collect::<String>();
            { let mut a = state.lock().unwrap(); a.server_note = note.clone(); a.push_log(format!("[admin] note updated")); }
            respond(stream, "200 OK", "application/json", r#"{"ok":true}"#);
        }
        ("POST", "/api/motd") => {
            let motd = body.chars().take(300).collect::<String>();
            { let mut a = state.lock().unwrap(); a.live_motd = motd.clone(); a.push_log(format!("[admin] MOTD updated: {}", &motd[..motd.len().min(40)])); }
            respond(stream, "200 OK", "application/json", r#"{"ok":true}"#);
        }
        ("POST", "/api/clear-rooms") => {
            let mut deleted = 0usize;
            if let Ok(entries) = std::fs::read_dir(data_dir.as_ref()) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|x| x.to_str()) == Some("room") {
                        if std::fs::remove_file(&p).is_ok() { deleted += 1; }
                    }
                }
            }
            state.lock().unwrap().push_log(format!("[admin] cleared {} room files", deleted));
            respond(stream, "200 OK", "application/json", &format!(r#"{{"ok":true,"deleted":{}}}"#, deleted));
        }
        _ => respond(stream, "404 Not Found", "text/plain", "Not found"),
    }
}

fn respond(stream: &mut std::net::TcpStream, status: &str, ct: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}; charset=utf-8\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n{}",
        status, ct, body.len(), body
    );
    let _ = stream.write_all(resp.as_bytes());
}

fn json_status(state: &SharedAdmin, cfg: &ServerConfig) -> String {
    let s = state.lock().unwrap();
    let up = s.uptime_start.elapsed().as_secs();
    format!(
        r#"{{"server_name":"{}","rooms":{},"total_games":{},"total_moves":{},"total_conns":{},"uptime_secs":{}}}"#,
        esc_json(&cfg.server_name), s.rooms.len(), s.total_games, s.total_moves, s.total_conns, up
    )
}

fn json_rooms(state: &SharedAdmin) -> String {
    let s = state.lock().unwrap();
    let parts: Vec<String> = s.rooms.iter().map(|r| format!(
        r#"{{"code":"{}","moves":{},"white_conn":{},"black_conn":{},"white_name":"{}","black_name":"{}","started":{},"time_secs":{},"chat_msgs":{}}}"#,
        r.code, r.moves, r.white_conn, r.black_conn,
        esc_json(&r.white_name), esc_json(&r.black_name),
        r.started, r.time_secs, r.chat_msgs
    )).collect();
    format!("[{}]", parts.join(","))
}

fn json_log(state: &SharedAdmin) -> String {
    let s = state.lock().unwrap();
    let parts: Vec<String> = s.log.iter().rev().take(100)
        .map(|l| format!(r#""{}""#, esc_json(l))).collect();
    format!("[{}]", parts.join(","))
}

fn esc_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn html_esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn build_dashboard(state: &SharedAdmin, cfg: &ServerConfig, pw_param: &str) -> String {
    let s = state.lock().unwrap();
    let up   = s.uptime_start.elapsed().as_secs();
    let uh   = up / 3600; let um = (up % 3600) / 60; let us = up % 60;
    let name = html_esc(&cfg.server_name);
    let note = html_esc(&s.server_note);
    let motd = html_esc(&s.live_motd);
    let auth_cls = if cfg.server_password.is_some() { "locked" } else { "open" };
    let auth_lbl = if cfg.server_password.is_some() { "🔒 Password protected" } else { "🔓 Open access" };

    let room_rows: String = s.rooms.iter().map(|r| {
        let w  = if r.white_conn { "🟢" } else { "⚪" };
        let b  = if r.black_conn { "🟢" } else { "⚪" };
        let st = if r.started { "<span class='badge open'>active</span>" } else { "<span class='badge waiting'>waiting</span>" };
        let t  = if r.time_secs == 0 { "∞".to_string() } else { format!("{}m", r.time_secs/60) };
        let wn = if r.white_name.is_empty() { "White".to_string() } else { html_esc(&r.white_name) };
        let bn = if r.black_name.is_empty() { "Black".to_string() } else { html_esc(&r.black_name) };
        format!(
            "<tr>\
              <td><code>{code}</code></td>\
              <td>{w} {wn}</td>\
              <td>{b} {bn}</td>\
              <td class='num'>{mv}</td>\
              <td class='num'>{cm}</td>\
              <td>{st}</td>\
              <td>{t}+{inc}s</td>\
              <td class='actions'>\
                <button class='btn btn-kick' onclick=\"kickRoom('{code}')\">⚡ Kick</button>\
                <button class='btn btn-msg' onclick=\"msgRoom('{code}')\">📨 Msg</button>\
              </td>\
            </tr>",
            code=r.code, w=w, wn=wn, b=b, bn=bn,
            mv=r.moves, cm=r.chat_msgs, st=st, t=t, inc=r.inc_secs
        )
    }).collect();

    let log_rows: String = s.log.iter().rev().take(80).map(|l| {
        let col = if l.contains("KICK") || l.contains("kick") { "var(--red)" }
                  else if l.contains("disconnect") || l.contains("expire") { "var(--yellow)" }
                  else if l.contains("started") || l.contains("created") { "var(--green)" }
                  else if l.contains("broadcast") || l.contains("message") { "var(--purple)" }
                  else { "var(--dim)" };
        format!("<div class='ll' style='color:{}'>{}</div>", col, html_esc(l))
    }).collect();

    let rooms_len   = s.rooms.len();
    let tg = s.total_games; let tm = s.total_moves; let tc = s.total_conns;
    drop(s);

    format!(r###"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{name} — RChess Admin</title>
<style>
:root{{--bg:#0d1117;--bg2:#161b22;--bg3:#21262d;--acc:#58a6ff;--green:#3fb950;--red:#f85149;--yellow:#d29922;--purple:#bc8cff;--txt:#c9d1d9;--dim:#484f58;--border:#30363d}}
*{{box-sizing:border-box;margin:0;padding:0}}
body{{font-family:'Courier New',monospace;background:var(--bg);color:var(--txt);font-size:13px;padding:16px}}
h1{{color:var(--acc);font-size:1.3em;margin-bottom:4px;display:flex;align-items:center;gap:8px}}
.sub{{color:var(--dim);font-size:.78em;margin-bottom:16px;display:flex;gap:12px;flex-wrap:wrap;align-items:center}}
.cards{{display:grid;grid-template-columns:repeat(auto-fit,minmax(120px,1fr));gap:8px;margin-bottom:16px}}
.card{{background:var(--bg2);border:1px solid var(--border);border-radius:8px;padding:12px;text-align:center}}
.card .v{{font-size:1.8em;font-weight:bold;color:var(--acc)}}
.card .l{{font-size:.72em;color:var(--dim);margin-top:3px}}
table{{width:100%;border-collapse:collapse;background:var(--bg2);border-radius:8px;overflow:hidden;margin-bottom:14px}}
th{{background:var(--bg3);color:var(--dim);font-size:.72em;padding:6px 10px;text-align:left;letter-spacing:.05em}}
td{{padding:6px 10px;border-top:1px solid var(--border);font-size:.82em;vertical-align:middle}}
td.num{{color:var(--acc);font-weight:bold}} td.actions{{display:flex;gap:4px}}
tr:hover td{{background:var(--bg3)}}
code{{color:var(--acc);letter-spacing:.1em;font-size:1.05em}}
.badge{{display:inline-block;padding:2px 8px;border-radius:4px;font-size:.72em;font-weight:bold}}
.open{{background:#0d2a1a;color:var(--green)}} .locked{{background:#2a0d0d;color:var(--red)}}
.waiting{{background:#1c1c0a;color:var(--yellow)}}
.btn{{background:var(--bg3);color:var(--txt);border:1px solid var(--border);border-radius:4px;padding:3px 10px;cursor:pointer;font-size:.75em;font-family:inherit;transition:all .15s}}
.btn:hover{{background:var(--bg2);border-color:var(--acc);color:var(--acc)}}
.btn-kick{{background:#2d0f0f;color:var(--red);border-color:#5a2020}} .btn-kick:hover{{background:var(--red);color:#fff;border-color:var(--red)}}
.btn-msg{{background:#0d1a2d;color:var(--acc);border-color:#1a3a5a}} .btn-msg:hover{{background:var(--acc);color:#000}}
.btn-danger{{background:#2d0f0f;color:var(--red);border-color:#5a2020}}
.btn-ok{{background:#0d2a1a;color:var(--green);border-color:#1a5a30}}
.btn-purple{{background:#1a0d2d;color:var(--purple);border-color:#3a1a5a}}
.box{{background:var(--bg2);border:1px solid var(--border);border-radius:8px;padding:12px;margin-bottom:14px}}
.box h3{{color:var(--dim);font-size:.75em;letter-spacing:.06em;margin-bottom:8px;text-transform:uppercase}}
.row{{display:flex;gap:6px;align-items:center;margin-top:6px}}
.inp{{background:var(--bg3);border:1px solid var(--border);color:var(--txt);padding:6px 10px;border-radius:4px;font-family:inherit;font-size:.85em;flex:1;outline:none;transition:border-color .15s}}
.inp:focus{{border-color:var(--acc)}}
.log-panel{{background:var(--bg2);border:1px solid var(--border);border-radius:8px;padding:10px;max-height:280px;overflow-y:auto;margin-bottom:14px;font-size:.75em}}
.ll{{padding:2px 0;border-bottom:1px solid var(--border);white-space:pre-wrap;word-break:break-all}}
.stitle{{color:var(--dim);font-size:.78em;letter-spacing:.06em;margin-bottom:6px;margin-top:14px;text-transform:uppercase;border-bottom:1px solid var(--border);padding-bottom:4px}}
.api-links{{color:var(--dim);font-size:.72em;margin-top:12px}}
.api-links a{{color:var(--acc);text-decoration:none;margin-right:12px}}
.api-links a:hover{{text-decoration:underline}}
.pulse{{animation:pulse 2s infinite}} @keyframes pulse{{0%,100%{{opacity:1}} 50%{{opacity:.5}}}}
</style>
</head>
<body>
<h1>♟ {name} <span style="font-size:.6em;color:var(--dim)">admin</span></h1>
<div class="sub">
  <span class="badge {auth_cls}">{auth_lbl}</span>
  <span>Game <strong style="color:var(--acc)">:{port}</strong></span>
  <span>Admin <strong style="color:var(--acc)">:{aport}</strong></span>
  <span class="pulse" style="color:var(--dim);font-size:.85em">● live</span>
  <span style="margin-left:auto;color:var(--dim)">{uh:02}:{um:02}:{us:02} uptime</span>
</div>

<div class="cards">
  <div class="card"><div class="v">{rooms_len}</div><div class="l">Active Rooms</div></div>
  <div class="card"><div class="v">{tg}</div><div class="l">Total Games</div></div>
  <div class="card"><div class="v">{tm}</div><div class="l">Total Moves</div></div>
  <div class="card"><div class="v">{tc}</div><div class="l">Connections</div></div>
</div>

<div class="box">
  <h3>📢 MOTD (shown to connecting players)</h3>
  <div style="color:var(--yellow);font-size:.9em;min-height:1.4em">{motd_disp}</div>
  <div class="row">
    <input class="inp" id="motd-in" value="{motd}" placeholder="Message of the day...">
    <button class="btn btn-ok" onclick="api('/api/motd','motd-in')">Save</button>
  </div>
  <h3 style="margin-top:10px">📋 Server Note (admin only)</h3>
  <div style="color:var(--yellow);font-size:.9em;min-height:1.4em">{note_disp}</div>
  <div class="row">
    <input class="inp" id="note-in" value="{note}" placeholder="Internal notes...">
    <button class="btn btn-ok" onclick="api('/api/note','note-in')">Save</button>
  </div>
  <h3 style="margin-top:10px">📡 Broadcast to all rooms</h3>
  <div class="row">
    <input class="inp" id="bcast-in" placeholder="Message all connected players...">
    <button class="btn btn-purple" onclick="broadcast()">Broadcast</button>
  </div>
</div>

<div class="stitle">Active Rooms ({rooms_len})</div>
<table>
  <thead><tr>
    <th>Code</th><th>White</th><th>Black</th>
    <th>Moves</th><th>Chat</th><th>Status</th><th>Time</th><th>Actions</th>
  </tr></thead>
  <tbody id="room-tbody">{room_rows}</tbody>
</table>
<div style="margin-bottom:14px;display:flex;gap:8px;align-items:center">
  <button class="btn btn-danger" onclick="clearRooms()">🗑 Clear persisted rooms</button>
  <span style="color:var(--dim);font-size:.75em">Deletes .room files — does not disconnect live games</span>
</div>

<div class="stitle">Event Log</div>
<div class="log-panel" id="log">{log_rows}</div>

<div class="api-links">
  REST API:
  <a href="/api/status{pw_param}">/status</a>
  <a href="/api/rooms{pw_param}">/rooms</a>
  <a href="/api/log{pw_param}">/log</a>
</div>

<script>
const PW = '{pw_param}';
async function post(url, body='') {{
  const r = await fetch(url + PW, {{method:'POST', headers:{{'Content-Type':'text/plain'}}, body}});
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}}
async function api(endpoint, inputId) {{
  const v = document.getElementById(inputId).value;
  try {{ await post(endpoint, v); flash('Saved!'); }} catch(e) {{ alert('Error: '+e); }}
}}
function kickRoom(code) {{
  if (!confirm('Kick room ' + code + '?\nBoth players will be IMMEDIATELY disconnected.')) return;
  post('/api/rooms/' + code + '/kick')
    .then(d => {{
      flash('⚡ Kicked: ' + d.kicked + (d.had_players ? ' (players notified)' : ' (no live players)'));
      document.querySelectorAll('#room-tbody tr').forEach(tr => {{
        const codeEl = tr.querySelector('code');
        if (codeEl && codeEl.textContent === code) {{
          tr.style.transition = 'opacity 0.3s';
          tr.style.opacity = '0';
          setTimeout(() => tr.remove(), 300);
        }}
      }});
    }})
    .catch(e => alert('Kick failed: ' + e));
}}
function msgRoom(code) {{
  const msg = prompt('Message to send to room '+code+':');
  if (!msg) return;
  post('/api/rooms/'+code+'/message', msg)
    .then(() => flash('Message sent'))
    .catch(e => alert('Error: '+e));
}}
function broadcast() {{
  const msg = document.getElementById('bcast-in').value;
  if (!msg) return;
  post('/api/broadcast', msg)
    .then(() => {{ document.getElementById('bcast-in').value=''; flash('Broadcast sent!'); }})
    .catch(e => alert('Error: '+e));
}}
function clearRooms() {{
  if (!confirm('Delete all persisted room files?')) return;
  post('/api/clear-rooms').then(d => {{ flash('Deleted '+d.deleted+' files'); reload(); }}).catch(e=>alert(e));
}}
function flash(msg) {{
  const el = document.createElement('div');
  el.textContent = '✓ '+msg;
  el.style.cssText='position:fixed;bottom:20px;right:20px;background:var(--green);color:#000;padding:8px 16px;border-radius:6px;font-weight:bold;z-index:999;transition:opacity .5s';
  document.body.appendChild(el);
  setTimeout(()=>{{el.style.opacity=0;setTimeout(()=>el.remove(),500)}},2000);
}}
function reload() {{ setTimeout(()=>location.reload(),800); }}
async function liveRefresh() {{
  try {{
    const [rooms, status] = await Promise.all([
      fetch('/api/rooms'+PW).then(r=>r.json()),
      fetch('/api/status'+PW).then(r=>r.json()),
    ]);
    const vals = document.querySelectorAll('.card .v');
    if (vals.length >= 4) {{
      vals[0].textContent = rooms.length;
      vals[1].textContent = status.total_games;
      vals[2].textContent = status.total_moves;
      vals[3].textContent = status.total_conns;
    }}
    const tbody = document.getElementById('room-tbody');
    if (tbody) {{
      if (rooms.length === 0) {{
        tbody.innerHTML = '<tr><td colspan="8" style="color:var(--dim);text-align:center;padding:16px">No active rooms</td></tr>';
      }} else {{
        tbody.innerHTML = rooms.map(r => {{
          const t = r.time_secs===0 ? '∞' : Math.floor(r.time_secs/60)+'m';
          const st = r.started
            ? '<span class="badge open">active</span>'
            : '<span class="badge waiting">waiting</span>';
          const wn = r.white_name||'White';
          const bn = r.black_name||'Black';
          const wc = r.white_conn ? '🟢' : '⚪';
          const bc = r.black_conn ? '🟢' : '⚪';
          return '<tr>'+
            '<td><code>'+r.code+'</code></td>'+
            '<td>'+wc+' '+wn+'</td>'+
            '<td>'+bc+' '+bn+'</td>'+
            '<td class="num">'+r.moves+'</td>'+
            '<td class="num">'+r.chat_msgs+'</td>'+
            '<td>'+st+'</td>'+
            '<td>'+t+'</td>'+
            '<td class="actions">'+
              '<button class="btn btn-kick" onclick="kickRoom(\''+r.code+'\')">⚡ Kick</button>'+
              '<button class="btn btn-msg"  onclick="msgRoom(\''+r.code+'\')">📨 Msg</button>'+
            '</td></tr>';
        }}).join('');
      }}
    }}
  }} catch(e) {{ /* ignore */ }}
  setTimeout(liveRefresh, 3000);
}}
setTimeout(liveRefresh, 3000);
const log = document.getElementById('log');
if (log) log.scrollTop = 0;
</script>
</body></html>"###,
        name=name, auth_cls=auth_cls, auth_lbl=auth_lbl,
        port=cfg.port, aport=cfg.admin_port,
        rooms_len=rooms_len, tg=tg, tm=tm, tc=tc,
        uh=uh, um=um, us=us,
        motd_disp = if motd.is_empty() { "<em style='color:var(--dim)'>(empty)</em>".to_string() } else { motd.clone() },
        note_disp = if note.is_empty() { "<em style='color:var(--dim)'>(none)</em>".to_string() } else { note.clone() },
        motd=motd, note=note,
        room_rows=room_rows, log_rows=log_rows,
        pw_param=pw_param,
    )
}
