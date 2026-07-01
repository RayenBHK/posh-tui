use crate::search::FuzzySearch;

pub struct SearchState {
    pub search_query: String,
    pub fuzzy: FuzzySearch,
}

impl super::App {
    pub fn apply_search(&mut self, query: &str) {
        self.search_state.search_query = query.to_string();

        let matches = self.search_state.fuzzy.query(query);

        self.theme_state.filtered = matches
            .iter()
            .filter_map(|name| self.theme_state.themes.iter().position(|t| &t.name == name))
            .collect();

        self.theme_state.selected = 0;
        self.preview_state.scroll_offset = 0;
        self.preview_state.preview_timer = 0;
    }

    pub fn clear_search(&mut self) {
        self.search_state.search_query.clear();
        let matches = self.search_state.fuzzy.query("");
        self.theme_state.filtered = matches
            .iter()
            .filter_map(|name| self.theme_state.themes.iter().position(|t| &t.name == name))
            .collect();
        self.theme_state.selected = 0;
        self.preview_state.scroll_offset = 0;
        self.preview_state.preview_timer = 0;
    }
}
