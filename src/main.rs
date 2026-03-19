// src/main.rs
mod engine;
mod ai;
mod app;
mod config;
mod ui;

use std::io;
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use app::{App, Screen};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut app       = App::new();
    let tick           = Duration::from_millis(50);
    let mut last_tick  = Instant::now();

    loop {
        term.draw(|f| ui::render(&app, f))?;

        let timeout = tick.checked_sub(last_tick.elapsed()).unwrap_or_default();
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.screen {
                        Screen::Menu      => app.handle_menu_key(key.code),
                        Screen::ColorPick => app.handle_color_key(key.code),
                        Screen::Settings  => app.handle_settings_key(key.code),
                        Screen::Game      => app.handle_game_key(key.code),
                        Screen::Promo     => app.handle_promo_key(key.code),
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick {
            app.poll_ai();
            last_tick = Instant::now();
        }

        if app.should_quit { break; }
    }

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;
    Ok(())
}
