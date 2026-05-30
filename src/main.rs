mod app;
mod config;
mod error;
mod preview;
mod search;
mod shell;
mod themes;
mod ui;

use app::{App, Mode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, path::PathBuf, time::Duration};

#[tokio::main]
async fn main() -> error::Result<()> {
    println!("Fetching themes...");
    let themes = themes::fetch_theme_list().await?;

    let cache_dir = dirs_next::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("posh-tui")
        .join("themes");
    std::fs::create_dir_all(&cache_dir)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut app = App::new(themes, cache_dir);
    let res = run_app(&mut term, &mut app).await;

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;

    if let Err(e) = res { eprintln!("Error: {e}"); }
    Ok(())
}

async fn run_app(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app:  &mut App,
) -> error::Result<()> {
    loop {
        app.poll_preview().await;
        app.tick();

        term.draw(|f| ui::draw(f, app))?;

        if !event::poll(Duration::from_millis(33))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) => match app.mode {
                Mode::Normal    => handle_normal(app, key.code, key.modifiers).await,
                Mode::Search    => handle_search(app, key.code),
                Mode::Confirm   => handle_confirm(app, key.code),
                Mode::Immersive => handle_immersive(app, key.code).await,
                Mode::Help      => {
                    if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
                        app.mode = Mode::Normal;
                    }
                }
            },
            Event::Resize(_, _) => {
                term.autoresize()?;
                if !app.preview_output.is_empty() {
                    app.preview_output.clear();
                    app.trigger_preview().await;
                }
            }
            _ => {}
        }

        if app.should_quit { break; }
    }
    Ok(())
}

async fn handle_normal(app: &mut App, key: KeyCode, mods: KeyModifiers) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => app.should_quit = true,

        KeyCode::Up   | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Char('g') => app.move_top(),
        KeyCode::Char('G') => app.move_bottom(),
        KeyCode::PageUp    => app.page_up(),
        KeyCode::PageDown  => app.page_down(),

        KeyCode::Char('<') => app.scroll_left(),
        KeyCode::Char('>') => app.scroll_right(),
        KeyCode::Char('-') => {
            app.zoom_out();
            app.trigger_preview().await;
        }
        KeyCode::Char('=') => {
            app.zoom_in();
            app.trigger_preview().await;
        }
        KeyCode::Char('0') => {
            app.zoom_reset();
            app.trigger_preview().await;
        }

        KeyCode::Char('/') => app.mode = Mode::Search,
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Char('f') => app.toggle_favourite(),
        KeyCode::Char('F') => app.show_favs = !app.show_favs,

        KeyCode::Char(' ') => app.trigger_preview().await,

        KeyCode::Char('p') => {
            app.mode = Mode::Immersive;
            app.trigger_immersive_preview().await;
        }

        KeyCode::Enter => {
            if app.selected_theme().is_some() {
                app.mode = Mode::Confirm;
            }
        }

        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => { app.mode = Mode::Normal; app.clear_search(); }
        KeyCode::Backspace => {
            app.search_query.pop();
            let q = app.search_query.clone();
            app.apply_search(&q);
        }
        KeyCode::Char(c) => {
            let mut q = app.search_query.clone();
            q.push(c);
            app.apply_search(&q);
        }
        KeyCode::Down | KeyCode::Up => app.mode = Mode::Normal,
        _ => {}
    }
}

fn handle_confirm(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            if let Some(t) = app.selected_theme() {
                app.last_applied = Some(t.name.clone());
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Esc => app.mode = Mode::Normal,
        _ => {}
    }
}

async fn handle_immersive(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.imm_history.clear();
            app.imm_input.clear();
        }
        KeyCode::Enter => {
            if app.selected_theme().is_some() {
                app.mode = Mode::Confirm;
            }
        }
        KeyCode::Backspace => app.imm_backspace(),
        KeyCode::Char(c)   => app.imm_push(c),
        _ => {}
    }
}