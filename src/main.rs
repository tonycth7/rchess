// src/main.rs  — RChess TUI v0.7
#![allow(dead_code)]
#![allow(unused)]
#[macro_use]
mod log;
mod engine;
mod puzzle;
mod pgn_import;
mod save;
mod ai;
mod book;
mod app;
mod config;
mod ui;
mod pieces;
mod png_export;
mod analysis;
mod tt;
mod http;
mod lichess;
mod rcengine;
mod net;

use std::io;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture,
        Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use app::{App, Screen, VERSION};

fn main() -> io::Result<()> {
    // ── CLI flags ─────────────────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();
    if let Some(arg) = args.get(1) {
        match arg.as_str() {
            "-v" | "--version" | "-V" => {
                println!("RChess-tui v{}", VERSION);
                println!("Features: PNG export, opening book, PGN, analyze, draw offers, mouse, Lichess online play");
                println!("Built with ratatui 0.30 + crossterm 0.29");
                return Ok(());
            }
            "-h" | "--help" => {
                println!("RChess-tui v{}", VERSION);
                println!();
                println!("USAGE:");
                println!("  rchess                 Start the game (TUI)");
                println!("  rchess --version       Print version");
                println!("  rchess --help          Print this help");
                println!("  rchess --engine        Run UCI engine (stdin/stdout)");
                println!();
                println!("IN-GAME KEYS:");
                println!("  arrows / hjkl         Move cursor");
                println!("  Enter / Space         Select piece / confirm");
                println!("  any letter            Type a move (e2e4, Nf3, O-O)");
                println!("  u / r                 Undo / redo");
                println!("  e                     Engine hint arrow");
                println!("  G                     Save PGN");
                println!("  F                     Copy FEN to clipboard");
                println!("  E (Shift+E)           Export board as PNG");
                println!("  T                     Cycle UI mode");
                println!("  ?                     Keyboard help overlay");
                println!("  d                     Offer draw (two players)");
                println!("  n / s / q             New game / Settings / Menu");
                println!("  Ctrl+C                Quit");
                println!("  mouse click           Select and move pieces");
                println!("  right-click           Undo");
                println!("  scroll wheel          Scroll in menus/lists");
                println!();
                println!("ONLINE (LICHESS) KEYS:");
                println!("  p                     Seek a random opponent");
                println!("  c                     Challenge a user");
                println!("  d / D                 Accept / decline draw offer");
                println!("  t                     Offer or accept a takeback");
                println!("  v / x / a             Claim win / resign / abort");
                println!("  R                     Resync the game");
                println!("  Esc / q               Back to lobby");
                println!();
                println!("FILES:");
                println!("  Config:  ~/.config/rchess/rchess_tui.conf");
                println!("  Exports: ~/rchess_export/");
                return Ok(());
            }
            "--engine" => {
                crate::rcengine::init();
                let mut engine = crate::rcengine::uci::UciEngine::new();
                engine.uci_loop();
                return Ok(());
            }
            s if s.starts_with("--server") => {
                println!("--server flag removed: online play now uses Lichess API");
                println!("Set lichess_token in ~/.config/rchess/rchess_tui.conf and use Online Play in-game.");
                return Ok(());
            }
            _ => {
                println!("Unknown argument: {}", arg);
                println!("Run 'rchess --help' for usage.");
                std::process::exit(1);
            }
        }
    }

    // ── Terminal setup ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    // A panic must not leave the user's terminal in raw/alternate-screen mode.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(info);
    }));
    let mut stdout = io::stdout();
    crate::rcengine::init();
    let mut app       = App::new();
    // Skip mouse on basic TTYs that don't support it
    let has_mouse = !is_basic_tty();
    if has_mouse {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    } else {
        execute!(stdout, EnterAlternateScreen)?;
    }
    // TTY compatibility: auto-detect from TERM, but allow config override to force
    let tty_auto = is_basic_tty();
    crate::ui::TTY_MODE.store(tty_auto || app.cfg.tty_mode, Ordering::Relaxed);
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;
    let tick          = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        if let Ok((tw, th)) = crossterm::terminal::size() {
            app.update_cell_size(tw, th);
        }
        term.draw(|f| ui::render(&app, f))?;

        let timeout = tick.checked_sub(last_tick.elapsed()).unwrap_or_default();
        if event::poll(timeout)? {
            match event::read()? {
                // ── Keyboard ──────────────────────────────────────────────────
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q'))
                    {
                        app.should_quit = true;
                    } else if key.code == KeyCode::Char('?') && !app.text_input_active() {
                        app.help = !app.help;
                    } else if app.help {
                        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
                            app.help = false;
                        }
                    } else {
                    match app.screen {
                        Screen::Menu        => app.handle_menu_key(key.code),
                        Screen::ColorPick   => app.handle_color_key(key.code),
                        Screen::Settings    => app.handle_settings_key(key.code),
                        Screen::Game        => app.handle_game_key(key.code),
                        Screen::Promo       => app.handle_promo_key(key.code),
                        Screen::DrawOffer   => app.handle_draw_offer_key(key.code),
                        Screen::Analyze      => app.handle_analyze_key(key.code),
                        Screen::PngPreview  => app.handle_png_preview_key(key.code),
                        Screen::Import      => app.handle_import_key(key.code),
                        Screen::Puzzle      => app.handle_puzzle_key(key.code),
                        Screen::PgnSaved    => { app.screen = Screen::Game; }  // any key closes
                        Screen::OnlineLobby => app.handle_online_lobby_key(key.code),
                        // OnlineJoin removed (challenge via lobby)
                        Screen::OnlineGame  => app.handle_online_game_key(key.code),
                        Screen::LoadGame    => app.handle_load_game_key(key.code),
                    }
                    }
                }
                // ── Mouse ─────────────────────────────────────────────────────
                Event::Mouse(m) => {
                    match m.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            app.handle_mouse_click(m.column as usize, m.row as usize);
                        }
                        MouseEventKind::Down(MouseButton::Right) => {
                            app.undo();
                        }
                        MouseEventKind::ScrollUp => {
                            app.handle_mouse_scroll(-1);
                        }
                        MouseEventKind::ScrollDown => {
                            app.handle_mouse_scroll(1);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick {
            app.poll_ai();
            app.poll_puzzle();
            app.poll_online();
            last_tick = Instant::now();
        }

        if app.should_quit { break; }
    }

    // ── Restore terminal ──────────────────────────────────────────────────────
    disable_raw_mode()?;
    if has_mouse {
        execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    } else {
        execute!(term.backend_mut(), LeaveAlternateScreen)?;
    }
    term.show_cursor()?;
    Ok(())
}

/// Check if we're on a basic TTY that doesn't support mouse
fn is_basic_tty() -> bool {
    let term = std::env::var("TERM").unwrap_or_default();
    term == "linux" || term == "vt100" || term == "vt220"
        || term.starts_with("screen") || term == "dumb"
}
