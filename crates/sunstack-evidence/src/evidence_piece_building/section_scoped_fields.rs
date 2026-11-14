use super::*;

pub(super) fn scoped_range(
    file: &MatchedFile,
    scope: &crate::SectionScope,
    subject: &MatchRange,
    pieces: &[EvidencePiece],
) -> Result<MatchRange, FieldUnavailable> {
    let mut range =
        super::section_prefix_fields::nearest_section_range(file, scope.kind(), subject, pieces)?;
    if matches!(scope, crate::SectionScope::Prefix { .. }) {
        range.end = subject.start;
    }
    file.text
        .get(range.start..range.end)
        .ok_or(FieldUnavailable::Unreadable)?;
    Ok(range)
}

pub(super) fn scoped_inside_field(
    file: &MatchedFile,
    kind: &EvidenceKindName,
    scope: &crate::SectionScope,
    subject: &MatchRange,
    pieces: &[EvidencePiece],
) -> FieldValue {
    let unavailable = |reason| FieldValue::Unavailable { reason };
    if file
        .native_units
        .get(kind)
        .map_or(file.index.limited, NativeMatchedKind::is_limited)
    {
        return unavailable(FieldUnavailable::Unreadable);
    }
    let range = match scoped_range(file, scope, subject, pieces) {
        Ok(range) => range,
        Err(reason) => return unavailable(reason),
    };
    let mut selected = pieces
        .iter()
        .filter(|piece| {
            piece.kind == *kind
                && piece.location.span.source == file.source
                && match_within(
                    &MatchRange {
                        start: piece.location.span.byte_start,
                        end: piece.location.span.byte_end,
                    },
                    &range,
                )
        })
        .collect::<Vec<_>>();
    selected.sort_by_key(|piece| (piece.location.span.byte_start, piece.location.span.byte_end));
    FieldValue::Texts {
        values: selected.iter().map(|piece| piece.text.clone()).collect(),
        spans: selected
            .iter()
            .map(|piece| piece.location.span.clone())
            .collect(),
    }
}

#[cfg(test)]
#[path = "section_scoped_fields_test.rs"]
mod tests;
