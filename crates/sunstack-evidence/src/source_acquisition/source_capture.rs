use crate::*;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path, sync::Arc};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct CaptureInterval {
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Clone, Debug)]
pub struct CapturedDocument {
    pub interval: CaptureInterval,
    pub source: SourceRef,
    pub text: Arc<str>,
}
pub fn content_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn text_position(text: &str, offset: usize) -> (usize, usize) {
    let prefix = &text[..offset];
    (
        prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
        prefix
            .rsplit('\n')
            .next()
            .map_or(1, |line| line.chars().count() + 1),
    )
}
pub fn source_span(source: &SourceRef, text: &str, range: std::ops::Range<usize>) -> SourceSpan {
    let (start_line, start_column) = text_position(text, range.start);
    let (end_line, end_column) = text_position(text, range.end);
    SourceSpan {
        source: source.clone(),
        byte_start: range.start,
        byte_end: range.end,
        start_line,
        start_column,
        end_line,
        end_column,
    }
}
pub fn path_is_allowed(binding: &RepositoryBinding, relative: &Path) -> EvidenceResult<bool> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Ok(false);
    }
    for allowed in &binding.input_paths {
        match allowed {
            QueryPath::Exact { path } if path == relative => return Ok(true),
            QueryPath::Glob { pattern } => {
                let matcher = globset::GlobBuilder::new(pattern)
                    .literal_separator(true)
                    .build()
                    .map_err(|_| EvidenceCode::InvalidSelector)?
                    .compile_matcher();
                if matcher.is_match(relative) {
                    return Ok(true);
                }
            }
            QueryPath::Exact { .. } => {}
        }
    }
    Ok(false)
}
pub async fn capture_document(
    binding: RepositoryBinding,
    path: std::path::PathBuf,
    max_bytes: usize,
    cancellation: CancellationToken,
) -> EvidenceResult<CapturedDocument> {
    capture_document_accounted(DocumentCapture {
        binding,
        path,
        max_bytes,
        cancellation,
        progress: crate::acquisition_progress::AcquisitionProgress::shared(),
    })
    .await
}
pub(crate) struct DocumentCapture {
    pub binding: RepositoryBinding,
    pub path: std::path::PathBuf,
    pub max_bytes: usize,
    pub cancellation: CancellationToken,
    pub progress: Arc<crate::acquisition_progress::AcquisitionProgress>,
}
pub(crate) async fn capture_document_accounted(
    request: DocumentCapture,
) -> EvidenceResult<CapturedDocument> {
    let DocumentCapture {
        binding,
        path,
        max_bytes,
        cancellation,
        progress,
    } = request;
    tokio::task::spawn_blocking(move || {
        let started_at = chrono::Utc::now();
        if cancellation.is_cancelled() {
            return Err(EvidenceCode::MissingContext.into());
        }
        if !path_is_allowed(&binding, &path)? {
            return Err(EvidenceCode::PathOutsideRoot.into());
        }
        let root = binding
            .root
            .canonicalize()
            .map_err(|_| EvidenceCode::SourceUnavailable)?;
        let resolved = root
            .join(&path)
            .canonicalize()
            .map_err(|_| EvidenceCode::SourceUnavailable)?;
        if !resolved.starts_with(&root) {
            return Err(EvidenceCode::PathOutsideRoot.into());
        }
        let mut file = File::open(&resolved).map_err(|_| EvidenceCode::SourceUnavailable)?;
        let before = file
            .metadata()
            .map_err(|_| EvidenceCode::SourceUnavailable)?;
        if before.len() > max_bytes as u64 {
            return Err(EvidenceCode::EvidenceLimit.into());
        }
        progress.repository_file();
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            if cancellation.is_cancelled() {
                return Err(EvidenceCode::MissingContext.into());
            }
            let remaining = max_bytes.saturating_sub(bytes.len()).min(buffer.len());
            if remaining == 0 {
                break;
            }
            let count = file
                .read(&mut buffer[..remaining])
                .map_err(|_| EvidenceCode::SourceUnavailable)?;
            if count == 0 {
                break;
            }
            progress.source_bytes(count);
            bytes.extend_from_slice(&buffer[..count]);
        }
        let after = file
            .metadata()
            .map_err(|_| EvidenceCode::SourceUnavailable)?;
        let at_path = resolved
            .metadata()
            .map_err(|_| EvidenceCode::SourceChanged)?;
        if before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
            || file_identity(&before) != file_identity(&at_path)
        {
            return Err(EvidenceCode::SourceChanged.into());
        }
        let identity = content_digest(&bytes);
        let text = String::from_utf8(bytes).map_err(|_| EvidenceCode::UnsupportedEncoding)?;
        Ok(CapturedDocument {
            interval: CaptureInterval {
                started_at,
                completed_at: chrono::Utc::now(),
            },
            source: SourceRef::Document {
                repository: binding.key,
                path,
                content_identity: identity,
            },
            text: text.into(),
        })
    })
    .await
    .map_err(|_| EvidenceCode::SourceUnavailable)?
}
#[cfg(unix)]
pub(crate) fn file_identity(metadata: &std::fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!("{}:{}", metadata.dev(), metadata.ino())
}
#[cfg(not(unix))]
pub(crate) fn file_identity(metadata: &std::fs::Metadata) -> String {
    format!("{:?}", metadata.created().ok())
}
