// src/ui.rs
use std::collections::HashSet;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use crate::app::{App, Mode, Screen};
use crate::config::{MoveHints, PieceStyle, Theme};
use crate::engine::{Color as PC, Kind, Status, king_sq};

// ── Theme-derived colours ─────────────────────────────────────────────────────
fn rgb(c: (u8,u8,u8)) -> Color { Color::Rgb(c.0,c.1,c.2) }
fn bg0(th: Theme)  -> Color { rgb(th.bg_colors().0) }
fn bg1(th: Theme)  -> Color { rgb(th.bg_colors().1) }
fn ac(th: Theme)   -> Color { rgb(th.accent()) }
fn sq_l(th: Theme) -> Color { rgb(th.squares().0) }
fn sq_d(th: Theme) -> Color { rgb(th.squares().1) }
fn pw(th: Theme)   -> Color { rgb(th.piece_colors().0) }
fn pb(th: Theme)   -> Color { rgb(th.piece_colors().1) }
fn sty(c: Color)   -> Style { Style::default().fg(c) }

// ── Static fallback colours (independent of theme) ────────────────────────────
const CREAM:  Color = Color::Rgb(220,210,185);
const DIM:    Color = Color::Rgb(110,130,110);
const DIMMER: Color = Color::Rgb(55, 70, 55);
const RED:    Color = Color::Rgb(200, 70, 70);
const TEAL:   Color = Color::Rgb(80, 180,140);
const AMBER:  Color = Color::Rgb(220,160, 40);

// ── Cell dimensions ───────────────────────────────────────────────────────────
// Each square is CELL_W chars wide, CELL_H lines tall.
// Board inner width  = 8 * CELL_W = 56
// Board total width  = 56 + 3 (left rank) + 3 (right rank) + 2 (border) = 64
// Board inner height = 8 * CELL_H = 24
// Board total height = 24 + 1 (top files) + 1 (bot files) + 2 (border) = 28
const CELL_W: usize = 7;
const CELL_H: usize = 3;

// ── Entry ─────────────────────────────────────────────────────────────────────
pub fn render(app: &App, f: &mut Frame) {
    let th = app.cfg.theme;
    f.render_widget(Block::default().style(Style::default().bg(bg0(th))), f.size());
    match app.screen {
        Screen::Menu      => draw_menu(app, f),
        Screen::ColorPick => draw_color_pick(app, f),
        Screen::Settings  => draw_settings(app, f),
        Screen::Game      => draw_game(app, f),
        Screen::Promo     => { draw_game(app, f); draw_promo(app, f); }
    }
}

// ── MENU ──────────────────────────────────────────────────────────────────────
fn draw_menu(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(56, 26, f.size());
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th))),
        rect,
    );
    let inner = pad(rect, 2, 1);
    let opts  = ["  ♟  TWO PLAYERS", "  ◈  VS COMPUTER   [AI]", "  ⚙  SETTINGS"];

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![Span::raw("  "), Span::styled("♔ ♕ ♗ ♘ ♖ ♙", sty(pw(th)).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::raw("  "), Span::styled("♟ ♜ ♞ ♝ ♛ ♚", sty(DIM).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("        C H E S S", sty(ac(th)).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("     TERMINAL  EDITION", sty(DIM))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<48}",""), sty(DIMMER))]),
        Line::from(""),
    ];
    for (i, &opt) in opts.iter().enumerate() {
        let s = app.menu_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s{" ▶ "}else{"   "}, sty(ac(th))),
            Span::styled(format!("{:<44}",opt),
                if s { Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD) }
                else { sty(CREAM) }),
        ]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(format!("  {:─<48}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  ↑↓ navigate    Enter select    q quit", sty(DIMMER))]));
    f.render_widget(Paragraph::new(lines), inner);
}

// ── COLOR PICK ────────────────────────────────────────────────────────────────
fn draw_color_pick(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(52, 18, f.size());
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th))),
        rect,
    );
    let inner = pad(rect, 2, 1);
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![Span::styled("     CHOOSE YOUR SIDE", sty(ac(th)).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(DIMMER))]),
        Line::from(""),
    ];
    for (i, (sym, label, fg)) in [
        ("♔","WHITE  — moves first",  pw(th)),
        ("♚","BLACK  — moves second", DIM),
    ].iter().enumerate() {
        let s = app.color_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s{" ▶ "}else{"   "}, sty(ac(th))),
            Span::styled(format!("{} {}", sym, label),
                Style::default().fg(*fg).bg(if s { bg0(th) } else { bg1(th) })
                    .add_modifier(if s { Modifier::BOLD } else { Modifier::empty() })),
        ]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("  ↑↓ navigate   Enter confirm   Esc back", sty(DIMMER))]));
    f.render_widget(Paragraph::new(lines), inner);
}

// ── SETTINGS ─────────────────────────────────────────────────────────────────
fn draw_settings(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(68, 28, f.size());
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" ⚙  SETTINGS ", sty(ac(th)).add_modifier(Modifier::BOLD))),
        rect,
    );
    let inner = pad(rect, 2, 1);
    let rows: &[(&str, &str)] = &[
        ("Theme",         app.cfg.theme.name()),
        ("Piece style",   app.cfg.piece_style.name()),
        ("AI difficulty", app.cfg.ai_depth.name()),
        ("Move hints",    app.cfg.move_hints.name()),
        ("Show coords",   if app.cfg.show_coords  {"ON"} else {"OFF"}),
        ("Show clock",    if app.cfg.show_clock    {"ON"} else {"OFF"}),
        ("Flip board",    if app.cfg.flip_board    {"ON"} else {"OFF"}),
        ("Confirm move",  if app.cfg.confirm_move  {"ON"} else {"OFF"}),
    ];
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<60}",""), sty(DIMMER))]),
        Line::from(""),
    ];
    for (i,(label,value)) in rows.iter().enumerate() {
        let s = app.settings_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s{" ▶ "}else{"   "}, sty(ac(th))),
            Span::styled(format!("{:<18}", label),
                if s { sty(ac(th)).add_modifier(Modifier::BOLD) } else { sty(CREAM) }),
            Span::raw("  "),
            Span::styled(
                if s { format!("◀  {}  ▶", value) } else { format!("   {}   ", value) },
                if s { Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD) }
                else { sty(DIM) }),
        ]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(format!("  {:─<60}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    let (hint, hcol) = if app.saved_notice.is_some() {
        ("  ✓ Saved to ~/.chess_tui.conf", TEAL)
    } else {
        ("  w save    r reset    ←→ change value    Esc back", DIMMER)
    };
    lines.push(Line::from(vec![Span::styled(hint, sty(hcol))]));
    f.render_widget(Paragraph::new(lines), inner);
}

// ── GAME ──────────────────────────────────────────────────────────────────────
fn draw_game(app: &App, f: &mut Frame) {
    let th = app.cfg.theme;
    let a  = f.size();

    // board width = 3 + 8*CELL_W + 3 + 2 borders = 64
    // but we give it a bit more breathing room: 66
    let board_w = (3 + 8 * CELL_W as u16 + 3 + 2).min(a.width);
    let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(a);
    let [board_col, side_col] = Layout::horizontal([Constraint::Length(board_w), Constraint::Min(0)]).areas(body);
    let [board_area, input_area] = Layout::vertical([Constraint::Min(0), Constraint::Length(4)]).areas(board_col);

    draw_topbar(app, f, top);
    draw_board(app, f, board_area);
    draw_input(app, f, input_area);
    draw_sidebar(app, f, side_col);
    // Game-over overlay on top of everything
    if matches!(app.gs.status, Status::Checkmate | Status::Stalemate) {
        draw_gameover(app, f);
    }
}

fn draw_topbar(app: &App, f: &mut Frame, area: Rect) {
    let th     = app.cfg.theme;
    let mode_s = match app.mode {
        Mode::PvP => "TWO PLAYERS".to_string(),
        Mode::CPU  => format!("VS CPU  [YOU:{}]  [{}]",
            app.player_color.name(),
            app.cfg.ai_depth.name().split_whitespace().next().unwrap_or("")),
    };
    let (st, col) = status_str(app);
    let clock = if app.cfg.show_clock { format!("  MV {}  ", app.gs.fullmove) } else { String::new() };
    f.render_widget(
        Paragraph::new(format!(" ♟ CHESS{}│  {}  │  {}", clock, mode_s, st))
            .style(Style::default().fg(col).bg(bg1(th))),
        area,
    );
}

// ── BOARD ─────────────────────────────────────────────────────────────────────
// ── BOARD ─────────────────────────────────────────────────────────────────────
fn draw_board(app: &App, f: &mut Frame, area: Rect) {
    let th      = app.cfg.theme;
    let flipped = app.flipped();
    let row_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };
    let col_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };

    let tgts: HashSet<(usize,usize)> = app.targets.iter().map(|m| m.to).collect();
    let king_chk = if app.gs.status == Status::Check { king_sq(&app.gs.board, app.gs.turn) } else { None };
    let last     = app.gs.history.last();
    let cur_bc   = app.to_board(app.cursor);

    let sel_bg  = rgb(th.select());
    let cur_bg  = rgb(th.cursor());
    let chk_bg  = Color::Rgb(180, 40, 40);
    let dot_col = TEAL;

    let lm_color = |is_light: bool| {
        let sq = if is_light { th.squares().0 } else { th.squares().1 };
        let lm = th.last_move();
        Color::Rgb(
            ((sq.0 as u16 * 2 + lm.0 as u16) / 3) as u8,
            ((sq.1 as u16 * 2 + lm.1 as u16) / 3) as u8,
            ((sq.2 as u16 * 2 + lm.2 as u16) / 3) as u8,
        )
    };
    let hl_color = |is_light: bool| {
        let sq = if is_light { th.squares().0 } else { th.squares().1 };
        Color::Rgb(
            sq.0.saturating_add(30).min(255),
            sq.1.saturating_add(30).min(255),
            sq.2.saturating_add(10).min(255),
        )
    };

    let mut lines: Vec<Line> = vec![];

    // file labels top
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for &c in &col_ord {
            spans.push(Span::styled(
                format!("{:^width$}", (b'a'+c as u8) as char, width=CELL_W),
                sty(DIM),
            ));
        }
        lines.push(Line::from(spans));
    }

    // rows
    for &r in &row_ord {
        for line_idx in 0..CELL_H {
            let is_mid = line_idx == CELL_H / 2;
            let mut spans: Vec<Span> = vec![];

            // left rank label
            if app.cfg.show_coords {
                spans.push(Span::styled(
                    if is_mid { format!(" {} ", 8-r) } else { "   ".to_string() },
                    sty(DIM),
                ));
            }

            for &c in &col_ord {
                let is_light = (r+c) % 2 == 0;
                let piece    = app.gs.board[r][c];
                let is_sel   = app.selected == Some((r,c));
                let is_tgt   = tgts.contains(&(r,c));
                let is_kchk  = king_chk == Some((r,c));
                // cursor is ALWAYS visible, even with a piece selected
                let is_cur   = cur_bc == (r,c);
                let is_last  = !is_sel && !is_kchk
                    && last.map(|h| h.from==(r,c) || h.to==(r,c)).unwrap_or(false);

                let bg = if is_sel  { sel_bg }
                    else if is_kchk { chk_bg }
                    else if is_cur  { cur_bg }
                    else if is_tgt && matches!(app.cfg.move_hints, MoveHints::Highlight) && piece.is_none() {
                        hl_color(is_light)
                    }
                    else if is_last { lm_color(is_light) }
                    else if is_light { sq_l(th) }
                    else             { sq_d(th) };

                let cell_str = if is_mid {
                    if let Some(p) = piece {
                        center_str(&piece_sym(p, &app.cfg.piece_style), CELL_W)
                    } else if is_tgt && matches!(app.cfg.move_hints, MoveHints::Dots) {
                        center_str("·", CELL_W)
                    } else {
                        " ".repeat(CELL_W)
                    }
                } else if line_idx == CELL_H - 1 && is_tgt && piece.is_some() {
                    format!("  {:─<width$}  ", "", width = CELL_W.saturating_sub(4))
                } else {
                    " ".repeat(CELL_W)
                };

                let mut style = Style::default().bg(bg);
                if let Some(p) = piece {
                    style = style
                        .fg(if p.c == PC::White { pw(th) } else { pb(th) })
                        .add_modifier(Modifier::BOLD);
                } else if is_tgt {
                    style = style.fg(dot_col);
                }

                spans.push(Span::styled(cell_str, style));
            }

            // right rank label
            if app.cfg.show_coords {
                spans.push(Span::styled(
                    if is_mid { format!(" {} ", 8-r) } else { "   ".to_string() },
                    sty(DIM),
                ));
            }
            lines.push(Line::from(spans));
        }
    }

    // file labels bottom
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for &c in &col_ord {
            spans.push(Span::styled(
                format!("{:^width$}", (b'a'+c as u8) as char, width=CELL_W),
                sty(DIM),
            ));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(ac(th)))
                .style(Style::default().bg(bg0(th)))
                .title(Span::styled(" BOARD ", sty(ac(th)).add_modifier(Modifier::BOLD)))
        ),
        area,
    );
}

/// Centre a string in a field of `width` chars (space-padded).
fn center_str(s: &str, width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    if len >= width { return chars.iter().take(width).collect(); }
    let pad = width - len;
    let left = pad / 2;
    let right = pad - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}

fn piece_sym(p: crate::engine::Piece, style: &PieceStyle) -> String {
    let letter = match p.k { Kind::K=>'K',Kind::Q=>'Q',Kind::R=>'R',Kind::B=>'B',Kind::N=>'N',Kind::P=>'P' };
    style.render(p.sym(), letter, p.c == PC::White)
}

// ── INPUT BOX ─────────────────────────────────────────────────────────────────
fn draw_input(app: &App, f: &mut Frame, area: Rect) {
    let th  = app.cfg.theme;
    let cur_bc   = app.to_board(app.cursor);
    let cur_name = format!("{}{}", (b'a' + cur_bc.1 as u8) as char, 8 - cur_bc.0);
    let sel_str  = if let Some(sel) = app.selected {
        format!("{}{}→ ", (b'a'+sel.1 as u8) as char, 8-sel.0)
    } else { String::new() };

    let (input_line, input_col) = if !app.input_buf.is_empty() {
        (format!("  [{}]  {}{}█", cur_name, sel_str, app.input_buf), CREAM)
    } else if app.selected.is_some() {
        (format!("  [{}]  {}navigate to target + Enter", cur_name, sel_str), TEAL)
    } else {
        (format!("  [{}]  type move (e2e4 · Nf3 · O-O) or use cursor + Enter", cur_name), DIM)
    };

    let (hint_line, hint_col) = if let Some(err) = &app.input_err {
        (format!("  ✗  {}", err), RED)
    } else if !app.input_buf.is_empty() {
        ("  Enter execute  ·  Esc cancel  ·  Backspace delete".to_string(), DIMMER)
    } else {
        ("  arrows/hjkl cursor  ·  any letter starts notation  ·  Esc deselect".to_string(), DIMMER)
    };

    let border_col = if !app.input_buf.is_empty()   { ac(th) }
                     else if app.input_err.is_some() { RED }
                     else { DIM };

    let lines = vec![
        Line::from(vec![Span::styled(input_line, sty(input_col).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(hint_line, sty(hint_col))]),
    ];
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(border_col))
                .style(Style::default().bg(bg1(th)))
        ),
        area,
    );
}

// ── SIDEBAR ───────────────────────────────────────────────────────────────────
fn draw_sidebar(app: &App, f: &mut Frame, area: Rect) {
    let [pl, st, hi, kb] = Layout::vertical([
        Constraint::Length(8),
        Constraint::Length(5),
        Constraint::Min(5),
        Constraint::Length(8),
    ]).areas(area);
    draw_players(app, f, pl);
    draw_status(app, f, st);
    draw_history(app, f, hi);
    draw_keybinds(app, f, kb);
}

fn panel_block(title: &'static str, th: Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(sty(ac(th)))
        .style(Style::default().bg(bg1(th)))
        .title(Span::styled(format!(" {} ", title), sty(ac(th))))
}

fn draw_players(app: &App, f: &mut Frame, area: Rect) {
    let th  = app.cfg.theme;
    let iw  = app.gs.turn == PC::White;
    let ib  = app.gs.turn == PC::Black;
    let cap_w: String = app.gs.cap_w.iter().map(|p| p.sym()).collect::<Vec<_>>().join(" ");
    let cap_b: String = app.gs.cap_b.iter().map(|p| p.sym()).collect::<Vec<_>>().join(" ");
    let w_tag = match app.mode { Mode::CPU if app.player_color==PC::White=>"[YOU]", Mode::CPU=>"[CPU]", _=>"" };
    let b_tag = match app.mode { Mode::CPU if app.player_color==PC::Black=>"[YOU]", Mode::CPU=>"[CPU]", _=>"" };

    let lines = vec![
        Line::from(vec![
            Span::styled(if ib{"▶ "}else{"  "}, sty(ac(th))),
            Span::styled("♚ BLACK ", sty(DIM).add_modifier(Modifier::BOLD)),
            Span::styled(b_tag, sty(DIMMER)),
        ]),
        Line::from(vec![Span::raw("  "), Span::styled(if cap_w.is_empty(){"—"}else{&cap_w}, sty(DIM))]),
        Line::from(vec![Span::styled(format!("  {:─<28}",""), sty(DIMMER))]),
        Line::from(vec![
            Span::styled(if iw{"▶ "}else{"  "}, sty(ac(th))),
            Span::styled("♔ WHITE ", sty(pw(th)).add_modifier(Modifier::BOLD)),
            Span::styled(w_tag, sty(DIMMER)),
        ]),
        Line::from(vec![Span::raw("  "), Span::styled(if cap_b.is_empty(){"—"}else{&cap_b}, sty(CREAM))]),
    ];
    f.render_widget(Paragraph::new(lines).block(panel_block("PLAYERS", th)), area);
}

fn draw_status(app: &App, f: &mut Frame, area: Rect) {
    let th = app.cfg.theme;
    let (s, col) = status_str(app);
    let clock = if app.cfg.show_clock { format!(" Move {}  ", app.gs.fullmove) } else { String::new() };
    let last_mv = app.gs.history.last().map(|h| {
        let who = if h.color == crate::engine::Color::White { "W" } else { "B" };
        format!("  last: [{}] {}", who, h.notation)
    }).unwrap_or_default();
    let undo_info = if !app.undo_stack.is_empty() {
        format!("  undo: {} available", app.undo_stack.len())
    } else { String::new() };
    let lines = vec![
        Line::from(vec![Span::raw(" "), Span::styled(&s, sty(col).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(format!("{}{}", clock, last_mv), sty(DIM))]),
        Line::from(vec![Span::styled(undo_info, sty(DIMMER))]),
    ];
    f.render_widget(Paragraph::new(lines).block(panel_block("STATUS", th)), area);
}

fn draw_history(app: &App, f: &mut Frame, area: Rect) {
    let th  = app.cfg.theme;
    let h   = &app.gs.history;
    let cap = area.height.saturating_sub(2) as usize;
    let mut pairs: Vec<(usize, String, String)> = vec![];
    let mut i = 0; let mut n = 1;
    while i < h.len() {
        let w = h[i].notation.clone();
        let b = h.get(i+1).map(|e| e.notation.clone()).unwrap_or_default();
        pairs.push((n, w, b)); i += 2; n += 1;
    }
    let start = pairs.len().saturating_sub(cap);
    let mut lines: Vec<Line> = vec![];
    if pairs.is_empty() {
        lines.push(Line::from(vec![Span::styled(" — no moves yet —", sty(DIMMER))]));
    } else {
        for (n, w, b) in &pairs[start..] {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:>3}. ", n), sty(DIM)),
                Span::styled(format!("{:<9}", w), sty(CREAM).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<9}", b), sty(DIM)),
            ]));
        }
    }
    f.render_widget(Paragraph::new(lines).block(panel_block("HISTORY", th)), area);
}

fn draw_keybinds(app: &App, f: &mut Frame, area: Rect) {
    let th    = app.cfg.theme;
    let lines = vec![
        Line::from(vec![Span::styled(" ↑↓←→        move cursor on board", sty(DIM))]),
        Line::from(vec![Span::styled(" hjkl        also move cursor", sty(DIMMER))]),
        Line::from(vec![Span::styled(" Enter/Spc   select piece / confirm move", sty(DIM))]),
        Line::from(vec![Span::styled(" any letter  start typing notation", sty(DIM))]),
        Line::from(vec![Span::styled(" u undo move (PvP=1ply, Easy AI=1ply, else 2)", sty(DIM))]),
        Line::from(vec![Span::styled(" n new game  s settings  q main menu", sty(DIM))]),
    ];
    f.render_widget(Paragraph::new(lines).block(panel_block("KEYS", th)), area);
}

// ── GAME OVER OVERLAY ────────────────────────────────────────────────────────
fn draw_gameover(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(46, 11, f.size());
    f.render_widget(Clear, rect);

    let (title_str, title_col, body_lines) = match app.gs.status {
        Status::Checkmate => {
            let winner = app.gs.turn.opp().name();
            let loser  = app.gs.turn.name();
            (
                format!("  ♛  CHECKMATE  ♛"),
                Color::Rgb(210,175,60),
                vec![
                    format!("  {} wins!", winner),
                    format!("  {} king is mated.", loser),
                ]
            )
        }
        Status::Stalemate => (
            "  ½  STALEMATE  ½".to_string(),
            AMBER,
            vec![
                "  The game is drawn.".to_string(),
                "  No legal moves, not in check.".to_string(),
            ]
        ),
        _ => return,
    };

    let total_moves = app.gs.history.len();

    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(title_col))
            .style(Style::default().bg(bg1(th)))
            .title(Span::styled(title_str, sty(title_col).add_modifier(Modifier::BOLD))),
        rect,
    );

    let inner = pad(rect, 2, 1);
    let mut lines: Vec<Line> = vec![Line::from("")];
    for b in &body_lines {
        lines.push(Line::from(vec![Span::styled(b.as_str(), sty(CREAM).add_modifier(Modifier::BOLD))]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("  Game length: {} moves", (total_moves + 1) / 2), sty(DIM)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(format!("  {:─<38}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  n  new game", sty(ac(th)).add_modifier(Modifier::BOLD)),
        Span::raw("    "),
        Span::styled("u  undo last move", sty(DIM)),
        Span::raw("    "),
        Span::styled("q  menu", sty(DIM)),
    ]));

    f.render_widget(Paragraph::new(lines), inner);
}

// ── PROMO ─────────────────────────────────────────────────────────────────────
fn draw_promo(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(50, 9, f.size());
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
        (if color==PC::White{"♕"}else{"♛"}, "Queen"),
        (if color==PC::White{"♖"}else{"♜"}, "Rook"),
        (if color==PC::White{"♗"}else{"♝"}, "Bishop"),
        (if color==PC::White{"♘"}else{"♞"}, "Knight"),
    ];
    let mut row: Vec<Span> = vec![Span::raw(" ")];
    for (i, (sym, name)) in pieces.iter().enumerate() {
        let s = app.promo_cur == i;
        row.push(Span::styled(
            format!(" {} {} ", sym, name),
            if s { Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD) }
            else { sty(CREAM) },
        ));
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
fn status_str(app: &App) -> (String, Color) {
    match app.gs.status {
        Status::Checkmate =>
            (format!("✕ CHECKMATE — {} WINS", app.gs.turn.opp().name()), Color::Rgb(200,170,60)),
        Status::Stalemate =>
            ("½ STALEMATE — DRAW".into(), AMBER),
        Status::Check =>
            (format!("⚠ CHECK — {} TO MOVE", app.gs.turn.name()), RED),
        Status::Active if app.thinking =>
            ("⏳ THINKING...".into(), DIM),
        Status::Active =>
            (format!("► {} TO MOVE", app.gs.turn.name()), TEAL),
    }
}

fn center(w: u16, h: u16, a: Rect) -> Rect {
    Rect::new(
        a.x + (a.width.saturating_sub(w)) / 2,
        a.y + (a.height.saturating_sub(h)) / 2,
        w.min(a.width),
        h.min(a.height),
    )
}
fn pad(r: Rect, px: u16, py: u16) -> Rect {
    Rect::new(r.x+px, r.y+py, r.width.saturating_sub(px*2), r.height.saturating_sub(py*2))
}
