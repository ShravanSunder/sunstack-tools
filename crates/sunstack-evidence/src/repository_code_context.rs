use crate::{
    repository_capture::{CachedAcquisition, RepositoryCapture},
    repository_catalogue::CaptureView,
    *,
};
use tokio_util::sync::CancellationToken;

pub(crate) async fn capture_revision_context(
    request: ChangeCaptureRequest,
    revision: RevisionHead,
    cached: CachedAcquisition,
    cancellation: CancellationToken,
) -> EvidenceResult<CapturedChangeSelection> {
    if matches!(revision, RevisionHead::Worktree {}) {
        return crate::repository_context::capture_code_worktree_context_cached(
            request,
            cached,
            cancellation,
        )
        .await;
    }
    let actor = cancellation.clone();
    let capture =
        tokio::task::spawn_blocking(move || -> EvidenceResult<CapturedChangeSelection> {
            let mut request = request;
            request.binding.root = request
                .binding
                .root
                .canonicalize()
                .map_err(|_| EvidenceCode::InvalidRepositoryBinding)?;
            let repository = git2::Repository::open(&request.binding.root)
                .map_err(|_| EvidenceCode::InvalidRepositoryBinding)?;
            if repository
                .workdir()
                .and_then(|path| path.canonicalize().ok())
                .as_ref()
                != Some(&request.binding.root)
            {
                return Err(EvidenceCode::InvalidRepositoryBinding.into());
            }
            let RevisionHead::Commit { revision } = revision else {
                return Err(EvidenceCode::InvalidSelector.into());
            };
            let commit = repository
                .revparse_single(&revision)
                .and_then(|object| object.peel_to_commit())
                .map_err(|_| EvidenceCode::RevisionUnavailable)?;
            let mut capture = RepositoryCapture::new(&repository, &request, actor)?;
            capture.reuse_captures(cached);
            capture
                .capture_code_context_view(&CaptureView::Commit(commit.id()), SourceSide::After)?;
            Ok(CapturedChangeSelection {
                repository: request.binding.key,
                anchor_utc: request.anchor_utc,
                scope: ChangeScope::Full {},
                snapshot: capture.snapshot.clone(),
                members: vec![],
                coverage: capture.coverage(),
                diagnostics: capture.diagnostics,
                sources: capture.sources,
                mutable_files: capture.mutable_files,
                acquisitions: capture.acquisitions,
                scope_diagnostics: capture.scope_diagnostics,
            })
        })
        .await
        .map_err(|_| EvidenceCode::SourceUnavailable)??;
    verify_change_capture(&capture, cancellation).await?;
    Ok(capture)
}
