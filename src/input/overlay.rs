use crate::app::{App, Mode};
use crossterm::event::KeyCode;

pub(super) fn handle_overlay(app: &mut App, key: KeyCode) {
    match app.mode {
        Mode::Help => {
            if matches!(key, KeyCode::Char('?') | KeyCode::Esc) {
                app.mode = Mode::Normal;
            }
        }
        Mode::Confirm => match key {
            KeyCode::Enter => app.do_apply(),
            KeyCode::Esc => app.mode = Mode::Normal,
            _ => {}
        },
        Mode::SoftRevert => match key {
            KeyCode::Enter => app.do_soft_revert(),
            KeyCode::Esc => app.mode = Mode::Normal,
            _ => {}
        },
        Mode::HardRevert => match key {
            KeyCode::Enter => app.do_hard_revert(),
            KeyCode::Esc => app.mode = Mode::Normal,
            _ => {}
        },
        Mode::Message => {
            app.mode = Mode::Normal;
        }
        _ => {}
    }
}
