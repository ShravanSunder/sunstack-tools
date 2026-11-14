use sunstack_evidence::document_evidence::text_content_measures::count_text_content_words;
use sunstack_evidence::{
    EvidencePiece, FieldName, FieldShape, FieldValue, ModelFieldValue, ModelView,
};

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
        settings: &sunstack_evidence::HtmlEvidenceSettings,
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
        settings: &sunstack_evidence::HtmlEvidenceSettings,
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
                let ModelView::Bounded { fields, cut } = &piece.model_view else {
                    return Err(field.clone());
                };
                if cut.contains(field) {
                    return Err(field.clone());
                }
                let count = |value: &ModelFieldValue| match value {
                    ModelFieldValue::Text(text) => {
                        count_text_content_words(text, settings).map_err(|_| field.clone())
                    }
                    _ => Err(field.clone()),
                };
                match fields.get(field) {
                    Some(ModelFieldValue::Text(_)) => count(&fields[field]),
                    Some(ModelFieldValue::List(values)) => {
                        values.iter().try_fold(0usize, |total, value| {
                            total
                                .checked_add(count(value)?)
                                .ok_or_else(|| field.clone())
                        })
                    }
                    Some(ModelFieldValue::Absent) if matches!(value, FieldValue::Absent) => Ok(0),
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
    settings: &sunstack_evidence::HtmlEvidenceSettings,
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
