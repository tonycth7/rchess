// src/network.rs — RChess online multiplayer client
//
// Architecture:
//   • A background thread owns the TcpStream read loop.
//   • Inbound messages arrive on  net_rx  (mpsc Receiver<NetMsg>).
//   • Outbound messages are sent via  net_tx  (mpsc Sender<String>),
//     which drives a second background write thread.
//   • The main game loop calls  poll_network()  every tick.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

/// Messages the network layer sends to the game.
#[derive(Debug, Clone)]
pub enum NetMsg {
    /// Server assigned us this room code (after CREATE).
    RoomCreated(String),
    /// We joined a room and were assigned this colour ("white"/"black").
    Joined { color: String },
    /// Opponent connected — game starts now.
    Start,
    /// Opponent made a move (UCI string like "e2e4" or "e7e8Q").
    OpponentMove(String),
    /// Opponent offered a draw.
    DrawOffer,
    /// Opponent accepted our draw offer.
    DrawAccepted,
    /// Opponent declined our draw offer.
    DrawDeclined,
    /// Opponent resigned.
    OpponentResigned,
    /// Opponent disconnected / connection lost.
    Disconnected,
    /// Server sent an error message.
    Error(String),
    /// Keep-alive round-trip completed.
    Pong,
}

/// Messages the game sends to the network layer.
#[derive(Debug, Clone)]
pub enum NetCmd {
    /// Disconnect cleanly.
    Disconnect,
    /// Raw line to send (already formatted).
    Raw(String),
}

pub struct NetClient {
    /// Receive inbound events.
    pub rx:  Receiver<NetMsg>,
    /// Send outbound commands.
    tx:      Sender<NetCmd>,
    pub connected: bool,
}

impl NetClient {
    /// Connect to `addr` (e.g. `"127.0.0.1:9001"` or `"chess.example.com:9001"`).
    /// Returns `Err(String)` if the TCP connection fails immediately.
    pub fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr)
            .map_err(|e| format!("Cannot connect to {addr}: {e}"))?;
        stream.set_read_timeout(Some(Duration::from_millis(100))).ok();

        let (in_tx,  in_rx)  = mpsc::channel::<NetMsg>();
        let (out_tx, out_rx) = mpsc::channel::<NetCmd>();

        // ── Write thread ──────────────────────────────────────────────────────
        let mut write_stream = stream.try_clone()
            .map_err(|e| format!("Clone stream: {e}"))?;
        thread::spawn(move || {
            loop {
                match out_rx.recv() {
                    Ok(NetCmd::Raw(line)) => {
                        let msg = format!("{line}\n");
                        if write_stream.write_all(msg.as_bytes()).is_err() { break; }
                    }
                    Ok(NetCmd::Disconnect) | Err(_) => break,
                }
            }
        });

        // ── Read thread ───────────────────────────────────────────────────────
        let reader = BufReader::new(stream);
        let in_tx2 = in_tx.clone();
        thread::spawn(move || {
            let mut lines = reader.lines();
            loop {
                match lines.next() {
                    Some(Ok(line)) => {
                        let line = line.trim().to_string();
                        if line.is_empty() { continue; }
                        let msg = parse_server_line(&line);
                        if matches!(msg, NetMsg::Disconnected) {
                            let _ = in_tx2.send(msg);
                            break;
                        }
                        if in_tx2.send(msg).is_err() { break; }
                    }
                    Some(Err(e)) if e.kind() == std::io::ErrorKind::WouldBlock
                                 || e.kind() == std::io::ErrorKind::TimedOut => {
                        // read timeout — keep looping
                    }
                    _ => {
                        let _ = in_tx2.send(NetMsg::Disconnected);
                        break;
                    }
                }
            }
        });

        Ok(NetClient { rx: in_rx, tx: out_tx, connected: true })
    }

    // ── Outbound helpers ──────────────────────────────────────────────────────

    pub fn send_create(&self) {
        self.raw("CREATE");
    }
    pub fn send_join(&self, room: &str) {
        self.raw(&format!("JOIN:{room}"));
    }
    pub fn send_move(&self, uci: &str) {
        self.raw(&format!("MOVE:{uci}"));
    }
    pub fn send_draw_offer(&self) {
        self.raw("DRAW");
    }
    pub fn send_draw_accept(&self) {
        self.raw("DRAW_ACCEPT");
    }
    pub fn send_draw_decline(&self) {
        self.raw("DRAW_DECLINE");
    }
    pub fn send_resign(&self) {
        self.raw("RESIGN");
    }
    pub fn send_ping(&self) {
        self.raw("PING");
    }

    pub fn disconnect(&mut self) {
        let _ = self.tx.send(NetCmd::Disconnect);
        self.connected = false;
    }

    fn raw(&self, s: &str) {
        let _ = self.tx.send(NetCmd::Raw(s.to_string()));
    }

    /// Drain all pending inbound messages (non-blocking).
    pub fn drain(&self) -> Vec<NetMsg> {
        let mut out = vec![];
        loop {
            match self.rx.try_recv() {
                Ok(msg) => out.push(msg),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    out.push(NetMsg::Disconnected);
                    break;
                }
            }
        }
        out
    }
}

/// Parse a single server line into a `NetMsg`.
fn parse_server_line(line: &str) -> NetMsg {
    if let Some(rest) = line.strip_prefix("ROOM:") {
        return NetMsg::RoomCreated(rest.trim().to_string());
    }
    if let Some(rest) = line.strip_prefix("JOINED:") {
        return NetMsg::Joined { color: rest.trim().to_lowercase() };
    }
    if line == "START" {
        return NetMsg::Start;
    }
    if let Some(rest) = line.strip_prefix("MOVE:") {
        return NetMsg::OpponentMove(rest.trim().to_string());
    }
    if line == "DRAW_OFFER" {
        return NetMsg::DrawOffer;
    }
    if line == "DRAW_ACCEPTED" {
        return NetMsg::DrawAccepted;
    }
    if line == "DRAW_DECLINED" {
        return NetMsg::DrawDeclined;
    }
    if line == "RESIGNED" {
        return NetMsg::OpponentResigned;
    }
    if line == "PONG" {
        return NetMsg::Pong;
    }
    if line == "DISCONNECTED" {
        return NetMsg::Disconnected;
    }
    if let Some(rest) = line.strip_prefix("ERROR:") {
        return NetMsg::Error(rest.trim().to_string());
    }
    // Unknown line — treat as informational / ignore.
    NetMsg::Pong
}
