// src/config.rs
use std::fs;
use std::path::PathBuf;

// ── Theme ─────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Classic, Tournament, Slate, MatteBlack, Mocha,
}
impl Theme {
    pub const ALL: &'static [Theme] = &[
        Theme::Classic, Theme::Tournament, Theme::Slate, Theme::MatteBlack, Theme::Mocha,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Theme::Classic    => "Classic     (cream/brown)",
            Theme::Tournament => "Tournament  (green/gold)",
            Theme::Slate      => "Slate       (grey/cyan)",
            Theme::MatteBlack => "Matte Black (dark grey/gold)",
            Theme::Mocha      => "Mocha       (Catppuccin Mocha)",
        }
    }
    pub fn squares(self) -> ((u8,u8,u8),(u8,u8,u8)) {
        match self {
            Theme::Classic    => ((240,217,181),(181,136, 99)),
            Theme::Tournament => ((100,140,100),( 50, 80, 50)),
            Theme::Slate      => (( 90,110,130),( 50, 65, 80)),
            Theme::MatteBlack => (( 55, 55, 55),( 22, 22, 22)),
            Theme::Mocha      => (( 69, 71, 90),( 49, 50, 68)),
        }
    }
    pub fn accent(self) -> (u8,u8,u8) {
        match self {
            Theme::Classic    => (160,110, 60),
            Theme::Tournament => (210,175, 60),
            Theme::Slate      => ( 80,200,200),
            Theme::MatteBlack => (200,160, 30),
            Theme::Mocha      => (203,166,247),
        }
    }
    pub fn select(self) -> (u8,u8,u8) {
        match self {
            Theme::Classic    => (205,210, 60),
            Theme::Tournament => (180,160, 40),
            Theme::Slate      => ( 60,180,180),
            Theme::MatteBlack => (180,140, 20),
            Theme::Mocha      => (250,179,135),
        }
    }
    pub fn cursor(self) -> (u8,u8,u8) {
        match self {
            Theme::MatteBlack => (220,180, 30),
            Theme::Mocha      => (137,220,235),
            _              => (200,180, 50),
        }
    }
    pub fn last_move(self) -> (u8,u8,u8) {
        match self {
            Theme::MatteBlack => (160,130, 20),
            Theme::Mocha      => (180,190,254),
            _              => (160,140, 30),
        }
    }
    pub fn piece_colors(self) -> ((u8,u8,u8),(u8,u8,u8)) {
        match self {
            Theme::MatteBlack => ((240,230,200),(180,170,160)),
            Theme::Classic    => ((255,248,220),(80,60,40)),
            Theme::Tournament => ((240,240,240),(50,55,50)),
            Theme::Slate      => ((230,230,240),(100,140,160)),
            Theme::Mocha      => ((205,214,244),(127,132,156)),
        }
    }
    pub fn bg_colors(self) -> ((u8,u8,u8),(u8,u8,u8)) {
        match self {
            Theme::Classic    => ((36, 22, 12),(52, 34, 18)),
            Theme::Tournament => ((14, 20, 14),(22, 30, 22)),
            Theme::Slate      => ((16, 20, 28),(22, 28, 40)),
            Theme::MatteBlack => (( 8,  8,  8),(16, 16, 16)),
            Theme::Mocha      => ((30, 30, 46),(24, 24, 37)),
        }
    }
    pub fn danger(self) -> (u8,u8,u8) {
        match self {
            Theme::Mocha => (243,139,168),
            _            => (200, 70, 70),
        }
    }
    pub fn warning(self) -> (u8,u8,u8) {
        match self {
            Theme::Mocha => (249,226,175),
            _            => (220,160, 40),
        }
    }
    pub fn success(self) -> (u8,u8,u8) {
        match self {
            Theme::Mocha => (166,227,161),
            _            => (120,220,160),
        }
    }
    pub fn info(self) -> (u8,u8,u8) {
        match self {
            Theme::Mocha => (137,180,250),
            _            => (80,180,140),
        }
    }
    pub fn text_dim(self) -> (u8,u8,u8) {
        match self {
            Theme::Mocha      => (108,112,134),
            Theme::MatteBlack => (180,170,160),
            _                 => (110,130,110),
        }
    }
}

// ── PieceStyle ────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceStyle { Unicode, Ascii }
impl PieceStyle {
    pub const ALL: &'static [PieceStyle] = &[
        PieceStyle::Unicode, PieceStyle::Ascii,
    ];
    pub fn name(self) -> &'static str {
        match self {
            PieceStyle::Unicode => "Unicode symbols",
            PieceStyle::Ascii   => "ASCII art (star pieces)",
        }
    }
}

// ── AiDepth ───────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AiDepth { Easy=1, Medium=2, Hard=3, Expert=4, Stockfish=5 }
impl AiDepth {
    pub const ALL: &'static [AiDepth] = &[
        AiDepth::Easy, AiDepth::Medium, AiDepth::Hard, AiDepth::Expert, AiDepth::Stockfish
    ];
    pub fn name(self) -> &'static str {
        match self {
            AiDepth::Easy      => "Easy      (depth 1)",
            AiDepth::Medium    => "Medium    (depth 2)",
            AiDepth::Hard      => "Hard      (depth 3)",
            AiDepth::Expert    => "Expert    (depth 4)",
            AiDepth::Stockfish => "Stockfish (strongest)",
        }
    }
    /// Returns minimax depth. Returns 4 for Stockfish (fallback if SF not installed).
    pub fn depth(self) -> u8 {
        match self {
            AiDepth::Easy   => 1,
            AiDepth::Medium => 2,
            AiDepth::Hard   => 3,
            AiDepth::Expert => 4,
            AiDepth::Stockfish => 4, // fallback depth
        }
    }
    pub fn is_stockfish(self) -> bool { self == AiDepth::Stockfish }
}
impl std::str::FromStr for AiDepth {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "1" | "easy"      => AiDepth::Easy,
            "5" | "stockfish" => AiDepth::Stockfish,
            "2" | "medium"    => AiDepth::Medium,
            "4" | "expert"    => AiDepth::Expert,
            _                  => AiDepth::Hard,
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

// ── Settings catalog ──────────────────────────────────────────────────────────
/// Every user-facing setting, in display order.
///
/// This is the single source of truth: both the settings screen and the key
/// handler iterate [`Setting::ALL`], so the two can never drift apart again.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Setting {
    Theme,
    PieceStyle,
    AiDepth,
    MoveHints,
    TimeControl,
    ShowCoords,
    ShowClock,
    FlipBoard,
    AutoFlip,
    ConfirmMove,
    AutoPromote,
    UiMode,
    AnalysisEngine,
    AnalysisDepth,
    StockfishSkill,
    AutoSavePng,
    AutoSaveGame,
    HighlightBrightness,
    TtyMode,
    LichessToken,
}

impl Setting {
    pub const ALL: &'static [Setting] = &[
        Setting::Theme,
        Setting::PieceStyle,
        Setting::AiDepth,
        Setting::MoveHints,
        Setting::TimeControl,
        Setting::ShowCoords,
        Setting::ShowClock,
        Setting::FlipBoard,
        Setting::AutoFlip,
        Setting::ConfirmMove,
        Setting::AutoPromote,
        Setting::UiMode,
        Setting::AnalysisEngine,
        Setting::AnalysisDepth,
        Setting::StockfishSkill,
        Setting::AutoSavePng,
        Setting::AutoSaveGame,
        Setting::HighlightBrightness,
        Setting::TtyMode,
        Setting::LichessToken,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Setting::Theme               => "Theme",
            Setting::PieceStyle          => "Piece style",
            Setting::AiDepth             => "AI difficulty",
            Setting::MoveHints           => "Move hints",
            Setting::TimeControl         => "Time control",
            Setting::ShowCoords          => "Show coords",
            Setting::ShowClock           => "Show clock",
            Setting::FlipBoard           => "Flip board",
            Setting::AutoFlip            => "Auto-flip PvP",
            Setting::ConfirmMove         => "Confirm move",
            Setting::AutoPromote         => "Auto-promote",
            Setting::UiMode              => "UI mode",
            Setting::AnalysisEngine      => "Analysis engine",
            Setting::AnalysisDepth       => "Analysis depth",
            Setting::StockfishSkill      => "Stockfish skill",
            Setting::AutoSavePng         => "Auto-save PNG",
            Setting::AutoSaveGame        => "Auto-save game",
            Setting::HighlightBrightness => "Highlight bright",
            Setting::TtyMode             => "TTY mode",
            Setting::LichessToken        => "Lichess token",
        }
    }

    /// Text shown on the right-hand side of the settings row.
    pub fn value(self, cfg: &Config) -> String {
        match self {
            Setting::Theme          => cfg.theme.name().to_string(),
            Setting::PieceStyle     => cfg.piece_style.name().to_string(),
            Setting::AiDepth        => cfg.ai_depth.name().to_string(),
            Setting::MoveHints      => cfg.move_hints.name().to_string(),
            Setting::TimeControl    => cfg.time_control.name().to_string(),
            Setting::ShowCoords     => on_off(cfg.show_coords),
            Setting::ShowClock      => on_off(cfg.show_clock),
            Setting::FlipBoard      => on_off(cfg.flip_board),
            Setting::AutoFlip       => on_off(cfg.auto_flip),
            Setting::ConfirmMove    => on_off(cfg.confirm_move),
            Setting::AutoPromote    => if cfg.auto_promote { "ON (queen)" } else { "OFF" }.to_string(),
            Setting::UiMode         => cfg.ui_mode.name().to_string(),
            Setting::AnalysisEngine => cfg.analysis_engine.name().to_string(),
            Setting::AnalysisDepth  => match cfg.analysis_depth {
                1 => "1  (fast)", 2 => "2  (balanced)", _ => "3  (strong)",
            }.to_string(),
            Setting::StockfishSkill => format!("{} (0-20)", cfg.stockfish_skill),
            Setting::AutoSavePng    => on_off(cfg.auto_save_png),
            Setting::AutoSaveGame   => on_off(cfg.auto_save_game),
            Setting::HighlightBrightness => match cfg.highlight_brightness {
                0 => "0  (dim)".to_string(),
                5 => "5  (default)".to_string(),
                10 => "10 (vivid)".to_string(),
                n => n.to_string(),
            },
            Setting::TtyMode => if cfg.tty_mode { "ON  (16-col / ASCII)" } else { "OFF (truecolor)" }.to_string(),
            Setting::LichessToken => {
                if cfg.has_lichess_token() { mask_token(&cfg.lichess_token) }
                else { "(not set - Enter to add)".to_string() }
            }
        }
    }

    /// True when the row is edited by typing text rather than cycling values.
    pub fn is_text(self) -> bool {
        matches!(self, Setting::LichessToken)
    }
}

fn on_off(v: bool) -> String {
    if v { "ON".to_string() } else { "OFF".to_string() }
}

/// Never print a token in full; keep the non-secret prefix and last 2 chars.
fn mask_token(token: &str) -> String {
    let n = token.chars().count();
    if n <= 6 { return "lip_****** (configured)".to_string(); }
    let tail: String = token.chars().skip(n - 2).collect();
    format!("lip_********{tail}  (configured)")
}

// ── Config ────────────────────────────────────────────────────────────────────
#[derive(Clone, Debug, PartialEq)]
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
    pub auto_promote:    bool,
    pub ui_mode:         UiMode,
    /// Auto-save board PNG when a game ends (default: false)
    pub auto_save_png:   bool,
    /// Auto-save game JSON when a game ends (default: true)
    pub auto_save_game:  bool,
    /// Optional Lichess API token (puzzles + online play)
    pub lichess_token:   String,
    /// Lichess username (auto-filled from token, cached here)
    pub lichess_username: String,
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
    /// TTY compatibility mode: 16-color, ASCII pieces, plain borders (default: false)
    pub tty_mode: bool,
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
            auto_promote:    false,
            ui_mode:         UiMode::Standard,
            auto_save_png:   false,
            auto_save_game:  true,
            lichess_token:   String::new(),
            lichess_username: String::new(),
            blunder_cp:      300,
            mistake_cp:      100,
            inaccuracy_cp:    50,
            stockfish_skill:  10,
            analysis_engine: AnalysisEngine::Builtin,
            analysis_depth: 2,
            highlight_brightness: 5,
            tty_mode: false,
        }
    }
}

impl Config {
    /// A token is usable unless it is empty or one of the boolean strings that
    /// older versions could accidentally write into the field.
    pub fn has_lichess_token(&self) -> bool {
        let t = self.lichess_token.trim();
        !t.is_empty() && !matches!(t, "true" | "false") && t != "none"
    }

    pub fn load() -> Self {
        let Some(path) = config_path() else { return Config::default(); };
        let Ok(text) = fs::read_to_string(&path) else { return Config::default(); };
        let (cfg, repaired) = Config::parse_impl(&text);
        if repaired {
            // Persist the recovery so the bad values don't return next launch.
            let _ = fs::write(&path, cfg.serialise());
            rlog!("[config] repaired values shifted by the old writer; config rewritten");
        }
        cfg
    }

    pub fn save(&self) {
        if let Some(path) = config_path() {
            let _ = fs::write(path, self.serialise());
        }
    }

    /// Serialise to the on-disk `key = value` format.
    ///
    /// Every line is written from an explicit (key, value) pair; the old
    /// positional `format!` call silently shifted all arguments by one, which
    /// overwrote the Lichess token with a boolean whenever settings were saved.
    pub fn serialise(&self) -> String {
        let mut out = String::from(
            "# RChess TUI - edit here or use in-game Settings (s)\n\
             #\n\
             # analysis_engine options:\n\
             #   builtin   - pure Rust minimax, always works, no install needed\n\
             #   stockfish - install: sudo pacman -S stockfish  (Arch)\n\
             #               install: sudo apt install stockfish (Ubuntu)\n\
             #\n\
             # ui_mode options: minimal | standard | analysis\n\
             #\n\
             # lichess_token: create at https://lichess.org/account/oauth/token\n\
             #   (enable the board:play scope). Can also be set in Settings.\n\n",
        );
        let mut row = |key: &str, value: String| {
            out.push_str(&format!("{key:<20}= {value}\n"));
        };
        row("theme",            self.theme_key().to_string());
        row("piece_style",      self.piece_style_key().to_string());
        row("ai_depth",         match self.ai_depth { AiDepth::Stockfish => 5, d => d.depth() }.to_string());
        row("move_hints",       self.move_hints_key().to_string());
        row("time_control",     self.time_control_key().to_string());
        row("show_coords",      self.show_coords.to_string());
        row("show_clock",       self.show_clock.to_string());
        row("flip_board",       self.flip_board.to_string());
        row("auto_flip",        self.auto_flip.to_string());
        row("confirm_move",     self.confirm_move.to_string());
        row("auto_promote",     self.auto_promote.to_string());
        row("ui_mode",          self.ui_mode_key().to_string());
        row("auto_save_png",    self.auto_save_png.to_string());
        row("auto_save_game",   self.auto_save_game.to_string());
        row("lichess_token",    self.lichess_token.clone());
        row("lichess_username", self.lichess_username.clone());
        row("blunder_cp",       self.blunder_cp.to_string());
        row("mistake_cp",       self.mistake_cp.to_string());
        row("inaccuracy_cp",    self.inaccuracy_cp.to_string());
        row("stockfish_skill",  self.stockfish_skill.to_string());
        row("analysis_engine",  self.analysis_engine_key().to_string());
        row("analysis_depth",   self.analysis_depth.to_string());
        row("highlight_brightness", self.highlight_brightness.to_string());
        row("tty_mode",         self.tty_mode.to_string());
        out
    }

    /// Parse the on-disk format. Unknown keys and malformed values are ignored.
    pub fn parse(text: &str) -> Self {
        Self::parse_impl(text).0
    }

    fn parse_impl(text: &str) -> (Self, bool) {
        // Collect first: a pre-0.7.4 bug shifted every value one key down, and
        // the original ordering is needed to undo that.
        let mut pairs: Vec<(String, String)> = text.lines().filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { return None; }
            let (key, val) = line.split_once('=')?;
            Some((key.trim().to_string(), val.trim().to_string()))
        }).collect();
        let repaired = take_shifted_values(&mut pairs);

        let mut cfg = Config::default();
        for (key, val) in &pairs {
            let val = val.as_str();
            match key.as_str() {
                "theme" => cfg.theme = match val {
                    "tournament" => Theme::Tournament,
                    "slate"      => Theme::Slate,
                    "matteblack" => Theme::MatteBlack,
                    "mocha"      => Theme::Mocha,
                    _            => Theme::Classic,
                },
                "piece_style" => cfg.piece_style = match val {
                    "ascii" => PieceStyle::Ascii,
                    _       => PieceStyle::Unicode,
                },
                "ai_depth"     => cfg.ai_depth    = val.parse().unwrap_or(AiDepth::Hard),
                "move_hints"   => cfg.move_hints  = match val {
                    "highlight" => MoveHints::Highlight,
                    "none"      => MoveHints::None,
                    _           => MoveHints::Dots,
                },
                "time_control" => cfg.time_control = match val {
                    "bullet"    => TimeControl::Bullet,
                    "blitz"     => TimeControl::Blitz,
                    "rapid"     => TimeControl::Rapid,
                    "classical" => TimeControl::Classical,
                    _           => TimeControl::Infinite,
                },
                "show_coords"      => cfg.show_coords  = val == "true",
                "show_clock"       => cfg.show_clock   = val == "true",
                "flip_board"       => cfg.flip_board   = val == "true",
                "auto_flip"        => cfg.auto_flip    = val == "true",
                "confirm_move"     => cfg.confirm_move = val == "true",
                "auto_promote"     => cfg.auto_promote = val == "true",
                "ui_mode" => cfg.ui_mode = match val {
                    "minimal"  => UiMode::Minimal,
                    "analysis" => UiMode::Analysis,
                    _          => UiMode::Standard,
                },
                "auto_save_png"    => cfg.auto_save_png   = val == "true",
                "auto_save_game"   => cfg.auto_save_game  = val == "true",
                "lichess_token"    => cfg.lichess_token   = val.to_string(),
                "lichess_username" => cfg.lichess_username = val.to_string(),
                "blunder_cp"       => cfg.blunder_cp      = val.parse().unwrap_or(300),
                "mistake_cp"       => cfg.mistake_cp      = val.parse().unwrap_or(100),
                "inaccuracy_cp"    => cfg.inaccuracy_cp   = val.parse().unwrap_or(50),
                "stockfish_skill"  => cfg.stockfish_skill = val.parse::<u8>().unwrap_or(10).min(20),
                "analysis_engine"  => cfg.analysis_engine = match val {
                    "stockfish" => AnalysisEngine::Stockfish,
                    _           => AnalysisEngine::Builtin,
                },
                "analysis_depth" => cfg.analysis_depth = val.parse::<u8>().unwrap_or(2).clamp(1, 3),
                "highlight_brightness" => cfg.highlight_brightness = val.parse::<u8>().unwrap_or(5).min(10),
                "tty_mode" => cfg.tty_mode = val == "true",
                _ => {}
            }
        }
        cfg.repair_legacy();
        (cfg, repaired)
    }

    /// Recover from the argument-shift bug that wrote `true`/`false` into
    /// `lichess_token` and the real token into `lichess_username`.
    fn repair_legacy(&mut self) {
        let username = self.lichess_username.trim().to_string();
        if !self.has_lichess_token() && username.starts_with("lip_") {
            self.lichess_token = username;
            self.lichess_username.clear();
        } else if self.lichess_username == "true" || self.lichess_username == "false" {
            self.lichess_username.clear();
        }
        if !self.has_lichess_token() {
            self.lichess_token.clear();
        }
    }

    fn theme_key(&self) -> &'static str {
        match self.theme {
            Theme::Classic => "classic", Theme::Tournament => "tournament",
            Theme::Slate => "slate", Theme::MatteBlack => "matteblack",
            Theme::Mocha => "mocha",
        }
    }
    fn piece_style_key(&self) -> &'static str {
        match self.piece_style { PieceStyle::Unicode => "unicode", PieceStyle::Ascii => "ascii" }
    }
    fn move_hints_key(&self) -> &'static str {
        match self.move_hints {
            MoveHints::Dots => "dots", MoveHints::Highlight => "highlight", MoveHints::None => "none",
        }
    }
    fn time_control_key(&self) -> &'static str {
        match self.time_control {
            TimeControl::Infinite => "infinite", TimeControl::Bullet => "bullet",
            TimeControl::Blitz => "blitz", TimeControl::Rapid => "rapid",
            TimeControl::Classical => "classical",
        }
    }
    fn ui_mode_key(&self) -> &'static str {
        match self.ui_mode {
            UiMode::Minimal => "minimal", UiMode::Standard => "standard", UiMode::Analysis => "analysis",
        }
    }
    fn analysis_engine_key(&self) -> &'static str {
        match self.analysis_engine {
            AnalysisEngine::Builtin => "builtin", AnalysisEngine::Stockfish => "stockfish",
        }
    }
}

// ── Legacy config repair ─────────────────────────────────────────────────────
// The pre-0.7.4 serialiser passed positional `format!` arguments, so every
// value landed one key too low (theme got a bool, piece_style got the theme,
// …) and the last value was dropped. The signature is unmistakable: `theme` is
// not a theme name while `piece_style` is. Rotate the values back so user
// preferences survive the upgrade.
const LEGACY_KEYS: [&str; 24] = [
    "theme", "piece_style", "ai_depth", "move_hints", "time_control",
    "show_coords", "show_clock", "flip_board", "auto_flip", "confirm_move",
    "auto_promote", "ui_mode", "auto_save_png", "auto_save_game",
    "lichess_token", "lichess_username", "blunder_cp", "mistake_cp",
    "inaccuracy_cp", "stockfish_skill", "analysis_engine", "analysis_depth",
    "highlight_brightness", "tty_mode",
];

fn is_theme_key(s: &str) -> bool {
    matches!(s, "classic" | "tournament" | "slate" | "matteblack" | "mocha")
}

fn take_shifted_values(pairs: &mut [(String, String)]) -> bool {
    let get = |k: &str| pairs.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str());
    let (Some(theme), Some(piece_style)) = (get("theme"), get("piece_style")) else {
        return false;
    };
    if is_theme_key(theme) || !is_theme_key(piece_style) {
        return false;
    }
    let original: Vec<Option<String>> = LEGACY_KEYS
        .iter()
        .map(|k| pairs.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone()))
        .collect();
    for (i, key) in LEGACY_KEYS.iter().enumerate() {
        let next = original.get(i + 1).and_then(|v| v.clone());
        if let Some((_, slot)) = pairs.iter_mut().find(|(k, _)| k == key) {
            *slot = next.unwrap_or_default();
        }
    }
    true
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        Config {
            theme: Theme::Mocha,
            piece_style: PieceStyle::Ascii,
            ai_depth: AiDepth::Expert,
            move_hints: MoveHints::Highlight,
            time_control: TimeControl::Blitz,
            show_coords: false,
            show_clock: true,
            flip_board: true,
            auto_flip: true,
            confirm_move: true,
            auto_promote: true,
            ui_mode: UiMode::Analysis,
            auto_save_png: true,
            auto_save_game: false,
            lichess_token: "lip_abcdefghijklmnopqrstuvwxyz".into(),
            lichess_username: "testuser".into(),
            blunder_cp: 280,
            mistake_cp: 90,
            inaccuracy_cp: 40,
            stockfish_skill: 17,
            analysis_engine: AnalysisEngine::Stockfish,
            analysis_depth: 3,
            highlight_brightness: 9,
            tty_mode: true,
        }
    }

    #[test]
    fn roundtrip_preserves_every_field() {
        let cfg = sample();
        assert_eq!(Config::parse(&cfg.serialise()), cfg);
    }

    #[test]
    fn saving_preserves_the_token() {
        let cfg = sample();
        let reloaded = Config::parse(&cfg.serialise());
        assert_eq!(reloaded.lichess_token, cfg.lichess_token);
        assert_eq!(reloaded.lichess_username, cfg.lichess_username);
        assert!(reloaded.has_lichess_token());
    }

    #[test]
    fn legacy_corruption_is_repaired() {
        // What the old writer produced: token=true, username=<real token>.
        let text = "lichess_token = true\nlichess_username = lip_realtoken123\n";
        let cfg = Config::parse(text);
        assert_eq!(cfg.lichess_token, "lip_realtoken123");
        assert_eq!(cfg.lichess_username, "");
        assert!(cfg.has_lichess_token());
    }

    #[test]
    fn boolean_token_is_treated_as_unset() {
        let cfg = Config::parse("lichess_token = false\nlichess_username = alice\n");
        assert!(!cfg.has_lichess_token());
        assert_eq!(cfg.lichess_username, "alice");
    }

    #[test]
    fn unknown_theme_falls_back_to_default() {
        assert_eq!(Config::parse("theme = nonsense\n").theme, Theme::Classic);
    }

    #[test]
    fn repairs_config_shifted_by_the_old_serialiser() {
        // Verbatim shape of a config written by the positional-format bug.
        let text = "theme            = false\n\
                    piece_style      = matteblack\n\
                    ai_depth         = ascii\n\
                    move_hints       = 3\n\
                    time_control     = dots\n\
                    show_coords      = infinite\n\
                    show_clock       = false\n\
                    flip_board       = false\n\
                    auto_flip        = false\n\
                    confirm_move     = false\n\
                    auto_promote     = false\n\
                    ui_mode          = false\n\
                    auto_save_png    = standard\n\
                    auto_save_game   = false\n\
                    lichess_token    = false\n\
                    lichess_username = false\n\
                    blunder_cp       = false\n\
                    mistake_cp       = 300\n\
                    inaccuracy_cp    = 300\n\
                    stockfish_skill  = 300\n\
                    analysis_engine  = 10\n\
                    analysis_depth   = builtin\n\
                    highlight_brightness = 2\n\
                    tty_mode         = 2\n";
        let cfg = Config::parse(text);
        assert_eq!(cfg.theme, Theme::MatteBlack);
        assert_eq!(cfg.piece_style, PieceStyle::Ascii);
        assert_eq!(cfg.ai_depth, AiDepth::Hard);
        assert_eq!(cfg.move_hints, MoveHints::Dots);
        assert_eq!(cfg.time_control, TimeControl::Infinite);
        assert_eq!(cfg.ui_mode, UiMode::Standard);
        assert_eq!(cfg.stockfish_skill, 10);
        assert_eq!(cfg.analysis_engine, AnalysisEngine::Builtin);
        assert_eq!(cfg.analysis_depth, 2);
        assert_eq!(cfg.blunder_cp, 300);
        assert!(!cfg.has_lichess_token());
        assert!(cfg.lichess_username.is_empty());
    }

    #[test]
    fn a_correct_config_is_not_rotated() {
        let cfg = Config::parse("theme = matteblack\npiece_style = ascii\nai_depth = 2\n");
        assert_eq!(cfg.theme, Theme::MatteBlack);
        assert_eq!(cfg.piece_style, PieceStyle::Ascii);
        assert_eq!(cfg.ai_depth, AiDepth::Medium);
    }

    #[test]
    fn settings_catalog_is_complete_and_unique() {
        let mut labels: Vec<&str> = Setting::ALL.iter().map(|s| s.label()).collect();
        let count = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), count, "duplicate settings labels");
        assert_eq!(count, 20);
    }

    #[test]
    fn token_is_never_shown_in_full() {
        let cfg = sample();
        let shown = Setting::LichessToken.value(&cfg);
        assert!(!shown.contains(&cfg.lichess_token));
        assert!(shown.contains("configured"));
    }
}
