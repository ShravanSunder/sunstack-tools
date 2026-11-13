use super::{BlockNode, MarkdownDocument, heading_level};
use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MarkdownSection {
    pub heading_range: Range<usize>,
    pub heading_text: String,
    pub section_range: Range<usize>,
    pub own_ranges: Vec<Range<usize>>,
    pub enclosing_index: Option<usize>,
    pub heading_level: u8,
}

fn strictly_contains(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end && outer != inner
}

impl MarkdownDocument {
    pub(crate) fn section_outline(&self) -> Vec<MarkdownSection> {
        let mut sections = self
            .tree
            .root()
            .dfs()
            .filter(|node| matches!(node.kind().as_ref(), "atx_heading" | "setext_heading"))
            .map(|node| MarkdownSection {
                heading_range: node.range(),
                heading_text: heading_inline_source(&node),
                section_range: self.section_range(&node),
                own_ranges: Vec::new(),
                enclosing_index: None,
                heading_level: heading_level(&node).unwrap_or(6),
            })
            .collect::<Vec<_>>();

        for index in 0..sections.len() {
            let range = sections[index].section_range.clone();
            let level = sections[index].heading_level;
            let enclosing_index = sections
                .iter()
                .enumerate()
                .filter(|(_, candidate)| {
                    candidate.heading_level < level
                        && strictly_contains(&candidate.section_range, &range)
                })
                .min_by_key(|(_, candidate)| candidate.section_range.len())
                .map(|(position, _)| position);

            let mut nested = sections
                .iter()
                .enumerate()
                .filter(|(position, candidate)| {
                    *position != index && strictly_contains(&range, &candidate.section_range)
                })
                .map(|(_, candidate)| candidate.section_range.clone())
                .collect::<Vec<_>>();
            nested.sort_by_key(|child| (child.start, child.end));
            let mut own_ranges = Vec::new();
            let mut cursor = range.start;
            for child in nested {
                if cursor < child.start {
                    own_ranges.push(cursor..child.start);
                }
                cursor = cursor.max(child.end);
            }
            if cursor < range.end {
                own_ranges.push(cursor..range.end);
            }
            sections[index].own_ranges = own_ranges;
            sections[index].enclosing_index = enclosing_index;
        }
        sections
    }
}

fn heading_inline_source(heading: &BlockNode<'_>) -> String {
    let Some(content) = heading.field("heading_content") else {
        return String::new();
    };
    let text = content.text();
    let text = text.trim_matches([' ', '\t', '\r', '\n']);
    if heading.kind() == "atx_heading" {
        let without_hashes = text.trim_end_matches('#');
        if without_hashes.is_empty() || without_hashes.ends_with([' ', '\t']) {
            return without_hashes.trim_end_matches([' ', '\t']).into();
        }
    }
    text.into()
}

#[cfg(test)]
#[path = "section_outline_test.rs"]
mod tests;
