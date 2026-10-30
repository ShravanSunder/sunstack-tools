use crate::{CapturedChange, CapturedFileSide, ChangeSpan, EvidencePiece, FieldValue};

pub fn span_touches_piece(span: &ChangeSpan, start: usize, end: usize) -> bool {
    if span.byte_start == span.byte_end {
        start < span.byte_start && span.byte_start < end
    } else {
        span.byte_start < end && start < span.byte_end
    }
}
pub fn pick_pieces(
    pieces: &[EvidencePiece],
    changes: &[CapturedChange],
    full: bool,
    repository_root: &std::path::Path,
) -> (Vec<usize>, usize) {
    let mut picked = Vec::new();
    let mut outside = 0;
    for (index, piece) in pieces.iter().enumerate() {
        let selected = full
            || changes.iter().any(|change| {
                let checked_hunk = change.path == piece.location.file
                    && (matches!(change.before, CapturedFileSide::Absent)
                        || change.after_spans.iter().any(|span| {
                            span_touches_piece(
                                span,
                                piece.location.span.byte_start,
                                piece.location.span.byte_end,
                            )
                        }));
                let changed_target = piece.fields.values().any(|field| {
                    let FieldValue::Targets { values } = field else {
                        return false;
                    };
                    values.iter().any(|target| {
                        crate::checked_file_source::contained_target_identity(
                            repository_root,
                            &target.path,
                        )
                        .as_ref()
                            == Some(&change.path)
                    })
                });
                checked_hunk || changed_target
            });
        if selected {
            picked.push(index);
        } else {
            outside += 1;
        }
    }
    (picked, outside)
}
#[cfg(test)]
#[path = "evidence_piece_picking_test.rs"]
mod tests;

/// Selection considers a whole loading file whenever a hunk or a resolved target changed.
pub fn selection_files_for_changes(
    files: &[std::path::PathBuf],
    targets: &std::collections::BTreeMap<std::path::PathBuf, Vec<std::path::PathBuf>>,
    changes: &[CapturedChange],
    full: bool,
) -> std::collections::BTreeSet<std::path::PathBuf> {
    files
        .iter()
        .filter(|file| {
            full || changes.iter().any(|change| {
                (&change.path == *file
                    && (!change.after_spans.is_empty()
                        || matches!(change.before, CapturedFileSide::Absent)))
                    || targets
                        .get(*file)
                        .is_some_and(|targets| targets.contains(&change.path))
            })
        })
        .cloned()
        .collect()
}
