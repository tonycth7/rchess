// src/ui.rs — Full TUI renderer, theme-aware
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use crate::app::{App, Mode, Screen};
use crate::config::{MoveHints, PieceStyle, Theme};
use crate::engine::{Color as PC, Kind, Status};

// ── Static palette (non-theme colours) ───────────────────────────────────────
const BG:     Color = Color::Rgb(18, 22, 20);
const BG1:    Color = Color::Rgb(24, 30, 26);
const BG2:    Color = Color::Rgb(30, 38, 32);
const CREAM:  Color = Color::Rgb(220,210,185);
const DIM:    Color = Color::Rgb(100,120,100);
const DIMMER: Color = Color::Rgb(50, 65, 50);
const RED:    Color = Color::Rgb(200, 70, 70);
const TEAL:   Color = Color::Rgb(80, 180,140);
const AMBER:  Color = Color::Rgb(220,160, 40);
const PW:     Color = Color::Rgb(240,235,210);
const PB:     Color = Color::Rgb(28, 18,  8);

// ── Theme-derived colours ─────────────────────────────────────────────────────
fn sq_light(t: Theme)  -> Color { let (l,_)=t.squares(); Color::Rgb(l.0,l.1,l.2) }
fn sq_dark(t: Theme)   -> Color { let (_,d)=t.squares(); Color::Rgb(d.0,d.1,d.2) }
fn accent(t: Theme)    -> Color { let a=t.accent();      Color::Rgb(a.0,a.1,a.2) }
fn sel_col(t: Theme)   -> Color { let s=t.select();      Color::Rgb(s.0,s.1,s.2) }

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

    let block = Block::default()
        .borders(Borders::ALL).border_type(BorderType::Double)
        .border_style(sty(ac)).style(Style::default().bg(BG1));
    f.render_widget(block, rect);

    let inner = pad(rect, 2, 1);
    let opts  = ["  ♟  TWO PLAYERS", "  ◈  VS COMPUTER   [AI]", "  ⚙  SETTINGS"];

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("♔ ♕ ♗ ♘ ♖ ♙", Style::default().fg(PW).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("♟ ♜ ♞ ♝ ♛ ♚", sty(DIM).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("        C H E S S", sty(ac).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled("     TERMINAL  EDITION", sty(DIM))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<48}",""), sty(DIMMER))]),
        Line::from(""),
    ];

    for (i, &opt) in opts.iter().enumerate() {
        let sel   = app.menu_cur == i;
        let style = if sel { Style::default().fg(BG).bg(ac).add_modifier(Modifier::BOLD) }
                    else   { sty(CREAM) };
        lines.push(Line::from(vec![
            Span::styled(if sel { " ▶ " } else { "   " }, sty(ac)),
            Span::styled(format!("{:<44}", opt), style),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![Span::styled(format!("  {:─<48}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  ↑↓/jk navigate    Enter select    q quit",
        sty(DIMMER),
    )]));

    f.render_widget(Paragraph::new(lines), inner);
}

// ── COLOR PICK ────────────────────────────────────────────────────────────────
fn draw_color_pick(app: &App, f: &mut Frame) {
    let a    = f.size();
    let rect = center(52, 18, a);
    let ac   = accent(app.cfg.theme);

    let block = Block::default()
        .borders(Borders::ALL).border_type(BorderType::Double)
        .border_style(sty(ac)).style(Style::default().bg(BG1));
    f.render_widget(block, rect);

    let inner = pad(rect, 2, 1);
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled("     CHOOSE YOUR SIDE", sty(ac).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(DIMMER))]),
        Line::from(""),
    ];

    for (i, (sym, label, col)) in [("♔","WHITE", PW), ("♚","BLACK", DIM)].iter().enumerate() {
        let sel = app.color_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if sel { " ▶ " } else { "   " }, sty(ac)),
            Span::styled(format!("{} {}", sym, label),
                Style::default().fg(*col)
                    .bg(if sel { BG2 } else { BG1 })
                    .add_modifier(if sel { Modifier::BOLD } else { Modifier::empty() })),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![Span::styled(format!("  {:─<46}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  ↑↓ navigate   Enter select   Esc back",
        sty(DIMMER),
    )]));

    f.render_widget(Paragraph::new(lines), inner);
}

// ── SETTINGS ─────────────────────────────────────────────────────────────────
fn draw_settings(app: &App, f: &mut Frame) {
    let a    = f.size();
    let rect = center(64, 26, a);
    let ac   = accent(app.cfg.theme);

    let block = Block::default()
        .borders(Borders::ALL).border_type(BorderType::Double)
        .border_style(sty(ac)).style(Style::default().bg(BG1))
        .title(Span::styled(" ⚙  SETTINGS ", sty(ac).add_modifier(Modifier::BOLD)));
    f.render_widget(block, rect);

    let inner = pad(rect, 2, 1);

    let rows: Vec<(&str, String)> = vec![
        ("Theme",        app.cfg.theme.name().to_string()),
        ("Piece style",  app.cfg.piece_style.name().to_string()),
        ("AI difficulty",app.cfg.ai_depth.name().to_string()),
        ("Move hints",   app.cfg.move_hints.name().to_string()),
        ("Show coords",  bool_label(app.cfg.show_coords)),
        ("Show clock",   bool_label(app.cfg.show_clock)),
        ("Flip board",   bool_label(app.cfg.flip_board)),
        ("Confirm move", bool_label(app.cfg.confirm_move)),
    ];

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<56}",""), sty(DIMMER))]),
        Line::from(""),
    ];

    for (i, (label, value)) in rows.iter().enumerate() {
        let sel = app.settings_cur == i;
        let label_style = if sel { sty(ac).add_modifier(Modifier::BOLD) } else { sty(CREAM) };
        let value_style = if sel { Style::default().fg(BG).bg(ac).add_modifier(Modifier::BOLD) }
                          else   { sty(DIM) };
        lines.push(Line::from(vec![
            Span::styled(if sel { " ▶ " } else { "   " }, sty(ac)),
            Span::styled(format!("{:<18}", label), label_style),
            Span::raw("  "),
            Span::styled(if sel { format!("◀ {} ▶", value) } else { format!("  {}  ", value) }, value_style),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![Span::styled(format!("  {:─<56}",""), sty(DIMMER))]));
    lines.push(Line::from(""));

    // Saved notice or hint
    let hint = if app.saved_notice.is_some() {
        "  ✓ Saved to ~/.chess_tui.conf"
    } else {
        "  w save    r reset defaults    Esc back to menu"
    };
    let hint_col = if app.saved_notice.is_some() { TEAL } else { DIMMER };
    lines.push(Line::from(vec![Span::styled(hint, sty(hint_col))]));

    f.render_widget(Paragraph::new(lines), inner);
}

// ── GAME ──────────────────────────────────────────────────────────────────────
fn draw_game(app: &App, f: &mut Frame) {
    let a = f.size();
    let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(a);
    draw_topbar(app, f, top);

    let [board_area, side_area] = Layout::horizontal([
        Constraint::Length(39),
        Constraint::Min(0),
    ]).areas(body);

    draw_board(app, f, board_area);
    draw_sidebar(app, f, side_area);
}

fn draw_topbar(app: &App, f: &mut Frame, area: Rect) {
    let mode_s = match app.mode {
        Mode::PvP => "TWO PLAYERS".to_string(),
        Mode::CPU  => format!("VS CPU  [YOU:{}]  [AI:{}]",
            app.player_color.name(), app.cfg.ai_depth.name().split_whitespace().next().unwrap_or("")),
    };
    let (st, col) = status_str(app);
    let clock = if app.cfg.show_clock { format!("  Move {}  ", app.gs.fullmove) } else { String::new() };
    let bar = format!(" ♟ CHESS TUI  │  {}{}│  {}", mode_s, clock, st);
    f.render_widget(Paragraph::new(bar).style(Style::default().fg(col).bg(BG2)), area);
}

fn draw_board(app: &App, f: &mut Frame, area: Rect) {
    let ac      = accent(app.cfg.theme);
    let flipped = app.flipped();
    let row_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };
    let col_ord: Vec<usize> = if flipped { (0..8).rev().collect() } else { (0..8).collect() };

    let tgts: std::collections::HashSet<(usize,usize)> = app.targets.iter().map(|m|m.to).collect();
    let pending = app.pending_to;
    let king_chk = if app.gs.status == Status::Check {
        crate::engine::king_sq(&app.gs.board, app.gs.turn)
    } else { None };
    let last = app.gs.history.last();

    let mut lines: Vec<Line> = vec![];

    // File labels top
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("    ")];
        for &c in &col_ord {
            spans.push(Span::styled(format!(" {:^3}", (b'a'+c as u8) as char), sty(DIM)));
        }
        lines.push(Line::from(spans));
    }

    for &r in &row_ord {
        let rank = format!(" {} ", 8-r);
        for half in 0..2usize {
            let mut spans: Vec<Span> = vec![];

            if app.cfg.show_coords {
                spans.push(Span::styled(if half==0 { rank.clone() } else { "   ".to_string() }, sty(DIM)));
            }

            for &c in &col_ord {
                let is_light  = (r+c)%2==0;
                let is_sel    = app.selected == Some((r,c));
                let is_tgt    = tgts.contains(&(r,c));
                let is_pend   = pending == Some((r,c));
                let is_last   = last.map(|h| h.from==(r,c)||h.to==(r,c)).unwrap_or(false);
                let is_kchk   = king_chk == Some((r,c));
                let is_cursor = app.to_board(app.cursor) == (r,c);
                let piece     = app.gs.board[r][c];

                let bg = if is_sel         { sel_col(app.cfg.theme) }
                         else if is_kchk   { Color::Rgb(130,25,25) }
                         else if is_pend   { Color::Rgb(120,120,20) }
                         else if is_cursor { Color::Rgb(60,110,80) }
                         else if is_last   { Color::Rgb(60,90,65) }
                         else if is_light  { sq_light(app.cfg.theme) }
                         else              { sq_dark(app.cfg.theme) };

                let cell = if half == 0 {
                    if let Some(p) = piece {
                        let sym = piece_render(p, &app.cfg.piece_style);
                        format!(" {:<2} ", sym)
                    } else {
                        match (is_tgt, app.cfg.move_hints) {
                            (true, MoveHints::Dots)      => " ·  ".to_string(),
                            (true, MoveHints::Highlight) => "    ".to_string(),
                            _                            => "    ".to_string(),
                        }
                    }
                } else {
                    // bottom half: capture ring
                    if is_tgt && piece.is_some() { " ╌╌ ".to_string() } else { "    ".to_string() }
                };

                let mut style = Style::default().bg(if is_tgt && matches!(app.cfg.move_hints, MoveHints::Highlight) && piece.is_none() {
                    Color::Rgb(50,100,60)
                } else { bg });

                if let Some(p) = piece {
                    style = style.fg(if p.c==PC::White { PW } else { PB })
                                 .add_modifier(Modifier::BOLD);
                } else if is_tgt { style = style.fg(TEAL); }

                spans.push(Span::styled(cell, style));
            }

            if app.cfg.show_coords {
                spans.push(Span::styled(if half==0 { rank.clone() } else { "   ".to_string() }, sty(DIM)));
            }
            lines.push(Line::from(spans));
        }
    }

    // File labels bottom
    if app.cfg.show_coords {
        let mut spans = vec![Span::raw("    ")];
        for &c in &col_ord {
            spans.push(Span::styled(format!(" {:^3}", (b'a'+c as u8) as char), sty(DIM)));
        }
        lines.push(Line::from(spans));
    }

    let block = Block::default()
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(sty(Color::Rgb(70,100,70)))
        .style(Style::default().bg(BG1))
        .title(Span::styled(" BOARD ", sty(ac)));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn piece_render(p: crate::engine::Piece, style: &PieceStyle) -> String {
    let letter = match p.k { Kind::K=>'k', Kind::Q=>'q', Kind::R=>'r', Kind::B=>'b', Kind::N=>'n', Kind::P=>'p' };
    style.render(p.sym(), letter, p.c == PC::White)
}

fn draw_sidebar(app: &App, f: &mut Frame, area: Rect) {
    let [pl, st, hi, kb] = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(4),
        Constraint::Min(5),
        Constraint::Length(6),
    ]).areas(area);

    draw_players(app, f, pl);
    draw_status(app, f, st);
    draw_history(app, f, hi);
    draw_keybinds(app, f, kb);
}

fn draw_players(app: &App, f: &mut Frame, area: Rect) {
    let ac     = accent(app.cfg.theme);
    let cap_w: String = app.gs.cap_w.iter().map(|p| p.sym()).collect::<Vec<_>>().join("");
    let cap_b: String = app.gs.cap_b.iter().map(|p| p.sym()).collect::<Vec<_>>().join("");
    let w_tag = match app.mode { Mode::CPU if app.player_color==PC::White=>"[YOU]", Mode::CPU=>"[CPU]", _=>"" };
    let b_tag = match app.mode { Mode::CPU if app.player_color==PC::Black=>"[YOU]", Mode::CPU=>"[CPU]", _=>"" };
    let ib = app.gs.turn==PC::Black; let iw = app.gs.turn==PC::White;

    let lines = vec![
        Line::from(vec![
            Span::styled(if ib{"▶ "}else{"  "}, sty(ac)),
            Span::styled("♚ BLACK ", sty(DIM).add_modifier(Modifier::BOLD)),
            Span::styled(b_tag, sty(DIMMER)),
        ]),
        Line::from(vec![Span::raw("  "), Span::styled(if cap_w.is_empty(){"—"}else{&cap_w}, sty(DIM))]),
        Line::from(vec![Span::styled(format!("  {:─<26}",""), sty(DIMMER))]),
        Line::from(vec![
            Span::styled(if iw{"▶ "}else{"  "}, sty(ac)),
            Span::styled("♔ WHITE ", sty(PW).add_modifier(Modifier::BOLD)),
            Span::styled(w_tag, sty(DIMMER)),
        ]),
        Line::from(vec![Span::raw("  "), Span::styled(if cap_b.is_empty(){"—"}else{&cap_b}, sty(CREAM))]),
    ];

    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(sty(Color::Rgb(70,100,70))).style(Style::default().bg(BG1))
        .title(Span::styled(" PLAYERS ", sty(ac)));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_status(app: &App, f: &mut Frame, area: Rect) {
    let ac = accent(app.cfg.theme);
    let (s, col) = status_str(app);
    let clock = if app.cfg.show_clock { format!(" Move {} ", app.gs.fullmove) } else { String::new() };
    let confirm_hint = if app.pending_to.is_some() { " [press Enter again to confirm]" } else { "" };

    let lines = vec![
        Line::from(vec![Span::raw(" "), Span::styled(s, sty(col).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::styled(format!("{}{}", clock, confirm_hint), sty(DIM))]),
    ];

    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(sty(Color::Rgb(70,100,70))).style(Style::default().bg(BG1))
        .title(Span::styled(" STATUS ", sty(ac)));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_history(app: &App, f: &mut Frame, area: Rect) {
    let ac = accent(app.cfg.theme);
    let h  = &app.gs.history;
    let cap = area.height.saturating_sub(2) as usize;

    let mut pairs: Vec<(usize, String, String)> = vec![];
    let mut i = 0; let mut n = 1;
    while i < h.len() {
        let w = h[i].notation.clone();
        let b = if i+1 < h.len() { h[i+1].notation.clone() } else { String::new() };
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
                Span::styled(format!("{:<8}", w), sty(CREAM).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<8}", b), sty(DIM)),
            ]));
        }
    }

    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(sty(Color::Rgb(70,100,70))).style(Style::default().bg(BG1))
        .title(Span::styled(" HISTORY ", sty(ac)));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_keybinds(app: &App, f: &mut Frame, area: Rect) {
    let ac = accent(app.cfg.theme);
    let lines = vec![
        Line::from(vec![Span::styled(" ↑↓←→ / hjkl   move cursor", sty(DIM))]),
        Line::from(vec![Span::styled(" Enter / Space  select · move", sty(DIM))]),
        Line::from(vec![Span::styled(" n  new game   s  settings   q  menu", sty(DIM))]),
        Line::from(vec![Span::styled(" Esc  cancel selection", sty(DIMMER))]),
    ];
    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(sty(Color::Rgb(70,100,70))).style(Style::default().bg(BG1))
        .title(Span::styled(" KEYS ", sty(ac)));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

// ── PROMO ─────────────────────────────────────────────────────────────────────
fn draw_promo(app: &App, f: &mut Frame) {
    let a    = f.size();
    let rect = center(48, 9, a);
    let ac   = accent(app.cfg.theme);
    f.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL).border_type(BorderType::Double)
        .border_style(sty(ac)).style(Style::default().bg(BG2))
        .title(Span::styled(" PROMOTE PAWN ", sty(ac).add_modifier(Modifier::BOLD)));
    f.render_widget(block, rect);

    let inner = pad(rect, 2, 1);
    let color = app.gs.turn;
    let pieces = [
        (Kind::Q, if color==PC::White{"♕"}else{"♛"}, "Queen"),
        (Kind::R, if color==PC::White{"♖"}else{"♜"}, "Rook"),
        (Kind::B, if color==PC::White{"♗"}else{"♝"}, "Bishop"),
        (Kind::N, if color==PC::White{"♘"}else{"♞"}, "Knight"),
    ];

    let mut row: Vec<Span> = vec![Span::raw(" ")];
    for (i, (_, sym, name)) in pieces.iter().enumerate() {
        let sel = app.promo_cur == i;
        row.push(Span::styled(
            format!(" {} {} ", sym, name),
            if sel { Style::default().fg(BG).bg(ac).add_modifier(Modifier::BOLD) }
            else   { sty(CREAM) },
        ));
        row.push(Span::raw("  "));
    }

    let lines = vec![
        Line::from(vec![Span::styled(" Select promotion:", sty(CREAM))]),
        Line::from(""),
        Line::from(row),
        Line::from(""),
        Line::from(vec![Span::styled(" ←→/hl select   Enter confirm   Esc cancel", sty(DIMMER))]),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

// ── Helpers ───────────────────────────────────────────────────────────────────
fn status_str(app: &App) -> (String, Color) {
    match app.gs.status {
        Status::Checkmate => (format!("✕ CHECKMATE — {} WINS", app.gs.turn.opp().name()), Color::Rgb(200,170,60)),
        Status::Stalemate => ("½ STALEMATE — DRAW".into(), AMBER),
        Status::Check     => (format!("⚠ CHECK — {} TO MOVE", app.gs.turn.name()), RED),
        Status::Active if app.thinking => ("⏳ THINKING...".into(), DIM),
        Status::Active => (format!("► {} TO MOVE", app.gs.turn.name()), TEAL),
    }
}

fn bool_label(b: bool) -> String { if b { "ON ".to_string() } else { "OFF".to_string() } }
fn sty(c: Color) -> Style { Style::default().fg(c) }
fn center(w: u16, h: u16, a: Rect) -> Rect {
    Rect::new(a.x+(a.width.saturating_sub(w))/2, a.y+(a.height.saturating_sub(h))/2,
              w.min(a.width), h.min(a.height))
}
fn pad(r: Rect, px: u16, py: u16) -> Rect {
    Rect::new(r.x+px, r.y+py, r.width.saturating_sub(px*2), r.height.saturating_sub(py*2))
}
