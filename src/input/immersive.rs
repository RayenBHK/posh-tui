use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) async fn handle_immersive(app: &mut App, key: KeyCode, mods: KeyModifiers) {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.immersive_state.imm_history.clear();
            app.immersive_state.imm_input.clear();
        }
        KeyCode::Char('a') if mods.contains(KeyModifiers::CONTROL) => {
            if app.selected_theme().is_some() {
                app.mode = Mode::Confirm;
            }
        }
        KeyCode::Enter => app.imm_submit(),
        KeyCode::Backspace => app.imm_backspace(),
        KeyCode::Char(c) => app.imm_push(c),
        _ => {}
    }
}
