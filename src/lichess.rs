// src/lichess.rs — Lichess API client for online play
//
// Uses ureq (native-tls) for HTTP. NDJSON streams run in background threads,
// same pattern as ai.rs / analysis.rs.

use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use crate::http::{agent, read_text, streaming_agent};

const BASE: &str = "https://lichess.org";

/// Agent for the NDJSON event/game streams: long-lived, so no global timeout.
fn make_agent() -> ureq::Agent {
    streaming_agent(Duration::from_secs(10), Duration::from_secs(30))
}

/// Agent for ordinary request/response calls.
fn make_agent_timeout(connect_secs: u64, total_secs: u64) -> ureq::Agent {
    agent(
        Duration::from_secs(connect_secs),
        Some(Duration::from_secs(total_secs)),
    )
}

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Challenge {
    pub id:                String,
    pub challenger_name:   String,
    pub challenger_rating: Option<i32>,
    pub variant:           String,
    pub time_control:      String,
    pub color:             Option<String>,
}

#[derive(Debug, Clone)]
pub enum LichessEvent {
    ChallengeReceived(Challenge),
    ChallengeCanceled(String),
    ChallengeDeclined(String),
    GameStarted { game_id: String, opponent_name: String, opponent_rating: Option<i32>, color: crate::engine::Color },
    GameFinished { game_id: String, result: String },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum GameStreamEvent {
    GameFull { fen: String, moves: Vec<String>, w_ms: u64, b_ms: u64, w_name: String, b_name: String, w_rating: Option<i32>, b_rating: Option<i32> },
    MovePlayed { uci: String, w_ms: u64, b_ms: u64, move_number: u32 },
    GameEnd { result: String, winner: Option<crate::engine::Color> },
    ChatLine { text: String, room: String },
    Error(String),
}

pub type ApiResult<T> = Result<T, String>;

// ── API client ─────────────────────────────────────────────────────────────────

pub struct LichessClient {
    token: String,
    agent: ureq::Agent,
}

impl LichessClient {
    pub fn new(token: &str) -> Self {
        let agent = make_agent_timeout(10, 30);
        LichessClient { token: token.to_string(), agent }
    }

    /// Authenticated GET returning the response body as text.
    fn get_text(&self, path: &str) -> ApiResult<String> {
        let mut resp = self
            .agent
            .get(format!("{BASE}{path}"))
            .header("Authorization", format!("Bearer {}", self.token))
            .call()
            .map_err(|e| format!("HTTP error: {e}"))?;
        read_text(&mut resp)
    }

    /// Authenticated POST with no body, discarding the response.
    fn post(&self, path: &str) -> ApiResult<()> {
        let mut resp = self
            .agent
            .post(format!("{BASE}{path}"))
            .header("Authorization", format!("Bearer {}", self.token))
            .send_empty()
            .map_err(|e| format!("HTTP error: {e}"))?;
        read_text(&mut resp).map(|_| ())
    }

    pub fn get_username(&self) -> ApiResult<String> {
        let body = self.get_text("/api/account")?;
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("json: {e}"))?;
        v["username"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "no username field".into())
    }

    pub fn fetch_daily_puzzle(&self) -> ApiResult<String> {
        self.get_text("/api/puzzle/daily")
    }

    pub fn spawn_event_stream(&self) -> (mpsc::Receiver<LichessEvent>, Arc<AtomicBool>) {
        let (tx, rx) = mpsc::channel();
        let token = self.token.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let sd = shutdown.clone();
        std::thread::spawn(move || {
            let agent = make_agent();
            let mut delay = 1;
            loop {
                if sd.load(Ordering::Relaxed) { return; }
                match agent.get(format!("{BASE}/api/stream/event"))
                    .header("Authorization", format!("Bearer {token}"))
                    .call()
                {
                    Ok(mut resp) => {
                        if !resp.status().is_success() {
                            let status = resp.status();
                            let body = resp.body_mut().read_to_string().unwrap_or_default();
                            rlog!("[lichess] event stream HTTP {status}: {body}");
                            let _ = tx.send(LichessEvent::Error(format!(
                                "Stream error: HTTP {status}: {body}"
                            )));
                        } else {
                        delay = 1;
                        let reader = resp.into_body().into_reader();
                        let buf = std::io::BufReader::new(reader);
                        for line_result in buf.lines() {
                            if sd.load(Ordering::Relaxed) { return; }
                            match line_result {
                                Ok(line) => {
                                    if line.trim().is_empty() { continue; }
                                    if let Ok(Some(ev)) = parse_event_line(&line) {
                                        if tx.send(ev).is_err() { return; }
                                    }
                                }
                                Err(e) => {
                                    rlog!("[lichess] stream read: {e}");
                                    break;
                                }
                            }
                        }
                        }
                    }
                    Err(e) => {
                        rlog!("[lichess] event stream error: {e}");
                        let _ = tx.send(LichessEvent::Error(format!("Stream error: {e}")));
                    }
                }
                if sd.load(Ordering::Relaxed) { return; }
                std::thread::sleep(std::time::Duration::from_secs(delay));
                delay = (delay * 2).min(60);
            }
        });
        (rx, shutdown)
    }

    pub fn spawn_game_stream(&self, game_id: &str) -> (mpsc::Receiver<GameStreamEvent>, Arc<AtomicBool>) {
        let (tx, rx) = mpsc::channel();
        let token = self.token.clone();
        let gid = game_id.to_string();
        let shutdown = Arc::new(AtomicBool::new(false));
        let sd = shutdown.clone();
        std::thread::spawn(move || {
            let agent = make_agent();
            let mut delay = 1;
            loop {
                if sd.load(Ordering::Relaxed) { return; }
                match agent.get(format!("{BASE}/api/board/game/stream/{gid}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .call()
                {
                    Ok(mut resp) => {
                        if !resp.status().is_success() {
                            let status = resp.status();
                            let body = resp.body_mut().read_to_string().unwrap_or_default();
                            rlog!("[lichess] game stream HTTP {status}: {body}");
                            let _ = tx.send(GameStreamEvent::Error(format!(
                                "HTTP {status}: {body}"
                            )));
                        } else {
                        delay = 1;
                        let reader = resp.into_body().into_reader();
                        let buf = std::io::BufReader::new(reader);
                        let mut ended = false;
                        for line_result in buf.lines() {
                            if sd.load(Ordering::Relaxed) { return; }
                            match line_result {
                                Ok(line) => {
                                    if line.trim().is_empty() { continue; }
                                    match parse_game_stream_line(&line) {
                                        Ok(Some(ev @ GameStreamEvent::GameEnd { .. })) => {
                                            let _ = tx.send(ev);
                                            ended = true;
                                            break;
                                        }
                                        Ok(Some(ev)) => { if tx.send(ev).is_err() { return; } }
                                        Ok(None) => {}
                                        Err(e) => rlog!("[lichess] game stream parse: {e}"),
                                    }
                                }
                                Err(e) => {
                                    rlog!("[lichess] game stream read: {e}");
                                    let _ = tx.send(GameStreamEvent::Error(format!("read: {e}")));
                                    break;
                                }
                            }
                        }
                        if ended { break; }
                        }
                    }
                    Err(e) => {
                        rlog!("[lichess] game stream error: {e}");
                        let _ = tx.send(GameStreamEvent::Error(format!("stream: {e}")));
                    }
                }
                if sd.load(Ordering::Relaxed) { return; }
                if tx.send(GameStreamEvent::Error("reconnecting".into())).is_err() { return; }
                std::thread::sleep(std::time::Duration::from_secs(delay));
                delay = (delay * 2).min(60);
            }
        });
        (rx, shutdown)
    }

    pub fn play_move(&self, game_id: &str, uci: &str) -> ApiResult<()> {
        self.post(&format!("/api/board/game/{game_id}/move/{uci}"))?;
        Ok(())
    }

    pub fn challenge_user(
        &self,
        username: &str,
        rated: bool,
        clock_limit: u32,
        clock_increment: u32,
    ) -> ApiResult<()> {
        let limit = clock_limit.to_string();
        let increment = clock_increment.to_string();
        let mut resp = self
            .agent
            .post(format!("{BASE}/api/challenge/{username}"))
            .header("Authorization", format!("Bearer {}", self.token))
            .send_form([
                ("rated", if rated { "true" } else { "false" }),
                ("clock.limit", limit.as_str()),
                ("clock.increment", increment.as_str()),
            ])
            .map_err(|e| format!("challenge: {e}"))?;
        read_text(&mut resp).map(|_| ())
    }

    pub fn accept_challenge(&self, challenge_id: &str) -> ApiResult<()> {
        self.post(&format!("/api/challenge/{challenge_id}/accept"))?;
        Ok(())
    }

    pub fn decline_challenge(&self, challenge_id: &str) -> ApiResult<()> {
        self.post(&format!("/api/challenge/{challenge_id}/decline"))?;
        Ok(())
    }

    pub fn resign_game(&self, game_id: &str) -> ApiResult<()> {
        self.post(&format!("/api/board/game/{game_id}/resign"))?;
        Ok(())
    }

    pub fn offer_draw(&self, game_id: &str, accept: bool) -> ApiResult<()> {
        let v = if accept { "true" } else { "false" };
        self.post(&format!("/api/board/game/{game_id}/draw/{v}"))?;
        Ok(())
    }

    pub fn create_seek(
        &self,
        rated: bool,
        clock_limit: u32,
        clock_increment: u32,
        variant: &str,
        color: &str,
    ) -> ApiResult<()> {
        let time = clock_limit.to_string();
        let increment = clock_increment.to_string();
        let mut resp = self
            .agent
            .post(format!("{BASE}/api/board/seek"))
            .header("Authorization", format!("Bearer {}", self.token))
            .send_form([
                ("rated", if rated { "true" } else { "false" }),
                ("time", time.as_str()),
                ("increment", increment.as_str()),
                ("variant", variant),
                ("color", color),
            ])
            .map_err(|e| format!("seek: {e}"))?;
        read_text(&mut resp).map(|_| ())
    }

    pub fn cancel_seek(&self) -> ApiResult<()> {
        self.post("/api/board/seek/cancel")?;
        Ok(())
    }

    pub fn abort_game(&self, game_id: &str) -> ApiResult<()> {
        self.post(&format!("/api/board/game/{game_id}/abort"))?;
        Ok(())
    }

    pub fn chat(&self, game_id: &str, room: &str, text: &str) -> ApiResult<()> {
        let mut resp = self
            .agent
            .post(format!("{BASE}/api/board/game/{game_id}/chat"))
            .header("Authorization", format!("Bearer {}", self.token))
            .send_form([("room", room), ("text", text)])
            .map_err(|e| format!("chat: {e}"))?;
        read_text(&mut resp).map(|_| ())
    }

    pub fn takeback(&self, game_id: &str, accept: bool) -> ApiResult<()> {
        let v = if accept { "true" } else { "false" };
        self.post(&format!("/api/board/game/{game_id}/takeback/{v}"))?;
        Ok(())
    }

    pub fn claim_victory(&self, game_id: &str) -> ApiResult<()> {
        self.post(&format!("/api/board/game/{game_id}/claim-victory"))?;
        Ok(())
    }
}

// ── NDJSON parsers ─────────────────────────────────────────────────────────────

fn parse_event_line(line: &str) -> Result<Option<LichessEvent>, String> {
    let v: serde_json::Value = serde_json::from_str(line).map_err(|e| format!("json: {e}"))?;
    let typ = v["type"].as_str().unwrap_or("");
    Ok(match typ {
        "challenge" => {
            let ch = &v["challenge"];
            let chal = &ch["challenger"];
            Some(LichessEvent::ChallengeReceived(Challenge {
                id:                ch["id"].as_str().unwrap_or("").to_string(),
                challenger_name:   chal["name"].as_str().unwrap_or("Anonymous").to_string(),
                challenger_rating: chal["rating"].as_i64().map(|r| r as i32),
                variant:           ch["variant"]["name"].as_str().unwrap_or("Standard").to_string(),
                time_control: {
                    let tc = &ch["timeControl"];
                    if tc["type"].as_str() == Some("clock") {
                        let l = tc["limit"].as_u64().unwrap_or(0) / 60;
                        let i = tc["increment"].as_u64().unwrap_or(0);
                        format!("{l}+{i}")
                    } else { "∞".into() }
                },
                color: ch["color"].as_str().map(String::from),
            }))
        }
        "challengeCanceled" => Some(LichessEvent::ChallengeCanceled(
            v["challenge"]["id"].as_str().unwrap_or("").to_string())),
        "challengeDeclined" => Some(LichessEvent::ChallengeDeclined(
            v["challenge"]["id"].as_str().unwrap_or("").to_string())),
        "gameStart" => {
            let g = &v["game"];
            let opp = &g["opponent"];
            let color_str = g["color"].as_str().unwrap_or("white");
            let color = if color_str == "black" { crate::engine::Color::Black } else { crate::engine::Color::White };
            Some(LichessEvent::GameStarted {
                game_id:         g["id"].as_str().unwrap_or("").to_string(),
                opponent_name:   opp["username"].as_str().unwrap_or("?").to_string(),
                opponent_rating: opp["rating"].as_i64().map(|r| r as i32),
                color,
            })
        }
        "gameFinish" => Some(LichessEvent::GameFinished {
            game_id: v["game"]["id"].as_str().unwrap_or("").to_string(),
            result:  v["game"]["status"].as_str().unwrap_or("?").to_string(),
        }),
        _ => { rlog!("[lichess] unknown event: {typ}"); None }
    })
}

fn parse_game_stream_line(line: &str) -> Result<Option<GameStreamEvent>, String> {
    let v: serde_json::Value = serde_json::from_str(line).map_err(|e| format!("json: {e}"))?;
    let typ = v["type"].as_str().unwrap_or("");
    Ok(match typ {
        "gameFull" => {
            let s = &v["state"];
            let w = &v["white"];
            let b = &v["black"];
            Some(GameStreamEvent::GameFull {
                fen:     v["initialFen"].as_str().unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").to_string(),
                moves:   s["moves"].as_str().map(|m| m.split_whitespace().map(String::from).collect()).unwrap_or_default(),
                w_ms:    s["wtime"].as_u64().unwrap_or(0),
                b_ms:    s["btime"].as_u64().unwrap_or(0),
                w_name:  w["name"].as_str().unwrap_or("White").to_string(),
                b_name:  b["name"].as_str().unwrap_or("Black").to_string(),
                w_rating: w["rating"].as_i64().map(|r| r as i32),
                b_rating: b["rating"].as_i64().map(|r| r as i32),
            })
        }
        "gameState" => {
            let moves_str = v["moves"].as_str().unwrap_or("");
            let moves: Vec<&str> = moves_str.split_whitespace().collect();
            let status = v["status"].as_str().unwrap_or("");
            if status == "started" || status == "created" || status == "playing" {
                let last_move = moves.last().unwrap_or(&"").to_string();
                Some(GameStreamEvent::MovePlayed {
                    uci:         last_move,
                    w_ms:        v["wtime"].as_u64().unwrap_or(0),
                    b_ms:        v["btime"].as_u64().unwrap_or(0),
                    move_number: moves.len() as u32,
                })
            } else {
                let winner = v["winner"].as_str().and_then(|w| match w {
                    "white" => Some(crate::engine::Color::White),
                    "black" => Some(crate::engine::Color::Black),
                    _ => None,
                });
                Some(GameStreamEvent::GameEnd { result: status.into(), winner })
            }
        }
        "chatLine" => Some(GameStreamEvent::ChatLine {
            text: v["text"].as_str().unwrap_or("").to_string(),
            room: v["room"].as_str().unwrap_or("player").to_string(),
        }),
        _ => { rlog!("[lichess] unknown game event: {typ}"); None }
    })
}
