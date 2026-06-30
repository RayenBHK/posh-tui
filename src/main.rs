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
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AppEvent {
    ThemesLoaded(Vec<themes::Theme>),
    ThemeLoadError(String),
}

#[tokio::main]
async fn main() -> error::Result<()> {
    let cache_dir = dirs_next::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("posh-tui")
        .join("themes");
    std::fs::create_dir_all(&cache_dir)?;

    // open TUI immediately — don't wait for themes
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    // start fetching themes in background
    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(4);
    let cache_dir_clone = cache_dir.clone();
    tokio::spawn(async move {
        match themes::fetch_theme_list().await {
            Ok(list) => {
                themes::save_theme_list_cache(&list, &cache_dir_clone);
                let _ = event_tx.send(AppEvent::ThemesLoaded(list)).await;
            }
            Err(_) => {
                // network failed — try cached theme list
                if let Some(cached) = themes::load_cached_theme_list(&cache_dir_clone) {
                    let _ = event_tx.send(AppEvent::ThemesLoaded(cached)).await;
                } else {
                    let _ = event_tx.send(AppEvent::ThemeLoadError(
                        "failed to fetch themes and no cache available\n\ncheck your internet connection.".into()
                    )).await;
                }
            }
        }
    });

    let mut app = App::new(vec![], cache_dir);
    app.loading  = true;
    app.load_shell_info();

    let res = run_app(&mut term, &mut app, &mut event_rx).await;

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;

    if let Err(e) = res { eprintln!("Error: {e}"); }
    Ok(())
}

async fn run_app(
    term:     &mut Terminal<CrosstermBackend<io::Stdout>>,
    app:      &mut App,
    event_rx: &mut mpsc::Receiver<AppEvent>,
) -> error::Result<()> {
    loop {
        // check for background events (theme load completing)
        if let Ok(evt) = event_rx.try_recv() {
            match evt {
                AppEvent::ThemesLoaded(list) => {
                    app.init_themes(list);
                }
                AppEvent::ThemeLoadError(e) => {
                    app.loading       = false;
                    app.message       = format!("failed to fetch themes: {e}\n\ncheck your internet connection.\npress r to retry.");
                    app.message_is_err = true;
                    app.mode          = Mode::Message;
                }
            }
        }

        app.poll_preview().await;
        app.poll_refresh().await;
        app.tick();

        if app.step_preview_timer() {
            app.trigger_preview().await;
        }

        term.draw(|f| ui::draw(f, app))?;

        if !event::poll(Duration::from_millis(33))? {
            continue;
        }

        match event::read()? {
            Event::Key(key) => match app.mode {
                Mode::Normal    => handle_normal(app, key.code, key.modifiers).await,
                Mode::Search    => handle_search(app, key.code),
                Mode::Confirm   => handle_confirm(app, key.code),
                Mode::Immersive => handle_immersive(app, key.code, key.modifiers).await,
                Mode::Help      => {
                    if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
                        app.mode = Mode::Normal;
                    }
                }
                Mode::SoftRevert => match key.code {
                    KeyCode::Enter => app.do_soft_revert(),
                    KeyCode::Esc   => app.mode = Mode::Normal,
                    _              => {}
                },
                Mode::HardRevert => match key.code {
                    KeyCode::Enter => app.do_hard_revert(),
                    KeyCode::Esc   => app.mode = Mode::Normal,
                    _              => {}
                },
                Mode::Message => { app.mode = Mode::Normal; }
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
    // block navigation until themes are loaded
    if app.loading {
        if key == KeyCode::Char('q') {
            app.should_quit = true;
        }
        return;
    }

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
        KeyCode::Char('-') => { app.zoom_out(); app.trigger_preview().await; }
        KeyCode::Char('=') => { app.zoom_in();  app.trigger_preview().await; }
        KeyCode::Char('0') => { app.zoom_reset(); app.trigger_preview().await; }

        KeyCode::Char('/') => app.mode = Mode::Search,
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Char('f') => app.toggle_favourite(),
        KeyCode::Char('F') => { app.show_favs = !app.show_favs; app.show_recent = false; app.preview_timer = 0; }
        KeyCode::Char('R') => { app.show_recent = !app.show_recent; app.show_favs = false; app.preview_timer = 0; }
        KeyCode::Char('x') => app.random_theme(),
        KeyCode::Char('r') => { app.start_refresh(); app.preview_timer = 0; }

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
        KeyCode::Char('u') if mods.contains(KeyModifiers::CONTROL) => {
            app.prepare_hard_revert();
        }
        KeyCode::Char('U') => app.mode = Mode::SoftRevert,
        KeyCode::Char('u') => app.do_undo(),

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
        KeyCode::Enter => app.do_apply(),
        KeyCode::Esc   => app.mode = Mode::Normal,
        _              => {}
    }
}

async fn handle_immersive(app: &mut App, key: KeyCode, mods: KeyModifiers) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.imm_history.clear();
            app.imm_input.clear();
        }
        KeyCode::Char('a') if mods.contains(KeyModifiers::CONTROL) => {
            if app.selected_theme().is_some() {
                app.mode = Mode::Confirm;
            }
        }
        KeyCode::Enter     => app.imm_submit(),
        KeyCode::Backspace => app.imm_backspace(),
        KeyCode::Char(c)   => app.imm_push(c),
        _ => {}
    }
}