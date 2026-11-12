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

pub(crate) fn html_sections(raw: &str) -> Vec<HtmlSection> {
    let tree = SupportLang::Html.ast_grep(raw);
    let mut sections: Vec<HtmlSection> = Vec::new();
    let mut open_sections: Vec<usize> = Vec::new();

    for element in tree.root().dfs().filter(|node| node.kind() == "element") {
        let Some(heading_level) = heading_level(&element) else {
            continue;
        };
        if is_in_excluded_source(&element) {
            continue;
        }

        let heading_range = element.range();
        let start = heading_range.start;
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
    sections
}

fn heading_level(element: &HtmlNode<'_>) -> Option<u8> {
    let tag = element
        .children()
        .find(|child| child.kind() == "start_tag")?;
    let name = tag.children().find(|child| child.kind() == "tag_name")?;
    let name = name.text();
    let bytes = name.as_bytes();
    if bytes.len() == 2 && bytes[0].eq_ignore_ascii_case(&b'h') && (b'1'..=b'6').contains(&bytes[1])
    {
        Some(bytes[1] - b'0')
    } else {
        None
    }
}

fn is_in_excluded_source(element: &HtmlNode<'_>) -> bool {
    let mut ancestor = element.parent();
    while let Some(node) = ancestor {
        if node.kind() == "element" && is_excluded_source_element(&node) {
            return true;
        }
        ancestor = node.parent();
    }
    false
}

fn is_excluded_source_element(element: &HtmlNode<'_>) -> bool {
    element
        .children()
        .find(|child| child.kind() == "start_tag")
        .and_then(|tag| tag.children().find(|child| child.kind() == "tag_name"))
        .is_some_and(|name| {
            let name = name.text();
            name.eq_ignore_ascii_case("template") || name.eq_ignore_ascii_case("ex-markdown")
        })
}

#[cfg(test)]
#[path = "html_outline_test.rs"]
mod tests;
