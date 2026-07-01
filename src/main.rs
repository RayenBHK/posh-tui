mod app;
mod core;
mod input;
mod preview;
mod search;
mod ui;

pub use crate::core::*;

use app::{App, Mode};
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell as CompletionShell};
use crossterm::{
event::{
    self, DisableMouseCapture, EnableMouseCapture, Event,
},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf, time::Duration};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AppEvent {
    ThemesLoaded(Vec<themes::Theme>),
    ThemeLoadError(String),
}

#[derive(Parser, Debug)]
#[command(name = "posh-tui", author, version, about, long_about = None)]
pub struct Cli {
    /// Skip applying the theme for testing
    #[arg(long)]
    dry_run: bool,

    /// Generate shell completions
    #[arg(long, value_enum)]
    generate_completions: Option<CompletionShell>,
}

#[tokio::main]
async fn main() -> error::Result<()> {
    let cli = Cli::parse();

    if let Some(shell) = cli.generate_completions {
        let mut cmd = Cli::command();
        let bin_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, bin_name, &mut io::stdout());
        return Ok(());
    }

    let dry_run = cli.dry_run;

    let cache_dir = dirs_next::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("posh-tui")
        .join("themes");
    std::fs::create_dir_all(&cache_dir)?;

    // open TUI immediately — don't wait for themes
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
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
    app.dry_run = dry_run;
    app.loading = true;
    app.load_shell_info();

    let res = run_app(&mut term, &mut app, &mut event_rx).await;

    disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {e}");
    }
    Ok(())
}

async fn run_app(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
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
                    app.loading = false;
                    app.message       = format!("failed to fetch themes: {e}\n\ncheck your internet connection.\npress r to retry.");
                    app.message_is_err = true;
                    app.mode = Mode::Message;
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
            Event::Key(key) => input::handle_key_event(app, key).await,
            Event::Mouse(mouse_event) => {
                input::handle_mouse(app, mouse_event);
            }
            Event::Resize(_, _) => {
                term.autoresize()?;
                if !app.preview_state.preview_output.is_empty() {
                    app.preview_state.preview_output.clear();
                    app.trigger_preview().await;
                }
            }
            _ => {}
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}


