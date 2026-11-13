use std::collections::HashMap;

include!("github_slug_ranges.rs");

/// Generates unique GitHub-compatible heading slugs within one document.
#[derive(Default)]
pub(crate) struct GithubHeadingSlugger {
    occurrences: HashMap<String, usize>,
}

impl GithubHeadingSlugger {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn slug(&mut self, text: &str) -> String {
        let original_slug = github_heading_slug(text);
        let mut result = original_slug.clone();

        while self.occurrences.contains_key(&result) {
            let count = self.occurrences.entry(original_slug.clone()).or_default();
            *count += 1;
            result = format!("{original_slug}-{count}");
        }
        self.occurrences.insert(result.clone(), 0);
        result
    }
}

fn github_heading_slug(text: &str) -> String {
    // Rust and the renderer may use different Unicode lowercase mappings, so
    // headings containing recently changed mappings can produce different slugs.
    text.to_lowercase()
        .chars()
        .filter_map(|character| {
            if is_removed_by_github_slugger(character) {
                None
            } else if character == ' ' {
                Some('-')
            } else {
                Some(character)
            }
        })
        .collect()
}

fn is_removed_by_github_slugger(character: char) -> bool {
    let code_point = u32::from(character);
    let index = REMOVED_SCALAR_RANGES.partition_point(|&(_, end)| end < code_point);
    REMOVED_SCALAR_RANGES
        .get(index)
        .is_some_and(|&(start, _)| start <= code_point)
}

#[cfg(test)]
#[path = "github_heading_slug_test.rs"]
mod tests;
