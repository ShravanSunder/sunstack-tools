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
) -> (Vec<usize>, usize) {
    let mut picked = Vec::new();
    let mut outside = 0;
    for (index, piece) in pieces.iter().enumerate() {
        let selected=full||changes.iter().any(|change| {
            (change.path==piece.location.file && (matches!(change.before,CapturedFileSide::Absent)
                ||change.after_spans.iter().any(|span|span_touches_piece(span,piece.location.span.byte_start,piece.location.span.byte_end))))
            ||piece.fields.values().any(|field|matches!(field,FieldValue::Targets {values} if values.iter().any(|target|target.path==change.path)))
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
