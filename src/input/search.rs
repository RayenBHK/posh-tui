use crate::app::{App, Mode};
use crossterm::event::KeyCode;

pub(super) fn handle_search(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.clear_search();
        }
        KeyCode::Backspace => {
            app.search_state.search_query.pop();
            let q = app.search_state.search_query.clone();
            app.apply_search(&q);
        }
        KeyCode::Char(c) => {
            let mut q = app.search_state.search_query.clone();
            q.push(c);
            app.apply_search(&q);
        }
        KeyCode::Down | KeyCode::Up => app.mode = Mode::Normal,
        _ => {}
    }
}
