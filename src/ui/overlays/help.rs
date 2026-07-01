use crate::ui::utilities::layout::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub(crate) fn draw_help(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(52, 80, area);
    frame.render_widget(Clear, popup);

    let lines = vec![
        Line::from(Span::styled(
            " posh-tui — keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ↑/k  ↓/j   ", Style::default().fg(Color::Yellow)),
            Span::raw("navigate list"),
        ]),
        Line::from(vec![
            Span::styled(" g / G      ", Style::default().fg(Color::Yellow)),
            Span::raw("top / bottom"),
        ]),
        Line::from(vec![
            Span::styled(" PgUp/PgDn  ", Style::default().fg(Color::Yellow)),
            Span::raw("page scroll"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Space      ", Style::default().fg(Color::Green)),
            Span::raw("preview theme in pane"),
        ]),
        Line::from(vec![
            Span::styled(" p          ", Style::default().fg(Color::Cyan)),
            Span::raw("immersive full-screen preview"),
        ]),
        Line::from(vec![
            Span::styled(" Enter      ", Style::default().fg(Color::Green)),
            Span::raw("apply theme to shell"),
        ]),
        Line::from(vec![
            Span::styled(" e          ", Style::default().fg(Color::Cyan)),
            Span::raw("edit theme in $EDITOR"),
        ]),
        Line::from(vec![
            Span::styled(" u          ", Style::default().fg(Color::Red)),
            Span::raw("undo last apply"),
        ]),
        Line::from(vec![
            Span::styled(" U          ", Style::default().fg(Color::Red)),
            Span::raw("soft revert (remove managed block)"),
        ]),
        Line::from(vec![
            Span::styled(" Ctrl+U     ", Style::default().fg(Color::Red)),
            Span::raw("hard revert (remove all omp lines)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" < / >      ", Style::default().fg(Color::Cyan)),
            Span::raw("scroll preview left / right"),
        ]),
        Line::from(vec![
            Span::styled(" - / =      ", Style::default().fg(Color::Cyan)),
            Span::raw("zoom out / in"),
        ]),
        Line::from(vec![
            Span::styled(" 0          ", Style::default().fg(Color::Cyan)),
            Span::raw("reset zoom"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" f          ", Style::default().fg(Color::Cyan)),
            Span::raw("toggle favourite"),
        ]),
        Line::from(vec![
            Span::styled(" F          ", Style::default().fg(Color::Cyan)),
            Span::raw("favourites view"),
        ]),
        Line::from(vec![
            Span::styled(" R          ", Style::default().fg(Color::Cyan)),
            Span::raw("recently viewed"),
        ]),
        Line::from(vec![
            Span::styled(" x          ", Style::default().fg(Color::Cyan)),
            Span::raw("random theme"),
        ]),
        Line::from(vec![
            Span::styled(" /          ", Style::default().fg(Color::Cyan)),
            Span::raw("search themes"),
        ]),
        Line::from(vec![
            Span::styled(" r          ", Style::default().fg(Color::Cyan)),
            Span::raw("refresh theme list from GitHub"),
        ]),
        Line::from(vec![
            Span::styled(" n / N      ", Style::default().fg(Color::DarkGray)),
            Span::raw("dismiss Nerd Font warning"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ?          ", Style::default().fg(Color::DarkGray)),
            Span::raw("toggle help"),
        ]),
        Line::from(vec![
            Span::styled(" q / Ctrl+C ", Style::default().fg(Color::DarkGray)),
            Span::raw("quit"),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" help "))
            .wrap(Wrap { trim: false }),
        popup,
    );
}
