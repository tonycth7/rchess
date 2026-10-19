// src/app.rs
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;
use std::fs;
use std::path::PathBuf;
use crossterm::event::KeyCode;
use crate::engine::*;
use crate::ai::best_mv;
use crate::config::{Config, PieceStyle, TimeControl, UiMode};
use crate::lichess::{Challenge, LichessEvent, GameStreamEvent};
use crate::net::client::LichessNetClient;

pub const VERSION: &str = "0.7.3";

fn whoami() -> String {
    std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "Player".into())
}

// ── Screens ───────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Menu, ColorPick, Settings,
    Game, Promo,
    DrawOffer,
    Analyze,
    PngPreview,
    Import,        // Combined FEN/PGN import
    Puzzle,        // Lichess daily puzzle
    PgnSaved,      // PGN save confirmation popup
    OnlineLobby,   // Online play menu
    OnlineGame,    // Active online match
    LoadGame,      // Browse saved games
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode { PvP, Cpu }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClockState { Running, Paused, Flagged }

// ── Data types ────────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct ReplaySnap {
    pub board:    Board,
    pub turn:     Color,
    pub notation: String,
    pub mv_num:   u32,
}

#[derive(Clone)]
pub struct HistEntry {
    pub notation: String,
    pub san:      String,
    pub color:    Color,
    pub from:     (usize, usize),
    pub to:       (usize, usize),
    /// Time taken for this move in milliseconds
    pub move_time_ms: u64,
}

// ── Move classification ───────────────────────────────────────────────────────
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoveLabel { Book, Good, Inaccuracy, Mistake, Blunder }

impl MoveLabel {
    /// delta_cp = (eval_after_white - eval_before_white) for White mover,
    ///          = (eval_before_white - eval_after_white) for Black mover.
    /// Negative delta = bad move.
    pub fn from_delta_cp(delta_cp: i32) -> Self {
        Self::from_delta_cp_thresholds(delta_cp, 50, 100, 300)
    }
    pub fn from_delta_cp_thresholds(delta_cp: i32, inaccuracy: i32, mistake: i32, blunder: i32) -> Self {
        if      delta_cp > -inaccuracy { MoveLabel::Good }
        else if delta_cp > -mistake   { MoveLabel::Inaccuracy }
        else if delta_cp > -blunder   { MoveLabel::Mistake }
        else                          { MoveLabel::Blunder }
    }
    pub fn icon(self) -> &'static str {
        match self {
            MoveLabel::Book       => "B",
            MoveLabel::Good       => "+",
            MoveLabel::Inaccuracy => "?!",
            MoveLabel::Mistake    => "?",
            MoveLabel::Blunder    => "??",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            MoveLabel::Book       => "Book",
            MoveLabel::Good       => "Good",
            MoveLabel::Inaccuracy => "Inaccuracy",
            MoveLabel::Mistake    => "Mistake",
            MoveLabel::Blunder    => "Blunder",
        }
    }
}

/// Per-move analysis result stored after the engine finishes.
#[derive(Clone, Debug)]
pub struct MoveReview {
    pub san:          String,
    pub eval_before:  f32,    // centipawns / 100, White-positive
    pub eval_after:   f32,    // centipawns / 100, White-positive
    pub delta_cp:     i32,    // how much the mover gained/lost (negative = bad)
    pub best_move:    Option<String>,
    pub best_pv:      Option<String>,
    pub top_moves:    Vec<(String, i32)>,
    pub label:        MoveLabel,
    pub is_book:      bool,
}

// ── Game state ────────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct Gs {
    pub board:       Board,
    pub turn:        Color,
    pub ep:          Option<(usize, usize)>,
    pub cast:        Castle,
    pub status:      Status,
    pub history:     Vec<HistEntry>,
    pub cap_w:       Vec<Piece>,
    pub cap_b:       Vec<Piece>,
    pub fullmove:    u32,
    pub white_ms:    Option<u64>,
    pub black_ms:    Option<u64>,
    pub clock_state: ClockState,
}
impl Gs {
    pub fn new(tc: TimeControl) -> Self {
        let t = tc.initial_ms();
        Self {
            board: start_board(), turn: Color::White, ep: None,
            cast: Castle::all(), status: Status::Active,
            history: vec![], cap_w: vec![], cap_b: vec![], fullmove: 1,
            white_ms: t, black_ms: t, clock_state: ClockState::Paused,
        }
    }
    pub fn fmt_time(ms_opt: Option<u64>) -> String {
        match ms_opt {
            None    => "   \u{221e}  ".to_string(),
            Some(0) => " 0:00 ".to_string(),
            Some(ms) => {
                let s = ms / 1000; let m = s / 60; let sec = s % 60;
                if ms < 10_000 { format!(" 0:{:02}.{} ", sec, (ms % 1000) / 100) }
                else           { format!("{}:{:02}  ", m, sec) }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameEnd { Normal, Draw, Flag }

// ── App ───────────────────────────────────────────────────────────────────────
pub struct App {
    pub screen:       Screen,
    pub mode:         Mode,
    pub player_color: Color,
    pub gs:           Gs,
    pub cfg:          Config,
    // Board navigation
    pub cursor:       (usize, usize),
    pub selected:     Option<(usize, usize)>,
    pub targets:      Vec<Mv>,
    // Notation input
    pub input_buf:    String,
    pub input_err:    Option<String>,
    // Goto-move input (type move number, Enter to jump)
    pub goto_buf:     String,
    pub goto_mode:    bool,
    // Promotion picker
    pub promo_from:   (usize, usize),
    pub promo_to:     (usize, usize),
    pub promo_cur:    usize,
    // Menu cursors
    pub menu_cur:     usize,
    pub color_cur:    usize,
    pub settings_cur: usize,
    // Draw offer
    pub draw_offer:     Option<Color>,
    pub draw_offer_msg: Option<String>,
    // Analyze
    pub replay_snaps:  Vec<ReplaySnap>,
    pub replay_idx:    usize,
    pub replay_result: String,
    // Export
    pub pgn_saved_path:   Option<String>,
    pub pgn_notice:       Option<u8>,  // countdown for "PGN saved" banner
    pub png_export_path:  Option<String>,
    pub png_notice:       Option<u8>,
    pub png_preview_path: Option<PathBuf>,
    // Clock
    pub last_tick:    Instant,
    // Built-in minimax AI (opponent)
    pub thinking:     bool,
    pub ai_rx:        Option<Receiver<Option<Mv>>>,
    // Analysis engine (background, always running)
    pub analysis:     crate::analysis::AnalysisHandle,
    pub engine_busy:  bool,
    engine_busy_since: Option<std::time::Instant>,  // detects stuck analysis
    /// Queued analysis to fire once engine is free (move_idx, uci_before, uci_after, color)
    pending_analysis: Option<(usize, String, String, crate::engine::Color)>,
    // Move review data
    pub move_reviews: Vec<MoveReview>,
    // Latest White-positive eval in centipawns
    pub last_eval_cp: i32,
    // Eval at each half-move for sparkline (White-positive cp)
    pub eval_history: Vec<i32>,
    // Misc
    pub should_quit:  bool,
    pub saved_notice: Option<u8>,
    pub undo_stack: Vec<(Gs, usize, usize)>,
    pub redo_stack: Vec<(Gs, usize, usize)>,
    pub game_end:     GameEnd,
    pub board_col_off: usize,
    pub board_row_off: usize,
    // FEN input screen
    pub cell_w:          u16,
    pub cell_h:          u16,
    pub import_buf:      String,
    pub import_err:      Option<String>,
    // Puzzle
    pub puzzle:          Option<crate::puzzle::Puzzle>,
    pub puzzle_state:    crate::puzzle::PuzzleState,
    pub puzzle_move_idx: usize,
    pub puzzle_rx:       Option<std::sync::mpsc::Receiver<Result<crate::puzzle::Puzzle, String>>>,
    // Online (Lichess)
    pub lichess:          Option<LichessNetClient>,
    pub player_name:      String,
    pub lichess_game_id:  Option<String>,
    pub lichess_opponent: Option<String>,
    pub lichess_opponent_rating: Option<i32>,
    pub lichess_connected: bool,
    pub lichess_challenges: Vec<Challenge>,
    pub challenge_buf:    String,
    pub challenge_mode:   bool,
    pub online_status:    String,
    pub online_rated:     bool,
    pub online_tc_idx:    usize,
    pub online_seeking:   bool,
    pub online_chat_mode: bool,
    pub chat_buf:         String,
    pub chat_messages:    Vec<String>,
    pub saved_games: Vec<(std::path::PathBuf, crate::save::GameSave)>,
    // Screen stack for e.g. Settings → back to Game
    pub prev_screen:  Screen,
    // Engine hint overlay
    pub show_hints:   bool,
    // Move arrows on the board
    pub arrows:       Vec<((usize, usize), (usize, usize))>,
}

impl App {
    pub fn new() -> Self {
        let cfg = Config::load();
        let analysis = crate::analysis::AnalysisHandle::spawn(cfg.analysis_engine, cfg.analysis_depth, cfg.stockfish_skill);
        Self {
            screen: Screen::Menu, mode: Mode::PvP,
            player_color: Color::White, gs: Gs::new(cfg.time_control), cfg,
            cursor: (6, 4), selected: None, targets: vec![],
            input_buf: String::new(), input_err: None,
            goto_buf: String::new(), goto_mode: false,
            promo_from: (0,0), promo_to: (0,0), promo_cur: 0,
            menu_cur: 0, color_cur: 0, settings_cur: 0,
            draw_offer: None, draw_offer_msg: None,
            replay_snaps: vec![], replay_idx: 0, replay_result: String::new(),
            pgn_saved_path: None, pgn_notice: None, png_export_path: None,
            png_notice: None, png_preview_path: None,
            last_tick: Instant::now(),
            thinking: false, ai_rx: None,
            analysis,
            engine_busy: false,
            engine_busy_since: None,
            pending_analysis: None,
            move_reviews: vec![],
            last_eval_cp: 0,
            eval_history: vec![0],  // start with balanced
            should_quit: false, saved_notice: None, undo_stack: vec![], redo_stack: vec![],
            game_end: GameEnd::Normal,
            board_col_off: 4, board_row_off: 3,
            import_buf: String::new(), import_err: None,
            puzzle: None,
            puzzle_state: crate::puzzle::PuzzleState::Loading,
            puzzle_move_idx: 0,
            puzzle_rx: None,
            cell_w: 8, cell_h: 4,
            lichess: None,
            player_name: whoami(),
            lichess_game_id: None,
            lichess_opponent: None,
            lichess_opponent_rating: None,
            lichess_connected: false,
            lichess_challenges: vec![],
            challenge_buf: String::new(),
            challenge_mode: false,
            online_status: "Set lichess_token in Settings".into(),
            online_rated: false,
            online_tc_idx: 2,
            online_seeking: false,
            online_chat_mode: false,
            chat_buf: String::new(),
            chat_messages: vec![],
            saved_games: vec![],
            prev_screen: Screen::Menu,
            show_hints: false,
            arrows: vec![],
        }
    }

    /// Auto-compute board cell dimensions from terminal area.
    /// Call once per frame before drawing.
    pub fn update_cell_size(&mut self, term_w: u16, term_h: u16) {
        let coords = self.cfg.show_coords;
        let coord_w: u16 = if coords { 8 } else { 2 };
        let coord_h: u16 = if coords { 2 } else { 0 };
        let input_h: u16 = 4;
        let topbar_h: u16 = 1;
        let max_cw = if self.cfg.piece_style == PieceStyle::Ascii { 14 } else { 20 };

        let board_w = term_w.saturating_sub(coord_w + 2);
        let board_h = term_h.saturating_sub(topbar_h + input_h + coord_h + 2);

        let ch = (board_h / 8).max(1);
        let mut cw = (board_w / 8).max(1).min(max_cw);
        // Visual squareness: terminal chars are ~2:1 height:width,
        // so cell_w needs to be ~2× cell_h to appear square.
        cw = cw.min(ch * 2);
        self.cell_w = cw;
        self.cell_h = ch;
    }

    pub fn flipped(&self) -> bool {
        if self.cfg.flip_board { return true; }
        if self.mode == Mode::Cpu && self.player_color == Color::Black { return true; }
        if self.screen == Screen::OnlineGame {
            return self.player_color == Color::Black;
        }
        if self.mode == Mode::PvP && self.cfg.auto_flip && self.gs.turn == Color::Black { return true; }
        false
    }
    pub fn to_board(&self, vis: (usize, usize)) -> (usize, usize) {
        if self.flipped() { (7 - vis.0, 7 - vis.1) } else { vis }
    }

    // ── Start game ────────────────────────────────────────────────────────────
    pub fn start_game(&mut self) {
        // Don't clear lichess_state — it's set before start_game is called
        let lichess_game_id = self.lichess_game_id.clone();
        let lichess_opponent = self.lichess_opponent.clone();
        let lichess_color = self.player_color;
        self.gs = Gs::new(self.cfg.time_control);
        self.cursor = if self.flipped() { (6, 3) } else { (6, 4) };
        self.selected = None; self.targets = vec![];
        self.input_buf.clear(); self.input_err = None;
        self.thinking = false; self.ai_rx = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.draw_offer = None; self.draw_offer_msg = None;
        self.pgn_saved_path = None; self.pgn_notice = None;
        self.png_export_path = None; self.png_notice = None; self.png_preview_path = None;
        self.replay_snaps.clear(); self.game_end = GameEnd::Normal;
        self.show_hints = false; self.arrows.clear();
        self.online_chat_mode = false; self.chat_buf.clear();
        self.last_tick = Instant::now();
        self.engine_busy = false;
        self.engine_busy_since = None;
        self.pending_analysis = None;
        self.analysis.drain_results();
        self.move_reviews.clear();
        self.last_eval_cp = 0;
        self.eval_history = vec![0];
        self.replay_snaps.push(ReplaySnap {
            board: self.gs.board, turn: Color::White,
            notation: String::new(), mv_num: 0,
        });
        // Restore lichess state that start_game resets
        self.lichess_game_id = lichess_game_id;
        self.lichess_opponent = lichess_opponent;
        self.player_color = lichess_color;
        self.screen = Screen::Game;
        if self.mode == Mode::Cpu && self.player_color == Color::Black { self.kick_ai(); }
    }

    // ── Built-in CPU AI ───────────────────────────────────────────────────────
    pub fn kick_ai(&mut self) {
        if self.thinking { return; }
        self.thinking = true;
        self.gs.clock_state = ClockState::Paused;

        let (tx, rx) = mpsc::channel();
        self.ai_rx = Some(rx);

        if self.cfg.ai_depth.is_stockfish() {
            // Stockfish level: spawn a SEPARATE stockfish instance (not the analysis one)
            // We use a thread with a hard 3s timeout via channel so it never hangs
            let uci_history: String = self.gs.history.iter()
                .map(|h| h.notation.trim_end_matches(['+','#']).to_string())
                .collect::<Vec<_>>().join(" ");
            let skill = self.cfg.stockfish_skill;
            let board = self.gs.board;
            let color = self.gs.turn;
            let ep    = self.gs.ep;
            let cast  = self.gs.cast;
            let history = self.gs.history.clone();
            thread::spawn(move || {
                // Inner channel with timeout
                let (inner_tx, inner_rx) = std::sync::mpsc::channel();
                let moves_clone = uci_history.clone();
                std::thread::spawn(move || {
                    let mv = stockfish_best_move(&moves_clone, skill);
                    let _ = inner_tx.send(mv);
                });
                // Wait max 3 seconds, then fall back to minimax
                let mv = inner_rx
                    .recv_timeout(std::time::Duration::from_secs(3))
                    .ok()
                    .flatten()
                    .or_else(|| {
                        crate::rlog!("[rchess/sf-cpu] timeout — falling back to minimax d4");
                        best_mv(&board, color, ep, &cast, 4, &history)
                    });
                tx.send(mv).ok();
            });
        } else {
            // Use built-in minimax
            let (board, color, ep, cast, depth) = (
                self.gs.board, self.gs.turn, self.gs.ep, self.gs.cast,
                self.cfg.ai_depth.depth(),
            );
            let history = self.gs.history.clone();
            thread::spawn(move || {
                tx.send(best_mv(&board, color, ep, &cast, depth, &history)).ok();
            });
        }
    }

    // ── Main 50 ms tick ───────────────────────────────────────────────────────
    pub fn poll_ai(&mut self) {
        if self.screen != Screen::OnlineGame && self.screen != Screen::DrawOffer {
            self.tick_clock();
        }
        self.poll_minimax();
        self.poll_analysis();
        self.poll_lichess();
        if let Some(n) = self.saved_notice { self.saved_notice = if n == 0 { None } else { Some(n - 1) }; }
        if let Some(n) = self.pgn_notice   { self.pgn_notice   = if n == 0 { None } else { Some(n - 1) }; }
        if let Some(n) = self.png_notice   { self.png_notice   = if n == 0 { None } else { Some(n - 1) }; }
    }

    fn poll_minimax(&mut self) {
        if !self.thinking { return; }
        if let Some(rx) = &self.ai_rx {
            match rx.try_recv() {
                Ok(mv) => {
                    self.thinking = false; self.ai_rx = None;
                    if let Some(mv) = mv {
                        // Safety: verify move is legal before executing
                        // (guards against stale board state in threaded context)
                        let legal_moves = crate::engine::legal(
                            &self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast
                        );
                        if legal_moves.iter().any(|m| m.fr == mv.fr && m.to == mv.to && m.promo == mv.promo) {
                            self.execute(mv);
                        } else {
                            rlog!("[rchess/ai] ILLEGAL move filtered: {:?} turn={:?}", mv, self.gs.turn);
                        }
                    }
                }
                Err(TryRecvError::Disconnected) => { self.thinking = false; self.ai_rx = None; }
                Err(TryRecvError::Empty) => {}
            }
        }
    }

    fn poll_lichess(&mut self) {
        let events = {
            let mut v = vec![];
            if let Some(c) = &mut self.lichess {
                while let Some(e) = c.poll_event() { v.push(e); }
            }
            v
        };
        for event in events {
            match event {
                LichessEvent::ChallengeReceived(ch) => {
                    if !self.lichess_challenges.iter().any(|c| c.id == ch.id) {
                        self.lichess_challenges.push(ch);
                        self.online_status = "Challenge received".into();
                    }
                }
                LichessEvent::ChallengeCanceled(id) => {
                    self.lichess_challenges.retain(|c| c.id != id);
                    self.online_status = "Challenge canceled".into();
                }
                LichessEvent::ChallengeDeclined(id) => {
                    self.lichess_challenges.retain(|c| c.id != id);
                    self.online_status = "Challenge declined".into();
                }
                LichessEvent::GameStarted { game_id, opponent_name, opponent_rating, color } => {
                    self.lichess_game_id = Some(game_id.clone());
                    self.lichess_opponent = Some(opponent_name.clone());
                    self.lichess_opponent_rating = opponent_rating;
                    self.lichess_challenges.clear();
                    self.player_color = color;
                    self.mode = Mode::PvP;
                    self.start_game();
                    self.screen = Screen::OnlineGame;
                    self.online_status = format!("Playing {opponent_name} on Lichess");
                    rlog!("[lichess] game started vs {opponent_name} id={game_id} color={color:?}");
                    if let Some(client) = &mut self.lichess {
                        client.start_game_stream(&game_id);
                    }
                }
                LichessEvent::GameFinished { result, .. } => {
                    self.online_status = format!("Game over: {result}");
                    if self.screen == Screen::OnlineGame {
                        self.replay_result = result.clone();
                        self.export_pgn();
                        if self.cfg.auto_save_png {
                            self.auto_png_export();
                        }
                    }
                }
                LichessEvent::Error(msg) => {
                    self.online_status = format!("Lichess: {msg}");
                }
            }
        }
        // Collect game stream messages
        if self.screen == Screen::OnlineGame {
            let g_events = {
                let mut v = vec![];
                if let Some(c) = &mut self.lichess {
                    while let Some(e) = c.poll_game() { v.push(e); }
                }
                v
            };
            for event in g_events {
                match event {
                    GameStreamEvent::GameFull { fen, moves, w_ms, b_ms, w_name, b_name, .. } => {
                        // Dedup: if we already have these moves, skip replay
                        if self.gs.history.len() >= moves.len() {
                            self.gs.white_ms = Some(w_ms);
                            self.gs.black_ms = Some(b_ms);
                        } else {
                            if fen != "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1" {
                                if let Ok((board, turn, ep, cast, _)) = crate::puzzle::parse_fen(&fen) {
                                    self.gs.board = board;
                                    self.gs.turn = turn;
                                    self.gs.ep = ep;
                                    self.gs.cast = cast;
                                }
                            }
                            self.gs.white_ms = Some(w_ms);
                            self.gs.black_ms = Some(b_ms);
                            let start_idx = self.gs.history.len();
                            for m in &moves[start_idx..] {
                                if let Ok(mv) = parse_notation(
                                    &self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast, m
                                ) {
                                    let (b, e, c) = apply(&self.gs.board, &mv, self.gs.ep, &self.gs.cast);
                                    self.gs.board = b;
                                    self.gs.ep = e;
                                    self.gs.cast = c;
                                    self.gs.turn = self.gs.turn.opp();
                                    self.gs.history.push(HistEntry {
                                        notation: m.clone(),
                                        san: m.clone(),
                                        color: self.gs.turn,
                                        from: mv.fr, to: mv.to,
                                        move_time_ms: 0,
                                    });
                                    if mv.fr.0 == 0 && self.gs.turn == Color::Black {
                                        self.gs.fullmove += 1;
                                    }
                                }
                            }
                        }
                        self.online_status = format!("Playing {} vs {}", w_name, b_name);
                    }
                    GameStreamEvent::MovePlayed { uci, w_ms, b_ms, .. } => {
                        if !uci.is_empty() {
                            if let Ok(mv) = parse_notation(
                                &self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast, &uci
                            ) {
                                self.execute(mv);
                            } else {
                                rlog!("[lichess] MovePlayed parse failed: {uci} board turn={:?}", self.gs.turn);
                            }
                        }
                        self.gs.white_ms = Some(w_ms);
                        self.gs.black_ms = Some(b_ms);
                    }
                    GameStreamEvent::GameEnd { result, winner: _ } => {
                        self.online_status = format!("Game over: {result}");
                        if result == "mate" { self.gs.status = Status::Checkmate; }
                        else if result == "draw" { self.gs.status = Status::Draw; self.game_end = GameEnd::Draw; }
                        else { self.gs.status = Status::Stalemate; }
                        self.finish_game();
                    }
                    GameStreamEvent::ChatLine { text, room } => {
                        rlog!("[lichess] chat ({room}): {text}");
                        if room == "player" {
                            let who = self.lichess_opponent.as_deref().unwrap_or("Opponent");
                            self.chat_messages.push(format!("{who}: {text}"));
                            if self.chat_messages.len() > 50 { self.chat_messages.remove(0); }
                            if text.contains("draw") || text.contains("Draw") {
                                self.online_status = "Opponent offers draw".into();
                                self.draw_offer = Some(self.gs.turn.opp());
                                self.draw_offer_msg = Some(text);
                                if self.screen == Screen::OnlineGame {
                                    self.screen = Screen::DrawOffer;
                                }
                            }
                        }
                    }
                    GameStreamEvent::Error(msg) => {
                        rlog!("[lichess] game stream error: {msg}");
                        self.online_status = if msg == "reconnecting" {
                            "Reconnecting to game...".to_string()
                        } else {
                            format!("Stream error: {msg}")
                        };
                    }
                }
            }
        }
    }

    fn poll_analysis(&mut self) {
        // Watchdog: if analysis has been busy for >20s with no result, reset it
        if self.engine_busy {
            if let Some(since) = self.engine_busy_since {
                if since.elapsed().as_secs() > 20 {
                    rlog!("[rchess/analysis] watchdog: resetting stuck analysis engine");
                    self.engine_busy = false;
                    self.engine_busy_since = None;
                    self.analysis.drain_results();
                    return;
                }
            }
        }
        if !self.engine_busy { return; }
        let result = match self.analysis.try_recv() {
            Some(r) => r,
            None    => return,
        };
        self.engine_busy = false;
        self.engine_busy_since = None;

        let move_idx = result.move_idx;
        // delta_cp: how much the mover's side changed in White-positive terms
        let delta_cp = match result.color_moved {
            Color::White => result.eval_after_cp - result.eval_before_cp,
            Color::Black => result.eval_before_cp - result.eval_after_cp,
        };
        self.last_eval_cp = result.eval_after_cp;
        // Update eval history for sparkline (index = move_idx + 1)
        let hist_idx = result.move_idx + 1;
        if hist_idx >= self.eval_history.len() {
            self.eval_history.resize(hist_idx + 1, self.last_eval_cp);
        }
        self.eval_history[hist_idx] = result.eval_after_cp;

        let is_book = self.check_is_book_move(move_idx);
        let label = if is_book {
            MoveLabel::Book
        } else {
            MoveLabel::from_delta_cp_thresholds(
                delta_cp,
                self.cfg.inaccuracy_cp,
                self.cfg.mistake_cp,
                self.cfg.blunder_cp,
            )
        };

        let san = self.gs.history.get(move_idx)
            .map(|h| h.san.clone())
            .unwrap_or_default();

        rlog!(
            "[rchess/review] move {} ({}) before={} after={} delta={} label={:?} book={}",
            move_idx, san, result.eval_before_cp, result.eval_after_cp, delta_cp, label, is_book
        );

        // Sound bell on blunder
        if label == MoveLabel::Blunder {
            print!("\x07"); use std::io::Write; let _ = std::io::stdout().flush();
        }
        let review = MoveReview {
            san,
            eval_before:  result.eval_before_cp as f32 / 100.0,
            eval_after:   result.eval_after_cp  as f32 / 100.0,
            delta_cp,
            best_move:    result.best_move_uci,
            best_pv:      result.best_pv,
            top_moves:    result.top_moves,
            label, is_book,
        };

        if move_idx < self.move_reviews.len() {
            self.move_reviews[move_idx] = review;
        } else {
            while self.move_reviews.len() < move_idx {
                self.move_reviews.push(MoveReview {
                    san: String::new(), eval_before: 0.0, eval_after: 0.0,
                    delta_cp: 0, best_move: None, best_pv: None,
                    top_moves: vec![], label: MoveLabel::Good, is_book: false,
                });
            }
            self.move_reviews.push(review);
        }

        // Fire any queued analysis now that engine is free
        if let Some((mi, ub, ua, cm)) = self.pending_analysis.take() {
            rlog!("[rchess/analysis] firing pending move {}", mi);
            self.analysis.analyse(mi, &ub, &ua, cm, self.cfg.analysis_depth);
            self.engine_busy = true;
            self.engine_busy_since = Some(std::time::Instant::now());
        }
    }

    fn check_is_book_move(&self, move_idx: usize) -> bool {
        if move_idx >= self.gs.history.len() { return false; }
        let hist_before = &self.gs.history[..move_idx];
        let (board_before, ep_before, cast_before) = self.undo_stack.last()
            .map(|(gs, _, _)| (gs.board, gs.ep, gs.cast))
            .unwrap_or_else(|| (start_board(), None, Castle::all()));
        let turn_before = if move_idx.is_multiple_of(2) { Color::White } else { Color::Black };
        let current_uci = self.gs.history.get(move_idx)
            .map(|h| h.notation.trim_end_matches(['+', '#']).to_string());
        let book_mv = crate::book::book_move(
            hist_before, &board_before, turn_before, ep_before, &cast_before,
        );
        if let (Some(bm), Some(uci)) = (book_mv, current_uci) {
            let bm_uci = format!(
                "{}{}{}{}",
                (b'a' + bm.fr.1 as u8) as char, 8 - bm.fr.0,
                (b'a' + bm.to.1 as u8) as char, 8 - bm.to.0,
            );
            uci.starts_with(&bm_uci)
        } else {
            false
        }
    }


    /// Respawn the analysis engine with current config settings.
    /// Called when the user changes analysis engine or depth in Settings.
    pub fn restart_analysis_engine(&mut self) {
        self.engine_busy = false;
        self.engine_busy_since = None;
        self.pending_analysis = None;
        self.analysis = crate::analysis::AnalysisHandle::spawn(
            self.cfg.analysis_engine,
            self.cfg.analysis_depth,
            self.cfg.stockfish_skill,
        );
        rlog!("[rchess] analysis engine restarted: {} d{}",
            self.analysis.engine_name, self.cfg.analysis_depth);
    }

    fn queue_analysis_after_move(&mut self) {
        if self.cfg.ui_mode == UiMode::Minimal { return; }
        let h = &self.gs.history;
        let move_idx    = h.len().saturating_sub(1);
        let color_moved = h.last().map(|e| e.color).unwrap_or(Color::White);
        let uci_before: String = h[..move_idx].iter()
            .map(|e| e.notation.trim_end_matches(['+', '#']).to_string())
            .collect::<Vec<_>>().join(" ");
        let uci_after: String = h.iter()
            .map(|e| e.notation.trim_end_matches(['+', '#']).to_string())
            .collect::<Vec<_>>().join(" ");
        if self.engine_busy {
            rlog!("[rchess/analysis] engine busy — queuing move {} for later", move_idx);
            self.pending_analysis = Some((move_idx, uci_before, uci_after, color_moved));
            return;
        }
        rlog!("[rchess/analysis] queuing move {} ({:?})", move_idx, color_moved);
        self.analysis.analyse(move_idx, &uci_before, &uci_after, color_moved, self.cfg.analysis_depth);
        self.engine_busy = true;
        self.engine_busy_since = Some(std::time::Instant::now());
    }

    // ── Clock ─────────────────────────────────────────────────────────────────
    fn tick_clock(&mut self) {
        if self.screen == Screen::OnlineGame { return; }
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_tick).as_millis() as u64;
        self.last_tick = now;
        if self.gs.clock_state != ClockState::Running { return; }
        if matches!(self.gs.status, Status::Checkmate | Status::Stalemate | Status::Draw) { return; }
        let flagged = match self.gs.turn {
            Color::White => {
                if let Some(ms) = &mut self.gs.white_ms {
                    *ms = ms.saturating_sub(elapsed_ms); *ms == 0
                } else { false }
            }
            Color::Black => {
                if let Some(ms) = &mut self.gs.black_ms {
                    *ms = ms.saturating_sub(elapsed_ms); *ms == 0
                } else { false }
            }
        };
        if flagged {
            self.gs.clock_state = ClockState::Flagged;
            self.game_end = GameEnd::Flag;
            self.finish_game();
        }
    }

    fn finish_game(&mut self) {
        self.replay_result = match self.game_end {
            GameEnd::Flag  => format!("\u{1fa0f} {} winner on time \u{1fa09}", self.gs.turn.opp().name()),
            GameEnd::Draw  => "\u{bd}-\u{bd} Draw".to_string(),
            GameEnd::Normal => match self.gs.status {
                Status::Checkmate => format!("\u{1fa0f} {} wins \u{1fa09}", self.gs.turn.opp().name()),
                Status::Stalemate => "\u{bd}-\u{bd} Stalemate".to_string(),
                _                 => String::new(),
            }
        };
        self.export_pgn();
        // Auto-save PNG on game end if configured
        if self.cfg.auto_save_png {
            self.auto_png_export();
        }
        if self.cfg.auto_save_game {
            let result_str = match self.game_end {
                GameEnd::Flag => format!("{} wins on time", self.gs.turn.opp().name()),
                GameEnd::Draw => "\u{bd}-\u{bd} Draw".to_string(),
                GameEnd::Normal => match self.gs.status {
                    Status::Checkmate => format!("{} wins", self.gs.turn.opp().name()),
                    Status::Stalemate => "Stalemate".to_string(),
                    _ => String::new(),
                }
            };
            let moves: Vec<String> = self.gs.history.iter()
                .map(|h| format!("{}{}{}{}",
                    (b'a' + h.from.1 as u8) as char, 8 - h.from.0,
                    (b'a' + h.to.1 as u8) as char, 8 - h.to.0))
                .collect();
            let save = crate::save::GameSave {
                initial_fen: self.current_fen(),
                moves,
                white_name: if self.player_color == crate::engine::Color::White { self.player_name.clone() } else { self.lichess_opponent.clone().unwrap_or_else(|| "White".into()) },
                black_name: if self.player_color == crate::engine::Color::Black { self.player_name.clone() } else { self.lichess_opponent.clone().unwrap_or_else(|| "Black".into()) },
                time_control: self.cfg.time_control.short_label().into(),
                result: result_str,
                date: format!("{:?}", std::time::SystemTime::now()),
                incomplete: false,
            };
            crate::save::auto_save_game(&save);
        }
    }

    /// Save current (possibly incomplete) game to the saves directory.
    pub fn save_incomplete_game(&mut self) {
        let moves: Vec<String> = self.gs.history.iter()
            .map(|h| format!("{}{}{}{}",
                (b'a' + h.from.1 as u8) as char, 8 - h.from.0,
                (b'a' + h.to.1 as u8) as char, 8 - h.to.0))
            .collect();
        let save = crate::save::GameSave {
            initial_fen: self.current_fen(),
            moves,
            white_name: self.player_name.clone(),
            black_name: self.lichess_opponent.clone().unwrap_or_else(|| "Opponent".into()),
            time_control: self.cfg.time_control.short_label().into(),
            result: "*".into(),
            date: format!("{:?}", std::time::SystemTime::now()),
            incomplete: true,
        };
        crate::save::auto_save_game(&save);
    }

    /// Silently saves a PNG without showing the preview screen.
    fn auto_png_export(&mut self) {
        let ts = {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
        };
        let path = export_dir().join(format!("rchess_board_{}.png", ts));
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let abs = if path.is_absolute() {
            path.clone()
        } else {
            std::env::current_dir().map(|cwd| cwd.join(&path)).unwrap_or(path.clone())
        };
        crate::rlog!("[rchess] auto PNG export: {}", abs.display());
        let last_from = self.gs.history.last().map(|h| h.from);
        let last_to   = self.gs.history.last().map(|h| h.to);
        let ok = crate::png_export::render_board_png(
            &self.gs.board, self.cfg.theme, self.flipped(),
            last_from, last_to, &abs,
        );
        if ok {
            self.png_export_path = Some(abs.to_string_lossy().to_string());
            self.png_notice = Some(80);
        }
        crate::rlog!("[rchess] auto PNG: {}", if ok { "saved" } else { "failed" });
    }

    // ── PGN export ────────────────────────────────────────────────────────────
    pub fn export_pgn(&mut self) {
        let result_tag = match self.game_end {
            GameEnd::Flag => match self.gs.turn.opp() {
                Color::White => "1-0", Color::Black => "0-1",
            },
            GameEnd::Draw => "1/2-1/2",
            GameEnd::Normal => match self.gs.status {
                Status::Checkmate => match self.gs.turn.opp() {
                    Color::White => "1-0", Color::Black => "0-1",
                },
                Status::Stalemate => "1/2-1/2",
                _ => "*",
            }
        };
        let date = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let secs = SystemTime::now().duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or(0);
            format!("{}.??.??", 1970 + secs / 86400 / 365)
        };
        let is_online = self.lichess.is_some();
        let player_name = if is_online { self.player_name.clone() } else { "Player".to_string() };
        let opp_name = self.lichess_opponent.clone().unwrap_or_else(|| "Opponent".to_string());
        let white_name = if is_online {
            if self.player_color == Color::White { player_name.clone() } else { opp_name.clone() }
        } else {
            match self.mode {
                Mode::PvP => "Player 1".to_string(),
                Mode::Cpu => if self.player_color == Color::White { "Player".to_string() }
                             else {
                                 let label = if self.cfg.ai_depth.is_stockfish() {
                                     format!("CPU (Stockfish sk{})", self.cfg.stockfish_skill)
                                 } else {
                                     format!("CPU ({})", self.cfg.ai_depth.name().split_whitespace().next().unwrap_or("AI"))
                                 };
                                 label
                             },
            }
        };
        let black_name = if is_online {
            if self.player_color == Color::Black { player_name } else { opp_name }
        } else {
            match self.mode {
                Mode::PvP => "Player 2".to_string(),
                Mode::Cpu => if self.player_color == Color::Black { "Player".to_string() }
                             else {
                                 let label = if self.cfg.ai_depth.is_stockfish() {
                                     format!("CPU (Stockfish sk{})", self.cfg.stockfish_skill)
                                 } else {
                                     format!("CPU ({})", self.cfg.ai_depth.name().split_whitespace().next().unwrap_or("AI"))
                                 };
                                 label
                             },
            }
        };
        let tc_str = match self.cfg.time_control {
            TimeControl::Infinite => "-", TimeControl::Bullet => "60",
            TimeControl::Blitz => "180",  TimeControl::Rapid => "600",
            TimeControl::Classical => "1800",
        };
        let mut move_text = String::new();
        let h = &self.gs.history;
        let mut i = 0; let mut n = 1u32;
        while i < h.len() {
            move_text.push_str(&format!("{}. {} ", n, h[i].san));
            if i + 1 < h.len() { move_text.push_str(&format!("{} ", h[i+1].san)); }
            i += 2; n += 1;
            if n.is_multiple_of(5) { move_text.push('\n'); }
        }
        move_text.push_str(result_tag);
        let pgn = format!(
            "[Event \"RChess TUI\"]\n[Site \"Terminal\"]\n[Date \"{}\"]\n\
             [White \"{}\"]\n[Black \"{}\"]\n[Result \"{}\"]\n\
             [TimeControl \"{}\"]\n[Generator \"RChess TUI v{}\"]\n\n{}\n",
            date, white_name, black_name, result_tag, tc_str, VERSION, move_text
        );
        let dir  = export_dir();
        let path = dir.join(format!("rchess_{}.pgn", self.gs.fullmove));
        rlog!("[rchess] PGN: {}", path.display());
        match fs::write(&path, &pgn) {
            Ok(_)  => { self.pgn_saved_path = Some(path.to_string_lossy().to_string()); self.pgn_notice = Some(80); }
            Err(e) => { rlog!("[rchess] PGN write error: {}", e); },
        }
    }

    // ── PNG preview / export ──────────────────────────────────────────────────
    pub fn open_png_preview(&mut self) {
        let ts = {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
        };
        let path = export_dir().join(format!("rchess_board_{}.png", ts));
        rlog!("[rchess] PNG preview: {}", path.display());
        self.png_preview_path = Some(path);
        self.screen = Screen::PngPreview;
    }

    pub fn confirm_png_export(&mut self) {
        if let Some(path) = self.png_preview_path.take() {
            if let Some(parent) = path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    rlog!("[rchess] mkdir error: {}", e);
                }
            }
            let abs = if path.is_absolute() {
                path.clone()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(&path))
                    .unwrap_or_else(|_| path.clone())
            };
            rlog!("[rchess] exporting PNG: {}", abs.display());
            let last_from = self.gs.history.last().map(|h| h.from);
            let last_to   = self.gs.history.last().map(|h| h.to);
            let ok = crate::png_export::render_board_png(
                &self.gs.board, self.cfg.theme, self.flipped(),
                last_from, last_to, &abs,
            );
            rlog!("[rchess] PNG: {}", if ok { "OK" } else { "FAILED (install python-pillow)" });
            if ok {
                self.png_export_path = Some(abs.to_string_lossy().to_string());
                self.png_notice = Some(80);
            }
        }
    }

    // ── FEN ───────────────────────────────────────────────────────────────────
    pub fn current_fen(&self) -> String {
        let mut fen = String::new();
        for r in 0..8 {
            let mut empty = 0u8;
            for c in 0..8 {
                match self.gs.board[r][c] {
                    None => empty += 1,
                    Some(p) => {
                        if empty > 0 { fen.push((b'0' + empty) as char); empty = 0; }
                        let ch = match p.k { Kind::K=>'k',Kind::Q=>'q',Kind::R=>'r',Kind::B=>'b',Kind::N=>'n',Kind::P=>'p' };
                        fen.push(if p.c == Color::White { ch.to_ascii_uppercase() } else { ch });
                    }
                }
            }
            if empty > 0 { fen.push((b'0' + empty) as char); }
            if r < 7 { fen.push('/'); }
        }
        let turn = if self.gs.turn == Color::White { 'w' } else { 'b' };
        let mut castle = String::new();
        if self.gs.cast.wk { castle.push('K'); }
        if self.gs.cast.wq { castle.push('Q'); }
        if self.gs.cast.bk { castle.push('k'); }
        if self.gs.cast.bq { castle.push('q'); }
        if castle.is_empty() { castle.push('-'); }
        let ep = self.gs.ep.map(|(r, c)| format!("{}{}", (b'a' + c as u8) as char, 8 - r))
            .unwrap_or_else(|| "-".to_string());
        format!("{} {} {} {} 0 {}", fen, turn, castle, ep, self.gs.fullmove)
    }

    // ── Key handlers ──────────────────────────────────────────────────────────
    pub fn handle_menu_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up   | KeyCode::Char('k') => { if self.menu_cur > 0 { self.menu_cur -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if self.menu_cur < 7 { self.menu_cur += 1; } }
            KeyCode::Enter | KeyCode::Char(' ') => match self.menu_cur {
                0 => {
                    let saves = crate::save::list_saves();
                    if let Some((path, save)) = saves.into_iter().find(|(_, s)| s.incomplete) {
                        self.load_saved_game(path, save);
                    }
                }
                1 => { self.mode = Mode::PvP; self.player_color = Color::White; self.start_game(); }
                2 => { self.mode = Mode::Cpu; self.screen = Screen::ColorPick; }
                3 => self.screen = Screen::OnlineLobby,
                4 => { self.prev_screen = Screen::Menu; self.screen = Screen::Settings; }
                5 => self.open_import(),
                6 => self.open_load_game(),
                7 => self.open_puzzle(),
                _ => {}
            },
            KeyCode::Char('s') => { self.prev_screen = Screen::Menu; self.screen = Screen::Settings; }
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }

    pub fn handle_color_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Up   | KeyCode::Char('k') => self.color_cur = 0,
            KeyCode::Right| KeyCode::Char('l') | KeyCode::Down | KeyCode::Char('j') => self.color_cur = 1,
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.player_color = if self.color_cur == 0 { Color::White } else { Color::Black };
                self.start_game();
            }
            KeyCode::Char('w') => { self.player_color = Color::White; self.start_game(); }
            KeyCode::Char('b') => { self.player_color = Color::Black; self.start_game(); }
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Menu,
            _ => {}
        }
    }

    pub fn handle_settings_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up   | KeyCode::Char('k') => { if self.settings_cur > 0  { self.settings_cur -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if self.settings_cur < 18 { self.settings_cur += 1; } }
            KeyCode::PageUp   => { self.settings_cur = self.settings_cur.saturating_sub(5); }
            KeyCode::PageDown => { self.settings_cur = (self.settings_cur + 5).min(18); }
            KeyCode::Home     => { self.settings_cur = 0; }
            KeyCode::End      => { self.settings_cur = 18; }
            KeyCode::Left | KeyCode::Char('h') => self.settings_cycle(false),
            KeyCode::Right| KeyCode::Char('l') => self.settings_cycle(true),
            KeyCode::Char('w') | KeyCode::Char('s') => { self.cfg.save(); self.saved_notice = Some(120); }
            KeyCode::Char('r') => self.cfg = Config::default(),
            KeyCode::Esc | KeyCode::Char('q') => self.screen = self.prev_screen,
            _ => {}
        }
    }

    fn settings_cycle(&mut self, fwd: bool) {
        use crate::config::*;
        fn cyc<T: Copy + PartialEq>(arr: &[T], cur: T, fwd: bool) -> T {
            let i = arr.iter().position(|x| x == &cur).unwrap_or(0);
            if fwd { arr[(i + 1) % arr.len()] } else { arr[(i + arr.len() - 1) % arr.len()] }
        }
        match self.settings_cur {
            0  => self.cfg.theme           = cyc(Theme::ALL,          self.cfg.theme,           fwd),
            1  => self.cfg.piece_style     = cyc(PieceStyle::ALL,     self.cfg.piece_style,     fwd),
            2  => self.cfg.ai_depth        = cyc(AiDepth::ALL,        self.cfg.ai_depth,        fwd),
            3  => self.cfg.move_hints      = cyc(MoveHints::ALL,      self.cfg.move_hints,      fwd),
            4  => self.cfg.time_control    = cyc(TimeControl::ALL,    self.cfg.time_control,    fwd),
            5  => self.cfg.show_coords     = !self.cfg.show_coords,
            6  => self.cfg.show_clock      = !self.cfg.show_clock,
            7  => self.cfg.flip_board      = !self.cfg.flip_board,
            8  => self.cfg.auto_flip       = !self.cfg.auto_flip,
             9  => self.cfg.confirm_move    = !self.cfg.confirm_move,
             10 => self.cfg.auto_promote     = !self.cfg.auto_promote,
             11 => self.cfg.ui_mode         = cyc(UiMode::ALL,         self.cfg.ui_mode,         fwd),
             12 => {
                self.cfg.analysis_engine = cyc(AnalysisEngine::ALL, self.cfg.analysis_engine, fwd);
                self.restart_analysis_engine();
            }
              13 => {
                self.cfg.analysis_depth = if fwd {
                    (self.cfg.analysis_depth % 3) + 1
                } else {
                    if self.cfg.analysis_depth <= 1 { 3 } else { self.cfg.analysis_depth - 1 }
                };
                self.restart_analysis_engine();
            }
              14 => self.cfg.auto_save_png   = !self.cfg.auto_save_png,
              15 => self.cfg.auto_save_game  = !self.cfg.auto_save_game,
               16 => {
                let cur = self.cfg.highlight_brightness as i16;
                self.cfg.highlight_brightness = if fwd { (cur + 1).min(10) as u8 } else { (cur - 1).max(0) as u8 };
            }
               17 => {
                self.cfg.tty_mode = !self.cfg.tty_mode;
                let tty_auto = std::env::var("TERM").unwrap_or_default();
                let tty_auto = tty_auto == "linux" || tty_auto == "vt100" || tty_auto == "vt220"
                    || tty_auto.starts_with("screen") || tty_auto == "dumb";
                crate::ui::TTY_MODE.store(self.cfg.tty_mode || tty_auto, std::sync::atomic::Ordering::Relaxed);
            }
              18 => {
                let cur = self.cfg.stockfish_skill as i16;
                self.cfg.stockfish_skill = if fwd { (cur + 1).min(20) as u8 } else { (cur - 1).max(0) as u8 };
            }
            _  => {}
        }
    }

    pub fn handle_game_key(&mut self, code: KeyCode) {
        let over  = matches!(self.gs.status, Status::Checkmate | Status::Stalemate | Status::Draw)
                    || self.gs.clock_state == ClockState::Flagged;
        let human = self.mode == Mode::PvP || self.gs.turn == self.player_color;

        // Typing mode — keys go into notation buffer
        if !self.input_buf.is_empty() {
            match code {
                KeyCode::Enter => {
                    if human && !over && !self.thinking {
                        let buf = self.input_buf.clone();
                        match parse_notation(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast, &buf) {
                            Ok(mv) => {
                                self.input_buf.clear(); self.input_err = None;
                                self.selected = None; self.targets = vec![];
                                self.execute(mv);
                            }
                            Err(e) => self.input_err = Some(e),
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.input_buf.pop();
                    if self.input_buf.is_empty() { self.input_err = None; }
                }
                KeyCode::Esc     => { self.input_buf.clear(); self.input_err = None; }
                KeyCode::Up      => { if self.cursor.0 > 0 { self.cursor.0 -= 1; } }
                KeyCode::Down    => { if self.cursor.0 < 7 { self.cursor.0 += 1; } }
                KeyCode::Left    => { if self.cursor.1 > 0 { self.cursor.1 -= 1; } }
                KeyCode::Right   => { if self.cursor.1 < 7 { self.cursor.1 += 1; } }
                KeyCode::Char(c) => { self.input_buf.push(c); self.input_err = None; }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Char('q') => {
                if !matches!(self.gs.status, Status::Checkmate | Status::Stalemate | Status::Draw) {
                    self.save_incomplete_game();
                }
                self.screen = Screen::Menu; self.selected = None; self.targets = vec![];
            }
            KeyCode::Char('n') => self.start_game(),
            KeyCode::Char('s') => { self.prev_screen = self.screen; self.screen = Screen::Settings; }
            KeyCode::Char('u') => self.undo(),
            KeyCode::Char('r') => {
                use crate::engine::Status;
                let game_over = matches!(self.gs.status, Status::Checkmate | Status::Stalemate | Status::Draw)
                    || self.gs.clock_state == ClockState::Flagged
                    || self.game_end == GameEnd::Draw;
                if game_over && !self.replay_snaps.is_empty()
                { self.open_analyze(); }
                else { self.redo(); }
            }
            KeyCode::Char('E') => self.open_png_preview(),
            KeyCode::Char('F') => {
                let fen = self.current_fen();
                let copied = copy_to_clipboard(&fen);
                self.input_err = Some(if copied { "FEN copied to clipboard ✓".into() } else { "FEN copy failed (install xclip/wl-copy)".into() });
            }
            // G (Shift+G) = save PGN game notation manually
            KeyCode::Char('G') => { self.export_pgn(); if self.pgn_saved_path.is_some() { self.screen = Screen::PgnSaved; } }
            // T = cycle UI mode without going to settings
            KeyCode::Char('T') => {
                self.cfg.ui_mode = match self.cfg.ui_mode {
                    UiMode::Minimal  => UiMode::Standard,
                    UiMode::Standard => UiMode::Analysis,
                    UiMode::Analysis => UiMode::Minimal,
                };
            }
            KeyCode::Char('d') if self.mode == Mode::PvP && human && !over && !self.thinking => {
                self.draw_offer = Some(self.gs.turn);
                self.screen = Screen::DrawOffer;
            }
            KeyCode::Char('e') => {
                self.show_hints = !self.show_hints;
                if self.show_hints {
                    let best = self.move_reviews.last().and_then(|r| r.best_move.clone());
                    if let Some(uci) = best {
                        if let Some(mv) = uci_to_board_move(&uci, &self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast) {
                            self.arrows.push((mv.fr, mv.to));
                        }
                    }
                } else {
                    // Remove hint arrows but keep last-move arrows
                    self.arrows.retain(|&(fr, to)| {
                        self.gs.history.last().map(|h| h.from == fr && h.to == to).unwrap_or(false)
                    });
                }
            }
            KeyCode::Up    | KeyCode::Char('k') => { if self.cursor.0 > 0 { self.cursor.0 -= 1; } }
            KeyCode::Down  | KeyCode::Char('j') => { if self.cursor.0 < 7 { self.cursor.0 += 1; } }
            KeyCode::Left  | KeyCode::Char('h') => { if self.cursor.1 > 0 { self.cursor.1 -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => { if self.cursor.1 < 7 { self.cursor.1 += 1; } }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if !human || over || self.thinking { return; }
                self.do_select(self.to_board(self.cursor));
            }
            KeyCode::Esc => { self.selected = None; self.targets = vec![]; }
            KeyCode::Char(c)
                if human && !over && !self.thinking => { self.input_buf.push(c); self.input_err = None; }
            _ => {}
        }
    }

    pub fn handle_draw_offer_key(&mut self, code: KeyCode) {
        let was_online = self.screen == Screen::OnlineGame || self.screen == Screen::DrawOffer && self.lichess.is_some();
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if was_online {
                    if let Some(client) = &self.lichess {
                        if let Some(ref id) = self.lichess_game_id {
                            if let Err(e) = client.offer_draw(id, true) {
                                rlog!("[lichess] draw offer error: {e}");
                                self.online_status = format!("Draw offer failed: {e}");
                            }
                        }
                    }
                }
                self.game_end  = GameEnd::Draw;
                self.draw_offer = None;
                self.screen    = if was_online { Screen::OnlineGame } else { Screen::Game };
                self.finish_game();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.draw_offer_msg = Some("Draw offer declined.".to_string());
                self.draw_offer = None;
                self.screen    = if was_online { Screen::OnlineGame } else { Screen::Game };
            }
            _ => {
                // In online mode, decline draw and pass key to game handler
                if was_online {
                    self.online_chat_mode = false;
                    self.chat_buf.clear();
                    self.draw_offer = None;
                    self.screen = Screen::OnlineGame;
                    self.handle_online_game_key(code);
                }
            }
        }
    }

    pub fn handle_promo_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left  | KeyCode::Char('h') => { if self.promo_cur > 0 { self.promo_cur -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => { if self.promo_cur < 3 { self.promo_cur += 1; } }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let kind = [Kind::Q, Kind::R, Kind::B, Kind::N][self.promo_cur];
                let (from, to) = (self.promo_from, self.promo_to);
                let all = legal(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast);
                if let Some(&mv) = all.iter().find(|m| m.fr == from && m.to == to && m.promo == Some(kind)) {
                    self.screen = Screen::Game;
                    self.selected = None; self.targets = vec![];
                    self.execute(mv);
                }
            }
            KeyCode::Esc => { self.screen = Screen::Game; self.selected = None; self.targets = vec![]; }
            _ => {}
        }
    }

    pub fn handle_analyze_key(&mut self, code: KeyCode) {
        let is_bad = |i: usize| -> bool {
            self.move_reviews.get(i).map(|r| matches!(r.label, MoveLabel::Inaccuracy | MoveLabel::Mistake | MoveLabel::Blunder)).unwrap_or(false)
        };
        let max = self.replay_snaps.len().saturating_sub(1);
        match code {
            KeyCode::Left  | KeyCode::Char('h') => { if self.replay_idx > 0 { self.replay_idx -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.replay_idx < max { self.replay_idx += 1; }
            }
            KeyCode::PageUp => { self.replay_idx = self.replay_idx.saturating_sub(10); }
            KeyCode::PageDown => { self.replay_idx = (self.replay_idx + 10).min(max); }
            KeyCode::Char('0') | KeyCode::Home => self.replay_idx = 0,
            KeyCode::Char('$') | KeyCode::End  => self.replay_idx = max,
            KeyCode::Char('[') => {
                // Jump to previous bad move
                let start = self.replay_idx.saturating_sub(1);
                for i in (0..start).rev() {
                    if is_bad(i) { self.replay_idx = i + 1; break; }
                }
            }
            KeyCode::Char(']') => {
                // Jump to next bad move
                let start = self.replay_idx;
                for i in start..self.move_reviews.len() {
                    if is_bad(i) { self.replay_idx = i + 1; break; }
                }
            }
            KeyCode::Char('E')                  => self.open_png_preview(),
            KeyCode::Char('G')                  => { self.export_pgn(); if self.pgn_saved_path.is_some() { self.screen = Screen::PgnSaved; } }
            KeyCode::Char('q') | KeyCode::Esc  => self.screen = Screen::Game,
            _ => {}
        }
    }

    pub fn handle_png_preview_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                self.confirm_png_export();
                self.screen = Screen::Game;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.png_preview_path = None;
                self.screen = Screen::Game;
            }
            _ => {}
        }
    }

    // ── Mouse ─────────────────────────────────────────────────────────────────
    pub fn handle_mouse_click(&mut self, col: usize, row: usize) {
        // Allow puzzle screen to handle mouse too
        let is_puzzle = self.screen == Screen::Puzzle;
        let is_online = self.screen == Screen::OnlineGame;
        if self.screen != Screen::Game && !is_puzzle && !is_online { return; }
        let over  = matches!(self.gs.status, Status::Checkmate | Status::Stalemate | Status::Draw)
                    || self.gs.clock_state == ClockState::Flagged;
        let human = is_puzzle || self.mode == Mode::PvP || self.gs.turn == self.player_color;
        if !human || over || self.thinking { return; }
        // Board cell dimensions (computed each frame from terminal size)
        let cell_w = self.cell_w as usize;
        let cell_h = self.cell_h as usize;
        // Row 0 = top bar, row 1 = board border top
        // Then optional coord row, then 8*CELL_H board rows
        const TOP_BAR: usize = 1;
        const BORDER:  usize = 1;
        let coord_rows: usize = if self.cfg.show_coords { 1 } else { 0 };
        // Coord label column is 3 chars wide ("1  " or "8  ")
        let coord_cols: usize = if self.cfg.show_coords { 3 } else { 0 };

        let cell_start_row = TOP_BAR + BORDER + coord_rows;
        let cell_start_col = BORDER + coord_cols;

        if row < cell_start_row || col < cell_start_col { return; }
        let board_row = (row - cell_start_row) / cell_h;
        let board_col = (col - cell_start_col) / cell_w;
        if board_row > 7 || board_col > 7 { return; }
        let (r, c) = if self.flipped() { (7 - board_row, 7 - board_col) } else { (board_row, board_col) };
        self.do_select((r, c));
    }

    pub fn handle_mouse_scroll(&mut self, dir: i8) {
        match self.screen {
            Screen::Settings => {
                if dir < 0 { if self.settings_cur > 0 { self.settings_cur -= 1; } }
                else { if self.settings_cur < 18 { self.settings_cur += 1; } }
            }
            Screen::Menu => {
                if dir < 0 { if self.menu_cur > 0 { self.menu_cur -= 1; } }
                else { if self.menu_cur < 10 { self.menu_cur += 1; } }
            }
            _ => {}
        }
    }

    // ── Analyze ───────────────────────────────────────────────────────────────
    pub fn open_analyze(&mut self) {
        if !self.replay_snaps.is_empty() {
            self.replay_idx = self.replay_snaps.len() - 1;
            self.screen = Screen::Analyze;
        }
    }

    // ── Selection ─────────────────────────────────────────────────────────────
    fn do_select(&mut self, bc: (usize, usize)) {
        let piece = self.gs.board[bc.0][bc.1];
        if let Some(sel) = self.selected {
            if self.targets.iter().any(|m| m.fr == sel && m.to == bc) {
                let has_promo = self.targets.iter().any(|m| m.fr == sel && m.to == bc && m.promo.is_some());
                if has_promo {
                    if self.cfg.auto_promote {
                        // Auto-promote to queen
                        if let Some(&mv) = self.targets.iter().find(|m| m.fr == sel && m.to == bc && m.promo == Some(Kind::Q)) {
                            self.selected = None; self.targets = vec![];
                            self.execute(mv);
                        }
                        return;
                    }
                    self.promo_from = sel; self.promo_to = bc; self.promo_cur = 0;
                    self.screen = Screen::Promo; return;
                }
                if let Some(&mv) = self.targets.iter().find(|m| m.fr == sel && m.to == bc && m.promo.is_none()) {
                    self.selected = None; self.targets = vec![];
                    if self.screen == Screen::Puzzle {
                        // Puzzle: validate move against solution
                        let ff = (b'a' + sel.1 as u8) as char; let fr = 8 - sel.0;
                        let tf = (b'a' + bc.1  as u8) as char; let tr = 8 - bc.0;
                        let expected = self.puzzle.as_ref()
                            .and_then(|p| p.moves.get(self.puzzle_move_idx))
                            .cloned().unwrap_or_default();
                        self.check_puzzle_move(&expected, &format!("{}{}{}{}", ff, fr, tf, tr));
                    } else {
                        self.execute(mv);
                    }
                }
                return;
            }
            if piece.map(|p| p.c == self.gs.turn).unwrap_or(false) {
                let all = legal(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast);
                self.selected = Some(bc);
                self.targets = all.into_iter().filter(|m| m.fr == bc).collect();
            } else {
                self.selected = None; self.targets = vec![];
            }
        } else if piece.map(|p| p.c == self.gs.turn).unwrap_or(false) {
            let all = legal(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast);
            self.selected = Some(bc);
            self.targets = all.into_iter().filter(|m| m.fr == bc).collect();
        }
    }

    // ── Undo ──────────────────────────────────────────────────────────────────
    pub fn undo(&mut self) {
        if self.screen == Screen::OnlineGame || self.screen == Screen::Puzzle { return; }
        let pops = match self.mode {
            Mode::PvP => 1,
            Mode::Cpu => {
                use crate::config::AiDepth;
                if self.cfg.ai_depth == AiDepth::Easy { 1 } else { 2 }
            }
        };
        let take = pops.min(self.undo_stack.len());
        if take == 0 { return; }
        // Push current state onto redo stack before undoing
        self.redo_stack.push((self.gs.clone(), self.cursor.0, self.cursor.1));
        if self.redo_stack.len() > 200 { self.redo_stack.remove(0); }
        let mut snap = None;
        for _ in 0..take { snap = self.undo_stack.pop(); }
        if let Some((gs, cr, cc)) = snap {
            self.gs = gs; self.cursor = (cr, cc);
            self.selected = None; self.targets = vec![];
            self.input_buf.clear(); self.input_err = None;
            if let Some(rx) = self.ai_rx.take() { drop(rx); }
            self.thinking = false; self.last_tick = Instant::now();
            if self.replay_snaps.len() > self.gs.history.len() + 1 {
                self.replay_snaps.truncate(self.gs.history.len() + 1);
            }
            if self.move_reviews.len() > self.gs.history.len() {
                self.move_reviews.truncate(self.gs.history.len());
            }
            if self.eval_history.len() > self.gs.history.len() + 1 {
                self.eval_history.truncate(self.gs.history.len() + 1);
            }
            self.analysis.drain_results();
            self.pending_analysis = None;
            self.engine_busy = false;
            if !self.gs.history.is_empty()
                && self.screen != Screen::OnlineGame
                && self.screen != Screen::Puzzle
            {
                self.queue_analysis_after_move();
            }
        }
    }

    // ── Redo ──────────────────────────────────────────────────────────────────
    pub fn redo(&mut self) {
        if self.screen == Screen::OnlineGame || self.screen == Screen::Puzzle { return; }
        let take = 1.min(self.redo_stack.len());
        if take == 0 { return; }
        // Push current state back onto undo before redoing
        self.undo_stack.push((self.gs.clone(), self.cursor.0, self.cursor.1));
        if self.undo_stack.len() > 200 { self.undo_stack.remove(0); }
        if let Some((gs, cr, cc)) = self.redo_stack.pop() {
            self.gs = gs; self.cursor = (cr, cc);
            self.selected = None; self.targets = vec![];
            self.input_buf.clear(); self.input_err = None;
            self.thinking = false; self.last_tick = Instant::now();
            if self.replay_snaps.len() > self.gs.history.len() + 1 {
                self.replay_snaps.truncate(self.gs.history.len() + 1);
            }
            if self.move_reviews.len() > self.gs.history.len() {
                self.move_reviews.truncate(self.gs.history.len());
            }
            if self.eval_history.len() > self.gs.history.len() + 1 {
                self.eval_history.truncate(self.gs.history.len() + 1);
            }
            self.analysis.drain_results();
            self.pending_analysis = None;
            self.engine_busy = false;
        }
    }

    // ── Execute a move ────────────────────────────────────────────────────────
    pub fn execute(&mut self, mv: Mv) {
        let Some(piece) = self.gs.board[mv.fr.0][mv.fr.1] else { rlog!("execute: empty source square"); return; };
        self.undo_stack.push((self.gs.clone(), self.cursor.0, self.cursor.1));
        if self.undo_stack.len() > 200 { self.undo_stack.remove(0); }
        self.redo_stack.clear();
        let cap   = self.gs.board[mv.to.0][mv.to.1];
        let cap   = if cap.is_none() && mv.ep {
            let cr = if piece.c == Color::White { mv.to.0 + 1 } else { mv.to.0.wrapping_sub(1) };
            self.gs.board[cr][mv.to.1]
        } else { cap };

        let san          = mv_to_san(&self.gs.board, &mv, self.gs.ep, &self.gs.cast);
        let (nb, ne, nc) = apply(&self.gs.board, &mv, self.gs.ep, &self.gs.cast);
        let next         = piece.c.opp();
        let new_s        = game_status(&nb, next, ne, &nc);

        if let Some(c) = cap {
            if piece.c == Color::White { self.gs.cap_w.push(c); } else { self.gs.cap_b.push(c); }
        }

        let ff = (b'a' + mv.fr.1 as u8) as char; let fr = 8 - mv.fr.0;
        let tf = (b'a' + mv.to.1 as u8) as char; let tr = 8 - mv.to.0;
        let ps  = mv.promo.map(|k| match k { Kind::Q=>"Q",Kind::R=>"R",Kind::B=>"B",_=>"N" }).unwrap_or("");
        let sfx = match new_s { Status::Checkmate => "#", Status::Check => "+", _ => "" };
        let coord_note = format!("{}{}{}{}{}{}", ff, fr, tf, tr, ps, sfx);
        let uci_note = format!("{}{}{}{}", ff, fr, tf, tr); // raw UCI for net forwarding
        let san_full   = format!("{}{}", san, sfx);

        let move_time_ms = self.last_tick.elapsed().as_millis() as u64;
        self.gs.history.push(HistEntry {
            notation: coord_note, san: san_full,
            color: piece.c, from: mv.fr, to: mv.to,
            move_time_ms,
        });
        // Sound: bell on check or game events (non-blocking, ignored if terminal has no bell)
        match new_s {
            Status::Check | Status::Checkmate => { print!("\x07"); use std::io::Write; let _ = std::io::stdout().flush(); }
            _ => {}
        }
        self.gs.board = nb; self.gs.turn = next;
        self.gs.ep = ne; self.gs.cast = nc; self.gs.status = new_s;
        if next == Color::White { self.gs.fullmove += 1; }
        self.last_tick = Instant::now();
        self.draw_offer_msg = None;

        self.replay_snaps.push(ReplaySnap {
            board: self.gs.board, turn: next,
            notation: self.gs.history.last().map(|h| h.san.clone()).unwrap_or_default(),
            mv_num: self.gs.fullmove,
        });

        if self.gs.clock_state == ClockState::Paused
            && !matches!(new_s, Status::Checkmate | Status::Stalemate | Status::Draw)
            && self.gs.white_ms.is_some()
        {
            self.gs.clock_state = ClockState::Running;
        }

        if self.cfg.auto_flip && self.mode == Mode::PvP && self.screen != Screen::OnlineGame {
            self.cursor = if self.gs.turn == Color::Black { (6, 3) } else { (6, 4) };
        }

        // Queue background analysis (non-blocking)
        self.queue_analysis_after_move();

        if matches!(new_s, Status::Active | Status::Check) {
            if self.mode == Mode::Cpu && self.gs.turn != self.player_color {
                self.kick_ai();
            }
        } else {
            self.gs.clock_state = ClockState::Paused;
            self.game_end = GameEnd::Normal;
            self.finish_game();
        }

        // Forward move to opponent via Lichess API (only if we moved, not the opponent)
        if self.screen == Screen::OnlineGame && piece.c == self.player_color {
            if let Some(client) = &self.lichess {
                if let Some(ref id) = self.lichess_game_id {
                    let uci = &*uci_note;
                    rlog!("[lichess] sending move: {uci}");
                    if let Err(e) = client.play_move(id, uci) {
                        rlog!("[lichess] send move error: {e}");
                        self.online_status = format!("Move error: {e} — undoing");
                        // Rollback: restore previous state
                        if let Some((prev_gs, prev_cx, prev_cy)) = self.undo_stack.pop() {
                            self.gs = prev_gs;
                            self.cursor = (prev_cx, prev_cy);
                        }
                    }
                }
            }
        }
    }

    // ── FEN Import ────────────────────────────────────────────────────────────
    pub fn open_load_game(&mut self) {
        self.saved_games = crate::save::list_saves();
        self.menu_cur = 0;
        self.screen = Screen::LoadGame;
    }

    pub fn load_saved_game(&mut self, _path: std::path::PathBuf, save: crate::save::GameSave) {
        self.start_game();
        self.gs.history.clear();
        self.replay_snaps.clear();
        for (i, uci) in save.moves.iter().enumerate() {
            if let Ok(mv) = crate::app::parse_notation(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast, uci) {
                let san = crate::engine::mv_to_san(&self.gs.board, &mv, self.gs.ep, &self.gs.cast);
                self.replay_snaps.push(crate::app::ReplaySnap {
                    board: self.gs.board,
                    turn: self.gs.turn,
                    notation: san.clone(),
                    mv_num: (i as u32 / 2) + 1,
                });
                let (nb, ne, nc) = crate::engine::apply(&self.gs.board, &mv, self.gs.ep, &self.gs.cast);
                self.gs.board = nb; self.gs.ep = ne; self.gs.cast = nc;
                self.gs.turn = self.gs.turn.opp();
            }
        }
        self.replay_snaps.push(crate::app::ReplaySnap {
            board: self.gs.board,
            turn: self.gs.turn,
            notation: save.result.clone(),
            mv_num: (save.moves.len() as u32 / 2) + 1,
        });
        self.replay_idx = self.replay_snaps.len() - 1;
        self.screen = Screen::Analyze;
    }

    pub fn handle_load_game_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up | KeyCode::Char('k') => { if self.menu_cur > 0 { self.menu_cur -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if self.menu_cur + 1 < self.saved_games.len() { self.menu_cur += 1; } }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some((path, save)) = self.saved_games.get(self.menu_cur).cloned() {
                    self.load_saved_game(path, save);
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Menu,
            _ => {}
        }
    }

    pub fn open_import(&mut self) {
        self.import_buf.clear();
        self.import_err = None;
        self.screen = Screen::Import;
    }

    pub fn handle_import_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.import_buf.push(c);
                self.import_err = None;
            }
            KeyCode::Backspace => {
                self.import_buf.pop();
                self.import_err = None;
            }
            KeyCode::Esc => { self.screen = Screen::Menu; }
            KeyCode::Enter => {
                let text = self.import_buf.trim().to_string();
                if text.is_empty() { return; }

                // Auto-detect: file path, FEN, or PGN
                let is_path = text.starts_with('/') || text.starts_with('~');
                let is_fen = !is_path && (text.contains('/') && text.chars().filter(|&c| c == ' ').count() <= 5
                    && (text.starts_with(|c: char| c.is_ascii_digit() || "KkQqRrBbNnPp".contains(c))));

                let result: Result<crate::pgn_import::PgnGame, String> = if is_path {
                    let path = shellexpand_tilde(&text);
                    crate::pgn_import::load_pgn_file(&path)
                } else if is_fen {
                    // Parse as FEN, create a synthetic PgnGame
                    match crate::puzzle::parse_fen(&text) {
                        Ok((board, turn, ep, cast, fullmove)) => {
                            return self.apply_fen(board, turn, ep, cast, fullmove);
                        }
                        Err(e) => Err(format!("Invalid FEN: {}", e)),
                    }
                } else {
                    crate::pgn_import::parse_pgn(&text)
                };

                match result {
                    Ok(game) => self.load_pgn(game),
                    Err(e)   => self.import_err = Some(e),
                }
            }
            _ => {}
        }
    }

    fn apply_fen(&mut self, board: crate::engine::Board, turn: crate::engine::Color,
        ep: Option<(usize, usize)>, cast: crate::engine::Castle, fullmove: u32)
    {
        self.gs = Gs::new(self.cfg.time_control);
        self.gs.board    = board;
        self.gs.turn     = turn;
        self.gs.ep       = ep;
        self.gs.cast     = cast;
        self.gs.fullmove = fullmove;
        self.import_buf.clear();
        self.import_err = None;
        self.screen = Screen::Game;
        self.selected = None; self.targets = vec![];
        self.thinking = false; self.ai_rx = None;
        self.move_reviews.clear(); self.eval_history = vec![0];
        self.replay_snaps.clear();
        self.replay_snaps.push(ReplaySnap {
            board: self.gs.board, turn: self.gs.turn,
            notation: String::new(), mv_num: fullmove,
        });
        if self.mode == Mode::Cpu && self.gs.turn != self.player_color {
            self.kick_ai();
        }
    }

    /// Load a PGN game (by text or file path) and open analyze.
    fn load_pgn(&mut self, game: crate::pgn_import::PgnGame) {
        self.mode = Mode::PvP;
        self.player_color = Color::White;
        self.start_game();
        let moves = game.uci_moves.clone();
        for uci in &moves {
            match parse_notation(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast, uci) {
                Ok(m)  => self.execute(m),
                Err(e) => { rlog!("[rchess/pgn] move error {}: {}", uci, e); break; }
            }
        }
        self.import_buf.clear();
        self.import_err = None;
        self.open_analyze();
    }

    // ── Puzzle ────────────────────────────────────────────────────────────────
    pub fn open_puzzle(&mut self) {
        self.puzzle       = None;
        self.puzzle_state = crate::puzzle::PuzzleState::Loading;
        self.puzzle_move_idx = 0;
        self.screen       = Screen::Puzzle;

        // Fetch in background thread — capture token before move into thread
        let cfg_token = self.cfg.lichess_token.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.puzzle_rx = Some(rx);
        std::thread::spawn(move || {
            let result = crate::puzzle::fetch_daily_puzzle(&cfg_token);
            let _ = tx.send(result);
        });
    }

    pub fn poll_puzzle(&mut self) {
        // Poll background fetch
        if self.puzzle_state == crate::puzzle::PuzzleState::Loading {
            if let Some(rx) = &self.puzzle_rx {
                if let Ok(result) = rx.try_recv() {
                    self.puzzle_rx = None;
                    match result {
                        Ok(p) => {
                            rlog!("[rchess/puzzle] loaded puzzle {} rating={}", p.id, p.rating);
                            self.setup_puzzle(p);
                        }
                        Err(e) => {
                            rlog!("[rchess/puzzle] fetch failed: {}", e);
                            self.puzzle_state = crate::puzzle::PuzzleState::Failed(e);
                        }
                    }
                }
            }
        }

        // Transition straight to waiting for human input
        if self.puzzle_state == crate::puzzle::PuzzleState::Setup {
            if let Some(ref p) = self.puzzle {
                self.puzzle_move_idx = p.solution_start;
            }
            self.puzzle_state = crate::puzzle::PuzzleState::WaitingInput;
        }
    }

    pub fn poll_online(&mut self) {
        if let Some(client) = &mut self.lichess {
            client.poll_username();
            if client.connected && !self.lichess_connected {
                self.player_name = client.username.clone();
                self.cfg.lichess_username = client.username.clone();
                self.lichess_connected = true;
                self.online_status = "Connected to Lichess".into();
            } else if !client.connected && client.username.is_empty() {
                // still waiting
            } else if !client.connected && !client.username.is_empty() && client.username != "?" {
                self.lichess_connected = true;
                self.player_name = client.username.clone();
                self.cfg.lichess_username = client.username.clone();
                self.online_status = "Connected to Lichess".into();
            }
        }
    }

    fn setup_puzzle(&mut self, p: crate::puzzle::Puzzle) {
        // Parse the puzzle FEN
        if let Ok((board, turn, ep, cast, fullmove)) = crate::puzzle::parse_fen(&p.fen) {
            self.gs = Gs::new(self.cfg.time_control);
            self.gs.board    = board;
            self.gs.turn     = turn;
            self.gs.ep       = ep;
            self.gs.cast     = cast;
            self.gs.fullmove = fullmove;
            self.gs.clock_state = ClockState::Paused;
            self.selected = None; self.targets = vec![];
            self.move_reviews.clear(); self.eval_history = vec![0];
            self.replay_snaps.clear();
            self.replay_snaps.push(ReplaySnap {
                board: self.gs.board, turn: self.gs.turn,
                notation: String::new(), mv_num: fullmove,
            });
            self.puzzle = Some(p);
            self.puzzle_state = crate::puzzle::PuzzleState::Setup;
        } else {
            self.puzzle_state = crate::puzzle::PuzzleState::Failed("FEN parse failed".to_string());
        }
    }

    pub fn handle_puzzle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.screen = Screen::Menu;
                return;
            }
            KeyCode::Char('n') => {
                self.open_puzzle();
                return;
            }
            _ => {}
        }

        // Navigation always works
        match code {
            KeyCode::Up    | KeyCode::Char('k') => { if self.cursor.0 > 0 { self.cursor.0 -= 1; } }
            KeyCode::Down  | KeyCode::Char('j') => { if self.cursor.0 < 7 { self.cursor.0 += 1; } }
            KeyCode::Left  | KeyCode::Char('h') => { if self.cursor.1 > 0 { self.cursor.1 -= 1; } }
            KeyCode::Right | KeyCode::Char('l') if self.cursor.1 < 7 => { self.cursor.1 += 1; }
            _ => {}
        }

        // CorrectMove → WaitingInput only on action keys, not navigation
        if self.puzzle_state == crate::puzzle::PuzzleState::CorrectMove {
            match code {
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Tab | KeyCode::Char('\n') => {
                    self.puzzle_state = crate::puzzle::PuzzleState::WaitingInput;
                }
                _ => {}
            }
        }

        let is_wrong = matches!(self.puzzle_state, crate::puzzle::PuzzleState::WrongMove(_));
        if self.puzzle_state != crate::puzzle::PuzzleState::WaitingInput && !is_wrong { return; }

        // Clear WrongMove on any input
        if is_wrong {
            self.puzzle_state = crate::puzzle::PuzzleState::WaitingInput;
        }

        let p = match &self.puzzle { Some(p) => p.clone(), None => return };
        let expected_uci = match p.moves.get(self.puzzle_move_idx) {
            Some(m) => m.clone(),
            None    => return,
        };

        // Input: typing notation or selecting
        if !self.input_buf.is_empty() {
            match code {
                KeyCode::Enter => {
                    let buf = self.input_buf.clone();
                    self.input_buf.clear(); self.input_err = None;
                    self.check_puzzle_move(&expected_uci, &buf);
                }
                KeyCode::Backspace => { self.input_buf.pop(); }
                KeyCode::Esc       => { self.input_buf.clear(); self.input_err = None; }
                KeyCode::Char(c)   => { self.input_buf.push(c); }
                _ => {}
            }
        } else {
            match code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    let pos = self.to_board(self.cursor);
                    if let Some(sel) = self.selected {
                        if self.targets.iter().any(|m| m.fr == sel && m.to == pos) {
                            let ff = (b'a' + sel.1 as u8) as char;
                            let fr = 8 - sel.0;
                            let tf = (b'a' + pos.1 as u8) as char;
                            let tr = 8 - pos.0;
                            let uci_played = format!("{}{}{}{}", ff, fr, tf, tr);
                            self.selected = None; self.targets = vec![];
                            self.check_puzzle_move(&expected_uci, &uci_played);
                        } else {
                            self.selected = None; self.targets = vec![];
                        }
                    } else if self.gs.board[pos.0][pos.1]
                        .map(|p| p.c == self.gs.turn).unwrap_or(false)
                    {
                        let all = legal(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast);
                        self.selected = Some(pos);
                        self.targets  = all.into_iter().filter(|m| m.fr == pos).collect();
                    }
                }
                KeyCode::Char(c) => { self.input_buf.push(c); }
                _ => {}
            }
        }
    }

    fn check_puzzle_move(&mut self, expected_uci: &str, played: &str) {
        let Some(_) = self.puzzle.as_ref() else { return; };
        let played_clean = played.trim().to_lowercase();
        let expected_clean = expected_uci.trim().to_lowercase();

        // Parse played move to canonical UCI
        let played_uci = match parse_notation(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast, &played_clean) {
            Ok(mv) => {
                let ff = (b'a' + mv.fr.1 as u8) as char;
                let fr = 8 - mv.fr.0;
                let tf = (b'a' + mv.to.1 as u8) as char;
                let tr = 8 - mv.to.0;
                let promo = mv.promo.map(|k| match k {
                    crate::engine::Kind::Q => "q",
                    crate::engine::Kind::R => "r",
                    crate::engine::Kind::B => "b",
                    _ => "n",
                }).unwrap_or("");
                format!("{}{}{}{}{}", ff, fr, tf, tr, promo)
            }
            Err(e) => { self.input_err = Some(e); return; }
        };

        if played_uci == expected_clean {
            // Correct! Execute the move
            if let Ok(mv) = parse_notation(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast, &played_clean) {
                self.execute(mv);
            }
            self.puzzle_move_idx += 1;
            self.input_buf.clear();

            let p = self.puzzle.as_ref().unwrap();
            if self.puzzle_move_idx >= p.moves.len() {
                // Solved!
                self.puzzle_state = crate::puzzle::PuzzleState::Solved;
                print!("\x07"); use std::io::Write; let _ = std::io::stdout().flush();
            } else {
                // Play the opponent's response
                let p = self.puzzle.as_ref().unwrap().clone();
                if let Some(opp_uci) = p.moves.get(self.puzzle_move_idx) {
                    if let Ok(opp_mv) = parse_notation(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast, opp_uci) {
                        self.execute(opp_mv);
                        self.puzzle_move_idx += 1;
                    }
                }
                // Check if more human moves remain
                let p = self.puzzle.as_ref().unwrap();
                if self.puzzle_move_idx >= p.moves.len() {
                    self.puzzle_state = crate::puzzle::PuzzleState::Solved;
                } else {
                    self.puzzle_state = crate::puzzle::PuzzleState::CorrectMove;
                }
            }
        } else {
            self.puzzle_state = crate::puzzle::PuzzleState::WrongMove(
                format!("Wrong: expected {}, played {}", expected_uci, played_uci));
        }
    }

}

// ── Helpers ───────────────────────────────────────────────────────────────────

// ── Stockfish as CPU opponent ─────────────────────────────────────────────────
/// Spawn Stockfish, send position, get best move. Returns None if SF not installed.
fn stockfish_best_move(uci_moves: &str, skill: u8) -> Option<crate::engine::Mv> {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    let mut child = Command::new("/usr/bin/stockfish")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().ok()?;

    let stdin  = child.stdin.take()?;
    let stdout = child.stdout.take()?;
    let mut w      = std::io::BufWriter::new(stdin);
    let mut reader = BufReader::new(stdout);

    writeln!(w, "uci").ok();
    writeln!(w, "setoption name Skill Level value {}", skill.min(20)).ok();
    writeln!(w, "isready").ok();
    w.flush().ok();

    // Wait for readyok
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 { break; }
        if line.trim() == "readyok" { break; }
    }

    // Send position
    if uci_moves.trim().is_empty() {
        writeln!(w, "position startpos").ok();
    } else {
        writeln!(w, "position startpos moves {}", uci_moves.trim()).ok();
    }
    writeln!(w, "go movetime 300").ok();
    w.flush().ok();

    // Read until bestmove
    let mut best_uci: Option<String> = None;
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let t = line.trim();
        if t.starts_with("bestmove") {
            best_uci = t.split_whitespace().nth(1)
                .filter(|&s| s != "(none)")
                .map(|s| s.to_string());
            break;
        }
    }
    writeln!(w, "quit").ok();
    let _ = child.wait();

    let uci = best_uci?;
    crate::rlog!("[rchess/sf-cpu] bestmove: {}", uci);

    let b = uci.as_bytes();
    if b.len() < 4 { return None; }
    if !(b[0] >= b'a' && b[0] <= b'h') { return None; }
    let from = ((b'8' - b[1]) as usize, (b[0] - b'a') as usize);
    let to   = ((b'8' - b[3]) as usize, (b[2] - b'a') as usize);
    let promo = b.get(4).and_then(|&p| match p.to_ascii_lowercase() {
        b'q' => Some(crate::engine::Kind::Q),
        b'r' => Some(crate::engine::Kind::R),
        b'b' => Some(crate::engine::Kind::B),
        b'n' => Some(crate::engine::Kind::N),
        _    => None,
    });

    Some(crate::engine::Mv { fr: from, to, promo, ep: false, castle: 0 })
}


/// Expand leading ~ to $HOME in file paths
fn shellexpand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{}{}", home, &path[1..])
    } else { path.to_string() }
}

pub fn export_dir() -> PathBuf {
    let base = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("rchess_export");
    if let Err(e) = fs::create_dir_all(&dir) {
        rlog!("[rchess] export_dir error: {}", e);
    }
    dir
}

// ── Notation parser ───────────────────────────────────────────────────────────
pub fn parse_notation(
    board: &Board, color: Color,
    ep: Option<(usize, usize)>, cast: &Castle,
    input: &str,
) -> Result<Mv, String> {
    let moves = legal(board, color, ep, cast);
    if moves.is_empty() { return Err("No legal moves".into()); }
    let raw = input.trim();
    let s   = raw.trim_end_matches('+').trim_end_matches('#').trim();
    match s.to_lowercase().as_str() {
        "o-o-o" | "0-0-0" =>
            return moves.iter().find(|m| m.castle == 2).copied()
                .ok_or_else(|| "Queenside castling not available".into()),
        "o-o" | "0-0" =>
            return moves.iter().find(|m| m.castle == 1).copied()
                .ok_or_else(|| "Kingside castling not available".into()),
        _ => {}
    }
    let coord: String = s.chars().filter(|c| c.is_alphanumeric()).collect();
    let cb = coord.as_bytes();
    if cb.len() >= 4
        && (b'a'..=b'h').contains(&cb[0]) && (b'1'..=b'8').contains(&cb[1])
        && (b'a'..=b'h').contains(&cb[2]) && (b'1'..=b'8').contains(&cb[3])
    {
        let from = ((8 - (cb[1] - b'0')) as usize, (cb[0] - b'a') as usize);
        let to   = ((8 - (cb[3] - b'0')) as usize, (cb[2] - b'a') as usize);
        let promo = cb.get(4).and_then(|&p| match p.to_ascii_lowercase() {
            b'q' => Some(Kind::Q), b'r' => Some(Kind::R),
            b'b' => Some(Kind::B), b'n' => Some(Kind::N), _ => None,
        });
        if let Some(mv) = moves.iter().find(|m| {
            m.fr == from && m.to == to
            && match promo {
                Some(p) => m.promo == Some(p),
                None    => m.promo.is_none() || m.promo == Some(Kind::Q),
            }
        }) {
            let fin = if promo.is_none() && mv.promo.is_some() {
                moves.iter().find(|m| m.fr == from && m.to == to && m.promo == Some(Kind::Q))
                    .copied().unwrap_or(*mv)
            } else { *mv };
            return Ok(fin);
        }
        let fn_ = format!("{}{}", (b'a' + from.1 as u8) as char, 8 - from.0);
        let tn  = format!("{}{}", (b'a' + to.1   as u8) as char, 8 - to.0);
        let hint = if board[from.0][from.1].is_none() { " (empty square)" }
                   else if board[from.0][from.1].map(|p| p.c != color).unwrap_or(false) { " (opponent piece)" }
                   else { "" };
        return Err(format!("{} \u{2192} {} not legal{}", fn_, tn, hint));
    }
    san_parse(board, color, &moves, s)
        .ok_or_else(|| format!("'{}' \u{2014} try: e2e4  Nf3  Qg4  O-O  O-O-O", raw))
}

// ── Networking ───────────────────────────────────────────────────────────────

impl App {
    pub fn handle_online_lobby_key(&mut self, code: KeyCode) {
        // Auto-connect if token is set
        if self.lichess.is_none() && !self.cfg.lichess_token.is_empty() {
            let client = LichessNetClient::new(&self.cfg.lichess_token);
            self.lichess = Some(client);
            self.lichess_connected = false;
            self.online_status = "Connecting...".into();
        }

        // Poll for async username fetch
        if let Some(client) = &mut self.lichess {
            client.poll_username();
            if !client.connected && client.username.is_empty() {
                // still waiting
            } else if !self.lichess_connected {
                self.player_name = client.username.clone();
                self.cfg.lichess_username = client.username.clone();
                self.lichess_connected = true;
                self.online_status = "Connected to Lichess".into();
            }
        }

        if self.challenge_mode {
            match code {
                KeyCode::Esc => { self.challenge_mode = false; self.challenge_buf.clear(); }
                KeyCode::Char(c) if c.is_ascii_alphanumeric() || c == '-' || c == '_' => {
                    self.challenge_buf.push(c);
                }
                KeyCode::Backspace => { self.challenge_buf.pop(); }
                KeyCode::Enter
                    if !self.challenge_buf.is_empty() => {
                        if let Some(client) = &self.lichess {
                            let uname = self.challenge_buf.clone();
                            let tcs = [(10u32, 0u32), (5u32, 5u32), (15u32, 10u32), (30u32, 0u32)];
                            let (limit, inc) = tcs[self.online_tc_idx.min(tcs.len() - 1)];
                            match client.challenge_user(&uname, self.online_rated, limit, inc) {
                                Ok(()) => self.online_status = format!("Challenge sent to {uname}"),
                                Err(e) => self.online_status = format!("Challenge failed: {e}"),
                            }
                        }
                        self.challenge_buf.clear();
                        self.challenge_mode = false;
                    }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                if self.online_seeking {
                    if let Some(client) = &self.lichess {
                        if let Err(e) = client.cancel_seek() {
                            rlog!("[lichess] cancel seek error: {e}");
                        }
                    }
                }
                self.online_seeking = false;
                self.screen = Screen::Menu;
            }
            KeyCode::Char('c') => {
                if self.lichess.is_some() {
                    self.challenge_mode = true;
                    self.challenge_buf.clear();
                    self.online_status = "Type opponent username, Enter to challenge".into();
                } else {
                    self.online_status = "Set lichess_token in Settings first".into();
                }
            }
            KeyCode::Char('a') => {
                // Accept the first challenge in the list
                if let Some(ch) = self.lichess_challenges.first() {
                    let cid = ch.id.clone();
                    if let Some(client) = &self.lichess {
                        match client.accept_challenge(&cid) {
                            Ok(()) => self.online_status = "Challenge accepted!".into(),
                            Err(e) => self.online_status = format!("Accept failed: {e}"),
                        }
                    }
                }
            }
            KeyCode::Char('d') => {
                // Decline the first challenge
                if let Some(ch) = self.lichess_challenges.first() {
                    let cid = ch.id.clone();
                    if let Some(client) = &self.lichess {
                        if let Err(e) = client.decline_challenge(&cid) {
                            rlog!("[lichess] decline challenge error: {e}");
                            self.online_status = format!("Decline failed: {e}");
                        }
                    }
                    self.lichess_challenges.remove(0);
                }
            }
            KeyCode::Char('p') => {
                // Play random: create a lobby seek
                if let Some(client) = &self.lichess {
                    let tcs = [("10+0", 10u32, 0u32), ("5+5", 5u32, 5u32),
                               ("15+10", 15u32, 10u32), ("30+0", 30u32, 0u32)];
                    let (_, limit, inc) = tcs[self.online_tc_idx.min(tcs.len() - 1)];
                    match client.create_seek(self.online_rated, limit, inc) {
                        Ok(()) => {
                            self.online_seeking = true;
                            self.online_status = format!("Seeking {} {}+{} game...", if self.online_rated { "rated" } else { "casual" }, limit, inc);
                        }
                        Err(e) => self.online_status = format!("Seek failed: {e}"),
                    }
                } else {
                    self.online_status = "Set lichess_token in Settings first".into();
                }
            }
            KeyCode::Char('r') => {
                self.online_rated = !self.online_rated;
                self.online_status = if self.online_rated { "Rated games".into() } else { "Casual games".into() };
            }
            KeyCode::Char('t') => {
                let tcs = ["10+0", "5+5", "15+10", "30+0"];
                self.online_tc_idx = (self.online_tc_idx + 1) % tcs.len();
                self.online_status = format!("Time control: {}", tcs[self.online_tc_idx]);
            }
            KeyCode::Char('s') => {
                self.prev_screen = self.screen;
                self.screen = Screen::Settings;
            }
            KeyCode::Up => {
                if self.menu_cur > 0 { self.menu_cur -= 1; }
            }
            KeyCode::Down
                if self.menu_cur < 10 => { self.menu_cur += 1; }
            _ => {}
        }
    }

    /// Repurposed: handle online game screen input
    pub fn handle_online_game_key(&mut self, code: KeyCode) {
        // Chat compose mode
        if self.online_chat_mode {
            match code {
                KeyCode::Esc => { self.online_chat_mode = false; self.chat_buf.clear(); }
                KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => { self.chat_buf.push(c); }
                KeyCode::Backspace => { self.chat_buf.pop(); }
                KeyCode::Enter => {
                    if !self.chat_buf.is_empty() {
                        let msg = self.chat_buf.clone();
                        if let Some(client) = &self.lichess {
                            if let Some(ref id) = self.lichess_game_id {
                                if let Err(e) = client.chat(id, "player", &msg) {
                                    rlog!("[lichess] chat error: {e}");
                                }
                            }
                        }
                        self.chat_messages.push(format!("You: {msg}"));
                        if self.chat_messages.len() > 50 { self.chat_messages.remove(0); }
                        self.chat_buf.clear();
                    }
                    self.online_chat_mode = false;
                    self.online_status = "Chat sent".into();
                }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                if !matches!(self.gs.status, Status::Checkmate | Status::Stalemate | Status::Draw) {
                    self.save_incomplete_game();
                }
                if let Some(client) = &mut self.lichess {
                    client.stop_game_stream();
                }
                self.screen = Screen::OnlineLobby;
            }
            KeyCode::Char('s') => {
                self.prev_screen = self.screen;
                self.screen = Screen::Settings;
            }
            KeyCode::Char('a') => {
                if self.lichess.is_none() || self.lichess_game_id.is_none() {
                    self.online_status = "Abort: not connected to a game".into();
                } else if let Some(client) = &self.lichess {
                    if let Some(ref id) = self.lichess_game_id {
                        match client.abort_game(id) {
                            Ok(()) => {
                                self.online_status = "Game aborted".into();
                                self.screen = Screen::OnlineLobby;
                            }
                            Err(e) => {
                                self.online_status = format!("Abort failed: {e}");
                                rlog!("[lichess] abort error: {e}");
                            }
                        }
                    }
                }
            }
            KeyCode::Char('c') => {
                self.online_chat_mode = true;
                self.chat_buf.clear();
                self.online_status = "Type message, Enter to send, Esc to cancel".into();
            }
            KeyCode::Char('d') => {
                if self.lichess.is_none() || self.lichess_game_id.is_none() {
                    self.online_status = "Draw: not connected to a game".into();
                } else if let Some(client) = &self.lichess {
                    if let Some(ref id) = self.lichess_game_id {
                        match client.offer_draw(id, true) {
                            Ok(()) => self.online_status = "Draw offered".into(),
                            Err(e) => {
                                self.online_status = format!("Draw offer failed: {e}");
                                rlog!("[lichess] draw error: {e}");
                            }
                        }
                    }
                }
            }
            KeyCode::Char('x') => {
                if self.lichess.is_none() || self.lichess_game_id.is_none() {
                    self.online_status = "Resign: not connected to a game".into();
                } else if let Some(client) = &self.lichess {
                    if let Some(ref id) = self.lichess_game_id {
                        match client.resign_game(id) {
                            Ok(()) => {
                                self.online_status = "You resigned".into();
                                self.screen = Screen::OnlineLobby;
                            }
                            Err(e) => {
                                self.online_status = format!("Resign failed: {e}");
                                rlog!("[lichess] resign error: {e}");
                            }
                        }
                    }
                }
            }
            KeyCode::Char('t') => {
                if self.lichess.is_none() || self.lichess_game_id.is_none() {
                    self.online_status = "Takeback: not connected to a game".into();
                } else if let Some(client) = &self.lichess {
                    if let Some(ref id) = self.lichess_game_id {
                        match client.takeback(id, true) {
                            Ok(()) => self.online_status = "Takeback offered".into(),
                            Err(e) => {
                                self.online_status = format!("Takeback failed: {e}");
                                rlog!("[lichess] takeback error: {e}");
                            }
                        }
                    }
                }
            }
            KeyCode::Char('v') => {
                if self.lichess.is_none() || self.lichess_game_id.is_none() {
                    self.online_status = "Claim victory: not connected to a game".into();
                } else if let Some(client) = &self.lichess {
                    if let Some(ref id) = self.lichess_game_id {
                        match client.claim_victory(id) {
                            Ok(()) => self.online_status = "Victory claimed".into(),
                            Err(e) => {
                                self.online_status = format!("Claim failed: {e}");
                                rlog!("[lichess] claim_victory error: {e}");
                            }
                        }
                    }
                }
            }
            // Block new-game (n) in online mode — would desync the board
            KeyCode::Char('n') => {
                self.online_status = "Press Esc/q to return to lobby, then start a new game".into();
            }
            _ => self.handle_game_key(code),
        }
    }
}

fn san_parse(board: &Board, color: Color, moves: &[Mv], s: &str) -> Option<Mv> {
    let raw_b = s.as_bytes();
    if raw_b.is_empty() { return None; }
    let (piece_kind, skip) = match raw_b[0] {
        b'N' | b'n' => (Kind::N, 1), b'B' => (Kind::B, 1),
        b'R' | b'r' => (Kind::R, 1), b'Q' | b'q' => (Kind::Q, 1), b'K' | b'k' => (Kind::K, 1),
        b'b' => {
            let nx = raw_b.get(1).copied().unwrap_or(0);
            if (b'1'..=b'8').contains(&nx) { (Kind::P, 0) } else { (Kind::B, 1) }
        }
        _ => (Kind::P, 0),
    };
    let rest: String = s[skip..].chars()
        .filter(|&c| c != 'x' && c != 'X' && c != '+' && c != '#')
        .collect::<String>().to_lowercase();
    let (dest_raw, promo) = if let Some(eq) = rest.find('=') {
        let p = rest.as_bytes().get(eq + 1).and_then(|&c| match c {
            b'q'=>Some(Kind::Q),b'r'=>Some(Kind::R),b'b'=>Some(Kind::B),b'n'=>Some(Kind::N),_=>None,
        });
        (rest[..eq].to_string(), p)
    } else { (rest.clone(), None) };
    let db = dest_raw.as_bytes();
    if db.len() < 2 { return None; }
    let to_f = *db.get(db.len() - 2)?;
    let to_r = *db.last()?;
    if !(b'a'..=b'h').contains(&to_f) || !(b'1'..=b'8').contains(&to_r) { return None; }
    let to = ((8 - (to_r - b'0')) as usize, (to_f - b'a') as usize);
    let disambig = &dest_raw[..dest_raw.len() - 2];
    let dis_f = disambig.bytes().find(|&b| (b'a'..=b'h').contains(&b)).map(|b| (b - b'a') as usize);
    let dis_r = disambig.bytes().find(|b| b.is_ascii_digit()).map(|b| (8 - (b - b'0')) as usize);
    let is_promo_sq = to.0 == 0 || to.0 == 7;
    let tgt_promo = promo.or_else(|| if piece_kind == Kind::P && is_promo_sq { Some(Kind::Q) } else { None });
    moves.iter().find(|m| {
        let sq = board[m.fr.0][m.fr.1];
        sq.map(|p| p.c == color && p.k == piece_kind).unwrap_or(false)
            && m.to == to
            && dis_f.map(|f| m.fr.1 == f).unwrap_or(true)
            && dis_r.map(|r| m.fr.0 == r).unwrap_or(true)
            && match tgt_promo { Some(p) => m.promo == Some(p), None => m.promo.is_none() }
    }).copied()
}

/// Convert a UCI string (e.g. "e2e4") to an Mv by matching against legal moves.
fn uci_to_board_move(uci: &str, board: &Board, turn: Color, ep: Option<(usize, usize)>, cast: &Castle) -> Option<Mv> {
    let cb = uci.as_bytes();
    if cb.len() < 4 { return None; }
    let from = ((8 - (cb[1] - b'0')) as usize, (cb[0] - b'a') as usize);
    let to   = ((8 - (cb[3] - b'0')) as usize, (cb[2] - b'a') as usize);
    let promo = cb.get(4).and_then(|&p| match p.to_ascii_lowercase() {
        b'q' => Some(Kind::Q), b'r' => Some(Kind::R),
        b'b' => Some(Kind::B), b'n' => Some(Kind::N), _ => None,
    });
    let moves = crate::engine::legal(board, turn, ep, cast);
    let mv = if from.0 < 8 && from.1 < 8 && to.0 < 8 && to.1 < 8 {
        moves.iter().find(|m| m.fr == from && m.to == to && m.promo == promo).copied()
            .or_else(|| moves.iter().find(|m| m.fr == from && m.to == to && promo.is_none()).copied())
    } else { None };
    mv
}

fn copy_to_clipboard(text: &str) -> bool {
    use std::process::Command;
    if Command::new("/usr/bin/xclip").arg("-selection").arg("clipboard").arg("-i").arg("-l").arg("1").arg("-silent")
        .stdin(std::process::Stdio::piped()).spawn()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin.take().unwrap().write_all(text.as_bytes())?;
            c.wait()?;
            Ok(())
        }).is_ok() { return true; }
    if Command::new("/usr/bin/wl-copy").arg("--trim").arg("--foreground")
        .stdin(std::process::Stdio::piped()).spawn()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin.take().unwrap().write_all(text.as_bytes())?;
            c.wait()?;
            Ok(())
        }).is_ok() { return true; }
    false
}
