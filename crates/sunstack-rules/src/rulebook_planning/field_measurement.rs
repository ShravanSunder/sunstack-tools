use sunstack_evidence::document_evidence::text_content_measures::{
    count_html_content_words, count_text_content_words,
};
use sunstack_evidence::{EvidencePiece, FieldName, FieldShape, FieldValue};

impl crate::FieldMeasurementSettings {
    pub(crate) fn for_kind(
        name: &sunstack_evidence::EvidenceKindName,
        definition: &sunstack_evidence::EvidenceKindDefinition,
        kinds: &std::collections::BTreeMap<
            sunstack_evidence::EvidenceKindName,
            sunstack_evidence::EvidenceKindDefinition,
        >,
        project: &sunstack_evidence::EvidenceProject,
    ) -> Self {
        use sunstack_evidence::{FieldSource, MatchSource, evidence_kind_language};
        let kind_language = |name| {
            kinds
                .get(name)
                .and_then(|kind| evidence_kind_language(project, name, kind))
        };
        let own_language = evidence_kind_language(project, name, definition);
        let field_languages = definition
            .fields
            .iter()
            .filter_map(|(field, source)| {
                let language = match source {
                    FieldSource::Document { rule, .. }
                    | FieldSource::Next { rule, .. }
                    | FieldSource::Previous { rule, .. } => project.language_for_rule(rule),
                    FieldSource::Inside { of }
                    | FieldSource::SameFile { of, .. }
                    | FieldSource::Enclosing { of, .. } => match of {
                        MatchSource::Rule { rule } => project.language_for_rule(rule),
                        MatchSource::Kind { kind } => kind_language(kind),
                    },
                    FieldSource::ScopedInside { kind, .. }
                    | FieldSource::SectionPrefix { kind } => kind_language(kind),
                    FieldSource::FirstLine {}
                    | FieldSource::Section { .. }
                    | FieldSource::Capture { .. }
                    | FieldSource::Remainder { .. } => own_language,
                    // These are already plain strings, not source markup; preserve existing measurement.
                    FieldSource::Captures { .. }
                    | FieldSource::Yaml { .. }
                    | FieldSource::Target { .. } => None,
                };
                language.map(|language| (field.clone(), language))
            })
            .collect();
        Self {
            html_settings: project.document_settings.html.clone(),
            field_languages,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum FieldMeasure {
    Words,
    Pieces,
}

impl FieldMeasure {
    pub(super) fn read_fields(
        self,
        piece: &EvidencePiece,
        fields: &crate::MeasureFields,
        settings: &crate::FieldMeasurementSettings,
    ) -> Result<usize, FieldName> {
        fields.as_slice().iter().try_fold(0usize, |total, field| {
            total
                .checked_add(self.read(piece, field, settings)?)
                .ok_or_else(|| field.clone())
        })
    }
    pub(super) fn supports(self, shape: FieldShape) -> bool {
        match self {
            Self::Words => matches!(shape, FieldShape::Text | FieldShape::Texts),
            Self::Pieces => shape == FieldShape::Texts,
        }
    }

    pub(super) fn read(
        self,
        piece: &EvidencePiece,
        field: &FieldName,
        settings: &crate::FieldMeasurementSettings,
    ) -> Result<usize, FieldName> {
        let value = piece.fields.get(field).ok_or_else(|| field.clone())?;
        if matches!(
            value,
            FieldValue::Unavailable { .. } | FieldValue::MissingDocument { .. }
        ) {
            return Err(field.clone());
        }
        match self {
            Self::Pieces => match value {
                FieldValue::Texts { values, .. } | FieldValue::Document { values, .. } => {
                    Ok(values.len())
                }
                FieldValue::Absent => Ok(0),
                _ => Err(field.clone()),
            },
            Self::Words => {
                let count = |text: &str| {
                    if settings.field_languages.get(field)
                        == Some(&sunstack_evidence::EvidenceLanguage::Html)
                    {
                        count_html_content_words(text, &settings.html_settings)
                    } else {
                        count_text_content_words(text, &settings.html_settings)
                    }
                    .map_err(|_| field.clone())
                };
                match value {
                    FieldValue::Text { value, .. } => count(value),
                    FieldValue::Texts { values, .. } | FieldValue::Document { values, .. } => {
                        values.iter().try_fold(0usize, |total, value| {
                            total
                                .checked_add(count(value)?)
                                .ok_or_else(|| field.clone())
                        })
                    }
                    FieldValue::Absent => Ok(0),
                    _ => Err(field.clone()),
                }
            }
        }
    }
}

pub(super) fn finding_measurements(
    template: &str,
) -> impl Iterator<Item = (&str, FieldMeasure, &str)> {
    template.match_indices("{{").filter_map(|(start, _)| {
        let token = template[start..].split_once("}}")?.0;
        let body = token.strip_prefix("{{")?;
        let (measure, field) = body.split_once(':')?;
        let measure = match measure {
            "word_count" => FieldMeasure::Words,
            "piece_count" => FieldMeasure::Pieces,
            _ => return None,
        };
        Some((&template[start..start + token.len() + 2], measure, field))
    })
}

pub(super) fn render_measured_finding(
    template: &str,
    piece: &EvidencePiece,
    settings: &crate::FieldMeasurementSettings,
) -> Result<String, Option<FieldName>> {
    let mut rendered = String::new();
    let mut remaining = template;
    for (token, measure, name) in finding_measurements(template) {
        let fields = name
            .split(',')
            .map(FieldName::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| None)?;
        let value = measure
            .read_fields(piece, &crate::MeasureFields::Many(fields), settings)
            .map_err(Some)?;
        if let Some((before, after)) = remaining.split_once(token) {
            rendered.push_str(before);
            rendered.push_str(&value.to_string());
            remaining = after;
        }
    }
    rendered.push_str(remaining);
    Ok(rendered)
}

#[cfg(test)]
#[path = "field_measurement_test.rs"]
mod tests;
