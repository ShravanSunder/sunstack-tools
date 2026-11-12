use std::ops::Range;

use ast_grep_core::{Node, tree_sitter::StrDoc};
use ast_grep_language::{LanguageExt, SupportLang};

type HtmlNode<'tree> = Node<'tree, StrDoc<SupportLang>>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct HtmlSection {
    pub heading_range: Range<usize>,
    pub section_range: Range<usize>,
    pub own_range: Range<usize>,
    pub enclosing_index: Option<usize>,
    pub heading_level: u8,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum HtmlOutlineError {
    #[error("html_outline_unplaceable_heading at bytes {start}..{end}")]
    UnplaceableHeading { start: usize, end: usize },
}

#[derive(PartialEq, Eq)]
enum ExcludedSource {
    Template,
    MarkdownIsland,
}

pub(crate) fn html_sections(raw: &str) -> Result<Vec<HtmlSection>, HtmlOutlineError> {
    let tree = SupportLang::Html.ast_grep(raw);
    let mut sections: Vec<HtmlSection> = Vec::new();
    let mut open_sections: Vec<usize> = Vec::new();
    let mut excluded_sources = Vec::new();

    // Recovery may put valid opening tags directly inside ERROR instead of element nodes.
    for tag in tree.root().dfs().filter(|node| {
        matches!(
            node.kind().as_ref(),
            "start_tag" | "self_closing_tag" | "end_tag"
        )
    }) {
        let Some(name) = tag.children().find(|child| child.kind() == "tag_name") else {
            continue;
        };
        let name = name.text();
        let excluded = excluded_source(&name);
        if tag.kind() == "end_tag" {
            if let Some(source) = excluded
                && let Some(index) = excluded_sources.iter().rposition(|open| *open == source)
            {
                excluded_sources.truncate(index);
            }
            continue;
        }
        if let Some(source) = excluded {
            if tag.kind() != "self_closing_tag" {
                excluded_sources.push(source);
            }
            continue;
        }
        if !excluded_sources.is_empty() {
            continue;
        }
        let Some(heading_level) = heading_level(&name) else {
            continue;
        };

        let heading_range = heading_range(&tag);
        let start = heading_range.start;
        if start >= heading_range.end
            || raw.get(heading_range.clone()).is_none()
            || sections
                .last()
                .is_some_and(|section| section.section_range.start >= start)
        {
            return Err(HtmlOutlineError::UnplaceableHeading {
                start,
                end: heading_range.end,
            });
        }
        while let Some(&index) = open_sections.last() {
            if sections[index].heading_level < heading_level {
                break;
            }
            sections[index].section_range.end = start;
            sections[index].own_range.end = sections[index].own_range.end.min(start);
            open_sections.pop();
        }
        let enclosing_index = open_sections.last().copied();
        if let Some(index) = enclosing_index {
            sections[index].own_range.end = sections[index].own_range.end.min(start);
        }
        sections.push(HtmlSection {
            heading_range,
            section_range: start..raw.len(),
            own_range: start..raw.len(),
            enclosing_index,
            heading_level,
        });
        open_sections.push(sections.len() - 1);
    }
    Ok(sections)
}

fn heading_level(name: &str) -> Option<u8> {
    let bytes = name.as_bytes();
    if bytes.len() == 2 && bytes[0].eq_ignore_ascii_case(&b'h') && (b'1'..=b'6').contains(&bytes[1])
    {
        Some(bytes[1] - b'0')
    } else {
        None
    }
}

fn heading_range(tag: &HtmlNode<'_>) -> Range<usize> {
    if let Some(element) = tag.parent()
        && element.kind() == "element"
        && element.range().start == tag.range().start
        && element.children().any(|child| child.kind() == "end_tag")
    {
        return element.range();
    }
    tag.range()
}

fn excluded_source(name: &str) -> Option<ExcludedSource> {
    if name.eq_ignore_ascii_case("template") {
        Some(ExcludedSource::Template)
    } else if name.eq_ignore_ascii_case("ex-markdown") {
        Some(ExcludedSource::MarkdownIsland)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "html_outline_test.rs"]
mod tests;
