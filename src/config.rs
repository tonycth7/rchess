// src/config.rs
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Classic,      // lichess cream/brown  ← default, "original"
    Tournament,   // forest green + gold
    Mocha,        // warm brown + cream
    Slate,        // cool grey + cyan
    Midnight,     // dark navy + purple
    Crimson,      // deep red + white
}

impl Theme {
    pub const ALL: &'static [Theme] = &[
        Theme::Classic, Theme::Tournament, Theme::Mocha,
        Theme::Slate,   Theme::Midnight,   Theme::Crimson,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Theme::Classic     => "Classic      (cream/brown, original)",
            Theme::Tournament  => "Tournament   (green/gold)",
            Theme::Mocha       => "Mocha        (brown/cream)",
            Theme::Slate       => "Slate        (grey/cyan)",
            Theme::Midnight    => "Midnight     (navy/purple)",
            Theme::Crimson     => "Crimson      (red/white)",
        }
    }
    /// (light_sq, dark_sq)
    pub fn squares(self) -> ((u8,u8,u8),(u8,u8,u8)) {
        match self {
            Theme::Classic    => ((240,217,181),(181,136, 99)),
            Theme::Tournament => ((100,140,100),( 50, 80, 50)),
            Theme::Mocha      => ((180,140,100),(110, 80, 50)),
            Theme::Slate      => (( 90,110,130),( 50, 65, 80)),
            Theme::Midnight   => (( 60, 60,120),( 30, 30, 70)),
            Theme::Crimson    => ((160, 60, 60),( 90, 30, 30)),
        }
    }
    /// UI chrome accent (borders, headings)
    pub fn accent(self) -> (u8,u8,u8) {
        match self {
            Theme::Classic    => (160,110, 60),
            Theme::Tournament => (210,175, 60),
            Theme::Mocha      => (200,160, 90),
            Theme::Slate      => ( 80,200,200),
            Theme::Midnight   => (160,100,220),
            Theme::Crimson    => (240,200,200),
        }
    }
    /// Selected-piece square highlight
    pub fn select(self) -> (u8,u8,u8) {
        match self {
            Theme::Classic    => (205,210, 60),   // classic yellow-green
            Theme::Tournament => (180,160, 40),
            Theme::Mocha      => (200,170, 80),
            Theme::Slate      => ( 60,180,180),
            Theme::Midnight   => (120, 80,200),
            Theme::Crimson    => (220, 80, 80),
        }
    }
    /// Cursor (un-selected hover) square highlight
    pub fn cursor(self) -> (u8,u8,u8) {
        match self {
            Theme::Classic => (246,246,130),   // soft yellow, like lichess
            _              => ( 60,110, 80),
        }
    }
    /// Last-move square tint (blended with square)
    pub fn last_move(self) -> (u8,u8,u8) {
        match self {
            Theme::Classic => (206,210, 90),
            _              => self.accent(),
        }
    }
    /// Piece colors (white_piece, black_piece)
    pub fn piece_colors(self) -> ((u8,u8,u8),(u8,u8,u8)) {
        match self {
            Theme::Classic => ((255,255,255),(10,10,10)),
            _              => ((240,235,210),(28,18, 8)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceStyle {
    Unicode,
    Letters,
    FatLetters,
}
impl PieceStyle {
    pub const ALL: &'static [PieceStyle] = &[
        PieceStyle::Unicode, PieceStyle::Letters, PieceStyle::FatLetters,
    ];
    pub fn name(self) -> &'static str {
        match self {
            PieceStyle::Unicode    => "Unicode symbols  ♔♕♖♗♘♙",
            PieceStyle::Letters    => "ASCII letters    K Q R B N P",
            PieceStyle::FatLetters => "Bracketed        [K][Q][R]…",
        }
    }
    pub fn render(self, sym: &'static str, letter: char, is_white: bool) -> String {
        match self {
            PieceStyle::Unicode => sym.to_string(),
            PieceStyle::Letters => {
                let c = if is_white { letter.to_ascii_uppercase() } else { letter.to_ascii_lowercase() };
                c.to_string()
            }
            PieceStyle::FatLetters => {
                let c = if is_white { letter.to_ascii_uppercase() } else { letter.to_ascii_lowercase() };
                format!("[{}]", c)
            }
        }
    }
}

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveHints { Dots, Highlight, None }
impl MoveHints {
    pub const ALL: &'static [MoveHints] = &[MoveHints::Dots, MoveHints::Highlight, MoveHints::None];
    pub fn name(self) -> &'static str {
        match self {
            MoveHints::Dots      => "Dots       (· on target squares)",
            MoveHints::Highlight => "Highlight  (full square glow)",
            MoveHints::None      => "Off        (no hints)",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub theme:        Theme,
    pub piece_style:  PieceStyle,
    pub ai_depth:     AiDepth,
    pub move_hints:   MoveHints,
    pub show_coords:  bool,
    pub show_clock:   bool,
    pub flip_board:   bool,
    pub confirm_move: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme:        Theme::Classic,
            piece_style:  PieceStyle::Unicode,
            ai_depth:     AiDepth::Hard,
            move_hints:   MoveHints::Dots,
            show_coords:  true,
            show_clock:   true,
            flip_board:   false,
            confirm_move: false,
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
                            _            => Theme::Tournament,
                        },
                        "piece_style" => cfg.piece_style = match val {
                            "letters"    => PieceStyle::Letters,
                            "fatletters" => PieceStyle::FatLetters,
                            _            => PieceStyle::Unicode,
                        },
                        "ai_depth" => cfg.ai_depth = match val {
                            "1"=>"AiDepth::Easy", "2"=>"AiDepth::Medium", "4"=>"AiDepth::Expert", _=>"AiDepth::Hard"
                        }.parse().unwrap_or(AiDepth::Hard),
                        "move_hints" => cfg.move_hints = match val {
                            "highlight" => MoveHints::Highlight,
                            "none"      => MoveHints::None,
                            _           => MoveHints::Dots,
                        },
                        "show_coords"  => cfg.show_coords  = val == "true",
                        "show_clock"   => cfg.show_clock   = val == "true",
                        "flip_board"   => cfg.flip_board   = val == "true",
                        "confirm_move" => cfg.confirm_move = val == "true",
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
                "# Chess TUI  — edit here or use in-game Settings (s)\n\n\
                 theme        = {}\n\
                 piece_style  = {}\n\
                 ai_depth     = {}\n\
                 move_hints   = {}\n\
                 show_coords  = {}\n\
                 show_clock   = {}\n\
                 flip_board   = {}\n\
                 confirm_move = {}\n",
                match self.theme { Theme::Classic=>"classic", Theme::Tournament=>"tournament", Theme::Mocha=>"mocha", Theme::Slate=>"slate", Theme::Midnight=>"midnight", Theme::Crimson=>"crimson" },
                match self.piece_style { PieceStyle::Unicode=>"unicode", PieceStyle::Letters=>"letters", PieceStyle::FatLetters=>"fatletters" },
                self.ai_depth.depth(),
                match self.move_hints { MoveHints::Dots=>"dots", MoveHints::Highlight=>"highlight", MoveHints::None=>"none" },
                self.show_coords, self.show_clock, self.flip_board, self.confirm_move,
            ));
        }
    }
}

fn config_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".chess_tui.conf"))
}

// Allow parsing AiDepth from string slice (used in load())
impl std::str::FromStr for AiDepth {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "AiDepth::Easy"   => AiDepth::Easy,
            "AiDepth::Medium" => AiDepth::Medium,
            "AiDepth::Expert" => AiDepth::Expert,
            _                 => AiDepth::Hard,
        })
    }
}
