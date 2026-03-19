// src/app.rs
use std::sync::mpsc::{self,Receiver,TryRecvError};
use std::thread;
use crossterm::event::KeyCode;
use crate::engine::*;
use crate::ai::best_mv;
use crate::config::Config;

#[derive(Clone,Copy,PartialEq,Eq)]
pub enum Screen { Menu, ColorPick, Settings, Game, Promo }
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum Mode   { PvP, CPU }

#[derive(Clone)]
pub struct HistEntry {
    pub notation: String,
    pub color:    Color,
    pub from:     (usize, usize),
    pub to:       (usize, usize),
}

pub struct Gs {
    pub board: Board, pub turn: Color, pub ep: Option<(usize,usize)>,
    pub cast: Castle, pub status: Status, pub history: Vec<HistEntry>,
    pub cap_w: Vec<Piece>, pub cap_b: Vec<Piece>, pub fullmove: u32,
}
impl Gs {
    pub fn new() -> Self {
        Self { board: start_board(), turn: Color::White, ep: None,
               cast: Castle::all(), status: Status::Active,
               history: vec![], cap_w: vec![], cap_b: vec![], fullmove: 1 }
    }
}

// Settings cursor: which row / which value within that row
pub struct SettingsCursor { pub row: usize }

pub struct App {
    pub screen:        Screen,
    pub mode:          Mode,
    pub player_color:  Color,
    pub gs:            Gs,
    pub cfg:           Config,
    pub cursor:        (usize,usize),
    pub selected:      Option<(usize,usize)>,
    pub pending_to:    Option<(usize,usize)>,   // for confirm_move second Enter
    pub targets:       Vec<Mv>,
    pub promo_from:    (usize,usize),
    pub promo_to:      (usize,usize),
    pub promo_cur:     usize,
    pub menu_cur:      usize,
    pub color_cur:     usize,
    pub settings_cur:  usize,   // active settings row
    pub thinking:      bool,
    pub ai_rx:         Option<Receiver<Option<Mv>>>,
    pub should_quit:   bool,
    pub saved_notice:  Option<u8>, // countdown frames to show "Saved!"
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Menu, mode: Mode::PvP,
            player_color: Color::White, gs: Gs::new(),
            cfg: Config::load(),
            cursor: (6,4), selected: None, pending_to: None, targets: vec![],
            promo_from: (0,0), promo_to: (0,0), promo_cur: 0,
            menu_cur: 0, color_cur: 0, settings_cur: 0,
            thinking: false, ai_rx: None, should_quit: false, saved_notice: None,
        }
    }

    pub fn flipped(&self) -> bool {
        self.cfg.flip_board || (self.mode == Mode::CPU && self.player_color == Color::Black)
    }
    pub fn to_board(&self, vis: (usize,usize)) -> (usize,usize) {
        if self.flipped() { (7-vis.0, 7-vis.1) } else { vis }
    }

    pub fn start_game(&mut self) {
        self.gs = Gs::new();
        self.cursor = if self.flipped() { (6,3) } else { (6,4) };
        self.selected = None; self.pending_to = None; self.targets = vec![];
        self.thinking = false; self.ai_rx = None; self.screen = Screen::Game;
        if self.mode == Mode::CPU && self.player_color == Color::Black { self.kick_ai(); }
    }

    pub fn kick_ai(&mut self) {
        if self.thinking { return; }
        let board = self.gs.board; let color = self.gs.turn;
        let ep = self.gs.ep; let cast = self.gs.cast;
        let depth = self.cfg.ai_depth.depth();
        let (tx, rx) = mpsc::channel();
        self.thinking = true; self.ai_rx = Some(rx);
        thread::spawn(move || { let mv = best_mv(&board, color, ep, &cast, depth); tx.send(mv).ok(); });
    }

    pub fn poll_ai(&mut self) {
        if !self.thinking { return; }
        if let Some(rx) = &self.ai_rx {
            match rx.try_recv() {
                Ok(mv_opt) => {
                    self.thinking = false; self.ai_rx = None;
                    if let Some(mv) = mv_opt { self.execute(mv); }
                }
                Err(TryRecvError::Disconnected) => { self.thinking = false; self.ai_rx = None; }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(n) = self.saved_notice { self.saved_notice = if n == 0 { None } else { Some(n-1) }; }
    }

    // ── keyboard handlers ────────────────────────────────────────────────────

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
            KeyCode::Left  | KeyCode::Char('h') | KeyCode::Up   | KeyCode::Char('k') => { self.color_cur = 0; }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Down | KeyCode::Char('j') => { self.color_cur = 1; }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.player_color = if self.color_cur == 0 { Color::White } else { Color::Black };
                self.start_game();
            }
            KeyCode::Char('w') => { self.player_color = Color::White;  self.start_game(); }
            KeyCode::Char('b') => { self.player_color = Color::Black; self.start_game(); }
            KeyCode::Esc | KeyCode::Char('q') => { self.screen = Screen::Menu; }
            _ => {}
        }
    }

    // Settings: 8 rows, ←/→ changes value, ↑/↓ moves row, Enter/W saves
    pub fn handle_settings_key(&mut self, code: KeyCode) {
        const ROWS: usize = 8;
        match code {
            KeyCode::Up   | KeyCode::Char('k') => { if self.settings_cur > 0 { self.settings_cur -= 1; } }
            KeyCode::Down | KeyCode::Char('j') => { if self.settings_cur < ROWS-1 { self.settings_cur += 1; } }
            KeyCode::Left | KeyCode::Char('h') => self.settings_cycle(false),
            KeyCode::Right| KeyCode::Char('l') => self.settings_cycle(true),
            KeyCode::Char('w') | KeyCode::Char('s') => {
                self.cfg.save();
                self.saved_notice = Some(40); // ~2 s at 50 ms ticks
            }
            KeyCode::Char('r') => { self.cfg = Config::default(); }
            KeyCode::Esc | KeyCode::Char('q') => { self.screen = Screen::Menu; }
            _ => {}
        }
    }

    fn settings_cycle(&mut self, forward: bool) {
        use crate::config::*;
        fn cyc<T: Copy>(arr: &[T], cur: T, fwd: bool) -> T
        where T: PartialEq
        {
            let pos = arr.iter().position(|x| x == &cur).unwrap_or(0);
            let n = arr.len();
            if fwd { arr[(pos+1)%n] } else { arr[(pos+n-1)%n] }
        }
        match self.settings_cur {
            0 => self.cfg.theme       = cyc(Theme::ALL,      self.cfg.theme,       forward),
            1 => self.cfg.piece_style = cyc(PieceStyle::ALL, self.cfg.piece_style, forward),
            2 => self.cfg.ai_depth    = cyc(AiDepth::ALL,    self.cfg.ai_depth,    forward),
            3 => self.cfg.move_hints  = cyc(MoveHints::ALL,  self.cfg.move_hints,  forward),
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
        match code {
            KeyCode::Char('q') => { self.screen = Screen::Menu; self.selected = None; self.targets = vec![]; self.pending_to = None; }
            KeyCode::Char('n') => { self.start_game(); }
            KeyCode::Char('s') => { self.screen = Screen::Settings; }
            KeyCode::Up    | KeyCode::Char('k') => { if self.cursor.0 > 0 { self.cursor.0 -= 1; } }
            KeyCode::Down  | KeyCode::Char('j') => { if self.cursor.0 < 7 { self.cursor.0 += 1; } }
            KeyCode::Left  | KeyCode::Char('h') => { if self.cursor.1 > 0 { self.cursor.1 -= 1; } }
            KeyCode::Right | KeyCode::Char('l') => { if self.cursor.1 < 7 { self.cursor.1 += 1; } }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if !human || over || self.thinking { return; }
                let bc = self.to_board(self.cursor);
                // confirm_move: first Enter sets pending, second Enter executes
                if self.cfg.confirm_move {
                    if let Some(pending) = self.pending_to {
                        if pending == bc {
                            if let Some(sel) = self.selected {
                                if let Some(mv) = self.targets.iter().find(|m| m.fr==sel && m.to==bc && m.promo.is_none()) {
                                    let mv = *mv;
                                    self.pending_to = None; self.selected = None; self.targets = vec![];
                                    self.execute(mv); return;
                                }
                                let has_promo = self.targets.iter().any(|m| m.fr==sel && m.to==bc && m.promo.is_some());
                                if has_promo {
                                    self.promo_from = sel; self.promo_to = bc; self.promo_cur = 0;
                                    self.screen = Screen::Promo; self.pending_to = None; return;
                                }
                            }
                        }
                        self.pending_to = None;
                    } else {
                        self.do_select(bc);
                        if self.targets.iter().any(|m| self.selected.map(|s|m.fr==s).unwrap_or(false) && m.to==bc) {
                            self.pending_to = Some(bc);
                        }
                    }
                } else {
                    self.do_select(bc);
                }
            }
            KeyCode::Esc => { self.selected = None; self.targets = vec![]; self.pending_to = None; }
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

    fn do_select(&mut self, bc: (usize,usize)) {
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
            if piece.map(|p| p.c==self.gs.turn).unwrap_or(false) {
                let all = legal(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast);
                self.selected = Some(bc); self.targets = all.into_iter().filter(|m| m.fr==bc).collect();
            } else { self.selected = None; self.targets = vec![]; }
        } else if piece.map(|p| p.c==self.gs.turn).unwrap_or(false) {
            let all = legal(&self.gs.board, self.gs.turn, self.gs.ep, &self.gs.cast);
            self.selected = Some(bc); self.targets = all.into_iter().filter(|m| m.fr==bc).collect();
        }
    }

    pub fn execute(&mut self, mv: Mv) {
        let piece = self.gs.board[mv.fr.0][mv.fr.1].unwrap();
        let cap   = self.gs.board[mv.to.0][mv.to.1];
        let cap   = if cap.is_none() && mv.ep {
            let cr = if piece.c==Color::White { mv.to.0+1 } else { mv.to.0.wrapping_sub(1) };
            self.gs.board[cr][mv.to.1]
        } else { cap };
        let (nb, ne, nc) = apply(&self.gs.board, &mv, self.gs.ep, &self.gs.cast);
        let next  = piece.c.opp();
        let new_s = game_status(&nb, next, ne, &nc);
        if let Some(c) = cap {
            if piece.c==Color::White { self.gs.cap_w.push(c); } else { self.gs.cap_b.push(c); }
        }
        let f = (b'a'+mv.fr.1 as u8) as char; let fr = 8-mv.fr.0;
        let t = (b'a'+mv.to.1 as u8) as char; let tr = 8-mv.to.0;
        let ps = mv.promo.map(|k| match k { Kind::Q=>"Q",Kind::R=>"R",Kind::B=>"B",_=>"N" }).unwrap_or("");
        let sfx = match new_s { Status::Checkmate=>"#", Status::Check=>"+", _=>"" };
        self.gs.history.push(HistEntry { notation: format!("{}{}{}{}{}{}", f,fr,t,tr,ps,sfx), color: piece.c, from: mv.fr, to: mv.to });
        self.gs.board = nb; self.gs.turn = next; self.gs.ep = ne;
        self.gs.cast  = nc; self.gs.status = new_s;
        if next == Color::White { self.gs.fullmove += 1; }
        if matches!(new_s, Status::Active | Status::Check) {
            if self.mode==Mode::CPU && self.gs.turn!=self.player_color { self.kick_ai(); }
        }
    }
}
