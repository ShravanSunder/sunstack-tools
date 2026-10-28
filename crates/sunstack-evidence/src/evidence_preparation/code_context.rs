use super::*;
use crate::repository_capture::CachedAcquisition;
impl Preparation {
    pub(crate) async fn build_code_context(
        &mut self,
        input: &SourceInput,
    ) -> EvidenceResult<CodeContextIndex> {
        let input_id = input.source_identity()?;
        if let Some(index) = self.snapshot.code_contexts.get(&input_id) {
            return index
                .as_ref()
                .map(|index| index.as_ref().clone())
                .map_err(Clone::clone);
        }
        let repository = match input {
            SourceInput::Diff { repository, .. } | SourceInput::Code { repository, .. } => {
                repository
            }
            _ => return Err(EvidenceCode::InvalidSelector.into()),
        };
        let mut binding = self
            .input
            .repositories
            .iter()
            .find(|binding| binding.key == *repository)
            .cloned()
            .ok_or(EvidenceCode::UnknownRepository)?;
        binding.change_scope = match input {
            SourceInput::Diff { scope, .. } => scope.clone(),
            _ => ChangeScope::Full {},
        };
        let mut limits = self.discovery_limits()?;
        let remaining = limits.max_local_time_ms.get().saturating_sub(
            self.discovery_elapsed
                .as_millis()
                .try_into()
                .unwrap_or(usize::MAX),
        );
        limits.max_local_time_ms = remaining
            .try_into()
            .map_err(|_| EvidenceCode::DiscoveryLimit)?;
        let progress = crate::acquisition_progress::AcquisitionProgress::shared();
        let cached = CachedAcquisition {
            worktree: self
                .document_cache
                .iter()
                .filter(|((key, _), _)| key == repository)
                .map(|((_, path), document)| (path.clone(), document.clone()))
                .collect(),
            remaining_files: limits
                .max_files
                .get()
                .saturating_sub(self.measurements.files_read),
            remaining_bytes: limits.max_total_bytes.get().saturating_sub(self.total_read),
            snapshot: Some(self.snapshot_id.clone()),
            progress: progress.clone(),
            remaining_history_commits: limits
                .max_history_commits
                .get()
                .saturating_sub(self.discovery_history_commits),
        };
        let request = ChangeCaptureRequest {
            binding,
            limits: limits.clone(),
            anchor_utc: self.anchor_utc,
        };
        let started = Instant::now();
        let captured = match input {
            SourceInput::Diff { .. } => {
                crate::change_selection::capture_code_selection_cached(
                    request,
                    cached,
                    self.cancellation.clone(),
                )
                .await
            }
            SourceInput::Code { revision, .. } => {
                crate::repository_code_context::capture_revision_context(
                    request,
                    revision.clone(),
                    cached,
                    self.cancellation.clone(),
                )
                .await
            }
            _ => return Err(EvidenceCode::InvalidSelector.into()),
        };
        self.measurements.files_read += progress.consumed_repository_files();
        self.measurements.files_considered += progress.consumed_repository_files();
        self.total_read += progress.consumed_bytes();
        self.discovery_history_commits += progress.consumed_history_commits();
        if let Ok(capture) = &captured {
            self.captures.extend(capture.acquisitions.clone());
            for file in &capture.sources {
                if let SourceRef::RepositoryFile {
                    path,
                    revision: SourceRevision::Worktree { .. },
                    ..
                } = &file.source
                {
                    self.document_cache
                        .entry((repository.clone(), path.clone()))
                        .or_insert_with(|| {
                            Arc::new(CapturedDocument {
                                interval: CaptureInterval {
                                    started_at: self.anchor_utc,
                                    completed_at: chrono::Utc::now(),
                                },
                                source: file.source.clone(),
                                text: Arc::from(file.text.as_str()),
                            })
                        });
                }
            }
        }
        let actor = self.cancellation.clone();
        let source = input.clone();
        let result = match captured {
            Ok(capture) => tokio::task::spawn_blocking(move || {
                crate::code_context::build_index(source, capture, limits, actor)
            })
            .await
            .map_err(|_| EvidenceCode::MissingContext)?,
            Err(error) => Err(error),
        };
        self.discovery_elapsed += started.elapsed();
        if let Ok(index) = &result {
            self.measurements.files_parsed += index.capture.sources.len();
            self.measurements.bytes_scanned += index
                .capture
                .sources
                .iter()
                .map(|file| file.text.len())
                .sum::<usize>();
        }
        self.snapshot
            .code_contexts
            .insert(input_id, result.clone().map(Arc::new));
        result
    }
}
