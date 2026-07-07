use crate::core::themes::Theme;
use crate::search::FuzzySearch;
use rand::Rng;
use std::collections::HashSet;

use super::PREVIEW_DEBOUNCE;

pub struct ThemeState {
    pub themes: Vec<Theme>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub favourites: HashSet<String>,
    pub show_favs: bool,
    pub show_recent: bool,
    pub last_applied: Option<String>,
}

impl super::App {
    pub fn selected_theme(&self) -> Option<&Theme> {
        self.visible_themes()
            .get(self.theme_state.selected)
            .copied()
    }

    pub fn visible_themes(&self) -> Vec<&Theme> {
        self.theme_state
            .filtered
            .iter()
            .filter_map(|&i| self.theme_state.themes.get(i))
            .filter(|t| {
                if self.theme_state.show_favs {
                    self.theme_state.favourites.contains(&t.name)
                } else if self.theme_state.show_recent {
                    self.config.recent.contains(&t.name)
                } else {
                    true
                }
            })
            .collect()
    }

    pub fn move_up(&mut self) {
        if self.theme_state.selected > 0 {
            self.theme_state.selected -= 1;
        }
        self.preview_state.scroll_offset = 0;
        self.preview_state.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn move_down(&mut self) {
        if self.theme_state.selected + 1 < self.visible_themes().len() {
            self.theme_state.selected += 1;
        }
        self.preview_state.scroll_offset = 0;
        self.preview_state.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn move_top(&mut self) {
        self.theme_state.selected = 0;
        self.preview_state.scroll_offset = 0;
        self.preview_state.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn move_bottom(&mut self) {
        let len = self.visible_themes().len();
        if len > 0 {
            self.theme_state.selected = len - 1;
        }
        self.preview_state.scroll_offset = 0;
        self.preview_state.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn page_up(&mut self) {
        self.theme_state.selected = self.theme_state.selected.saturating_sub(10);
        self.preview_state.scroll_offset = 0;
        self.preview_state.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn page_down(&mut self) {
        let max = self.visible_themes().len().saturating_sub(1);
        self.theme_state.selected = (self.theme_state.selected + 10).min(max);
        self.preview_state.scroll_offset = 0;
        self.preview_state.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn toggle_favourite(&mut self) {
        if let Some(t) = self.selected_theme() {
            let name = t.name.clone();
            if self.theme_state.favourites.contains(&name) {
                self.theme_state.favourites.remove(&name);
            } else {
                self.theme_state.favourites.insert(name);
            }
        }
        self.save_config();
    }

    pub fn random_theme(&mut self) {
        let len = self.visible_themes().len();
        if len == 0 {
            return;
        }
        self.theme_state.selected = rand::rng().random_range(0..len);
        self.preview_state.scroll_offset = 0;
        self.preview_state.preview_timer = PREVIEW_DEBOUNCE;
    }

    pub fn init_themes(&mut self, list: Vec<Theme>) {
        let theme_names: Vec<String> = list.iter().map(|t| t.name.clone()).collect();
        self.search_state.fuzzy = FuzzySearch::new(theme_names);
        self.theme_state.filtered = (0..list.len()).collect();
        self.theme_state.themes = list;
        self.loading = false;

        if let Some(name) = &self.theme_state.last_applied.clone() {
            if let Some(pos) = self.theme_state.themes.iter().position(|t| &t.name == name) {
                self.theme_state.selected = pos;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::App;
    use crate::core::themes::Theme;
    use std::path::PathBuf;

    /// Delegate to the shared helper in app/mod.rs
    fn dummy_themes() -> Vec<Theme> {
        super::super::dummy_themes()
    }

    #[tokio::test]
    async fn test_toggle_favourite_adds_and_removes() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        app.theme_state.selected = 0;

        app.toggle_favourite();
        assert!(
            app.theme_state.favourites.contains("catppuccin"),
            "expected catppuccin in favourites after first toggle"
        );

        app.toggle_favourite();
        assert!(
            !app.theme_state.favourites.contains("catppuccin"),
            "expected catppuccin removed from favourites after second toggle"
        );
    }

    #[tokio::test]
    async fn test_move_up_at_top_clamps() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        app.theme_state.selected = 0;
        app.move_up();
        assert_eq!(app.theme_state.selected, 0, "move_up at index 0 should stay at 0");
    }

    #[tokio::test]
    async fn test_visible_themes_favs_filter() {
        let mut app = App::new(dummy_themes(), PathBuf::from("/tmp"));
        app.theme_state.favourites.insert("catppuccin".into());
        app.theme_state.show_favs = true;
        let visible = app.visible_themes();
        assert_eq!(visible.len(), 1, "only the favourite should be visible");
        assert_eq!(visible[0].name, "catppuccin");
    }
}
