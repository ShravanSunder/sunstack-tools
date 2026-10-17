//! Markdown source ranges and rule bindings share the block and inline grammars.
use crate::{SourceRef, markdown_contracts::*, source_span};
use ast_grep_core::{AstGrep, Node, tree_sitter::StrDoc};
use ast_grep_language::{LanguageExt, SupportLang};
use std::{collections::BTreeMap, ops::Range};

type BlockNode<'tree> = Node<'tree, StrDoc<SupportLang>>;

pub struct MarkdownDocument {
    tree: AstGrep<StrDoc<SupportLang>>,
}

impl MarkdownDocument {
    pub fn parse(text: &str) -> Self {
        Self {
            tree: SupportLang::Markdown.ast_grep(text),
        }
    }

    pub fn bind_markers(
        &self,
        source: &SourceRef,
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
                    span: source_span(source, text, range.clone()),
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

    pub(crate) fn paragraph_ranges(&self) -> Vec<Range<usize>> {
        self.tree
            .root()
            .dfs()
            .filter(|node| {
                node.kind() == "paragraph"
                    && !node
                        .ancestors()
                        .any(|ancestor| ancestor.kind() == "block_quote")
            })
            .filter_map(|node| {
                node.children()
                    .find(|child| child.kind() == "inline")
                    .map(|inline| inline.range())
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
                    if crate::markdown_projection::inside_code_span(
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
            .find(|node| {
                matches!(
                    node.kind().as_ref(),
                    "atx_heading" | "setext_heading" | "paragraph"
                )
            })
            .ok_or(MarkdownBindingRefusal::ForbiddenHost)?;
        let host = if host.kind() == "paragraph" {
            host.parent()
                .filter(|parent| parent.kind() == "list_item")
                .unwrap_or_else(|| host.clone())
        } else {
            host.clone()
        };
        let (kind, host_range) = match host.kind().as_ref() {
            "atx_heading" | "setext_heading" => {
                if text[..range.start]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace)
                {
                    return Err(MarkdownBindingRefusal::SpacedHeading);
                }
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
            span: source_span(&marker.span.source, text, host_range),
            marker,
            host: kind,
            question_text,
            filter_text,
        })
    }

    fn section_range(&self, heading: &BlockNode<'_>) -> Range<usize> {
        let level = heading_level(heading).unwrap_or(6);
        let start = heading.range().start;
        let end = self
            .tree
            .root()
            .dfs()
            .filter(|node| {
                node.range().start > start && heading_level(node).is_some_and(|next| next <= level)
            })
            .find(|node| {
                !node
                    .ancestors()
                    .any(|ancestor| matches!(ancestor.kind().as_ref(), "block_quote" | "list_item"))
            })
            .map_or(self.tree.source().len(), |node| node.range().start);
        start..end
    }

    fn project_range(&self, range: Range<usize>) -> (String, String) {
        let root = self.tree.root();
        let references = root
            .dfs()
            .filter(|node| node.kind() == "link_reference_definition")
            .filter_map(|node| {
                node.children()
                    .find(|child| child.kind() == "link_label")
                    .map(|label| crate::markdown_projection::reference_label(&label.text()))
            })
            .collect();
        let nodes = root.dfs().filter(|node| {
            contains_range(
                &range,
                &(node.range().start..node.range().start + node.text().trim_end().len()),
            ) && matches!(
                node.kind().as_ref(),
                "paragraph"
                    | "atx_heading"
                    | "setext_heading"
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
        let mut filter = String::new();
        let mut previous_list = false;
        for node in nodes {
            let (visible, filter_visible) = match node.kind().as_ref() {
                "pipe_table" => {
                    crate::markdown_projection::project_inline(node.text().trim(), &references)
                }
                "fenced_code_block" | "indented_code_block" => {
                    let literal = node
                        .children()
                        .find(|child| child.kind() == "code_fence_content")
                        .map_or_else(
                            || node.text().trim().to_owned(),
                            |child| child.text().trim_end().to_owned(),
                        );
                    (literal.clone(), literal)
                }
                _ => {
                    let inline = node.children().find(|child| child.kind() == "inline");
                    let raw = inline.map_or_else(String::new, |child| child.text().into_owned());
                    crate::markdown_projection::project_inline(&raw, &references)
                }
            };
            let list_prefix = node
                .parent()
                .filter(|parent| parent.kind() == "list_item")
                .filter(|parent| {
                    parent
                        .children()
                        .find(|child| {
                            !child.kind().starts_with("list_marker_")
                                && child.kind() != "block_continuation"
                        })
                        .is_some_and(|first| first.range().start == node.range().start)
                })
                .and_then(|parent| {
                    parent
                        .children()
                        .find(|child| child.kind().starts_with("list_marker_"))
                        .map(|marker| {
                            format!(
                                "{}{}",
                                " ".repeat(parent.start_pos().byte_point().1),
                                marker.text()
                            )
                        })
                });
            let separator = if previous_list && list_prefix.is_some() {
                "\n"
            } else {
                "\n\n"
            };
            append_projection(&mut question, &visible, list_prefix.as_deref(), separator);
            append_projection(
                &mut filter,
                &filter_visible,
                list_prefix.as_deref(),
                separator,
            );
            previous_list = list_prefix.is_some();
        }
        (question, filter)
    }
}

fn contains_range(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
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
    text.strip_prefix('[')
        .and_then(|tail| tail.split_once("]:"))
        .is_some_and(|(label, _)| !label.is_empty() && !label.contains(['\n', '\r']))
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
