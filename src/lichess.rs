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
pub struct OngoingGame {
    pub game_id:         String,
    pub color:           crate::engine::Color,
    pub opponent:        String,
    pub opponent_rating: Option<i32>,
    pub fen:             String,
    pub is_my_turn:      bool,
    pub speed:           String,
    pub rated:           bool,
}

#[derive(Debug, Clone)]
pub struct GameState {
    /// All moves played so far, in UCI. This is authoritative: the client
    /// applies the *diff* against its own list instead of trusting a single
    /// "last move" field, which loses moves across reconnects.
    pub moves:  Vec<String>,
    pub w_ms:   u64,
    pub b_ms:   u64,
    pub w_inc:  u64,
    pub b_inc:  u64,
    pub status: String,
    pub winner: Option<crate::engine::Color>,
    pub w_draw_offer: bool,
    pub b_draw_offer: bool,
    pub w_takeback:   bool,
    pub b_takeback:   bool,
}

/// An open seek. Lichess cancels the seek when this is dropped, because
/// dropping it closes the connection that keeps the seek alive.
pub struct SeekHandle {
    _resp: ureq::http::Response<ureq::Body>,
}

#[derive(Debug, Clone)]
pub enum LichessEvent {
    ChallengeReceived(Challenge),
    ChallengeCanceled(String),
    ChallengeDeclined(String),
    GameStarted {
        game_id: String,
        opponent_name: String,
        opponent_rating: Option<i32>,
        color: crate::engine::Color,
        variant: String,
    },
    GameFinished { game_id: String, result: String },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum GameStreamEvent {
    GameFull {
        /// `"startpos"` for standard games, otherwise a FEN (Chess960 etc.).
        base_fen:    String,
        state:       GameState,
        w_name:      String,
        b_name:      String,
        w_rating:    Option<i32>,
        b_rating:    Option<i32>,
        color:       Option<crate::engine::Color>,
        speed:       String,
        rated:       bool,
        initial_ms:  u64,
        increment_ms: u64,
    },
    StateSync(GameState),
    ChatLine { username: String, text: String, room: String },
    OpponentGone { gone: bool, claim_win_in: Option<u64> },
    Error(String),
}

pub type ApiResult<T> = Result<T, String>;

#[derive(Debug, Clone)]
pub struct Account {
    pub username: String,
    pub rating:   Option<u32>,
    pub title:    Option<String>,
}

// ── API client ─────────────────────────────────────────────────────────────────

pub struct LichessClient {
    token: String,
    agent: ureq::Agent,
}

impl LichessClient {
    pub fn new(token: &str) -> Self {
        // API calls run on the UI thread, so keep the worst-case block short.
        let agent = make_agent_timeout(5, 10);
        LichessClient { token: token.to_string(), agent }
    }

    /// The token this client authenticates with (used to spawn one-shot helpers).
    pub fn token(&self) -> &str {
        &self.token
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

    /// Account summary: username plus a representative rating for the lobby.
    pub fn get_account(&self) -> ApiResult<Account> {
        let body = self.get_text("/api/account")?;
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("json: {e}"))?;
        let username = v["username"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "no username field".to_string())?;
        let perfs = &v["perfs"];
        let rating = ["blitz", "rapid", "classical", "bullet"]
            .iter()
            .find_map(|k| perfs[*k]["rating"].as_u64())
            .map(|r| r as u32);
        Ok(Account { username, rating, title: v["title"].as_str().map(String::from) })
    }

    /// Games in progress for this account, used to resume after a reconnect.
    pub fn ongoing_games(&self) -> ApiResult<Vec<OngoingGame>> {
        let body = self.get_text("/api/account/playing")?;
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("json: {e}"))?;
        let list = v["nowPlaying"].as_array().cloned().unwrap_or_default();
        let games = list.iter().filter_map(|g| {
            let game_id = g["gameId"].as_str()?.to_string();
            let color = match g["color"].as_str()? {
                "black" => crate::engine::Color::Black,
                _       => crate::engine::Color::White,
            };
            let opp = &g["opponent"];
            Some(OngoingGame {
                game_id,
                color,
                opponent:        opp["username"].as_str().unwrap_or("Anonymous").to_string(),
                opponent_rating: opp["rating"].as_i64().map(|r| r as i32),
                fen:             g["fen"].as_str().unwrap_or("startpos").to_string(),
                is_my_turn:      g["isMyTurn"].as_bool().unwrap_or(false),
                speed:           g["speed"].as_str().unwrap_or("?").to_string(),
                rated:           g["rated"].as_bool().unwrap_or(false),
            })
        }).collect();
        Ok(games)
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
                            let fatal = matches!(status.as_u16(), 401 | 403);
                            let _ = tx.send(LichessEvent::Error(if fatal {
                                "Invalid or expired Lichess token".to_string()
                            } else {
                                format!("HTTP {status}: {body}")
                            }));
                            // Retrying with bad credentials only spams the API.
                            if fatal { return; }
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
                            let fatal = matches!(status.as_u16(), 401 | 403 | 404);
                            let _ = tx.send(GameStreamEvent::Error(format!(
                                "HTTP {status}: {body}"
                            )));
                            if fatal { return; }
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
                                        Ok(Some(ev)) => {
                                            let terminal = matches!(
                                                &ev,
                                                GameStreamEvent::StateSync(s) if is_terminal_status(&s.status)
                                            );
                                            if tx.send(ev).is_err() { return; }
                                            if terminal { ended = true; break; }
                                        }
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
        let v = if accept { "yes" } else { "no" };
        self.post(&format!("/api/board/game/{game_id}/draw/{v}"))?;
        Ok(())
    }

    /// Open a public seek and keep its connection alive.
    ///
    /// Lichess cancels a seek as soon as the client closes the response
    /// stream, so the old request/response call cancelled every seek it
    /// created. We hand the live `Response` back to the caller; dropping the
    /// [`SeekHandle`] closes the connection and cancels the seek.
    pub fn spawn_seek(
        &self,
        rated: bool,
        clock_limit: u32,
        clock_increment: u32,
        variant: &str,
        color: &str,
    ) -> mpsc::Receiver<ApiResult<SeekHandle>> {
        let (tx, rx) = mpsc::channel();
        let token = self.token.clone();
        let variant = variant.to_string();
        let color = color.to_string();
        std::thread::spawn(move || {
            let agent = make_agent_timeout(10, 30);
            let limit = clock_limit.to_string();
            let increment = clock_increment.to_string();
            let resp = agent
                .post(format!("{BASE}/api/board/seek"))
                .header("Authorization", format!("Bearer {token}"))
                .send_form([
                    ("rated", if rated { "true" } else { "false" }),
                    ("time", limit.as_str()),
                    ("increment", increment.as_str()),
                    ("variant", variant.as_str()),
                    ("color", color.as_str()),
                ]);
            match resp {
                Ok(resp) if resp.status().is_success() => {
                    let _ = tx.send(Ok(SeekHandle { _resp: resp }));
                }
                Ok(mut resp) => {
                    let status = resp.status();
                    let body = resp.body_mut().read_to_string().unwrap_or_default();
                    let _ = tx.send(Err(format!("HTTP {status}: {body}")));
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("seek: {e}")));
                }
            }
        });
        rx
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
        let v = if accept { "yes" } else { "no" };
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
                game_id: g["gameId"].as_str()
                    .or_else(|| g["id"].as_str())
                    .unwrap_or("").to_string(),
                opponent_name:   opp["username"].as_str().unwrap_or("?").to_string(),
                opponent_rating: opp["rating"].as_i64().map(|r| r as i32),
                color,
                variant: g["variant"]["key"].as_str().unwrap_or("standard").to_string(),
            })
        }
        "gameFinish" => {
            let g = &v["game"];
            Some(LichessEvent::GameFinished {
                game_id: g["gameId"].as_str()
                    .or_else(|| g["id"].as_str())
                    .unwrap_or("").to_string(),
                result:  g["status"].as_str().unwrap_or("?").to_string(),
            })
        }
        _ => { rlog!("[lichess] unknown event: {typ}"); None }
    })
}

/// True for statuses after which Lichess closes the game stream.
pub fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        "mate" | "resign" | "stalemate" | "timeout" | "outoftime"
            | "draw" | "aborted" | "noStart" | "cheat" | "unknownFinish"
            | "insufficientMaterialClaim" | "variantEnd"
    )
}

fn parse_state(v: &serde_json::Value) -> GameState {
    GameState {
        moves: v["moves"].as_str().unwrap_or("")
            .split_whitespace().map(String::from).collect(),
        w_ms: v["wtime"].as_u64().unwrap_or(0),
        b_ms: v["btime"].as_u64().unwrap_or(0),
        w_inc: v["winc"].as_u64().unwrap_or(0),
        b_inc: v["binc"].as_u64().unwrap_or(0),
        status: v["status"].as_str().unwrap_or("started").to_string(),
        winner: match v["winner"].as_str() {
            Some("white") => Some(crate::engine::Color::White),
            Some("black") => Some(crate::engine::Color::Black),
            _             => None,
        },
        w_draw_offer: v["wdraw"].as_bool().unwrap_or(false),
        b_draw_offer: v["bdraw"].as_bool().unwrap_or(false),
        w_takeback:   v["wtakeback"].as_bool().unwrap_or(false),
        b_takeback:   v["btakeback"].as_bool().unwrap_or(false),
    }
}

fn parse_game_stream_line(line: &str) -> Result<Option<GameStreamEvent>, String> {
    let v: serde_json::Value = serde_json::from_str(line).map_err(|e| format!("json: {e}"))?;
    let typ = v["type"].as_str().unwrap_or("");
    Ok(match typ {
        "gameFull" => {
            let w = &v["white"];
            let b = &v["black"];
            let color = match v["color"].as_str() {
                Some("black") => Some(crate::engine::Color::Black),
                Some("white") => Some(crate::engine::Color::White),
                _             => None,
            };
            Some(GameStreamEvent::GameFull {
                base_fen: v["initialFen"].as_str().unwrap_or("startpos").to_string(),
                state:    parse_state(&v["state"]),
                w_name:   w["name"].as_str().unwrap_or("White").to_string(),
                b_name:   b["name"].as_str().unwrap_or("Black").to_string(),
                w_rating: w["rating"].as_i64().map(|r| r as i32),
                b_rating: b["rating"].as_i64().map(|r| r as i32),
                color,
                speed:    v["speed"].as_str().unwrap_or("?").to_string(),
                rated:    v["rated"].as_bool().unwrap_or(false),
                initial_ms: v["clock"]["initial"].as_u64().unwrap_or(0),
                increment_ms: v["clock"]["increment"].as_u64().unwrap_or(0),
            })
        }
        "gameState" => Some(GameStreamEvent::StateSync(parse_state(&v))),
        "chatLine" => Some(GameStreamEvent::ChatLine {
            username: v["username"].as_str().unwrap_or("?").to_string(),
            text:     v["text"].as_str().unwrap_or("").to_string(),
            room:     v["room"].as_str().unwrap_or("player").to_string(),
        }),
        "opponentGone" => Some(GameStreamEvent::OpponentGone {
            gone: v["gone"].as_bool().unwrap_or(false),
            claim_win_in: v["claimWinInSeconds"].as_u64(),
        }),
        _ => { rlog!("[lichess] unknown game event: {typ}"); None }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_game_full_with_startpos() {
        let line = r#"{"type":"gameFull","id":"abcd1234","initialFen":"startpos","color":"black","white":{"name":"alice","rating":1600},"black":{"name":"bob","rating":1700},"state":{"type":"started","moves":"e2e4 e7e5","wtime":599000,"btime":598000,"winc":0,"binc":0,"status":"started"}}"#;
        let ev = parse_game_stream_line(line).unwrap().unwrap();
        let GameStreamEvent::GameFull { base_fen, state, color, w_rating, .. } = ev else {
            panic!("expected GameFull");
        };
        assert_eq!(base_fen, "startpos");
        assert_eq!(state.moves, vec!["e2e4", "e7e5"]);
        assert_eq!(state.w_ms, 599_000);
        assert_eq!(color, Some(crate::engine::Color::Black));
        assert_eq!(w_rating, Some(1600));
    }

    #[test]
    fn parses_game_state_with_winner() {
        let line = r#"{"type":"gameState","moves":"e2e4","wtime":0,"btime":1000,"status":"resign","winner":"black"}"#;
        let GameStreamEvent::StateSync(state) = parse_game_stream_line(line).unwrap().unwrap() else {
            panic!("expected StateSync");
        };
        assert_eq!(state.status, "resign");
        assert_eq!(state.winner, Some(crate::engine::Color::Black));
        assert!(is_terminal_status(&state.status));
    }

    #[test]
    fn empty_move_list_is_not_a_move() {
        let line = r#"{"type":"gameState","moves":"","wtime":1000,"btime":1000,"status":"started"}"#;
        let GameStreamEvent::StateSync(state) = parse_game_stream_line(line).unwrap().unwrap() else {
            panic!("expected StateSync");
        };
        assert!(state.moves.is_empty());
    }

    #[test]
    fn game_start_uses_game_id_and_variant_key() {
        let line = r#"{"type":"gameStart","game":{"gameId":"wtRWmfWC","id":"wtRWmfWC","color":"white","variant":{"key":"standard","name":"Standard"},"opponent":{"username":"Aaron","rating":734}}}"#;
        let Some(LichessEvent::GameStarted { game_id, variant, color, .. }) =
            parse_event_line(line).unwrap()
        else {
            panic!("expected GameStarted");
        };
        assert_eq!(game_id, "wtRWmfWC");
        assert_eq!(variant, "standard");
        assert_eq!(color, crate::engine::Color::White);
    }

    #[test]
    fn game_start_falls_back_to_id_when_game_id_is_missing() {
        let line = r#"{"type":"gameStart","game":{"id":"abcd1234","color":"black","opponent":{"username":"bob"}}}"#;
        let Some(LichessEvent::GameStarted { game_id, color, .. }) =
            parse_event_line(line).unwrap()
        else {
            panic!("expected GameStarted");
        };
        assert_eq!(game_id, "abcd1234");
        assert_eq!(color, crate::engine::Color::Black);
    }
}
