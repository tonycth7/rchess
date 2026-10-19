// src/net/client.rs — Lichess network client for online play
//
// Wraps the raw lichess API client with a polling interface
// that mirrors the old NetClient::poll() pattern.

use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::lichess::{LichessClient, LichessEvent, GameStreamEvent, ApiResult};

pub struct LichessNetClient {
    pub lichess:  LichessClient,
    event_rx:     Option<mpsc::Receiver<LichessEvent>>,
    game_rx:      Option<mpsc::Receiver<GameStreamEvent>>,
    event_shutdown: Option<Arc<AtomicBool>>,
    game_shutdown: Option<Arc<AtomicBool>>,
    pub connected:   bool,
    pub username:    String,
    username_rx:   Option<mpsc::Receiver<String>>,
}

impl LichessNetClient {
    pub fn new(token: &str) -> Self {
        let lichess = LichessClient::new(token);
        let (tx, rx) = mpsc::channel();
        let (event_rx, event_shutdown) = lichess.spawn_event_stream();
        // Background username fetch — non-blocking
        let t = token.to_string();
        std::thread::spawn(move || {
            let c = LichessClient::new(&t);
            let name = c.get_username().unwrap_or_else(|_| "?".into());
            let _ = tx.send(name);
        });
        LichessNetClient {
            lichess,
            event_rx: Some(event_rx),
            event_shutdown: Some(event_shutdown),
            game_rx: None,
            game_shutdown: None,
            connected: false,
            username: String::new(),
            username_rx: Some(rx),
        }
    }

    /// Poll for async username fetch completion. Call each frame.
    pub fn poll_username(&mut self) {
        if self.connected { return; }
        if let Some(rx) = &self.username_rx {
            if let Ok(name) = rx.try_recv() {
                self.username = name;
                self.connected = !self.username.is_empty() && self.username != "?";
                self.username_rx = None;
            }
        }
    }

    /// Poll for events from the event stream.
    pub fn poll_event(&mut self) -> Option<LichessEvent> {
        self.event_rx.as_ref()?.try_recv().ok()
    }

    /// Start a game stream for a specific game.
    pub fn start_game_stream(&mut self, game_id: &str) {
        let (rx, sd) = self.lichess.spawn_game_stream(game_id);
        self.game_rx = Some(rx);
        self.game_shutdown = Some(sd);
    }

    /// Stop the current game stream (if any).
    pub fn stop_game_stream(&mut self) {
        if let Some(sd) = self.game_shutdown.take() {
            sd.store(true, Ordering::Relaxed);
        }
        self.game_rx = None;
    }

    /// Poll for events from the current game stream.
    pub fn poll_game(&mut self) -> Option<GameStreamEvent> {
        self.game_rx.as_ref()?.try_recv().ok()
    }

    // ── Convenience wrappers ──

    pub fn play_move(&self, game_id: &str, uci: &str) -> ApiResult<()> {
        self.lichess.play_move(game_id, uci)
    }

    pub fn challenge_user(&self, username: &str, rated: bool, clock_limit: u32, clock_increment: u32) -> ApiResult<()> {
        self.lichess.challenge_user(username, rated, clock_limit, clock_increment)
    }

    pub fn accept_challenge(&self, challenge_id: &str) -> ApiResult<()> {
        self.lichess.accept_challenge(challenge_id)
    }

    pub fn decline_challenge(&self, challenge_id: &str) -> ApiResult<()> {
        self.lichess.decline_challenge(challenge_id)
    }

    pub fn resign_game(&self, game_id: &str) -> ApiResult<()> {
        self.lichess.resign_game(game_id)
    }

    pub fn offer_draw(&self, game_id: &str, accept: bool) -> ApiResult<()> {
        self.lichess.offer_draw(game_id, accept)
    }

    pub fn create_seek(&self, rated: bool, clock_limit: u32, clock_increment: u32) -> ApiResult<()> {
        self.lichess.create_seek(rated, clock_limit, clock_increment, "standard", "random")
    }

    pub fn cancel_seek(&self) -> ApiResult<()> {
        self.lichess.cancel_seek()
    }

    pub fn abort_game(&self, game_id: &str) -> ApiResult<()> {
        self.lichess.abort_game(game_id)
    }

    pub fn chat(&self, game_id: &str, room: &str, text: &str) -> ApiResult<()> {
        self.lichess.chat(game_id, room, text)
    }

    pub fn takeback(&self, game_id: &str, accept: bool) -> ApiResult<()> {
        self.lichess.takeback(game_id, accept)
    }

    pub fn claim_victory(&self, game_id: &str) -> ApiResult<()> {
        self.lichess.claim_victory(game_id)
    }
}
