use super::*;
use crate::repository_capture::CachedAcquisition;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum RepositoryView {
    ChangeSelection,
    Context,
}

impl Preparation {
    pub(super) fn discovery_limits(&self) -> EvidenceResult<DiscoveryLimits> {
        let EvidenceDiscovery::Bounded { limits } = &self.input.discovery else {
            return Err(EvidenceCode::DiscoveryNotEnabled.into());
        };
        let mut limits = limits.clone();
        limits.max_files = limits.max_files.min(self.input.limits.max_files);
        limits.max_file_bytes = limits.max_file_bytes.min(self.input.limits.max_file_bytes);
        limits.max_total_bytes = limits
            .max_total_bytes
            .min(self.input.limits.max_total_bytes);
        limits.max_matches = limits.max_matches.min(self.input.limits.max_matches);
        Ok(limits)
    }
    pub(super) async fn repository_capture(
        &mut self,
        repository: &RepositoryId,
        view: RepositoryView,
    ) -> EvidenceResult<Arc<CapturedChangeSelection>> {
        let key = (repository.clone(), view);
        if let Some(capture) = self.snapshot.repository_views.get(&key) {
            return capture.clone();
        }
        let binding = self
            .input
            .repositories
            .iter()
            .find(|binding| &binding.key == repository)
            .ok_or(EvidenceCode::UnknownRepository)?
            .clone();
        let mut limits = self.discovery_limits()?;
        let remaining_ms = limits.max_local_time_ms.get().saturating_sub(
            self.discovery_elapsed
                .as_millis()
                .try_into()
                .unwrap_or(usize::MAX),
        );
        if remaining_ms == 0 {
            return Err(EvidenceCode::DiscoveryLimit.into());
        }
        limits.max_local_time_ms = remaining_ms
            .try_into()
            .map_err(|_| EvidenceCode::DiscoveryLimit)?;
        let progress = crate::acquisition_progress::AcquisitionProgress::shared();
        let cached = CachedAcquisition {
            worktree: self
                .document_cache
                .iter()
                .filter(|((source_repository, _), _)| source_repository == repository)
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
            binding: binding.clone(),
            limits,
            anchor_utc: self.anchor_utc,
        };
        let started = Instant::now();
        let result = match view {
            RepositoryView::ChangeSelection => {
                crate::change_selection::capture_change_selection_cached(
                    request,
                    cached,
                    self.cancellation.clone(),
                )
                .await
            }
            RepositoryView::Context => {
                crate::repository_context::capture_repository_context_cached(
                    request,
                    cached,
                    self.cancellation.clone(),
                )
                .await
            }
        };
        self.discovery_elapsed += started.elapsed();
        self.measurements.files_read += progress.consumed_repository_files();
        self.measurements.files_considered += progress.consumed_repository_files();
        self.total_read += progress.consumed_bytes();
        self.discovery_history_commits += progress.consumed_history_commits();
        let result = result.map(Arc::new);
        if let Ok(capture) = &result {
            self.captures.extend(capture.acquisitions.clone());
            // The existing document cache remains the authority for current bytes.
            // Blob versions never enter it, even when their paths are identical.
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
            if view == RepositoryView::ChangeSelection {
                self.snapshot
                    .change_selections
                    .insert(repository.clone(), report_change_selection(capture)?);
            }
        } else if view == RepositoryView::ChangeSelection {
            self.snapshot.change_selections.insert(
                repository.clone(),
                ResolvedChangeSelection {
                    repository: repository.clone(),
                    anchor_utc: self.anchor_utc,
                    members: vec![],
                    coverage: DiscoveryCoverage::Unavailable {
                        diagnostics: result
                            .as_ref()
                            .err()
                            .map_or_else(DiagnosticList::default, EvidenceError::diagnostics),
                    },
                },
            );
        }
        self.snapshot.repository_views.insert(key, result.clone());
        result
    }
    pub(super) async fn connected_select(
        &mut self,
        repository: &RepositoryId,
        declaration: &EvidenceDeclaration,
    ) -> EvidenceResult<SelectionReadiness> {
        let binding = self
            .input
            .repositories
            .iter()
            .find(|binding| &binding.key == repository)
            .ok_or(EvidenceCode::UnknownRepository)?;
        let changed_seeds = matches!(
            declaration.select,
            EvidenceSelection::ChangedUnits { .. } | EvidenceSelection::RelatedUnits { .. }
        );
        if changed_seeds && !binding.change_scope.has_change_selection() {
            return Err(EvidenceCode::NoChangeSelection.into());
        }
        if let EvidenceSelection::RelatedUnits {
            languages,
            unit_kinds,
            source_sides,
            directions,
            requested_hops,
            chunk_settings,
            minimum_related_units,
        } = &declaration.select
        {
            let query = RelationshipQuery {
                source_units: SourceUnitQuery {
                    seeds: SourceUnitSeeds::ChangedUnits {
                        repository_ids: vec![repository.clone()],
                    },
                    languages: languages
                        .iter()
                        .map(|language| match language {
                            GraphLanguage::Rust => SourceLanguage::Rust,
                            GraphLanguage::Typescript => SourceLanguage::Typescript,
                            GraphLanguage::Tsx => SourceLanguage::Tsx,
                        })
                        .collect(),
                    unit_kinds: unit_kinds.clone(),
                    source_sides: *source_sides,
                },
                repository_ids: self
                    .input
                    .repositories
                    .iter()
                    .map(|binding| binding.key.clone())
                    .collect(),
                directions: directions.clone(),
                requested_hops: *requested_hops,
            };
            return self
                .related_select(&query, chunk_settings, *minimum_related_units, declaration)
                .await;
        }
        let view = if binding.change_scope.has_change_selection() {
            RepositoryView::ChangeSelection
        } else {
            RepositoryView::Context
        };
        let capture = self.repository_capture(repository, view).await?;
        let limits = self.discovery_limits()?;
        let capture_for_selection = capture.clone();
        let selection = declaration.select.clone();
        let remaining = limits
            .max_matches
            .get()
            .saturating_sub(self.discovery_matches);
        let selected = tokio::task::spawn_blocking(move || {
            select_change_set(&capture_for_selection, &selection, remaining)
        })
        .await
        .map_err(|_| EvidenceCode::InvalidSelector)??;
        self.discovery_matches += selected.matches_considered;
        self.measurements.files_parsed += selected.files_parsed;
        self.measurements.bytes_scanned += selected.bytes_scanned;
        let coverage = add_cutoffs(capture.coverage.clone(), selected.cutoffs);
        self.admit_connected(selected.content, coverage, vec![capture], declaration)
    }
    pub(super) fn admit_connected(
        &mut self,
        content: SelectionContent,
        discovery: DiscoveryCoverage,
        captures: Vec<Arc<CapturedChangeSelection>>,
        declaration: &EvidenceDeclaration,
    ) -> EvidenceResult<SelectionReadiness> {
        let mut sources = Vec::new();
        for capture in &captures {
            for file in &capture.sources {
                if !sources.contains(&file.source) {
                    sources.push(file.source.clone());
                }
            }
        }
        let repository = captures
            .first()
            .map(|capture| capture.repository.clone())
            .ok_or(EvidenceCode::MissingContext)?;
        let items = match &content {
            SelectionContent::Matches { matches } => matches.len(),
            SelectionContent::ChangedUnits { units } => units.len(),
            SelectionContent::RelatedGroups { groups } => groups.len(),
            _ => 1,
        };
        let bytes = serde_json::to_vec(&content)
            .map_err(|_| EvidenceCode::EvidenceLimit)?
            .len();
        if declaration
            .max_items
            .is_some_and(|limit| items > limit.get())
            || declaration
                .max_bytes
                .is_some_and(|limit| bytes > limit.get())
        {
            return Err(EvidenceCode::SelectorLimit.into());
        }
        let omissions = match &content {
            SelectionContent::RelatedGroup { group } => match group.as_ref() {
                ContextGroupOutcome::Partial { omissions, .. } => omissions.clone(),
                _ => vec![],
            },
            _ => vec![],
        };
        self.measurements.bytes_included += bytes;
        Ok(SelectionReadiness::Ready {
            content,
            omissions,
            coverage: Box::new(SelectionCoverage::ChangeSet {
                repository,
                sources,
                discovery,
            }),
        })
    }
}
pub(super) fn add_cutoffs(
    coverage: DiscoveryCoverage,
    additional: Vec<DiscoveryCutoff>,
) -> DiscoveryCoverage {
    if additional.is_empty() {
        return coverage;
    }
    match coverage {
        DiscoveryCoverage::CompleteWithinDeclaredScope { query_bounds } => {
            DiscoveryCoverage::Limited {
                query_bounds,
                cutoffs: additional,
            }
        }
        DiscoveryCoverage::Limited {
            query_bounds,
            mut cutoffs,
        } => {
            cutoffs.extend(additional);
            DiscoveryCoverage::Limited {
                query_bounds,
                cutoffs,
            }
        }
        other => other,
    }
}
