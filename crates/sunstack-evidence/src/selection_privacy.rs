use crate::*;
use std::collections::BTreeSet;

pub(crate) fn project_readiness(
    readiness: SelectionReadiness,
    grants: &BTreeSet<PrivateKind>,
) -> EvidenceResult<(SelectionReadiness, PrivacyProjection)> {
    let mut value = serde_json::to_value(readiness).map_err(|_| EvidenceCode::InvalidSelector)?;
    let mut privacy = PrivacyProjection::default();
    crate::record_privacy::project_json_text(&mut value, grants, &mut privacy)?;
    let readiness = serde_json::from_value(value).map_err(|_| EvidenceCode::InvalidSelector)?;
    Ok((readiness, privacy))
}

/// Derive a granted view from captured line envelopes; never re-read or decrypt the source.
pub(crate) fn project_transcript(
    capture: &ConversationCapture,
    grants: &BTreeSet<PrivateKind>,
) -> EvidenceResult<(ConversationCapture, PrivacyProjection)> {
    let lines = capture
        .transcript_lines()
        .ok_or(EvidenceCode::AdapterOutputInvalid)?;
    let mut projected_lines = Vec::new();
    let mut privacy = PrivacyProjection::default();
    for line in lines {
        let mut projected = line.clone();
        projected.legacy = LegacyDisposition::Folded;
        projected.events.clear();
        for event in &line.events {
            match event {
                TranscriptEvent::MessagePart {
                    blocks,
                    private_kind,
                    ..
                } => {
                    let blocks =
                        crate::positioned_records::positioned_blocks(blocks, *private_kind);
                    let blocks =
                        crate::record_privacy::retain_private_blocks(blocks, grants, &mut privacy)
                            .into_iter()
                            .map(|(_, block)| block.block)
                            .collect::<Vec<_>>();
                    if !blocks.is_empty() {
                        let mut event = event.clone();
                        if let TranscriptEvent::MessagePart { blocks: target, .. } = &mut event {
                            *target = blocks;
                        }
                        projected.events.push(event);
                    }
                }
                TranscriptEvent::PrivateRecord {
                    record_id,
                    author,
                    position,
                    private_kind,
                    text,
                } => {
                    if !grants.contains(private_kind) {
                        *privacy.left_out.entry(*private_kind).or_default() += 1;
                    } else if let Some(text) = text {
                        projected.events.push(TranscriptEvent::MessagePart {
                            message_id: record_id.clone(),
                            author: *author,
                            position: *position,
                            blocks: vec![MessageBlock::Text { text: text.clone() }],
                            injection: InjectionOrigin::NotInjected,
                            private_kind: Some(*private_kind),
                        });
                    } else {
                        projected.events.push(TranscriptEvent::Gap {
                            gap: CoverageGap::UnsupportedRecord {
                                position: position.byte_offset,
                            },
                        });
                    }
                }
                TranscriptEvent::CustomRecord {
                    fields,
                    field_privacy,
                    ..
                } => {
                    let mut fields = fields.clone();
                    let mut field_privacy = field_privacy.clone();
                    crate::record_privacy::retain_mapped_fields(
                        &mut fields,
                        &mut field_privacy,
                        grants,
                        &mut privacy,
                    );
                    let mut event = event.clone();
                    if let TranscriptEvent::CustomRecord {
                        fields: target,
                        field_privacy: classifications,
                        ..
                    } = &mut event
                    {
                        *target = fields;
                        *classifications = field_privacy;
                    }
                    projected.events.push(event);
                }
                TranscriptEvent::ToolCall { .. }
                | TranscriptEvent::ToolResult { .. }
                | TranscriptEvent::Gap { .. } => projected.events.push(event.clone()),
            }
        }
        projected_lines.push(projected);
    }
    let folded = crate::transcript_normalization::fold_transcript_lines(projected_lines);
    let mut projected = capture.clone();
    projected.records = folded.records;
    projected.turns = folded.turns;
    projected.gaps = folded.gaps;
    projected.coverage.gaps = projected.gaps.clone();
    if let Some(supplement) = &capture.supplement {
        if projected.turns.is_empty() {
            projected.turns.push(ConversationTurn {
                opening: TurnOpening::CaptureStart,
                records: vec![],
                boundary: TurnBoundary::Clear,
            });
        }
        if let Some(turn) = projected.turns.last_mut() {
            turn.records.push(supplement.record_id.clone());
        }
        projected.records.push(supplement.clone());
    }
    Ok((projected, privacy))
}
