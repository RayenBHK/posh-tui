use crate::ui::utilities::layout::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub(crate) fn draw_soft_revert(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let popup = centered_rect(50, 40, area);
    frame.render_widget(Clear, popup);

    let (rc, has) = match &app.shell_info {
        Some(i) => (i.rc_path.display().to_string(), i.has_managed),
        None => ("unknown".into(), false),
    };

    let lines = if has {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  Soft revert",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Removes: ", Style::default().fg(Color::DarkGray)),
                Span::raw("posh-tui managed block only"),
            ]),
            Line::from(vec![
                Span::styled("  File:    ", Style::default().fg(Color::DarkGray)),
                Span::styled(rc, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Lines removed:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "    # posh-tui:start",
                Style::default().fg(Color::Red),
            )),
            Line::from(Span::styled(
                "    eval \"$(oh-my-posh init ...)\"",
                Style::default().fg(Color::Red),
            )),
            Line::from(Span::styled(
                "    # posh-tui:end",
                Style::default().fg(Color::Red),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Enter", Style::default().fg(Color::Green)),
                Span::raw(": confirm   "),
                Span::styled("Esc", Style::default().fg(Color::Red)),
                Span::raw(": cancel"),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No posh-tui block found in your rc file.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Nothing to remove.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Esc", Style::default().fg(Color::Yellow)),
                Span::raw(": close"),
            ]),
        ]
    };

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" soft revert "),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn draw_hard_revert(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let popup = centered_rect(55, 55, area);
    frame.render_widget(Clear, popup);

    let rc = match &app.shell_info {
        Some(i) => i.rc_path.display().to_string(),
        None => "unknown".into(),
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "  ⚠ Hard revert",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(Span::styled(
            "  Removes ALL oh-my-posh lines, including",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  ones you may have added manually.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  File: ", Style::default().fg(Color::DarkGray)),
            Span::styled(rc, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Lines that will be deleted:",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    if app.hard_revert_preview.is_empty() {
        lines.push(Line::from(Span::styled(
            "    (none found — config is already clean)",
            Style::default().fg(Color::Green),
        )));
    } else {
        for (num, content) in &app.hard_revert_preview {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    line {num}: "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(content.clone(), Style::default().fg(Color::Red)),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Enter", Style::default().fg(Color::Green)),
        Span::raw(": confirm   "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(": cancel"),
    ]));

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" hard revert "),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}
