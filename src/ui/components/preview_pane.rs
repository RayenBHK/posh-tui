use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub(crate) fn draw_preview(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let theme_name = app
        .selected_theme()
        .map(|t| format!(" preview — {} ", t.name))
        .unwrap_or(" preview ".to_string());

    let zoom_label = if (app.preview_state.zoom_factor - 1.0).abs() < 0.01 {
        String::new()
    } else {
        format!(" zoom {:.0}% ", (100.0 / app.preview_state.zoom_factor) as u16)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{theme_name}{zoom_label}"));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.preview_state.preview_loading {
        frame.render_widget(Paragraph::new("  loading..."), inner);
        return;
    }

    if app.preview_state.preview_output.is_empty() {
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("  press ", Style::default().fg(Color::DarkGray)),
            Span::styled("Space", Style::default().fg(Color::Yellow)),
            Span::styled(" to preview  ·  ", Style::default().fg(Color::DarkGray)),
            Span::styled("p", Style::default().fg(Color::Cyan)),
            Span::styled(" for immersive mode", Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(hint, inner);
        return;
    }

    let offset = app.preview_state.scroll_offset as usize;
    // Use the pre-parsed cache populated by poll_preview(); never re-parses on render.
    let text = app.preview_state.cached_preview.clone().unwrap_or_default();

    // scroll by skipping N chars worth of spans per line — preserves styling
    let scrolled: Vec<Line> = text
        .lines
        .into_iter()
        .map(|line| {
            let mut remaining_skip = offset;
            let mut new_spans: Vec<Span<'static>> = Vec::new();

            for span in line.spans {
                let content = span.content.to_string();
                let char_count = content.chars().count();

                if remaining_skip >= char_count {
                    // skip entire span
                    remaining_skip -= char_count;
                } else if remaining_skip > 0 {
                    // partial skip — take chars after the offset
                    let visible: String = content.chars().skip(remaining_skip).collect();
                    remaining_skip = 0;
                    if !visible.is_empty() {
                        new_spans.push(Span::styled(visible, span.style));
                    }
                } else {
                    // no skip needed — take whole span
                    new_spans.push(Span::styled(content, span.style));
                }
            }

            Line::from(new_spans)
        })
        .collect();

    let para = Paragraph::new(Text::from(scrolled)).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);

    // scroll position indicator
    if offset > 0 {
        let label = format!(" ◀ +{offset} ");
        let ind_area = Rect {
            x: inner.x,
            y: inner.y,
            width: (label.len() as u16).min(inner.width),
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(label).style(Style::default().fg(Color::DarkGray)),
            ind_area,
        );
    }
}
