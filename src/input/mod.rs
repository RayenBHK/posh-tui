mod immersive;
mod normal;
mod overlay;
mod search;

use crate::app::{App, Mode};
use crossterm::event::{KeyEvent, MouseEvent, MouseEventKind};

pub(super) async fn handle_key_event(app: &mut App, key: KeyEvent) {
    match app.mode {
        Mode::Normal => normal::handle_normal(app, key.code, key.modifiers).await,
        Mode::Search => search::handle_search(app, key.code),
        Mode::Immersive => immersive::handle_immersive(app, key.code, key.modifiers).await,
        Mode::Help | Mode::Confirm | Mode::SoftRevert | Mode::HardRevert | Mode::Message => {
            overlay::handle_overlay(app, key.code);
        }
    }
}

pub(super) fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    if mouse.kind == MouseEventKind::ScrollUp {
        app.move_up();
    } else if mouse.kind == MouseEventKind::ScrollDown {
        app.move_down();
    }
}
