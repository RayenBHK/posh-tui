use crate::app::ImmKind;
use crate::ui::utilities::ansi::ansi_to_text;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
    Frame,
};

pub(crate) fn draw_immersive(frame: &mut Frame, app: &mut crate::app::App) {
    let area = frame.area();
    app.preview_state.terminal_width = area.width;

    // full black background
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    // top hint bar — 1 line
    let theme_name = app
        .selected_theme()
        .map(|t| t.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let hint = Line::from(vec![
        Span::styled(
            " immersive — ",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled(
            &*theme_name,
            Style::default().fg(Color::Green).bg(Color::Black),
        ),
        Span::styled(
            "   Esc: back",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled(
            "   Ctrl+A: apply",
            Style::default().fg(Color::Yellow).bg(Color::Black),
        ),
        Span::styled(
            "   Enter: run command",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled(
            "   type 'help' for commands",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
    ]);
    let hint_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().bg(Color::Black)),
        hint_area,
    );

    // separator under hint
    let sep1_area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize))
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black)),
        sep1_area,
    );

    // input line pinned at very bottom
    let cursor_char = if app.immersive_state.imm_cursor_tick < 30 { "█" } else { " " };
    let input_line = format!("❯ {}{}", app.immersive_state.imm_input, cursor_char);
    let input_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(input_line).style(Style::default().fg(Color::Green).bg(Color::Black)),
        input_area,
    );

    // separator above input
    let sep2_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize))
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black)),
        sep2_area,
    );

    // shell history area — between the two separators
    let history_area = Rect {
        x: area.x,
        y: area.y + 2,
        width: area.width,
        height: area.height.saturating_sub(4), // hint + sep1 + sep2 + input
    };

    // build all lines from history
    let mut all_lines: Vec<Line<'static>> = Vec::new();

    for entry in &app.immersive_state.imm_history {
        match entry.kind {
            ImmKind::Prompt => {
                // render the actual oh-my-posh ANSI prompt
                let parsed = ansi_to_text(&entry.content);
                for line in parsed.lines {
                    all_lines.push(line);
                }
            }
            ImmKind::Input => {
                // show what was typed, indented with a dim arrow
                all_lines.push(Line::from(vec![
                    Span::styled("❯ ", Style::default().fg(Color::Green).bg(Color::Black)),
                    Span::styled(
                        entry.content.clone(),
                        Style::default().fg(Color::White).bg(Color::Black),
                    ),
                ]));
            }
            ImmKind::Output => {
                // parse ANSI in output too (ls colors, git colors etc.)
                let parsed = ansi_to_text(&entry.content);
                for line in parsed.lines {
                    // ensure bg is black
                    let styled: Vec<Span<'static>> = line
                        .spans
                        .into_iter()
                        .map(|s| Span::styled(s.content, s.style.bg(Color::Black)))
                        .collect();
                    all_lines.push(Line::from(styled));
                }
            }
        }
    }

    // always scroll to show latest lines — like a real terminal
    let visible_h = history_area.height as usize;
    let skip = all_lines.len().saturating_sub(visible_h);
    let visible: Vec<Line<'static>> = all_lines.into_iter().skip(skip).collect();

    frame.render_widget(
        Paragraph::new(Text::from(visible))
            .style(Style::default().fg(Color::White).bg(Color::Black)),
        history_area,
    );
}
