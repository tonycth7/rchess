// src/ui.rs  — v0.7
use std::collections::HashSet;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use crate::app::{App, Gs, Mode, Screen, ClockState, GameEnd, ReplaySnap, MoveLabel, VERSION};
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

const CELL_W: usize = 9;
const CELL_H: usize = 5;

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
    }
}

// ── MENU ──────────────────────────────────────────────────────────────────────
fn draw_menu(app: &App, f: &mut Frame) {
    let th   = app.cfg.theme;
    let rect = center(58, 28, f.area());
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th))),
        rect,
    );
    let inner = pad(rect, 2, 1);
    let opts  = ["  ♟  TWO PLAYERS", "  ◈  VS COMPUTER   [AI + Opening Book]", "  ⚙  SETTINGS"];
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
    let rect = center(68, 38, f.area());
    f.render_widget(
        Block::default().borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(sty(ac(th))).style(Style::default().bg(bg1(th)))
            .title(Span::styled(" ⚙  SETTINGS ", sty(ac(th)).add_modifier(Modifier::BOLD))),
        rect,
    );
    let inner = pad(rect, 2, 1);
    let rows: &[(&str,&str)] = &[
        ("Theme",           app.cfg.theme.name()),
        ("Piece style",     app.cfg.piece_style.name()),
        ("AI difficulty",   app.cfg.ai_depth.name()),
        ("Move hints",      app.cfg.move_hints.name()),
        ("Time control",    app.cfg.time_control.name()),
        ("Show coords",     if app.cfg.show_coords {"ON"} else {"OFF"}),
        ("Show clock",      if app.cfg.show_clock  {"ON"} else {"OFF"}),
        ("Flip board",      if app.cfg.flip_board  {"ON"} else {"OFF"}),
        ("Auto-flip PvP",   if app.cfg.auto_flip   {"ON"} else {"OFF"}),
        ("Confirm move",    if app.cfg.confirm_move{"ON"} else {"OFF"}),
        ("UI mode",         app.cfg.ui_mode.name()),
        ("Analysis engine", app.cfg.analysis_engine.name()),
        ("Analysis depth",  match app.cfg.analysis_depth { 1=>"1  (fast)", 2=>"2  (balanced)", 3=>"3  (strong)", _=>"2" }),
        ("Auto-save PNG",   if app.cfg.auto_save_png  {"ON  (saves on game end)"} else {"OFF (manual only)"}),
    ];
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(format!("  {:─<60}",""), sty(DIMMER))]),
        Line::from(""),
    ];
    for (i,(label,value)) in rows.iter().enumerate() {
        let s = app.settings_cur == i;
        lines.push(Line::from(vec![
            Span::styled(if s{" ▶ "}else{"   "}, sty(ac(th))),
            Span::styled(format!("{:<18}",label), if s{sty(ac(th)).add_modifier(Modifier::BOLD)}else{sty(CREAM)}),
            Span::raw("  "),
            Span::styled(
                if s{format!("◀  {}  ▶",value)}else{format!("   {}   ",value)},
                if s{Style::default().fg(bg0(th)).bg(ac(th)).add_modifier(Modifier::BOLD)}else{sty(DIM)}),
        ]));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled(format!("  {:─<60}",""), sty(DIMMER))]));
    lines.push(Line::from(""));
    let (hint,hcol) = if app.saved_notice.is_some() {
        ("  ✓ Saved to ~/.config/rchess/rchess_tui.conf", GREEN)
    } else {
        ("  w save    r reset    ←→ change value    Esc back", DIMMER)
    };
    lines.push(Line::from(vec![Span::styled(hint, sty(hcol))]));
    f.render_widget(Paragraph::new(lines), inner);
}

// ── GAME ──────────────────────────────────────────────────────────────────────
fn draw_game(app: &App, f: &mut Frame) {
    let a  = f.area();
    let bw = (3 + 8*CELL_W as u16 + 3 + 2).min(a.width);
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
        Mode::PvP => "TWO PLAYERS".to_string(),
        Mode::CPU  => format!("VS CPU [YOU:{}] [{}]",
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
    let hl_color = |is_light: bool| {
        let sq = if is_light { th.squares().0 } else { th.squares().1 };
        Color::Rgb(sq.0.saturating_add(30).min(255), sq.1.saturating_add(30).min(255), sq.2.saturating_add(10).min(255))
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
                    else if is_tgt && matches!(app.cfg.move_hints,MoveHints::Highlight) && piece.is_none() { hl_color(is_light) }
                    else if is_last      { lm_color(is_light) }
                    else if is_light     { sq_l(th) }
                    else                 { sq_d(th) };

                let cell_str = if is_mid {
                    if let Some(p) = piece { center_str(&piece_sym(p,&app.cfg.piece_style), CELL_W) }
                    else if is_tgt && matches!(app.cfg.move_hints,MoveHints::Dots) { center_str("·", CELL_W) }
                    else { " ".repeat(CELL_W) }
                } else if line_idx==CELL_H-1 && is_tgt && piece.is_some() {
                    format!("  {:─<width$}  ","",width=CELL_W.saturating_sub(4))
                } else { " ".repeat(CELL_W) };

                let mut style = Style::default().bg(bg);
                if let Some(p) = piece {
                    style = style.fg(if p.c==PC::White{pw(th)}else{pb_c(th)}).add_modifier(Modifier::BOLD);
                } else if is_tgt { style = style.fg(TEAL); }
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
        (format!("  ✓ PNG: {}", fname), GREEN)
    } else if let Some(path) = &app.pgn_saved_path {
        let fname = path.split('/').last().unwrap_or(path);
        (format!("  PGN: {}", fname), TEAL)
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
            lines.push(Line::from(vec![
                Span::styled(format!(" {:>3}. ", n), sty(DIMMER)),
                Span::styled(format!("{:<8}", w), w_sty),
                Span::styled(format!("{:<3} ", wi), sty(wlc)),
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
            Span::styled("  export PNG  ", sty(DIM)),
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
        (format!("  ✓ PNG: {}", path.split('/').last().unwrap_or(path)), GREEN)
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
    f.render_widget(Block::default().style(Style::default().bg(bg0(th))), a);

    let bw = (3 + 8*CELL_W as u16 + 3 + 2).min(a.width);
    let [top, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(a);
    let [board_col, side_col] = Layout::horizontal([Constraint::Length(bw), Constraint::Min(0)]).areas(body);

    let snap  = app.replay_snaps.get(app.replay_idx);
    let title = format!(" ♟ RChess — REPLAY  [{}/{}]  {}",
        app.replay_idx, app.replay_snaps.len().saturating_sub(1), app.replay_result);
    f.render_widget(Paragraph::new(title).style(Style::default().fg(TEAL).bg(bg1(th))), top);

    if let Some(snap) = snap { draw_replay_board(app, f, board_col, snap); }
    draw_replay_sidebar(app, f, side_col);
}

fn draw_replay_board(app: &App, f: &mut Frame, area: Rect, snap: &ReplaySnap) {
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
