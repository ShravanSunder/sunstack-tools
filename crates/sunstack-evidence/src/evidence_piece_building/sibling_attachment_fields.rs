use super::*;

/// Extend selected leading matches; filtering always runs before this operation.
pub(super) fn attach_field(
    source: &FieldSource,
    value: FieldValue,
    attach: &crate::SiblingAttachment,
    context: &super::derived_fields::DerivedFieldsContext<'_>,
) -> FieldValue {
    let owner = context.file;
    let contents = context.contents;
    let project = context.project;
    let max_matches = context.max_matches;
    let (values, spans, mut cut, path) = match value {
        FieldValue::Texts { values, spans, cut } => (values, spans, cut, None),
        FieldValue::Document {
            values,
            spans,
            cut,
            path,
        } => (values, spans, cut, Some(path)),
        other => return other,
    };
    let unavailable = || FieldValue::Unavailable {
        reason: FieldUnavailable::AttachmentUnavailable,
    };
    if values.len() != spans.len() {
        return unavailable();
    }
    let mut attached_values = Vec::new();
    let mut attached_spans = Vec::new();
    let mut remote_indexes = BTreeMap::new();
    for span in spans {
        let (text, index) =
            if !matches!(source, FieldSource::Document { .. }) && span.source == owner.source {
                (owner.text.as_str(), &owner.index)
            } else {
                let SourceRef::Document { path, .. } = &span.source else {
                    return unavailable();
                };
                let Some(text) = contents
                    .and_then(|contents| contents.get(path))
                    .and_then(|text| text.as_ref().ok())
                else {
                    return unavailable();
                };
                let Some(project) = project else {
                    return unavailable();
                };
                let FieldSource::Document { rule, .. } = source else {
                    return unavailable();
                };
                let Some(language) = project.language_for_rule(rule) else {
                    return unavailable();
                };
                let remote = remote_indexes.entry(path.clone()).or_insert_with(|| {
                    project.match_file_as_language(path, text, max_matches, language)
                });
                if remote.limited || remote.failed_languages.contains(&language) {
                    return unavailable();
                }
                (text.as_str(), &*remote)
            };
        let rule = match source {
            FieldSource::Document { rule, .. }
            | FieldSource::SameFile {
                of: MatchSource::Rule { rule },
                ..
            } => Some(rule.clone()),
            FieldSource::SameFile {
                of: MatchSource::Kind { kind },
                ..
            } => context
                .kinds
                .get(kind)
                .and_then(|definition| definition.matcher.ast_rule_id(kind)),
            _ => None,
        };
        let entry = rule.as_ref().and_then(|rule| {
            index
                .by_rule
                .get(rule)
                .zip(index.match_nodes.get(rule))
                .and_then(|(ranges, nodes)| {
                    ranges
                        .iter()
                        .zip(nodes)
                        .find(|(range, _)| {
                            range.start == span.byte_start && range.end == span.byte_end
                        })
                        .and_then(|(_, node)| index.parents.get(node))
                })
        });
        let Some(entry) = entry else {
            return unavailable();
        };
        let boundary = index.match_nodes.get(&attach.until);
        let mut end = entry.children[entry.position].range.end;
        let mut count = 0;
        for sibling in &entry.children[entry.position + 1..] {
            if sibling.continuation {
                continue;
            }
            if sibling.heading || boundary.is_some_and(|nodes| nodes.contains(&sibling.node)) {
                break;
            }
            if count == attach.max_siblings.get() {
                cut = true;
                break;
            }
            end = sibling.range.end;
            count += 1;
        }
        let Some(block) = text.get(span.byte_start..end) else {
            return unavailable();
        };
        // Keep one result per leading match, even when their attached contexts overlap.
        attached_values.push(block.to_owned());
        attached_spans.push(crate::source_acquisition::source_capture::source_span(
            &span.source,
            text,
            span.byte_start..end,
        ));
    }
    match path {
        Some(path) => FieldValue::Document {
            path,
            values: attached_values,
            spans: attached_spans,
            cut,
        },
        None => FieldValue::Texts {
            values: attached_values,
            spans: attached_spans,
            cut,
        },
    }
}

#[cfg(test)]
#[path = "sibling_attachment_fields_test.rs"]
mod tests;
