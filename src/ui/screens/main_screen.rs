use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use super::super::components::{preview_pane, theme_list};

pub(crate) fn draw_main(frame: &mut Frame, app: &mut crate::app::App, area: ratatui::layout::Rect) {
    let mut main_area = area;
    if !app.hide_font_warning {
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area);
        main_area = v_chunks[0];

        let banner = Paragraph::new(
            " ℹ️  Ensure a Nerd Font is installed in your terminal. Press 'N' to dismiss. ",
        )
        .style(Style::default().fg(Color::Black).bg(Color::Yellow));
        frame.render_widget(banner, v_chunks[1]);
    }

    let list_pct: u16 = if main_area.width < 100 { 28 } else { 22 };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(list_pct),
            Constraint::Percentage(100 - list_pct),
        ])
        .split(main_area);

    app.preview_state.preview_width = chunks[1].width;
    app.preview_state.terminal_width = main_area.width;

    theme_list::draw_list(frame, app, chunks[0]);
    preview_pane::draw_preview(frame, app, chunks[1]);
}
