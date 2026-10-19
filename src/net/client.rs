// src/net/client.rs — Lichess network client for online play
//
// Owns the background event/game streams and exposes a non-blocking polling
// interface. All network work happens on its own threads; the UI thread only
// ever calls `try_recv`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use crate::lichess::{
    Account, ApiResult, GameStreamEvent, LichessClient, LichessEvent, OngoingGame, SeekHandle,
};

pub struct LichessNetClient {
    pub lichess:      LichessClient,
    event_rx:         Option<mpsc::Receiver<LichessEvent>>,
    game_rx:          Option<mpsc::Receiver<GameStreamEvent>>,
    event_shutdown:   Option<Arc<AtomicBool>>,
    game_shutdown:    Option<Arc<AtomicBool>>,
    pub connected:    bool,
    pub username:     String,
    pub account:      Option<Account>,
    pub username_error: Option<String>,
    pub games:        Vec<OngoingGame>,
    account_rx:       Option<mpsc::Receiver<ApiResult<Account>>>,
    games_rx:         Option<mpsc::Receiver<ApiResult<Vec<OngoingGame>>>>,
    seek_rx:          Option<mpsc::Receiver<ApiResult<SeekHandle>>>,
    /// Keeping this alive keeps the seek alive; dropping it cancels the seek.
    seek_handle:      Option<SeekHandle>,
}

impl LichessNetClient {
    pub fn new(token: &str) -> Self {
        let lichess = LichessClient::new(token);
        let (event_rx, event_shutdown) = lichess.spawn_event_stream();

        let (tx, rx) = mpsc::channel();
        let token = token.to_string();
        std::thread::spawn(move || {
            let client = LichessClient::new(&token);
            let _ = tx.send(client.get_account());
        });

        LichessNetClient {
            lichess,
            event_rx: Some(event_rx),
            event_shutdown: Some(event_shutdown),
            game_rx: None,
            game_shutdown: None,
            connected: false,
            username: String::new(),
            account: None,
            username_error: None,
            games: Vec::new(),
            account_rx: Some(rx),
            games_rx: None,
            seek_rx: None,
            seek_handle: None,
        }
    }

    /// Poll the one-shot account fetch. Returns true when it completed.
    pub fn poll_account(&mut self) -> bool {
        let Some(rx) = &self.account_rx else { return false; };
        match rx.try_recv() {
            Ok(Ok(account)) => {
                self.connected = true;
                self.username = account.username.clone();
                self.account = Some(account);
                self.username_error = None;
                self.account_rx = None;
                self.fetch_ongoing();
                true
            }
            Ok(Err(e)) => {
                self.connected = false;
                self.username_error = Some(e);
                self.account_rx = None;
                true
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.connected = false;
                self.username_error = Some("account request failed".into());
                self.account_rx = None;
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
        }
    }

    /// Fetch games in progress so the lobby can offer to resume them.
    pub fn fetch_ongoing(&mut self) {
        let token = self.lichess.token().to_string();
        let (tx, rx) = mpsc::channel();
        self.games_rx = Some(rx);
        std::thread::spawn(move || {
            let client = LichessClient::new(&token);
            let _ = tx.send(client.ongoing_games());
        });
    }

    pub fn poll_games(&mut self) {
        let Some(rx) = &self.games_rx else { return; };
        match rx.try_recv() {
            Ok(Ok(games)) => { self.games = games; self.games_rx = None; }
            Ok(Err(e))    => { crate::rlog!("[lichess] ongoing games: {e}"); self.games_rx = None; }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => { self.games_rx = None; }
        }
    }

    /// Poll for events from the account-wide event stream.
    pub fn poll_event(&mut self) -> Option<LichessEvent> {
        self.event_rx.as_ref()?.try_recv().ok()
    }

    /// Start a game stream for a specific game.
    pub fn start_game_stream(&mut self, game_id: &str) {
        self.stop_game_stream();
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

    /// Create a seek and keep its connection open in the background.
    pub fn start_seek(&mut self, rated: bool, clock_limit: u32, clock_increment: u32) {
        self.stop_seek();
        self.seek_rx = Some(self.lichess.spawn_seek(
            rated, clock_limit, clock_increment, "standard", "random",
        ));
    }

    /// Poll the seek request. `None` means it is still in flight.
    /// `Some(Ok(()))` means the seek is registered and the connection is held.
    pub fn poll_seek(&mut self) -> Option<ApiResult<()>> {
        let rx = self.seek_rx.take()?;
        match rx.try_recv() {
            Ok(Ok(handle)) => {
                self.seek_handle = Some(handle);
                Some(Ok(()))
            }
            Ok(Err(e)) => Some(Err(e)),
            Err(mpsc::TryRecvError::Empty) => {
                self.seek_rx = Some(rx);
                None
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("seek request failed".to_string()))
            }
        }
    }

    /// Cancelling the seek is done by closing the response stream.
    pub fn stop_seek(&mut self) {
        self.seek_rx = None;
        self.seek_handle = None;
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

impl Drop for LichessNetClient {
    fn drop(&mut self) {
        if let Some(sd) = self.event_shutdown.take() {
            sd.store(true, Ordering::Relaxed);
        }
        if let Some(sd) = self.game_shutdown.take() {
            sd.store(true, Ordering::Relaxed);
        }
        self.seek_handle = None;
    }
}
