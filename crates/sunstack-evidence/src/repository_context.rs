use crate::repository_capture::CachedAcquisition;
use crate::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};
use tokio_util::sync::CancellationToken;

/// Freeze an allowed current-source context. Full-scope roots need not be Git repositories.
pub async fn capture_repository_context(
    request: ChangeCaptureRequest,
    cancellation: CancellationToken,
) -> EvidenceResult<CapturedChangeSelection> {
    let cached = CachedAcquisition::fresh(&request.limits);
    capture_repository_context_cached(request, cached, cancellation).await
}
pub(crate) async fn capture_repository_context_cached(
    mut request: ChangeCaptureRequest,
    cached: CachedAcquisition,
    cancellation: CancellationToken,
) -> EvidenceResult<CapturedChangeSelection> {
    let started = Instant::now();
    request.binding.root = request
        .binding
        .root
        .canonicalize()
        .map_err(|_| EvidenceCode::SourceUnavailable)?;
    let binding = request.binding.clone();
    let include_ignored = request.limits.include_ignored;
    let max_time_ms = request.limits.max_local_time_ms.get();
    let max_files = request.limits.max_files.get().min(cached.remaining_files);
    let max_bytes = request
        .limits
        .max_total_bytes
        .get()
        .min(cached.remaining_bytes);
    let cached_paths = cached.worktree.keys().cloned().collect::<BTreeSet<_>>();
    let scan_cached_paths = cached_paths.clone();
    let scan_cancellation = cancellation.clone();
    let (paths, mut metadata_by_path, scan_timed_out) = tokio::task::spawn_blocking(move || {
        let mut paths = BTreeSet::new();
        let mut timed_out = false;
        for entry in ignore::WalkBuilder::new(&binding.root)
            .hidden(false)
            .parents(false)
            .git_global(false)
            .git_ignore(!include_ignored)
            .git_exclude(!include_ignored)
            .filter_entry(|entry| entry.file_name() != ".git")
            .build()
        {
            if scan_cancellation.is_cancelled() {
                return Err(EvidenceCode::MissingContext.into());
            }
            if started.elapsed().as_millis() >= max_time_ms as u128 {
                timed_out = true;
                break;
            }
            let entry = entry.map_err(|_| EvidenceCode::SourceUnavailable)?;
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                continue;
            }
            let path = entry
                .path()
                .strip_prefix(&binding.root)
                .map_err(|_| EvidenceCode::PathOutsideRoot)?;
            if path_is_allowed(&binding, path)? {
                paths.insert(path.to_path_buf());
            }
            if paths.difference(&scan_cached_paths).count() > max_files {
                break;
            }
        }
        let mut metadata_by_path = BTreeMap::new();
        for path in paths.difference(&scan_cached_paths) {
            if scan_cancellation.is_cancelled() {
                return Err(EvidenceCode::MissingContext.into());
            }
            if started.elapsed().as_millis() >= max_time_ms as u128 {
                timed_out = true;
                break;
            }
            metadata_by_path.insert(path.clone(), std::fs::metadata(binding.root.join(path)));
        }
        Ok::<_, EvidenceError>((paths, metadata_by_path, timed_out))
    })
    .await
    .map_err(|_| EvidenceCode::SourceUnavailable)??;
    let snapshot = cached.snapshot.clone().map(Ok).unwrap_or_else(|| {
        SnapshotId::new(content_digest(
            format!(
                "{}:{}",
                request.binding.root.display(),
                chrono::Utc::now().to_rfc3339()
            )
            .as_bytes(),
        ))
    })?;
    let paths = paths.union(&cached_paths).cloned().collect::<BTreeSet<_>>();
    let mut sources = Vec::new();
    let mut mutable_files = Vec::new();
    let mut cutoffs = Vec::new();
    if scan_timed_out {
        cutoff(&mut cutoffs, DiscoveryLimit::LocalTime);
    }
    let mut diagnostics = DiagnosticList::default();
    let mut bytes = 0usize;
    let mut acquired_files = 0usize;
    let mut acquisitions = Vec::new();
    for path in paths {
        if cancellation.is_cancelled() {
            return Err(EvidenceCode::MissingContext.into());
        }
        if let Some(document) = cached.worktree.get(&path) {
            if path_is_allowed(&request.binding, &path)? {
                sources.push(FrozenRepositoryFile {
                    source: SourceRef::RepositoryFile {
                        repository: request.binding.key.clone(),
                        revision: SourceRevision::Worktree {
                            snapshot: snapshot.clone(),
                            side: SourceSide::After,
                        },
                        path,
                        content_identity: content_digest(document.text.as_bytes()),
                    },
                    text: document.text.to_string(),
                    side: SourceSide::After,
                });
            }
            continue;
        }
        if started.elapsed().as_millis() >= request.limits.max_local_time_ms.get() as u128 {
            cutoff(&mut cutoffs, DiscoveryLimit::LocalTime);
            break;
        }
        if acquired_files >= max_files {
            cutoff(&mut cutoffs, DiscoveryLimit::Files);
            break;
        }
        let metadata = match metadata_by_path
            .remove(&path)
            .ok_or(EvidenceCode::SourceUnavailable)?
        {
            Ok(metadata) => metadata,
            Err(_) => {
                diagnostics.push(Diagnostic::new(EvidenceCode::SourceUnavailable));
                continue;
            }
        };
        if metadata.len() > request.limits.max_file_bytes.get() as u64 {
            cutoff(&mut cutoffs, DiscoveryLimit::FileBytes);
            continue;
        }
        if metadata.len() > max_bytes.saturating_sub(bytes) as u64 {
            cutoff(&mut cutoffs, DiscoveryLimit::TotalBytes);
            continue;
        }
        let document = crate::source_capture::capture_document_accounted(
            crate::source_capture::DocumentCapture {
                binding: request.binding.clone(),
                path: path.clone(),
                max_bytes: request
                    .limits
                    .max_file_bytes
                    .get()
                    .min(max_bytes.saturating_sub(bytes)),
                cancellation: cancellation.clone(),
                progress: cached.progress.clone(),
            },
        )
        .await?;
        let text = document.text.to_string();
        bytes += text.len();
        acquired_files += 1;
        mutable_files.push(MutableRepositoryFile::Worktree {
            root: request.binding.root.clone(),
            path: path.clone(),
            bytes: Some(text.as_bytes().to_vec()),
        });
        let file = FrozenRepositoryFile {
            source: SourceRef::RepositoryFile {
                repository: request.binding.key.clone(),
                revision: SourceRevision::Worktree {
                    snapshot: snapshot.clone(),
                    side: SourceSide::After,
                },
                path,
                content_identity: content_digest(text.as_bytes()),
            },
            text,
            side: SourceSide::After,
        };
        acquisitions.push(crate::repository_capture::repository_file_coverage(&file));
        sources.push(file);
    }
    let query_bounds = QueryBounds {
        repository_ids: vec![request.binding.key.clone()],
        requested_hops: 0,
        windows: vec![],
    };
    let coverage = if !diagnostics.items.is_empty() {
        DiscoveryCoverage::Unavailable {
            diagnostics: diagnostics.clone(),
        }
    } else if cutoffs.is_empty() {
        DiscoveryCoverage::CompleteWithinDeclaredScope { query_bounds }
    } else {
        DiscoveryCoverage::Limited {
            query_bounds,
            cutoffs,
        }
    };
    let capture = CapturedChangeSelection {
        repository: request.binding.key,
        anchor_utc: request.anchor_utc,
        scope: request.binding.change_scope,
        snapshot,
        members: vec![],
        sources,
        mutable_files,
        coverage,
        diagnostics,
        acquisitions,
        scope_diagnostics: std::collections::BTreeMap::new(),
    };
    verify_change_capture(&capture, cancellation).await?;
    Ok(capture)
}
fn cutoff(cutoffs: &mut Vec<DiscoveryCutoff>, limit: DiscoveryLimit) {
    if !cutoffs.iter().any(|cutoff| cutoff.limit == limit) {
        cutoffs.push(DiscoveryCutoff {
            limit,
            frontier: vec![],
        });
    }
}
