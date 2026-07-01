use crate::ui::utilities::layout::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub(crate) fn draw_confirm(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let popup = centered_rect(50, 45, area);
    frame.render_widget(Clear, popup);

    let name = app
        .selected_theme()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "unknown".into());

    let (shell_name, rc, backup) = match &app.shell_info {
        Some(i) => (
            i.shell.name().to_string(),
            i.rc_path.display().to_string(),
            i.backup_path.display().to_string(),
        ),
        None => ("unknown".into(), "unknown".into(), "unknown".into()),
    };

    let theme_path = app.cache_dir.join(
        app.selected_theme()
            .map(|t| t.filename.as_str())
            .unwrap_or(""),
    );

    let init_line = match &app.shell_info {
        Some(i) => i.shell.init_line(&theme_path),
        None => String::new(),
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Apply theme: "),
            Span::styled(
                &*name,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Shell:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(shell_name, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  File:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(rc, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Backup: ", Style::default().fg(Color::DarkGray)),
            Span::styled(backup, Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Writes: ", Style::default().fg(Color::DarkGray)),
            Span::styled(init_line, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Green)),
            Span::raw(": confirm   "),
            Span::styled("Esc", Style::default().fg(Color::Red)),
            Span::raw(": cancel"),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" apply theme "),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}
