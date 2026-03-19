// src/main.rs  — RChess TUI v0.3
mod engine;
mod ai;
mod book;
mod app;
mod config;
mod ui;

use std::io;
use std::time::{Duration, Instant};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture,
        Event, KeyEventKind, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use app::{App, Screen, VERSION};

fn main() -> io::Result<()> {
    // ── CLI flags ─────────────────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        match arg.as_str() {
            "-v" | "--version" | "-V" => {
                println!("RChess-tui v{}", VERSION);
                println!("Features: opening book, PGN export, draw offers, replay, mouse");
                println!("Built with ratatui 0.27 + crossterm 0.27");
                return Ok(());
            }
            "-h" | "--help" => {
                println!("RChess-tui v{}", VERSION);
                println!();
                println!("USAGE:");
                println!("  rchess                 Start the game");
                println!("  rchess --version       Print version");
                println!("  rchess --help          Print this help");
                println!();
                println!("IN-GAME KEYS:");
                println!("  arrows / hjkl         Move cursor");
                println!("  Enter / Space         Select piece / confirm");
                println!("  any letter            Type a move (e2e4, Nf3, O-O)");
                println!("  u                     Undo");
                println!("  d                     Offer draw (PvP only)");
                println!("  r                     Replay (after game ends)");
                println!("  n / s / q             New game / Settings / Menu");
                println!("  mouse click           Select and move pieces");
                return Ok(());
            }
            _ => {
                eprintln!("Unknown argument: {}", arg);
                eprintln!("Run 'RChess --help' for usage.");
                std::process::exit(1);
            }
        }

    }

    // ── Terminal setup ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut app       = App::new();
    let tick          = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        term.draw(|f| ui::render(&app, f))?;

        let timeout = tick.checked_sub(last_tick.elapsed()).unwrap_or_default();
        if event::poll(timeout)? {
            match event::read()? {
                // ── Keyboard ──────────────────────────────────────────────────
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match app.screen {
                        Screen::Menu      => app.handle_menu_key(key.code),
                        Screen::ColorPick => app.handle_color_key(key.code),
                        Screen::Settings  => app.handle_settings_key(key.code),
                        Screen::Game      => app.handle_game_key(key.code),
                        Screen::Promo     => app.handle_promo_key(key.code),
                        Screen::DrawOffer => app.handle_draw_offer_key(key.code),
                        Screen::Replay    => app.handle_replay_key(key.code),
                    }
                }
                // ── Mouse ─────────────────────────────────────────────────────
                Event::Mouse(m) => {
                    if let MouseEventKind::Down(MouseButton::Left) = m.kind {
                        app.handle_mouse_click(m.column as usize, m.row as usize);
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick {
            app.poll_ai();
            last_tick = Instant::now();
        }

        if app.should_quit { break; }
    }

    // ── Restore terminal ──────────────────────────────────────────────────────
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;
    Ok(())
}
