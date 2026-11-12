use super::{MarkdownDocument, heading_level};
use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MarkdownSection {
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

#[cfg(test)]
#[path = "section_outline_test.rs"]
mod tests;
