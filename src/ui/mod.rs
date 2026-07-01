use crate::app::{App, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

pub mod components;
pub mod overlays;
pub mod screens;
pub mod utilities;

pub use utilities::ansi::ansi_to_text;

pub fn draw(frame: &mut Frame, app: &mut App) {
    if app.mode == Mode::Immersive {
        screens::immersive_screen::draw_immersive(frame, app);
        return;
    }

    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    components::search_bar::draw_search(frame, app, outer[0]);
    screens::main_screen::draw_main(frame, app, outer[1]);
    components::status_bar::draw_statusbar(frame, app, outer[2]);

    match app.mode {
        Mode::Help => overlays::help::draw_help(frame, area),
        Mode::Confirm => overlays::confirm::draw_confirm(frame, app, area),
        Mode::SoftRevert => overlays::revert::draw_soft_revert(frame, app, area),
        Mode::HardRevert => overlays::revert::draw_hard_revert(frame, app, area),
        Mode::Message => overlays::message::draw_message(frame, app, area),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::core::themes::Theme;
    use ratatui::{backend::TestBackend, Terminal};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_ui_snapshot() {
        let themes = vec![
            Theme {
                name: "theme1".to_string(),
                filename: "theme1.json".to_string(),
                local: None,
                raw_url: "".into(),
            },
            Theme {
                name: "theme2".to_string(),
                filename: "theme2.json".to_string(),
                local: None,
                raw_url: "".into(),
            },
        ];
        let mut app = App::new(themes, PathBuf::from("/tmp"));
        app.theme_state.selected = 0;
        app.search_state.search_query = "".to_string();
        // Force deterministic zoom so this test does not depend on
        // whatever zoom_factor happens to be persisted on disk.
        app.preview_state.zoom_factor = 1.0;
        app.save_config();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                draw(f, &mut app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                content.push_str(buffer.cell((x, y)).unwrap().symbol());
            }
            content.push('\n');
        }

        insta::assert_snapshot!(content);
    }
}
