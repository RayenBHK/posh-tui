use nucleo::{
    pattern::{CaseMatching, Normalization},
    Config, Nucleo,
};
use std::sync::Arc;

pub struct FuzzySearch {
    matcher: Nucleo<String>,
}

impl FuzzySearch {
    pub fn new(items: Vec<String>) -> Self {
        let matcher = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);

        let injector = matcher.injector();
        for item in items {
            let _ = injector.push(item.clone(), |s, cols| {
                cols[0] = s.clone().into();
            });
        }

        // tick once to load all items
        let mut m = Self { matcher };
        m.matcher.tick(100);
        m
    }

    pub fn query(&mut self, query: &str) -> Vec<String> {
        self.matcher
            .pattern
            .reparse(0, query, CaseMatching::Ignore, Normalization::Smart, false);

        loop {
            let status = self.matcher.tick(10);
            if !status.running {
                break;
            }
        }

        let snapshot = self.matcher.snapshot();
        let mut results: Vec<String> = (0..snapshot.matched_item_count())
            .filter_map(|i| snapshot.get_matched_item(i))
            .map(|item| item.data.clone())
            .collect();

        // when query is empty nucleo returns items in match score order
        // which may differ from original — sort alphabetically for consistency
        if query.is_empty() {
            results.sort();
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_names() -> Vec<String> {
        vec!["catppuccin".into(), "tokyo-night".into(), "agnoster".into()]
    }

    #[test]
    fn test_exact_match_included() {
        let names = make_names();
        let mut fs = FuzzySearch::new(names);
        let results = fs.query("catppuccin");
        assert!(!results.is_empty());
        assert!(results.contains(&"catppuccin".to_string()));
    }

    #[test]
    fn test_no_match_returns_empty() {
        let names = make_names();
        let mut fs = FuzzySearch::new(names);
        let results = fs.query("zzznomatch999");
        assert!(results.is_empty());
    }

    #[test]
    fn test_case_insensitive() {
        let names = make_names();
        let mut fs = FuzzySearch::new(names);
        let results = fs.query("CATPPUCCIN");
        assert!(results.contains(&"catppuccin".to_string()));
    }

    #[test]
    fn test_empty_query_returns_all() {
        let names = make_names();
        let count = names.len();
        let mut fs = FuzzySearch::new(names);
        let results = fs.query("");
        assert_eq!(results.len(), count);
    }
}
