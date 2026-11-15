use super::*;

pub(super) struct DerivedFieldsContext<'a> {
    pub kinds: &'a BTreeMap<EvidenceKindName, EvidenceKindDefinition>,
    pub file: &'a MatchedFile,
    pub pieces: &'a [EvidencePiece],
    pub own_range: &'a MatchRange,
    pub contents: Option<&'a BTreeMap<PathBuf, Result<String, CheckedReadError>>>,
    pub project: Option<&'a EvidenceProject>,
    pub max_matches: usize,
}

pub(super) fn apply_derived_fields(
    definition: &EvidenceKindDefinition,
    fields: &mut BTreeMap<FieldName, FieldValue>,
    documents_ready: bool,
    context: DerivedFieldsContext<'_>,
) {
    let DerivedFieldsContext {
        kinds,
        file,
        pieces,
        own_range,
        contents,
        project,
        max_matches,
    } = context;
    let Ok(order) = definition.derived_field_order() else {
        return;
    };
    for name in order {
        if !documents_ready && definition.field_needs_document(name) {
            continue;
        }
        let deferral = fields
            .get(name)
            .and_then(FieldValue::selection_deferral)
            .max(
                definition.fields[name]
                    .input_field()
                    .and_then(|input| fields.get(input))
                    .and_then(FieldValue::selection_deferral),
            );
        if let Some(reason) = deferral {
            fields.insert(
                name.clone(),
                FieldValue::Unavailable {
                    reason: reason.field_unavailable(),
                },
            );
            continue;
        }
        if let FieldSource::Captures { regex, of } = &definition.fields[name] {
            let value = super::regex_capture_fields::capture_list(
                regex,
                of.as_ref().and_then(|name| fields.get(name)),
                of.is_some(),
                file,
                own_range,
                contents,
            );
            fields.insert(name.clone(), value);
            continue;
        }
        let needles = match definition.fields[name]
            .containing_field()
            .and_then(|needle| fields.get(needle))
        {
            Some(FieldValue::Text { value, .. }) if !value.is_empty() => vec![value.as_str()],
            Some(FieldValue::Texts { values, .. } | FieldValue::Document { values, .. })
                if values.iter().all(|value| !value.is_empty()) =>
            {
                values.iter().map(String::as_str).collect()
            }
            None if definition.fields[name].containing_field().is_none() => vec![],
            _ => {
                let reason = match definition.fields[name]
                    .containing_field()
                    .and_then(|needle| fields.get(needle))
                {
                    Some(FieldValue::Unavailable { reason }) => *reason,
                    _ => FieldUnavailable::ContainingFieldUnavailable,
                };
                fields.insert(name.clone(), FieldValue::Unavailable { reason });
                continue;
            }
        };
        let has_filter = definition.fields[name].containing_field().is_some();
        let Some(value) = fields.get(name) else {
            continue;
        };
        let filtered = match value {
            FieldValue::Texts { .. }
                if matches!(&definition.fields[name], FieldSource::SameFile { .. }) =>
            {
                let FieldSource::SameFile {
                    of, text, attach, ..
                } = &definition.fields[name]
                else {
                    continue;
                };
                let ranges = source_ranges(of, file, pieces)
                    .into_iter()
                    .filter(|range| {
                        !has_filter
                            || needles.iter().any(|needle| {
                                crate::contains_whole_token(
                                    &file.text[range.start..range.end],
                                    needle,
                                )
                            })
                    })
                    .map(|mut range| {
                        if attach.is_none() && *text == crate::ContextText::FirstLine {
                            range.end = file.text[range.start..range.end]
                                .find('\n')
                                .map_or(range.end, |length| range.start + length + 1);
                        }
                        range
                    })
                    .collect::<Vec<_>>();
                FieldValue::Texts {
                    cut: false,
                    values: ranges
                        .iter()
                        .map(|range| {
                            let value = &file.text[range.start..range.end];
                            if attach.is_some() || *text == crate::ContextText::Full {
                                value.into()
                            } else {
                                value.trim_end_matches(['\r', '\n']).into()
                            }
                        })
                        .collect(),
                    spans: ranges
                        .iter()
                        .map(|range| source_span(file, range))
                        .collect(),
                }
            }
            FieldValue::Texts { values, spans, .. }
            | FieldValue::Document { values, spans, .. } => {
                let selected = values
                    .iter()
                    .zip(spans)
                    .filter(|(value, _)| {
                        !has_filter
                            || needles
                                .iter()
                                .any(|needle| crate::contains_whole_token(value, needle))
                    })
                    .collect::<Vec<_>>();
                let values = selected.iter().map(|(value, _)| (*value).clone()).collect();
                let spans = selected.iter().map(|(_, span)| (*span).clone()).collect();
                match value {
                    FieldValue::Document { path, .. } => FieldValue::Document {
                        cut: value.is_cut(),
                        path: path.clone(),
                        values,
                        spans,
                    },
                    _ => FieldValue::Texts {
                        cut: value.is_cut(),
                        values,
                        spans,
                    },
                }
            }
            _ => value.clone(),
        };
        let attachment = match &definition.fields[name] {
            FieldSource::SameFile { attach, .. } | FieldSource::Document { attach, .. } => {
                attach.as_ref()
            }
            _ => None,
        };
        let mut filtered = filtered;
        if let Some(attach) = attachment {
            filtered = super::sibling_attachment_fields::attach_field(
                &definition.fields[name],
                filtered,
                attach,
                &DerivedFieldsContext {
                    kinds,
                    file,
                    pieces,
                    own_range,
                    contents,
                    project,
                    max_matches,
                },
            );
        }
        if definition.fields[name]
            .input_field()
            .and_then(|input| fields.get(input))
            .is_some_and(FieldValue::is_cut)
        {
            match &mut filtered {
                FieldValue::Texts { cut, .. } | FieldValue::Document { cut, .. } => *cut = true,
                _ => {}
            }
        }
        fields.insert(name.clone(), filtered);
    }
}

#[cfg(test)]
#[path = "derived_fields_test.rs"]
mod tests;
