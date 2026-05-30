use nucleo::{Config, Nucleo, pattern::{CaseMatching, Normalization}};
use std::sync::Arc;

pub struct FuzzySearch {
    matcher: Nucleo<String>,
}

impl FuzzySearch {
    pub fn new(items: Vec<String>) -> Self {
        let matcher = Nucleo::new(
            Config::DEFAULT,
            Arc::new(|| {}),
            None,
            1,
        );

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
        self.matcher.pattern.reparse(
            0,
            query,
            CaseMatching::Ignore,
            Normalization::Smart,
            false,
        );

        loop {
            let status = self.matcher.tick(10);
            if !status.running { break; }
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