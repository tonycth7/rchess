// src/app.rs
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use crossterm::event::KeyCode;
use crate::engine::*;
use crate::ai::best_mv;
use crate::config::Config;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen { Menu, ColorPick, Settings, Game, Promo }
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode   { PvP, CPU }

#[derive(Clone)]
pub struct HistEntry {
    pub notation: String,
    pub color:    Color,
    pub from:     (usize, usize),
    pub to:       (usize, usize),
}

pub struct Gs {
    pub board:    Board,
    pub turn:     Color,
    pub ep:       Option<(usize, usize)>,
    pub cast:     Castle,
    pub status:   Status,
    pub history:  Vec<HistEntry>,
    pub cap_w:    Vec<Piece>,
    pub cap_b:    Vec<Piece>,
    pub fullmove: u32,
}
impl Gs {
    pub fn new() -> Self {
        Self {
            board: start_board(), turn: Color::White, ep: None,
            cast: Castle::all(), status: Status::Active,
            history: vec![], cap_w: vec![], cap_b: vec![], fullmove: 1,
        }
    }
}

pub struct App {
    pub screen:       Screen,
    pub mode:         Mode,
    pub player_color: Color,
    pub gs:           Gs,
    pub cfg:          Config,
    // Board cursor
    pub cursor:       (usize, usize),
    pub selected:     Option<(usize, usize)>,
    pub targets:      Vec<Mv>,
    // Notation input
    pub input_buf:    String,
    pub input_err:    Option<String>,
    // Promotion
    pub promo_from:   (usize, usize),
    pub promo_to:     (usize, usize),
    pub promo_cur:    usize,
    // Menu/settings state
    pub menu_cur:     usize,
    pub color_cur:    usize,
    pub settings_cur: usize,
    // Misc
    pub thinking:     bool,
    pub ai_rx:        Option<Receiver<Option<Mv>>>,
    pub should_quit:  bool,
    pub saved_notice: Option<u8>,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Menu, mode: Mode::PvP,
            player_color: Color::White, gs: Gs::new(),
            cfg: Config::load(),
            cursor: (6, 4), selected: None, targets: vec![],
            input_buf: String::new(), input_err: None,
            promo_from: (0,0), promo_to: (0,0), promo_cur: 0,
            menu_cur: 0, color_cur: 0, settings_cur: 0,
            thinking: false, ai_rx: None, should_quit: false, saved_notice: None,
        }
    }

    pub fn flipped(&self) -> bool {
        self.cfg.flip_board || (self.mode == Mode::CPU && self.player_color == Color::Black)
    }
    pub fn to_board(&self, vis: (usize, usize)) -> (usize, usize) {
        if self.flipped() { (7 - vis.0, 7 - vis.1) } else { vis }
    }

    pub fn start_game(&mut self) {
        self.gs = Gs::new();
        self.cursor = if self.flipped() { (6, 3) } else { (6, 4) };
        self.selected = None; self.targets = vec![];
        self.input_buf.clear(); self.input_err = None;
        self.thinking = false; self.ai_rx = None;
        self.screen = Screen::Game;
        if self.mode == Mode::CPU && self.player_color == Color::Black { self.kick_ai(); }
    }

    pub fn kick_ai(&mut self) {
        if self.thinking { return; }
        let (board, color, ep, cast, depth) = (self.gs.board, self.gs.turn, self.gs.ep, self.gs.cast, self.cfg.ai_depth.depth());
        let (tx, rx) = mpsc::channel();
        self.thinking = true; self.ai_rx = Some(rx);
        thread::spawn(move || { tx.send(best_mv(&board, color, ep, &cast, depth)).ok(); });
    }

    pub fn poll_ai(&mut self) {
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
        if let Some(n) = self.saved_notice {
            self.saved_notice = if n == 0 { None } else { Some(n - 1) };
        }
    }

    // ── Keyboard handlers ─────────────────────────────────────────────────────

    pub fn handle_menu_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up   | KeyCode::Char('k') => { if self.menu_cur > 0 { self.menu_cur -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if self.menu_cur < 2 { self.menu_cur += 1; } }
            KeyCode::Enter | KeyCode::Char(' ') => match self.menu_cur {
                0 => { self.mode = Mode::PvP; self.player_color = Color::White; self.start_game(); }
                1 => { self.mode = Mode::CPU; self.screen = Screen::ColorPick; }
                2 => { self.screen = Screen::Settings; }
                _ => {}
            },
            KeyCode::Char('s') => { self.screen = Screen::Settings; }
            KeyCode::Char('q') | KeyCode::Esc => { self.should_quit = true; }
            _ => {}
        }
    }

    pub fn handle_color_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left  | KeyCode::Char('h') | KeyCode::Up   | KeyCode::Char('k') => self.color_cur = 0,
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Down | KeyCode::Char('j') => self.color_cur = 1,
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.player_color = if self.color_cur == 0 { Color::White } else { Color::Black };
                self.start_game();
            }
            KeyCode::Char('w') => { self.player_color = Color::White;  self.start_game(); }
            KeyCode::Char('b') => { self.player_color = Color::Black; self.start_game(); }
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Menu,
            _ => {}
        }
    }

    pub fn handle_settings_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up    | KeyCode::Char('k') => { if self.settings_cur > 0 { self.settings_cur -= 1; } }
            KeyCode::Down  | KeyCode::Char('j') => { if self.settings_cur < 7 { self.settings_cur += 1; } }
            KeyCode::Left  | KeyCode::Char('h') => self.settings_cycle(false),
            KeyCode::Right | KeyCode::Char('l') => self.settings_cycle(true),
            KeyCode::Char('w') | KeyCode::Char('s') => { self.cfg.save(); self.saved_notice = Some(40); }
            KeyCode::Char('r') => { self.cfg = Config::default(); }
            KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Menu,
            _ => {}
        }
    }

    fn settings_cycle(&mut self, fwd: bool) {
        use crate::config::*;
        fn cyc<T: Copy + PartialEq>(arr: &[T], cur: T, fwd: bool) -> T {
            let i = arr.iter().position(|x| x == &cur).unwrap_or(0);
            if fwd { arr[(i+1)%arr.len()] } else { arr[(i+arr.len()-1)%arr.len()] }
        }
        match self.settings_cur {
            0 => self.cfg.theme       = cyc(Theme::ALL,      self.cfg.theme,       fwd),
            1 => self.cfg.piece_style = cyc(PieceStyle::ALL, self.cfg.piece_style, fwd),
            2 => self.cfg.ai_depth    = cyc(AiDepth::ALL,    self.cfg.ai_depth,    fwd),
            3 => self.cfg.move_hints  = cyc(MoveHints::ALL,  self.cfg.move_hints,  fwd),
            4 => self.cfg.show_coords  = !self.cfg.show_coords,
            5 => self.cfg.show_clock   = !self.cfg.show_clock,
            6 => self.cfg.flip_board   = !self.cfg.flip_board,
            7 => self.cfg.confirm_move = !self.cfg.confirm_move,
            _ => {}
        }
    }

    pub fn handle_game_key(&mut self, code: KeyCode) {
        let over  = matches!(self.gs.status, Status::Checkmate | Status::Stalemate);
        let human = self.mode == Mode::PvP || self.gs.turn == self.player_color;

        // ── When notation is being typed, most keys feed the input box ──────
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
                            Err(e) => { self.input_err = Some(e); }
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.input_buf.pop();
                    if self.input_buf.is_empty() { self.input_err = None; }
                }
                KeyCode::Esc => { self.input_buf.clear(); self.input_err = None; }
                // Arrow keys still move the cursor while typing
                KeyCode::Up    => { if self.cursor.0 > 0 { self.cursor.0 -= 1; } }
                KeyCode::Down  => { if self.cursor.0 < 7 { self.cursor.0 += 1; } }
                KeyCode::Left  => { if self.cursor.1 > 0 { self.cursor.1 -= 1; } }
                KeyCode::Right => { if self.cursor.1 < 7 { self.cursor.1 += 1; } }
                KeyCode::Char(c) => { self.input_buf.push(c); self.input_err = None; }
                _ => {}
            }
            return;
        }

        // ── Normal cursor / command mode ─────────────────────────────────────
        match code {
            // Commands (only when input_buf empty)
            KeyCode::Char('q') => {
                self.screen = Screen::Menu;
                self.selected = None; self.targets = vec![];
            }
            KeyCode::Char('n') => self.start_game(),
            KeyCode::Char('s') => self.screen = Screen::Settings,

            // Cursor movement — arrow keys primary, hjkl also work
            KeyCode::Up    | KeyCode::Char('k') => { if self.cursor.0 > 0 { self.cursor.0 -= 1; } }
            KeyCode::Down  | KeyCode::Char('j') => { if self.cursor.0 < 7 { self.cursor.0 += 1; } }
            KeyCode::Left  | KeyCode::Char('h') => { if self.cursor.1 > 0 { self.cursor.1 -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => { if self.cursor.1 < 7 { self.cursor.1 += 1; } }

            // Select / confirm with Enter or Space
            KeyCode::Enter | KeyCode::Char(' ') => {
                if !human || over || self.thinking { return; }
                self.do_select(self.to_board(self.cursor));
            }

            // Cancel selection
            KeyCode::Esc => { self.selected = None; self.targets = vec![]; }

            // Any other printable char → start typing a move
            KeyCode::Char(c) => {
                if human && !over && !self.thinking {
                    self.input_buf.push(c);
                    self.input_err = None;
                }
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
                if let Some(&mv) = all.iter().find(|m| m.fr==from && m.to==to && m.promo==Some(kind)) {
                    self.screen = Screen::Game; self.selected = None; self.targets = vec![];
                    self.execute(mv);
                }
            }
            KeyCode::Esc => { self.screen = Screen::Game; self.selected = None; self.targets = vec![]; }
            _ => {}
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn do_select(&mut self, bc: (usize, usize)) {
        let piece = self.gs.board[bc.0][bc.1];
        if let Some(sel) = self.selected {
            if self.targets.iter().any(|m| m.fr==sel && m.to==bc) {
                let has_promo = self.targets.iter().any(|m| m.fr==sel && m.to==bc && m.promo.is_some());
                if has_promo {
                    self.promo_from = sel; self.promo_to = bc; self.promo_cur = 0;
                    self.screen = Screen::Promo; return;
                }
                if let Some(&mv) = self.targets.iter().find(|m| m.fr==sel && m.to==bc && m.promo.is_none()) {
                    self.selected = None; self.targets = vec![]; self.execute(mv);
                }
                return;
            }
            // Re-select a different piece
            if piece.map(|p| p.c == self.gs.turn).unwrap_or(false) {
                let all = legal(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast);
                self.selected = Some(bc); self.targets = all.into_iter().filter(|m| m.fr==bc).collect();
            } else {
                self.selected = None; self.targets = vec![];
            }
        } else if piece.map(|p| p.c == self.gs.turn).unwrap_or(false) {
            let all = legal(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast);
            self.selected = Some(bc); self.targets = all.into_iter().filter(|m| m.fr==bc).collect();
        }
    }

    pub fn execute(&mut self, mv: Mv) {
        let piece = self.gs.board[mv.fr.0][mv.fr.1].unwrap();
        let cap   = self.gs.board[mv.to.0][mv.to.1];
        let cap   = if cap.is_none() && mv.ep {
            let cr = if piece.c == Color::White { mv.to.0 + 1 } else { mv.to.0.wrapping_sub(1) };
            self.gs.board[cr][mv.to.1]
        } else { cap };

        let (nb, ne, nc) = apply(&self.gs.board, &mv, self.gs.ep, &self.gs.cast);
        let next  = piece.c.opp();
        let new_s = game_status(&nb, next, ne, &nc);

        if let Some(c) = cap {
            if piece.c == Color::White { self.gs.cap_w.push(c); } else { self.gs.cap_b.push(c); }
        }

        let f  = (b'a' + mv.fr.1 as u8) as char; let fr = 8 - mv.fr.0;
        let t  = (b'a' + mv.to.1 as u8) as char; let tr = 8 - mv.to.0;
        let ps = mv.promo.map(|k| match k { Kind::Q=>"Q",Kind::R=>"R",Kind::B=>"B",_=>"N" }).unwrap_or("");
        let sfx = match new_s { Status::Checkmate => "#", Status::Check => "+", _ => "" };

        self.gs.history.push(HistEntry {
            notation: format!("{}{}{}{}{}{}", f, fr, t, tr, ps, sfx),
            color: piece.c, from: mv.fr, to: mv.to,
        });
        self.gs.board = nb; self.gs.turn = next;
        self.gs.ep = ne; self.gs.cast = nc; self.gs.status = new_s;
        if next == Color::White { self.gs.fullmove += 1; }

        if matches!(new_s, Status::Active | Status::Check) {
            if self.mode == Mode::CPU && self.gs.turn != self.player_color { self.kick_ai(); }
        }
    }
}

// ── Notation parser ───────────────────────────────────────────────────────────

pub fn parse_notation(
    board: &Board, color: Color,
    ep: Option<(usize,usize)>, cast: &Castle,
    input: &str,
) -> Result<Mv, String> {
    let moves = legal(board, color, ep, cast);
    if moves.is_empty() { return Err("No legal moves".into()); }

    let raw = input.trim();
    let s   = raw.trim_end_matches('+').trim_end_matches('#').trim();

    // ── Castling ──────────────────────────────────────────────────────────────
    match s.to_lowercase().as_str() {
        "o-o-o" | "0-0-0" =>
            return moves.iter().find(|m| m.castle==2).copied().ok_or_else(|| "Queenside castle not available".into()),
        "o-o" | "0-0" =>
            return moves.iter().find(|m| m.castle==1).copied().ok_or_else(|| "Kingside castle not available".into()),
        _ => {}
    }

    // ── Coordinate notation: e2e4 / e2 e4 / e2-e4 ────────────────────────────
    let coord: String = s.chars().filter(|c| c.is_alphanumeric()).collect();
    let cb = coord.as_bytes();
    if cb.len() >= 4
        && (b'a'..=b'h').contains(&cb[0])
        && (b'1'..=b'8').contains(&cb[1])
        && (b'a'..=b'h').contains(&cb[2])
        && (b'1'..=b'8').contains(&cb[3])
    {
        let from = ((8 - (cb[1]-b'0')) as usize, (cb[0]-b'a') as usize);
        let to   = ((8 - (cb[3]-b'0')) as usize, (cb[2]-b'a') as usize);
        let promo: Option<Kind> = cb.get(4).and_then(|&p| match p.to_ascii_lowercase() {
            b'q'=>Some(Kind::Q), b'r'=>Some(Kind::R), b'b'=>Some(Kind::B), b'n'=>Some(Kind::N), _=>None
        });
        let mv = moves.iter().find(|m| {
            m.fr == from && m.to == to &&
            match promo { Some(p) => m.promo == Some(p), None => m.promo.is_none() || m.promo == Some(Kind::Q) }
        });
        if let Some(mv) = mv {
            // auto-promote to queen when no promo specified but it's a promotion move
            let final_mv = if promo.is_none() && mv.promo.is_some() {
                moves.iter().find(|m| m.fr==from && m.to==to && m.promo==Some(Kind::Q)).copied().unwrap_or(*mv)
            } else { *mv };
            return Ok(final_mv);
        }
        let from_name = format!("{}{}", (b'a'+from.1 as u8) as char, 8-from.0);
        let to_name   = format!("{}{}", (b'a'+to.1   as u8) as char, 8-to.0  );
        let hint = if board[from.0][from.1].is_none() { " (empty square)" }
                   else if board[from.0][from.1].map(|p| p.c != color).unwrap_or(false) { " (opponent's piece)" }
                   else { "" };
        return Err(format!("{} → {} is not legal{}", from_name, to_name, hint));
    }

    // ── SAN: Nf3, e4, Qxd5, Bxc4, exd5, h4, O-O … ──────────────────────────
    san_parse(board, color, &moves, s)
        .ok_or_else(|| format!("'{}' — try: e2e4  Nf3  Qg4  O-O  O-O-O", raw))
}

fn san_parse(board: &Board, color: Color, moves: &[Mv], s: &str) -> Option<Mv> {
    let raw_b = s.as_bytes();
    if raw_b.is_empty() { return None; }

    // Determine piece kind from first char (uppercase = piece, lowercase = pawn)
    // Special: lowercase 'n','r','q','k' accepted for user convenience
    let (piece_kind, skip) = match raw_b[0] {
        b'N' | b'n' => (Kind::N, 1),
        b'B'         => (Kind::B, 1),   // uppercase B = bishop
        b'R' | b'r' => (Kind::R, 1),
        b'Q' | b'q' => (Kind::Q, 1),
        b'K'         => (Kind::K, 1),
        // lowercase 'k' could be king OR part of pawn notation — unlikely pawn starts with k
        b'k'         => (Kind::K, 1),
        // lowercase 'b' followed by digit = pawn (b4), followed by file = bishop (bc4)
        b'b' => {
            let next = raw_b.get(1).copied().unwrap_or(0);
            if (b'1'..=b'8').contains(&next) { (Kind::P, 0) } else { (Kind::B, 1) }
        }
        _ => (Kind::P, 0),
    };

    // Strip capture/check symbols and build working string (lowercase)
    let rest: String = s[skip..].chars()
        .filter(|&c| c != 'x' && c != 'X' && c != '+' && c != '#')
        .collect::<String>()
        .to_lowercase();

    // Handle promotion: e8=Q or e8Q
    let (dest_raw, promo) = if let Some(eq) = rest.find('=') {
        let p = rest.as_bytes().get(eq+1).and_then(|&c| match c {
            b'q'=>Some(Kind::Q), b'r'=>Some(Kind::R), b'b'=>Some(Kind::B), b'n'=>Some(Kind::N), _=>None
        });
        (rest[..eq].to_string(), p)
    } else { (rest.clone(), None) };

    let db = dest_raw.as_bytes();
    if db.len() < 2 { return None; }

    let to_f = *db.get(db.len()-2)?;
    let to_r = *db.get(db.len()-1)?;
    if !(b'a'..=b'h').contains(&to_f) { return None; }
    if !(b'1'..=b'8').contains(&to_r) { return None; }

    let to = ((8-(to_r-b'0')) as usize, (to_f-b'a') as usize);
    let disambig = &dest_raw[..dest_raw.len()-2];

    // Disambiguation: optional file and/or rank before destination
    let dis_f: Option<usize> = disambig.bytes()
        .find(|&b| (b'a'..=b'h').contains(&b))
        .map(|b| (b-b'a') as usize);
    let dis_r: Option<usize> = disambig.bytes()
        .find(|b| b.is_ascii_digit())
        .map(|b| (8-(b-b'0')) as usize);

    let is_promo_sq = to.0 == 0 || to.0 == 7;
    let tgt_promo = promo.or_else(|| if piece_kind==Kind::P && is_promo_sq { Some(Kind::Q) } else { None });

    moves.iter().find(|m| {
        let sq = board[m.fr.0][m.fr.1];
        sq.map(|p| p.c==color && p.k==piece_kind).unwrap_or(false)
            && m.to == to
            && dis_f.map(|f| m.fr.1==f).unwrap_or(true)
            && dis_r.map(|r| m.fr.0==r).unwrap_or(true)
            && match tgt_promo { Some(p)=>m.promo==Some(p), None=>m.promo.is_none() }
    }).copied()
}
