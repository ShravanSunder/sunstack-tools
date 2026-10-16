use super::connected_sources::{RepositoryView, add_cutoffs};
use super::*;

impl Preparation {
    pub(super) async fn related_select(
        &mut self,
        query: &RelationshipQuery,
        settings: &ChunkSettings,
        minimum: usize,
        declaration: &EvidenceDeclaration,
    ) -> EvidenceResult<SelectionReadiness> {
        let mut captures = Vec::new();
        if let SourceUnitSeeds::ChangedUnits { repository_ids } = &query.source_units.seeds {
            for repository in repository_ids {
                let binding = self
                    .input
                    .repositories
                    .iter()
                    .find(|binding| &binding.key == repository)
                    .ok_or(EvidenceCode::UnknownRepository)?;
                if !binding.change_scope.has_change_selection() {
                    return Err(EvidenceCode::NoChangeSelection.into());
                }
                captures.push(
                    self.repository_capture(repository, RepositoryView::ChangeSelection)
                        .await?,
                );
            }
        }
        for repository in &query.repository_ids {
            let captured = match self
                .repository_capture(repository, RepositoryView::Context)
                .await
            {
                Ok(capture) => capture,
                Err(error) => Arc::new(unavailable_context(
                    repository.clone(),
                    self.snapshot_id.clone(),
                    self.anchor_utc,
                    error,
                )),
            };
            captures.push(captured);
        }
        let owned_captures = captures
            .iter()
            .map(|capture| capture.as_ref().clone())
            .collect::<Vec<_>>();
        let mut limits = self.discovery_limits()?;
        let source_query = query.source_units.clone();
        let seed_captures = owned_captures.clone();
        let remaining_matches = limits
            .max_matches
            .get()
            .saturating_sub(self.discovery_matches);
        let started = Instant::now();
        let seed_selection = tokio::task::spawn_blocking(move || {
            select_source_units(&seed_captures, &source_query, remaining_matches)
        })
        .await
        .map_err(|_| EvidenceCode::InvalidSelector);
        self.discovery_elapsed += started.elapsed();
        let seed_selection = seed_selection??;
        self.discovery_matches += seed_selection.matches_considered;
        self.measurements.files_parsed += seed_selection.files_parsed;
        self.measurements.bytes_scanned += seed_selection.bytes_scanned;
        if seed_selection.units.is_empty() {
            return Err(if captures.iter().any(|capture| {
                !matches!(
                    capture.coverage,
                    DiscoveryCoverage::CompleteWithinDeclaredScope { .. }
                )
            }) {
                EvidenceCode::DiscoveryLimit
            } else {
                EvidenceCode::MissingRelatedContext
            }
            .into());
        }
        let visits = limits
            .max_unit_visits
            .get()
            .saturating_sub(self.discovery_unit_visits);
        let candidates = limits
            .max_relationship_candidates
            .get()
            .saturating_sub(self.discovery_candidates);
        let remaining_ms = limits.max_local_time_ms.get().saturating_sub(
            self.discovery_elapsed
                .as_millis()
                .try_into()
                .unwrap_or(usize::MAX),
        );
        let mut exhausted = Vec::new();
        for (available, limit) in [
            (visits, DiscoveryLimit::UnitVisits),
            (candidates, DiscoveryLimit::RelationshipCandidates),
            (remaining_ms, DiscoveryLimit::LocalTime),
        ] {
            if available == 0 {
                exhausted.push(DiscoveryCutoff {
                    limit,
                    frontier: seed_selection
                        .units
                        .iter()
                        .map(|unit| unit.unit.key.clone())
                        .collect(),
                });
            }
        }
        let graph_started = Instant::now();
        let discovery_result = if exhausted.is_empty() {
            limits.max_unit_visits = visits
                .try_into()
                .map_err(|_| EvidenceCode::DiscoveryLimit)?;
            limits.max_relationship_candidates = candidates
                .try_into()
                .map_err(|_| EvidenceCode::DiscoveryLimit)?;
            limits.max_local_time_ms = remaining_ms
                .try_into()
                .map_err(|_| EvidenceCode::DiscoveryLimit)?;
            let graph_query = query.clone();
            let bindings = self.input.package_bindings.clone();
            let cancellation = self.cancellation.clone();
            let seeds = seed_selection.units;
            tokio::task::spawn_blocking(move || {
                discover_relationships(RelationshipDiscoveryRequest {
                    captures: &owned_captures,
                    seeds: &seeds,
                    query: &graph_query,
                    package_bindings: &bindings,
                    limits: &limits,
                    cancellation,
                })
            })
            .await
            .map_err(|_| EvidenceCode::SourceUnavailable.into())
            .and_then(|result| result)
        } else {
            Ok(RelationshipDiscovery {
                seeds: seed_selection.units.clone(),
                units: seed_selection.units,
                relationships: vec![],
                coverage: DiscoveryCoverage::Limited {
                    query_bounds: QueryBounds {
                        repository_ids: query.repository_ids.clone(),
                        requested_hops: query.requested_hops.get(),
                        windows: vec![],
                    },
                    cutoffs: exhausted,
                },
                import_context: BTreeMap::new(),
                unknown_recency_context: vec![],
                unit_visits: 0,
                relationship_candidates: 0,
                files_parsed: 0,
                bytes_scanned: 0,
            })
        };
        self.discovery_elapsed += graph_started.elapsed();
        let mut discovery = discovery_result?;
        self.discovery_unit_visits += discovery.unit_visits;
        self.discovery_candidates += discovery.relationship_candidates;
        self.measurements.files_parsed += discovery.files_parsed;
        self.measurements.bytes_scanned += discovery.bytes_scanned;
        discovery.coverage = add_cutoffs(discovery.coverage, seed_selection.cutoffs);
        let excluded = captures
            .iter()
            .flat_map(|capture| {
                capture
                    .members
                    .iter()
                    .flat_map(move |member| match &member.recency {
                        CapturedRecencyBasis::ApproximateFileMtime {
                            excluded_unknown_paths,
                            ..
                        } => excluded_unknown_paths
                            .iter()
                            .map(|path| (capture.repository.clone(), path.clone()))
                            .collect::<Vec<_>>(),
                        _ => vec![],
                    })
            })
            .collect::<BTreeSet<_>>();
        for capture in &captures {
            for file in &capture.sources {
                if let SourceRef::RepositoryFile {
                    repository, path, ..
                } = &file.source
                    && excluded.contains(&(repository.clone(), path.clone()))
                    && !discovery.unknown_recency_context.contains(&file.source)
                {
                    discovery.unknown_recency_context.push(file.source.clone());
                }
            }
        }
        let outcomes = build_semantic_groups(&discovery, settings, minimum)?;
        let content = match settings.grouping {
            ChunkGrouping::SingleGroup => {
                let group = outcomes
                    .into_iter()
                    .next()
                    .ok_or(EvidenceCode::InvalidContextGroup)?;
                if declaration.requirement == EvidenceRequirement::Required
                    && !matches!(group, ContextGroupOutcome::Complete { .. })
                {
                    let diagnostics = match &group {
                        ContextGroupOutcome::Partial { diagnostics, .. }
                        | ContextGroupOutcome::Unavailable { diagnostics } => diagnostics.clone(),
                        ContextGroupOutcome::Complete { .. } => DiagnosticList::default(),
                    };
                    return Ok(SelectionReadiness::Refused { diagnostics });
                }
                SelectionContent::RelatedGroup {
                    group: Box::new(group),
                }
            }
            ChunkGrouping::PerSeed => SelectionContent::RelatedGroups {
                groups: discovery
                    .seeds
                    .iter()
                    .zip(outcomes)
                    .map(|(seed, outcome)| SeedContextGroup {
                        seed: seed.unit.key.clone(),
                        outcome,
                    })
                    .collect(),
            },
        };
        self.admit_connected(content, discovery.coverage, captures, declaration)
    }
}
fn unavailable_context(
    repository: RepositoryId,
    snapshot: SnapshotId,
    anchor_utc: chrono::DateTime<chrono::Utc>,
    error: EvidenceError,
) -> CapturedChangeSelection {
    let mut diagnostics = error.diagnostics();
    for diagnostic in &mut diagnostics.items {
        diagnostic.location = DiagnosticLocation::Field {
            field_path: vec!["repositories".into(), repository.as_str().into()],
        };
    }
    CapturedChangeSelection {
        repository,
        anchor_utc,
        scope: ChangeScope::Full,
        snapshot,
        members: vec![],
        coverage: DiscoveryCoverage::Unavailable {
            diagnostics: diagnostics.clone(),
        },
        diagnostics,
        sources: vec![],
        mutable_files: vec![],
        acquisitions: vec![],
        scope_diagnostics: std::collections::BTreeMap::new(),
    }
}
