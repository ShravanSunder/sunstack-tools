//! Markdown source ranges and rule bindings share the block and inline grammars.
use crate::document_evidence::markdown_contracts::*;
use ast_grep_core::{AstGrep, Node, tree_sitter::StrDoc};
use ast_grep_language::{LanguageExt, SupportLang};
use std::{collections::BTreeMap, ops::Range};

type BlockNode<'tree> = Node<'tree, StrDoc<SupportLang>>;

mod unit_selection;

pub struct MarkdownDocument {
    tree: AstGrep<StrDoc<SupportLang>>,
}

impl MarkdownDocument {
    /// Match the whole original tree, then retain facts contained in this raw unit.
    pub fn matching_spans<M: ast_grep_core::Matcher>(
        &self,
        matcher: M,
        unit: &crate::MarkdownByteRange,
        max_matches: usize,
    ) -> (Vec<MarkdownTextSpan>, bool) {
        let text = self.tree.source();
        let mut spans = self
            .tree
            .root()
            .find_all(matcher)
            .filter(|found| found.range().start >= unit.start && found.range().end <= unit.end)
            .take(max_matches.saturating_add(1))
            .map(|found| MarkdownTextSpan::from_range(text, found.range()))
            .collect::<Vec<_>>();
        let limited = spans.len() > max_matches;
        spans.truncate(max_matches);
        (spans, limited)
    }

    pub fn parse(text: &str) -> Self {
        Self {
            tree: SupportLang::Markdown.ast_grep(text),
        }
    }

    pub fn bind_markers(
        &self,
        prefix: &MarkerPrefix,
        max_question_words: usize,
    ) -> Vec<MarkdownMarkerBinding> {
        let text = self.tree.source();
        let markers = self.marker_ranges(prefix);
        let mut counts = BTreeMap::new();
        for (id, _) in &markers {
            *counts.entry(id).or_insert(0usize) += 1;
        }
        markers
            .iter()
            .map(|(id, range)| {
                let marker = MarkdownMarker {
                    id: id.clone(),
                    span: MarkdownTextSpan::from_range(text, range.clone()),
                };
                let result = if counts.get(id).copied().unwrap_or_default() > 1 {
                    Err(MarkdownBindingRefusal::DuplicateId)
                } else {
                    self.bind_marker(marker.clone(), max_question_words)
                };
                match result {
                    Ok(binding) => MarkdownMarkerBinding::Bound { binding },
                    Err(reason) => MarkdownMarkerBinding::Refused { marker, reason },
                }
            })
            .collect()
    }

    fn marker_ranges(&self, prefix: &MarkerPrefix) -> Vec<(MarkerId, Range<usize>)> {
        let text = self.tree.source();
        let mut offset = 0;
        let mut markers = Vec::new();
        for line in text.split_inclusive('\n') {
            let line_end = line.trim_end().len();
            if let Some(start) = line[..line_end].rfind("<!-- ") {
                let candidate = &line[start..line_end];
                if candidate.len() == 13
                    && candidate.ends_with(" -->")
                    && line[..start]
                        .bytes()
                        .rev()
                        .take_while(|byte| *byte == b'\\')
                        .count()
                        % 2
                        == 0
                    && let Ok(id) = MarkerId::new(&candidate[5..9])
                    && id.as_str().starts_with(prefix.as_str())
                {
                    let range = offset + start..offset + line_end;
                    if !self.marker_in_code(&range) {
                        markers.push((id, range));
                    }
                }
            }
            offset += line.len();
        }
        markers
    }

    fn marker_in_code(&self, range: &Range<usize>) -> bool {
        let root = self.tree.root();
        for node in root
            .dfs()
            .filter(|node| contains_range(&node.range(), range))
        {
            match node.kind().as_ref() {
                "fenced_code_block" | "indented_code_block" => return true,
                "inline"
                    if crate::document_evidence::markdown_projection::inside_code_span(
                        &node.text(),
                        range.start - node.range().start,
                    ) =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    fn bind_marker(
        &self,
        marker: MarkdownMarker,
        max_question_words: usize,
    ) -> Result<BoundMarkdownText, MarkdownBindingRefusal> {
        let text = self.tree.source();
        let range = marker.span.byte_start..marker.span.byte_end;
        let root = self.tree.root();
        let path = root
            .dfs()
            .filter(|node| contains_range(&node.range(), &range))
            .collect::<Vec<_>>();
        // Section/list wrappers are structural. Every other ancestor must itself be allowed.
        if path.iter().any(|node| {
            !matches!(
                node.kind().as_ref(),
                "document"
                    | "section"
                    | "list"
                    | "list_item"
                    | "paragraph"
                    | "atx_heading"
                    | "setext_heading"
                    | "inline"
            ) && node.is_named()
        }) || path.iter().any(|node| heading_level(node) == Some(1))
            || path
                .iter()
                .any(|node| node.kind() == "paragraph" && definition_prefix(&node.text()))
        {
            return Err(MarkdownBindingRefusal::ForbiddenHost);
        }
        let host = path
            .iter()
            .rev()
            .find(|node| matches!(node.kind().as_ref(), "atx_heading" | "setext_heading"))
            .or_else(|| path.iter().rev().find(|node| node.kind() == "paragraph"))
            .ok_or(MarkdownBindingRefusal::ForbiddenHost)?;
        if heading_level(host).is_some()
            && text[..range.start]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace)
        {
            return Err(MarkdownBindingRefusal::SpacedHeading);
        }
        let host = enclosing_list_item(host)
            .filter(|item| {
                is_first_item_content(item, host)
                    && item.start_pos().byte_point().0 == host.start_pos().byte_point().0
            })
            .unwrap_or_else(|| host.clone());
        let (kind, host_range) = match host.kind().as_ref() {
            "atx_heading" | "setext_heading" => {
                (MarkdownHostKind::Section, self.section_range(&host))
            }
            "list_item" => (MarkdownHostKind::ListItem, host.range()),
            _ => (MarkdownHostKind::Paragraph, host.range()),
        };
        let (question_text, filter_text) = self.project_range(host_range.clone());
        let word_count = question_text.split_whitespace().count();
        if word_count > max_question_words {
            return Err(MarkdownBindingRefusal::QuestionTooLarge {
                word_count,
                max_question_words,
            });
        }
        Ok(BoundMarkdownText {
            span: MarkdownTextSpan::from_range(text, host_range),
            marker,
            host: kind,
            question_text,
            filter_text,
        })
    }

    fn section_range(&self, heading: &BlockNode<'_>) -> Range<usize> {
        let level = heading_level(heading).unwrap_or(6);
        let start = heading.range().start;
        let item_range = enclosing_list_item(heading).map(|item| item.range());
        let boundary = item_range
            .as_ref()
            .map_or(self.tree.source().len(), |range| range.end);
        let end = self
            .tree
            .root()
            .dfs()
            .filter(|node| {
                node.range().start > start
                    && node.range().start < boundary
                    && heading_level(node).is_some_and(|next| next <= level)
            })
            .find(|node| {
                !node
                    .ancestors()
                    .any(|ancestor| ancestor.kind() == "block_quote")
                    && enclosing_list_item(node).map(|item| item.range()) == item_range
            })
            .map_or(boundary, |node| node.range().start);
        start..end
    }

    fn project_range(&self, range: Range<usize>) -> (String, String) {
        let (question, filter) = self.project_mapped_range(range);
        (question, filter.text)
    }

    fn project_mapped_range(
        &self,
        range: Range<usize>,
    ) -> (
        String,
        crate::document_evidence::markdown_mapped_text::MappedMarkdownText,
    ) {
        let normalized =
            crate::document_evidence::markdown_offset_projection::NormalizedMarkdown::new(
                self.tree.source(),
            );
        let projection =
            crate::document_evidence::markdown_offset_projection::OffsetProjection::new(
                &normalized,
            );
        self.project_with_offsets(range, &projection)
    }

    fn project_with_offsets(
        &self,
        range: Range<usize>,
        projection: &crate::document_evidence::markdown_offset_projection::OffsetProjection<'_>,
    ) -> (
        String,
        crate::document_evidence::markdown_mapped_text::MappedMarkdownText,
    ) {
        let root = self.tree.root();
        let nodes = root.dfs().filter(|node| {
            contains_range(
                &range,
                &(node.range().start..node.range().start + node.text().trim_end().len()),
            ) && matches!(
                node.kind().as_ref(),
                "paragraph"
                    | "atx_heading"
                    | "fenced_code_block"
                    | "indented_code_block"
                    | "pipe_table"
            ) && !node.ancestors().any(|ancestor| {
                matches!(
                    ancestor.kind().as_ref(),
                    "html_block" | "minus_metadata" | "plus_metadata" | "link_reference_definition"
                )
            })
        });
        let mut question = String::new();
        let mut filter =
            crate::document_evidence::markdown_mapped_text::MappedMarkdownText::default();
        let mut previous_list = false;
        for node in nodes {
            let (visible, mut filter_visible) = match node.kind().as_ref() {
                "pipe_table" => (
                    projection.table(node.range()).0,
                    projection.mapped_table(node.range(), false),
                ),
                "fenced_code_block" | "indented_code_block" => (
                    projection.code(node.range()).0,
                    projection.mapped_code(node.range()),
                ),
                _ => {
                    let in_list = node
                        .ancestors()
                        .any(|ancestor| ancestor.kind() == "list_item");
                    let quoted = node
                        .ancestors()
                        .any(|ancestor| ancestor.kind() == "block_quote");
                    (
                        projection.prose(node.range(), in_list && !quoted).0,
                        projection.mapped_prose(node.range(), false, in_list && !quoted),
                    )
                }
            };
            let list_prefix = enclosing_list_item(&node)
                .filter(|parent| is_first_item_content(parent, &node))
                .and_then(|parent| {
                    parent
                        .children()
                        .find(|child| child.kind().starts_with("list_marker_"))
                        .filter(|marker| contains_range(&range, &marker.range()))
                        .map(|marker| {
                            format!(
                                "{}{} ",
                                " ".repeat(parent.start_pos().byte_point().1),
                                marker.text().trim_end()
                            )
                        })
                });
            let separator = if previous_list && list_prefix.is_some() {
                "\n"
            } else {
                "\n\n"
            };
            append_projection(&mut question, &visible, list_prefix.as_deref(), separator);
            filter_visible.trim_end();
            if !filter_visible.text.trim().is_empty() {
                let anchor = node.range().start;
                if !filter.text.is_empty() {
                    filter.push(separator, anchor..anchor, false);
                }
                if let Some(prefix) = list_prefix.as_deref() {
                    filter.push(prefix, anchor..anchor, false);
                }
                filter.append(filter_visible);
            }
            previous_list = list_prefix.is_some();
        }
        (question, filter)
    }
}

fn contains_range(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn is_first_item_content(item: &BlockNode<'_>, content: &BlockNode<'_>) -> bool {
    item.children()
        .find(|child| {
            !child.kind().starts_with("list_marker_") && child.kind() != "block_continuation"
        })
        .is_some_and(|first| first.range().start == content.range().start)
}

fn enclosing_list_item<'tree>(node: &BlockNode<'tree>) -> Option<BlockNode<'tree>> {
    node.ancestors()
        .find(|ancestor| ancestor.kind() == "list_item")
}

fn heading_level(node: &BlockNode<'_>) -> Option<u8> {
    if !matches!(node.kind().as_ref(), "atx_heading" | "setext_heading") {
        return None;
    }
    node.children()
        .find_map(|child| match child.kind().as_ref() {
            "atx_h1_marker" | "setext_h1_underline" => Some(1),
            "atx_h2_marker" | "setext_h2_underline" => Some(2),
            "atx_h3_marker" => Some(3),
            "atx_h4_marker" => Some(4),
            "atx_h5_marker" => Some(5),
            "atx_h6_marker" => Some(6),
            _ => None,
        })
}

fn definition_prefix(text: &str) -> bool {
    let Some(tail) = text.trim_start().strip_prefix('[') else {
        return false;
    };
    let mut escaped = false;
    for (offset, character) in tail.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == ']' {
            return !tail[..offset].trim().is_empty() && tail[offset + 1..].starts_with(':');
        }
    }
    false
}

fn append_projection(output: &mut String, visible: &str, prefix: Option<&str>, separator: &str) {
    let visible = visible.trim_end();
    if visible.trim().is_empty() {
        return;
    }
    if !output.is_empty() {
        output.push_str(separator);
    }
    if let Some(prefix) = prefix {
        output.push_str(prefix);
    }
    output.push_str(visible);
}

#[cfg(test)]
#[path = "markdown_document_test.rs"]
mod tests;
