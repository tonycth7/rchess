// src/ui.rs  — v0.7
use std::collections::HashSet;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use crate::app::{App, Gs, Mode, Screen, ClockState, GameEnd, ReplaySnap, MoveLabel, OnlineStep, VERSION};
use crate::config::{MoveHints, PieceStyle, Theme, UiMode};
use crate::engine::{Color as PC, Kind, Status, king_sq};

// ── Palette helpers ───────────────────────────────────────────────────────────
fn rgb(c: (u8,u8,u8)) -> Color { Color::Rgb(c.0,c.1,c.2) }
fn bg0(th: Theme) -> Color { rgb(th.bg_colors().0) }
fn bg1(th: Theme) -> Color { rgb(th.bg_colors().1) }
fn ac(th: Theme)  -> Color { rgb(th.accent()) }
fn sq_l(th: Theme)-> Color { rgb(th.squares().0) }
fn sq_d(th: Theme)-> Color { rgb(th.squares().1) }
fn pw(th: Theme)  -> Color { rgb(th.piece_colors().0) }
fn pb_c(th: Theme)-> Color { rgb(th.piece_colors().1) }
fn sty(c: Color)  -> Style { Style::default().fg(c) }

const CREAM:  Color = Color::Rgb(220,210,185);
const DIM:    Color = Color::Rgb(110,130,110);
const DIMMER: Color = Color::Rgb(55, 70, 55);
const RED:    Color = Color::Rgb(200, 70, 70);
const TEAL:   Color = Color::Rgb(80, 180,140);
const AMBER:  Color = Color::Rgb(220,160, 40);
const GREEN:  Color = Color::Rgb(120,220,160);

// CELL_W and CELL_H are now read from app.cfg.cell_w / app.cfg.cell_h
// Defaults kept here as fallback for non-game screens
const DEFAULT_CELL_W: usize = 8;
const DEFAULT_CELL_H: usize = 4;

// Safely truncate to max visible chars
fn trunc(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max { return s.to_string(); }
    let mut out: String = chars[..max.saturating_sub(2)].iter().collect();
    out.push_str("..");
    out
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
        Screen::Replay      => draw_replay(app, f),
        Screen::PngPreview  => { draw_game(app, f); draw_png_preview(app, f); }
        Screen::PgnSaved    => { draw_game(app, f); draw_pgn_saved(app, f); }
        Screen::FenInput    => draw_fen_input(app, f),
        Screen::PgnImport   => draw_pgn_import(app, f),
        Screen::Puzzle      => draw_puzzle(app, f),
        Screen::OnlineSetup   => draw_online_setup(app, f),
        Screen::OnlineWaiting => draw_online_waiting(app, f),
    }
}

// ── MENU ──────────────────────────────────────────────────────────────────────
fn draw_menu(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(58, 30, f.area());
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th))),
        rect,
    );
    let inner = pad(rect, 2, 1);
    let opts  = [
        "  ♟  TWO PLAYERS",
        "  ◈  VS COMPUTER   [AI + Opening Book]",
        "  ⚡  ONLINE vs FRIEND  [Real-time]",
        "  ⚙  SETTINGS",
        "  ≡  START FROM FEN",
        "  ↥  LOAD PGN FILE",
        "  ★  DAILY PUZZLE  [Lichess]",
    ];
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::raw("  "), Span::styled("♔ ♕ ♗ ♘ ♖ ♙", sty(pw(th)).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::raw("  "), Span::styled("♟ ♜ ♞ ♝ ♛ ♚", sty(DIM).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("         R  C H E S S", sty(ac(th)).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(format!("      TERMINAL EDITION  v{}", VERSION), sty(DIM))]),
        Line::from(vec![Span::styled("      PNG export · PGN · Replay · Mouse", sty(DIMMER))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<50}",""), sty(DIMMER))]),
        Line::from(""),
    ];
    for (i, &opt) in opts.iter().enumerate() {
        let s = app.menu_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s{" ▶ "}else{"   "}, sty(ac(th))),
            Span::styled(format!("{:<48}", opt),
                if s { Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD) } else { sty(CREAM) }),
        ]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(format!("  {:─<50}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  ↑↓ navigate    Enter select    q quit", sty(DIMMER))]));
    f.render_widget(Paragraph::new(lines), inner);
}

// ── COLOR PICK ────────────────────────────────────────────────────────────────
fn draw_color_pick(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(52, 18, f.area());
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th))),
        rect,
    );
    let inner = pad(rect, 2, 1);
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled("     CHOOSE YOUR SIDE", sty(ac(th)).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(DIMMER))]),
        Line::from(""),
    ];
    for (i,(sym,label,fg)) in [("♔","WHITE  — moves first",pw(th)),("♚","BLACK  — moves second",DIM)].iter().enumerate() {
        let s = app.color_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s{" ▶ "}else{"   "}, sty(ac(th))),
            Span::styled(format!("{} {}",sym,label),
                Style::default().fg(*fg).bg(if s{bg0(th)}else{bg1(th)})
                    .add_modifier(if s{Modifier::BOLD}else{Modifier::empty()})),
        ]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  ↑↓ navigate   Enter confirm   Esc back", sty(DIMMER))]));
    f.render_widget(Paragraph::new(lines), inner);
}

// ── SETTINGS ──────────────────────────────────────────────────────────────────
fn draw_settings(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let area = f.area();

    // Use most of the terminal — leaves 2 rows top/bottom for breathing room
    let h    = area.height.saturating_sub(4).max(10);
    let w    = 72u16.min(area.width.saturating_sub(4));
    let rect = center(w, h, area);

    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" ⚙  SETTINGS ", sty(ac(th)).add_modifier(Modifier::BOLD))),
        rect,
    );
    let inner = pad(rect, 2, 1);

    // All settings rows
    let highlight_str = match app.cfg.highlight_brightness {
        0 => "0  (dim)", 5 => "5  (default)", 10 => "10 (vivid)",
        n => Box::leak(format!("{}", n).into_boxed_str()),
    };
    let rows: &[(&str, &str)] = &[
        ("Theme",            app.cfg.theme.name()),
        ("Piece style",      app.cfg.piece_style.name()),
        ("AI difficulty",    app.cfg.ai_depth.name()),
        ("Move hints",       app.cfg.move_hints.name()),
        ("Time control",     app.cfg.time_control.name()),
        ("Show coords",      if app.cfg.show_coords {"ON"} else {"OFF"}),
        ("Show clock",       if app.cfg.show_clock  {"ON"} else {"OFF"}),
        ("Flip board",       if app.cfg.flip_board  {"ON"} else {"OFF"}),
        ("Auto-flip PvP",    if app.cfg.auto_flip   {"ON"} else {"OFF"}),
        ("Confirm move",     if app.cfg.confirm_move{"ON"} else {"OFF"}),
        ("UI mode",          app.cfg.ui_mode.name()),
        ("Analysis engine",  app.cfg.analysis_engine.name()),
        ("Analysis depth",   match app.cfg.analysis_depth { 1=>"1  (fast)", 2=>"2  (balanced)", _=>"3  (strong)" }),
        ("Auto-save PNG",    if app.cfg.auto_save_png  {"ON"} else {"OFF"}),
        ("Highlight bright", highlight_str),
        ("Board cell width",  match app.cfg.cell_w { 4=>"4 (tiny)", 6=>"6 (small)", 8=>"8 (default)", 10=>"10 (large)", 12=>"12 (huge)", n => Box::leak(format!("{}", n).into_boxed_str()) }),
        ("Board cell height", match app.cfg.cell_h { 2=>"2 (flat)", 3=>"3", 4=>"4 (default)", 5=>"5", 6=>"6 (tall)", n => Box::leak(format!("{}", n).into_boxed_str()) }),
        ("Lichess token",    if app.cfg.lichess_token.is_empty() {"(not set)"} else {"(configured ✓)"}),
    ];
    let n = rows.len();

    // How many rows fit in the visible area (each row = 2 lines: label + blank)
    // inner height - 4 (header sep + footer sep + 2 lines footer) = visible budget
    let budget = inner.height.saturating_sub(5) as usize;
    let visible = (budget / 2).max(1);

    // Scroll to keep selected row visible
    let cur = app.settings_cur;
    // scroll_off: first row index to show
    let scroll = if cur < visible { 0 }
                 else { cur + 1 - visible };
    let scroll = scroll.min(n.saturating_sub(visible));

    let mut lines: Vec<Line> = vec![
        Line::from(vec![Span::styled(format!("  {:─<60}", ""), sty(DIMMER))]),
        Line::from(""),
    ];

    // Scroll indicator top
    if scroll > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  ▲ {} more above", scroll), sty(DIMMER))]));
    } else {
        lines.push(Line::from(""));
    }

    // Visible rows
    for i in scroll..(scroll + visible).min(n) {
        let (label, value) = rows[i];
        let s = cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s { " ▶ " } else { "   " }, sty(ac(th))),
            Span::styled(
                format!("{:<18}", label),
                if s { sty(ac(th)).add_modifier(Modifier::BOLD) } else { sty(CREAM) },
            ),
            Span::raw("  "),
            Span::styled(
                if s { format!("◀  {}  ▶", value) } else { format!("   {}   ", value) },
                if s { Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD) } else { sty(DIM) },
            ),
        ]));
        lines.push(Line::from(""));
    }

    // Scroll indicator bottom
    let below = n.saturating_sub(scroll + visible);
    if below > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  ▼ {} more below  (↓ to scroll)", below), sty(DIMMER))]));
    } else {
        lines.push(Line::from(""));
    }

    // Footer separator
    lines.push(Line::from(vec![Span::styled(format!("  {:─<60}", ""), sty(DIMMER))]));

    // Save notice or hint
    if app.saved_notice.is_some() {
        let home = std::env::var("HOME").unwrap_or_default();
        let path = format!("{}/.config/rchess/rchess_tui.conf", home);
        lines.push(Line::from(vec![
            Span::styled("  ✓ Saved  ", sty(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(path, sty(GREEN)),
        ]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  w/s save  r reset  ←→/hl change  jk navigate  Esc back",
            sty(DIMMER),
        )]));
    }

    f.render_widget(Paragraph::new(lines), inner);
}


// ── BLOCK ART PIECES ──────────────────────────────────────────────────────────
// Each piece is defined as 5 rows × 9 chars using block characters.
// ' ' = square bg shows through, '█' = piece colour.
// Pieces are designed to be visually distinct at a glance.
fn piece_block_row(k: Kind, line: usize) -> &'static str {
    match (k, line) {
        // ── Pawn: round head, tapered body, wide base ─────────────────────────
        (Kind::P, 0) => "         ",
        (Kind::P, 1) => "   ███   ",
        (Kind::P, 2) => "   ███   ",
        (Kind::P, 3) => "  █████  ",
        (Kind::P, _) => "         ",
        // ── Knight: L-shaped head charging left ───────────────────────────────
        (Kind::N, 0) => "   ████  ",
        (Kind::N, 1) => "  █████  ",
        (Kind::N, 2) => "  ████   ",
        (Kind::N, 3) => "  █████  ",
        (Kind::N, _) => "         ",
        // ── Bishop: tall diamond/mitre shape ──────────────────────────────────
        (Kind::B, 0) => "    █    ",
        (Kind::B, 1) => "   ███   ",
        (Kind::B, 2) => "  █████  ",
        (Kind::B, 3) => " ███████ ",
        (Kind::B, _) => "         ",
        // ── Rook: battlements on top ───────────────────────────────────────────
        (Kind::R, 0) => " █ █ █ █ ",
        (Kind::R, 1) => " ███████ ",
        (Kind::R, 2) => "  █████  ",
        (Kind::R, 3) => " ███████ ",
        (Kind::R, _) => "         ",
        // ── Queen: wide crown with 5 points ───────────────────────────────────
        (Kind::Q, 0) => " █ █ █ █ ",
        (Kind::Q, 1) => " ███████ ",
        (Kind::Q, 2) => " ███████ ",
        (Kind::Q, 3) => " ███████ ",
        (Kind::Q, _) => "         ",
        // ── King: cross on top, solid body ────────────────────────────────────
        (Kind::K, 0) => "    █    ",
        (Kind::K, 1) => "  █████  ",
        (Kind::K, 2) => "  █████  ",
        (Kind::K, 3) => " ███████ ",
        (Kind::K, _) => "         ",
    }
}

// ── GAME ──────────────────────────────────────────────────────────────────────
fn draw_game(app: &App, f: &mut Frame) {
    let a  = f.area();
    let cell_w = app.cfg.cell_w as u16;
    let bw = (3 + 8*cell_w + 3 + 2).min(a.width);
    let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(a);
    let [board_col, side_col] = Layout::horizontal([Constraint::Length(bw), Constraint::Min(0)]).areas(body);
    let [board_area, input_area] = Layout::vertical([Constraint::Min(0), Constraint::Length(4)]).areas(board_col);

    draw_topbar(app, f, top);
    draw_board(app, f, board_area);
    draw_input(app, f, input_area);
    draw_sidebar(app, f, side_col);

    let is_over = matches!(app.gs.status, Status::Checkmate|Status::Stalemate)
               || app.gs.clock_state == ClockState::Flagged
               || app.game_end == GameEnd::Draw;
    if is_over { draw_gameover(app, f); }
}

fn draw_topbar(app: &App, f: &mut Frame, area: Rect) {
    let th = app.cfg.theme;
    let mode_s = match app.mode {
        Mode::PvP    => "TWO PLAYERS".to_string(),
        Mode::Online => format!("ONLINE  [YOU:{}]",
            app.online_color.map(|c| c.name()).unwrap_or("?")),
        Mode::CPU    => format!("VS CPU [YOU:{}] [{}]",
            app.player_color.name(),
            app.cfg.ai_depth.name().split_whitespace().next().unwrap_or("")),
    };
    let (st, col) = status_str(app);
    let clock     = if app.cfg.show_clock { format!("  Mv {}  ", app.gs.fullmove) } else { String::new() };
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
        Paragraph::new(format!(" ♟ RChess v{}{}{}{}│  {}  │  {}",
            VERSION, clock, book_ind, eval_str, mode_s, st))
            .style(Style::default().fg(col).bg(bg1(th))),
        area,
    );
}

// ── BOARD ─────────────────────────────────────────────────────────────────────
fn draw_board(app: &App, f: &mut Frame, area: Rect) {
    #[allow(non_snake_case)]
    let CELL_W = app.cfg.cell_w as usize;
    #[allow(non_snake_case)]
    let CELL_H = app.cfg.cell_h as usize;
    let th      = app.cfg.theme;
    let flipped = app.flipped();
    let row_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };
    let col_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };

    let tgts: HashSet<(usize,usize)> = app.targets.iter().map(|m| m.to).collect();
    let king_chk = if app.gs.status==Status::Check { king_sq(&app.gs.board,app.gs.turn) } else { None };
    let last     = app.gs.history.last();
    let cur_bc   = app.to_board(app.cursor);

    let sel_bg = rgb(th.select());
    let cur_bg = rgb(th.cursor());
    let chk_bg = Color::Rgb(180,40,40);

    let lm_color = |is_light: bool| {
        let sq = if is_light { th.squares().0 } else { th.squares().1 };
        let lm = th.last_move();
        Color::Rgb(
            ((sq.0 as u16*2+lm.0 as u16)/3) as u8,
            ((sq.1 as u16*2+lm.1 as u16)/3) as u8,
            ((sq.2 as u16*2+lm.2 as u16)/3) as u8,
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
    // Hint dot fg: starts near white at 0 (barely visible on square), vivid green at 10
    let hint_fg = blend_color(
        if (0usize+0usize)%2==0 { sq_l(th) } else { sq_d(th) }, // approx avg
        (0, 255, 140)  // vivid neon green target
    );
    // Dot tint: blend square bg toward gold
    let dot_tint = |is_light: bool| {
        blend_color(if is_light { sq_l(th) } else { sq_d(th) }, (220, 200, 40))
    };
    // Full highlight square: blend toward bright teal
    let hl_color = |is_light: bool| {
        blend_color(if is_light { sq_l(th) } else { sq_d(th) },
            if is_light { (180, 230, 50) } else { (20, 200, 130) })
    };

    let mut lines: Vec<Line> = vec![];
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for &c in &col_ord { spans.push(Span::styled(format!("{:^width$}",(b'a'+c as u8) as char,width=CELL_W), sty(DIM))); }
        lines.push(Line::from(spans));
    }

    for &r in &row_ord {
        for line_idx in 0..CELL_H {
            let is_mid = line_idx == CELL_H/2;
            let mut spans: Vec<Span> = vec![];
            if app.cfg.show_coords {
                spans.push(Span::styled(if is_mid{format!(" {} ",8-r)}else{"   ".to_string()}, sty(DIM)));
            }
            for &c in &col_ord {
                let is_light = (r+c)%2==0;
                let piece    = app.gs.board[r][c];
                let is_sel   = app.selected==Some((r,c));
                let is_tgt   = tgts.contains(&(r,c));
                let is_kchk  = king_chk==Some((r,c));
                let is_cur   = cur_bc==(r,c);
                let is_last  = !is_sel&&!is_kchk&&last.map(|h|h.from==(r,c)||h.to==(r,c)).unwrap_or(false);

                let bg = if is_sel       { sel_bg }
                    else if is_kchk      { chk_bg }
                    else if is_cur       { cur_bg }
                    else if is_tgt && matches!(app.cfg.move_hints,MoveHints::Highlight) { hl_color(is_light) }
                // Dots mode: tint the square background slightly so dot stands out
                else if is_tgt && matches!(app.cfg.move_hints,MoveHints::Dots) && piece.is_none() {
                    dot_tint(is_light)
                }
                    else if is_last      { lm_color(is_light) }
                    else if is_light     { sq_l(th) }
                    else                 { sq_d(th) };

                let is_blocks = matches!(app.cfg.piece_style, PieceStyle::Blocks);

                let (cell_str, mut style) = if is_blocks {
                    // Block art: every row of the cell renders a slice of the piece shape
                    let (row_str, fg) = if let Some(p) = piece {
                        let row = piece_block_row(p.k, line_idx);
                        let fg  = if p.c == PC::White { pw(th) } else { pb_c(th) };
                        (row.to_string(), fg)
                    } else if is_tgt && matches!(app.cfg.move_hints, MoveHints::Dots) && line_idx == CELL_H/2 {
                        (center_str("⬤", CELL_W), hint_fg)
                    } else {
                        (" ".repeat(CELL_W), bg)
                    };
                    (row_str, Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD))
                } else {
                    // Classic: piece symbol on middle line only
                    let s = if is_mid {
                        if let Some(p) = piece { center_str(&piece_sym(p,&app.cfg.piece_style), CELL_W) }
                        else if is_tgt && matches!(app.cfg.move_hints,MoveHints::Dots) { center_str("⬤", CELL_W) }
                        else { " ".repeat(CELL_W) }
                    } else if line_idx==CELL_H-1 && is_tgt && piece.is_some() {
                        format!("  {:─<width$}  ","",width=CELL_W.saturating_sub(4))
                    } else { " ".repeat(CELL_W) };
                    let mut st = Style::default().bg(bg);
                    if let Some(p) = piece {
                        st = st.fg(if p.c==PC::White{pw(th)}else{pb_c(th)}).add_modifier(Modifier::BOLD);
                    } else if is_tgt {
                        st = st.fg(hint_fg).add_modifier(Modifier::BOLD);
                    }
                    (s, st)
                };
                spans.push(Span::styled(cell_str, style));
            }
            if app.cfg.show_coords {
                spans.push(Span::styled(if is_mid{format!(" {} ",8-r)}else{"   ".to_string()}, sty(DIM)));
            }
            lines.push(Line::from(spans));
        }
    }

    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for &c in &col_ord { spans.push(Span::styled(format!("{:^width$}",(b'a'+c as u8) as char,width=CELL_W), sty(DIM))); }
        lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(ac(th))).style(Style::default().bg(bg0(th)))
                .title(Span::styled(" BOARD ", sty(ac(th)).add_modifier(Modifier::BOLD)))
        ),
        area,
    );
}

fn piece_sym(p: crate::engine::Piece, style: &PieceStyle) -> String {
    let letter = match p.k { Kind::K=>'K',Kind::Q=>'Q',Kind::R=>'R',Kind::B=>'B',Kind::N=>'N',Kind::P=>'P' };
    style.render(p.sym(), letter, p.c==PC::White)
}

fn center_str(s: &str, w: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    if len >= w { return chars.iter().take(w).collect(); }
    let pad = w-len; let left = pad/2;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(pad-left))
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
        (format!("  [{}]  {}{}█", cur_name, sel_str, app.input_buf), CREAM)
    } else if app.selected.is_some() {
        (format!("  [{}]  {}navigate to target + Enter", cur_name, sel_str), TEAL)
    } else {
        (format!("  [{}]  type move (e2e4 · Nf3 · O-O) or navigate{}", cur_name, dm_hint), DIM)
    };

    let (hint_line, hint_col) = if let Some(err)=&app.input_err {
        (format!("  ✗  {}", err), RED)
    } else if !app.input_buf.is_empty() {
        ("  Enter execute  ·  Esc cancel  ·  Backspace delete".to_string(), DIMMER)
    } else {
        ("  arrows/hjkl  ·  letter=notation  ·  Shift+E=PNG  ·  d=draw  ·  r=replay".to_string(), DIMMER)
    };

    let border_col = if !app.input_buf.is_empty(){ac(th)} else if app.input_err.is_some(){RED} else {DIM};
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
fn draw_sidebar(app: &App, f: &mut Frame, area: Rect) {
    match app.cfg.ui_mode {
        UiMode::Minimal => {
            let [pl, st, hi, kb] = Layout::vertical([
                Constraint::Length(7), Constraint::Length(5),
                Constraint::Min(3),    Constraint::Length(8),
            ]).areas(area);
            draw_players(app, f, pl);
            draw_status(app, f, st);
            draw_history(app, f, hi);
            draw_keybinds(app, f, kb);
        }
        UiMode::Standard => {
            let [pl, st, an, hi, kb] = Layout::vertical([
                Constraint::Length(7), Constraint::Length(5),
                Constraint::Length(5), Constraint::Min(3),
                Constraint::Length(8),
            ]).areas(area);
            draw_players(app, f, pl);
            draw_status(app, f, st);
            draw_analysis_mini(app, f, an);
            draw_history(app, f, hi);
            draw_keybinds(app, f, kb);
        }
        UiMode::Analysis => {
            let [pl, st, an, hi, kb] = Layout::vertical([
                Constraint::Length(7), Constraint::Length(5),
                Constraint::Length(7), Constraint::Min(3),
                Constraint::Length(8),
            ]).areas(area);
            draw_players(app, f, pl);
            draw_status(app, f, st);
            draw_analysis_full(app, f, an);
            draw_history(app, f, hi);
            draw_keybinds(app, f, kb);
        }
    }
}

fn panel_block(title: &'static str, th: Theme) -> Block<'static> {
    Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
        .title(Span::styled(format!(" {} ", title), sty(ac(th))))
}

fn draw_players(app: &App, f: &mut Frame, area: Rect) {
    let th  = app.cfg.theme;
    let iw  = app.gs.turn == PC::White;
    let ib  = app.gs.turn == PC::Black;
    let cap_w: String = app.gs.cap_w.iter().map(|p| p.sym()).collect::<Vec<_>>().join("");
    let cap_b: String = app.gs.cap_b.iter().map(|p| p.sym()).collect::<Vec<_>>().join("");
    let w_tag = match app.mode { Mode::CPU if app.player_color==PC::White=>"[YOU]", Mode::CPU=>"[CPU]", _=>"" };
    let b_tag = match app.mode { Mode::CPU if app.player_color==PC::Black=>"[YOU]", Mode::CPU=>"[CPU]", _=>"" };

    // Material advantage (+N shown next to the leading side)
    let piece_val = |k: Kind| -> i32 {
        match k { Kind::P=>1, Kind::N=>3, Kind::B=>3, Kind::R=>5, Kind::Q=>9, Kind::K=>0 }
    };
    let mat_w: i32 = app.gs.cap_w.iter().map(|p| piece_val(p.k)).sum();
    let mat_b: i32 = app.gs.cap_b.iter().map(|p| piece_val(p.k)).sum();
    let adv_w = mat_w - mat_b;
    let adv_str = |adv: i32| if adv > 0 { format!("+{}", adv) } else { String::new() };

    let clock_col = |ms_opt: Option<u64>, active: bool| -> Color {
        match ms_opt {
            None    => if active{TEAL}else{DIMMER},
            Some(m) if m<30_000 => RED,
            Some(_) if active   => TEAL,
            _                   => DIMMER,
        }
    };
    let flagged = app.gs.clock_state == ClockState::Flagged;
    let wtime   = Gs::fmt_time(app.gs.white_ms);
    let btime   = Gs::fmt_time(app.gs.black_ms);
    let wcol    = if flagged&&!iw{RED}else{clock_col(app.gs.white_ms,iw)};
    let bcol    = if flagged&& iw{RED}else{clock_col(app.gs.black_ms,ib)};
    let has_clk = app.gs.white_ms.is_some();

    let lines = vec![
        Line::from(vec![
            Span::styled(if ib{"▶ "}else{"  "}, sty(ac(th))),
            Span::styled("♚ BLACK ", sty(DIM).add_modifier(Modifier::BOLD)),
            Span::styled(b_tag, sty(DIMMER)),
            Span::styled(format!("{:<4}", adv_str(-adv_w)), sty(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(if has_clk{format!("{:>6}",btime.trim())}else{String::new()},
                sty(bcol).add_modifier(if ib&&has_clk{Modifier::BOLD}else{Modifier::empty()})),
        ]),
        Line::from(vec![Span::raw("  "), Span::styled(if cap_w.is_empty(){"  —"}else{&cap_w}, sty(DIM))]),
        Line::from(vec![Span::styled(format!("  {:─<28}",""), sty(DIMMER))]),
        Line::from(vec![
            Span::styled(if iw{"▶ "}else{"  "}, sty(ac(th))),
            Span::styled("♔ WHITE ", sty(pw(th)).add_modifier(Modifier::BOLD)),
            Span::styled(w_tag, sty(DIMMER)),
            Span::styled(format!("{:<4}", adv_str(adv_w)), sty(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(if has_clk{format!("{:>6}",wtime.trim())}else{String::new()},
                sty(wcol).add_modifier(if iw&&has_clk{Modifier::BOLD}else{Modifier::empty()})),
        ]),
        Line::from(vec![Span::raw("  "), Span::styled(if cap_b.is_empty(){"  —"}else{&cap_b}, sty(CREAM))]),
    ];
    f.render_widget(Paragraph::new(lines).block(panel_block("PLAYERS", th)), area);
}

fn draw_status(app: &App, f: &mut Frame, area: Rect) {
    let th = app.cfg.theme;
    let (s, col) = status_str(app);
    let clock   = if app.cfg.show_clock { format!(" Move {}  ", app.gs.fullmove) } else { String::new() };
    let last_mv = app.gs.history.last().map(|h| {
        let who = if h.color==PC::White{"W"}else{"B"};
        format!("  last: [{}] {}", who, h.san)
    }).unwrap_or_default();

    let info_line = if app.png_notice.is_some() {
        let path  = app.png_export_path.as_deref().unwrap_or("");
        let fname = path.split('/').last().unwrap_or(path);
        (format!("  + PNG saved: {}", fname), GREEN)
    } else if app.pgn_notice.is_some() {
        let path  = app.pgn_saved_path.as_deref().unwrap_or("");
        let fname = path.split('/').last().unwrap_or(path);
        (format!("  + PGN saved: {}", fname), TEAL)
    } else {
        (String::new(), DIMMER)
    };

    let lines = vec![
        Line::from(vec![Span::raw(" "), Span::styled(&s, sty(col).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(format!("{}{}", clock, last_mv), sty(DIM))]),
        Line::from(vec![Span::styled(info_line.0, sty(info_line.1))]),
        Line::from(vec![Span::styled(
            if !app.undo_stack.is_empty() { format!("  {} undo available", app.undo_stack.len()) } else { String::new() },
            sty(DIMMER),
        )]),
    ];
    f.render_widget(Paragraph::new(lines).block(panel_block("STATUS", th)), area);
}

fn draw_history(app: &App, f: &mut Frame, area: Rect) {
    let th  = app.cfg.theme;
    let h   = &app.gs.history;
    let cap = area.height.saturating_sub(2) as usize;

    let mut pairs: Vec<(usize,String,String,bool,bool,Option<MoveLabel>,Option<MoveLabel>)> = vec![];
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

    let start = pairs.len().saturating_sub(cap);
    let mut lines: Vec<Line> = vec![];
    if pairs.is_empty() {
        lines.push(Line::from(vec![Span::styled(" — no moves yet —", sty(DIMMER))]));
    } else {
        for (n, w, b, wl, bl, wlab, blab) in &pairs[start..] {
            let w_sty = if *wl {
                Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD)
            } else { sty(CREAM).add_modifier(Modifier::BOLD) };
            let b_sty = if *bl {
                Style::default().fg(bg0(th)).bg(Color::Rgb(80,180,140)).add_modifier(Modifier::BOLD)
            } else { sty(DIM) };
            let wlc = wlab.map(|l| label_color(l, th)).unwrap_or(DIMMER);
            let blc = blab.map(|l| label_color(l, th)).unwrap_or(DIMMER);
            let wi  = wlab.map(|l| l.icon()).unwrap_or("");
            let bi  = blab.map(|l| l.icon()).unwrap_or("");
            // Format move times compactly
            let wt = app.gs.history.get(i).map(|h| fmt_move_time(h.move_time_ms)).unwrap_or_default();
            let bt = app.gs.history.get(i+1).map(|h| fmt_move_time(h.move_time_ms)).unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled(format!(" {:>3}. ", n), sty(DIMMER)),
                Span::styled(format!("{:<8}", w), w_sty),
                Span::styled(format!("{:<3}", wi), sty(wlc)),
                Span::styled(format!("{:<4} ", wt), sty(DIMMER)),
                Span::styled(format!("{:<8}", b), b_sty),
                Span::styled(format!("{:<3}", bi), sty(blc)),
            ]));
        }
    }
    f.render_widget(Paragraph::new(lines).block(panel_block("HISTORY", th)), area);
}

fn draw_keybinds(app: &App, f: &mut Frame, area: Rect) {
    let th = app.cfg.theme;
    let mode_label = match app.cfg.ui_mode {
        UiMode::Minimal  => "min",
        UiMode::Standard => "std",
        UiMode::Analysis => "ana",
    };
    let lines = vec![
        Line::from(vec![Span::styled(" ↑↓←→ / hjkl   move cursor", sty(DIM))]),
        Line::from(vec![Span::styled(" Enter / Spc    select & move", sty(DIM))]),
        Line::from(vec![Span::styled(" mouse click    select & move", sty(DIM))]),
        Line::from(vec![Span::styled(" any letter     type notation", sty(DIM))]),
        Line::from(vec![
            Span::styled(" E ", sty(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" PNG  ", sty(DIM)),
            Span::styled(" G ", sty(TEAL).add_modifier(Modifier::BOLD)),
            Span::styled(" PGN  ", sty(DIM)),
            Span::styled(" T ", sty(AMBER).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" mode[{}]", mode_label), sty(DIM)),
        ]),
        Line::from(vec![Span::styled(" d  draw  u  undo  r  replay", sty(DIM))]),
        Line::from(vec![Span::styled(" n  new   s  settings  q  menu", sty(DIM))]),
    ];
    f.render_widget(Paragraph::new(lines).block(panel_block("KEYS", th)), area);
}

// ── GAME OVER OVERLAY ─────────────────────────────────────────────────────────
fn draw_gameover(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(54, 16, f.area());
    f.render_widget(Clear, rect);

    let flagged = app.gs.clock_state == ClockState::Flagged;
    let is_draw = app.game_end == GameEnd::Draw;

    let (title, title_col, body) = if flagged {
        let loser = app.gs.turn.name(); let winner = app.gs.turn.opp().name();
        ("  ⏰  FLAG FALL  ⏰".to_string(), RED,
         vec![format!("  {} ran out of time!", loser), format!("  {} wins on time.", winner)])
    } else if is_draw {
        ("  ½  DRAW AGREED  ½".to_string(), AMBER,
         vec!["  The game is drawn.".to_string(), "  Both players agreed.".to_string()])
    } else {
        match app.gs.status {
            Status::Checkmate => {
                let w=app.gs.turn.opp().name(); let l=app.gs.turn.name();
                ("  ♛  CHECKMATE  ♛".to_string(), Color::Rgb(210,175,60),
                 vec![format!("  {} wins!", w), format!("  {} king is mated.", l)])
            }
            Status::Stalemate => (
                "  ½  STALEMATE  ½".to_string(), AMBER,
                vec!["  The game is drawn.".to_string(), "  No legal moves — stalemate.".to_string()]),
            _ => return,
        }
    };

    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(title_col)).style(Style::default().bg(bg1(th)))
            .title(Span::styled(&title, sty(title_col).add_modifier(Modifier::BOLD))),
        rect,
    );
    let inner = pad(rect, 2, 1);
    let total  = (app.gs.history.len()+1) / 2;

    let save_line = if app.png_notice.is_some() {
        let path = app.png_export_path.as_deref().unwrap_or("");
        (format!("  + PNG saved: {}", path.split('/').last().unwrap_or(path)), GREEN)
    } else if app.pgn_notice.is_some() {
        let path = app.pgn_saved_path.as_deref().unwrap_or("");
        (format!("  + PGN saved: {}", path.split('/').last().unwrap_or(path)), TEAL)
    } else if let Some(p) = &app.pgn_saved_path {
        (format!("  PGN: {}", p.split('/').last().unwrap_or(p)), TEAL)
    } else {
        (String::new(), DIMMER)
    };

    let fen = app.current_fen();
    let fen_display = if fen.len() > 46 { format!("{}…", &fen[..45]) } else { fen };

    let mut lines = vec![Line::from("")];
    for b in &body {
        lines.push(Line::from(vec![Span::styled(b.as_str(), sty(CREAM).add_modifier(Modifier::BOLD))]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(format!("  {} moves played", total), sty(DIM))]));
    lines.push(Line::from(vec![Span::styled(format!("  {}", fen_display), sty(DIMMER))]));
    if !save_line.0.is_empty() {
        lines.push(Line::from(vec![Span::styled(&save_line.0, sty(save_line.1))]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  n  new", sty(ac(th)).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("r  replay", sty(TEAL).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("E  PNG", sty(GREEN).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("G  PGN", sty(TEAL).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("u  undo", sty(DIM)),
        Span::raw("  "),
        Span::styled("q  menu", sty(DIM)),
    ]));
    f.render_widget(Paragraph::new(lines), inner);
}

// ── PNG PREVIEW OVERLAY ───────────────────────────────────────────────────────
fn draw_png_preview(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    // Popup: 56 wide, 24 tall
    let rect = center(56, 24, f.area());
    f.render_widget(Clear, rect);
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(GREEN)).style(Style::default().bg(bg1(th)))
            .title(Span::styled(
                format!(" 📸  EXPORT BOARD AS PNG  — v{} ", VERSION),
                sty(GREEN).add_modifier(Modifier::BOLD),
            )),
        rect,
    );
    let inner = pad(rect, 2, 1);

    let board   = &app.gs.board;
    let flipped = app.flipped();
    let row_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };
    let col_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };

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
    let mut file_row: Vec<Span> = vec![Span::styled("   ", sty(DIMMER))];
    for &c in &col_ord {
        file_row.push(Span::styled(
            format!("{} ", (b'a'+c as u8) as char),
            sty(DIM),
        ));
    }
    lines.push(Line::from(file_row));

    // Board rows
    for &r in &row_ord {
        let mut row_spans = vec![
            Span::styled(format!(" {} ", 8-r), sty(DIM)),
        ];
        for &c in &col_ord {
            let is_light = (r+c)%2 == 0;
            let bg_c     = if is_light { sq_l(th) } else { sq_d(th) };
            let (txt, fg) = match board[r][c] {
                None    => ("  ".to_string(), DIMMER),
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
    let mut file_row2: Vec<Span> = vec![Span::styled("   ", sty(DIMMER))];
    for &c in &col_ord {
        file_row2.push(Span::styled(
            format!("{} ", (b'a'+c as u8) as char),
            sty(DIM),
        ));
    }
    lines.push(Line::from(file_row2));

    // Save path
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  Save to:", sty(DIM))]));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", path_display),
        sty(GREEN).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        format!("  {:─<48}",""),
        sty(DIMMER),
    )]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Y / Enter  ", sty(GREEN).add_modifier(Modifier::BOLD)),
        Span::styled("save PNG", sty(CREAM).add_modifier(Modifier::BOLD)),
        Span::raw("          "),
        Span::styled("N / Esc  ", sty(RED)),
        Span::styled("cancel", sty(DIM)),
    ]));

    f.render_widget(Paragraph::new(lines), inner);
}

// ── DRAW OFFER OVERLAY ────────────────────────────────────────────────────────
fn draw_draw_offer(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(46, 11, f.area());
    f.render_widget(Clear, rect);
    let offerer   = app.draw_offer.map(|c| c.name()).unwrap_or("Unknown");
    let responder = app.draw_offer.map(|c| c.opp().name()).unwrap_or("Unknown");
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(AMBER)).style(Style::default().bg(bg1(th)))
            .title(Span::styled("  ½  DRAW OFFER  ½", sty(AMBER).add_modifier(Modifier::BOLD))),
        rect,
    );
    let inner = pad(rect, 2, 1);
    f.render_widget(Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {} offers a draw.", offerer), sty(CREAM).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(format!("  {}, do you accept?", responder), sty(CREAM))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<38}",""), sty(DIMMER))]),
        Line::from(""),
        Line::from(vec![Span::styled("  Y / Enter  accept draw", sty(TEAL).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("  N / Esc    decline & keep playing", sty(RED))]),
    ]), inner);
}

// ── REPLAY ────────────────────────────────────────────────────────────────────
fn draw_replay(app: &App, f: &mut Frame) {
    let th = app.cfg.theme;
    let a  = f.area();
    let cell_w = app.cfg.cell_w as u16;
    f.render_widget(Block::default().style(Style::default().bg(bg0(th))), a);

    let bw = (3 + 8*cell_w + 3 + 2).min(a.width);
    let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(a);
    let [board_col, side_col] = Layout::horizontal([Constraint::Length(bw), Constraint::Min(0)]).areas(body);

    let snap  = app.replay_snaps.get(app.replay_idx);
    let title = format!(" ♟ RChess — REPLAY  [{}/{}]  {}",
        app.replay_idx, app.replay_snaps.len().saturating_sub(1), app.replay_result);
    f.render_widget(Paragraph::new(title).style(Style::default().fg(TEAL).bg(bg1(th))), top);

    // Split board column: narrow eval bar on left, board on right
    let [eval_bar_col, actual_board_col] = Layout::horizontal([
        Constraint::Length(4),
        Constraint::Min(0),
    ]).areas(board_col);

    if let Some(snap) = snap { draw_replay_board(app, f, actual_board_col, snap); }

    // Draw vertical eval bar
    let cur_eval = app.eval_history.get(app.replay_idx).copied().unwrap_or(0);
    draw_eval_bar(cur_eval, app.engine_busy, f, eval_bar_col);

    draw_replay_sidebar(app, f, side_col);
}

fn draw_replay_board(app: &App, f: &mut Frame, area: Rect, snap: &ReplaySnap) {
    #[allow(non_snake_case)]
    let CELL_W = app.cfg.cell_w as usize;
    #[allow(non_snake_case)]
    let CELL_H = app.cfg.cell_h as usize;
    let th      = app.cfg.theme;
    let flipped = app.flipped();
    let row_ord: Vec<usize> = if flipped{(0..8).rev().collect()}else{(0..8).collect()};
    let col_ord: Vec<usize> = if flipped{(0..8).rev().collect()}else{(0..8).collect()};

    let mut lines: Vec<Line> = vec![];
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for &c in &col_ord { spans.push(Span::styled(format!("{:^width$}",(b'a'+c as u8) as char,width=CELL_W), sty(DIM))); }
        lines.push(Line::from(spans));
    }
    for &r in &row_ord {
        for line_idx in 0..CELL_H {
            let is_mid = line_idx == CELL_H/2;
            let mut spans: Vec<Span> = vec![];
            if app.cfg.show_coords { spans.push(Span::styled(if is_mid{format!(" {} ",8-r)}else{"   ".to_string()}, sty(DIM))); }
            for &c in &col_ord {
                let piece    = snap.board[r][c];
                let is_light = (r+c)%2==0;
                let bg = if is_light { sq_l(th) } else { sq_d(th) };
                let cell = if is_mid {
                    if let Some(p)=piece { center_str(&piece_sym(p,&app.cfg.piece_style),CELL_W) }
                    else { " ".repeat(CELL_W) }
                } else { " ".repeat(CELL_W) };
                let mut style = Style::default().bg(bg);
                if let Some(p)=piece {
                    style=style.fg(if p.c==PC::White{pw(th)}else{pb_c(th)}).add_modifier(Modifier::BOLD);
                }
                spans.push(Span::styled(cell, style));
            }
            if app.cfg.show_coords { spans.push(Span::styled(if is_mid{format!(" {} ",8-r)}else{"   ".to_string()}, sty(DIM))); }
            lines.push(Line::from(spans));
        }
    }
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for &c in &col_ord { spans.push(Span::styled(format!("{:^width$}",(b'a'+c as u8) as char,width=CELL_W), sty(DIM))); }
        lines.push(Line::from(spans));
    }
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(TEAL)).style(Style::default().bg(bg0(th)))
                .title(Span::styled(" REPLAY ", sty(TEAL).add_modifier(Modifier::BOLD)))
        ),
        area,
    );
}


/// Render a compact ASCII sparkline of eval history.
/// eval_history[0] = start (0), eval_history[N] = after move N.
/// Width = number of chars to use.
fn eval_sparkline(eval_history: &[i32], width: usize) -> (String, Vec<Color>) {
    if eval_history.len() < 2 || width < 3 {
        return ("no data".to_string(), vec![]);
    }
    // Sample at most `width` points
    let n = eval_history.len();
    let step = if n <= width { 1 } else { n / width };
    let samples: Vec<i32> = (0..n).step_by(step.max(1)).map(|i| eval_history[i]).collect();
    let samples = &samples[..samples.len().min(width)];

    let max_abs = samples.iter().map(|v| v.abs()).max().unwrap_or(1).max(50) as f32;
    // 5 levels: ▁▂▃▄▅▆▇█  (use 4: ▁▃▅▇ for clarity)
    let bars = ["▁","▃","▅","▇","█"];
    let mid  = 2usize; // index of zero bar

    let mut out = String::new();
    let mut colors = vec![];
    for &ev in samples {
        let norm = (ev as f32 / max_abs).clamp(-1.0, 1.0); // -1..1
        // Map to bar index 0..4 (0=deep black, 4=deep white)
        let idx = ((norm + 1.0) / 2.0 * 4.0).round() as usize;
        let idx = idx.min(4);
        out.push_str(bars[idx]);
        let col = if ev > 30 {
            Color::Rgb(120, 220, 160) // green = white better
        } else if ev < -30 {
            Color::Rgb(200, 70, 70)   // red = black better
        } else {
            Color::Rgb(110, 130, 110) // neutral
        };
        colors.push(col);
    }
    (out, colors)
}


// ── VERTICAL EVAL BAR ─────────────────────────────────────────────────────────
// Shows who is better — like chess.com's vertical bar on the left of the board.
// eval_cp: White-positive centipawns. Positive = White better, Negative = Black better.
fn draw_eval_bar(eval_cp: i32, busy: bool, f: &mut Frame, area: Rect) {
    let total = area.height as i32;
    if total < 4 { return; }

    // Clamp eval to ±10 pawns, map to 0..total (0=Black winning, total=White winning)
    let clamped = (eval_cp.max(-1000).min(1000)) as f32 / 100.0; // -10..10
    let frac    = ((clamped + 10.0) / 20.0).clamp(0.0, 1.0);      // 0..1
    // White is at the BOTTOM of the bar (rank 1), Black at top
    let white_rows = ((frac * total as f32) as i32).max(1).min(total - 1);
    let black_rows = total - white_rows;

    let mut lines: Vec<Line> = vec![];

    // Black section (top)
    for i in 0..black_rows as usize {
        let is_first = i == 0;
        let lbl = if is_first && !busy {
            let p = eval_cp.abs() as f32 / 100.0;
            if eval_cp < -20 { format!("{:.1}", p) } else { " ".to_string() }
        } else { " ".to_string() };
        lines.push(Line::from(vec![
            Span::styled(format!("{:<4}", lbl),
                Style::default().bg(Color::Rgb(20,20,20)).fg(Color::Rgb(200,200,200)))
        ]));
    }

    // White section (bottom)
    for i in 0..white_rows as usize {
        let is_last = i == white_rows as usize - 1;
        let lbl = if is_last && !busy {
            let p = eval_cp as f32 / 100.0;
            if eval_cp > 20 { format!("{:.1}", p) } else { " ".to_string() }
        } else { " ".to_string() };
        lines.push(Line::from(vec![
            Span::styled(format!("{:<4}", lbl),
                Style::default().bg(Color::Rgb(230,230,210)).fg(Color::Rgb(30,30,30)))
        ]));
    }

    // If equal or busy show a tiny indicator in the middle
    if busy || eval_cp.abs() < 20 {
        let mid = (total / 2) as usize;
        if mid < lines.len() {
            let lbl = if busy { "~   " } else { "=   " };
            lines[mid] = Line::from(vec![
                Span::styled(lbl, Style::default()
                    .bg(Color::Rgb(120,120,80)).fg(Color::Rgb(240,220,40))
                    .add_modifier(Modifier::BOLD))
            ]);
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn draw_replay_sidebar(app: &App, f: &mut Frame, area: Rect) {
    let th = app.cfg.theme;
    let [info_area, review_area, hist_area, keys_area] = Layout::vertical([
        Constraint::Length(5), Constraint::Length(7),
        Constraint::Min(3),    Constraint::Length(6),
    ]).areas(area);

    // ── Position info ─────────────────────────────────────────────────────────
    let snap    = app.replay_snaps.get(app.replay_idx);
    let mv_info = if app.replay_idx == 0 { "  Starting position".to_string() } else {
        let mnum = (app.replay_idx + 1) / 2;
        let who  = if app.replay_idx % 2 == 1 { "W" } else { "B" };
        let note = snap.map(|s| s.notation.as_str()).unwrap_or("");
        format!("  Move {}. [{}] {}", mnum, who, note)
    };
    let progress = format!("  {}/{}", app.replay_idx, app.replay_snaps.len().saturating_sub(1));
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(&mv_info, sty(CREAM).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(&progress, sty(DIM))]),
        Line::from(vec![Span::styled(format!("  Result: {}", app.replay_result), sty(AMBER))]),
    ]).block(panel_block("POSITION", th)), info_area);

    // ── Eval sparkline ────────────────────────────────────────────────────────
    let spark_width = (review_area.width.saturating_sub(4)) as usize;
    let (spark_str, spark_cols) = eval_sparkline(&app.eval_history, spark_width);
    let has_spark = !spark_str.is_empty() && app.eval_history.len() > 1;

    // ── Review for this move ──────────────────────────────────────────────────
    // replay_idx 0 = start (no move), idx N = after move N-1 in history
    let review_move_idx = if app.replay_idx == 0 { None } else {
        Some(app.replay_idx - 1)
    };
    let rev = review_move_idx.and_then(|i| app.move_reviews.get(i));
    let rev_lines: Vec<Line> = if let Some(rev) = rev {
        let col     = label_color(rev.label, th);
        let delta_p = rev.delta_cp as f32 / 100.0;
        let sign    = if delta_p > 0.0 { "+" } else { "" };
        let ecol    = if delta_p < -1.0 { RED } else if delta_p < -0.5 { AMBER } else { GREEN };
        vec![
            Line::from(vec![
                Span::styled(" Move  ", sty(DIMMER)),
                Span::styled(trunc(&rev.san, 10), sty(CREAM).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Label ", sty(DIMMER)),
                Span::styled(format!("[{}] {}", rev.label.icon(), rev.label.name()),
                    sty(col).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Delta ", sty(DIMMER)),
                Span::styled(format!("{}{:.2}p", sign, delta_p),
                    sty(ecol).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Eval  ", sty(DIMMER)),
                Span::styled(format!("{:.2}", rev.eval_before), sty(DIM)),
                Span::styled(" -> ", sty(DIMMER)),
                Span::styled(format!("{:.2}", rev.eval_after), sty(ecol)),
            ]),
            Line::from(vec![
                Span::styled(" Best  ", sty(DIMMER)),
                Span::styled(trunc(rev.best_move.as_deref().unwrap_or("--"), 10),
                    sty(TEAL).add_modifier(Modifier::BOLD)),
            ]),
        ]
    } else if app.replay_idx == 0 {
        vec![
            Line::from(vec![Span::styled(" Starting position", sty(DIMMER))]),
            Line::from(""), Line::from(""), Line::from(""), Line::from(""),
        ]
    } else {
        vec![
            Line::from(vec![Span::styled(" No analysis yet", sty(DIMMER))]),
            Line::from(vec![Span::styled(" (play game first)", sty(DIMMER))]),
            Line::from(""), Line::from(""), Line::from(""),
        ]
    };
    // Append sparkline as last line of review panel
    let mut rev_lines_with_spark = rev_lines;
    if has_spark {
        let chars: Vec<char> = spark_str.chars().collect();
        let mut spark_spans: Vec<Span> = vec![Span::styled(" ", sty(DIMMER))];
        for (i, ch) in chars.iter().enumerate() {
            let col = spark_cols.get(i).copied().unwrap_or(DIM);
            spark_spans.push(Span::styled(ch.to_string(), sty(col)));
        }
        rev_lines_with_spark.push(Line::from(spark_spans));
    }
    let busy  = if app.engine_busy { " ~" } else { " +" };
    let eng   = trunc(&app.analysis.engine_name, 12);
    let rtitle = format!(" REVIEW{} [{}] ", busy, eng);
    f.render_widget(
        Paragraph::new(rev_lines_with_spark).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
                .title(Span::styled(rtitle, sty(ac(th))))
        ),
        review_area,
    );

    // ── History with current-move highlight + review labels ───────────────────
    let h   = &app.gs.history;
    let cap = hist_area.height.saturating_sub(2) as usize;
    let mut pairs: Vec<(usize, String, String, bool, bool, Option<MoveLabel>, Option<MoveLabel>)> = vec![];
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
    let hist_lines: Vec<Line> = pairs[start..].iter().map(|(n,w,b,wc,bc,wl,bl)| {
        let w_label_str = wl.map(|l| format!(" {}", l.icon())).unwrap_or_default();
        let b_label_str = bl.map(|l| format!(" {}", l.icon())).unwrap_or_default();
        let w_lc = wl.map(|l| label_color(l, th)).unwrap_or(DIMMER);
        let b_lc = bl.map(|l| label_color(l, th)).unwrap_or(DIMMER);
        Line::from(vec![
            Span::styled(format!(" {:>3}. ", n), sty(DIMMER)),
            Span::styled(format!("{:<8}", w),
                if *wc { Style::default().fg(bg0(th)).bg(TEAL).add_modifier(Modifier::BOLD) } else { sty(CREAM) }),
            Span::styled(format!("{:<3}", w_label_str), sty(w_lc)),
            Span::styled(format!("{:<8}", b),
                if *bc { Style::default().fg(bg0(th)).bg(TEAL).add_modifier(Modifier::BOLD) } else { sty(DIM) }),
            Span::styled(format!("{:<3}", b_label_str), sty(b_lc)),
        ])
    }).collect();
    f.render_widget(Paragraph::new(hist_lines).block(panel_block("HISTORY", th)), hist_area);

    // ── Keys ──────────────────────────────────────────────────────────────────
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(" ←/h prev   →/l next   0 start   $ end", sty(DIM))]),
        Line::from(vec![
            Span::styled(" E ", sty(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" export PNG   ", sty(DIM)),
            Span::styled(" q ", sty(DIM)),
            Span::styled(" back", sty(DIM)),
        ]),
        Line::from(vec![Span::styled(" Labels: + Good  ?! Inaccuracy  ? Mistake  ?? Blunder", sty(DIMMER))]),
        Line::from(""),
    ]).block(panel_block("REPLAY KEYS", th)), keys_area);
}

// ── PROMOTION ─────────────────────────────────────────────────────────────────
fn draw_promo(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(50, 9, f.area());
    f.render_widget(Clear, rect);
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" PROMOTE PAWN ", sty(ac(th)).add_modifier(Modifier::BOLD))),
        rect,
    );
    let inner = pad(rect, 2, 1);
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
            if s{Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD)}else{sty(CREAM)}));
        row.push(Span::raw("  "));
    }
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(" Select promotion piece:", sty(CREAM))]),
        Line::from(""),
        Line::from(row),
        Line::from(""),
        Line::from(vec![Span::styled(" ←→/hl select   Enter confirm   Esc cancel", sty(DIMMER))]),
    ]), inner);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// -- ANALYSIS PANEL --

fn label_color(label: MoveLabel, _th: Theme) -> Color {
    match label {
        MoveLabel::Book       => TEAL,
        MoveLabel::Good       => GREEN,
        MoveLabel::Inaccuracy => AMBER,
        MoveLabel::Mistake    => Color::Rgb(220, 120, 40),
        MoveLabel::Blunder    => RED,
    }
}

fn draw_analysis_mini(app: &App, f: &mut Frame, area: Rect) {
    let th    = app.cfg.theme;
    let busy  = if app.engine_busy { "~" } else { "+" };
    let eng   = trunc(&app.analysis.engine_name, 14);
    let title = format!(" EVAL {} [{}] ", busy, eng);

    let lines: Vec<Line> = if let Some(rev) = app.move_reviews.last() {
        let col     = label_color(rev.label, th);
        // delta_cp is mover-perspective centipawns (negative = bad move)
        let delta_p = rev.delta_cp as f32 / 100.0;
        let sign    = if delta_p > 0.0 { "+" } else { "" };
        let best    = trunc(rev.best_move.as_deref().unwrap_or("--"), 8);
        vec![
            Line::from(vec![
                Span::styled(format!(" [{}] ", rev.label.icon()), sty(col).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<11}", rev.label.name()), sty(col)),
                Span::styled(format!("{}{:.2}p", sign, delta_p), sty(DIMMER)),
            ]),
            Line::from(vec![
                Span::styled(" Best: ", sty(DIMMER)),
                Span::styled(trunc(rev.best_move.as_deref().unwrap_or("--"), 8),
                    sty(TEAL).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {:.2}", rev.eval_after), sty(DIMMER)),
            ]),
            Line::from(""),
        ]
    } else if app.engine_busy {
        vec![Line::from(vec![Span::styled(" analysing...", sty(DIMMER))]),
             Line::from(""), Line::from("")]
    } else {
        vec![Line::from(vec![Span::styled(" make a move to analyse", sty(DIMMER))]),
             Line::from(""), Line::from("")]
    };

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
                .title(Span::styled(title, sty(ac(th))))
        ),
        area,
    );
}


/// Full 5-line analysis panel shown in Analysis mode.
fn draw_analysis_full(app: &App, f: &mut Frame, area: Rect) {
    let th    = app.cfg.theme;
    let busy  = if app.engine_busy { "~" } else { "+" };
    let eng   = trunc(&app.analysis.engine_name, 18);
    let title = format!(" ANALYSIS {} [{}] ", busy, eng);

    let lines: Vec<Line> = if let Some(rev) = app.move_reviews.last() {
        let col     = label_color(rev.label, th);
        let delta_p = rev.delta_cp as f32 / 100.0;
        let sign    = if delta_p > 0.0 { "+" } else { "" };
        let ecol    = if delta_p < -1.0 { RED } else if delta_p < -0.5 { AMBER } else { GREEN };
        let san     = trunc(&rev.san, 12);
        let mut out = vec![
            Line::from(vec![
                Span::styled(" Move  ", sty(DIMMER)),
                Span::styled(san, sty(CREAM).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Label ", sty(DIMMER)),
                Span::styled(format!("[{}] {}", rev.label.icon(), rev.label.name()),
                    sty(col).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Delta ", sty(DIMMER)),
                Span::styled(format!("{}{:.2}p", sign, delta_p),
                    sty(ecol).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Eval  ", sty(DIMMER)),
                Span::styled(format!("{:.2}", rev.eval_before), sty(DIM)),
                Span::styled(" -> ", sty(DIMMER)),
                Span::styled(format!("{:.2}", rev.eval_after),
                    sty(ecol).add_modifier(Modifier::BOLD)),
            ]),
        ];
        if !rev.top_moves.is_empty() {
            out.push(Line::from(vec![Span::styled(" Top   ", sty(DIMMER))]));
            for (i, (mv_uci, score)) in rev.top_moves.iter().enumerate().take(3) {
                let sp = *score as f32 / 100.0;
                let sc = if sp > 0.0 { format!("{:+.2}", sp) } else { format!("{:.2}", sp) };
                let mc = if i == 0 { TEAL } else { DIM };
                out.push(Line::from(vec![
                    Span::styled(format!("   {}. ", i+1), sty(DIMMER)),
                    Span::styled(format!("{:<8}", trunc(mv_uci, 7)), sty(mc).add_modifier(Modifier::BOLD)),
                    Span::styled(sc, sty(DIMMER)),
                ]));
            }
        } else if let Some(ref bm) = rev.best_move {
            out.push(Line::from(vec![
                Span::styled(" Best  ", sty(DIMMER)),
                Span::styled(trunc(bm, 8), sty(TEAL).add_modifier(Modifier::BOLD)),
            ]));
        }
        out
    } else {
        let msg = if app.engine_busy { " analysing..." } else { " make a move to analyse" };
        vec![Line::from(""), Line::from(""),
             Line::from(vec![Span::styled(msg, sty(DIMMER))]),
             Line::from(""), Line::from("")]
    };

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
                .title(Span::styled(title, sty(ac(th))))
        ),
        area,
    );
}


fn status_str(app: &App) -> (String, Color) {
    match app.gs.status {
        Status::Checkmate =>
            (format!("✕ CHECKMATE — {} WINS", app.gs.turn.opp().name()), Color::Rgb(210,175,60)),
        Status::Stalemate if app.game_end==GameEnd::Draw =>
            ("½ DRAW AGREED".into(), AMBER),
        Status::Stalemate =>
            ("½ STALEMATE — DRAW".into(), AMBER),
        Status::Check =>
            (format!("⚠ CHECK — {} TO MOVE", app.gs.turn.name()), RED),
        _ if app.gs.clock_state==ClockState::Flagged =>
            (format!("⏰ FLAG — {} WINS", app.gs.turn.opp().name()), RED),
        _ if app.thinking => ("⏳ THINKING...".into(), DIM),
        _ => (format!("► {} TO MOVE", app.gs.turn.name()), TEAL),
    }
}

fn center(w: u16, h: u16, a: Rect) -> Rect {
    Rect::new(
        a.x + (a.width.saturating_sub(w))/2,
        a.y + (a.height.saturating_sub(h))/2,
        w.min(a.width), h.min(a.height),
    )
}
fn pad(r: Rect, px: u16, py: u16) -> Rect {
    Rect::new(r.x+px, r.y+py, r.width.saturating_sub(px*2), r.height.saturating_sub(py*2))
}


// ── PGN SAVED OVERLAY ─────────────────────────────────────────────────────────
fn draw_pgn_saved(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(62, 16, f.area());
    f.render_widget(Clear, rect);
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(TEAL)).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" G  PGN SAVED ", sty(TEAL).add_modifier(Modifier::BOLD))),
        rect,
    );
    let inner = pad(rect, 2, 1);

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
            Span::styled("  Saved PGN game notation to:", sty(DIM)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {}", display), sty(TEAL).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<54}", ""), sty(DIMMER))]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} moves  |  {}", moves, result), sty(DIM)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<54}", ""), sty(DIMMER))]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Any key ", sty(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled("  close", sty(DIM)),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

// ── MOVE TIME FORMATTER ───────────────────────────────────────────────────────
fn fmt_move_time(ms: u64) -> String {
    if ms == 0   { return String::new(); }
    if ms < 1000 { return format!("{}ms", ms); }
    format!("{:.1}s", ms as f32 / 1000.0)
}

// ── FEN INPUT SCREEN ──────────────────────────────────────────────────────────
fn draw_fen_input(app: &App, f: &mut Frame) {
    use ratatui::widgets::Wrap;
    let th   = app.cfg.theme;
    let rect = center(70, 18, f.area());
    f.render_widget(Clear, rect);
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" \u{2261} START FROM FEN ", sty(ac(th)).add_modifier(Modifier::BOLD))),
        rect,
    );
    let inner = pad(rect, 2, 1);

    let cursor = if (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() / 500) % 2 == 0 { "\u{2588}" } else { "" };

    let (input_col, hint) = if let Some(err) = &app.fen_input_err {
        (RED,  format!("\u{2717}  {}", err))
    } else {
        (CREAM, "Paste or type a FEN string and press Enter".to_string())
    };

    let example = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";

    let lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(" FEN:", sty(DIM))]),
        Line::from(vec![
            Span::styled(
                format!(" {}{}", app.fen_input_buf, cursor),
                sty(input_col).add_modifier(Modifier::BOLD),
            )
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(format!(" {}", hint), sty(if app.fen_input_err.is_some() { RED } else { DIM }))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  \u{2500}\u{2500}\u{2500} {:─<56}", ""), sty(DIMMER))]),
        Line::from(""),
        Line::from(vec![Span::styled(" Example:", sty(DIMMER))]),
        Line::from(vec![Span::styled(format!(" {}", example), sty(DIMMER))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  \u{2500}\u{2500}\u{2500} {:─<56}", ""), sty(DIMMER))]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Enter ", sty(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(" load position    ", sty(DIM)),
            Span::styled(" Esc ", sty(DIM)),
            Span::styled(" cancel", sty(DIM)),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

// ── PGN IMPORT SCREEN ─────────────────────────────────────────────────────────
fn draw_pgn_import(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(72, 22, f.area());
    f.render_widget(Clear, rect);
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" IMPORT PGN ", sty(ac(th)).add_modifier(Modifier::BOLD))),
        rect,
    );
    let inner = pad(rect, 2, 1);

    let blink = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() / 500) % 2 == 0;
    let cursor = if blink { "\u{2588}" } else { " " };

    let char_count = app.pgn_input_buf.len();
    let mode_hint = if app.pgn_input_buf.starts_with('/') || app.pgn_input_buf.starts_with('~') {
        " (file path mode)"
    } else if app.pgn_input_buf.contains('[') {
        " (PGN text mode \u{2713})"
    } else if char_count > 0 {
        " (detecting...)"
    } else {
        ""
    };

    // Show last 6 lines of what the user typed/pasted
    let all_lines: Vec<&str> = app.pgn_input_buf.lines().collect();
    let show_from = all_lines.len().saturating_sub(6);
    let visible = &all_lines[show_from..];

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(" Paste PGN text  ", sty(DIM)),
            Span::styled("or", sty(DIMMER)),
            Span::styled("  type a file path", sty(DIM)),
        ]),
        Line::from(vec![Span::styled(format!("  {:=<64}", ""), sty(DIMMER))]),
    ];

    if visible.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            format!(" {}", cursor), sty(TEAL))
        ]));
        for _ in 0..5 { lines.push(Line::from("")); }
    } else {
        let last_i = visible.len() - 1;
        for (i, pline) in visible.iter().enumerate() {
            let txt = if i == last_i {
                format!(" {}{}", pline, cursor)
            } else {
                format!(" {}", pline)
            };
            lines.push(Line::from(vec![Span::styled(txt, sty(CREAM))]));
        }
        while lines.len() < 8 { lines.push(Line::from("")); }
    }

    lines.push(Line::from(vec![Span::styled(format!("  {:=<64}", ""), sty(DIMMER))]));

    if let Some(err) = &app.pgn_input_err {
        lines.push(Line::from(vec![Span::styled(format!(" x {}", err), sty(RED))]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(format!(" {} chars{}", char_count, mode_hint), sty(DIMMER)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  Examples:", sty(DIMMER))]));
    lines.push(Line::from(vec![Span::styled(
        "  ~/rchess_export/game.pgn       <- file path", sty(DIMMER))]));
    lines.push(Line::from(vec![Span::styled(
        "  [White \"Me\"] 1. e4 e5 2. ...  <- paste text", sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Enter ", sty(GREEN).add_modifier(Modifier::BOLD)),
        Span::styled(" load    ", sty(DIM)),
        Span::styled(" Backspace ", sty(DIM)),
        Span::styled(" delete    ", sty(DIM)),
        Span::styled(" Esc ", sty(DIM)),
        Span::styled(" cancel", sty(DIM)),
    ]));

    f.render_widget(Paragraph::new(lines), inner);
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
            f.render_widget(
                Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
                    .title(Span::styled(" \u{2605} DAILY PUZZLE ", sty(ac(th)).add_modifier(Modifier::BOLD))),
                rect,
            );
            let inner = pad(rect, 2, 1);
            f.render_widget(Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![Span::styled(" \u{23f3} Fetching from Lichess...", sty(TEAL))]),
                Line::from(""),
                Line::from(vec![Span::styled(" Esc to cancel", sty(DIMMER))]),
            ]), inner);
        }
        PuzzleState::Failed(e) => {
            let has_token = !app.cfg.lichess_token.is_empty();
            let rect = center(66, 19, a);
            f.render_widget(Clear, rect);
            f.render_widget(
                Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                    .border_style(sty(RED)).style(Style::default().bg(bg1(th)))
                    .title(Span::styled(" ★ DAILY PUZZLE — FAILED ", sty(RED).add_modifier(Modifier::BOLD))),
                rect,
            );
            let inner = pad(rect, 2, 1);
            let sep = format!("  {:─<56}", "");
            let mut lines: Vec<Line> = vec![
                Line::from(""),
                Line::from(vec![Span::styled(format!(" Error: {}", e), sty(RED).add_modifier(Modifier::BOLD))]),
                Line::from(""),
                Line::from(vec![Span::styled(&sep, sty(DIMMER))]),
                Line::from(""),
            ];
            if !has_token {
                lines.push(Line::from(vec![Span::styled(" Lichess works without a token, but may rate-limit", sty(DIM))]));
                lines.push(Line::from(vec![Span::styled(" anonymous requests. A free token fixes this:", sty(DIM))]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled("  1. Sign up free at lichess.org", sty(TEAL))]));
                lines.push(Line::from(vec![Span::styled("  2. Go to lichess.org/account/security", sty(TEAL))]));
                lines.push(Line::from(vec![Span::styled("  3. Create a Personal API Token (no scopes needed)", sty(TEAL))]));
                lines.push(Line::from(vec![Span::styled("  4. Add to ~/.config/rchess/rchess_tui.conf:", sty(DIM))]));
                lines.push(Line::from(vec![Span::styled("     lichess_token = lip_xxxxxxxxxxxx", sty(AMBER).add_modifier(Modifier::BOLD))]));
            } else {
                lines.push(Line::from(vec![Span::styled(" Token is configured. Check your internet.", sty(DIM))]));
                lines.push(Line::from(vec![Span::styled(" Also try: sudo pacman -S ca-certificates", sty(DIMMER))]));
                lines.push(Line::from(""));
                lines.push(Line::from(""));
                lines.push(Line::from(""));
                lines.push(Line::from(""));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(&sep, sty(DIMMER))]));
            lines.push(Line::from(vec![
                Span::styled(" n ", sty(GREEN).add_modifier(Modifier::BOLD)),
                Span::styled(" retry    ", sty(DIM)),
                Span::styled(" Esc ", sty(DIM)),
                Span::styled(" back to menu", sty(DIM)),
            ]));
            f.render_widget(Paragraph::new(lines), inner);
        }
        state => {
            // Puzzle is loaded — show board + info panel
            let cell_w = app.cfg.cell_w as u16;
            let bw = (3 + 8 * cell_w + 3 + 2).min(a.width);
            let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(a);
            let [board_col, side_col] = Layout::horizontal([Constraint::Length(bw), Constraint::Min(0)]).areas(body);

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
                PuzzleState::CorrectMove | PuzzleState::Solved => GREEN,
                PuzzleState::WrongMove(_) => RED,
                _ => TEAL,
            };

            f.render_widget(
                Paragraph::new(format!(
                    " \u{2605} DAILY PUZZLE  Rating: {}  Themes: {}  {}",
                    rating, themes_str, status_msg
                )).style(Style::default().fg(status_col).bg(bg1(th))),
                top,
            );

            // Board (reuse draw_board logic for the puzzle position)
            draw_board(app, f, board_col);

            // Side panel
            draw_puzzle_sidebar(app, f, side_col);
        }
    }
}

fn draw_puzzle_sidebar(app: &App, f: &mut Frame, area: Rect) {
    use crate::puzzle::PuzzleState;
    let th = app.cfg.theme;
    let [info_area, input_area, keys_area] = Layout::vertical([
        Constraint::Length(8), Constraint::Length(5), Constraint::Min(3),
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
                Span::styled(" Puzzle  ", sty(DIMMER)),
                Span::styled(format!("#{}", p.id), sty(CREAM).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Rating  ", sty(DIMMER)),
                Span::styled(format!("{}", p.rating), sty(AMBER).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" You are ", sty(DIMMER)),
                Span::styled(you_are, sty(if app.gs.turn == crate::engine::Color::White { pw(th) } else { pb_c(th) }).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(if solved { " \u{2605} SOLVED!" } else if wrong { " \u{2717} Wrong" } else { " Find best move" },
                    sty(if solved { GREEN } else if wrong { RED } else { TEAL }).add_modifier(Modifier::BOLD))
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(" Themes:", sty(DIMMER))]),
            Line::from(vec![Span::styled(format!("  {}", themes), sty(DIM))]),
        ]
    } else {
        vec![Line::from("")]
    };
    f.render_widget(Paragraph::new(info_lines).block(panel_block("PUZZLE", th)), info_area);

    // Input (same as game input)
    let cur_bc   = app.to_board(app.cursor);
    let cur_name = format!("{}{}", (b'a' + cur_bc.1 as u8) as char, 8 - cur_bc.0);
    let (input_line, input_col) = if !app.input_buf.is_empty() {
        (format!("  [{}]  {}\u{2588}", cur_name, app.input_buf), CREAM)
    } else {
        (format!("  [{}]  type move or click", cur_name), DIM)
    };
    let hint = if let Some(err) = &app.input_err {
        (format!("  \u{2717}  {}", err), RED)
    } else {
        ("  e.g. Nf3  or  navigate + Enter".to_string(), DIMMER)
    };
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(input_line, sty(input_col).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(hint.0, sty(hint.1))]),
    ]).block(panel_block("INPUT", th)), input_area);

    // Keys
    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(" \u{2191}\u{2193}\u{2190}\u{2192}/hjkl  move cursor", sty(DIM))]),
        Line::from(vec![Span::styled(" Enter/click  select & move", sty(DIM))]),
        Line::from(vec![Span::styled(" n  new puzzle    q  menu", sty(DIM))]),
    ]).block(panel_block("KEYS", th)), keys_area);
}

// ── ONLINE SETUP ──────────────────────────────────────────────────────────────
fn draw_online_setup(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(60, 22, f.area());
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th))),
        rect,
    );
    let inner = pad(rect, 2, 1);

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled("  ⚡ ONLINE VS FRIEND", sty(ac(th)).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("  Real-time multiplayer over TCP", sty(DIM))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<54}", ""), sty(DIMMER))]),
        Line::from(""),
    ];

    match app.online_step {
        OnlineStep::EnterAddr => {
            lines.push(Line::from(vec![Span::styled("  SERVER ADDRESS", sty(DIM))]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  ▶ ", sty(ac(th))),
                Span::styled(
                    format!("{:<50}", &app.online_buf),
                    Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Default: 127.0.0.1:9001 (local)", sty(DIMMER))]));
            lines.push(Line::from(vec![Span::styled("  Use your host's public IP for cross-network play", sty(DIMMER))]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(format!("  {:─<54}", ""), sty(DIMMER))]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Enter confirm    Esc back to menu", sty(DIMMER))]));
        }
        OnlineStep::ChooseAction => {
            lines.push(Line::from(vec![
                Span::styled("  Server: ", sty(DIM)),
                Span::styled(&app.online_buf, sty(ac(th))),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(format!("  {:─<54}", ""), sty(DIMMER))]));
            lines.push(Line::from(""));
            for (key, label, desc) in [
                ("C", "CREATE ROOM", "Get a code to share with your friend"),
                ("J", "JOIN ROOM  ", "Enter a code your friend shared"),
            ] {
                lines.push(Line::from(vec![
                    Span::styled(format!("  [{key}]  "), sty(ac(th)).add_modifier(Modifier::BOLD)),
                    Span::styled(label, sty(CREAM).add_modifier(Modifier::BOLD)),
                ]));
                lines.push(Line::from(vec![Span::styled(format!("        {desc}"), sty(DIM))]));
                lines.push(Line::from(""));
            }
            lines.push(Line::from(vec![Span::styled(format!("  {:─<54}", ""), sty(DIMMER))]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Esc go back", sty(DIMMER))]));
        }
        OnlineStep::EnterRoom => {
            lines.push(Line::from(vec![Span::styled("  ENTER ROOM CODE", sty(DIM))]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  ▶ ", sty(ac(th))),
                Span::styled(
                    format!("{:<8}", &app.online_buf),
                    Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Ask your friend for their 6-char room code", sty(DIMMER))]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(format!("  {:─<54}", ""), sty(DIMMER))]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("  Enter join    Esc back", sty(DIMMER))]));
        }
        OnlineStep::Connecting => {
            lines.push(Line::from(vec![Span::styled("  Connecting…", sty(ac(th)).add_modifier(Modifier::BOLD))]));
        }
    }

    if !app.online_msg.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("  ⚠  {}", app.online_msg),
            Style::default().fg(Color::Yellow),
        )]));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

// ── ONLINE WAITING ────────────────────────────────────────────────────────────
fn draw_online_waiting(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(58, 20, f.area());
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th))),
        rect,
    );
    let inner = pad(rect, 2, 1);

    // Spinner animation based on tick (online_ping_tick cycles 0-99)
    let spinner = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];
    let spin = spinner[(app.online_ping_tick as usize / 5) % spinner.len()];

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled("  ⚡ ONLINE  —  WAITING", sty(ac(th)).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<50}", ""), sty(DIMMER))]),
        Line::from(""),
    ];

    if !app.online_room_code.is_empty() {
        lines.push(Line::from(vec![Span::styled("  YOUR ROOM CODE", sty(DIM))]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("       ", sty(DIM)),
            Span::styled(
                format!("  {}  ", app.online_room_code),
                Style::default()
                    .fg(bg0(th)).bg(ac(th))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled("  Share this code with your friend", sty(DIMMER))]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(format!("  {:─<50}", ""), sty(DIMMER))]));
        lines.push(Line::from(""));
    }

    // Status message
    let status = if app.online_msg.is_empty() {
        format!("{spin}  Connecting…")
    } else {
        format!("{spin}  {}", app.online_msg)
    };
    lines.push(Line::from(vec![Span::styled(format!("  {}", status), sty(ac(th)))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(format!("  {:─<50}", ""), sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  Esc / q  cancel", sty(DIMMER))]));

    f.render_widget(Paragraph::new(lines), inner);
}
