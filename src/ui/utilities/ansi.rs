use ansi_to_tui::IntoText;
use ratatui::text::Text;

pub fn ansi_to_text(s: &str) -> Text<'static> {
    s.to_string().into_text().unwrap_or_default()
}
