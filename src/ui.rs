// src/ui.rs — ratatui renderer
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

// ── Static palette ────────────────────────────────────────────────────────────
const BG:     Color = Color::Rgb(18, 22, 20);
const BG1:    Color = Color::Rgb(24, 30, 26);
const BG2:    Color = Color::Rgb(30, 38, 32);
const CREAM:  Color = Color::Rgb(220,210,185);
const DIM:    Color = Color::Rgb(100,120,100);
const DIMMER: Color = Color::Rgb(50,  65, 50);
const RED:    Color = Color::Rgb(200, 70, 70);
const TEAL:   Color = Color::Rgb(80, 180,140);
const AMBER:  Color = Color::Rgb(220,160, 40);

// ── Theme helpers ─────────────────────────────────────────────────────────────
fn t(c: (u8,u8,u8)) -> Color { Color::Rgb(c.0,c.1,c.2) }
fn accent(th: Theme)  -> Color { t(th.accent()) }
fn sel(th: Theme)     -> Color { t(th.select()) }
fn cursor(th: Theme)  -> Color { t(th.cursor()) }
fn sq_l(th: Theme)    -> Color { t(th.squares().0) }
fn sq_d(th: Theme)    -> Color { t(th.squares().1) }
fn pw(th: Theme)      -> Color { t(th.piece_colors().0) }
fn pb_col(th: Theme)  -> Color { t(th.piece_colors().1) }

fn sty(c: Color) -> Style { Style::default().fg(c) }

// ── Entry ─────────────────────────────────────────────────────────────────────
pub fn render(app: &App, f: &mut Frame) {
    f.render_widget(Block::default().style(Style::default().bg(BG)), f.size());
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
    let a    = f.size();
    let rect = center(54, 26, a);
    let ac   = accent(app.cfg.theme);

    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac)).style(Style::default().bg(BG1)),
        rect,
    );

    let inner = pad(rect, 2, 1);
    let opts  = ["  ♟  TWO PLAYERS", "  ◈  VS COMPUTER   [AI]", "  ⚙  SETTINGS"];

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![Span::raw("  "), Span::styled("♔ ♕ ♗ ♘ ♖ ♙", Style::default().fg(pw(app.cfg.theme)).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::raw("  "), Span::styled("♟ ♜ ♞ ♝ ♛ ♚", sty(DIM).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("        C H E S S", sty(ac).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("     TERMINAL  EDITION", sty(DIM))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<48}",""), sty(DIMMER))]),
        Line::from(""),
    ];

    for (i, &opt) in opts.iter().enumerate() {
        let sel_row = app.menu_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if sel_row {" ▶ "} else {"   "}, sty(ac)),
            Span::styled(format!("{:<44}",opt),
                if sel_row { Style::default().fg(BG).bg(ac).add_modifier(Modifier::BOLD) }
                else       { sty(CREAM) }),
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
    let a    = f.size();
    let rect = center(52, 18, a);
    let ac   = accent(app.cfg.theme);

    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac)).style(Style::default().bg(BG1)),
        rect,
    );

    let inner = pad(rect, 2, 1);
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![Span::styled("     CHOOSE YOUR SIDE", sty(ac).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(DIMMER))]),
        Line::from(""),
    ];

    for (i, (sym, label, fg)) in [
        ("♔", "WHITE  — moves first",  pw(app.cfg.theme)),
        ("♚", "BLACK  — moves second", DIM),
    ].iter().enumerate() {
        let sel_row = app.color_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if sel_row {" ▶ "} else {"   "}, sty(ac)),
            Span::styled(format!("{} {}", sym, label),
                Style::default().fg(*fg)
                    .bg(if sel_row { BG2 } else { BG1 })
                    .add_modifier(if sel_row { Modifier::BOLD } else { Modifier::empty() })),
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
    let a    = f.size();
    let rect = center(66, 28, a);
    let ac   = accent(app.cfg.theme);

    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac)).style(Style::default().bg(BG1))
            .title(Span::styled(" ⚙  SETTINGS ", sty(ac).add_modifier(Modifier::BOLD))),
        rect,
    );

    let inner = pad(rect, 2, 1);
    let rows: &[(&str, &str)] = &[
        ("Theme",         app.cfg.theme.name()),
        ("Piece style",   app.cfg.piece_style.name()),
        ("AI difficulty", app.cfg.ai_depth.name()),
        ("Move hints",    app.cfg.move_hints.name()),
        ("Show coords",   if app.cfg.show_coords  { "ON" } else { "OFF" }),
        ("Show clock",    if app.cfg.show_clock    { "ON" } else { "OFF" }),
        ("Flip board",    if app.cfg.flip_board    { "ON" } else { "OFF" }),
        ("Confirm move",  if app.cfg.confirm_move  { "ON" } else { "OFF" }),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<58}",""), sty(DIMMER))]),
        Line::from(""),
    ];

    for (i, (label, value)) in rows.iter().enumerate() {
        let sel_row = app.settings_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if sel_row {" ▶ "} else {"   "}, sty(ac)),
            Span::styled(format!("{:<18}", label),
                if sel_row { sty(ac).add_modifier(Modifier::BOLD) } else { sty(CREAM) }),
            Span::raw("  "),
            Span::styled(
                if sel_row { format!("◀  {}  ▶", value) } else { format!("   {}   ", value) },
                if sel_row { Style::default().fg(BG).bg(ac).add_modifier(Modifier::BOLD) }
                else       { sty(DIM) }),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![Span::styled(format!("  {:─<58}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    let (hint, hcol) = if app.saved_notice.is_some() {
        ("  ✓ Saved to ~/.chess_tui.conf", TEAL)
    } else {
        ("  w save    r reset defaults    ←→ change    Esc back", DIMMER)
    };
    lines.push(Line::from(vec![Span::styled(hint, sty(hcol))]));

    f.render_widget(Paragraph::new(lines), inner);
}

// ── GAME ──────────────────────────────────────────────────────────────────────
fn draw_game(app: &App, f: &mut Frame) {
    let a = f.size();
    let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(a);
    draw_topbar(app, f, top);

    // Board area: 3 (rank) + 8×5 (cells) + 3 (rank) + 2 (border) = 48 wide
    let [left, right] = Layout::horizontal([Constraint::Length(48), Constraint::Min(0)]).areas(body);
    let [board_area, input_area] = Layout::vertical([Constraint::Min(0), Constraint::Length(4)]).areas(left);

    draw_board(app, f, board_area);
    draw_input(app, f, input_area);
    draw_sidebar(app, f, right);
}

fn draw_topbar(app: &App, f: &mut Frame, area: Rect) {
    let mode_s = match app.mode {
        Mode::PvP => "TWO PLAYERS".to_string(),
        Mode::CPU  => format!("VS CPU  [YOU:{}]  [{}]",
            app.player_color.name(), app.cfg.ai_depth.name().split_whitespace().next().unwrap_or("")),
    };
    let (st, col) = status_str(app);
    let clock     = if app.cfg.show_clock { format!("  Mv {}  ", app.gs.fullmove) } else { String::new() };
    f.render_widget(
        Paragraph::new(format!(" ♟ CHESS{}│  {}  │  {}", clock, mode_s, st))
            .style(Style::default().fg(col).bg(BG2)),
        area,
    );
}

// ── BOARD (5-wide × 2-tall cells) ────────────────────────────────────────────
fn draw_board(app: &App, f: &mut Frame, area: Rect) {
    let th      = app.cfg.theme;
    let ac      = accent(th);
    let flipped = app.flipped();
    let row_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };
    let col_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };

    let tgts: HashSet<(usize,usize)> = app.targets.iter().map(|m| m.to).collect();
    let king_chk = if app.gs.status == Status::Check { king_sq(&app.gs.board, app.gs.turn) } else { None };
    let last     = app.gs.history.last();
    let cur_bc   = app.to_board(app.cursor);

    let mut lines: Vec<Line> = vec![];

    // ── file labels top ──
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for &c in &col_ord {
            spans.push(Span::styled(format!("  {}  ", (b'a'+c as u8) as char), sty(DIM)));
        }
        lines.push(Line::from(spans));
    }

    // ── rows ──
    for &r in &row_ord {
        for half in 0..2usize {
            let mut spans: Vec<Span> = vec![];

            // rank label
            if app.cfg.show_coords {
                spans.push(Span::styled(
                    if half == 0 { format!(" {} ", 8-r) } else { "   ".to_string() },
                    sty(DIM),
                ));
            }

            for &c in &col_ord {
                let is_light = (r+c) % 2 == 0;
                let is_sel   = app.selected == Some((r,c));
                let is_tgt   = tgts.contains(&(r,c));
                let is_kchk  = king_chk == Some((r,c));
                let is_cur   = cur_bc == (r,c);
                let is_last  = last.map(|h| h.from==(r,c) || h.to==(r,c)).unwrap_or(false);
                let piece    = app.gs.board[r][c];

                // Background priority: sel > check > cursor > last > normal
                let bg = if is_sel       { sel(th) }
                    else if is_kchk      { Color::Rgb(180,40,40) }
                    else if is_cur       { cursor(th) }
                    else if is_last      {
                        let lm = th.last_move();
                        let sq = if is_light { th.squares().0 } else { th.squares().1 };
                        Color::Rgb(
                            ((sq.0 as u16*2 + lm.0 as u16) / 3) as u8,
                            ((sq.1 as u16*2 + lm.1 as u16) / 3) as u8,
                            ((sq.2 as u16*2 + lm.2 as u16) / 3) as u8,
                        )
                    }
                    else if is_tgt && matches!(app.cfg.move_hints, MoveHints::Highlight) && piece.is_none() {
                        let sq = if is_light { th.squares().0 } else { th.squares().1 };
                        Color::Rgb(
                            sq.0.saturating_add(35).min(255),
                            sq.1.saturating_add(35).min(255),
                            sq.2.saturating_add(15).min(255),
                        )
                    }
                    else if is_light     { sq_l(th) }
                    else                 { sq_d(th) };

                let cell = if half == 0 {
                    if let Some(p) = piece {
                        format!("  {}  ", piece_sym(p, &app.cfg.piece_style))
                    } else {
                        match (is_tgt, &app.cfg.move_hints) {
                            (true, MoveHints::Dots) => "  ·  ".to_string(),
                            _                       => "     ".to_string(),
                        }
                    }
                } else {
                    // Bottom half: ring around capturable targets
                    if is_tgt && piece.is_some() { " ╌╌╌ ".to_string() }
                    else { "     ".to_string() }
                };

                let mut style = Style::default().bg(bg);
                if let Some(p) = piece {
                    let fg = if p.c == PC::White { pw(th) } else { pb_col(th) };
                    style = style.fg(fg).add_modifier(Modifier::BOLD);
                } else if is_tgt {
                    style = style.fg(TEAL);
                }
                spans.push(Span::styled(cell, style));
            }

            // right rank label
            if app.cfg.show_coords {
                spans.push(Span::styled(
                    if half == 0 { format!(" {} ", 8-r) } else { "   ".to_string() },
                    sty(DIM),
                ));
            }
            lines.push(Line::from(spans));
        }
    }

    // ── file labels bottom ──
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("   ")];
        for &c in &col_ord {
            spans.push(Span::styled(format!("  {}  ", (b'a'+c as u8) as char), sty(DIM)));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(Color::Rgb(70,100,70)))
                .style(Style::default().bg(BG1))
                .title(Span::styled(" BOARD ", sty(ac)))
        ),
        area,
    );
}

fn piece_sym(p: crate::engine::Piece, style: &PieceStyle) -> String {
    let letter = match p.k { Kind::K=>'K',Kind::Q=>'Q',Kind::R=>'R',Kind::B=>'B',Kind::N=>'N',Kind::P=>'P' };
    style.render(p.sym(), letter, p.c == PC::White)
}

// ── INPUT BOX ─────────────────────────────────────────────────────────────────
fn draw_input(app: &App, f: &mut Frame, area: Rect) {
    let th  = app.cfg.theme;
    let ac  = accent(th);

    // Show cursor coordinate
    let cur_bc    = app.to_board(app.cursor);
    let cur_name  = format!("{}{}", (b'a' + cur_bc.1 as u8) as char, 8 - cur_bc.0);
    // Show selected piece too
    let sel_str   = if let Some(sel) = app.selected {
        format!("{}{}→  ", (b'a'+sel.1 as u8) as char, 8-sel.0)
    } else { String::new() };

    let (input_line, input_col) = if !app.input_buf.is_empty() {
        (format!("  [{}]  {}{}█", cur_name, sel_str, app.input_buf),
         CREAM)
    } else if app.selected.is_some() {
        (format!("  [{}]  {}navigate to target, press Enter", cur_name, sel_str),
         sty(TEAL).fg.unwrap_or(TEAL))
    } else {
        (format!("  [{}]  type a move (e2e4 · Nf3 · O-O) or navigate + Enter", cur_name),
         DIM)
    };

    let (hint_line, hint_col) = if let Some(err) = &app.input_err {
        (format!("  ✗ {}", err), RED)
    } else if !app.input_buf.is_empty() {
        ("  Enter → execute    Esc → cancel    Backspace → delete".to_string(), DIMMER)
    } else {
        ("  arrows move cursor · Enter selects/moves · any letter starts typing".to_string(), DIMMER)
    };

    let border_col = if !app.input_buf.is_empty() { ac }
        else if app.input_err.is_some() { RED }
        else { Color::Rgb(70,100,70) };

    let lines = vec![
        Line::from(vec![Span::styled(input_line, sty(input_col).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(hint_line, sty(hint_col))]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(border_col))
                .style(Style::default().bg(BG1))
        ),
        area,
    );
}

// ── SIDEBAR ───────────────────────────────────────────────────────────────────
fn draw_sidebar(app: &App, f: &mut Frame, area: Rect) {
    let [pl, st, hi, kb] = Layout::vertical([
        Constraint::Length(8),
        Constraint::Length(4),
        Constraint::Min(5),
        Constraint::Length(7),
    ]).areas(area);

    draw_players(app, f, pl);
    draw_status(app, f, st);
    draw_history(app, f, hi);
    draw_keybinds(app, f, kb);
}

fn draw_players(app: &App, f: &mut Frame, area: Rect) {
    let th  = app.cfg.theme;
    let ac  = accent(th);
    let iw  = app.gs.turn == PC::White;
    let ib  = app.gs.turn == PC::Black;

    let cap_w: String = app.gs.cap_w.iter().map(|p| p.sym()).collect::<Vec<_>>().join("");
    let cap_b: String = app.gs.cap_b.iter().map(|p| p.sym()).collect::<Vec<_>>().join("");
    let w_tag = match app.mode { Mode::CPU if app.player_color==PC::White=>"[YOU]", Mode::CPU=>"[CPU]", _=>"" };
    let b_tag = match app.mode { Mode::CPU if app.player_color==PC::Black=>"[YOU]", Mode::CPU=>"[CPU]", _=>"" };

    let lines = vec![
        Line::from(vec![
            Span::styled(if ib{"▶ "}else{"  "}, sty(ac)),
            Span::styled("♚ BLACK ", sty(DIM).add_modifier(Modifier::BOLD)),
            Span::styled(b_tag, sty(DIMMER)),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(if cap_w.is_empty(){"—"}else{&cap_w}, sty(DIM)),
        ]),
        Line::from(vec![Span::styled(format!("  {:─<28}",""), sty(DIMMER))]),
        Line::from(vec![
            Span::styled(if iw{"▶ "}else{"  "}, sty(ac)),
            Span::styled("♔ WHITE ", sty(pw(th)).add_modifier(Modifier::BOLD)),
            Span::styled(w_tag, sty(DIMMER)),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(if cap_b.is_empty(){"—"}else{&cap_b}, sty(CREAM)),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(Color::Rgb(70,100,70)))
                .style(Style::default().bg(BG1))
                .title(Span::styled(" PLAYERS ", sty(ac)))
        ),
        area,
    );
}

fn draw_status(app: &App, f: &mut Frame, area: Rect) {
    let ac = accent(app.cfg.theme);
    let (s, col) = status_str(app);
    let clock = if app.cfg.show_clock { format!("Move {}  ", app.gs.fullmove) } else { String::new() };

    let lines = vec![
        Line::from(vec![Span::raw(" "), Span::styled(&s, sty(col).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(format!(" {}", clock), sty(DIM))]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(Color::Rgb(70,100,70)))
                .style(Style::default().bg(BG1))
                .title(Span::styled(" STATUS ", sty(ac)))
        ),
        area,
    );
}

fn draw_history(app: &App, f: &mut Frame, area: Rect) {
    let ac  = accent(app.cfg.theme);
    let h   = &app.gs.history;
    let cap = area.height.saturating_sub(2) as usize;

    let mut pairs: Vec<(usize, String, String)> = vec![];
    let mut i = 0; let mut n = 1;
    while i < h.len() {
        let w = h[i].notation.clone();
        let b = h.get(i+1).map(|e| e.notation.clone()).unwrap_or_default();
        pairs.push((n, w, b)); i += 2; n += 1;
    }

    let start  = pairs.len().saturating_sub(cap);
    let mut lines: Vec<Line> = vec![];

    if pairs.is_empty() {
        lines.push(Line::from(vec![Span::styled(" — no moves yet —", sty(DIMMER))]));
    } else {
        for (n, w, b) in &pairs[start..] {
            lines.push(Line::from(vec![
                Span::styled(format!(" {:>3}. ", n), sty(DIM)),
                Span::styled(format!("{:<8}", w), sty(CREAM).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<8}", b), sty(DIM)),
            ]));
        }
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(Color::Rgb(70,100,70)))
                .style(Style::default().bg(BG1))
                .title(Span::styled(" HISTORY ", sty(ac)))
        ),
        area,
    );
}

fn draw_keybinds(app: &App, f: &mut Frame, area: Rect) {
    let ac = accent(app.cfg.theme);
    let lines = vec![
        Line::from(vec![Span::styled(" ↑↓←→        move cursor", sty(DIM))]),
        Line::from(vec![Span::styled(" hjkl        also move cursor", sty(DIMMER))]),
        Line::from(vec![Span::styled(" Enter/Spc   select · confirm move", sty(DIM))]),
        Line::from(vec![Span::styled(" any letter  start typing notation", sty(DIM))]),
        Line::from(vec![Span::styled(" n  new      s  settings   q  menu", sty(DIM))]),
    ];
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .border_style(sty(Color::Rgb(70,100,70)))
                .style(Style::default().bg(BG1))
                .title(Span::styled(" KEYS ", sty(ac)))
        ),
        area,
    );
}

// ── PROMO OVERLAY ─────────────────────────────────────────────────────────────
fn draw_promo(app: &App, f: &mut Frame) {
    let a    = f.size();
    let rect = center(50, 9, a);
    let ac   = accent(app.cfg.theme);
    f.render_widget(Clear, rect);

    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac)).style(Style::default().bg(BG2))
            .title(Span::styled(" PROMOTE PAWN ", sty(ac).add_modifier(Modifier::BOLD))),
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
        let sel_row = app.promo_cur == i;
        row.push(Span::styled(
            format!(" {} {} ", sym, name),
            if sel_row { Style::default().fg(BG).bg(ac).add_modifier(Modifier::BOLD) }
            else       { sty(CREAM) },
        ));
        row.push(Span::raw("  "));
    }

    f.render_widget(Paragraph::new(vec![
        Line::from(vec![Span::styled(" Select promotion piece:", sty(CREAM))]),
        Line::from(""),
        Line::from(row),
        Line::from(""),
        Line::from(vec![Span::styled(" ←→ / hl choose   Enter confirm   Esc cancel", sty(DIMMER))]),
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
