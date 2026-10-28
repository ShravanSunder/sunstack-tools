use crate::source_capture::file_identity;
use crate::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ConversationCaptureRequest {
    pub transcript_path: PathBuf,
    pub format: TranscriptFormatName,
    pub cursor: Option<String>,
    pub final_assistant_text: Option<String>,
    pub max_bytes: usize,
}
impl MonitoringCursor {
    pub fn encode(&self) -> EvidenceResult<String> {
        serde_json::to_vec(self)
            .map(|bytes| URL_SAFE_NO_PAD.encode(bytes))
            .map_err(|_| EvidenceCode::InvalidCursor.into())
    }
    pub fn decode(encoded: &str) -> EvidenceResult<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| EvidenceCode::InvalidCursor)?;
        serde_json::from_slice(&bytes).map_err(|_| EvidenceCode::InvalidCursor.into())
    }
}
pub async fn capture_conversation(
    request: ConversationCaptureRequest,
    registry: AdapterRegistry,
    cancellation: CancellationToken,
) -> EvidenceResult<ConversationCapture> {
    capture_conversation_accounted(ConversationAcquisition {
        request,
        registry,
        cancellation,
        progress: crate::acquisition_progress::AcquisitionProgress::shared(),
    })
    .await
}
pub(crate) struct ConversationAcquisition {
    pub request: ConversationCaptureRequest,
    pub registry: AdapterRegistry,
    pub cancellation: CancellationToken,
    pub progress: Arc<crate::acquisition_progress::AcquisitionProgress>,
}
pub(crate) async fn capture_conversation_accounted(
    acquisition: ConversationAcquisition,
) -> EvidenceResult<ConversationCapture> {
    // Resolve the adapter before any file read.
    acquisition.registry.adapter(&acquisition.request.format)?;
    tokio::task::spawn_blocking(move || capture_blocking(acquisition))
        .await
        .map_err(|_| EvidenceCode::TranscriptUnavailable)?
}
fn capture_blocking(acquisition: ConversationAcquisition) -> EvidenceResult<ConversationCapture> {
    let ConversationAcquisition {
        request,
        registry,
        cancellation,
        progress,
    } = acquisition;
    let started_at = chrono::Utc::now();
    let previous_cursor = request
        .cursor
        .as_deref()
        .map(MonitoringCursor::decode)
        .transpose()?;
    let path = request
        .transcript_path
        .canonicalize()
        .map_err(|_| EvidenceCode::TranscriptUnavailable)?;
    let mut file = File::open(&path).map_err(|_| EvidenceCode::TranscriptUnavailable)?;
    let before = file
        .metadata()
        .map_err(|_| EvidenceCode::TranscriptUnavailable)?;
    let source = TranscriptSource {
        path: path.clone(),
        file_identity: file_identity(&before),
    };
    if let Some(cursor) = &previous_cursor
        && (cursor.source != source
            || cursor.format != request.format
            || before.len() < cursor.committed_bytes)
    {
        return Err(EvidenceCode::TranscriptDiscontinuous.into());
    }
    let mut bytes = Vec::new();
    if before.len() > request.max_bytes as u64 {
        return Err(EvidenceCode::TranscriptLimit.into());
    }
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        if cancellation.is_cancelled() {
            return Err(EvidenceCode::MissingContext.into());
        }
        let remaining = request
            .max_bytes
            .saturating_sub(bytes.len())
            .min(buffer.len());
        if remaining == 0 {
            break;
        }
        let count = file
            .read(&mut buffer[..remaining])
            .map_err(|_| EvidenceCode::TranscriptUnavailable)?;
        if count == 0 {
            break;
        }
        progress.transcript_bytes(count);
        bytes.extend_from_slice(&buffer[..count]);
    }
    let at_path = path
        .metadata()
        .map_err(|_| EvidenceCode::TranscriptDiscontinuous)?;
    if file_identity(&at_path) != source.file_identity {
        return Err(EvidenceCode::TranscriptDiscontinuous.into());
    }
    let framing = registry
        .event_reader(&request.format)
        .map(|reader| reader.framing())
        .unwrap_or(TranscriptFraming::JsonLines);
    let committed = if framing == TranscriptFraming::JsonDocument {
        bytes.len()
    } else {
        bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |position| position + 1)
    };
    let document_complete = framing != TranscriptFraming::JsonDocument
        || serde_json::from_slice::<serde_json::Value>(&bytes).is_ok();
    let tail = if committed == bytes.len() && document_complete {
        CaptureTail::Complete
    } else {
        CaptureTail::Incomplete {
            bytes: if document_complete {
                (bytes.len() - committed) as u64
            } else {
                bytes.len() as u64
            },
        }
    };
    if let Some(cursor) = &previous_cursor {
        let prefix_length =
            usize::try_from(cursor.committed_bytes).map_err(|_| EvidenceCode::InvalidCursor)?;
        if prefix_length > committed
            || content_digest(&bytes[..prefix_length]) != cursor.prefix_digest
        {
            return Err(EvidenceCode::TranscriptDiscontinuous.into());
        }
    }
    let captured = CapturedTranscript {
        bytes: &bytes[..committed],
        source_identity: &source.file_identity,
    };
    let lines = registry
        .event_reader(&request.format)
        .ok()
        .map(|reader| reader.lines(&captured));
    let output = match &lines {
        Some(lines) => crate::transcript_normalization::fold_transcript_lines(lines.clone()),
        None => registry.adapter(&request.format)?.normalize(&captured),
    };
    if cancellation.is_cancelled() {
        return Err(EvidenceCode::MissingContext.into());
    }
    validate_adapter_output(&output, committed as u64)?;
    let mut records = output.records;
    let mut turns = output.turns;
    let mut watermarks: BTreeMap<RecordId, u64> = records
        .iter()
        .map(|record| (record.record_id.clone(), record.position))
        .collect();
    let mut supplement_digest = previous_cursor
        .as_ref()
        .and_then(|cursor| cursor.supplement_digest.clone())
        .filter(|digest| !records.iter().any(|record| {
            matches!(&record.body, ConversationBody::AssistantMessage { blocks } if content_digest(message_text(blocks).as_bytes()) == *digest)
        }));
    let supplement = if let Some(final_text) =
        request.final_assistant_text.filter(|text| !text.is_empty())
    {
        let found = records.iter().any(|record| matches!(&record.body, ConversationBody::AssistantMessage { blocks } if message_text(blocks) == final_text));
        if found {
            None
        } else {
            let digest = content_digest(final_text.as_bytes());
            supplement_digest = Some(digest.clone());
            let record_id = RecordId::new(format!("hook:{digest}"))?;
            let record = ConversationRecord {
                record_id: record_id.clone(),
                position: committed as u64,
                body: ConversationBody::AssistantMessage {
                    blocks: vec![MessageBlock::Text { text: final_text }],
                },
                provenance: RecordProvenance::HookSupplement,
                injection: InjectionOrigin::NotInjected,
            };
            if turns.is_empty() {
                turns.push(ConversationTurn {
                    opening: TurnOpening::CaptureStart,
                    records: Vec::new(),
                    boundary: TurnBoundary::Clear,
                });
            }
            if let Some(turn) = turns.last_mut() {
                turn.records.push(record_id.clone());
            }
            watermarks.insert(record_id, committed as u64);
            records.push(record.clone());
            Some(record)
        }
    } else {
        None
    };
    let cursor = MonitoringCursor {
        source: source.clone(),
        format: request.format.clone(),
        committed_bytes: committed as u64,
        prefix_digest: content_digest(&bytes[..committed]),
        record_watermarks: watermarks,
        supplement_digest,
    };
    let source_ref = SourceRef::Transcript {
        path,
        format: request.format.clone(),
        committed_bytes: committed as u64,
    };
    let coverage = CaptureCoverage {
        source: source_ref,
        cursor_basis: if previous_cursor.is_some() {
            CursorBasis::ContinuedFromCursor
        } else {
            CursorBasis::NoCursorSupplied
        },
        tail: tail.clone(),
        gaps: output.gaps.clone(),
        bytes_read: bytes.len() as u64,
        bytes_parsed: committed as u64,
    };
    Ok(ConversationCapture {
        interval: CaptureInterval {
            started_at,
            completed_at: chrono::Utc::now(),
        },
        source,
        format: request.format,
        committed_bytes: committed as u64,
        records,
        turns,
        tail,
        gaps: output.gaps,
        supplement,
        coverage,
        cursor,
        previous_cursor,
        lines,
    })
}
pub(crate) fn message_text(blocks: &[MessageBlock]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            MessageBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}
