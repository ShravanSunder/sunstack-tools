use crate::repository_capture::{CachedAcquisition, PairCapture, RepositoryCapture};
use crate::repository_catalogue::CaptureView;
use crate::*;
use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
pub struct ChangeCaptureRequest {
    pub binding: RepositoryBinding,
    pub limits: DiscoveryLimits,
    pub anchor_utc: DateTime<Utc>,
}
/// Freeze and verify every mutable source in this Git capture before returning it.
pub async fn capture_change_selection(
    request: ChangeCaptureRequest,
    cancellation: CancellationToken,
) -> EvidenceResult<CapturedChangeSelection> {
    let cached = CachedAcquisition::fresh(&request.limits);
    capture_change_selection_cached(request, cached, cancellation).await
}
pub(crate) async fn capture_change_selection_cached(
    request: ChangeCaptureRequest,
    cached: CachedAcquisition,
    cancellation: CancellationToken,
) -> EvidenceResult<CapturedChangeSelection> {
    let capture_cancellation = cancellation.clone();
    let capture = tokio::task::spawn_blocking(move || {
        capture_blocking(request, cached, capture_cancellation)
    })
    .await
    .map_err(|_| EvidenceCode::SourceUnavailable)??;
    verify_change_capture(&capture, cancellation).await?;
    Ok(capture)
}
fn capture_blocking(
    mut request: ChangeCaptureRequest,
    cached: CachedAcquisition,
    cancellation: CancellationToken,
) -> EvidenceResult<CapturedChangeSelection> {
    request.binding.change_scope.validate()?;
    if !request.binding.change_scope.has_change_selection() {
        return Err(EvidenceCode::NoChangeSelection.into());
    }
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
    let mut capture = RepositoryCapture::new(&repository, &request, cancellation)?;
    capture.reuse_captures(cached);
    let mut members = Vec::new();
    let scopes = match &request.binding.change_scope {
        ChangeScope::Union { members } => members.clone(),
        scope => vec![scope.clone()],
    };
    for (scope_position, scope) in scopes.into_iter().enumerate() {
        capture.scope_position = scope_position;
        let first_member = members.len();
        resolve_scope(&mut capture, &scope, request.anchor_utc, &mut members)?;
        for member in &mut members[first_member..] {
            member.scope_position = scope_position;
        }
    }
    let coverage = capture.coverage();
    Ok(CapturedChangeSelection {
        repository: request.binding.key,
        anchor_utc: request.anchor_utc,
        scope: request.binding.change_scope,
        snapshot: capture.snapshot,
        members,
        coverage,
        diagnostics: capture.diagnostics,
        sources: capture.sources,
        mutable_files: capture.mutable_files,
        acquisitions: capture.acquisitions,
        scope_diagnostics: capture.scope_diagnostics,
    })
}
fn resolve_scope(
    capture: &mut RepositoryCapture<'_>,
    scope: &ChangeScope,
    anchor: DateTime<Utc>,
    members: &mut Vec<CapturedChangeMember>,
) -> EvidenceResult<()> {
    match scope {
        ChangeScope::Full {} => {
            capture.capture_context()?;
            members.push(CapturedChangeMember {
                scope_position: 0,
                basis: ChangeMemberBasis::Full,
                changes: vec![],
                recency: CapturedRecencyBasis::NotTimeScoped,
            });
        }
        ChangeScope::Union { .. } => return Err(EvidenceCode::InvalidRepositoryBinding.into()),
        ChangeScope::Uncommitted { layers } => capture_dirty(capture, layers, None, members)?,
        ChangeScope::RecentUncommitted {
            layers,
            window,
            unknown_recency,
        } => {
            let window = resolve_time_window(window, anchor)?;
            capture.windows.push(window.clone());
            capture_dirty(capture, layers, Some((window, *unknown_recency)), members)?;
        }
        ChangeScope::RevisionDiff {
            base_revision,
            head,
        } => {
            let base = resolve_commit(capture.repository, base_revision)?;
            let (after, head) = match head {
                RevisionHead::Commit { revision } => {
                    let oid = resolve_commit(capture.repository, revision)?;
                    (
                        CaptureView::Commit(oid),
                        SourceRevision::Commit {
                            commit_id: GitObjectId::new(oid.to_string())?,
                            side: SourceSide::After,
                        },
                    )
                }
                RevisionHead::Worktree {} => (
                    CaptureView::Worktree,
                    SourceRevision::Worktree {
                        snapshot: capture.snapshot.clone(),
                        side: SourceSide::After,
                    },
                ),
            };
            members.push(capture.capture_pair(PairCapture {
                before: CaptureView::Commit(base),
                after,
                basis: ChangeMemberBasis::RevisionDiff {
                    base: GitObjectId::new(base.to_string())?,
                    head,
                },
                recency: CapturedRecencyBasis::NotTimeScoped,
                unknown_recency: None,
                untracked_only: false,
            })?);
        }
        ChangeScope::CommitSet {
            commit_ids,
            parent_policy,
        } => {
            for commit_id in commit_ids {
                if !capture.checkpoint()? || !capture.admit_history_commit() {
                    break;
                }
                let oid = git2::Oid::from_str(commit_id.as_str())
                    .map_err(|_| EvidenceCode::RevisionUnavailable)?;
                capture_commit(
                    capture,
                    oid,
                    *parent_policy,
                    CapturedRecencyBasis::NotTimeScoped,
                    members,
                )?;
            }
        }
        ChangeScope::RecentCommits {
            reference,
            window,
            parent_policy,
        } => {
            let window = resolve_time_window(window, anchor)?;
            capture.windows.push(window.clone());
            let head = resolve_commit(capture.repository, reference)?;
            let mut walk = capture
                .repository
                .revwalk()
                .map_err(|_| EvidenceCode::RevisionUnavailable)?;
            walk.push(head)
                .map_err(|_| EvidenceCode::RevisionUnavailable)?;
            // Every reachable ancestor is examined up to the resource bound; dates do not prune traversal.
            if capture.repository.is_shallow() {
                capture.warn_scope(EvidenceCode::HistoryIncomplete);
                capture.limited(DiscoveryLimit::HistoryCommits);
            }
            for oid in walk {
                if !capture.checkpoint()? {
                    break;
                }
                if !capture.admit_history_commit() {
                    break;
                }
                let oid = oid.map_err(|_| EvidenceCode::HistoryIncomplete)?;
                let commit = capture
                    .repository
                    .find_commit(oid)
                    .map_err(|_| EvidenceCode::RevisionUnavailable)?;
                let time = DateTime::<Utc>::from_timestamp(commit.time().seconds(), 0)
                    .ok_or(EvidenceCode::InvalidRepositoryBinding)?;
                if time >= window.since_utc && time <= window.until_utc {
                    capture_commit(
                        capture,
                        oid,
                        *parent_policy,
                        CapturedRecencyBasis::CommitTime {
                            window: window.clone(),
                        },
                        members,
                    )?;
                }
            }
        }
    }
    Ok(())
}
fn capture_dirty(
    capture: &mut RepositoryCapture<'_>,
    layers: &[DirtyLayer],
    recent: Option<(ResolvedTimeWindow, UnknownRecencyPolicy)>,
    members: &mut Vec<CapturedChangeMember>,
) -> EvidenceResult<()> {
    let head = capture
        .repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .map_or(CaptureView::Empty, CaptureView::Commit);
    for layer in layers {
        let (before, after, untracked_only) = match layer {
            DirtyLayer::Staged => (head.clone(), CaptureView::Index, false),
            DirtyLayer::Unstaged => (CaptureView::Index, CaptureView::Worktree, false),
            DirtyLayer::Untracked => (CaptureView::Index, CaptureView::Worktree, true),
        };
        let recency = recent
            .as_ref()
            .map_or(CapturedRecencyBasis::NotTimeScoped, |(window, _)| {
                CapturedRecencyBasis::ApproximateFileMtime {
                    window: window.clone(),
                    excluded_unknown_paths: vec![],
                    included_unknown: vec![],
                }
            });
        members.push(capture.capture_pair(PairCapture {
            before,
            after,
            basis: ChangeMemberBasis::DirtyLayer { layer: *layer },
            recency,
            unknown_recency: recent.as_ref().map(|(_, policy)| *policy),
            untracked_only,
        })?);
    }
    Ok(())
}
fn capture_commit(
    capture: &mut RepositoryCapture<'_>,
    oid: git2::Oid,
    policy: CommitParentPolicy,
    recency: CapturedRecencyBasis,
    members: &mut Vec<CapturedChangeMember>,
) -> EvidenceResult<()> {
    let commit = capture
        .repository
        .find_commit(oid)
        .map_err(|_| EvidenceCode::RevisionUnavailable)?;
    let commit_id = GitObjectId::new(oid.to_string())?;
    if commit.parent_count() == 0 {
        members.push(capture.capture_pair(PairCapture {
            before: CaptureView::Empty,
            after: CaptureView::Commit(oid),
            basis: ChangeMemberBasis::Commit {
                commit_id,
                parent: ParentBasis::Root,
            },
            recency,
            unknown_recency: None,
            untracked_only: false,
        })?);
    } else {
        let count = if policy == CommitParentPolicy::FirstParent {
            1
        } else {
            commit.parent_count()
        };
        for position in 0..count {
            let parent = match commit.parent_id(position) {
                Ok(parent) => parent,
                Err(_) => {
                    capture
                        .diagnostics
                        .push(Diagnostic::new(EvidenceCode::HistoryIncomplete));
                    continue;
                }
            };
            members.push(capture.capture_pair(PairCapture {
                before: CaptureView::Commit(parent),
                after: CaptureView::Commit(oid),
                basis: ChangeMemberBasis::Commit {
                    commit_id: commit_id.clone(),
                    parent: ParentBasis::Parent {
                        commit_id: GitObjectId::new(parent.to_string())?,
                        position,
                    },
                },
                recency: recency.clone(),
                unknown_recency: None,
                untracked_only: false,
            })?);
        }
    }
    Ok(())
}
fn resolve_commit(repository: &git2::Repository, revision: &str) -> EvidenceResult<git2::Oid> {
    repository
        .revparse_single(revision)
        .and_then(|object| object.peel_to_commit())
        .map(|commit| commit.id())
        .map_err(|_| EvidenceCode::RevisionUnavailable.into())
}
pub fn resolve_time_window(
    window: &TimeWindow,
    anchor: DateTime<Utc>,
) -> EvidenceResult<ResolvedTimeWindow> {
    let (since_utc, until_utc) = match window {
        TimeWindow::Absolute {
            since_utc,
            until_utc,
        } => (*since_utc, *until_utc),
        TimeWindow::Lookback { seconds } => {
            let seconds =
                i64::try_from(seconds.get()).map_err(|_| EvidenceCode::InvalidRepositoryBinding)?;
            let duration = chrono::Duration::try_seconds(seconds)
                .ok_or(EvidenceCode::InvalidRepositoryBinding)?;
            (
                anchor
                    .checked_sub_signed(duration)
                    .ok_or(EvidenceCode::InvalidRepositoryBinding)?,
                anchor,
            )
        }
    };
    if since_utc > until_utc {
        return Err(EvidenceCode::InvalidRepositoryBinding.into());
    }
    Ok(ResolvedTimeWindow {
        anchor_utc: anchor,
        since_utc,
        until_utc,
    })
}
/// Reread mutable views once. A mismatch refuses this snapshot; it never recaptures or retries.
pub async fn verify_change_capture(
    capture: &CapturedChangeSelection,
    cancellation: CancellationToken,
) -> EvidenceResult<()> {
    let checks = capture.mutable_files.clone();
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        for check in checks {
            if cancellation.is_cancelled() {
                return Err(EvidenceCode::MissingContext.into());
            }
            match check {
                MutableRepositoryFile::Worktree { root, path, bytes } => {
                    let path = root.join(path);
                    match bytes {
                        None => {
                            if path.exists() {
                                return Err(EvidenceCode::SourceChanged.into());
                            }
                        }
                        Some(expected) => {
                            let metadata = std::fs::metadata(&path)
                                .map_err(|_| EvidenceCode::SourceChanged)?;
                            if metadata.len() != expected.len() as u64 {
                                return Err(EvidenceCode::SourceChanged.into());
                            }
                            let mut actual = Vec::new();
                            std::fs::File::open(&path)
                                .map_err(|_| EvidenceCode::SourceChanged)?
                                .take(expected.len().saturating_add(1) as u64)
                                .read_to_end(&mut actual)
                                .map_err(|_| EvidenceCode::SourceChanged)?;
                            if actual != expected {
                                return Err(EvidenceCode::SourceChanged.into());
                            }
                        }
                    }
                }
                MutableRepositoryFile::Index { root, path, blob } => {
                    let repository =
                        git2::Repository::open(root).map_err(|_| EvidenceCode::SourceChanged)?;
                    let index = repository
                        .index()
                        .map_err(|_| EvidenceCode::SourceChanged)?;
                    let actual = index
                        .get_path(&path, 0)
                        .map(|entry| GitObjectId::new(entry.id.to_string()))
                        .transpose()?;
                    if actual != blob {
                        return Err(EvidenceCode::SourceChanged.into());
                    }
                }
            }
        }
        Ok(())
    })
    .await
    .map_err(|_| EvidenceCode::SourceChanged)?
}
