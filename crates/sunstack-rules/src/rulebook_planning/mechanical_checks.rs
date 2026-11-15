use crate::DecisionMechanicalTool;
use sunstack_evidence::{
    EvidencePiece, FieldName, FieldValue, FileMatchIndex, MatchRange, ModelView,
    contains_whole_token, match_within,
};

pub(super) fn mechanical_check(
    tool: &DecisionMechanicalTool,
    piece: &EvidencePiece,
    index: &FileMatchIndex,
    measurements: &crate::FieldMeasurementSettings,
) -> Result<bool, FieldName> {
    let field = match tool {
        DecisionMechanicalTool::AstGrep { rule } => {
            return Ok(index.by_rule.get(rule).into_iter().flatten().any(|span| {
                match_within(
                    span,
                    &MatchRange {
                        start: piece.location.span.byte_start,
                        end: piece.location.span.byte_end,
                    },
                )
            }));
        }
        DecisionMechanicalTool::Regex { field, .. }
        | DecisionMechanicalTool::Contains { field, .. }
        | DecisionMechanicalTool::MaxChars { field, .. }
        | DecisionMechanicalTool::FileExists { field }
        | DecisionMechanicalTool::AnchorExists { field }
        | DecisionMechanicalTool::ContainsField { field, .. }
        | DecisionMechanicalTool::Complete { field } => field,
        DecisionMechanicalTool::WordCount { field, bound } => {
            return super::field_measurement::FieldMeasure::Words
                .read_fields(piece, field, measurements)
                .map(|count| bound.matches(count));
        }
        DecisionMechanicalTool::PieceCount { field, bound } => {
            return super::field_measurement::FieldMeasure::Pieces
                .read_fields(piece, field, measurements)
                .map(|count| bound.matches(count));
        }
    };
    if matches!(tool, DecisionMechanicalTool::Complete { .. })
        && matches!(piece.model_view, ModelView::SelectionPending)
    {
        return Err(field.clone());
    }
    let value = piece.fields.get(field);
    if matches!(value, Some(FieldValue::MissingDocument { .. })) {
        return if matches!(tool, DecisionMechanicalTool::Complete { .. }) {
            Ok(false)
        } else {
            Err(field.clone())
        };
    }
    if matches!(value, Some(FieldValue::Unavailable { .. })) {
        return Err(field.clone());
    }
    if matches!(
        tool,
        DecisionMechanicalTool::AnchorExists { .. } | DecisionMechanicalTool::ContainsField { .. }
    ) && matches!(value, None | Some(FieldValue::Absent))
    {
        return Err(field.clone());
    }
    let texts = match value {
        Some(FieldValue::Text { value, .. }) => vec![value.as_str()],
        Some(FieldValue::Texts { values, .. } | FieldValue::Document { values, .. }) => {
            values.iter().map(String::as_str).collect()
        }
        _ => vec![],
    };
    Ok(match tool {
        DecisionMechanicalTool::WordCount { .. } | DecisionMechanicalTool::PieceCount { .. } => {
            false
        }
        DecisionMechanicalTool::Regex { pattern, .. } => regex::Regex::new(pattern)
            .is_ok_and(|regex| texts.iter().any(|text| regex.is_match(text))),
        DecisionMechanicalTool::Contains { text, .. } => {
            texts.iter().any(|value| value.contains(text))
        }
        DecisionMechanicalTool::ContainsField { needle, .. } => {
            let needle_value = match piece.fields.get(needle) {
                Some(FieldValue::Text { value, .. }) if !value.is_empty() => value,
                _ => return Err(needle.clone()),
            };
            texts
                .iter()
                .any(|text| contains_whole_token(text, needle_value))
        }
        DecisionMechanicalTool::AnchorExists { .. } => {
            let Some(FieldValue::Targets { values }) = value else {
                return Err(field.clone());
            };
            let mut found = !values.is_empty();
            for target in values {
                if !target.exists {
                    found = false;
                } else {
                    match &target.anchor {
                        None | Some(sunstack_evidence::TargetAnchor::Found { .. }) => {}
                        Some(sunstack_evidence::TargetAnchor::Missing { .. }) => found = false,
                        Some(sunstack_evidence::TargetAnchor::Unsupported { .. }) => {
                            return Err(field.clone());
                        }
                    }
                }
            }
            found
        }
        DecisionMechanicalTool::MaxChars { max, .. } => {
            texts.first().map_or(0, |text| text.chars().count()) <= *max
        }
        DecisionMechanicalTool::FileExists { .. } => {
            matches!(value, Some(FieldValue::Targets { values }) if !values.is_empty() && values.iter().all(|target| target.exists))
        }
        DecisionMechanicalTool::Complete { .. } => {
            let bounded =
                matches!(&piece.model_view, ModelView::Bounded {cut, ..} if !cut.contains(field));
            bounded
                && match value {
                    Some(FieldValue::Targets { values }) => values.iter().all(|target| {
                        target.exists
                            && target.candidates.is_empty()
                            && target.opening.is_some()
                            && !target.opening_cut
                    }),
                    Some(
                        FieldValue::Text { .. }
                        | FieldValue::Texts { .. }
                        | FieldValue::Document { .. }
                        | FieldValue::Absent,
                    ) => true,
                    Some(FieldValue::Unavailable { .. } | FieldValue::MissingDocument { .. })
                    | None => false,
                }
        }
        DecisionMechanicalTool::AstGrep { .. } => false,
    })
}
