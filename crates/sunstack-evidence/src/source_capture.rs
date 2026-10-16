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
pub fn source_span(source: &SourceRef, text: &str, range: std::ops::Range<usize>) -> SourceSpan {
    let position = |offset: usize| {
        let prefix = &text[..offset];
        (
            prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
            prefix
                .rsplit('\n')
                .next()
                .map_or(1, |line| line.chars().count() + 1),
        )
    };
    let (start_line, start_column) = position(range.start);
    let (end_line, end_column) = position(range.end);
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
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            if cancellation.is_cancelled() {
                return Err(EvidenceCode::MissingContext.into());
            }
            let count = file
                .read(&mut buffer)
                .map_err(|_| EvidenceCode::SourceUnavailable)?;
            if count == 0 {
                break;
            }
            if bytes.len().saturating_add(count) > max_bytes {
                return Err(EvidenceCode::EvidenceLimit.into());
            }
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
