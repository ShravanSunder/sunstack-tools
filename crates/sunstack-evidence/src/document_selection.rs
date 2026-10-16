use crate::{
    source_capture::{CapturedDocument, source_span},
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
    let mut quote_depth = 0_usize;
    let mut paragraph_start = None;
    let mut item_start = None;
    let mut candidates = Vec::new();
    for (event, range) in Parser::new(&document.text).into_offset_iter() {
        match event {
            Event::Start(Tag::BlockQuote(_)) => quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote_depth = quote_depth.saturating_sub(1),
            Event::Start(Tag::Paragraph) if quote_depth == 0 => paragraph_start = Some(range.start),
            Event::End(TagEnd::Paragraph) if quote_depth == 0 => {
                if let Some(start) = paragraph_start.take() {
                    candidates.push(start..range.end);
                }
            }
            Event::Start(Tag::Item) if quote_depth == 0 => item_start = Some(range.start),
            Event::Text(_) if quote_depth == 0 && paragraph_start.is_none() => {
                if item_start.take().is_some() {
                    candidates.push(range);
                }
            }
            Event::End(TagEnd::Item) => item_start = None,
            _ => {}
        }
    }
    let marker = format!("{}:", target.as_str());
    let matches = candidates
        .into_iter()
        .filter(|range| document.text[range.clone()].starts_with(&marker))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [range] => {
            let end = document.text[range.clone()]
                .trim_end_matches(['\n', '\r'])
                .len()
                + range.start;
            let range = range.start..end;
            Ok(SelectionContent::Paragraph {
                span: source_span(&document.source, &document.text, range.clone()),
                text: document.text[range].into(),
            })
        }
        [] => Err(EvidenceCode::MissingTarget.into()),
        _ => Err(EvidenceCode::DuplicateTarget.into()),
    }
}
/// Every returned AST match is a syntactic fact. No symbol identity is inferred.
pub fn select_document(
    document: &CapturedDocument,
    selection: &EvidenceSelection,
    max_matches: usize,
) -> EvidenceResult<SelectionContent> {
    let text = document.text.as_ref();
    let make_match = |range: std::ops::Range<usize>, captures| EvidenceMatch {
        span: source_span(&document.source, text, range.clone()),
        text: text[range].into(),
        captures,
    };
    let matches = match selection {
        EvidenceSelection::Paragraph { target_id } => {
            return resolve_paragraph(document, target_id);
        }
        EvidenceSelection::WholeDocument => {
            return Ok(SelectionContent::DocumentText {
                span: source_span(&document.source, text, 0..text.len()),
                text: text.into(),
            });
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
    if matches.len() > max_matches {
        return Err(EvidenceCode::SelectorLimit.into());
    }
    Ok(SelectionContent::Matches { matches })
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
