use crate::{
    source_acquisition::source_capture::{CapturedDocument, source_span},
    *,
};
use ast_grep_core::{Pattern, meta_var::MetaVariable};
use ast_grep_language::{LanguageExt, SupportLang};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::collections::BTreeMap;

pub fn resolve_paragraph(
    document: &CapturedDocument,
    target: &TargetId,
) -> EvidenceResult<SelectionContent> {
    let candidates = paragraph_ranges(&document.text);
    let marker = format!("{}:", target.as_str());
    let matches = candidates
        .into_iter()
        .filter(|range| document.text[range.clone()].starts_with(&marker))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [range] => {
            let range = range.clone();
            Ok(SelectionContent::Paragraph {
                span: source_span(&document.source, &document.text, range.clone()),
                text: document.text[range].into(),
            })
        }
        [] => Err(EvidenceCode::MissingTarget.into()),
        _ => Err(EvidenceCode::DuplicateTarget.into()),
    }
}

pub(crate) fn paragraph_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut block_stack = Vec::new();
    let mut paragraph_range: Option<std::ops::Range<usize>> = None;
    let mut candidates = Vec::new();
    for (event, range) in Parser::new(text).into_offset_iter() {
        match &event {
            Event::Start(tag) if is_block_end(tag.to_end()) => {
                if let Some(paragraph) = paragraph_range.take() {
                    candidates.push(paragraph);
                }
                block_stack.push(tag.to_end());
                if matches!(tag, Tag::Paragraph)
                    && !block_stack
                        .iter()
                        .any(|tag| matches!(tag, TagEnd::BlockQuote(_)))
                {
                    paragraph_range = Some(range);
                }
            }
            Event::End(tag) if is_block_end(*tag) => {
                if let Some(paragraph) = paragraph_range.take() {
                    candidates.push(paragraph);
                }
                block_stack.pop();
            }
            Event::Rule => {
                if let Some(paragraph) = paragraph_range.take() {
                    candidates.push(paragraph);
                }
            }
            _ if matches!(block_stack.last(), Some(TagEnd::Item | TagEnd::Paragraph))
                && !block_stack
                    .iter()
                    .any(|tag| matches!(tag, TagEnd::BlockQuote(_))) =>
            {
                // Tight lists omit Paragraph events. Keep the complete inline source range,
                // including markup delimiters and wrapped lines, until the next block boundary.
                if let Some(paragraph) = &mut paragraph_range {
                    paragraph.end = paragraph.end.max(range.end);
                } else {
                    paragraph_range = Some(range);
                }
            }
            _ => {}
        }
    }
    candidates
        .into_iter()
        .map(|range| {
            let end = text[range.clone()].trim_end_matches(['\n', '\r']).len() + range.start;
            range.start..end
        })
        .collect()
}
fn is_block_end(tag: TagEnd) -> bool {
    !matches!(
        tag,
        TagEnd::Emphasis
            | TagEnd::Strong
            | TagEnd::Strikethrough
            | TagEnd::Superscript
            | TagEnd::Subscript
            | TagEnd::Link
            | TagEnd::Image
    )
}

/// Every returned AST match is a syntactic fact. No symbol identity is inferred.
pub fn select_document(
    document: &CapturedDocument,
    selection: &EvidenceSelection,
    max_matches: usize,
) -> EvidenceResult<SelectionContent> {
    let (content, limited) = select_document_bounded(document, selection, max_matches)?;
    if limited {
        return Err(EvidenceCode::SelectorLimit.into());
    }
    Ok(content)
}

pub(crate) fn select_document_bounded(
    document: &CapturedDocument,
    selection: &EvidenceSelection,
    max_matches: usize,
) -> EvidenceResult<(SelectionContent, bool)> {
    let text = document.text.as_ref();
    let make_match = |range: std::ops::Range<usize>, captures| EvidenceMatch {
        span: source_span(&document.source, text, range.clone()),
        text: text[range].into(),
        captures,
    };
    let mut matches = match selection {
        EvidenceSelection::FrontmatterField { field } => {
            return crate::document_evidence::markdown_frontmatter::select_frontmatter(
                document, field,
            )
            .map(|content| (content, false));
        }
        EvidenceSelection::MarkdownUnits(selection) => {
            let (units, limited) = MarkdownDocument::parse(text).select_units(
                &document.source,
                selection,
                max_matches,
            )?;
            return Ok((SelectionContent::MarkdownUnits { units }, limited));
        }
        EvidenceSelection::Paragraph { target_id } => {
            return resolve_paragraph(document, target_id).map(|content| (content, false));
        }
        EvidenceSelection::WholeDocument {} => {
            return Ok((
                SelectionContent::DocumentText {
                    span: source_span(&document.source, text, 0..text.len()),
                    text: text.into(),
                },
                false,
            ));
        }
        EvidenceSelection::LiteralText { text: needle } => {
            if needle.is_empty() {
                return Err(EvidenceCode::InvalidSelector.into());
            }
            text.match_indices(needle)
                .take(max_matches.saturating_add(1))
                .map(|(start, value)| make_match(start..start + value.len(), BTreeMap::new()))
                .collect::<Vec<_>>()
        }
        EvidenceSelection::RegexText { pattern } => {
            let matcher = regex::Regex::new(pattern).map_err(|_| EvidenceCode::InvalidSelector)?;
            matcher
                .captures_iter(text)
                .take(max_matches.saturating_add(1))
                .filter_map(|found| {
                    let whole = found.get(0)?;
                    let captures = matcher
                        .capture_names()
                        .flatten()
                        .filter_map(|name| {
                            found.name(name).map(|value| {
                                (
                                    name.into(),
                                    source_span(&document.source, text, value.range()),
                                )
                            })
                        })
                        .collect();
                    Some(make_match(whole.range(), captures))
                })
                .collect()
        }
        EvidenceSelection::AstPattern { language, pattern } => {
            let language: SupportLang = language
                .parse()
                .map_err(|_| EvidenceCode::UnsupportedLanguage)?;
            let matcher =
                Pattern::try_new(pattern, language).map_err(|_| EvidenceCode::InvalidSelector)?;
            let tree = language.ast_grep(text);
            tree.root()
                .find_all(matcher)
                .take(max_matches.saturating_add(1))
                .map(|found| {
                    let environment = found.get_env();
                    let mut captures = BTreeMap::new();
                    for variable in environment.get_matched_variables() {
                        match variable {
                            MetaVariable::Capture(name, _) => {
                                if let Some(node) = environment.get_match(&name) {
                                    captures.insert(
                                        name,
                                        source_span(&document.source, text, node.range()),
                                    );
                                }
                            }
                            MetaVariable::MultiCapture(name) => {
                                let nodes = environment.get_multiple_matches(&name);
                                if let (Some(first), Some(last)) = (nodes.first(), nodes.last()) {
                                    captures.insert(
                                        name,
                                        source_span(
                                            &document.source,
                                            text,
                                            first.range().start..last.range().end,
                                        ),
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    make_match(found.range(), captures)
                })
                .collect()
        }
        _ => return Err(EvidenceCode::InvalidSelector.into()),
    };
    let limited = matches.len() > max_matches;
    matches.truncate(max_matches);
    Ok((SelectionContent::Matches { matches }, limited))
}

/// Resolve deletion from caller-supplied frozen before/current captures, without rereading either.
pub fn resolve_document_target(
    current: &CapturedDocument,
    previous: Option<&CapturedDocument>,
    target_id: &TargetId,
) -> TargetResolution {
    match resolve_paragraph(current, target_id) {
        Ok(SelectionContent::Paragraph { span, .. }) => TargetResolution::Resolved {
            target: TargetLocation::Paragraph {
                span,
                target_id: target_id.clone(),
            },
        },
        Err(EvidenceError {
            code: EvidenceCode::DuplicateTarget,
            ..
        }) => TargetResolution::Ambiguous,
        Err(EvidenceError {
            code: EvidenceCode::MissingTarget,
            ..
        }) => {
            if previous.is_some_and(|document| resolve_paragraph(document, target_id).is_ok()) {
                TargetResolution::Deleted
            } else {
                TargetResolution::Missing
            }
        }
        _ => TargetResolution::Unavailable,
    }
}
