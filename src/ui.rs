// src/ui.rs  — v0.7
use std::collections::HashSet;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
};
use crate::app::{App, Gs, Mode, Screen, ClockState, GameEnd, ReplaySnap, MoveLabel, VERSION, ONLINE_TCS};
use crate::config::{MoveHints, PieceStyle, Setting, Theme, UiMode};
use crate::engine::{Color as PC, Kind, Piece, Status, king_sq};
use crate::pieces;

/// One move-list row: half-move index, white and black SAN, the two
/// selection/highlight flags, and optional annotations for each side.
type MovePair = (usize, String, String, bool, bool, Option<MoveLabel>, Option<MoveLabel>);

// ── TTY colour fallback ───────────────────────────────────────────────────────
use std::sync::atomic::{AtomicBool, Ordering};
pub static TTY_MODE: AtomicBool = AtomicBool::new(false);

fn tty_rgb(c: (u8,u8,u8), th: Theme) -> Color {
    if !TTY_MODE.load(Ordering::Relaxed) { return Color::Rgb(c.0,c.1,c.2); }
    if !matches!(th, Theme::MatteBlack | Theme::Mocha) {
        return Color::Rgb(c.0,c.1,c.2);
    }
    let (r, g, b) = c;
    let r16 = r as u16; let g16 = g as u16; let b16 = b as u16;
    let max = r16.max(g16).max(b16);
    let min = r16.min(g16).min(b16);
    let luma  = (r16 as f32 * 0.2126 + g16 as f32 * 0.7152 + b16 as f32 * 0.0722) as u16;
    let chroma = (max - min) as i32;
    // Map colored elements (chroma ≥ 50) to nearest ANSI hue
    let hue = if chroma < 50 {
        255 // neutral marker
    } else if b16 > r16 + 30 && b16 > g16 + 30 {
        12  // clearly blue-dominant → BrightBlue
    } else if b16 > r16 && b16 > g16 {
        14  // slightly blue-dominant → BrightCyan
    } else if r16 > b16 + 80 && g16 > b16 + 40 {
        11  // both r and g high vs b → BrightYellow
    } else if r16 > g16 + 30 && r16 > b16 + 30 {
        9   // r strongly dominant → BrightRed
    } else if g16 > r16 + 30 && g16 > b16 + 30 {
        10  // g strongly dominant → BrightGreen
    } else if g16 > b16 + 60 {
        14  // g significantly > b → BrightCyan
    } else {
        13  // default → BrightMagenta
    };
    let idx = if matches!(th, Theme::MatteBlack) {
        if      luma < 20  { 0 }        // bg0,bg1
        else if luma < 40  { 8 }        // dark sq
        else if luma < 60  { 7 }        // light sq
        else if chroma < 50 { 15 }      // neutral high-luma (pieces, dim)
        else               { hue }      // colored accent/select/cursor
    } else { // Mocha
        if      luma < 30  { 0 }        // bg1
        else if luma < 55  { 8 }        // bg0, dark sq
        else if luma < 90  { 7 }        // light sq
        else if chroma < 50 {           // neutral
            if luma < 160 { 0 } else { 15 } // piece_black=0, dim=0, piece_white=15
        } else               { hue }    // colored accent/select/cursor/last_move
    };
    Color::Indexed(idx)
}

// ── Palette helpers ───────────────────────────────────────────────────────────
fn rgb(c: (u8,u8,u8), th: Theme) -> Color { tty_rgb(c, th) }
fn bg0(th: Theme) -> Color { rgb(th.bg_colors().0, th) }
fn bg1(th: Theme) -> Color { rgb(th.bg_colors().1, th) }
fn ac(th: Theme)  -> Color { rgb(th.accent(), th) }
fn sq_l(th: Theme)-> Color { rgb(th.squares().0, th) }
fn sq_d(th: Theme)-> Color { rgb(th.squares().1, th) }
fn pw(th: Theme)  -> Color { rgb(th.piece_colors().0, th) }
fn pb_c(th: Theme)-> Color { rgb(th.piece_colors().1, th) }
fn danger(th: Theme)  -> Color { rgb(th.danger(), th) }
fn warning(th: Theme) -> Color { rgb(th.warning(), th) }
fn success(th: Theme) -> Color { rgb(th.success(), th) }
fn info(th: Theme)    -> Color { rgb(th.info(), th) }
fn dim(th: Theme)     -> Color { rgb(th.text_dim(), th) }
fn sty(c: Color)  -> Style { Style::default().fg(c) }

fn tty_boost(c: (u8,u8,u8)) -> Color {
    if !TTY_MODE.load(Ordering::Relaxed) { return Color::Rgb(c.0,c.1,c.2); }
    let (r,g,b) = c;
    let max_ch = r.max(g).max(b) as u16;
    if max_ch < 80 {
        let add = 80 - max_ch;
        Color::Rgb(
            (r as u16 + add).min(255) as u8,
            (g as u16 + add).min(255) as u8,
            (b as u16 + add).min(255) as u8,
        )
    } else { Color::Rgb(r,g,b) }
}
fn cream()  -> Color { tty_boost((220,210,185)) }
fn dimmer() -> Color { tty_boost((55,70,55)) }

// Board cell dimensions are auto-computed from terminal size each frame.

// Safely truncate to at most `max` characters (Unicode-aware).
fn trunc(s: &str, max: usize) -> String {
    if s.chars().count() <= max { return s.to_string(); }
    let mut out: String = s.chars().take(max.saturating_sub(2)).collect();
    out.push_str("..");
    out
}

/// Format a centipawn evaluation for display.
/// Handles mate scores: ±99999 cp (rcengine) → "M1", Stockfish ±30000 → "M1".
fn fmt_eval(cp: i32) -> String {
    if cp > 90_000 {
        let d = (99_999 - cp).max(1);
        format!("M{}", d)
    } else if cp < -90_000 {
        let d = (99_999 + cp).max(1);
        format!("-M{}", d)
    } else if cp.abs() == 30_000 {
        format!("{}M1", if cp > 0 { "" } else { "-" })
    } else {
        let v = cp as f32 / 100.0;
        if v > 0.0 { format!("+{:.2}", v) } else { format!("{:.2}", v) }
    }
}

/// Same as fmt_eval but takes an f32 (already divided by 100).
fn fmt_eval_f32(v: f32) -> String {
    fmt_eval((v * 100.0).round() as i32)
}

// ── Entry ─────────────────────────────────────────────────────────────────────
pub fn render(app: &App, f: &mut Frame) {
    let th = app.cfg.theme;
    f.render_widget(Block::default().style(Style::default().bg(bg0(th))), f.area());
    match app.screen {
        Screen::Menu        => draw_menu(app, f),
        Screen::ColorPick   => draw_color_pick(app, f),
        Screen::Settings    => draw_settings(app, f),
        Screen::Game        => draw_game(app, f),
        Screen::Promo       => { draw_game(app, f); draw_promo(app, f); }
        Screen::DrawOffer   => { draw_game(app, f); draw_draw_offer(app, f); }
        Screen::Analyze      => draw_analyze(app, f),
        Screen::PngPreview  => { draw_game(app, f); draw_png_preview(app, f); }
        Screen::PgnSaved    => { draw_game(app, f); draw_pgn_saved(app, f); }
        Screen::Import      => draw_import(app, f),
        Screen::Puzzle      => draw_puzzle(app, f),
        Screen::OnlineLobby => draw_online_lobby(app, f),
        Screen::OnlineGame  => draw_online_game(app, f),
        Screen::LoadGame    => draw_load_game(app, f),
    }
    if app.help { draw_help(app, f); }
}

// ── HELP OVERLAY ──────────────────────────────────────────────────────────────
fn draw_help(app: &App, f: &mut Frame) {
    let th = app.cfg.theme;
    let (title, rows): (&str, Vec<(&str, &str)>) = match app.screen {
        Screen::Menu => ("MENU", vec![
            ("↑↓ / jk", "move selection"),
            ("Enter / Space", "open the selected item"),
            ("s", "settings"),
            ("q / Esc", "quit"),
        ]),
        Screen::Settings => ("SETTINGS", vec![
            ("↑↓ / jk", "move between settings"),
            ("←→ / hl", "change value"),
            ("Enter", "edit the Lichess token"),
            ("w / s", "save config"),
            ("r", "restore defaults"),
            ("Esc / q", "back"),
        ]),
        Screen::OnlineLobby => ("LICHESS LOBBY", vec![
            ("↑↓ / jk", "select a challenge"),
            ("a / d", "accept / decline challenge"),
            ("Enter", "accept challenge, or resume a game"),
            ("1 / 2 / 3", "resume an ongoing game"),
            ("p", "find a random opponent"),
            ("r / t", "rated toggle / time control"),
            ("c", "challenge a user"),
            ("R", "reconnect and refresh"),
            ("s / q", "settings / main menu"),
        ]),
        Screen::OnlineGame => ("ONLINE GAME", vec![
            ("mouse / arrows", "move pieces (your turn only)"),
            ("letters + Enter", "type a move (e2e4, Nf3, O-O)"),
            ("d / D", "offer-or-accept / decline draw"),
            ("c", "chat"),
            ("t", "offer-or-accept takeback"),
            ("v / x / a", "claim win / resign / abort"),
            ("R", "resync with server"),
            ("q / Esc", "leave to lobby"),
        ]),
        Screen::Game => ("GAME", vec![
            ("mouse / arrows", "move pieces"),
            ("letters + Enter", "type a move (e2e4, Nf3, O-O)"),
            ("u / r", "undo / redo"),
            ("e", "engine hint arrow"),
            ("T", "cycle UI mode"),
            ("G / F / E", "save PGN / copy FEN / export PNG"),
            ("d", "offer a draw (two players)"),
            ("n", "new game"),
        ]),
        Screen::Analyze => ("ANALYSIS", vec![
            ("←→ / hl", "step through moves"),
            ("PgUp / PgDn", "jump 10 moves"),
            ("[ / ]", "previous / next mistake"),
            ("Home / 0", "jump to start"),
            ("End / $", "jump to end"),
            ("Esc / q", "back to game"),
        ]),
        Screen::Puzzle => ("PUZZLE", vec![
            ("mouse / arrows", "play the solution"),
            ("Enter / Space", "continue after a correct move"),
            ("n", "next puzzle"),
            ("Esc / q", "menu"),
        ]),
        _ => ("GENERAL", vec![
            ("Enter", "confirm"),
            ("Esc / q", "back"),
            ("↑↓ / jk", "navigate"),
        ]),
    };

    let mut lines = vec![Line::from("")];
    for (key, desc) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!("  {key:<18}"), sty(ac(th)).add_modifier(Modifier::BOLD)),
            Span::styled(desc, sty(cream())),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  ? or Esc closes this help", sty(dim(th)))]));

    let w = 58u16.min(f.area().width.saturating_sub(4));
    let h = (lines.len() as u16 + 2).min(f.area().height.saturating_sub(2));
    let rect = center(w, h, f.area());
    f.render_widget(Clear, rect);
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
                .title(Span::styled(format!(" ⌨  KEYBOARD HELP · {title} "),
                    sty(ac(th)).add_modifier(Modifier::BOLD)))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        rect,
    );
}

// ── MENU ──────────────────────────────────────────────────────────────────────
fn draw_menu(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(62, 32, f.area());
    let opts: &[&str] = &[
        "  ↻  RESUME GAME",
        "  ♟  TWO PLAYERS",
        "  ♟  VS COMPUTER   [AI + Opening Book]",
        "  ♟  ONLINE PLAY   [Lichess]",
        "  ⚙  SETTINGS",
        "  ♟  IMPORT GAME   [FEN / PGN / file]",
        "  ♟  LOAD SAVED GAME",
        "  ★  DAILY PUZZLE  [Lichess]",
    ];
    let logo_st = sty(ac(th)).add_modifier(Modifier::BOLD);
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled("  ██████╗  ██████╗██╗  ██╗███████╗███████╗███████╗", logo_st)]),
        Line::from(vec![Span::styled("  ██╔══██╗██╔════╝██║  ██║██╔════╝██╔════╝██╔════╝", logo_st)]),
        Line::from(vec![Span::styled("  ██████╔╝██║     ███████║█████╗  ███████╗███████╗", logo_st)]),
        Line::from(vec![Span::styled("  ██╔══██╗██║     ██╔══██║██╔══╝  ╚════██║╚════██║", logo_st)]),
        Line::from(vec![Span::styled("  ██║  ██║╚██████╗██║  ██║███████╗███████║███████║", logo_st)]),
        Line::from(vec![Span::styled("  ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝", logo_st)]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("      TERMINAL EDITION  v{}   PNG · PGN · Analyze · Mouse", VERSION), sty(dim(th)))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<54}",""), sty(dimmer()))]),
        Line::from(""),
    ];
    for (i, &opt) in opts.iter().enumerate() {
        let s = app.menu_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s{" ▶ "}else{"   "}, sty(ac(th))),
            Span::styled(format!("{:<48}", opt),
                if s { Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD) } else { sty(cream()) }),
        ]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(format!("  {:─<54}",""), sty(dimmer()))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  ↑↓ navigate    Enter select    ? help    q quit", sty(dimmer()))]));
    f.render_widget(Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .padding(Padding::new(2, 2, 1, 1)),
    ), rect);
}

// ── COLOR PICK ────────────────────────────────────────────────────────────────
fn draw_color_pick(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(52, 18, f.area());
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled("  CHOOSE YOUR SIDE", sty(ac(th)))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(dimmer()))]),
        Line::from(""),
    ];
    for (i,(sym,label,fg)) in [("♔","WHITE  — moves first",pw(th)),("♚","BLACK  — moves second",dim(th))].iter().enumerate() {
        let s = app.color_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s{" ▶ "}else{"   "}, sty(ac(th))),
            Span::styled(format!("{} {}",sym,label),
                Style::default().fg(*fg).bg(if s{bg0(th)}else{bg1(th)})
                    .add_modifier(if s{Modifier::BOLD}else{Modifier::empty()})),
        ]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(dimmer()))]));
    lines.push(Line::from(vec![Span::styled("  ↑↓/w,b navigate   Enter confirm   Esc back", sty(dimmer()))]));
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
                .padding(Padding::new(2, 2, 1, 1)),
        ),
        rect,
    );
}

// ── SETTINGS ──────────────────────────────────────────────────────────────────
fn draw_settings(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let area = f.area();

    // Use most of the terminal — leaves 2 rows top/bottom for breathing room
    let h    = area.height.saturating_sub(4).max(10);
    let w    = 72u16.min(area.width.saturating_sub(4));
    let rect = center(w, h, area);

    // All settings rows, generated from the shared catalog so the UI and the
    // key handler can never drift apart.
    let rows: Vec<(&str, String)> = Setting::ALL
        .iter()
        .map(|s| (s.label(), s.value(&app.cfg)))
        .collect();
    let n = rows.len();

    // Content area: rect minus 2 border rows minus 2 padding rows
    let content_h = rect.height.saturating_sub(4);
    // Overhead: leading blank + top separator + blank + scroll top + scroll bottom
    // + bottom separator + footer. The token editor needs a second footer line.
    let footer_lines = if app.token_edit { 2 } else { 1 };
    let budget = (content_h as usize).saturating_sub(5 + footer_lines);
    let visible = (budget / 2).max(3);

    // Scroll to keep selected row visible
    let cur = app.settings_cur;
    let scroll = if cur < visible { 0 }
                 else { cur + 1 - visible };
    let scroll = scroll.min(n.saturating_sub(visible));

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(format!("  {:─<60}", ""), sty(dimmer()))]),
        Line::from(""),
    ];

    // Scroll indicator top
    if scroll > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  ▲ {} more above", scroll), sty(dimmer()))]));
    } else {
        lines.push(Line::from(""));
    }

    // Visible rows
    for (i, (label, value)) in rows
        .iter()
        .enumerate()
        .take((scroll + visible).min(n))
        .skip(scroll)
    {
        let s = cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s { " ▶ " } else { "   " }, sty(ac(th))),
            Span::styled(
                format!("{:<18}", label),
                if s { sty(ac(th)).add_modifier(Modifier::BOLD) } else { sty(cream()) },
            ),
            Span::raw("  "),
            Span::styled(
                if s { format!("◀  {}  ▶", value) } else { format!("   {}   ", value) },
                if s { Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD) } else { sty(dim(th)) },
            ),
        ]));
        lines.push(Line::from(""));
    }

    // Scroll indicator bottom
    let below = n.saturating_sub(scroll + visible);
    if below > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  ▼ {} more below  (↓ to scroll)", below), sty(dimmer()))]));
    } else {
        lines.push(Line::from(""));
    }

    // Footer separator
    lines.push(Line::from(vec![Span::styled(format!("  {:─<60}", ""), sty(dimmer()))]));

    // Save notice or token editor
    if app.token_edit {
        let shown = if app.token_buf.is_empty() {
            "█".to_string()
        } else {
            let n = app.token_buf.chars().count();
            format!("{}{}", "•".repeat(n), "█")
        };
        lines.push(Line::from(vec![
            Span::styled("  Token: ", sty(ac(th)).add_modifier(Modifier::BOLD)),
            Span::styled(shown, sty(success(th))),
        ]));
        lines.push(Line::from(vec![Span::styled(
            if let Some(err) = &app.token_err {
                format!("  ✗ {err}")
            } else {
                "  Enter save & connect   Esc cancel   (lichess.org/account/oauth/token)".to_string()
            },
            sty(if app.token_err.is_some() { danger(th) } else { dim(th) }),
        )]));
    } else if app.saved_notice.is_some() {
        let home = std::env::var("HOME").unwrap_or_default();
        let path = format!("{}/.config/rchess/rchess_tui.conf", home);
        lines.push(Line::from(vec![
            Span::styled("  ✓ Saved  ", sty(success(th))),
            Span::styled(path, sty(success(th))),
        ]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  w/s save  r reset  ←→/hl change  Enter edit token  jk move  Esc back",
            sty(dimmer()),
        )]));
    }

    f.render_widget(Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" ⚙  SETTINGS ", sty(ac(th)).add_modifier(Modifier::BOLD)))
            .padding(Padding::new(2, 2, 1, 1)),
    ), rect);
}


// ── BLOCK/PIECE ART ─────────────────────────────────────────────────────
// Delegates to pieces::render_star_lines or render_hash_lines.

// ── GAME ──────────────────────────────────────────────────────────────────────
fn draw_game(app: &App, f: &mut Frame) {
    let a     = f.area();
    let cw    = app.cell_w;
    let bw    = (3 + 8*cw + 3 + 2).min(a.width);
    let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(a);
    let [board_col, side_col] = Layout::horizontal([Constraint::Length(bw), Constraint::Fill(1)]).areas(body);
    let [board_area, input_area] = Layout::vertical([Constraint::Fill(1), Constraint::Length(4)]).areas(board_col);

    draw_topbar(app, f, top);
    draw_board(app, f, board_area, app.cell_w as usize, app.cell_h as usize);
    draw_input(app, f, input_area);
    draw_sidebar(app, f, side_col);

    let is_over = matches!(app.gs.status, Status::Checkmate|Status::Stalemate|Status::Draw)
               || app.gs.clock_state == ClockState::Flagged
               || app.game_end == GameEnd::Draw;
    // Online games draw their own result banner (it also covers resign/timeout).
    if is_over && app.screen != Screen::OnlineGame { draw_gameover(app, f); }
}

fn draw_topbar(app: &App, f: &mut Frame, area: Rect) {
    let th = app.cfg.theme;
    let mode_s = if matches!(app.screen, Screen::OnlineGame) {
        let opp = app.lichess_opponent.clone().unwrap_or_else(|| "Lichess".into());
        let rating = app.lichess_opponent_rating.map(|r| format!(" ({r})")).unwrap_or_default();
        let tc = if app.online_tc_label.is_empty() { String::new() } else { format!(" {}", app.online_tc_label) };
        let turn = if app.online_result.is_some() {
            " · FINISHED"
        } else if app.gs.turn == app.player_color {
            " · YOUR MOVE"
        } else {
            ""
        };
        format!("ONLINE{tc} vs {opp}{rating}{turn}")
    } else {
        match app.mode {
            Mode::PvP => "TWO PLAYERS".to_string(),
            Mode::Cpu  => format!("VS CPU [YOU:{}] [{}]",
                app.player_color.name(),
                app.cfg.ai_depth.name().split_whitespace().next().unwrap_or("")),
        }
    };
    let (st, col) = status_str(app, th);
    let clock     = if app.cfg.show_clock { format!("  Mv {}", app.gs.fullmove) } else { String::new() };
    let opening_str: String = if !app.gs.history.is_empty() && app.gs.history.len() <= 30 {
        crate::book::opening_name(&app.gs.history)
            .map(|n| format!(" [{}]", n))
            .unwrap_or_default()
    } else { String::new() };
    let book_ind = opening_str.as_str();
    // Live eval from analysis engine
    let eval_str = if app.cfg.ui_mode != crate::config::UiMode::Minimal {
        let cp = app.last_eval_cp;
        let p  = cp as f32 / 100.0;
        if app.engine_busy {
            "  [~]".to_string()
        } else if !app.move_reviews.is_empty() {
            if p > 0.0 { format!("  [{:+.1}]", p) } else { format!("  [{:.1}]", p) }
        } else {
            String::new()
        }
    } else { String::new() };
    f.render_widget(
        Paragraph::new(format!(" ♟ RChess v{}{}{}{}│ {} {}",
            VERSION, clock, book_ind, eval_str, mode_s, st))
            .style(Style::default().fg(col).bg(bg1(th))),
        area,
    );
}

// ── BOARD ─────────────────────────────────────────────────────────────────────
fn draw_board(app: &App, f: &mut Frame, area: Rect, cell_w: usize, cell_h: usize) {
    #[allow(non_snake_case)]
    let CELL_W = cell_w;
    #[allow(non_snake_case)]
    let CELL_H = cell_h;
    let th      = app.cfg.theme;
    let flipped = app.flipped();
    let row_ord: [usize; 8] = if flipped { [7,6,5,4,3,2,1,0] } else { [0,1,2,3,4,5,6,7] };
    let col_ord: [usize; 8] = if flipped { [7,6,5,4,3,2,1,0] } else { [0,1,2,3,4,5,6,7] };

    // Precomputed blank line and underline for cells
    let blank_line: String = " ".repeat(CELL_W);
    let mid = CELL_H / 2;
    let coord_labels: Vec<String> = col_ord.iter().map(|&c| format!("{:^width$}", (b'a' + c as u8) as char, width = CELL_W)).collect();
    let bar_line: String = "─".repeat(CELL_W.saturating_sub(2));
    let tgt_underline: String = format!("  {:─<width$}  ","",width=CELL_W.saturating_sub(4));

    let tgts: HashSet<(usize,usize)> = app.targets.iter().map(|m| m.to).collect();
    let king_chk = if app.gs.status==Status::Check { king_sq(&app.gs.board,app.gs.turn) } else { None };
    let last     = app.gs.history.last();
    let cur_bc   = app.to_board(app.cursor);

    // Arrow sets for move arrows and hint arrows
    let arrow_src: HashSet<(usize,usize)> = app.arrows.iter().map(|&(fr,_)| fr).collect();
    let arrow_tgt: HashSet<(usize,usize)> = app.arrows.iter().map(|&(_,to)| to).collect();

    let sel_bg = rgb(th.select(), th);
    let cur_cl = rgb(th.cursor(), th);
    let chk_bg = danger(th);

    let tty = || TTY_MODE.load(Ordering::Relaxed) && matches!(th, Theme::MatteBlack | Theme::Mocha);

    let lm_color = |is_light: bool| {
        if tty() { return Color::Indexed(11); } // BrightYellow last-move
        let sq = if is_light { th.squares().0 } else { th.squares().1 };
        let lm = th.last_move();
        Color::Rgb(
            (((sq.0 as u16)+lm.0 as u16*2)/3) as u8,
            (((sq.1 as u16)+lm.1 as u16*2)/3) as u8,
            (((sq.2 as u16)+lm.2 as u16*2)/3) as u8,
        )
    };
    let arrow_color = |is_light: bool| {
        let sq = if is_light { th.squares().0 } else { th.squares().1 };
        let ac = th.accent();
        Color::Rgb(
            (((sq.0 as u16)+ac.0 as u16*2)/3) as u8,
            (((sq.1 as u16)+ac.1 as u16*2)/3) as u8,
            (((sq.2 as u16)+ac.2 as u16*2)/3) as u8,
        )
    };
    // highlight_brightness: 0=square barely changes, 10=vivid neon
    // Works by blending between the natural square color and a vivid target color
    let brightness = app.cfg.highlight_brightness;
    let blend_color = |base: Color, target: (u8,u8,u8)| -> Color {
        if let Color::Rgb(br, bg, bb) = base {
            let t = brightness as u16; // 0..10
            let r = ((br as u16 * (10-t) + target.0 as u16 * t) / 10) as u8;
            let g = ((bg as u16 * (10-t) + target.1 as u16 * t) / 10) as u8;
            let b = ((bb as u16 * (10-t) + target.2 as u16 * t) / 10) as u8;
            Color::Rgb(r, g, b)
        } else { base }
    };
    // Hint dot fg: starts near square color at 0, vivid at 10
    let to_tuple = |c: Color| -> (u8,u8,u8) {
        match c {
            Color::Rgb(r,g,b) => (r,g,b),
            Color::Indexed(i) => match i {
                0 => (0,0,0), 1 => (128,0,0), 2 => (0,128,0), 3 => (128,128,0),
                4 => (0,0,128),5 => (128,0,128),6 => (0,128,128),7 => (192,192,192),
                8 => (128,128,128),9 => (255,0,0),10=>(0,255,0),11=>(255,255,0),
                12=>(0,0,255),13=>(255,0,255),14=>(0,255,255),15=>(255,255,255),
                _ => (128,128,128),
            },
            _ => (0,0,0),
        }
    };
    let hint_fg = |is_light: bool| {
        blend_color(if is_light { sq_l(th) } else { sq_d(th) }, to_tuple(dim(th)))
    };
    let dot_tint = |is_light: bool| {
        blend_color(if is_light { sq_l(th) } else { sq_d(th) }, to_tuple(warning(th)))
    };
    let hl_color = |is_light: bool| {
        blend_color(if is_light { sq_l(th) } else { sq_d(th) }, to_tuple(success(th)))
    };

    let mut lines: Vec<Line> = vec![];
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for label in &coord_labels { spans.push(Span::styled(label.clone(), sty(dim(th)))); }
        lines.push(Line::from(spans));
    }

    for &r in &row_ord {
        for line_idx in 0..CELL_H {
            let is_mid = line_idx == mid;
            let mut spans: Vec<Span> = vec![];
            if app.cfg.show_coords {
                let coord = if is_mid { format!(" {} ", 8 - r) } else { "   ".to_string() };
                spans.push(Span::styled(coord, sty(dim(th))));
            }
            for &c in &col_ord {
                let is_light = (r+c)%2==0;
                let piece    = app.gs.board[r][c];
                let is_sel   = app.selected==Some((r,c));
                let is_tgt   = tgts.contains(&(r,c));
                let is_kchk  = king_chk==Some((r,c));
                let is_cur   = cur_bc==(r,c);
                let is_last  = !is_sel&&!is_kchk&&last.map(|h|h.from==(r,c)||h.to==(r,c)).unwrap_or(false);
                let is_arrow = (arrow_src.contains(&(r,c)) || arrow_tgt.contains(&(r,c))) && !is_last && !is_sel && !is_kchk && !is_tgt;

                let bg = if is_sel       { sel_bg }
                    else if is_kchk      { chk_bg }
                    else if is_tgt && matches!(app.cfg.move_hints,MoveHints::Highlight) { hl_color(is_light) }
                // Dots mode: tint the square background slightly so dot stands out
                else if is_tgt && matches!(app.cfg.move_hints,MoveHints::Dots) {
                    dot_tint(is_light)
                }
                    else if is_last      { lm_color(is_light) }
                    else if is_arrow     { arrow_color(is_light) }
                    else if is_light     { sq_l(th) }
                    else                 { sq_d(th) };

                let use_blocks = app.cfg.piece_style == PieceStyle::Ascii;

                let is_white = piece.map(|p| p.c == PC::White).unwrap_or(false);
                let bold = if is_white { Modifier::BOLD } else { Modifier::empty() };

                let (cell_str, style) = if use_blocks {
                    let (row_str, fg) = if is_cur && (line_idx == 0 || line_idx == CELL_H - 1) {
                        (center_str("─", CELL_W), cur_cl)
                    } else if is_tgt && piece.is_some() && matches!(app.cfg.move_hints,MoveHints::Dots) && line_idx == 0 {
                        (center_str("○", CELL_W), hint_fg(is_light))
                    } else if let Some(p) = piece {
                        let lines = crate::pieces::render_star_lines(p, CELL_W, CELL_H);
                        let row_str = lines.get(line_idx).cloned().unwrap_or_else(|| blank_line.clone());
                        let fg  = if p.c == PC::White { pw(th) } else { pb_c(th) };
                        (row_str, fg)
                    } else if is_tgt && matches!(app.cfg.move_hints, MoveHints::Dots) && line_idx == mid {
                        (center_str("⬤", CELL_W), hint_fg(is_light))
                    } else {
                        (blank_line.clone(), bg)
                    };
                    (row_str, Style::default().bg(bg).fg(fg).add_modifier(bold))
                } else {
                    // Classic: piece symbol on middle line, decorative bars for Unicode
                    let (s, st) = if is_cur && line_idx == 0 {
                        let bar = &bar_line;
                        (format!(" {} ", bar), Style::default().bg(bg).fg(cur_cl))
                    } else if is_tgt && piece.is_some() && matches!(app.cfg.move_hints,MoveHints::Dots) && line_idx == 0 {
                        (center_str("○", CELL_W), Style::default().bg(bg).fg(hint_fg(is_light)))
                    } else if let Some(p) = piece {
                        let sym = piece_sym(p);
                        let pc = if p.c == PC::White { pw(th) } else { pb_c(th) };
                        let is_unicode = matches!(app.cfg.piece_style, PieceStyle::Unicode);
                        if is_mid {
                            (center_str(&sym, CELL_W), Style::default().bg(bg).fg(pc).add_modifier(bold))
                        } else if is_unicode && CELL_H >= 4 && (line_idx == mid - 1 || line_idx == mid + 1) {
                            let bar = &bar_line;
                            (format!(" {} ", bar), Style::default().bg(bg).fg(pc).add_modifier(Modifier::DIM))
                        } else if line_idx == CELL_H - 1 {
                            let bcl = if is_cur { cur_cl } else { pc };
                            (tgt_underline.clone(), Style::default().bg(bg).fg(bcl).add_modifier(if is_cur { Modifier::BOLD } else { Modifier::DIM }))
                        } else {
                            (blank_line.clone(), Style::default().bg(bg))
                        }
                    } else if is_cur && line_idx == CELL_H - 1 {
                        let bar = &bar_line;
                        (format!(" {} ", bar), Style::default().bg(bg).fg(cur_cl))
                    } else if is_tgt && matches!(app.cfg.move_hints,MoveHints::Dots) && is_mid {
                        (center_str("⬤", CELL_W), Style::default().bg(bg).fg(hint_fg(is_light)))
                    } else if line_idx==CELL_H-1 && is_tgt {
                        let st = if is_cur { Style::default().bg(bg).fg(cur_cl) } else { Style::default().bg(bg) };
                        (tgt_underline.clone(), st)
                    } else {
                        (blank_line.clone(), Style::default().bg(bg))
                    };
                    (s, st)
                };
                spans.push(Span::styled(cell_str, style));
            }
            if app.cfg.show_coords {
                spans.push(Span::styled(if is_mid{format!(" {} ",8-r)}else{"   ".to_string()}, sty(dim(th))));
            }
            lines.push(Line::from(spans));
        }
    }

    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for &c in &col_ord { spans.push(Span::styled(format!("{:^width$}",(b'a'+c as u8) as char,width=CELL_W), sty(dim(th)))); }
        lines.push(Line::from(spans));
    }

    let board_title = if app.screen == Screen::OnlineGame {
        format!(" BOARD · {} ", trunc(&app.online_status, 42))
    } else {
        " BOARD ".to_string()
    };
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(ac(th))).style(Style::default().bg(bg0(th)))
                .title(Span::styled(board_title, sty(ac(th)).add_modifier(Modifier::BOLD)))
        ),
        area,
    );
}

fn piece_sym(p: crate::engine::Piece) -> String {
    p.sym().to_string()
}

fn center_str(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len >= w { return s.chars().take(w).collect(); }
    let pad = w - len;
    let left = pad / 2;
    let right = pad - left;
    let mut out = String::with_capacity(w);
    for _ in 0..left { out.push(' '); }
    out.push_str(s);
    for _ in 0..right { out.push(' '); }
    out
}

// ── INPUT ─────────────────────────────────────────────────────────────────────
fn draw_input(app: &App, f: &mut Frame, area: Rect) {
    let th = app.cfg.theme;
    let cur_bc   = app.to_board(app.cursor);
    let cur_name = format!("{}{}",(b'a'+cur_bc.1 as u8) as char, 8-cur_bc.0);
    let sel_str  = if let Some(sel)=app.selected {
        format!("{}{}→ ", (b'a'+sel.1 as u8) as char, 8-sel.0)
    } else { String::new() };
    let dm_hint  = if let Some(msg)=&app.draw_offer_msg { format!("  {}", msg) } else { String::new() };

    let (input_line, input_col) = if !app.input_buf.is_empty() {
        (format!(" [{}] {}{}█", cur_name, sel_str, app.input_buf), cream())
    } else if app.selected.is_some() {
        (format!(" [{}] {}target + Enter", cur_name, sel_str), info(th))
    } else {
        (format!(" [{}] type move or navigate{}", cur_name, dm_hint), dim(th))
    };

    let is_online = matches!(app.screen, Screen::OnlineGame);
    let (hint_line, hint_col) = if let Some(err)=&app.input_err {
        (format!(" ✗ {}", err), danger(th))
    } else if !app.input_buf.is_empty() {
        (" Enter=exec  Esc=cancel  Del=pop".to_string(), dimmer())
    } else if is_online {
        let keys = if app.online_result.is_some() {
            " q=lobby  r=analyze  E=PNG  c=chat".to_string()
        } else {
            format!(" d=draw  D=decline  c=chat  t=takeback  v=claim  x=resign  q=lobby  ({})",
                if app.gs.turn == app.player_color { "your move" } else { "opponent" })
        };
        (keys, dimmer())
    } else {
        (" arrows  e=hints  u=undo  r=redo  G=PGN  F=FEN  ?=help  q=menu".to_string(), dimmer())
    };

    let border_col = if !app.input_buf.is_empty(){ac(th)} else if app.input_err.is_some(){danger(th)} else {dim(th)};
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(input_line, sty(input_col).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled(hint_line, sty(hint_col))]),
        ]).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(border_col)).style(Style::default().bg(bg1(th)))
        ),
        area,
    );
}

// ── SIDEBAR ───────────────────────────────────────────────────────────────────
fn panel_block(title: &'static str, th: Theme) -> Block<'static> {
    Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
        .title(Span::styled(format!(" {} ", title), sty(ac(th))))
}

fn divider_line(label: &str, th: Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(" ─ ", sty(dimmer())),
        Span::styled(label.to_string(), sty(dim(th))),
        Span::styled(" ─", sty(dimmer())),
    ])
}

fn draw_sidebar(app: &App, f: &mut Frame, area: Rect) {
    let th = app.cfg.theme;
    let mut lines: Vec<Line> = vec![];

    // ── PLAYERS section ──
    let iw  = app.gs.turn == PC::White;
    let ib  = app.gs.turn == PC::Black;
    let is_online = matches!(app.screen, Screen::OnlineGame | Screen::OnlineLobby);
    let w_tag = if is_online {
        if app.player_color == PC::White { format!("[{}]", app.player_name) } else { app.lichess_opponent.as_ref().map(|o| format!("[{o}]")).unwrap_or_default() }
    } else { match app.mode { Mode::Cpu if app.player_color==PC::White=>"[YOU]".into(), Mode::Cpu=>"[CPU]".into(), _=>String::new() } };
    let b_tag = if is_online {
        if app.player_color == PC::Black { format!("[{}]", app.player_name) } else { app.lichess_opponent.as_ref().map(|o| format!("[{o}]")).unwrap_or_default() }
    } else { match app.mode { Mode::Cpu if app.player_color==PC::Black=>"[YOU]".into(), Mode::Cpu=>"[CPU]".into(), _=>String::new() } };
    let piece_val = |k: Kind| -> i32 {
        match k { Kind::P=>1, Kind::N=>3, Kind::B=>3, Kind::R=>5, Kind::Q=>9, Kind::K=>0 }
    };
    let mat_w: i32 = app.gs.cap_w.iter().map(|p| piece_val(p.k)).sum();
    let mat_b: i32 = app.gs.cap_b.iter().map(|p| piece_val(p.k)).sum();
    let adv_str = |adv: i32| -> String { if adv > 0 { format!("+{}", adv) } else { String::new() } };
    let clock_col = |ms_opt: Option<u64>, active: bool| -> Color {
        match ms_opt {
            None    => if active { info(th) } else { dimmer() },
            Some(m) if m<30_000 => danger(th),
            Some(_) if active   => info(th),
            _                   => dimmer(),
        }
    };
    let flagged = app.gs.clock_state == ClockState::Flagged;
    let wtime   = Gs::fmt_time(app.gs.white_ms);
    let btime   = Gs::fmt_time(app.gs.black_ms);
    let wcol    = if flagged&&!iw{danger(th)}else{clock_col(app.gs.white_ms,iw)};
    let bcol    = if flagged&& iw{danger(th)}else{clock_col(app.gs.black_ms,ib)};
    let has_clk = app.gs.white_ms.is_some();
    let cap_w = sorted_captures(&app.gs.cap_w);
    let cap_b = sorted_captures(&app.gs.cap_b);

    lines.push(Line::from(vec![
        Span::styled(if iw{"▶"}else{" "}, sty(ac(th))),
        Span::styled(" ♔ WHITE ", sty(pw(th)).add_modifier(Modifier::BOLD)),
        Span::styled(w_tag, sty(dimmer())),
        Span::styled(format!(" {:<3}", adv_str(mat_b - mat_w)), sty(success(th)).add_modifier(Modifier::BOLD)),
        Span::styled(if has_clk{wtime.trim().to_string()}else{String::new()},
            Style::default().fg(wcol).add_modifier(if iw&&has_clk{Modifier::BOLD}else{Modifier::empty()})),
    ]));
    lines.push(Line::from(vec![Span::raw(" "), Span::styled(&cap_b, sty(dim(th)))]));
    lines.push(Line::from(vec![
        Span::styled(if ib{"▶"}else{" "}, sty(ac(th))),
        Span::styled(" ♚ BLACK ", sty(dim(th)).add_modifier(Modifier::BOLD)),
        Span::styled(b_tag, sty(dimmer())),
        Span::styled(format!(" {:<3}", adv_str(mat_w - mat_b)), sty(success(th)).add_modifier(Modifier::BOLD)),
        Span::styled(if has_clk{btime.trim().to_string()}else{String::new()},
            Style::default().fg(bcol).add_modifier(if ib&&has_clk{Modifier::BOLD}else{Modifier::empty()})),
    ]));
    lines.push(Line::from(vec![Span::raw(" "), Span::styled(&cap_w, sty(dim(th)))]));

    // ── STATUS section ──
    lines.push(divider_line("status", th));
    let (s, col) = status_str(app, th);
    let clock   = if app.cfg.show_clock { format!(" Move {}  ", app.gs.fullmove) } else { String::new() };
    let last_mv = app.gs.history.last().map(|h| {
        let who = if h.color==PC::White{"W"}else{"B"};
        format!("  last: [{}] {}", who, h.san)
    }).unwrap_or_default();
    lines.push(Line::from(vec![Span::raw(" "), Span::styled(&s, sty(col))]));
    lines.push(Line::from(vec![Span::styled(format!("{}{}", clock, last_mv), sty(dim(th)))]));

    // ── EVAL/ANALYSIS section ──
    let (rev_title, rev_lines) = match app.cfg.ui_mode {
        UiMode::Standard => {
            let busy  = if app.engine_busy { "~" } else { "+" };
            let eng   = trunc(&app.analysis.engine_name, 14);
            let title = format!(" eval {} [{}] ", busy, eng);
            let an_lines: Vec<Line> = if let Some(rev) = app.move_reviews.last() {
                let lc = label_color(rev.label, th);
                vec![
                    Line::from(vec![
                        Span::styled(format!(" [{}]", rev.label.icon()), sty(lc)),
                        Span::styled(format!(" {:.2}p", rev.delta_cp as f32 / 100.0), sty(lc)),
                    ]),
                    Line::from(vec![
                        Span::styled("  Best: ", sty(dimmer())),
                        Span::styled(trunc(rev.best_move.as_deref().unwrap_or("--"), 8), sty(dim(th))),
                        Span::styled(format!(" {:.1}", rev.eval_after / 100.0), sty(dim(th))),
                    ]),
                ]
            } else if app.thinking {
                vec![Line::from(vec![Span::styled("  analysing...", sty(dimmer()))])]
            } else {
                vec![Line::from(vec![Span::styled("  make a move to analyse", sty(dimmer()))])]
            };
            (title, an_lines)
        }
        UiMode::Analysis => {
            let busy  = if app.engine_busy { "~" } else { "+" };
            let eng   = trunc(&app.analysis.engine_name, 14);
            let title = format!(" analysis {} [{}] ", busy, eng);
            let an_lines: Vec<Line> = if let Some(rev) = app.move_reviews.last() {
                let lc = label_color(rev.label, th);
                vec![
                    Line::from(vec![Span::styled(format!(" Move: {}", rev.san), sty(cream()).add_modifier(Modifier::BOLD))]),
                    Line::from(vec![
                        Span::styled(" Label: ", sty(dimmer())),
                        Span::styled(format!("[{}] {}", rev.label.icon(), rev.label.name()), sty(lc)),
                    ]),
                    Line::from(vec![
                        Span::styled(" Delta: ", sty(dimmer())),
                        Span::styled(format!("{:.2}p", rev.delta_cp as f32 / 100.0),
                            sty(if rev.delta_cp < -300 { danger(th) } else if rev.delta_cp < -100 { warning(th) } else { success(th) })),
                    ]),
                    Line::from(vec![
                        Span::styled(" Eval: ", sty(dimmer())),
                        Span::styled(format!("{:.1} → {:.1}", rev.eval_before / 100.0, rev.eval_after / 100.0), sty(dim(th))),
                    ]),
                    Line::from(vec![
                        Span::styled(" Best: ", sty(dimmer())),
                        Span::styled(trunc(rev.best_move.as_deref().unwrap_or("--"), 8), sty(info(th))),
                        Span::styled(format!(" {:.1}", rev.eval_after / 100.0), sty(dim(th))),
                    ]),
                ]
            } else if app.thinking {
                vec![Line::from(vec![Span::styled("  analysing...", sty(dimmer()))])]
            } else {
                vec![Line::from(vec![Span::styled("  make a move to analyse", sty(dimmer()))])]
            };
            (title, an_lines)
        }
        UiMode::Minimal => (String::new(), vec![]),
    };
    if !rev_lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(" ─ ", sty(dimmer())),
            Span::styled(rev_title.trim().to_string(), sty(dim(th))),
            Span::styled(" ─", sty(dimmer())),
        ]));
        lines.extend(rev_lines);
    }

    // ── HISTORY section ──
    lines.push(divider_line("moves", th));
    let h   = &app.gs.history;
    let mut pairs: Vec<MovePair> = vec![];
    let mut i = 0; let mut n = 1;
    let last_idx = h.len().saturating_sub(1);
    while i < h.len() {
        let w_last  = i == last_idx;
        let b_last  = (i+1) == last_idx;
        let wlabel  = app.move_reviews.get(i).map(|r| r.label);
        let blabel  = app.move_reviews.get(i+1).map(|r| r.label);
        let w = h[i].san.clone();
        let b = h.get(i+1).map(|e| e.san.clone()).unwrap_or_default();
        pairs.push((n, w, b, w_last, b_last, wlabel, blabel));
        i += 2; n += 1;
    }
    let cap = area.height.saturating_sub(3) as usize;
    let start = pairs.len().saturating_sub(cap);
    if pairs.is_empty() {
        lines.push(Line::from(vec![Span::styled("  — no moves yet —", sty(dimmer()))]));
    } else {
        for (n, w, b, wl, bl, wlab, blab) in &pairs[start..] {
            let w_sty = if *wl {
                Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD)
            } else { sty(cream()).add_modifier(Modifier::BOLD) };
            let b_sty = if *bl {
                Style::default().fg(bg0(th)).bg(info(th)).add_modifier(Modifier::BOLD)
            } else { sty(dim(th)) };
            let wlc = wlab.map(|l| label_color(l, th)).unwrap_or(dimmer());
            let blc = blab.map(|l| label_color(l, th)).unwrap_or(dimmer());
            let wi  = wlab.map(|l| l.icon()).unwrap_or("");
            let bi  = blab.map(|l| l.icon()).unwrap_or("");
            lines.push(Line::from(vec![
                Span::styled(format!("{:>3}. ", n), sty(dimmer())),
                Span::styled(format!("{:<7}", w), w_sty),
                Span::styled(format!("{:<2}", wi), sty(wlc)),
                Span::raw(" "),
                Span::styled(format!("{:<7}", b), b_sty),
                Span::styled(format!("{:<2}", bi), sty(blc)),
            ]));
        }
    }

    // ── KEYS section ──
    lines.push(divider_line("keys", th));
    let mode_label = match app.cfg.ui_mode {
        UiMode::Minimal  => "min",
        UiMode::Standard => "std",
        UiMode::Analysis => "ana",
    };
    lines.push(Line::from(vec![Span::styled(" ↑↓←→/hjkl cursor   Enter/click move", sty(dim(th)))]));
    lines.push(Line::from(vec![
        Span::styled(" E", sty(success(th)).add_modifier(Modifier::BOLD)),
        Span::styled(":PNG", sty(dim(th))),
        Span::styled(" G", sty(info(th)).add_modifier(Modifier::BOLD)),
        Span::styled(":PGN", sty(dim(th))),
        Span::styled(" T", sty(warning(th)).add_modifier(Modifier::BOLD)),
        Span::styled(format!(":{}", mode_label), sty(dim(th))),
        Span::styled(" d", sty(dim(th))),
        Span::styled(":draw", sty(dimmer())),
        Span::styled(" u", sty(dim(th))),
        Span::styled(":undo", sty(dimmer())),
        Span::styled(" r", sty(dim(th))),
        Span::styled(":redo", sty(dimmer())),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" n", sty(dim(th))),
        Span::styled(":new  ", sty(dimmer())),
        Span::styled(" e", sty(dim(th))),
        Span::styled(":hints  ", sty(dimmer())),
        Span::styled(" s", sty(dim(th))),
        Span::styled(":settings  ", sty(dimmer())),
        Span::styled(" q", sty(dim(th))),
        Span::styled(":menu  ", sty(dimmer())),
        Span::styled(" type", sty(dim(th))),
        Span::styled(":notation", sty(dimmer())),
    ]));

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
                .title(Span::styled(" SIDEBAR ", sty(ac(th))))
        ),
        area,
    );
}

fn sorted_captures(captured: &[Piece]) -> String {
    let order = [Kind::Q, Kind::R, Kind::B, Kind::N, Kind::P];
    let mut counts: [usize; 5] = [0; 5];
    for p in captured {
        for (i, &k) in order.iter().enumerate() {
            if p.k == k { counts[i] += 1; break; }
        }
    }
    let mut out = String::new();
    for (i, &k) in order.iter().enumerate() {
        let sym = Piece { k, c: PC::White }.sym();
        for _ in 0..counts[i] { out.push_str(sym); }
    }
    if out.is_empty() { "  —".to_string() } else { format!(" {}", out) }
}

// (dead panel functions removed — sidebar is now a single block with dividers)

// ── GAME OVER OVERLAY ─────────────────────────────────────────────────────────
fn draw_gameover(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(52, 16, f.area());
    f.render_widget(Clear, rect);

    let flagged = app.gs.clock_state == ClockState::Flagged;
    let is_draw = app.game_end == GameEnd::Draw;

    let (title, title_col, body) = if flagged {
        let loser = app.gs.turn.name(); let winner = app.gs.turn.opp().name();
        ("  FLAG FALL".to_string(), danger(th),
         vec![format!("{} ran out of time.", loser), format!("{} wins.", winner)])
    } else if is_draw {
        ("  DRAW".to_string(), warning(th),
         vec!["Game drawn by agreement.".to_string()])
    } else {
        match app.gs.status {
            Status::Checkmate => {
                let w=app.gs.turn.opp().name(); let l=app.gs.turn.name();
                ("  CHECKMATE".to_string(), warning(th),
                 vec![format!("{} wins!", w), format!("{} king is mated.", l)])
            }
            Status::Stalemate => (
                "  STALEMATE".to_string(), warning(th),
                vec!["Game drawn.".to_string(), "No legal moves.".to_string()]),
            _ => return,
        }
    };

    let total  = app.gs.history.len().div_ceil(2);

    let fen = app.current_fen();
    let fen_display = if fen.len() > 46 { format!("{}…", &fen[..45]) } else { fen };

    let piece_val = |k: Kind| -> i32 {
        match k { Kind::P=>1, Kind::N=>3, Kind::B=>3, Kind::R=>5, Kind::Q=>9, Kind::K=>0 }
    };
    let mat_w: i32 = app.gs.cap_w.iter().map(|p| piece_val(p.k)).sum();
    let mat_b: i32 = app.gs.cap_b.iter().map(|p| piece_val(p.k)).sum();
    let diff = (mat_w as i16 - mat_b as i16).abs() as i32;
    let mat_line = if diff > 0 {
        let side = if mat_w > mat_b { "White" } else { "Black" };
        format!("Material: {} up +{}", side, diff)
    } else {
        "Material: equal".to_string()
    };

    let time_line = match (app.gs.white_ms, app.gs.black_ms) {
        (Some(w), Some(b)) => {
            let total_ms = w + b;
            let secs = total_ms / 1000;
            let mins = secs / 60; let secs = secs % 60;
            format!("Time: {}m {:02}s", mins, secs)
        }
        _ => String::new(),
    };

    let mut lines = vec![Line::from("")];
    for b in &body {
        lines.push(Line::from(vec![Span::styled(b, sty(cream()))]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(format!(" {} moves", total), sty(dim(th)))]));
    lines.push(Line::from(vec![Span::styled(format!(" {}", mat_line), sty(dim(th)))]));
    if !time_line.is_empty() {
        lines.push(Line::from(vec![ Span::styled(format!(" {}", time_line), sty(dim(th)))]));
    }
    lines.push(Line::from(vec![Span::styled(format!(" {}", fen_display), sty(dimmer()))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" n:new", sty(ac(th)).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("r:analyze", sty(info(th)).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("E:PNG", sty(success(th)).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("G:PGN", sty(info(th)).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("F:FEN", sty(dim(th)).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("u:undo", sty(dim(th))),
        Span::raw("  "),
        Span::styled("q:menu", sty(dim(th))),
    ]));
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(title_col)).style(Style::default().bg(bg1(th)))
                .title(Span::styled(&title, sty(title_col).add_modifier(Modifier::BOLD)))
                .padding(Padding::new(2, 2, 1, 1)),
        ),
        rect,
    );
}

// ── PNG PREVIEW OVERLAY ───────────────────────────────────────────────────────
fn draw_png_preview(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    // Popup: 56 wide, 24 tall
    let rect = center(56, 24, f.area());
    f.render_widget(Clear, rect);
    let board   = &app.gs.board;
    let flipped = app.flipped();
    let row_ord: [usize; 8] = if flipped { [7,6,5,4,3,2,1,0] } else { [0,1,2,3,4,5,6,7] };
    let col_ord: [usize; 8] = if flipped { [7,6,5,4,3,2,1,0] } else { [0,1,2,3,4,5,6,7] };

    // Save path display
    let path_str = app.png_preview_path.as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    // Shorten ~/... for display
    let home = std::env::var("HOME").unwrap_or_default();
    let path_display = if path_str.starts_with(&home) {
        format!("~{}", &path_str[home.len()..])
    } else {
        path_str.clone()
    };

    let mut lines: Vec<Line> = vec![Line::from("")];

    // ── Mini board (compact: 2 chars per square + rank label) ─────────────
    // File label row
    let mut file_row: Vec<Span> = vec![Span::styled("   ", sty(dimmer()))];
    for &c in &col_ord {
        file_row.push(Span::styled(
            format!("{} ", (b'a'+c as u8) as char),
            sty(dim(th)),
        ));
    }
    lines.push(Line::from(file_row));

    // Board rows
    for &r in &row_ord {
        let mut row_spans = vec![
            Span::styled(format!(" {} ", 8-r), sty(dim(th))),
        ];
        for &c in &col_ord {
            let is_light = (r+c)%2 == 0;
            let bg_c     = if is_light { sq_l(th) } else { sq_d(th) };
            let (txt, fg) = match board[r][c] {
                None    => ("  ".to_string(), dimmer()),
                Some(p) => {
                    let fg = if p.c==PC::White { pw(th) } else { pb_c(th) };
                    (format!("{} ", p.sym()), fg)
                }
            };
            row_spans.push(Span::styled(txt,
                Style::default().fg(fg).bg(bg_c).add_modifier(Modifier::BOLD)));
        }
        lines.push(Line::from(row_spans));
    }

    // File label row (bottom)
    let mut file_row2: Vec<Span> = vec![Span::styled("   ", sty(dimmer()))];
    for &c in &col_ord {
        file_row2.push(Span::styled(
            format!("{} ", (b'a'+c as u8) as char),
            sty(dim(th)),
        ));
    }
    lines.push(Line::from(file_row2));

    // Save path
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  Save to:", sty(dim(th)))]));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", path_display),
        sty(success(th)).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        format!("  {:─<48}",""),
        sty(dimmer()),
    )]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Y / Enter  ", sty(success(th)).add_modifier(Modifier::BOLD)),
        Span::styled("save PNG", sty(cream()).add_modifier(Modifier::BOLD)),
        Span::raw("          "),
        Span::styled("N / Esc  ", sty(danger(th))),
        Span::styled("cancel", sty(dim(th))),
    ]));

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(success(th))).style(Style::default().bg(bg1(th)))
                .title(Span::styled(
                    format!(" 📸  EXPORT BOARD AS PNG  — v{} ", VERSION),
                    sty(success(th)).add_modifier(Modifier::BOLD),
                ))
                .padding(Padding::new(2, 2, 1, 1)),
        ),
        rect,
    );
}

// ── DRAW OFFER OVERLAY ────────────────────────────────────────────────────────
fn draw_draw_offer(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(46, 11, f.area());
    f.render_widget(Clear, rect);
    let offerer   = app.draw_offer.map(|c| c.name()).unwrap_or("Unknown");
    let responder = app.draw_offer.map(|c| c.opp().name()).unwrap_or("Unknown");
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(format!("  {} offers a draw.", offerer), sty(cream()).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled(format!("  {}, do you accept?", responder), sty(cream()))]),
            Line::from(""),
            Line::from(vec![Span::styled(format!("  {:─<38}",""), sty(dimmer()))]),
            Line::from(""),
            Line::from(vec![Span::styled("  Y / Enter  accept draw", sty(info(th)).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("  N / Esc    decline & keep playing", sty(danger(th)))]),
        ]).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(warning(th))).style(Style::default().bg(bg1(th)))
                .title(Span::styled("  ½  DRAW OFFER  ½", sty(warning(th)).add_modifier(Modifier::BOLD)))
                .padding(Padding::new(2, 2, 1, 1)),
        ),
        rect,
    );
}

// ── ANALYZE ───────────────────────────────────────────────────────────────────
fn draw_analyze(app: &App, f: &mut Frame) {
    let th = app.cfg.theme;
    let a  = f.area();
    let cw = app.cell_w;
    f.render_widget(Block::default().style(Style::default().bg(bg0(th))), a);

    let bw = (3 + 8*cw + 3 + 2).min(a.width);
    let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(a);
    let [board_col, side_col] = Layout::horizontal([Constraint::Length(bw), Constraint::Fill(1)]).areas(body);

    let snap  = app.replay_snaps.get(app.replay_idx);
    let title = format!(" ♟ RChess — ANALYZE  [{}/{}]  {}",
        app.replay_idx, app.replay_snaps.len().saturating_sub(1), app.replay_result);
    f.render_widget(Paragraph::new(title).style(Style::default().fg(info(th)).bg(bg1(th))), top);

    let [eval_bar_col, actual_board_col] = Layout::horizontal([
        Constraint::Length(4),
        Constraint::Fill(1),
    ]).areas(board_col);

    if let Some(snap) = snap { draw_analyze_board(app, f, actual_board_col, snap); }

    let cur_eval = if app.replay_idx >= app.eval_history.len()
        && app.replay_idx == app.replay_snaps.len().saturating_sub(1)
    {
        // Terminal position with no engine eval — use game result for full fill
        match app.replay_result.as_str() {
            "1-0" => 10000,
            "0-1" => -10000,
            _ => 0,
        }
    } else {
        app.eval_history.get(app.replay_idx).copied().unwrap_or(0)
    };
    draw_eval_bar(cur_eval, app.engine_busy, f, eval_bar_col);

    draw_analyze_sidebar(app, f, side_col);
}

fn draw_analyze_board(app: &App, f: &mut Frame, area: Rect, snap: &ReplaySnap) {
    #[allow(non_snake_case)]
    let CELL_W = app.cell_w as usize;
    #[allow(non_snake_case)]
    let CELL_H = app.cell_h as usize;
    let th      = app.cfg.theme;
    let flipped = app.flipped();
    let row_ord: [usize; 8] = if flipped { [7,6,5,4,3,2,1,0] } else { [0,1,2,3,4,5,6,7] };
    let col_ord: [usize; 8] = if flipped { [7,6,5,4,3,2,1,0] } else { [0,1,2,3,4,5,6,7] };

    let use_blocks = app.cfg.piece_style == PieceStyle::Ascii;
    let blank_line: String = " ".repeat(CELL_W);
    let mid = CELL_H / 2;
    let coord_labels: Vec<String> = col_ord.iter().map(|&c| format!("{:^width$}", (b'a' + c as u8) as char, width = CELL_W)).collect();
    let bar_line: String = "─".repeat(CELL_W.saturating_sub(2));

    let mut lines: Vec<Line> = vec![];
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for label in &coord_labels { spans.push(Span::styled(label.clone(), sty(dim(th)))); }
        lines.push(Line::from(spans));
    }
    for &r in &row_ord {
        for line_idx in 0..CELL_H {
            let is_mid = line_idx == mid;
            let mut spans: Vec<Span> = vec![];
            if app.cfg.show_coords { spans.push(Span::styled(if is_mid{format!(" {} ",8-r)}else{"   ".to_string()}, sty(dim(th)))); }
            for &c in &col_ord {
                let piece    = snap.board[r][c];
                let is_light = (r+c)%2==0;
                let bg = if is_light { sq_l(th) } else { sq_d(th) };
                let (cell_str, style) = if let Some(p) = piece {
                    let pc = if p.c == PC::White { pw(th) } else { pb_c(th) };
                    if use_blocks {
                        let blines = crate::pieces::render_star_lines(p, CELL_W, CELL_H);
                        let row_str = blines.get(line_idx).cloned().unwrap_or_else(|| blank_line.clone());
                        (row_str, Style::default().bg(bg).fg(pc).add_modifier(Modifier::BOLD))
                    } else if is_mid {
                        (center_str(&piece_sym(p),CELL_W), Style::default().bg(bg).fg(pc).add_modifier(Modifier::BOLD))
                    } else if matches!(app.cfg.piece_style, PieceStyle::Unicode) && CELL_H >= 4 && (line_idx == mid - 1 || line_idx == mid + 1) {
                        (format!(" {} ", bar_line), Style::default().bg(bg).fg(pc).add_modifier(Modifier::DIM))
                    } else {
                        (blank_line.clone(), Style::default().bg(bg))
                    }
                } else {
                    (blank_line.clone(), Style::default().bg(bg))
                };
                spans.push(Span::styled(cell_str, style));
            }
            if app.cfg.show_coords { spans.push(Span::styled(if is_mid{format!(" {} ",8-r)}else{"   ".to_string()}, sty(dim(th)))); }
            lines.push(Line::from(spans));
        }
    }
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for label in &coord_labels { spans.push(Span::styled(label.clone(), sty(dim(th)))); }
        lines.push(Line::from(spans));
    }
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(info(th))).style(Style::default().bg(bg0(th)))
                .title(Span::styled(" REPLAY ", sty(info(th)).add_modifier(Modifier::BOLD)))
        ),
        area,
    );
}


/// Render a compact ASCII sparkline of eval history.
/// eval_history[0] = start (0), eval_history[N] = after move N.
/// Width = number of chars to use. Always B&W grayscale.
fn eval_sparkline(eval_history: &[i32], width: usize) -> (String, Vec<Color>) {
    if eval_history.len() < 2 || width < 3 {
        return ("no data".to_string(), vec![]);
    }
    let n = eval_history.len();
    let step = if n <= width { 1 } else { n / width };
    let samples: Vec<i32> = (0..n).step_by(step.max(1)).map(|i| eval_history[i]).collect();
    let samples = &samples[..samples.len().min(width)];

    let max_abs = samples.iter().map(|v| v.abs()).max().unwrap_or(1).max(50) as f32;
    let bars = ["▁","▃","▅","▇","█"];

    let mut out = String::new();
    let mut colors = vec![];
    for &ev in samples {
        let norm = (ev as f32 / max_abs).clamp(-1.0, 1.0);
        let idx = ((norm + 1.0) / 2.0 * 4.0).round() as usize;
        let idx = idx.min(4);
        out.push_str(bars[idx]);
        let col = if ev > 30 { Color::Indexed(15) } else if ev < -30 { Color::Indexed(8) } else { Color::Indexed(7) };
        colors.push(col);
    }
    (out, colors)
}


// ── VERTICAL EVAL BAR ──────────────────────────────────────────────────────
// Shows who is better — White-positive centipawns.
// Bar fills completely toward the winning side (no minimum row constraint).
// Always B&W grayscale, independent of theme.
fn draw_eval_bar(eval_cp: i32, busy: bool, f: &mut Frame, area: Rect) {
    let total = area.height as i32;
    if total < 4 { return; }

    let is_mate = eval_cp.abs() >= 10_000;

    // Full fill toward winning side — no minimum row constraint
    let (white_rows, black_rows) = if is_mate {
        if eval_cp > 0 { (total, 0) } else { (0, total) }
    } else {
        let clamped = (eval_cp.clamp(-1000, 1000)) as f32 / 100.0;
        let frac    = ((clamped + 10.0) / 20.0).clamp(0.0, 1.0);
        let wr = (frac * total as f32).round() as i32;
        (wr.clamp(0, total), (total - wr).clamp(0, total))
    };

    let mut lines: Vec<Line> = vec![];

    // Black section (top) — pure black background, bright white text
    for i in 0..black_rows as usize {
        let is_mate_row = is_mate && i == 0;
        let lbl = if is_mate_row {
            "MATE".to_string()
        } else if i == 0 && !busy && eval_cp < -20 {
            format!("{:.1}", eval_cp.abs() as f32 / 100.0)
        } else {
            " ".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:<4}", lbl),
                Style::default().bg(Color::Indexed(0)).fg(Color::Indexed(15))
                    .add_modifier(if is_mate_row{Modifier::BOLD}else{Modifier::empty()}))
        ]));
    }

    // White section (bottom) — bright white background, black text
    for i in 0..white_rows as usize {
        let is_last = i == white_rows as usize - 1;
        let is_mate_row = is_mate && is_last;
        let lbl = if is_mate_row {
            "MATE".to_string()
        } else if is_last && !busy && eval_cp > 20 {
            format!("{:.1}", eval_cp as f32 / 100.0)
        } else {
            " ".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:<4}", lbl),
                Style::default().bg(Color::Indexed(15)).fg(Color::Indexed(0))
                    .add_modifier(if is_mate_row{Modifier::BOLD}else{Modifier::empty()}))
        ]));
    }

    if !is_mate && (busy || eval_cp.abs() < 20) {
        let mid = (total / 2) as usize;
        if mid < lines.len() {
            let lbl = if busy { "~   " } else { "=   " };
            lines[mid] = Line::from(vec![
                Span::styled(lbl, Style::default()
                    .bg(Color::Indexed(15)).fg(Color::Indexed(0)).add_modifier(Modifier::BOLD))
            ]);
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn bw_panel_block(title: &str) -> Block<'static> {
    Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Indexed(8)))
        .style(Style::default().bg(Color::Reset))
        .title(Span::styled(format!(" {} ", title), Style::default().fg(Color::Indexed(15)).add_modifier(Modifier::BOLD)))
}

fn bw_label_color(label: MoveLabel) -> Color {
    match label {
        MoveLabel::Book       => Color::Indexed(8),
        MoveLabel::Good       => Color::Indexed(15),
        MoveLabel::Inaccuracy => Color::Indexed(7),
        MoveLabel::Mistake    => Color::Indexed(8),
        MoveLabel::Blunder    => Color::Indexed(0),
    }
}

fn draw_analyze_sidebar(app: &App, f: &mut Frame, area: Rect) {
    let [info_area, review_area, hist_area, keys_area] = Layout::vertical([
        Constraint::Length(5), Constraint::Length(8),
        Constraint::Fill(1),   Constraint::Length(6),
    ]).areas(area);

    let fg     = Color::Indexed(15);
    let dim    = Color::Indexed(7);
    let dimmer  = Color::Indexed(8);
    let hilite = Color::Indexed(7);

    // Available content width (minus block borders)
    let cw = area.width.saturating_sub(2) as usize;

    // ── Position info ─────────────────────────────────────────────────────────
    let snap    = app.replay_snaps.get(app.replay_idx);
    let mv_info = if app.replay_idx == 0 { "  Starting position".to_string() } else {
        let mnum = app.replay_idx.div_ceil(2);
        let who  = if app.replay_idx % 2 == 1 { "W" } else { "B" };
        let note = snap.map(|s| s.notation.as_str()).unwrap_or("");
        format!("  Move {}. [{}] {}", mnum, who, note)
    };
    let progress = format!("  {}/{}", app.replay_idx, app.replay_snaps.len().saturating_sub(1));
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(trunc(&mv_info, cw), sty(fg).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(&progress, sty(dim))]),
        Line::from(vec![Span::styled(trunc(&format!("  Result: {}", app.replay_result), cw), sty(dim))]),
    ]).block(bw_panel_block("POSITION")), info_area);

    // ── Eval sparkline ────────────────────────────────────────────────────────
    let spark_width = (review_area.width.saturating_sub(4)) as usize;
    let (spark_str, spark_cols) = eval_sparkline(&app.eval_history, spark_width);
    let has_spark = !spark_str.is_empty() && app.eval_history.len() > 1;

    // ── Review for this move ──────────────────────────────────────────────────
    let review_move_idx = if app.replay_idx == 0 { None } else {
        Some(app.replay_idx - 1)
    };
    let rev = review_move_idx.and_then(|i| app.move_reviews.get(i));
    let mut rev_lines: Vec<Line> = if let Some(rev) = rev {
        let col     = bw_label_color(rev.label);
        let delta_p = rev.delta_cp as f32 / 100.0;
        let sign    = if delta_p > 0.0 { "+" } else { "" };
        let ecol    = if delta_p < -0.5 { dimmer } else if delta_p < -0.2 { dim } else { fg };

        let move_time = review_move_idx
            .and_then(|i| app.gs.history.get(i))
            .map(|h| h.move_time_ms)
            .unwrap_or(0);
        let time_str = if move_time > 0 {
            if move_time >= 60_000 {
                format!("{:.0}m{:0>2}s", move_time as f64 / 60_000.0, (move_time / 1000) % 60)
            } else {
                format!("{:.1}s", move_time as f64 / 1000.0)
            }
        } else { String::new() };

        let pv_str = rev.best_pv.as_deref().map(|pv| {
            let rest: Vec<&str> = pv.split_whitespace().skip(1).collect();
            if rest.is_empty() { String::new() } else { format!("PV: {}", rest.join(" ")) }
        }).unwrap_or_default();

        let top_str: String = rev.top_moves.iter().map(|(uci, cp)| {
            let sign = if *cp >= 0 { "+" } else { "" };
            format!("{}{}{:.2}", trunc(uci, 5), sign, *cp as f32 / 100.0)
        }).collect::<Vec<_>>().join(" ");

        let label_w = cw.saturating_sub(8);
        let move_w  = cw.saturating_sub(8);
        let best_w  = cw.saturating_sub(8);
        let top_w   = cw.saturating_sub(8);
        let pv_w    = best_w.saturating_sub(10);

        let dm = |s: &str| Span::styled(s.to_string(), sty(dimmer));
        vec![
            Line::from(vec![
                dm(" Move  "),
                Span::styled(trunc(&rev.san, move_w), sty(fg).add_modifier(Modifier::BOLD)),
                if !time_str.is_empty() {
                    Span::styled(format!(" {}", time_str), sty(dim))
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(vec![
                dm(" Label "),
                Span::styled(trunc(&format!("[{}] {}", rev.label.icon(), rev.label.name()), label_w),
                    sty(col).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                dm(" Eval  "),
                Span::styled(trunc(&format!("{} -> {}", fmt_eval_f32(rev.eval_before), fmt_eval_f32(rev.eval_after)), cw), sty(dim)),
            ]),
            Line::from(vec![
                dm(" Best  "),
                Span::styled(trunc(rev.best_move.as_deref().unwrap_or("--"), best_w),
                    sty(fg).add_modifier(Modifier::BOLD)),
                if !pv_str.is_empty() && pv_w > 4 {
                    Span::styled(format!("  {}", trunc(&pv_str, pv_w)), sty(dim))
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(vec![
                dm(" Top:  "),
                Span::styled(trunc(&top_str, top_w), sty(dim)),
            ]),
        ]
    } else if app.replay_idx == 0 {
        vec![
            Line::from(vec![Span::styled(" Starting position", sty(dimmer))]),
            Line::from(""), Line::from(""), Line::from(""),
        ]
    } else {
        vec![
            Line::from(vec![Span::styled(" No analysis yet", sty(dimmer))]),
            Line::from(vec![Span::styled(" (play game first)", sty(dimmer))]),
            Line::from(""), Line::from(""),
        ]
    };
    if has_spark {
        let chars: Vec<char> = spark_str.chars().collect();
        let mut spark_spans: Vec<Span> = vec![Span::styled(" ", sty(dimmer))];
        for (i, ch) in chars.iter().enumerate() {
            let col = spark_cols.get(i).copied().unwrap_or(dim);
            spark_spans.push(Span::styled(ch.to_string(), sty(col)));
        }
        rev_lines.push(Line::from(spark_spans));
    }
    let busy  = if app.engine_busy { " ~" } else { " +" };
    let eng   = trunc(&app.analysis.engine_name, 12);
    let rtitle = trunc(&format!(" REVIEW{} [{}] ", busy, eng), cw.saturating_add(2));
    f.render_widget(
        Paragraph::new(rev_lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Indexed(8)))
                .style(Style::default().bg(Color::Reset))
                .title(Span::styled(rtitle, Style::default().fg(Color::Indexed(15))))
        ),
        review_area,
    );

    // ── History with current-move highlight + review labels ───────────────────
    let h   = &app.gs.history;
    let cap = hist_area.height.saturating_sub(2) as usize;
    let mut pairs: Vec<MovePair> = vec![];
    let mut i = 0; let mut n = 1;
    while i < h.len() {
        let wc = app.replay_idx == i + 1;
        let bc = app.replay_idx == i + 2;
        let wlabel = app.move_reviews.get(i).map(|r| r.label);
        let blabel = app.move_reviews.get(i+1).map(|r| r.label);
        pairs.push((n, h[i].san.clone(),
            h.get(i+1).map(|e| e.san.clone()).unwrap_or_default(),
            wc, bc, wlabel, blabel));
        i += 2; n += 1;
    }
    let start = pairs.len().saturating_sub(cap);
    let cur_hl = Style::default().fg(Color::Indexed(0)).bg(hilite).add_modifier(Modifier::BOLD);
    let hist_w = cw.saturating_sub(2).max(6); // width for move text
    let hist_lines: Vec<Line> = pairs[start..].iter().map(|(n,w,b,wc,bc,wl,bl)| {
        let w_label_str = wl.map(|l| format!(" {}", l.icon())).unwrap_or_default();
        let b_label_str = bl.map(|l| format!(" {}", l.icon())).unwrap_or_default();
        let w_lc = wl.map(bw_label_color).unwrap_or(dimmer);
        let b_lc = bl.map(bw_label_color).unwrap_or(dimmer);
        // Collapse to fit: each pair gets hist_w/2 chars per side
        let half = (hist_w / 2).max(4);
        Line::from(vec![
            Span::styled(format!(" {:>3}. ", n), sty(dimmer)),
            Span::styled(trunc(&format!("{:<w$}", w, w=half), half),
                if *wc { cur_hl } else { sty(fg) }),
            Span::styled(trunc(&w_label_str, 3), sty(w_lc)),
            Span::styled(trunc(&format!("{:<w$}", b, w=half), half),
                if *bc { cur_hl } else { sty(dim) }),
            Span::styled(trunc(&b_label_str, 3), sty(b_lc)),
        ])
    }).collect();
    f.render_widget(Paragraph::new(hist_lines).block(bw_panel_block("HISTORY")), hist_area);

    // ── Keys ──────────────────────────────────────────────────────────────────
    let k1 = trunc(" ←/h prev  →/l next  PgUp/PgDn ±10", cw);
    let k2 = trunc(" 0 start  $ end  [ prev bad  ] next bad", cw);
    let k4 = trunc(" Labels: + Good  ?! Inacc  ? Mist  ?? Blunder", cw);
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(k1, sty(dim))]),
        Line::from(vec![Span::styled(k2, sty(dim))]),
        Line::from(vec![
            Span::styled(" E ", sty(fg).add_modifier(Modifier::BOLD)),
            Span::styled(" PNG  ", sty(dim)),
            Span::styled(" q ", sty(dim)),
            Span::styled(" back", sty(dim)),
        ]),
        Line::from(vec![Span::styled(k4, sty(dimmer))]),
    ]).block(bw_panel_block("ANALYZE KEYS")), keys_area);
}

// ── PROMOTION ─────────────────────────────────────────────────────────────────
fn draw_promo(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(50, 9, f.area());
    f.render_widget(Clear, rect);
    let color = app.gs.turn;
    let pieces = [
        (if color==PC::White{"♕"}else{"♛"},"Queen"),
        (if color==PC::White{"♖"}else{"♜"},"Rook"),
        (if color==PC::White{"♗"}else{"♝"},"Bishop"),
        (if color==PC::White{"♘"}else{"♞"},"Knight"),
    ];
    let mut row: Vec<Span> = vec![Span::raw(" ")];
    for (i,(sym,name)) in pieces.iter().enumerate() {
        let s = app.promo_cur == i;
        row.push(Span::styled(format!(" {} {} ",sym,name),
            if s{Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD)}else{sty(cream())}));
        row.push(Span::raw("  "));
    }
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(" Select promotion piece:", sty(cream()))]),
        Line::from(""),
        Line::from(row),
        Line::from(""),
        Line::from(vec![Span::styled(" ←→/hl select   Enter confirm   Esc cancel", sty(dimmer()))]),
    ]).block(
        Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" PROMOTE PAWN ", sty(ac(th)).add_modifier(Modifier::BOLD)))
            .padding(Padding::new(2, 2, 1, 1)),
    ), rect);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// -- ANALYSIS PANEL --

fn label_color(label: MoveLabel, th: Theme) -> Color {
    match label {
        MoveLabel::Book       => info(th),
        MoveLabel::Good       => success(th),
        MoveLabel::Inaccuracy => warning(th),
        MoveLabel::Mistake    => rgb(th.warning(), th),
        MoveLabel::Blunder    => danger(th),
    }
}

// (dead analysis panel functions removed — inline in draw_sidebar)


fn status_str(app: &App, th: Theme) -> (String, Color) {
    match app.gs.status {
        Status::Checkmate =>
            (format!("\u{1fa0f} CHECKMATE — {} WINNER \u{1fa09}", app.gs.turn.opp().name()), warning(th)),
        Status::Stalemate if app.game_end==GameEnd::Draw =>
            ("\u{bd}-\u{bd} DRAW AGREED".into(), warning(th)),
        Status::Stalemate =>
            ("\u{bd}-\u{bd} STALEMATE — DRAW".into(), warning(th)),
        Status::Check =>
            (format!("\u{26a0} CHECK — {} TO MOVE", app.gs.turn.name()), danger(th)),
        _ if app.gs.clock_state==ClockState::Flagged =>
            (format!("\u{1fa0f} FLAG — {} WINNER \u{1fa09}", app.gs.turn.opp().name()), danger(th)),
        _ if app.thinking => ("\u{23f3} THINKING...".into(), dim(th)),
        _ => (format!("\u{25b6} {} TO MOVE", app.gs.turn.name()), info(th)),
    }
}

fn center(w: u16, h: u16, a: Rect) -> Rect {
    Rect::new(
        a.x + (a.width.saturating_sub(w))/2,
        a.y + (a.height.saturating_sub(h))/2,
        w.min(a.width), h.min(a.height),
    )
}


// ── PGN SAVED OVERLAY ─────────────────────────────────────────────────────────
fn draw_pgn_saved(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(62, 16, f.area());
    f.render_widget(Clear, rect);
    let path_str = app.pgn_saved_path.as_deref().unwrap_or("?");
    let home     = std::env::var("HOME").unwrap_or_default();
    let display  = if path_str.starts_with(&home) {
        format!("~{}", &path_str[home.len()..])
    } else { path_str.to_string() };
    let moves    = app.gs.history.len();
    let result   = if app.replay_result.is_empty() { "Game in progress".to_string() }
                   else { app.replay_result.clone() };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Saved PGN game notation to:", sty(dim(th))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {}", display), sty(info(th)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<54}", ""), sty(dimmer()))]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} moves  |  {}", moves, result), sty(dim(th))),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<54}", ""), sty(dimmer()))]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Any key ", sty(success(th)).add_modifier(Modifier::BOLD)),
            Span::styled("  close", sty(dim(th))),
        ]),
    ];
    f.render_widget(Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(sty(info(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" G  PGN SAVED ", sty(info(th)).add_modifier(Modifier::BOLD)))
            .padding(Padding::new(2, 2, 1, 1)),
    ), rect);
}

// ── MOVE TIME FORMATTER ───────────────────────────────────────────────────────
fn fmt_move_time(ms: u64) -> String {
    if ms == 0   { return String::new(); }
    if ms < 1000 { return format!("{}ms", ms); }
    format!("{:.1}s", ms as f32 / 1000.0)
}

// ── FEN INPUT SCREEN ──────────────────────────────────────────────────────────
fn draw_import(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(72, 22, f.area());
    f.render_widget(Clear, rect);
    let blink = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() / 500).is_multiple_of(2);
    let cursor = if blink { "\u{2588}" } else { " " };

    let buf = &app.import_buf;
    let char_count = buf.len();

    // Auto-detect mode based on input content
    let is_path = buf.starts_with('/') || buf.starts_with('~');
    let is_fen = !is_path && char_count > 0 && !buf.starts_with('[')
        && (buf.contains('/') || buf.starts_with(|c: char| c.is_ascii_digit()));

    let mode_str = if is_path {
        "file path"
    } else if is_fen {
        "FEN"
    } else if buf.starts_with('[') || char_count > 10 {
        "PGN"
    } else if char_count > 0 {
        "detecting..."
    } else {
        ""
    };

    // Show last 6 lines of input
    let all_lines: Vec<&str> = buf.lines().collect();
    let show_from = all_lines.len().saturating_sub(6);
    let visible = &all_lines[show_from..];
    let hidden_above = show_from;

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(" Paste FEN, PGN text  ", sty(dim(th))),
            Span::styled("or", sty(dimmer())),
            Span::styled("  type a file path", sty(dim(th))),
        ]),
        Line::from(vec![Span::styled(format!("  {:=<64}", ""), sty(dimmer()))]),
    ];

    if visible.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            format!(" {}", cursor), sty(info(th)))
        ]));
        for _ in 0..5 { lines.push(Line::from("")); }
    } else {
        let last_i = visible.len().saturating_sub(1);
        for (i, pline) in visible.iter().enumerate() {
            let txt = if i == last_i {
                format!(" {}{}", pline, cursor)
            } else {
                format!(" {}", pline)
            };
            lines.push(Line::from(vec![Span::styled(txt, sty(cream()))]));
        }
        while lines.len() < 8 { lines.push(Line::from("")); }
    }

    if hidden_above > 0 {
        lines.push(Line::from(vec![Span::styled(format!("  \u{2191} {} more line{} above", hidden_above, if hidden_above==1{""}else{"s"}), sty(dimmer()))]));
    }

    lines.push(Line::from(vec![Span::styled(format!("  {:=<64}", ""), sty(dimmer()))]));

    if let Some(err) = &app.import_err {
        lines.push(Line::from(vec![Span::styled(format!(" x {}", err), sty(danger(th)))]));
    } else {
        let hint = if is_fen { " FEN detected" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} chars", char_count), sty(dimmer())),
            Span::styled(hint, sty(if is_fen { success(th) } else { dim(th) })),
            if !mode_str.is_empty() && !is_fen {
                Span::styled(format!(" ({})", mode_str), sty(dim(th)))
            } else {
                Span::raw("")
            },
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  Examples:", sty(dimmer()))]));
    lines.push(Line::from(vec![Span::styled(
        "  rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 1  <- FEN", sty(dimmer()))]));
    lines.push(Line::from(vec![Span::styled(
        "  ~/rchess_export/game.pgn       <- file path", sty(dimmer()))]));
    lines.push(Line::from(vec![Span::styled(
        "  [White \"Me\"] 1. e4 e5 2. Nf3  <- PGN text", sty(dimmer()))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Enter ", sty(success(th)).add_modifier(Modifier::BOLD)),
        Span::styled(" load    ", sty(dim(th))),
        Span::styled(" Backspace ", sty(dim(th))),
        Span::styled(" delete    ", sty(dim(th))),
        Span::styled(" Esc ", sty(dim(th))),
        Span::styled(" cancel", sty(dim(th))),
    ]));

    f.render_widget(Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" \u{2261} IMPORT GAME ", sty(ac(th)).add_modifier(Modifier::BOLD)))
            .padding(Padding::new(2, 2, 1, 1)),
    ), rect);
}


// ── PUZZLE SCREEN ─────────────────────────────────────────────────────────────
fn draw_puzzle(app: &App, f: &mut Frame) {
    let a  = f.area();
    let th = app.cfg.theme;
    f.render_widget(Block::default().style(Style::default().bg(bg0(th))), a);

    use crate::puzzle::PuzzleState;

    match &app.puzzle_state {
        PuzzleState::Loading => {
            let rect = center(40, 7, a);
            f.render_widget(Clear, rect);
            f.render_widget(Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![Span::styled(" \u{23f3} Fetching from Lichess...", sty(info(th)))]),
                Line::from(""),
                Line::from(vec![Span::styled(" Esc to cancel", sty(dimmer()))]),
            ]).block(
                Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
                    .title(Span::styled(" \u{2605} DAILY PUZZLE ", sty(ac(th)).add_modifier(Modifier::BOLD)))
                    .padding(Padding::new(2, 2, 1, 1)),
            ), rect);
        }
        PuzzleState::Failed(e) => {
            let has_token = !app.cfg.lichess_token.is_empty();
            let rect = center(66, 19, a);
            f.render_widget(Clear, rect);
            let sep = format!("  {:─<56}", "");
            let mut lines: Vec<Line> = vec![
                Line::from(""),
                Line::from(vec![Span::styled(format!(" Error: {}", e), sty(danger(th)).add_modifier(Modifier::BOLD))]),
                Line::from(""),
                Line::from(vec![Span::styled(&sep, sty(dimmer()))]),
                Line::from(""),
            ];
            if !has_token {
                lines.push(Line::from(vec![Span::styled(" Lichess works without a token, but may rate-limit", sty(dim(th)))]));
                lines.push(Line::from(vec![Span::styled(" anonymous requests. A free token fixes this:", sty(dim(th)))]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled("  1. Sign up free at lichess.org", sty(info(th)))]));
                lines.push(Line::from(vec![Span::styled("  2. Go to lichess.org/account/security", sty(info(th)))]));
                lines.push(Line::from(vec![Span::styled("  3. Create a Personal API Token (no scopes needed)", sty(info(th)))]));
                lines.push(Line::from(vec![Span::styled("  4. Add to ~/.config/rchess/rchess_tui.conf:", sty(dim(th)))]));
                lines.push(Line::from(vec![Span::styled("     lichess_token = lip_xxxxxxxxxxxx", sty(warning(th)).add_modifier(Modifier::BOLD))]));
            } else {
                lines.push(Line::from(vec![Span::styled(" Token is configured. Check your internet.", sty(dim(th)))]));
                lines.push(Line::from(vec![Span::styled(" Also try: sudo pacman -S ca-certificates", sty(dimmer()))]));
                lines.push(Line::from(""));
                lines.push(Line::from(""));
                lines.push(Line::from(""));
                lines.push(Line::from(""));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(&sep, sty(dimmer()))]));
            lines.push(Line::from(vec![
                Span::styled(" n ", sty(success(th)).add_modifier(Modifier::BOLD)),
                Span::styled(" retry    ", sty(dim(th))),
                Span::styled(" Esc ", sty(dim(th))),
                Span::styled(" back to menu", sty(dim(th))),
            ]));
            f.render_widget(Paragraph::new(lines).block(
                Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(sty(danger(th))).style(Style::default().bg(bg1(th)))
                    .title(Span::styled(" ★ DAILY PUZZLE — FAILED ", sty(danger(th)).add_modifier(Modifier::BOLD)))
                    .padding(Padding::new(2, 2, 1, 1)),
            ), rect);
        }
        state => {
            // Puzzle is loaded — show board + info panel
            let cw = app.cell_w;
            let bw = (3 + 8 * cw + 3 + 2).min(a.width);
            let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(a);
            let [board_col, side_col] = Layout::horizontal([Constraint::Length(bw), Constraint::Fill(1)]).areas(body);

            // Top bar
            let puzzle = app.puzzle.as_ref();
            let (rating, themes_str) = if let Some(p) = puzzle {
                let t = p.themes.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
                (format!("{}",p.rating), t)
            } else { ("?".to_string(), String::new()) };

            let status_msg = match state {
                PuzzleState::WaitingInput  => " Find the best move!",
                PuzzleState::CorrectMove   => " \u{2713} Correct! Keep going...",
                PuzzleState::WrongMove(_)  => " \u{2717} Wrong move — try again",
                PuzzleState::Solved        => " \u{2605} Puzzle solved!",
                PuzzleState::Setup         => " Setting up...",
                _ => "",
            };
            let status_col = match state {
                PuzzleState::CorrectMove | PuzzleState::Solved => success(th),
                PuzzleState::WrongMove(_) => danger(th),
                _ => info(th),
            };

            f.render_widget(
                Paragraph::new(format!(
                    " \u{2605} DAILY PUZZLE  Rating: {}  Themes: {}  {}",
                    rating, themes_str, status_msg
                )).style(Style::default().fg(status_col).bg(bg1(th))),
                top,
            );

            // Board (reuse draw_board logic for the puzzle position)
            draw_board(app, f, board_col, app.cell_w as usize, app.cell_h as usize);

            // Side panel
            draw_puzzle_sidebar(app, f, side_col);
        }
    }
}

fn draw_puzzle_sidebar(app: &App, f: &mut Frame, area: Rect) {
    use crate::puzzle::PuzzleState;
    let th = app.cfg.theme;
    let [info_area, input_area, keys_area] = Layout::vertical([
        Constraint::Length(8), Constraint::Length(5), Constraint::Fill(1),
    ]).areas(area);

    // Puzzle info
    let puzzle = app.puzzle.as_ref();
    let you_are = app.gs.turn.name();
    let solved  = app.puzzle_state == PuzzleState::Solved;
    let wrong   = matches!(&app.puzzle_state, PuzzleState::WrongMove(_));

    let info_lines = if let Some(p) = puzzle {
        let themes: String = p.themes.iter().take(4)
            .cloned().collect::<Vec<_>>().join("\n  ");
        vec![
            Line::from(vec![
                Span::styled(" Puzzle  ", sty(dimmer())),
                Span::styled(format!("#{}", p.id), sty(cream()).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Rating  ", sty(dimmer())),
                Span::styled(format!("{}", p.rating), sty(warning(th)).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" You are ", sty(dimmer())),
                Span::styled(you_are, sty(if app.gs.turn == crate::engine::Color::White { pw(th) } else { pb_c(th) }).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(if solved { " \u{2605} SOLVED!" } else if wrong { " \u{2717} Wrong" } else { " Find best move" },
                    sty(if solved { success(th) } else if wrong { danger(th) } else { info(th) }).add_modifier(Modifier::BOLD))
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(" Themes:", sty(dimmer()))]),
            Line::from(vec![Span::styled(format!("  {}", themes), sty(dim(th)))]),
        ]
    } else {
        vec![Line::from("")]
    };
    f.render_widget(Paragraph::new(info_lines).block(panel_block("PUZZLE", th)), info_area);

    // Input (same as game input)
    let cur_bc   = app.to_board(app.cursor);
    let cur_name = format!("{}{}", (b'a' + cur_bc.1 as u8) as char, 8 - cur_bc.0);
    let (input_line, input_col) = if !app.input_buf.is_empty() {
        (format!("  [{}]  {}\u{2588}", cur_name, app.input_buf), cream())
    } else {
        (format!("  [{}]  type move or click", cur_name), dim(th))
    };
    let hint = if let Some(err) = &app.input_err {
        (format!("  \u{2717}  {}", err), danger(th))
    } else {
        ("  e.g. Nf3  or  navigate + Enter".to_string(), dimmer())
    };
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(input_line, sty(input_col).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(hint.0, sty(hint.1))]),
    ]).block(panel_block("INPUT", th)), input_area);

    // Keys
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(" \u{2191}\u{2193}\u{2190}\u{2192}/hjkl  move cursor", sty(dim(th)))]),
        Line::from(vec![Span::styled(" Enter/click  select & move", sty(dim(th)))]),
        Line::from(vec![Span::styled(" n  new puzzle    q  menu", sty(dim(th)))]),
    ]).block(panel_block("KEYS", th)), keys_area);
}

// ── ONLINE ──────────────────────────────────────────────────────────────────
fn draw_load_game(app: &App, f: &mut Frame) {
    let th = app.cfg.theme;
    let rect = center(72, 28, f.area());
    let mut lines: Vec<Line> = vec![];

    if app.saved_games.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled("  No saved games found.", sty(dim(th)))]));
    } else {
        for (i, (_, save)) in app.saved_games.iter().enumerate() {
            let s = app.menu_cur == i;
            let status_tag = if save.incomplete { " ⟳" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(if s {" ▶ "} else {"   "}, sty(ac(th))),
                Span::styled(
                    format!("{} vs {}  [{}]  {}{}", save.white_name, save.black_name, save.time_control, save.result, status_tag),
                    if s { sty(cream()).add_modifier(Modifier::BOLD) } else { sty(dim(th)) },
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("      "),
                Span::styled(format!("{} moves — {}",
                    save.moves.len(), &save.date[..19]), sty(dimmer())),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(format!("  {:─<60}", ""), sty(dimmer()))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Enter ", sty(success(th)).add_modifier(Modifier::BOLD)),
        Span::styled(" load and analyze    ", sty(dim(th))),
        Span::styled(" Esc ", sty(dim(th))),
        Span::styled(" back to menu", sty(dim(th))),
    ]));

    f.render_widget(Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" ⇪ SAVED GAMES ", sty(ac(th)).add_modifier(Modifier::BOLD)))
            .padding(Padding::new(2, 2, 1, 1)),
    ), rect);
}

fn draw_online_lobby(app: &App, f: &mut Frame) {
    let th = app.cfg.theme;
    let has_token = app.cfg.has_lichess_token();
    let connected = app.lichess_connected;
    let mut lines: Vec<Line> = vec![Line::from("")];

    // ── Account ──
    if has_token {
        let name = if connected && !app.player_name.is_empty() {
            app.player_name.as_str()
        } else {
            "not signed in"
        };
        let rating = if connected {
            app.lichess.as_ref()
                .and_then(|c| c.account.as_ref())
                .and_then(|a| a.rating)
                .map(|r| format!(" ({r})"))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let state = if connected {
            "● connected"
        } else if app.lichess.is_some() {
            "○ connecting…"
        } else {
            "○ offline"
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {name}{rating}"), sty(cream()).add_modifier(Modifier::BOLD)),
            Span::styled(format!("   {state}"), sty(if connected { success(th) } else { warning(th) })),
        ]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  No Lichess token configured", sty(danger(th)).add_modifier(Modifier::BOLD))]));
    }
    lines.push(Line::from(vec![Span::styled(format!("  {}", trunc(&app.online_status, 62)), sty(dim(th)))]));
    lines.push(Line::from(""));

    if !has_token {
        // Onboarding
        lines.push(Line::from(vec![Span::styled(format!("  {:─<60}", ""), sty(dimmer()))]));
        lines.push(Line::from(vec![Span::styled("  TO PLAY ONLINE", sty(ac(th)).add_modifier(Modifier::BOLD))]));
        lines.push(Line::from(vec![Span::styled("   1. Open lichess.org/account/oauth/token", sty(cream()))]));
        lines.push(Line::from(vec![Span::styled("   2. Create a token with the board:play scope", sty(cream()))]));
        lines.push(Line::from(vec![Span::styled("   3. Press s, select Lichess token, Enter to paste", sty(cream()))]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  s ", sty(success(th)).add_modifier(Modifier::BOLD)),
            Span::styled("Settings   ", sty(dim(th))),
            Span::styled("q ", sty(dim(th))),
            Span::styled("main menu", sty(dim(th))),
        ]));
    } else {
        // ── Match settings ──
        lines.push(Line::from(vec![Span::styled(format!("  {:─<60}", ""), sty(dimmer()))]));
        let (tc_label, _) = ONLINE_TCS[app.online_tc_idx.min(ONLINE_TCS.len() - 1)];
        lines.push(Line::from(vec![
            Span::styled("  MATCH ", sty(ac(th)).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} · {tc_label}", if app.online_rated { "Rated" } else { "Casual" }), sty(cream())),
            Span::styled("      r rated   t time", sty(dim(th))),
        ]));
        if app.online_seeking {
            lines.push(Line::from(vec![Span::styled(
                "  ● Searching for an opponent…  Esc to cancel", sty(warning(th)))]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  p ", sty(success(th)).add_modifier(Modifier::BOLD)),
                Span::styled("Find a random opponent", sty(cream())),
            ]));
        }
        lines.push(Line::from(""));

        // ── Ongoing games ──
        if !app.online_games.is_empty() {
            lines.push(Line::from(vec![Span::styled("  ONGOING GAMES", sty(ac(th)).add_modifier(Modifier::BOLD))]));
            for (i, g) in app.online_games.iter().take(3).enumerate() {
                let rating = g.opponent_rating.map(|r| format!(" ({r})")).unwrap_or_default();
                let turn = if g.is_my_turn { "your turn" } else { "waiting" };
                lines.push(Line::from(vec![
                    Span::styled(format!("   {} vs {}{rating}", i + 1, g.opponent), sty(cream())),
                    Span::styled(format!("  {} · {} · {turn}", if g.rated { "rated" } else { "casual" }, g.speed), sty(dim(th))),
                ]));
            }
            lines.push(Line::from(vec![Span::styled("   1/2/3 resume · Enter first game", sty(dim(th)))]));
            lines.push(Line::from(""));
        }

        // ── Challenges ──
        lines.push(Line::from(vec![Span::styled(format!("  {:─<60}", ""), sty(dimmer()))]));
        if app.lichess_challenges.is_empty() {
            lines.push(Line::from(vec![Span::styled("  No incoming challenges", sty(dim(th)))]));
        } else {
            lines.push(Line::from(vec![Span::styled("  CHALLENGES", sty(ac(th)).add_modifier(Modifier::BOLD))]));
            for (i, ch) in app.lichess_challenges.iter().take(3).enumerate() {
                let selected = i == app.challenge_cur.min(app.lichess_challenges.len() - 1);
                let rating = ch.challenger_rating.map(|r| r.to_string()).unwrap_or_else(|| "?".into());
                let line = format!("{} ({rating}) · {} · {}", ch.challenger_name, ch.time_control, ch.variant);
                lines.push(Line::from(vec![
                    Span::styled(if selected { "  ▸ " } else { "    " }, sty(ac(th))),
                    Span::styled(
                        trunc(&line, 58),
                        if selected { sty(cream()).add_modifier(Modifier::BOLD) } else { sty(dim(th)) },
                    ),
                ]));
            }
            lines.push(Line::from(vec![Span::styled(
                "   j/k select   a accept   d decline", sty(dim(th)))]));
        }
        lines.push(Line::from(""));

        // ── Challenge composer ──
        if app.challenge_mode {
            let blink = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() / 500).is_multiple_of(2);
            let cursor = if blink { "█" } else { " " };
            lines.push(Line::from(vec![
                Span::styled("  Challenge: ", sty(ac(th))),
                Span::styled(format!("{}{cursor}", app.challenge_buf), sty(cream())),
            ]));
            lines.push(Line::from(vec![Span::styled(
                "  Enter send   Esc cancel", sty(dim(th)))]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  c ", sty(ac(th)).add_modifier(Modifier::BOLD)),
                Span::styled("Challenge a user   ", sty(cream())),
                Span::styled("R ", sty(ac(th))),
                Span::styled("reconnect   ", sty(cream())),
                Span::styled("s ", sty(ac(th))),
                Span::styled("settings", sty(cream())),
            ]));
        }
        lines.push(Line::from(vec![
            Span::styled("  q ", sty(dim(th))),
            Span::styled("main menu", sty(dim(th))),
        ]));
    }

    let h = (lines.len() as u16 + 4).min(f.area().height.saturating_sub(2));
    let rect = center(70, h, f.area());
    f.render_widget(Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" ♞ LICHESS ONLINE ", sty(ac(th)).add_modifier(Modifier::BOLD)))
            .padding(Padding::new(1, 1, 0, 0)),
    ), rect);
}

fn draw_online_game(app: &App, f: &mut Frame) {
    draw_game(app, f);
    let th = app.cfg.theme;
    let rect = f.area();

    // Chat panel (bottom-right, above the input line)
    if !app.chat_messages.is_empty() || app.online_chat_mode || !app.chat_buf.is_empty() {
        let panel_w = 38u16.min(rect.width.saturating_sub(6));
        let max_vis = 6usize;
        let chat_h = (app.chat_messages.len().min(max_vis) + if app.online_chat_mode { 2 } else { 1 }) as u16;
        let chat_rect = Rect::new(
            rect.x + rect.width.saturating_sub(panel_w + 2),
            rect.y + rect.height.saturating_sub(chat_h + 7),
            panel_w, chat_h + 2,
        );
        let mut chat_lines: Vec<Line> = app.chat_messages.iter()
            .rev().take(max_vis).rev()
            .map(|m| Line::from(vec![Span::styled(trunc(m, (panel_w - 4) as usize), sty(dim(th)))]))
            .collect();
        if app.online_chat_mode {
            let prompt = format!(" > {}", app.chat_buf);
            chat_lines.push(Line::from(vec![Span::styled(trunc(&prompt, (panel_w - 4) as usize), sty(ac(th)))]));
            chat_lines.push(Line::from(vec![Span::styled(" Esc cancel  Enter send", sty(dimmer()))]));
        }
        f.render_widget(Clear, chat_rect);
        f.render_widget(Paragraph::new(chat_lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Plain)
                .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
                .title(Span::styled(" CHAT ", sty(ac(th))))
                .padding(Padding::new(1, 1, 0, 0)),
        ), chat_rect);
    }

    if let Some(result) = &app.online_result {
        let w = 52u16.min(rect.width.saturating_sub(4));
        let box_rect = center(w, 9, rect);
        f.render_widget(Clear, box_rect);
        let lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                trunc(result, (w - 4) as usize),
                sty(cream()).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled(
                format!(" {}", app.online_tc_label), sty(dim(th)))]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" Esc ", sty(ac(th))), Span::styled("lobby   ", sty(dim(th))),
                Span::styled(" r ", sty(ac(th))),   Span::styled("analyze   ", sty(dim(th))),
                Span::styled(" E ", sty(ac(th))),   Span::styled("PNG", sty(dim(th))),
            ]),
        ];
        f.render_widget(Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(success(th))).style(Style::default().bg(bg1(th)))
                .title(Span::styled(" GAME OVER ", sty(success(th)).add_modifier(Modifier::BOLD)))
                .padding(Padding::new(1, 1, 0, 0)),
        ), box_rect);
    }
}
