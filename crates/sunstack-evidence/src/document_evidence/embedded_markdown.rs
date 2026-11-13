use ast_grep_core::{Node, tree_sitter::StrDoc};
use ast_grep_language::{LanguageExt, SupportLang};
use std::ops::Range;

use super::{html_content::decode_entities, markdown_document::MarkdownDocument};
use crate::{
    HtmlEvidenceSettings, MarkdownOccurrence, MarkdownUnitSelection, SelectedMarkdownUnit,
    SourceRef, source_span,
};

type HtmlNode<'tree> = Node<'tree, StrDoc<SupportLang>>;

/// The parser sees original bytes, so translation adds only the inner source start.
pub(crate) fn island_ranges(text: &str, settings: &HtmlEvidenceSettings) -> Vec<Range<usize>> {
    if settings.embedded_markdown.is_empty() {
        return Vec::new();
    }
    let tree = SupportLang::Html.ast_grep(text);
    let mut ranges = Vec::<Range<usize>>::new();
    for element in tree.root().dfs().filter(|node| {
        matches!(
            node.kind().as_ref(),
            "element" | "script_element" | "style_element"
        )
    }) {
        let Some(opening) = element.children().find(|node| node.kind() == "start_tag") else {
            continue;
        };
        let Some(closing) = element
            .children()
            .find(|node| node.kind() == "end_tag" && !node.is_missing())
        else {
            continue;
        };
        if opening.dfs().any(|node| node.is_missing())
            || closing.dfs().any(|node| node.is_missing())
            || closing.range().start < opening.range().end
        {
            continue;
        }
        if !settings
            .embedded_markdown
            .iter()
            .any(|selector| opening_matches(&opening, selector))
        {
            continue;
        }
        let range = opening.range().end..closing.range().start;
        // DFS visits the outer element before any nested selected element.
        if !ranges
            .iter()
            .any(|outer| outer.start <= range.start && range.end <= outer.end)
        {
            ranges.push(range);
        }
    }
    ranges.sort_by_key(|range| (range.start, range.end));
    ranges
}

fn opening_matches(opening: &HtmlNode<'_>, selector: &crate::HtmlMarkdownIsland) -> bool {
    opening
        .children()
        .find(|node| node.kind() == "tag_name")
        .is_some_and(|name| name.text().eq_ignore_ascii_case(selector.tag.as_str()))
        && selector.attributes.iter().all(|(name, expected)| {
            opening
                .children()
                .filter(|node| node.kind() == "attribute")
                .any(|attribute| {
                    attribute.children().any(|part| {
                        part.kind() == "attribute_name"
                            && part.text().eq_ignore_ascii_case(name.as_str())
                    }) && attribute
                        .dfs()
                        .find(|part| part.kind() == "attribute_value")
                        .map_or_else(String::new, |part| decode_entities(&part.text()))
                        == *expected
                })
        })
}

pub(crate) fn select_island_units(
    text: &str,
    source: &SourceRef,
    ranges: &[Range<usize>],
    selection: &MarkdownUnitSelection,
    max_matches: usize,
) -> crate::EvidenceResult<(Vec<SelectedMarkdownUnit>, bool)> {
    selection.validate()?;
    let mut units = Vec::new();
    let mut limited = false;
    for range in ranges {
        let (mut selected, cut) = MarkdownDocument::parse(&text[range.clone()]).select_units(
            source,
            selection,
            max_matches.saturating_sub(units.len()),
        )?;
        for unit in &mut selected {
            let translate = |span: &crate::SourceSpan| {
                source_span(
                    source,
                    text,
                    range.start + span.byte_start..range.start + span.byte_end,
                )
            };
            unit.span = translate(&unit.span);
            unit.occurrence = match &unit.occurrence {
                MarkdownOccurrence::Unit { span } => MarkdownOccurrence::Unit {
                    span: translate(span),
                },
                MarkdownOccurrence::CallSite {
                    unit_span,
                    match_span,
                    visible_range,
                } => MarkdownOccurrence::CallSite {
                    unit_span: translate(unit_span),
                    match_span: translate(match_span),
                    visible_range: visible_range.clone(),
                },
            };
        }
        units.extend(selected);
        if cut {
            limited = true;
            break;
        }
    }
    Ok((units, limited))
}

pub(crate) fn island_outline(
    text: &str,
    ranges: &[Range<usize>],
) -> Vec<super::markdown_document::section_outline::MarkdownSection> {
    let mut outline = Vec::new();
    for range in ranges {
        let mut sections = MarkdownDocument::parse(&text[range.clone()]).section_outline();
        let index_offset = outline.len();
        for section in &mut sections {
            for span in std::iter::once(&mut section.heading_range)
                .chain(std::iter::once(&mut section.section_range))
                .chain(section.own_ranges.iter_mut())
            {
                span.start += range.start;
                span.end += range.start;
            }
            section.enclosing_index = section.enclosing_index.map(|index| index + index_offset);
        }
        outline.extend(sections);
    }
    outline
}

#[cfg(test)]
#[path = "embedded_markdown_test.rs"]
mod tests;
