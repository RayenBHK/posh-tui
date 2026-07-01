use crate::app::Mode;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

pub(crate) fn draw_statusbar(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let text = match app.mode {
        Mode::Search  => " Esc: cancel  ↑↓: navigate",
        Mode::Normal  => " Space: preview  p: immersive  Enter: apply  u: undo  U: revert  Ctrl+U: hard revert  r: refresh  /: search  x: random  R: recent  ?: help  q: quit",
        Mode::Confirm => " Enter: confirm  Esc: cancel",
        Mode::Help    => " ?: close",
        Mode::Message => " any key: close",
        _             => "",
    };
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
