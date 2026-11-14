use super::*;

pub(super) fn section_prefix_field(
    file: &MatchedFile,
    kind: &EvidenceKindName,
    subject: &MatchRange,
    pieces: &[EvidencePiece],
) -> FieldValue {
    match nearest_section_range(file, kind, subject, pieces) {
        Ok(nearest) => field_composition::text_for_ranges(
            file,
            &[MatchRange {
                start: nearest.start,
                end: subject.start,
            }],
        ),
        Err(reason) => FieldValue::Unavailable { reason },
    }
}

pub(super) fn nearest_section_range(
    file: &MatchedFile,
    kind: &EvidenceKindName,
    subject: &MatchRange,
    pieces: &[EvidencePiece],
) -> Result<MatchRange, FieldUnavailable> {
    // A truncated candidate set cannot establish which configured section is nearest.
    if file
        .native_units
        .get(kind)
        .map_or(file.index.limited, NativeMatchedKind::is_limited)
    {
        return Err(FieldUnavailable::Unreadable);
    }
    let mut containing = source_ranges(&MatchSource::Kind { kind: kind.clone() }, file, pieces)
        .into_iter()
        .filter(|candidate| match_within(subject, candidate))
        .collect::<Vec<_>>();
    containing.sort_by_key(|range| (range.end - range.start, range.start));
    let Some(nearest) = containing.first() else {
        return Err(FieldUnavailable::Unreadable);
    };
    // Crossing configured units have no single nearest container; do not guess by length.
    if !containing.iter().all(|outer| match_within(nearest, outer))
        || file.text.get(nearest.start..subject.start).is_none()
    {
        return Err(FieldUnavailable::Unreadable);
    }
    Ok(nearest.clone())
}

#[cfg(test)]
#[path = "section_prefix_fields_test.rs"]
mod tests;
