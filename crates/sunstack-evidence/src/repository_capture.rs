use crate::repository_catalogue::{CaptureView, FileCatalogue, ReadTarget};
use crate::*;
use git2::Repository;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
use tokio_util::sync::CancellationToken;

pub(crate) fn repository_file_coverage(file: &FrozenRepositoryFile) -> CaptureCoverage {
    CaptureCoverage {
        source: file.source.clone(),
        cursor_basis: CursorBasis::NoCursorSupplied,
        tail: CaptureTail::Complete,
        gaps: vec![],
        bytes_read: file.text.len() as u64,
        bytes_parsed: file.text.len() as u64,
    }
}

pub(crate) struct CachedAcquisition {
    pub worktree: BTreeMap<PathBuf, Arc<CapturedDocument>>,
    pub remaining_files: usize,
    pub remaining_bytes: usize,
    pub snapshot: Option<SnapshotId>,
    pub progress: Arc<crate::acquisition_progress::AcquisitionProgress>,
    pub remaining_history_commits: usize,
}
impl CachedAcquisition {
    pub fn fresh(limits: &DiscoveryLimits) -> Self {
        Self {
            worktree: BTreeMap::new(),
            remaining_files: limits.max_files.get(),
            remaining_bytes: limits.max_total_bytes.get(),
            snapshot: None,
            progress: crate::acquisition_progress::AcquisitionProgress::shared(),
            remaining_history_commits: limits.max_history_commits.get(),
        }
    }
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ReadIdentity {
    Blob { path: PathBuf, object_id: git2::Oid },
    Worktree { path: PathBuf },
}

pub(crate) struct PairCapture {
    pub before: CaptureView,
    pub after: CaptureView,
    pub basis: ChangeMemberBasis,
    pub recency: CapturedRecencyBasis,
    pub unknown_recency: Option<UnknownRecencyPolicy>,
    pub untracked_only: bool,
}
pub(crate) struct RepositoryCapture<'repo> {
    pub repository: &'repo Repository,
    pub binding: RepositoryBinding,
    pub limits: DiscoveryLimits,
    pub snapshot: SnapshotId,
    pub sources: Vec<FrozenRepositoryFile>,
    pub mutable_files: Vec<MutableRepositoryFile>,
    pub cutoffs: Vec<DiscoveryCutoff>,
    pub windows: Vec<ResolvedTimeWindow>,
    pub diagnostics: DiagnosticList,
    pub acquisitions: Vec<CaptureCoverage>,
    pub scope_position: usize,
    pub scope_diagnostics: BTreeMap<usize, DiagnosticList>,
    cache: BTreeMap<ReadIdentity, Vec<u8>>,
    pub(crate) total_bytes: usize,
    pub(crate) read_files: usize,
    available_files: usize,
    available_bytes: usize,
    available_history_commits: usize,
    history_commits: usize,
    progress: Arc<crate::acquisition_progress::AcquisitionProgress>,
    started: Instant,
    cancellation: CancellationToken,
}
impl<'repo> RepositoryCapture<'repo> {
    pub fn new(
        repository: &'repo Repository,
        request: &ChangeCaptureRequest,
        cancellation: CancellationToken,
    ) -> EvidenceResult<Self> {
        let snapshot = SnapshotId::new(content_digest(
            format!(
                "{}:{}",
                request.binding.root.display(),
                chrono::Utc::now().to_rfc3339()
            )
            .as_bytes(),
        ))?;
        Ok(Self {
            repository,
            binding: request.binding.clone(),
            limits: request.limits.clone(),
            snapshot,
            sources: vec![],
            mutable_files: vec![],
            cutoffs: vec![],
            windows: vec![],
            diagnostics: DiagnosticList::default(),
            acquisitions: vec![],
            scope_position: 0,
            scope_diagnostics: BTreeMap::new(),
            cache: BTreeMap::new(),
            total_bytes: 0,
            read_files: 0,
            available_files: request.limits.max_files.get(),
            available_bytes: request.limits.max_total_bytes.get(),
            available_history_commits: request.limits.max_history_commits.get(),
            history_commits: 0,
            progress: crate::acquisition_progress::AcquisitionProgress::shared(),
            started: Instant::now(),
            cancellation,
        })
    }
    pub(crate) fn reuse_captures(&mut self, cached: CachedAcquisition) {
        self.progress = cached.progress;
        self.available_history_commits = self
            .available_history_commits
            .min(cached.remaining_history_commits);
        if let Some(snapshot) = cached.snapshot {
            self.snapshot = snapshot;
        }
        self.available_files = self.available_files.min(cached.remaining_files);
        self.available_bytes = self.available_bytes.min(cached.remaining_bytes);
        for (path, document) in cached.worktree {
            self.cache.insert(
                ReadIdentity::Worktree { path },
                document.text.as_bytes().to_vec(),
            );
        }
    }
    pub fn warn_scope(&mut self, code: EvidenceCode) {
        let diagnostics = self
            .scope_diagnostics
            .entry(self.scope_position)
            .or_default();
        if !diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == code)
        {
            diagnostics.push(Diagnostic::new(code));
        }
        if !self
            .diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == code)
        {
            self.diagnostics.push(Diagnostic::new(code));
        }
    }
    pub fn admit_history_commit(&mut self) -> bool {
        if self.history_commits >= self.available_history_commits {
            self.limited(DiscoveryLimit::HistoryCommits);
            self.warn_scope(EvidenceCode::HistoryIncomplete);
            false
        } else {
            self.history_commits += 1;
            self.progress.history_commit();
            true
        }
    }
    pub(super) fn cached_worktree_paths(&self) -> Vec<PathBuf> {
        self.cache
            .keys()
            .filter_map(|identity| match identity {
                ReadIdentity::Worktree { path } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }
    pub fn limited(&mut self, limit: DiscoveryLimit) {
        if !self.cutoffs.iter().any(|cutoff| cutoff.limit == limit) {
            self.cutoffs.push(DiscoveryCutoff {
                limit,
                frontier: vec![],
            });
        }
    }
    pub(super) fn check_cancel(&self) -> EvidenceResult<()> {
        if self.cancellation.is_cancelled() {
            Err(EvidenceCode::MissingContext.into())
        } else {
            Ok(())
        }
    }
    pub(super) fn time_available(&mut self) -> bool {
        if self.deadline_exhausted() {
            self.limited(DiscoveryLimit::LocalTime);
            false
        } else {
            true
        }
    }
    pub(super) fn deadline_exhausted(&self) -> bool {
        self.started.elapsed().as_millis() >= self.limits.max_local_time_ms.get() as u128
    }
    fn read_side(
        &mut self,
        catalogue: &FileCatalogue,
        path: &PathBuf,
        side: SourceSide,
    ) -> EvidenceResult<Option<CapturedFileSide>> {
        self.check_cancel()?;
        let FileCatalogue::Captured { revision, entries } = catalogue else {
            return Ok(Some(CapturedFileSide::Absent));
        };
        let Some(target) = entries.get(path) else {
            match revision {
                SourceRevision::Index { .. } => {
                    self.mutable_files.push(MutableRepositoryFile::Index {
                        root: self.binding.root.clone(),
                        path: path.clone(),
                        blob: None,
                    })
                }
                SourceRevision::Worktree { .. } => {
                    self.mutable_files.push(MutableRepositoryFile::Worktree {
                        root: self.binding.root.clone(),
                        path: path.clone(),
                        bytes: None,
                    })
                }
                SourceRevision::Commit { .. } => {}
            }
            return Ok(Some(CapturedFileSide::Absent));
        };
        let read_identity = match target {
            ReadTarget::Blob(oid) => ReadIdentity::Blob {
                path: path.clone(),
                object_id: *oid,
            },
            ReadTarget::Worktree => ReadIdentity::Worktree { path: path.clone() },
        };
        if let ReadTarget::Blob(oid) = target
            && matches!(revision, SourceRevision::Index { .. })
        {
            self.mutable_files.push(MutableRepositoryFile::Index {
                root: self.binding.root.clone(),
                path: path.clone(),
                blob: Some(GitObjectId::new(oid.to_string())?),
            });
        }
        let newly_acquired = !self.cache.contains_key(&read_identity);
        let bytes = if let Some(bytes) = self.cache.get(&read_identity) {
            bytes.clone()
        } else {
            if self.read_files >= self.available_files {
                self.limited(DiscoveryLimit::Files);
                return Ok(None);
            }
            if !self.time_available() {
                return Ok(None);
            }
            let bytes = match target {
                ReadTarget::Blob(oid) => {
                    let blob = self
                        .repository
                        .find_blob(*oid)
                        .map_err(|_| EvidenceCode::RevisionUnavailable)?;
                    if blob.size() > self.limits.max_file_bytes.get() {
                        self.limited(DiscoveryLimit::FileBytes);
                        return Ok(None);
                    }
                    if self.total_bytes.saturating_add(blob.size()) > self.available_bytes {
                        self.limited(DiscoveryLimit::TotalBytes);
                        return Ok(None);
                    }
                    self.progress.repository_file();
                    self.progress.source_bytes(blob.size());
                    blob.content().to_vec()
                }
                ReadTarget::Worktree => {
                    let resolved = self
                        .binding
                        .root
                        .join(path)
                        .canonicalize()
                        .map_err(|_| EvidenceCode::SourceUnavailable)?;
                    if !resolved.starts_with(&self.binding.root) {
                        return Err(EvidenceCode::PathOutsideRoot.into());
                    }
                    let metadata = std::fs::metadata(&resolved)
                        .map_err(|_| EvidenceCode::SourceUnavailable)?;
                    if metadata.len() > self.limits.max_file_bytes.get() as u64 {
                        self.limited(DiscoveryLimit::FileBytes);
                        return Ok(None);
                    }
                    if metadata.len() > self.available_bytes.saturating_sub(self.total_bytes) as u64
                    {
                        self.limited(DiscoveryLimit::TotalBytes);
                        return Ok(None);
                    }
                    let mut bytes = Vec::new();
                    let file = std::fs::File::open(&resolved)
                        .map_err(|_| EvidenceCode::SourceUnavailable)?;
                    self.progress.repository_file();
                    let read = file
                        .take(
                            self.limits
                                .max_file_bytes
                                .get()
                                .min(self.available_bytes.saturating_sub(self.total_bytes))
                                as u64,
                        )
                        .read_to_end(&mut bytes);
                    self.progress.source_bytes(bytes.len());
                    read.map_err(|_| EvidenceCode::SourceUnavailable)?;
                    if bytes.len() > self.limits.max_file_bytes.get() {
                        self.limited(DiscoveryLimit::FileBytes);
                        return Ok(None);
                    }
                    if bytes.len() > self.available_bytes.saturating_sub(self.total_bytes) {
                        self.limited(DiscoveryLimit::TotalBytes);
                        return Ok(None);
                    }
                    self.mutable_files.push(MutableRepositoryFile::Worktree {
                        root: self.binding.root.clone(),
                        path: path.clone(),
                        bytes: Some(bytes.clone()),
                    });
                    bytes
                }
            };
            self.read_files += 1;
            self.total_bytes += bytes.len();
            self.cache.insert(read_identity, bytes.clone());
            bytes
        };
        let identity = match target {
            ReadTarget::Blob(oid) => oid.to_string(),
            ReadTarget::Worktree => content_digest(&bytes),
        };
        let text = String::from_utf8(bytes).map_err(|_| EvidenceCode::UnsupportedEncoding)?;
        let file = FrozenRepositoryFile {
            source: SourceRef::RepositoryFile {
                repository: self.binding.key.clone(),
                revision: revision.clone(),
                path: path.clone(),
                content_identity: identity,
            },
            text,
            side,
        };
        if newly_acquired {
            self.acquisitions.push(repository_file_coverage(&file));
        }
        if !self
            .sources
            .iter()
            .any(|existing| existing.source == file.source)
        {
            self.sources.push(file.clone());
        }
        Ok(Some(CapturedFileSide::Present { file }))
    }
    pub fn capture_pair(&mut self, request: PairCapture) -> EvidenceResult<CapturedChangeMember> {
        let before = self.catalogue(&request.before, SourceSide::Before)?;
        let after = self.catalogue(&request.after, SourceSide::After)?;
        let before_paths = before.paths().into_iter().collect::<BTreeSet<_>>();
        let paths = before_paths
            .iter()
            .cloned()
            .chain(after.paths())
            .collect::<BTreeSet<_>>();
        let mut changes = Vec::new();
        let mut excluded_unknown_paths = Vec::new();
        let mut included_unknown = Vec::new();
        for path in paths {
            if request.untracked_only {
                if before_paths.contains(&path) {
                    continue;
                }
            } else if matches!(request.after, CaptureView::Worktree)
                && matches!(request.before, CaptureView::Index)
                && !before_paths.contains(&path)
            {
                continue;
            }
            let recency_unknown =
                if let (Some(policy), CapturedRecencyBasis::ApproximateFileMtime { window, .. }) =
                    (request.unknown_recency, &request.recency)
                {
                    let staged = matches!(
                        request.basis,
                        ChangeMemberBasis::DirtyLayer {
                            layer: DirtyLayer::Staged
                        }
                    );
                    // Tree and index identities locate staged candidates without admitting file bytes.
                    if staged && before.blob_id(&path) == after.blob_id(&path) {
                        continue;
                    }
                    let mtime = if staged || !after.paths().contains(&path) {
                        None
                    } else {
                        std::fs::metadata(self.binding.root.join(&path))
                            .ok()
                            .and_then(|metadata| metadata.modified().ok())
                            .map(chrono::DateTime::<chrono::Utc>::from)
                    };
                    match mtime {
                        Some(time) if time <= window.anchor_utc => {
                            if time < window.since_utc || time > window.until_utc {
                                continue;
                            }
                            false
                        }
                        _ if policy == UnknownRecencyPolicy::ExcludeWithOmission => {
                            excluded_unknown_paths.push(path.clone());
                            continue;
                        }
                        _ => true,
                    }
                } else {
                    false
                };
            let Some(old) = self.read_side(&before, &path, SourceSide::Before)? else {
                continue;
            };
            let Some(new) = self.read_side(&after, &path, SourceSide::After)? else {
                continue;
            };
            let old_text = side_bytes(&old);
            let new_text = side_bytes(&new);
            if old == CapturedFileSide::Absent && new == CapturedFileSide::Absent {
                continue;
            }
            if old_text == new_text && std::mem::discriminant(&old) == std::mem::discriminant(&new)
            {
                continue;
            }
            if recency_unknown {
                included_unknown.push(path.clone());
                self.diagnostics
                    .push(Diagnostic::new(EvidenceCode::RecencyUnknown));
            }
            let (old_spans, new_spans) = buffer_change_spans(old_text, new_text)?;
            changes.push(CapturedChange {
                path,
                before: old,
                after: new,
                before_spans: old_spans,
                after_spans: new_spans,
            });
        }
        let recency = match request.recency {
            CapturedRecencyBasis::ApproximateFileMtime { window, .. } => {
                CapturedRecencyBasis::ApproximateFileMtime {
                    window,
                    excluded_unknown_paths,
                    included_unknown,
                }
            }
            other => other,
        };
        Ok(CapturedChangeMember {
            scope_position: 0,
            basis: request.basis,
            changes,
            recency,
        })
    }
    pub fn capture_context(&mut self) -> EvidenceResult<()> {
        let catalogue = self.catalogue(&CaptureView::Worktree, SourceSide::After)?;
        for path in catalogue.paths() {
            self.read_side(&catalogue, &path, SourceSide::After)?;
        }
        Ok(())
    }
    pub fn checkpoint(&mut self) -> EvidenceResult<bool> {
        self.check_cancel()?;
        Ok(self.time_available())
    }
    pub fn coverage(&self) -> DiscoveryCoverage {
        if self
            .diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == EvidenceCode::HistoryIncomplete)
        {
            return DiscoveryCoverage::Unavailable {
                diagnostics: self.diagnostics.clone(),
            };
        }
        let query_bounds = QueryBounds {
            repository_ids: vec![self.binding.key.clone()],
            requested_hops: 0,
            windows: self.windows.clone(),
        };
        if self.cutoffs.is_empty() {
            DiscoveryCoverage::CompleteWithinDeclaredScope { query_bounds }
        } else {
            DiscoveryCoverage::Limited {
                query_bounds,
                cutoffs: self.cutoffs.clone(),
            }
        }
    }
}
fn side_bytes(side: &CapturedFileSide) -> &[u8] {
    match side {
        CapturedFileSide::Present { file } => file.text.as_bytes(),
        CapturedFileSide::Absent => &[],
    }
}
fn buffer_change_spans(
    before: &[u8],
    after: &[u8],
) -> EvidenceResult<(Vec<ChangeSpan>, Vec<ChangeSpan>)> {
    let mut options = git2::DiffOptions::new();
    options.context_lines(0).force_text(true);
    let patch = git2::Patch::from_buffers(before, None, after, None, Some(&mut options))
        .map_err(|_| EvidenceCode::SourceUnavailable)?;
    let mut old = Vec::new();
    let mut new = Vec::new();
    for index in 0..patch.num_hunks() {
        let (hunk, _) = patch
            .hunk(index)
            .map_err(|_| EvidenceCode::SourceUnavailable)?;
        old.push(line_span(before, hunk.old_start(), hunk.old_lines()));
        new.push(line_span(after, hunk.new_start(), hunk.new_lines()));
    }
    Ok((old, new))
}
fn line_span(bytes: &[u8], start: u32, count: u32) -> ChangeSpan {
    let mut boundaries = vec![0];
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            boundaries.push(index + 1);
        }
    }
    if boundaries.last().copied() != Some(bytes.len()) {
        boundaries.push(bytes.len());
    }
    let line = if count == 0 {
        start as usize
    } else {
        start.saturating_sub(1) as usize
    };
    ChangeSpan {
        byte_start: boundaries.get(line).copied().unwrap_or(bytes.len()),
        byte_end: boundaries
            .get(line + count as usize)
            .copied()
            .unwrap_or(bytes.len()),
    }
}
