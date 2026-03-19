// src/app.rs
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;
use std::fs;
use std::path::PathBuf;
use crossterm::event::KeyCode;
use crate::engine::*;
use crate::ai::best_mv;
use crate::config::{Config, TimeControl, UiMode};

pub const VERSION: &str = "0.7.0";

// ── Screens ───────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Menu, ColorPick, Settings,
    Game, Promo,
    DrawOffer,
    Replay,
    PngPreview,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode { PvP, CPU }

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
}

// ── Move classification ───────────────────────────────────────────────────────
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoveLabel { Book, Good, Inaccuracy, Mistake, Blunder }

impl MoveLabel {
    /// delta_cp = (eval_after_white - eval_before_white) for White mover,
    ///          = (eval_before_white - eval_after_white) for Black mover.
    /// Negative delta = bad move.
    pub fn from_delta_cp(delta_cp: i32) -> Self {
        if      delta_cp > -50  { MoveLabel::Good }
        else if delta_cp > -100 { MoveLabel::Inaccuracy }
        else if delta_cp > -300 { MoveLabel::Mistake }
        else                    { MoveLabel::Blunder }
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
    // Replay
    pub replay_snaps:  Vec<ReplaySnap>,
    pub replay_idx:    usize,
    pub replay_result: String,
    // Export
    pub pgn_saved_path:   Option<String>,
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
    // Move review data
    pub move_reviews: Vec<MoveReview>,
    // Latest White-positive eval in centipawns
    pub last_eval_cp: i32,
    // Eval at each half-move for sparkline (White-positive cp)
    pub eval_history: Vec<i32>,
    // Misc
    pub should_quit:  bool,
    pub saved_notice: Option<u8>,
    pub undo_stack:   Vec<(Gs, usize, usize)>,
    pub game_end:     GameEnd,
    pub board_col_off: usize,
    pub board_row_off: usize,
}

impl App {
    pub fn new() -> Self {
        let cfg = Config::load();
        let analysis = crate::analysis::AnalysisHandle::spawn(cfg.analysis_engine, cfg.analysis_depth);
        Self {
            screen: Screen::Menu, mode: Mode::PvP,
            player_color: Color::White, gs: Gs::new(cfg.time_control), cfg,
            cursor: (6, 4), selected: None, targets: vec![],
            input_buf: String::new(), input_err: None,
            promo_from: (0,0), promo_to: (0,0), promo_cur: 0,
            menu_cur: 0, color_cur: 0, settings_cur: 0,
            draw_offer: None, draw_offer_msg: None,
            replay_snaps: vec![], replay_idx: 0, replay_result: String::new(),
            pgn_saved_path: None, png_export_path: None,
            png_notice: None, png_preview_path: None,
            last_tick: Instant::now(),
            thinking: false, ai_rx: None,
            analysis,
            engine_busy: false,
            move_reviews: vec![],
            last_eval_cp: 0,
            eval_history: vec![0],  // start with balanced
            should_quit: false, saved_notice: None, undo_stack: vec![],
            game_end: GameEnd::Normal,
            board_col_off: 4, board_row_off: 3,
        }
    }

    pub fn flipped(&self) -> bool {
        if self.cfg.flip_board { return true; }
        if self.mode == Mode::CPU && self.player_color == Color::Black { return true; }
        if self.mode == Mode::PvP && self.cfg.auto_flip && self.gs.turn == Color::Black { return true; }
        false
    }
    pub fn to_board(&self, vis: (usize, usize)) -> (usize, usize) {
        if self.flipped() { (7 - vis.0, 7 - vis.1) } else { vis }
    }

    // ── Start game ────────────────────────────────────────────────────────────
    pub fn start_game(&mut self) {
        self.gs = Gs::new(self.cfg.time_control);
        self.cursor = if self.flipped() { (6, 3) } else { (6, 4) };
        self.selected = None; self.targets = vec![];
        self.input_buf.clear(); self.input_err = None;
        self.thinking = false; self.ai_rx = None;
        self.undo_stack.clear();
        self.draw_offer = None; self.draw_offer_msg = None;
        self.pgn_saved_path = None;
        self.png_export_path = None; self.png_notice = None; self.png_preview_path = None;
        self.replay_snaps.clear(); self.game_end = GameEnd::Normal;
        self.last_tick = Instant::now();
        self.engine_busy = false;
        self.move_reviews.clear();
        self.last_eval_cp = 0;
        self.eval_history = vec![0];
        self.replay_snaps.push(ReplaySnap {
            board: self.gs.board, turn: Color::White,
            notation: String::new(), mv_num: 0,
        });
        self.screen = Screen::Game;
        if self.mode == Mode::CPU && self.player_color == Color::Black { self.kick_ai(); }
    }

    // ── Built-in CPU AI ───────────────────────────────────────────────────────
    pub fn kick_ai(&mut self) {
        if self.thinking { return; }
        let (board, color, ep, cast, depth) = (
            self.gs.board, self.gs.turn, self.gs.ep, self.gs.cast,
            self.cfg.ai_depth.depth(),
        );
        let history = self.gs.history.clone();
        let (tx, rx) = mpsc::channel();
        self.thinking = true; self.ai_rx = Some(rx);
        self.gs.clock_state = ClockState::Paused;
        thread::spawn(move || {
            tx.send(best_mv(&board, color, ep, &cast, depth, &history)).ok();
        });
    }

    // ── Main 50 ms tick ───────────────────────────────────────────────────────
    pub fn poll_ai(&mut self) {
        self.tick_clock();
        self.poll_minimax();
        self.poll_analysis();
        if let Some(n) = self.saved_notice { self.saved_notice = if n == 0 { None } else { Some(n - 1) }; }
        if let Some(n) = self.png_notice   { self.png_notice   = if n == 0 { None } else { Some(n - 1) }; }
    }

    fn poll_minimax(&mut self) {
        if !self.thinking { return; }
        if let Some(rx) = &self.ai_rx {
            match rx.try_recv() {
                Ok(mv) => {
                    self.thinking = false; self.ai_rx = None;
                    if let Some(mv) = mv { self.execute(mv); }
                }
                Err(TryRecvError::Disconnected) => { self.thinking = false; self.ai_rx = None; }
                Err(TryRecvError::Empty) => {}
            }
        }
    }

    fn poll_analysis(&mut self) {
        if !self.engine_busy { return; }
        let result = match self.analysis.try_recv() {
            Some(r) => r,
            None    => return,
        };
        self.engine_busy = false;

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
            MoveLabel::from_delta_cp(delta_cp)
        };

        let san = self.gs.history.get(move_idx)
            .map(|h| h.san.clone())
            .unwrap_or_default();

        rlog!(
            "[rchess/review] move {} ({}) before={} after={} delta={} label={:?} book={}",
            move_idx, san, result.eval_before_cp, result.eval_after_cp, delta_cp, label, is_book
        );

        let review = MoveReview {
            san,
            eval_before:  result.eval_before_cp as f32 / 100.0,
            eval_after:   result.eval_after_cp  as f32 / 100.0,
            delta_cp,
            best_move:    result.best_move_uci,
            top_moves:    result.top_moves,
            label, is_book,
        };

        if move_idx < self.move_reviews.len() {
            self.move_reviews[move_idx] = review;
        } else {
            while self.move_reviews.len() < move_idx {
                self.move_reviews.push(MoveReview {
                    san: String::new(), eval_before: 0.0, eval_after: 0.0,
                    delta_cp: 0, best_move: None, top_moves: vec![], label: MoveLabel::Good, is_book: false,
                });
            }
            self.move_reviews.push(review);
        }
    }

    fn check_is_book_move(&self, move_idx: usize) -> bool {
        if move_idx >= self.gs.history.len() { return false; }
        let hist_before = &self.gs.history[..move_idx];
        let (board_before, ep_before, cast_before) = self.undo_stack.last()
            .map(|(gs, _, _)| (gs.board, gs.ep, gs.cast.clone()))
            .unwrap_or_else(|| (start_board(), None, Castle::all()));
        let turn_before = if move_idx % 2 == 0 { Color::White } else { Color::Black };
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

    fn queue_analysis_after_move(&mut self) {
        if self.cfg.ui_mode == UiMode::Minimal { return; }
        if self.engine_busy {
            rlog!("[rchess/analysis] busy — skipping move {}",
                self.gs.history.len().saturating_sub(1));
            return;
        }
        let h = &self.gs.history;
        let move_idx    = h.len() - 1;
        let color_moved = h[move_idx].color;
        let uci_before: String = h[..move_idx].iter()
            .map(|e| e.notation.trim_end_matches(['+', '#']).to_string())
            .collect::<Vec<_>>().join(" ");
        let uci_after: String = h.iter()
            .map(|e| e.notation.trim_end_matches(['+', '#']).to_string())
            .collect::<Vec<_>>().join(" ");
        rlog!("[rchess/analysis] queuing move {} ({:?})", move_idx, color_moved);
        self.analysis.analyse(move_idx, &uci_before, &uci_after, color_moved, self.cfg.analysis_depth);
        self.engine_busy = true;
    }

    // ── Clock ─────────────────────────────────────────────────────────────────
    fn tick_clock(&mut self) {
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_tick).as_millis() as u64;
        self.last_tick = now;
        if self.gs.clock_state != ClockState::Running { return; }
        if matches!(self.gs.status, Status::Checkmate | Status::Stalemate) { return; }
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
            GameEnd::Flag  => format!("{} wins on time", self.gs.turn.opp().name()),
            GameEnd::Draw  => "\u{bd}-\u{bd} Draw".to_string(),
            GameEnd::Normal => match self.gs.status {
                Status::Checkmate => format!("{} wins", self.gs.turn.opp().name()),
                Status::Stalemate => "\u{bd}-\u{bd} Stalemate".to_string(),
                _                 => String::new(),
            }
        };
        self.export_pgn();
        // Auto-save PNG on game end if configured
        if self.cfg.auto_save_png {
            self.auto_png_export();
        }
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
        let white_name = match self.mode {
            Mode::PvP => "Player 1".to_string(),
            Mode::CPU => if self.player_color == Color::White { "Player".to_string() }
                         else { format!("CPU ({})", self.cfg.ai_depth.name().split_whitespace().next().unwrap_or("AI")) },
        };
        let black_name = match self.mode {
            Mode::PvP => "Player 2".to_string(),
            Mode::CPU => if self.player_color == Color::Black { "Player".to_string() }
                         else { format!("CPU ({})", self.cfg.ai_depth.name().split_whitespace().next().unwrap_or("AI")) },
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
            if n % 5 == 0 { move_text.push('\n'); }
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
            Ok(_)  => self.pgn_saved_path = Some(path.to_string_lossy().to_string()),
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
        if self.gs.cast.wk { castle.push('K'); } if self.gs.cast.wq { castle.push('Q'); }
        if self.gs.cast.bk { castle.push('k'); } if self.gs.cast.bq { castle.push('q'); }
        if castle.is_empty() { castle.push('-'); }
        let ep = self.gs.ep.map(|(r, c)| format!("{}{}", (b'a' + c as u8) as char, 8 - r))
            .unwrap_or_else(|| "-".to_string());
        format!("{} {} {} {} 0 {}", fen, turn, castle, ep, self.gs.fullmove)
    }

    // ── Key handlers ──────────────────────────────────────────────────────────
    pub fn handle_menu_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up   | KeyCode::Char('k') => { if self.menu_cur > 0 { self.menu_cur -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if self.menu_cur < 2 { self.menu_cur += 1; } }
            KeyCode::Enter | KeyCode::Char(' ') => match self.menu_cur {
                0 => { self.mode = Mode::PvP; self.player_color = Color::White; self.start_game(); }
                1 => { self.mode = Mode::CPU; self.screen = Screen::ColorPick; }
                2 => self.screen = Screen::Settings,
                _ => {}
            },
            KeyCode::Char('s') => self.screen = Screen::Settings,
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
            KeyCode::Down | KeyCode::Char('j') => { if self.settings_cur < 13 { self.settings_cur += 1; } }
            KeyCode::Left | KeyCode::Char('h') => self.settings_cycle(false),
            KeyCode::Right| KeyCode::Char('l') => self.settings_cycle(true),
            KeyCode::Char('w') | KeyCode::Char('s') => { self.cfg.save(); self.saved_notice = Some(40); }
            KeyCode::Char('r') => self.cfg = Config::default(),
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Menu,
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
            10 => self.cfg.ui_mode         = cyc(UiMode::ALL,         self.cfg.ui_mode,         fwd),
            11 => self.cfg.analysis_engine = cyc(AnalysisEngine::ALL, self.cfg.analysis_engine, fwd),
            13 => self.cfg.auto_save_png   = !self.cfg.auto_save_png,
            12 => self.cfg.analysis_depth = if fwd {
                       (self.cfg.analysis_depth % 3) + 1
                   } else {
                       if self.cfg.analysis_depth <= 1 { 3 } else { self.cfg.analysis_depth - 1 }
                   },
            _  => {}
        }
    }

    pub fn handle_game_key(&mut self, code: KeyCode) {
        let over  = matches!(self.gs.status, Status::Checkmate | Status::Stalemate)
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
            KeyCode::Char('q') => { self.screen = Screen::Menu; self.selected = None; self.targets = vec![]; }
            KeyCode::Char('n') => self.start_game(),
            KeyCode::Char('s') => self.screen = Screen::Settings,
            KeyCode::Char('u') => self.undo(),
            KeyCode::Char('E') => self.open_png_preview(),
            KeyCode::Char('r') => { if !self.replay_snaps.is_empty() { self.open_replay(); } }
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
            KeyCode::Up    | KeyCode::Char('k') => { if self.cursor.0 > 0 { self.cursor.0 -= 1; } }
            KeyCode::Down  | KeyCode::Char('j') => { if self.cursor.0 < 7 { self.cursor.0 += 1; } }
            KeyCode::Left  | KeyCode::Char('h') => { if self.cursor.1 > 0 { self.cursor.1 -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => { if self.cursor.1 < 7 { self.cursor.1 += 1; } }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if !human || over || self.thinking { return; }
                self.do_select(self.to_board(self.cursor));
            }
            KeyCode::Esc => { self.selected = None; self.targets = vec![]; }
            KeyCode::Char(c) => {
                if human && !over && !self.thinking { self.input_buf.push(c); self.input_err = None; }
            }
            _ => {}
        }
    }

    pub fn handle_draw_offer_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                self.gs.status = Status::Stalemate;
                self.game_end  = GameEnd::Draw;
                self.draw_offer = None;
                self.screen    = Screen::Game;
                self.finish_game();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.draw_offer_msg = Some("Draw offer declined.".to_string());
                self.draw_offer = None;
                self.screen    = Screen::Game;
            }
            _ => {}
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

    pub fn handle_replay_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left  | KeyCode::Char('h') => { if self.replay_idx > 0 { self.replay_idx -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.replay_idx + 1 < self.replay_snaps.len() { self.replay_idx += 1; }
            }
            KeyCode::Char('0') | KeyCode::Home => self.replay_idx = 0,
            KeyCode::Char('$') | KeyCode::End  => self.replay_idx = self.replay_snaps.len().saturating_sub(1),
            KeyCode::Char('E')                  => self.open_png_preview(),
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
        if self.screen != Screen::Game { return; }
        let over  = matches!(self.gs.status, Status::Checkmate | Status::Stalemate)
                    || self.gs.clock_state == ClockState::Flagged;
        let human = self.mode == Mode::PvP || self.gs.turn == self.player_color;
        if !human || over || self.thinking { return; }
        // These MUST match CELL_W and CELL_H in ui.rs
        const CELL_W: usize = 9; const CELL_H: usize = 5;
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
        let board_row = (row - cell_start_row) / CELL_H;
        let board_col = (col - cell_start_col) / CELL_W;
        if board_row > 7 || board_col > 7 { return; }
        let (r, c) = if self.flipped() { (7 - board_row, 7 - board_col) } else { (board_row, board_col) };
        self.do_select((r, c));
    }

    // ── Replay ────────────────────────────────────────────────────────────────
    pub fn open_replay(&mut self) {
        if !self.replay_snaps.is_empty() {
            self.replay_idx = self.replay_snaps.len() - 1;
            self.screen = Screen::Replay;
        }
    }

    // ── Selection ─────────────────────────────────────────────────────────────
    fn do_select(&mut self, bc: (usize, usize)) {
        let piece = self.gs.board[bc.0][bc.1];
        if let Some(sel) = self.selected {
            if self.targets.iter().any(|m| m.fr == sel && m.to == bc) {
                let has_promo = self.targets.iter().any(|m| m.fr == sel && m.to == bc && m.promo.is_some());
                if has_promo {
                    self.promo_from = sel; self.promo_to = bc; self.promo_cur = 0;
                    self.screen = Screen::Promo; return;
                }
                if let Some(&mv) = self.targets.iter().find(|m| m.fr == sel && m.to == bc && m.promo.is_none()) {
                    self.selected = None; self.targets = vec![];
                    self.execute(mv);
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
        let pops = match self.mode {
            Mode::PvP => 1,
            Mode::CPU => {
                use crate::config::AiDepth;
                if self.cfg.ai_depth == AiDepth::Easy { 1 } else { 2 }
            }
        };
        let take = pops.min(self.undo_stack.len());
        if take == 0 { return; }
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
        }
    }

    // ── Execute a move ────────────────────────────────────────────────────────
    pub fn execute(&mut self, mv: Mv) {
        self.undo_stack.push((self.gs.clone(), self.cursor.0, self.cursor.1));
        if self.undo_stack.len() > 200 { self.undo_stack.remove(0); }

        let piece = self.gs.board[mv.fr.0][mv.fr.1].unwrap();
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
        let san_full   = format!("{}{}", san, sfx);

        self.gs.history.push(HistEntry {
            notation: coord_note, san: san_full,
            color: piece.c, from: mv.fr, to: mv.to,
        });
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
            && !matches!(new_s, Status::Checkmate | Status::Stalemate)
            && self.gs.white_ms.is_some()
        {
            self.gs.clock_state = ClockState::Running;
        }

        if self.cfg.auto_flip && self.mode == Mode::PvP {
            self.cursor = if self.gs.turn == Color::Black { (6, 3) } else { (6, 4) };
        }

        // Queue background analysis (non-blocking)
        self.queue_analysis_after_move();

        if matches!(new_s, Status::Active | Status::Check) {
            if self.mode == Mode::CPU && self.gs.turn != self.player_color {
                self.kick_ai();
            }
        } else {
            self.gs.clock_state = ClockState::Paused;
            self.game_end = GameEnd::Normal;
            self.finish_game();
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

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
    let to_r = *db.get(db.len() - 1)?;
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
