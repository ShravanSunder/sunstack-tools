use super::*;

pub(super) fn apply_containing_fields(
    definition: &EvidenceKindDefinition,
    fields: &mut BTreeMap<FieldName, FieldValue>,
    documents_ready: bool,
    file: &MatchedFile,
    pieces: &[EvidencePiece],
) {
    let Ok(order) = definition.containing_field_order() else {
        return;
    };
    for name in order {
        if !documents_ready && definition.field_needs_document(name) {
            continue;
        }
        let Some(needle) = definition.fields[name].containing_field() else {
            continue;
        };
        let needles = match fields.get(needle) {
            Some(FieldValue::Text { value, .. }) if !value.is_empty() => vec![value.as_str()],
            Some(FieldValue::Texts { values, .. } | FieldValue::Document { values, .. })
                if values.iter().all(|value| !value.is_empty()) =>
            {
                values.iter().map(String::as_str).collect()
            }
            _ => {
                let reason = match fields.get(needle) {
                    Some(FieldValue::Unavailable { reason }) => *reason,
                    _ => FieldUnavailable::ContainingFieldUnavailable,
                };
                fields.insert(name.clone(), FieldValue::Unavailable { reason });
                continue;
            }
        };
        let Some(value) = fields.get(name) else {
            continue;
        };
        let filtered = match value {
            FieldValue::Texts { .. }
                if matches!(&definition.fields[name], FieldSource::SameFile { .. }) =>
            {
                let FieldSource::SameFile { of, text, .. } = &definition.fields[name] else {
                    continue;
                };
                let ranges = source_ranges(of, file, pieces)
                    .into_iter()
                    .filter(|range| {
                        needles.iter().any(|needle| {
                            crate::contains_whole_token(&file.text[range.start..range.end], needle)
                        })
                    })
                    .map(|mut range| {
                        if *text == crate::ContextText::FirstLine {
                            range.end = file.text[range.start..range.end]
                                .find('\n')
                                .map_or(range.end, |length| range.start + length + 1);
                        }
                        range
                    })
                    .collect::<Vec<_>>();
                FieldValue::Texts {
                    values: ranges
                        .iter()
                        .map(|range| {
                            let value = &file.text[range.start..range.end];
                            if *text == crate::ContextText::Full {
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
            FieldValue::Texts { values, spans } | FieldValue::Document { values, spans, .. } => {
                let selected = values
                    .iter()
                    .zip(spans)
                    .filter(|(value, _)| {
                        needles
                            .iter()
                            .any(|needle| crate::contains_whole_token(value, needle))
                    })
                    .collect::<Vec<_>>();
                let values = selected.iter().map(|(value, _)| (*value).clone()).collect();
                let spans = selected.iter().map(|(_, span)| (*span).clone()).collect();
                match value {
                    FieldValue::Document { path, .. } => FieldValue::Document {
                        path: path.clone(),
                        values,
                        spans,
                    },
                    _ => FieldValue::Texts { values, spans },
                }
            }
            _ => value.clone(),
        };
        fields.insert(name.clone(), filtered);
    }
}
