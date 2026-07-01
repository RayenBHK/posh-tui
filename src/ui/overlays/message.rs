use crate::ui::utilities::layout::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub(crate) fn draw_message(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let popup = centered_rect(48, 38, area);
    frame.render_widget(Clear, popup);

    let color = if app.message_is_err {
        Color::Red
    } else {
        Color::Green
    };
    let title = if app.message_is_err {
        " error "
    } else {
        " done "
    };

    let mut lines = vec![Line::from("")];
    for l in app.message.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {l}"),
            Style::default().fg(color),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  any key", Style::default().fg(Color::DarkGray)),
        Span::raw(": close"),
    ]));

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: false }),
        popup,
    );
}
