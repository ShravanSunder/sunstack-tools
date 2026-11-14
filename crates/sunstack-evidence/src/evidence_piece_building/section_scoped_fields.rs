use super::*;

pub(super) fn scoped_range(
    file: &MatchedFile,
    scope: &crate::SectionScope,
    subject: &MatchRange,
    pieces: &[EvidencePiece],
) -> Result<MatchRange, FieldUnavailable> {
    let mut range = if matches!(scope, crate::SectionScope::Own { .. }) {
        let kinds = scope.kinds();
        if kinds.iter().any(|kind| {
            file.native_units
                .get(kind)
                .map_or(file.index.limited, NativeMatchedKind::is_limited)
        }) {
            return Err(FieldUnavailable::Unreadable);
        }
        let outlines = pieces
            .iter()
            .filter(|piece| {
                kinds.contains(&piece.kind) && piece.location.span.source == file.source
            })
            .collect::<Vec<_>>();
        let mut containing = outlines
            .iter()
            .filter(|piece| {
                match_within(
                    subject,
                    &MatchRange {
                        start: piece.location.span.byte_start,
                        end: piece.location.span.byte_end,
                    },
                )
            })
            .collect::<Vec<_>>();
        containing
            .sort_by_key(|piece| piece.location.span.byte_end - piece.location.span.byte_start);
        let nearest = containing.first().ok_or(FieldUnavailable::Unreadable)?;
        let start = nearest.location.span.byte_start;
        let mut end = nearest.location.span.byte_end;
        if !containing.iter().all(|piece| {
            piece.location.span.byte_start <= start && end <= piece.location.span.byte_end
        }) {
            return Err(FieldUnavailable::Unreadable);
        }
        if let Some(next) = outlines
            .iter()
            .map(|piece| piece.location.span.byte_start)
            .filter(|next| *next > start)
            .min()
        {
            end = end.min(next);
        }
        MatchRange { start, end }
    } else {
        super::section_prefix_fields::nearest_section_range(
            file,
            &scope.kinds()[0],
            subject,
            pieces,
        )?
    };
    match scope {
        crate::SectionScope::Prefix { .. } => range.end = subject.start,
        crate::SectionScope::Section { .. } | crate::SectionScope::Own { .. } => {}
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
