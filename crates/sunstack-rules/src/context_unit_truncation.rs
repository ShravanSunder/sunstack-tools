//! Mandatory signatures and whole source-line truncation for one indivisible context unit.
use super::*;

fn signature(unit: &ContextUnit) -> &str {
    match &unit.unit.kind {
        SourceUnitKind::Function { signature }
        | SourceUnitKind::Method { signature }
        | SourceUnitKind::Class { signature } => signature,
        SourceUnitKind::Paragraph | SourceUnitKind::Section => "",
    }
}
pub(super) fn signature_header(unit: &ContextUnit) -> String {
    let signature = signature(unit);
    if signature.is_empty() {
        String::new()
    } else {
        format!("signature: {signature}\n")
    }
}
impl PagePiece<'_> {
    pub(super) fn redacted(&self, ranges: &[CodeRedactionRange]) -> usize {
        if self.excluded.is_some() {
            return 0;
        }
        let visible = |range: &CodeRedactionRange| {
            let Some((unit, omitted)) = self.unit.zip(self.omitted_lines.as_ref()) else {
                return true;
            };
            let head_end: usize = unit
                .text
                .split_inclusive('\n')
                .take(omitted.first - unit.unit.span.start_line)
                .map(str::len)
                .sum();
            let tail_start: usize = unit
                .text
                .split_inclusive('\n')
                .take(omitted.last - unit.unit.span.start_line + 1)
                .map(str::len)
                .sum();
            range.byte_end <= head_end || range.byte_start >= tail_start
        };
        let body_count = ranges
            .iter()
            .filter(|range| range.part == self.part && visible(range))
            .count();
        let Some(unit) = self.unit else {
            return body_count;
        };
        let signature_part = CodeTextPart::UnitSignature {
            piece_id: unit.piece_id.clone(),
        };
        body_count
            + ranges
                .iter()
                .filter(|range| range.part == signature_part)
                .filter(|range| {
                    // Signature and body projections can name the same original replacement.
                    !unit.text.find(signature(unit)).is_some_and(|offset| {
                        ranges.iter().any(|body| {
                            body.part == self.part
                                && visible(body)
                                && body.byte_start == offset + range.byte_start
                                && body.byte_end == offset + range.byte_end
                        })
                    })
                })
                .count()
    }
}
pub(super) fn shrink_unit(
    piece: &mut PagePiece<'_>,
    settings: &ContextSettings,
) -> Result<(), DiagnosticList> {
    let unit = piece.unit.ok_or_else(over_budget)?;
    let lines = unit.text.split_inclusive('\n').collect::<Vec<_>>();
    // Keep a source head and tail even when configuration asks for zero lines.
    let (mut head, mut tail) = if let Some(omitted) = &piece.omitted_lines {
        (
            omitted.first - unit.unit.span.start_line,
            unit.unit.span.start_line + lines.len() - 1 - omitted.last,
        )
    } else {
        (
            settings.truncation.head_lines.max(1).min(lines.len()),
            settings.truncation.tail_lines.max(1).min(lines.len()),
        )
    };
    if piece.omitted_lines.is_some() {
        if head >= tail && head > 1 {
            head -= 1;
        } else if tail > 1 {
            tail -= 1;
        } else {
            return Err(over_budget());
        }
    }
    while head.saturating_add(tail) >= lines.len() {
        if head >= tail && head > 1 {
            head -= 1;
        } else if tail > 1 {
            tail -= 1;
        } else {
            return Err(over_budget());
        }
    }
    let omitted_count = lines.len() - head - tail;
    let head_text = lines[..head].concat();
    let tail_text = lines[lines.len() - tail..].concat();
    let gap_ending = if lines[head - 1].ends_with("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let prefix = signature_header(unit);
    piece.body =
        format!("{prefix}{head_text}… {omitted_count} lines truncated …{gap_ending}{tail_text}");
    piece.omitted_lines = Some(OmittedSourceLines {
        first: unit.unit.span.start_line + head,
        last: unit.unit.span.start_line + lines.len() - tail - 1,
    });
    Ok(())
}
