// src/config.rs
use std::fs;
use std::path::PathBuf;

// ── Theme ─────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Classic, Tournament, Mocha, Slate, Midnight, Crimson, MatteBlack, Ocean,
}
impl Theme {
    pub const ALL: &'static [Theme] = &[
        Theme::Classic, Theme::Tournament, Theme::Mocha,
        Theme::Slate,   Theme::Midnight,   Theme::Crimson, Theme::MatteBlack, Theme::Ocean,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Theme::Classic     => "Classic      (cream/brown)",
            Theme::Tournament  => "Tournament   (green/gold)",
            Theme::Mocha       => "Mocha        (brown/cream)",
            Theme::Slate       => "Slate        (grey/cyan)",
            Theme::Midnight    => "Midnight     (navy/purple)",
            Theme::Crimson     => "Crimson      (red/white)",
            Theme::MatteBlack  => "Matte Black  (dark grey/gold)",
            Theme::Ocean       => "Ocean        (navy/cyan)",
        }
    }
    pub fn squares(self) -> ((u8,u8,u8),(u8,u8,u8)) {
        match self {
            Theme::Classic    => ((240,217,181),(181,136, 99)),
            Theme::Tournament => ((100,140,100),( 50, 80, 50)),
            Theme::Mocha      => ((180,140,100),(110, 80, 50)),
            Theme::Slate      => (( 90,110,130),( 50, 65, 80)),
            Theme::Midnight   => (( 60, 60,120),( 30, 30, 70)),
            Theme::Crimson    => ((160, 60, 60),( 90, 30, 30)),
            Theme::MatteBlack => (( 55, 55, 55),( 22, 22, 22)),
            Theme::Ocean      => (( 40, 70,110),( 15, 35, 65)),
        }
    }
    pub fn accent(self) -> (u8,u8,u8) {
        match self {
            Theme::Classic    => (160,110, 60),
            Theme::Tournament => (210,175, 60),
            Theme::Mocha      => (200,160, 90),
            Theme::Slate      => ( 80,200,200),
            Theme::Midnight   => (160,100,220),
            Theme::Crimson    => (240,200,200),
            Theme::MatteBlack => (200,160, 30),
            Theme::Ocean      => ( 40,220,220),
        }
    }
    pub fn select(self) -> (u8,u8,u8) {
        match self {
            Theme::Classic    => (205,210, 60),
            Theme::Tournament => (180,160, 40),
            Theme::Mocha      => (200,170, 80),
            Theme::Slate      => ( 60,180,180),
            Theme::Midnight   => (120, 80,200),
            Theme::Crimson    => (220, 80, 80),
            Theme::MatteBlack => (180,140, 20),
            Theme::Ocean      => ( 20,180,200),
        }
    }
    pub fn cursor(self) -> (u8,u8,u8) {
        match self {
            Theme::MatteBlack => (220,180, 30),
            Theme::Ocean      => ( 50,230,230),
            _              => (200,180, 50),
        }
    }
    pub fn last_move(self) -> (u8,u8,u8) {
        match self {
            Theme::MatteBlack => (160,130, 20),
            Theme::Ocean      => ( 30,160,180),
            _ => (160,140, 30),
            _              => self.accent(),
        }
    }
    pub fn piece_colors(self) -> ((u8,u8,u8),(u8,u8,u8)) {
        match self {
            Theme::MatteBlack => ((240,230,200),(190,190,195)),  // warm ivory / bright silver
            Theme::Ocean      => ((240,250,255),( 20, 60,120)),
            Theme::Classic => ((255,248,220),(15,10,5)),  // warm cream white / near-black
            _ => ((240,240,240),(15,15,15)),
            _              => ((240,235,210),(28,18, 8)),
        }
    }
    pub fn bg_colors(self) -> ((u8,u8,u8),(u8,u8,u8)) {
        match self {
            Theme::Classic    => ((36, 22, 12),(52, 34, 18)),
            Theme::Tournament => ((14, 20, 14),(22, 30, 22)),
            Theme::Mocha      => ((30, 18,  8),(44, 28, 14)),
            Theme::Slate      => ((16, 20, 28),(22, 28, 40)),
            Theme::Midnight   => (( 8,  8, 20),(14, 14, 32)),
            Theme::Crimson    => ((28,  8,  8),(42, 14, 14)),
            Theme::MatteBlack => (( 8,  8,  8),(16, 16, 16)),
            Theme::Ocean      => (( 5, 15, 35),(10, 22, 50)),
        }
    }
}

// ── PieceStyle ────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceStyle { Unicode, Letters, FatLetters, Blocks }
impl PieceStyle {
    pub const ALL: &'static [PieceStyle] = &[
        PieceStyle::Unicode, PieceStyle::Letters, PieceStyle::FatLetters, PieceStyle::Blocks,
    ];
    pub fn name(self) -> &'static str {
        match self {
            PieceStyle::Unicode    => "Unicode symbols  \u{2654}\u{2655}\u{2656}\u{2657}\u{2658}\u{2659}",
            PieceStyle::Letters    => "ASCII letters    K Q R B N P",
            PieceStyle::FatLetters => "Bracketed        [K][Q][R]\u{2026}",
            PieceStyle::Blocks      => "Block art        \u{2588}\u{2588}\u{2588} (big pieces)",
        }
    }
    pub fn render(self, sym: &'static str, letter: char, is_white: bool) -> String {
        match self {
            PieceStyle::Unicode => sym.to_string(),
            PieceStyle::Letters => {
                let c = if is_white { letter.to_ascii_uppercase() } else { letter.to_ascii_lowercase() };
                c.to_string()
            }
            PieceStyle::Blocks     => sym.to_string(), // unused — handled in draw_board
            PieceStyle::FatLetters => {
                let c = if is_white { letter.to_ascii_uppercase() } else { letter.to_ascii_lowercase() };
                format!("[{}]", c)
            }
        }
    }
}

// ── AiDepth ───────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AiDepth { Easy=1, Medium=2, Hard=3, Expert=4 }
impl AiDepth {
    pub const ALL: &'static [AiDepth] = &[AiDepth::Easy, AiDepth::Medium, AiDepth::Hard, AiDepth::Expert];
    pub fn name(self) -> &'static str {
        match self {
            AiDepth::Easy   => "Easy    (depth 1)",
            AiDepth::Medium => "Medium  (depth 2)",
            AiDepth::Hard   => "Hard    (depth 3)",
            AiDepth::Expert => "Expert  (depth 4, slow)",
        }
    }
    pub fn depth(self) -> u8 { self as u8 }
}
impl std::str::FromStr for AiDepth {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "1" => AiDepth::Easy, "2" => AiDepth::Medium, "4" => AiDepth::Expert,
            _   => AiDepth::Hard,
        })
    }
}

// ── MoveHints ─────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveHints { Dots, Highlight, None }
impl MoveHints {
    pub const ALL: &'static [MoveHints] = &[MoveHints::Dots, MoveHints::Highlight, MoveHints::None];
    pub fn name(self) -> &'static str {
        match self {
            MoveHints::Dots      => "Dots       (\u{b7} on target squares)",
            MoveHints::Highlight => "Highlight  (full square glow)",
            MoveHints::None      => "Off        (no hints)",
        }
    }
}

// ── TimeControl ───────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimeControl { Infinite, Bullet, Blitz, Rapid, Classical }
impl TimeControl {
    pub const ALL: &'static [TimeControl] = &[
        TimeControl::Infinite, TimeControl::Bullet, TimeControl::Blitz,
        TimeControl::Rapid,    TimeControl::Classical,
    ];
    pub fn name(self) -> &'static str {
        match self {
            TimeControl::Infinite  => "Infinite   (no clock)",
            TimeControl::Bullet    => "Bullet     (1 min each)",
            TimeControl::Blitz     => "Blitz      (3 min each)",
            TimeControl::Rapid     => "Rapid      (10 min each)",
            TimeControl::Classical => "Classical  (30 min each)",
        }
    }
    pub fn initial_ms(self) -> Option<u64> {
        match self {
            TimeControl::Infinite  => None,
            TimeControl::Bullet    => Some(60_000),
            TimeControl::Blitz     => Some(180_000),
            TimeControl::Rapid     => Some(600_000),
            TimeControl::Classical => Some(1_800_000),
        }
    }
    pub fn short_label(self) -> &'static str {
        match self {
            TimeControl::Infinite  => "\u{221e}",
            TimeControl::Bullet    => "1'",
            TimeControl::Blitz     => "3'",
            TimeControl::Rapid     => "10'",
            TimeControl::Classical => "30'",
        }
    }
}

// ── UiMode ────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiMode {
    /// Original clean UI — no engine panel shown.
    Minimal,
    /// Current UI + compact two-line eval bar (default).
    Standard,
    /// Full engine analysis panel with eval arrow, bar, and best move.
    Analysis,
}
impl UiMode {
    pub const ALL: &'static [UiMode] = &[UiMode::Minimal, UiMode::Standard, UiMode::Analysis];
    pub fn name(self) -> &'static str {
        match self {
            UiMode::Minimal  => "Minimal   (no engine info)",
            UiMode::Standard => "Standard  (compact eval)",
            UiMode::Analysis => "Analysis  (full panel)",
        }
    }
}

// ── AnalysisEngine ────────────────────────────────────────────────────────────
/// Which analysis backend to use.
///
/// Arch:    sudo pacman -S stockfish
/// Ubuntu:  sudo apt install stockfish
/// macOS:   brew install stockfish
///
/// If Stockfish is selected but not found, the game falls back to
/// the Built-in engine automatically.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnalysisEngine {
    /// Pure-Rust minimax at depth 2 — always available, no install needed.
    Builtin,
    /// Stockfish via UCI protocol — must be installed separately.
    Stockfish,
}
impl AnalysisEngine {
    pub const ALL: &'static [AnalysisEngine] = &[AnalysisEngine::Builtin, AnalysisEngine::Stockfish];
    pub fn name(self) -> &'static str {
        match self {
            AnalysisEngine::Builtin   => "Built-in  (minimax, always works)",
            AnalysisEngine::Stockfish => "Stockfish (pacman -S stockfish)",
        }
    }
}

// ── Config ────────────────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
pub struct Config {
    pub theme:           Theme,
    pub piece_style:     PieceStyle,
    pub ai_depth:        AiDepth,
    pub move_hints:      MoveHints,
    pub time_control:    TimeControl,
    pub show_coords:     bool,
    pub show_clock:      bool,
    pub flip_board:      bool,
    pub auto_flip:       bool,
    pub confirm_move:    bool,
    pub ui_mode:         UiMode,
    /// Auto-save board PNG when a game ends (default: false)
    pub auto_save_png:   bool,
    /// Optional Lichess API token for puzzle access (leave empty for anonymous)
    pub lichess_token:   String,
    /// Centipawn threshold for Blunder classification (default 300 = 3.0p)
    pub blunder_cp:      i32,
    /// Centipawn threshold for Mistake classification (default 100 = 1.0p)
    pub mistake_cp:      i32,
    /// Centipawn threshold for Inaccuracy classification (default 50 = 0.5p)
    pub inaccuracy_cp:   i32,
    /// Stockfish skill level 0-20 when used as CPU opponent (default 10)
    pub stockfish_skill: u8,
    pub analysis_engine: AnalysisEngine,
    /// Depth for built-in analysis engine (1=fast, 2=balanced, 3=strong)
    pub analysis_depth: u8,
    /// Highlight brightness 0-10 (0=subtle, 5=default, 10=vivid)
    pub highlight_brightness: u8,
    /// Board cell width in chars (default 8). Increase to make board bigger.
    pub cell_w: u8,
    /// Board cell height in lines (default 4). Keep cell_w ≈ cell_h*2 for square cells.
    pub cell_h: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme:           Theme::Classic,
            piece_style:     PieceStyle::Unicode,
            ai_depth:        AiDepth::Hard,
            move_hints:      MoveHints::Dots,
            time_control:    TimeControl::Infinite,
            show_coords:     true,
            show_clock:      true,
            flip_board:      false,
            auto_flip:       false,
            confirm_move:    false,
            ui_mode:         UiMode::Standard,
            auto_save_png:   false,
            lichess_token:   String::new(),
            blunder_cp:      300,
            mistake_cp:      100,
            inaccuracy_cp:    50,
            stockfish_skill:  10,
            analysis_engine: AnalysisEngine::Builtin,
            analysis_depth: 2,
            highlight_brightness: 5,
            cell_w: 8,
            cell_h: 4,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = Config::default();
        if let Some(path) = config_path() {
            if let Ok(text) = fs::read_to_string(&path) {
                for line in text.lines() {
                    let line = line.trim();
                    if line.starts_with('#') || !line.contains('=') { continue; }
                    let mut parts = line.splitn(2, '=');
                    let key = parts.next().unwrap_or("").trim();
                    let val = parts.next().unwrap_or("").trim();
                    match key {
                        "theme" => cfg.theme = match val {
                            "classic"    => Theme::Classic,
                            "mocha"      => Theme::Mocha,
                            "slate"      => Theme::Slate,
                            "midnight"   => Theme::Midnight,
                            "crimson"    => Theme::Crimson,
                            "matteblack" => Theme::MatteBlack,
                            "ocean"      => Theme::Ocean,
                            _            => Theme::Tournament,
                        },
                        "piece_style" => cfg.piece_style = match val {
                            "letters"    => PieceStyle::Letters,
                            "fatletters" => PieceStyle::FatLetters,
                            "blocks"     => PieceStyle::Blocks,
                            _            => PieceStyle::Unicode,
                        },
                        "ai_depth"     => cfg.ai_depth    = val.parse().unwrap_or(AiDepth::Hard),
                        "move_hints"   => cfg.move_hints  = match val {
                            "highlight"  => MoveHints::Highlight,
                            "none"       => MoveHints::None,
                            _            => MoveHints::Dots,
                        },
                        "time_control" => cfg.time_control = match val {
                            "bullet"    => TimeControl::Bullet,
                            "blitz"     => TimeControl::Blitz,
                            "rapid"     => TimeControl::Rapid,
                            "classical" => TimeControl::Classical,
                            _           => TimeControl::Infinite,
                        },
                        "show_coords"     => cfg.show_coords  = val == "true",
                        "show_clock"      => cfg.show_clock   = val == "true",
                        "flip_board"      => cfg.flip_board   = val == "true",
                        "auto_flip"       => cfg.auto_flip    = val == "true",
                        "confirm_move"    => cfg.confirm_move = val == "true",
                        "ui_mode" => cfg.ui_mode = match val {
                            "minimal"  => UiMode::Minimal,
                            "analysis" => UiMode::Analysis,
                            _          => UiMode::Standard,
                        },
                        "auto_save_png"   => cfg.auto_save_png   = val == "true",
                        "lichess_token"   => cfg.lichess_token   = val.to_string(),
                        "blunder_cp"      => cfg.blunder_cp      = val.parse().unwrap_or(300),
                        "mistake_cp"      => cfg.mistake_cp      = val.parse().unwrap_or(100),
                        "inaccuracy_cp"   => cfg.inaccuracy_cp   = val.parse().unwrap_or(50),
                        "stockfish_skill" => cfg.stockfish_skill = val.parse().unwrap_or(10),
                        "analysis_engine" => cfg.analysis_engine = match val {
                            "stockfish" => AnalysisEngine::Stockfish,
                            _           => AnalysisEngine::Builtin,
                        },
                        "analysis_depth" => cfg.analysis_depth = val.parse::<u8>().unwrap_or(2).clamp(1, 3),
                        "highlight_brightness" => cfg.highlight_brightness = val.parse::<u8>().unwrap_or(5).clamp(0, 10),
                        "cell_w" => cfg.cell_w = val.parse::<u8>().unwrap_or(8).clamp(4, 20),
                        "cell_h" => cfg.cell_h = val.parse::<u8>().unwrap_or(4).clamp(2, 10),
                        _ => {}
                    }
                }
            }
        }
        cfg
    }

    pub fn save(&self) {
        if let Some(path) = config_path() {
            let _ = fs::write(path, format!(
                "# RChess TUI v0.7 — edit here or use in-game Settings (s)\n\
                 #\n\
                 # analysis_engine options:\n\
                 #   builtin   — pure Rust minimax, always works, no install needed\n\
                 #   stockfish — install: sudo pacman -S stockfish  (Arch)\n\
                 #               install: sudo apt install stockfish (Ubuntu)\n\
                 #               then set: analysis_engine = stockfish\n\
                 #\n\
                 # ui_mode options: minimal | standard | analysis\n\
                 # Press T in-game to cycle ui_mode quickly.\n\n\
                 theme            = {}\n\
                 piece_style      = {}\n\
                 ai_depth         = {}\n\
                 move_hints       = {}\n\
                 time_control     = {}\n\
                 show_coords      = {}\n\
                 show_clock       = {}\n\
                 flip_board       = {}\n\
                 auto_flip        = {}\n\
                 confirm_move     = {}\n\
                 ui_mode          = {}\n\
                 auto_save_png    = {}\n\
                 lichess_token    = {}\n\
                 blunder_cp       = {}\n\
                 mistake_cp       = {}\n\
                 inaccuracy_cp    = {}\n\
                 stockfish_skill  = {}\n\
                 analysis_engine  = {}\n\
                 analysis_depth   = {}\n\
                 highlight_brightness = {}\n\
                 cell_w           = {}\n\
                 cell_h           = {}\n",
                match self.theme {
                    Theme::Classic=>"classic", Theme::Tournament=>"tournament",
                    Theme::Mocha=>"mocha",    Theme::Slate=>"slate",
                    Theme::Midnight=>"midnight", Theme::Crimson=>"crimson", Theme::MatteBlack=>"matteblack", Theme::Ocean=>"ocean",
                },
                match self.piece_style {
                    PieceStyle::Unicode=>"unicode", PieceStyle::Letters=>"letters",
                    PieceStyle::FatLetters=>"fatletters", PieceStyle::Blocks=>"blocks",
                },
                self.ai_depth.depth(),
                match self.move_hints {
                    MoveHints::Dots=>"dots", MoveHints::Highlight=>"highlight",
                    MoveHints::None=>"none",
                },
                match self.time_control {
                    TimeControl::Infinite=>"infinite",  TimeControl::Bullet=>"bullet",
                    TimeControl::Blitz=>"blitz",        TimeControl::Rapid=>"rapid",
                    TimeControl::Classical=>"classical",
                },
                self.show_coords, self.show_clock, self.flip_board,
                self.auto_flip,   self.confirm_move,
                match self.ui_mode {
                    UiMode::Minimal=>"minimal", UiMode::Standard=>"standard",
                    UiMode::Analysis=>"analysis",
                },
                self.auto_save_png,
                self.lichess_token,
                self.blunder_cp, self.mistake_cp, self.inaccuracy_cp,
                self.stockfish_skill,
                match self.analysis_engine {
                    AnalysisEngine::Builtin=>"builtin",
                    AnalysisEngine::Stockfish=>"stockfish",
                },
                self.analysis_depth,
                self.highlight_brightness,
                self.cell_w, self.cell_h,
            ));
        }
    }
}

// ── Config path: ~/.config/rchess/rchess_tui.conf ────────────────────────────
fn config_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| {
            let dir = PathBuf::from(h).join(".config").join("rchess");
            let _ = fs::create_dir_all(&dir);
            dir.join("rchess_tui.conf")
        })
}
