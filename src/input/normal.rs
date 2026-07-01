use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) async fn handle_normal(app: &mut App, key: KeyCode, mods: KeyModifiers) {
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

        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Char('g') => app.move_top(),
        KeyCode::Char('G') => app.move_bottom(),
        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),

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
        KeyCode::Char('F') => {
            app.theme_state.show_favs = !app.theme_state.show_favs;
            app.theme_state.show_recent = false;
            app.preview_state.preview_timer = 0;
        }
        KeyCode::Char('R') => {
            app.theme_state.show_recent = !app.theme_state.show_recent;
            app.theme_state.show_favs = false;
            app.preview_state.preview_timer = 0;
        }
        KeyCode::Char('x') => app.random_theme(),
        KeyCode::Char('r') => {
            app.start_refresh();
            app.preview_state.preview_timer = 0;
        }

        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.hide_font_warning = true;
            app.save_config();
        }

        KeyCode::Char(' ') => app.trigger_preview().await,
        KeyCode::Char('p') => {
            app.mode = Mode::Immersive;
            app.trigger_immersive_preview().await;
        }
        KeyCode::Char('e') => {
            app.edit_theme().await;
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
